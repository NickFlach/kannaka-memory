//! Storage layer: MemoryStore trait, InMemoryStore, and MemoryEngine.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::encoding::{EncodingError, EncodingPipeline};
use crate::memory::HyperMemory;
use crate::wave::cosine_similarity;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("memory not found: {0}")]
    NotFound(Uuid),
    #[error("duplicate id: {0}")]
    DuplicateId(Uuid),
    #[error("store error: {0}")]
    Other(String),
}

#[derive(Debug, Error)]
pub enum EngineError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Encoding(#[from] EncodingError),
}

// ---------------------------------------------------------------------------
// QueryResult
// ---------------------------------------------------------------------------

/// Rich search result with wave-modulated scoring.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub id: Uuid,
    pub similarity: f32,
    pub effective_strength: f32,
    pub combined_score: f32,
}

// ---------------------------------------------------------------------------
// MemoryStore trait
// ---------------------------------------------------------------------------

/// Pluggable storage backend for hypervector memories.
pub trait MemoryStore: Send + Sync {
    fn insert(&mut self, memory: HyperMemory) -> Result<Uuid, StoreError>;
    fn get(&self, id: &Uuid) -> Result<Option<&HyperMemory>, StoreError>;
    fn get_mut(&mut self, id: &Uuid) -> Result<Option<&mut HyperMemory>, StoreError>;
    fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(Uuid, f32)>, StoreError>;
    fn search_with_wave(
        &self,
        query: &[f32],
        top_k: usize,
        now: DateTime<Utc>,
    ) -> Result<Vec<(Uuid, f32)>, StoreError>;
    fn all_memories(&self) -> Result<Vec<&HyperMemory>, StoreError>;
    fn delete(&mut self, id: &Uuid) -> Result<bool, StoreError>;
    fn count(&self) -> usize;
}

// ---------------------------------------------------------------------------
// InMemoryStore
// ---------------------------------------------------------------------------

/// HashMap-backed reference implementation with brute-force cosine similarity.
pub struct InMemoryStore {
    memories: HashMap<Uuid, HyperMemory>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            memories: HashMap::new(),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore for InMemoryStore {
    fn insert(&mut self, memory: HyperMemory) -> Result<Uuid, StoreError> {
        let id = memory.id;
        if self.memories.contains_key(&id) {
            return Err(StoreError::DuplicateId(id));
        }
        self.memories.insert(id, memory);
        Ok(id)
    }

    fn get(&self, id: &Uuid) -> Result<Option<&HyperMemory>, StoreError> {
        Ok(self.memories.get(id))
    }

    fn get_mut(&mut self, id: &Uuid) -> Result<Option<&mut HyperMemory>, StoreError> {
        Ok(self.memories.get_mut(id))
    }

    fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(Uuid, f32)>, StoreError> {
        let mut scored: Vec<(Uuid, f32)> = self
            .memories
            .values()
            .map(|m| (m.id, cosine_similarity(query, &m.vector)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        Ok(scored)
    }

    fn search_with_wave(
        &self,
        query: &[f32],
        top_k: usize,
        now: DateTime<Utc>,
    ) -> Result<Vec<(Uuid, f32)>, StoreError> {
        let mut scored: Vec<(Uuid, f32)> = self
            .memories
            .values()
            .map(|m| {
                let sim = cosine_similarity(query, &m.vector);
                let strength = m.effective_strength(now);
                (m.id, sim * strength)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        Ok(scored)
    }

    fn all_memories(&self) -> Result<Vec<&HyperMemory>, StoreError> {
        Ok(self.memories.values().collect())
    }

    fn delete(&mut self, id: &Uuid) -> Result<bool, StoreError> {
        Ok(self.memories.remove(id).is_some())
    }

    fn count(&self) -> usize {
        self.memories.len()
    }
}

// ---------------------------------------------------------------------------
// MemoryEngine
// ---------------------------------------------------------------------------

/// High-level API: remember() and recall() over a pluggable store.
pub struct MemoryEngine {
    store: Box<dyn MemoryStore>,
    pipeline: EncodingPipeline,
}

impl MemoryEngine {
    pub fn new(store: Box<dyn MemoryStore>, pipeline: EncodingPipeline) -> Self {
        Self { store, pipeline }
    }

    /// Encode text and store as a new memory. Returns the memory id.
    pub fn remember(&mut self, text: &str) -> Result<Uuid, EngineError> {
        let memory = self.pipeline.encode_memory(text, Utc::now())?;
        let id = self.store.insert(memory)?;
        Ok(id)
    }

    /// Encode a query and search with wave-modulated ranking.
    pub fn recall(&self, query: &str, top_k: usize) -> Result<Vec<QueryResult>, EngineError> {
        let qvec = self.pipeline.encode_text(query)?;
        let now = Utc::now();
        // Get raw similarity and wave-modulated scores
        let raw = self.store.search(&qvec, self.store.count())?;
        let raw_map: HashMap<Uuid, f32> = raw.into_iter().collect();
        let wave_results = self.store.search_with_wave(&qvec, top_k, now)?;

        let results = wave_results
            .into_iter()
            .map(|(id, combined)| {
                let similarity = raw_map.get(&id).copied().unwrap_or(0.0);
                let effective_strength = if similarity.abs() > 1e-9 {
                    combined / similarity
                } else {
                    0.0
                };
                QueryResult {
                    id,
                    similarity,
                    effective_strength,
                    combined_score: combined,
                }
            })
            .collect();
        Ok(results)
    }

    /// Get a memory by id.
    pub fn get_memory(&self, id: &Uuid) -> Result<Option<&HyperMemory>, EngineError> {
        Ok(self.store.get(id)?)
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
    use crate::wave::normalize;
    use chrono::Duration;

    fn make_pipeline() -> EncodingPipeline {
        let encoder = SimpleHashEncoder::new(384, 42);
        let codebook = Codebook::new(384, 10_000, 42);
        EncodingPipeline::new(Box::new(encoder), codebook)
    }

    fn make_memory(vector: Vec<f32>, content: &str) -> HyperMemory {
        HyperMemory::new(vector, content.to_string())
    }

    fn unit_vec(dim: usize, index: usize) -> Vec<f32> {
        let mut v = vec![0.0; dim];
        v[index] = 1.0;
        v
    }

    // -- InMemoryStore tests --

    #[test]
    fn store_insert_get_count() {
        let mut store = InMemoryStore::new();
        assert_eq!(store.count(), 0);
        let mem = make_memory(vec![1.0; 10], "hello");
        let id = store.insert(mem).unwrap();
        assert_eq!(store.count(), 1);
        let got = store.get(&id).unwrap().unwrap();
        assert_eq!(got.content, "hello");
    }

    #[test]
    fn store_delete() {
        let mut store = InMemoryStore::new();
        let mem = make_memory(vec![1.0; 10], "bye");
        let id = store.insert(mem).unwrap();
        assert!(store.delete(&id).unwrap());
        assert_eq!(store.count(), 0);
        assert!(!store.delete(&id).unwrap());
    }

    #[test]
    fn store_duplicate_id_rejected() {
        let mut store = InMemoryStore::new();
        let mem = make_memory(vec![1.0; 10], "a");
        let id = mem.id;
        store.insert(mem).unwrap();
        let mut mem2 = make_memory(vec![2.0; 10], "b");
        mem2.id = id;
        assert!(matches!(store.insert(mem2), Err(StoreError::DuplicateId(_))));
    }

    #[test]
    fn search_returns_closest_first() {
        let mut store = InMemoryStore::new();
        // Insert 3 memories at orthogonal-ish directions
        let mut v1 = unit_vec(100, 0);
        let mut v2 = unit_vec(100, 1);
        let mut v3 = unit_vec(100, 2);
        normalize(&mut v1);
        normalize(&mut v2);
        normalize(&mut v3);
        let m1 = make_memory(v1.clone(), "v1");
        let m2 = make_memory(v2, "v2");
        let m3 = make_memory(v3, "v3");
        let id1 = store.insert(m1).unwrap();
        store.insert(m2).unwrap();
        store.insert(m3).unwrap();

        let results = store.search(&v1, 3).unwrap();
        assert_eq!(results[0].0, id1);
        assert!((results[0].1 - 1.0).abs() < 1e-5);
    }

    #[test]
    fn search_with_wave_older_ranks_lower() {
        let mut store = InMemoryStore::new();
        let v = vec![1.0; 50];

        // Recent memory
        let m_recent = make_memory(v.clone(), "recent");
        let id_recent = m_recent.id;
        store.insert(m_recent).unwrap();

        // Old memory — backdate creation
        let mut m_old = make_memory(v.clone(), "old");
        m_old.created_at = Utc::now() - Duration::days(30);
        m_old.frequency = 0.0; // pure decay
        m_old.decay_rate = 0.001;
        let id_old = m_old.id;
        store.insert(m_old).unwrap();

        let now = Utc::now();
        let results = store.search_with_wave(&v, 2, now).unwrap();
        // Recent memory should rank first
        assert_eq!(results[0].0, id_recent);
        assert_eq!(results[1].0, id_old);
        assert!(results[0].1 > results[1].1);
    }

    // -- MemoryEngine tests --

    #[test]
    fn engine_remember_recall_roundtrip() {
        let store = InMemoryStore::new();
        let pipeline = make_pipeline();
        let mut engine = MemoryEngine::new(Box::new(store), pipeline);

        let id = engine.remember("the cat sat on the mat").unwrap();
        let results = engine.recall("cat on mat", 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, id);
        assert!(results[0].combined_score > 0.0);
    }

    #[test]
    fn engine_recall_ranks_relevant_higher() {
        let store = InMemoryStore::new();
        let pipeline = make_pipeline();
        let mut engine = MemoryEngine::new(Box::new(store), pipeline);

        engine.remember("the cat sat on the mat").unwrap();
        engine.remember("quantum physics and string theory").unwrap();
        engine.remember("dogs playing in the park").unwrap();

        let results = engine.recall("cat mat", 3).unwrap();
        // The cat memory should be most relevant
        let top = engine.get_memory(&results[0].id).unwrap().unwrap();
        assert_eq!(top.content, "the cat sat on the mat");
    }

    #[test]
    fn engine_get_memory() {
        let store = InMemoryStore::new();
        let pipeline = make_pipeline();
        let mut engine = MemoryEngine::new(Box::new(store), pipeline);

        let id = engine.remember("test memory").unwrap();
        let mem = engine.get_memory(&id).unwrap().unwrap();
        assert_eq!(mem.content, "test memory");

        let fake = Uuid::new_v4();
        assert!(engine.get_memory(&fake).unwrap().is_none());
    }
}
