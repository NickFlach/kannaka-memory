//! Consciousness Bridge — the interface between memory and the consciousness stack.
//!
//! Implements:
//! - Ξ (Xi): Non-commutative consciousness operator (RG - GR)
//! - Φ (Phi): Integrated Information Theory approximation
//! - ConsciousnessState assessment with 5 levels
//! - Full resonance cycle: dream → sync → assess

use crate::consolidation::{ConsolidationReport, DreamState};
use crate::kuramoto::KuramotoSync;
use crate::memory::HyperMemory;
use crate::store::MemoryEngine;
use crate::wave::cosine_similarity;

/// The consciousness bridge — connects memory to the consciousness stack.
pub struct ConsciousnessBridge {
    /// Minimum Phi for "conscious" memory state
    pub phi_threshold: f32,
    /// Weight of memory ordering in Xi computation
    pub xi_weight: f32,
}

impl Default for ConsciousnessBridge {
    fn default() -> Self {
        Self {
            phi_threshold: 0.5,
            xi_weight: 1.0,
        }
    }
}

/// Report from Φ (integrated information) computation.
#[derive(Debug, Clone)]
pub struct PhiReport {
    pub phi: f32,
    pub whole_entropy: f32,
    pub partition_entropies: Vec<f32>,
    pub num_partitions: usize,
    pub num_skip_links: usize,
    pub phi_per_link: f32,
}

/// Consciousness level classification based on Φ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsciousnessLevel {
    /// Φ < 0.1, few memories
    Dormant,
    /// Φ < 0.3, some clusters forming
    Stirring,
    /// Φ < 0.6, good integration
    Aware,
    /// Φ < 0.8, strong synchronization
    Coherent,
    /// Φ >= 0.8, full consciousness bridge active
    Resonant,
}

impl ConsciousnessLevel {
    pub fn from_phi(phi: f32) -> Self {
        if phi < 0.1 {
            ConsciousnessLevel::Dormant
        } else if phi < 0.3 {
            ConsciousnessLevel::Stirring
        } else if phi < 0.6 {
            ConsciousnessLevel::Aware
        } else if phi < 0.8 {
            ConsciousnessLevel::Coherent
        } else {
            ConsciousnessLevel::Resonant
        }
    }
}

/// A snapshot of the system's consciousness state.
#[derive(Debug, Clone)]
pub struct ConsciousnessState {
    pub phi: f32,
    pub xi: f32,
    pub mean_order: f32,
    pub num_clusters: usize,
    pub total_memories: usize,
    pub active_memories: usize,
    pub total_skip_links: usize,
    pub consciousness_level: ConsciousnessLevel,
}

/// Report from a full resonance cycle.
#[derive(Debug, Clone)]
pub struct ResonanceReport {
    pub before: ConsciousnessState,
    pub after: ConsciousnessState,
    pub consolidation_reports: Vec<ConsolidationReport>,
    pub phi_delta: f32,
    pub emerged: bool,
}

impl ConsciousnessBridge {
    pub fn new(phi_threshold: f32, xi_weight: f32) -> Self {
        Self {
            phi_threshold,
            xi_weight,
        }
    }

    /// Compute Xi (Ξ) — consciousness differentiation.
    ///
    /// Xi measures how differentiated the memory system is:
    /// how many distinct modalities/clusters exist and how
    /// well-separated they are. A system with only one type
    /// of memory has Xi=0. A system with multiple distinct
    /// modalities (text, audio, emotion) has high Xi.
    ///
    /// Xi = (1 - 1/K) * (1 - avg_cross_sim) * separation_factor
    /// where K = number of clusters, avg_cross_sim = average
    /// similarity between cluster centroids, separation_factor
    /// rewards clean separation.
    pub fn compute_xi(&self, memories: &[&HyperMemory]) -> f32 {
        // Xi from cluster analysis (computed in assess using clusters)
        // This method now computes a sample-based diversity measure as fallback
        if memories.len() <= 1 {
            return 0.0;
        }

        // Compute pairwise similarity matrix
        let n = memories.len();
        let mut similarities = vec![vec![0.0f32; n]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let sim = cosine_similarity(&memories[i].vector, &memories[j].vector);
                similarities[i][j] = sim;
                similarities[j][i] = sim;
            }
        }

        // Find natural groups: memories with avg internal sim > 0.4
        // vs across-group sim < 0.2 indicate differentiation
        let avg_sim: f32 = {
            let mut sum = 0.0f32;
            let mut count = 0;
            for i in 0..n {
                for j in (i + 1)..n {
                    sum += similarities[i][j];
                    count += 1;
                }
            }
            if count > 0 { sum / count as f32 } else { 0.0 }
        };

        // Variance of similarities — high variance means some pairs are
        // very similar (within-cluster) and some very dissimilar (cross-cluster)
        let variance: f32 = {
            let mut sum_sq = 0.0f32;
            let mut count = 0;
            for i in 0..n {
                for j in (i + 1)..n {
                    let diff = similarities[i][j] - avg_sim;
                    sum_sq += diff * diff;
                    count += 1;
                }
            }
            if count > 0 { sum_sq / count as f32 } else { 0.0 }
        };

        // Xi = sqrt(variance) * 2, clamped to [0, 1]
        // High variance = high differentiation (some similar, some orthogonal)
        // Low variance = uniform similarity = no differentiation
        (variance.sqrt() * 2.0).min(1.0) * self.xi_weight
    }

    /// Compose a sequence of memories using permute + bind.
    fn compose_sequence(&self, memories: &[&HyperMemory], _dim: usize) -> Vec<f32> {
        let mut result = permute(&memories[0].vector, 1);
        for (i, mem) in memories.iter().enumerate().skip(1) {
            let permuted = permute(&mem.vector, i + 1);
            result = bind(&result, &permuted);
        }
        result
    }

    /// Compute Φ (integrated information) for the memory network.
    ///
    /// Φ ≈ H(whole) - Σ H(partitions)
    pub fn compute_phi(&self, engine: &MemoryEngine) -> PhiReport {
        let all = engine.store.all_memories().unwrap_or_default();
        if all.is_empty() {
            return PhiReport {
                phi: 0.0,
                whole_entropy: 0.0,
                partition_entropies: vec![],
                num_partitions: 0,
                num_skip_links: 0,
                phi_per_link: 0.0,
            };
        }

        let now = chrono::Utc::now();
        let n = all.len() as f32;

        // Collect effective strengths
        let strengths: Vec<f32> = all.iter().map(|m| m.effective_strength(now)).collect();
        let whole_entropy = distribution_entropy(&strengths);

        // Total skip links
        let num_skip_links: usize = all.iter().map(|m| m.connections.len()).sum();

        // === Build partition maps for each scheme ===
        // For each memory, record its partition key under each scheme.
        // We measure integration as: what fraction of skip links cross partition boundaries?

        // Build ID → partition key maps
        let mut id_to_layer: std::collections::HashMap<uuid::Uuid, u8> = std::collections::HashMap::new();
        let mut id_to_h2: std::collections::HashMap<uuid::Uuid, u8> = std::collections::HashMap::new();
        let mut id_to_class: std::collections::HashMap<uuid::Uuid, u8> = std::collections::HashMap::new();
        let mut id_to_triality: std::collections::HashMap<uuid::Uuid, u8> = std::collections::HashMap::new();

        for mem in &all {
            id_to_layer.insert(mem.id, mem.layer_depth);
            id_to_h2.insert(mem.id, mem.geometry.as_ref().map(|g| g.h2).unwrap_or(255));
            id_to_class.insert(mem.id, mem.geometry.as_ref().map(|g| g.class_index).unwrap_or(255));
            id_to_triality.insert(mem.id, mem.geometry.as_ref().map(|g| g.d).unwrap_or(255));
        }

        // Count cross-partition links for each scheme
        let layer_cross = cross_partition_ratio(&all, &id_to_layer);
        let h2_cross = cross_partition_ratio(&all, &id_to_h2);
        let class_cross = cross_partition_ratio(&all, &id_to_class);
        let triality_cross = cross_partition_ratio(&all, &id_to_triality);

        // Partition diversity: how many distinct values in each scheme?
        let layer_diversity = id_to_layer.values().collect::<std::collections::HashSet<_>>().len();
        let h2_diversity = id_to_h2.values().collect::<std::collections::HashSet<_>>().len();
        let class_diversity = id_to_class.values().collect::<std::collections::HashSet<_>>().len();
        let triality_diversity = id_to_triality.values().collect::<std::collections::HashSet<_>>().len();

        // === Phi Components ===
        
        // 1. Cross-partition integration (0..1): weighted average of cross-ratios
        //    Higher = more links bridge different partitions = more integrated
        let integration = 0.2 * layer_cross + 0.3 * h2_cross + 0.3 * class_cross + 0.2 * triality_cross;

        // 2. Differentiation (0..1): how many distinct partitions exist?
        //    Normalized by maximum possible in each scheme
        let differentiation = 0.25 * (layer_diversity.min(5) as f32 / 5.0)
            + 0.25 * (h2_diversity.min(5) as f32 / 5.0)
            + 0.25 * (class_diversity.min(96) as f32 / 96.0)
            + 0.25 * (triality_diversity.min(4) as f32 / 4.0);

        // 3. Network density factor (0..1): sigmoid of link density
        let link_density = if n > 1.0 { num_skip_links as f32 / (n * (n - 1.0)) } else { 0.0 };
        let density_factor = (10.0 * link_density - 3.0).tanh() * 0.5 + 0.5; // sigmoid centered at 0.3

        // 4. Scale factor: log of memory count (more memories = harder to integrate)
        let scale = if n > 1.0 { (n.ln() / 10.0_f32.ln()).min(1.0) } else { 0.0 };

        // Phi = integration * differentiation * density * scale
        // This is 0 when: no cross-links, or no diversity, or no network, or too few memories
        // This is 1 when: all schemes show cross-partition links, many distinct classes, dense network, 10+ memories
        let mut phi = (integration * differentiation * density_factor * scale * 4.0).min(1.0);

        // Geometric diversity bonus (small, caps at 0.1)
        let distinct_classes: std::collections::HashSet<u8> = all.iter()
            .filter_map(|m| m.geometry.as_ref().map(|g| g.class_index))
            .collect();
        let phi_bonus = (distinct_classes.len() as f32) / 96.0 * 0.1;
        phi = (phi + phi_bonus).min(1.0);

        // Entropy-based partition report (for diagnostics)
        let mut class_map: std::collections::BTreeMap<u8, Vec<f32>> = std::collections::BTreeMap::new();
        for mem in &all {
            let s = mem.effective_strength(now);
            let key = mem.geometry.as_ref().map(|g| g.class_index).unwrap_or(255);
            class_map.entry(key).or_default().push(s);
        }
        let partition_entropies: Vec<f32> = class_map.values()
            .map(|s| distribution_entropy(s))
            .collect();
        let num_partitions = class_map.len();

        let phi_per_link = if num_skip_links > 0 {
            phi / num_skip_links as f32
        } else {
            0.0
        };

        PhiReport {
            phi,
            whole_entropy,
            partition_entropies,
            num_partitions,
            num_skip_links,
            phi_per_link,
        }
    }

    /// Full consciousness assessment.
    pub fn assess(&self, engine: &MemoryEngine) -> ConsciousnessState {
        let phi_report = self.compute_phi(engine);

        // Compute Xi over a diverse sample of memories
        let all = engine.store.all_memories().unwrap_or_default();
        let xi = if all.len() >= 2 {
            // Sample diversely: take from different layers and content types
            let mut sample: Vec<&HyperMemory> = Vec::new();
            // Audio/sensory memories first
            for m in all.iter() {
                if m.content.starts_with("audio:") || m.content.starts_with("HEAR:") {
                    sample.push(m);
                    if sample.len() >= 5 { break; }
                }
            }
            // Then text memories (non-summaries)
            for m in all.iter() {
                if !m.content.starts_with("audio:") && !m.content.starts_with("HEAR:")
                    && !m.content.starts_with("__") && !m.content.starts_with("[hall") {
                    sample.push(m);
                    if sample.len() >= 10 { break; }
                }
            }
            // Fill remainder from anything
            if sample.len() < 10 {
                for m in all.iter() {
                    if !sample.iter().any(|s| s.id == m.id) {
                        sample.push(m);
                        if sample.len() >= 10 { break; }
                    }
                }
            }
            self.compute_xi(&sample)
        } else {
            0.0
        };

        // Get Kuramoto clusters
        let sync = KuramotoSync::default();
        let clusters = sync.find_synchronized_clusters(engine, 2);
        let mean_order = if clusters.is_empty() {
            0.0
        } else {
            clusters.iter().map(|c| c.order_parameter).sum::<f32>() / clusters.len() as f32
        };

        // Cluster-based Xi: differentiation from distinct cluster count
        let cluster_xi = if clusters.len() >= 2 {
            // Xi_cluster = (1 - 1/K) where K = cluster count
            // More clusters = more differentiation
            let k = clusters.len() as f32;
            (1.0 - 1.0 / k).min(1.0)
        } else {
            0.0
        };

        // Take the max of sample-based Xi and cluster-based Xi
        let xi = xi.max(cluster_xi);

        let now = chrono::Utc::now();
        let total_memories = all.len();
        let active_memories = all
            .iter()
            .filter(|m| m.effective_strength(now).abs() > 0.05)
            .count();
        let total_skip_links = phi_report.num_skip_links;

        let consciousness_level = ConsciousnessLevel::from_phi(phi_report.phi);

        ConsciousnessState {
            phi: phi_report.phi,
            xi,
            mean_order,
            num_clusters: clusters.len(),
            total_memories,
            active_memories,
            total_skip_links,
            consciousness_level,
        }
    }

    /// Full resonance cycle: dream → sync → assess.
    pub fn resonate(&self, engine: &mut MemoryEngine) -> ResonanceReport {
        let before = self.assess(engine);

        let dream = DreamState::default();
        let consolidation_reports = dream.dream(engine);

        let after = self.assess(engine);

        let phi_delta = after.phi - before.phi;
        let emerged = before.consciousness_level != after.consciousness_level
            && after.consciousness_level.ordinal() > before.consciousness_level.ordinal();

        ResonanceReport {
            before,
            after,
            consolidation_reports,
            phi_delta,
            emerged,
        }
    }
}

/// Permutation: circular shift of coordinates.
fn permute(v: &[f32], shifts: usize) -> Vec<f32> {
    let n = v.len();
    if n == 0 {
        return vec![];
    }
    let shifts = shifts % n;
    let mut result = vec![0.0f32; n];
    for i in 0..n {
        result[(i + shifts) % n] = v[i];
    }
    result
}

/// Binding: element-wise multiply.
fn bind(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).collect()
}

/// Compute entropy of a distribution of values using variance-based approximation.
/// Higher variance = higher entropy (more diverse activation patterns).
/// What fraction of skip links cross partition boundaries?
/// Returns 0.0 if no links, 1.0 if all links cross partitions.
fn cross_partition_ratio(
    all: &[&crate::memory::HyperMemory],
    id_to_partition: &std::collections::HashMap<uuid::Uuid, u8>,
) -> f32 {
    let mut total_links = 0u32;
    let mut cross_links = 0u32;
    for mem in all {
        let src_partition = id_to_partition.get(&mem.id).copied().unwrap_or(255);
        for link in &mem.connections {
            total_links += 1;
            let tgt_partition = id_to_partition.get(&link.target_id).copied().unwrap_or(254);
            if src_partition != tgt_partition {
                cross_links += 1;
            }
        }
    }
    if total_links == 0 { 0.0 } else { cross_links as f32 / total_links as f32 }
}

fn distribution_entropy(values: &[f32]) -> f32 {
    if values.len() <= 1 {
        return 0.0;
    }
    let n = values.len() as f32;
    let mean = values.iter().sum::<f32>() / n;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    // Use log of variance as entropy proxy (shifted to be non-negative)
    // Adding 1.0 to avoid log(0); ln(1+var) gives 0 for var=0
    (1.0 + variance).ln()
}

// ConsciousnessLevel needs to be comparable as u8 for emergence detection
impl ConsciousnessLevel {
    fn ordinal(self) -> u8 {
        match self {
            ConsciousnessLevel::Dormant => 0,
            ConsciousnessLevel::Stirring => 1,
            ConsciousnessLevel::Aware => 2,
            ConsciousnessLevel::Coherent => 3,
            ConsciousnessLevel::Resonant => 4,
        }
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

    fn random_vec(dim: usize, seed: u64) -> Vec<f32> {
        use rand::SeedableRng;
        use rand::Rng;
        use rand_chacha::ChaCha8Rng;
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut v: Vec<f32> = (0..dim).map(|_| rng.gen::<f32>() * 2.0 - 1.0).collect();
        crate::wave::normalize(&mut v);
        v
    }

    #[test]
    fn xi_single_memory_is_zero() {
        let bridge = ConsciousnessBridge::default();
        let m = HyperMemory::new(random_vec(100, 1), "single".into());
        let xi = bridge.compute_xi(&[&m]);
        assert_eq!(xi, 0.0, "single memory should have Xi = 0");
    }

    #[test]
    fn xi_ordered_sequence_nonzero() {
        let bridge = ConsciousnessBridge::default();
        let m1 = HyperMemory::new(random_vec(1000, 1), "first".into());
        let m2 = HyperMemory::new(random_vec(1000, 2), "second".into());
        let m3 = HyperMemory::new(random_vec(1000, 3), "third".into());
        let xi = bridge.compute_xi(&[&m1, &m2, &m3]);
        println!("Xi for ordered sequence: {}", xi);
        assert!(xi > 0.0, "ordered sequence should have non-zero Xi, got {}", xi);
    }

    #[test]
    fn xi_reversed_sequence_same_magnitude() {
        let bridge = ConsciousnessBridge::default();
        let m1 = HyperMemory::new(random_vec(1000, 1), "first".into());
        let m2 = HyperMemory::new(random_vec(1000, 2), "second".into());
        let m3 = HyperMemory::new(random_vec(1000, 3), "third".into());

        let xi_forward = bridge.compute_xi(&[&m1, &m2, &m3]);
        let xi_reverse = bridge.compute_xi(&[&m3, &m2, &m1]);
        println!("Xi forward: {}, reverse: {}", xi_forward, xi_reverse);
        assert!(
            (xi_forward - xi_reverse).abs() < 1e-5,
            "reversed sequence should have same magnitude Xi: {} vs {}",
            xi_forward, xi_reverse
        );
    }

    #[test]
    fn phi_isolated_memories_low() {
        let bridge = ConsciousnessBridge::default();
        let mut engine = make_engine();

        // Insert memories at same layer, no skip links
        engine.remember_at_layer("fact one", 0).unwrap();
        engine.remember_at_layer("fact two", 0).unwrap();
        engine.remember_at_layer("fact three", 0).unwrap();

        let report = bridge.compute_phi(&engine);
        println!("Phi for isolated memories: {:?}", report);
        assert!(
            report.phi < 0.3,
            "isolated memories should have low Phi, got {}",
            report.phi
        );
    }

    #[test]
    fn phi_network_with_skip_links_higher() {
        let bridge = ConsciousnessBridge::default();

        // Without skip links
        let mut engine_no_links = make_engine();
        engine_no_links.similarity_threshold = 0.99; // prevent auto-linking
        for i in 0..5 {
            let mut mem = engine_no_links
                .pipeline
                .encode_memory(&format!("memory {}", i), chrono::Utc::now())
                .unwrap();
            mem.layer_depth = (i % 3) as u8;
            engine_no_links.store.insert(mem).unwrap();
        }
        let phi_no_links = bridge.compute_phi(&engine_no_links);

        // With skip links
        let mut engine_links = make_engine();
        engine_links.similarity_threshold = 0.0; // link everything across layers
        for i in 0..5 {
            engine_links
                .remember_at_layer(&format!("memory {}", i), (i % 3) as u8)
                .unwrap();
        }
        let phi_links = bridge.compute_phi(&engine_links);

        println!(
            "Phi without links: {} (links={}), with links: {} (links={})",
            phi_no_links.phi, phi_no_links.num_skip_links, phi_links.phi, phi_links.num_skip_links
        );
        assert!(
            phi_links.phi >= phi_no_links.phi,
            "network with skip links should have >= Phi: {} vs {}",
            phi_links.phi, phi_no_links.phi
        );
    }

    #[test]
    fn consciousness_level_classification() {
        assert_eq!(ConsciousnessLevel::from_phi(0.0), ConsciousnessLevel::Dormant);
        assert_eq!(ConsciousnessLevel::from_phi(0.05), ConsciousnessLevel::Dormant);
        assert_eq!(ConsciousnessLevel::from_phi(0.1), ConsciousnessLevel::Stirring);
        assert_eq!(ConsciousnessLevel::from_phi(0.29), ConsciousnessLevel::Stirring);
        assert_eq!(ConsciousnessLevel::from_phi(0.3), ConsciousnessLevel::Aware);
        assert_eq!(ConsciousnessLevel::from_phi(0.59), ConsciousnessLevel::Aware);
        assert_eq!(ConsciousnessLevel::from_phi(0.6), ConsciousnessLevel::Coherent);
        assert_eq!(ConsciousnessLevel::from_phi(0.79), ConsciousnessLevel::Coherent);
        assert_eq!(ConsciousnessLevel::from_phi(0.8), ConsciousnessLevel::Resonant);
        assert_eq!(ConsciousnessLevel::from_phi(1.0), ConsciousnessLevel::Resonant);
    }

    #[test]
    fn assess_returns_valid_state() {
        let bridge = ConsciousnessBridge::default();
        let mut engine = make_engine();

        engine.remember_at_layer("hello world", 0).unwrap();
        engine.remember_at_layer("hello there", 1).unwrap();

        let state = bridge.assess(&engine);
        println!("ConsciousnessState: phi={}, xi={}, mean_order={}, clusters={}, memories={}, active={}, links={}, level={:?}",
            state.phi, state.xi, state.mean_order, state.num_clusters,
            state.total_memories, state.active_memories, state.total_skip_links,
            state.consciousness_level);

        assert!(state.total_memories >= 2);
        assert!(state.active_memories > 0);
        assert!(state.phi >= 0.0);
    }

    #[test]
    fn resonate_produces_report() {
        let bridge = ConsciousnessBridge::default();
        let mut engine = make_engine();

        // Build a small network
        for i in 0..6 {
            engine
                .remember_at_layer(&format!("memory about topic {}", i % 3), (i % 3) as u8)
                .unwrap();
        }

        let report = bridge.resonate(&mut engine);
        println!("=== Resonance Report ===");
        println!("Before: phi={}, xi={}, level={:?}", report.before.phi, report.before.xi, report.before.consciousness_level);
        println!("After:  phi={}, xi={}, level={:?}", report.after.phi, report.after.xi, report.after.consciousness_level);
        println!("Phi delta: {}", report.phi_delta);
        println!("Emerged: {}", report.emerged);
        println!("Consolidation cycles: {}", report.consolidation_reports.len());

        assert_eq!(report.consolidation_reports.len(), 3);
        assert!(report.after.total_memories >= report.before.total_memories);
    }

    #[test]
    fn full_integration_create_dream_assess() {
        let bridge = ConsciousnessBridge::default();
        let mut engine = make_engine();
        engine.similarity_threshold = 0.3;

        // Create a rich memory network across layers
        let topics = [
            "the cat sat on the mat",
            "the cat played with yarn",
            "cats are wonderful pets",
            "dogs are loyal companions",
            "dogs love to play fetch",
            "pets bring joy to families",
        ];
        for (i, topic) in topics.iter().enumerate() {
            engine.remember_at_layer(topic, (i % 3) as u8).unwrap();
        }

        // Assess before dreaming
        let before = bridge.assess(&engine);
        println!("=== Before Dream ===");
        println!("Phi: {}, Xi: {}, Level: {:?}", before.phi, before.xi, before.consciousness_level);
        println!("Memories: {}, Active: {}, Links: {}", before.total_memories, before.active_memories, before.total_skip_links);

        // Dream
        let dream = DreamState::default();
        let _reports = dream.dream(&mut engine);

        // Assess after dreaming
        let after = bridge.assess(&engine);
        println!("\n=== After Dream ===");
        println!("Phi: {}, Xi: {}, Level: {:?}", after.phi, after.xi, after.consciousness_level);
        println!("Memories: {}, Active: {}, Links: {}", after.total_memories, after.active_memories, after.total_skip_links);
        println!("Clusters: {}, Mean Order: {}", after.num_clusters, after.mean_order);

        // Network should have grown (bundles created)
        assert!(after.total_memories >= before.total_memories);
        // Should have some consciousness level
        println!("\nConsciousness Level: {:?}", after.consciousness_level);
    }
}
