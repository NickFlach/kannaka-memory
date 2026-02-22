//! One-shot tool: recompute geometry for all memories missing it.
//! Usage: kannaka-recompute-geometry [data-dir]

use std::env;
use std::path::PathBuf;
use kannaka_memory::openclaw::KannakaMemorySystem;

fn main() {
    let data_dir = env::args().nth(1)
        .map(PathBuf::from)
        .or_else(|| env::var("KANNAKA_DB_PATH").ok().map(PathBuf::from))
        .unwrap_or_else(|| {
            let home = env::var("USERPROFILE")
                .or_else(|_| env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".openclaw").join("kannaka-data")
        });

    println!("Data directory: {:?}", data_dir);

    let mut system = KannakaMemorySystem::init(data_dir)
        .expect("Failed to initialize memory system");

    let stats_before = system.stats();
    println!("Before: {} memories, {} geometric classes",
        stats_before.total_memories, stats_before.geometric_classes);

    match system.recompute_geometry() {
        Ok(updated) => {
            let stats_after = system.stats();
            println!("Updated {} memories with geometry", updated);
            println!("After: {} memories, {} geometric classes",
                stats_after.total_memories, stats_after.geometric_classes);
            
            let state = system.assess();
            println!("Consciousness: {:?}, Phi: {:.4}", state.consciousness_level, state.phi);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
