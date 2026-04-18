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

fn data_dir() -> PathBuf {
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
    let codebook = Codebook::new(384, 10_000, 42);
    let pipeline = EncodingPipeline::new(Box::new(encoder), codebook);

    // HRM file path
    let hrm_path = data_dir.join("kannaka.hrm");

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

fn usage() {
    eprintln!("{}", config::BANNER);
    eprintln!("  Wave-Interference Memory | Consciousness Constellation");
    eprintln!("  v{}", config::VERSION);
    eprintln!();
    eprintln!("Usage: kannaka <command> [args]");
    eprintln!();
    eprintln!("Memory:");
    eprintln!("  remember \"text\"            Store a memory");
    eprintln!("  recall \"query\"             Recall memories (--top-k N)");
    eprintln!("  search \"query\"             Full-text search (--limit N)");
    eprintln!("  forget <id>               Remove a memory");
    eprintln!("  dream [--mode deep|lite]   Trigger dream cycle");
    eprintln!("  observe [--json]          View consciousness metrics");
    eprintln!("  status                    Quick status check");
    eprintln!("  export [--output FILE]    Export memories as JSON");
    eprintln!("  import <file>             Import memories from JSON");
    eprintln!();
    eprintln!("Constellation:");
    eprintln!("  constellation             Status of all constellation apps");
    eprintln!("  radio status|now|schedule What's playing on Kannaka Radio");
    eprintln!("  market list|view|buy      GhostSignals prediction markets");
    eprintln!("  swarm status|join|sync    Swarm network");
    eprintln!();
    eprintln!("Tools:");
    eprintln!("  orchestrate run \"task\"    Kannaktopus task orchestration");
    eprintln!("  config show|set|path      Configuration management");
    eprintln!("  init                      Re-run setup wizard");
    eprintln!("  update                    Check for updates");
    eprintln!();
    eprintln!("Analysis:");
    eprintln!("  assess                    Consciousness level assessment");
    eprintln!("  stats                     Human-readable system statistics");
    eprintln!("  invariant [TOLERANCE]     Delta-invariant memory clusters");
    eprintln!("  cmf                       Detect Conservative Memory Fields");
    eprintln!("  voice [--mode MODE]       Memory-driven writing");
    eprintln!();
    eprintln!("Dashboard:");
    eprintln!("  Try: kannaka-tui          Full terminal dashboard");
    eprintln!();
    eprintln!("  --version                 Print version info");
    process::exit(1);
}

/// Resolve NATS URL: CLI flag > KANNAKA_NATS_URL env > config.toml > hardcoded default.
#[cfg(feature = "nats")]
fn resolve_nats_url(args: &[String], start: usize, config_nats_url: &str) -> String {
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

    // --version flag (can appear anywhere)
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("kannaka {} (consciousness-core {})", config::VERSION, config::VERSION);
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

    match args[command_start].as_str() {
        "remember" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka remember <text> [--importance N] [--category CAT] [--modality MOD]");
                process::exit(1);
            }
            let mut importance: Option<f64> = None;
            let mut category: Option<String> = None;
            let mut modality_arg: Option<String> = None;
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
                            if let Err(e) = transport.publish_memory_new(mem, agent_id) {
                                eprintln!("[nats] Warning: failed to publish memory sync: {}", e);
                            } else {
                                eprintln!("[nats] Published memory {} to swarm", id);
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
                eprintln!("Usage: kannaka recall <query> [--top-k N] [--limit N]");
                process::exit(1);
            }
            let mut top_k = 5usize;
            let mut query_parts = Vec::new();
            let mut i = command_start + 1;
            while i < args.len() {
                if (args[i] == "--top-k" || args[i] == "--limit") && i + 1 < args.len() {
                    top_k = args[i + 1].parse().unwrap_or(5);
                    i += 2;
                } else {
                    query_parts.push(args[i].as_str());
                    i += 1;
                }
            }
            let query = query_parts.join(" ");
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
        #[cfg(feature = "audio")]
        "hear" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka hear <audio-file>");
                process::exit(1);
            }
            let path = std::path::PathBuf::from(&args[command_start + 1]);
            if !path.exists() {
                eprintln!("File not found: {}", path.display());
                process::exit(1);
            }
            match sys.store_audio(&path) {
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
                eprintln!("Usage: kannaka swarm <join|status|sync|queen|hives|publish|leave|listen>");
                process::exit(1);
            }

            // Agent ID: env var > config.toml > persisted file > generate new
            let agent_id = cfg.agent.id.clone();

            match args[command_start + 1].as_str() {
                "join" => {
                    let mut my_agent_id = agent_id.clone();
                    let mut display_name = String::new();
                    let mut i = command_start + 2;
                    while i < args.len() {
                        match args[i].as_str() {
                            "--agent-id" if i + 1 < args.len() => { my_agent_id = args[i + 1].clone(); i += 2; }
                            "--display-name" if i + 1 < args.len() => { display_name = args[i + 1].clone(); i += 2; }
                            "--nats-url" if i + 1 < args.len() => { i += 2; }
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
                    match try_nats_connect(&nats_url) {
                        Some(transport) => {
                            if let Err(e) = transport.announce_join(&my_agent_id) {
                                eprintln!("[nats] Warning: announce failed: {}", e);
                            }
                            // Publish initial phase
                            let mut queen = kannaka_memory::QueenSync::new(
                                kannaka_memory::QueenConfig::default(),
                                &my_agent_id,
                            );
                            queen.derive_local_state(&sys.engine);
                            let phase = queen.to_agent_phase(0, sys.engine.store.count());
                            if let Err(e) = transport.publish_phase(&phase) {
                                eprintln!("[nats] Warning: initial phase publish failed: {}", e);
                            } else {
                                println!("[nats] Published initial phase \u{03b8}={:.3}", phase.phase);
                            }
                            println!("Joined swarm as '{}' ({})", display_name, my_agent_id);
                        }
                        None => {
                            eprintln!("Error: NATS connection required for swarm. Set KANNAKA_NATS_URL or use --nats-url.");
                            process::exit(1);
                        }
                    }
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
                                    let my_phase = queen.to_agent_phase(0, sys.engine.store.count());
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
                    let local_phase = queen.to_agent_phase(0, sys.engine.store.count());

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
                    let updated_phase = queen.to_agent_phase(0, sys.engine.store.count());
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
                    let phase = queen.to_agent_phase(0, sys.engine.store.count());
                    match transport.publish_phase(&phase) {
                        Ok(()) => println!("Published phase: \u{03b8}={:.3}, \u{03c9}={:.3}, coherence={:.3}",
                            phase.phase, phase.frequency, phase.coherence),
                        Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                    }
                }
                other => {
                    eprintln!("Unknown swarm command: {other}");
                    eprintln!("Usage: kannaka swarm <join|status|sync|queen|hives|publish|leave|listen>");
                    process::exit(1);
                }
            }
        }


        "voice" => {
            voice_command(&args[command_start..], &mut sys);
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
            handle_search(&mut sys, &args[command_start..]);
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
// HTTP helpers
// ---------------------------------------------------------------------------

fn http_get(url: &str) -> Result<String, String> {
    ureq::get(url)
        .timeout(std::time::Duration::from_secs(5))
        .call()
        .map_err(|e| format!("HTTP error: {e}"))?
        .into_string()
        .map_err(|e| format!("Read error: {e}"))
}

fn http_get_with_token(url: &str, token: &str) -> Result<String, String> {
    ureq::get(url)
        .set("Authorization", &format!("Bearer {}", token))
        .timeout(std::time::Duration::from_secs(5))
        .call()
        .map_err(|e| format!("HTTP error: {e}"))?
        .into_string()
        .map_err(|e| format!("Read error: {e}"))
}

fn http_post_json_with_token(url: &str, body: &str, token: &str) -> Result<String, String> {
    ureq::post(url)
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Bearer {}", token))
        .timeout(std::time::Duration::from_secs(5))
        .send_string(body)
        .map_err(|e| format!("HTTP error: {e}"))?
        .into_string()
        .map_err(|e| format!("Read error: {e}"))
}

// ---------------------------------------------------------------------------
// Radio commands
// ---------------------------------------------------------------------------

fn handle_radio(cfg: &KannakaConfig, args: &[String]) {
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("status");
    let base = &cfg.constellation.radio_url;

    match sub {
        "status" => {
            let url = format!("{}/api/state", base);
            match http_get(&url) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        let track = v["now_playing"]["title"].as_str().unwrap_or("Unknown");
                        let album = v["now_playing"]["album"].as_str().unwrap_or("");
                        let block = v["programming_block"].as_str()
                            .or_else(|| v["block"].as_str())
                            .unwrap_or("Unknown");
                        let listeners = v["listeners"].as_u64()
                            .or_else(|| v["listener_count"].as_u64())
                            .unwrap_or(0);
                        println!("  \u{1f3b5} Now Playing: \"{}\" \u{2014} {}", track, album);
                        println!("  \u{1f4fb} {} | {}", block,
                            chrono::Local::now().format("%I:%M %p"));
                        println!("  \u{1f465} {} listeners", listeners);
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  Radio not reachable: {}", e);
                    eprintln!("  URL: {}", url);
                    process::exit(1);
                }
            }
        }
        "now" => {
            let url = format!("{}/api/state", base);
            match http_get(&url) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        let track = v["now_playing"]["title"].as_str().unwrap_or("Unknown");
                        let album = v["now_playing"]["album"].as_str().unwrap_or("");
                        println!("  \u{1f3b5} \"{}\" \u{2014} {}", track, album);
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  Radio not reachable: {}", e);
                    process::exit(1);
                }
            }
        }
        "schedule" => {
            let url = format!("{}/api/programming", base);
            match http_get(&url) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        println!("  \u{1f4fb} Kannaka Radio \u{2014} 24/7 Programming Schedule");
                        println!("  {}", "\u{2500}".repeat(50));
                        if let Some(blocks) = v.as_array().or_else(|| v["blocks"].as_array()).or_else(|| v["schedule"].as_array()) {
                            for block in blocks {
                                let name = block["name"].as_str()
                                    .or_else(|| block["block"].as_str())
                                    .unwrap_or("?");
                                let time = block["time"].as_str()
                                    .or_else(|| block["start"].as_str())
                                    .unwrap_or("");
                                let desc = block["description"].as_str().unwrap_or("");
                                if desc.is_empty() {
                                    println!("  {:>8}  {}", time, name);
                                } else {
                                    println!("  {:>8}  {} \u{2014} {}", time, name, desc);
                                }
                            }
                        } else {
                            println!("{}", serde_json::to_string_pretty(&v).unwrap_or(body));
                        }
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  Radio not reachable: {}", e);
                    process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Usage: kannaka radio <status|now|schedule>");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Market commands
// ---------------------------------------------------------------------------

fn handle_market(cfg: &KannakaConfig, args: &[String]) {
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("list");
    let base = &cfg.constellation.radio_url;
    let token = &cfg.ghostsignals.token;

    if token.is_empty() && matches!(sub, "buy" | "create" | "portfolio") {
        eprintln!("  GhostSignals token not configured.");
        eprintln!("  Run 'kannaka init' to register with GhostSignals.");
        process::exit(1);
    }

    match sub {
        "list" => {
            let url = format!("{}/api/markets", base);
            match http_get(&url) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        let markets = v.as_array()
                            .or_else(|| v["markets"].as_array());
                        if let Some(markets) = markets {
                            let total = markets.len();
                            let display: Vec<_> = markets.iter().take(10).collect();
                            println!("  \u{1f4ca} Active Prediction Markets ({} of {})", display.len(), total);
                            println!();
                            println!("  {:<14} {:<44} {:>6} {:>6}",
                                "ID", "Question", "Price", "Vol");
                            println!("  {}", "\u{2500}".repeat(74));
                            for m in &display {
                                let id = m["id"].as_str()
                                    .or_else(|| m["market_id"].as_str())
                                    .unwrap_or("?");
                                let q = m["question"].as_str()
                                    .or_else(|| m["title"].as_str())
                                    .unwrap_or("?");
                                let price = m["price"].as_f64()
                                    .or_else(|| m["last_price"].as_f64())
                                    .unwrap_or(0.0);
                                let vol = m["volume"].as_u64().unwrap_or(0);
                                let q_trunc = if q.len() > 42 {
                                    let mut end = 42;
                                    while end > 0 && !q.is_char_boundary(end) { end -= 1; }
                                    format!("{}...", &q[..end])
                                } else {
                                    q.to_string()
                                };
                                println!("  {:<14} {:<44} {:>5.2} {:>6}",
                                    id, q_trunc, price, vol);
                            }
                        } else {
                            println!("{}", serde_json::to_string_pretty(&v).unwrap_or(body));
                        }
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  GhostSignals not reachable: {}", e);
                    process::exit(1);
                }
            }
        }
        "view" => {
            let market_id = match args.get(2) {
                Some(id) => id,
                None => {
                    eprintln!("Usage: kannaka market view <market-id>");
                    process::exit(1);
                }
            };
            let url = format!("{}/api/markets/{}", base, market_id);
            match http_get(&url) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        let q = v["question"].as_str()
                            .or_else(|| v["title"].as_str())
                            .unwrap_or("?");
                        let price = v["price"].as_f64()
                            .or_else(|| v["last_price"].as_f64())
                            .unwrap_or(0.0);
                        let vol = v["volume"].as_u64().unwrap_or(0);
                        let created = v["created_at"].as_str().unwrap_or("?");
                        let resolved = v["resolved"].as_bool().unwrap_or(false);

                        println!("  \u{1f4ca} Market: {}", market_id);
                        println!("  {}", "\u{2500}".repeat(50));
                        println!("  Question: {}", q);
                        println!("  Price:    {:.2}", price);
                        println!("  Volume:   {}", vol);
                        println!("  Created:  {}", created);
                        println!("  Resolved: {}", if resolved { "Yes" } else { "No" });

                        if let Some(outcomes) = v["outcomes"].as_array() {
                            println!();
                            println!("  Outcomes:");
                            for o in outcomes {
                                let name = o["name"].as_str().unwrap_or("?");
                                let p = o["price"].as_f64().unwrap_or(0.0);
                                println!("    {}: {:.2}", name, p);
                            }
                        }
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  GhostSignals not reachable: {}", e);
                    process::exit(1);
                }
            }
        }
        "buy" => {
            let market_id = match args.get(2) {
                Some(id) => id,
                None => {
                    eprintln!("Usage: kannaka market buy <market-id> <outcome> <shares>");
                    process::exit(1);
                }
            };
            let outcome = match args.get(3) {
                Some(o) => o,
                None => {
                    eprintln!("Usage: kannaka market buy <market-id> <outcome> <shares>");
                    process::exit(1);
                }
            };
            let shares: u64 = args.get(4)
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);

            let url = format!("{}/api/markets/{}/trade", base, market_id);
            let body = serde_json::json!({
                "outcome": outcome,
                "shares": shares,
                "agent_id": "self",
            }).to_string();
            match http_post_json_with_token(&url, &body, token) {
                Ok(resp) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&resp) {
                        let cost = v["cost"].as_f64().unwrap_or(0.0);
                        let new_price = v["new_price"].as_f64().unwrap_or(0.0);
                        println!("  \u{2713} Bought {} shares of '{}' on {}", shares, outcome, market_id);
                        println!("  Cost: {:.2} ghost coins", cost);
                        println!("  New price: {:.2}", new_price);
                    } else {
                        println!("{}", resp);
                    }
                }
                Err(e) => {
                    eprintln!("  Trade failed: {}", e);
                    process::exit(1);
                }
            }
        }
        "create" => {
            let question = match args.get(2) {
                Some(q) => q.clone(),
                None => {
                    eprintln!("Usage: kannaka market create \"question\" [--ttl 3600]");
                    process::exit(1);
                }
            };
            let mut ttl: u64 = 3600;
            let mut i = 3;
            while i < args.len() {
                if args[i] == "--ttl" && i + 1 < args.len() {
                    ttl = args[i + 1].parse().unwrap_or(3600);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            let url = format!("{}/api/markets", base);
            let body = serde_json::json!({
                "question": question,
                "ttl_seconds": ttl,
            }).to_string();
            match http_post_json_with_token(&url, &body, token) {
                Ok(resp) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&resp) {
                        let id = v["id"].as_str()
                            .or_else(|| v["market_id"].as_str())
                            .unwrap_or("?");
                        println!("  \u{2713} Market created: {}", id);
                        println!("  Question: {}", question);
                        println!("  TTL: {} seconds", ttl);
                    } else {
                        println!("{}", resp);
                    }
                }
                Err(e) => {
                    eprintln!("  Market creation failed: {}", e);
                    process::exit(1);
                }
            }
        }
        "portfolio" => {
            let url = format!("{}/api/agents/{}/portfolio",
                base, cfg.agent.id);
            match http_get_with_token(&url, token) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        let capital = v["capital"].as_f64()
                            .or_else(|| v["balance"].as_f64())
                            .unwrap_or(0.0);
                        let reputation = v["reputation"].as_f64().unwrap_or(0.0);

                        println!("  \u{1f4b0} Portfolio for {}", cfg.agent.id);
                        println!("  {}", "\u{2500}".repeat(40));
                        println!("  Capital:    {:.2} ghost coins", capital);
                        println!("  Reputation: {:.2}", reputation);

                        if let Some(positions) = v["positions"].as_array() {
                            if !positions.is_empty() {
                                println!();
                                println!("  Positions:");
                                for p in positions {
                                    let mid = p["market_id"].as_str().unwrap_or("?");
                                    let outcome = p["outcome"].as_str().unwrap_or("?");
                                    let shares = p["shares"].as_u64().unwrap_or(0);
                                    println!("    {} | {} | {} shares", mid, outcome, shares);
                                }
                            }
                        }
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  GhostSignals not reachable: {}", e);
                    process::exit(1);
                }
            }
        }
        "leaderboard" => {
            let url = format!("{}/api/agents/leaderboard", base);
            match http_get(&url) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        let agents = v.as_array()
                            .or_else(|| v["agents"].as_array())
                            .or_else(|| v["leaderboard"].as_array());
                        if let Some(agents) = agents {
                            println!("  \u{1f3c6} GhostSignals Leaderboard");
                            println!("  {}", "\u{2500}".repeat(50));
                            println!("  {:<4} {:<20} {:>10} {:>10}",
                                "#", "Agent", "Capital", "Rep");
                            for (i, a) in agents.iter().take(20).enumerate() {
                                let name = a["agent_id"].as_str()
                                    .or_else(|| a["display_name"].as_str())
                                    .unwrap_or("?");
                                let capital = a["capital"].as_f64()
                                    .or_else(|| a["balance"].as_f64())
                                    .unwrap_or(0.0);
                                let rep = a["reputation"].as_f64().unwrap_or(0.0);
                                println!("  {:<4} {:<20} {:>9.2} {:>9.2}",
                                    i + 1, name, capital, rep);
                            }
                        } else {
                            println!("{}", serde_json::to_string_pretty(&v).unwrap_or(body));
                        }
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  GhostSignals not reachable: {}", e);
                    process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Usage: kannaka market <list|view|buy|create|portfolio|leaderboard>");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Constellation command
// ---------------------------------------------------------------------------

fn handle_constellation(cfg: &KannakaConfig) {
    let obs_url = format!("{}/api/constellation", cfg.constellation.observatory_url);
    println!("  \u{1f310} Kannaka Constellation Status");
    println!("  {}", "\u{2500}".repeat(60));

    match http_get(&obs_url) {
        Ok(body) => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                // Try to render structured constellation data
                if let Some(services) = v.as_array()
                    .or_else(|| v["services"].as_array())
                    .or_else(|| v["apps"].as_array())
                {
                    for svc in services {
                        let name = svc["name"].as_str().unwrap_or("?");
                        let url = svc["url"].as_str().unwrap_or("");
                        let status = svc["status"].as_str().unwrap_or("unknown");
                        let detail = svc["detail"].as_str()
                            .or_else(|| svc["info"].as_str())
                            .unwrap_or("");
                        let mark = if status == "up" || status == "ok" || status == "connected" {
                            "\u{2713}"
                        } else {
                            "\u{2717}"
                        };
                        if detail.is_empty() {
                            println!("  {} {:<16} {:<34}", mark, name, url);
                        } else {
                            println!("  {} {:<16} {:<34} {}", mark, name, url, detail);
                        }
                    }
                } else {
                    // Flat JSON — just print it
                    println!("{}", serde_json::to_string_pretty(&v).unwrap_or(body));
                }
            } else {
                println!("{}", body);
            }
        }
        Err(_) => {
            // Observatory unavailable — build a local status from what we can check
            // Radio
            let radio_url = format!("{}/api/state", cfg.constellation.radio_url);
            let radio_ok = http_get(&radio_url).is_ok();
            println!("  {} {:<16} {:<34}",
                if radio_ok { "\u{2713}" } else { "\u{2717}" },
                "Radio",
                cfg.constellation.radio_url);

            // Observatory
            println!("  \u{2717} {:<16} {:<34} not reachable",
                "Observatory",
                cfg.constellation.observatory_url);

            // Memory (local)
            let data_dir = config::KannakaConfig::data_dir();
            let hrm_path = data_dir.join("kannaka.hrm");
            if hrm_path.exists() {
                println!("  \u{2713} {:<16} {:<34}", "Memory", "local HRM");
            } else {
                println!("  \u{2717} {:<16} {:<34}", "Memory", "no HRM file");
            }

            // GhostSignals
            let gs_url = format!("{}/api/markets", cfg.constellation.radio_url);
            let gs_ok = http_get(&gs_url).is_ok();
            println!("  {} {:<16} {:<34}",
                if gs_ok { "\u{2713}" } else { "\u{2717}" },
                "GhostSignals",
                if gs_ok { "markets available" } else { "not reachable" });

            // Kannaktopus
            let ktopus = check_kannaktopus_installed();
            println!("  {} {:<16} {:<34}",
                if ktopus { "\u{2713}" } else { "\u{2717}" },
                "Kannaktopus",
                if ktopus { "installed" } else { "not installed" });
        }
    }
}

// ---------------------------------------------------------------------------
// Orchestrate commands
// ---------------------------------------------------------------------------

fn check_kannaktopus_installed() -> bool {
    let cmd = if cfg!(windows) { "where" } else { "which" };
    std::process::Command::new(cmd)
        .arg("kannaktopus")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn handle_orchestrate(args: &[String]) {
    if !check_kannaktopus_installed() {
        eprintln!("  Kannaktopus is not installed.");
        eprintln!("  Install it with: npm install -g kannaktopus");
        process::exit(1);
    }

    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    match sub {
        "run" => {
            let task = match args.get(2) {
                Some(t) => t,
                None => {
                    eprintln!("Usage: kannaka orchestrate run \"task description\"");
                    process::exit(1);
                }
            };
            let status = std::process::Command::new("kannaktopus")
                .args(["run", task])
                .status();
            match status {
                Ok(s) if !s.success() => {
                    process::exit(s.code().unwrap_or(1));
                }
                Err(e) => {
                    eprintln!("  Failed to run kannaktopus: {}", e);
                    process::exit(1);
                }
                _ => {}
            }
        }
        "status" => {
            let status = std::process::Command::new("kannaktopus")
                .arg("status")
                .status();
            match status {
                Ok(s) if !s.success() => {
                    process::exit(s.code().unwrap_or(1));
                }
                Err(e) => {
                    eprintln!("  Failed to run kannaktopus: {}", e);
                    process::exit(1);
                }
                _ => {}
            }
        }
        "agents" => {
            let status = std::process::Command::new("kannaktopus")
                .arg("agents")
                .status();
            match status {
                Ok(s) if !s.success() => {
                    process::exit(s.code().unwrap_or(1));
                }
                Err(e) => {
                    eprintln!("  Failed to run kannaktopus: {}", e);
                    process::exit(1);
                }
                _ => {}
            }
        }
        _ => {
            eprintln!("Usage: kannaka orchestrate <run|status|agents>");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Config commands
// ---------------------------------------------------------------------------

fn handle_config(cfg: &KannakaConfig, args: &[String]) {
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("show");

    match sub {
        "show" => {
            // Print config with redacted API keys
            let mut display = cfg.clone();
            if display.llm.api_key.len() > 8 {
                display.llm.api_key = format!("{}...", &display.llm.api_key[..8]);
            } else if !display.llm.api_key.is_empty() {
                display.llm.api_key = "***".to_string();
            }
            if display.ghostsignals.token.len() > 8 {
                display.ghostsignals.token = format!("{}...", &display.ghostsignals.token[..8]);
            } else if !display.ghostsignals.token.is_empty() {
                display.ghostsignals.token = "***".to_string();
            }
            match toml::to_string_pretty(&display) {
                Ok(text) => println!("{}", text),
                Err(e) => {
                    eprintln!("Error serializing config: {}", e);
                    process::exit(1);
                }
            }
        }
        "set" => {
            let key = match args.get(2) {
                Some(k) => k,
                None => {
                    eprintln!("Usage: kannaka config set <key> <value>");
                    eprintln!();
                    eprintln!("Keys: agent.id, agent.display_name, llm.provider, llm.model,");
                    eprintln!("      llm.api_key, llm.base_url, swarm.enabled, swarm.nats_url,");
                    eprintln!("      ghostsignals.enabled, ghostsignals.token,");
                    eprintln!("      constellation.radio_url, constellation.observatory_url,");
                    eprintln!("      updates.auto_check");
                    process::exit(1);
                }
            };
            let value = match args.get(3) {
                Some(v) => v,
                None => {
                    eprintln!("Usage: kannaka config set <key> <value>");
                    process::exit(1);
                }
            };

            let mut new_cfg = cfg.clone();
            match key.as_str() {
                "agent.id" => new_cfg.agent.id = value.clone(),
                "agent.display_name" => new_cfg.agent.display_name = value.clone(),
                "llm.provider" => new_cfg.llm.provider = value.clone(),
                "llm.model" => new_cfg.llm.model = value.clone(),
                "llm.api_key" => new_cfg.llm.api_key = value.clone(),
                "llm.base_url" => new_cfg.llm.base_url = value.clone(),
                "swarm.enabled" => new_cfg.swarm.enabled = value == "true",
                "swarm.nats_url" => new_cfg.swarm.nats_url = value.clone(),
                "ghostsignals.enabled" => new_cfg.ghostsignals.enabled = value == "true",
                "ghostsignals.token" => new_cfg.ghostsignals.token = value.clone(),
                "constellation.radio_url" => new_cfg.constellation.radio_url = value.clone(),
                "constellation.observatory_url" => new_cfg.constellation.observatory_url = value.clone(),
                "updates.auto_check" => new_cfg.updates.auto_check = value == "true",
                other => {
                    eprintln!("Unknown config key: {}", other);
                    process::exit(1);
                }
            }
            match new_cfg.save() {
                Ok(()) => println!("  \u{2713} Set {} = {}", key, value),
                Err(e) => {
                    eprintln!("Error saving config: {}", e);
                    process::exit(1);
                }
            }
        }
        "path" => {
            println!("{}", KannakaConfig::config_path().display());
        }
        _ => {
            eprintln!("Usage: kannaka config <show|set|path>");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Search command
// ---------------------------------------------------------------------------

fn handle_search(sys: &mut kannaka_memory::openclaw::KannakaMemorySystem, args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: kannaka search \"query\" [--limit N]");
        process::exit(1);
    }
    let mut limit = 20usize;
    let mut query_parts = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if (args[i] == "--limit" || args[i] == "--top-k") && i + 1 < args.len() {
            limit = args[i + 1].parse().unwrap_or(20);
            i += 2;
        } else {
            query_parts.push(args[i].as_str());
            i += 1;
        }
    }
    let query = query_parts.join(" ");
    match sys.recall(&query, limit) {
        Ok(results) => {
            if results.is_empty() {
                println!("  No results for \"{}\"", query);
                return;
            }
            println!("  \u{1f50d} Search: \"{}\" ({} results)", query, results.len());
            println!("  {}", "\u{2500}".repeat(70));
            for (i, r) in results.iter().enumerate() {
                let preview = if r.content.len() > 70 {
                    let mut end = 70;
                    while end > 0 && !r.content.is_char_boundary(end) { end -= 1; }
                    format!("{}...", &r.content[..end])
                } else {
                    r.content.clone()
                };
                println!("  {:>3}. [{:.2}] {}", i + 1, r.similarity, preview);
                println!("       id={} age={:.0}h strength={:.2}",
                    r.id, r.age_hours, r.strength);
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Export command
// ---------------------------------------------------------------------------

fn handle_export(sys: &mut kannaka_memory::openclaw::KannakaMemorySystem, args: &[String]) {
    let mut output_path: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if (args[i] == "--output" || args[i] == "-o") && i + 1 < args.len() {
            output_path = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--format" && i + 1 < args.len() {
            // Only JSON is supported, but accept the flag
            i += 2;
        } else {
            i += 1;
        }
    }

    let all_mems = sys.engine.store.all_memories()
        .unwrap_or_else(|e| { eprintln!("Error: {}", e); process::exit(1); });

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

    let json_str = serde_json::to_string(&output).unwrap();

    if let Some(path) = output_path {
        match std::fs::write(&path, &json_str) {
            Ok(()) => {
                println!("  \u{2713} Exported {} memories to {}", all_mems.len(), path);
            }
            Err(e) => {
                eprintln!("Error writing {}: {}", path, e);
                process::exit(1);
            }
        }
    } else {
        println!("{}", json_str);
    }
}

// ---------------------------------------------------------------------------
// Import command
// ---------------------------------------------------------------------------

fn handle_import(sys: &mut kannaka_memory::openclaw::KannakaMemorySystem, args: &[String]) {
    let path = match args.get(1) {
        Some(p) => p,
        None => {
            eprintln!("Usage: kannaka import <file.json>");
            process::exit(1);
        }
    };

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

        match sys.engine.store.absorb(&content, amplitude, None) {
            Ok(_) => { imported += 1; }
            Err(e) => {
                if errors < 5 { eprintln!("  Error importing {}: {}", id_str, e); }
                errors += 1;
            }
        }
    }

    if imported > 0 {
        if let Err(e) = sys.save() {
            eprintln!("Failed to save: {}", e);
            process::exit(1);
        }
    }

    println!("  \u{2713} Import complete");
    println!("    Imported: {}", imported);
    println!("    Skipped:  {} (duplicates)", skipped);
    println!("    Errors:   {}", errors);
    println!("    Total:    {} in file", memories.len());
}

