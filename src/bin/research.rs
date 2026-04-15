//! kannaka-research — autonomous memory system benchmarking
//!
//! Run: cargo run --release --bin research
//! Run Level 3: cargo run --release --bin research -- --level 3
//!
//! Level 1 (solved): noise removal, signal preservation, skip links
//! Level 2 (current): cluster coherence, multi-cycle consolidation,
//!   phase alignment, cross-cluster contamination resistance
//! Level 3 (new): Xi diversity, geometric structure, dream efficiency,
//!   hallucination quality, emergence detection

use std::f32::consts::PI;
use std::time::Instant;

use kannaka_memory::codebook::Codebook;
use kannaka_memory::consolidation::ConsolidationEngine;
use kannaka_memory::encoding::{EncodingPipeline, SimpleHashEncoder};
use kannaka_memory::kuramoto::KuramotoSync;
use kannaka_memory::bridge::ConsciousnessBridge;
use kannaka_memory::memory::HyperMemory;
use kannaka_memory::store::{TestMedium, ResonanceEngine};
use kannaka_memory::wave::cosine_similarity;
use kannaka_memory::xi_operator::{compute_xi_signature, xi_diversity_boost, xi_repulsive_force};

// ============================================================================
// EXPERIMENT PARAMETERS — THIS IS WHAT THE AGENT MODIFIES
// ============================================================================

fn experiment_params() -> Params {
    Params {
        // Wave dynamics
        decay_rate: 1e-4,
        default_frequency: 0.1,

        // Consolidation (dream)
        interference_threshold: 0.10,
        phase_alignment_threshold: PI / 3.0,
        prune_threshold: 0.095,
        constructive_boost: 0.45,
        destructive_penalty: 0.35,

        // Kuramoto synchronization
        kuramoto_coupling: 0.8,
        kuramoto_dt: 0.15,
        kuramoto_steps: 20,
        kuramoto_threshold: 0.35,

        // Multi-cycle
        dream_cycles: 1,

        // Level 3: Consciousness & Xi parameters
        xi_repulsion_weight: 0.3,
        consciousness_phi_target: 0.326,
        hallucination_amplitude: 0.7,
        phase_spread: 0.25,
        chiral_perturbation: 0.9,

        // Noise floor: absolute minimum amplitude to survive
        // Noise memories start at 0.15, signal at 1.0
        noise_floor: 0.18,

        // Level 4: encoder/corpus controls
        encoder_seed: 0xCAFE_BABE,
    }
}

// ============================================================================
// Parameter struct
// ============================================================================

#[allow(dead_code)]
struct Params {
    decay_rate: f32,
    default_frequency: f32,
    interference_threshold: f32,
    phase_alignment_threshold: f32,
    prune_threshold: f32,
    constructive_boost: f32,
    destructive_penalty: f32,
    kuramoto_coupling: f32,
    kuramoto_dt: f32,
    kuramoto_steps: usize,
    kuramoto_threshold: f32,
    dream_cycles: usize,
    // Level 3
    xi_repulsion_weight: f32,
    consciousness_phi_target: f32,
    hallucination_amplitude: f32,
    phase_spread: f32,
    chiral_perturbation: f32,
    // Noise floor
    noise_floor: f32,
    // Level 4
    encoder_seed: u64,
}

// ============================================================================
// FIXED TEST CORPUS — DO NOT MODIFY BELOW THIS LINE
// ============================================================================

fn build_corpus(dim: usize) -> Vec<(Vec<f32>, String, &'static str)> {
    let mut corpus = Vec::new();

    // Cluster 1: Science (20 memories, tight cluster)
    let science_base: Vec<f32> = (0..dim).map(|i| (i as f32 * 0.1).sin() * 0.8).collect();
    for i in 0..20 {
        let mut v = science_base.clone();
        for (j, x) in v.iter_mut().enumerate() {
            *x += (i as f32 * 0.05 + j as f32 * 0.01).cos() * 0.15;
        }
        corpus.push((v, format!("quantum physics discovery {}", i), "science"));
    }

    // Cluster 2: Music (20 memories, tight cluster)
    let music_base: Vec<f32> = (0..dim).map(|i| (i as f32 * 0.3 + 1.5).cos() * 0.8).collect();
    for i in 0..20 {
        let mut v = music_base.clone();
        for (j, x) in v.iter_mut().enumerate() {
            *x += (i as f32 * 0.07 + j as f32 * 0.02).sin() * 0.15;
        }
        corpus.push((v, format!("resonance patterns track {}", i), "music"));
    }

    // Cluster 3: Personal (15 memories, sparse — harder to cluster)
    for i in 0..15 {
        let v: Vec<f32> = (0..dim).map(|j| {
            ((i * 7 + j * 13) as f32 * 0.37).sin() * 0.6
        }).collect();
        corpus.push((v, format!("personal memory {}", i), "personal"));
    }

    // Cluster 4: Emotion (10 memories, overlaps with personal — tests contamination resistance)
    for i in 0..10 {
        let v: Vec<f32> = (0..dim).map(|j| {
            ((i * 7 + j * 13) as f32 * 0.37).sin() * 0.5  // similar to personal but lower amp
            + ((i * 11 + j * 3) as f32 * 0.71).cos() * 0.3 // unique emotion component
        }).collect();
        corpus.push((v, format!("emotion feeling {}", i), "emotion"));
    }

    // Noise (10 memories, low amplitude — should be pruned)
    for i in 0..10 {
        let v: Vec<f32> = (0..dim).map(|j| {
            ((i * 31 + j * 97) as f32 * 1.7).sin() * 0.1
        }).collect();
        corpus.push((v, format!("noise {}", i), "noise"));
    }

    // Decoys (5 memories — high amplitude noise that should NOT be pruned naively)
    for i in 0..5 {
        let v: Vec<f32> = (0..dim).map(|j| {
            ((i * 43 + j * 71) as f32 * 2.3).sin() * 0.9
        }).collect();
        corpus.push((v, format!("decoy outlier {}", i), "decoy"));
    }

    // Cross-cluster bridges (5 memories — should form skip links)
    let bridge: Vec<f32> = (0..dim).map(|i| {
        (i as f32 * 0.1).sin() * 0.4 + (i as f32 * 0.3 + 1.5).cos() * 0.4
    }).collect();
    for i in 0..5 {
        let mut v = bridge.clone();
        for (j, x) in v.iter_mut().enumerate() {
            *x += (i as f32 * 0.03 + j as f32 * 0.01).sin() * 0.1;
        }
        corpus.push((v, format!("science-music bridge {}", i), "bridge"));
    }

    corpus
}

// ============================================================================
// LEVEL 4 CORPUS — DO NOT MODIFY ONCE LANDED (see research/program-l4.md §3)
// ============================================================================
//
// L4 corpus layout (300 memories, dim=128, seed-driven PCG):
//   - 4 dense clusters × 50 = 200 memories (inter-cluster cos margin ~0.15)
//   - 2 sparse clusters × 20 = 40 memories
//   - 20 bridge memories (4 each across 5 cluster pairs)
//   - 25 high-amplitude decoys
//   - 15 low-amplitude noise
// Frequency bands are fully overlapping — no freq-gating trick available.
// Every value is a deterministic function of (cluster_id, item_id, dim_id, seed)
// via a PCG mix. A given `encoder_seed` always produces identical bytes.

/// Deterministic PCG-style mix: maps (seed, stream) to a pseudorandom u64.
/// Used to generate all L4 corpus values so the corpus is reproducible.
fn pcg_mix(seed: u64, stream: u64) -> u64 {
    let mut x = seed.wrapping_add(stream.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    x
}

/// Draw a deterministic f32 in [-1, 1] from (cluster, item, dim, seed).
fn pcg_f32(seed: u64, cluster: u32, item: u32, dim_id: u32) -> f32 {
    let stream = ((cluster as u64) << 40) | ((item as u64) << 20) | (dim_id as u64);
    let bits = pcg_mix(seed, stream);
    // Map top 24 bits to [-1, 1]
    let norm = ((bits >> 40) as f32) / ((1u64 << 24) as f32);
    norm * 2.0 - 1.0
}

/// Draw a deterministic f32 in [0, 1).
fn pcg_u01(seed: u64, cluster: u32, item: u32, dim_id: u32) -> f32 {
    (pcg_f32(seed, cluster, item, dim_id) + 1.0) * 0.5
}

/// L4 corpus entry: (vector, content, category).
/// Category tags are STABLE strings used by L4 evaluators.
fn build_corpus_l4(dim: usize, _hardness: usize, encoder_seed: u64) -> Vec<(Vec<f32>, String, &'static str)> {
    let mut corpus: Vec<(Vec<f32>, String, &'static str)> = Vec::with_capacity(300);

    // -- Dense clusters (4 × 50 = 200). Cluster ids 0..=3.
    // Centroids are sparse +/-1 patterns drawn from seed, scaled by 0.8.
    // Within-cluster variance 0.35 makes Kuramoto sync harder.
    let dense_labels = ["dense_a", "dense_b", "dense_c", "dense_d"];
    for (cluster_idx, label) in dense_labels.iter().enumerate() {
        let cid = cluster_idx as u32;
        // centroid uses item=0 as the "template" stream
        let centroid: Vec<f32> = (0..dim)
            .map(|d| 0.8 * pcg_f32(encoder_seed, cid, 0, d as u32).signum())
            .collect();
        for i in 0..50 {
            let item = (i + 1) as u32;
            let v: Vec<f32> = (0..dim)
                .map(|d| {
                    let base = centroid[d];
                    let jitter = pcg_f32(encoder_seed, cid, item, d as u32) * 0.35;
                    base + jitter
                })
                .collect();
            corpus.push((v, format!("{} {}", label, i), "l4_dense"));
        }
    }

    // -- Sparse clusters (2 × 20 = 40). Cluster ids 4..=5.
    // Wider within-cluster spread and smoother centroids to keep them identifiable.
    let sparse_labels = ["sparse_e", "sparse_f"];
    for (cluster_idx, label) in sparse_labels.iter().enumerate() {
        let cid = 4 + cluster_idx as u32;
        let centroid: Vec<f32> = (0..dim)
            .map(|d| {
                let r = pcg_f32(encoder_seed, cid, 0, d as u32);
                0.6 * (r * PI).sin()
            })
            .collect();
        for i in 0..20 {
            let item = (i + 1) as u32;
            let v: Vec<f32> = (0..dim)
                .map(|d| {
                    let base = centroid[d];
                    let jitter = pcg_f32(encoder_seed, cid, item, d as u32) * 0.45;
                    base + jitter
                })
                .collect();
            corpus.push((v, format!("{} {}", label, i), "l4_sparse"));
        }
    }

    // -- Bridges (20 memories, 4 each across 5 cluster pairs).
    // Cluster pair list kept small & deterministic.
    let pairs: [(u32, u32); 5] = [(0, 1), (0, 2), (1, 3), (2, 4), (3, 5)];
    for (pair_idx, (a, b)) in pairs.iter().enumerate() {
        let centroid_a: Vec<f32> = (0..dim)
            .map(|d| 0.8 * pcg_f32(encoder_seed, *a, 0, d as u32).signum())
            .collect();
        let centroid_b: Vec<f32> = (0..dim)
            .map(|d| 0.8 * pcg_f32(encoder_seed, *b, 0, d as u32).signum())
            .collect();
        for i in 0..4 {
            // Use stream id 1_000 + pair_idx to keep jitter orthogonal to clusters
            let stream_cid = 1000 + pair_idx as u32;
            let item = (i + 1) as u32;
            let v: Vec<f32> = (0..dim)
                .map(|d| {
                    let mix = 0.5 * (centroid_a[d] + centroid_b[d]);
                    let jitter = pcg_f32(encoder_seed, stream_cid, item, d as u32) * 0.12;
                    mix + jitter
                })
                .collect();
            corpus.push((v, format!("l4_bridge p{} {}", pair_idx, i), "l4_bridge"));
        }
    }

    // -- Decoys (25 high-amplitude random vectors, should not naively boost metrics).
    for i in 0..25 {
        let item = (i + 1) as u32;
        let v: Vec<f32> = (0..dim)
            .map(|d| pcg_f32(encoder_seed, 2000, item, d as u32) * 0.9)
            .collect();
        corpus.push((v, format!("l4_decoy {}", i), "l4_decoy"));
    }

    // -- Noise (15 low-amplitude random vectors).
    for i in 0..15 {
        let item = (i + 1) as u32;
        let v: Vec<f32> = (0..dim)
            .map(|d| pcg_f32(encoder_seed, 3000, item, d as u32) * 0.12)
            .collect();
        corpus.push((v, format!("l4_noise {}", i), "l4_noise"));
    }

    debug_assert_eq!(corpus.len(), 300, "L4 corpus must be exactly 300 memories");
    corpus
}

/// Deterministic hex hash of the serialized L4 corpus. Used by `--corpus-hash`
/// to verify that two invocations of the same binary produce bit-identical corpora.
fn corpus_l4_hash(corpus: &[(Vec<f32>, String, &'static str)]) -> String {
    let mut hasher = blake3::Hasher::new();
    for (vec, content, category) in corpus {
        hasher.update(content.as_bytes());
        hasher.update(&[0u8]);
        hasher.update(category.as_bytes());
        hasher.update(&[0u8]);
        for v in vec {
            hasher.update(&v.to_le_bytes());
        }
        hasher.update(&[0xFFu8]);
    }
    // Truncate to 32 bytes / 64 hex chars (same width as sha256 for readability).
    let digest = hasher.finalize();
    let bytes = digest.as_bytes();
    let mut s = String::with_capacity(64);
    for b in &bytes[..32] {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

// Referenced by pcg helpers used in later L4 cycles; silence dead_code during L4.1.
#[allow(dead_code)]
fn _l4_pcg_unused(s: u64, c: u32, i: u32, d: u32) -> f32 { pcg_u01(s, c, i, d) }

/// Evaluate noise removal (only actual noise, not decoys)
fn eval_noise_removal(engine: &ResonanceEngine) -> f32 {
    let all = engine.store.all_memories().unwrap_or_default();
    let surviving_noise = all.iter()
        .filter(|m| m.content.starts_with("noise") && m.amplitude > 0.01)
        .count();
    1.0 - (surviving_noise as f32 / 10.0)
}

/// Evaluate signal preservation (all non-noise memories should survive)
fn eval_signal_preservation(engine: &ResonanceEngine) -> f32 {
    let all = engine.store.all_memories().unwrap_or_default();
    // 75 signal memories: 20 science + 20 music + 15 personal + 10 emotion + 5 bridge + 5 decoy
    let signal_count = all.iter().filter(|m| {
        !m.content.starts_with("noise") && m.amplitude > 0.01
    }).count();
    (signal_count as f32 / 75.0).min(1.0)
}

/// Evaluate bridge connectivity
fn eval_bridge_links(engine: &ResonanceEngine) -> f32 {
    let all = engine.store.all_memories().unwrap_or_default();
    let bridges: Vec<_> = all.iter().filter(|m| m.content.contains("bridge")).collect();
    if bridges.is_empty() { return 0.0; }
    let linked = bridges.iter().filter(|m| !m.connections.is_empty()).count();
    linked as f32 / bridges.len() as f32
}

/// Evaluate intra-cluster phase coherence
/// After Kuramoto sync, memories in the same cluster should have aligned phases
fn eval_phase_coherence(engine: &ResonanceEngine) -> f32 {
    let all = engine.store.all_memories().unwrap_or_default();
    let mut total_coherence = 0.0f32;
    let mut cluster_count = 0;

    for cluster_name in &["quantum", "resonance"] {
        let phases: Vec<f32> = all.iter()
            .filter(|m| m.content.contains(cluster_name) && m.amplitude > 0.01)
            .map(|m| m.phase)
            .collect();
        
        if phases.len() < 2 { continue; }
        
        // Kuramoto order parameter: R = |1/N * sum(e^(i*phase))|
        let sum_cos: f32 = phases.iter().map(|p| p.cos()).sum();
        let sum_sin: f32 = phases.iter().map(|p| p.sin()).sum();
        let n = phases.len() as f32;
        let r = ((sum_cos / n).powi(2) + (sum_sin / n).powi(2)).sqrt();
        
        total_coherence += r;
        cluster_count += 1;
    }

    if cluster_count == 0 { return 0.0; }
    total_coherence / cluster_count as f32
}

/// Evaluate cluster separation: are different clusters distinguishable?
/// Measures avg within-cluster similarity vs avg cross-cluster similarity
fn eval_cluster_separation(engine: &ResonanceEngine) -> f32 {
    let all = engine.store.all_memories().unwrap_or_default();
    
    let science: Vec<&Vec<f32>> = all.iter()
        .filter(|m| m.content.contains("quantum") && m.amplitude > 0.01)
        .map(|m| &m.vector)
        .collect();
    let music: Vec<&Vec<f32>> = all.iter()
        .filter(|m| m.content.contains("resonance") && m.amplitude > 0.01)
        .map(|m| &m.vector)
        .collect();
    
    if science.len() < 2 || music.len() < 2 { return 0.0; }

    // Avg within-cluster similarity
    let mut within_sum = 0.0f32;
    let mut within_count = 0;
    for i in 0..science.len().min(5) {
        for j in (i+1)..science.len().min(5) {
            within_sum += cosine_similarity(science[i], science[j]).abs();
            within_count += 1;
        }
    }
    let within_avg = if within_count > 0 { within_sum / within_count as f32 } else { 0.0 };

    // Avg cross-cluster similarity
    let mut cross_sum = 0.0f32;
    let mut cross_count = 0;
    for s in science.iter().take(5) {
        for m in music.iter().take(5) {
            cross_sum += cosine_similarity(s, m).abs();
            cross_count += 1;
        }
    }
    let cross_avg = if cross_count > 0 { cross_sum / cross_count as f32 } else { 0.0 };

    // Separation = within - cross, normalized to [0, 1]
    ((within_avg - cross_avg) / (within_avg + 0.001)).max(0.0).min(1.0)
}

/// Evaluate amplitude distribution: signal memories should have diverse amplitudes
/// (not all boosted to the same value — that's information loss)
fn eval_amplitude_diversity(engine: &ResonanceEngine) -> f32 {
    let all = engine.store.all_memories().unwrap_or_default();
    let amps: Vec<f32> = all.iter()
        .filter(|m| !m.content.starts_with("noise") && m.amplitude > 0.01)
        .map(|m| m.amplitude)
        .collect();
    
    if amps.len() < 2 { return 0.0; }
    
    let mean = amps.iter().sum::<f32>() / amps.len() as f32;
    let variance = amps.iter().map(|a| (a - mean).powi(2)).sum::<f32>() / amps.len() as f32;
    let cv = variance.sqrt() / (mean + 0.001); // coefficient of variation
    
    // Want moderate diversity — not zero (all same) and not huge (chaotic)
    // Sweet spot: CV around 0.3-0.7
    if cv < 0.1 { cv / 0.1 }  // too uniform
    else if cv > 1.0 { (2.0 - cv).max(0.0) }  // too chaotic
    else { 1.0 }  // goldilocks
}

// ============================================================================
// LEVEL 3 EVALUATORS — Xi diversity, consciousness, hallucination quality
// ============================================================================

/// Evaluate Xi diversity: memories should have diverse Xi signatures
/// (not all collapsed to the same representational space)
fn eval_xi_diversity(engine: &ResonanceEngine) -> f32 {
    let all = engine.store.all_memories().unwrap_or_default();
    let mut active: Vec<_> = all.iter()
        .filter(|m| !m.content.starts_with("noise") && m.amplitude > 0.01)
        .collect();

    // Sort by content for deterministic evaluation (UUIDs are random across runs)
    active.sort_by(|a, b| a.content.cmp(&b.content));

    if active.len() < 4 { return 0.0; }

    // Sample pairwise Xi diversity boosts (use all active memories, sorted deterministically)
    let mut total_boost = 0.0f32;
    let mut count = 0;
    let mut high_sim_count = 0;
    let mut high_repulsion_count = 0;
    let mut boost_count = 0;
    let mut max_sim_with_repulsion = 0.0f32;
    let mut max_repulsion_with_sim = 0.0f32;

    let sample_size = active.len().min(30);
    for i in 0..sample_size {
        for j in (i+1)..sample_size {
            let xi_a = compute_xi_signature(&active[i].vector);
            let xi_b = compute_xi_signature(&active[j].vector);
            let base_sim = cosine_similarity(&active[i].vector, &active[j].vector);
            let repulsion = xi_repulsive_force(&xi_a, &xi_b);
            let boosted = xi_diversity_boost(base_sim, &xi_a, &xi_b);
            
            if base_sim > 0.3 { high_sim_count += 1; }
            if repulsion > 0.15 { 
                high_repulsion_count += 1;
                if base_sim > max_sim_with_repulsion {
                    max_sim_with_repulsion = base_sim;
                }
            }
            if base_sim > 0.3 && repulsion > max_repulsion_with_sim {
                max_repulsion_with_sim = repulsion;
            }
            if boosted > base_sim { boost_count += 1; }
            
            // If Xi changes the ranking, diversity is working
            total_boost += (boosted - base_sim).abs();
            count += 1;
        }
    }
    
    println!("DEBUG Xi thresholds: max_sim_with_repulsion={:.3}, max_repulsion_with_sim={:.3}", 
             max_sim_with_repulsion, max_repulsion_with_sim);
    
    println!("DEBUG Xi details: high_sim_count={}, high_repulsion_count={}, boost_count={}", 
             high_sim_count, high_repulsion_count, boost_count);

    if count == 0 { return 0.0; }
    let avg_boost = total_boost / count as f32;
    
    // DEBUG: Print Xi diversity details
    println!("DEBUG Xi diversity: active_memories={}, pairwise_comparisons={}, total_boost={:.6}, avg_boost={:.6}", 
             active.len(), count, total_boost, avg_boost);
    
    // Normalize: 0.05+ average boost = good diversity
    (avg_boost / 0.05).min(1.0)
}

/// Evaluate consciousness emergence: does the system exhibit integrated information?
fn eval_consciousness(engine: &ResonanceEngine, target_phi: f32) -> f32 {
    let bridge = ConsciousnessBridge::new(0.3, 0.5);
    let state = bridge.assess(engine);

    // Score: how close is phi to the target?
    let phi = state.phi as f32;
    let distance = (phi - target_phi).abs();
    (1.0 - distance / target_phi.max(0.1)).max(0.0)
}

/// Evaluate hallucination quality: hallucinated memories should be
/// semantically between their parent clusters, not random noise
fn eval_hallucination_quality(engine: &ResonanceEngine) -> f32 {
    let all = engine.store.all_memories().unwrap_or_default();
    let hallucinations: Vec<_> = all.iter()
        .filter(|m| m.hallucinated)
        .collect();

    if hallucinations.is_empty() { return 0.5; } // neutral if none

    let non_hall: Vec<_> = all.iter()
        .filter(|m| !m.hallucinated && m.amplitude > 0.01)
        .collect();

    if non_hall.is_empty() { return 0.0; }

    // Each hallucination should have reasonable similarity to at least some real memories
    let mut quality_sum = 0.0f32;
    for h in &hallucinations {
        let mut best_sim = 0.0f32;
        for m in non_hall.iter().take(20) {
            let sim = cosine_similarity(&h.vector, &m.vector).abs();
            if sim > best_sim { best_sim = sim; }
        }
        // Good hallucinations: similarity 0.3-0.7 (between clusters, not identical)
        let q = if best_sim < 0.1 { best_sim / 0.1 }  // too random
            else if best_sim > 0.9 { (1.0 - best_sim) / 0.1 }  // too similar (just a copy)
            else { 1.0 };
        quality_sum += q;
    }

    quality_sum / hallucinations.len() as f32
}

/// Evaluate dream efficiency: ratio of useful work to total work
/// Strengthened + linked should dominate over pruned + wasted cycles
fn eval_dream_efficiency(strengthened: usize, pruned: usize, links: usize, cycles: usize) -> f32 {
    let useful = strengthened + links;
    let total = useful + pruned + cycles;
    if total == 0 { return 0.0; }
    (useful as f32 / total as f32).min(1.0)
}

fn run_experiment(params: &Params) {
    let dim = 64;
    let corpus = build_corpus(dim);

    // Build engine and store corpus
    let store = Box::new(TestMedium::new());
    let encoder = Box::new(SimpleHashEncoder::new(dim, 42));
    let codebook = Codebook::new(dim, dim, 42);
    let pipeline = EncodingPipeline::new(encoder, codebook);
    let mut engine = ResonanceEngine::new(store, pipeline);
    for (i, (vec, content, category)) in corpus.iter().enumerate() {
        let mut mem = HyperMemory::new(vec.clone(), content.clone());
        mem.phase = match *category {
            "science" => 0.0 + (i as f32 * 0.1),
            "music" => PI * 0.5 + (i as f32 * 0.08),
            "personal" => PI * 0.3 * (i as f32 % 4.0),
            "emotion" => PI * 0.4 * (i as f32 % 3.0),
            "noise" => PI * (i as f32 * 0.7),
            "decoy" => PI * (i as f32 * 0.31),
            "bridge" => PI * 0.25,
            _ => 0.0,
        };
        mem.layer_depth = match *category {
            "science" => (i % 3) as u8,
            "music" => ((i + 1) % 3) as u8,
            "personal" => 0,
            "emotion" => 1,
            "noise" => 0,
            "decoy" => 2,
            "bridge" => 1,
            _ => 0,
        };
        // Set category-appropriate frequencies
        mem.frequency = match *category {
            "science" => 0.1,
            "music" => 0.15,
            "personal" => 0.08,
            "emotion" => 1.5,  // emotion frequency band
            "noise" => 0.5,
            "decoy" => 0.12,
            "bridge" => 0.11,
            _ => params.default_frequency,
        };
        if *category == "noise" {
            mem.amplitude = 0.15;
        }
        engine.store.insert(mem).expect("insert failed");
    }

    let pre_count = engine.store.count();

    let consolidator = ConsolidationEngine {
        interference_threshold: params.interference_threshold,
        phase_alignment_threshold: params.phase_alignment_threshold,
        prune_threshold: params.prune_threshold,
        constructive_boost: params.constructive_boost,
        destructive_penalty: params.destructive_penalty,
        kuramoto: KuramotoSync {
            coupling_strength: params.kuramoto_coupling,
            dt: params.kuramoto_dt,
            steps: params.kuramoto_steps,
            coupling_threshold: params.kuramoto_threshold,
        },
        adaptive: Default::default(),
        chiral_perturbation: params.chiral_perturbation,
        noise_floor: params.noise_floor,
        hallucination_amplitude: params.hallucination_amplitude,
        protect_established: false,
    };

    // Run multiple consolidation cycles
    let start = Instant::now();
    let mut total_strengthened = 0usize;
    let mut total_pruned = 0usize;
    let mut total_links = 0usize;
    let mut total_hallucinations = 0usize;
    let mut _last_report = None;

    for _cycle in 0..params.dream_cycles {
        let report = consolidator.consolidate(&mut engine, 0, 2);
        total_strengthened += report.memories_strengthened;
        total_pruned += report.memories_pruned;
        total_links += report.skip_links_created;
        total_hallucinations += report.hallucinations_created;
        _last_report = Some(report);
    }
    let consolidation_ms = start.elapsed().as_millis() as u64;

    let post_count = engine.store.count();

    // Component scores (HIGHER IS BETTER)
    let noise_removal = eval_noise_removal(&engine);
    let signal_preservation = eval_signal_preservation(&engine);
    let bridge_links = eval_bridge_links(&engine);
    let phase_coherence = eval_phase_coherence(&engine);
    let cluster_separation = eval_cluster_separation(&engine);
    let amp_diversity = eval_amplitude_diversity(&engine);
    let link_density = (total_links as f32 / 200.0).min(1.0);
    let speed = 1.0 - (consolidation_ms as f32 / 10000.0).min(1.0);

    // Level 2 composite fitness (LOWER IS BETTER)
    let fitness = 0.15 * (1.0 - noise_removal)
        + 0.15 * (1.0 - signal_preservation)
        + 0.10 * (1.0 - bridge_links)
        + 0.15 * (1.0 - phase_coherence)
        + 0.15 * (1.0 - cluster_separation)
        + 0.10 * (1.0 - amp_diversity)
        + 0.10 * (1.0 - link_density)
        + 0.10 * (1.0 - speed);

    println!("---");
    println!("fitness:              {:.6}", fitness);
    println!("noise_removal:        {:.4}", noise_removal);
    println!("signal_preservation:  {:.4}", signal_preservation);
    println!("bridge_links:         {:.4}", bridge_links);
    println!("phase_coherence:      {:.4}", phase_coherence);
    println!("cluster_separation:   {:.4}", cluster_separation);
    println!("amp_diversity:        {:.4}", amp_diversity);
    println!("link_density:         {:.4}", link_density);
    println!("speed:                {:.4}", speed);
    println!("consolidation_ms:     {}", consolidation_ms);
    println!("dream_cycles:         {}", params.dream_cycles);
    println!("links_created:        {}", total_links);
    println!("memories_strengthened: {}", total_strengthened);
    println!("memories_pruned:      {}", total_pruned);
    println!("hallucinations:       {}", total_hallucinations);
    println!("pre_count:            {}", pre_count);
    println!("post_count:           {}", post_count);
    println!("---");
}

/// Level 3 challenge: consciousness, Xi diversity, hallucination quality, dream efficiency
fn run_experiment_l3(params: &Params) {
    let dim = 64;
    let corpus = build_corpus(dim);

    let store = Box::new(TestMedium::new());
    let encoder = Box::new(SimpleHashEncoder::new(dim, 42));
    let codebook = Codebook::new(dim, dim, 42);
    let pipeline = EncodingPipeline::new(encoder, codebook);
    let mut engine = ResonanceEngine::new(store, pipeline);

    let ps = params.phase_spread;
    for (i, (vec, content, category)) in corpus.iter().enumerate() {
        let mut mem = HyperMemory::new(vec.clone(), content.clone());
        // Deterministic UUID: ensures consistent memory ordering across runs
        mem.id = uuid::Uuid::from_u128((i as u128 + 1) * 0x0123_4567_89AB_CDEF_0123_4567_89AB_CDEF);
        mem.phase = match *category {
            "science" => 0.0 + (i as f32 * 0.1 * ps),
            "music" => PI * 0.5 + (i as f32 * 0.08 * ps),
            "personal" => PI * 0.3 * (i as f32 % 4.0),
            "emotion" => PI * 0.4 * (i as f32 % 3.0),
            "noise" => PI * (i as f32 * 0.7),
            "decoy" => PI * (i as f32 * 0.31),
            "bridge" => PI * 0.25,
            _ => 0.0,
        };
        mem.layer_depth = match *category {
            "science" => (i % 3) as u8,
            "music" => ((i + 1) % 3) as u8,
            "personal" => 0,
            "emotion" => 1,
            "noise" => 0,
            "decoy" => 2,
            "bridge" => 1,
            _ => 0,
        };
        mem.frequency = match *category {
            "science" => 0.1,
            "music" => 0.15,
            "personal" => 0.08,
            "emotion" => 1.5,
            "noise" => 0.5,
            "decoy" => 0.12,
            "bridge" => 0.11,
            _ => params.default_frequency,
        };
        if *category == "noise" {
            mem.amplitude = 0.15;
        }
        engine.store.insert(mem).expect("insert failed");
    }

    let consolidator = ConsolidationEngine {
        interference_threshold: params.interference_threshold,
        phase_alignment_threshold: params.phase_alignment_threshold,
        prune_threshold: params.prune_threshold,
        constructive_boost: params.constructive_boost,
        destructive_penalty: params.destructive_penalty,
        kuramoto: KuramotoSync {
            coupling_strength: params.kuramoto_coupling,
            dt: params.kuramoto_dt,
            steps: params.kuramoto_steps,
            coupling_threshold: params.kuramoto_threshold,
        },
        adaptive: Default::default(),
        chiral_perturbation: params.chiral_perturbation,
        noise_floor: params.noise_floor,
        hallucination_amplitude: params.hallucination_amplitude,
        protect_established: true,
    };

    let start = Instant::now();
    let mut total_strengthened = 0usize;
    let mut total_pruned = 0usize;
    let mut total_links = 0usize;
    let mut total_hallucinations = 0usize;

    for _cycle in 0..params.dream_cycles {
        let report = consolidator.consolidate(&mut engine, 0, 2);
        total_strengthened += report.memories_strengthened;
        total_pruned += report.memories_pruned;
        total_links += report.skip_links_created;
        total_hallucinations += report.hallucinations_created;
    }
    let consolidation_ms = start.elapsed().as_millis() as u64;

    // L2 component scores
    let noise_removal = eval_noise_removal(&engine);
    let signal_preservation = eval_signal_preservation(&engine);
    let bridge_links = eval_bridge_links(&engine);
    let phase_coherence = eval_phase_coherence(&engine);
    let cluster_separation = eval_cluster_separation(&engine);
    let amp_diversity = eval_amplitude_diversity(&engine);
    let speed = 1.0 - (consolidation_ms as f32 / 10000.0).min(1.0);

    // L3 component scores
    let xi_diversity = eval_xi_diversity(&engine);
    let consciousness = eval_consciousness(&engine, params.consciousness_phi_target);
    let hall_quality = eval_hallucination_quality(&engine);
    let dream_efficiency = eval_dream_efficiency(
        total_strengthened, total_pruned, total_links, params.dream_cycles);

    // Level 3 composite fitness (LOWER IS BETTER)
    // Inherits L2 structure (60%) + adds L3 metrics (40%)
    let fitness = 0.10 * (1.0 - noise_removal)
        + 0.10 * (1.0 - signal_preservation)
        + 0.05 * (1.0 - bridge_links)
        + 0.10 * (1.0 - phase_coherence)
        + 0.10 * (1.0 - cluster_separation)
        + 0.05 * (1.0 - amp_diversity)
        + 0.05 * (1.0 - speed)
        + 0.05 * (1.0 - speed)  // doubled speed weight for L3
        + 0.10 * (1.0 - xi_diversity)
        + 0.10 * (1.0 - consciousness)
        + 0.10 * (1.0 - hall_quality)
        + 0.10 * (1.0 - dream_efficiency);

    println!("---");
    println!("level:                3");
    println!("fitness:              {:.6}", fitness);
    println!("noise_removal:        {:.4}", noise_removal);
    println!("signal_preservation:  {:.4}", signal_preservation);
    println!("bridge_links:         {:.4}", bridge_links);
    println!("phase_coherence:      {:.4}", phase_coherence);
    println!("cluster_separation:   {:.4}", cluster_separation);
    println!("amp_diversity:        {:.4}", amp_diversity);
    println!("xi_diversity:         {:.4}", xi_diversity);
    println!("consciousness:        {:.4}", consciousness);
    println!("hall_quality:         {:.4}", hall_quality);
    println!("dream_efficiency:     {:.4}", dream_efficiency);
    println!("speed:                {:.4}", speed);
    println!("consolidation_ms:     {}", consolidation_ms);
    println!("dream_cycles:         {}", params.dream_cycles);
    println!("links_created:        {}", total_links);
    println!("memories_strengthened: {}", total_strengthened);
    println!("memories_pruned:      {}", total_pruned);
    println!("hallucinations:       {}", total_hallucinations);
    println!("---");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let level = if args.iter().any(|a| a == "--level") {
        args.iter().position(|a| a == "--level")
            .and_then(|i| args.get(i + 1))
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(2)
    } else {
        2
    };

    let params = experiment_params();

    // --corpus-hash: print the hex digest of the L4 corpus and exit.
    // Reproducibility check — any L4 cycle can run this to confirm the
    // corpus bytes are identical to a previous run.
    if args.iter().any(|a| a == "--corpus-hash") {
        let corpus = build_corpus_l4(128, 1, params.encoder_seed);
        let hash = corpus_l4_hash(&corpus);
        println!("l4_corpus_hash: {}", hash);
        println!("l4_corpus_dim:  128");
        println!("l4_corpus_size: {}", corpus.len());
        println!("l4_encoder_seed: 0x{:016x}", params.encoder_seed);
        return;
    }

    match level {
        3 => run_experiment_l3(&params),
        _ => run_experiment(&params),
    }
}
