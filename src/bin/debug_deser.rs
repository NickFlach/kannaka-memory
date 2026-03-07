use std::env;
use std::fs;
use chrono::{DateTime, Utc};
use kannaka_memory::memory::HyperMemory;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct SnapshotMetadata {
    pub created_at: DateTime<Utc>,
    pub last_saved_at: DateTime<Utc>,
    pub total_consolidations: u64,
    pub consciousness_level: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MemorySnapshot {
    pub version: u32,
    pub memories: Vec<HyperMemory>,
    pub codebook_seed: u64,
    pub codebook_input_dim: usize,
    pub codebook_output_dim: usize,
    pub metadata: SnapshotMetadata,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = args.iter().find(|a| !a.starts_with('-') && *a != &args[0])
        .cloned()
        .unwrap_or_else(|| {
            let home = env::var("USERPROFILE").unwrap_or_else(|_| env::var("HOME").unwrap());
            format!("{}/.openclaw/kannaka-data/kannaka.bin", home)
        });
    
    eprintln!("Reading: {}", path);
    let data = fs::read(&path).expect("failed to read file");
    eprintln!("Size: {} bytes", data.len());
    eprintln!("Version: {}", u32::from_le_bytes([data[0], data[1], data[2], data[3]]));
    
    match bincode::deserialize::<MemorySnapshot>(&data) {
        Ok(snap) => {
            eprintln!("✅ V2 deserialization succeeded!");
            eprintln!("Memories: {}", snap.memories.len());
            eprintln!("Codebook: seed={} in={} out={}", snap.codebook_seed, snap.codebook_input_dim, snap.codebook_output_dim);
            eprintln!("Consciousness: {}", snap.metadata.consciousness_level);
            
            let dump_json = env::args().any(|a| a == "--json");
            if dump_json {
                // Output full JSON for migration
                println!("{}", serde_json::to_string(&snap.memories).expect("json serialize failed"));
            } else {
                for mem in &snap.memories {
                    println!("{}|{}|{}|{}|{}|{}|{}|{}|{}",
                        mem.id, 
                        mem.content.replace('|', "\\|").chars().take(200).collect::<String>(),
                        mem.amplitude, mem.frequency, mem.phase, mem.decay_rate,
                        mem.layer_depth, mem.hallucinated, mem.connections.len()
                    );
                }
            }
        }
        Err(e) => {
            eprintln!("❌ V2 deser failed: {}", e);
        }
    }
}
