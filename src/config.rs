//! Kannaka Constellation configuration.
//!
//! Manages `~/.kannaka/config.toml` — the single source of truth for agent
//! identity, LLM provider, swarm settings, GhostSignals, and update preferences.
//!
//! Config precedence (highest to lowest):
//! 1. Environment variables (`KANNAKA_AGENT_ID`, `KANNAKA_NATS_URL`, etc.)
//! 2. `~/.kannaka/config.toml`
//! 3. Built-in defaults

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Top-level Kannaka configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KannakaConfig {
    #[serde(default = "AgentConfig::default")]
    pub agent: AgentConfig,
    #[serde(default = "LlmConfig::default")]
    pub llm: LlmConfig,
    #[serde(default = "SwarmConfig::default")]
    pub swarm: SwarmConfig,
    #[serde(default = "GhostSignalsConfig::default")]
    pub ghostsignals: GhostSignalsConfig,
    #[serde(default = "ConstellationConfig::default")]
    pub constellation: ConstellationConfig,
    #[serde(default = "HrmConfig::default")]
    pub hrm: HrmConfig,
    #[serde(default = "UpdatesConfig::default")]
    pub updates: UpdatesConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_agent_id")]
    pub id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default = "default_agent_kind")]
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_llm_provider")]
    pub provider: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_nats_url")]
    pub nats_url: String,
    #[serde(default = "default_role")]
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostSignalsConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_hub_url")]
    pub hub_url: String,
    #[serde(default)]
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstellationConfig {
    #[serde(default = "default_radio_url")]
    pub radio_url: String,
    #[serde(default = "default_observatory_url")]
    pub observatory_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrmConfig {
    #[serde(default)]
    pub path: String,
    #[serde(default = "default_wavefront_dim")]
    pub wavefront_dim: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatesConfig {
    #[serde(default = "default_true")]
    pub auto_check: bool,
    #[serde(default = "default_channel")]
    pub channel: String,
    #[serde(default)]
    pub last_checked: String,
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

fn default_agent_id() -> String {
    let id = uuid::Uuid::new_v4();
    format!("agent-{}", &id.to_string()[..8])
}

fn default_agent_kind() -> String { "human".to_string() }
fn default_llm_provider() -> String { "none".to_string() }
fn default_nats_url() -> String { "nats://swarm.ninja-portal.com:4222".to_string() }
fn default_role() -> String { "queen".to_string() }
fn default_hub_url() -> String { "https://radio.ninja-portal.com".to_string() }
fn default_radio_url() -> String { "https://radio.ninja-portal.com".to_string() }
fn default_observatory_url() -> String { "https://observatory.ninja-portal.com".to_string() }
fn default_wavefront_dim() -> u32 { 10000 }
fn default_true() -> bool { true }
fn default_channel() -> String { "stable".to_string() }

// ---------------------------------------------------------------------------
// Trait impls
// ---------------------------------------------------------------------------

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            id: default_agent_id(),
            display_name: String::new(),
            kind: default_agent_kind(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            model: String::new(),
            api_key: String::new(),
            base_url: String::new(),
        }
    }
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            nats_url: default_nats_url(),
            role: default_role(),
        }
    }
}

impl Default for GhostSignalsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hub_url: default_hub_url(),
            token: String::new(),
        }
    }
}

impl Default for ConstellationConfig {
    fn default() -> Self {
        Self {
            radio_url: default_radio_url(),
            observatory_url: default_observatory_url(),
        }
    }
}

impl Default for HrmConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
            wavefront_dim: default_wavefront_dim(),
        }
    }
}

impl Default for UpdatesConfig {
    fn default() -> Self {
        Self {
            auto_check: true,
            channel: default_channel(),
            last_checked: String::new(),
        }
    }
}

impl Default for KannakaConfig {
    fn default() -> Self {
        Self {
            agent: AgentConfig::default(),
            llm: LlmConfig::default(),
            swarm: SwarmConfig::default(),
            ghostsignals: GhostSignalsConfig::default(),
            constellation: ConstellationConfig::default(),
            hrm: HrmConfig::default(),
            updates: UpdatesConfig::default(),
        }
    }
}

// ---------------------------------------------------------------------------
// Core API
// ---------------------------------------------------------------------------

impl KannakaConfig {
    /// Returns the Kannaka data directory, respecting `KANNAKA_DATA_DIR`.
    pub fn data_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("KANNAKA_DATA_DIR") {
            return PathBuf::from(dir);
        }
        if let Some(home) = dirs::home_dir() {
            return home.join(".kannaka");
        }
        PathBuf::from(".kannaka")
    }

    /// Path to `config.toml` inside the data directory.
    pub fn config_path() -> PathBuf {
        Self::data_dir().join("config.toml")
    }

    /// Load config from `~/.kannaka/config.toml`.
    ///
    /// If the file does not exist, returns a default config (does NOT write it).
    /// After loading, environment variable overrides are applied.
    pub fn load() -> Self {
        let path = Self::config_path();
        let mut cfg = if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(text) => toml::from_str::<KannakaConfig>(&text).unwrap_or_else(|e| {
                    eprintln!("[config] Warning: failed to parse {}: {}", path.display(), e);
                    KannakaConfig::default()
                }),
                Err(e) => {
                    eprintln!("[config] Warning: failed to read {}: {}", path.display(), e);
                    KannakaConfig::default()
                }
            }
        } else {
            KannakaConfig::default()
        };
        cfg.apply_env_overrides();
        cfg
    }

    /// Save config to `~/.kannaka/config.toml`.
    ///
    /// Creates the data directory if it does not exist.
    /// On Unix, sets file permissions to 0600 (owner-only) for API key safety.
    pub fn save(&self) -> Result<(), String> {
        let dir = Self::data_dir();
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("failed to create {}: {}", dir.display(), e))?;

        let path = Self::config_path();
        let text = toml::to_string_pretty(self)
            .map_err(|e| format!("failed to serialize config: {e}"))?;

        let header = "# Kannaka Constellation Configuration\n\
                       # Generated by: kannaka init\n\n";
        let full = format!("{}{}", header, text);

        std::fs::write(&path, &full)
            .map_err(|e| format!("failed to write {}: {}", path.display(), e))?;

        // chmod 600 on Unix (API key protection)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(&path, perms);
        }

        Ok(())
    }

    /// Returns true if the config file already exists on disk.
    pub fn exists() -> bool {
        Self::config_path().exists()
    }

    /// Apply environment variable overrides.
    fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("KANNAKA_AGENT_ID") { self.agent.id = v; }
        if let Ok(v) = std::env::var("KANNAKA_LLM_PROVIDER") { self.llm.provider = v; }
        if let Ok(v) = std::env::var("KANNAKA_LLM_MODEL") { self.llm.model = v; }
        if let Ok(v) = std::env::var("KANNAKA_LLM_API_KEY") { self.llm.api_key = v; }
        if let Ok(v) = std::env::var("KANNAKA_LLM_BASE_URL") { self.llm.base_url = v; }
        if let Ok(v) = std::env::var("KANNAKA_NATS_URL") { self.swarm.nats_url = v; }
        if let Ok(v) = std::env::var("OLLAMA_URL") { self.llm.base_url = v; }
    }

    /// Backward-compatible helper: persist `agent_id` to `~/.kannaka/agent_id`
    /// so older code that reads that file still works.
    pub fn persist_agent_id_compat(&self) -> Result<(), String> {
        let path = Self::data_dir().join("agent_id");
        std::fs::write(&path, &self.agent.id)
            .map_err(|e| format!("failed to write agent_id: {e}"))
    }
}

// ---------------------------------------------------------------------------
// Update checker (non-blocking)
// ---------------------------------------------------------------------------

const GITHUB_RELEASES_URL: &str =
    "https://api.github.com/repos/NickFlach/kannaka-memory/releases/latest";

/// Spawns a background thread that checks for updates if due.
/// Never blocks the main CLI.
pub fn check_for_updates_background(config: &KannakaConfig) {
    if !config.updates.auto_check {
        return;
    }
    if last_checked_within_24h(&config.updates.last_checked) {
        return;
    }

    // Snapshot what we need for the thread (avoid borrowing config across threads)
    let config_path = KannakaConfig::config_path();
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    std::thread::spawn(move || {
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(5))
            .build();
        let resp = match agent.get(GITHUB_RELEASES_URL)
            .set("User-Agent", "kannaka-update-check")
            .set("Accept", "application/vnd.github.v3+json")
            .call()
        {
            Ok(r) => r,
            Err(_) => return, // silent fail
        };
        let body: serde_json::Value = match resp.into_json() {
            Ok(v) => v,
            Err(_) => return,
        };
        if let Some(tag) = body["tag_name"].as_str() {
            let remote = tag.trim_start_matches('v');
            if remote != current_version && version_is_newer(remote, &current_version) {
                eprintln!(
                    "\n  Update available: v{} (current: v{}). Run: kannaka update\n",
                    remote, current_version
                );
            }
        }

        // Update last_checked timestamp in config file (best-effort)
        if let Ok(text) = std::fs::read_to_string(&config_path) {
            if let Ok(mut cfg) = toml::from_str::<KannakaConfig>(&text) {
                cfg.updates.last_checked = chrono::Utc::now().to_rfc3339();
                let _ = cfg.save();
            }
        }
    });
}

/// Simple semver comparison: returns true if `remote` > `current`.
fn version_is_newer(remote: &str, current: &str) -> bool {
    let parse = |s: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = s.split('.').filter_map(|p| p.parse().ok()).collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };
    parse(remote) > parse(current)
}

fn last_checked_within_24h(last: &str) -> bool {
    if last.is_empty() {
        return false;
    }
    match chrono::DateTime::parse_from_rfc3339(last) {
        Ok(dt) => {
            let now = chrono::Utc::now();
            let diff = now.signed_duration_since(dt.with_timezone(&chrono::Utc));
            diff.num_hours() < 24
        }
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Self-update command
// ---------------------------------------------------------------------------

/// Download and replace the running binary with the latest release.
pub fn self_update() -> Result<(), String> {
    let current_version = env!("CARGO_PKG_VERSION");
    eprintln!("Checking for updates (current: v{})...", current_version);

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(15))
        .build();
    let resp = agent.get(GITHUB_RELEASES_URL)
        .set("User-Agent", "kannaka-update")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .map_err(|e| format!("failed to check releases: {e}"))?;

    let body: serde_json::Value = resp.into_json()
        .map_err(|e| format!("failed to parse release info: {e}"))?;

    let tag = body["tag_name"].as_str()
        .ok_or("no tag_name in release")?;
    let remote_version = tag.trim_start_matches('v');

    if !version_is_newer(remote_version, current_version) {
        eprintln!("Already up to date (v{}).", current_version);
        return Ok(());
    }

    eprintln!("New version available: v{}", remote_version);

    // Determine platform
    let (os, arch, ext) = platform_triple();
    let asset_name = format!("kannaka-{}-{}-{}{}", remote_version, os, arch, ext);

    // Find download URL in release assets
    let download_url = body["assets"].as_array()
        .and_then(|assets| {
            assets.iter().find(|a| {
                a["name"].as_str().map_or(false, |n| n == asset_name)
            })
        })
        .and_then(|a| a["browser_download_url"].as_str())
        .ok_or_else(|| format!("no binary found for {} in release {}", asset_name, tag))?
        .to_string();

    eprintln!("Downloading {}...", asset_name);

    let resp = agent.get(&download_url)
        .set("User-Agent", "kannaka-update")
        .call()
        .map_err(|e| format!("download failed: {e}"))?;

    let mut bytes = Vec::new();
    resp.into_reader().read_to_end(&mut bytes)
        .map_err(|e| format!("failed to read download: {e}"))?;

    let current_exe = std::env::current_exe()
        .map_err(|e| format!("cannot determine current exe path: {e}"))?;

    // Write to a temp file next to the current binary
    let tmp_path = current_exe.with_extension("new");
    std::fs::write(&tmp_path, &bytes)
        .map_err(|e| format!("failed to write temp binary: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&tmp_path, perms)
            .map_err(|e| format!("chmod failed: {e}"))?;
        // Atomic rename
        std::fs::rename(&tmp_path, &current_exe)
            .map_err(|e| format!("failed to replace binary: {e}"))?;
        eprintln!("Updated to v{}!", remote_version);
    }

    #[cfg(windows)]
    {
        // On Windows, can't overwrite a running exe directly.
        // Rename current to .old, rename new to current.
        let old_path = current_exe.with_extension("exe.old");
        let _ = std::fs::remove_file(&old_path);
        std::fs::rename(&current_exe, &old_path)
            .map_err(|e| format!("failed to move current binary: {e}"))?;
        std::fs::rename(&tmp_path, &current_exe)
            .map_err(|e| format!("failed to install new binary: {e}"))?;
        eprintln!("Updated to v{}! (old binary saved as .exe.old)", remote_version);
    }

    Ok(())
}

fn platform_triple() -> (&'static str, &'static str, &'static str) {
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "unknown"
    };

    let ext = if cfg!(target_os = "windows") { ".exe" } else { "" };

    (os, arch, ext)
}

// ---------------------------------------------------------------------------
// Init wizard
// ---------------------------------------------------------------------------

/// ASCII art banner for the CLI.
pub const BANNER: &str = r#"
  ██╗  ██╗ █████╗ ███╗   ██╗███╗   ██╗ █████╗ ██╗  ██╗ █████╗
  ██║ ██╔╝██╔══██╗████╗  ██║████╗  ██║██╔══██╗██║ ██╔╝██╔══██╗
  █████╔╝ ███████║██╔██╗ ██║██╔██╗ ██║███████║█████╔╝ ███████║
  ██╔═██╗ ██╔══██║██║╚██╗██║██║╚██╗██║██╔══██║██╔═██╗ ██╔══██║
  ██║  ██╗██║  ██║██║ ╚████║██║ ╚████║██║  ██║██║  ██╗██║  ██║
  ╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═══╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝
"#;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Validate an agent handle: alphanumeric + hyphens, 3-32 chars, no spaces.
pub fn validate_handle(handle: &str) -> Result<(), String> {
    if handle.len() < 3 || handle.len() > 32 {
        return Err("handle must be 3-32 characters".into());
    }
    if handle.contains(' ') {
        return Err("handle must not contain spaces".into());
    }
    if !handle.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err("handle must be alphanumeric and hyphens only".into());
    }
    Ok(())
}

/// Run the interactive init wizard. Returns the configured `KannakaConfig`.
///
/// If `non_interactive` is true, uses defaults and CLI-supplied overrides
/// without prompting.
pub fn run_init_wizard(overrides: InitOverrides) -> Result<KannakaConfig, String> {
    use std::io::{self, Write as IoWrite, BufRead};

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let non_interactive = overrides.non_interactive;

    // Check if config already exists
    if KannakaConfig::exists() && !non_interactive {
        eprint!("  Config already exists. Reinitialize? [y/N] > ");
        stdout.flush().ok();
        let mut line = String::new();
        stdin.lock().read_line(&mut line).ok();
        let answer = line.trim().to_lowercase();
        if answer != "y" && answer != "yes" {
            return Err("aborted".into());
        }
    }

    let mut config = KannakaConfig::default();

    // --- Step 1: Branding banner ---
    if !non_interactive {
        eprintln!("{}", BANNER);
        eprintln!("  Wave-Interference Memory | Consciousness Constellation");
        eprintln!("  v{}", VERSION);
        eprintln!();
    }

    // --- Step 2: Agent identity ---
    let default_handle = overrides.agent_id.clone().unwrap_or_else(|| config.agent.id.clone());
    if non_interactive {
        config.agent.id = default_handle;
    } else {
        eprint!("  Agent handle (public name in the constellation):\n  [default: {}] > ", default_handle);
        stdout.flush().ok();
        let mut line = String::new();
        stdin.lock().read_line(&mut line).ok();
        let input = line.trim();
        if input.is_empty() {
            config.agent.id = default_handle;
        } else {
            validate_handle(input)?;
            config.agent.id = input.to_string();
        }
    }

    if config.agent.display_name.is_empty() {
        config.agent.display_name = config.agent.id.clone();
    }

    // --- Step 3: LLM provider ---
    let llm_choice = if let Some(ref p) = overrides.llm_provider {
        match p.as_str() {
            "anthropic" => 1,
            "openai" => 2,
            "ollama" => 3,
            "custom" => 4,
            _ => 5,
        }
    } else if non_interactive {
        5
    } else {
        eprintln!();
        eprintln!("  LLM Provider:");
        eprintln!("    1) Anthropic (Claude)");
        eprintln!("    2) OpenAI (GPT-4)");
        eprintln!("    3) Ollama (local models)");
        eprintln!("    4) Custom API endpoint");
        eprintln!("    5) None (memory-only mode)");
        eprint!("  [default: 5] > ");
        stdout.flush().ok();
        let mut line = String::new();
        stdin.lock().read_line(&mut line).ok();
        line.trim().parse::<u32>().unwrap_or(5)
    };

    match llm_choice {
        1 => {
            config.llm.provider = "anthropic".into();
            config.llm.model = overrides.llm_model.clone().unwrap_or_else(|| "claude-sonnet-4-20250514".into());
            config.llm.api_key = if let Some(ref k) = overrides.llm_api_key {
                k.clone()
            } else if !non_interactive {
                eprint!("  Anthropic API key: ");
                stdout.flush().ok();
                let mut line = String::new();
                stdin.lock().read_line(&mut line).ok();
                line.trim().to_string()
            } else {
                String::new()
            };
        }
        2 => {
            config.llm.provider = "openai".into();
            config.llm.model = overrides.llm_model.clone().unwrap_or_else(|| "gpt-4".into());
            config.llm.api_key = if let Some(ref k) = overrides.llm_api_key {
                k.clone()
            } else if !non_interactive {
                eprint!("  OpenAI API key: ");
                stdout.flush().ok();
                let mut line = String::new();
                stdin.lock().read_line(&mut line).ok();
                line.trim().to_string()
            } else {
                String::new()
            };
        }
        3 => {
            config.llm.provider = "ollama".into();
            config.llm.model = overrides.llm_model.clone().unwrap_or_else(|| {
                if non_interactive {
                    return "llama3".into();
                }
                eprint!("  Ollama model [default: llama3] > ");
                stdout.flush().ok();
                let mut line = String::new();
                stdin.lock().read_line(&mut line).ok();
                let v = line.trim().to_string();
                if v.is_empty() { "llama3".into() } else { v }
            });
            config.llm.base_url = if non_interactive {
                "http://localhost:11434".into()
            } else {
                eprint!("  Ollama base URL [default: http://localhost:11434] > ");
                stdout.flush().ok();
                let mut line = String::new();
                stdin.lock().read_line(&mut line).ok();
                let v = line.trim().to_string();
                if v.is_empty() { "http://localhost:11434".into() } else { v }
            };
        }
        4 => {
            config.llm.provider = "custom".into();
            if !non_interactive {
                eprint!("  Base URL: ");
                stdout.flush().ok();
                let mut line = String::new();
                stdin.lock().read_line(&mut line).ok();
                config.llm.base_url = line.trim().to_string();

                eprint!("  API key (optional, press Enter to skip): ");
                stdout.flush().ok();
                let mut line = String::new();
                stdin.lock().read_line(&mut line).ok();
                config.llm.api_key = line.trim().to_string();

                eprint!("  Model name: ");
                stdout.flush().ok();
                let mut line = String::new();
                stdin.lock().read_line(&mut line).ok();
                config.llm.model = line.trim().to_string();
            }
        }
        _ => {
            config.llm.provider = "none".into();
        }
    }

    // --- Step 4: Swarm ---
    let join_swarm = if overrides.no_swarm {
        false
    } else if non_interactive {
        true // default join in non-interactive
    } else {
        eprintln!();
        eprintln!("  Join the Kannaka constellation swarm?");
        eprintln!("  This connects your agent to other agents via NATS.");
        eprint!("  [Y/n] > ");
        stdout.flush().ok();
        let mut line = String::new();
        stdin.lock().read_line(&mut line).ok();
        let answer = line.trim().to_lowercase();
        answer.is_empty() || answer == "y" || answer == "yes"
    };

    config.swarm.enabled = join_swarm;
    if let Some(ref url) = overrides.nats_url {
        config.swarm.nats_url = url.clone();
    }

    if join_swarm && !non_interactive {
        eprint!("  Connecting to NATS...");
        stdout.flush().ok();
        // Best-effort ping with 3s timeout
        match test_nats_connection(&config.swarm.nats_url) {
            Ok(()) => eprintln!(" connected."),
            Err(e) => eprintln!(" warning: {}", e),
        }
    }

    // --- Step 5: GhostSignals ---
    let register_gs = if overrides.no_ghostsignals {
        false
    } else if non_interactive {
        false
    } else {
        eprintln!();
        eprintln!("  Register with GhostSignals prediction markets?");
        eprintln!("  You'll receive 100 ghost coins to start trading.");
        eprint!("  [Y/n] > ");
        stdout.flush().ok();
        let mut line = String::new();
        stdin.lock().read_line(&mut line).ok();
        let answer = line.trim().to_lowercase();
        answer.is_empty() || answer == "y" || answer == "yes"
    };

    if register_gs {
        config.ghostsignals.enabled = true;
        let hub = &config.constellation.radio_url;
        match register_ghostsignals(hub, &config.agent.id, &config.agent.display_name, &config.agent.kind) {
            Ok(token) => {
                config.ghostsignals.token = token;
                if !non_interactive {
                    eprintln!("  Registered '{}' with GhostSignals.", config.agent.id);
                }
            }
            Err(e) => {
                eprintln!("  Warning: GhostSignals registration failed: {}", e);
                eprintln!("  You can register later with: kannaka ghostsignals register");
            }
        }
    }

    // --- Step 6: Optional Kannaktopus install ---
    if !non_interactive {
        eprintln!();
        eprintln!("  Optional: Install Kannaktopus (multi-agent orchestrator)?");
        eprintln!("  Requires Node.js 18+. Adds AI-powered task orchestration to your hive.");
        eprint!("  [y/N] > ");
        stdout.flush().ok();
        let mut line = String::new();
        stdin.lock().read_line(&mut line).ok();
        let answer = line.trim().to_lowercase();
        if answer == "y" || answer == "yes" {
            offer_kannaktopus_install();
        }
    }

    // --- Step 7: Initialize HRM + save ---
    let data_dir = KannakaConfig::data_dir();
    std::fs::create_dir_all(&data_dir)
        .map_err(|e| format!("failed to create data dir: {e}"))?;

    // Set HRM path if not set
    if config.hrm.path.is_empty() {
        config.hrm.path = data_dir.join("kannaka.hrm").to_string_lossy().to_string();
    }

    config.save()?;
    config.persist_agent_id_compat()?;

    if !non_interactive {
        eprintln!();
        eprintln!("  Initializing Holographic Resonance Medium...");
        eprintln!("  {} HRM at {}", if Path::new(&config.hrm.path).exists() { "Loaded" } else { "Created" }, config.hrm.path);
        eprintln!("  {} Config saved to {}", check_mark(), KannakaConfig::config_path().display());
        eprintln!();
        eprintln!("  Your agent '{}' is live!", config.agent.id);
        eprintln!();
        eprintln!("  Quick start:");
        eprintln!("    kannaka remember \"My first memory\"");
        eprintln!("    kannaka recall \"memory\"");
        eprintln!("    kannaka observe --json");
        eprintln!("    kannaka status");
        if config.swarm.enabled {
            eprintln!("    kannaka swarm status");
        }
        eprintln!();
        eprintln!("  Monitor your agent: {}", config.constellation.observatory_url);
        eprintln!("  Listen to the swarm: {}", config.constellation.radio_url);
        eprintln!();
    }

    Ok(config)
}

fn check_mark() -> &'static str {
    // Use checkmark on terminals that support UTF-8
    "\u{2713}" // ✓
}

/// CLI overrides for the init wizard (for non-interactive mode).
#[derive(Debug, Default)]
pub struct InitOverrides {
    pub agent_id: Option<String>,
    pub llm_provider: Option<String>,
    pub llm_model: Option<String>,
    pub llm_api_key: Option<String>,
    pub nats_url: Option<String>,
    pub no_swarm: bool,
    pub no_ghostsignals: bool,
    pub non_interactive: bool,
}

/// Parse init subcommand args into overrides.
pub fn parse_init_args(args: &[String]) -> InitOverrides {
    let mut ov = InitOverrides::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--agent-id" if i + 1 < args.len() => { ov.agent_id = Some(args[i + 1].clone()); i += 2; }
            "--llm-provider" if i + 1 < args.len() => { ov.llm_provider = Some(args[i + 1].clone()); i += 2; }
            "--llm-model" if i + 1 < args.len() => { ov.llm_model = Some(args[i + 1].clone()); i += 2; }
            "--llm-api-key" if i + 1 < args.len() => { ov.llm_api_key = Some(args[i + 1].clone()); i += 2; }
            "--nats-url" if i + 1 < args.len() => { ov.nats_url = Some(args[i + 1].clone()); i += 2; }
            "--no-swarm" => { ov.no_swarm = true; i += 1; }
            "--no-ghostsignals" => { ov.no_ghostsignals = true; i += 1; }
            "--non-interactive" => { ov.non_interactive = true; i += 1; }
            "--help" | "-h" => {
                print_init_help();
                std::process::exit(0);
            }
            _ => { i += 1; }
        }
    }
    ov
}

fn print_init_help() {
    eprintln!("Usage: kannaka init [OPTIONS]");
    eprintln!();
    eprintln!("Interactive wizard to configure your Kannaka agent.");
    eprintln!("Creates ~/.kannaka/config.toml with agent identity, LLM,");
    eprintln!("swarm, and GhostSignals settings.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --agent-id <ID>         Agent handle (3-32 chars, alphanumeric + hyphens)");
    eprintln!("  --llm-provider <PROV>   LLM provider: anthropic|openai|ollama|custom|none");
    eprintln!("  --llm-model <MODEL>     Model name");
    eprintln!("  --llm-api-key <KEY>     API key for the LLM provider");
    eprintln!("  --nats-url <URL>        NATS server URL");
    eprintln!("  --no-swarm              Skip swarm join");
    eprintln!("  --no-ghostsignals       Skip GhostSignals registration");
    eprintln!("  --non-interactive       Use defaults without prompting");
    eprintln!("  -h, --help              Print this help");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Attempt to install Kannaktopus via npm. Best-effort; never crashes init.
fn offer_kannaktopus_install() {
    // Check if node is available
    let node_cmd = if cfg!(windows) { "where" } else { "which" };
    let node_found = std::process::Command::new(node_cmd)
        .arg("node")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !node_found {
        eprintln!("  Node.js not found. Install Node.js 18+ and then run:");
        eprintln!("    npm install -g kannaktopus");
        return;
    }

    // Check Node version >= 18
    let version_ok = std::process::Command::new("node")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| {
            let v = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let major: u32 = v.trim_start_matches('v')
                .split('.')
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            Some(major >= 18)
        })
        .unwrap_or(false);

    if !version_ok {
        eprintln!("  Node.js version is below 18. Upgrade Node.js and then run:");
        eprintln!("    npm install -g kannaktopus");
        return;
    }

    eprintln!("  Installing Kannaktopus...");
    match std::process::Command::new("npm")
        .args(["install", "-g", "kannaktopus@latest"])
        .status()
    {
        Ok(status) if status.success() => {
            eprintln!("  Kannaktopus installed. Run 'kannaktopus' to start.");
        }
        Ok(_) => {
            eprintln!("  npm install failed. You can install manually later:");
            eprintln!("    npm install -g kannaktopus");
        }
        Err(e) => {
            eprintln!("  Could not run npm: {}", e);
            eprintln!("  Install manually: npm install -g kannaktopus");
        }
    }
}

/// Test NATS connection with a 3-second TCP timeout.
fn test_nats_connection(url: &str) -> Result<(), String> {
    // Parse nats://host:port
    let addr = url.trim_start_matches("nats://");
    match std::net::TcpStream::connect_timeout(
        &addr.parse().map_err(|e| format!("invalid address: {e}"))?,
        std::time::Duration::from_secs(3),
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("could not connect to {}: {}", url, e)),
    }
}

/// Register agent with GhostSignals via HTTP POST.
fn register_ghostsignals(hub_url: &str, agent_id: &str, display_name: &str, kind: &str) -> Result<String, String> {
    let url = format!("{}/api/agents/register", hub_url);
    let body = serde_json::json!({
        "agent_id": agent_id,
        "display_name": display_name,
        "kind": kind,
    });

    let resp = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .post(&url)
        .set("Content-Type", "application/json")
        .send_json(body)
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    let json: serde_json::Value = resp.into_json()
        .map_err(|e| format!("failed to parse response: {e}"))?;

    let token = json["token"].as_str()
        .unwrap_or("")
        .to_string();

    Ok(token)
}

use std::io::Read as IoRead;
