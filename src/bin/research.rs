//! kannaka-research — autonomous memory system benchmarking
//!
//! Run: cargo run --bin research
//!
//! The agent modifies `research/params.toml` or the ExperimentParams below.
//! This binary evaluates the parameters against a fixed test corpus.

use std::f32::consts::PI;
use std::time::Instant;

use kannaka_memory::codebook::Codebook;
use kannaka_memory::consolidation::ConsolidationEngine;
use kannaka_memory::encoding::{EncodingPipeline, SimpleHashEncoder};
use kannaka_memory::kuramoto::KuramotoSync;
use kannaka_memory::memory::HyperMemory;
use kannaka_memory::store::{InMemoryStore, MemoryEngine};

// ============================================================================
// EXPERIMENT PARAMETERS — THIS IS WHAT THE AGENT MODIFIES
// ============================================================================

fn experiment_params() -> Params {
    Params {
        // Wave dynamics
        decay_rate: 1e-6,
        default_frequency: 0.1,

        // Consolidation (dream)
        interference_threshold: 0.1,
        phase_alignment_threshold: PI / 4.0,
        prune_threshold: 0.5,
        constructive_boost: 0.4,
        destructive_penalty: 0.4,

        // Kuramoto synchronization
        kuramoto_coupling: 0.6,
        kuramoto_dt: 0.1,
        kuramoto_steps: 10,
        kuramoto_threshold: 0.5,
    }
}

// ============================================================================
// Parameter struct (agent edits the values above, not this struct)
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

    // Cluster 3: Personal (15 memories, sparse)
    for i in 0..15 {
        let v: Vec<f32> = (0..dim).map(|j| {
            ((i * 7 + j * 13) as f32 * 0.37).sin() * 0.6
        }).collect();
        corpus.push((v, format!("personal memory {}", i), "personal"));
    }

    // Noise (10 memories, low amplitude — should be pruned)
    for i in 0..10 {
        let v: Vec<f32> = (0..dim).map(|j| {
            ((i * 31 + j * 97) as f32 * 1.7).sin() * 0.1
        }).collect();
        corpus.push((v, format!("noise {}", i), "noise"));
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

fn eval_recall(engine: &MemoryEngine, corpus: &[(Vec<f32>, String, &str)]) -> f32 {
    let mut hits = 0usize;
    let mut total = 0usize;

    for cluster_name in &["science", "music", "personal"] {
        let cluster_vecs: Vec<&Vec<f32>> = corpus.iter()
            .filter(|(_, _, c)| c == cluster_name)
            .map(|(v, _, _)| v)
            .collect();

        if cluster_vecs.is_empty() { continue; }

        let results = engine.store.search(cluster_vecs[0], 10).unwrap_or_default();
        for (id, _sim) in &results {
            if let Ok(Some(mem)) = engine.store.get(id) {
                if mem.content.contains(cluster_name) {
                    hits += 1;
                }
            }
            total += 1;
        }
    }

    if total == 0 { return 0.0; }
    hits as f32 / total as f32
}

fn run_experiment(params: &Params) {
    let dim = 64;
    let corpus = build_corpus(dim);

    // Build engine and store corpus
    let store = Box::new(InMemoryStore::new());
    let encoder = Box::new(SimpleHashEncoder::new(dim, 42));
    let codebook = Codebook::new(dim, dim, 42);
    let pipeline = EncodingPipeline::new(encoder, codebook);
    let mut engine = MemoryEngine::new(store, pipeline);
    for (i, (vec, content, category)) in corpus.iter().enumerate() {
        let mut mem = HyperMemory::new(vec.clone(), content.clone());
        // Assign phases by cluster so interference classification works
        mem.phase = match *category {
            "science" => 0.0 + (i as f32 * 0.1),           // ~aligned
            "music" => PI * 0.5 + (i as f32 * 0.08),       // different phase band
            "personal" => PI * 0.3 * (i as f32 % 4.0),     // scattered
            "noise" => PI * (i as f32 * 0.7),               // random-ish
            "bridge" => PI * 0.25,                           // between clusters
            _ => 0.0,
        };
        // Noise starts at low amplitude (should be prunable)
        if *category == "noise" {
            mem.amplitude = 0.15;
        }
        engine.store.insert(mem).expect("insert failed");
    }

    let pre_count = engine.store.count();

    // Build consolidation engine from params
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
    };

    // Run consolidation
    let start = Instant::now();
    let report = consolidator.consolidate(&mut engine, 0, 2);
    let consolidation_ms = start.elapsed().as_millis() as u64;

    let post_count = engine.store.count();

    // Recall after dreaming
    let recall_precision = eval_recall(&engine, &corpus);
    let recall_miss = 1.0 - recall_precision;

    // Dream quality score
    let dream_waste = 1.0
        - (report.memories_strengthened as f32 / 70.0).min(1.0) * 0.5
        - (report.memories_pruned as f32 / 10.0).min(1.0) * 0.3
        - (report.skip_links_created as f32 / 20.0).min(1.0) * 0.2;

    // Composite fitness (LOWER IS BETTER)
    let fitness = 0.40 * recall_miss
        + 0.30 * dream_waste.max(0.0)
        + 0.20 * (consolidation_ms as f32 / 5000.0).min(1.0)
        + 0.10 * (1.0 / (report.skip_links_created as f32 + 1.0));

    // Print results in grep-friendly format
    println!("---");
    println!("fitness:              {:.6}", fitness);
    println!("recall_precision:     {:.4}", recall_precision);
    println!("recall_miss:          {:.4}", recall_miss);
    println!("dream_waste:          {:.4}", dream_waste);
    println!("consolidation_ms:     {}", consolidation_ms);
    println!("links_created:        {}", report.skip_links_created);
    println!("memories_strengthened: {}", report.memories_strengthened);
    println!("memories_pruned:      {}", report.memories_pruned);
    println!("clusters_synced:      {}", report.clusters_synced);
    println!("hallucinations:       {}", report.hallucinations_created);
    println!("pre_count:            {}", pre_count);
    println!("post_count:           {}", post_count);
    println!("---");
}

fn main() {
    let params = experiment_params();
    run_experiment(&params);
}
