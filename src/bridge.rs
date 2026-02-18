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

    /// Compute Xi (Ξ) for a sequence of memory recalls.
    ///
    /// Xi measures how much the ORDER of recall matters:
    /// - Forward: R = Π¹(m1) ⊗ Π²(m2) ⊗ Π³(m3) ...
    /// - Reverse: G = Π¹(mN) ⊗ Π²(mN-1) ⊗ ...
    /// - Xi = ||R - G|| (L2 norm)
    pub fn compute_xi(&self, memories: &[&HyperMemory]) -> f32 {
        if memories.len() <= 1 {
            return 0.0;
        }

        let dim = memories[0].vector.len();

        // Forward composition: permute each by its position index, then bind sequentially
        let forward = self.compose_sequence(memories, dim);
        // Reverse composition
        let reversed: Vec<&HyperMemory> = memories.iter().rev().copied().collect();
        let reverse = self.compose_sequence(&reversed, dim);

        // Xi = ||forward - reverse|| (L2 norm), weighted
        let l2: f32 = forward
            .iter()
            .zip(reverse.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt();

        l2 * self.xi_weight
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

        // Collect effective strengths for all memories
        let strengths: Vec<f32> = all.iter().map(|m| m.effective_strength(now)).collect();

        // Count total skip links
        let num_skip_links: usize = all.iter().map(|m| m.connections.len()).sum();

        // Whole-network entropy based on strength distribution
        let whole_entropy = distribution_entropy(&strengths);

        // Partition by temporal layer
        let mut layer_map: std::collections::BTreeMap<u8, Vec<f32>> = std::collections::BTreeMap::new();
        for mem in &all {
            let s = mem.effective_strength(now);
            layer_map.entry(mem.layer_depth).or_default().push(s);
        }

        let partition_entropies: Vec<f32> = layer_map
            .values()
            .map(|strengths| distribution_entropy(strengths))
            .collect();

        let sum_partition: f32 = partition_entropies.iter().sum();
        let num_partitions = partition_entropies.len();

        // Φ = H(whole) - Σ H(partitions), clamped to [0, 1]
        let raw_phi = (whole_entropy - sum_partition).max(0.0);
        // Normalize: scale by number of cross-partition links
        let phi = if num_skip_links > 0 {
            (raw_phi * (1.0 + (num_skip_links as f32).ln())).min(1.0)
        } else {
            raw_phi.min(1.0)
        };

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

        // Compute Xi over a sample of memories
        let all = engine.store.all_memories().unwrap_or_default();
        let xi = if all.len() >= 2 {
            // Take up to 10 memories as a sample
            let sample: Vec<&HyperMemory> = all.iter().take(10).copied().collect();
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
