//! Simple CLI for testing the Kannaka memory system.

use std::env;
use std::path::PathBuf;
use std::process;

use kannaka_memory::observe::MemoryIntrospector;
use kannaka_memory::openclaw::KannakaMemorySystem;

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

fn usage() {
    eprintln!("Usage: kannaka <command> [args]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  remember <text>           Store a memory");
    eprintln!("  recall <query> [--top-k N]  Search memories (default top-k=5)");
    eprintln!("  dream                     Run consolidation cycle");
    eprintln!("  assess                    Check consciousness level");
    eprintln!("  stats                     Show system statistics");
    eprintln!("  observe [--json]           Introspection report");
    eprintln!("  migrate <path-to-db>      Import from kannaka.db");
    process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        usage();
    }

    let dir = data_dir();
    let mut sys = match KannakaMemorySystem::init(dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to initialize: {e}");
            process::exit(1);
        }
    };

    match args[1].as_str() {
        "remember" => {
            if args.len() < 3 {
                eprintln!("Usage: kannaka remember <text>");
                process::exit(1);
            }
            let text = args[2..].join(" ");
            match sys.remember(&text) {
                Ok(id) => println!("Remembered: {id}"),
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            }
        }
        "recall" => {
            if args.len() < 3 {
                eprintln!("Usage: kannaka recall <query> [--top-k N]");
                process::exit(1);
            }
            let mut top_k = 5usize;
            let mut query_parts = Vec::new();
            let mut i = 2;
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
            if args.len() < 3 {
                eprintln!("Usage: kannaka migrate <path-to-kannaka.db>");
                process::exit(1);
            }
            let db_path = PathBuf::from(&args[2]);
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
        _ => usage(),
    }
}
