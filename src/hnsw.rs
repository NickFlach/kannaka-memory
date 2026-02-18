//! Lightweight HNSW (Hierarchical Navigable Small World) index for
//! approximate nearest neighbor search in O(log n).

use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::wave::cosine_similarity;

// ---------------------------------------------------------------------------
// Scored neighbor (max-heap by similarity)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct Candidate {
    id: Uuid,
    similarity: f32,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.similarity == other.similarity
    }
}
impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.similarity
            .partial_cmp(&other.similarity)
            .unwrap_or(Ordering::Equal)
    }
}

/// Reverse-ordered candidate for min-heap usage.
#[derive(Clone, Debug)]
struct RevCandidate(Candidate);

impl PartialEq for RevCandidate {
    fn eq(&self, other: &Self) -> bool { self.0.eq(&other.0) }
}
impl Eq for RevCandidate {}
impl PartialOrd for RevCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for RevCandidate {
    fn cmp(&self, other: &Self) -> Ordering { other.0.cmp(&self.0) }
}

// ---------------------------------------------------------------------------
// HNSW Node
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HnswNode {
    id: Uuid,
    vector: Vec<f32>,
    /// Neighbors at each layer. neighbors[layer] = vec of neighbor ids.
    neighbors: Vec<Vec<Uuid>>,
    level: usize,
}

// ---------------------------------------------------------------------------
// HnswIndex
// ---------------------------------------------------------------------------

/// A lightweight HNSW index for approximate nearest neighbor search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswIndex {
    max_layers: usize,
    ef_construction: usize,
    ef_search: usize,
    m: usize,
    m_max0: usize,
    nodes: HashMap<Uuid, HnswNode>,
    entry_point: Option<Uuid>,
    max_level: usize,
    ml: f64, // normalization factor for level generation: 1/ln(M)
}

impl HnswIndex {
    /// Create a new HNSW index with default parameters.
    pub fn new() -> Self {
        Self::with_params(6, 200, 50, 16)
    }

    /// Create with custom parameters.
    pub fn with_params(max_layers: usize, ef_construction: usize, ef_search: usize, m: usize) -> Self {
        let ml = 1.0 / (m as f64).ln();
        Self {
            max_layers,
            ef_construction,
            ef_search,
            m,
            m_max0: m * 2,
            nodes: HashMap::new(),
            entry_point: None,
            max_level: 0,
            ml,
        }
    }

    /// Number of vectors in the index.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Generate a random level for a new node.
    fn random_level(&self) -> usize {
        let mut rng = rand::thread_rng();
        let r: f64 = rng.gen::<f64>();
        let level = (-r.ln() * self.ml).floor() as usize;
        level.min(self.max_layers - 1)
    }

    /// Insert a vector into the index.
    pub fn insert(&mut self, id: Uuid, vector: &[f32]) {
        let level = self.random_level();

        let node = HnswNode {
            id,
            vector: vector.to_vec(),
            neighbors: vec![Vec::new(); level + 1],
            level,
        };

        // First node
        if self.entry_point.is_none() {
            self.entry_point = Some(id);
            self.max_level = level;
            self.nodes.insert(id, node);
            return;
        }

        let ep = self.entry_point.unwrap();
        self.nodes.insert(id, node);

        // Phase 1: Greedy descent from top to level+1
        let mut current_ep = ep;
        for lc in (level + 1..=self.max_level).rev() {
            current_ep = self.greedy_closest(vector, current_ep, lc);
        }

        // Phase 2: Insert at each layer from min(level, max_level) down to 0
        let start_layer = level.min(self.max_level);
        let mut ep_set = vec![current_ep];

        for lc in (0..=start_layer).rev() {
            let m_max = if lc == 0 { self.m_max0 } else { self.m };
            let candidates = self.search_layer(vector, &ep_set, self.ef_construction, lc);

            // Select M nearest
            let neighbors: Vec<Uuid> = candidates.iter()
                .take(m_max)
                .map(|c| c.id)
                .collect();

            // Set neighbors for the new node at this layer
            if let Some(n) = self.nodes.get_mut(&id) {
                if lc < n.neighbors.len() {
                    n.neighbors[lc] = neighbors.clone();
                }
            }

            // Add bidirectional connections
            for &nb_id in &neighbors {
                // Add id to neighbor's neighbor list
                let nb_level = self.nodes.get(&nb_id).map(|n| n.level).unwrap_or(0);
                if lc <= nb_level {
                    if let Some(nb) = self.nodes.get_mut(&nb_id) {
                        if lc < nb.neighbors.len() && !nb.neighbors[lc].contains(&id) {
                            nb.neighbors[lc].push(id);
                            // Prune if over capacity
                            if nb.neighbors[lc].len() > m_max {
                                self.prune_neighbors(nb_id, lc, m_max);
                            }
                        }
                    }
                }
            }

            // Use top candidates as entry points for next layer
            ep_set = candidates.iter().take(self.ef_construction).map(|c| c.id).collect();
        }

        // Update entry point if new node has higher level
        if level > self.max_level {
            self.entry_point = Some(id);
            self.max_level = level;
        }
    }

    /// Prune a node's neighbors at a given layer to keep only the best `m_max`.
    fn prune_neighbors(&mut self, node_id: Uuid, layer: usize, m_max: usize) {
        let node_vec = match self.nodes.get(&node_id) {
            Some(n) => n.vector.clone(),
            None => return,
        };
        let neighbors = match self.nodes.get(&node_id) {
            Some(n) if layer < n.neighbors.len() => n.neighbors[layer].clone(),
            _ => return,
        };

        let mut scored: Vec<(Uuid, f32)> = neighbors.iter()
            .filter_map(|&nb_id| {
                self.nodes.get(&nb_id).map(|nb| (nb_id, cosine_similarity(&node_vec, &nb.vector)))
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        scored.truncate(m_max);

        if let Some(n) = self.nodes.get_mut(&node_id) {
            if layer < n.neighbors.len() {
                n.neighbors[layer] = scored.into_iter().map(|(id, _)| id).collect();
            }
        }
    }

    /// Greedily find the closest node to `query` starting from `ep` at `layer`.
    fn greedy_closest(&self, query: &[f32], ep: Uuid, layer: usize) -> Uuid {
        let mut current = ep;
        let mut current_sim = self.nodes.get(&ep)
            .map(|n| cosine_similarity(query, &n.vector))
            .unwrap_or(-1.0);

        loop {
            let mut changed = false;
            let neighbors = match self.nodes.get(&current) {
                Some(n) if layer < n.neighbors.len() => n.neighbors[layer].clone(),
                _ => break,
            };
            for &nb_id in &neighbors {
                if let Some(nb) = self.nodes.get(&nb_id) {
                    let sim = cosine_similarity(query, &nb.vector);
                    if sim > current_sim {
                        current = nb_id;
                        current_sim = sim;
                        changed = true;
                    }
                }
            }
            if !changed { break; }
        }
        current
    }

    /// Search a single layer with beam search (ef candidates).
    fn search_layer(&self, query: &[f32], entry_points: &[Uuid], ef: usize, layer: usize) -> Vec<Candidate> {
        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut candidates: BinaryHeap<Candidate> = BinaryHeap::new(); // max-heap
        let mut results: BinaryHeap<RevCandidate> = BinaryHeap::new(); // min-heap (worst at top)

        for &ep in entry_points {
            if visited.insert(ep) {
                if let Some(n) = self.nodes.get(&ep) {
                    let sim = cosine_similarity(query, &n.vector);
                    let c = Candidate { id: ep, similarity: sim };
                    candidates.push(c.clone());
                    results.push(RevCandidate(c));
                }
            }
        }

        while let Some(c) = candidates.pop() {
            // If best candidate is worse than worst result, stop
            if let Some(worst) = results.peek() {
                if c.similarity < worst.0.similarity && results.len() >= ef {
                    break;
                }
            }

            let neighbors = match self.nodes.get(&c.id) {
                Some(n) if layer < n.neighbors.len() => n.neighbors[layer].clone(),
                _ => continue,
            };

            for nb_id in neighbors {
                if !visited.insert(nb_id) { continue; }
                if let Some(nb) = self.nodes.get(&nb_id) {
                    let sim = cosine_similarity(query, &nb.vector);
                    let should_add = results.len() < ef || {
                        results.peek().map(|w| sim > w.0.similarity).unwrap_or(true)
                    };
                    if should_add {
                        let cand = Candidate { id: nb_id, similarity: sim };
                        candidates.push(cand.clone());
                        results.push(RevCandidate(cand));
                        if results.len() > ef {
                            results.pop(); // remove worst
                        }
                    }
                }
            }
        }

        // Extract sorted by similarity descending
        let mut out: Vec<Candidate> = results.into_iter().map(|rc| rc.0).collect();
        out.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(Ordering::Equal));
        out
    }

    /// Search for the top_k nearest neighbors of `query`.
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(Uuid, f32)> {
        if self.nodes.is_empty() {
            return Vec::new();
        }

        let ep = match self.entry_point {
            Some(ep) => ep,
            None => return Vec::new(),
        };

        // Greedy descent from top layer
        let mut current_ep = ep;
        if self.max_level > 0 {
            for lc in (1..=self.max_level).rev() {
                current_ep = self.greedy_closest(query, current_ep, lc);
            }
        }

        // Search layer 0 with ef_search candidates
        let ef = self.ef_search.max(top_k);
        let candidates = self.search_layer(query, &[current_ep], ef, 0);

        candidates.into_iter()
            .take(top_k)
            .map(|c| (c.id, c.similarity))
            .collect()
    }

    /// Remove a vector from the index.
    pub fn remove(&mut self, id: &Uuid) -> bool {
        let node = match self.nodes.remove(id) {
            Some(n) => n,
            None => return false,
        };

        // Remove id from all neighbors' neighbor lists
        for (layer, neighbors) in node.neighbors.iter().enumerate() {
            for nb_id in neighbors {
                if let Some(nb) = self.nodes.get_mut(nb_id) {
                    if layer < nb.neighbors.len() {
                        nb.neighbors[layer].retain(|x| x != id);
                    }
                }
            }
        }

        // If we removed the entry point, pick a new one
        if self.entry_point == Some(*id) {
            self.entry_point = self.nodes.keys().next().copied();
            self.max_level = self.entry_point
                .and_then(|ep| self.nodes.get(&ep).map(|n| n.level))
                .unwrap_or(0);
            // Find actual max level
            for n in self.nodes.values() {
                if n.level > self.max_level {
                    self.max_level = n.level;
                    self.entry_point = Some(n.id);
                }
            }
        }

        true
    }
}

impl Default for HnswIndex {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// HnswStore — MemoryStore backed by HNSW index
// ---------------------------------------------------------------------------

use chrono::{DateTime, Utc};
use crate::memory::HyperMemory;
use crate::store::{MemoryStore, StoreError};

/// Brute-force fallback threshold: use HNSW only above this count.
const HNSW_THRESHOLD: usize = 100;

/// MemoryStore implementation using HNSW for similarity search.
#[derive(Clone, Serialize, Deserialize)]
pub struct HnswStore {
    memories: HashMap<Uuid, HyperMemory>,
    index: HnswIndex,
}

impl HnswStore {
    pub fn new() -> Self {
        Self {
            memories: HashMap::new(),
            index: HnswIndex::new(),
        }
    }

    pub fn with_params(max_layers: usize, ef_construction: usize, ef_search: usize, m: usize) -> Self {
        Self {
            memories: HashMap::new(),
            index: HnswIndex::with_params(max_layers, ef_construction, ef_search, m),
        }
    }

    /// Brute-force search (fallback for small stores).
    fn brute_force_search(&self, query: &[f32], top_k: usize) -> Vec<(Uuid, f32)> {
        let mut scored: Vec<(Uuid, f32)> = self.memories.values()
            .map(|m| (m.id, cosine_similarity(query, &m.vector)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        scored.truncate(top_k);
        scored
    }
}

impl Default for HnswStore {
    fn default() -> Self { Self::new() }
}

impl MemoryStore for HnswStore {
    fn insert(&mut self, memory: HyperMemory) -> Result<Uuid, StoreError> {
        let id = memory.id;
        if self.memories.contains_key(&id) {
            return Err(StoreError::DuplicateId(id));
        }
        self.index.insert(id, &memory.vector);
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
        if self.memories.len() < HNSW_THRESHOLD {
            return Ok(self.brute_force_search(query, top_k));
        }
        Ok(self.index.search(query, top_k))
    }

    fn search_with_wave(
        &self,
        query: &[f32],
        top_k: usize,
        now: DateTime<Utc>,
    ) -> Result<Vec<(Uuid, f32)>, StoreError> {
        if self.memories.len() < HNSW_THRESHOLD {
            // Brute-force with wave modulation
            let mut scored: Vec<(Uuid, f32)> = self.memories.values()
                .map(|m| {
                    let sim = cosine_similarity(query, &m.vector);
                    let strength = m.effective_strength(now);
                    (m.id, sim * strength)
                })
                .collect();
            scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
            scored.truncate(top_k);
            return Ok(scored);
        }

        // HNSW: get more candidates, then re-rank with wave modulation
        let candidates = self.index.search(query, top_k * 3);
        let mut scored: Vec<(Uuid, f32)> = candidates.into_iter()
            .filter_map(|(id, sim)| {
                self.memories.get(&id).map(|m| {
                    let strength = m.effective_strength(now);
                    (id, sim * strength)
                })
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
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
        if self.memories.remove(id).is_some() {
            self.index.remove(id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn count(&self) -> usize {
        self.memories.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave::normalize;
    use crate::memory::HyperMemory;
    use chrono::Duration;
    use std::time::Instant;

    fn random_vector(dim: usize, seed: u64) -> Vec<f32> {
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut v: Vec<f32> = (0..dim).map(|_| rng.gen::<f32>() * 2.0 - 1.0).collect();
        normalize(&mut v);
        v
    }

    fn unit_vec(dim: usize, index: usize) -> Vec<f32> {
        let mut v = vec![0.0; dim];
        v[index] = 1.0;
        v
    }

    fn make_memory(vector: Vec<f32>, content: &str) -> HyperMemory {
        HyperMemory::new(vector, content.to_string())
    }

    // -- HnswIndex unit tests --

    #[test]
    fn hnsw_empty_search() {
        let index = HnswIndex::new();
        let results = index.search(&[1.0, 0.0, 0.0], 5);
        assert!(results.is_empty());
    }

    #[test]
    fn hnsw_single_insert_search() {
        let mut index = HnswIndex::new();
        let id = Uuid::new_v4();
        index.insert(id, &[1.0, 0.0, 0.0]);
        let results = index.search(&[1.0, 0.0, 0.0], 1);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, id);
        assert!((results[0].1 - 1.0).abs() < 1e-5);
    }

    #[test]
    fn hnsw_finds_nearest_neighbor() {
        let mut index = HnswIndex::new();
        let dim = 50;
        let target = random_vector(dim, 42);
        let target_id = Uuid::new_v4();
        index.insert(target_id, &target);

        // Insert 100 random vectors
        for i in 0..100 {
            index.insert(Uuid::new_v4(), &random_vector(dim, 100 + i));
        }

        // Query with target itself — should find it
        let results = index.search(&target, 1);
        assert_eq!(results[0].0, target_id);
        assert!((results[0].1 - 1.0).abs() < 1e-4);
    }

    #[test]
    fn hnsw_search_quality_recall() {
        // Test recall: fraction of true top-k found by HNSW
        let dim = 64;
        let n = 500;
        let top_k = 10;

        let mut index = HnswIndex::with_params(6, 200, 50, 16);
        let mut vectors: Vec<(Uuid, Vec<f32>)> = Vec::new();

        for i in 0..n {
            let id = Uuid::new_v4();
            let v = random_vector(dim, i as u64);
            index.insert(id, &v);
            vectors.push((id, v));
        }

        // Run multiple queries and measure recall
        let num_queries = 20;
        let mut total_recall = 0.0;

        for q in 0..num_queries {
            let query = random_vector(dim, 10000 + q);

            // Brute force ground truth
            let mut brute: Vec<(Uuid, f32)> = vectors.iter()
                .map(|(id, v)| (*id, cosine_similarity(&query, v)))
                .collect();
            brute.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
            let truth: HashSet<Uuid> = brute.iter().take(top_k).map(|(id, _)| *id).collect();

            // HNSW search
            let hnsw_results = index.search(&query, top_k);
            let found: HashSet<Uuid> = hnsw_results.iter().map(|(id, _)| *id).collect();

            let recall = truth.intersection(&found).count() as f64 / top_k as f64;
            total_recall += recall;
        }

        let avg_recall = total_recall / num_queries as f64;
        eprintln!("HNSW recall@{top_k} over {num_queries} queries on {n} vectors: {avg_recall:.3}");
        assert!(avg_recall > 0.8, "Recall should be > 0.8, got {avg_recall:.3}");
    }

    #[test]
    fn hnsw_remove() {
        let mut index = HnswIndex::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        index.insert(id1, &[1.0, 0.0]);
        index.insert(id2, &[0.0, 1.0]);
        assert_eq!(index.len(), 2);

        assert!(index.remove(&id1));
        assert_eq!(index.len(), 1);
        assert!(!index.remove(&id1)); // already removed

        let results = index.search(&[1.0, 0.0], 5);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, id2);
    }

    #[test]
    fn hnsw_performance_vs_bruteforce() {
        let dim = 128;
        let n = 2000;
        let top_k = 10;
        let num_queries = 5;

        let mut index = HnswIndex::with_params(6, 100, 50, 16);
        let mut vectors: Vec<Vec<f32>> = Vec::new();

        for i in 0..n {
            let v = random_vector(dim, i as u64);
            index.insert(Uuid::new_v4(), &v);
            vectors.push(v);
        }

        let queries: Vec<Vec<f32>> = (0..num_queries).map(|q| random_vector(dim, 50000 + q)).collect();

        // Time HNSW
        let start = Instant::now();
        for q in &queries {
            let _ = index.search(q, top_k);
        }
        let hnsw_time = start.elapsed();

        // Time brute force
        let start = Instant::now();
        for q in &queries {
            let mut scored: Vec<f32> = vectors.iter().map(|v| cosine_similarity(q, v)).collect();
            scored.sort_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));
            scored.truncate(top_k);
        }
        let brute_time = start.elapsed();

        eprintln!("HNSW: {:?}, Brute-force: {:?} ({n} vectors, {num_queries} queries)", hnsw_time, brute_time);
        // HNSW should be faster (or at least complete in reasonable time)
        assert!(hnsw_time.as_millis() < 5000, "HNSW search took too long: {:?}", hnsw_time);
    }

    // -- HnswStore tests (mirrors InMemoryStore tests) --

    #[test]
    fn hnsw_store_insert_get_count() {
        let mut store = HnswStore::new();
        assert_eq!(store.count(), 0);
        let mem = make_memory(vec![1.0; 10], "hello");
        let id = store.insert(mem).unwrap();
        assert_eq!(store.count(), 1);
        let got = store.get(&id).unwrap().unwrap();
        assert_eq!(got.content, "hello");
    }

    #[test]
    fn hnsw_store_delete() {
        let mut store = HnswStore::new();
        let mem = make_memory(vec![1.0; 10], "bye");
        let id = store.insert(mem).unwrap();
        assert!(store.delete(&id).unwrap());
        assert_eq!(store.count(), 0);
        assert!(!store.delete(&id).unwrap());
    }

    #[test]
    fn hnsw_store_duplicate_rejected() {
        let mut store = HnswStore::new();
        let mem = make_memory(vec![1.0; 10], "a");
        let id = mem.id;
        store.insert(mem).unwrap();
        let mut mem2 = make_memory(vec![2.0; 10], "b");
        mem2.id = id;
        assert!(matches!(store.insert(mem2), Err(StoreError::DuplicateId(_))));
    }

    #[test]
    fn hnsw_store_search_closest_first() {
        let mut store = HnswStore::new();
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

        // Under HNSW_THRESHOLD so brute-force is used
        let results = store.search(&v1, 3).unwrap();
        assert_eq!(results[0].0, id1);
        assert!((results[0].1 - 1.0).abs() < 1e-5);
    }

    #[test]
    fn hnsw_store_search_with_wave() {
        let mut store = HnswStore::new();
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

    #[test]
    fn hnsw_store_all_memories_and_ids() {
        let mut store = HnswStore::new();
        let m1 = make_memory(vec![1.0; 10], "a");
        let m2 = make_memory(vec![2.0; 10], "b");
        let id1 = m1.id;
        let id2 = m2.id;
        store.insert(m1).unwrap();
        store.insert(m2).unwrap();

        let all = store.all_memories().unwrap();
        assert_eq!(all.len(), 2);

        let ids = store.all_ids().unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn hnsw_store_persistence_roundtrip() {
        let mut store = HnswStore::new();
        for i in 0..50 {
            let v = random_vector(64, i);
            let m = make_memory(v, &format!("mem_{i}"));
            store.insert(m).unwrap();
        }

        // Serialize and deserialize
        let serialized = serde_json::to_string(&store).unwrap();
        let restored: HnswStore = serde_json::from_str(&serialized).unwrap();

        assert_eq!(restored.count(), store.count());
        assert_eq!(restored.index.len(), store.index.len());

        // Search should produce same results
        let query = random_vector(64, 9999);
        let orig_results = store.search(&query, 5).unwrap();
        let rest_results = restored.search(&query, 5).unwrap();
        assert_eq!(orig_results.len(), rest_results.len());
        for (o, r) in orig_results.iter().zip(rest_results.iter()) {
            assert_eq!(o.0, r.0); // same IDs
            assert!((o.1 - r.1).abs() < 1e-5); // same similarities
        }
    }

    #[test]
    fn hnsw_store_large_search_uses_index() {
        // Insert enough to exceed HNSW_THRESHOLD
        let mut store = HnswStore::new();
        let dim = 64;
        for i in 0..150 {
            let v = random_vector(dim, i);
            let m = make_memory(v, &format!("mem_{i}"));
            store.insert(m).unwrap();
        }

        // This should use HNSW index (> 100 memories)
        let query = random_vector(dim, 9999);
        let results = store.search(&query, 10).unwrap();
        assert_eq!(results.len(), 10);
        // Results should be sorted by similarity descending
        for w in results.windows(2) {
            assert!(w[0].1 >= w[1].1, "Results should be sorted: {} >= {}", w[0].1, w[1].1);
        }
    }
}
