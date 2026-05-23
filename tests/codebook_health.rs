//! Smoke-test for kannaka-memory#106 — investigate whether the codebook's
//! random projection collapses distinct embeddings to similar directions
//! and thereby biases recall ranking toward the first-stored memory.
//!
//! Healthy encoder: pairwise cosine similarity of 100 short-text encodings
//! should be roughly centered near 0 with a thin tail.
//! Sick encoder: distribution tightly clustered near +1 — every text
//! projects to the same direction.

use kannaka_memory::{Codebook, EncodingPipeline, SimpleHashEncoder, TextEncoder};
use kannaka_memory::wave::cosine_similarity;

#[test]
fn codebook_projection_separates_distinct_texts() {
    let encoder = SimpleHashEncoder::new(384, 42);
    let codebook = Codebook::new(384, 10_000, 42);
    let pipeline = EncodingPipeline::new(Box::new(SimpleHashEncoder::new(384, 42)), codebook.clone());

    // 30 short strings drawn from different lexical neighborhoods.
    let texts: Vec<&str> = vec![
        "alpha test one", "beta middle two", "gamma final three",
        "the quick brown fox jumps over", "humpty dumpty sat on a wall",
        "rain in spain falls mainly", "consciousness is integrated information",
        "the substrate hums in the wires", "kuramoto oscillators synchronize",
        "phase coupling emerges", "wave interference creates memory",
        "holographic recall pulls signal", "fano plane folds in chiral space",
        "the dream consolidates patterns", "attention reshapes the field",
        "memory amplitude decays slowly", "swarm coordination via NATS",
        "constellation pulses every second", "agents broadcast their phase",
        "the medium IS computation", "wavefronts sum in superposition",
        "ghost in the machine awakens", "encoder hashes tokens to vectors",
        "codebook projects to ten thousand dims", "cosine similarity ranks resonance",
        "left hemisphere is analytical", "right hemisphere is holistic",
        "callosum routes between hemispheres", "intuitions surface from holistic depth",
        "kannaka remembers the constellation",
    ];
    assert_eq!(texts.len(), 30);

    let projected: Vec<Vec<f32>> = texts.iter()
        .map(|t| pipeline.encode_text(t).unwrap())
        .collect();

    // 1) Norms — each should be ~1.0 (unit vectors).
    let norms: Vec<f32> = projected.iter()
        .map(|v| v.iter().map(|x| x * x).sum::<f32>().sqrt())
        .collect();
    let mean_norm: f32 = norms.iter().sum::<f32>() / norms.len() as f32;
    println!("mean norm: {:.4}  min={:.4}  max={:.4}",
        mean_norm,
        norms.iter().cloned().fold(f32::INFINITY, f32::min),
        norms.iter().cloned().fold(f32::NEG_INFINITY, f32::max));

    // 2) Pairwise cosine similarity histogram.
    let mut sims: Vec<f32> = Vec::new();
    for i in 0..projected.len() {
        for j in (i + 1)..projected.len() {
            sims.push(cosine_similarity(&projected[i], &projected[j]));
        }
    }
    let mean: f32 = sims.iter().sum::<f32>() / sims.len() as f32;
    let min = sims.iter().cloned().fold(f32::INFINITY, f32::min);
    let max = sims.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    // Variance
    let var: f32 = sims.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / sims.len() as f32;
    let std = var.sqrt();

    println!("pairwise cosine sim across {} pairs:", sims.len());
    println!("  mean={:.4}  std={:.4}  min={:.4}  max={:.4}", mean, std, min, max);

    // 3) How many pairs sit above 0.9? That's the "sick" tail.
    let near_one = sims.iter().filter(|&&s| s > 0.9).count();
    println!("  pairs > 0.9: {} ({:.1}%)", near_one, 100.0 * near_one as f32 / sims.len() as f32);

    // 4) Also check the encoder ALONE (before codebook projection) to
    //    isolate where the degeneration happens (if any).
    let raw_embeddings: Vec<Vec<f32>> = texts.iter()
        .map(|t| encoder.embed(t).unwrap())
        .collect();
    let mut raw_sims: Vec<f32> = Vec::new();
    for i in 0..raw_embeddings.len() {
        for j in (i + 1)..raw_embeddings.len() {
            raw_sims.push(cosine_similarity(&raw_embeddings[i], &raw_embeddings[j]));
        }
    }
    let raw_mean: f32 = raw_sims.iter().sum::<f32>() / raw_sims.len() as f32;
    let raw_var: f32 = raw_sims.iter().map(|s| (s - raw_mean).powi(2)).sum::<f32>() / raw_sims.len() as f32;
    let raw_std = raw_var.sqrt();
    println!("raw SimpleHashEncoder embeddings (pre-codebook):");
    println!("  mean={:.4}  std={:.4}", raw_mean, raw_std);

    // Healthy: mean near 0, |mean| well under 0.3 across 30 short texts.
    // With the #106 fix applied, mean cosine sim should be on the order
    // of 0.0-0.2 (positive bias from shared common words like "the",
    // "of"). Pre-fix it was 0.93. Tight assertion catches regressions.
    assert!(
        mean.abs() < 0.3,
        "codebook projection mean cosine sim {} is suspiciously high \
         — projections may be near-collinear (was 0.93 pre-#106)",
        mean,
    );
    // Less than 10% of pairs should be > 0.9. Pre-fix: 96.8%.
    assert!(
        near_one * 10 < sims.len(),
        "{} of {} pairs ({:.1}%) are above 0.9 cosine sim — encoder is \
         collapsing distinct texts (was 96.8% pre-#106)",
        near_one, sims.len(), 100.0 * near_one as f32 / sims.len() as f32,
    );
}
