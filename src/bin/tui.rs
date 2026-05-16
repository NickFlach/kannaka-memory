//! kannaka-tui — Rich terminal dashboard for the Kannaka constellation.
//!
//! A full-screen TUI built on ratatui + crossterm that shells out to the
//! `kannaka` CLI binary for all memory operations, status polling, and
//! dream control.  This binary is a pure FRONTEND — it never links
//! against kannaka-memory as a library.

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph, Tabs, Wrap},
    Frame, Terminal,
};
use std::io;
use std::process::Command;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Colour palette — the Kannaka brand
// ---------------------------------------------------------------------------

const BG: Color = Color::Rgb(10, 10, 26);
const ACCENT: Color = Color::Rgb(123, 104, 238); // purple
const SUCCESS: Color = Color::Rgb(74, 222, 128);
const ERROR: Color = Color::Rgb(248, 113, 113);
const WARNING: Color = Color::Rgb(251, 191, 36);
const INFO: Color = Color::Rgb(0, 229, 255);
const TEXT: Color = Color::Rgb(224, 224, 224);
const DIM: Color = Color::Rgb(102, 102, 102);

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct Message {
    role: Role,
    content: String,
}

#[derive(Clone)]
enum Role {
    User,
    System,
    Result,
    Error,
}

#[derive(Clone)]
struct MemoryEntry {
    content: String,
    amplitude: f32,
}

#[derive(Clone, Default)]
struct Status {
    phi: f32,
    xi: f32,
    order: f32,
    memories: u64,
    clusters: u64,
    links: u64,
    level: String,
    active: u64,
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct App {
    active_tab: usize,
    tabs: Vec<&'static str>,
    input: String,
    cursor_pos: usize,
    messages: Vec<Message>,
    memories: Vec<MemoryEntry>,
    status: Option<Status>,
    agent_name: String,
    should_quit: bool,
    scroll_offset: usize,
    last_status_poll: Instant,
    show_help: bool,
    history: Vec<String>,
    history_idx: Option<usize>,
    kannaka_bin: String,
    // Chat tab — persistent conversation with the agent. Each turn shells
    // out to `kannaka ask --session kannaka-tui` in a background thread so
    // the UI doesn't block during the API round-trip.
    chat_messages: Vec<ChatLine>,
    chat_pending: Option<std::sync::mpsc::Receiver<Result<String, String>>>,
    chat_tick: usize,
    // Async status/observe loading — set when a background thread is
    // working on a fresh poll, drained by the event loop. Without this
    // the initial `App::new()` would block ~30s on the first
    // `kannaka status` (eigendecomp on ~600 memories) and the TUI
    // looked like it never started.
    status_pending: Option<std::sync::mpsc::Receiver<Result<Status, String>>>,
    observe_pending: Option<std::sync::mpsc::Receiver<Result<(u64, Vec<MemoryEntry>), String>>>,
    // Persistent `kannaka chat --json` child process — HRM loads once
    // at first chat turn, every subsequent turn reuses the loaded
    // medium for ~3-5s per turn instead of 30s per `kannaka ask`.
    chat_child: Option<ChatChildHandle>,
    chat_child_rx: Option<std::sync::mpsc::Receiver<ChatChildEvent>>,
    chat_pending_msg: Option<String>,
}

/// Handle to the spawned `kannaka chat --json` child. Stdin is held here
/// so the main thread can write user turns into it; stdout/stderr are
/// owned by reader threads inside the spawn helper and dispatch events
/// back via `chat_child_rx`.
struct ChatChildHandle {
    stdin: Option<std::process::ChildStdin>,
    ready: bool,
}

/// Events streamed from the chat-child worker threads back to the TUI.
enum ChatChildEvent {
    /// First event after spawn — hands stdin over for turn-sending.
    Stdin(std::process::ChildStdin),
    /// Child printed its `{"kind":"ready"}` line on stderr — HRM loaded.
    Ready,
    /// One NDJSON line from stdout: a response (chat / slash / error).
    Response { kind: String, text: String },
    /// Child exited or pipe broke. Next turn will re-spawn.
    Closed(String),
}

#[derive(Clone)]
struct ChatLine {
    who: ChatWho,
    text: String,
}

#[derive(Clone, PartialEq, Eq)]
enum ChatWho { User, Kannaka, System }

impl App {
    fn new() -> Self {
        // Find the kannaka binary — prefer the release build next to us
        let kannaka_bin = Self::find_kannaka_binary();
        let agent_name = Self::load_agent_name();

        Self {
            // Chat is the primary surface. The other tabs are still
            // reachable via Tab/Shift+Tab but the user lands in chat.
            active_tab: 4,
            tabs: vec!["Memory", "Status", "Constellation", "Dreams", "Chat"],
            input: String::new(),
            cursor_pos: 0,
            messages: vec![Message {
                role: Role::System,
                content: format!(
                    "Welcome to Kannaka TUI. Agent: {}. Type a command or press F1 for help.",
                    agent_name
                ),
            }],
            memories: Vec::new(),
            status: None,
            agent_name,
            should_quit: false,
            scroll_offset: 0,
            last_status_poll: Instant::now() - Duration::from_secs(60), // force initial poll
            show_help: false,
            history: Vec::new(),
            history_idx: None,
            kannaka_bin,
            chat_messages: vec![ChatLine {
                who: ChatWho::System,
                text: "Chat with Kannaka. Memories surface via wave resonance each turn. Enter to send.".into(),
            }],
            chat_pending: None,
            chat_tick: 0,
            status_pending: None,
            observe_pending: None,
            chat_child: None,
            chat_child_rx: None,
            chat_pending_msg: None,
        }
    }

    fn find_kannaka_binary() -> String {
        // Check for release build next to this binary
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let sibling = dir.join("kannaka.exe");
                if sibling.exists() {
                    return sibling.to_string_lossy().to_string();
                }
                let sibling = dir.join("kannaka");
                if sibling.exists() {
                    return sibling.to_string_lossy().to_string();
                }
            }
        }
        // Fallback: the known release path
        let release = dirs::home_dir()
            .map(|h| h.join("Source/kannaka-memory/target/release/kannaka.exe"))
            .unwrap_or_default();
        if release.exists() {
            return release.to_string_lossy().to_string();
        }
        // Last resort: rely on PATH
        "kannaka".to_string()
    }

    fn load_agent_name() -> String {
        let config_path = dirs::home_dir()
            .map(|h| h.join(".kannaka/config.toml"))
            .unwrap_or_default();
        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                // Parse TOML for agent.id or agent.display_name
                if let Ok(val) = content.parse::<toml::Table>() {
                    if let Some(agent) = val.get("agent").and_then(|a| a.as_table()) {
                        if let Some(name) = agent.get("display_name").and_then(|v| v.as_str()) {
                            if !name.is_empty() {
                                return name.to_string();
                            }
                        }
                        if let Some(id) = agent.get("id").and_then(|v| v.as_str()) {
                            return id.to_string();
                        }
                    }
                }
            }
        }
        "unknown".to_string()
    }

    /// Spawn a background `kannaka status` poll. The TUI used to block
    /// `App::new()` on this for ~30s while the eigendecomp ran on the
    /// loaded HRM — users thought the TUI hadn't started. Now we kick
    /// off a worker and drain its result in the event loop.
    fn load_status(&mut self) {
        if self.status_pending.is_some() { return; } // already in flight
        let bin = self.kannaka_bin.clone();
        let (tx, rx) = std::sync::mpsc::channel::<Result<Status, String>>();
        self.status_pending = Some(rx);
        self.last_status_poll = Instant::now();
        std::thread::spawn(move || {
            let output = Command::new(&bin)
                .args(["status"])
                .env("KANNAKA_QUIET", "1")
                .output();
            let result = match output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    serde_json::from_str::<serde_json::Value>(&stdout)
                        .map(|val| Status {
                            phi: val["phi"].as_f64().unwrap_or(0.0) as f32,
                            xi: val["xi"].as_f64().unwrap_or(0.0) as f32,
                            order: val["mean_order"].as_f64().unwrap_or(0.0) as f32,
                            memories: val["total_memories"].as_u64().unwrap_or(0),
                            clusters: val["num_clusters"].as_u64().unwrap_or(0),
                            links: 0,
                            level: val["consciousness_level"]
                                .as_str()
                                .unwrap_or("Unknown")
                                .to_string(),
                            active: val["active_memories"].as_u64().unwrap_or(0),
                        })
                        .map_err(|e| format!("status parse: {e}"))
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    Err(format!("status failed: {}", stderr.trim()))
                }
                Err(e) => Err(format!("status spawn failed at '{}': {e}", bin)),
            };
            let _ = tx.send(result);
        });
    }

    /// Spawn a background `kannaka observe --json` poll. Same async
    /// pattern as load_status — never blocks the event loop.
    fn load_observe(&mut self) {
        if self.observe_pending.is_some() { return; }
        let bin = self.kannaka_bin.clone();
        let (tx, rx) = std::sync::mpsc::channel::<Result<(u64, Vec<MemoryEntry>), String>>();
        self.observe_pending = Some(rx);
        std::thread::spawn(move || {
            let output = Command::new(&bin)
                .args(["observe", "--json"])
                .env("KANNAKA_QUIET", "1")
                .output();
            let result = match output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    match serde_json::from_str::<serde_json::Value>(&stdout) {
                        Ok(val) => {
                            let links = val["topology"]["total_links"].as_u64().unwrap_or(0);
                            let memories = val["waves"]["strongest"].as_array()
                                .map(|arr| arr.iter()
                                    .map(|m| MemoryEntry {
                                        content: m["content_preview"].as_str().unwrap_or("").to_string(),
                                        amplitude: m["amplitude"].as_f64().unwrap_or(0.0) as f32,
                                    })
                                    .collect())
                                .unwrap_or_default();
                            Ok((links, memories))
                        }
                        Err(e) => Err(format!("observe parse: {e}")),
                    }
                }
                Ok(_) => Err("observe failed".to_string()),
                Err(e) => Err(format!("observe spawn failed: {e}")),
            };
            let _ = tx.send(result);
        });
    }

    /// Drain async status/observe responses if ready. Called every event
    /// loop tick. Non-blocking.
    fn poll_async_data(&mut self) {
        if let Some(rx) = &self.status_pending {
            match rx.try_recv() {
                Ok(Ok(s)) => { self.status = Some(s); self.status_pending = None; }
                Ok(Err(e)) => {
                    self.messages.push(Message { role: Role::Error, content: e });
                    self.status_pending = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(_) => { self.status_pending = None; }
            }
        }
        if let Some(rx) = &self.observe_pending {
            match rx.try_recv() {
                Ok(Ok((links, mems))) => {
                    if let Some(ref mut s) = self.status { s.links = links; }
                    self.memories = mems;
                    self.observe_pending = None;
                }
                Ok(Err(e)) => {
                    self.messages.push(Message { role: Role::Error, content: e });
                    self.observe_pending = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(_) => { self.observe_pending = None; }
            }
        }
    }

    fn execute_remember(&mut self, text: &str) {
        self.messages.push(Message {
            role: Role::User,
            content: format!("remember \"{}\"", text),
        });

        let output = Command::new(&self.kannaka_bin)
            .args(["remember", text])
            .env("KANNAKA_QUIET", "1")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let id = String::from_utf8_lossy(&out.stdout).trim().to_string();
                self.messages.push(Message {
                    role: Role::Result,
                    content: format!("Stored (id: {})", id),
                });
                // Refresh memories list
                self.load_observe();
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.messages.push(Message {
                    role: Role::Error,
                    content: format!("Error: {}", stderr.trim()),
                });
            }
            Err(e) => {
                self.messages.push(Message {
                    role: Role::Error,
                    content: format!("Failed to run kannaka: {}", e),
                });
            }
        }
    }

    fn execute_recall(&mut self, query: &str) {
        self.messages.push(Message {
            role: Role::User,
            content: format!("recall \"{}\"", query),
        });

        let start = Instant::now();
        let output = Command::new(&self.kannaka_bin)
            .args(["recall", query, "--top-k", "5"])
            .env("KANNAKA_QUIET", "1")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let elapsed = start.elapsed();
                let stdout = String::from_utf8_lossy(&out.stdout);
                if let Ok(results) = serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                    self.messages.push(Message {
                        role: Role::System,
                        content: format!(
                            "{} results ({:.0}ms):",
                            results.len(),
                            elapsed.as_secs_f64() * 1000.0
                        ),
                    });
                    for (i, r) in results.iter().enumerate() {
                        let content = r["content"].as_str().unwrap_or("?");
                        let sim = r["similarity"].as_f64().unwrap_or(0.0);
                        // Truncate content for display
                        let preview: String = content.chars().take(60).collect();
                        self.messages.push(Message {
                            role: Role::Result,
                            content: format!("  {}. {} ({:.2})", i + 1, preview, sim),
                        });
                    }
                } else {
                    self.messages.push(Message {
                        role: Role::Result,
                        content: stdout.trim().to_string(),
                    });
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.messages.push(Message {
                    role: Role::Error,
                    content: format!("Error: {}", stderr.trim()),
                });
            }
            Err(e) => {
                self.messages.push(Message {
                    role: Role::Error,
                    content: format!("Failed: {}", e),
                });
            }
        }
    }

    fn execute_dream(&mut self) {
        self.messages.push(Message {
            role: Role::User,
            content: "dream --mode deep".to_string(),
        });
        self.messages.push(Message {
            role: Role::System,
            content: "Starting dream cycle...".to_string(),
        });

        let output = Command::new(&self.kannaka_bin)
            .args(["dream", "--mode", "deep"])
            .env("KANNAKA_QUIET", "1")
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);
                if out.status.success() {
                    self.messages.push(Message {
                        role: Role::Result,
                        content: format!("Dream complete. {}", stdout.trim()),
                    });
                } else {
                    self.messages.push(Message {
                        role: Role::Error,
                        content: format!("Dream failed: {}", stderr.trim()),
                    });
                }
                // Refresh status after dream
                self.load_status();
                self.load_observe();
            }
            Err(e) => {
                self.messages.push(Message {
                    role: Role::Error,
                    content: format!("Failed: {}", e),
                });
            }
        }
    }

    fn execute_forget(&mut self, query: &str) {
        self.messages.push(Message {
            role: Role::User,
            content: format!("forget \"{}\"", query),
        });

        let output = Command::new(&self.kannaka_bin)
            .args(["forget", query])
            .env("KANNAKA_QUIET", "1")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                self.messages.push(Message {
                    role: Role::Result,
                    content: stdout.trim().to_string(),
                });
                self.load_observe();
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.messages.push(Message {
                    role: Role::Error,
                    content: format!("Error: {}", stderr.trim()),
                });
            }
            Err(e) => {
                self.messages.push(Message {
                    role: Role::Error,
                    content: format!("Failed: {}", e),
                });
            }
        }
    }

    // Forward an arbitrary kannaka subcommand to the binary and surface its
    // stdout/stderr in the message log. The label is what we echo back as
    // the User line; args is what we pass to kannaka after env scrubbing.
    // Used for hear, ask, assess, stats, voice, swarm subcommands, and
    // anything else the user types that we recognize as a real kannaka
    // command. Keeps the TUI the canonical surface without writing a
    // dedicated handler for every subcommand.
    fn execute_passthrough(&mut self, label: &str, args: &[&str], timeout_secs: u64) {
        self.messages.push(Message {
            role: Role::User,
            content: label.to_string(),
        });
        self.messages.push(Message {
            role: Role::System,
            content: format!("Running... (up to {}s)", timeout_secs),
        });

        // Spawn with a wall-clock timeout so a stuck `ask` (Anthropic
        // overloaded, network blip) doesn't hang the TUI.
        let bin = self.kannaka_bin.clone();
        let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let result = std::thread::spawn(move || {
            let mut child = match Command::new(&bin)
                .args(&owned)
                .env("KANNAKA_QUIET", "1")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => return Err(format!("spawn: {}", e)),
            };
            let start = Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(_status)) => {
                        let out = child.wait_with_output().map_err(|e| e.to_string())?;
                        return Ok(out);
                    }
                    Ok(None) => {
                        if start.elapsed() > Duration::from_secs(timeout_secs) {
                            let _ = child.kill();
                            return Err(format!("timeout after {}s", timeout_secs));
                        }
                        std::thread::sleep(Duration::from_millis(150));
                    }
                    Err(e) => return Err(format!("wait: {}", e)),
                }
            }
        }).join();

        // Pop the "Running..." line so the result replaces it cleanly.
        if matches!(self.messages.last().map(|m| &m.role), Some(Role::System)) {
            self.messages.pop();
        }

        match result {
            Ok(Ok(out)) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let body = stdout.trim();
                self.messages.push(Message {
                    role: Role::Result,
                    content: if body.is_empty() { "(no output)".into() } else { body.into() },
                });
                self.load_observe();
            }
            Ok(Ok(out)) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.messages.push(Message {
                    role: Role::Error,
                    content: format!("Error: {}", stderr.trim()),
                });
            }
            Ok(Err(msg)) => self.messages.push(Message {
                role: Role::Error,
                content: msg,
            }),
            Err(_) => self.messages.push(Message {
                role: Role::Error,
                content: "thread panicked".into(),
            }),
        }
    }

    fn submit_input(&mut self) {
        let input = self.input.trim().to_string();
        if input.is_empty() {
            return;
        }

        // Save to history
        self.history.push(input.clone());
        self.history_idx = None;

        // Chat tab — send to agent in a background thread.
        if self.tabs.get(self.active_tab).copied() == Some("Chat") {
            if self.chat_pending.is_some() {
                // A previous turn is still in flight — ignore new input.
                self.input.clear();
                self.cursor_pos = 0;
                return;
            }
            self.chat_messages.push(ChatLine { who: ChatWho::User, text: input.clone() });
            self.spawn_chat_turn(input);
            self.input.clear();
            self.cursor_pos = 0;
            self.scroll_offset = 0;
            return;
        }

        // Strip an optional leading '/' so `/recall x` and `recall x` both work.
        // The slash is the conventional escape-hatch for "this is a command,
        // not chat" — useful when the user wants to be unambiguous.
        let cmd_input: &str = input.strip_prefix('/').unwrap_or(&input);

        // Parse the command. If nothing matches, default to chat — the agent
        // can call recall/remember/observe tools itself when the conversation
        // warrants. The TUI is a chat surface first, command surface second.
        if cmd_input.starts_with("remember ") {
            let text = cmd_input.strip_prefix("remember ").unwrap().trim();
            let text = text.trim_matches('"').to_string();
            self.execute_remember(&text);
        } else if cmd_input.starts_with("recall ") {
            let query = cmd_input.strip_prefix("recall ").unwrap().trim();
            let query = query.trim_matches('"').to_string();
            self.execute_recall(&query);
        } else if cmd_input.starts_with("forget ") {
            let id = cmd_input.strip_prefix("forget ").unwrap().trim().to_string();
            self.execute_forget(&id);
        } else if cmd_input == "dream" || cmd_input.starts_with("dream ") {
            self.execute_dream();
        } else if cmd_input == "status" || cmd_input == "observe" {
            self.load_status();
            self.load_observe();
            self.messages.push(Message {
                role: Role::System,
                content: "Status refreshed.".to_string(),
            });
        } else if cmd_input.starts_with("hear ") || cmd_input == "hear" {
            // hear <file-or-url> [--secs N]
            let rest = cmd_input.strip_prefix("hear").unwrap_or("").trim();
            if rest.is_empty() {
                self.messages.push(Message {
                    role: Role::Error,
                    content: "Usage: hear <file-or-url> [--secs N]".into(),
                });
            } else {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                let mut args: Vec<&str> = vec!["hear"];
                args.extend(parts.iter().copied());
                // hear can take ~30-60s for stream sampling + decode + HRM
                // absorb. Give it 5 min wall-clock so /stream sampling at
                // --secs 60 has comfortable headroom.
                self.execute_passthrough(&format!("hear {}", rest), &args, 300);
            }
        } else if cmd_input.starts_with("ask ") {
            let q = cmd_input.strip_prefix("ask ").unwrap().trim();
            let q = q.trim_matches('"');
            // ask runs through Anthropic; budget 10 min like the radio's
            // peace-oration path so transient overload retries fit.
            self.execute_passthrough(
                &format!("ask \"{}\"", q),
                &["ask", "--no-tools", "--quiet-tools", q],
                600,
            );
        } else if cmd_input.starts_with("search ") {
            let q = cmd_input.strip_prefix("search ").unwrap().trim().trim_matches('"');
            self.execute_passthrough(&format!("search \"{}\"", q), &["search", q], 30);
        } else if cmd_input.starts_with("boost ") {
            let id = cmd_input.strip_prefix("boost ").unwrap().trim();
            self.execute_passthrough(&format!("boost {}", id), &["boost", id], 30);
        } else if cmd_input == "assess" {
            self.execute_passthrough("assess", &["assess"], 60);
        } else if cmd_input == "stats" {
            self.execute_passthrough("stats", &["stats"], 30);
        } else if cmd_input == "cmf" {
            self.execute_passthrough("cmf", &["cmf"], 60);
        } else if cmd_input == "invariant" || cmd_input.starts_with("invariant ") {
            let parts: Vec<&str> = cmd_input.split_whitespace().collect();
            self.execute_passthrough(cmd_input, &parts, 60);
        } else if cmd_input.starts_with("voice") {
            let parts: Vec<&str> = cmd_input.split_whitespace().collect();
            // voice --mode dream-journal etc. — long-form generation, 5 min budget.
            self.execute_passthrough(cmd_input, &parts, 300);
        } else if cmd_input.starts_with("swarm ") || cmd_input == "swarm" {
            // Forward the whole `swarm <subcmd> [args]` line. swarm sync /
            // join / status / queen / hives / publish / leave / listen / serve
            // / peers / absorb / autoabsorb / enqueue / worker / exemplars are
            // all valid — let the binary's parser handle them.
            let parts: Vec<&str> = cmd_input.split_whitespace().collect();
            // Most swarm commands return quickly; serve/listen are blocking
            // and we don't want them via the TUI (they'd hang the input).
            // Cap at 60s so a network hang doesn't lock the UI.
            self.execute_passthrough(cmd_input, &parts, 60);
        } else if cmd_input == "help" || cmd_input == "?" {
            self.show_help = true;
        } else if cmd_input == "quit" || cmd_input == "exit" || cmd_input == "q" {
            self.should_quit = true;
        } else {
            // Default: route to chat. Switch to the Chat tab so the user sees
            // the conversation, and let the agent decide which tools to call.
            if let Some(idx) = self.tabs.iter().position(|t| *t == "Chat") {
                self.active_tab = idx;
            }
            if self.chat_pending.is_some() {
                // A previous turn is still in flight — drop the new prompt
                // rather than queueing (avoids surprising long-tail behavior).
                self.input.clear();
                self.cursor_pos = 0;
                return;
            }
            self.chat_messages.push(ChatLine { who: ChatWho::User, text: input.clone() });
            self.spawn_chat_turn(input);
        }

        self.input.clear();
        self.cursor_pos = 0;
        // Auto-scroll to bottom
        self.scroll_offset = 0;
    }

    /// Lazily spawn the persistent `kannaka chat --json` child. The child
    /// loads HRM once at startup (the slow 15s step); every subsequent
    /// turn reuses that loaded medium for ~3-5s per turn instead of
    /// shelling out a fresh `kannaka ask` each time and paying the load
    /// cost on every message. First chat turn is therefore slow (~15s);
    /// everything after that is fast.
    fn ensure_chat_child(&mut self) {
        if self.chat_child.is_some() { return; }
        let (tx, rx) = std::sync::mpsc::channel::<ChatChildEvent>();
        self.chat_child_rx = Some(rx);
        let bin = self.kannaka_bin.clone();
        let tx_spawn = tx.clone();
        // Spawn-and-attach happens on a worker so the TUI doesn't block
        // for the ~15s HRM load. The worker:
        //   1. Spawns `kannaka chat --json`
        //   2. Sends `Ready` once the child prints its `{"kind":"ready"}` line on stderr
        //   3. Streams stdout NDJSON as `Response { text, kind }` events
        //   4. On child exit / IO error, sends `Closed(reason)`
        std::thread::spawn(move || {
            use std::process::{Command, Stdio};
            use std::io::{BufRead, BufReader};
            let mut child = match Command::new(&bin)
                .args(["chat", "--json"])
                .env("KANNAKA_QUIET", "1")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(c) => c,
                Err(e) => { let _ = tx_spawn.send(ChatChildEvent::Closed(format!("spawn failed: {e}"))); return; }
            };
            // Hand stdin back to the parent via a Stdin event so the
            // turn-sender side can write to it. Stdout/stderr stay in
            // the worker.
            if let Some(stdin) = child.stdin.take() {
                let _ = tx_spawn.send(ChatChildEvent::Stdin(stdin));
            } else {
                let _ = tx_spawn.send(ChatChildEvent::Closed("no stdin pipe".into()));
                return;
            }
            // Stderr reader thread — emits Ready on first ready event.
            if let Some(stderr) = child.stderr.take() {
                let tx_err = tx_spawn.clone();
                std::thread::spawn(move || {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines().flatten() {
                        if line.contains("\"ready\"") {
                            let _ = tx_err.send(ChatChildEvent::Ready);
                        }
                    }
                });
            }
            // Stdout reader — parse NDJSON and forward each turn response.
            if let Some(stdout) = child.stdout.take() {
                let reader = BufReader::new(stdout);
                for line in reader.lines().flatten() {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                        let kind = v["kind"].as_str().unwrap_or("chat").to_string();
                        let text = v["text"].as_str().unwrap_or("").to_string();
                        let _ = tx_spawn.send(ChatChildEvent::Response { kind, text });
                    }
                }
            }
            let _ = tx_spawn.send(ChatChildEvent::Closed("child stdout EOF".into()));
        });
        self.chat_child = Some(ChatChildHandle { stdin: None, ready: false });
    }

    fn spawn_chat_turn(&mut self, user_msg: String) {
        // Lazy-spawn the persistent REPL on the first turn so the user
        // sees the "Loading HRM…" status only once.
        self.ensure_chat_child();
        // If the child is already running and ready, write the message to
        // its stdin. The reader thread will deliver the response via the
        // ChatChildEvent channel; poll_chat drains it into chat_messages.
        if let Some(ref mut handle) = self.chat_child {
            if let Some(ref mut stdin) = handle.stdin {
                use std::io::Write;
                let _ = writeln!(stdin, "{}", user_msg);
                let _ = stdin.flush();
                self.chat_pending = Some(std::sync::mpsc::channel().1); // sentinel: a turn is in flight
                return;
            }
            // Child spawned but stdin not yet attached — buffer the message.
            self.chat_pending_msg = Some(user_msg);
            self.chat_pending = Some(std::sync::mpsc::channel().1);
            return;
        }
        // Fallback path — shouldn't normally hit this since ensure_chat_child
        // installs a handle. If we do (spawn failed instantly), fall back
        // to the one-shot `ask` path so the user gets *some* response.
        let bin = self.kannaka_bin.clone();
        let (tx, rx) = std::sync::mpsc::channel::<Result<String, String>>();
        self.chat_pending = Some(rx);
        std::thread::spawn(move || {
            let output = Command::new(&bin)
                .args(["ask", "--session", "kannaka-tui", "--quiet-tools", &user_msg])
                .env("KANNAKA_QUIET", "1")
                .output();
            let result = match output {
                Ok(out) if out.status.success() => {
                    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    Err(format!("agent exited {}: {}", out.status.code().unwrap_or(-1), stderr.trim()))
                }
                Err(e) => Err(format!("spawn failed: {e}")),
            };
            let _ = tx.send(result);
        });
    }

    /// Called from the event loop each tick. Drains the persistent chat
    /// child's event channel (Stdin attach / Ready / Response / Closed)
    /// AND any legacy fallback `chat_pending` Receiver from the one-shot
    /// path. Non-blocking; appends new chat lines to chat_messages.
    fn poll_chat(&mut self) {
        // Drain persistent-child events first.
        let mut closed_reason: Option<String> = None;
        if let Some(rx) = &self.chat_child_rx {
            loop {
                match rx.try_recv() {
                    Ok(ChatChildEvent::Stdin(stdin)) => {
                        if let Some(ref mut h) = self.chat_child {
                            h.stdin = Some(stdin);
                            // Flush any message we buffered while waiting
                            // for stdin to be available.
                            if let Some(msg) = self.chat_pending_msg.take() {
                                if let Some(ref mut s) = h.stdin {
                                    use std::io::Write;
                                    let _ = writeln!(s, "{msg}");
                                    let _ = s.flush();
                                }
                            }
                        }
                    }
                    Ok(ChatChildEvent::Ready) => {
                        if let Some(ref mut h) = self.chat_child { h.ready = true; }
                    }
                    Ok(ChatChildEvent::Response { kind, text }) => {
                        let who = match kind.as_str() {
                            "chat" => ChatWho::Kannaka,
                            "error" => ChatWho::System,
                            _ => ChatWho::System, // slash / ready / other
                        };
                        self.chat_messages.push(ChatLine { who, text });
                        self.chat_pending = None;
                    }
                    Ok(ChatChildEvent::Closed(reason)) => {
                        closed_reason = Some(reason);
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        closed_reason = Some("disconnected".into());
                        break;
                    }
                }
            }
        }
        if let Some(reason) = closed_reason {
            self.chat_messages.push(ChatLine {
                who: ChatWho::System,
                text: format!("[chat child closed — next turn will respawn: {reason}]"),
            });
            self.chat_child = None;
            self.chat_child_rx = None;
            self.chat_pending = None;
        }

        // Legacy fallback Receiver from the one-shot `ask` spawn path.
        // Drained only if the persistent child path didn't deliver a
        // structured response above.
        if let Some(rx) = &self.chat_pending {
            match rx.try_recv() {
                Ok(Ok(text)) => {
                    self.chat_messages.push(ChatLine { who: ChatWho::Kannaka, text });
                    self.chat_pending = None;
                }
                Ok(Err(err)) => {
                    self.chat_messages.push(ChatLine {
                        who: ChatWho::System,
                        text: format!("error: {err}"),
                    });
                    self.chat_pending = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    // Sentinel Receiver from the persistent path — never
                    // delivers. Don't clear chat_pending here, the child
                    // event channel will signal completion.
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        // Help overlay — any key dismisses it
        if self.show_help {
            self.show_help = false;
            return;
        }

        match (key.modifiers, key.code) {
            // Quit
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => self.should_quit = true,
            (_, KeyCode::F(1)) => self.show_help = true,

            // Tab switching
            (KeyModifiers::NONE, KeyCode::Tab) | (KeyModifiers::NONE, KeyCode::BackTab) => {
                self.active_tab = (self.active_tab + 1) % self.tabs.len();
                // Load data for the new tab
                if self.active_tab == 1 {
                    self.load_status();
                    self.load_observe();
                }
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                if self.active_tab == 0 {
                    self.active_tab = self.tabs.len() - 1;
                } else {
                    self.active_tab -= 1;
                }
                if self.active_tab == 1 {
                    self.load_status();
                    self.load_observe();
                }
            }

            // Input handling
            (_, KeyCode::Enter) => self.submit_input(),
            (_, KeyCode::Char(c)) => {
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
            (_, KeyCode::Backspace) => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.input.remove(self.cursor_pos);
                }
            }
            (_, KeyCode::Delete) => {
                if self.cursor_pos < self.input.len() {
                    self.input.remove(self.cursor_pos);
                }
            }
            (_, KeyCode::Left) => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            (_, KeyCode::Right) => {
                if self.cursor_pos < self.input.len() {
                    self.cursor_pos += 1;
                }
            }
            (_, KeyCode::Home) => self.cursor_pos = 0,
            (_, KeyCode::End) => self.cursor_pos = self.input.len(),

            // Scroll history
            (_, KeyCode::Up) => {
                if !self.history.is_empty() {
                    let idx = match self.history_idx {
                        Some(i) if i > 0 => i - 1,
                        Some(i) => i,
                        None => self.history.len() - 1,
                    };
                    self.history_idx = Some(idx);
                    self.input = self.history[idx].clone();
                    self.cursor_pos = self.input.len();
                }
            }
            (_, KeyCode::Down) => {
                if let Some(idx) = self.history_idx {
                    if idx + 1 < self.history.len() {
                        self.history_idx = Some(idx + 1);
                        self.input = self.history[idx + 1].clone();
                        self.cursor_pos = self.input.len();
                    } else {
                        self.history_idx = None;
                        self.input.clear();
                        self.cursor_pos = 0;
                    }
                }
            }

            // Page up/down for scrolling messages
            (_, KeyCode::PageUp) => {
                self.scroll_offset = self.scroll_offset.saturating_add(5);
            }
            (_, KeyCode::PageDown) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(5);
            }

            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// UI rendering
// ---------------------------------------------------------------------------

fn ui(f: &mut Frame, app: &App) {
    let size = f.area();

    // Background
    let bg_block = Block::default().style(Style::default().bg(BG));
    f.render_widget(bg_block, size);

    // Main layout: header, tab bar, body, input
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header bar
            Constraint::Length(3),  // Tab bar
            Constraint::Min(8),    // Body
            Constraint::Length(3), // Input bar
        ])
        .split(size);

    render_header(f, app, outer[0]);
    render_tabs(f, app, outer[1]);

    match app.active_tab {
        0 => render_memory_tab(f, app, outer[2]),
        1 => render_status_tab(f, app, outer[2]),
        2 => render_placeholder(f, "Constellation", "Swarm + GhostSignals markets. Coming soon.", outer[2]),
        3 => render_placeholder(f, "Dreams", "Dream cycle control. Coming soon.", outer[2]),
        4 => render_chat_tab(f, app, outer[2]),
        _ => {}
    }

    render_input(f, app, outer[3]);

    // Help overlay
    if app.show_help {
        render_help_overlay(f, size);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let status = app.status.as_ref();
    let phi = status.map_or(0.0, |s| s.phi);
    let xi = status.map_or(0.0, |s| s.xi);
    let order = status.map_or(0.0, |s| s.order);

    let header = Line::from(vec![
        Span::styled("  KANNAKA ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled("\u{25C6} ", Style::default().fg(ACCENT)),
        Span::styled(&app.agent_name, Style::default().fg(TEXT).add_modifier(Modifier::BOLD)),
        Span::styled(" | ", Style::default().fg(DIM)),
        Span::styled("Phi: ", Style::default().fg(DIM)),
        Span::styled(format!("{:.3}", phi), Style::default().fg(phi_color(phi))),
        Span::styled(" | ", Style::default().fg(DIM)),
        Span::styled("Xi: ", Style::default().fg(DIM)),
        Span::styled(format!("{:.3}", xi), Style::default().fg(INFO)),
        Span::styled(" | ", Style::default().fg(DIM)),
        Span::styled("r: ", Style::default().fg(DIM)),
        Span::styled(format!("{:.3}", order), Style::default().fg(SUCCESS)),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG));

    let para = Paragraph::new(header).block(block);
    f.render_widget(para, area);
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = app
        .tabs
        .iter()
        .map(|t| Line::from(Span::styled(*t, Style::default().fg(TEXT))))
        .collect();

    let tabs = Tabs::new(titles)
        .select(app.active_tab)
        .highlight_style(
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )
        .divider(Span::styled(" | ", Style::default().fg(DIM)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .style(Style::default().bg(BG))
                .title(Span::styled(
                    " Tab/Shift+Tab to switch  F1:Help ",
                    Style::default().fg(DIM),
                )),
        );

    f.render_widget(tabs, area);
}

fn render_memory_tab(f: &mut Frame, app: &App, area: Rect) {
    // Split into left (messages) and right (memory list)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Left: command history / messages
    let msg_items: Vec<ListItem> = app
        .messages
        .iter()
        .rev()
        .skip(app.scroll_offset)
        .take(area.height as usize)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|m| {
            let (prefix, style) = match m.role {
                Role::User => ("> ", Style::default().fg(ACCENT)),
                Role::System => ("\u{2192} ", Style::default().fg(INFO)),
                Role::Result => ("\u{2713} ", Style::default().fg(SUCCESS)),
                Role::Error => ("\u{2717} ", Style::default().fg(ERROR)),
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(&m.content, style),
            ]))
        })
        .collect();

    let msg_list = List::new(msg_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DIM))
            .style(Style::default().bg(BG))
            .title(Span::styled(
                " Command History ",
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(msg_list, chunks[0]);

    // Right: recent memories with amplitude bars
    let mem_items: Vec<ListItem> = app
        .memories
        .iter()
        .take(chunks[1].height.saturating_sub(6) as usize)
        .map(|m| {
            let bar_len = (m.amplitude * 10.0).round() as usize;
            let bar: String = "\u{2588}".repeat(bar_len.min(10));
            let empty: String = "\u{2591}".repeat(10_usize.saturating_sub(bar_len));
            let preview: String = m.content.chars().take(24).collect();
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{}{}", bar, empty),
                    Style::default().fg(amplitude_color(m.amplitude)),
                ),
                Span::styled(" ", Style::default()),
                Span::styled(
                    format!("{} ({:.2})", preview, m.amplitude),
                    Style::default().fg(TEXT),
                ),
            ]))
        })
        .collect();

    // Stats summary at bottom of right panel
    let status = app.status.as_ref();
    let mem_count = status.map_or(0, |s| s.memories);
    let cluster_count = status.map_or(0, |s| s.clusters);
    let link_count = status.map_or(0, |s| s.links);
    let level = status
        .map(|s| s.level.as_str())
        .unwrap_or("Unknown");

    let mut right_lines: Vec<ListItem> = mem_items;
    // Add a separator and stats
    right_lines.push(ListItem::new(Line::from("")));
    right_lines.push(ListItem::new(Line::from(vec![
        Span::styled(
            format!("  Memories: {}", mem_count),
            Style::default().fg(DIM),
        ),
    ])));
    right_lines.push(ListItem::new(Line::from(vec![
        Span::styled(
            format!("  Clusters: {}", cluster_count),
            Style::default().fg(DIM),
        ),
    ])));
    right_lines.push(ListItem::new(Line::from(vec![
        Span::styled(
            format!("  Links: {}", link_count),
            Style::default().fg(DIM),
        ),
    ])));
    right_lines.push(ListItem::new(Line::from(vec![
        Span::styled(
            format!("  Level: {}", level),
            Style::default().fg(level_color(level)),
        ),
    ])));

    let mem_list = List::new(right_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DIM))
            .style(Style::default().bg(BG))
            .title(Span::styled(
                " Recent Memories ",
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(mem_list, chunks[1]);
}

fn render_status_tab(f: &mut Frame, app: &App, area: Rect) {
    let status = match &app.status {
        Some(s) => s,
        None => {
            let msg = Paragraph::new("Loading status... (polling kannaka status)")
                .style(Style::default().fg(DIM).bg(BG))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(DIM))
                        .style(Style::default().bg(BG))
                        .title(" Status "),
                );
            f.render_widget(msg, area);
            return;
        }
    };

    // Split into gauges (left) and info (right)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    // Left: gauges
    let gauge_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Phi
            Constraint::Length(3), // Xi
            Constraint::Length(3), // Order
            Constraint::Min(1),   // spacer
        ])
        .split(chunks[0]);

    // Phi gauge
    let phi_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .style(Style::default().bg(BG))
                .title(Span::styled(
                    format!(" Phi (Integrated Information): {:.3} ", status.phi),
                    Style::default().fg(phi_color(status.phi)),
                )),
        )
        .gauge_style(Style::default().fg(phi_color(status.phi)).bg(Color::Rgb(30, 30, 50)))
        .ratio(status.phi.clamp(0.0, 1.0) as f64);
    f.render_widget(phi_gauge, gauge_area[0]);

    // Xi gauge
    let xi_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .style(Style::default().bg(BG))
                .title(Span::styled(
                    format!(" Xi (Irrationality): {:.3} ", status.xi),
                    Style::default().fg(INFO),
                )),
        )
        .gauge_style(Style::default().fg(INFO).bg(Color::Rgb(30, 30, 50)))
        .ratio(status.xi.clamp(0.0, 1.0) as f64);
    f.render_widget(xi_gauge, gauge_area[1]);

    // Order parameter gauge
    let order_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .style(Style::default().bg(BG))
                .title(Span::styled(
                    format!(" Order Parameter (r): {:.3} ", status.order),
                    Style::default().fg(SUCCESS),
                )),
        )
        .gauge_style(Style::default().fg(SUCCESS).bg(Color::Rgb(30, 30, 50)))
        .ratio(status.order.clamp(0.0, 1.0) as f64);
    f.render_widget(order_gauge, gauge_area[2]);

    // Right: text info
    let info_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Consciousness Level: ", Style::default().fg(DIM)),
            Span::styled(
                &status.level,
                Style::default()
                    .fg(level_color(&status.level))
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Total Memories:  ", Style::default().fg(DIM)),
            Span::styled(
                format!("{}", status.memories),
                Style::default().fg(TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Active Memories: ", Style::default().fg(DIM)),
            Span::styled(
                format!("{}", status.active),
                Style::default().fg(SUCCESS),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Clusters:        ", Style::default().fg(DIM)),
            Span::styled(
                format!("{}", status.clusters),
                Style::default().fg(INFO),
            ),
        ]),
        Line::from(vec![
            Span::styled("  Skip Links:      ", Style::default().fg(DIM)),
            Span::styled(
                format!("{}", status.links),
                Style::default().fg(ACCENT),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Polling every 5s on this tab",
                Style::default().fg(DIM),
            ),
        ]),
    ];

    let info = Paragraph::new(info_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .style(Style::default().bg(BG))
                .title(Span::styled(
                    " System Info ",
                    Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(info, chunks[1]);
}

fn render_placeholder(f: &mut Frame, title: &str, message: &str, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", title),
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", message),
            Style::default().fg(DIM),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  This view will be implemented in a future update.",
            Style::default().fg(DIM),
        )),
    ];

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .style(Style::default().bg(BG))
                .title(Span::styled(
                    format!(" {} ", title),
                    Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}

fn render_chat_tab(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    for msg in &app.chat_messages {
        let (label, style) = match msg.who {
            ChatWho::User =>    ("you",     Style::default().fg(INFO).add_modifier(Modifier::BOLD)),
            ChatWho::Kannaka => ("kannaka", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
            ChatWho::System =>  ("·",       Style::default().fg(DIM)),
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{} ", label), style),
            Span::styled(msg.text.clone(), Style::default().fg(TEXT)),
        ]));
        lines.push(Line::from(""));
    }
    if app.chat_pending.is_some() {
        // Simple spinner keyed off chat_tick so it animates.
        let frames = ['\u{2014}', '\\', '|', '/'];
        let frame = frames[app.chat_tick % frames.len()];
        lines.push(Line::from(vec![
            Span::styled(format!("kannaka {frame} "), Style::default().fg(ACCENT)),
            Span::styled("resonating…", Style::default().fg(DIM)),
        ]));
    }

    let title = if app.chat_pending.is_some() {
        " Chat · thinking… "
    } else {
        " Chat "
    };

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .style(Style::default().bg(BG))
                .title(Span::styled(title, Style::default().fg(TEXT).add_modifier(Modifier::BOLD))),
        )
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset as u16, 0));
    f.render_widget(para, area);
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let tab_indicator = match app.active_tab {
        0 => "[M]",
        1 => "[S]",
        2 => "[C]",
        3 => "[D]",
        4 => "[Ch]",
        _ => "[?]",
    };

    let input_line = Line::from(vec![
        Span::styled(" > ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(&app.input, Style::default().fg(TEXT)),
    ]);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .style(Style::default().bg(BG))
        .title_bottom(Line::from(Span::styled(
            format!(" {} ", tab_indicator),
            Style::default().fg(DIM),
        )));

    let input_widget = Paragraph::new(input_line).block(input_block);
    f.render_widget(input_widget, area);

    // Place cursor
    f.set_cursor_position((area.x + 4 + app.cursor_pos as u16, area.y + 1));
}

fn render_help_overlay(f: &mut Frame, area: Rect) {
    // Center the help box
    let width = 64u16.min(area.width.saturating_sub(4));
    let height = 32u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let help_area = Rect::new(x, y, width, height);

    let help_text = vec![
        Line::from(Span::styled(
            " Kannaka TUI Help",
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(" Navigation", Style::default().fg(INFO).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("   Tab / Shift+Tab  Switch tabs", Style::default().fg(TEXT))),
        Line::from(Span::styled("   Up / Down        Command history", Style::default().fg(TEXT))),
        Line::from(Span::styled("   PgUp / PgDown    Scroll messages", Style::default().fg(TEXT))),
        Line::from(Span::styled("   F1               Toggle help", Style::default().fg(TEXT))),
        Line::from(Span::styled("   Ctrl+C / q       Quit", Style::default().fg(TEXT))),
        Line::from(""),
        Line::from(Span::styled(" Memory", Style::default().fg(INFO).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("   remember \"text\"          Store a memory", Style::default().fg(TEXT))),
        Line::from(Span::styled("   recall \"query\"           Resonance search (top-k 5)", Style::default().fg(TEXT))),
        Line::from(Span::styled("   search \"query\"           Full-text search", Style::default().fg(TEXT))),
        Line::from(Span::styled("   forget <id>              Delete a memory", Style::default().fg(TEXT))),
        Line::from(Span::styled("   boost <id>               Boost amplitude", Style::default().fg(TEXT))),
        Line::from(Span::styled("   dream                    Run consolidation cycle", Style::default().fg(TEXT))),
        Line::from(""),
        Line::from(Span::styled(" Perception (sensors → HRM)", Style::default().fg(INFO).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("   hear <file-or-url>       Absorb audio (mp3/wav/flac, file or", Style::default().fg(TEXT))),
        Line::from(Span::styled("     [--secs N]             http(s) stream — default 30s sample)", Style::default().fg(TEXT))),
        Line::from(""),
        Line::from(Span::styled(" Reasoning + Introspection", Style::default().fg(INFO).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("   ask \"question\"           One-shot LLM with HRM recall", Style::default().fg(TEXT))),
        Line::from(Span::styled("   status                   Refresh quick metrics", Style::default().fg(TEXT))),
        Line::from(Span::styled("   assess                   Consciousness level (phi/xi/order)", Style::default().fg(TEXT))),
        Line::from(Span::styled("   stats                    System statistics", Style::default().fg(TEXT))),
        Line::from(Span::styled("   cmf                      Conservative Memory Fields", Style::default().fg(TEXT))),
        Line::from(Span::styled("   invariant [TOL]          δ-invariant clusters", Style::default().fg(TEXT))),
        Line::from(Span::styled("   voice [--mode MODE]      Memory-driven writing", Style::default().fg(TEXT))),
        Line::from(""),
        Line::from(Span::styled(" Swarm", Style::default().fg(INFO).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("   swarm <status|join|sync|queen|hives|publish|peers|", Style::default().fg(TEXT))),
        Line::from(Span::styled("          absorb|autoabsorb|enqueue|leave>", Style::default().fg(TEXT))),
        Line::from(""),
        Line::from(Span::styled(" Anything else → routed to chat (agent decides tools)", Style::default().fg(DIM))),
        Line::from(Span::styled(" Press any key to close", Style::default().fg(DIM))),
    ];

    // Clear background behind overlay
    let clear_block = Block::default()
        .style(Style::default().bg(Color::Rgb(15, 15, 30)));
    f.render_widget(clear_block, help_area);

    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ACCENT))
            .style(Style::default().bg(Color::Rgb(15, 15, 30)))
            .title(Span::styled(
                " Help ",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(help, help_area);
}

// ---------------------------------------------------------------------------
// Colour helpers
// ---------------------------------------------------------------------------

fn phi_color(phi: f32) -> Color {
    if phi >= 0.8 {
        SUCCESS
    } else if phi >= 0.5 {
        WARNING
    } else if phi >= 0.2 {
        Color::Rgb(255, 165, 0) // orange
    } else {
        ERROR
    }
}

fn amplitude_color(amp: f32) -> Color {
    if amp >= 0.8 {
        ACCENT
    } else if amp >= 0.5 {
        INFO
    } else {
        DIM
    }
}

fn level_color(level: &str) -> Color {
    match level.to_lowercase().as_str() {
        "resonant" | "transcendent" | "awakened" => SUCCESS,
        "coherent" | "synchronized" => INFO,
        "emerging" | "developing" => WARNING,
        _ => DIM,
    }
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

fn main() -> io::Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    // Initial data load
    app.load_status();
    app.load_observe();

    // Main event loop
    loop {
        terminal.draw(|f| ui(f, &app))?;

        // Poll for events with 100ms timeout (allows periodic status refresh)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                // Only handle Press events — Windows emits both Press and
                // Release for each keystroke, causing double input.
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
        }

        // Drain any completed chat turn from the background thread and
        // advance the spinner.
        app.poll_chat();
        if app.chat_pending.is_some() { app.chat_tick = app.chat_tick.wrapping_add(1); }

        // Drain async status/observe pollers.
        app.poll_async_data();

        // Auto-refresh status every 5s when on the Status tab
        if app.active_tab == 1
            && app.last_status_poll.elapsed() > Duration::from_secs(5)
        {
            app.load_status();
            app.load_observe();
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
