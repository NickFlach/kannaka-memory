//! Integration tests for the audio perception module.

#![cfg(feature = "audio")]

use std::f32::consts::PI;

use kannaka_memory::codebook::Codebook;
use kannaka_memory::ear::{AudioPipeline, AUDIO_CODEBOOK_SEED, AUDIO_FEATURE_DIM, HYPERVECTOR_DIM, SAMPLE_RATE};
use kannaka_memory::wave::cosine_similarity;

fn sine_wave(freq: f32, duration_secs: f32, amplitude: f32) -> Vec<f32> {
    let n = (SAMPLE_RATE as f32 * duration_secs) as usize;
    (0..n)
        .map(|i| amplitude * (2.0 * PI * freq * i as f32 / SAMPLE_RATE as f32).sin())
        .collect()
}

fn white_noise(duration_secs: f32) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let n = (SAMPLE_RATE as f32 * duration_secs) as usize;
    (0..n)
        .map(|i| {
            let mut h = DefaultHasher::new();
            i.hash(&mut h);
            (h.finish() as f32 / u64::MAX as f32) * 2.0 - 1.0
        })
        .collect()
}

#[test]
fn same_sound_is_deterministic() {
    let pipeline = AudioPipeline::new();
    let tone = sine_wave(440.0, 2.0, 0.5);
    let (mem1, _) = pipeline.encode_samples(&tone, "A440").unwrap();
    let (mem2, _) = pipeline.encode_samples(&tone, "A440").unwrap();
    let sim = cosine_similarity(&mem1.vector, &mem2.vector);
    assert!(
        (sim - 1.0).abs() < 1e-4,
        "same input should produce identical vectors, sim={}",
        sim
    );
}

#[test]
fn different_frequencies_produce_different_vectors() {
    let pipeline = AudioPipeline::new();
    let low = sine_wave(200.0, 2.0, 0.5);
    let high = sine_wave(4000.0, 2.0, 0.5);
    let (mem_low, _) = pipeline.encode_samples(&low, "200Hz").unwrap();
    let (mem_high, _) = pipeline.encode_samples(&high, "4kHz").unwrap();
    let sim = cosine_similarity(&mem_low.vector, &mem_high.vector);
    // Pure sine waves differ mainly in which mel band is active;
    // similarity can be high because most features (ZCR, RMS, etc.) are similar.
    // The important thing is they're not identical.
    assert!(
        sim < 0.98,
        "different frequencies should produce distinct vectors, sim={}",
        sim
    );
}

#[test]
fn sine_vs_noise_are_distinct() {
    let pipeline = AudioPipeline::new();
    let tone = sine_wave(440.0, 2.0, 0.5);
    let noise = white_noise(2.0);
    let (mem_tone, _) = pipeline.encode_samples(&tone, "A440").unwrap();
    let (mem_noise, _) = pipeline.encode_samples(&noise, "noise").unwrap();
    let sim = cosine_similarity(&mem_tone.vector, &mem_noise.vector);
    assert!(
        sim < 0.7,
        "tone and noise should be distinct, sim={}",
        sim
    );
}

#[test]
fn audio_memory_has_correct_wave_params() {
    let pipeline = AudioPipeline::new();
    let tone = sine_wave(440.0, 1.0, 0.5);
    let (mem, _) = pipeline.encode_samples(&tone, "test").unwrap();
    assert_eq!(mem.frequency, 0.05);
    assert!((mem.phase - PI / 4.0).abs() < 1e-6);
    assert_eq!(mem.decay_rate, 5e-7);
}

#[test]
fn audio_has_xi_signature() {
    let pipeline = AudioPipeline::new();
    let tone = sine_wave(440.0, 2.0, 0.5);
    let (mem, _) = pipeline.encode_samples(&tone, "A440").unwrap();
    assert!(!mem.xi_signature.is_empty(), "audio memories should have Xi signatures");
}

/// The critical orthogonality test: audio and text hypervectors must be
/// nearly orthogonal because they use different codebooks with different seeds.
#[test]
fn audio_orthogonal_to_text() {
    let audio_pipeline = AudioPipeline::new();
    let text_codebook = Codebook::new(384, HYPERVECTOR_DIM, 42);

    let tone = sine_wave(440.0, 2.0, 0.5);
    let (audio_mem, _) = audio_pipeline.encode_samples(&tone, "A440").unwrap();

    // Project a representative text-like embedding
    let text_embedding = vec![0.5f32; 384];
    let text_hv = text_codebook.project(&text_embedding);

    let sim = cosine_similarity(&audio_mem.vector, &text_hv);
    assert!(
        sim.abs() < 0.15,
        "audio and text vectors should be nearly orthogonal, sim={}",
        sim
    );
}

/// Test with multiple text embeddings to be thorough.
#[test]
fn audio_orthogonal_to_multiple_texts() {
    let audio_pipeline = AudioPipeline::new();
    let text_codebook = Codebook::new(384, HYPERVECTOR_DIM, 42);

    let sounds = [
        sine_wave(200.0, 2.0, 0.5),
        sine_wave(1000.0, 2.0, 0.3),
        white_noise(2.0),
    ];

    let texts: Vec<Vec<f32>> = (0..5)
        .map(|i| {
            let mut v = vec![0.0f32; 384];
            v[i * 50] = 1.0;
            v
        })
        .collect();

    for sound in &sounds {
        let (audio_mem, _) = audio_pipeline.encode_samples(sound, "test").unwrap();
        for text_emb in &texts {
            let text_hv = text_codebook.project(text_emb);
            let sim = cosine_similarity(&audio_mem.vector, &text_hv);
            assert!(
                sim.abs() < 0.15,
                "audio/text orthogonality violated: sim={}",
                sim
            );
        }
    }
}

#[test]
fn feature_vector_has_correct_dim() {
    let tone = sine_wave(440.0, 2.0, 0.5);
    let features = kannaka_memory::ear::extract_features(&tone).unwrap();
    assert_eq!(features.vector.len(), AUDIO_FEATURE_DIM);
}
