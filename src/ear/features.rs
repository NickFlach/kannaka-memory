//! Perceptual feature extraction: mel, MFCC, spectral, rhythm, pitch, chroma, valence.
//!
//! Produces a 296-dimensional feature vector (see ADR for breakdown).

use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

use super::mel::{dct_ii, mel_spectrogram};
use super::{EarError, AUDIO_FEATURE_DIM, FFT_SIZE, HOP_SIZE, N_MELS, N_MFCC, SAMPLE_RATE};

/// Extracted audio features with metadata.
#[derive(Debug, Clone)]
pub struct AudioFeatures {
    /// The 296-dim feature vector.
    pub vector: Vec<f32>,
    /// Duration in seconds.
    pub duration_secs: f32,
    /// Estimated BPM.
    pub tempo_bpm: f32,
    /// Mean RMS energy.
    pub rms_mean: f32,
    /// Spectral centroid (kHz).
    pub spectral_centroid_khz: f32,
    /// Detected features summary for tagging.
    pub feature_tags: Vec<String>,
}

/// Extract the full 296-dim perceptual feature vector from mono audio at [`SAMPLE_RATE`].
pub fn extract_features(samples: &[f32]) -> Result<AudioFeatures, EarError> {
    let duration_secs = samples.len() as f32 / SAMPLE_RATE as f32;
    let mut features = Vec::with_capacity(AUDIO_FEATURE_DIM);

    // ── Mel spectrogram stats (128 + 128 = 256 dims) ──
    let mel_spec = mel_spectrogram(samples);
    if mel_spec.is_empty() {
        return Err(EarError::Feature("audio too short for mel spectrogram".into()));
    }
    let n_frames = mel_spec.len();

    // Mean per band
    let mut mel_means = [0.0f32; N_MELS];
    for frame in &mel_spec {
        for (i, &v) in frame.iter().enumerate() {
            mel_means[i] += v;
        }
    }
    for m in &mut mel_means {
        *m /= n_frames as f32;
    }
    features.extend_from_slice(&mel_means);

    // Std per band
    for band in 0..N_MELS {
        let mean = mel_means[band];
        let var: f32 = mel_spec.iter().map(|f| (f[band] - mean).powi(2)).sum::<f32>()
            / n_frames as f32;
        features.push(var.sqrt());
    }

    // ── MFCC mean (13 dims) ──
    let mfcc_mean = compute_mfcc_mean(&mel_spec);
    features.extend_from_slice(&mfcc_mean);

    // ── Spectral features (4 dims) ──
    let centroid = spectral_centroid(samples);
    let bandwidth = spectral_bandwidth(samples, centroid);
    let rolloff = spectral_rolloff(samples);
    let zcr = zero_crossing_rate(samples);
    features.push(centroid);
    features.push(bandwidth);
    features.push(rolloff);
    features.push(zcr);

    // ── RMS stats (2 dims) ──
    let (rms_mean, rms_std) = rms_stats(samples);
    features.push(rms_mean);
    features.push(rms_std);

    // ── Rhythm (2 dims) ──
    let (tempo, onset_density) = rhythm_features(samples);
    features.push(tempo / 200.0); // normalize
    features.push(onset_density);

    // ── Pitch (2 dims) ──
    let (pitch_mean, pitch_std) = pitch_stats(samples);
    features.push(pitch_mean / 1000.0);
    features.push(pitch_std / 1000.0);

    // ── Chromagram (12 dims) ──
    let chroma = chromagram_mean(samples);
    features.extend_from_slice(&chroma);

    // ── Emotional valence (5 dims) ──
    let valence = emotional_valence(centroid, rms_mean, &chroma, tempo);
    features.extend_from_slice(&valence);

    assert_eq!(
        features.len(),
        AUDIO_FEATURE_DIM,
        "feature dim mismatch: got {}",
        features.len()
    );

    // Build tags
    let mut feature_tags = Vec::new();
    if tempo > 0.0 {
        feature_tags.push(format!("{:.0}bpm", tempo));
    }
    if centroid > 3.0 {
        feature_tags.push("bright".into());
    } else if centroid < 1.0 {
        feature_tags.push("dark".into());
    }
    if rms_mean > 0.1 {
        feature_tags.push("loud".into());
    } else if rms_mean < 0.01 {
        feature_tags.push("quiet".into());
    }
    if zcr > 0.2 {
        feature_tags.push("noisy".into());
    } else if zcr < 0.05 {
        feature_tags.push("tonal".into());
    }

    Ok(AudioFeatures {
        vector: features,
        duration_secs,
        tempo_bpm: tempo,
        rms_mean,
        spectral_centroid_khz: centroid,
        feature_tags,
    })
}

// ── MFCC ───────────────────────────────────────────────────

fn compute_mfcc_mean(mel_spec: &[[f32; N_MELS]]) -> Vec<f32> {
    let n_frames = mel_spec.len();
    let mut mfcc_sum = vec![0.0f32; N_MFCC];

    for frame in mel_spec {
        let dct = dct_ii(frame.as_slice(), N_MFCC);
        for (i, &v) in dct.iter().enumerate() {
            mfcc_sum[i] += v;
        }
    }

    mfcc_sum.iter().map(|v| v / n_frames as f32).collect()
}

// ── Spectral features ──────────────────────────────────────

fn spectral_centroid(samples: &[f32]) -> f32 {
    let (magnitudes, _) = magnitude_spectrum(samples);
    let total: f32 = magnitudes.iter().sum();
    if total < 1e-10 {
        return 0.0;
    }
    let weighted: f32 = magnitudes
        .iter()
        .enumerate()
        .map(|(i, &m)| i as f32 * m)
        .sum();
    (weighted / total) * SAMPLE_RATE as f32 / FFT_SIZE as f32 / 1000.0
}

fn spectral_bandwidth(samples: &[f32], centroid_khz: f32) -> f32 {
    let (magnitudes, _) = magnitude_spectrum(samples);
    let total: f32 = magnitudes.iter().sum();
    if total < 1e-10 {
        return 0.0;
    }
    let centroid_bin = centroid_khz * 1000.0 * FFT_SIZE as f32 / SAMPLE_RATE as f32;
    let var: f32 = magnitudes
        .iter()
        .enumerate()
        .map(|(i, &m)| m * (i as f32 - centroid_bin).powi(2))
        .sum::<f32>()
        / total;
    (var.sqrt()) * SAMPLE_RATE as f32 / FFT_SIZE as f32 / 1000.0
}

fn spectral_rolloff(samples: &[f32]) -> f32 {
    let (magnitudes, _) = magnitude_spectrum(samples);
    let total: f32 = magnitudes.iter().sum();
    let threshold = total * 0.85;
    let mut cumsum = 0.0f32;
    for (i, &m) in magnitudes.iter().enumerate() {
        cumsum += m;
        if cumsum >= threshold {
            return i as f32 / magnitudes.len() as f32;
        }
    }
    1.0
}

fn zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }
    let crossings = samples
        .windows(2)
        .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
        .count();
    crossings as f32 / samples.len() as f32
}

/// Compute magnitude spectrum of first FFT_SIZE samples.
fn magnitude_spectrum(samples: &[f32]) -> (Vec<f32>, usize) {
    let n = FFT_SIZE.min(samples.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);
    let mut buf: Vec<Complex<f32>> = samples[..n]
        .iter()
        .map(|&s| Complex::new(s, 0.0))
        .collect();
    fft.process(&mut buf);
    let mags: Vec<f32> = buf[..n / 2].iter().map(|c| c.norm()).collect();
    (mags, n)
}

// ── Energy ─────────────────────────────────────────────────

fn rms_stats(samples: &[f32]) -> (f32, f32) {
    let frame_size = 2048;
    let rms_values: Vec<f32> = samples
        .chunks(frame_size)
        .map(|chunk| {
            let ms = chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32;
            ms.sqrt()
        })
        .collect();
    let n = rms_values.len().max(1) as f32;
    let mean = rms_values.iter().sum::<f32>() / n;
    let var = rms_values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n;
    (mean, var.sqrt())
}

// ── Rhythm ─────────────────────────────────────────────────

fn rhythm_features(samples: &[f32]) -> (f32, f32) {
    let duration_secs = samples.len() as f32 / SAMPLE_RATE as f32;
    if duration_secs < 0.5 {
        return (0.0, 0.0);
    }

    // Onset envelope: RMS difference between consecutive frames
    let rms: Vec<f32> = samples
        .chunks(HOP_SIZE)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();

    let onset_env: Vec<f32> = rms.windows(2).map(|w| (w[1] - w[0]).max(0.0)).collect();

    let threshold = onset_env.iter().cloned().fold(0.0f32, f32::max) * 0.3;
    let n_onsets = onset_env.iter().filter(|&&v| v > threshold).count();
    let onset_density = (n_onsets as f32 / duration_secs).min(50.0) / 50.0;

    let tempo = estimate_tempo(&onset_env);
    (tempo, onset_density)
}

fn estimate_tempo(onset_env: &[f32]) -> f32 {
    if onset_env.len() < 100 {
        return 120.0;
    }

    let fps = SAMPLE_RATE as f32 / HOP_SIZE as f32;
    let min_lag = (fps * 60.0 / 200.0) as usize;
    let max_lag = ((fps * 60.0 / 60.0) as usize).min(onset_env.len() / 2);

    if min_lag >= max_lag {
        return 120.0;
    }

    let mut best_lag = min_lag;
    let mut best_corr = f32::NEG_INFINITY;

    for lag in min_lag..=max_lag {
        let corr: f32 = onset_env
            .iter()
            .zip(onset_env[lag..].iter())
            .map(|(a, b)| a * b)
            .sum();
        if corr > best_corr {
            best_corr = corr;
            best_lag = lag;
        }
    }

    fps * 60.0 / best_lag as f32
}

// ── Pitch ──────────────────────────────────────────────────

fn pitch_stats(samples: &[f32]) -> (f32, f32) {
    let frame_size = 2048;
    let mut pitches = Vec::new();

    for chunk in samples.chunks(frame_size) {
        if chunk.len() < frame_size {
            break;
        }
        if let Some(f0) = detect_pitch_autocorr(chunk) {
            pitches.push(f0);
        }
    }

    if pitches.is_empty() {
        return (0.0, 0.0);
    }
    let mean = pitches.iter().sum::<f32>() / pitches.len() as f32;
    let var = pitches
        .iter()
        .map(|p| (p - mean).powi(2))
        .sum::<f32>()
        / pitches.len() as f32;
    (mean, var.sqrt())
}

fn detect_pitch_autocorr(frame: &[f32]) -> Option<f32> {
    let min_lag = SAMPLE_RATE as usize / 1000; // 1000 Hz max
    let max_lag = (SAMPLE_RATE as usize / 80).min(frame.len() / 2); // 80 Hz min

    if min_lag >= max_lag {
        return None;
    }

    let mut best_lag = min_lag;
    let mut best_corr = f32::NEG_INFINITY;

    for lag in min_lag..=max_lag {
        let corr: f32 = frame.iter().zip(frame[lag..].iter()).map(|(a, b)| a * b).sum();
        if corr > best_corr {
            best_corr = corr;
            best_lag = lag;
        }
    }

    let energy: f32 = frame.iter().map(|s| s * s).sum();
    if energy > 1e-10 && best_corr / energy > 0.3 {
        Some(SAMPLE_RATE as f32 / best_lag as f32)
    } else {
        None
    }
}

// ── Chromagram ─────────────────────────────────────────────

fn chromagram_mean(samples: &[f32]) -> Vec<f32> {
    let n = FFT_SIZE.min(samples.len());
    let mut chroma = vec![0.0f32; 12];
    let mut n_frames = 0;

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);

    let mut pos = 0;
    while pos + n <= samples.len() {
        let mut buf: Vec<Complex<f32>> = samples[pos..pos + n]
            .iter()
            .map(|&s| Complex::new(s, 0.0))
            .collect();
        fft.process(&mut buf);

        for (i, c) in buf[1..n / 2].iter().enumerate() {
            let freq = (i + 1) as f32 * SAMPLE_RATE as f32 / n as f32;
            if freq < 20.0 || freq > 5000.0 {
                continue;
            }
            let midi = 12.0 * (freq / 440.0).log2() + 69.0;
            let pitch_class = ((midi as i32 % 12 + 12) % 12) as usize;
            chroma[pitch_class] += c.norm_sqr();
        }
        n_frames += 1;
        pos += HOP_SIZE;
    }

    if n_frames > 0 {
        for v in &mut chroma {
            *v /= n_frames as f32;
        }
    }
    let max = chroma.iter().cloned().fold(0.0f32, f32::max).max(1e-10);
    chroma.iter().map(|v| v / max).collect()
}

// ── Emotional valence ──────────────────────────────────────

fn emotional_valence(centroid_khz: f32, rms_mean: f32, chroma: &[f32], tempo: f32) -> Vec<f32> {
    // 5-dim: [brightness, energy, tension, movement, warmth]
    let brightness = centroid_khz.min(5.0) / 5.0;
    let energy = rms_mean.min(0.5) / 0.5;

    let major_third = chroma.get(4).copied().unwrap_or(0.0);
    let minor_third = chroma.get(3).copied().unwrap_or(0.0);
    let tension = if major_third + minor_third > 0.01 {
        minor_third / (major_third + minor_third)
    } else {
        0.5
    };

    let movement = (tempo / 180.0).min(1.0);
    let warmth = 1.0 - brightness;

    vec![brightness, energy, tension, movement, warmth]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

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
    fn feature_vector_correct_dimension() {
        let tone = sine_wave(440.0, 2.0, 0.5);
        let af = extract_features(&tone).unwrap();
        assert_eq!(af.vector.len(), AUDIO_FEATURE_DIM);
    }

    #[test]
    fn sine_vs_noise_features_differ() {
        let tone = sine_wave(440.0, 2.0, 0.5);
        let noise = white_noise(2.0);
        let f_tone = extract_features(&tone).unwrap();
        let f_noise = extract_features(&noise).unwrap();
        // ZCR should differ significantly
        assert!(
            (f_tone.vector[259] - f_noise.vector[259]).abs() > 0.01,
            "ZCR should differ between tone and noise"
        );
    }

    #[test]
    fn duration_computed_correctly() {
        let tone = sine_wave(440.0, 3.0, 0.5);
        let af = extract_features(&tone).unwrap();
        assert!((af.duration_secs - 3.0).abs() < 0.1);
    }
}
