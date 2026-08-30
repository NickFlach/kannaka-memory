//! L4.S0 data exporter: dump (cosine_sim, xi_repulsion, current xi_diversity_boost)
//! triples for 300 uniformly sampled pairs from the L4 corpus.
//!
//! This binary is a throwaway used by experiments/scripts/eml_train_xi.py. It
//! re-implements `build_corpus_l4` inline (copied from src/bin/research.rs) to
//! avoid touching research.rs, which is being edited concurrently by a parallel
//! agent.

use std::f32::consts::PI;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;

use kannaka_memory::wave::cosine_similarity;
use kannaka_memory::xi_operator::{
    compute_xi_signature, xi_diversity_boost, xi_repulsive_force,
};

// ---------------- copied from research.rs (do not edit research.rs) ----------------

fn pcg_mix(seed: u64, stream: u64) -> u64 {
    let mut x = seed.wrapping_add(stream.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    x
}

fn pcg_f32(seed: u64, cluster: u32, item: u32, dim_id: u32) -> f32 {
    let stream = ((cluster as u64) << 40) | ((item as u64) << 20) | (dim_id as u64);
    let bits = pcg_mix(seed, stream);
    let norm = ((bits >> 40) as f32) / ((1u64 << 24) as f32);
    norm * 2.0 - 1.0
}

fn build_corpus_l4(dim: usize, _hardness: usize, encoder_seed: u64) -> Vec<(Vec<f32>, String, &'static str)> {
    let mut corpus: Vec<(Vec<f32>, String, &'static str)> = Vec::with_capacity(300);

    let dense_labels = ["dense_a", "dense_b", "dense_c", "dense_d"];
    for (cluster_idx, label) in dense_labels.iter().enumerate() {
        let cid = cluster_idx as u32;
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
            corpus.push((v, format!("{label} {i}"), "l4_dense"));
        }
    }

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
            corpus.push((v, format!("{label} {i}"), "l4_sparse"));
        }
    }

    let pairs: [(u32, u32); 5] = [(0, 1), (0, 2), (1, 3), (2, 4), (3, 5)];
    for (pair_idx, (a, b)) in pairs.iter().enumerate() {
        let centroid_a: Vec<f32> = (0..dim)
            .map(|d| 0.8 * pcg_f32(encoder_seed, *a, 0, d as u32).signum())
            .collect();
        let centroid_b: Vec<f32> = (0..dim)
            .map(|d| 0.8 * pcg_f32(encoder_seed, *b, 0, d as u32).signum())
            .collect();
        for i in 0..4 {
            let stream_cid = 1000 + pair_idx as u32;
            let item = (i + 1) as u32;
            let v: Vec<f32> = (0..dim)
                .map(|d| {
                    let mix = 0.5 * (centroid_a[d] + centroid_b[d]);
                    let jitter = pcg_f32(encoder_seed, stream_cid, item, d as u32) * 0.12;
                    mix + jitter
                })
                .collect();
            corpus.push((v, format!("l4_bridge p{pair_idx} {i}"), "l4_bridge"));
        }
    }

    for i in 0..25 {
        let item = (i + 1) as u32;
        let v: Vec<f32> = (0..dim)
            .map(|d| pcg_f32(encoder_seed, 2000, item, d as u32) * 0.9)
            .collect();
        corpus.push((v, format!("l4_decoy {i}"), "l4_decoy"));
    }

    for i in 0..15 {
        let item = (i + 1) as u32;
        let v: Vec<f32> = (0..dim)
            .map(|d| pcg_f32(encoder_seed, 3000, item, d as u32) * 0.12)
            .collect();
        corpus.push((v, format!("l4_noise {i}"), "l4_noise"));
    }

    debug_assert_eq!(corpus.len(), 300, "L4 corpus must be exactly 300 memories");
    corpus
}

// ---------------- /copy ----------------

#[derive(Serialize)]
struct PairRecord {
    sim: f32,
    repulsion: f32,
    current_boost: f32,
}

fn main() {
    let dim = 128usize;
    let encoder_seed: u64 = 0xCAFE_BABE;
    let corpus = build_corpus_l4(dim, 1, encoder_seed);
    assert_eq!(corpus.len(), 300);

    // Precompute xi signatures for each memory.
    let xi_sigs: Vec<Vec<f32>> = corpus
        .iter()
        .map(|(v, _, _)| compute_xi_signature(v))
        .collect();

    // Sample 300 uniform random pairs (i != j). Deterministic seed for reproducibility.
    let mut rng = StdRng::seed_from_u64(0xF00D_FACE);
    let n = corpus.len();
    let mut records: Vec<PairRecord> = Vec::with_capacity(300);
    while records.len() < 300 {
        let i = rng.gen_range(0..n);
        let j = rng.gen_range(0..n);
        if i == j {
            continue;
        }
        let sim = cosine_similarity(&corpus[i].0, &corpus[j].0);
        let repulsion = xi_repulsive_force(&xi_sigs[i], &xi_sigs[j]);
        let current_boost = xi_diversity_boost(sim, &xi_sigs[i], &xi_sigs[j]);
        records.push(PairRecord {
            sim,
            repulsion,
            current_boost,
        });
    }

    // Resolve output path relative to the crate root (cargo sets CARGO_MANIFEST_DIR).
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("experiments");
    std::fs::create_dir_all(&out_dir).expect("create experiments dir");
    let out_path = out_dir.join("xi_pairs.json");
    let json = serde_json::to_string_pretty(&records).expect("serialize");
    let mut f = File::create(&out_path).expect("create xi_pairs.json");
    f.write_all(json.as_bytes()).expect("write");

    // Quick stats for operator feedback.
    let n_f = records.len() as f32;
    let mean_sim = records.iter().map(|r| r.sim).sum::<f32>() / n_f;
    let mean_rep = records.iter().map(|r| r.repulsion).sum::<f32>() / n_f;
    let mean_boost = records.iter().map(|r| r.current_boost).sum::<f32>() / n_f;
    let var_boost = records
        .iter()
        .map(|r| (r.current_boost - mean_boost).powi(2))
        .sum::<f32>()
        / n_f;
    println!(
        "wrote {} pairs -> {}",
        records.len(),
        out_path.display()
    );
    println!(
        "mean sim={mean_sim:.4} mean rep={mean_rep:.4} mean boost={mean_boost:.4} var boost={var_boost:.6}"
    );
}
