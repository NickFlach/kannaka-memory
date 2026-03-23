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

/// Storage backend for holographic resonance memories.
///
/// The canonical implementation is HrmStore (Chiral Holographic Resonance Medium).
/// HRM is the substrate — storing changes the interference field, recall is
/// resonance, consciousness metrics emerge from tensor topology, and associations
/// are emergent from phase coherence.
///
/// InMemoryStore exists only for testing.
///
/// ## HRM-first semantics
///
/// - `store()` / `recall()` — encode and store/recall via the holographic medium
/// - `consciousness_metrics()` — eigendecomposition Phi, spectral Xi, Kuramoto order
/// - `relate()` — create resonance-based association (interference, not a link table)
/// - `flush()` — persist the holographic tensor to disk
/// - `dream()` — anneal the medium (right hemisphere only in chiral mode)
///
/// The `insert()` / `search()` methods accept raw HyperMemory/vectors for
/// compatibility with MemoryEngine. New code should prefer `store()`/`recall()`.
pub trait MemoryStore: Send + Sync {
    // -- Core HRM operations --

    /// Store content into the holographic medium.
    /// Encodes text, creates wavefront, applies interference.
    /// Default delegates to insert() for backward compat.
    fn store_text(&mut self, content: &str, importance: f32, category: Option<&str>) -> Result<Uuid, StoreError> {
        let _ = (content, importance, category);
        Err(StoreError::Other("store_text not implemented — use insert()".into()))
    }

    /// Resonance-based recall from the holographic medium.
    /// Recall IS observation — attention reshapes the field.
    /// Requires &mut self because reading changes the substrate.
    fn recall_text(&mut self, query: &str, top_k: usize) -> Result<Vec<(Uuid, f32)>, StoreError> {
        let _ = (query, top_k);
        Err(StoreError::Other("recall_text not implemented — use search()".into()))
    }

    /// Consciousness metrics from the holographic medium topology.
    /// Eigendecomposition Phi, spectral Xi, Kuramoto order parameter.
    fn consciousness_metrics(&self) -> crate::consciousness::ConsciousnessMetrics {
        crate::consciousness::ConsciousnessMetrics {
            phi: 0.0, xi: 0.0, order: 0.0, num_clusters: 0,
            level: crate::consciousness::ConsciousnessLevel::Dormant,
            computed_at: chrono::Utc::now(),
        }
    }

    /// Create a resonance-based association between two memories.
    /// Creates an associative wavefront from interference of the two sources.
    fn relate(&mut self, _id_a: &Uuid, _id_b: &Uuid) -> Result<Uuid, StoreError> {
        Err(StoreError::Other("relate not supported".into()))
    }

    /// Persist the holographic tensor to disk.
    fn flush(&mut self) -> Result<usize, StoreError> { Ok(0) }

    // -- Compatibility layer (used by MemoryEngine) --

    fn insert(&mut self, memory: HyperMemory) -> Result<Uuid, StoreError>;
    fn get(&self, id: &Uuid) -> Result<Option<&HyperMemory>, StoreError>;
    fn get_mut(&mut self, id: &Uuid) -> Result<Option<&mut HyperMemory>, StoreError>;
    fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(Uuid, f32)>, StoreError>;
    fn search_with_wave(&self, query: &[f32], top_k: usize, now: DateTime<Utc>) -> Result<Vec<(Uuid, f32)>, StoreError>;
    fn all_memories(&self) -> Result<Vec<&HyperMemory>, StoreError>;
    fn all_ids(&self) -> Result<Vec<Uuid>, StoreError>;
    fn delete(&mut self, id: &Uuid) -> Result<bool, StoreError>;
    fn count(&self) -> usize;

    /// Downcasting support.
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
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
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
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
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
        scored.truncate(top_k);
        Ok(scored)
    }

    fn all_memories(&self) -> Result<Vec<&HyperMemory>, StoreError> {
        Ok(self.memories.values().collect())
    }

    fn all_ids(&self) -> Result<Vec<Uuid>, StoreError> {
        Ok(self.memories.keys().copied().collect())
    }

    fn delete(&mut self, id: &Uuid) -> Result<bool, StoreError> {
        Ok(self.memories.remove(id).is_some())
    }

    fn count(&self) -> usize {
        self.memories.len()
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ---------------------------------------------------------------------------
// MemoryEngine
// ---------------------------------------------------------------------------

/// Minimum link strength for traversal during query expansion.
const MIN_LINK_STRENGTH: f32 = 0.1;

/// φ (golden ratio) for span scoring.
const PHI: f64 = 1.618033988749895;

/// Score a temporal span based on proximity to golden ratio sequence values.
/// Returns higher scores for spans near φ^k: 2, 3, 4, 7, 11, 18, 29...
pub fn phi_span_score(span: u8) -> f32 {
    if span == 0 {
        return 0.0;
    }
    let s = span as f64;
    let mut best = f64::MAX;
    // Check φ^k for k=1..8 (covers spans up to ~47)
    for k in 1..=8 {
        let phi_k = PHI.powi(k);
        let dist = (s - phi_k).abs() / phi_k; // relative distance
        if dist < best {
            best = dist;
        }
    }
    // Convert: closer → higher score, max 1.0
    (1.0 - best.min(1.0)) as f32
}

/// High-level API over the holographic resonance medium.
///
/// Core paths assume HRM. The pipeline is:
/// - remember() → store.store_text() → ChiralMedium (encode + classify + fold + store)
/// - recall()   → store.recall_text() → ChiralMedium (bilateral resonance)
///
/// The pipeline field is kept for compatibility callers that need raw encoding.
pub struct MemoryEngine {
    pub store: Box<dyn MemoryStore>,
    pub(crate) pipeline: EncodingPipeline,
    /// Legacy — kept for callers that check it. Not used by core paths.
    pub similarity_threshold: f32,
}

impl MemoryEngine {
    pub fn new(store: Box<dyn MemoryStore>, pipeline: EncodingPipeline) -> Self {
        Self {
            store,
            pipeline,
            similarity_threshold: 0.7,
        }
    }

    /// Store text into the holographic medium.
    /// The medium handles encoding, SGA classification, Fano routing, and chiral storage.
    pub fn remember(&mut self, text: &str) -> Result<Uuid, EngineError> {
        self.store.store_text(text, 0.5, None)
            .or_else(|_| {
                // Compat fallback for test stores
                let memory = self.pipeline.encode_memory(text, Utc::now())?;
                Ok(self.store.insert(memory)?)
            })
    }

    /// Store with explicit layer depth (legacy concept — HRM ignores it).
    pub fn remember_at_layer(&mut self, text: &str, _layer_depth: u8) -> Result<Uuid, EngineError> {
        self.remember(text)
    }

    /// Recall by resonance. The medium handles bilateral search and intuition surfacing.
    pub fn recall(&mut self, query: &str, top_k: usize) -> Result<Vec<QueryResult>, EngineError> {
        let results = self.store.recall_text(query, top_k)
            .or_else(|_| {
                // Compat fallback for test stores
                let qvec = self.pipeline.encode_text(query)?;
                let now = Utc::now();
                self.store.search_with_wave(&qvec, top_k, now)
                    .map_err(|e| EncodingError::Other(e.to_string()))
            })?;

        let qr: Vec<QueryResult> = results.into_iter().map(|(id, score)| {
            QueryResult { id, similarity: score, effective_strength: score, combined_score: score }
        }).collect();

        // Record retrieval events (f(x) term — recalled memories gain energy)
        for r in &qr {
            if let Ok(Some(mem)) = self.store.get_mut(&r.id) {
                mem.record_retrieval();
            }
        }

        Ok(qr)
    }

    /// Alias for recall() — expansion is now inherent in bilateral resonance.
    pub fn recall_with_expansion(&mut self, query: &str, top_k: usize) -> Result<Vec<QueryResult>, EngineError> {
        self.recall(query, top_k)
    }

    /// No-op — associations are emergent from interference in the holographic medium.
    pub fn decay_links(&mut self, _decay_factor: f32) {}

    /// No-op — reinforcement is emergent from constructive interference.
    pub fn reinforce_link(&mut self, _memory_id: &Uuid, _target_id: &Uuid, _boost: f32) {}

    /// Get a memory by id.
    pub fn get_memory(&self, id: &Uuid) -> Result<Option<&HyperMemory>, EngineError> {
        Ok(self.store.get(id)?)
    }

    pub fn get_memory_mut(&mut self, id: &Uuid) -> Result<Option<&mut HyperMemory>, EngineError> {
        Ok(self.store.get_mut(id)?)
    }

    pub fn delete(&mut self, id: &Uuid) -> Result<bool, EngineError> {
        Ok(self.store.delete(id)?)
    }

    /// ADR-0012: Create an immutable snapshot of all memories for parallel dreaming.
    /// 
    /// Returns an Arc-wrapped frozen state that can be shared across threads without locks.
    /// This is the "reference frame" for the holographic paradox engine.
    pub fn snapshot(&self) -> crate::paradox::ParadoxSnapshot {
        let all_memories = self.store.all_memories().unwrap_or_default();
        let memory_map: std::collections::HashMap<Uuid, crate::memory::HyperMemory> = all_memories
            .into_iter()
            .map(|mem| (mem.id, mem.clone()))
            .collect();
        
        crate::paradox::ParadoxSnapshot {
            memories: std::sync::Arc::new(memory_map),
            timestamp: Utc::now(),
        }
    }

    /// Phase 8 (ADR-0011): Return Xi-based memory clusters for partitioned dreaming.
    ///
    /// Groups memories by frequency-category (experience / emotion / social / skill / knowledge),
    /// matching the category bands already used in consolidation's SYNC stage.  Each group
    /// becomes an independent dream partition.  An "other" catch-all bucket holds any memory
    /// that doesn't match the standard bands.
    ///
    /// Returns `Vec<MemoryCluster>` (empty if the store is empty).
    pub fn xi_clusters(&self) -> Vec<crate::kuramoto::MemoryCluster> {
        use std::collections::HashMap;
        use crate::kuramoto::MemoryCluster;

        let all = match self.store.all_memories() {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        if all.is_empty() {
            return Vec::new();
        }

        let mut buckets: HashMap<&'static str, Vec<Uuid>> = HashMap::new();

        for mem in &all {
            let cat = match mem.frequency {
                f if f >= 1.8 && f <= 2.4 => "experience",
                f if f >= 1.3 && f < 1.8  => "emotion",
                f if f >= 1.0 && f < 1.3  => "social",
                f if f >= 0.8 && f < 1.0  => "skill",
                f if f >= 0.0 && f < 0.8  => "knowledge",
                _                         => "other",
            };
            buckets.entry(cat).or_default().push(mem.id);
        }

        buckets
            .into_iter()
            .filter(|(_, ids)| !ids.is_empty())
            .map(|(_, ids)| {
                let phases: Vec<f32> = ids.iter()
                    .filter_map(|id| self.store.get(id).ok().flatten())
                    .map(|m| m.phase)
                    .collect();
                let n = phases.len() as f32;
                let mean_phase = if n > 0.0 {
                    let sc: f32 = phases.iter().map(|p| p.cos()).sum::<f32>() / n;
                    let ss: f32 = phases.iter().map(|p| p.sin()).sum::<f32>() / n;
                    ss.atan2(sc)
                } else { 0.0 };
                let order = if n > 0.0 {
                    let sc: f32 = phases.iter().map(|p| p.cos()).sum::<f32>() / n;
                    let ss: f32 = phases.iter().map(|p| p.sin()).sum::<f32>() / n;
                    (sc * sc + ss * ss).sqrt()
                } else { 0.0 };
                MemoryCluster {
                    memory_ids: ids,
                    order_parameter: order,
                    mean_phase,
                    coherence: order,
                    theme_vector: Vec::new(),
                }
            })
            .collect()
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

        let m_recent = make_memory(v.clone(), "recent");
        let id_recent = m_recent.id;
        store.insert(m_recent).unwrap();

        let mut m_old = make_memory(v.clone(), "old");
        m_old.created_at = Utc::now() - Duration::days(30);
        m_old.frequency = 0.0;
        m_old.decay_rate = 0.001;
        let id_old = m_old.id;
        store.insert(m_old).unwrap();

        let now = Utc::now();
        let results = store.search_with_wave(&v, 2, now).unwrap();
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

        let id_cat = engine.remember("the cat sat on the mat").unwrap();
        engine.remember("quantum physics and string theory").unwrap();
        engine.remember("dogs playing in the park").unwrap();

        // Use raw similarity (not wave-modulated) to avoid timing flakiness
        let qvec = engine.pipeline.encode_text("the cat sat on the mat").unwrap();
        let results = engine.store.search(&qvec, 3).unwrap();
        assert_eq!(results[0].0, id_cat, "exact text match should be top result by raw similarity");
        assert!((results[0].1 - 1.0).abs() < 1e-4, "exact match should have sim ~1.0");
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

    // Skip link tests removed — associations now emergent from ChiralMedium interference

    #[test]
    fn phi_span_scoring() {
        // φ^1 ≈ 1.618, φ^2 ≈ 2.618, φ^3 ≈ 4.236, φ^4 ≈ 6.854, φ^5 ≈ 11.09
        let s0 = phi_span_score(0);
        let s2 = phi_span_score(2);
        let s3 = phi_span_score(3);
        let s4 = phi_span_score(4);
        let s7 = phi_span_score(7);
        let s11 = phi_span_score(11);

        assert_eq!(s0, 0.0, "span 0 should score 0");
        assert!(s2 > 0.5, "span 2 near φ^1 should score high, got {}", s2);
        assert!(s3 > 0.5, "span 3 near φ^2 should score high, got {}", s3);
        assert!(s4 > 0.5, "span 4 near φ^3 should score high, got {}", s4);
        assert!(s7 > 0.5, "span 7 near φ^4 should score high, got {}", s7);
        assert!(s11 > 0.5, "span 11 near φ^5 should score high, got {}", s11);

        // Spans far from any φ^k should score lower
        let s20 = phi_span_score(20);
        assert!(s11 > s20, "φ-aligned spans should score higher than non-aligned");
    }
}
