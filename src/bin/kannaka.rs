//! Simple CLI for testing the Kannaka memory system.

use std::env;
use std::path::PathBuf;
use std::process;

use kannaka_memory::observe::MemoryIntrospector;
use kannaka_memory::openclaw::KannakaMemorySystem;

#[cfg(feature = "dolt")]
use kannaka_memory::{DoltMemoryStore, MemoryStore};

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
fn init_with_dolt(data_dir: PathBuf) -> Result<KannakaMemorySystem, Box<dyn std::error::Error>> {
    use mysql::*;
    
    // Create MySQL connection pool to Dolt server on port 3307
    let url = "mysql://root@localhost:3307/kannaka_memory";
    let pool = Pool::new(url)?;
    
    // Create DoltMemoryStore
    let store = DoltMemoryStore::new(pool)?;
    eprintln!("DoltMemoryStore initialized with {} memories", store.count());
    
    // Create the KannakaMemorySystem with custom store
    KannakaMemorySystem::init_with_store(data_dir, Box::new(store))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}

fn usage() {
    eprintln!("Usage: kannaka [--dolt] <command> [args]");
    eprintln!();
    eprintln!("Flags:");
    #[cfg(feature = "dolt")]
    eprintln!("  --dolt                    Use Dolt database backend (port 3307)");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  remember <text>           Store a memory");
    eprintln!("  recall <query> [--top-k N]  Search memories (default top-k=5)");
    eprintln!("  dream                     Run consolidation cycle");
    eprintln!("  assess                    Check consciousness level");
    eprintln!("  stats                     Show system statistics");
    eprintln!("  observe [--json]           Introspection report");
    eprintln!("  migrate <path-to-db>      Import from kannaka.db");
    eprintln!("  export-json               Export all memories as JSON (vectors included)");
    eprintln!("  announce-status           Publish agent status event to Flux (FLUX_URL must be set)");
    #[cfg(feature = "audio")]
    eprintln!("  hear <file>               Store an audio file as a sensory memory");
    #[cfg(feature = "glyph")]
    eprintln!("  see <file>                Store a file as a glyph (visual) memory");
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
    }

    // Parse global flags
    #[cfg(feature = "dolt")]
    let use_dolt;
    let command_start;
    
    // Check for --dolt flag
    #[cfg(feature = "dolt")]
    {
        if args.len() > 1 && args[1] == "--dolt" {
            use_dolt = true;
            command_start = 2;
            if args.len() < 3 {
                usage();
            }
        } else {
            use_dolt = false;
            command_start = 1;
        }
    }
    
    #[cfg(not(feature = "dolt"))]
    {
        command_start = 1;
    }

    let dir = data_dir();
    
    #[cfg(feature = "dolt")]
    let mut sys = if use_dolt {
        // Initialize with Dolt backend
        match init_with_dolt(dir) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to initialize with Dolt: {e}");
                process::exit(1);
            }
        }
    } else {
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
                eprintln!("Usage: kannaka remember <text>");
                process::exit(1);
            }
            let text = args[command_start + 1..].join(" ");
            match sys.remember(&text) {
                Ok(id) => println!("Remembered: {id}"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        "recall" => {
            if args.len() < command_start + 2 {
                eprintln!("Usage: kannaka recall <query> [--top-k N]");
                process::exit(1);
            }
            let mut top_k = 5usize;
            let mut query_parts = Vec::new();
            let mut i = command_start + 1;
            while i < args.len() {
                if args[i] == "--top-k" && i + 1 < args.len() {
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
                    if results.is_empty() {
                        println!("No memories found.");
                    } else {
                        for (i, r) in results.iter().enumerate() {
                            println!(
                                "{}. [sim={:.3} str={:.3} age={:.1}h L{}] {}",
                                i + 1, r.similarity, r.strength, r.age_hours, r.layer, r.content
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        "dream" => {
            match sys.dream() {
                Ok(report) => {
                    println!("Dream complete ({} cycles)", report.cycles);
                    println!("  Strengthened: {}", report.memories_strengthened);
                    println!("  Pruned: {}", report.memories_pruned);
                    println!("  New connections: {}", report.new_connections);
                    println!("  Consciousness: {} → {}", report.consciousness_before, report.consciousness_after);
                    if report.emerged {
                        println!("  ✨ Emergence detected!");
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
        _ => usage(),
    }
}
