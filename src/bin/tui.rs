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
}

impl App {
    fn new() -> Self {
        // Find the kannaka binary — prefer the release build next to us
        let kannaka_bin = Self::find_kannaka_binary();
        let agent_name = Self::load_agent_name();

        Self {
            active_tab: 0,
            tabs: vec!["Memory", "Status", "Constellation", "Dreams"],
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

    fn load_status(&mut self) {
        let output = Command::new(&self.kannaka_bin)
            .args(["status"])
            .env("KANNAKA_QUIET", "1")
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    self.status = Some(Status {
                        phi: val["phi"].as_f64().unwrap_or(0.0) as f32,
                        xi: val["xi"].as_f64().unwrap_or(0.0) as f32,
                        order: val["mean_order"].as_f64().unwrap_or(0.0) as f32,
                        memories: val["total_memories"].as_u64().unwrap_or(0),
                        clusters: val["num_clusters"].as_u64().unwrap_or(0),
                        links: 0, // not in status, use observe
                        level: val["consciousness_level"]
                            .as_str()
                            .unwrap_or("Unknown")
                            .to_string(),
                        active: val["active_memories"].as_u64().unwrap_or(0),
                    });
                }
            }
            Ok(out) => {
                let stderr = String::from_utf8_lossy(&out.stderr);
                self.messages.push(Message {
                    role: Role::Error,
                    content: format!("Status failed: {}", stderr.trim()),
                });
            }
            Err(e) => {
                self.status = None;
                self.messages.push(Message {
                    role: Role::Error,
                    content: format!(
                        "Cannot find kannaka binary at '{}': {}. Install with: cargo install --path .",
                        self.kannaka_bin, e
                    ),
                });
            }
        }
        self.last_status_poll = Instant::now();
    }

    fn load_observe(&mut self) {
        let output = Command::new(&self.kannaka_bin)
            .args(["observe", "--json"])
            .env("KANNAKA_QUIET", "1")
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    // Extract topology links
                    if let Some(links) = val["topology"]["total_links"].as_u64() {
                        if let Some(ref mut status) = self.status {
                            status.links = links;
                        }
                    }
                    // Extract recent memories from waves.strongest
                    if let Some(strongest) = val["waves"]["strongest"].as_array() {
                        self.memories = strongest
                            .iter()
                            .map(|m| MemoryEntry {
                                content: m["content_preview"]
                                    .as_str()
                                    .unwrap_or("")
                                    .to_string(),
                                amplitude: m["amplitude"].as_f64().unwrap_or(0.0) as f32,
                            })
                            .collect();
                    }
                }
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

    fn submit_input(&mut self) {
        let input = self.input.trim().to_string();
        if input.is_empty() {
            return;
        }

        // Save to history
        self.history.push(input.clone());
        self.history_idx = None;

        // Parse the command
        if input.starts_with("remember ") {
            let text = input.strip_prefix("remember ").unwrap().trim();
            let text = text.trim_matches('"').to_string();
            self.execute_remember(&text);
        } else if input.starts_with("recall ") {
            let query = input.strip_prefix("recall ").unwrap().trim();
            let query = query.trim_matches('"').to_string();
            self.execute_recall(&query);
        } else if input.starts_with("forget ") {
            let id = input.strip_prefix("forget ").unwrap().trim().to_string();
            self.execute_forget(&id);
        } else if input == "dream" || input.starts_with("dream ") {
            self.execute_dream();
        } else if input == "status" || input == "observe" {
            self.load_status();
            self.load_observe();
            self.messages.push(Message {
                role: Role::System,
                content: "Status refreshed.".to_string(),
            });
        } else if input == "help" || input == "?" {
            self.show_help = true;
        } else if input == "quit" || input == "exit" || input == "q" {
            self.should_quit = true;
        } else {
            self.messages.push(Message {
                role: Role::Error,
                content: format!(
                    "Unknown command: '{}'. Try: remember, recall, forget, dream, status, help",
                    input
                ),
            });
        }

        self.input.clear();
        self.cursor_pos = 0;
        // Auto-scroll to bottom
        self.scroll_offset = 0;
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

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let tab_indicator = match app.active_tab {
        0 => "[M]",
        1 => "[S]",
        2 => "[C]",
        3 => "[D]",
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
    let width = 50u16.min(area.width.saturating_sub(4));
    let height = 18u16.min(area.height.saturating_sub(4));
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
        Line::from(Span::styled(" Commands", Style::default().fg(INFO).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled("   remember \"text\"  Store a memory", Style::default().fg(TEXT))),
        Line::from(Span::styled("   recall \"query\"   Search memories", Style::default().fg(TEXT))),
        Line::from(Span::styled("   forget <id>      Delete memory", Style::default().fg(TEXT))),
        Line::from(Span::styled("   dream            Run dream cycle", Style::default().fg(TEXT))),
        Line::from(Span::styled("   status           Refresh metrics", Style::default().fg(TEXT))),
        Line::from(""),
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
