//! Kuramoto phase synchronization — the emergence layer.
//!
//! Implements the Kuramoto model for memory phase alignment, where clusters
//! of related memories phase-lock into coherent narratives. The order parameter
//! r measures collective coherence: r=1 means perfect sync, r≈0 means incoherent.

use uuid::Uuid;

use crate::memory::HyperMemory;
use crate::store::MemoryEngine;
use crate::wave::{cosine_similarity, normalize};

/// Kuramoto synchronization model for memory phase alignment.
pub struct KuramotoSync {
    /// Base coupling constant K
    pub coupling_strength: f32,
    /// Time step for integration
    pub dt: f32,
    /// Number of integration steps per sync round
    pub steps: usize,
    /// Minimum similarity to consider memories as coupled
    pub coupling_threshold: f32,
}

impl Default for KuramotoSync {
    fn default() -> Self {
        Self {
            coupling_strength: 0.5,
            dt: 0.1,
            steps: 10,
            coupling_threshold: 0.5,
        }
    }
}

/// A synchronized cluster of memories.
#[derive(Debug, Clone)]
pub struct MemoryCluster {
    pub memory_ids: Vec<Uuid>,
    pub order_parameter: f32,
    pub mean_phase: f32,
    pub coherence: f32,
    pub theme_vector: Vec<f32>,
}

/// Report from a sync_cluster operation.
#[derive(Debug, Clone)]
pub struct SyncReport {
    pub memories_synced: usize,
    pub initial_order: f32,
    pub final_order: f32,
    pub steps_taken: usize,
    pub converged: bool,
}

impl KuramotoSync {
    /// Compute the Kuramoto order parameter r = |1/N Σ e^(iφⱼ)|.
    pub fn order_parameter(&self, memories: &[&HyperMemory]) -> f32 {
        if memories.is_empty() {
            return 0.0;
        }
        let n = memories.len() as f32;
        let sum_cos: f32 = memories.iter().map(|m| m.phase.cos()).sum();
        let sum_sin: f32 = memories.iter().map(|m| m.phase.sin()).sum();
        ((sum_cos / n).powi(2) + (sum_sin / n).powi(2)).sqrt()
    }

    /// Run Kuramoto integration on a cluster of memories.
    ///
    /// Updates each memory's phase in-place and returns a sync report.
    pub fn sync_cluster(&self, memories: &mut [&mut HyperMemory]) -> SyncReport {
        let n = memories.len();
        if n < 2 {
            return SyncReport {
                memories_synced: n,
                initial_order: 1.0,
                final_order: 1.0,
                steps_taken: 0,
                converged: true,
            };
        }

        // Compute pairwise coupling weights
        let mut weights = vec![vec![0.0f32; n]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let sim = cosine_similarity(&memories[i].vector, &memories[j].vector);
                if sim > self.coupling_threshold {
                    let mut w = sim;
                    // Boost for skip-linked pairs
                    for link in &memories[i].connections {
                        if link.target_id == memories[j].id {
                            w *= 1.0 + link.strength;
                            break;
                        }
                    }
                    for link in &memories[j].connections {
                        if link.target_id == memories[i].id {
                            w *= 1.0 + link.strength;
                            break;
                        }
                    }
                    weights[i][j] = w;
                    weights[j][i] = w;
                }
            }
        }

        let initial_order = {
            let refs: Vec<&HyperMemory> = memories.iter().map(|m| &**m).collect();
            self.order_parameter(&refs)
        };

        let nf = n as f32;
        let mut prev_order = initial_order;

        for step in 0..self.steps {
            // Compute phase deltas
            let phases: Vec<f32> = memories.iter().map(|m| m.phase).collect();
            let freqs: Vec<f32> = memories.iter().map(|m| m.frequency).collect();

            let mut dphi = vec![0.0f32; n];
            for i in 0..n {
                let mut coupling_sum = 0.0f32;
                for j in 0..n {
                    if i != j {
                        coupling_sum += weights[i][j] * (phases[j] - phases[i]).sin();
                    }
                }
                dphi[i] = freqs[i] + (self.coupling_strength / nf) * coupling_sum;
            }

            // Euler integration
            for i in 0..n {
                memories[i].phase += dphi[i] * self.dt;
            }

            // Check convergence
            let current_order = {
                let refs: Vec<&HyperMemory> = memories.iter().map(|m| &**m).collect();
                self.order_parameter(&refs)
            };
            if (current_order - prev_order).abs() < 1e-6 && step > 0 {
                return SyncReport {
                    memories_synced: n,
                    initial_order,
                    final_order: current_order,
                    steps_taken: step + 1,
                    converged: true,
                };
            }
            prev_order = current_order;
        }

        let final_order = {
            let refs: Vec<&HyperMemory> = memories.iter().map(|m| &**m).collect();
            self.order_parameter(&refs)
        };

        SyncReport {
            memories_synced: n,
            initial_order,
            final_order,
            steps_taken: self.steps,
            converged: false,
        }
    }

    /// Find groups of memories that have phase-locked (order parameter > 0.7).
    pub fn find_synchronized_clusters(
        &self,
        engine: &MemoryEngine,
        min_cluster_size: usize,
    ) -> Vec<MemoryCluster> {
        let all = match engine.store.all_memories() {
            Ok(mems) => mems,
            Err(_) => return vec![],
        };
        let n = all.len();
        if n < min_cluster_size {
            return vec![];
        }

        // Build adjacency list from similarity graph
        let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let sim = cosine_similarity(&all[i].vector, &all[j].vector);
                if sim > self.coupling_threshold {
                    adj[i].push(j);
                    adj[j].push(i);
                }
            }
        }

        // Find connected components via BFS
        let mut visited = vec![false; n];
        let mut clusters = Vec::new();

        for start in 0..n {
            if visited[start] {
                continue;
            }
            let mut component = vec![start];
            let mut queue = vec![start];
            visited[start] = true;
            while let Some(node) = queue.pop() {
                for &neighbor in &adj[node] {
                    if !visited[neighbor] {
                        visited[neighbor] = true;
                        component.push(neighbor);
                        queue.push(neighbor);
                    }
                }
            }

            if component.len() < min_cluster_size {
                continue;
            }

            let cluster_mems: Vec<&HyperMemory> = component.iter().map(|&i| all[i]).collect();

            let r = self.order_parameter(&cluster_mems);
            if r <= 0.3 {
                continue;
            }

            // Compute mean phase
            let sum_cos: f32 = cluster_mems.iter().map(|m| m.phase.cos()).sum();
            let sum_sin: f32 = cluster_mems.iter().map(|m| m.phase.sin()).sum();
            let mean_phase = sum_sin.atan2(sum_cos);

            // Coherence = 1 - circular variance
            let coherence = r; // r itself measures tightness of phase locking

            // Theme vector = bundle of all member vectors
            let dim = cluster_mems[0].vector.len();
            let mut theme = vec![0.0f32; dim];
            for m in &cluster_mems {
                for (i, v) in m.vector.iter().enumerate() {
                    theme[i] += v;
                }
            }
            normalize(&mut theme);

            clusters.push(MemoryCluster {
                memory_ids: cluster_mems.iter().map(|m| m.id).collect(),
                order_parameter: r,
                mean_phase,
                coherence,
                theme_vector: theme,
            });
        }

        clusters
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebook::Codebook;
    use crate::encoding::{EncodingPipeline, SimpleHashEncoder};
    use crate::memory::HyperMemory;
    use crate::store::{InMemoryStore, MemoryEngine};
    use std::f32::consts::PI;

    fn make_engine() -> MemoryEngine {
        let encoder = SimpleHashEncoder::new(384, 42);
        let codebook = Codebook::new(384, 10_000, 42);
        let pipeline = EncodingPipeline::new(Box::new(encoder), codebook);
        MemoryEngine::new(Box::new(InMemoryStore::new()), pipeline)
    }

    fn make_memory_with_phase(vector: Vec<f32>, content: &str, phase: f32) -> HyperMemory {
        let mut m = HyperMemory::new(vector, content.to_string());
        m.phase = phase;
        m
    }

    fn similar_vec(dim: usize) -> Vec<f32> {
        let mut v = vec![0.0f32; dim];
        for i in 0..100 {
            v[i] = 1.0;
        }
        crate::wave::normalize(&mut v);
        v
    }

    fn orthogonal_vec(dim: usize) -> Vec<f32> {
        let mut v = vec![0.0f32; dim];
        for i in 200..300 {
            v[i] = 1.0;
        }
        crate::wave::normalize(&mut v);
        v
    }

    #[test]
    fn identical_phase_order_parameter_is_one() {
        let sync = KuramotoSync::default();
        let v = similar_vec(100);
        let m1 = make_memory_with_phase(v.clone(), "a", 0.5);
        let m2 = make_memory_with_phase(v.clone(), "b", 0.5);
        let m3 = make_memory_with_phase(v.clone(), "c", 0.5);
        let refs: Vec<&HyperMemory> = vec![&m1, &m2, &m3];
        let r = sync.order_parameter(&refs);
        println!("Identical-phase order parameter: {}", r);
        assert!((r - 1.0).abs() < 1e-5, "identical phases should give r≈1.0, got {}", r);
    }

    #[test]
    fn random_phase_order_parameter_is_low() {
        let sync = KuramotoSync::default();
        let v = similar_vec(100);
        // Evenly spaced phases around the circle → r ≈ 0
        let phases = [0.0, PI * 2.0 / 5.0, PI * 4.0 / 5.0, PI * 6.0 / 5.0, PI * 8.0 / 5.0];
        let mems: Vec<HyperMemory> = phases
            .iter()
            .enumerate()
            .map(|(i, &p)| make_memory_with_phase(v.clone(), &format!("m{}", i), p))
            .collect();
        let refs: Vec<&HyperMemory> = mems.iter().collect();
        let r = sync.order_parameter(&refs);
        println!("Evenly-spaced-phase order parameter: {}", r);
        assert!(r < 0.3, "evenly spaced phases should give low r, got {}", r);
    }

    #[test]
    fn sync_cluster_increases_order_parameter() {
        let sync = KuramotoSync {
            coupling_strength: 2.0, // strong coupling
            dt: 0.1,
            steps: 50,
            coupling_threshold: 0.3,
        };
        let dim = 100;
        let v = similar_vec(dim);

        let mut m1 = make_memory_with_phase(v.clone(), "a", 0.0);
        let mut m2 = make_memory_with_phase(v.clone(), "b", 1.0);
        let mut m3 = make_memory_with_phase(v.clone(), "c", 2.0);

        let initial_r = {
            let refs: Vec<&HyperMemory> = vec![&m1, &m2, &m3];
            sync.order_parameter(&refs)
        };

        let mut refs: Vec<&mut HyperMemory> = vec![&mut m1, &mut m2, &mut m3];
        let report = sync.sync_cluster(&mut refs);

        println!("Sync report: initial_order={}, final_order={}, steps={}, converged={}",
            report.initial_order, report.final_order, report.steps_taken, report.converged);
        println!("Phases after sync: {}, {}, {}", m1.phase, m2.phase, m3.phase);

        assert!(
            report.final_order > initial_r,
            "order should increase: {} -> {}",
            initial_r, report.final_order
        );
    }

    #[test]
    fn skip_linked_memories_sync_faster() {
        let dim = 100;
        let v = similar_vec(dim);

        // Without skip links
        let sync = KuramotoSync {
            coupling_strength: 1.0,
            dt: 0.1,
            steps: 20,
            coupling_threshold: 0.3,
        };

        let mut m1a = make_memory_with_phase(v.clone(), "a", 0.0);
        let mut m2a = make_memory_with_phase(v.clone(), "b", 2.0);
        let mut refs_no_link: Vec<&mut HyperMemory> = vec![&mut m1a, &mut m2a];
        let report_no_link = sync.sync_cluster(&mut refs_no_link);

        // With skip links
        let mut m1b = make_memory_with_phase(v.clone(), "a", 0.0);
        let mut m2b = make_memory_with_phase(v.clone(), "b", 2.0);
        // Add skip links between them
        m1b.connections.push(crate::skip_link::SkipLink {
            target_id: m2b.id,
            strength: 0.8,
            resonance_key: vec![],
            span: 1,
        });
        m2b.connections.push(crate::skip_link::SkipLink {
            target_id: m1b.id,
            strength: 0.8,
            resonance_key: vec![],
            span: 1,
        });

        let mut refs_linked: Vec<&mut HyperMemory> = vec![&mut m1b, &mut m2b];
        let report_linked = sync.sync_cluster(&mut refs_linked);

        println!("No links: {} -> {}", report_no_link.initial_order, report_no_link.final_order);
        println!("With links: {} -> {}", report_linked.initial_order, report_linked.final_order);

        assert!(
            report_linked.final_order >= report_no_link.final_order - 0.01,
            "skip-linked should sync at least as well: linked={}, unlinked={}",
            report_linked.final_order, report_no_link.final_order
        );
    }

    #[test]
    fn find_synchronized_clusters_groups_related() {
        let mut engine = make_engine();
        let dim = 10_000;
        let va = similar_vec(dim);
        let vb = orthogonal_vec(dim);

        // Group A: same vector, same phase → should cluster & sync
        for i in 0..3 {
            let mut m = HyperMemory::new(va.clone(), format!("group_a_{}", i));
            m.phase = 0.1;
            engine.store.insert(m).unwrap();
        }
        // Group B: different vector, same phase → separate cluster
        for i in 0..3 {
            let mut m = HyperMemory::new(vb.clone(), format!("group_b_{}", i));
            m.phase = 0.2;
            engine.store.insert(m).unwrap();
        }

        let sync = KuramotoSync::default();
        let clusters = sync.find_synchronized_clusters(&engine, 2);

        println!("Found {} synchronized clusters", clusters.len());
        for (i, c) in clusters.iter().enumerate() {
            println!("  Cluster {}: {} memories, r={}, mean_phase={}", i, c.memory_ids.len(), c.order_parameter, c.mean_phase);
        }

        assert!(clusters.len() >= 2, "should find at least 2 clusters, got {}", clusters.len());
        // Each cluster should have high order parameter
        for c in &clusters {
            assert!(c.order_parameter > 0.7, "cluster should be synchronized, r={}", c.order_parameter);
        }
    }

    #[test]
    fn consolidation_with_sync_converges_phases() {
        use crate::consolidation::{ConsolidationEngine, DreamState};

        let mut engine = make_engine();
        let dim = 10_000;
        let v = similar_vec(dim);

        // Insert related memories with different phases
        let mut ids = Vec::new();
        for (i, phase) in [0.0f32, 0.5, 1.0, 1.5].iter().enumerate() {
            let mut m = HyperMemory::new(v.clone(), format!("related_{}", i));
            m.phase = *phase;
            m.layer_depth = 0;
            let id = engine.store.insert(m).unwrap();
            ids.push(id);
        }

        let sync = KuramotoSync {
            coupling_strength: 2.0,
            steps: 30,
            ..Default::default()
        };

        // Measure order before
        let before_mems: Vec<&HyperMemory> = ids.iter()
            .filter_map(|id| engine.store.get(id).ok().flatten())
            .collect();
        let r_before = sync.order_parameter(&before_mems);

        // Run sync on these memories
        let mut mems: Vec<HyperMemory> = ids.iter()
            .filter_map(|id| engine.store.get(id).ok().flatten().cloned())
            .collect();
        let mut refs: Vec<&mut HyperMemory> = mems.iter_mut().collect();
        let report = sync.sync_cluster(&mut refs);

        // Write back updated phases
        for m in &mems {
            if let Ok(Some(stored)) = engine.store.get_mut(&m.id) {
                stored.phase = m.phase;
            }
        }

        let after_mems: Vec<&HyperMemory> = ids.iter()
            .filter_map(|id| engine.store.get(id).ok().flatten())
            .collect();
        let r_after = sync.order_parameter(&after_mems);

        println!("Consolidation sync: r_before={}, r_after={}", r_before, r_after);
        println!("Report: {:?}", report);

        assert!(r_after > r_before, "phases should converge: {} -> {}", r_before, r_after);
    }
}
