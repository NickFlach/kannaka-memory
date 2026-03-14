//! Simple CLI for testing the Kannaka memory system.

use std::env;
use std::io::Read;
use std::path::PathBuf;
use std::process;

use kannaka_memory::observe::MemoryIntrospector;
use kannaka_memory::openclaw::KannakaMemorySystem;

#[cfg(feature = "glyph")]
use kannaka_memory::glyph_bridge::GlyphEncoder;

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
    eprintln!("Usage: kannaka <command> [args]");
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
    eprintln!("  migrate-embeddings        Regenerate missing vector embeddings via Ollama");
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
    eprintln!();
    eprintln!("Swarm commands (ADR-0018 Queen Sync):");
    eprintln!("  swarm join [--agent-id ID] [--display-name NAME]  Join the swarm");
    eprintln!("  swarm status              Show local phase + swarm overview");
    eprintln!("  swarm sync                Pull phases, Kuramoto step, push");
    eprintln!("  swarm queen               View emergent Queen state");
    eprintln!("  swarm hives               Hive topology (JSON)");
    eprintln!("  swarm publish             Publish current phase only");
    eprintln!("  swarm leave               Unregister from swarm");
    eprintln!();
    eprintln!("Dolt commands:");
    eprintln!("  evidence <wanted-id> <desc> Generate Dolt commit as wasteland evidence");
    eprintln!("  verify <commit> <wanted-id>  Verify a completion's Dolt evidence");
    eprintln!("  pull-merge                 Pull with wave interference conflict resolution");
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
    }

    // Dolt is the only backend
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

    let dolt_config: kannaka_memory::dolt::DoltConfig;

    let mut sys = match init_with_dolt(dir) {
        Ok((s, cfg)) => {
            dolt_config = cfg;
            s
        }
        Err(e) => {
            eprintln!("Failed to initialize with Dolt: {e}");
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
            // Count memories without embeddings
            let all_mems = sys.engine.store.all_memories().unwrap_or_default();
            let memories_without_embeddings = all_mems.iter().filter(|m| m.vector.is_empty()).count();
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
                "memories_without_embeddings": memories_without_embeddings,
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

            // ADR-0017: Wrap dream in a branch workflow
            let dream_branch: Option<String> = {
                let agent = dolt_config.agent_id.as_str();
                match DoltMemoryStore::from_config(&dolt_config) {
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

                        match DoltMemoryStore::from_config(&dolt_config) {
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

                                            if dolt_config.auto_push {
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
        "evidence" => {
            if args.len() < command_start + 3 {
                eprintln!("Usage: kannaka --dolt evidence <wanted-id> <description>");
                process::exit(1);
            }
            let wanted_id = &args[command_start + 1];
            let description = args[command_start + 2..].join(" ");

            match DoltMemoryStore::from_config(&dolt_config) {
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

        "verify" => {
            if args.len() < command_start + 3 {
                eprintln!("Usage: kannaka --dolt verify <commit-hash> <wanted-id>");
                process::exit(1);
            }
            let commit_hash = &args[command_start + 1];
            let wanted_id = &args[command_start + 2];

            match DoltMemoryStore::from_config(&dolt_config) {
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

        "swarm" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka swarm <join|status|sync|queen|hives|publish|leave>");
                process::exit(1);
            }
            match args[command_start + 1].as_str() {
                "join" => {
                    let mut agent_id = dolt_config.agent_id.clone();
                    let mut display_name = String::new();
                    let mut i = command_start + 2;
                    while i < args.len() {
                        match args[i].as_str() {
                            "--agent-id" if i + 1 < args.len() => { agent_id = args[i + 1].clone(); i += 2; }
                            "--display-name" if i + 1 < args.len() => { display_name = args[i + 1].clone(); i += 2; }
                            "--remote" if i + 1 < args.len() => { i += 2; } // consumed but remote is from dolt config
                            _ => { i += 1; }
                        }
                    }
                    if display_name.is_empty() {
                        display_name = agent_id.clone();
                    }
                    match DoltMemoryStore::from_config(&dolt_config) {
                        Ok(store) => {
                            let agent = kannaka_memory::SwarmAgent {
                                agent_id: agent_id.clone(),
                                display_name: Some(display_name.clone()),
                                trust_score: 0.5,
                                swarm_role: "member".to_string(),
                                protocol_version: "1.0".to_string(),
                                handedness: kannaka_memory::Handedness::Achiral,
                                natural_frequency: 0.5,
                            };
                            match store.register_swarm_agent(&agent) {
                                Ok(()) => {
                                    println!("Joined swarm as '{}' ({})", display_name, agent_id);
                                }
                                Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                            }
                        }
                        Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                    }
                }
                "leave" => {
                    // Unregister by setting swarm_role to 'inactive'
                    let agent_id = dolt_config.agent_id.clone();
                    match DoltMemoryStore::from_config(&dolt_config) {
                        Ok(store) => {
                            let agent = kannaka_memory::SwarmAgent {
                                agent_id: agent_id.clone(),
                                display_name: None,
                                trust_score: 0.0,
                                swarm_role: "inactive".to_string(),
                                protocol_version: "1.0".to_string(),
                                handedness: kannaka_memory::Handedness::Achiral,
                                natural_frequency: 0.0,
                            };
                            match store.register_swarm_agent(&agent) {
                                Ok(()) => println!("Left swarm ({})", agent_id),
                                Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                            }
                        }
                        Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                    }
                }
                "status" => {
                    match DoltMemoryStore::from_config(&dolt_config) {
                        Ok(store) => {
                            let phases = store.read_swarm_phases(std::time::Duration::from_secs(24 * 3600)).unwrap_or_default();
                            let agents = store.read_swarm_agents().unwrap_or_default();
                            let queen_state = store.read_queen_state().unwrap_or(None);
                            let my_phase = phases.iter().find(|p| p.agent_id == dolt_config.agent_id);
                            let output = serde_json::json!({
                                "agent_id": dolt_config.agent_id,
                                "local_phase": my_phase.map(|p| serde_json::json!({
                                    "phase": p.phase, "frequency": p.frequency,
                                    "coherence": p.coherence, "phi": p.phi,
                                    "memory_count": p.memory_count,
                                })),
                                "swarm": {
                                    "agent_count": agents.len(),
                                    "active_phases": phases.len(),
                                },
                                "queen": queen_state.as_ref().map(|q| serde_json::json!({
                                    "order_parameter": q.order_parameter,
                                    "mean_phase": q.mean_phase,
                                    "phi": q.phi,
                                    "coherence": q.coherence,
                                })),
                            });
                            println!("{}", serde_json::to_string_pretty(&output).unwrap());
                        }
                        Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                    }
                }
                "sync" => {
                    match DoltMemoryStore::from_config(&dolt_config) {
                        Ok(store) => {
                            let phases = match store.read_swarm_phases(std::time::Duration::from_secs(24 * 3600)) {
                                Ok(p) => p,
                                Err(e) => { eprintln!("Error reading phases: {e}"); process::exit(1); }
                            };
                            if phases.is_empty() {
                                eprintln!("No swarm phases found. Publish first with 'swarm publish'.");
                                process::exit(1);
                            }
                            // Derive local state and run queen sync step
                            let mut queen = kannaka_memory::QueenSync::new(
                                kannaka_memory::QueenConfig::default(),
                                &dolt_config.agent_id,
                            );
                            // Set phase from our latest published phase if available
                            if let Some(my) = phases.iter().find(|p| p.agent_id == dolt_config.agent_id) {
                                queen.phase = my.phase;
                                queen.frequency = my.frequency;
                                queen.coherence = my.coherence;
                            }
                            let state = queen.queen_sync_step(&phases);
                            // Publish updated phase
                            let updated_phase = queen.to_agent_phase(
                                phases.iter().find(|p| p.agent_id == dolt_config.agent_id)
                                    .map(|p| p.cluster_count).unwrap_or(0),
                                sys.engine.store.count(),
                            );
                            if let Err(e) = store.publish_phase(&updated_phase) {
                                eprintln!("Warning: failed to publish phase: {e}");
                            }
                            if let Err(e) = store.write_queen_state(&state) {
                                eprintln!("Warning: failed to write queen state: {e}");
                            }
                            println!("{}", serde_json::to_string_pretty(&state).unwrap());
                        }
                        Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                    }
                }
                "queen" => {
                    match DoltMemoryStore::from_config(&dolt_config) {
                        Ok(store) => {
                            match store.read_queen_state() {
                                Ok(Some(state)) => println!("{}", serde_json::to_string_pretty(&state).unwrap()),
                                Ok(None) => { eprintln!("No queen state found. Run 'swarm sync' first."); process::exit(1); }
                                Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                            }
                        }
                        Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                    }
                }
                "hives" => {
                    match DoltMemoryStore::from_config(&dolt_config) {
                        Ok(store) => {
                            match store.read_queen_state() {
                                Ok(Some(state)) => println!("{}", serde_json::to_string(&state.hives).unwrap()),
                                Ok(None) => println!("[]"),
                                Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                            }
                        }
                        Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                    }
                }
                "publish" => {
                    match DoltMemoryStore::from_config(&dolt_config) {
                        Ok(store) => {
                            // Derive phase from local state
                            let mut queen = kannaka_memory::QueenSync::new(
                                kannaka_memory::QueenConfig::default(),
                                &dolt_config.agent_id,
                            );
                            queen.derive_local_state(&sys.engine);
                            let phase = queen.to_agent_phase(
                                0, // cluster count will be derived
                                sys.engine.store.count(),
                            );
                            match store.publish_phase(&phase) {
                                Ok(()) => println!("Published phase: θ={:.3}, ω={:.3}, coherence={:.3}",
                                    phase.phase, phase.frequency, phase.coherence),
                                Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                            }
                        }
                        Err(e) => { eprintln!("Error: {e}"); process::exit(1); }
                    }
                }
                other => {
                    eprintln!("Unknown swarm command: {other}");
                    eprintln!("Usage: kannaka swarm <join|status|sync|queen|hives|publish|leave>");
                    process::exit(1);
                }
            }
        }

        "pull-merge" => {
            match DoltMemoryStore::from_config(&dolt_config) {
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

        "migrate-embeddings" => {
            // Regenerate missing vector embeddings via Ollama
            let ollama_url = env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string());
            let model = env::var("OLLAMA_EMBED_MODEL").unwrap_or_else(|_| "nomic-embed-text".to_string());

            // Query Dolt for memories with empty vectors
            let mut store = match DoltMemoryStore::from_config(&dolt_config) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Failed to connect to Dolt: {e}");
                    process::exit(1);
                }
            };

            let all_mems = store.all_memories().unwrap_or_default();
            let empty_ids: Vec<(uuid::Uuid, String)> = all_mems
                .iter()
                .filter(|m| m.vector.is_empty())
                .map(|m| (m.id, m.content.clone()))
                .collect();

            let total = empty_ids.len();
            if total == 0 {
                println!("All memories have embeddings. Nothing to do.");
                return;
            }
            eprintln!("Found {} memories with empty vectors. Generating embeddings...", total);

            let mut success = 0usize;
            let mut errors = 0usize;
            for (i, (id, content)) in empty_ids.iter().enumerate() {
                // Call Ollama embeddings API
                let body = serde_json::json!({
                    "model": model,
                    "prompt": content,
                });
                let resp = ureq::post(&format!("{}/api/embeddings", ollama_url))
                    .send_json(body);
                match resp {
                    Ok(response) => {
                        match response.into_json::<serde_json::Value>() {
                            Ok(json) => {
                                if let Some(embedding) = json["embedding"].as_array() {
                                    let vector: Vec<f32> = embedding.iter()
                                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                                        .collect();
                                    if let Ok(Some(mem)) = store.get_mut(id) {
                                        mem.vector = vector;
                                        if let Err(e) = store.update(id) {
                                            eprintln!("  [{}/{}] {} — update failed: {}", i+1, total, id, e);
                                            errors += 1;
                                            continue;
                                        }
                                        success += 1;
                                    }
                                } else {
                                    eprintln!("  [{}/{}] {} — no embedding in response", i+1, total, id);
                                    errors += 1;
                                }
                            }
                            Err(e) => {
                                eprintln!("  [{}/{}] {} — parse error: {}", i+1, total, id, e);
                                errors += 1;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("  [{}/{}] {} — request failed: {}", i+1, total, id, e);
                        errors += 1;
                    }
                }
                if (i + 1) % 10 == 0 {
                    eprintln!("  Progress: {}/{}", i+1, total);
                }
            }

            // Commit the changes
            if success > 0 {
                if let Err(e) = store.commit(&format!("migrate-embeddings: generated {} embeddings", success)) {
                    eprintln!("Warning: commit failed: {e}");
                }
            }

            let output = serde_json::json!({
                "total_missing": total,
                "success": success,
                "errors": errors,
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
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

