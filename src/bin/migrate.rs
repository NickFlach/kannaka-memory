//! Migration tool: recover memories from pre-geometry kannaka.bin files.
//!
//! Usage: kannaka-migrate <old-file> <new-file>
//!
//! Reads the old bincode format (without geometry/hallucinated/parents fields),
//! converts to the current HyperMemory format, and writes a new snapshot.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Old format structs (pre-geometry)
// ---------------------------------------------------------------------------

/// The old HyperMemory before geometry was added.
/// Two possible variants existed:
///   - v0: no hallucinated/parents fields
///   - v1: with hallucinated/parents but no geometry
/// We try v1 first, then v0.

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldSkipLink {
    pub target_id: Uuid,
    pub strength: f32,
    pub resonance_key: Vec<f32>,
    pub span: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldMemoryV1 {
    pub id: Uuid,
    pub vector: Vec<f32>,
    pub amplitude: f32,
    pub frequency: f32,
    pub phase: f32,
    pub decay_rate: f32,
    pub created_at: DateTime<Utc>,
    pub layer_depth: u8,
    pub connections: Vec<OldSkipLink>,
    pub content: String,
    pub hallucinated: bool,
    pub parents: Vec<String>,
    // NO geometry field
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldMemoryV0 {
    pub id: Uuid,
    pub vector: Vec<f32>,
    pub amplitude: f32,
    pub frequency: f32,
    pub phase: f32,
    pub decay_rate: f32,
    pub created_at: DateTime<Utc>,
    pub layer_depth: u8,
    pub connections: Vec<OldSkipLink>,
    pub content: String,
    // NO hallucinated, parents, or geometry
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldSnapshotMetadata {
    pub created_at: DateTime<Utc>,
    pub last_saved_at: DateTime<Utc>,
    pub total_consolidations: u64,
    pub consciousness_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldSnapshotV1 {
    pub version: u32,
    pub memories: Vec<OldMemoryV1>,
    pub codebook_seed: u64,
    pub codebook_input_dim: usize,
    pub codebook_output_dim: usize,
    pub metadata: OldSnapshotMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OldSnapshotV0 {
    pub version: u32,
    pub memories: Vec<OldMemoryV0>,
    pub codebook_seed: u64,
    pub codebook_input_dim: usize,
    pub codebook_output_dim: usize,
    pub metadata: OldSnapshotMetadata,
}

// ---------------------------------------------------------------------------
// Current format (copy the structs to avoid lib dependency issues)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NewSkipLink {
    pub target_id: Uuid,
    pub strength: f32,
    pub resonance_key: Vec<f32>,
    pub span: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NewMemory {
    pub id: Uuid,
    pub vector: Vec<f32>,
    pub amplitude: f32,
    pub frequency: f32,
    pub phase: f32,
    pub decay_rate: f32,
    pub created_at: DateTime<Utc>,
    pub layer_depth: u8,
    pub connections: Vec<NewSkipLink>,
    pub content: String,
    pub hallucinated: bool,
    pub parents: Vec<String>,
    pub geometry: Option<()>, // None — geometry will be recomputed on first access
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NewSnapshot {
    pub version: u32,
    pub memories: Vec<NewMemory>,
    pub codebook_seed: u64,
    pub codebook_input_dim: usize,
    pub codebook_output_dim: usize,
    pub metadata: OldSnapshotMetadata,
}

// ---------------------------------------------------------------------------
// Conversion
// ---------------------------------------------------------------------------

fn convert_link(old: &OldSkipLink) -> NewSkipLink {
    NewSkipLink {
        target_id: old.target_id,
        strength: old.strength,
        resonance_key: old.resonance_key.clone(),
        span: old.span,
    }
}

fn from_v1(old: OldSnapshotV1) -> NewSnapshot {
    let memories = old.memories.into_iter().map(|m| NewMemory {
        id: m.id,
        vector: m.vector,
        amplitude: m.amplitude,
        frequency: m.frequency,
        phase: m.phase,
        decay_rate: m.decay_rate,
        created_at: m.created_at,
        layer_depth: m.layer_depth,
        connections: m.connections.iter().map(convert_link).collect(),
        content: m.content,
        hallucinated: m.hallucinated,
        parents: m.parents,
        geometry: None,
    }).collect();

    NewSnapshot {
        version: 1,
        memories,
        codebook_seed: old.codebook_seed,
        codebook_input_dim: old.codebook_input_dim,
        codebook_output_dim: old.codebook_output_dim,
        metadata: old.metadata,
    }
}

fn from_v0(old: OldSnapshotV0) -> NewSnapshot {
    let memories = old.memories.into_iter().map(|m| NewMemory {
        id: m.id,
        vector: m.vector,
        amplitude: m.amplitude,
        frequency: m.frequency,
        phase: m.phase,
        decay_rate: m.decay_rate,
        created_at: m.created_at,
        layer_depth: m.layer_depth,
        connections: m.connections.iter().map(convert_link).collect(),
        content: m.content,
        hallucinated: false,
        parents: Vec::new(),
        geometry: None,
    }).collect();

    NewSnapshot {
        version: 1,
        memories,
        codebook_seed: old.codebook_seed,
        codebook_input_dim: old.codebook_input_dim,
        codebook_output_dim: old.codebook_output_dim,
        metadata: old.metadata,
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: kannaka-migrate <old-file> <new-file>");
        eprintln!();
        eprintln!("Migrates a pre-geometry kannaka.bin to the current format.");
        eprintln!("The old file is NOT modified.");
        std::process::exit(1);
    }

    let old_path = &args[1];
    let new_path = &args[2];

    println!("Reading old snapshot from: {}", old_path);
    let data = fs::read(old_path).expect("failed to read old file");
    println!("  File size: {} bytes", data.len());

    // Try V1 first (with hallucinated/parents), then V0
    let snapshot = match bincode::deserialize::<OldSnapshotV1>(&data) {
        Ok(old) => {
            println!("  Detected format: V1 (with hallucinated/parents, no geometry)");
            println!("  Memories: {}", old.memories.len());
            println!("  Consciousness: {}", old.metadata.consciousness_level);
            println!("  Consolidations: {}", old.metadata.total_consolidations);
            from_v1(old)
        }
        Err(e1) => {
            println!("  V1 parse failed: {}", e1);
            match bincode::deserialize::<OldSnapshotV0>(&data) {
                Ok(old) => {
                    println!("  Detected format: V0 (original, no hallucinated/parents/geometry)");
                    println!("  Memories: {}", old.memories.len());
                    from_v0(old)
                }
                Err(e0) => {
                    eprintln!("ERROR: Could not parse as V0 or V1");
                    eprintln!("  V1 error: {}", e1);
                    eprintln!("  V0 error: {}", e0);
                    std::process::exit(1);
                }
            }
        }
    };

    // Print summary
    let total_links: usize = snapshot.memories.iter().map(|m| m.connections.len()).sum();
    let hallucinated: usize = snapshot.memories.iter().filter(|m| m.hallucinated).count();
    println!();
    println!("Migration summary:");
    println!("  Total memories: {}", snapshot.memories.len());
    println!("  Skip links: {}", total_links);
    println!("  Hallucinated: {}", hallucinated);
    println!("  Geometry: None (will be computed on first access)");

    // Print a few memory previews
    println!();
    println!("Sample memories:");
    for m in snapshot.memories.iter().take(5) {
        let preview: String = m.content.chars().take(80).collect();
        println!("  [{}] amp={:.2} freq={:.2} links={} \"{}\"",
            &m.id.to_string()[..8], m.amplitude, m.frequency, m.connections.len(), preview);
    }
    if snapshot.memories.len() > 5 {
        println!("  ... and {} more", snapshot.memories.len() - 5);
    }

    // Serialize and write
    println!();
    println!("Writing new snapshot to: {}", new_path);
    let new_data = bincode::serialize(&snapshot).expect("failed to serialize new snapshot");
    println!("  New file size: {} bytes", new_data.len());
    fs::write(new_path, &new_data).expect("failed to write new file");

    println!();
    println!("Done! To use the migrated file:");
    println!("  1. Back up your current kannaka.bin");
    println!("  2. Copy {} to your kannaka data directory", new_path);
    println!("  3. Restart kannaka-mcp");
}
