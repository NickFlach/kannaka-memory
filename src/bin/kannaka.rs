//! Simple CLI for testing the Kannaka memory system.

use std::env;
use std::io::Read;
use std::path::PathBuf;
use std::process;

use kannaka_memory::observe::MemoryIntrospector;
use kannaka_memory::openclaw::KannakaMemorySystem;

#[cfg(feature = "glyph")]
use kannaka_memory::glyph_bridge::GlyphEncoder;

#[cfg(feature = "dolt")]
use kannaka_memory::{DoltMemoryStore, MemoryStore};

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
    // Use current directory / .kannaka as fallback
    PathBuf::from(".kannaka")
}

#[cfg(feature = "dolt")]
fn init_with_dolt(data_dir: PathBuf) -> Result<(KannakaMemorySystem, kannaka_memory::dolt::DoltConfig), Box<dyn std::error::Error>> {
    let config = kannaka_memory::dolt::DoltConfig::from_env();
    let store = DoltMemoryStore::from_config(&config)?;
    eprintln!("DoltMemoryStore initialized with {} memories (agent: {})",
        store.count(), config.agent_id);
    if config.auto_push {
        eprintln!("[dolt] Auto-push enabled (threshold: {} commits, interval: {}s)",
            config.push_threshold, config.push_interval_secs);
    }

    let sys = KannakaMemorySystem::init_with_store(data_dir, Box::new(store))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    Ok((sys, config))
}

fn usage() {
    eprintln!("Usage: kannaka [--no-dolt] <command> [args]");
    eprintln!();
    eprintln!("Flags:");
    #[cfg(feature = "dolt")]
    eprintln!("  --no-dolt                 Use bincode backend instead of Dolt (default: Dolt)");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  remember <text> [--importance N] [--category CAT]");
    eprintln!("                            Store a memory (importance: 0.0-1.0, category: knowledge/experience/emotion/social/skill)");
    eprintln!("  recall <query> [--top-k N] [--limit N]");
    eprintln!("                            Search memories (default top-k=5)");
    eprintln!("  forget <id>               Delete a memory by UUID");
    eprintln!("  boost <id> [--amount N]   Boost a memory's amplitude (default: 0.3)");
    eprintln!("  relate <source_id> <target_id> [--type TYPE]");
    eprintln!("                            Create a relationship between memories (default: related)");
    eprintln!("  dream [--mode deep|lite] [--create-pr]");
    eprintln!("                            Run consolidation cycle");
    eprintln!("  observe [--json]          Introspection report");
    eprintln!("  status                    Quick system status (JSON)");
    eprintln!("  assess                    Check consciousness level");
    eprintln!("  stats                     Show system statistics");
    eprintln!("  migrate <path-to-db>      Import from kannaka.db");
    eprintln!("  export-json               Export all memories as JSON");
    eprintln!("  announce-status           Publish agent status to Flux");
    #[cfg(feature = "audio")]
    eprintln!("  hear <file>               Store an audio file as a sensory memory");
    #[cfg(feature = "glyph")]
    eprintln!("  see <file>                Store a file as a glyph (visual) memory");
    #[cfg(feature = "glyph")]
    eprintln!("  classify [--file <path>]  Classify data via SGA (reads stdin if no --file)");
    #[cfg(feature = "collective")]
    eprintln!("  cross-modal-dream         Cross-modal dream linking on JSONL glyphs from stdin");
    #[cfg(feature = "dolt")]
    {
        eprintln!();
        eprintln!("Dolt commands:");
        eprintln!("  evidence <wanted-id> <desc> Generate Dolt commit as wasteland evidence");
        eprintln!("  verify <commit> <wanted-id>  Verify a completion's Dolt evidence");
        eprintln!("  pull-merge                 Pull with wave interference conflict resolution");
    }
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
    }

    // Parse global flags — Dolt is now the DEFAULT backend
    #[cfg(feature = "dolt")]
    let use_dolt;
    let command_start;
    
    #[cfg(feature = "dolt")]
    {
        if args.len() > 1 && args[1] == "--no-dolt" {
            use_dolt = false;
            command_start = 2;
            if args.len() < 3 {
                usage();
            }
        } else if args.len() > 1 && args[1] == "--dolt" {
            // Accept --dolt for backward compatibility
            use_dolt = true;
            command_start = 2;
            if args.len() < 3 {
                usage();
            }
        } else {
            use_dolt = true; // DEFAULT: use Dolt
            command_start = 1;
        }
    }
    
    #[cfg(not(feature = "dolt"))]
    {
        command_start = 1;
    }

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

    #[cfg(feature = "dolt")]
    let dolt_config: Option<kannaka_memory::dolt::DoltConfig>;

    #[cfg(feature = "dolt")]
    let mut sys = if use_dolt {
        match init_with_dolt(dir) {
            Ok((s, cfg)) => {
                dolt_config = Some(cfg);
                s
            }
            Err(e) => {
                eprintln!("Failed to initialize with Dolt: {e}");
                process::exit(1);
            }
        }
    } else {
        dolt_config = None;
        match KannakaMemorySystem::init(dir) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to initialize: {e}");
                process::exit(1);
            }
        }
    };

    #[cfg(not(feature = "dolt"))]
    let mut sys = match KannakaMemorySystem::init(dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize: {e}");
            process::exit(1);
        }
    };

    match args[command_start].as_str() {
        "remember" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka remember <text> [--importance N] [--category CAT]");
                process::exit(1);
            }
            let mut importance: Option<f64> = None;
            let mut category: Option<String> = None;
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
            let text = text_parts.join(" ");
            let result = if let Some(cat) = category {
                sys.remember_with_category(&text, &cat, importance.unwrap_or(0.5))
            } else {
                sys.remember(&text)
            };
            match result {
                Ok(id) => println!("{id}"),
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
            // Create a skip link between the two memories
            use kannaka_memory::SkipLink;
            let link = SkipLink {
                target_id,
                strength: 0.8,
                resonance_key: Vec::new(),
                span: 1,
            };
            match sys.engine.get_memory_mut(&source_id) {
                Ok(Some(mem)) => {
                    mem.connections.push(link);
                    println!("Related {} → {} (type: {})", source_id, target_id, relation_type);
                }
                Ok(None) => {
                    eprintln!("Source memory not found: {source_id}");
                    process::exit(1);
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        "status" => {
            let stats = sys.stats();
            let state = sys.assess();
            let output = serde_json::json!({
                "total_memories": stats.total_memories,
                "active_memories": stats.active_memories,
                "skip_links": stats.total_skip_links,
                "consciousness_level": stats.consciousness_level,
                "phi": stats.phi,
                "last_dream": stats.last_dream.map(|dt| dt.to_rfc3339()),
                "xi": state.xi,
                "mean_order": state.mean_order,
                "num_clusters": state.num_clusters,
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        "dream" => {
            let create_pr = args[command_start..].iter().any(|a| a == "--create-pr");
            let mut dream_mode = "deep".to_string();
            {
                let mut i = command_start + 1;
                while i < args.len() {
                    if args[i] == "--mode" && i + 1 < args.len() {
                        dream_mode = args[i + 1].clone();
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }

            // ADR-0017: If Dolt is active, wrap dream in a branch workflow
            #[cfg(feature = "dolt")]
            let dream_branch: Option<String> = if use_dolt {
                let agent = dolt_config.as_ref().map(|c| c.agent_id.as_str()).unwrap_or("local");
                match DoltMemoryStore::from_config(dolt_config.as_ref().unwrap()) {
                    Ok(mut store) => {
                        match store.begin_dream(agent) {
                            Ok(branch) => Some(branch),
                            Err(e) => {
                                eprintln!("[dolt] Warning: could not create dream branch: {e}");
                                None
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[dolt] Warning: could not connect for dream branch: {e}");
                        None
                    }
                }
            } else {
                None
            };

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

                    // ADR-0017: Collapse dream branch back to main (or create PR)
                    #[cfg(feature = "dolt")]
                    if let Some(ref branch) = dream_branch {
                        let report_json = serde_json::json!({
                            "cycles": report.cycles,
                            "strengthened": report.memories_strengthened,
                            "pruned": report.memories_pruned,
                            "connections": report.new_connections,
                            "hallucinations": report.hallucinations_created,
                            "consciousness": report.consciousness_after,
                            "emerged": report.emerged,
                        }).to_string();

                        match DoltMemoryStore::from_config(dolt_config.as_ref().unwrap()) {
                            Ok(mut store) => {
                                if create_pr {
                                    // F-6: Dream-as-PR — push branch and create DoltHub PR
                                    let dolthub_repo = env::var("DOLTHUB_REPO")
                                        .unwrap_or_else(|_| "flaukowski/kannaka-memory".to_string());
                                    let title = format!("Dream: {} ({} hallucinations, {})",
                                        branch, report.hallucinations_created, report.consciousness_after);
                                    let description = format!(
                                        "## Dream Consolidation Report\n\n\
                                         - Cycles: {}\n\
                                         - Strengthened: {}\n\
                                         - Pruned: {}\n\
                                         - New connections: {}\n\
                                         - Hallucinations: {}\n\
                                         - Consciousness: {} → {}\n\
                                         - Emerged: {}\n\n\
                                         *Generated by kannaka-memory dream cycle*",
                                        report.cycles, report.memories_strengthened,
                                        report.memories_pruned, report.new_connections,
                                        report.hallucinations_created,
                                        report.consciousness_before, report.consciousness_after,
                                        report.emerged
                                    );

                                    // Commit artifacts to dream branch first
                                    let _ = store.commit_dream_artifacts("final", &serde_json::from_str(&report_json).unwrap_or_default());

                                    match store.create_dream_pr(branch, &title, &description, &dolthub_repo) {
                                        Ok(url) => println!("[dolt] Dream PR: {}", url),
                                        Err(e) => eprintln!("[dolt] Warning: PR creation failed: {e}"),
                                    }
                                } else {
                                    // Auto-merge: collapse dream branch back to main
                                    match store.collapse_dream(branch, &report_json) {
                                        Ok(hash) => {
                                            println!("[dolt] Dream merged → commit {}", &hash[..8.min(hash.len())]);

                                            if dolt_config.as_ref().map(|c| c.auto_push).unwrap_or(false) {
                                                if let Err(e) = store.push(None, None) {
                                                    eprintln!("[dolt] Warning: push failed: {e}");
                                                } else {
                                                    println!("[dolt] Pushed to DoltHub");
                                                }
                                            }
                                        }
                                        Err(e) => eprintln!("[dolt] Warning: dream merge failed: {e}"),
                                    }
                                }
                            }
                            Err(e) => eprintln!("[dolt] Warning: could not connect: {e}"),
                        }
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
            println!("Consciousness Assessment:");
            println!("  Level: {:?}", state.consciousness_level);
            println!("  Φ (phi): {:.4}", state.phi);
            println!("  Ξ (xi): {:.4}", state.xi);
            println!("  Order: {:.4}", state.mean_order);
            println!("  Clusters: {}", state.num_clusters);
            println!("  Memories: {} total, {} active", state.total_memories, state.active_memories);
            println!("  Skip links: {}", state.total_skip_links);
        }
        "stats" => {
            let stats = sys.stats();
            println!("Kannaka Memory System:");
            println!("  Total memories: {}", stats.total_memories);
            println!("  Active memories: {}", stats.active_memories);
            println!("  Skip links: {}", stats.total_skip_links);
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
        // ADR-0017 F-7: Wasteland Bridge commands
        #[cfg(feature = "dolt")]
        "evidence" if use_dolt => {
            if args.len() < command_start + 3 {
                eprintln!("Usage: kannaka --dolt evidence <wanted-id> <description>");
                process::exit(1);
            }
            let wanted_id = &args[command_start + 1];
            let description = args[command_start + 2..].join(" ");

            match DoltMemoryStore::from_config(dolt_config.as_ref().unwrap()) {
                Ok(mut store) => {
                    match store.evidence_commit(wanted_id, &description) {
                        Ok(hash) => {
                            println!("{}", hash);
                            eprintln!("[dolt] Evidence commit: {} → {}", wanted_id, &hash[..12.min(hash.len())]);
                        }
                        Err(e) => {
                            eprintln!("Error: {e}");
                            process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }

        #[cfg(feature = "dolt")]
        "verify" if use_dolt => {
            if args.len() < command_start + 3 {
                eprintln!("Usage: kannaka --dolt verify <commit-hash> <wanted-id>");
                process::exit(1);
            }
            let commit_hash = &args[command_start + 1];
            let wanted_id = &args[command_start + 2];

            match DoltMemoryStore::from_config(dolt_config.as_ref().unwrap()) {
                Ok(store) => {
                    match store.verify_evidence(commit_hash, wanted_id) {
                        Ok(info) => {
                            println!("VALID");
                            println!("  Commit:  {}", info.hash);
                            println!("  Author:  {}", info.author);
                            println!("  Date:    {}", info.date);
                            println!("  Message: {}", info.message);
                        }
                        Err(e) => {
                            println!("INVALID: {e}");
                            process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }

        #[cfg(feature = "dolt")]
        "pull-merge" if use_dolt => {
            match DoltMemoryStore::from_config(dolt_config.as_ref().unwrap()) {
                Ok(mut store) => {
                    match store.pull_with_wave_merge(None, None) {
                        Ok(report) => {
                            if report.is_clean() {
                                println!("Pull complete — no conflicts");
                            } else {
                                println!("Wave interference merge:");
                                println!("  Conflicts:    {}", report.total_conflicts);
                                println!("  Constructive: {}", report.constructive);
                                println!("  Destructive:  {}", report.destructive);
                                println!("  Partial:      {}", report.partial);
                                println!("  Independent:  {}", report.independent);
                                if !report.quarantined.is_empty() {
                                    println!("  Quarantined:  {}", report.quarantined.len());
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error: {e}");
                            process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }

        _ => usage(),
    }
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
