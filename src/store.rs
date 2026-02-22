//! Storage layer: MemoryStore trait, InMemoryStore, and MemoryEngine.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

use crate::encoding::{EncodingError, EncodingPipeline};
use crate::memory::HyperMemory;
use crate::xi_operator::{xi_diversity_boost, compute_xi_signature};
use crate::skip_link::SkipLink;
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
    fn all_ids(&self) -> Result<Vec<Uuid>, StoreError>;
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

    fn all_ids(&self) -> Result<Vec<Uuid>, StoreError> {
        Ok(self.memories.keys().copied().collect())
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

/// High-level API: remember() and recall() over a pluggable store.
pub struct MemoryEngine {
    pub(crate) store: Box<dyn MemoryStore>,
    pub(crate) pipeline: EncodingPipeline,
    /// Threshold for automatic skip link creation
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

    /// Encode text and store as a new memory. Returns the memory id.
    pub fn remember(&mut self, text: &str) -> Result<Uuid, EngineError> {
        let memory = self.pipeline.encode_memory(text, Utc::now())?;
        let id = self.store.insert(memory)?;
        // Wire up skip links to similar existing memories
        let _links = self.create_skip_links(&id)?;
        Ok(id)
    }

    /// Encode text and store with a specific layer_depth. Returns the memory id.
    pub fn remember_at_layer(&mut self, text: &str, layer_depth: u8) -> Result<Uuid, EngineError> {
        let mut memory = self.pipeline.encode_memory(text, Utc::now())?;
        memory.layer_depth = layer_depth;
        let id = self.store.insert(memory)?;
        let _links = self.create_skip_links(&id)?;
        Ok(id)
    }

    /// Create skip links from a new memory to similar existing memories.
    /// Links are only created when memories are at different temporal layers
    /// and similarity exceeds the threshold.
    pub fn create_skip_links(&mut self, new_id: &Uuid) -> Result<Vec<SkipLink>, EngineError> {
        let new_mem = self.store.get(new_id)?.ok_or(StoreError::NotFound(*new_id))?;
        let new_vec = new_mem.vector.clone();
        let new_layer = new_mem.layer_depth;
        let threshold = self.similarity_threshold;

        // Find all similar memories at different layers
        let all = self.store.all_memories()?;
        let mut links_to_create: Vec<(Uuid, f32, u8)> = Vec::new(); // (target_id, sim, span)

        for mem in &all {
            if mem.id == *new_id {
                continue;
            }
            if mem.layer_depth == new_layer {
                continue;
            }
            let sim = cosine_similarity(&new_vec, &mem.vector);
            if sim > threshold {
                let span = (new_layer as i16 - mem.layer_depth as i16).unsigned_abs() as u8;
                links_to_create.push((mem.id, sim, span));
            }
        }

        // Create SkipLinks with φ-weighted strength
        let mut created_links = Vec::new();
        for (target_id, sim, span) in &links_to_create {
            let phi_weight = phi_span_score(*span);
            let strength = sim * (0.5 + 0.5 * phi_weight); // base strength + φ bonus

            let link = SkipLink {
                target_id: *target_id,
                strength,
                resonance_key: Vec::new(), // simplified for now
                span: *span,
            };
            created_links.push(link.clone());

            // Also create reverse link
            let reverse_link = SkipLink {
                target_id: *new_id,
                strength,
                resonance_key: Vec::new(),
                span: *span,
            };
            if let Some(target_mem) = self.store.get_mut(target_id)? {
                target_mem.connections.push(reverse_link);
            }
        }

        // Add forward links to new memory
        if !created_links.is_empty() {
            if let Some(new_mem) = self.store.get_mut(new_id)? {
                new_mem.connections.extend(created_links.clone());
            }
        }

        Ok(created_links)
    }

    /// Encode a query and search with wave-modulated ranking and Xi diversity boosting.
    pub fn recall(&self, query: &str, top_k: usize) -> Result<Vec<QueryResult>, EngineError> {
        let qvec = self.pipeline.encode_text(query)?;
        let query_xi = compute_xi_signature(&qvec);
        let now = Utc::now();
        let raw = self.store.search(&qvec, self.store.count())?;
        let raw_map: HashMap<Uuid, f32> = raw.into_iter().collect();
        let wave_results = self.store.search_with_wave(&qvec, top_k * 2, now)?; // Get more candidates for diversity

        let results = wave_results
            .into_iter()
            .map(|(id, combined)| {
                let base_similarity = raw_map.get(&id).copied().unwrap_or(0.0);
                
                // Apply Xi diversity boosting
                let xi_boosted_similarity = if let Ok(Some(mem)) = self.store.get(&id) {
                    let mem_xi = if mem.xi_signature.is_empty() {
                        // Compute on-the-fly for backward compatibility
                        compute_xi_signature(&mem.vector)
                    } else {
                        mem.xi_signature.clone()
                    };
                    xi_diversity_boost(base_similarity, &query_xi, &mem_xi)
                } else {
                    base_similarity
                };
                
                let effective_strength = if base_similarity.abs() > 1e-9 {
                    combined / base_similarity
                } else {
                    0.0
                };
                
                QueryResult {
                    id,
                    similarity: xi_boosted_similarity, // Use Xi-boosted similarity
                    effective_strength,
                    combined_score: combined * (xi_boosted_similarity / base_similarity.max(1e-9)),
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .take(top_k) // Take only top_k after Xi boosting
            .collect();
            
        Ok(results)
    }

    /// Recall with skip link expansion — follows connections to find related memories.
    pub fn recall_with_expansion(
        &mut self,
        query: &str,
        top_k: usize,
    ) -> Result<Vec<QueryResult>, EngineError> {
        let qvec = self.pipeline.encode_text(query)?;
        let query_xi = compute_xi_signature(&qvec);
        let now = Utc::now();

        // Step 1: Get initial candidates (top_k * 3)
        let initial = self.store.search_with_wave(&qvec, top_k * 3, now)?;
        let raw_all = self.store.search(&qvec, self.store.count())?;
        let raw_map: HashMap<Uuid, f32> = raw_all.into_iter().collect();

        // Step 2: Follow skip links from candidates
        let mut candidate_scores: HashMap<Uuid, f32> = HashMap::new();
        let mut links_traversed: Vec<(Uuid, Uuid)> = Vec::new();

        for (id, combined) in &initial {
            candidate_scores.insert(*id, *combined);

            // Follow skip links
            if let Some(mem) = self.store.get(id)? {
                for link in mem.connections.clone() {
                    if link.strength > MIN_LINK_STRENGTH {
                        let linked_sim = raw_map.get(&link.target_id).copied().unwrap_or(0.0);
                        if linked_sim > 0.0 {
                            let boosted = linked_sim * link.strength;
                            let entry = candidate_scores
                                .entry(link.target_id)
                                .or_insert(0.0);
                            if boosted > *entry {
                                *entry = boosted;
                            }
                            links_traversed.push((*id, link.target_id));
                        }
                    }
                }
            }
        }

        // Step 3: Reinforce traversed links
        for (from_id, to_id) in &links_traversed {
            self.reinforce_link(from_id, to_id, 0.05);
        }

        // Step 4: Re-rank with Xi diversity boosting and return top_k
        let mut results: Vec<QueryResult> = candidate_scores
            .into_iter()
            .map(|(id, combined_score)| {
                let base_similarity = raw_map.get(&id).copied().unwrap_or(0.0);
                
                // Apply Xi diversity boosting
                let xi_boosted_similarity = if let Ok(Some(mem)) = self.store.get(&id) {
                    let mem_xi = if mem.xi_signature.is_empty() {
                        // Compute on-the-fly for backward compatibility
                        compute_xi_signature(&mem.vector)
                    } else {
                        mem.xi_signature.clone()
                    };
                    xi_diversity_boost(base_similarity, &query_xi, &mem_xi)
                } else {
                    base_similarity
                };
                
                let effective_strength = if base_similarity.abs() > 1e-9 {
                    combined_score / base_similarity
                } else {
                    0.0
                };
                
                QueryResult {
                    id,
                    similarity: xi_boosted_similarity,
                    effective_strength,
                    combined_score: combined_score * (xi_boosted_similarity / base_similarity.max(1e-9)),
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.combined_score
                .partial_cmp(&a.combined_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);
        Ok(results)
    }

    /// Decay all skip link strengths by a factor (0..1).
    pub fn decay_links(&mut self, decay_factor: f32) {
        if let Ok(memories) = self.store.all_ids() {
            for id in memories {
                if let Ok(Some(mem)) = self.store.get_mut(&id) {
                    for link in &mut mem.connections {
                        link.strength *= decay_factor;
                    }
                }
            }
        }
    }

    /// Reinforce a skip link between two memories.
    pub fn reinforce_link(&mut self, memory_id: &Uuid, target_id: &Uuid, boost: f32) {
        if let Ok(Some(mem)) = self.store.get_mut(memory_id) {
            for link in &mut mem.connections {
                if link.target_id == *target_id {
                    link.strength = (link.strength + boost).min(1.0);
                }
            }
        }
    }

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

    // -- HyperConnections tests --

    #[test]
    fn skip_links_created_for_similar_memories_at_different_layers() {
        let store = InMemoryStore::new();
        let pipeline = make_pipeline();
        let mut engine = MemoryEngine::new(Box::new(store), pipeline);
        engine.similarity_threshold = 0.3; // lower for test (hash encoder produces moderate similarity)

        // Insert similar content at different layers
        let id1 = engine.remember_at_layer("the cat sat on the mat", 0).unwrap();
        let id2 = engine.remember_at_layer("the cat sat on the mat today", 2).unwrap();

        let mem2 = engine.get_memory(&id2).unwrap().unwrap();
        assert!(!mem2.connections.is_empty(), "should have skip links to similar memory at different layer");
        assert_eq!(mem2.connections[0].target_id, id1);

        // Verify reverse link
        let mem1 = engine.get_memory(&id1).unwrap().unwrap();
        assert!(!mem1.connections.is_empty(), "reverse link should exist");
        assert_eq!(mem1.connections[0].target_id, id2);
    }

    #[test]
    fn skip_links_not_created_for_dissimilar_memories() {
        let store = InMemoryStore::new();
        let pipeline = make_pipeline();
        let mut engine = MemoryEngine::new(Box::new(store), pipeline);
        // Use high threshold so hash-encoder similarity doesn't trigger links
        engine.similarity_threshold = 0.95;

        let _id1 = engine.remember_at_layer("quantum physics string theory", 0).unwrap();
        let id2 = engine.remember_at_layer("cooking pasta with tomato sauce recipe", 2).unwrap();

        let mem2 = engine.get_memory(&id2).unwrap().unwrap();
        assert!(mem2.connections.is_empty(), "dissimilar memories should not be linked");
    }

    #[test]
    fn skip_links_not_created_for_same_layer() {
        let store = InMemoryStore::new();
        let pipeline = make_pipeline();
        let mut engine = MemoryEngine::new(Box::new(store), pipeline);
        engine.similarity_threshold = 0.0; // even with zero threshold

        let _id1 = engine.remember_at_layer("the cat sat on the mat", 0).unwrap();
        let id2 = engine.remember_at_layer("the cat sat on the mat again", 0).unwrap();

        let mem2 = engine.get_memory(&id2).unwrap().unwrap();
        assert!(mem2.connections.is_empty(), "same-layer memories should not be linked");
    }

    #[test]
    fn recall_with_expansion_finds_linked_memories() {
        let store = InMemoryStore::new();
        let pipeline = make_pipeline();
        let mut engine = MemoryEngine::new(Box::new(store), pipeline);
        engine.similarity_threshold = 0.3;

        // Create a chain: A (layer 0) --link--> B (layer 2) --link--> C (layer 4)
        let _id_a = engine.remember_at_layer("the cat sat on the mat", 0).unwrap();
        let _id_b = engine.remember_at_layer("the cat sat on the mat yesterday", 2).unwrap();
        let _id_c = engine.remember_at_layer("cats sitting on mats is common", 4).unwrap();

        // Query should find all related via expansion
        let results = engine.recall_with_expansion("cat mat", 10).unwrap();
        assert!(results.len() >= 2, "expansion should find multiple linked memories, got {}", results.len());
    }

    #[test]
    fn link_reinforcement_increases_strength() {
        let store = InMemoryStore::new();
        let pipeline = make_pipeline();
        let mut engine = MemoryEngine::new(Box::new(store), pipeline);
        engine.similarity_threshold = 0.3;

        let id1 = engine.remember_at_layer("the cat sat on the mat", 0).unwrap();
        let id2 = engine.remember_at_layer("the cat sat on the mat today", 2).unwrap();

        let initial_strength = engine.get_memory(&id2).unwrap().unwrap()
            .connections.iter().find(|l| l.target_id == id1).unwrap().strength;

        engine.reinforce_link(&id2, &id1, 0.1);

        let new_strength = engine.get_memory(&id2).unwrap().unwrap()
            .connections.iter().find(|l| l.target_id == id1).unwrap().strength;

        assert!(new_strength > initial_strength, "reinforcement should increase strength");
    }

    #[test]
    fn link_decay_decreases_strength() {
        let store = InMemoryStore::new();
        let pipeline = make_pipeline();
        let mut engine = MemoryEngine::new(Box::new(store), pipeline);
        engine.similarity_threshold = 0.3;

        let id1 = engine.remember_at_layer("the cat sat on the mat", 0).unwrap();
        let id2 = engine.remember_at_layer("the cat sat on the mat today", 2).unwrap();

        let initial_strength = engine.get_memory(&id2).unwrap().unwrap()
            .connections.iter().find(|l| l.target_id == id1).unwrap().strength;

        engine.decay_links(0.5);

        let new_strength = engine.get_memory(&id2).unwrap().unwrap()
            .connections.iter().find(|l| l.target_id == id1).unwrap().strength;

        assert!(
            (new_strength - initial_strength * 0.5).abs() < 1e-5,
            "decay should halve strength: expected {}, got {}",
            initial_strength * 0.5, new_strength
        );
    }

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
