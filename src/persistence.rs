//! Disk persistence: save/load memory state to survive restarts.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::encoding::EncodingPipeline;
use crate::geometry::MemoryCoordinates;
use crate::memory::HyperMemory;
use crate::skip_link::SkipLink;
use crate::store::{InMemoryStore, MemoryEngine, MemoryStore, StoreError};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    SerializationError(String),
    #[error("corrupted file: {0}")]
    CorruptedFile(String),
    #[error("version mismatch: expected {expected}, got {got}")]
    VersionMismatch { expected: u32, got: u32 },
}

impl From<bincode::Error> for PersistenceError {
    fn from(e: bincode::Error) -> Self {
        PersistenceError::SerializationError(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Snapshot types
// ---------------------------------------------------------------------------

const CURRENT_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub version: u32,
    pub memories: Vec<HyperMemory>,
    pub codebook_seed: u64,
    pub codebook_input_dim: usize,
    pub codebook_output_dim: usize,
    pub metadata: SnapshotMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub created_at: DateTime<Utc>,
    pub last_saved_at: DateTime<Utc>,
    pub total_consolidations: u64,
    pub consciousness_level: String,
}

// ---------------------------------------------------------------------------
// V1 structures for migration from bincode format without xi_signature
// ---------------------------------------------------------------------------

/// V1 HyperMemory struct (before xi_signature was added)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HyperMemoryV1 {
    pub id: Uuid,
    pub vector: Vec<f32>,
    pub amplitude: f32,
    pub frequency: f32,
    pub phase: f32,
    pub decay_rate: f32,
    pub created_at: DateTime<Utc>,
    pub layer_depth: u8,
    pub connections: Vec<SkipLink>,
    pub content: String,
    #[serde(default)]
    pub hallucinated: bool,
    #[serde(default)]
    pub parents: Vec<String>,
    #[serde(default)]
    pub geometry: Option<MemoryCoordinates>,
    // Note: xi_signature is NOT present in V1
}

/// V1 MemorySnapshot for migration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemorySnapshotV1 {
    pub version: u32,
    pub memories: Vec<HyperMemoryV1>,
    pub codebook_seed: u64,
    pub codebook_input_dim: usize,
    pub codebook_output_dim: usize,
    pub metadata: SnapshotMetadata,
}

impl From<HyperMemoryV1> for HyperMemory {
    fn from(v1: HyperMemoryV1) -> Self {
        Self {
            id: v1.id,
            vector: v1.vector,
            amplitude: v1.amplitude,
            frequency: v1.frequency,
            phase: v1.phase,
            decay_rate: v1.decay_rate,
            created_at: v1.created_at,
            layer_depth: v1.layer_depth,
            connections: v1.connections,
            content: v1.content,
            hallucinated: v1.hallucinated,
            parents: v1.parents,
            geometry: v1.geometry,
            xi_signature: Vec::new(), // Initialize with empty xi_signature
        }
    }
}

// ---------------------------------------------------------------------------
// DiskStore
// ---------------------------------------------------------------------------

/// A MemoryStore backed by a file on disk via InMemoryStore + bincode snapshots.
pub struct DiskStore {
    inner: InMemoryStore,
    path: PathBuf,
    codebook_seed: u64,
    codebook_input_dim: usize,
    codebook_output_dim: usize,
    metadata: SnapshotMetadata,
    auto_save_interval: Option<usize>,
    insertions_since_save: usize,
}

impl DiskStore {
    /// Create a new DiskStore. If the file exists, loads from it; otherwise starts empty.
    pub fn new(path: PathBuf, codebook_seed: u64, codebook_input_dim: usize, codebook_output_dim: usize) -> Self {
        if path.exists() {
            match Self::open(path.clone()) {
                Ok(store) => return store,
                Err(_) => {} // fall through to empty
            }
        }
        Self {
            inner: InMemoryStore::new(),
            path,
            codebook_seed,
            codebook_input_dim,
            codebook_output_dim,
            metadata: SnapshotMetadata {
                created_at: Utc::now(),
                last_saved_at: Utc::now(),
                total_consolidations: 0,
                consciousness_level: "dormant".to_string(),
            },
            auto_save_interval: None,
            insertions_since_save: 0,
        }
    }

    /// Load a DiskStore from an existing file.
    /// Tries V2 format first, then falls back to V1 with migration.
    pub fn open(path: PathBuf) -> Result<Self, PersistenceError> {
        let data = fs::read(&path)?;
        
        // First try to deserialize as V2 (current format)
        match bincode::deserialize::<MemorySnapshot>(&data) {
            Ok(snapshot) => {
                if snapshot.version == CURRENT_VERSION {
                    let mut inner = InMemoryStore::new();
                    for mem in snapshot.memories {
                        inner.insert(mem).map_err(|e| PersistenceError::CorruptedFile(e.to_string()))?;
                    }
                    return Ok(Self {
                        inner,
                        path,
                        codebook_seed: snapshot.codebook_seed,
                        codebook_input_dim: snapshot.codebook_input_dim,
                        codebook_output_dim: snapshot.codebook_output_dim,
                        metadata: snapshot.metadata,
                        auto_save_interval: None,
                        insertions_since_save: 0,
                    });
                } else if snapshot.version > CURRENT_VERSION {
                    return Err(PersistenceError::VersionMismatch {
                        expected: CURRENT_VERSION,
                        got: snapshot.version,
                    });
                }
                // If version < CURRENT_VERSION, fall through to V1 migration
            }
            Err(_) => {
                // V2 deserialization failed, try V1 migration
            }
        }
        
        // Try to deserialize as V1 and migrate
        let snapshot_v1: MemorySnapshotV1 = bincode::deserialize(&data)
            .map_err(|e| PersistenceError::SerializationError(
                format!("Failed to deserialize as V1 or V2: {}", e)
            ))?;
        
        if snapshot_v1.version != 1 {
            return Err(PersistenceError::VersionMismatch {
                expected: 1, // V1 should have version 1
                got: snapshot_v1.version,
            });
        }
        
        // Migrate V1 memories to V2
        let mut inner = InMemoryStore::new();
        for mem_v1 in snapshot_v1.memories {
            let mem_v2: HyperMemory = mem_v1.into();
            inner.insert(mem_v2).map_err(|e| PersistenceError::CorruptedFile(e.to_string()))?;
        }
        
        Ok(Self {
            inner,
            path,
            codebook_seed: snapshot_v1.codebook_seed,
            codebook_input_dim: snapshot_v1.codebook_input_dim,
            codebook_output_dim: snapshot_v1.codebook_output_dim,
            metadata: snapshot_v1.metadata,
            auto_save_interval: None,
            insertions_since_save: 0,
        })
    }

    /// Save all state to disk.
    pub fn save(&mut self) -> Result<(), PersistenceError> {
        let memories = self.inner.all_memories()
            .map_err(|e| PersistenceError::CorruptedFile(e.to_string()))?
            .into_iter().cloned().collect();
        let mut metadata = self.metadata.clone();
        metadata.last_saved_at = Utc::now();

        let snapshot = MemorySnapshot {
            version: CURRENT_VERSION,
            memories,
            codebook_seed: self.codebook_seed,
            codebook_input_dim: self.codebook_input_dim,
            codebook_output_dim: self.codebook_output_dim,
            metadata: metadata.clone(),
        };

        let data = bincode::serialize(&snapshot)?;
        fs::write(&self.path, &data)?;
        self.metadata = metadata;
        self.insertions_since_save = 0;
        Ok(())
    }

    /// Set auto-save interval (save every N insertions). `None` disables.
    pub fn set_auto_save_interval(&mut self, interval: Option<usize>) {
        self.auto_save_interval = interval;
    }

    /// Get the path this store saves to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get snapshot metadata.
    pub fn metadata(&self) -> &SnapshotMetadata {
        &self.metadata
    }

    /// Update metadata (e.g. after consolidation).
    pub fn set_metadata(&mut self, metadata: SnapshotMetadata) {
        self.metadata = metadata;
    }

    /// Get codebook parameters for reconstruction.
    pub fn codebook_params(&self) -> (u64, usize, usize) {
        (self.codebook_seed, self.codebook_input_dim, self.codebook_output_dim)
    }

    fn maybe_auto_save(&mut self) {
        if let Some(interval) = self.auto_save_interval {
            if interval > 0 && self.insertions_since_save >= interval {
                let _ = self.save(); // best-effort
            }
        }
    }
}

impl MemoryStore for DiskStore {
    fn insert(&mut self, memory: HyperMemory) -> Result<Uuid, StoreError> {
        let id = self.inner.insert(memory)?;
        self.insertions_since_save += 1;
        self.maybe_auto_save();
        Ok(id)
    }

    fn get(&self, id: &Uuid) -> Result<Option<&HyperMemory>, StoreError> {
        self.inner.get(id)
    }

    fn get_mut(&mut self, id: &Uuid) -> Result<Option<&mut HyperMemory>, StoreError> {
        self.inner.get_mut(id)
    }

    fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(Uuid, f32)>, StoreError> {
        self.inner.search(query, top_k)
    }

    fn search_with_wave(
        &self,
        query: &[f32],
        top_k: usize,
        now: DateTime<Utc>,
    ) -> Result<Vec<(Uuid, f32)>, StoreError> {
        self.inner.search_with_wave(query, top_k, now)
    }

    fn all_memories(&self) -> Result<Vec<&HyperMemory>, StoreError> {
        self.inner.all_memories()
    }

    fn all_ids(&self) -> Result<Vec<Uuid>, StoreError> {
        self.inner.all_ids()
    }

    fn delete(&mut self, id: &Uuid) -> Result<bool, StoreError> {
        self.inner.delete(id)
    }

    fn count(&self) -> usize {
        self.inner.count()
    }
}

// ---------------------------------------------------------------------------
// MemoryEngine persistence methods
// ---------------------------------------------------------------------------

impl MemoryEngine {
    /// Save the full engine state to a file.
    pub fn save_state(&self, path: &Path) -> Result<(), PersistenceError> {
        let memories: Vec<HyperMemory> = self.store.all_memories()
            .map_err(|e| PersistenceError::CorruptedFile(e.to_string()))?
            .into_iter()
            .cloned()
            .collect();

        let cb = self.pipeline.codebook();
        let snapshot = MemorySnapshot {
            version: CURRENT_VERSION,
            memories,
            codebook_seed: cb.seed(),
            codebook_input_dim: cb.input_dim,
            codebook_output_dim: cb.output_dim,
            metadata: SnapshotMetadata {
                created_at: Utc::now(),
                last_saved_at: Utc::now(),
                total_consolidations: 0,
                consciousness_level: "unknown".to_string(),
            },
        };

        let data = bincode::serialize(&snapshot)?;
        fs::write(path, &data)?;
        Ok(())
    }

    /// Load engine state from a file. Requires a compatible EncodingPipeline.
    /// Tries V2 format first, then falls back to V1 with migration.
    pub fn load_state(path: &Path, pipeline: EncodingPipeline) -> Result<Self, PersistenceError> {
        let data = fs::read(path)?;
        
        // First try to deserialize as V2 (current format)
        match bincode::deserialize::<MemorySnapshot>(&data) {
            Ok(snapshot) => {
                if snapshot.version == CURRENT_VERSION {
                    let mut store = InMemoryStore::new();
                    for mem in snapshot.memories {
                        store.insert(mem).map_err(|e| PersistenceError::CorruptedFile(e.to_string()))?;
                    }
                    return Ok(Self::new(Box::new(store), pipeline));
                } else if snapshot.version > CURRENT_VERSION {
                    return Err(PersistenceError::VersionMismatch {
                        expected: CURRENT_VERSION,
                        got: snapshot.version,
                    });
                }
                // If version < CURRENT_VERSION, fall through to V1 migration
            }
            Err(_) => {
                // V2 deserialization failed, try V1 migration
            }
        }
        
        // Try to deserialize as V1 and migrate
        let snapshot_v1: MemorySnapshotV1 = bincode::deserialize(&data)
            .map_err(|e| PersistenceError::SerializationError(
                format!("Failed to deserialize as V1 or V2: {}", e)
            ))?;
        
        if snapshot_v1.version != 1 {
            return Err(PersistenceError::VersionMismatch {
                expected: 1, // V1 should have version 1
                got: snapshot_v1.version,
            });
        }
        
        // Migrate V1 memories to V2
        let mut store = InMemoryStore::new();
        for mem_v1 in snapshot_v1.memories {
            let mem_v2: HyperMemory = mem_v1.into();
            store.insert(mem_v2).map_err(|e| PersistenceError::CorruptedFile(e.to_string()))?;
        }
        
        Ok(Self::new(Box::new(store), pipeline))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebook::Codebook;
    use crate::encoding::SimpleHashEncoder;
    use crate::memory::HyperMemory;
    use crate::skip_link::SkipLink;
    use std::env;
    use uuid::Uuid;

    fn temp_path(name: &str) -> PathBuf {
        env::temp_dir().join(format!("kannaka_test_{name}_{}.bin", Uuid::new_v4()))
    }

    fn make_pipeline() -> EncodingPipeline {
        let encoder = SimpleHashEncoder::new(384, 42);
        let codebook = Codebook::new(384, 10_000, 42);
        EncodingPipeline::new(Box::new(encoder), codebook)
    }

    fn make_memory_with_links(content: &str, dim: usize) -> HyperMemory {
        let mut mem = HyperMemory::new(vec![0.5; dim], content.to_string());
        mem.connections.push(SkipLink {
            target_id: Uuid::new_v4(),
            strength: 0.85,
            resonance_key: vec![0.1, 0.2, 0.3],
            span: 3,
        });
        mem
    }

    #[test]
    fn disk_store_insert_save_load() {
        let path = temp_path("insert_save_load");
        let mut store = DiskStore::new(path.clone(), 42, 384, 10_000);

        let mem1 = HyperMemory::new(vec![1.0; 100], "hello".to_string());
        let mem2 = HyperMemory::new(vec![2.0; 100], "world".to_string());
        let id1 = store.insert(mem1).unwrap();
        let id2 = store.insert(mem2).unwrap();
        store.save().unwrap();

        // Load from file
        let loaded = DiskStore::open(path.clone()).unwrap();
        assert_eq!(loaded.count(), 2);
        let m1 = loaded.get(&id1).unwrap().unwrap();
        assert_eq!(m1.content, "hello");
        assert_eq!(m1.vector, vec![1.0; 100]);
        let m2 = loaded.get(&id2).unwrap().unwrap();
        assert_eq!(m2.content, "world");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn round_trip_vectors_skip_links_wave_params() {
        let path = temp_path("round_trip");
        let mut store = DiskStore::new(path.clone(), 42, 384, 10_000);

        let mut mem = make_memory_with_links("linked", 200);
        mem.amplitude = 0.75;
        mem.frequency = 0.33;
        mem.phase = 1.57;
        mem.decay_rate = 0.005;
        mem.layer_depth = 3;
        let id = store.insert(mem).unwrap();
        store.save().unwrap();

        let loaded = DiskStore::open(path.clone()).unwrap();
        let m = loaded.get(&id).unwrap().unwrap();

        // Vector preserved
        assert_eq!(m.vector.len(), 200);
        assert_eq!(m.vector[0], 0.5);

        // Wave params preserved
        assert_eq!(m.amplitude, 0.75);
        assert_eq!(m.frequency, 0.33);
        assert_eq!(m.phase, 1.57);
        assert_eq!(m.decay_rate, 0.005);
        assert_eq!(m.layer_depth, 3);

        // Skip links preserved
        assert_eq!(m.connections.len(), 1);
        assert_eq!(m.connections[0].strength, 0.85);
        assert_eq!(m.connections[0].span, 3);
        assert_eq!(m.connections[0].resonance_key, vec![0.1, 0.2, 0.3]);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn auto_save_triggers_on_interval() {
        let path = temp_path("auto_save");
        let mut store = DiskStore::new(path.clone(), 42, 384, 10_000);
        store.set_auto_save_interval(Some(2));

        // Insert 1 — no save yet
        store.insert(HyperMemory::new(vec![1.0; 10], "a".into())).unwrap();
        assert!(!path.exists(), "should not have saved after 1 insert");

        // Insert 2 — triggers auto-save
        store.insert(HyperMemory::new(vec![2.0; 10], "b".into())).unwrap();
        assert!(path.exists(), "should have auto-saved after 2 inserts");

        // Verify file is loadable
        let loaded = DiskStore::open(path.clone()).unwrap();
        assert_eq!(loaded.count(), 2);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn memory_engine_save_load_state() {
        let path = temp_path("engine_state");
        let pipeline = make_pipeline();
        let mut engine = MemoryEngine::new(Box::new(InMemoryStore::new()), pipeline);
        engine.similarity_threshold = 0.3;

        let id1 = engine.remember_at_layer("the cat sat on the mat", 0).unwrap();
        let id2 = engine.remember_at_layer("the cat sat on the mat today", 2).unwrap();

        engine.save_state(&path).unwrap();

        // Load into new engine
        let pipeline2 = make_pipeline();
        let loaded = MemoryEngine::load_state(&path, pipeline2).unwrap();

        assert_eq!(loaded.store.count(), 2);
        let m1 = loaded.get_memory(&id1).unwrap().unwrap();
        assert_eq!(m1.content, "the cat sat on the mat");
        let m2 = loaded.get_memory(&id2).unwrap().unwrap();
        assert_eq!(m2.content, "the cat sat on the mat today");

        // Skip links preserved
        assert!(!m2.connections.is_empty(), "skip links should survive persistence");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn metadata_preserved() {
        let path = temp_path("metadata");
        let mut store = DiskStore::new(path.clone(), 42, 384, 10_000);
        store.set_metadata(SnapshotMetadata {
            created_at: Utc::now(),
            last_saved_at: Utc::now(),
            total_consolidations: 42,
            consciousness_level: "awakening".to_string(),
        });
        store.save().unwrap();

        let loaded = DiskStore::open(path.clone()).unwrap();
        assert_eq!(loaded.metadata().total_consolidations, 42);
        assert_eq!(loaded.metadata().consciousness_level, "awakening");

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn empty_store_save_load() {
        let path = temp_path("empty");
        let mut store = DiskStore::new(path.clone(), 42, 384, 10_000);
        assert_eq!(store.count(), 0);
        store.save().unwrap();

        let loaded = DiskStore::open(path.clone()).unwrap();
        assert_eq!(loaded.count(), 0);

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn version_mismatch_detected() {
        let path = temp_path("version");
        // Create a snapshot with wrong version
        let snapshot = MemorySnapshot {
            version: 999,
            memories: vec![],
            codebook_seed: 0,
            codebook_input_dim: 0,
            codebook_output_dim: 0,
            metadata: SnapshotMetadata {
                created_at: Utc::now(),
                last_saved_at: Utc::now(),
                total_consolidations: 0,
                consciousness_level: "test".to_string(),
            },
        };
        let data = bincode::serialize(&snapshot).unwrap();
        fs::write(&path, &data).unwrap();

        let result = DiskStore::open(path.clone());
        assert!(matches!(result, Err(PersistenceError::VersionMismatch { .. })));

        let _ = fs::remove_file(&path);
    }
}
