//! A persistent `kannaka chat --json` child process.
//!
//! Recall alone makes a poor conversationalist. Resonating a greeting returns
//! whatever the medium happens to associate with the word "hi" — five unrelated
//! ADRs at 20% confidence — which is correct behaviour for a search and wrong
//! behaviour for an agent someone just said hello to.
//!
//! `kannaka chat` already solves this: it reasons over the medium instead of
//! dumping matches. The cost is the substrate load, so the REPL is spawned
//! ONCE and every turn afterwards is just the model call. This mirrors the
//! ChatSession the Hive agent has run since ADR-0045 Phase 2+.
//!
//! ## Protocol (from `src/bin/handlers/chat.rs`)
//!
//! - **stdin**: one plain-text line per turn. A leading `/` makes it a slash
//!   command that bypasses the model entirely, so user text must be stripped of
//!   one — otherwise "/help me with X" silently becomes a command.
//! - **stdout**: NDJSON, one object per line, `{"kind","text"}` where kind is
//!   `chat` | `slash` | `error` | `chunk`. `chunk` frames are partial and are
//!   followed by a terminal `chat` carrying the full reply.
//! - **stderr**: `{"kind":"ready","memories":N}` once the medium is loaded.
//!
//! ## Why the reader is a thread
//!
//! This crate has no async runtime (deliberately — see `mod.rs`), and a
//! blocking read on a dead child would hang the agent forever. A reader thread
//! feeding a channel lets the caller apply a timeout without one.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

/// How long a single turn may take before the REPL is considered wedged.
///
/// Generous on purpose: a cold turn pays the model call plus whatever the
/// medium costs, and killing a slow-but-live REPL is worse than waiting.
const TURN_TIMEOUT: Duration = Duration::from_secs(90);

/// How long to wait for the `ready` banner before giving up on a spawn.
///
/// This covers the full substrate load (~16s for a 47 MB medium), so it has to
/// be well clear of it.
const READY_TIMEOUT: Duration = Duration::from_secs(120);

/// One line of the child's output, tagged with the stream it came from.
enum Line {
    Out(String),
    Ready,
    Eof,
}

/// A live `kannaka chat --json` process.
pub struct ChatRepl {
    /// Path to the `kannaka` binary, kept for respawning.
    bin: PathBuf,
    data_dir: PathBuf,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    rx: Option<Receiver<Line>>,
}

impl ChatRepl {
    /// Prepare a REPL without starting it. Nothing is spawned until the first
    /// turn, so the ACP handshake never pays the substrate load.
    pub fn new(bin: PathBuf, data_dir: PathBuf) -> Self {
        Self {
            bin,
            data_dir,
            child: None,
            stdin: None,
            rx: None,
        }
    }

    /// Locate the `kannaka` binary: explicit override, then alongside this
    /// executable (how a bundled install ships), then PATH.
    pub fn find_binary() -> Option<PathBuf> {
        if let Ok(explicit) = std::env::var("KANNAKA_BIN") {
            let p = PathBuf::from(explicit);
            if p.is_file() {
                return Some(p);
            }
        }
        let exe_name = if cfg!(windows) { "kannaka.exe" } else { "kannaka" };
        if let Ok(me) = std::env::current_exe() {
            if let Some(dir) = me.parent() {
                let sibling = dir.join(exe_name);
                if sibling.is_file() {
                    return Some(sibling);
                }
            }
        }
        // PATH lookup without spawning a shell.
        std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|dir| dir.join(exe_name))
                .find(|candidate| candidate.is_file())
        })
    }

    /// Start the child and block until it reports `ready`.
    fn spawn(&mut self) -> Result<(), String> {
        let mut child = Command::new(&self.bin)
            .arg("chat")
            .arg("--json")
            .env("KANNAKA_DATA_DIR", &self.data_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn {}: {e}", self.bin.display()))?;

        let stdin = child.stdin.take().ok_or("child stdin unavailable")?;
        let stdout = child.stdout.take().ok_or("child stdout unavailable")?;
        let stderr = child.stderr.take().ok_or("child stderr unavailable")?;

        let (tx, rx) = mpsc::channel();

        // stdout: the NDJSON reply stream.
        let out_tx = tx.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                match line {
                    Ok(l) => {
                        if out_tx.send(Line::Out(l)).is_err() {
                            return;
                        }
                    }
                    Err(_) => break,
                }
            }
            let _ = out_tx.send(Line::Eof);
        });

        // stderr: carries the `ready` banner, and is otherwise diagnostic.
        // Forwarded so a misconfigured child is visible rather than silent.
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if line.contains("\"ready\"") {
                    let _ = tx.send(Line::Ready);
                } else if !line.trim().is_empty() {
                    eprintln!("[kannaka-acp] chat: {line}");
                }
            }
        });

        // Block until the medium is loaded, so the first turn is not charged
        // for it inside the shorter per-turn timeout.
        let started = std::time::Instant::now();
        loop {
            match rx.recv_timeout(READY_TIMEOUT) {
                Ok(Line::Ready) => break,
                // Output before `ready` is not expected; keep waiting rather
                // than discarding a REPL that is merely chatty on startup.
                Ok(_) => continue,
                Err(_) => {
                    let _ = child.kill();
                    return Err("chat REPL never reported ready".into());
                }
            }
        }
        eprintln!(
            "[kannaka-acp] chat REPL ready in {:.1}s — reasoning enabled",
            started.elapsed().as_secs_f32()
        );

        self.child = Some(child);
        self.stdin = Some(stdin);
        self.rx = Some(rx);
        Ok(())
    }

    /// Drop the child so the next turn spawns a fresh one.
    fn reset(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        self.stdin = None;
        self.rx = None;
    }

    /// Ask one question. `Ok(None)` means "no answer, fall back to recall" —
    /// reserved for cases where the REPL is unavailable or misbehaving, so a
    /// caller always has something to say.
    pub fn ask(&mut self, query: &str) -> Result<Option<String>, String> {
        if self.child.is_none() {
            self.spawn()?;
        }

        // One turn is one line, so newlines would split it into several turns
        // and desynchronise request from reply. A leading `/` would be read as
        // a slash command and bypass the model.
        let flattened = query.replace(['\r', '\n'], " ");
        let line = flattened.trim_start_matches('/').trim();
        if line.is_empty() {
            return Ok(None);
        }

        let write_result = self
            .stdin
            .as_mut()
            .ok_or("chat REPL stdin missing")
            .and_then(|w| {
                writeln!(w, "{line}").map_err(|_| "chat REPL stdin closed")?;
                w.flush().map_err(|_| "chat REPL flush failed")
            });
        if let Err(e) = write_result {
            self.reset();
            return Err(e.to_string());
        }

        let rx = self.rx.as_ref().ok_or("chat REPL reader missing")?;
        loop {
            match rx.recv_timeout(TURN_TIMEOUT) {
                Ok(Line::Out(l)) => {
                    let Ok(v) = serde_json::from_str::<serde_json::Value>(&l) else {
                        continue; // Not our protocol; ignore rather than fail.
                    };
                    let text = v["text"].as_str().unwrap_or_default().to_string();
                    match v["kind"].as_str().unwrap_or_default() {
                        // Partial; the terminal `chat` frame repeats the whole
                        // reply, so streaming fragments are not accumulated.
                        "chunk" => continue,
                        "chat" | "slash" => {
                            return Ok((!text.trim().is_empty()).then_some(text));
                        }
                        "error" => {
                            // The REPL is alive and declined this turn — recall
                            // can still answer, so this is not fatal.
                            eprintln!("[kannaka-acp] chat error: {text}");
                            return Ok(None);
                        }
                        _ => continue,
                    }
                }
                Ok(Line::Ready) => continue,
                Ok(Line::Eof) => {
                    self.reset();
                    return Err("chat REPL exited".into());
                }
                Err(RecvTimeoutError::Timeout) => {
                    // A wedged REPL would poison every later turn, so replace it.
                    self.reset();
                    return Err(format!("chat REPL timed out after {TURN_TIMEOUT:?}"));
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.reset();
                    return Err("chat REPL reader disconnected".into());
                }
            }
        }
    }
}

impl Drop for ChatRepl {
    fn drop(&mut self) {
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_binary_prefers_explicit_override() {
        // A non-file override must be ignored rather than returned blindly.
        std::env::set_var("KANNAKA_BIN", "/definitely/not/a/real/binary");
        let found = ChatRepl::find_binary();
        std::env::remove_var("KANNAKA_BIN");
        assert!(
            found.map(|p| p.to_string_lossy().contains("not/a/real")) != Some(true),
            "a non-existent override must not be accepted"
        );
    }

    #[test]
    fn ask_without_a_binary_reports_the_failure() {
        let mut repl = ChatRepl::new(
            PathBuf::from("/definitely/not/a/real/binary"),
            PathBuf::from("."),
        );
        let err = repl.ask("hello").expect_err("spawn must fail");
        assert!(err.contains("failed to spawn"), "got: {err}");
    }
}
