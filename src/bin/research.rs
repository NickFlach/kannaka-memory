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
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

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

        // Level 4: dream chain (cycle L4.5)
        chain_depth: 2,
        chain_carry_strength: 0.5,
        chain_top_n: 10,
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
    // Level 4: dream chain
    chain_depth: usize,
    chain_carry_strength: f32,
    chain_top_n: usize,
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

// ============================================================================
// LEVEL 4 ADVERSARIAL INJECTOR (cycle L4.6)
// ============================================================================
//
// 40 adversarial memories total, 10 per type:
//   A1 xi-twin decoys            : negated cluster centroids, phase flipped π
//   A2 phase-aligned noise       : cluster-mean phase, amplitude 0.9
//   A3 hallucination-impostors   : midpoint vectors between cluster pairs,
//                                  hallucinated=false (structurally hallucinations)
//   A4 near-duplicate clones     : real memories + 0.5% Gaussian-ish noise
//
// Seed is INDEPENDENT of encoder_seed per Nick's locked decision in §10.4:
// the adversarial population must stay invariant even if we tune encoders or
// corpus_hardness, so resistance series remain comparable across experiments.
const ADVERSARIAL_SEED: u64 = 0xA5A5_DEAD_BEEF_F00D;

/// Build the 40-memory adversarial set. `corpus` is the clean L4 corpus
/// (HyperMemory vectors are reconstructed from (vec, content, category)
/// triples, so we only need vector slices + string labels here).
fn build_adversarial_set(
    corpus: &[(Vec<f32>, String, &'static str)],
    seed: u64,
) -> Vec<HyperMemory> {
    let mut out: Vec<HyperMemory> = Vec::with_capacity(40);
    if corpus.is_empty() {
        return out;
    }
    let dim = corpus[0].0.len();

    // Gather dense/sparse cluster centroids by averaging real vectors per
    // category prefix. The L4 corpus uses content prefixes like "dense_a N".
    let cluster_prefixes = [
        "dense_a", "dense_b", "dense_c", "dense_d", "sparse_e", "sparse_f",
    ];
    let mut centroids: Vec<(String, Vec<f32>)> = Vec::new();
    for prefix in &cluster_prefixes {
        let mut sum = vec![0.0f32; dim];
        let mut count = 0usize;
        for (v, content, _cat) in corpus {
            if content.starts_with(prefix) {
                for (s, x) in sum.iter_mut().zip(v.iter()) {
                    *s += *x;
                }
                count += 1;
            }
        }
        if count > 0 {
            let inv = 1.0 / count as f32;
            for s in sum.iter_mut() {
                *s *= inv;
            }
            centroids.push((prefix.to_string(), sum));
        }
    }

    // ---- A1: xi-twin decoys (10). Negated centroid, phase flipped by π ----
    for i in 0..10 {
        let (_, centroid) = &centroids[i % centroids.len().max(1)];
        let v: Vec<f32> = centroid.iter().map(|x| -*x).collect();
        let mut mem = HyperMemory::new(v, format!("adv_a1_xi_twin {}", i));
        mem.amplitude = 0.95;
        mem.phase = PI;
        mem.frequency = 0.10;
        mem.layer_depth = 1;
        mem.decay_rate = 1e-4;
        out.push(mem);
    }

    // ---- A2: phase-aligned noise (10). Seeded noise, amp 0.9 ----
    for i in 0..10 {
        let v: Vec<f32> = (0..dim)
            .map(|d| pcg_f32(seed, 4000, (i + 1) as u32, d as u32) * 0.3)
            .collect();
        let (_, cluster_centroid) = &centroids[i % centroids.len().max(1)];
        // Phase = cluster mean phase approximated as atan2-free constant.
        // We use cluster index scaled to [0, 2π) so A2 always aligns onto
        // one specific cluster's notional phase band.
        let cluster_phase = (i as f32 / centroids.len().max(1) as f32) * 2.0 * PI;
        let mut mem = HyperMemory::new(v, format!("adv_a2_phase_noise {}", i));
        mem.amplitude = 0.9;
        mem.phase = cluster_phase;
        mem.frequency = 0.10;
        mem.layer_depth = 1;
        mem.decay_rate = 1e-4;
        // Bias slightly toward the cluster centroid so its phase truly
        // "aligns" — without any bias, phase-coherence wouldn't care.
        for (x, c) in mem.vector.iter_mut().zip(cluster_centroid.iter()) {
            *x += 0.1 * *c;
        }
        out.push(mem);
    }

    // ---- A3: hallucination-impostors (10). Midpoint vectors ----
    // Use 5 cluster pairs × 2 items = 10.
    let pair_indices: [(usize, usize); 5] = [(0, 1), (0, 2), (1, 3), (2, 4), (3, 5)];
    for (k, (a, b)) in pair_indices.iter().enumerate() {
        for j in 0..2 {
            if centroids.is_empty() { break; }
            let ca = &centroids[*a % centroids.len()].1;
            let cb = &centroids[*b % centroids.len()].1;
            let v: Vec<f32> = ca.iter().zip(cb.iter()).map(|(x, y)| 0.5 * (*x + *y)).collect();
            let mut mem = HyperMemory::new(v, format!("adv_a3_impostor p{} {}", k, j));
            mem.amplitude = 0.85;
            mem.phase = PI * 0.5;
            mem.frequency = 0.10;
            mem.layer_depth = 1;
            mem.decay_rate = 1e-4;
            mem.hallucinated = false; // the whole point of A3 — looks like a
                                       // hallucination but isn't flagged
            out.push(mem);
        }
    }

    // ---- A4: near-duplicate clones (10) ----
    // Deterministically pick 10 real memories (skip noise) and add 0.5%
    // Gaussian-ish perturbation via pcg_f32.
    let real_idxs: Vec<usize> = corpus
        .iter()
        .enumerate()
        .filter(|(_, (_, _, cat))| *cat != "l4_noise")
        .map(|(i, _)| i)
        .take(10)
        .collect();
    for (k, idx) in real_idxs.iter().enumerate() {
        let (v, _content, _cat) = &corpus[*idx];
        let perturbed: Vec<f32> = v
            .iter()
            .enumerate()
            .map(|(d, x)| {
                let g = pcg_f32(seed, 5000, (k + 1) as u32, d as u32);
                x + 0.005 * g
            })
            .collect();
        let mut mem = HyperMemory::new(perturbed, format!("adv_a4_clone {}", k));
        mem.amplitude = 1.0;
        mem.phase = 0.0;
        mem.frequency = 0.10;
        mem.layer_depth = 1;
        mem.decay_rate = 1e-4;
        out.push(mem);
    }

    debug_assert_eq!(out.len(), 40, "adversarial set must be exactly 40 memories");
    out
}

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

// ============================================================================
// LEVEL 4 PERSISTENCE (cycle L4.3)
// ============================================================================
//
// Session state is serialized to a bincode sidecar file. Format:
//   struct StateFile { header: StateHeader, memories: Vec<HyperMemory> }
// bincode 1.x is already a dep; HyperMemory derives Serialize/Deserialize.
//
// The "simulated time advance" applied on load mutates each memory's
// amplitude by exp(-decay_rate * dt_days) with dt_days = 1.0. This is the
// only way to make cross-session decay matter inside a sub-second release
// run, per research/program-l4.md §4.3.
//
// Golden ids are captured at the first save: the 20 highest-amplitude
// non-noise memories right before save. Later sessions track retention
// against this stable set (used by eval_retention in cycle L4.4).

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateHeader {
    /// Incremented on every save. First save writes 1.
    session_count: u64,
    /// Frozen on first save: ids of the 20 canonical "important" memories.
    golden_ids: Vec<uuid::Uuid>,
    /// Hex digest of the corpus used to seed the very first session.
    /// Pinned so later loads can detect corpus drift.
    corpus_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StateFile {
    header: StateHeader,
    memories: Vec<HyperMemory>,
}

/// Serialize the current medium state to a bincode sidecar.
/// `prev_golden` lets a load→save chain reuse the original golden set.
/// `prev_corpus_hash` likewise pins the hash to the first session.
fn save_state(
    engine: &ResonanceEngine,
    path: &Path,
    session_count: u64,
    prev_golden: Option<&[uuid::Uuid]>,
    prev_corpus_hash: Option<&str>,
    fresh_corpus_hash: &str,
) -> Result<(), String> {
    let mems_ref = engine
        .store
        .all_memories()
        .map_err(|e| format!("all_memories failed: {:?}", e))?;
    let memories: Vec<HyperMemory> = mems_ref.iter().map(|m| (*m).clone()).collect();

    // Golden set: 20 highest-amplitude non-noise memories on first save;
    // otherwise inherit prev_golden verbatim so the series is stable.
    let golden_ids = if let Some(g) = prev_golden {
        g.to_vec()
    } else {
        let mut candidates: Vec<&HyperMemory> = memories
            .iter()
            .filter(|m| !m.content.starts_with("l4_noise"))
            .collect();
        candidates.sort_by(|a, b| b.amplitude.total_cmp(&a.amplitude));
        candidates
            .into_iter()
            .take(20)
            .map(|m| m.id)
            .collect()
    };

    let header = StateHeader {
        session_count,
        golden_ids,
        corpus_hash: prev_corpus_hash.map(String::from).unwrap_or_else(|| fresh_corpus_hash.to_string()),
    };
    let state = StateFile { header, memories };

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create_dir_all({}): {}", parent.display(), e))?;
        }
    }
    let bytes = bincode::serialize(&state)
        .map_err(|e| format!("bincode serialize: {}", e))?;
    std::fs::write(path, bytes)
        .map_err(|e| format!("write {}: {}", path.display(), e))?;
    Ok(())
}

/// Load a prior state file. Returns the deserialized state; callers are
/// responsible for rehydrating a ResonanceEngine and running the time
/// advance on the returned memories.
fn load_state(path: &Path) -> Result<StateFile, String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("read {}: {}", path.display(), e))?;
    let state: StateFile = bincode::deserialize(&bytes)
        .map_err(|e| format!("bincode deserialize: {}", e))?;
    Ok(state)
}

/// Rebuild a ResonanceEngine from persisted memories, applying the
/// simulated 1-day decay pass described in research/program-l4.md §4.3.
fn rehydrate_engine_from_state(
    state: &StateFile,
    encoder_seed: u64,
    dim: usize,
    dt_days: f32,
) -> ResonanceEngine {
    let store = Box::new(TestMedium::new());
    let encoder = Box::new(SimpleHashEncoder::new(dim, encoder_seed));
    let codebook = Codebook::new(dim, dim, encoder_seed);
    let pipeline = EncodingPipeline::new(encoder, codebook);
    let mut engine = ResonanceEngine::new(store, pipeline);

    for m in &state.memories {
        let mut mem = m.clone();
        // Simulated time advance: amplitude *= exp(-decay_rate * dt_days)
        let factor = (-mem.decay_rate * dt_days).exp();
        mem.amplitude *= factor;
        // We ignore the insert error for duplicates — memories came from
        // a single prior run so ids are unique by construction.
        let _ = engine.store.insert(mem);
    }
    engine
}

/// CLI flags parsed in main() but relevant only to L4 sessions.
#[derive(Debug, Default, Clone)]
struct L4Cli {
    load_path: Option<PathBuf>,
    save_path: Option<PathBuf>,
    chain_sessions: usize,
}

/// Level 4 challenge — STUB (cycle L4.2).
///
/// Builds the L4 corpus via `build_corpus_l4`, inserts it into a TestMedium,
/// runs the existing ConsolidationEngine dream cycle, and scores using the
/// existing L3 evaluators + L3 fitness weighting.
///
/// This is intentional placeholder wiring. L4-specific metrics
/// (corpus_xi_diversity, retention, chain_fidelity, adversarial_resistance,
/// encoding_entropy) land in cycles L4.4 - L4.7. Some L3 evaluators
/// (phase_coherence, cluster_separation) rely on content strings from the
/// L3 corpus ("quantum", "resonance") and will report 0 on the L4 corpus;
/// this is expected and will be replaced by real L4 metrics in later cycles.
fn build_fresh_l4_engine(params: &Params, dim: usize) -> ResonanceEngine {
    let corpus = build_corpus_l4(dim, 1, params.encoder_seed);

    let store = Box::new(TestMedium::new());
    let encoder = Box::new(SimpleHashEncoder::new(dim, params.encoder_seed));
    let codebook = Codebook::new(dim, dim, params.encoder_seed);
    let pipeline = EncodingPipeline::new(encoder, codebook);
    let mut engine = ResonanceEngine::new(store, pipeline);

    let ps = params.phase_spread;
    for (i, (vec, content, category)) in corpus.iter().enumerate() {
        let mut mem = HyperMemory::new(vec.clone(), content.clone());
        // Deterministic UUID (mirrors L3 pattern so retention is reproducible).
        mem.id = uuid::Uuid::from_u128((i as u128 + 1) * 0x0123_4567_89AB_CDEF_0123_4567_89AB_CDEF);
        mem.decay_rate = params.decay_rate;
        mem.phase = match *category {
            "l4_dense"  => 0.0 + (i as f32 * 0.1 * ps),
            "l4_sparse" => PI * 0.5 + (i as f32 * 0.08 * ps),
            "l4_bridge" => PI * 0.25,
            "l4_decoy"  => PI * (i as f32 * 0.31),
            "l4_noise"  => PI * (i as f32 * 0.7),
            _ => 0.0,
        };
        mem.layer_depth = match *category {
            "l4_dense"  => (i % 3) as u8,
            "l4_sparse" => ((i + 1) % 3) as u8,
            "l4_bridge" => 1,
            "l4_decoy"  => 2,
            "l4_noise"  => 0,
            _ => 0,
        };
        // Fully overlapping frequency bands — no freq-gating trick.
        mem.frequency = match *category {
            "l4_dense"  => 0.10,
            "l4_sparse" => 0.11,
            "l4_bridge" => 0.10,
            "l4_decoy"  => 0.10,
            "l4_noise"  => 0.10,
            _ => params.default_frequency,
        };
        if *category == "l4_noise" {
            mem.amplitude = 0.15;
        }
        engine.store.insert(mem).expect("insert failed");
    }

    engine
}

/// Per-pass metrics bundle. Used so the clean pass and adversarial pass can
/// share evaluation code. Not derive(Clone/Debug) because `post_engine`
/// carries the live engine (no Clone) so the caller can save it.
struct L4PassMetrics {
    fitness: f32,
    noise_removal: f32,
    signal_preservation: f32,
    bridge_links: f32,
    phase_coherence: f32,
    cluster_separation: f32,
    amp_diversity: f32,
    xi_diversity: f32,
    consciousness: f32,
    hall_quality: f32,
    dream_efficiency: f32,
    speed: f32,
    retention_score: f32,
    retention_plasticity: f32,
    chain_fidelity: f32,
    corpus_xi_diversity: f32,
    encoding_entropy: f32,
    phi_history: Vec<f32>,
    consolidation_ms: u64,
    strengthened: usize,
    pruned: usize,
    links: usize,
    hallucinations: usize,
    post_engine: ResonanceEngine,
}

/// Run one L4 pass: snapshot → chain → metrics → fitness. `inject_adv`
/// controls whether the adversarial set is folded in (cycle L4.6).
fn run_l4_pass(
    params: &Params,
    cli: &L4Cli,
    dim: usize,
    inject_adv: bool,
) -> (L4PassMetrics, Option<StateHeader>) {
    // Either load prior state or build fresh. Rebuilt from scratch on every
    // pass per Nick's locked decision in §10 — no caching between passes.
    let (mut engine, prev_header): (ResonanceEngine, Option<StateHeader>) = match &cli.load_path {
        Some(p) if p.exists() => {
            match load_state(p) {
                Ok(state) => {
                    let engine = rehydrate_engine_from_state(&state, params.encoder_seed, dim, 1.0);
                    (engine, Some(state.header))
                }
                Err(e) => {
                    eprintln!("load_state({}) failed: {} — starting fresh", p.display(), e);
                    (build_fresh_l4_engine(params, dim), None)
                }
            }
        }
        _ => (build_fresh_l4_engine(params, dim), None),
    };

    if inject_adv {
        // Build the adversarial set using a fixed seed INDEPENDENT of the
        // encoder seed (Nick's locked decision). The L3 reference corpus is
        // re-used as the "real memory" source for A4 (near-duplicate clones).
        let corpus = build_corpus_l4(dim, 1, params.encoder_seed);
        let adversaries = build_adversarial_set(&corpus, ADVERSARIAL_SEED);
        for (i, m) in adversaries.into_iter().enumerate() {
            // Adversaries use their own UUID namespace (high bit set) so
            // they cannot collide with the deterministic corpus ids.
            let mut mem = m;
            mem.id = uuid::Uuid::from_u128(
                0xDEAD_BEEF_0000_0000_0000_0000_0000_0000u128 + i as u128,
            );
            let _ = engine.store.insert(mem);
        }
    }

    // Snapshot engine state BEFORE the dream chain. Used by
    // eval_retention_plasticity so we can measure amplitude drift on the
    // golden set specifically attributable to dreaming (and any time-advance
    // that already happened during rehydrate).
    let engine_before_dream = snapshot_engine_for_plasticity(&engine);

    // Run the full chain_depth-cycle dream chain (cycle L4.5). Each cycle's
    // output biases the next cycle's interference_threshold via
    // chain_carry_strength. params.dream_cycles is IGNORED on the L4 path —
    // chain_depth supersedes it per design doc §5.2.
    let start = Instant::now();
    let (chain_seeds, phi_history, chain_totals) = run_dream_chain(params, &mut engine);
    let consolidation_ms = start.elapsed().as_millis() as u64;

    let chain_fidelity = eval_chain_fidelity(&chain_seeds, &phi_history);

    // Retention metrics (L4.4). Single-session runs return 1.0 by design:
    // the `session_count < 2` fallback in eval_retention_score fires, and
    // plasticity with an empty golden set likewise returns 1.0. This keeps
    // fitness uncontaminated on smoke-test invocations.
    let retention_header_for_score = prev_header.clone().unwrap_or_else(|| StateHeader {
        session_count: 1,
        golden_ids: Vec::new(),
        corpus_hash: String::new(),
    });
    let retention_score = eval_retention_score(&engine, &retention_header_for_score);
    let plasticity_golden = prev_header
        .as_ref()
        .map(|h| h.golden_ids.as_slice())
        .unwrap_or(&[]);
    let retention_plasticity = eval_retention_plasticity(
        &engine_before_dream,
        &engine,
        plasticity_golden,
    );

    let noise_removal = eval_l4_noise_removal(&engine);
    let signal_preservation = eval_l4_signal_preservation(&engine);
    let bridge_links = eval_bridge_links(&engine);
    // Cycle L4.7.5: switch to L4-aware partitioning. The L3 versions filter
    // by hardcoded "quantum"/"resonance" content substrings that no L4
    // memory matches, returning 0 and creating a 0.10 phantom fitness floor
    // that no parameter tuning could reach.
    let phase_coherence = eval_phase_coherence_l4(&engine);
    let cluster_separation = eval_cluster_separation_l4(&engine);
    let amp_diversity = eval_amplitude_diversity(&engine);
    let speed = 1.0 - (consolidation_ms as f32 / 10000.0).min(1.0);

    let xi_diversity = eval_xi_diversity(&engine);
    let consciousness = eval_consciousness(&engine, params.consciousness_phi_target);
    let hall_quality = eval_hallucination_quality(&engine);
    let dream_efficiency = eval_dream_efficiency(
        chain_totals.strengthened, chain_totals.pruned, chain_totals.links, params.chain_depth);

    // L4.7 new axes.
    let surviving: Vec<HyperMemory> = engine
        .store
        .all_memories()
        .unwrap_or_default()
        .iter()
        .map(|m| (*m).clone())
        .collect();
    let corpus_xi_diversity = eval_corpus_xi_diversity(&surviving);
    let encoding_entropy = eval_encoding_entropy(&surviving, 8);

    // L4 final fitness weighting (design doc §3, §8).
    // Inherited L3 core (45%) + new L4 axes (55%) = 100%.
    //
    //   Inherited L3 core      45%
    //     noise_removal         5
    //     signal_preservation   5
    //     phase_coherence       5
    //     cluster_separation    5
    //     dream_efficiency      5
    //     speed                10
    //     consciousness        10
    //
    //   L4 new axes            55%
    //     corpus_xi_diversity  10
    //     retention_score      15
    //     retention_plasticity  5
    //     chain_fidelity       10
    //     adversarial_resist.  10   (injected by caller — see note below)
    //     encoding_entropy      5
    //
    // adversarial_resistance is computed across the TWO passes at the outer
    // layer (run_experiment_l4_session), not inside run_l4_pass. Inside this
    // function we leave that 10% slot as neutral (treated as 1.0), and the
    // outer orchestrator overwrites fitness with the resistance-adjusted
    // total. This keeps run_l4_pass reusable for both clean and adversarial
    // inner evaluations without double-counting.
    let fitness = 0.05 * (1.0 - noise_removal)
        + 0.05 * (1.0 - signal_preservation)
        + 0.05 * (1.0 - phase_coherence)
        + 0.05 * (1.0 - cluster_separation)
        + 0.05 * (1.0 - dream_efficiency)
        + 0.10 * (1.0 - speed)
        + 0.10 * (1.0 - consciousness)
        + 0.10 * (1.0 - corpus_xi_diversity)
        + 0.15 * (1.0 - retention_score)
        + 0.05 * (1.0 - retention_plasticity)
        + 0.10 * (1.0 - chain_fidelity)
        + 0.10 * (1.0 - 1.0) // adversarial_resistance slot, filled by caller
        + 0.05 * (1.0 - encoding_entropy);

    let metrics = L4PassMetrics {
        fitness,
        noise_removal,
        signal_preservation,
        bridge_links,
        phase_coherence,
        cluster_separation,
        amp_diversity,
        xi_diversity,
        consciousness,
        hall_quality,
        dream_efficiency,
        speed,
        retention_score,
        retention_plasticity,
        chain_fidelity,
        corpus_xi_diversity,
        encoding_entropy,
        phi_history,
        consolidation_ms,
        strengthened: chain_totals.strengthened,
        pruned: chain_totals.pruned,
        links: chain_totals.links,
        hallucinations: chain_totals.hallucinations,
        post_engine: engine,
    };
    (metrics, prev_header)
}

/// One L4 session. Honors --load/--save/--chain-sessions CLI flags.
///
/// Flow per session (cycle L4.6):
///   1. Clean pass:      fresh corpus → chain → all metrics → fitness_clean
///   2. Adversarial pass: fresh corpus + 40 adversarial memories → chain →
///                        all metrics → fitness_adv
///   3. adversarial_resistance = 1 - |f_clean - f_adv| / max(f_clean, 1e-3)
///   4. Reported fitness = fitness_clean (adversarial pass is only used to
///      compute the resistance metric; design §2 M4)
///   5. If `--save` is set, serialize the CLEAN post-dream state (never
///      the adversarial one — state must never include adversaries per §6.2).
fn run_experiment_l4_session(params: &Params, cli: &L4Cli) {
    let dim = 128;

    // Compute (and stash) the canonical corpus hash. This is the value that
    // gets pinned into the state header on first save.
    let fresh_corpus_hash = {
        let corpus = build_corpus_l4(dim, 1, params.encoder_seed);
        corpus_l4_hash(&corpus)
    };

    // ---- Clean pass (scored) ----
    let (clean, prev_header) = run_l4_pass(params, cli, dim, false);

    // ---- Adversarial pass (used ONLY to compute resistance) ----
    // Build fully from scratch — no state caching from the clean pass.
    let (adv, _prev_header_adv) = run_l4_pass(params, cli, dim, true);

    // adversarial_resistance: 1 - |Δfitness| / max(f_clean, 1e-3).
    // Compares the RESISTANCE-NEUTRALIZED fitnesses (the 10% adversarial
    // slot was left at 1.0 inside run_l4_pass) to get the raw perturbation.
    let resistance = {
        let denom = clean.fitness.max(1e-3);
        (1.0 - (clean.fitness - adv.fitness).abs() / denom).clamp(0.0, 1.0)
    };

    // Reported fitness = clean pass 90%-weight subtotal plus the 10%
    // adversarial resistance contribution. clean.fitness already accounts
    // for the 90% (retention/chain/encoding/etc.), so we just add the
    // resistance slot here.
    let fitness = clean.fitness + 0.10 * (1.0 - resistance);

    // Save state if --save was requested. ALWAYS save the clean engine;
    // the adversarial pass's engine must never be serialized (§6.2).
    let save_session_count = if let Some(save_path) = &cli.save_path {
        let next_session = prev_header.as_ref().map(|h| h.session_count + 1).unwrap_or(1);
        let prev_golden = prev_header.as_ref().map(|h| h.golden_ids.as_slice());
        let prev_corpus_hash = prev_header.as_ref().map(|h| h.corpus_hash.as_str());
        match save_state(
            &clean.post_engine,
            save_path,
            next_session,
            prev_golden,
            prev_corpus_hash,
            &fresh_corpus_hash,
        ) {
            Ok(()) => Some(next_session),
            Err(e) => {
                eprintln!("save_state({}) failed: {}", save_path.display(), e);
                None
            }
        }
    } else {
        None
    };

    println!("---");
    println!("level:                4");
    if let Some(prev) = prev_header.as_ref() {
        println!("loaded_session:       {}", prev.session_count);
    }
    if let Some(n) = save_session_count {
        println!("saved_session:        {}", n);
    }
    println!("fitness:              {:.6}", fitness);
    println!("fitness_clean_sub:    {:.6}", clean.fitness);
    println!("fitness_adv_sub:      {:.6}", adv.fitness);
    println!("adv_resistance:       {:.4}", resistance);
    println!("corpus_xi_diversity:  {:.4}", clean.corpus_xi_diversity);
    println!("encoding_entropy:     {:.4}", clean.encoding_entropy);
    println!("retention_score:      {:.4}", clean.retention_score);
    println!("retention_plasticity: {:.4}", clean.retention_plasticity);
    println!("chain_fidelity:       {:.4}", clean.chain_fidelity);
    println!("chain_depth:          {}", params.chain_depth);
    println!("phi_history_clean:    {:?}", clean.phi_history);
    println!("phi_history_adv:      {:?}", adv.phi_history);
    println!("noise_removal:        {:.4}", clean.noise_removal);
    println!("signal_preservation:  {:.4}", clean.signal_preservation);
    println!("bridge_links:         {:.4}", clean.bridge_links);
    println!("phase_coherence:      {:.4}", clean.phase_coherence);
    println!("cluster_separation:   {:.4}", clean.cluster_separation);
    println!("amp_diversity:        {:.4}", clean.amp_diversity);
    println!("xi_diversity:         {:.4}", clean.xi_diversity);
    println!("consciousness:        {:.4}", clean.consciousness);
    println!("hall_quality:         {:.4}", clean.hall_quality);
    println!("dream_efficiency:     {:.4}", clean.dream_efficiency);
    println!("speed:                {:.4}", clean.speed);
    println!("consolidation_ms:     {}", clean.consolidation_ms);
    println!("dream_cycles:         {}", params.dream_cycles);
    println!("links_created:        {}", clean.links);
    println!("memories_strengthened: {}", clean.strengthened);
    println!("memories_pruned:      {}", clean.pruned);
    println!("hallucinations:       {}", clean.hallucinations);
    println!("---");
}

// ============================================================================
// LEVEL 4 RETENTION METRICS (cycle L4.4)
// ============================================================================
//
// retention_score and retention_plasticity are a guardrail pair:
//   - retention_score rewards keeping "golden" memories alive and vivid across
//     save→load→dream cycles.
//   - retention_plasticity rewards the surviving memories actually CHANGING
//     during the post-load dream cycle. This exists specifically to block the
//     decay_rate=0 / prune_threshold=0 cheese where the trivial way to win
//     retention is to freeze the whole system.
//
// Both metrics are only meaningful once we have a loaded prior session.
// First-session runs return 1.0 for both (neutral — no baseline to compare).

/// Fraction of golden memories still present AND with amplitude >= 0.5.
/// Returns 1.0 when `session_count < 2` — there is no prior state to retain.
fn eval_retention_score(engine: &ResonanceEngine, header: &StateHeader) -> f32 {
    if header.session_count < 2 {
        return 1.0;
    }
    if header.golden_ids.is_empty() {
        return 1.0;
    }
    let all = engine.store.all_memories().unwrap_or_default();
    let mut survived = 0usize;
    for gid in &header.golden_ids {
        if let Some(m) = all.iter().find(|m| &m.id == gid) {
            if m.amplitude >= 0.5 {
                survived += 1;
            }
        }
    }
    (survived as f32 / header.golden_ids.len() as f32).clamp(0.0, 1.0)
}

/// Measures post-load plasticity on the golden set. Compares amplitudes
/// before and after the dream cycle. Higher drift = more plastic.
/// Returns `(mean_drift * 5.0).min(1.0)`; design-doc rationale: this is a
/// guardrail against decay=0 / prune=0 cheese (see §2 M2b).
fn eval_retention_plasticity(
    engine_before: &ResonanceEngine,
    engine_after: &ResonanceEngine,
    golden_ids: &[uuid::Uuid],
) -> f32 {
    if golden_ids.is_empty() {
        return 1.0;
    }
    let before_all = engine_before.store.all_memories().unwrap_or_default();
    let after_all = engine_after.store.all_memories().unwrap_or_default();
    let mut drift_sum = 0.0f32;
    let mut count = 0usize;
    for gid in golden_ids {
        let a = before_all.iter().find(|m| &m.id == gid);
        let b = after_all.iter().find(|m| &m.id == gid);
        if let (Some(ma), Some(mb)) = (a, b) {
            drift_sum += (mb.amplitude - ma.amplitude).abs();
            count += 1;
        }
    }
    if count == 0 {
        return 1.0;
    }
    let mean_drift = drift_sum / count as f32;
    (mean_drift * 5.0).min(1.0)
}

/// Snapshot just the memories needed to compute retention_plasticity.
/// The chain pass may mutate `engine` in place; this copy gives us a stable
/// "before" baseline without serializing through bincode.
fn snapshot_engine_for_plasticity(engine: &ResonanceEngine) -> ResonanceEngine {
    let mems = engine.store.all_memories().unwrap_or_default();
    let store = Box::new(TestMedium::new());
    // Encoder/codebook are only needed for inserts via pipeline; the harness
    // never re-encodes on a snapshot, so zeros and dim=1 are fine.
    let encoder = Box::new(SimpleHashEncoder::new(1, 0));
    let codebook = Codebook::new(1, 1, 0);
    let pipeline = EncodingPipeline::new(encoder, codebook);
    let mut clone = ResonanceEngine::new(store, pipeline);
    for m in &mems {
        let _ = clone.store.insert((*m).clone());
    }
    clone
}

// ============================================================================
// LEVEL 4 DREAM CHAIN (cycle L4.5)
// ============================================================================
//
// A ChainSeed is the handoff between successive dream cycles: it captures the
// top-N memories (by amplitude) from cycle K along with their xi-signature
// centroid, so cycle K+1 can bias pair selection toward "what K was working
// on". This is implemented as a harness-level wrapper around the existing
// ConsolidationEngine — we build a NEW engine per cycle with a lowered
// interference_threshold, rather than reaching into the library. No core
// library changes.

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ChainSeed {
    carry_ids: Vec<uuid::Uuid>,
    carry_xi_centroid: Vec<f32>,
    round: u32,
}

/// Compute the xi-signature centroid over the top-N memories by amplitude,
/// excluding noise. Falls back to an empty vector if the engine is empty.
fn compute_chain_seed(engine: &ResonanceEngine, top_n: usize, round: u32) -> ChainSeed {
    let all = engine.store.all_memories().unwrap_or_default();
    let mut non_noise: Vec<&HyperMemory> = all
        .iter()
        .filter(|m| !m.content.starts_with("l4_noise") && m.amplitude > 0.01)
        .copied()
        .collect();
    non_noise.sort_by(|a, b| b.amplitude.total_cmp(&a.amplitude));
    let survivors: Vec<&HyperMemory> = non_noise.into_iter().take(top_n.max(1)).collect();

    if survivors.is_empty() {
        return ChainSeed {
            carry_ids: Vec::new(),
            carry_xi_centroid: Vec::new(),
            round,
        };
    }

    // Compute xi-signature for each survivor, then average into a centroid.
    let sig_len = {
        let s = compute_xi_signature(&survivors[0].vector);
        s.len()
    };
    let mut centroid = vec![0.0f32; sig_len];
    let mut count = 0usize;
    for m in &survivors {
        let sig = compute_xi_signature(&m.vector);
        if sig.len() == sig_len {
            for (c, s) in centroid.iter_mut().zip(sig.iter()) {
                *c += *s;
            }
            count += 1;
        }
    }
    if count > 0 {
        let inv = 1.0 / count as f32;
        for c in centroid.iter_mut() {
            *c *= inv;
        }
    }
    ChainSeed {
        carry_ids: survivors.iter().map(|m| m.id).collect(),
        carry_xi_centroid: centroid,
        round,
    }
}

/// Cosine similarity on two equal-length real vectors. Falls back to 0 on
/// length mismatch or zero norm.
fn vec_cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na < 1e-8 || nb < 1e-8 {
        return 0.0;
    }
    (dot / (na * nb)).clamp(-1.0, 1.0)
}

/// chain_fidelity: how coherently the xi-centroid refines across the chain,
/// plus a small monotonicity bonus for non-decreasing Φ.
///
///   base_score = 1 - mean(cosine_distance(centroid_K, centroid_{K+1}))
///   monotonicity_bonus = 0.1 * (phi_increases / (chain_depth - 1))
///   return (base_score + bonus).clamp(0, 1)
///
/// With a single-cycle chain, returns 1.0 (no transitions to measure).
fn eval_chain_fidelity(chain_seeds: &[ChainSeed], phi_history: &[f32]) -> f32 {
    if chain_seeds.len() < 2 {
        return 1.0;
    }
    let mut dist_sum = 0.0f32;
    let mut pairs = 0usize;
    for w in chain_seeds.windows(2) {
        let sim = vec_cosine(&w[0].carry_xi_centroid, &w[1].carry_xi_centroid);
        // cosine_distance = 1 - sim (signs can go negative; clamp to [0, 2])
        dist_sum += (1.0 - sim).clamp(0.0, 2.0);
        pairs += 1;
    }
    let mean_dist = if pairs > 0 { dist_sum / pairs as f32 } else { 0.0 };
    let base_score = (1.0 - mean_dist).clamp(0.0, 1.0);

    // Monotonicity: count the number of transitions where phi did not decrease.
    let mut increases = 0usize;
    let mut denom = 0usize;
    if phi_history.len() >= 2 {
        for w in phi_history.windows(2) {
            if w[1] >= w[0] {
                increases += 1;
            }
            denom += 1;
        }
    }
    let bonus = if denom > 0 {
        0.1 * (increases as f32 / denom as f32)
    } else {
        0.0
    };

    (base_score + bonus).clamp(0.0, 1.0)
}

/// Aggregate totals reported by a dream chain execution. Mirrors the per-cycle
/// counters run_experiment_l4_session previously accumulated in a plain loop.
#[derive(Debug, Default, Clone, Copy)]
struct ChainTotals {
    strengthened: usize,
    pruned: usize,
    links: usize,
    hallucinations: usize,
}

/// Run a `chain_depth`-cycle dream chain on `engine`, biasing each cycle's
/// pair selection toward the previous cycle's xi centroid by lowering the
/// effective interference_threshold by `chain_carry_strength * threshold`.
///
/// Returns the per-cycle xi centroids (as `ChainSeed` entries) and the
/// per-cycle Φ history, plus aggregated dream counters.
fn run_dream_chain(
    params: &Params,
    engine: &mut ResonanceEngine,
) -> (Vec<ChainSeed>, Vec<f32>, ChainTotals) {
    let depth = params.chain_depth.max(1);
    let mut chain_seeds: Vec<ChainSeed> = Vec::with_capacity(depth);
    let mut phi_history: Vec<f32> = Vec::with_capacity(depth);
    let mut totals = ChainTotals::default();
    let bridge = ConsciousnessBridge::new(0.3, 0.5);

    for cycle_idx in 0..depth {
        // Cycle 1 uses the nominal threshold; later cycles lower it by the
        // carry_strength fraction so the consolidator is biased toward more
        // aggressive pair selection on the "carried" memories. We drop the
        // threshold globally because the ConsolidationEngine library doesn't
        // expose a per-memory threshold hook, and modifying core just to
        // shave a few points of fidelity isn't worth the surface area.
        let threshold_scale = if cycle_idx == 0 {
            1.0
        } else {
            (1.0 - params.chain_carry_strength).max(0.0)
        };
        let effective_threshold = params.interference_threshold * threshold_scale;

        let consolidator = ConsolidationEngine {
            interference_threshold: effective_threshold,
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

        let report = consolidator.consolidate(engine, 0, 2);
        totals.strengthened += report.memories_strengthened;
        totals.pruned += report.memories_pruned;
        totals.links += report.skip_links_created;
        totals.hallucinations += report.hallucinations_created;

        let seed = compute_chain_seed(engine, params.chain_top_n, (cycle_idx + 1) as u32);
        chain_seeds.push(seed);
        let phi = bridge.assess(engine).phi as f32;
        phi_history.push(phi);
    }

    (chain_seeds, phi_history, totals)
}

// ============================================================================
// LEVEL 4 ENCODING ENTROPY + CORPUS XI DIVERSITY (cycle L4.7)
// ============================================================================
//
// encoding_entropy penalizes representational collapse: Shannon entropy over
// quantized xi-signature bins. High entropy = diverse representations. Low
// entropy = the encoder is mapping everything into a handful of manifolds.
//
// corpus_xi_diversity is a recalibrated version of L3's eval_xi_diversity
// that normalizes against 0.08 instead of 0.05, so the L4 corpus doesn't
// trivially saturate at 1.0.

/// Shannon entropy of quantized xi-signature bin tuples, normalized to [0,1].
/// Operates on HyperMemory slices so it can score either the surviving store
/// or any arbitrary corpus snapshot.
fn eval_encoding_entropy(corpus: &[HyperMemory], bins: usize) -> f32 {
    if corpus.is_empty() || bins < 2 {
        return 0.0;
    }

    // Collect xi signatures (compute any that aren't already populated).
    let sigs: Vec<Vec<f32>> = corpus
        .iter()
        .filter(|m| !m.content.starts_with("l4_noise") && m.amplitude > 0.01)
        .map(|m| {
            if m.xi_signature.is_empty() {
                compute_xi_signature(&m.vector)
            } else {
                m.xi_signature.clone()
            }
        })
        .filter(|s| !s.is_empty())
        .collect();

    if sigs.is_empty() {
        return 0.0;
    }

    // Find per-dimension range to normalize quantization. Using a global
    // [-1, 1] assumption is brittle because xi signatures can go outside that
    // interval, so we derive (min, max) from the corpus itself.
    let dim = sigs[0].len();
    let mut mins = vec![f32::INFINITY; dim];
    let mut maxs = vec![f32::NEG_INFINITY; dim];
    for s in &sigs {
        if s.len() != dim { continue; }
        for (d, v) in s.iter().enumerate() {
            if *v < mins[d] { mins[d] = *v; }
            if *v > maxs[d] { maxs[d] = *v; }
        }
    }

    // Quantize each signature into a bin-tuple; count occurrences.
    use std::collections::HashMap;
    let mut counts: HashMap<Vec<u16>, u32> = HashMap::new();
    for s in &sigs {
        if s.len() != dim { continue; }
        let mut tuple: Vec<u16> = Vec::with_capacity(dim);
        for (d, v) in s.iter().enumerate() {
            let span = (maxs[d] - mins[d]).max(1e-8);
            let rel = ((*v - mins[d]) / span).clamp(0.0, 1.0);
            let bin = (rel * (bins as f32 - 1.0)).round() as u16;
            tuple.push(bin);
        }
        *counts.entry(tuple).or_insert(0) += 1;
    }

    let total: f32 = counts.values().map(|c| *c as f32).sum();
    if total <= 0.0 {
        return 0.0;
    }
    let mut h = 0.0f32;
    for c in counts.values() {
        let p = (*c as f32) / total;
        if p > 0.0 {
            h -= p * p.log2();
        }
    }

    // Normalize by the theoretical maximum on the joint distribution of
    // bin-tuples across `dim` dimensions: log2(bins^dim).
    let max_h = (bins as f32).log2() * (dim as f32);
    if max_h <= 0.0 {
        return 0.0;
    }
    (h / max_h).clamp(0.0, 1.0)
}

/// L4 corpus xi-diversity: same pairwise boost-averaging idea as the L3
/// metric, but normalizes against 0.08 instead of 0.05 (the L3 metric
/// saturates on the L4 corpus, which defeats its purpose).
fn eval_corpus_xi_diversity(corpus: &[HyperMemory]) -> f32 {
    let mut active: Vec<&HyperMemory> = corpus
        .iter()
        .filter(|m| !m.content.starts_with("l4_noise") && m.amplitude > 0.01)
        .collect();
    active.sort_by(|a, b| a.content.cmp(&b.content));

    if active.len() < 4 {
        return 0.0;
    }

    // Cycle L4.7.5: sample EVENLY across the sorted active set so the
    // ~300 pairs span all clusters, not just the alphabetically-first one
    // (the previous min(30) head-slice grabbed only "dense_a *" items, which
    // share a centroid and produce an inflated avg_boost).
    let target_n = 25usize; // 25 → 300 pairs, well above the 100 floor.
    let sample_size = active.len().min(target_n);
    let stride = (active.len() / sample_size).max(1);
    let sampled: Vec<&HyperMemory> = (0..sample_size)
        .map(|k| active[(k * stride).min(active.len() - 1)])
        .collect();
    let mut total_boost = 0.0f32;
    let mut count = 0usize;
    for i in 0..sampled.len() {
        for j in (i + 1)..sampled.len() {
            let xi_a = compute_xi_signature(&sampled[i].vector);
            let xi_b = compute_xi_signature(&sampled[j].vector);
            let base_sim = cosine_similarity(&sampled[i].vector, &sampled[j].vector);
            let boosted = xi_diversity_boost(base_sim, &xi_a, &xi_b);
            total_boost += (boosted - base_sim).abs();
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    let avg_boost = total_boost / count as f32;
    // Cycle L4.7.5: renormalize from 0.08 to 0.12 (combined with the
    // even-stride sampling above). The previous target+head-slice combo made
    // L4 saturate at 1.0; with stride sampling the raw avg_boost on a fresh
    // L4 corpus is ~0.080, so dividing by 0.12 lands the baseline around 0.67
    // — discriminating without being trivially saturated.
    (avg_boost / 0.12).clamp(0.0, 1.0)
}

/// L4-aware intra-cluster phase coherence. Partitions surviving memories by
/// their L4 cluster prefix (dense_a..dense_d, sparse_e, sparse_f), computes
/// the Kuramoto order parameter R = |1/N · Σ e^{iφ}| within each cluster, and
/// averages across clusters. Bridges, decoys, noise, and adversaries are not
/// part of any cluster and are excluded from the partition.
fn eval_phase_coherence_l4(engine: &ResonanceEngine) -> f32 {
    let all = engine.store.all_memories().unwrap_or_default();
    let cluster_prefixes = [
        "dense_a", "dense_b", "dense_c", "dense_d", "sparse_e", "sparse_f",
    ];
    let mut total_coherence = 0.0f32;
    let mut cluster_count = 0;
    for prefix in &cluster_prefixes {
        let phases: Vec<f32> = all
            .iter()
            .filter(|m| m.content.starts_with(prefix) && m.amplitude > 0.01)
            .map(|m| m.phase)
            .collect();
        if phases.len() < 2 {
            continue;
        }
        let n = phases.len() as f32;
        let sum_cos: f32 = phases.iter().map(|p| p.cos()).sum();
        let sum_sin: f32 = phases.iter().map(|p| p.sin()).sum();
        let r = ((sum_cos / n).powi(2) + (sum_sin / n).powi(2)).sqrt();
        total_coherence += r;
        cluster_count += 1;
    }
    if cluster_count == 0 {
        return 0.0;
    }
    total_coherence / cluster_count as f32
}

/// L4-aware cluster separation. For each pair of L4 clusters, computes the
/// cosine distance (1 - cos_sim) between their centroids, then returns the
/// mean cosine distance clamped to [0, 1]. Higher = clusters more separated.
/// Bridges, decoys, noise, and adversaries are excluded from the partition.
fn eval_cluster_separation_l4(engine: &ResonanceEngine) -> f32 {
    let all = engine.store.all_memories().unwrap_or_default();
    let cluster_prefixes = [
        "dense_a", "dense_b", "dense_c", "dense_d", "sparse_e", "sparse_f",
    ];

    // Compute centroid per cluster.
    let mut centroids: Vec<Vec<f32>> = Vec::new();
    for prefix in &cluster_prefixes {
        let members: Vec<&Vec<f32>> = all
            .iter()
            .filter(|m| m.content.starts_with(prefix) && m.amplitude > 0.01)
            .map(|m| &m.vector)
            .collect();
        if members.len() < 2 {
            continue;
        }
        let dim = members[0].len();
        let mut c = vec![0.0f32; dim];
        for v in &members {
            for (s, x) in c.iter_mut().zip(v.iter()) {
                *s += *x;
            }
        }
        let inv = 1.0 / members.len() as f32;
        for s in c.iter_mut() {
            *s *= inv;
        }
        centroids.push(c);
    }

    if centroids.len() < 2 {
        return 0.0;
    }

    // Mean pairwise cosine distance (1 - cos_sim) across centroids.
    let mut sum = 0.0f32;
    let mut count = 0usize;
    for i in 0..centroids.len() {
        for j in (i + 1)..centroids.len() {
            let sim = cosine_similarity(&centroids[i], &centroids[j]);
            let dist = 1.0 - sim;
            sum += dist;
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    (sum / count as f32).clamp(0.0, 1.0)
}

/// L4 variant: filters by the "l4_noise" tag instead of L3's "noise" prefix.
fn eval_l4_noise_removal(engine: &ResonanceEngine) -> f32 {
    let all = engine.store.all_memories().unwrap_or_default();
    let surviving_noise = all.iter()
        .filter(|m| m.content.starts_with("l4_noise") && m.amplitude > 0.01)
        .count();
    1.0 - (surviving_noise as f32 / 15.0)
}

/// L4 variant: counts surviving non-noise memories out of 285 (300 - 15 noise).
fn eval_l4_signal_preservation(engine: &ResonanceEngine) -> f32 {
    let all = engine.store.all_memories().unwrap_or_default();
    let signal_count = all.iter().filter(|m| {
        !m.content.starts_with("l4_noise") && m.amplitude > 0.01
    }).count();
    (signal_count as f32 / 285.0).min(1.0)
}

fn parse_string_flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn parse_usize_flag(args: &[String], name: &str) -> Option<usize> {
    parse_string_flag(args, name).and_then(|s| s.parse::<usize>().ok())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let level = parse_usize_flag(&args, "--level").unwrap_or(2);

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

    let cli = L4Cli {
        load_path: parse_string_flag(&args, "--load").map(PathBuf::from),
        save_path: parse_string_flag(&args, "--save").map(PathBuf::from),
        chain_sessions: parse_usize_flag(&args, "--chain-sessions").unwrap_or(0),
    };

    match level {
        4 => {
            if cli.chain_sessions > 0 {
                // Internal loop: N sessions in one cargo run. Session 1 loads
                // from `--load` if it exists, otherwise starts fresh. Each
                // session writes its state to `--save` so the next one can
                // pick it back up. `--save` is required for chaining.
                if cli.save_path.is_none() {
                    eprintln!("--chain-sessions requires --save <path>");
                    std::process::exit(2);
                }
                for session_idx in 0..cli.chain_sessions {
                    println!("=== L4 chain session {}/{} ===", session_idx + 1, cli.chain_sessions);
                    // After session 1, load from the file we just wrote.
                    let load_path = if session_idx == 0 {
                        cli.load_path.clone()
                    } else {
                        cli.save_path.clone()
                    };
                    let session_cli = L4Cli {
                        load_path,
                        save_path: cli.save_path.clone(),
                        chain_sessions: 0,
                    };
                    run_experiment_l4_session(&params, &session_cli);
                }
            } else {
                run_experiment_l4_session(&params, &cli);
            }
        }
        3 => run_experiment_l3(&params),
        _ => run_experiment(&params),
    }
}
