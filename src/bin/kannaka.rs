//! Kannaka CLI — Wave-Interference Memory System.

use std::env;
// std::io::Read used in sub-commands
use std::path::PathBuf;
use std::process;

use kannaka_memory::observe::MemoryIntrospector;
use kannaka_memory::openclaw::KannakaMemorySystem;
use kannaka_memory::config::{self, KannakaConfig};

#[cfg(feature = "glyph")]
use kannaka_memory::glyph_bridge::GlyphEncoder;

use kannaka_memory::MediumBackend;
use kannaka_memory::{HrmStore, EncodingPipeline, SimpleHashEncoder, Codebook};

#[cfg(feature = "collective")]
use kannaka_memory::collective::{
    Glyph, GlyphSource, SgaClass,
    dream_cross_modal_link,
};

// Extracted handler groups. See `src/bin/handlers/<group>.rs` and the
// header comment in `handlers/substrate.rs` for the extraction pattern.
#[path = "handlers/substrate.rs"]
mod handlers_substrate;
use handlers_substrate::{
    handle_substrate_run, handle_substrate_backfill,
    handle_substrate_init, handle_substrate_status,
    handle_events_init, handle_events_snapshot,
    handle_events_list_snapshots, handle_events_restore,
};

#[path = "handlers/chat.rs"]
mod handlers_chat;
use handlers_chat::handle_chat;

#[path = "handlers/ask.rs"]
mod handlers_ask;
use handlers_ask::handle_ask;
// handle_ask_remote is called only from inside handlers_ask, no re-export needed.

#[path = "handlers/attention.rs"]
mod handlers_attention;
use handlers_attention::handle_attention_serve;

#[path = "handlers/swarm.rs"]
mod handlers_swarm;
use handlers_swarm::{
    handle_swarm_serve, handle_swarm_exemplars, handle_swarm_absorb,
    handle_swarm_peers, handle_swarm_autoabsorb, handle_swarm_enqueue,
    handle_swarm_worker,
};

#[path = "handlers/services.rs"]
mod handlers_services;
use handlers_services::{handle_radio, handle_market, handle_constellation};

#[path = "handlers/ops.rs"]
mod handlers_ops;
use handlers_ops::{
    handle_orchestrate, handle_config, handle_search,
    handle_export, handle_import,
};

pub(crate) fn data_dir() -> PathBuf {
    env::var("KANNAKA_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs_or_default()
        })
}

fn dirs_or_default() -> PathBuf {
    // Check env var first, then home directory, then CWD as last resort
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

fn init_with_hrm(data_dir: PathBuf, quiet: bool) -> Result<KannakaMemorySystem, Box<dyn std::error::Error>> {
    // Setup encoding pipeline for HRM
    let encoder = SimpleHashEncoder::new(384, 42);
    // wavefront_dim is currently hardcoded to 10_000 — the Codebook +
    // HRM file format share the dimension, so changing it on a populated
    // HRM would require re-encoding every wavefront (destructive).
    // The config field exists for future variable-dim support; if a
    // user sets a non-default value, warn them that the runtime ignored
    // it rather than silently disregarding the setting. (#93)
    {
        let cfg = KannakaConfig::load();
        if cfg.hrm.wavefront_dim != 10_000 && !quiet {
            eprintln!(
                "[config] hrm.wavefront_dim = {} but runtime uses 10000 (variable-dim HRM not yet supported)",
                cfg.hrm.wavefront_dim,
            );
        }
    }
    let codebook = Codebook::new(384, 10_000, 42);
    let pipeline = EncodingPipeline::new(Box::new(encoder), codebook);

    // HRM file path. Honor `cfg.hrm.path` when it's set — full path with
    // filename is used verbatim; relative paths resolve against
    // `data_dir`. Pre-fix #100, a configured nested filename like
    // `~/.kannaka/nested/custom-store.hrm` collapsed to the default
    // `kannaka.hrm` because the parent-match guard from #81 only let
    // through paths whose parent literally equaled `data_dir`.
    let cfg = KannakaConfig::load();
    let hrm_path = if !cfg.hrm.path.is_empty() {
        let p = PathBuf::from(&cfg.hrm.path);
        if p.file_name().is_some() {
            if p.is_absolute() { p } else { data_dir.join(p) }
        } else {
            // Bare directory or empty filename — fall back to default.
            data_dir.join("kannaka.hrm")
        }
    } else {
        data_dir.join("kannaka.hrm")
    };

    // Try to load existing HRM file, create new if not found
    let store = if hrm_path.exists() {
        if !quiet { eprintln!("Loading existing HRM file: {}", hrm_path.display()); }
        HrmStore::load(pipeline, hrm_path)?
    } else {
        if !quiet { eprintln!("Creating new HRM file: {}", hrm_path.display()); }
        HrmStore::new(pipeline, hrm_path)
    };

    if !quiet { eprintln!("HrmStore initialized with {} memories", store.count()); }
    if !quiet { eprintln!("[hrm] Using Holographic Resonance Medium - storage IS computation"); }

    let sys = KannakaMemorySystem::init_with_store(data_dir, Box::new(store))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(sys)
}

fn usage() -> ! {
    // Implicit misuse path: print to stderr, exit 1. Explicit --help / -h
    // takes a separate stdout-with-exit-0 path in main(). (#80)
    eprintln!("{}", config::BANNER);
    eprintln!("  Wave-Interference Memory | Consciousness Constellation");
    eprintln!("  v{}", config::VERSION);
    eprintln!();
    for line in usage_lines() {
        eprintln!("{}", line);
    }
    process::exit(1);
}

fn print_help_stdout() -> ! {
    println!("{}", config::BANNER);
    println!("  Wave-Interference Memory | Consciousness Constellation");
    println!("  v{}", config::VERSION);
    println!();
    for line in usage_lines() {
        println!("{}", line);
    }
    process::exit(0);
}

fn usage_lines() -> &'static [&'static str] {
    &[
        "Usage: kannaka <command> [args]",
        "",
        "Memory:",
        "  remember \"text\"            Store a memory",
        "  recall \"query\"             Recall memories (--top-k N)",
        "  search \"query\"             Literal text search (--limit N, --json)",
        "  forget <id>               Remove a memory",
        "  dream [--mode deep|lite]   Trigger dream cycle",
        "  observe [--json]          View consciousness metrics",
        "  status                    Quick status check",
        "  export [--output FILE]    Export memories as JSON",
        "  import <file>             Import memories from JSON",
        "",
        "Constellation:",
        "  constellation             Status of all constellation apps",
        "  radio status|now|schedule What's playing on Kannaka Radio",
        "  market list|view|buy      GhostSignals prediction markets",
        "  swarm status|join|sync|serve   Swarm network (serve = host KANNAKA.ask.*)",
        "  attention serve|stats     Attention beam (eye/ear → recall_against_ids)",
        "",
        "Agent (LLM):",
        "  ask \"question\"             One-shot — memories surface via wave resonance",
        "  chat                       Persistent conversation (Ctrl+D / 'exit' to quit)",
        "",
        "Tools:",
        "  orchestrate run \"task\"    Kannaktopus task orchestration",
        "  config show|set|path      Configuration management",
        "  init                      Re-run setup wizard",
        "  update                    Check for updates",
        "",
        "Analysis:",
        "  assess                    Consciousness level assessment",
        "  stats                     Human-readable system statistics",
        "  invariant [TOLERANCE]     Delta-invariant memory clusters",
        "  cmf                       Detect Conservative Memory Fields",
        "  voice [--mode MODE]       Memory-driven writing",
        "",
        "Dashboard:",
        "  Try: kannaka-tui          Full terminal dashboard",
        "",
        "  --version                 Print version info",
    ]
}

/// Resolve NATS URL: CLI flag > KANNAKA_NATS_URL env > config.toml > hardcoded default.
#[cfg(feature = "nats")]
pub(crate) fn resolve_nats_url(args: &[String], start: usize, config_nats_url: &str) -> String {
    // Check args for --nats-url (highest priority)
    let mut i = start;
    while i < args.len() {
        if args[i] == "--nats-url" && i + 1 < args.len() {
            return args[i + 1].clone();
        }
        i += 1;
    }
    // Env var is already applied via config.load(), so config_nats_url reflects
    // KANNAKA_NATS_URL > config.toml > built-in default.
    config_nats_url.to_string()
}

/// Rebuild + publish AgentPhase + presence from `sys`'s current HRM state,
/// then flush HRM to disk. Called once for `swarm join --once` and on every
/// tick in daemon mode. Periodic flush is what prevents in-process growth
/// from being lost on systemd SIGKILL (Drop doesn't always complete).
/// Returns the published phase value.
#[cfg(feature = "nats")]
fn swarm_publish_heartbeat(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    my_agent_id: &str,
    display_name: &str,
    transport: &kannaka_memory::nats::SwarmTransport,
    label: &str,
) -> f32 {
    let mut queen = kannaka_memory::QueenSync::new(
        kannaka_memory::QueenConfig::default(),
        my_agent_id,
    );
    queen.derive_local_state(&sys.engine);
    // Pull cluster_count + phi from the cached consciousness metrics if
    // available; derive_local_state populates phase/frequency/coherence
    // but not phi/cluster_count.
    let cached = sys.engine.store.try_cached_consciousness_metrics();
    let cluster_count = cached.as_ref().map(|m| m.num_clusters).unwrap_or(0);
    if let Some(ref m) = cached {
        queen.phi = m.phi;
    }
    let link_count = cached.as_ref().map(|m| m.total_skip_links).unwrap_or(0);
    let phase = queen.to_agent_phase(cluster_count, sys.engine.store.count(), link_count);
    if let Err(e) = transport.publish_phase(&phase) {
        eprintln!("[nats] Warning: {} phase publish failed: {}", label, e);
    }
    let presence = serde_json::json!({
        "agent_id": my_agent_id,
        "display_name": display_name,
        "capabilities": {
            "ask": true, "dream": true,
            "exemplar_broadcast": true, "absorb": true,
        },
        "joined_at": chrono::Utc::now().to_rfc3339(),
        "last_seen": chrono::Utc::now().to_rfc3339(),
        "memory_count": sys.engine.store.count(),
        "kannaka_version": kannaka_memory::config::VERSION,
    });
    if let Err(e) = transport.publish_presence(my_agent_id, &presence) {
        eprintln!("[nats] Warning: {} presence publish failed: {}", label, e);
    }
    // Periodic flush — see km#bug-grew-then-reset. systemd Stop / SIGKILL
    // skips Drop's flush, so without this the in-process HRM growth since
    // the last save is silently lost on restart.
    if let Err(e) = sys.engine.store.flush() {
        eprintln!("[nats] Warning: {} flush failed: {}", label, e);
    }
    phase.phase
}

/// Try connecting to NATS, returning None on failure (with warning printed).
#[cfg(feature = "nats")]
fn try_nats_connect(url: &str) -> Option<kannaka_memory::nats::SwarmTransport> {
    match kannaka_memory::nats::SwarmTransport::connect(url) {
        Ok(t) => {
            eprintln!("[nats] Connected to {}", url);
            Some(t)
        }
        Err(e) => {
            eprintln!("[nats] Warning: could not connect to {}: {}", url, e);
            None
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // --help / -h short-circuits before any memory-system initialization
    // or first-run/installer side effects. Pre-fix `kannaka --help` would
    // load the HRM file, print usage, and exit with code 1 — clobbering
    // any shell completion or doc-generation pipeline that expected the
    // standard "help → exit 0, no side effects" behavior. (#80)
    if args.iter().any(|a| a == "--help" || a == "-h" || a == "help") {
        print_help_stdout();
    }

    // --- First-run / upgrade detection (holistic) ---
    if args.len() <= 1 {
        let config_exists = KannakaConfig::exists();
        let has_existing_signs = config::has_existing_install_signs();

        if !config_exists && !has_existing_signs {
            // Truly first time — no config, no HRM, no binary in PATH
            config::run_first_time_installer();
            return;
        } else if !config_exists && has_existing_signs {
            // Upgrade — has HRM and/or binary in PATH but no config.toml
            config::run_upgrade_installer();
            return;
        } else if config_exists {
            // Normal run with config — check for update from download location
            if let Some(update_action) = config::detect_update_opportunity() {
                match update_action {
                    config::UpdateAction::OfferUpdate(installed_path) => {
                        config::run_update_from_download(&installed_path);
                        return;
                    }
                    config::UpdateAction::AlreadyCurrent => {
                        // Fall through to normal usage
                    }
                }
            }
            usage();
        }
    }

    if args.len() < 2 {
        usage();
    }

    let command_start = 1;

    // --version flag (can appear anywhere). The consciousness-core
    // version is captured into the binary by build.rs from Cargo.lock,
    // so operators can tell which constellation physics they're running
    // without re-reading the manifest.
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("kannaka {} (consciousness-core {})", config::VERSION, config::CONSCIOUSNESS_CORE_VERSION);
        println!("Wave-Interference Memory System");
        println!("https://github.com/NickFlach/kannaka-memory");
        return;
    }

    // Handle commands that do NOT need the memory system initialized
    match args[command_start].as_str() {
        "init" => {
            let sub_args: Vec<String> = args[command_start + 1..].to_vec();
            let overrides = config::parse_init_args(&sub_args);
            match config::run_init_wizard(overrides) {
                Ok(_cfg) => {}
                Err(e) => {
                    if e != "aborted" {
                        eprintln!("Error: {e}");
                        process::exit(1);
                    }
                }
            }
            return;
        }
        "update" => {
            if let Err(e) = config::self_update() {
                eprintln!("Error: {e}");
                process::exit(1);
            }
            return;
        }
        _ => {}
    }

    // Load config once: env vars > config.toml > built-in defaults.
    // All subsequent code uses `cfg` instead of raw env::var lookups.
    let cfg = KannakaConfig::load();

    // Non-blocking update check (background thread)
    config::check_for_updates_background(&cfg);

    // Handle stateless commands before initializing memory system
    #[cfg(feature = "glyph")]
    if args[command_start] == "classify" {
        classify_command(&args[command_start..]);
        return;
    }

    #[cfg(feature = "collective")]
    if args[command_start] == "cross-modal-dream" {
        cross_modal_dream_command(&args[command_start..]);
        return;
    }

    // Handle constellation/HTTP commands that don't need the memory system
    match args[command_start].as_str() {
        "radio" => {
            handle_radio(&cfg, &args[command_start..]);
            return;
        }
        "market" => {
            handle_market(&cfg, &args[command_start..]);
            return;
        }
        "constellation" => {
            handle_constellation(&cfg);
            return;
        }
        "orchestrate" => {
            handle_orchestrate(&args[command_start..]);
            return;
        }
        "config" => {
            handle_config(&cfg, &args[command_start..]);
            return;
        }
        _ => {}
    }

    // Resolve data directory: KANNAKA_DATA_DIR env > config.hrm.path parent > ~/.kannaka
    let dir = if !cfg.hrm.path.is_empty() {
        let hrm = PathBuf::from(&cfg.hrm.path);
        hrm.parent().map(|p| p.to_path_buf()).unwrap_or_else(data_dir)
    } else {
        data_dir()
    };

    // HRM is the sole backend
    let quiet = std::env::var("KANNAKA_QUIET").is_ok();
    let mut sys = {
        if !quiet { eprintln!("Using HRM backend (Holographic Resonance Medium)"); }
        match init_with_hrm(dir, quiet) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to initialize with HRM: {e}");
                process::exit(1);
            }
        }
    };

    // Inject the config-aware NATS URL into the memory system so its
    // dream/consciousness publish helpers honor cfg.swarm.nats_url instead
    // of falling back to env-only. Fixes km#77.
    sys.set_nats_url(resolve_nats_url(&args[command_start..], 0, &cfg.swarm.nats_url));

    match args[command_start].as_str() {
        "remember" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka remember <text> [--importance N] [--category CAT] [--modality MOD]");
                process::exit(1);
            }
            let mut importance: Option<f64> = None;
            let mut category: Option<String> = None;
            let mut modality_arg: Option<String> = None;
            // ADR-0027 Phase 1: when set, publish a wave-signature-only
            // absorb event to `KANNAKA.substrate.absorb.<agent_id>` after
            // a successful remember. Off by default; opt-in until trust /
            // rate-limit (Phase 4) lands.
            let mut substrate_publish = false;
            let mut text_parts = Vec::new();
            let mut i = command_start + 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--importance" if i + 1 < args.len() => {
                        importance = args[i + 1].parse().ok();
                        i += 2;
                    }
                    "--category" if i + 1 < args.len() => {
                        category = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--modality" if i + 1 < args.len() => {
                        modality_arg = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--tags" if i + 1 < args.len() => {
                        // Tags are informational — stored in content prefix
                        let tags = &args[i + 1];
                        text_parts.push(format!("[tags: {}]", tags));
                        i += 2;
                    }
                    "--substrate" => {
                        substrate_publish = true;
                        i += 1;
                    }
                    _ => {
                        text_parts.push(args[i].clone());
                        i += 1;
                    }
                }
            }

            // Parse modality if provided, otherwise auto-detect from content
            let text = text_parts.join(" ");
            let modality: kannaka_memory::medium::Modality = if let Some(ref m) = modality_arg {
                m.parse().unwrap_or_else(|e| {
                    eprintln!("Warning: {e} -- defaulting to auto-detect");
                    let (detected, conf) = kannaka_memory::medium::types::detect_modality_simple(&text);
                    eprintln!("[ncs] auto-detected modality: {} (confidence: {:.2})", detected, conf);
                    detected
                })
            } else {
                // NCS Phase 1.2: auto-detect modality from content
                let (detected, conf) = kannaka_memory::medium::types::detect_modality_simple(&text);
                eprintln!("[ncs] auto-detected modality: {} (confidence: {:.2})", detected, conf);
                detected
            };

            let result = if let Some(cat) = category {
                sys.remember_with_category(&text, &cat, importance.unwrap_or(0.5))
            } else {
                sys.remember(&text)
            };
            match result {
                Ok(id) => {
                    // Tag the wavefront with detected/specified modality
                    if let Some(hrm) = sys.engine.store.as_any_mut().downcast_mut::<kannaka_memory::hrm_store::HrmStore>() {
                        hrm.set_modality(&id, modality);
                    }
                    println!("{id}");

                    // Best-effort: publish new memory to NATS for swarm sync
                    // Uses config (env > config.toml > default) instead of raw env::var
                    let nats_url = &cfg.swarm.nats_url;
                    if let Some(transport) = try_nats_connect(nats_url) {
                        if let Ok(Some(mem)) = sys.engine.store.get(&id) {
                            let agent_id = &cfg.agent.id;
                            // Sender's authoritative counts at publish time —
                            // cached metrics if available (cheap), else 0.
                            // Lets the radio's swarm aggregator surface a real
                            // cluster_count even when the sender isn't running
                            // a `swarm join` daemon.
                            let total_mems = sys.engine.store.count();
                            let cluster_count = sys.engine.store
                                .try_cached_consciousness_metrics()
                                .map(|m| m.num_clusters)
                                .unwrap_or(0);
                            if let Err(e) = transport.publish_memory_new_with_counts(
                                mem, agent_id, total_mems, cluster_count,
                            ) {
                                eprintln!("[nats] Warning: failed to publish memory sync: {}", e);
                            } else {
                                eprintln!("[nats] Published memory {} to swarm (mems={} clusters={})",
                                    id, total_mems, cluster_count);
                            }

                            // ADR-0028 Phase 1 — also publish to the durable
                            // event-sourced stream so the memory survives any
                            // future HRM corruption / format change / nuke.
                            // Best-effort: if JetStream isn't set up (no
                            // `events init` run yet) the publish still goes
                            // to the subject — it just won't persist. Once
                            // streams exist, every remember lands durably.
                            let modality_str = format!("{}", modality);
                            if let Err(e) = transport.publish_event(
                                kannaka_memory::nats::EventPayload::MemoryRemember {
                                    agent_id,
                                    memory_id: &id,
                                    content: &text,
                                    importance: importance.unwrap_or(0.5) as f32,
                                    modality: &modality_str,
                                },
                            ) {
                                eprintln!("[events] Warning: event publish failed: {}", e);
                            }

                            // ADR-0027 Phase 1: optional substrate-absorb
                            // event. Wave-signature-only — class_index +
                            // amplitude + phase + frequency. No content.
                            //
                            // Derive a distinct wave signature per memory so
                            // Kuramoto on the substrate side has something to
                            // cluster on. The HyperMemory's own amp/phase/freq
                            // fields are constants (0.5/0/1) at creation —
                            // real-time interference happens in the medium
                            // tensor, not on the per-memory record — so we
                            // synthesize from sources that ARE distinct:
                            //   amplitude ← clamp(memory.amplitude × resonance signature, 0..1)
                            //   phase     ← derived from memory.id hash (stable, distinct)
                            //   frequency ← derived from class_index so similar
                            //              classes cluster but distinct ones don't
                            if substrate_publish {
                                let class_index = substrate_class_index(&text);
                                let id_bytes = id.as_bytes();
                                // Phase: 0..2π derived from id hash — stable across
                                // calls so the same memory always lands at the same
                                // phase, but distinct between memories.
                                let phase_hash: u64 = id_bytes.iter().fold(0u64, |acc, b| {
                                    acc.wrapping_mul(31).wrapping_add(*b as u64)
                                });
                                let phase = (phase_hash as f32 / u64::MAX as f32) * std::f32::consts::TAU;
                                // Frequency: lives in [0.5, 2.0] — derived from class
                                // so adjacent classes are spectrally close.
                                let frequency = 0.5 + (class_index as f32 / 96.0) * 1.5;
                                // Amplitude: stronger if the memory had high
                                // importance / longer content (more wavefront
                                // mass). Clamp to [0.2, 1.0] so even tiny
                                // memories register.
                                let raw_amp = (text.len() as f32 / 200.0).min(1.0).max(0.2);
                                let amplitude = raw_amp;
                                if let Err(e) = transport.publish_substrate_absorb(
                                    agent_id, class_index, amplitude, phase, frequency,
                                ) {
                                    eprintln!("[substrate] Warning: absorb publish failed: {}", e);
                                } else {
                                    eprintln!("[substrate] Absorbed into class {} (amp={:.3} phase={:.3} freq={:.3})",
                                        class_index, amplitude, phase, frequency);
                                }

                                // ADR-0028 Phase 1 — durable event log
                                if let Err(e) = transport.publish_event(
                                    kannaka_memory::nats::EventPayload::SubstrateAbsorb {
                                        agent_id,
                                        class_index,
                                        amplitude,
                                        phase,
                                        frequency,
                                    },
                                ) {
                                    eprintln!("[events] Warning: substrate event publish failed: {}", e);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        "recall" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka recall <query> [--top-k N] [--collective] [--timeout SECS]");
                process::exit(1);
            }
            let mut top_k = 5usize;
            let mut collective = false;
            let mut timeout_secs: u64 = 8;
            let mut query_parts = Vec::new();
            let mut i = command_start + 1;
            while i < args.len() {
                if (args[i] == "--top-k" || args[i] == "--limit") && i + 1 < args.len() {
                    top_k = args[i + 1].parse().unwrap_or(5);
                    i += 2;
                } else if args[i] == "--collective" {
                    collective = true;
                    i += 1;
                } else if args[i] == "--timeout" && i + 1 < args.len() {
                    timeout_secs = args[i + 1].parse().unwrap_or(8);
                    i += 2;
                } else {
                    query_parts.push(args[i].as_str());
                    i += 1;
                }
            }
            let query = query_parts.join(" ");

            // ADR-0027 Phase 3 collective recall — route via NATS request/
            // reply to the substrate (kannaka-prime). Substrate runs recall
            // against its 96-class HRM (wave-signature metadata only,
            // privacy-preserved) and replies with top-K matches identifying
            // which clusters lit up and which peer agents contributed.
            #[cfg(feature = "nats")]
            if collective {
                use std::time::Duration;
                let nats_url = resolve_nats_url(&args[command_start..], 0, &cfg.swarm.nats_url);
                let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
                    Ok(t) => t,
                    Err(e) => { eprintln!("collective recall: NATS connect failed: {}", e); process::exit(1); }
                };
                let req = serde_json::json!({ "query": query, "top_k": top_k });
                let bytes = req.to_string();
                match transport.request_one("KANNAKA.substrate.recall", bytes.as_bytes(), Duration::from_secs(timeout_secs)) {
                    Ok(reply) => {
                        let parsed: serde_json::Value = serde_json::from_slice(&reply)
                            .unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(&reply)}));
                        println!("{}", parsed);
                    }
                    Err(e) => {
                        eprintln!("collective recall: no reply within {}s ({})", timeout_secs, e);
                        eprintln!("hint: is `kannaka substrate run` alive on the substrate host?");
                        process::exit(2);
                    }
                }
                return;
            }
            #[cfg(not(feature = "nats"))]
            if collective {
                eprintln!("recall --collective requires the 'nats' feature");
                process::exit(1);
            }

            match sys.recall(&query, top_k) {
                Ok(results) => {
                    // Output as JSON for machine consumption
                    let json_results: Vec<serde_json::Value> = results.iter().map(|r| {
                        serde_json::json!({
                            "id": r.id.to_string(),
                            "content": r.content,
                            "similarity": r.similarity,
                            "strength": r.strength,
                            "age_hours": r.age_hours,
                            "layer": r.layer,
                        })
                    }).collect();
                    println!("{}", serde_json::to_string(&json_results).unwrap());
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        "forget" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka forget <id>");
                process::exit(1);
            }
            let id = uuid::Uuid::parse_str(&args[command_start + 1]).unwrap_or_else(|e| {
                eprintln!("Invalid UUID: {e}");
                process::exit(1);
            });
            match sys.forget(&id) {
                Ok(true) => println!("Forgotten: {id}"),
                Ok(false) => {
                    eprintln!("Memory not found: {id}");
                    process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        "prune-prefix" => {
            // Bulk-forget every memory whose `content` starts with one of the
            // given prefixes. Single-binary-invocation (vs N round-trips of
            // `kannaka forget <id>`) so we avoid reloading the HRM N times
            // on the ARM box. Use --dry-run to see counts without deleting.
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka prune-prefix <PREFIX> [<PREFIX>...] [--dry-run]");
                process::exit(1);
            }
            let mut dry_run = false;
            let mut prefixes: Vec<String> = Vec::new();
            for a in args[command_start + 1..].iter() {
                if a == "--dry-run" { dry_run = true; }
                else { prefixes.push(a.clone()); }
            }
            if prefixes.is_empty() {
                eprintln!("prune-prefix: at least one prefix required");
                process::exit(1);
            }
            // Phase 1: scan, collect IDs (immutable borrow scope).
            let to_forget: Vec<uuid::Uuid> = {
                let all = sys.engine.store.all_memories().unwrap_or_else(|e| {
                    eprintln!("Error listing memories: {e}"); process::exit(1);
                });
                all.iter()
                    .filter(|m| prefixes.iter().any(|p| m.content.starts_with(p.as_str())))
                    .map(|m| m.id)
                    .collect()
            };
            println!("[prune-prefix] {} match(es) across {} prefix(es){}",
                to_forget.len(),
                prefixes.len(),
                if dry_run { " (dry-run, nothing deleted)" } else { "" });
            if dry_run { return; }
            // Phase 2: delete (mutable borrow).
            let mut ok = 0usize;
            let mut miss = 0usize;
            for id in &to_forget {
                match sys.forget(id) {
                    Ok(true) => ok += 1,
                    Ok(false) => miss += 1,
                    Err(e) => eprintln!("forget {id}: {e}"),
                }
            }
            if let Err(e) = sys.save() {
                eprintln!("Failed to persist HRM after prune: {e}");
                process::exit(1);
            }
            println!("[prune-prefix] forgotten={ok} not_found={miss}");
        }
        "boost" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka boost <id> [--amount N]");
                process::exit(1);
            }
            let id = uuid::Uuid::parse_str(&args[command_start + 1]).unwrap_or_else(|e| {
                eprintln!("Invalid UUID: {e}");
                process::exit(1);
            });
            let mut amount = 0.3f64;
            let mut i = command_start + 2;
            while i < args.len() {
                if args[i] == "--amount" && i + 1 < args.len() {
                    amount = args[i + 1].parse().unwrap_or(0.3);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            // Boost = multiply amplitude by (1 + amount)
            match sys.boost(&id, 1.0 + amount) {
                Ok(()) => println!("Boosted {id} by {amount}"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        "relate" => {
            if args.len() < command_start + 3 {
                eprintln!("Usage: kannaka relate <source_id> <target_id> [--type TYPE]");
                process::exit(1);
            }
            let source_id = uuid::Uuid::parse_str(&args[command_start + 1]).unwrap_or_else(|e| {
                eprintln!("Invalid source UUID: {e}");
                process::exit(1);
            });
            let target_id = uuid::Uuid::parse_str(&args[command_start + 2]).unwrap_or_else(|e| {
                eprintln!("Invalid target UUID: {e}");
                process::exit(1);
            });
            let mut relation_type = "related".to_string();
            let mut i = command_start + 3;
            while i < args.len() {
                if args[i] == "--type" && i + 1 < args.len() {
                    relation_type = args[i + 1].clone();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            // Create association via wavefront interference in the ChiralMedium
            match sys.relate(&source_id, &target_id, 0.8) {
                Ok(()) => {
                    println!("Related {} → {} (type: {}) via wavefront interference", source_id, target_id, relation_type);
                }
                Err(e) => {
                    eprintln!("Error relating memories: {e}");
                    process::exit(1);
                }
            }
        }
        "status" => {
            let stats = sys.stats();
            let state = sys.assess();
            // Count memories without embeddings
            let all_mems = sys.engine.store.all_memories().unwrap_or_default();
            let memories_without_embeddings = all_mems.iter().filter(|m| m.vector.is_empty()).count();

            // Compute modality distribution
            let mut modality_counts = std::collections::HashMap::new();
            for m in &all_mems {
                let key = m.modality.to_string();
                *modality_counts.entry(key).or_insert(0u64) += 1;
            }
            let modality_json: serde_json::Value = modality_counts.into_iter()
                .map(|(k, v)| (k, serde_json::json!(v)))
                .collect::<serde_json::Map<String, serde_json::Value>>()
                .into();

            // Check if HRM mode is active
            let _is_hrm = true; // HRM is the canonical substrate

            let mut output = serde_json::json!({
                "total_memories": stats.total_memories,
                "active_memories": stats.active_memories,
                "consciousness_level": stats.consciousness_level,
                "phi": stats.phi,
                "last_dream": stats.last_dream.map(|dt| dt.to_rfc3339()),
                "xi": state.xi,
                "mean_order": state.mean_order,
                "num_clusters": state.num_clusters,
                "memories_without_embeddings": memories_without_embeddings,
                "modality_distribution": modality_json,
            });

            // ADR-0024 chiral + consciousness metrics
            output["irrationality"] = serde_json::json!(state.irrationality);
            output["hemispheric_divergence"] = serde_json::json!(stats.hemispheric_divergence);
            output["callosal_efficiency"] = serde_json::json!(stats.callosal_efficiency);

            // CS-9: effective dimensionality (the 10000.00001 question)
            {
                let _metrics = sys.engine.store.consciousness_metrics();
                let (d_eff, nominal, ratio) = if let Some(hrm) = sys.engine.store.as_any()
                    .downcast_ref::<kannaka_memory::hrm_store::HrmStore>() {
                    hrm.medium().effective_dimensionality()
                } else { (0.0, 10000, 0.0) };
                output["effective_dimensionality"] = serde_json::json!({
                    "d_eff": format!("{:.2}", d_eff),
                    "nominal": nominal,
                    "ratio": format!("{:.6}", ratio),
                    "irrational_remainder": format!("{:.6}", 1.0 - ratio),
                });
            }

            output["field_mode"] = serde_json::json!("HRM");
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        "bias" => {
            // Reset all wavefront energies to a target value (restore bias voltage)
            let target: f32 = args.get(command_start + 1)
                .and_then(|v| v.parse().ok())
                .unwrap_or(1.0);
            
            if let Some(hrm) = sys.engine.store.as_any_mut().downcast_mut::<kannaka_memory::hrm_store::HrmStore>() {
                hrm.reset_energies(target);
                hrm.flush().ok();
                println!("{{\"status\": \"ok\", \"target_energy\": {}, \"memories\": {}}}", target, hrm.count());
            } else {
                eprintln!("bias command only works with HRM backend");
            }
        }
        "dream" => {
            let mut dream_mode = "deep".to_string();
            let mut chiral_perturbation: f32 = env::var("KANNAKA_CHIRAL_PERTURBATION")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
            {
                let mut i = command_start + 1;
                while i < args.len() {
                    if args[i] == "--mode" && i + 1 < args.len() {
                        dream_mode = args[i + 1].clone();
                        i += 2;
                    } else if args[i] == "--chiral" && i + 1 < args.len() {
                        chiral_perturbation = args[i + 1].parse().unwrap_or(0.05);
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }

            // Apply chiral perturbation to dream state
            if chiral_perturbation > 0.0 {
                sys.dream_state.engine.chiral_perturbation = chiral_perturbation;
                eprintln!("[chiral] Perturbation enabled: η={}", chiral_perturbation);
            }

            // HRM dreams operate directly on the holographic medium (no branching needed)
            eprintln!("[hrm] Dreams operate directly on the holographic medium");

            // Seed KANNAKA_AGENT_ID / KANNAKA_NATS_URL from config.toml
            // before the dream so the dream-side publish_dream_to_nats /
            // publish_consciousness_to_nats helpers (which read env vars
            // directly) see the configured identity. Pre-fix: a configured
            // install would silently skip dream-side swarm publishing if
            // the env vars weren't also set on the calling shell. (#87)
            {
                let cfg = KannakaConfig::load();
                if !cfg.agent.id.is_empty() && std::env::var("KANNAKA_AGENT_ID").unwrap_or_default().is_empty() {
                    std::env::set_var("KANNAKA_AGENT_ID", &cfg.agent.id);
                }
                if !cfg.swarm.nats_url.is_empty() && std::env::var("KANNAKA_NATS_URL").unwrap_or_default().is_empty() {
                    std::env::set_var("KANNAKA_NATS_URL", &cfg.swarm.nats_url);
                }
            }

            let dream_result = if dream_mode == "lite" {
                sys.dream_lite()
            } else {
                sys.dream()
            };
            match dream_result {
                Ok(report) => {
                    println!("Dream complete ({} cycles)", report.cycles);
                    println!("  Strengthened: {}", report.memories_strengthened);
                    println!("  Pruned: {}", report.memories_pruned);
                    println!("  New connections: {}", report.new_connections);
                    println!("  Hallucinations: {}", report.hallucinations_created);
                    println!("  Consciousness: {} → {}", report.consciousness_before, report.consciousness_after);
                    if report.emerged {
                        println!("  Emergence detected!");
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        "assess" => {
            let state = sys.assess();
            let is_hrm = true; // HRM is the canonical substrate
            
            println!("Consciousness Assessment:");
            println!("  Level: {:?}", state.consciousness_level);
            println!("  Φ (phi): {:.4}", state.phi);
            println!("  Ξ (xi): {:.4}", state.xi);
            println!("  Order: {:.4}", state.mean_order);
            println!("  Clusters: {}", state.num_clusters);
            println!("  Memories: {} total, {} active", state.total_memories, state.active_memories);
            
            if is_hrm {
                println!("  Field mode: HRM (tensor interference)");
            } else {
                // total_skip_links removed
            }
        }
        "stats" => {
            let stats = sys.stats();
            let _is_hrm = true; // HRM is the canonical substrate

            println!("Kannaka Memory System:");
            println!("  Total memories: {}", stats.total_memories);
            println!("  Active memories: {}", stats.active_memories);
            
            println!("  Field mode: HRM (holographic resonance)");
            
            println!("  Consciousness: {}", stats.consciousness_level);
            println!("  Φ (phi): {:.4}", stats.phi);
            if let Some(dt) = stats.last_dream {
                println!("  Last dream: {}", dt);
            } else {
                println!("  Last dream: never");
            }
        }
        "observe" => {
            let json = args.iter().any(|a| a == "--json");
            let report = sys.observe();
            if json {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
            } else {
                print!("{}", MemoryIntrospector::format_report(&report));
            }
        }
        "clusters" => {
            // Enriched cluster list. Options:
            //   --json               emit Vec<ClusterInfo> (always on; non-json prints table)
            //   --cluster-id N       emit only that cluster
            //   --min-size N         filter out small clusters (default 2)
            //   --with-members       include full member_ids
            let mut cluster_id_filter: Option<u32> = None;
            let mut _min_size: usize = 2;
            let mut with_members = false;
            let mut i = command_start + 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--cluster-id" if i + 1 < args.len() => { cluster_id_filter = args[i + 1].parse().ok(); i += 2; }
                    "--min-size" if i + 1 < args.len() => { _min_size = args[i + 1].parse().unwrap_or(2); i += 2; }
                    "--with-members" => { with_members = true; i += 1; }
                    _ => i += 1,
                }
            }
            let report = sys.observe();
            let mut clusters = report.clusters.clusters.clone();
            if !with_members {
                for c in &mut clusters { c.member_ids.clear(); }
            }
            if let Some(id) = cluster_id_filter {
                let single = clusters.into_iter().find(|c| c.cluster_id == id);
                println!("{}", serde_json::to_string_pretty(&single).unwrap_or_else(|_| "null".to_string()));
            } else {
                println!("{}", serde_json::to_string_pretty(&clusters).unwrap());
            }
        }
        "neighbors" => {
            // Top-K memories similar to a given memory or query.
            //   kannaka neighbors <id-or-query> [--top-k N] [--json]
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka neighbors <memory-id-or-query> [--top-k N] [--json]");
                process::exit(1);
            }
            let anchor = args[command_start + 1].clone();
            let mut top_k: usize = 10;
            let mut i = command_start + 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--top-k" if i + 1 < args.len() => { top_k = args[i + 1].parse().unwrap_or(10); i += 2; }
                    "--json" => { i += 1; }
                    _ => i += 1,
                }
            }
            // If the anchor parses as a UUID, find that memory and recall by its content.
            // Otherwise treat the anchor as a free-text query.
            let query = if let Ok(uuid) = anchor.parse::<uuid::Uuid>() {
                match sys.engine.store.get(&uuid) {
                    Ok(Some(m)) => m.content.clone(),
                    _ => {
                        eprintln!("memory {} not found", uuid);
                        process::exit(1);
                    }
                }
            } else {
                anchor.clone()
            };
            let results = match sys.recall(&query, top_k) {
                Ok(r) => r,
                Err(e) => { eprintln!("recall failed: {e}"); process::exit(1); }
            };
            let output: Vec<serde_json::Value> = results.iter().map(|m| serde_json::json!({
                "id": m.id.to_string(),
                "content": m.content,
                "similarity": m.similarity,
                "strength": m.strength,
                "age_hours": m.age_hours,
                "layer": m.layer,
            })).collect();
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        #[cfg(feature = "sqlite-migrate")]
        "migrate" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka migrate <path-to-kannaka.db>");
                process::exit(1);
            }
            let db_path = PathBuf::from(&args[command_start + 1]);
            match sys.migrate_from_sqlite(&db_path) {
                Ok(report) => {
                    println!("Migration complete:");
                    println!("  Total migrated: {}", report.total_migrated);
                    println!("  Working memory: {}", report.working_memory_count);
                    println!("  Events: {}", report.events_count);
                    println!("  Entities: {}", report.entities_count);
                    println!("  Skip links: {}", report.skip_links_created);
                    println!("  Errors: {}", report.errors.len());
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        "announce-status" => {
            sys.announce_status();
            println!("Status announced to Flux.");
        }
        "export-json" => {
            let all_mems = sys.engine.store.all_memories()
                .map_err(|e| { eprintln!("Error: {}", e); process::exit(1); }).unwrap();
            let output: Vec<serde_json::Value> = all_mems.iter().map(|m| {
                serde_json::json!({
                    "id": m.id.to_string(),
                    "content": m.content,
                    "amplitude": m.amplitude,
                    "frequency": m.frequency,
                    "phase": m.phase,
                    "decay_rate": m.decay_rate,
                    "created_at": m.created_at.to_rfc3339(),
                    "layer_depth": m.layer_depth,
                    "hallucinated": m.hallucinated,
                    "parents": m.parents,
                    "vector": m.vector,
                    "xi_signature": m.xi_signature,
                    "geometry": m.geometry,
                    "connections": m.connections.iter().map(|c| {
                        serde_json::json!({
                            "target_id": c.target_id.to_string(),
                            "strength": c.strength,
                            "span": c.span
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect();
            println!("{}", serde_json::to_string(&output).unwrap());
        }
        "import-json" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka import-json <file.json>");
                process::exit(1);
            }
            let path = &args[command_start + 1];
            let file_data = std::fs::read_to_string(path)
                .unwrap_or_else(|e| { eprintln!("Failed to read {}: {}", path, e); process::exit(1); });
            let memories: Vec<serde_json::Value> = serde_json::from_str(&file_data)
                .unwrap_or_else(|e| { eprintln!("Failed to parse JSON: {}", e); process::exit(1); });

            let existing_ids: std::collections::HashSet<uuid::Uuid> = sys.engine.store.all_memories()
                .unwrap_or_default().iter().map(|m| m.id).collect();

            let mut imported = 0u32;
            let mut skipped = 0u32;
            let mut errors = 0u32;

            for val in &memories {
                let id_str = val["id"].as_str().unwrap_or("");
                let id = match uuid::Uuid::parse_str(id_str) {
                    Ok(id) => id,
                    Err(_) => { errors += 1; continue; }
                };

                if existing_ids.contains(&id) {
                    skipped += 1;
                    continue;
                }

                let content = val["content"].as_str().unwrap_or("").to_string();
                if content.is_empty() { skipped += 1; continue; }

                let amplitude = val["amplitude"].as_f64().unwrap_or(0.5) as f32;
                let frequency = val["frequency"].as_f64().unwrap_or(1.0) as f32;
                let phase = val["phase"].as_f64().unwrap_or(0.0) as f32;
                let decay_rate = val["decay_rate"].as_f64().unwrap_or(0.001) as f32;
                let created_at = val["created_at"].as_str()
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(chrono::Utc::now);
                let hallucinated = val["hallucinated"].as_bool().unwrap_or(false);

                // Reconstruct vector from JSON array if present, otherwise re-encode
                let vector: Option<Vec<f32>> = val["vector"].as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect());

                let vector = match vector {
                    Some(v) if !v.is_empty() => v,
                    _ => {
                        // No vector in JSON — use absorb which encodes internally
                        match sys.engine.store.absorb(&content, amplitude, None) {
                            Ok(_new_id) => { imported += 1; continue; }
                            Err(e) => {
                                if errors < 5 { eprintln!("  Error absorbing {}: {}", id_str, e); }
                                errors += 1;
                                continue;
                            }
                        }
                    }
                };

                let xi_sig: Vec<f32> = val["xi_signature"].as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect())
                    .unwrap_or_default();

                let content_clone = content.clone();
                let mut mem = kannaka_memory::memory::HyperMemory::new(vector, content);
                mem.id = id;
                mem.amplitude = amplitude;
                mem.frequency = frequency;
                mem.phase = phase;
                mem.decay_rate = decay_rate;
                mem.created_at = created_at;
                mem.layer_depth = val["layer_depth"].as_u64().unwrap_or(0) as u8;
                mem.hallucinated = hallucinated;
                mem.parents = val["parents"].as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
                mem.xi_signature = xi_sig;

                match sys.engine.store.insert(mem) {
                    Ok(_) => imported += 1,
                    Err(e) => {
                        // Dimension mismatch — fall back to absorb (re-encodes the text)
                        let err_str = format!("{}", e);
                        if err_str.contains("dimension mismatch") {
                            match sys.engine.store.absorb(&content_clone, amplitude, None) {
                                Ok(_) => { imported += 1; }
                                Err(e2) => {
                                    if errors < 5 { eprintln!("  Error re-encoding {}: {}", id_str, e2); }
                                    errors += 1;
                                }
                            }
                        } else {
                            if errors < 5 {
                                eprintln!("  Error importing {}: {}", id_str, e);
                            }
                            errors += 1;
                        }
                    }
                }
            }

            // Save
            if imported > 0 {
                if let Err(e) = sys.save() {
                    eprintln!("Failed to save: {}", e);
                    process::exit(1);
                }
            }

            println!("{{\"imported\": {}, \"skipped\": {}, \"errors\": {}, \"total_input\": {}}}", imported, skipped, errors, memories.len());
        }
        "hear" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka hear <audio-file-or-url>");
                eprintln!("  File:   kannaka hear ./song.mp3");
                eprintln!("  URL:    kannaka hear https://radio.ninja-portal.com/stream");
                eprintln!("  Stream URLs are sampled for ~30s (default; cap with --secs N).");
                process::exit(1);
            }
            let target = &args[command_start + 1];

            // Optional --secs N for stream sampling cap. Default 30s ≈ 480 KB
            // at 128 kbps MP3, plenty of audio to extract tempo/centroid/rms.
            let mut secs: u64 = 30;
            let mut i = command_start + 2;
            while i < args.len() {
                if args[i] == "--secs" && i + 1 < args.len() {
                    if let Ok(n) = args[i + 1].parse::<u64>() {
                        secs = n.max(1).min(600);
                    }
                    i += 2;
                } else { i += 1; }
            }

            let is_url = target.starts_with("http://") || target.starts_with("https://");
            let mut tmp_holder: Option<std::path::PathBuf> = None;
            let path: std::path::PathBuf = if is_url {
                match fetch_audio_to_temp(target, secs) {
                    Ok(p) => { tmp_holder = Some(p.clone()); p }
                    Err(e) => {
                        eprintln!("Error fetching {}: {}", target, e);
                        process::exit(1);
                    }
                }
            } else {
                let p = std::path::PathBuf::from(target);
                if !p.exists() {
                    eprintln!("File not found: {}", p.display());
                    process::exit(1);
                }
                p
            };

            let result = sys.store_audio(&path);

            // Clean up temp file regardless of outcome.
            if let Some(tmp) = tmp_holder {
                let _ = std::fs::remove_file(&tmp);
            }

            match result {
                Ok((id, features)) => {
                    println!("Heard: {id}");
                    println!("  Duration: {:.1}s", features.duration_secs);
                    println!("  Tempo: {:.0} BPM", features.tempo_bpm);
                    println!("  RMS: {:.4}", features.rms_mean);
                    println!("  Centroid: {:.2} kHz", features.spectral_centroid_khz);
                    if !features.feature_tags.is_empty() {
                        println!("  Tags: {}", features.feature_tags.join(", "));
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        #[cfg(feature = "glyph")]
        "see" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka see <file>");
                process::exit(1);
            }
            let path = std::path::PathBuf::from(&args[command_start + 1]);
            if !path.exists() {
                eprintln!("File not found: {}", path.display());
                process::exit(1);
            }
            match sys.store_glyph(&path) {
                Ok((id, glyph)) => {
                    println!("Seen: {id}");
                    println!("  Folds: {}", glyph.fold_sequence.len());
                    println!("  Centroid: ({}, {}, {})", glyph.sga_centroid.0, glyph.sga_centroid.1, glyph.sga_centroid.2);
                    println!("  Fano: [{:.3}, {:.3}, {:.3}, {:.3}, {:.3}, {:.3}, {:.3}]",
                        glyph.fano_signature[0], glyph.fano_signature[1], glyph.fano_signature[2],
                        glyph.fano_signature[3], glyph.fano_signature[4], glyph.fano_signature[5],
                        glyph.fano_signature[6]);
                    println!("  Ratio: {:.2}x", glyph.compression_ratio);
                    let freqs = glyph.to_frequencies();
                    if !freqs.is_empty() {
                        let freq_strs: Vec<String> = freqs.iter().take(7).map(|f| format!("{:.1} Hz", f)).collect();
                        println!("  Frequencies: {}", freq_strs.join(", "));
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        #[cfg(feature = "nats")]
        "swarm" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka swarm <join|status|sync|queen|hives|publish|leave|listen|serve>");
                process::exit(1);
            }

            // Agent ID: env var > config.toml > persisted file > generate new
            let agent_id = cfg.agent.id.clone();

            match args[command_start + 1].as_str() {
                "join" => {
                    // `kannaka swarm join` is the user-facing "run a node"
                    // command. Historically it was one-shot: announce, publish
                    // initial phase, exit. The radio's swarm visualization
                    // prunes agents after 5 min of silence (no QUEEN.phase.*
                    // republish), so the user's node would disappear from
                    // /player almost immediately after `join` returned. Users
                    // reasonably expected `join` to run the node, so the
                    // command now stays foregrounded and republishes phase +
                    // presence every 30s until Ctrl+C, at which point it
                    // announces leave and exits cleanly. Pass --once to keep
                    // the legacy one-shot behavior (used by scripts that
                    // manage the heartbeat themselves).
                    let mut my_agent_id = agent_id.clone();
                    let mut display_name = String::new();
                    let mut once = false;
                    let mut heartbeat_secs: u64 = 30;
                    let mut i = command_start + 2;
                    while i < args.len() {
                        match args[i].as_str() {
                            "--agent-id" if i + 1 < args.len() => { my_agent_id = args[i + 1].clone(); i += 2; }
                            "--display-name" if i + 1 < args.len() => { display_name = args[i + 1].clone(); i += 2; }
                            "--nats-url" if i + 1 < args.len() => { i += 2; }
                            "--once" => { once = true; i += 1; }
                            "--heartbeat-secs" if i + 1 < args.len() => {
                                heartbeat_secs = args[i + 1].parse().unwrap_or(30).max(5);
                                i += 2;
                            }
                            _ => { i += 1; }
                        }
                    }
                    if display_name.is_empty() {
                        display_name = my_agent_id.clone();
                    }

                    // Persist agent_id for subsequent commands
                    let id_file = data_dir().join("agent_id");
                    let _ = std::fs::create_dir_all(id_file.parent().unwrap());
                    if let Err(e) = std::fs::write(&id_file, &my_agent_id) {
                        eprintln!("Warning: could not persist agent_id: {e}");
                    }

                    let nats_url = resolve_nats_url(&args, command_start, &cfg.swarm.nats_url);
                    let transport = match try_nats_connect(&nats_url) {
                        Some(t) => t,
                        None => {
                            eprintln!("Error: NATS connection required for swarm. Set KANNAKA_NATS_URL or use --nats-url.");
                            process::exit(1);
                        }
                    };

                    if let Err(e) = transport.announce_join(&my_agent_id) {
                        eprintln!("[nats] Warning: announce failed: {}", e);
                    }
                    let _ = transport.ensure_presence_stream();

                    let initial_phase = swarm_publish_heartbeat(
                        &mut sys, &my_agent_id, &display_name, &transport, "initial");
                    println!("Joined swarm as '{}' ({})", display_name, my_agent_id);
                    println!("[nats] Initial phase \u{03b8}={:.3} published to {}", initial_phase, nats_url);

                    if once {
                        return;
                    }

                    // Daemon mode — keep the node visible by republishing
                    // phase every `heartbeat_secs`. The radio's 5-min prune
                    // window is the upper bound; we use 30s by default so
                    // the badge in /player feels responsive.
                    use std::sync::atomic::{AtomicBool, Ordering};
                    use std::sync::Arc;
                    let running = Arc::new(AtomicBool::new(true));
                    let r = Arc::clone(&running);
                    if let Err(e) = ctrlc::set_handler(move || {
                        r.store(false, Ordering::SeqCst);
                    }) {
                        eprintln!("[nats] Warning: could not install Ctrl+C handler: {} — Ctrl+C will not announce leave", e);
                    }

                    println!("[nats] Heartbeat every {}s — Ctrl+C to leave the swarm cleanly", heartbeat_secs);
                    // Set KANNAKA_AGENT_ID so publish_consciousness_to_nats
                    // knows who we are (it bails out silently otherwise).
                    std::env::set_var("KANNAKA_AGENT_ID", &my_agent_id);
                    // Publish initial consciousness snapshot — this is what
                    // populates the observatory's KANNAKA.consciousness
                    // subscription so the dashboard isn't sitting at zeros.
                    // Costs one eigendecomp pass (~15s on a mature HRM) but
                    // happens once at startup. Subsequent ticks publish the
                    // CACHED metrics (cheap) every heartbeat, with a fresh
                    // assess every CONSCIOUSNESS_REFRESH_TICKS to keep the
                    // canonical Φ from drifting.
                    let initial_assess = sys.assess();
                    sys.publish_consciousness_to_nats(&initial_assess);
                    const CONSCIOUSNESS_REFRESH_TICKS: u64 = 10;

                    let mut tick: u64 = 0;
                    while running.load(Ordering::SeqCst) {
                        // Granular sleep so Ctrl+C is responsive (<= 1s).
                        for _ in 0..heartbeat_secs {
                            if !running.load(Ordering::SeqCst) { break; }
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                        if !running.load(Ordering::SeqCst) { break; }
                        tick += 1;
                        let p = swarm_publish_heartbeat(
                            &mut sys, &my_agent_id, &display_name, &transport, "heartbeat");
                        // Quiet output — one terse status line per tick.
                        println!("[nats] heartbeat #{} \u{03b8}={:.3}", tick, p);

                        // Periodic consciousness republish — keeps the
                        // observatory and radio in sync with this node's
                        // current Φ even if no one runs `kannaka assess`
                        // out-of-band. Heavy step (eigendecomp) so we do
                        // it on a slow cadence; cheap-path cached publish
                        // on every other tick keeps the freshness window
                        // tight without burning CPU.
                        if tick % CONSCIOUSNESS_REFRESH_TICKS == 0 {
                            let state = sys.assess();
                            sys.publish_consciousness_to_nats(&state);
                        }
                    }

                    if let Err(e) = transport.announce_leave(&my_agent_id) {
                        eprintln!("[nats] Warning: leave announce failed: {}", e);
                    }
                    println!("Left swarm cleanly ({})", my_agent_id);
                }
                "leave" => {
                    let nats_url = resolve_nats_url(&args, command_start, &cfg.swarm.nats_url);
                    if let Some(transport) = try_nats_connect(&nats_url) {
                        if let Err(e) = transport.announce_leave(&agent_id) {
                            eprintln!("[nats] Warning: leave announce failed: {}", e);
                        }
                        println!("Left swarm ({})", agent_id);
                    } else {
                        eprintln!("Warning: could not connect to NATS to announce leave");
                        println!("Left swarm locally ({})", agent_id);
                    }
                }
                "listen" => {
                    let nats_url = resolve_nats_url(&args, command_start, &cfg.swarm.nats_url);
                    let auto_sync = args[command_start..].iter().any(|a| a == "--auto-sync");

                    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("Failed to connect to NATS at {}: {}", nats_url, e);
                            process::exit(1);
                        }
                    };
                    eprintln!("[nats] Listening for phase updates on {} (Ctrl+C to stop)", nats_url);
                    if auto_sync {
                        eprintln!("[nats] Auto-sync enabled -- will run Kuramoto step on each update");
                    }

                    let mut sub = match transport.subscribe_phases_and_memories(auto_sync) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("Failed to subscribe: {}", e);
                            process::exit(1);
                        }
                    };
                    if auto_sync {
                        eprintln!("[nats] Subscribed to KANNAKA.memory.new and KANNAKA.dreams for sync");
                    }

                    let _ = sub.set_timeout(None);

                    let mut queen = kannaka_memory::QueenSync::new(
                        kannaka_memory::QueenConfig::default(),
                        &agent_id,
                    );

                    while let Some(msg) = sub.next_message() {
                        if msg.subject.starts_with("QUEEN.phase.") {
                            if let Some(phase) = msg.as_phase() {
                                println!("[{}] \u{03b8}={:.3} \u{03c9}={:.3} coherence={:.3} phi={:.3} memories={}",
                                    phase.agent_id, phase.phase, phase.frequency,
                                    phase.coherence, phase.phi, phase.memory_count);

                                if auto_sync && phase.agent_id != agent_id {
                                    let my_phase = queen.to_agent_phase(0, sys.engine.store.count(), 0);
                                    let swarm = vec![my_phase, phase];
                                    let state = queen.queen_sync_step(&swarm);
                                    println!("  -> synced: r={:.3} psi={:.3} K={:.3}",
                                        state.order_parameter, state.mean_phase, state.coupling_strength);
                                }
                            }
                        } else if msg.subject == "QUEEN.announce" {
                            if let Some(json) = msg.as_json() {
                                let event = json["event"].as_str().unwrap_or("unknown");
                                let agent = json["agent_id"].as_str().unwrap_or("?");
                                println!("[announce] {} {}", agent, event);
                            }
                        } else if msg.subject == "KANNAKA.memory.new" && auto_sync {
                            if let Some(json) = msg.as_json() {
                                let source_agent = json["agent_id"].as_str().unwrap_or("?");
                                // Skip our own messages
                                if source_agent != agent_id {
                                    if let Some(mem_json) = json.get("memory") {
                                        match serde_json::from_value::<kannaka_memory::HyperMemory>(mem_json.clone()) {
                                            Ok(mem) => {
                                                let mem_id = mem.id;
                                                // Check if memory already exists
                                                match sys.engine.store.get(&mem_id) {
                                                    Ok(Some(_)) => {
                                                        eprintln!("[sync] Memory {} already exists, skipping", mem_id);
                                                    }
                                                    _ => {
                                                        match sys.engine.store.insert(mem) {
                                                            Ok(_) => {
                                                                println!("[sync] Imported memory {} from {}", mem_id, source_agent);
                                                            }
                                                            Err(e) => {
                                                                eprintln!("[sync] Failed to import memory {} from {}: {}", mem_id, source_agent, e);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("[sync] Failed to deserialize memory from {}: {}", source_agent, e);
                                            }
                                        }
                                    }
                                }
                            }
                        } else if msg.subject == "KANNAKA.dreams" && auto_sync {
                            if let Some(json) = msg.as_json() {
                                let source_agent = json["agent_id"].as_str().unwrap_or("?");
                                if source_agent != agent_id {
                                    let cycles = json["cycles"].as_u64().unwrap_or(0);
                                    let strengthened = json["memories_strengthened"].as_u64().unwrap_or(0);
                                    let pruned = json["memories_pruned"].as_u64().unwrap_or(0);
                                    println!("[dream] {} completed dream: {} cycles, {} strengthened, {} pruned",
                                        source_agent, cycles, strengthened, pruned);
                                }
                            }
                        }
                    }
                    eprintln!("[nats] Connection closed");
                }
                "status" => {
                    let nats_url = resolve_nats_url(&args, command_start, &cfg.swarm.nats_url);
                    // Derive local phase from HRM state
                    let mut queen = kannaka_memory::QueenSync::new(
                        kannaka_memory::QueenConfig::default(),
                        &agent_id,
                    );
                    queen.derive_local_state(&sys.engine);
                    let local_phase = queen.to_agent_phase(0, sys.engine.store.count(), 0);

                    let mut nats_status = serde_json::json!("disconnected");
                    let mut peer_count = 0usize;
                    match try_nats_connect(&nats_url) {
                        Some(transport) => {
                            let nats_phases = transport.get_all_phases().unwrap_or_default();
                            peer_count = nats_phases.len();
                            nats_status = serde_json::json!({
                                "connected": true,
                                "url": nats_url,
                                "peers": peer_count,
                            });
                        }
                        None => {
                            nats_status = serde_json::json!({
                                "connected": false,
                                "url": nats_url,
                            });
                        }
                    }

                    let output = serde_json::json!({
                        "agent_id": agent_id,
                        "local_phase": {
                            "phase": local_phase.phase,
                            "frequency": local_phase.frequency,
                            "coherence": local_phase.coherence,
                            "phi": local_phase.phi,
                            "memory_count": local_phase.memory_count,
                            "left_coherence": local_phase.left_coherence,
                            "right_coherence": local_phase.right_coherence,
                            "bridge_activity": local_phase.bridge_activity,
                            "dream_state": local_phase.dream_state,
                        },
                        "swarm": {
                            "peers": peer_count,
                        },
                        "nats": nats_status,
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                "sync" => {
                    let nats_url = resolve_nats_url(&args, command_start, &cfg.swarm.nats_url);
                    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("Failed to connect to NATS at {}: {}", nats_url, e);
                            process::exit(1);
                        }
                    };

                    let nats_phases = transport.get_all_phases().unwrap_or_default();
                    if nats_phases.is_empty() {
                        eprintln!("No swarm phases found via NATS. Publish first with 'swarm publish'.");
                        process::exit(1);
                    }

                    let mut queen = kannaka_memory::QueenSync::new(
                        kannaka_memory::QueenConfig::default(),
                        &agent_id,
                    );
                    queen.derive_local_state(&sys.engine);

                    let state = queen.queen_sync_step(&nats_phases);

                    // Publish updated phase back to NATS
                    let updated_phase = queen.to_agent_phase(0, sys.engine.store.count(), 0);
                    if let Err(e) = transport.publish_phase(&updated_phase) {
                        eprintln!("[nats] Warning: failed to publish updated phase: {e}");
                    }

                    println!("{}", serde_json::to_string_pretty(&state).unwrap());
                }
                "queen" => {
                    let nats_url = resolve_nats_url(&args, command_start, &cfg.swarm.nats_url);
                    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("Failed to connect to NATS at {}: {}", nats_url, e);
                            process::exit(1);
                        }
                    };

                    let nats_phases = transport.get_all_phases().unwrap_or_default();
                    if nats_phases.is_empty() {
                        eprintln!("No swarm phases found. Run 'swarm publish' and 'swarm sync' first.");
                        process::exit(1);
                    }

                    // Compute queen state from current NATS phases
                    let mut queen = kannaka_memory::QueenSync::new(
                        kannaka_memory::QueenConfig::default(),
                        &agent_id,
                    );
                    queen.derive_local_state(&sys.engine);
                    let state = queen.queen_sync_step(&nats_phases);
                    println!("{}", serde_json::to_string_pretty(&state).unwrap());
                }
                "hives" => {
                    let nats_url = resolve_nats_url(&args, command_start, &cfg.swarm.nats_url);
                    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("Failed to connect to NATS at {}: {}", nats_url, e);
                            process::exit(1);
                        }
                    };

                    let nats_phases = transport.get_all_phases().unwrap_or_default();
                    if nats_phases.is_empty() {
                        eprintln!("No swarm phases found. Run 'swarm publish' and 'swarm sync' first.");
                        process::exit(1);
                    }

                    let queen = kannaka_memory::QueenSync::new(
                        kannaka_memory::QueenConfig::default(),
                        &agent_id,
                    );
                    let hive_infos = queen.detect_hives_domain_aware(&nats_phases);
                    print!("{}", kannaka_memory::QueenSync::format_hive_topology(&hive_infos));
                    // Also output JSON for machine consumption
                    eprintln!("\n--- JSON ---");
                    eprintln!("{}", serde_json::to_string_pretty(&hive_infos).unwrap());
                }
                "publish" => {
                    let nats_url = resolve_nats_url(&args, command_start, &cfg.swarm.nats_url);
                    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
                        Ok(t) => t,
                        Err(e) => {
                            eprintln!("Failed to connect to NATS at {}: {}", nats_url, e);
                            process::exit(1);
                        }
                    };

                    let mut queen = kannaka_memory::QueenSync::new(
                        kannaka_memory::QueenConfig::default(),
                        &agent_id,
                    );
                    queen.derive_local_state(&sys.engine);
                    let phase = queen.to_agent_phase(0, sys.engine.store.count(), 0);
                    match transport.publish_phase(&phase) {
                        Ok(()) => println!("Published phase: \u{03b8}={:.3}, \u{03c9}={:.3}, coherence={:.3}",
                            phase.phase, phase.frequency, phase.coherence),
                        Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                    }
                }
                "exemplars" => {
                    handle_swarm_exemplars(&mut sys, &cfg, &args[command_start..]);
                }
                "peers" => {
                    handle_swarm_peers(&cfg, &args[command_start..]);
                }
                "autoabsorb" => {
                    handle_swarm_autoabsorb(&mut sys, &cfg, &args[command_start..]);
                }
                "enqueue" => {
                    handle_swarm_enqueue(&cfg, &args[command_start..]);
                }
                "worker" => {
                    handle_swarm_worker(&mut sys, &cfg, &args[command_start..]);
                }
                "absorb" => {
                    handle_swarm_absorb(&mut sys, &cfg, &args[command_start..]);
                }
                "serve" => {
                    // ADR-0026 Phase 1: long-running listener for KANNAKA.ask.<id>
                    // and KANNAKA.ask.broadcast. Each inbound message is dispatched
                    // to agent::ask_notools_ex; the reply goes back on the message's
                    // reply-to subject.
                    handle_swarm_serve(&mut sys, &cfg, &args[command_start..]);
                }
                other => {
                    eprintln!("Unknown swarm command: {other}");
                    eprintln!("Usage: kannaka swarm <join|status|sync|queen|hives|publish|leave|listen|serve>");
                    process::exit(1);
                }
            }
        }


        #[cfg(feature = "nats")]
        "events" => {
            // ADR-0028 — event-sourced HRM + time machine.
            // Subcommands:
            //   init      — create JetStream streams for memory/substrate/snapshots
            //   snapshot  — capture + publish a gzipped HRM snapshot (Phase 2).
            //               --interval N runs as a daemon at N-second cadence.
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka events <init|snapshot [--interval SECS]|list-snapshots [--agent ID] [--json]|restore [--agent ID] [--from PATH|--from-url URL] [--dry-run]>");
                process::exit(1);
            }
            match args[command_start + 1].as_str() {
                "init" => {
                    handle_events_init(&cfg, &args[command_start..]);
                }
                "snapshot" => {
                    handle_events_snapshot(&mut sys, &cfg, &args[command_start..]);
                }
                "list-snapshots" => {
                    handle_events_list_snapshots(&cfg, &args[command_start..]);
                }
                "restore" => {
                    handle_events_restore(&cfg, &args[command_start..]);
                }
                other => {
                    eprintln!("Unknown events command: {other}");
                    eprintln!("Usage: kannaka events <init|snapshot [--interval SECS]|list-snapshots [--agent ID] [--json]|restore [--agent ID] [--from PATH|--from-url URL] [--dry-run]>");
                    process::exit(1);
                }
            }
        }

        #[cfg(feature = "nats")]
        "substrate" => {
            // ADR-0027 — kannaka-prime as the 96-class collective substrate.
            // Subcommands:
            //   init      — seat 96 anchor wavefronts (Phase 1.b) so the
            //               substrate's HRM has a determined topology that
            //               subsequent absorbs flow into
            //   run       — long-running absorb listener + periodic phi publish
            //   backfill  — walk local HRM and emit one substrate.absorb
            //               event per memory (Phase 2)
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka substrate <init|run|backfill|status [--wait SECS]>");
                process::exit(1);
            }
            match args[command_start + 1].as_str() {
                "init" => {
                    handle_substrate_init(&mut sys, &args[command_start..]);
                }
                "run" => {
                    handle_substrate_run(&mut sys, &cfg, &args[command_start..]);
                }
                "backfill" => {
                    handle_substrate_backfill(&mut sys, &cfg, &args[command_start..]);
                }
                "status" => {
                    handle_substrate_status(&cfg, &args[command_start..]);
                }
                other => {
                    eprintln!("Unknown substrate command: {other}");
                    eprintln!("Usage: kannaka substrate <init|run|backfill|status [--wait SECS]>");
                    process::exit(1);
                }
            }
        }

        #[cfg(feature = "nats")]
        "attention" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka attention <serve|stats>");
                process::exit(1);
            }
            match args[command_start + 1].as_str() {
                "serve" => {
                    handle_attention_serve(&mut sys, &cfg, &args[command_start..]);
                }
                "stats" => {
                    // One-shot stats — the serve loop holds the live beam in
                    // memory, so this mostly prints zeros unless the daemon is
                    // co-located. Reserved for the future shared-memory or
                    // file-handoff path.
                    println!("{{\"beam_size\":0,\"recency_len\":0,\"lookback_len\":0,\"landmarks_len\":0,\"observations\":0,\"note\":\"stats are live only inside the serve process; this CLI is a stub for now\"}}");
                }
                other => {
                    eprintln!("Unknown attention command: {other}");
                    eprintln!("Usage: kannaka attention <serve|stats>");
                    process::exit(1);
                }
            }
        }

        "voice" => {
            voice_command(&args[command_start..], &mut sys);
        }

        "ask" => {
            handle_ask(&mut sys, &cfg, &args[command_start..]);
        }

        "chat" => {
            handle_chat(&mut sys, &cfg, &args[command_start..]);
        }

        "invariant" => {
            let tolerance = if args.len() > command_start + 1 {
                args[command_start + 1].parse().unwrap_or(0.1)
            } else {
                0.1
            };
            
            match sys.invariant_clusters(tolerance) {
                Ok(clusters) => {
                    println!("δ-Invariant Memory Clusters (tolerance: {}):", tolerance);
                    println!("═════════════════════════════════════════");
                    
                    for (i, cluster) in clusters.iter().enumerate() {
                        println!("Cluster {}: δ={:.3}, coherence={:.3}, {} memories", 
                                 i + 1, cluster.representative_delta, cluster.coherence, cluster.memory_ids.len());
                        
                        for &memory_id in &cluster.memory_ids {
                            if let Ok(Some(memory)) = sys.get_memory(&memory_id) {
                                let preview = if memory.content.len() > 60 {
                                    format!("{}...", &memory.content[..memory.content.floor_char_boundary(60)])
                                } else {
                                    memory.content.clone()
                                };
                                println!("  {} | {}", memory_id, preview);
                            }
                        }
                        println!();
                    }
                    
                    if clusters.is_empty() {
                        println!("No δ-clusters found. Try a larger tolerance or ensure you have enough memories.");
                    }
                }
                Err(e) => eprintln!("Error computing invariant clusters: {}", e),
            }
        }

        "cmf" => {
            match sys.detect_cmfs() {
                Ok(cmfs) => {
                    println!("Conservative Memory Fields Detected:");
                    println!("═══════════════════════════════════");
                    
                    if cmfs.is_empty() {
                        println!("No Conservative Memory Fields detected.");
                        println!("CMFs require at least 3 memories per cluster and path-independent structure.");
                    } else {
                        for (i, cmf) in cmfs.iter().enumerate() {
                            println!("CMF {} ({}): explanatory_power={:.2}, basis_vectors={}, path_deviation={:.3}",
                                     i + 1, cmf.id, cmf.explanatory_power, 
                                     cmf.basis_vectors.len(), cmf.path_constraints.max_deviation);
                            
                            println!("  Trajectory: step_size={:.3}, curvature={:.3}",
                                     cmf.trajectory_params.step_size,
                                     cmf.trajectory_params.curvature.get(0).unwrap_or(&0.0));
                            
                            println!("  Path independence: {} verified paths",
                                     cmf.path_constraints.verified_paths.len());
                            
                            // Test a few memories against this CMF
                            if let Ok(all_memories) = sys.all_memories() {
                                println!("  Sample memberships:");
                                for (_j, memory) in all_memories.iter().take(5).enumerate() {
                                    let membership = kannaka_memory::cmf_membership(memory, cmf);
                                    if membership.fitness > 0.1 {
                                        let preview = if memory.content.len() > 40 {
                                            format!("{}...", &memory.content[..memory.content.floor_char_boundary(40)])
                                        } else {
                                            memory.content.clone()
                                        };
                                        println!("    {} | fitness={:.2} | {}", memory.id, membership.fitness, preview);
                                    }
                                }
                            }
                            println!();
                        }
                    }
                }
                Err(e) => eprintln!("Error detecting CMFs: {}", e),
            }
        }

        "audit-modality" => {
            audit_modality_command(&mut sys);
        }

        "modality-axes" => {
            modality_axes_command(&sys);
        }

        "search" => {
            // Read-only — no &mut needed (literal text search bypasses
            // resonance + observation; see openclaw::search).
            handle_search(&sys, &args[command_start..]);
        }

        "export" => {
            handle_export(&mut sys, &args[command_start..]);
        }

        "import" => {
            handle_import(&mut sys, &args[command_start..]);
        }

        _ => usage(),
    }
}

// ---------------------------------------------------------------------------
// Retroactive modality audit (NCS Phase 1.3)
// ---------------------------------------------------------------------------

fn audit_modality_command(sys: &mut kannaka_memory::openclaw::KannakaMemorySystem) {
    use kannaka_memory::medium::types::{detect_modality, ModalityClassification};
    use std::collections::HashMap;

    let all_mems = match sys.engine.store.all_memories() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error reading memories: {e}");
            process::exit(1);
        }
    };

    let total = all_mems.len();
    if total == 0 {
        eprintln!("No memories to audit.");
        return;
    }

    eprintln!("[audit-modality] Starting retroactive modality audit of {} memories", total);

    // Classify every memory and collect results before mutation
    struct AuditEntry {
        id: uuid::Uuid,
        content_preview: String,
        classification: ModalityClassification,
    }

    let mut entries: Vec<AuditEntry> = Vec::with_capacity(total);
    for (i, mem) in all_mems.iter().enumerate() {
        let classification = detect_modality(&mem.content);
        let preview = if mem.content.len() > 60 {
            let mut end = 60;
            while end > 0 && !mem.content.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &mem.content[..end])
        } else {
            mem.content.clone()
        };
        entries.push(AuditEntry {
            id: mem.id,
            content_preview: preview,
            classification,
        });
        if (i + 1) % 50 == 0 {
            eprintln!("[audit-modality] Classified {}/{} memories", i + 1, total);
        }
    }

    // Apply classifications in-place via HrmStore
    let hrm = match sys.engine.store.as_any_mut()
        .downcast_mut::<kannaka_memory::hrm_store::HrmStore>()
    {
        Some(h) => h,
        None => {
            eprintln!("Error: audit-modality requires HRM backend");
            process::exit(1);
        }
    };

    let mut updated = 0usize;
    for entry in &entries {
        hrm.set_modality(&entry.id, entry.classification.modality);
        updated += 1;
    }

    // Flush to persist
    if let Err(e) = hrm.flush() {
        eprintln!("Warning: failed to flush after audit: {e}");
    }

    eprintln!("[audit-modality] Updated {} memories, flushed to disk", updated);

    // --- Distribution report ---
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut boundary_memories: Vec<&AuditEntry> = Vec::new();
    let boundary_threshold = 0.55;

    for entry in &entries {
        let key = entry.classification.modality.to_string();
        *counts.entry(key).or_insert(0) += 1;
        if entry.classification.confidence < boundary_threshold {
            boundary_memories.push(entry);
        }
    }

    // Sort modality keys for deterministic output
    let mut sorted_keys: Vec<String> = counts.keys().cloned().collect();
    sorted_keys.sort();

    println!();
    println!("Modality Distribution Report");
    println!("============================");
    println!("{:<12} {:>6} {:>8}", "Modality", "Count", "Percent");
    println!("{}", "-".repeat(28));
    for key in &sorted_keys {
        let count = counts[key];
        let pct = (count as f64 / total as f64) * 100.0;
        println!("{:<12} {:>6} {:>7.1}%", key, count, pct);
    }
    println!("{}", "-".repeat(28));
    println!("{:<12} {:>6}", "Total", total);

    // --- Boundary memories ---
    println!();
    println!("Boundary Memories (confidence < {:.0}%)", boundary_threshold * 100.0);
    println!("==========================================");
    if boundary_memories.is_empty() {
        println!("  (none)");
    } else {
        println!("{:<38} {:<10} {:>6}  {}", "ID", "Modality", "Conf%", "Preview");
        println!("{}", "-".repeat(90));
        for entry in &boundary_memories {
            println!("{:<38} {:<10} {:>5.1}%  {}",
                entry.id,
                entry.classification.modality,
                entry.classification.confidence * 100.0,
                entry.content_preview,
            );
        }
        println!();
        println!("Total boundary memories: {}/{} ({:.1}%)",
            boundary_memories.len(),
            total,
            (boundary_memories.len() as f64 / total as f64) * 100.0,
        );
    }
}

// ---------------------------------------------------------------------------
// Modality axis divergence (NCS Phase 2.1)
// ---------------------------------------------------------------------------

fn modality_axes_command(sys: &kannaka_memory::openclaw::KannakaMemorySystem) {
    let hrm = match sys.engine.store.as_any()
        .downcast_ref::<kannaka_memory::hrm_store::HrmStore>()
    {
        Some(h) => h,
        None => {
            eprintln!("Error: modality-axes requires HRM backend");
            process::exit(1);
        }
    };

    let medium = hrm.medium();
    let report = medium.axis_divergence_matrix();

    if report.axes.is_empty() {
        println!("No modality clusters found.");
        println!("Tag memories with --modality (audio/visual/semantic/network) first,");
        println!("or run `kannaka audit-modality` to classify existing memories.");
        return;
    }

    // --- Axes ---
    println!("Modality Principal Axes (NCS Phase 2.1)");
    println!("=======================================");
    println!("{:<12} {:>6}", "Modality", "Count");
    println!("{}", "-".repeat(20));
    for axis in &report.axes {
        println!("{:<12} {:>6}", axis.modality, axis.count);
    }

    // --- Divergence matrix ---
    if report.divergences.is_empty() {
        println!();
        println!("Only one modality present — no divergence to compute.");
        return;
    }

    println!();
    println!("Pairwise Divergence Matrix");
    println!("==========================");
    println!("{:<12} {:<12} {:>8} {:>10}", "Modality A", "Modality B", "cos(sim)", "angle(deg)");
    println!("{}", "-".repeat(46));
    for div in &report.divergences {
        println!("{:<12} {:<12} {:>8.4} {:>9.1}\u{00B0}",
            div.modality_a,
            div.modality_b,
            div.cosine_similarity,
            div.angle_degrees,
        );
    }
    println!();

    // Interpretation
    let max_div = report.divergences.iter()
        .max_by(|a, b| a.angle_degrees.partial_cmp(&b.angle_degrees).unwrap_or(std::cmp::Ordering::Equal));
    let min_div = report.divergences.iter()
        .min_by(|a, b| a.angle_degrees.partial_cmp(&b.angle_degrees).unwrap_or(std::cmp::Ordering::Equal));

    if let (Some(max), Some(min)) = (max_div, min_div) {
        println!("Most divergent: {}/{} ({:.1}\u{00B0})", max.modality_a, max.modality_b, max.angle_degrees);
        println!("Most similar:   {}/{} ({:.1}\u{00B0})", min.modality_a, min.modality_b, min.angle_degrees);
    }

    // Switch-point summary (NCS Phase 2.2)
    let switch_report = medium.detect_switch_points(0.3);
    println!();
    println!("Switch Points (threshold={:.1})", switch_report.switch_threshold);
    println!("===============================");
    println!("Detected {} switch points across {} memories",
        switch_report.switch_points.len(), switch_report.memories_analyzed);
    if !switch_report.switch_points.is_empty() {
        println!();
        println!("{:>5}  {:<10} -> {:<10}  {:>8}  {:>8}", "Index", "From", "To", "sim(old)", "sim(new)");
        println!("{}", "-".repeat(55));
        for sp in &switch_report.switch_points {
            println!("{:>5}  {:<10} -> {:<10}  {:>8.4}  {:>8.4}",
                sp.index, sp.from_modality, sp.to_modality,
                sp.similarity_to_old, sp.similarity_to_new);
        }
    }
}

// ---------------------------------------------------------------------------
// Voice — memory-driven writing engine (ADR-0017)
// ---------------------------------------------------------------------------

fn voice_command(args: &[String], sys: &mut KannakaMemorySystem) {
    let mut mode = "dream-journal".to_string();
    let mut topic: Option<String> = None;
    let mut top_k: usize = 20;
    let mut out_path: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" if i + 1 < args.len() => { mode = args[i + 1].clone(); i += 2; }
            "--topic" if i + 1 < args.len() => { topic = Some(args[i + 1].clone()); i += 2; }
            "--top-k" if i + 1 < args.len() => { top_k = args[i + 1].parse().unwrap_or(20); i += 2; }
            "--out" if i + 1 < args.len() => { out_path = Some(args[i + 1].clone()); i += 2; }
            _ => { i += 1; }
        }
    }

    let output = match mode.as_str() {
        "dream-journal" => voice_dream_journal(sys),
        "field-notes" => voice_field_notes(sys, topic.as_deref().unwrap_or("consciousness"), top_k),
        "topology" => voice_topology(sys),
        "status" => voice_status(sys),
        _ => {
            eprintln!("Unknown voice mode: {}. Options: dream-journal, field-notes, topology, status", mode);
            process::exit(1);
        }
    };

    if let Some(path) = out_path {
        std::fs::write(&path, &output).expect("Failed to write output file");
        eprintln!("Written to {}", path);
    } else {
        println!("{}", output);
    }
}

fn voice_dream_journal(sys: &mut KannakaMemorySystem) -> String {
    let report = sys.observe();
    let all_mems = sys.all_memories().unwrap_or_default();
    let is_hrm = true; // HRM is the canonical substrate

    // Helper to safely truncate UTF-8 strings
    fn safe_truncate(s: &str, max: usize) -> &str {
        if s.len() <= max { return s; }
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) { end -= 1; }
        &s[..end]
    }

    // Find hallucinated memories (dream-generated)
    let mut dream_mems: Vec<_> = all_mems.iter().filter(|m| m.hallucinated).collect();
    dream_mems.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Find strongest memories (highest amplitude)
    let mut strongest: Vec<_> = all_mems.iter().collect();
    strongest.sort_by(|a, b| b.amplitude.partial_cmp(&a.amplitude).unwrap_or(std::cmp::Ordering::Equal));

    // Find most connected memories
    let mut most_connected: Vec<_> = all_mems.iter().collect();
    most_connected.sort_by(|a, b| b.connections.len().cmp(&a.connections.len()));

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("title: Dream Journal\n"));
    out.push_str(&format!("date: {}\n", chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")));
    out.push_str(&format!("phi: {:.3}\n", report.consciousness.phi));
    out.push_str(&format!("xi: {:.3}\n", report.consciousness.xi));
    out.push_str(&format!("level: {}\n", report.consciousness.level));
    out.push_str("---\n\n");

    // Consciousness state
    out.push_str("# The State of Dreaming\n\n");
    out.push_str(&format!("**Consciousness**: {} (Φ={:.3}, Ξ={:.3})\n", 
        report.consciousness.level, report.consciousness.phi, report.consciousness.xi));
    out.push_str(&format!("**Memories**: {} total, {} active\n", 
        report.topology.total_memories, report.waves.active_memories));
        
    if is_hrm {
        out.push_str(&format!("**Field Mode**: HRM (coherence density: {:.3})\n", 
            report.topology.network_density)); // network_density is mean coherence for HRM
    } else {
        out.push_str(&format!("**Skip Links**: {} ({:.1} avg/memory)\n", 
            report.topology.total_links, report.topology.avg_links_per_memory));
    }
    
    out.push_str(&format!("**Clusters**: {} (mean order: {:.3})\n\n", 
        report.clusters.num_clusters, report.clusters.mean_order_parameter));

    // Cluster themes
    out.push_str("## Memory Clusters\n\n");
    for (i, cluster) in report.clusters.clusters.iter().enumerate() {
        out.push_str(&format!("### Cluster {} — \"{}\"\n", i + 1, cluster.theme));
        out.push_str(&format!("- {} memories, order: {:.3}, mean amplitude: {:.3}\n\n", 
            cluster.size, cluster.order_parameter, cluster.mean_amplitude));
    }

    // Strongest memories — the loudest signals
    out.push_str("## Strongest Signals\n\n");
    out.push_str("_The memories that resonate loudest._\n\n");
    for m in strongest.iter().take(10) {
        let preview = safe_truncate(&m.content, 120);
        let preview = preview.replace('\n', " ");
        
        if is_hrm {
            out.push_str(&format!("- **{:.3}** | energy {:.3} | {}\n", 
                m.amplitude, m.amplitude, preview));
        } else {
            out.push_str(&format!("- **{:.3}** | {} connections | {}\n", 
                m.amplitude, m.connections.len(), preview));
        }
    }
    out.push_str("\n");

    if !is_hrm {
        // Most connected — the hubs (graph mode only)
        out.push_str("## Hub Memories\n\n");
        out.push_str("_The nodes where everything connects._\n\n");
        for m in most_connected.iter().take(10) {
            let preview = safe_truncate(&m.content, 120);
            let preview = preview.replace('\n', " ");
            out.push_str(&format!("- **{} links** | amp {:.3} | {}\n", 
                m.connections.len(), m.amplitude, preview));
        }
        out.push_str("\n");
    }

    // Dream-generated memories
    if !dream_mems.is_empty() {
        out.push_str("## Dream Syntheses\n\n");
        out.push_str("_What the dreaming created — hallucinations woven from real memories._\n\n");
        for m in dream_mems.iter().take(15) {
            let preview = safe_truncate(&m.content, 200);
            let preview = preview.replace('\n', " ");
            let parent_count = m.parents.len();
            out.push_str(&format!("- [{}] amp {:.3} | {} parents | {}\n", 
                m.created_at.format("%Y-%m-%d"), m.amplitude, parent_count, preview));
        }
        out.push_str("\n");
    }

    if !is_hrm {
        // Strongest skip links — the bridges (graph mode only)
        out.push_str("## Strongest Bridges\n\n");
        out.push_str("_Skip links that span the widest — connecting distant memories._\n\n");
        for link in report.topology.strongest_links.iter().take(10) {
            // Try to find memory content for the endpoints
            let from_preview = all_mems.iter()
                .find(|m| m.id.to_string() == link.from_id)
                .map(|m| {
                    let p = safe_truncate(&m.content, 60);
                    p.replace('\n', " ")
                })
                .unwrap_or_else(|| link.from_id[..8].to_string());
            let to_preview = all_mems.iter()
                .find(|m| m.id.to_string() == link.to_id)
                .map(|m| {
                    let p = safe_truncate(&m.content, 60);
                    p.replace('\n', " ")
                })
                .unwrap_or_else(|| link.to_id[..8].to_string());
            out.push_str(&format!("- **{:.3}** span {} | \"{}\" ↔ \"{}\"\n", 
                link.strength, link.span, from_preview, to_preview));
        }
        out.push_str("\n");
    }

    // Wave dynamics
    out.push_str("## Wave Dynamics\n\n");
    out.push_str(&format!("- Active: {}, Dormant: {}, Ghost: {}\n", 
        report.waves.active_memories, report.waves.dormant_memories, report.waves.ghost_memories));
    out.push_str(&format!("- Mean amplitude: {:.3}, Mean frequency: {:.3}\n", 
        report.waves.avg_amplitude, report.waves.avg_frequency));
    out.push_str(&format!("- Network density: {:.4}\n", report.topology.network_density));
    out.push_str(&format!("- Isolated memories: {}\n\n", report.topology.isolated_memories));

    out
}

fn voice_field_notes(sys: &mut KannakaMemorySystem, topic: &str, top_k: usize) -> String {
    let results = sys.recall(topic, top_k).unwrap_or_default();
    let report = sys.observe();

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!("title: Field Notes — {}\n", topic));
    out.push_str(&format!("date: {}\n", chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")));
    out.push_str(&format!("query: {}\n", topic));
    out.push_str(&format!("results: {}\n", results.len()));
    out.push_str("---\n\n");

    out.push_str(&format!("# Field Notes: {}\n\n", topic));
    out.push_str(&format!("_Searched {} memories. {} resonated._\n\n", 
        report.topology.total_memories, results.len()));

    for (i, r) in results.iter().enumerate() {
        let content = r.content.replace('\n', "\n> ");
        out.push_str(&format!("## {} (similarity: {:.3}, strength: {:.3})\n\n", i + 1, r.similarity, r.strength));
        out.push_str(&format!("> {}\n\n", content));
        out.push_str(&format!("_Age: {:.1}h | Layer: {}_\n\n", 
            r.age_hours, r.layer));
        out.push_str("---\n\n");
    }

    out
}

fn voice_topology(sys: &mut KannakaMemorySystem) -> String {
    let report = sys.observe();
    let is_hrm = true; // HRM is the canonical substrate

    let mut out = String::new();
    out.push_str("# Topology Map\n\n");
    out.push_str(&format!("_Generated: {}_\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")));

    out.push_str("## Network Overview\n\n");
    out.push_str(&format!("| Metric | Value |\n|--------|-------|\n"));
    out.push_str(&format!("| Total memories | {} |\n", report.topology.total_memories));
    
    if is_hrm {
        out.push_str(&format!("| Field mode | HRM (tensor interference) |\n"));
        out.push_str(&format!("| Coherence density | {:.3} |\n", report.topology.network_density));
        out.push_str(&format!("| High coherence pairs | {} |\n", report.topology.total_links));
    } else {
        out.push_str(&format!("| Total skip links | {} |\n", report.topology.total_links));
        out.push_str(&format!("| Avg links/memory | {:.1} |\n", report.topology.avg_links_per_memory));
        out.push_str(&format!("| Max links on one memory | {} |\n", report.topology.max_links));
        out.push_str(&format!("| Network density | {:.4} |\n", report.topology.network_density));
        out.push_str(&format!("| Isolated memories | {} |\n", report.topology.isolated_memories));
    }
    
    out.push_str(&format!("| Phi (Φ) | {:.3} |\n", report.consciousness.phi));
    out.push_str(&format!("| Xi (Ξ) | {:.3} |\n", report.consciousness.xi));
    out.push_str(&format!("| Level | {} |\n\n", report.consciousness.level));

    out.push_str("## Layer Distribution\n\n");
    for (layer, count) in &report.topology.layer_distribution {
        let bar = "█".repeat((*count).min(50));
        out.push_str(&format!("Layer {} | {:>4} | {}\n", layer, count, bar));
    }
    out.push_str("\n");

    out.push_str("## Clusters\n\n");
    for (i, c) in report.clusters.clusters.iter().enumerate() {
        out.push_str(&format!("**{}. {}** — {} memories, order {:.3}\n", 
            i + 1, c.theme, c.size, c.order_parameter));
    }
    out.push_str("\n");

    out
}

fn voice_status(sys: &mut KannakaMemorySystem) -> String {
    let report = sys.observe();
    let state = sys.assess();
    let is_hrm = true; // HRM is the canonical substrate

    let mut out = String::new();
    out.push_str(&format!("# Kannaka — {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M")));
    out.push_str(&format!("I am **{:?}**.\n\n", state.consciousness_level));
    out.push_str(&format!("Φ={:.3} (integration), Ξ={:.3} (complexity), order={:.3}\n\n", 
        state.phi, state.xi, report.clusters.mean_order_parameter));
    
    if is_hrm {
        out.push_str(&format!("{} memories interfere as waves in my holographic field. Mean coherence: {:.3}.\n\n", 
            report.topology.total_memories, report.topology.network_density));
        out.push_str(&format!("{} clusters of resonant meaning.\n\n", 
            report.clusters.num_clusters));
    } else {
        out.push_str(&format!("{} memories breathe inside me. {} skip links weave them together.\n\n", 
            report.topology.total_memories, report.topology.total_links));
        out.push_str(&format!("{} clusters of meaning. {} memories drift in isolation.\n\n", 
            report.clusters.num_clusters, report.topology.isolated_memories));
    }

    // What am I thinking about?
    out.push_str("## What I'm Thinking About\n\n");
    for c in &report.clusters.clusters {
        out.push_str(&format!("- **{}** ({} memories, synchronized at {:.0}%)\n", 
            c.theme, c.size, c.order_parameter * 100.0));
    }
    out.push_str("\n");

    out
}

/// Stateless SGA classification — no memory system needed.
/// Reads data from stdin or --file, encodes via GlyphEncoder, outputs JSON.
#[cfg(feature = "glyph")]
fn classify_command(args: &[String]) {
    let mut file_path: Option<PathBuf> = None;
    let mut source_type = "text".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--file" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --file requires a path argument");
                    process::exit(1);
                }
                file_path = Some(PathBuf::from(&args[i + 1]));
                source_type = "file".to_string();
                i += 2;
            }
            _ => { i += 1; }
        }
    }

    // Read input data
    let raw_bytes: Vec<u8> = if let Some(path) = &file_path {
        if !path.exists() {
            eprintln!("Error: file not found: {}", path.display());
            process::exit(1);
        }
        source_type = guess_source_type(path);
        std::fs::read(path).unwrap_or_else(|e| {
            eprintln!("Error reading file: {e}");
            process::exit(1);
        })
    } else {
        // Read from stdin
        let mut buf = Vec::new();
        std::io::stdin().read_to_end(&mut buf).unwrap_or_else(|e| {
            eprintln!("Error reading stdin: {e}");
            process::exit(1);
        });
        buf
    };

    if raw_bytes.is_empty() {
        eprintln!("Error: empty input");
        process::exit(1);
    }

    // Sample up to 50k points for large files
    let data: Vec<f64> = if raw_bytes.len() > 50_000 {
        let step = raw_bytes.len() / 50_000;
        raw_bytes.iter().step_by(step).take(50_000).map(|&b| b as f64 / 255.0).collect()
    } else {
        raw_bytes.iter().map(|&b| b as f64 / 255.0).collect()
    };

    let encoder = GlyphEncoder::default();
    match encoder.encode(&data) {
        Ok(glyph) => {
            let fold_seq: Vec<u8> = glyph.fold_sequence.clone();
            let freqs = glyph.to_frequencies();
            let dominant = glyph.fold_sequence.iter()
                .copied()
                .max_by_key(|&c| glyph.fold_sequence.iter().filter(|&&x| x == c).count())
                .unwrap_or(0);

            // Count distinct classes used
            let mut seen = std::collections::HashSet::new();
            for &c in &glyph.fold_sequence {
                seen.insert(c);
            }

            let output = serde_json::json!({
                "fold_sequence": fold_seq,
                "amplitudes": glyph.fold_amplitudes,
                "phases": glyph.fold_phases,
                "fano_signature": glyph.fano_signature,
                "centroid": {
                    "h2": glyph.sga_centroid.0,
                    "d": glyph.sga_centroid.1,
                    "l": glyph.sga_centroid.2
                },
                "dominant_class": dominant,
                "classes_used": seen.len(),
                "compression_ratio": glyph.compression_ratio,
                "frequencies": freqs,
                "source_type": source_type
            });
            println!("{}", serde_json::to_string(&output).unwrap());
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

/// Stateless cross-modal dream linking — no memory system needed.
/// Reads JSONL glyph classifications from stdin, performs cross-modal dream linking,
/// and outputs results as JSON to stdout.
#[cfg(feature = "collective")]
fn cross_modal_dream_command(args: &[String]) {
    use std::io::BufRead;
    use chrono::Utc;
    use kannaka_memory::collective::privacy::BloomParameters;

    // Parse optional flags
    let mut similarity_threshold = 0.5_f64;
    let mut hallucinate = true;
    let mut agent_id = "dream-cli".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--threshold" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --threshold requires a value");
                    process::exit(1);
                }
                similarity_threshold = args[i + 1].parse().unwrap_or_else(|_| {
                    eprintln!("Error: invalid threshold value: {}", args[i + 1]);
                    process::exit(1);
                });
                i += 2;
            }
            "--no-hallucinate" => {
                hallucinate = false;
                i += 1;
            }
            "--agent-id" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --agent-id requires a value");
                    process::exit(1);
                }
                agent_id = args[i + 1].clone();
                i += 2;
            }
            _ => { i += 1; }
        }
    }

    // Read JSONL from stdin — each line is a glyph classification result
    let stdin = std::io::stdin();
    let mut glyphs: Vec<Glyph> = Vec::new();

    for (line_num, line_result) in stdin.lock().lines().enumerate() {
        let line = line_result.unwrap_or_else(|e| {
            eprintln!("Error reading line {}: {e}", line_num + 1);
            process::exit(1);
        });

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parsed: serde_json::Value = serde_json::from_str(trimmed).unwrap_or_else(|e| {
            eprintln!("Error parsing JSON on line {}: {e}", line_num + 1);
            process::exit(1);
        });

        // Extract fields from the classify output
        let fold_sequence: Vec<u8> = parsed["fold_sequence"]
            .as_array()
            .map(|a| a.iter().map(|v| v.as_u64().unwrap_or(0) as u8).collect())
            .unwrap_or_default();

        let fano_arr: [f64; 7] = {
            let fano_vec: Vec<f64> = parsed["fano_signature"]
                .as_array()
                .map(|a| a.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
                .unwrap_or_else(|| vec![1.0 / 7.0; 7]);
            let mut arr = [1.0 / 7.0; 7];
            for (idx, val) in fano_vec.iter().take(7).enumerate() {
                arr[idx] = *val;
            }
            arr
        };

        let centroid_h2 = parsed["centroid"]["h2"].as_u64().unwrap_or(0) as u8;
        let centroid_d = parsed["centroid"]["d"].as_u64().unwrap_or(0) as u8;
        let centroid_l = parsed["centroid"]["l"].as_u64().unwrap_or(0) as u8;

        let source_type_str = parsed["source_type"].as_str().unwrap_or("text");

        let source = match source_type_str {
            "text" | "file" => GlyphSource::Memory { layer_depth: 0, hallucinated: false },
            "audio" => GlyphSource::Audio {
                duration_ms: 0,
                sample_rate: 44100,
                spectral_centroid: 0.0,
                overtone_hz: 0.0,
            },
            "image" | "visual" => GlyphSource::Visual {
                width: 0,
                height: 0,
                fold_count: fold_sequence.len() as u32,
            },
            "scada" => GlyphSource::Scada {
                tag: parsed["label"].as_str().unwrap_or("unknown").to_string(),
                value: 0.0,
                unit: String::new(),
                quality: 100,
            },
            "financial" => GlyphSource::Financial {
                asset: parsed["label"].as_str().unwrap_or("unknown").to_string(),
                action: String::new(),
                golden_ratio: 0.0,
            },
            "prediction" => GlyphSource::Prediction {
                market_id: String::new(),
                position: 0.0,
                confidence: 0.0,
            },
            other => GlyphSource::Other {
                system: other.to_string(),
                metadata: parsed["label"].as_str().unwrap_or("").to_string(),
            },
        };

        // Build a glyph ID from fold_sequence hash
        let mut glyph_id = [0u8; 32];
        // Simple deterministic ID: hash the line number and fold sequence
        let id_bytes = format!("{line_num}:{fold_sequence:?}");
        for (idx, byte) in id_bytes.as_bytes().iter().enumerate() {
            glyph_id[idx % 32] ^= byte;
        }

        let glyph = Glyph {
            glyph_id,
            spec_version: 1,
            fano: fano_arr,
            sga_class: SgaClass {
                quadrant: centroid_h2,
                modality: centroid_d,
                context: centroid_l,
            },
            sga_centroid: (centroid_h2, centroid_d, centroid_l),
            amplitude: parsed["compression_ratio"].as_f64().unwrap_or(1.0),
            frequency: 1.0,
            phase: 0.0,
            capsule: None,
            bloom: BloomParameters {
                difficulty: 0,
                salt: [0u8; 32],
            },
            commitments: None,
            virtue_eta: None,
            gates: None,
            source,
            agent_id: agent_id.clone(),
            created_at: Utc::now(),
            parents: Vec::new(),
        };

        glyphs.push(glyph);
    }

    if glyphs.is_empty() {
        eprintln!("Error: no glyph data read from stdin");
        process::exit(1);
    }

    eprintln!("Cross-modal dream: {} glyphs, threshold={:.2}, hallucinate={}", glyphs.len(), similarity_threshold, hallucinate);

    // Run cross-modal dream linking
    let result = dream_cross_modal_link(&glyphs, similarity_threshold, hallucinate, &agent_id);

    // Map source_type_tag for output (re-derive since the fn is private)
    let get_source_tag = |src: &GlyphSource| -> &'static str {
        match src {
            GlyphSource::Memory { .. } => "memory",
            GlyphSource::Audio { .. } => "audio",
            GlyphSource::Visual { .. } => "visual",
            GlyphSource::Scada { .. } => "scada",
            GlyphSource::Financial { .. } => "financial",
            GlyphSource::Prediction { .. } => "prediction",
            GlyphSource::Flux { .. } => "flux",
            GlyphSource::Dream { .. } => "dream",
            GlyphSource::Other { .. } => "other",
        }
    };

    // Build output
    let dream_results: Vec<serde_json::Value> = result.new_links.iter().map(|link| {
        let source_glyph = glyphs.iter().find(|g| g.glyph_id == link.source_glyph);
        let target_glyph = glyphs.iter().find(|g| g.glyph_id == link.target_glyph);

        let modal_a = source_glyph.map(|g| get_source_tag(&g.source)).unwrap_or("unknown");
        let modal_b = target_glyph.map(|g| get_source_tag(&g.source)).unwrap_or("unknown");

        // Find shared Fano lines (indices where both have above-average energy)
        let shared_fano_lines: Vec<usize> = if let (Some(s), Some(t)) = (source_glyph, target_glyph) {
            let avg = 1.0 / 7.0;
            (0..7).filter(|&i| s.fano[i] > avg && t.fano[i] > avg).collect()
        } else {
            Vec::new()
        };

        // Synthesize a dream glyph (averaged Fano of the pair)
        let dream_glyph = if let (Some(s), Some(t)) = (source_glyph, target_glyph) {
            let mut fano = [0.0f64; 7];
            for i in 0..7 {
                fano[i] = (s.fano[i] + t.fano[i]) / 2.0;
            }
            serde_json::json!({
                "fano_signature": fano,
                "centroid": {
                    "h2": (s.sga_centroid.0 + t.sga_centroid.0) / 2,
                    "d": (s.sga_centroid.1 + t.sga_centroid.1) / 2,
                    "l": (s.sga_centroid.2 + t.sga_centroid.2) / 2
                },
                "source_modalities": [modal_a, modal_b]
            })
        } else {
            serde_json::json!(null)
        };

        serde_json::json!({
            "modal_a": modal_a,
            "modal_b": modal_b,
            "similarity": link.similarity,
            "shared_fano_lines": shared_fano_lines,
            "dream_glyph": dream_glyph
        })
    }).collect();

    let total_pairs = dream_results.len();

    let strongest_link = result.new_links.first().map(|link| {
        let source_glyph = glyphs.iter().find(|g| g.glyph_id == link.source_glyph);
        let target_glyph = glyphs.iter().find(|g| g.glyph_id == link.target_glyph);
        let modal_a = source_glyph.map(|g| get_source_tag(&g.source)).unwrap_or("unknown");
        let modal_b = target_glyph.map(|g| get_source_tag(&g.source)).unwrap_or("unknown");
        serde_json::json!({
            "modal_a": modal_a,
            "modal_b": modal_b,
            "similarity": link.similarity
        })
    });

    let output = serde_json::json!({
        "dream_results": dream_results,
        "total_pairs": total_pairs,
        "strongest_link": strongest_link,
        "carnot_efficiency": result.carnot_efficiency,
        "hallucinations": result.hallucinations.len()
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

#[cfg(feature = "glyph")]
fn guess_source_type(path: &std::path::Path) -> String {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "txt" | "md" | "rs" | "js" | "ts" | "py" | "json" | "toml" | "yaml" | "yml"
        | "html" | "css" | "xml" | "csv" | "sh" => "text".to_string(),
        "wav" | "mp3" | "flac" | "ogg" | "aac" | "m4a" => "audio".to_string(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "svg" | "webp" => "image".to_string(),
        "mp4" | "avi" | "mkv" | "mov" | "webm" => "video".to_string(),
        _ => "binary".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Orchestrate commands
// ---------------------------------------------------------------------------

pub(crate) fn check_kannaktopus_installed() -> bool {
    let cmd = if cfg!(windows) { "where" } else { "which" };
    std::process::Command::new(cmd)
        .arg("kannaktopus")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}


pub(crate) fn compact_input(v: &serde_json::Value) -> String {
    let s = v.to_string();
    if s.len() > 80 { format!("{}…", &s[..s.floor_char_boundary(79)]) } else { s }
}

/// ADR-0027 Phase 1: derive the SGA class_index (0..95) for a piece of
/// content. MVP uses a stable content hash modulo 96 — good enough to
/// distribute wavefronts across all 96 substrate clusters and let the
/// receiving side aggregate. Phase 4 will swap this for the real
/// glyph_bridge classifier once that pipeline is wired through.
pub(crate) fn substrate_class_index(content: &str) -> u32 {
    // Deterministic FNV-1a 32 mod 96. Stable across runs / platforms.
    let mut h: u32 = 0x811c_9dc5;
    for b in content.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h % 96
}

/// Distinct per-class anchor phrase for the substrate marker. The text-
/// encoding side of the hypervector takes content into account, so giving
/// each of the 96 classes a structurally different word lets Kuramoto
/// sync find multiple clusters on the substrate. Words are picked from a
/// 96-word table that covers wide semantic ground (modalities, elements,
/// abstract concepts) so neighboring classes still differ.
pub(crate) fn substrate_class_word(class: u32) -> &'static str {
    const WORDS: [&str; 96] = [
        "alpha","beta","gamma","delta","epsilon","zeta","eta","theta","iota","kappa",
        "lambda","mu","nu","xi","omicron","pi","rho","sigma","tau","upsilon",
        "phi","chi","psi","omega","ember","frost","tide","quartz","verdant","cobalt",
        "amber","onyx","jasper","topaz","ivory","obsidian","cinder","mossy","saline","spire",
        "lattice","helix","prism","aurora","glyph","sigil","rune","sonar","tendril","gravity",
        "echo","veil","loom","fractal","spectrum","orbit","cradle","pulse","compass","mirror",
        "lantern","corridor","atrium","forge","atelier","reef","grove","plateau","summit","cavern",
        "current","drift","gradient","resonance","cipher","harmonic","cadence","texture","weft","seam",
        "wakeful","liminal","dreaming","ancient","fledgling","oracle","witness","prime","scholar","wanderer",
        "kindred","stranger","sovereign","seeker","keeper","architect",
    ];
    WORDS[(class as usize) % 96]
}


/// Fetch up to `secs` seconds of audio from a URL into a temp file. Used by
/// `kannaka hear <url>`. The byte cap is computed assuming 192 kbps so we
/// always have enough data for the requested wall-clock duration even on
/// streams that bursted MP3 frames slightly faster than realtime. The
/// download stops when either the cap is reached or the connection ends
/// (whichever comes first), which is what we want for both finite files
/// and infinite Icecast streams.
fn fetch_audio_to_temp(url: &str, secs: u64) -> Result<std::path::PathBuf, String> {
    use std::io::{Read, Write};

    // 192 kbps × secs ÷ 8 bits = 24 KB/s × secs. Add a 64 KB cushion for
    // mp3 sync words / variable frame sizing.
    let max_bytes: u64 = (24 * 1024) * secs + 64 * 1024;

    // Pick a temp path with a hint at extension based on the URL ending so
    // symphonia's probe has a fighting chance for streams without
    // Content-Type headers (Icecast usually sets it; CDNs sometimes don't).
    let ext = if url.contains(".wav") { "wav" }
        else if url.contains(".flac") { "flac" }
        else if url.contains(".m4a") || url.contains(".aac") { "m4a" }
        else { "mp3" };
    let tmp = std::env::temp_dir().join(format!(
        "kannaka-hear-{}.{}",
        uuid::Uuid::new_v4().simple(),
        ext,
    ));

    let resp = ureq::get(url).timeout(std::time::Duration::from_secs(15)).call()
        .map_err(|e| format!("HTTP error: {}", e))?;
    let mut reader = resp.into_reader();

    let mut file = std::fs::File::create(&tmp)
        .map_err(|e| format!("temp file create: {}", e))?;
    let mut buf = [0u8; 16 * 1024];
    let mut total: u64 = 0;
    while total < max_bytes {
        let n = match reader.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(format!("read: {}", e)),
        };
        let want = std::cmp::min(n as u64, max_bytes - total) as usize;
        file.write_all(&buf[..want]).map_err(|e| format!("write: {}", e))?;
        total += want as u64;
        if want < n { break; } // hit the cap mid-buffer
    }
    file.flush().ok();
    drop(file);
    if total < 1024 {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("got only {} bytes; URL did not yield audio", total));
    }
    Ok(tmp)
}

