//! Simple CLI for testing the Kannaka memory system.

use std::env;
use std::io::Read;
use std::path::PathBuf;
use std::process;

use kannaka_memory::observe::MemoryIntrospector;
use kannaka_memory::openclaw::KannakaMemorySystem;

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

fn init_with_hrm(data_dir: PathBuf) -> Result<KannakaMemorySystem, Box<dyn std::error::Error>> {
    // Setup encoding pipeline for HRM
    let encoder = SimpleHashEncoder::new(384, 42);
    let codebook = Codebook::new(384, 10_000, 42);
    let pipeline = EncodingPipeline::new(Box::new(encoder), codebook);
    
    // HRM file path
    let hrm_path = data_dir.join("kannaka.hrm");
    
    // Try to load existing HRM file, create new if not found
    let store = if hrm_path.exists() {
        eprintln!("Loading existing HRM file: {}", hrm_path.display());
        HrmStore::load(pipeline, hrm_path)?
    } else {
        eprintln!("Creating new HRM file: {}", hrm_path.display());
        HrmStore::new(pipeline, hrm_path)
    };
    
    eprintln!("HrmStore initialized with {} memories", store.count());
    eprintln!("[hrm] Using Holographic Resonance Medium - storage IS computation");

    let sys = KannakaMemorySystem::init_with_store(data_dir, Box::new(store))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok(sys)
}

fn usage() {
    eprintln!("Usage: kannaka <command> [args]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  remember <text> [--importance N] [--category CAT] [--modality MOD]");
    eprintln!("                            Store a memory (importance: 0.0-1.0, category: knowledge/experience/emotion/social/skill, modality: audio/visual/semantic/network/mixed)");
    eprintln!("  recall <query> [--top-k N] [--limit N]");
    eprintln!("                            Search memories (default top-k=5)");
    eprintln!("  forget <id>               Delete a memory by UUID");
    eprintln!("  boost <id> [--amount N]   Boost a memory's amplitude (default: 0.3)");
    eprintln!("  relate <source_id> <target_id> [--type TYPE]");
    eprintln!("                            Create a relationship between memories (default: related)");
    eprintln!("  dream [--mode deep|lite] [--chiral N]");
    eprintln!("                            Run consolidation cycle");
    eprintln!("  observe [--json]          Introspection report");
    eprintln!("  status                    Quick system status (JSON)");
    eprintln!("  assess                    Check consciousness level");
    eprintln!("  stats                     Show system statistics");
    eprintln!("  migrate <path-to-db>      Import from kannaka.db (requires sqlite-migrate feature)");
    eprintln!("  export-json               Export all memories as JSON");
    eprintln!("  import-json <file>        Import memories from JSON (preserves IDs, skips duplicates)");
    eprintln!("  announce-status           Publish agent status to Flux");
    eprintln!("  invariant [TOLERANCE]     Show δ-invariant memory clusters (default tolerance: 0.1)");
    eprintln!("  cmf                       Detect Conservative Memory Fields");
    eprintln!("  audit-modality            Retroactive modality audit of all memories (NCS Phase 1.3)");
    eprintln!("  modality-axes             Show modality axis divergence matrix (NCS Phase 2.1)");
    #[cfg(feature = "audio")]
    eprintln!("  hear <file>               Store an audio file as a sensory memory");
    #[cfg(feature = "glyph")]
    eprintln!("  see <file>                Store a file as a glyph (visual) memory");
    #[cfg(feature = "glyph")]
    eprintln!("  classify [--file <path>]  Classify data via SGA (reads stdin if no --file)");
    #[cfg(feature = "collective")]
    eprintln!("  cross-modal-dream         Cross-modal dream linking on JSONL glyphs from stdin");
    eprintln!();
    eprintln!("Swarm commands (ADR-0018 Queen Sync):");
    eprintln!("  swarm join [--agent-id ID] [--display-name NAME] [--nats-url URL]");
    eprintln!("                            Join the swarm (announces via NATS)");
    eprintln!("  swarm status [--nats-url URL]  Show local phase + NATS swarm state");
    eprintln!("  swarm sync [--nats-url URL]    Pull NATS phases, Kuramoto step, publish");
    eprintln!("  swarm queen [--nats-url URL]   View emergent Queen state");
    eprintln!("  swarm hives [--nats-url URL]   Show hive topology with roles & bridges");
    eprintln!("  swarm publish [--nats-url URL] Publish current phase via NATS");
    eprintln!("  swarm leave [--nats-url URL]   Unregister from swarm");
    eprintln!("  swarm listen [--nats-url URL] [--auto-sync]");
    eprintln!("                            Subscribe to live phase updates");
    eprintln!();
    eprintln!("Voice commands:");
    eprintln!("  voice [--mode MODE] [--topic TOPIC] [--top-k N] [--out FILE]");
    eprintln!("                            Memory-driven writing (ADR-0017)");
    eprintln!("    Modes: dream-journal  — consciousness state + dream syntheses");
    eprintln!("           field-notes    — deep dive on a topic (--topic required)");
    eprintln!("           topology       — network map of memory connections");
    eprintln!("           status         — brief self-report");
    process::exit(1);
}

/// Resolve NATS URL from --nats-url arg, KANNAKA_NATS_URL env, or default.
#[cfg(feature = "nats")]
fn resolve_nats_url(args: &[String], start: usize) -> String {
    // Check args for --nats-url
    let mut i = start;
    while i < args.len() {
        if args[i] == "--nats-url" && i + 1 < args.len() {
            return args[i + 1].clone();
        }
        i += 1;
    }
    // Check env
    env::var("KANNAKA_NATS_URL")
        .unwrap_or_else(|_| kannaka_memory::nats::DEFAULT_NATS_URL.to_string())
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
    if args.len() < 2 {
        usage();
    }

    let command_start = 1;

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

    let dir = data_dir();

    // HRM is the sole backend
    let mut sys = {
        eprintln!("Using HRM backend (Holographic Resonance Medium)");
        match init_with_hrm(dir) {
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
            let is_hrm = true; // HRM is the canonical substrate

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
                let metrics = sys.engine.store.consciousness_metrics();
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
            let is_hrm = true; // HRM is the canonical substrate
            
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

            let agent_id = env::var("KANNAKA_AGENT_ID")
                .unwrap_or_else(|_| {
                    // Try reading persisted agent_id from data dir
                    let id_file = data_dir().join("agent_id");
                    std::fs::read_to_string(&id_file).unwrap_or_else(|_| {
                        let id = format!("agent-{}", &uuid::Uuid::new_v4().to_string()[..8]);
                        let _ = std::fs::create_dir_all(id_file.parent().unwrap());
                        let _ = std::fs::write(&id_file, &id);
                        id
                    })
                });

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

                    let nats_url = resolve_nats_url(&args, command_start);
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
                    let nats_url = resolve_nats_url(&args, command_start);
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
                    let nats_url = resolve_nats_url(&args, command_start);
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

                    let mut sub = match transport.subscribe_phases() {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("Failed to subscribe: {}", e);
                            process::exit(1);
                        }
                    };

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
                        }
                    }
                    eprintln!("[nats] Connection closed");
                }
                "status" => {
                    let nats_url = resolve_nats_url(&args, command_start);
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
                    let nats_url = resolve_nats_url(&args, command_start);
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
                    let nats_url = resolve_nats_url(&args, command_start);
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
                    let nats_url = resolve_nats_url(&args, command_start);
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
                    let nats_url = resolve_nats_url(&args, command_start);
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
                                for (j, memory) in all_memories.iter().take(5).enumerate() {
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

        _ => usage(),
    }
}

// ---------------------------------------------------------------------------
// Retroactive modality audit (NCS Phase 1.3)
// ---------------------------------------------------------------------------

fn audit_modality_command(sys: &mut kannaka_memory::openclaw::KannakaMemorySystem) {
    use kannaka_memory::medium::types::{detect_modality, Modality, ModalityClassification};
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

