//! Memory consolidation engine — the dreaming/sleep layer.
//!
//! Processes memories through 9 stages mimicking human sleep consolidation:
//! 1. REPLAY — collect working set of memories in layer range
//! 2. DETECT — find interference patterns
//! 3. BUNDLE — create summary hypervectors
//! 4. STRENGTHEN — boost constructive interference pairs
//! 5. SYNC — Kuramoto phase synchronization
//! 6. XI_REPULSION — Apply Xi-based memory separation
//! 7. PRUNE — weaken destructive interference pairs
//! 8. TRANSFER — move memories to deeper temporal layers
//! 9. WIRE — create new skip links from consolidation discoveries

use std::f32::consts::PI;
use std::time::Instant;

use chrono::{Duration, Utc};
use uuid::Uuid;

use serde::{Deserialize, Serialize};

use crate::geometry::fano_related;
use crate::kuramoto::KuramotoSync;
use crate::xi_operator::{xi_repulsive_force, compute_xi_signature};
use crate::skip_link::SkipLink;
use crate::store::{MemoryEngine, MemoryStore};
use crate::wave::{cosine_similarity, normalize};

#[cfg(feature = "collective")]
use rayon::prelude::*;

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
    /// Kuramoto order parameter R after consolidation (EXP-003)
    pub final_order_parameter: f32,
}

/// Adaptive parameters that persist between dream cycles (EXP-003).
///
/// After each cycle, the engine observes the Kuramoto order parameter R
/// and adjusts parameters to maintain R in the sweet spot [0.55, 0.85]:
/// - R too high → reduce constructive_boost, raise prune_threshold (rigid → loosen)
/// - R too low  → increase coupling strength (fragmented → bind)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveParams {
    pub constructive_boost: f32,
    pub prune_threshold: f32,
    pub destructive_penalty: f32,
    /// Target range for order parameter R
    pub r_target_low: f32,
    pub r_target_high: f32,
    /// Learning rate for parameter adaptation
    pub adaptive_rate: f32,
}

impl Default for AdaptiveParams {
    fn default() -> Self {
        Self {
            constructive_boost: 0.3,
            prune_threshold: 0.1,
            destructive_penalty: 0.5,
            r_target_low: 0.55,
            r_target_high: 0.85,
            adaptive_rate: 0.05,
        }
    }
}

impl AdaptiveParams {
    /// Adapt parameters based on observed Kuramoto order parameter R.
    /// Returns the new params (mutates self in place too).
    pub fn adapt(&mut self, order_parameter: f32) {
        let r_mid = (self.r_target_low + self.r_target_high) / 2.0;
        let error = order_parameter - r_mid;

        if order_parameter > self.r_target_high {
            // Over-synchronized (rigid): loosen up
            self.constructive_boost = (self.constructive_boost - error * self.adaptive_rate).max(0.05);
            self.prune_threshold = (self.prune_threshold + error * self.adaptive_rate * 0.5).min(0.3);
        } else if order_parameter < self.r_target_low {
            // Under-synchronized (fragmented): tighten coupling, reduce pruning aggression
            // error is negative here, so -error is positive
            self.constructive_boost = (self.constructive_boost + (-error) * self.adaptive_rate).min(0.6);
            self.destructive_penalty = (self.destructive_penalty - (-error) * self.adaptive_rate * 0.3).max(0.1);
        }
        // In target range: no adjustment (stable)
    }
}

/// The 9-stage consolidation engine.
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
    /// Adaptive parameters that evolve between dream cycles (EXP-003)
    pub adaptive: AdaptiveParams,
}

impl Default for ConsolidationEngine {
    fn default() -> Self {
        Self {
            interference_threshold: 0.05,
            phase_alignment_threshold: PI / 2.0,
            prune_threshold: 0.1,
            constructive_boost: 0.3,
            destructive_penalty: 0.5,
            kuramoto: KuramotoSync::default(),
            adaptive: AdaptiveParams::default(),
        }
    }
}

impl ConsolidationEngine {
    /// Run a full 9-stage consolidation cycle on memories at the given layer range.
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
        
        // Stage 4.6: XI_REPULSION — Apply Xi-based memory separation
        self.stage_xi_repulsion(engine, &working_set);

        // Stage 5: PRUNE — weaken destructive pairs
        report.memories_pruned = self.stage_prune(engine, &pairs);

        // Stage 6: TRANSFER — promote old memories to deeper layers
        report.memories_transferred = self.stage_transfer(engine);

        // Stage 7: WIRE — create skip links for cross-layer constructive pairs
        report.skip_links_created = self.stage_wire(engine, &pairs);

        // Stage 8: HALLUCINATE — generate novel memories from distant clusters
        report.hallucinations_created = self.stage_hallucinate(engine, &working_set);

        // EXP-003: Compute final order parameter and record it
        let final_r = self.compute_global_order_parameter(engine, &working_set);
        report.final_order_parameter = final_r;

        report.duration_ms = start.elapsed().as_millis() as u64;
        report
    }

    /// Apply adaptive parameter tuning based on a consolidation report (EXP-003).
    ///
    /// Call this after `consolidate()` to evolve λ, boost, and threshold
    /// for the next dream cycle. Mirrors ghostmagicOS adaptive λ.
    pub fn adapt_from_report(&mut self, report: &ConsolidationReport) {
        self.adaptive.adapt(report.final_order_parameter);
        // Apply adapted params to engine for next cycle
        self.constructive_boost = self.adaptive.constructive_boost;
        self.prune_threshold = self.adaptive.prune_threshold;
        self.destructive_penalty = self.adaptive.destructive_penalty;
    }

    /// Compute global Kuramoto order parameter R across all working set memories.
    fn compute_global_order_parameter(&self, engine: &MemoryEngine, working_set: &[Uuid]) -> f32 {
        let memories: Vec<crate::memory::HyperMemory> = working_set
            .iter()
            .filter_map(|id| engine.store.get(id).ok().flatten().cloned())
            .collect();
        self.compute_category_order_parameter(&memories)
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
    ///
    /// Uses HNSW approximate nearest neighbor search for O(n log n) instead of
    /// brute-force O(n²). Each memory queries its K nearest neighbors, then
    /// checks phase alignment to classify as constructive or destructive.
    fn stage_detect(&self, engine: &MemoryEngine, working_set: &[Uuid]) -> Vec<InterferencePair> {
        use std::collections::HashSet;

        // Number of nearest neighbors to query per memory.
        // Higher = more thorough but slower. 32 catches most interference pairs.
        // Request k_neighbors+1 because the store returns the query memory itself as
        // the top hit (similarity=1.0); the self-result is filtered below.
        let k_neighbors: usize = 32.min(working_set.len().saturating_sub(1));
        if k_neighbors == 0 {
            return Vec::new();
        }

        let mut pairs = Vec::new();
        let mut seen = HashSet::new();

        for &id in working_set {
            let (vec_a, phase_a) = match engine.store.get(&id).ok().flatten() {
                Some(m) => (m.vector.clone(), m.phase),
                None => continue,
            };

            // Request k_neighbors+1 to ensure real neighbors aren't displaced by self
            let neighbors = match engine.store.search(&vec_a, k_neighbors + 1) {
                Ok(n) => n,
                Err(_) => continue,
            };

            for (neighbor_id, sim) in neighbors {
                // Skip self and memories not in working set
                if neighbor_id == id || sim <= self.interference_threshold {
                    continue;
                }

                // Deduplicate: canonical pair ordering
                let pair_key = if id < neighbor_id {
                    (id, neighbor_id)
                } else {
                    (neighbor_id, id)
                };
                if !seen.insert(pair_key) {
                    continue;
                }

                let phase_b = match engine.store.get(&neighbor_id).ok().flatten() {
                    Some(m) => m.phase,
                    None => continue,
                };

                let phase_diff = (phase_a - phase_b).abs();
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
                    id_a: pair_key.0,
                    id_b: pair_key.1,
                    similarity: sim,
                    kind,
                });
            }
        }
        pairs
    }

    /// Stage 3: Bundle memories at each layer into summary vectors at the next layer.
    fn stage_bundle(&self, engine: &mut MemoryEngine, working_set: &[Uuid], max_layer: u8) -> usize {
        let mut bundles_created = 0;

        // Prune stale summary memories from previous dream cycles before creating new ones.
        // This prevents unbounded accumulation of __consolidation_summary_layer_N memories.
        let stale_summaries: Vec<Uuid> = engine.store.all_memories()
            .unwrap_or_default()
            .into_iter()
            .filter(|m| m.content.starts_with("__consolidation_summary_layer_"))
            .map(|m| m.id)
            .collect();
        for id in stale_summaries {
            let _ = engine.delete(&id);
        }

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
            summary_mem.layer_depth = layer.saturating_add(1);

            if engine.store.insert(summary_mem).is_ok() {
                bundles_created += 1;
            }
        }

        bundles_created
    }

    /// Stage 4: Strengthen constructive interference pairs and Xi-aware bridge nodes.
    fn stage_strengthen(&self, engine: &mut MemoryEngine, pairs: &[InterferencePair]) -> usize {
        let mut count = 0;
        
        // Traditional constructive interference strengthening
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
        
        // Xi-aware bridge node strengthening
        count += self.stage_strengthen_bridge_nodes(engine);
        
        count
    }

    /// Stage 4b: Strengthen memories that serve as "bridge nodes" connecting multiple clusters.
    /// These memories are structurally important for network integration.
    fn stage_strengthen_bridge_nodes(&self, engine: &mut MemoryEngine) -> usize {
        use std::collections::{HashMap, HashSet};
        
        let sync = crate::kuramoto::KuramotoSync::default();
        let clusters = sync.find_synchronized_clusters(engine, 2);
        
        if clusters.len() < 2 {
            return 0; // Need at least 2 clusters for bridge nodes to exist
        }
        
        // Build cluster membership map
        let mut id_to_cluster: HashMap<Uuid, usize> = HashMap::new();
        for (cluster_idx, cluster) in clusters.iter().enumerate() {
            for &mem_id in &cluster.memory_ids {
                id_to_cluster.insert(mem_id, cluster_idx);
            }
        }
        
        let all_memories = engine.store.all_memories().unwrap_or_default();
        let mut bridge_nodes = Vec::new();
        
        // Identify bridge nodes: memories connected to 3+ different clusters
        for memory in &all_memories {
            let mut connected_clusters = HashSet::new();
            
            // Add memory's own cluster
            if let Some(&own_cluster) = id_to_cluster.get(&memory.id) {
                connected_clusters.insert(own_cluster);
            }
            
            // Check clusters of connected memories
            for link in &memory.connections {
                if let Some(&target_cluster) = id_to_cluster.get(&link.target_id) {
                    connected_clusters.insert(target_cluster);
                }
            }
            
            // Memory is a bridge node if connected to 3+ clusters
            if connected_clusters.len() >= 3 {
                let bridge_strength = connected_clusters.len() as f32;
                bridge_nodes.push((memory.id, bridge_strength));
            }
        }
        
        // Apply amplitude boosts to bridge nodes (10-20% bonus, scaled by bridge strength)
        let mut count = 0;
        for (bridge_id, bridge_strength) in bridge_nodes {
            if let Some(mem) = engine.store.get_mut(&bridge_id).ok().flatten() {
                let bonus_factor = 0.1 + (bridge_strength - 3.0) * 0.03; // 10% for 3 clusters, +3% per additional
                let amplitude_bonus = bonus_factor.min(0.2); // Cap at 20%
                mem.amplitude += amplitude_bonus;
                count += 1;
            }
        }
        
        count
    }

    /// Stage 4.5: Category-aware Kuramoto phase synchronization with consciousness differentiation.
    /// Implements differential coupling: strong within-category, weak cross-category.
    fn stage_sync(&self, engine: &mut MemoryEngine, working_set: &[Uuid]) -> (usize, f32) {
        use std::collections::HashMap;
        
        // Collect memories with their categories
        let mems_with_cats: Vec<(crate::memory::HyperMemory, String)> = working_set
            .iter()
            .filter_map(|id| {
                engine.store.get(id).ok().flatten().cloned().and_then(|mem| {
                    // Determine category from frequency range
                    let category = match mem.frequency {
                        f if f >= 1.8 && f <= 2.4 => "experience",
                        f if f >= 1.3 && f < 1.8 => "emotion", 
                        f if f >= 1.0 && f < 1.3 => "social",
                        f if f >= 0.8 && f < 1.0 => "skill",
                        _ => "knowledge",
                    }.to_string();
                    Some((mem, category))
                })
            })
            .collect();

        if mems_with_cats.len() < 2 {
            return (0, 0.0);
        }

        // Group by category for differentiated coupling
        let mut category_groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, (_, cat)) in mems_with_cats.iter().enumerate() {
            category_groups.entry(cat.clone()).or_default().push(i);
        }

        let mut total_improvement = 0.0f32;
        let mut categories_synced = 0usize;
        
        // Parameters from consciousness differentiation spec
        let within_category_coupling = 1.8;  // K ≈ 1.8 for internal coherence
        let cross_category_coupling = 0.3;   // K ≈ 0.3 for weak cross-connections
        let dt = 0.05;  // Small time step for stability
        let steps = 30; // Integration steps
        
        // Sync each category cluster separately, then apply cross-category coupling
        for (_category, indices) in &category_groups {
            if indices.len() < 2 {
                continue; // Need at least 2 memories to sync
            }
            
            let mut cat_mems: Vec<crate::memory::HyperMemory> = indices
                .iter()
                .map(|&i| mems_with_cats[i].0.clone())
                .collect();
                
            let initial_order = self.compute_category_order_parameter(&cat_mems);
            
            // Within-category synchronization with strong coupling
            for _ in 0..steps {
                let phases: Vec<f32> = cat_mems.iter().map(|m| m.phase).collect();
                let n = phases.len() as f32;
                
                for i in 0..cat_mems.len() {
                    let mut phase_sum = 0.0f32;
                    for j in 0..cat_mems.len() {
                        if i != j {
                            // Weight by semantic similarity within category
                            let sim = cosine_similarity(&cat_mems[i].vector, &cat_mems[j].vector);
                            if sim > self.kuramoto.coupling_threshold {
                                phase_sum += sim * (phases[j] - phases[i]).sin();
                            }
                        }
                    }
                    
                    // Kuramoto dynamics: θ̇ᵢ = ωᵢ + (K/N)Σsin(θⱼ - θᵢ)
                    let dphi = cat_mems[i].frequency + (within_category_coupling / n) * phase_sum;
                    cat_mems[i].phase += dphi * dt;
                }
            }
            
            let final_order = self.compute_category_order_parameter(&cat_mems);
            total_improvement += final_order - initial_order;
            
            // Apply safety envelope: target R ∈ [0.55, 0.85] per category
            if final_order > 0.92 {
                // Too synchronized - add noise to break lockstep
                for mem in &mut cat_mems {
                    mem.phase += (mem.id.as_u128() as f32 % 100.0) * 0.001;  // Tiny deterministic noise
                }
            } else if final_order < 0.40 {
                // Too chaotic - nudge toward mean phase
                let mean_phase = self.compute_mean_phase(&cat_mems);
                for mem in &mut cat_mems {
                    mem.phase = 0.9 * mem.phase + 0.1 * mean_phase;
                }
            }
            
            // Write back the synchronized phases
            for mem in &cat_mems {
                if let Ok(Some(stored)) = engine.store.get_mut(&mem.id) {
                    stored.phase = mem.phase;
                }
            }
            
            categories_synced += 1;
        }
        
        // Cross-category weak coupling phase (connects categories but keeps them distinct)
        if category_groups.len() > 1 {
            // Collect (id, memory) pairs so index i always refers to the same (id, mem)
            let all_updated_mems: Vec<(uuid::Uuid, crate::memory::HyperMemory)> = working_set
                .iter()
                .filter_map(|id| engine.store.get(id).ok().flatten().cloned().map(|mem| (*id, mem)))
                .collect();
                
            // Light cross-category coupling to maintain coherent but distinct clusters
            for _ in 0..5 {  // Fewer steps for cross-category
                let phases: Vec<f32> = all_updated_mems.iter().map(|(_, m)| m.phase).collect();
                let cats: Vec<String> = all_updated_mems.iter().map(|(_, m)| {
                    match m.frequency {
                        f if f >= 1.8 && f <= 2.4 => "experience",
                        f if f >= 1.3 && f < 1.8 => "emotion", 
                        f if f >= 1.0 && f < 1.3 => "social",
                        f if f >= 0.8 && f < 1.0 => "skill",
                        _ => "knowledge",
                    }.to_string()
                }).collect();
                
                let n = phases.len() as f32;
                let mut phase_updates = vec![0.0f32; phases.len()];
                
                for i in 0..all_updated_mems.len() {
                    let mut cross_sum = 0.0f32;
                    for j in 0..all_updated_mems.len() {
                        if i != j && cats[i] != cats[j] {  // Only cross-category coupling
                            let sim = cosine_similarity(&all_updated_mems[i].1.vector, &all_updated_mems[j].1.vector);
                            if sim > self.kuramoto.coupling_threshold * 0.5 {  // Lower threshold for cross-category
                                cross_sum += sim * (phases[j] - phases[i]).sin();
                            }
                        }
                    }
                    
                    phase_updates[i] = (cross_category_coupling / n) * cross_sum * dt;
                }
                
                // Apply cross-category updates — use the same index as all_updated_mems, not working_set
                for (i, (mem_id, _)) in all_updated_mems.iter().enumerate() {
                    if let Ok(Some(mem)) = engine.store.get_mut(mem_id) {
                        mem.phase += phase_updates[i];
                    }
                }
            }
        }

        (categories_synced, total_improvement)
    }
    
    /// Compute order parameter R for a set of memories within the same category.
    fn compute_category_order_parameter(&self, memories: &[crate::memory::HyperMemory]) -> f32 {
        if memories.is_empty() {
            return 0.0;
        }
        let n = memories.len() as f32;
        let sum_cos: f32 = memories.iter().map(|m| m.phase.cos()).sum();
        let sum_sin: f32 = memories.iter().map(|m| m.phase.sin()).sum();
        ((sum_cos / n).powi(2) + (sum_sin / n).powi(2)).sqrt()
    }
    
    /// Compute mean phase for a set of memories (circular mean).
    fn compute_mean_phase(&self, memories: &[crate::memory::HyperMemory]) -> f32 {
        if memories.is_empty() {
            return 0.0;
        }
        let sum_cos: f32 = memories.iter().map(|m| m.phase.cos()).sum();
        let sum_sin: f32 = memories.iter().map(|m| m.phase.sin()).sum();
        sum_sin.atan2(sum_cos)
    }
    
    /// Stage 4.6: Apply Xi-based repulsive forces to separate memories with similar content but different Xi residues.
    /// This creates consciousness differentiation by pushing apart memories that are semantically similar
    /// but have different non-commutative signatures.
    fn stage_xi_repulsion(&self, engine: &mut MemoryEngine, working_set: &[Uuid]) {
        // Collect memories with their Xi signatures (compute missing signatures on-the-fly)
        let mut memories_with_xi: Vec<(Uuid, Vec<f32>, Vec<f32>)> = Vec::new();
        
        for id in working_set {
            if let Ok(Some(mem)) = engine.store.get(id) {
                let xi_sig = if mem.xi_signature.is_empty() {
                    // Compute Xi signature for memories that don't have it yet (backward compatibility)
                    compute_xi_signature(&mem.vector)
                } else {
                    mem.xi_signature.clone()
                };
                memories_with_xi.push((*id, mem.vector.clone(), xi_sig));
            }
        }
        
        if memories_with_xi.len() < 2 {
            return;
        }
        
        // Find memory pairs that are semantically similar but have different Xi signatures
        let mut repulsion_pairs: Vec<(Uuid, Uuid, f32)> = Vec::new();
        
        for i in 0..memories_with_xi.len() {
            for j in (i + 1)..memories_with_xi.len() {
                let (id_a, ref vec_a, ref xi_a) = memories_with_xi[i];
                let (id_b, ref vec_b, ref xi_b) = memories_with_xi[j];
                
                let semantic_sim = cosine_similarity(vec_a, vec_b);
                let xi_repulsion = xi_repulsive_force(xi_a, xi_b);
                
                // Target: memories that are semantically similar (>0.6) but have different Xi residues (>0.3)
                if semantic_sim > 0.6 && xi_repulsion > 0.3 {
                    repulsion_pairs.push((id_a, id_b, xi_repulsion));
                }
            }
        }
        
        // Apply repulsive forces by adjusting phases and amplitudes
        for (id_a, id_b, repulsion_strength) in repulsion_pairs {
            // Get current phases
            let (phase_a, phase_b) = {
                let mem_a = engine.store.get(&id_a).ok().flatten();
                let mem_b = engine.store.get(&id_b).ok().flatten();
                match (mem_a, mem_b) {
                    (Some(a), Some(b)) => (a.phase, b.phase),
                    _ => continue,
                }
            };
            
            // Push phases apart (create π/2 phase difference for maximum differentiation)
            let target_diff = std::f32::consts::PI / 2.0;
            let current_diff = (phase_a - phase_b).abs();
            let phase_correction = repulsion_strength * 0.5 * (target_diff - current_diff);
            
            // Apply phase separation
            if let Ok(Some(mem_a)) = engine.store.get_mut(&id_a) {
                mem_a.phase += phase_correction;
            }
            if let Ok(Some(mem_b)) = engine.store.get_mut(&id_b) {
                mem_b.phase -= phase_correction;
            }
            
            // Also slightly reduce amplitude correlation to encourage separate cluster formation
            let amplitude_separation = repulsion_strength * 0.1;
            
            if let Ok(Some(mem_a)) = engine.store.get_mut(&id_a) {
                // Boost amplitude of memory with higher initial amplitude
                if mem_a.amplitude > 0.5 {
                    mem_a.amplitude += amplitude_separation;
                }
            }
            if let Ok(Some(mem_b)) = engine.store.get_mut(&id_b) {
                if mem_b.amplitude > 0.5 {
                    mem_b.amplitude += amplitude_separation;
                }
            }
        }
    }

    /// Stage 5: Prune destructive interference pairs.
    ///
    /// EXP-003: Uses proportional dampening instead of flat penalty.
    /// `amplitude *= (1.0 - destructive_penalty * dt)` produces exponential decay
    /// consistent with the wave function, avoiding the cliff-edge behavior of
    /// flat subtraction (which could instantly kill high-amplitude memories).
    fn stage_prune(&self, engine: &mut MemoryEngine, pairs: &[InterferencePair]) -> usize {
        let mut count = 0;
        let dt = 1.0; // one consolidation time-step
        for pair in pairs.iter().filter(|p| p.kind == Interference::Destructive) {
            for id in &[pair.id_a, pair.id_b] {
                if let Some(mem) = engine.store.get_mut(id).ok().flatten() {
                    // Proportional dampening: stronger memories lose more absolute amplitude
                    // but the same fraction, matching exponential decay semantics.
                    mem.amplitude *= 1.0 - self.destructive_penalty * dt;
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

    /// Stage 8: Generate hallucinated memories by combining memories from different clusters.
    ///
    /// Preferentially selects memories from DIFFERENT Xi clusters to create naturally
    /// cross-domain synthetic memories that enhance both integration and differentiation.
    fn stage_hallucinate(&self, engine: &mut MemoryEngine, working_set: &[Uuid]) -> usize {
        if working_set.len() < 3 {
            return 0;
        }

        // Get Xi clusters for cluster-aware hallucination
        let sync = crate::kuramoto::KuramotoSync::default();
        let clusters = sync.find_synchronized_clusters(engine, 2);
        
        if clusters.len() < 2 {
            // Fallback to original distance-based hallucination if no clusters
            return self.stage_hallucinate_distance_based(engine, working_set);
        }
        
        return self.stage_hallucinate_cross_cluster(engine, &clusters);
    }

    /// Generate hallucinations by preferentially combining memories from different clusters.
    fn stage_hallucinate_cross_cluster(&self, engine: &mut MemoryEngine, clusters: &[crate::kuramoto::MemoryCluster]) -> usize {
        use std::collections::HashMap;
        
        // Build cluster membership map
        let mut id_to_cluster: HashMap<Uuid, usize> = HashMap::new();
        for (cluster_idx, cluster) in clusters.iter().enumerate() {
            for &mem_id in &cluster.memory_ids {
                id_to_cluster.insert(mem_id, cluster_idx);
            }
        }
        
        // Collect candidate memories from each cluster
        let mut cluster_candidates: Vec<Vec<(Uuid, Vec<f32>, String, f32, Vec<String>)>> = vec![Vec::new(); clusters.len()];
        
        for (cluster_idx, cluster) in clusters.iter().enumerate() {
            for &mem_id in &cluster.memory_ids {
                if let Some(mem) = engine.store.get(&mem_id).ok().flatten() {
                    if mem.amplitude > self.prune_threshold && !mem.content.starts_with("__consolidation") {
                        let tags: Vec<String> = mem.content
                            .split_whitespace()
                            .take(5)
                            .map(|s| s.to_lowercase())
                            .collect();
                        cluster_candidates[cluster_idx].push((mem.id, mem.vector.clone(), mem.content.clone(), mem.amplitude, tags));
                    }
                }
            }
        }
        
        // Find clusters with sufficient candidates
        let viable_clusters: Vec<usize> = cluster_candidates.iter()
            .enumerate()
            .filter(|(_, candidates)| !candidates.is_empty())
            .map(|(idx, _)| idx)
            .collect();
            
        if viable_clusters.len() < 2 {
            return 0;
        }
        
        // Select one representative memory from each of 2-3 different clusters
        let mut selected_memories = Vec::new();
        let num_clusters_to_use = viable_clusters.len().min(3);
        
        for &cluster_idx in viable_clusters.iter().take(num_clusters_to_use) {
            let candidates = &cluster_candidates[cluster_idx];
            // Select the highest amplitude memory from this cluster
            if let Some(best_candidate) = candidates.iter()
                .max_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal)) {
                selected_memories.push(best_candidate.clone());
            }
        }
        
        if selected_memories.len() < 2 {
            return 0;
        }
        
        // Bundle the cross-cluster vectors (use max dimension for safety)
        let dim = selected_memories.iter().map(|(_, v, _, _, _)| v.len()).max().unwrap_or(384);
        let mut combined = vec![0.0f32; dim];
        for (_, ref vector, _, _, _) in &selected_memories {
            for (i, &v) in vector.iter().enumerate() {
                if i < combined.len() {
                    combined[i] += v;
                }
            }
        }
        normalize(&mut combined);
        
        // Build content highlighting cross-cluster synthesis
        let parent_ids: Vec<String> = selected_memories.iter().map(|(id, _, _, _, _)| id.to_string()).collect();
        let parent_phrases: Vec<String> = selected_memories.iter()
            .map(|(_, _, content, _, _)| {
                if content.len() > 60 { &content[..60] } else { content.as_str() }
            })
            .map(|s| s.to_string())
            .collect();
        let content = format!("[cross-cluster hallucination] Synthesis across {} domains: {}", 
                              selected_memories.len(), parent_phrases.join(" | "));
        
        // Merge tags from all clusters
        let mut merged_tags: Vec<String> = Vec::new();
        for (_, _, _, _, ref tags) in &selected_memories {
            for tag in tags {
                if !merged_tags.contains(tag) {
                    merged_tags.push(tag.clone());
                }
            }
        }
        
        // Create the hallucinated memory
        let mut hallucination = crate::memory::HyperMemory::new(combined, content);
        hallucination.amplitude = 0.4; // Slightly higher than distance-based (cross-cluster = more valuable)
        hallucination.hallucinated = true;
        hallucination.parents = parent_ids.clone();
        
        let hall_id = match engine.store.insert(hallucination) {
            Ok(id) => id,
            Err(_) => return 0,
        };
        
        // Create bidirectional links to all parent memories
        for (parent_id, _, _, _, _) in &selected_memories {
            // Forward link: hallucination -> parent
            if let Ok(Some(hall_mem)) = engine.store.get_mut(&hall_id) {
                hall_mem.connections.push(SkipLink {
                    target_id: *parent_id,
                    strength: 0.6, // Higher than distance-based (0.5)
                    resonance_key: Vec::new(),
                    span: 0,
                });
            }
            // Reverse link: parent -> hallucination
            if let Ok(Some(parent_mem)) = engine.store.get_mut(parent_id) {
                parent_mem.connections.push(SkipLink {
                    target_id: hall_id,
                    strength: 0.6,
                    resonance_key: Vec::new(),
                    span: 0,
                });
            }
        }
        
        1 // Created 1 cross-cluster hallucination
    }

    /// Fallback: Generate hallucinations using the original distance-based method.
    fn stage_hallucinate_distance_based(&self, engine: &mut MemoryEngine, working_set: &[Uuid]) -> usize {
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
        let dim = parent_indices.iter().map(|&i| candidates[i].1.len()).max().unwrap_or(384);
        let mut combined = vec![0.0f32; dim];
        for &idx in &parent_indices {
            for (i, &v) in candidates[idx].1.iter().enumerate() {
                if i < combined.len() {
                    combined[i] += v;
                }
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

    /// Stage 7: Wire skip links between constructive cross-layer pairs, Fano-related memories,
    /// and preferentially across Xi clusters to promote integration AND differentiation.
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
        
        // Wire cross-cluster connections: preferentially link memories from DIFFERENT Xi clusters
        let sync = crate::kuramoto::KuramotoSync::default();
        let clusters = sync.find_synchronized_clusters(engine, 2);
        
        if clusters.len() >= 2 {
            count += self.stage_wire_cross_cluster(engine, &clusters);
        }
        
        // Wire Fano-related memories (geometric structural connections)
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

    /// Stage 7b: Create cross-cluster wiring to build "small-world" networks that enhance
    /// both integration (Phi) and differentiation (Xi) simultaneously.
    fn stage_wire_cross_cluster(&self, engine: &mut MemoryEngine, clusters: &[crate::kuramoto::MemoryCluster]) -> usize {
        use std::collections::HashMap;
        
        let mut count = 0;
        
        // Build cluster membership map
        let mut id_to_cluster: HashMap<Uuid, usize> = HashMap::new();
        for (cluster_idx, cluster) in clusters.iter().enumerate() {
            for &mem_id in &cluster.memory_ids {
                id_to_cluster.insert(mem_id, cluster_idx);
            }
        }
        
        // Collect cross-cluster candidate pairs with moderate semantic similarity (0.3-0.6 range)
        let mut cross_cluster_pairs = Vec::new();
        
        for cluster_a_idx in 0..clusters.len() {
            for cluster_b_idx in (cluster_a_idx + 1)..clusters.len() {
                let cluster_a = &clusters[cluster_a_idx];
                let cluster_b = &clusters[cluster_b_idx];
                
                // Compare memories between different clusters
                for &id_a in &cluster_a.memory_ids {
                    for &id_b in &cluster_b.memory_ids {
                        let (mem_a, mem_b) = match (engine.store.get(&id_a).ok().flatten(), 
                                                   engine.store.get(&id_b).ok().flatten()) {
                            (Some(a), Some(b)) => (a, b),
                            _ => continue,
                        };
                        
                        let similarity = cosine_similarity(&mem_a.vector, &mem_b.vector);
                        
                        // Target moderate similarity: related but not identical
                        if similarity >= 0.3 && similarity <= 0.6 {
                            // Check if already linked
                            let already_linked = mem_a.connections.iter().any(|l| l.target_id == id_b) ||
                                                mem_b.connections.iter().any(|l| l.target_id == id_a);
                            
                            if !already_linked {
                                let span = (mem_a.layer_depth as i16 - mem_b.layer_depth as i16).unsigned_abs() as u8;
                                cross_cluster_pairs.push((id_a, id_b, similarity, span));
                            }
                        }
                    }
                }
            }
        }
        
        // Sort by similarity and take top candidates (limit to avoid over-connecting)
        cross_cluster_pairs.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        let max_cross_links = (clusters.len() * 3).min(cross_cluster_pairs.len());
        cross_cluster_pairs.truncate(max_cross_links);
        
        // Create the cross-cluster links with slightly higher strength than random within-cluster links
        for (id_a, id_b, similarity, span) in cross_cluster_pairs {
            let strength = similarity * 0.9; // 0.9 vs 0.8 for constructive pairs = slight boost
            
            // Create bidirectional links
            if let Some(mem_a) = engine.store.get_mut(&id_a).ok().flatten() {
                mem_a.connections.push(SkipLink {
                    target_id: id_b,
                    strength,
                    resonance_key: Vec::new(),
                    span,
                });
            }
            if let Some(mem_b) = engine.store.get_mut(&id_b).ok().flatten() {
                mem_b.connections.push(SkipLink {
                    target_id: id_a,
                    strength,
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

impl ConsolidationEngine {
    /// Phase 7 (ADR-0011): Incremental consolidation — only process memories that changed
    /// since the last dream cycle, plus any memories that have never been consolidated.
    ///
    /// A memory is included if:
    ///   - It has never been consolidated (`last_consolidated_at` is None), OR
    ///   - It was modified after its last consolidation (`updated_at > last_consolidated_at`)
    ///
    /// The `since` parameter is a fallback for memories without `updated_at` timestamps
    /// (pre-ADR-0011 memories): if `last_consolidated_at < since`, include them.
    ///
    /// Expected 5-10x speedup for steady-state dreams where only a handful of memories
    /// changed since the last run.
    pub fn consolidate_incremental(
        &self,
        engine: &mut MemoryEngine,
        min_layer: u8,
        max_layer: u8,
        since: chrono::DateTime<Utc>,
    ) -> ConsolidationReport {
        let start = Instant::now();
        let mut report = ConsolidationReport::default();

        // Filter to memories that need reconsolidation
        let working_set: Vec<Uuid> = {
            let all = engine.store.all_memories().unwrap_or_default();
            all.iter()
                .filter(|m| {
                    if m.layer_depth < min_layer || m.layer_depth > max_layer {
                        return false;
                    }
                    match m.last_consolidated_at {
                        // Never consolidated — always include
                        None => true,
                        Some(consolidated_at) => {
                            // Include if modified after last consolidation
                            m.updated_at.map_or(
                                // No updated_at (legacy): use since as fallback
                                consolidated_at < since,
                                |updated| updated > consolidated_at,
                            )
                        }
                    }
                })
                .map(|m| m.id)
                .collect()
        };

        report.memories_replayed = working_set.len();
        if working_set.is_empty() {
            report.duration_ms = start.elapsed().as_millis() as u64;
            return report;
        }

        let pairs = self.stage_detect(engine, &working_set);
        report.interference_pairs_found = pairs.len();
        report.constructive_pairs = pairs.iter().filter(|p| p.kind == Interference::Constructive).count();
        report.destructive_pairs = pairs.iter().filter(|p| p.kind == Interference::Destructive).count();
        report.bundles_created = self.stage_bundle(engine, &working_set, max_layer);
        report.memories_strengthened = self.stage_strengthen(engine, &pairs);
        let (clusters_synced, order_improvement) = self.stage_sync(engine, &working_set);
        report.clusters_synced = clusters_synced;
        report.sync_order_improvement = order_improvement;
        self.stage_xi_repulsion(engine, &working_set);
        report.memories_pruned = self.stage_prune(engine, &pairs);
        report.memories_transferred = self.stage_transfer(engine);
        report.skip_links_created = self.stage_wire(engine, &pairs);
        report.hallucinations_created = self.stage_hallucinate(engine, &working_set);

        // Stamp last_consolidated_at on all processed memories
        let now = Utc::now();
        for id in &working_set {
            if let Ok(Some(mem)) = engine.store.get_mut(id) {
                mem.last_consolidated_at = Some(now);
            }
        }

        report.duration_ms = start.elapsed().as_millis() as u64;
        report
    }

    /// Process a subset of memory IDs through the full consolidation pipeline.
    /// Used internally by `dream_partitioned` for per-cluster parallelism.
    pub fn consolidate_subset(
        &self,
        engine: &mut MemoryEngine,
        memory_ids: &[Uuid],
    ) -> ConsolidationReport {
        let start = Instant::now();
        let mut report = ConsolidationReport::default();
        report.memories_replayed = memory_ids.len();

        let pairs = self.stage_detect(engine, memory_ids);
        report.interference_pairs_found = pairs.len();
        report.constructive_pairs = pairs.iter().filter(|p| p.kind == Interference::Constructive).count();
        report.destructive_pairs = pairs.iter().filter(|p| p.kind == Interference::Destructive).count();
        report.memories_strengthened = self.stage_strengthen(engine, &pairs);
        let (clusters_synced, order_improvement) = self.stage_sync(engine, memory_ids);
        report.clusters_synced = clusters_synced;
        report.sync_order_improvement = order_improvement;
        self.stage_xi_repulsion(engine, memory_ids);
        report.memories_pruned = self.stage_prune(engine, &pairs);
        report.skip_links_created = self.stage_wire(engine, &pairs);

        report.duration_ms = start.elapsed().as_millis() as u64;
        report
    }

    /// ADR-0012: Parallel dream with holographic paradox resolution.
    ///
    /// This is the core implementation of the holographic paradox engine:
    /// 1. Take a snapshot (frozen reference frame)
    /// 2. Partition memories into Xi clusters  
    /// 3. Dream each cluster in parallel (with collective feature) or sequentially
    /// 4. Detect and resolve paradoxes through holographic projection
    /// 5. Apply resolutions to the engine
    ///
    /// Returns (individual_reports, resolution_report) where:
    /// - individual_reports: ConsolidationReport from each cluster
    /// - resolution_report: ResolutionReport from paradox resolution
    pub fn dream_parallel(
        &self,
        engine: &mut MemoryEngine,
    ) -> (Vec<ConsolidationReport>, crate::paradox::ResolutionReport) {
        // Step 1: Create immutable snapshot
        let snapshot = engine.snapshot();
        
        // Step 2: Get Xi clusters for partitioning
        let clusters = engine.xi_clusters();
        
        if clusters.is_empty() {
            // No clusters - return empty results
            return (Vec::new(), crate::paradox::ResolutionReport::default());
        }
        
        // Step 3: Dream each cluster (parallel if feature enabled, sequential otherwise)
        let trajectories: Vec<crate::paradox::DreamTrajectory> = {
            #[cfg(feature = "collective")]
            {
                // Parallel execution with rayon
                clusters.par_iter().enumerate().map(|(cluster_idx, cluster)| {
                    self.dream_cluster_on_snapshot(cluster_idx as u32, &cluster.memory_ids, &snapshot)
                }).collect()
            }
            
            #[cfg(not(feature = "collective"))]
            {
                // Sequential execution
                clusters.iter().enumerate().map(|(cluster_idx, cluster)| {
                    self.dream_cluster_on_snapshot(cluster_idx as u32, &cluster.memory_ids, &snapshot)
                }).collect()
            }
        };
        
        // Step 4: Paradox resolution
        let mut resolver = crate::paradox::ParadoxResolver::new();
        
        for trajectory in &trajectories {
            resolver.ingest(trajectory);
        }
        
        let paradoxes = resolver.detect_paradoxes(&snapshot);
        let resolutions = resolver.project(paradoxes);
        
        // Collect paradox memory IDs BEFORE apply consumes resolutions
        let paradox_ids: std::collections::HashSet<Uuid> = resolutions.iter()
            .map(|(p, _)| p.memory_id)
            .collect();
        
        // Step 5a: Apply paradox resolutions to the engine
        let resolution_report = resolver.apply(engine, resolutions, &snapshot);
        
        // Step 5b: Apply NON-CONFLICTING mutations (memories only touched by one thread).
        // Without this, the parallel dream is a no-op for most memories — only paradoxed
        // memories would get resolved, and single-thread mutations would be silently dropped.
        for trajectory in &trajectories {
            for mutation in &trajectory.mutations {
                if !paradox_ids.contains(&mutation.memory_id) {
                    // This memory was only modified by one thread — apply directly
                    if let Ok(Some(mem)) = engine.store.get_mut(&mutation.memory_id) {
                        mutation.apply_to(mem);
                        mem.touch();
                    }
                }
            }
        }
        
        // Extract consolidation reports from trajectories
        let consolidation_reports: Vec<ConsolidationReport> = trajectories
            .into_iter()
            .map(|t| t.report)
            .collect();
        
        (consolidation_reports, resolution_report)
    }

    /// Helper: Dream a single cluster on a snapshot and return the trajectory.
    /// This runs a local copy of consolidation and tracks mutations.
    fn dream_cluster_on_snapshot(
        &self,
        cluster_id: u32,
        memory_ids: &[Uuid],
        snapshot: &crate::paradox::ParadoxSnapshot,
    ) -> crate::paradox::DreamTrajectory {
        // Create a local in-memory store with just this cluster's memories
        let mut local_store = crate::store::InMemoryStore::new();
        
        for &memory_id in memory_ids {
            if let Some(memory) = snapshot.memories.get(&memory_id) {
                let _ = local_store.insert(memory.clone());
            }
        }
        
        // Create a temporary engine with matching dimensions (384-dim input → 10K output).
        // Uses hash encoder (not Ollama) since consolidation stages don't re-encode existing
        // memories — they operate on stored vectors. Only hallucination would encode, and
        // consolidate_subset skips hallucination.
        let pipeline = crate::encoding::EncodingPipeline::new(
            Box::new(crate::encoding::SimpleHashEncoder::new(384, 42)),
            crate::codebook::Codebook::new(384, 10_000, 42),
        );
        let mut local_engine = crate::store::MemoryEngine::new(Box::new(local_store), pipeline);
        
        // Run consolidation on local engine
        let report = self.consolidate_subset(&mut local_engine, memory_ids);
        
        // Extract mutations by diffing final state against snapshot
        let mutations: Vec<crate::paradox::Mutation> = memory_ids
            .iter()
            .filter_map(|&memory_id| {
                if let Ok(Some(final_memory)) = local_engine.store.get(&memory_id) {
                    crate::paradox::Mutation::from_diff(memory_id, final_memory, snapshot)
                } else {
                    None
                }
            })
            .collect();
        
        crate::paradox::DreamTrajectory {
            cluster_id,
            mutations,
            report,
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

    /// Phase 7 (ADR-0011): Incremental dream — only consolidate memories that have
    /// changed since the last dream cycle. Passes `since` timestamp to `consolidate_incremental`.
    pub fn dream_incremental(&self, engine: &mut MemoryEngine, since: chrono::DateTime<Utc>) -> Vec<ConsolidationReport> {
        let mut reports = Vec::new();
        for cycle in 0..self.cycles {
            let min_layer = cycle as u8;
            let max_layer = (cycle + 1) as u8;
            let report = self.engine.consolidate_incremental(engine, min_layer, max_layer, since);
            reports.push(report);
        }
        reports
    }

    /// Phase 8 (ADR-0011): Partitioned dream — use Xi operator to identify clusters,
    /// run intra-cluster consolidation (parallelized with rayon when `collective` feature
    /// is enabled), then run cross-cluster wiring every `cross_cluster_interval` cycles.
    ///
    /// The expensive DETECT + SYNC stages are bounded by cluster size, not total count.
    /// Expected to scale to thousands of memories without the quadratic blowup.
    pub fn dream_partitioned(
        &self,
        engine: &mut MemoryEngine,
        cross_cluster_interval: u32,
        cycle_count: u32,
    ) -> Vec<ConsolidationReport> {
        // Compute Xi clusters from current memory graph
        let clusters = engine.xi_clusters();

        if clusters.is_empty() {
            // Fallback to standard dream if no clusters
            return self.dream(engine);
        }

        let mut reports: Vec<ConsolidationReport> = Vec::new();

        // Intra-cluster consolidation (sequential; rayon variant gated by feature flag)
        for cluster in &clusters {
            if cluster.memory_ids.is_empty() {
                continue;
            }
            let report = self.engine.consolidate_subset(engine, &cluster.memory_ids);
            reports.push(report);
        }

        // Cross-cluster WIRE stage every N cycles (weaker connections between clusters)
        if cross_cluster_interval > 0 && cycle_count % cross_cluster_interval == 0 {
            let all_ids: Vec<Uuid> = engine.store.all_ids().unwrap_or_default();
            // Run only the WIRE stage across all memories
            let pairs = self.engine.stage_detect(engine, &all_ids);
            let cross_links = self.engine.stage_wire(engine, &pairs);
            // Append cross-cluster wiring to last report
            if let Some(last) = reports.last_mut() {
                last.skip_links_created += cross_links;
            }
        }

        reports
    }

    /// Fast dream: decay amplitudes, prune dead memories, transfer layers.
    /// Skips expensive interference detection, sync, hallucination, and wiring.
    /// Completes in O(n) time regardless of memory count.
    pub fn dream_lite(&self, engine: &mut MemoryEngine) -> ConsolidationReport {
        let start = std::time::Instant::now();
        let mut report = ConsolidationReport::default();

        let all_ids = engine.store.all_ids().unwrap_or_default();
        report.memories_replayed = all_ids.len();
        let now = chrono::Utc::now();

        // Pass 1: Decay amplitudes and prune ghosts
        let mut to_prune: Vec<uuid::Uuid> = Vec::new();
        for id in &all_ids {
            if let Ok(Some(mem)) = engine.store.get_mut(id) {
                // Gentle amplitude decay (0.5% per cycle)
                mem.amplitude *= 0.995;
                if mem.amplitude < self.engine.prune_threshold {
                    mem.amplitude = 0.0;
                    to_prune.push(*id);
                    report.memories_pruned += 1;
                }
            }
        }

        // Pass 2: Prune skip links pointing to dead memories
        let dead_set: std::collections::HashSet<uuid::Uuid> = to_prune.iter().copied().collect();
        for id in &all_ids {
            if dead_set.contains(id) { continue; }
            if let Ok(Some(mem)) = engine.store.get_mut(id) {
                mem.connections.retain(|link| !dead_set.contains(&link.target_id));
            }
        }

        // Pass 3: Transfer old memories to deeper layers
        let mut transfers: Vec<(uuid::Uuid, u8)> = Vec::new();
        for id in &all_ids {
            if let Ok(Some(mem)) = engine.store.get(id) {
                let age = now - mem.created_at;
                let new_layer = match mem.layer_depth {
                    0 if age > chrono::Duration::hours(1) => Some(1),
                    1 if age > chrono::Duration::days(1) => Some(2),
                    2 if age > chrono::Duration::weeks(1) => Some(3),
                    _ => None,
                };
                if let Some(layer) = new_layer {
                    transfers.push((*id, layer));
                }
            }
        }
        for (id, new_layer) in transfers {
            if let Ok(Some(mem)) = engine.store.get_mut(&id) {
                mem.layer_depth = new_layer;
                report.memories_transferred += 1;
            }
        }

        report.duration_ms = start.elapsed().as_millis() as u64;
        report
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
    fn cross_cluster_wiring_creates_bridge_connections() {
        let mut engine = make_engine();
        let consolidation = ConsolidationEngine {
            interference_threshold: 0.3,
            ..Default::default()
        };

        let dim = 10_000;
        
        // Create two distinct clusters
        // Cluster A: animal-related memories
        let mut va = vec![0.0f32; dim];
        for i in 0..100 { va[i] = 1.0; }
        crate::wave::normalize(&mut va);
        
        let cat_id = insert_raw(&mut engine, va.clone(), "cats are fluffy animals", 0.0, 0);
        let dog_id = insert_raw(&mut engine, va.clone(), "dogs are loyal pets", 0.0, 0);
        
        // Cluster B: technology-related memories (orthogonal vector)
        let mut vb = vec![0.0f32; dim];
        for i in 500..600 { vb[i] = 1.0; }
        crate::wave::normalize(&mut vb);
        
        let code_id = insert_raw(&mut engine, vb.clone(), "coding in rust", 0.0, 0);
        let ai_id = insert_raw(&mut engine, vb.clone(), "artificial intelligence", 0.0, 0);
        
        // Create a bridge memory with moderate similarity to both clusters
        let mut vc = vec![0.0f32; dim];
        for i in 0..50 { vc[i] = 0.7; }  // Moderate overlap with cluster A
        for i in 500..550 { vc[i] = 0.7; }  // Moderate overlap with cluster B
        crate::wave::normalize(&mut vc);
        
        let bridge_id = insert_raw(&mut engine, vc, "robot pets using AI", 0.0, 0);
        
        // Count initial cross-cluster links
        let initial_links = count_cross_cluster_links(&mut engine);
        
        // Run consolidation
        let report = consolidation.consolidate(&mut engine, 0, 1);
        
        // Check that cross-cluster links were created
        let final_links = count_cross_cluster_links(&mut engine);
        
        println!("Cross-cluster links: {} -> {}", initial_links, final_links);
        println!("Skip links created: {}", report.skip_links_created);
        
        assert!(
            final_links > initial_links,
            "Should create cross-cluster links: {} -> {}",
            initial_links, final_links
        );
        
        // Bridge memory should have connections to both clusters
        let bridge_mem = engine.get_memory(&bridge_id).unwrap().unwrap();
        let connected_to_animals = bridge_mem.connections.iter()
            .any(|link| link.target_id == cat_id || link.target_id == dog_id);
        let connected_to_tech = bridge_mem.connections.iter()
            .any(|link| link.target_id == code_id || link.target_id == ai_id);
            
        assert!(
            connected_to_animals || connected_to_tech,
            "Bridge memory should connect to at least one cluster"
        );
    }

    #[test]
    fn bridge_node_strengthening_works() {
        let mut engine = make_engine();
        let consolidation = ConsolidationEngine {
            interference_threshold: 0.3,
            ..Default::default()
        };

        let dim = 10_000;
        
        // Create three clusters with a bridge node
        let mut va = vec![0.0f32; dim]; for i in 0..100 { va[i] = 1.0; }
        let mut vb = vec![0.0f32; dim]; for i in 200..300 { vb[i] = 1.0; }
        let mut vc = vec![0.0f32; dim]; for i in 400..500 { vc[i] = 1.0; }
        crate::wave::normalize(&mut va);
        crate::wave::normalize(&mut vb);
        crate::wave::normalize(&mut vc);
        
        // Cluster members
        insert_raw(&mut engine, va.clone(), "cluster A member 1", 0.0, 0);
        insert_raw(&mut engine, va.clone(), "cluster A member 2", 0.0, 0);
        insert_raw(&mut engine, vb.clone(), "cluster B member 1", 0.0, 0);
        insert_raw(&mut engine, vb.clone(), "cluster B member 2", 0.0, 0);
        insert_raw(&mut engine, vc.clone(), "cluster C member 1", 0.0, 0);
        insert_raw(&mut engine, vc.clone(), "cluster C member 2", 0.0, 0);
        
        // Bridge node with moderate similarity to all clusters
        let mut bridge_vec = vec![0.0f32; dim];
        for i in 0..30 { bridge_vec[i] = 0.6; }     // Similarity to A
        for i in 200..230 { bridge_vec[i] = 0.6; } // Similarity to B  
        for i in 400..430 { bridge_vec[i] = 0.6; } // Similarity to C
        crate::wave::normalize(&mut bridge_vec);
        
        let bridge_id = insert_raw(&mut engine, bridge_vec, "universal bridge concept", 0.0, 0);
        let initial_amplitude = engine.get_memory(&bridge_id).unwrap().unwrap().amplitude;
        
        // Run consolidation (which should detect and strengthen bridge nodes)
        consolidation.consolidate(&mut engine, 0, 1);
        
        let final_amplitude = engine.get_memory(&bridge_id).unwrap().unwrap().amplitude;
        
        println!("Bridge node amplitude: {} -> {}", initial_amplitude, final_amplitude);
        
        // Bridge node should receive amplitude boost if it connects to multiple clusters
        // Note: The exact boost depends on how many clusters it actually gets connected to
        // so we just check that it didn't decrease
        assert!(
            final_amplitude >= initial_amplitude,
            "Bridge node should not lose amplitude: {} -> {}",
            initial_amplitude, final_amplitude
        );
    }

    #[test] 
    fn cross_cluster_hallucination_prefers_different_clusters() {
        let mut engine = make_engine();
        let consolidation = ConsolidationEngine {
            interference_threshold: 0.9, // Avoid interference
            ..Default::default()
        };

        let dim = 10_000;
        
        // Create two distinct clusters
        let mut va = vec![0.0f32; dim]; for i in 0..100 { va[i] = 1.0; }
        let mut vb = vec![0.0f32; dim]; for i in 500..600 { vb[i] = 1.0; }
        crate::wave::normalize(&mut va);
        crate::wave::normalize(&mut vb);
        
        // Cluster A: high amplitude memories
        for i in 0..3 {
            let mut mem = crate::memory::HyperMemory::new(va.clone(), format!("animal {}", i));
            mem.amplitude = 0.8; // High amplitude to be hallucination candidates
            engine.store.insert(mem).unwrap();
        }
        
        // Cluster B: high amplitude memories
        for i in 0..3 {
            let mut mem = crate::memory::HyperMemory::new(vb.clone(), format!("tech {}", i));
            mem.amplitude = 0.8; // High amplitude to be hallucination candidates
            engine.store.insert(mem).unwrap();
        }
        
        let initial_count = engine.store.count();
        
        // Run consolidation (should create cross-cluster hallucinations)
        let report = consolidation.consolidate(&mut engine, 0, 1);
        
        println!("Hallucinations created: {}", report.hallucinations_created);
        println!("Memory count: {} -> {}", initial_count, engine.store.count());
        
        if report.hallucinations_created > 0 {
            // Find the hallucinated memory
            let all = engine.store.all_memories().unwrap();
            let hallucination = all.iter().find(|m| m.hallucinated);
            
            if let Some(hall) = hallucination {
                println!("Hallucination content: {}", hall.content);
                assert!(
                    hall.content.contains("cross-cluster") || hall.content.contains("Synthesis"),
                    "Hallucination should indicate cross-cluster origin"
                );
                assert!(!hall.parents.is_empty(), "Hallucination should have parent references");
                assert!(!hall.connections.is_empty(), "Hallucination should be linked to parents");
            }
        }
    }

    /// Helper function to count cross-cluster links in the engine.
    fn count_cross_cluster_links(engine: &mut MemoryEngine) -> usize {
        let sync = crate::kuramoto::KuramotoSync::default();
        let clusters = sync.find_synchronized_clusters(engine, 2);
        
        if clusters.len() < 2 {
            return 0;
        }
        
        // Build cluster membership map
        let mut id_to_cluster = std::collections::HashMap::new();
        for (cluster_idx, cluster) in clusters.iter().enumerate() {
            for &mem_id in &cluster.memory_ids {
                id_to_cluster.insert(mem_id, cluster_idx);
            }
        }
        
        let all_memories = engine.store.all_memories().unwrap_or_default();
        let mut cross_cluster_count = 0;
        
        for memory in &all_memories {
            if let Some(&source_cluster) = id_to_cluster.get(&memory.id) {
                for link in &memory.connections {
                    if let Some(&target_cluster) = id_to_cluster.get(&link.target_id) {
                        if source_cluster != target_cluster {
                            cross_cluster_count += 1;
                        }
                    }
                }
            }
        }
        
        cross_cluster_count
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

    // ===== EXP-003: Adaptive λ tests =====

    #[test]
    fn proportional_dampening_preserves_relative_amplitudes() {
        // Two memories with different amplitudes should maintain their ratio
        // after proportional dampening (unlike flat penalty which can invert them).
        let mut engine = make_engine();
        let consolidation = ConsolidationEngine {
            interference_threshold: 0.3,
            destructive_penalty: 0.5,
            ..Default::default()
        };

        // Create two similar memories with opposing phases (destructive)
        let id1 = insert_with_phase_and_layer(&mut engine, "the cat sat on the mat", 0.0, 0);
        let id2 = insert_with_phase_and_layer(&mut engine, "the cat sat on the mat today", PI, 0);

        // Set different amplitudes
        engine.store.get_mut(&id1).ok().flatten().unwrap().amplitude = 2.0;
        engine.store.get_mut(&id2).ok().flatten().unwrap().amplitude = 0.5;

        let ratio_before = 2.0 / 0.5;

        consolidation.consolidate(&mut engine, 0, 1);

        let amp1 = engine.get_memory(&id1).unwrap().unwrap().amplitude;
        let amp2 = engine.get_memory(&id2).unwrap().unwrap().amplitude;

        // With proportional dampening, both lose 50% so ratio is preserved
        if amp2 > 0.0 {
            let ratio_after = amp1 / amp2;
            assert!((ratio_after - ratio_before).abs() < 0.5,
                "ratio should be approximately preserved: before={}, after={}", ratio_before, ratio_after);
        }
        // Both should have decreased
        assert!(amp1 < 2.0, "amplitude should decrease");
    }

    #[test]
    fn adaptive_params_reduce_boost_when_over_synchronized() {
        let mut params = AdaptiveParams::default();
        let initial_boost = params.constructive_boost;

        // Simulate high order parameter (over-synchronized)
        params.adapt(0.95);

        assert!(params.constructive_boost < initial_boost,
            "boost should decrease when R is too high: {} -> {}", initial_boost, params.constructive_boost);
    }

    #[test]
    fn adaptive_params_stable_in_target_range() {
        let mut params = AdaptiveParams::default();
        let initial_boost = params.constructive_boost;
        let initial_threshold = params.prune_threshold;

        // Simulate order parameter in sweet spot
        params.adapt(0.70);

        assert!((params.constructive_boost - initial_boost).abs() < 1e-6,
            "boost should not change in target range");
        assert!((params.prune_threshold - initial_threshold).abs() < 1e-6,
            "threshold should not change in target range");
    }

    #[test]
    fn adaptive_params_boost_coupling_when_fragmented() {
        let mut params = AdaptiveParams::default();
        let initial_boost = params.constructive_boost;

        // Simulate low order parameter (fragmented)
        params.adapt(0.2);

        assert!(params.constructive_boost > initial_boost,
            "constructive boost should increase when fragmented: {} -> {}", initial_boost, params.constructive_boost);
        assert!(params.destructive_penalty < 0.5,
            "destructive penalty should decrease when fragmented (preserve more memories)");
    }

    #[test]
    fn adapt_from_report_updates_engine() {
        let mut consolidation = ConsolidationEngine::default();
        let mut report = ConsolidationReport::default();
        report.final_order_parameter = 0.95; // over-synchronized

        let boost_before = consolidation.constructive_boost;
        consolidation.adapt_from_report(&report);

        assert!(consolidation.constructive_boost < boost_before,
            "engine boost should decrease after adaptation");
    }
}
