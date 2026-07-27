//! ACP (Agent Client Protocol) agent for Kannaka.
//!
//! Exposes Kannaka's holographic memory as an ACP agent over stdio, so any ACP
//! client can drive it:
//!
//! ```text
//! Buzz Relay ──WS──→ buzz-acp ──ACP/stdio──→ kannaka-acp ──→ HRM (read-only)
//! ```
//!
//! The same binary registers in the Buzz desktop "bring your own harness"
//! gallery, which discovers harnesses from JSON definitions and spawns them
//! over ACP stdio — so no fork of Buzz is required.
//!
//! ## Read-only by policy, not by convention
//!
//! The HRM is single-writer: only the main `kannaka` process may persist to it
//! (see `oracle-hrm-single-writer`). `kannaka-acp` is a *reader* — it answers
//! prompts by resonating queries through the medium. [`HrmMemory::open`]
//! therefore enforces read-only in-process rather than trusting the operator to
//! export `KANNAKA_READONLY`, mirroring `attention serve` and `swarm`.
//!
//! ## stdout is protocol-only
//!
//! Every diagnostic goes to stderr. A stray `println!` corrupts the frame
//! stream and the client dies with a parse error.

pub mod buzz_cli;
mod prompt;
pub mod protocol;
mod render;
pub mod server;

use protocol::{decode, decode_error_frame};
use server::{Agent, MemorySource, Recollection};
use std::io::{BufRead, Write};
use std::path::PathBuf;

/// Default number of memories surfaced per prompt.
pub const DEFAULT_TOP_K: usize = 5;

/// Resolve the HRM data directory.
///
/// Mirrors the CLI's precedence: `KANNAKA_DATA_DIR` > `~/.kannaka` (when it
/// exists) > `./.kannaka`.
pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("KANNAKA_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Some(home) = dirs::home_dir() {
        let home_kannaka = home.join(".kannaka");
        if home_kannaka.exists() {
            return home_kannaka;
        }
    }
    PathBuf::from(".kannaka")
}

/// A [`MemorySource`] backed by the real holographic medium, opened read-only.
///
/// ## The medium loads lazily, and that is load-bearing
///
/// Opening the HRM reads and reconstructs the whole tensor — ~16s for a 47 MB
/// `kannaka.hrm`. ACP clients treat a silent agent as a dead one: `buzz-acp`'s
/// helper subcommands give an adapter **10 seconds** to answer `initialize`, so
/// an eager open makes the agent look broken before it can say hello.
///
/// The handshake therefore must not touch the substrate. The first
/// `session/prompt` pays the load, which fits comfortably inside the per-turn
/// idle timeout (60s by default) — and a client that only probes capabilities
/// never pays it at all.
pub struct HrmMemory {
    data_dir: PathBuf,
    /// `None` until the first recall forces the open.
    sys: Option<crate::openclaw::KannakaMemorySystem>,
}

impl HrmMemory {
    /// Prepare to serve from the medium at `data_dir` without opening it.
    ///
    /// Read-only is asserted here, before any code path can construct a store:
    /// `KANNAKA_READONLY` covers the code that consults the env directly, and
    /// `set_readonly(true)` in [`Self::system`] covers the HRM itself. Neither
    /// alone closes the write path, and the HRM is single-writer.
    pub fn new(data_dir: PathBuf) -> Self {
        std::env::set_var("KANNAKA_READONLY", "1");
        Self {
            data_dir,
            sys: None,
        }
    }

    /// Borrow the medium, opening it on first use.
    fn system(&mut self) -> Result<&mut crate::openclaw::KannakaMemorySystem, String> {
        if self.sys.is_none() {
            eprintln!(
                "[kannaka-acp] opening HRM at {} (first recall)",
                self.data_dir.display()
            );
            let started = std::time::Instant::now();

            let mut sys = crate::openclaw::KannakaMemorySystem::init(self.data_dir.clone())
                .map_err(|e| format!("failed to open HRM: {e}"))?;

            if let Some(hrm) = sys
                .engine
                .store
                .as_any_mut()
                .downcast_mut::<crate::hrm_store::HrmStore>()
            {
                hrm.set_readonly(true);
            }
            eprintln!(
                "[kannaka-acp] HRM open in {:.1}s — read-only enforced (single-writer policy)",
                started.elapsed().as_secs_f32()
            );

            self.sys = Some(sys);
        }
        // Just assigned above when it was absent.
        Ok(self.sys.as_mut().expect("system initialized"))
    }
}

impl MemorySource for HrmMemory {
    fn recall(&mut self, query: &str, top_k: usize) -> Result<Vec<Recollection>, String> {
        let hits = self
            .system()?
            .recall(query, top_k)
            .map_err(|e| format!("recall failed: {e}"))?;
        Ok(hits
            .into_iter()
            .map(|m| Recollection {
                content: m.content,
                similarity: m.similarity,
                age_hours: m.age_hours,
            })
            .collect())
    }
}

/// Serve ACP over the given streams until `input` reaches EOF.
///
/// EOF is the client hanging up and is a clean shutdown, not an error — the
/// reference client kills the agent process to end a session.
///
/// Split from [`run`] so tests can drive a full protocol conversation over
/// in-memory buffers.
pub fn serve<M, R, W>(agent: &mut Agent<M>, input: R, out: &mut W) -> std::io::Result<()>
where
    M: MemorySource,
    R: BufRead,
    W: Write,
{
    for line in protocol::lines(input) {
        let line = line?;
        let frames = match decode(&line) {
            Ok(inbound) => agent.handle(inbound),
            // Malformed input is answered and the loop continues: one bad frame
            // should not take down a session that is otherwise healthy.
            Err(err) => {
                eprintln!("[kannaka-acp] decode error: {err:?}");
                vec![decode_error_frame(&err)]
            }
        };
        for frame in &frames {
            frame.write(out)?;
        }
    }
    Ok(())
}

/// Entry point: open the medium read-only and serve ACP on stdin/stdout.
pub fn run(top_k: usize) -> Result<(), String> {
    let dir = data_dir();
    eprintln!(
        "[kannaka-acp] v{} · data_dir={} · top_k={top_k}",
        env!("CARGO_PKG_VERSION"),
        dir.display()
    );

    // Deliberately does not open the HRM — see `HrmMemory` on why the ACP
    // handshake must not block on loading the medium.
    let memory = HrmMemory::new(dir);
    let mut agent = Agent::new(memory, top_k);

    // Attach a channel sink only if the `buzz` CLI is actually runnable.
    // Probing once here turns a missing CLI into one startup line instead of a
    // failure on every turn. Without a sink the agent still answers — it just
    // streams, which is exactly right for the desktop harness gallery.
    let cli = buzz_cli::BuzzCli::new(
        std::env::var("BUZZ_CLI").unwrap_or_else(|_| "buzz".to_string()),
    );
    if cli.is_available() {
        eprintln!("[kannaka-acp] buzz CLI found — replies will post to the channel");
        agent = agent.with_sink(Box::new(cli));
    } else {
        eprintln!("[kannaka-acp] buzz CLI not found — streaming replies only");
    }

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    // Lock both for the process lifetime: this is the only sanctioned writer.
    let reader = stdin.lock();
    let mut writer = stdout.lock();

    serve(&mut agent, reader, &mut writer).map_err(|e| format!("transport error: {e}"))?;
    eprintln!("[kannaka-acp] client disconnected");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    /// A memory source with one fixed hit, for end-to-end transport tests.
    struct StubMemory;

    impl MemorySource for StubMemory {
        fn recall(&mut self, _query: &str, _top_k: usize) -> Result<Vec<Recollection>, String> {
            Ok(vec![Recollection {
                content: "kannaka remembers".to_string(),
                similarity: 0.8,
                age_hours: 1.0,
            }])
        }
    }

    /// Run `input` through a full serve loop and return the emitted frames.
    fn converse(input: &str) -> Vec<Value> {
        let mut agent = Agent::new(StubMemory, DEFAULT_TOP_K);
        let mut out = Vec::new();
        serve(&mut agent, input.as_bytes(), &mut out).unwrap();
        String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    #[test]
    fn full_handshake_and_prompt_round_trip() {
        let frames = converse(concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":2}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"cwd":".","mcpServers":[]}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"kannaka-1","prompt":[{"type":"text","text":"what do you remember"}]}}"#,
            "\n",
        ));

        // initialize, session/new, session/update, session/prompt
        assert_eq!(frames.len(), 4);
        assert_eq!(frames[0]["result"]["protocolVersion"], 2);
        assert_eq!(frames[1]["result"]["sessionId"], "kannaka-1");
        assert_eq!(frames[2]["method"], "session/update");
        assert!(frames[2]["params"]["update"]["content"]["text"]
            .as_str()
            .unwrap()
            .contains("kannaka remembers"));
        assert_eq!(frames[3]["result"]["stopReason"], "end_turn");
        // Every frame must carry the JSON-RPC version tag.
        assert!(frames.iter().all(|f| f["jsonrpc"] == "2.0"));
    }

    #[test]
    fn notifications_emit_no_response_frames() {
        let frames = converse(concat!(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
            "\n",
            r#"{"jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":"kannaka-1"}}"#,
            "\n",
        ));
        // Only the initialize response.
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0]["id"], 1);
    }

    #[test]
    fn malformed_line_is_answered_and_the_session_survives() {
        let frames = converse(concat!(
            "{not json\n",
            r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{}}"#,
            "\n",
        ));
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0]["error"]["code"], protocol::error_code::PARSE_ERROR);
        // The next request is still served — one bad frame is not fatal.
        assert_eq!(frames[1]["result"]["protocolVersion"], server::PROTOCOL_VERSION);
    }

    #[test]
    fn eof_without_input_is_a_clean_shutdown() {
        assert!(converse("").is_empty());
    }
}
