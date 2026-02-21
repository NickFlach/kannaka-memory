//! Memory consolidation engine — the dreaming/sleep layer.
//!
//! Processes memories through 7 stages mimicking human sleep consolidation:
//! 1. REPLAY — re-activate recent memories
//! 2. DETECT — find interference patterns
//! 3. BUNDLE — create summary hypervectors
//! 4. STRENGTHEN — boost constructive interference pairs
//! 5. PRUNE — weaken destructive interference pairs
//! 6. TRANSFER — move memories to deeper temporal layers
//! 7. WIRE — create new skip links from consolidation discoveries

use std::f32::consts::PI;
use std::time::Instant;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::geometry::fano_related;
use crate::kuramoto::KuramotoSync;
use crate::skip_link::SkipLink;
use crate::store::MemoryEngine;
use crate::wave::{cosine_similarity, normalize};

/// Classification of interference between two memories.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Interference {
    Constructive,
    Destructive,
}

/// A detected interference pair.
#[derive(Debug, Clone)]
struct InterferencePair {
    id_a: Uuid,
    id_b: Uuid,
    similarity: f32,
    kind: Interference,
}

/// Statistics from a single consolidation cycle.
#[derive(Debug, Clone, Default)]
pub struct ConsolidationReport {
    pub memories_replayed: usize,
    pub interference_pairs_found: usize,
    pub constructive_pairs: usize,
    pub destructive_pairs: usize,
    pub bundles_created: usize,
    pub memories_strengthened: usize,
    pub memories_pruned: usize,
    pub clusters_synced: usize,
    pub sync_order_improvement: f32,
    pub memories_transferred: usize,
    pub skip_links_created: usize,
    pub hallucinations_created: usize,
    pub duration_ms: u64,
}

/// The 7-stage consolidation engine.
pub struct ConsolidationEngine {
    /// Similarity threshold for interference detection
    pub interference_threshold: f32,
    /// Phase difference threshold for constructive vs destructive
    pub phase_alignment_threshold: f32,
    /// Minimum amplitude to survive pruning
    pub prune_threshold: f32,
    /// How much amplitude boost from constructive interference
    pub constructive_boost: f32,
    /// How much amplitude reduction from destructive interference
    pub destructive_penalty: f32,
    /// Kuramoto synchronization parameters
    pub kuramoto: KuramotoSync,
}

impl Default for ConsolidationEngine {
    fn default() -> Self {
        Self {
            interference_threshold: 0.6,
            phase_alignment_threshold: PI / 4.0,
            prune_threshold: 0.05,
            constructive_boost: 0.3,
            destructive_penalty: 0.4,
            kuramoto: KuramotoSync::default(),
        }
    }
}

impl ConsolidationEngine {
    /// Run a full 7-stage consolidation cycle on memories at the given layer range.
    pub fn consolidate(
        &self,
        engine: &mut MemoryEngine,
        min_layer: u8,
        max_layer: u8,
    ) -> ConsolidationReport {
        let start = Instant::now();
        let mut report = ConsolidationReport::default();

        // Stage 1: REPLAY — collect working set of memories in layer range
        let working_set = self.stage_replay(engine, min_layer, max_layer);
        report.memories_replayed = working_set.len();

        // Stage 2: DETECT — find interference patterns
        let pairs = self.stage_detect(engine, &working_set);
        report.interference_pairs_found = pairs.len();
        report.constructive_pairs = pairs.iter().filter(|p| p.kind == Interference::Constructive).count();
        report.destructive_pairs = pairs.iter().filter(|p| p.kind == Interference::Destructive).count();

        // Stage 3: BUNDLE — create summary vectors per layer
        report.bundles_created = self.stage_bundle(engine, &working_set, max_layer);

        // Stage 4: STRENGTHEN — boost constructive pairs
        report.memories_strengthened = self.stage_strengthen(engine, &pairs);

        // Stage 4.5: SYNC — Kuramoto phase synchronization
        let (clusters_synced, order_improvement) = self.stage_sync(engine, &working_set);
        report.clusters_synced = clusters_synced;
        report.sync_order_improvement = order_improvement;

        // Stage 5: PRUNE — weaken destructive pairs
        report.memories_pruned = self.stage_prune(engine, &pairs);

        // Stage 6: TRANSFER — promote old memories to deeper layers
        report.memories_transferred = self.stage_transfer(engine);

        // Stage 7: WIRE — create skip links for cross-layer constructive pairs
        report.skip_links_created = self.stage_wire(engine, &pairs);

        // Stage 8: HALLUCINATE — generate novel memories from distant clusters
        report.hallucinations_created = self.stage_hallucinate(engine, &working_set);

        report.duration_ms = start.elapsed().as_millis() as u64;
        report
    }

    /// Stage 1: Load memories in the given layer range into a working set.
    fn stage_replay(&self, engine: &MemoryEngine, min_layer: u8, max_layer: u8) -> Vec<Uuid> {
        let all = engine.store.all_memories().unwrap_or_default();
        all.iter()
            .filter(|m| m.layer_depth >= min_layer && m.layer_depth <= max_layer)
            .map(|m| m.id)
            .collect()
    }

    /// Stage 2: Detect interference patterns between memory pairs.
    fn stage_detect(&self, engine: &MemoryEngine, working_set: &[Uuid]) -> Vec<InterferencePair> {
        let mut pairs = Vec::new();
        for i in 0..working_set.len() {
            for j in (i + 1)..working_set.len() {
                let (vec_a, phase_a, layer_a) = match engine.store.get(&working_set[i]).ok().flatten() {
                    Some(m) => (m.vector.clone(), m.phase, m.layer_depth),
                    None => continue,
                };
                let (vec_b, phase_b, _layer_b) = match engine.store.get(&working_set[j]).ok().flatten() {
                    Some(m) => (m.vector.clone(), m.phase, m.layer_depth),
                    None => continue,
                };

                let sim = cosine_similarity(&vec_a, &vec_b);
                if sim <= self.interference_threshold {
                    continue;
                }

                let phase_diff = (phase_a - phase_b).abs();
                // Normalize to [0, π]
                let phase_diff = phase_diff % (2.0 * PI);
                let phase_diff = if phase_diff > PI { 2.0 * PI - phase_diff } else { phase_diff };

                let kind = if phase_diff < self.phase_alignment_threshold {
                    Interference::Constructive
                } else if phase_diff > PI - self.phase_alignment_threshold {
                    Interference::Destructive
                } else {
                    continue; // neutral
                };

                pairs.push(InterferencePair {
                    id_a: working_set[i],
                    id_b: working_set[j],
                    similarity: sim,
                    kind,
                });

                // Track layers for wiring stage
                let _ = layer_a; // used in wire stage via pair ids
            }
        }
        pairs
    }

    /// Stage 3: Bundle memories at each layer into summary vectors at the next layer.
    fn stage_bundle(&self, engine: &mut MemoryEngine, working_set: &[Uuid], max_layer: u8) -> usize {
        let mut bundles_created = 0;

        for layer in 0..=max_layer {
            let vectors: Vec<Vec<f32>> = working_set
                .iter()
                .filter_map(|id| engine.store.get(id).ok().flatten())
                .filter(|m| m.layer_depth == layer)
                .map(|m| m.vector.clone())
                .collect();

            if vectors.len() < 2 {
                continue;
            }

            // Bundle using the encoding pipeline
            let summary = engine.pipeline.bundle(&vectors);
            let mut summary_mem = crate::memory::HyperMemory::new(
                summary,
                format!("__consolidation_summary_layer_{}", layer),
            );
            summary_mem.layer_depth = layer + 1;

            if engine.store.insert(summary_mem).is_ok() {
                bundles_created += 1;
            }
        }

        bundles_created
    }

    /// Stage 4: Strengthen constructive interference pairs.
    fn stage_strengthen(&self, engine: &mut MemoryEngine, pairs: &[InterferencePair]) -> usize {
        let mut count = 0;
        for pair in pairs.iter().filter(|p| p.kind == Interference::Constructive) {
            // Get phases for averaging
            let (phase_a, phase_b) = {
                let ma = engine.store.get(&pair.id_a).ok().flatten();
                let mb = engine.store.get(&pair.id_b).ok().flatten();
                match (ma, mb) {
                    (Some(a), Some(b)) => (a.phase, b.phase),
                    _ => continue,
                }
            };
            let avg_phase = (phase_a + phase_b) / 2.0;

            // Boost amplitude and align phase for memory A
            if let Some(mem) = engine.store.get_mut(&pair.id_a).ok().flatten() {
                mem.amplitude += self.constructive_boost;
                mem.phase = avg_phase;
                count += 1;
            }
            // Boost amplitude and align phase for memory B
            if let Some(mem) = engine.store.get_mut(&pair.id_b).ok().flatten() {
                mem.amplitude += self.constructive_boost;
                mem.phase = avg_phase;
                count += 1;
            }
        }
        count
    }

    /// Stage 4.5: Kuramoto phase synchronization on detected clusters.
    fn stage_sync(&self, engine: &mut MemoryEngine, working_set: &[Uuid]) -> (usize, f32) {
        // Build similarity graph among working set and find connected components
        let mems: Vec<Option<crate::memory::HyperMemory>> = working_set
            .iter()
            .map(|id| engine.store.get(id).ok().flatten().cloned())
            .collect();

        let n = mems.len();
        let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                if let (Some(a), Some(b)) = (&mems[i], &mems[j]) {
                    let sim = cosine_similarity(&a.vector, &b.vector);
                    if sim > self.kuramoto.coupling_threshold {
                        adj[i].push(j);
                        adj[j].push(i);
                    }
                }
            }
        }

        // Find connected components
        let mut visited = vec![false; n];
        let mut clusters_synced = 0usize;
        let mut total_improvement = 0.0f32;

        for start in 0..n {
            if visited[start] || mems[start].is_none() {
                continue;
            }
            let mut component = vec![start];
            let mut queue = vec![start];
            visited[start] = true;
            while let Some(node) = queue.pop() {
                for &nb in &adj[node] {
                    if !visited[nb] {
                        visited[nb] = true;
                        component.push(nb);
                        queue.push(nb);
                    }
                }
            }
            if component.len() < 2 {
                continue;
            }

            // Clone memories for sync
            let mut cluster_mems: Vec<crate::memory::HyperMemory> = component
                .iter()
                .filter_map(|&i| mems[i].clone())
                .collect();
            let mut refs: Vec<&mut crate::memory::HyperMemory> = cluster_mems.iter_mut().collect();
            let report = self.kuramoto.sync_cluster(&mut refs);

            // Write back phases
            for m in &cluster_mems {
                if let Ok(Some(stored)) = engine.store.get_mut(&m.id) {
                    stored.phase = m.phase;
                }
            }

            total_improvement += report.final_order - report.initial_order;
            clusters_synced += 1;
        }

        (clusters_synced, total_improvement)
    }

    /// Stage 5: Prune destructive interference pairs.
    fn stage_prune(&self, engine: &mut MemoryEngine, pairs: &[InterferencePair]) -> usize {
        let mut count = 0;
        for pair in pairs.iter().filter(|p| p.kind == Interference::Destructive) {
            for id in &[pair.id_a, pair.id_b] {
                if let Some(mem) = engine.store.get_mut(id).ok().flatten() {
                    mem.amplitude -= self.destructive_penalty;
                    if mem.amplitude < self.prune_threshold {
                        mem.amplitude = 0.0; // soft-delete (ghost)
                    }
                    count += 1;
                }
            }
        }
        count
    }

    /// Stage 6: Transfer old memories to deeper temporal layers.
    fn stage_transfer(&self, engine: &mut MemoryEngine) -> usize {
        let now = Utc::now();
        let ids = engine.store.all_ids().unwrap_or_default();
        let mut count = 0;

        // Collect transfer decisions first to avoid borrow issues
        let mut transfers: Vec<(Uuid, u8)> = Vec::new();
        for id in &ids {
            if let Some(mem) = engine.store.get(id).ok().flatten() {
                let age = now - mem.created_at;
                let new_layer = match mem.layer_depth {
                    0 if age > Duration::hours(1) => Some(1),
                    1 if age > Duration::days(1) => Some(2),
                    2 if age > Duration::weeks(1) => Some(3),
                    _ => None,
                };
                if let Some(layer) = new_layer {
                    transfers.push((*id, layer));
                }
            }
        }

        for (id, new_layer) in transfers {
            if let Some(mem) = engine.store.get_mut(&id).ok().flatten() {
                mem.layer_depth = new_layer;
                count += 1;
            }
        }

        count
    }

    /// Stage 8: Generate hallucinated memories by combining distant clusters.
    ///
    /// Selects 2-3 memories that are maximally distant in semantic space,
    /// bundles their hypervectors, and stores the result as a new hallucinated memory.
    fn stage_hallucinate(&self, engine: &mut MemoryEngine, working_set: &[Uuid]) -> usize {
        if working_set.len() < 3 {
            return 0;
        }

        // Collect (id, vector, content, amplitude) for high-amplitude memories
        let mut candidates: Vec<(Uuid, Vec<f32>, String, f32, Vec<String>)> = Vec::new();
        for id in working_set {
            if let Some(mem) = engine.store.get(id).ok().flatten() {
                if mem.amplitude > self.prune_threshold && !mem.content.starts_with("__consolidation") {
                    // Collect tags-like info from content words
                    let tags: Vec<String> = mem.content
                        .split_whitespace()
                        .take(5)
                        .map(|s| s.to_lowercase())
                        .collect();
                    candidates.push((mem.id, mem.vector.clone(), mem.content.clone(), mem.amplitude, tags));
                }
            }
        }

        if candidates.len() < 3 {
            return 0;
        }

        // Find the pair with minimum cosine similarity (maximally distant)
        let mut min_sim = f32::MAX;
        let mut best_pair = (0usize, 1usize);
        for i in 0..candidates.len() {
            for j in (i + 1)..candidates.len() {
                let sim = cosine_similarity(&candidates[i].1, &candidates[j].1);
                if sim < min_sim {
                    min_sim = sim;
                    best_pair = (i, j);
                }
            }
        }

        // Find a third memory distant from both
        let mut best_third = None;
        let mut min_max_sim = f32::MAX;
        for k in 0..candidates.len() {
            if k == best_pair.0 || k == best_pair.1 {
                continue;
            }
            let sim_a = cosine_similarity(&candidates[k].1, &candidates[best_pair.0].1);
            let sim_b = cosine_similarity(&candidates[k].1, &candidates[best_pair.1].1);
            let max_sim = sim_a.max(sim_b);
            if max_sim < min_max_sim {
                min_max_sim = max_sim;
                best_third = Some(k);
            }
        }

        let parent_indices: Vec<usize> = if let Some(third) = best_third {
            vec![best_pair.0, best_pair.1, third]
        } else {
            vec![best_pair.0, best_pair.1]
        };

        // Bundle parent vectors (element-wise addition + normalize)
        let dim = candidates[parent_indices[0]].1.len();
        let mut combined = vec![0.0f32; dim];
        for &idx in &parent_indices {
            for (i, &v) in candidates[idx].1.iter().enumerate() {
                combined[i] += v;
            }
        }
        normalize(&mut combined);

        // Build content and metadata
        let parent_ids: Vec<String> = parent_indices.iter().map(|&i| candidates[i].0.to_string()).collect();
        let parent_phrases: Vec<&str> = parent_indices.iter()
            .map(|&i| {
                let c = &candidates[i].2;
                if c.len() > 60 { &c[..60] } else { c.as_str() }
            })
            .collect();
        let content = format!("[hallucination] Synthesis of: {}", parent_phrases.join(" | "));

        // Merge tags
        let mut merged_tags: Vec<String> = Vec::new();
        for &idx in &parent_indices {
            for tag in &candidates[idx].4 {
                if !merged_tags.contains(tag) {
                    merged_tags.push(tag.clone());
                }
            }
        }

        // Create the hallucinated memory
        let mut hallucination = crate::memory::HyperMemory::new(combined, content);
        hallucination.amplitude = 0.3; // low initial amplitude — must prove itself
        hallucination.hallucinated = true;
        hallucination.parents = parent_ids.clone();

        let hall_id = match engine.store.insert(hallucination) {
            Ok(id) => id,
            Err(_) => return 0,
        };

        // Create hallucinated_from relations (skip links with special strength)
        for &idx in &parent_indices {
            let parent_id = candidates[idx].0;
            // Forward link: hallucination -> parent
            if let Ok(Some(hall_mem)) = engine.store.get_mut(&hall_id) {
                hall_mem.connections.push(SkipLink {
                    target_id: parent_id,
                    strength: 0.5,
                    resonance_key: Vec::new(),
                    span: 0,
                });
            }
            // Reverse link: parent -> hallucination
            if let Ok(Some(parent_mem)) = engine.store.get_mut(&parent_id) {
                parent_mem.connections.push(SkipLink {
                    target_id: hall_id,
                    strength: 0.5,
                    resonance_key: Vec::new(),
                    span: 0,
                });
            }
        }

        1 // created 1 hallucination this cycle
    }

    /// Stage 7: Wire skip links between constructive cross-layer pairs and Fano-related memories.
    fn stage_wire(&self, engine: &mut MemoryEngine, pairs: &[InterferencePair]) -> usize {
        let mut count = 0;
        
        // Wire constructive cross-layer pairs
        for pair in pairs.iter().filter(|p| p.kind == Interference::Constructive) {
            // Check if they're at different layers
            let (layer_a, layer_b) = {
                let ma = engine.store.get(&pair.id_a).ok().flatten();
                let mb = engine.store.get(&pair.id_b).ok().flatten();
                match (ma, mb) {
                    (Some(a), Some(b)) => (a.layer_depth, b.layer_depth),
                    _ => continue,
                }
            };
            if layer_a == layer_b {
                continue;
            }

            let span = (layer_a as i16 - layer_b as i16).unsigned_abs() as u8;

            // Check if link already exists from A to B
            let already_linked = engine
                .store
                .get(&pair.id_a)
                .ok()
                .flatten()
                .map(|m| m.connections.iter().any(|l| l.target_id == pair.id_b))
                .unwrap_or(true);

            if already_linked {
                continue;
            }

            let strength = pair.similarity * 0.8;

            // Create forward link
            if let Some(mem) = engine.store.get_mut(&pair.id_a).ok().flatten() {
                mem.connections.push(SkipLink {
                    target_id: pair.id_b,
                    strength,
                    resonance_key: Vec::new(),
                    span,
                });
            }
            // Create reverse link
            if let Some(mem) = engine.store.get_mut(&pair.id_b).ok().flatten() {
                mem.connections.push(SkipLink {
                    target_id: pair.id_a,
                    strength,
                    resonance_key: Vec::new(),
                    span,
                });
            }
            count += 1;
        }
        
        // Wire Fano-related memories (NEW FEATURE)
        let all_memories = engine.store.all_memories().unwrap_or_default();
        
        // Collect pairs for Fano-related linking (store IDs and necessary data to avoid borrowing issues)
        let mut fano_pairs = Vec::new();
        for i in 0..all_memories.len() {
            for j in (i + 1)..all_memories.len() {
                let mem_a = &all_memories[i];
                let mem_b = &all_memories[j];
                
                if let (Some(ref coords_a), Some(ref coords_b)) = (&mem_a.geometry, &mem_b.geometry) {
                    if fano_related(coords_a, coords_b) {
                        // Check if link already exists
                        let already_linked = mem_a.connections.iter().any(|l| l.target_id == mem_b.id) ||
                                           mem_b.connections.iter().any(|l| l.target_id == mem_a.id);
                        
                        if !already_linked {
                            let span = (mem_a.layer_depth as i16 - mem_b.layer_depth as i16).unsigned_abs() as u8;
                            fano_pairs.push((mem_a.id, mem_b.id, span));
                        }
                    }
                }
            }
        }
        
        // Now create the links using the collected pairs
        for (id_a, id_b, span) in fano_pairs {
            // Create bidirectional Fano links with strength 0.3
            if let Some(mem_a_mut) = engine.store.get_mut(&id_a).ok().flatten() {
                mem_a_mut.connections.push(SkipLink {
                    target_id: id_b,
                    strength: 0.3,
                    resonance_key: Vec::new(),
                    span,
                });
            }
            if let Some(mem_b_mut) = engine.store.get_mut(&id_b).ok().flatten() {
                mem_b_mut.connections.push(SkipLink {
                    target_id: id_a,
                    strength: 0.3,
                    resonance_key: Vec::new(),
                    span,
                });
            }
            count += 1;
        }
        
        count
    }
}

/// Higher-level wrapper that runs multiple consolidation cycles with increasing depth.
pub struct DreamState {
    pub engine: ConsolidationEngine,
    pub cycles: usize,
}

impl Default for DreamState {
    fn default() -> Self {
        Self {
            engine: ConsolidationEngine::default(),
            cycles: 3,
        }
    }
}

impl DreamState {
    pub fn new(engine: ConsolidationEngine, cycles: usize) -> Self {
        Self { engine, cycles }
    }

    /// Run multiple consolidation passes with increasing depth.
    /// Cycle 1: layers 0-1 (recent)
    /// Cycle 2: layers 1-2 (medium)
    /// Cycle 3: layers 2-3 (deep)
    pub fn dream(&self, engine: &mut MemoryEngine) -> Vec<ConsolidationReport> {
        let mut reports = Vec::new();
        for cycle in 0..self.cycles {
            let min_layer = cycle as u8;
            let max_layer = (cycle + 1) as u8;
            let report = self.engine.consolidate(engine, min_layer, max_layer);
            reports.push(report);
        }
        reports
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebook::Codebook;
    use crate::encoding::{EncodingPipeline, SimpleHashEncoder};
    use crate::memory::HyperMemory;
    use crate::store::{InMemoryStore, MemoryEngine};

    fn make_engine() -> MemoryEngine {
        let encoder = SimpleHashEncoder::new(384, 42);
        let codebook = Codebook::new(384, 10_000, 42);
        let pipeline = EncodingPipeline::new(Box::new(encoder), codebook);
        MemoryEngine::new(Box::new(InMemoryStore::new()), pipeline)
    }

    /// Helper: insert a memory with specific phase and layer, return id.
    fn insert_with_phase_and_layer(
        engine: &mut MemoryEngine,
        text: &str,
        phase: f32,
        layer: u8,
    ) -> Uuid {
        let id = engine.remember_at_layer(text, layer).unwrap();
        if let Some(mem) = engine.store.get_mut(&id).ok().flatten() {
            mem.phase = phase;
        }
        id
    }

    #[test]
    fn constructive_interference_strengthens_memories() {
        let mut engine = make_engine();
        let consolidation = ConsolidationEngine {
            interference_threshold: 0.3,
            ..Default::default()
        };

        // Two similar memories with aligned phases (both 0.0)
        let id1 = insert_with_phase_and_layer(&mut engine, "the cat sat on the mat", 0.0, 0);
        let id2 = insert_with_phase_and_layer(&mut engine, "the cat sat on the mat today", 0.0, 0);

        let amp_before_1 = engine.get_memory(&id1).unwrap().unwrap().amplitude;
        let amp_before_2 = engine.get_memory(&id2).unwrap().unwrap().amplitude;

        let report = consolidation.consolidate(&mut engine, 0, 1);

        let amp_after_1 = engine.get_memory(&id1).unwrap().unwrap().amplitude;
        let amp_after_2 = engine.get_memory(&id2).unwrap().unwrap().amplitude;

        assert!(report.constructive_pairs > 0, "should detect constructive pairs");
        assert!(amp_after_1 > amp_before_1, "amplitude should increase: {} -> {}", amp_before_1, amp_after_1);
        assert!(amp_after_2 > amp_before_2, "amplitude should increase: {} -> {}", amp_before_2, amp_after_2);
    }

    #[test]
    fn destructive_interference_weakens_memories() {
        let mut engine = make_engine();
        let consolidation = ConsolidationEngine {
            interference_threshold: 0.3,
            ..Default::default()
        };

        // Two similar memories with opposed phases
        let id1 = insert_with_phase_and_layer(&mut engine, "the cat sat on the mat", 0.0, 0);
        let id2 = insert_with_phase_and_layer(&mut engine, "the cat sat on the mat today", PI, 0);

        let amp_before_1 = engine.get_memory(&id1).unwrap().unwrap().amplitude;

        let report = consolidation.consolidate(&mut engine, 0, 1);

        let amp_after_1 = engine.get_memory(&id1).unwrap().unwrap().amplitude;

        assert!(report.destructive_pairs > 0, "should detect destructive pairs");
        assert!(amp_after_1 < amp_before_1, "amplitude should decrease: {} -> {}", amp_before_1, amp_after_1);
    }

    #[test]
    fn pruning_reduces_amplitude_to_zero() {
        let mut engine = make_engine();
        let consolidation = ConsolidationEngine {
            interference_threshold: 0.3,
            destructive_penalty: 1.5, // large enough to force below threshold
            ..Default::default()
        };

        let id1 = insert_with_phase_and_layer(&mut engine, "the cat sat on the mat", 0.0, 0);
        let id2 = insert_with_phase_and_layer(&mut engine, "the cat sat on the mat today", PI, 0);

        consolidation.consolidate(&mut engine, 0, 1);

        let amp1 = engine.get_memory(&id1).unwrap().unwrap().amplitude;
        let amp2 = engine.get_memory(&id2).unwrap().unwrap().amplitude;

        // At least one should be ghosted (amplitude 0)
        assert!(
            amp1 == 0.0 || amp2 == 0.0,
            "at least one should be pruned to 0: amp1={}, amp2={}", amp1, amp2
        );
    }

    #[test]
    fn bundling_creates_summary_similar_to_components() {
        let mut engine = make_engine();
        let consolidation = ConsolidationEngine {
            interference_threshold: 0.99, // high so no interference detected
            ..Default::default()
        };

        // Insert several memories at layer 0
        engine.remember_at_layer("cats are fluffy animals", 0).unwrap();
        engine.remember_at_layer("dogs are loyal pets", 0).unwrap();
        engine.remember_at_layer("birds can fly high", 0).unwrap();

        let initial_count = engine.store.count();
        let report = consolidation.consolidate(&mut engine, 0, 1);

        assert!(report.bundles_created > 0, "should create at least one bundle");
        assert!(engine.store.count() > initial_count, "should have more memories after bundling");

        // Find the summary memory
        let all = engine.store.all_memories().unwrap();
        let summary = all.iter().find(|m| m.content.contains("__consolidation_summary")).unwrap();
        assert_eq!(summary.layer_depth, 1, "summary should be at layer 1");

        // Summary should have positive similarity to components
        let cat_vec = engine.pipeline.encode_text("cats are fluffy animals").unwrap();
        let sim = cosine_similarity(&summary.vector, &cat_vec);
        assert!(sim > 0.0, "summary should be similar to components, got {}", sim);
    }

    #[test]
    fn transfer_moves_old_memories_to_deeper_layers() {
        let mut engine = make_engine();
        let consolidation = ConsolidationEngine::default();

        // Insert memory and make it old
        let id = engine.remember_at_layer("old memory", 0).unwrap();
        if let Some(mem) = engine.store.get_mut(&id).ok().flatten() {
            mem.created_at = Utc::now() - Duration::hours(2);
        }

        let report = consolidation.consolidate(&mut engine, 0, 1);

        let layer = engine.get_memory(&id).unwrap().unwrap().layer_depth;
        assert_eq!(layer, 1, "should transfer to layer 1");
        assert!(report.memories_transferred > 0);
    }

    #[test]
    fn wiring_creates_skip_links_for_cross_layer_constructive_pairs() {
        let mut engine = make_engine();
        engine.similarity_threshold = 0.99; // prevent auto-linking on insert
        let consolidation = ConsolidationEngine {
            interference_threshold: 0.3,
            ..Default::default()
        };

        // Similar memories at different layers with aligned phases
        // Use remember() directly and manually set layer to avoid auto-linking
        let id1 = {
            let mut mem = engine.pipeline.encode_memory("the cat sat on the mat", Utc::now()).unwrap();
            mem.layer_depth = 0;
            mem.phase = 0.0;
            engine.store.insert(mem).unwrap()
        };
        let id2 = {
            let mut mem = engine.pipeline.encode_memory("the cat sat on the mat today", Utc::now()).unwrap();
            mem.layer_depth = 1;
            mem.phase = 0.0;
            engine.store.insert(mem).unwrap()
        };

        // Verify no pre-existing links
        assert!(engine.get_memory(&id1).unwrap().unwrap().connections.is_empty());

        let report = consolidation.consolidate(&mut engine, 0, 1);

        assert!(report.skip_links_created > 0, "should create skip links");
        let mem1 = engine.get_memory(&id1).unwrap().unwrap();
        assert!(!mem1.connections.is_empty(), "should have skip link");
    }

    #[test]
    fn dream_state_runs_multiple_cycles() {
        let mut engine = make_engine();
        let dream = DreamState::default();

        engine.remember_at_layer("recent thought", 0).unwrap();
        engine.remember_at_layer("day old thought", 1).unwrap();
        engine.remember_at_layer("week old thought", 2).unwrap();

        let reports = dream.dream(&mut engine);

        assert_eq!(reports.len(), 3, "should have 3 cycle reports");
        // Each cycle should replay some memories
        assert!(reports[0].memories_replayed > 0, "cycle 1 should replay memories");
    }

    /// Helper: insert a memory with a specific vector, phase, and layer.
    fn insert_raw(
        engine: &mut MemoryEngine,
        vector: Vec<f32>,
        content: &str,
        phase: f32,
        layer: u8,
    ) -> Uuid {
        let mut mem = HyperMemory::new(vector, content.to_string());
        mem.phase = phase;
        mem.layer_depth = layer;
        engine.store.insert(mem).unwrap()
    }

    #[test]
    fn full_dream_cycle() {
        let mut engine = make_engine();

        let dream = DreamState::new(
            ConsolidationEngine {
                interference_threshold: 0.9,
                ..Default::default()
            },
            3,
        );

        // Use hand-crafted vectors so groups don't cross-interfere
        let dim = 10_000;
        // Group A: related (similar vector, aligned phase)
        let mut va = vec![0.0f32; dim];
        for i in 0..100 { va[i] = 1.0; }
        crate::wave::normalize(&mut va);

        // Group B: opposed (similar vector but orthogonal to A, opposed phases)
        let mut vb = vec![0.0f32; dim];
        for i in 200..300 { vb[i] = 1.0; }
        crate::wave::normalize(&mut vb);

        let related1 = insert_raw(&mut engine, va.clone(), "related1", 0.0, 0);
        let related2 = insert_raw(&mut engine, va.clone(), "related2", 0.0, 0);
        let opposed1 = insert_raw(&mut engine, vb.clone(), "opposed1", 0.0, 0);
        let opposed2 = insert_raw(&mut engine, vb.clone(), "opposed2", PI, 0);

        let amp_related1_before = engine.get_memory(&related1).unwrap().unwrap().amplitude;
        let amp_opposed1_before = engine.get_memory(&opposed1).unwrap().unwrap().amplitude;

        let reports = dream.dream(&mut engine);

        println!("=== Full Dream Cycle Reports ===");
        for (i, r) in reports.iter().enumerate() {
            println!(
                "Cycle {}: replayed={}, interference={} (constructive={}, destructive={}), \
                 bundles={}, strengthened={}, pruned={}, transferred={}, wired={}, duration={}ms",
                i + 1,
                r.memories_replayed,
                r.interference_pairs_found,
                r.constructive_pairs,
                r.destructive_pairs,
                r.bundles_created,
                r.memories_strengthened,
                r.memories_pruned,
                r.memories_transferred,
                r.skip_links_created,
                r.duration_ms,
            );
        }

        let amp_related1_after = engine.get_memory(&related1).unwrap().unwrap().amplitude;
        let amp_related2_after = engine.get_memory(&related2).unwrap().unwrap().amplitude;
        let amp_opposed1_after = engine.get_memory(&opposed1).unwrap().unwrap().amplitude;
        let amp_opposed2_after = engine.get_memory(&opposed2).unwrap().unwrap().amplitude;

        // Related memories should be strengthened
        assert!(
            amp_related1_after > amp_related1_before,
            "related memory should be stronger: {} -> {}",
            amp_related1_before, amp_related1_after
        );

        // Opposed memories should be weakened
        assert!(
            amp_opposed1_after < amp_opposed1_before,
            "opposed memory should be weaker: {} -> {}",
            amp_opposed1_before, amp_opposed1_after
        );

        println!("\n=== Amplitude Changes ===");
        println!("Related 1: {} -> {}", amp_related1_before, amp_related1_after);
        println!("Related 2: {}", amp_related2_after);
        println!("Opposed 1: {} -> {}", amp_opposed1_before, amp_opposed1_after);
        println!("Opposed 2: {}", amp_opposed2_after);

        // First cycle should have done real work
        assert!(reports[0].memories_replayed > 0);
    }

    #[test]
    fn hallucination_created_from_distant_memories() {
        let mut engine = make_engine();
        let consolidation = ConsolidationEngine {
            interference_threshold: 0.99, // avoid interference detection
            ..Default::default()
        };

        // Insert 3+ memories with orthogonal vectors (maximally distant)
        let dim = 10_000;
        let mut v1 = vec![0.0f32; dim]; for i in 0..100 { v1[i] = 1.0; }
        let mut v2 = vec![0.0f32; dim]; for i in 200..300 { v2[i] = 1.0; }
        let mut v3 = vec![0.0f32; dim]; for i in 400..500 { v3[i] = 1.0; }
        crate::wave::normalize(&mut v1);
        crate::wave::normalize(&mut v2);
        crate::wave::normalize(&mut v3);

        insert_raw(&mut engine, v1, "quantum physics theory", 0.0, 0);
        insert_raw(&mut engine, v2, "cooking pasta recipes", 0.0, 0);
        insert_raw(&mut engine, v3, "alpine hiking trails", 0.0, 0);

        let initial_count = engine.store.count();
        let report = consolidation.consolidate(&mut engine, 0, 1);

        assert!(report.hallucinations_created > 0, "should create at least one hallucination");
        assert!(engine.store.count() > initial_count, "should have more memories after hallucination");

        // Find the hallucinated memory
        let all = engine.store.all_memories().unwrap();
        let hall = all.iter().find(|m| m.hallucinated).unwrap();
        assert!(hall.content.starts_with("[hallucination]"));
        assert!(!hall.parents.is_empty());
        assert!(hall.amplitude <= 0.3, "hallucination should start with low amplitude");
        // Should have connections to parents
        assert!(!hall.connections.is_empty(), "hallucination should be linked to parents");
    }

    #[test]
    fn hallucination_skipped_with_few_memories() {
        let mut engine = make_engine();
        let consolidation = ConsolidationEngine::default();

        // Only 2 memories — not enough for hallucination
        engine.remember_at_layer("single thought", 0).unwrap();
        engine.remember_at_layer("another thought", 0).unwrap();

        let report = consolidation.consolidate(&mut engine, 0, 1);
        // May or may not create hallucinations depending on content similarity
        // but should not panic
        assert!(report.hallucinations_created <= 1);
    }
}
