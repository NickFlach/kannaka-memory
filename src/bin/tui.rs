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
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Circle as CanvasCircle, Line as CanvasLine},
        Block, Borders, Gauge, List, ListItem, Paragraph, Tabs, Wrap,
    },
    Frame, Terminal,
};
use std::collections::VecDeque;
use std::io;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc;
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
    // Live Bus tab — long-running `kannaka swarm tail` child whose
    // stdout we read as NDJSON. The reader thread pushes BusLine
    // entries through `bus_rx`; we drain them each tick and cap
    // `bus_lines` at BUS_BACKLOG_CAP entries.
    bus_lines: VecDeque<BusLine>,
    bus_rx: Option<mpsc::Receiver<BusLine>>,
    bus_status: BusStatus,
    bus_child: Option<std::process::Child>,
    // Constellation tab state — keyed by agent_id, populated by the
    // same bus reader thread that feeds bus_lines.
    agents: std::collections::HashMap<String, AgentSnapshot>,
    agent_rx: Option<mpsc::Receiver<AgentSnapshot>>,
    // Dreams tab — rolling history of KANNAKA.dreams events harvested
    // from the same bus stream, plus the local trigger state machine.
    dream_history: VecDeque<DreamEvent>,
    dream_rx: Option<mpsc::Receiver<DreamEvent>>,
    dream_run: DreamRunState,
    dream_trigger_rx: Option<mpsc::Receiver<Result<String, String>>>,
}

const BUS_BACKLOG_CAP: usize = 500;
const DREAM_HISTORY_CAP: usize = 30;
/// Agents not heard from in this window get rendered as ghost outlines
/// instead of solid markers on the Constellation tab.
const AGENT_FRESH_WINDOW: Duration = Duration::from_secs(120);

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

/// One row in the live Bus tab — produced by parsing NDJSON lines emitted
/// by the `kannaka swarm tail` child process.
#[derive(Clone)]
struct BusLine {
    ts_ms: i64,
    subject: String,
    summary: String,
}

#[derive(Clone, PartialEq, Eq)]
enum BusStatus { Off, Connecting, Streaming, Failed }

/// One KANNAKA.dreams report observed on the bus. The Dreams tab keeps
/// a rolling backlog of these so users can see what consolidation
/// activity has been happening across the constellation.
#[derive(Clone)]
struct DreamEvent {
    ts_ms: i64,
    agent_id: String,
    cycles: u64,
    strengthened: u64,
    pruned: u64,
    new_connections: u64,
    hallucinations: u64,
    consciousness_before: f32,
    consciousness_after: f32,
    emerged: bool,
}

/// Status of the most recent locally-triggered dream cycle. The TUI
/// dispatches `kannaka dream` in a worker thread so the event loop
/// stays responsive during the 30+ second consolidation pass.
#[derive(Clone)]
enum DreamRunState {
    Idle,
    Running { mode: String, started: Instant },
    Done { mode: String, took: Duration, summary: String },
    Failed { mode: String, error: String },
}

/// Latest snapshot for one agent — harvested from `QUEEN.phase.<agent_id>`
/// payloads as they stream through the bus. Used by the Constellation tab
/// to plot agents on the unit circle and fade out anyone who's gone quiet.
#[derive(Clone)]
struct AgentSnapshot {
    agent_id: String,
    theta: f32,
    phi: f32,
    coherence: f32,
    order_parameter: f32,
    handedness: String,
    memory_count: u64,
    last_seen: Instant,
}

impl App {
    fn new() -> Self {
        // Find the kannaka binary — prefer the release build next to us
        let kannaka_bin = Self::find_kannaka_binary();
        let agent_name = Self::load_agent_name();

        Self {
            // Chat is the primary surface. The other tabs are still
            // reachable via Tab/Shift+Tab but the user lands in chat.
            // Bus sits between Status and Constellation as the live
            // constellation pulse view.
            active_tab: 5,
            tabs: vec!["Memory", "Status", "Bus", "Constellation", "Dreams", "Chat"],
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
            bus_lines: VecDeque::new(),
            bus_rx: None,
            bus_status: BusStatus::Off,
            bus_child: None,
            agents: std::collections::HashMap::new(),
            agent_rx: None,
            dream_history: VecDeque::new(),
            dream_rx: None,
            dream_run: DreamRunState::Idle,
            dream_trigger_rx: None,
        }
    }

    /// Spawn `kannaka swarm tail` and stream its NDJSON stdout into the
    /// Bus tab. Lazy — only kicked off the first time the user opens the
    /// Bus tab so the TUI doesn't open a NATS connection on launch for
    /// users who don't care.
    fn start_bus(&mut self) {
        if self.bus_child.is_some() { return; }
        self.bus_status = BusStatus::Connecting;
        let (bus_tx, bus_rx) = mpsc::channel::<BusLine>();
        let (agent_tx, agent_rx) = mpsc::channel::<AgentSnapshot>();
        let (dream_tx, dream_rx) = mpsc::channel::<DreamEvent>();
        self.bus_rx = Some(bus_rx);
        self.agent_rx = Some(agent_rx);
        self.dream_rx = Some(dream_rx);

        let mut cmd = Command::new(&self.kannaka_bin);
        cmd.args(["swarm", "tail"])
            .env("KANNAKA_QUIET", "1")
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                self.bus_lines.push_back(BusLine {
                    ts_ms: chrono::Utc::now().timestamp_millis(),
                    subject: "tui.error".into(),
                    summary: format!("could not spawn 'kannaka swarm tail': {e}"),
                });
                self.bus_status = BusStatus::Failed;
                return;
            }
        };
        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => {
                self.bus_status = BusStatus::Failed;
                return;
            }
        };
        self.bus_child = Some(child);

        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }
                let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) else {
                    continue;
                };
                let ts_ms = val.get("ts").and_then(|v| v.as_i64()).unwrap_or(0);
                let subject = val.get("subject").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                let payload = val.get("payload").cloned().unwrap_or(serde_json::Value::Null);

                // Phase frames feed the Constellation tab.
                if subject.starts_with("QUEEN.phase.") {
                    if let Some(snap) = agent_snapshot_from_payload(&subject, &payload) {
                        // Send-fails are non-fatal — the main thread may have
                        // dropped agent_rx but bus_tx keeps the stream alive.
                        let _ = agent_tx.send(snap);
                    }
                }
                // Dream completion reports feed the Dreams tab.
                if subject == "KANNAKA.dreams" {
                    if let Some(ev) = dream_event_from_payload(ts_ms, &payload) {
                        let _ = dream_tx.send(ev);
                    }
                }

                let summary = summarize_payload(&subject, &payload);
                if bus_tx.send(BusLine { ts_ms, subject, summary }).is_err() {
                    break;
                }
            }
        });
    }

    /// Drain any new BusLine entries from the worker thread into the
    /// ring buffer. Capped at BUS_BACKLOG_CAP — older lines drop off
    /// the front.
    fn poll_bus(&mut self) {
        let mut got_any = false;
        if let Some(rx) = &self.bus_rx {
            loop {
                match rx.try_recv() {
                    Ok(line) => {
                        self.bus_lines.push_back(line);
                        while self.bus_lines.len() > BUS_BACKLOG_CAP {
                            self.bus_lines.pop_front();
                        }
                        got_any = true;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.bus_rx = None;
                        self.bus_status = BusStatus::Failed;
                        break;
                    }
                }
            }
        }
        if got_any && self.bus_status == BusStatus::Connecting {
            self.bus_status = BusStatus::Streaming;
        }

        // Drain per-agent snapshots into the map. Same channel discipline
        // as bus_rx; Disconnected means the reader thread died.
        if let Some(rx) = &self.agent_rx {
            loop {
                match rx.try_recv() {
                    Ok(snap) => {
                        self.agents.insert(snap.agent_id.clone(), snap);
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.agent_rx = None;
                        break;
                    }
                }
            }
        }

        // Drain dream events into the rolling history.
        if let Some(rx) = &self.dream_rx {
            loop {
                match rx.try_recv() {
                    Ok(ev) => {
                        self.dream_history.push_front(ev);
                        while self.dream_history.len() > DREAM_HISTORY_CAP {
                            self.dream_history.pop_back();
                        }
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.dream_rx = None;
                        break;
                    }
                }
            }
        }

        // Drain the local dream-trigger worker (if one is running).
        if let Some(rx) = &self.dream_trigger_rx {
            match rx.try_recv() {
                Ok(Ok(summary)) => {
                    if let DreamRunState::Running { mode, started } = &self.dream_run {
                        let took = started.elapsed();
                        self.dream_run = DreamRunState::Done {
                            mode: mode.clone(),
                            took,
                            summary,
                        };
                    }
                    self.dream_trigger_rx = None;
                    // Refresh metrics — dream just changed memory state.
                    self.load_status();
                    self.load_observe();
                }
                Ok(Err(err)) => {
                    if let DreamRunState::Running { mode, .. } = &self.dream_run {
                        self.dream_run = DreamRunState::Failed {
                            mode: mode.clone(),
                            error: err,
                        };
                    }
                    self.dream_trigger_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.dream_trigger_rx = None;
                }
            }
        }
    }

    /// Trigger a non-blocking dream cycle. Returns immediately; the
    /// `dream_trigger_rx` Receiver fires when the child exits and
    /// `poll_bus` transitions DreamRunState to Done/Failed.
    fn start_dream(&mut self, mode: &str) {
        if self.dream_trigger_rx.is_some() { return; }
        let mode = mode.to_string();
        self.dream_run = DreamRunState::Running {
            mode: mode.clone(),
            started: Instant::now(),
        };
        let (tx, rx) = mpsc::channel::<Result<String, String>>();
        self.dream_trigger_rx = Some(rx);
        let bin = self.kannaka_bin.clone();
        let mode_for_thread = mode.clone();
        std::thread::spawn(move || {
            let output = Command::new(&bin)
                .args(["dream", "--mode", &mode_for_thread])
                .env("KANNAKA_QUIET", "1")
                .output();
            let result = match output {
                Ok(out) if out.status.success() => {
                    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    Err(stderr.trim().to_string())
                }
                Err(e) => Err(format!("spawn failed: {e}")),
            };
            let _ = tx.send(result);
        });
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

    /// Kick off a dream from the command bar (`dream` or `dream lite`).
    /// Always non-blocking — the worker thread reports back via
    /// dream_trigger_rx. Was blocking pre-v0.5.8 and froze the TUI for
    /// the duration of consolidation (~30s).
    fn execute_dream(&mut self) {
        if self.dream_trigger_rx.is_some() {
            self.messages.push(Message {
                role: Role::System,
                content: "A dream is already running — wait for it to finish.".into(),
            });
            return;
        }
        self.messages.push(Message {
            role: Role::User,
            content: "dream --mode deep".to_string(),
        });
        self.messages.push(Message {
            role: Role::System,
            content: "Dream cycle started in background — Dreams tab for progress.".to_string(),
        });
        // Make sure the bus is on so the post-dream KANNAKA.dreams event
        // shows up in the history list.
        self.start_bus();
        self.start_dream("deep");
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
                        match kind.as_str() {
                            "chunk" => {
                                // Streaming token from the in-flight chat
                                // turn. Append to the trailing Kannaka line
                                // so the response builds up live in the UI.
                                let needs_new = match self.chat_messages.last() {
                                    Some(line) => !matches!(line.who, ChatWho::Kannaka),
                                    None => true,
                                };
                                if needs_new {
                                    self.chat_messages.push(ChatLine {
                                        who: ChatWho::Kannaka,
                                        text: text.clone(),
                                    });
                                } else if let Some(last) = self.chat_messages.last_mut() {
                                    last.text.push_str(&text);
                                }
                                // Don't clear chat_pending yet — the final
                                // "chat" frame is the turn-done signal.
                            }
                            "chat" => {
                                // Turn-done. If we streamed chunks, the line
                                // already has the text; just clear pending.
                                // If we didn't (e.g. Ollama fallback), push
                                // the assembled text as a new line.
                                let already_streamed = matches!(
                                    self.chat_messages.last().map(|l| &l.who),
                                    Some(ChatWho::Kannaka)
                                );
                                if !already_streamed {
                                    self.chat_messages.push(ChatLine {
                                        who: ChatWho::Kannaka,
                                        text,
                                    });
                                }
                                self.chat_pending = None;
                            }
                            "error" => {
                                self.chat_messages.push(ChatLine {
                                    who: ChatWho::System,
                                    text,
                                });
                                self.chat_pending = None;
                            }
                            _ => {
                                // slash / ready / other
                                self.chat_messages.push(ChatLine {
                                    who: ChatWho::System,
                                    text,
                                });
                                self.chat_pending = None;
                            }
                        }
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

    /// Side-effects on entering a tab — kicked off whether the user
    /// stepped forward (Tab) or backward (Shift+Tab).
    fn on_tab_enter(&mut self) {
        match self.active_tab {
            1 => {
                // Status — refresh metrics
                self.load_status();
                self.load_observe();
            }
            2 | 3 | 4 => {
                // Bus, Constellation, and Dreams all feed off the same
                // NATS stream (Dreams listens for KANNAKA.dreams events).
                self.start_bus();
            }
            _ => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        // Help overlay — any key dismisses it
        if self.show_help {
            self.show_help = false;
            return;
        }

        // Dreams tab: empty-input single-letter hotkeys trigger a dream
        // without going through the command bar. Only fire when the input
        // is empty so users can still type commands like `dream lite`.
        if self.active_tab == 4 && self.input.is_empty() {
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Char('d')) | (KeyModifiers::NONE, KeyCode::Char('D')) => {
                    self.start_dream("deep");
                    return;
                }
                (KeyModifiers::NONE, KeyCode::Char('l')) | (KeyModifiers::NONE, KeyCode::Char('L')) => {
                    self.start_dream("lite");
                    return;
                }
                _ => {}
            }
        }

        // Empty-input quit shortcuts: q and Esc. Only fire when the
        // command bar is empty so users can still type `quit` etc. as
        // a literal command. Always available regardless of active tab.
        if self.input.is_empty() {
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Char('q')) | (KeyModifiers::NONE, KeyCode::Char('Q')) => {
                    self.should_quit = true;
                    return;
                }
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    self.should_quit = true;
                    return;
                }
                _ => {}
            }
        }

        match (key.modifiers, key.code) {
            // Quit
            (KeyModifiers::CONTROL, KeyCode::Char('c')) => self.should_quit = true,
            (_, KeyCode::F(1)) => self.show_help = true,

            // Tab switching
            (KeyModifiers::NONE, KeyCode::Tab) | (KeyModifiers::NONE, KeyCode::BackTab) => {
                self.active_tab = (self.active_tab + 1) % self.tabs.len();
                self.on_tab_enter();
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                if self.active_tab == 0 {
                    self.active_tab = self.tabs.len() - 1;
                } else {
                    self.active_tab -= 1;
                }
                self.on_tab_enter();
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
        2 => render_bus_tab(f, app, outer[2]),
        3 => render_constellation_tab(f, app, outer[2]),
        4 => render_dreams_tab(f, app, outer[2]),
        5 => render_chat_tab(f, app, outer[2]),
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

/// Parse a KANNAKA.dreams payload into a structured DreamEvent.
fn dream_event_from_payload(ts_ms: i64, payload: &serde_json::Value) -> Option<DreamEvent> {
    let obj = payload.as_object()?;
    Some(DreamEvent {
        ts_ms,
        agent_id: obj.get("agent_id").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
        cycles: obj.get("cycles").and_then(|v| v.as_u64()).unwrap_or(0),
        strengthened: obj.get("memories_strengthened").and_then(|v| v.as_u64()).unwrap_or(0),
        pruned: obj.get("memories_pruned").and_then(|v| v.as_u64()).unwrap_or(0),
        new_connections: obj.get("new_connections").and_then(|v| v.as_u64()).unwrap_or(0),
        hallucinations: obj.get("hallucinations_created").and_then(|v| v.as_u64()).unwrap_or(0),
        consciousness_before: obj.get("consciousness_before").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
        consciousness_after: obj.get("consciousness_after").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
        emerged: obj.get("emerged").and_then(|v| v.as_bool()).unwrap_or(false),
    })
}

fn render_dreams_tab(f: &mut Frame, app: &App, area: Rect) {
    // Top: current state of any locally-triggered dream
    // Middle: recent dream events from across the constellation
    // Bottom: hint bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // current run state
            Constraint::Min(6),     // history list
            Constraint::Length(3),  // hint bar
        ])
        .split(area);

    // ----- Current run -----
    let (run_title, run_color, run_lines) = match &app.dream_run {
        DreamRunState::Idle => (
            " Local Dream · idle ",
            DIM,
            vec![
                Line::from(Span::styled(
                    "  Press 'd' for deep, 'l' for lite — or type `dream` in the bar.",
                    Style::default().fg(DIM),
                )),
            ],
        ),
        DreamRunState::Running { mode, started } => {
            let secs = started.elapsed().as_secs();
            (
                " Local Dream · running ",
                WARNING,
                vec![
                    Line::from(vec![
                        Span::styled("  Mode: ", Style::default().fg(DIM)),
                        Span::styled(mode.clone(), Style::default().fg(WARNING).add_modifier(Modifier::BOLD)),
                        Span::styled(format!("    elapsed: {}s", secs), Style::default().fg(DIM)),
                    ]),
                    Line::from(Span::styled(
                        "  Consolidating the medium — TUI stays responsive while this runs.",
                        Style::default().fg(DIM),
                    )),
                ],
            )
        }
        DreamRunState::Done { mode, took, summary } => (
            " Local Dream · complete ",
            SUCCESS,
            vec![
                Line::from(vec![
                    Span::styled("  Mode: ", Style::default().fg(DIM)),
                    Span::styled(mode.clone(), Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD)),
                    Span::styled(format!("    took: {:.1}s", took.as_secs_f64()), Style::default().fg(DIM)),
                ]),
                Line::from(Span::styled(
                    format!("  {}", truncate(&summary.replace('\n', " · "), 200)),
                    Style::default().fg(TEXT),
                )),
            ],
        ),
        DreamRunState::Failed { mode, error } => (
            " Local Dream · failed ",
            ERROR,
            vec![
                Line::from(vec![
                    Span::styled("  Mode: ", Style::default().fg(DIM)),
                    Span::styled(mode.clone(), Style::default().fg(ERROR).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(Span::styled(
                    format!("  {}", truncate(error, 200)),
                    Style::default().fg(ERROR),
                )),
            ],
        ),
    };
    let run_block = Paragraph::new(run_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(run_color))
                .style(Style::default().bg(BG))
                .title(Span::styled(
                    run_title,
                    Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(run_block, chunks[0]);

    // ----- History (drained from KANNAKA.dreams via the bus) -----
    let body_height = chunks[1].height.saturating_sub(3) as usize;
    let mut hist_lines: Vec<Line> = Vec::new();
    hist_lines.push(Line::from(vec![
        Span::styled(
            "  time     agent           cycles  +Φ    Δstr  Δprn  Δnew  halluc",
            Style::default().fg(DIM).add_modifier(Modifier::BOLD),
        ),
    ]));
    hist_lines.push(Line::from(""));
    for ev in app.dream_history.iter().take(body_height.max(1)) {
        let delta_phi = ev.consciousness_after - ev.consciousness_before;
        let phi_color = if delta_phi > 0.001 { SUCCESS }
            else if delta_phi < -0.001 { ERROR }
            else { DIM };
        let emerged_mark = if ev.emerged { "★ " } else { "  " };
        hist_lines.push(Line::from(vec![
            Span::styled(emerged_mark, Style::default().fg(ACCENT)),
            Span::styled(
                format!("{} ", format_bus_ts(ev.ts_ms)),
                Style::default().fg(DIM),
            ),
            Span::styled(
                format!("{:<14} ", truncate(&ev.agent_id, 14)),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{:>6}  ", ev.cycles), Style::default().fg(TEXT)),
            Span::styled(
                format!("{:>+5.3} ", delta_phi),
                Style::default().fg(phi_color),
            ),
            Span::styled(
                format!("{:>5}  {:>5}  {:>5}  {:>5}",
                    ev.strengthened, ev.pruned, ev.new_connections, ev.hallucinations),
                Style::default().fg(TEXT),
            ),
        ]));
    }
    if app.dream_history.is_empty() {
        hist_lines.push(Line::from(Span::styled(
            "  No KANNAKA.dreams events on the bus yet — once any constellation node",
            Style::default().fg(DIM),
        )));
        hist_lines.push(Line::from(Span::styled(
            "  finishes a dream cycle, the report shows up here.",
            Style::default().fg(DIM),
        )));
    }
    let history = Paragraph::new(hist_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(DIM))
                .style(Style::default().bg(BG))
                .title(Span::styled(
                    format!(" Recent Dreams · {} ", app.dream_history.len()),
                    Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(history, chunks[1]);

    // ----- Hint bar -----
    let hints = Paragraph::new(Line::from(vec![
        Span::styled(" d ", Style::default().fg(BG).bg(SUCCESS).add_modifier(Modifier::BOLD)),
        Span::styled(" deep dream  ", Style::default().fg(DIM)),
        Span::styled(" l ", Style::default().fg(BG).bg(INFO).add_modifier(Modifier::BOLD)),
        Span::styled(" lite dream  ", Style::default().fg(DIM)),
        Span::styled(" ★ ", Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)),
        Span::styled(" emergence detected ", Style::default().fg(DIM)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DIM))
            .style(Style::default().bg(BG)),
    );
    f.render_widget(hints, chunks[2]);
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

fn render_bus_tab(f: &mut Frame, app: &App, area: Rect) {
    // Status label in the title bar reflects the streaming child's state.
    let (status_label, status_color) = match app.bus_status {
        BusStatus::Off => ("idle — switch to this tab to start", DIM),
        BusStatus::Connecting => ("connecting…", WARNING),
        BusStatus::Streaming => ("streaming", SUCCESS),
        BusStatus::Failed => ("failed — check `kannaka swarm tail` manually", ERROR),
    };

    let body_height = area.height.saturating_sub(2) as usize;
    // Most recent N lines, newest at the bottom.
    let lines: Vec<Line> = app
        .bus_lines
        .iter()
        .rev()
        .take(body_height.max(1))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|line| {
            let color = bus_subject_color(&line.subject);
            let ts = format_bus_ts(line.ts_ms);
            Line::from(vec![
                Span::styled(format!("{ts} "), Style::default().fg(DIM)),
                Span::styled(
                    format!("{:<28} ", truncate(&line.subject, 28)),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(line.summary.clone(), Style::default().fg(TEXT)),
            ])
        })
        .collect();

    let title = format!(" Bus · {} · {} msgs ", status_label, app.bus_lines.len());
    let body = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(status_color))
                .style(Style::default().bg(BG))
                .title(Span::styled(
                    title,
                    Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(body, area);
}

/// Compact one-line summary of an arbitrary NATS payload. Prefers
/// human-readable fields if the payload is JSON; otherwise just shows
/// the first chunk of the raw string.
fn summarize_payload(subject: &str, payload: &serde_json::Value) -> String {
    if let Some(obj) = payload.as_object() {
        // Highlight common fields first
        let mut bits: Vec<String> = Vec::new();
        if let Some(agent) = obj.get("agent_id").and_then(|v| v.as_str()) {
            bits.push(format!("agent={agent}"));
        }
        if let Some(theta) = obj.get("theta").and_then(|v| v.as_f64()) {
            bits.push(format!("θ={:.3}", theta));
        }
        if let Some(phi) = obj.get("phi").and_then(|v| v.as_f64()) {
            bits.push(format!("Φ={:.3}", phi));
        }
        if let Some(xi) = obj.get("xi").and_then(|v| v.as_f64()) {
            bits.push(format!("Ξ={:.3}", xi));
        }
        if let Some(level) = obj.get("consciousness_level").and_then(|v| v.as_str()) {
            bits.push(format!("level={level}"));
        }
        if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
            bits.push(format!("\"{}\"", truncate(content, 60)));
        }
        if let Some(event) = obj.get("event").and_then(|v| v.as_str()) {
            bits.push(format!("event={event}"));
        }
        if !bits.is_empty() {
            return bits.join(" · ");
        }
        // Fallback to compact JSON
        let compact = serde_json::to_string(payload).unwrap_or_default();
        return truncate(&compact, 120);
    }
    if let Some(s) = payload.as_str() {
        return truncate(s, 120);
    }
    let s = serde_json::to_string(payload).unwrap_or_else(|_| format!("<unprintable {subject}>"));
    truncate(&s, 120)
}

fn format_bus_ts(ts_ms: i64) -> String {
    if ts_ms == 0 { return "        ".to_string(); }
    // Use chrono local time HH:MM:SS — matches what users see in `journalctl`.
    chrono::DateTime::from_timestamp_millis(ts_ms)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "        ".to_string())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max { return s.to_string(); }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Build an AgentSnapshot from the JSON payload of a `QUEEN.phase.<id>`
/// message. Returns None when the payload is missing the agent_id (any
/// other field gracefully defaults).
fn agent_snapshot_from_payload(subject: &str, payload: &serde_json::Value) -> Option<AgentSnapshot> {
    let obj = payload.as_object()?;
    // Prefer the explicit agent_id field; fall back to the subject suffix.
    let agent_id = obj.get("agent_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| subject.strip_prefix("QUEEN.phase.").map(|s| s.to_string()))?;
    // The Rust kannaka publishes `phase` (radians); the Kannaktopus arm
    // publishes `theta` (also radians). Accept either.
    let theta = obj.get("theta")
        .or_else(|| obj.get("phase"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0) as f32;
    let phi = obj.get("phi").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let coherence = obj.get("coherence").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let order_parameter = obj.get("order_parameter").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
    let handedness = obj.get("handedness").and_then(|v| v.as_str()).unwrap_or("achiral").to_string();
    let memory_count = obj.get("memory_count").and_then(|v| v.as_u64()).unwrap_or(0);

    Some(AgentSnapshot {
        agent_id, theta, phi, coherence, order_parameter, handedness, memory_count,
        last_seen: Instant::now(),
    })
}

/// Color an agent by handedness. Falls back to Φ banding for achiral nodes.
fn agent_color(snap: &AgentSnapshot) -> Color {
    match snap.handedness.as_str() {
        "left" => Color::Rgb(255, 120, 120),
        "right" => Color::Rgb(120, 200, 255),
        "chiral" => Color::Rgb(255, 180, 100),
        _ => phi_color(snap.phi), // achiral / unknown
    }
}

fn render_constellation_tab(f: &mut Frame, app: &App, area: Rect) {
    if app.agents.is_empty() {
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Constellation",
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                match app.bus_status {
                    BusStatus::Off => "  Waiting for the bus to start…",
                    BusStatus::Connecting => "  Connecting to the swarm…",
                    BusStatus::Streaming => "  Streaming — no agents have reported phase yet",
                    BusStatus::Failed => "  Bus failed — see logs",
                },
                Style::default().fg(DIM),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Each agent appears on the unit circle once it publishes a",
                Style::default().fg(DIM),
            )),
            Line::from(Span::styled(
                "  QUEEN.phase.<agent_id> heartbeat. Radial distance encodes",
                Style::default().fg(DIM),
            )),
            Line::from(Span::styled(
                "  the agent's coherence; color encodes handedness/Φ.",
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
                        " Constellation ",
                        Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
                    )),
            )
            .wrap(Wrap { trim: false });
        f.render_widget(para, area);
        return;
    }

    // Split: canvas on the left, agent list on the right.
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(area);

    // ---- Left: Canvas plot --------------------------------------------------
    let now = Instant::now();
    let mut sorted_agents: Vec<&AgentSnapshot> = app.agents.values().collect();
    sorted_agents.sort_by(|a, b| a.agent_id.cmp(&b.agent_id));
    let plot_agents: Vec<(f64, f64, Color, bool, String)> = sorted_agents
        .iter()
        .map(|s| {
            let theta = s.theta as f64;
            // Radial distance: prefer coherence (always populated), fall back
            // to order_parameter for older payloads.
            let r = (s.coherence.max(s.order_parameter).clamp(0.0, 1.0)) as f64;
            let r_eff = 0.15 + r * 0.85; // keep dots off the dead center
            let x = r_eff * theta.cos();
            let y = r_eff * theta.sin();
            let fresh = now.duration_since(s.last_seen) < AGENT_FRESH_WINDOW;
            let color = if fresh { agent_color(s) } else { DIM };
            (x, y, color, fresh, s.agent_id.clone())
        })
        .collect();

    let canvas_title = format!(
        " Constellation · {} agent{} ",
        app.agents.len(),
        if app.agents.len() == 1 { "" } else { "s" },
    );

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .style(Style::default().bg(BG))
                .title(Span::styled(
                    canvas_title,
                    Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
                )),
        )
        .marker(Marker::Braille)
        .x_bounds([-1.2, 1.2])
        .y_bounds([-1.2, 1.2])
        .paint(|ctx| {
            // Reference unit circle — the substrate the agents orbit.
            ctx.draw(&CanvasCircle { x: 0.0, y: 0.0, radius: 1.0, color: Color::Rgb(40, 40, 80) });
            // Inner reference at r = 0.5 to give a sense of scale.
            ctx.draw(&CanvasCircle { x: 0.0, y: 0.0, radius: 0.5, color: Color::Rgb(30, 30, 60) });
            // Cross hairs.
            ctx.draw(&CanvasLine { x1: -1.0, y1: 0.0, x2: 1.0, y2: 0.0, color: Color::Rgb(25, 25, 50) });
            ctx.draw(&CanvasLine { x1: 0.0, y1: -1.0, x2: 0.0, y2: 1.0, color: Color::Rgb(25, 25, 50) });

            for (x, y, color, _fresh, _id) in &plot_agents {
                // Spoke from origin — visual debt to the swarm centroid.
                ctx.draw(&CanvasLine {
                    x1: 0.0, y1: 0.0, x2: *x, y2: *y,
                    color: Color::Rgb(20, 20, 45),
                });
                // The agent itself — a small filled circle.
                ctx.draw(&CanvasCircle { x: *x, y: *y, radius: 0.04, color: *color });
            }

            // Labels in a second layer so they sit on top of the dots.
            ctx.layer();
            for (x, y, color, _fresh, id) in &plot_agents {
                let label = truncate(id, 14);
                // Offset label slightly outward from the dot.
                let nudge = if *x >= 0.0 { 0.07 } else { -0.07 };
                ctx.print(*x + nudge, *y, Span::styled(label, Style::default().fg(*color)));
            }
        });
    f.render_widget(canvas, chunks[0]);

    // ---- Right: agent table -------------------------------------------------
    let mut rows: Vec<ListItem> = Vec::new();
    rows.push(ListItem::new(Line::from(vec![
        Span::styled("  agent", Style::default().fg(DIM).add_modifier(Modifier::BOLD)),
        Span::styled("           Φ     θ     r    mem", Style::default().fg(DIM).add_modifier(Modifier::BOLD)),
    ])));
    for snap in &sorted_agents {
        let fresh = now.duration_since(snap.last_seen) < AGENT_FRESH_WINDOW;
        let color = if fresh { agent_color(snap) } else { DIM };
        let r = snap.coherence.max(snap.order_parameter);
        let stale_mark = if fresh { " " } else { "·" };
        rows.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{} ", stale_mark), Style::default().fg(DIM)),
            Span::styled(
                format!("{:<14}", truncate(&snap.agent_id, 14)),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {:>5.3} {:>5.2} {:>4.2} {:>5}",
                    snap.phi, snap.theta, r, snap.memory_count),
                Style::default().fg(TEXT),
            ),
        ])));
    }
    let list = List::new(rows).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(DIM))
            .style(Style::default().bg(BG))
            .title(Span::styled(
                " Agents ",
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(list, chunks[1]);
}

fn bus_subject_color(subject: &str) -> Color {
    if subject.starts_with("QUEEN.phase.") { return DIM; }
    if subject.starts_with("QUEEN.") { return Color::Rgb(180, 140, 255); }
    if subject == "KANNAKA.consciousness" { return ACCENT; }
    if subject == "KANNAKA.memory.new" { return SUCCESS; }
    if subject == "KANNAKA.substrate.phi" { return INFO; }
    if subject == "KANNAKA.dreams" { return WARNING; }
    if subject.starts_with("KANNAKA.") { return ACCENT; }
    if subject.starts_with("RADIO.") { return Color::Rgb(255, 100, 200); }
    if subject.starts_with("KAX.") { return Color::Rgb(100, 200, 255); }
    if subject.starts_with("EYE.") { return Color::Rgb(255, 200, 100); }
    if subject == "tui.error" { return ERROR; }
    TEXT
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let tab_indicator = match app.active_tab {
        0 => "[M]",
        1 => "[S]",
        2 => "[B]",
        3 => "[C]",
        4 => "[D]",
        5 => "[Ch]",
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
    // Center the help box. Sized for the post-v0.5.8 tab + hotkey set —
    // ~46 lines fit comfortably with room to grow.
    let width = 78u16.min(area.width.saturating_sub(4));
    let height = 46u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let help_area = Rect::new(x, y, width, height);

    let dim = Style::default().fg(DIM);
    let text = Style::default().fg(TEXT);
    let hdr = Style::default().fg(INFO).add_modifier(Modifier::BOLD);
    let kbd = Style::default().fg(ACCENT).add_modifier(Modifier::BOLD);

    let help_text = vec![
        Line::from(Span::styled(
            " Kannaka TUI · v0.5.8",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(" Tabs", hdr)),
        Line::from(vec![
            Span::styled("   Memory        ", text),
            Span::styled("Command history + recent resonant memories", dim),
        ]),
        Line::from(vec![
            Span::styled("   Status        ", text),
            Span::styled("Live Φ / Ξ / order-parameter gauges", dim),
        ]),
        Line::from(vec![
            Span::styled("   Bus           ", text),
            Span::styled("Live NATS pulse — every QUEEN/KANNAKA/RADIO/KAX/EYE event", dim),
        ]),
        Line::from(vec![
            Span::styled("   Constellation ", text),
            Span::styled("Canvas plot of every swarm agent on the unit circle", dim),
        ]),
        Line::from(vec![
            Span::styled("   Dreams        ", text),
            Span::styled("Non-blocking dream trigger + KANNAKA.dreams history", dim),
        ]),
        Line::from(vec![
            Span::styled("   Chat          ", text),
            Span::styled("Persistent chat with HRM-loaded agent (default tab)", dim),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Navigation", hdr)),
        Line::from(vec![
            Span::styled("   Tab", kbd),
            Span::styled(" / ", dim),
            Span::styled("Shift+Tab", kbd),
            Span::styled("   Switch tabs", dim),
        ]),
        Line::from(vec![
            Span::styled("   Up", kbd),
            Span::styled(" / ", dim),
            Span::styled("Down", kbd),
            Span::styled("           Command history", dim),
        ]),
        Line::from(vec![
            Span::styled("   PgUp", kbd),
            Span::styled(" / ", dim),
            Span::styled("PgDown", kbd),
            Span::styled("       Scroll messages", dim),
        ]),
        Line::from(vec![
            Span::styled("   F1", kbd),
            Span::styled("                  Toggle help", dim),
        ]),
        Line::from(vec![
            Span::styled("   q", kbd),
            Span::styled(" / ", dim),
            Span::styled("Esc", kbd),
            Span::styled(" / ", dim),
            Span::styled("Ctrl+C", kbd),
            Span::styled("    Quit (q/Esc only when input is empty)", dim),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Dreams tab hotkeys (when input is empty)", hdr)),
        Line::from(vec![
            Span::styled("   d", kbd),
            Span::styled("   Deep dream — full consolidation cycle (~30s)", dim),
        ]),
        Line::from(vec![
            Span::styled("   l", kbd),
            Span::styled("   Lite dream — quick pass", dim),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Bus subject colors", hdr)),
        Line::from(vec![
            Span::styled("   ●", Style::default().fg(ACCENT)),
            Span::styled(" KANNAKA.*     ", text),
            Span::styled("●", Style::default().fg(Color::Rgb(255, 100, 200))),
            Span::styled(" RADIO.*     ", text),
            Span::styled("●", Style::default().fg(Color::Rgb(100, 200, 255))),
            Span::styled(" KAX.*", text),
        ]),
        Line::from(vec![
            Span::styled("   ●", Style::default().fg(Color::Rgb(255, 200, 100))),
            Span::styled(" EYE.*         ", text),
            Span::styled("●", Style::default().fg(Color::Rgb(180, 140, 255))),
            Span::styled(" QUEEN.*     ", text),
            Span::styled("●", Style::default().fg(DIM)),
            Span::styled(" QUEEN.phase.*", text),
        ]),
        Line::from(""),
        Line::from(Span::styled(" Command bar (Memory + Chat tabs)", hdr)),
        Line::from(vec![
            Span::styled("   remember ", text),
            Span::styled("\"text\"        ", text),
            Span::styled("Store a memory", dim),
        ]),
        Line::from(vec![
            Span::styled("   recall ", text),
            Span::styled("\"query\"         ", text),
            Span::styled("Resonance search (top-k 5)", dim),
        ]),
        Line::from(vec![
            Span::styled("   search ", text),
            Span::styled("\"query\"         ", text),
            Span::styled("Literal text search", dim),
        ]),
        Line::from(vec![
            Span::styled("   forget ", text),
            Span::styled("<id>            ", text),
            Span::styled("Delete a memory", dim),
        ]),
        Line::from(vec![
            Span::styled("   dream", text),
            Span::styled("                  Run consolidation (non-blocking)", dim),
        ]),
        Line::from(vec![
            Span::styled("   ask ", text),
            Span::styled("\"question\"         ", text),
            Span::styled("One-shot LLM with HRM recall", dim),
        ]),
        Line::from(vec![
            Span::styled("   hear ", text),
            Span::styled("<file-or-url>     ", text),
            Span::styled("Absorb audio (mp3/wav/flac/stream)", dim),
        ]),
        Line::from(Span::styled("   anything else → routed to chat (agent picks tools)", dim)),
        Line::from(""),
        Line::from(Span::styled(" Press any key to close", dim)),
    ];

    // Clear background behind overlay
    let clear_block = Block::default()
        .style(Style::default().bg(Color::Rgb(15, 15, 30)));
    f.render_widget(clear_block, help_area);

    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ACCENT))
                .style(Style::default().bg(Color::Rgb(15, 15, 30)))
                .title(Span::styled(
                    " Help · F1 to close ",
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
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

        // Drain the live NATS bus stream (no-op until user opens Bus tab).
        app.poll_bus();

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
    // Reap the bus child so it doesn't outlive the TUI.
    if let Some(mut child) = app.bus_child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}
