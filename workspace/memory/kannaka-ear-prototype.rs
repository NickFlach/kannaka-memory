//! kannaka-ear: Audio perception module prototype sketch
//!
//! This would live at kannaka-memory/src/ear/mod.rs
//! Dependencies to add to Cargo.toml:
//!   symphonia = { version = "0.5", features = ["mp3", "wav", "pcm", "isomp4"] }
//!   rustfft = "6"
//!   rubato = "0.14"
//!   pitch-detection = "0.3"

use std::path::Path;
use std::f32::consts::PI;

use crate::codebook::Codebook;
use crate::memory::HyperMemory;
use crate::wave::{normalize, WaveParams};

// ── Constants ──────────────────────────────────────────────

const SAMPLE_RATE: u32 = 22050;
const FFT_SIZE: usize = 2048;
const HOP_SIZE: usize = 512;
const N_MELS: usize = 128;
const N_MFCC: usize = 13;
const AUDIO_FEATURE_DIM: usize = 296; // see ADR for breakdown
const AUDIO_CODEBOOK_SEED: u64 = 0xEAR; // 3819 — distinct from text seed (42)
const HYPERVECTOR_DIM: usize = 10_000;

// ── Error Types ────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum EarError {
    #[error("failed to decode audio: {0}")]
    Decode(String),
    #[error("empty audio")]
    EmptyAudio,
    #[error("feature extraction failed: {0}")]
    Feature(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ── Audio Pipeline (top-level API) ─────────────────────────

/// The main entry point for audio → HyperMemory encoding.
pub struct AudioPipeline {
    codebook: Codebook,
}

impl AudioPipeline {
    pub fn new() -> Self {
        Self {
            codebook: Codebook::new(AUDIO_FEATURE_DIM, HYPERVECTOR_DIM, AUDIO_CODEBOOK_SEED),
        }
    }

    /// Encode an audio file into a HyperMemory.
    pub fn encode_file(&self, path: &Path) -> Result<HyperMemory, EarError> {
        // 1. Decode to mono f32 PCM at 22050 Hz
        let samples = decode_audio(path)?;
        if samples.is_empty() {
            return Err(EarError::EmptyAudio);
        }

        // 2. Extract perceptual feature vector
        let features = extract_features(&samples)?;
        assert_eq!(features.len(), AUDIO_FEATURE_DIM);

        // 3. Project through audio codebook → 10K hypervector
        let hv = self.codebook.project(&features);

        // 4. Create HyperMemory with audio-specific wave params
        let mut mem = HyperMemory::new(hv, format!("audio:{}", path.display()));
        mem.frequency = 0.05;           // slower oscillation
        mem.phase = PI / 4.0;           // offset from text phase
        mem.decay_rate = 5e-7;          // slower decay
        // TODO: mem.modality = Some("audio".to_string());
        // TODO: mem.xi_signature = compute audio-specific xi

        Ok(mem)
    }

    /// Encode raw samples (already mono f32 @ 22050 Hz).
    pub fn encode_samples(&self, samples: &[f32], label: &str) -> Result<HyperMemory, EarError> {
        if samples.is_empty() {
            return Err(EarError::EmptyAudio);
        }
        let features = extract_features(samples)?;
        let hv = self.codebook.project(&features);
        let mut mem = HyperMemory::new(hv, format!("audio:{}", label));
        mem.frequency = 0.05;
        mem.phase = PI / 4.0;
        mem.decay_rate = 5e-7;
        Ok(mem)
    }
}

// ── Audio Decoding ─────────────────────────────────────────

/// Decode audio file to mono f32 samples at SAMPLE_RATE Hz.
///
/// Uses symphonia for format-agnostic decoding.
fn decode_audio(path: &Path) -> Result<Vec<f32>, EarError> {
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| EarError::Decode(e.to_string()))?;

    let mut format = probed.format;
    let track = format.default_track()
        .ok_or_else(|| EarError::Decode("no audio track".into()))?;
    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let source_rate = codec_params.sample_rate.unwrap_or(44100);
    let channels = codec_params.channels.map(|c| c.count()).unwrap_or(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| EarError::Decode(e.to_string()))?;

    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let spec = *decoded.spec();
        let n_frames = decoded.capacity();
        let mut sample_buf = SampleBuffer::<f32>::new(n_frames as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);

        let interleaved = sample_buf.samples();
        // Mix to mono
        for frame in interleaved.chunks(channels) {
            let mono: f32 = frame.iter().sum::<f32>() / channels as f32;
            all_samples.push(mono);
        }
    }

    // Resample to SAMPLE_RATE if needed (simplified: linear interpolation)
    if source_rate != SAMPLE_RATE {
        all_samples = resample_linear(&all_samples, source_rate, SAMPLE_RATE);
    }

    Ok(all_samples)
}

/// Simple linear interpolation resampler.
/// For production, use `rubato` crate for high-quality resampling.
fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = src_pos - idx as f64;
        let s0 = samples.get(idx).copied().unwrap_or(0.0);
        let s1 = samples.get(idx + 1).copied().unwrap_or(s0);
        out.push(s0 + (s1 - s0) * frac as f32);
    }
    out
}

// ── Feature Extraction ─────────────────────────────────────

/// Extract the full 296-dim perceptual feature vector from mono audio.
fn extract_features(samples: &[f32]) -> Result<Vec<f32>, EarError> {
    let mut features = Vec::with_capacity(AUDIO_FEATURE_DIM);

    // Compute mel spectrogram (frames × N_MELS)
    let mel_spec = mel_spectrogram(samples);
    if mel_spec.is_empty() {
        return Err(EarError::Feature("audio too short for analysis".into()));
    }

    // Mel mean and std per band (128 + 128 = 256 dims)
    let n_frames = mel_spec.len();
    for band in 0..N_MELS {
        let vals: Vec<f32> = mel_spec.iter().map(|frame| frame[band]).collect();
        let mean = vals.iter().sum::<f32>() / n_frames as f32;
        features.push(mean);
    }
    for band in 0..N_MELS {
        let vals: Vec<f32> = mel_spec.iter().map(|frame| frame[band]).collect();
        let mean = vals.iter().sum::<f32>() / n_frames as f32;
        let var = vals.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n_frames as f32;
        features.push(var.sqrt());
    }

    // MFCC: DCT of log mel (first 13 coefficients, mean across frames) — 13 dims
    let mfcc_mean = compute_mfcc_mean(&mel_spec);
    features.extend_from_slice(&mfcc_mean);

    // Spectral centroid, bandwidth, rolloff, ZCR — 4 dims
    features.push(spectral_centroid(samples));
    features.push(spectral_bandwidth(samples));
    features.push(spectral_rolloff(samples));
    features.push(zero_crossing_rate(samples));

    // RMS mean and std — 2 dims
    let (rms_mean, rms_std) = rms_stats(samples);
    features.push(rms_mean);
    features.push(rms_std);

    // Tempo and onset density — 2 dims
    let (tempo, onset_density) = rhythm_features(samples);
    features.push(tempo / 200.0); // normalize BPM to ~[0,1]
    features.push(onset_density);

    // Pitch mean and std — 2 dims
    let (pitch_mean, pitch_std) = pitch_stats(samples);
    features.push(pitch_mean / 1000.0); // normalize Hz
    features.push(pitch_std / 1000.0);

    // Chromagram (12 bins mean) — 12 dims
    let chroma = chromagram_mean(samples);
    features.extend_from_slice(&chroma);

    // Emotional valence heuristic — 5 dims
    let valence = emotional_valence(&features, &chroma, tempo);
    features.extend_from_slice(&valence);

    assert_eq!(features.len(), AUDIO_FEATURE_DIM,
        "feature dim mismatch: got {}", features.len());

    Ok(features)
}

// ── Mel Spectrogram ────────────────────────────────────────

/// Compute mel spectrogram: returns Vec of frames, each frame is N_MELS f32 values.
fn mel_spectrogram(samples: &[f32]) -> Vec<Vec<f32>> {
    use rustfft::{FftPlanner, num_complex::Complex};

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);

    // Build mel filterbank
    let mel_filters = build_mel_filterbank(N_MELS, FFT_SIZE, SAMPLE_RATE);

    let mut frames = Vec::new();
    let hann = hann_window(FFT_SIZE);

    let mut pos = 0;
    while pos + FFT_SIZE <= samples.len() {
        // Window the frame
        let mut buffer: Vec<Complex<f32>> = (0..FFT_SIZE)
            .map(|i| Complex::new(samples[pos + i] * hann[i], 0.0))
            .collect();

        fft.process(&mut buffer);

        // Power spectrum (first half)
        let power: Vec<f32> = buffer[..FFT_SIZE / 2 + 1]
            .iter()
            .map(|c| c.norm_sqr())
            .collect();

        // Apply mel filterbank
        let mel_frame: Vec<f32> = mel_filters
            .iter()
            .map(|filter| {
                let energy: f32 = filter.iter().zip(power.iter()).map(|(f, p)| f * p).sum();
                (energy + 1e-10).ln() // log mel energy
            })
            .collect();

        frames.push(mel_frame);
        pos += HOP_SIZE;
    }

    frames
}

/// Build triangular mel filterbank matrix (n_mels × n_fft/2+1).
fn build_mel_filterbank(n_mels: usize, n_fft: usize, sample_rate: u32) -> Vec<Vec<f32>> {
    let n_bins = n_fft / 2 + 1;
    let f_max = sample_rate as f32 / 2.0;

    // Hz to mel
    let hz_to_mel = |f: f32| -> f32 { 2595.0 * (1.0 + f / 700.0).log10() };
    let mel_to_hz = |m: f32| -> f32 { 700.0 * (10.0_f32.powf(m / 2595.0) - 1.0) };

    let mel_min = hz_to_mel(0.0);
    let mel_max = hz_to_mel(f_max);

    // n_mels + 2 equally spaced points in mel space
    let mel_points: Vec<f32> = (0..n_mels + 2)
        .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (n_mels + 1) as f32)
        .collect();

    let hz_points: Vec<f32> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();
    let bin_points: Vec<f32> = hz_points
        .iter()
        .map(|&f| f * n_fft as f32 / sample_rate as f32)
        .collect();

    let mut filters = Vec::with_capacity(n_mels);
    for i in 0..n_mels {
        let mut filter = vec![0.0f32; n_bins];
        let left = bin_points[i];
        let center = bin_points[i + 1];
        let right = bin_points[i + 2];

        for j in 0..n_bins {
            let jf = j as f32;
            if jf >= left && jf <= center {
                filter[j] = (jf - left) / (center - left + 1e-10);
            } else if jf > center && jf <= right {
                filter[j] = (right - jf) / (right - center + 1e-10);
            }
        }
        filters.push(filter);
    }
    filters
}

fn hann_window(size: usize) -> Vec<f32> {
    (0..size)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / size as f32).cos()))
        .collect()
}

// ── MFCC ───────────────────────────────────────────────────

/// Compute mean MFCC (first N_MFCC coefficients) across all frames.
fn compute_mfcc_mean(mel_spec: &[Vec<f32>]) -> Vec<f32> {
    let n_frames = mel_spec.len();
    let mut mfcc_sum = vec![0.0f32; N_MFCC];

    for frame in mel_spec {
        // Type-II DCT of log-mel frame
        let dct = dct_ii(frame);
        for i in 0..N_MFCC.min(dct.len()) {
            mfcc_sum[i] += dct[i];
        }
    }

    mfcc_sum.iter().map(|v| v / n_frames as f32).collect()
}

/// Type-II DCT (naive implementation, sufficient for 128-point inputs).
fn dct_ii(input: &[f32]) -> Vec<f32> {
    let n = input.len();
    (0..n)
        .map(|k| {
            input
                .iter()
                .enumerate()
                .map(|(i, &x)| x * (PI * k as f32 * (2.0 * i as f32 + 1.0) / (2.0 * n as f32)).cos())
                .sum()
        })
        .collect()
}

// ── Spectral Features ──────────────────────────────────────

fn spectral_centroid(samples: &[f32]) -> f32 {
    use rustfft::{FftPlanner, num_complex::Complex};
    let n = FFT_SIZE.min(samples.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);
    let mut buf: Vec<Complex<f32>> = samples[..n].iter().map(|&s| Complex::new(s, 0.0)).collect();
    fft.process(&mut buf);
    let magnitudes: Vec<f32> = buf[..n / 2].iter().map(|c| c.norm()).collect();
    let total: f32 = magnitudes.iter().sum();
    if total < 1e-10 { return 0.0; }
    let weighted: f32 = magnitudes.iter().enumerate()
        .map(|(i, &m)| i as f32 * m)
        .sum();
    (weighted / total) * SAMPLE_RATE as f32 / n as f32 / 1000.0 // normalize to kHz range
}

fn spectral_bandwidth(samples: &[f32]) -> f32 {
    // Simplified: standard deviation of spectral energy distribution
    let centroid = spectral_centroid(samples);
    // Placeholder — full implementation would compute per-frame
    centroid * 0.5 // rough approximation
}

fn spectral_rolloff(samples: &[f32]) -> f32 {
    use rustfft::{FftPlanner, num_complex::Complex};
    let n = FFT_SIZE.min(samples.len());
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);
    let mut buf: Vec<Complex<f32>> = samples[..n].iter().map(|&s| Complex::new(s, 0.0)).collect();
    fft.process(&mut buf);
    let magnitudes: Vec<f32> = buf[..n / 2].iter().map(|c| c.norm()).collect();
    let total: f32 = magnitudes.iter().sum();
    let threshold = total * 0.85;
    let mut cumsum = 0.0f32;
    for (i, &m) in magnitudes.iter().enumerate() {
        cumsum += m;
        if cumsum >= threshold {
            return i as f32 / (n / 2) as f32; // normalized [0, 1]
        }
    }
    1.0
}

fn zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 { return 0.0; }
    let crossings = samples.windows(2)
        .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
        .count();
    crossings as f32 / samples.len() as f32
}

// ── Energy Features ────────────────────────────────────────

fn rms_stats(samples: &[f32]) -> (f32, f32) {
    let frame_size = 2048;
    let rms_values: Vec<f32> = samples.chunks(frame_size)
        .map(|chunk| {
            let ms = chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32;
            ms.sqrt()
        })
        .collect();
    let mean = rms_values.iter().sum::<f32>() / rms_values.len().max(1) as f32;
    let var = rms_values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / rms_values.len().max(1) as f32;
    (mean, var.sqrt())
}

// ── Rhythm Features ────────────────────────────────────────

fn rhythm_features(samples: &[f32]) -> (f32, f32) {
    // Onset detection via spectral flux (simplified)
    // Returns (estimated BPM, onsets per second)
    let duration_secs = samples.len() as f32 / SAMPLE_RATE as f32;
    if duration_secs < 0.5 { return (0.0, 0.0); }

    // Compute onset envelope: RMS difference between consecutive frames
    let frame_size = HOP_SIZE;
    let rms: Vec<f32> = samples.chunks(frame_size)
        .map(|c| (c.iter().map(|s| s * s).sum::<f32>() / c.len() as f32).sqrt())
        .collect();

    let onset_env: Vec<f32> = rms.windows(2)
        .map(|w| (w[1] - w[0]).max(0.0))
        .collect();

    // Count onsets (peaks above threshold)
    let threshold = onset_env.iter().cloned().fold(0.0f32, f32::max) * 0.3;
    let n_onsets = onset_env.iter().filter(|&&v| v > threshold).count();
    let onset_density = n_onsets as f32 / duration_secs;

    // Tempo via autocorrelation of onset envelope
    let tempo = estimate_tempo_autocorr(&onset_env);

    (tempo, onset_density.min(50.0) / 50.0) // normalize density
}

fn estimate_tempo_autocorr(onset_env: &[f32]) -> f32 {
    if onset_env.len() < 100 { return 120.0; } // default

    let frames_per_sec = SAMPLE_RATE as f32 / HOP_SIZE as f32;
    // Search BPM range 60-200
    let min_lag = (frames_per_sec * 60.0 / 200.0) as usize;
    let max_lag = (frames_per_sec * 60.0 / 60.0) as usize;
    let max_lag = max_lag.min(onset_env.len() / 2);

    let mut best_lag = min_lag;
    let mut best_corr = f32::NEG_INFINITY;

    for lag in min_lag..=max_lag {
        let corr: f32 = onset_env.iter()
            .zip(onset_env[lag..].iter())
            .map(|(a, b)| a * b)
            .sum();
        if corr > best_corr {
            best_corr = corr;
            best_lag = lag;
        }
    }

    frames_per_sec * 60.0 / best_lag as f32
}

// ── Pitch Features ─────────────────────────────────────────

fn pitch_stats(samples: &[f32]) -> (f32, f32) {
    // Simplified autocorrelation pitch detection
    // For production, use `pitch-detection` crate with YIN algorithm
    let frame_size = 2048;
    let mut pitches = Vec::new();

    for chunk in samples.chunks(frame_size) {
        if chunk.len() < frame_size { break; }
        if let Some(f0) = detect_pitch_autocorr(chunk) {
            pitches.push(f0);
        }
    }

    if pitches.is_empty() { return (0.0, 0.0); }
    let mean = pitches.iter().sum::<f32>() / pitches.len() as f32;
    let var = pitches.iter().map(|p| (p - mean).powi(2)).sum::<f32>() / pitches.len() as f32;
    (mean, var.sqrt())
}

fn detect_pitch_autocorr(frame: &[f32]) -> Option<f32> {
    // Search for fundamental between 80 Hz and 1000 Hz
    let min_lag = SAMPLE_RATE as usize / 1000;
    let max_lag = (SAMPLE_RATE as usize / 80).min(frame.len() / 2);

    let mut best_lag = min_lag;
    let mut best_corr = f32::NEG_INFINITY;

    for lag in min_lag..=max_lag {
        let corr: f32 = frame.iter()
            .zip(frame[lag..].iter())
            .map(|(a, b)| a * b)
            .sum();
        if corr > best_corr {
            best_corr = corr;
            best_lag = lag;
        }
    }

    // Confidence check
    let energy: f32 = frame.iter().map(|s| s * s).sum();
    if best_corr / energy > 0.3 {
        Some(SAMPLE_RATE as f32 / best_lag as f32)
    } else {
        None // unpitched / noise
    }
}

// ── Chromagram ─────────────────────────────────────────────

fn chromagram_mean(samples: &[f32]) -> Vec<f32> {
    // Fold spectrum into 12 pitch classes (C, C#, D, ...)
    use rustfft::{FftPlanner, num_complex::Complex};

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
            if freq < 20.0 || freq > 5000.0 { continue; }
            // Map frequency to pitch class
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
    // Normalize
    let max = chroma.iter().cloned().fold(0.0f32, f32::max).max(1e-10);
    chroma.iter().map(|v| v / max).collect()
}

// ── Emotional Valence Heuristic ────────────────────────────

fn emotional_valence(features: &[f32], chroma: &[f32], tempo: f32) -> Vec<f32> {
    // 5-dim emotional valence vector:
    // [brightness, energy, tension, movement, warmth]

    // Brightness: spectral centroid (already in features)
    let brightness = features.get(256).copied().unwrap_or(0.5); // spectral centroid position

    // Energy: RMS level
    let energy = features.get(260).copied().unwrap_or(0.5); // rms_mean position

    // Tension: minor-key indicator from chromagram
    // Simple: if minor third (chroma[3]) > major third (chroma[4]), more tense
    let major_third = chroma.get(4).copied().unwrap_or(0.0);
    let minor_third = chroma.get(3).copied().unwrap_or(0.0);
    let tension = if major_third + minor_third > 0.01 {
        minor_third / (major_third + minor_third)
    } else {
        0.5
    };

    // Movement: tempo-based (normalized)
    let movement = (tempo / 180.0).min(1.0);

    // Warmth: inverse of spectral centroid (low frequencies = warm)
    let warmth = 1.0 - brightness.min(1.0);

    vec![brightness, energy, tension, movement, warmth]
}

// ── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave::cosine_similarity;

    /// Generate a simple sine wave for testing.
    fn sine_wave(freq: f32, duration_secs: f32, amplitude: f32) -> Vec<f32> {
        let n = (SAMPLE_RATE as f32 * duration_secs) as usize;
        (0..n)
            .map(|i| amplitude * (2.0 * PI * freq * i as f32 / SAMPLE_RATE as f32).sin())
            .collect()
    }

    /// Generate white noise.
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
    fn different_frequencies_produce_different_vectors() {
        let pipeline = AudioPipeline::new();
        let low = sine_wave(200.0, 2.0, 0.5);
        let high = sine_wave(4000.0, 2.0, 0.5);
        let mem_low = pipeline.encode_samples(&low, "200Hz").unwrap();
        let mem_high = pipeline.encode_samples(&high, "4kHz").unwrap();
        let sim = cosine_similarity(&mem_low.vector, &mem_high.vector);
        assert!(sim < 0.8, "different frequencies should produce distinct vectors, sim={}", sim);
    }

    #[test]
    fn sine_vs_noise_are_distinct() {
        let pipeline = AudioPipeline::new();
        let tone = sine_wave(440.0, 2.0, 0.5);
        let noise = white_noise(2.0);
        let mem_tone = pipeline.encode_samples(&tone, "A440").unwrap();
        let mem_noise = pipeline.encode_samples(&noise, "noise").unwrap();
        let sim = cosine_similarity(&mem_tone.vector, &mem_noise.vector);
        assert!(sim < 0.7, "tone and noise should be distinct, sim={}", sim);
    }

    #[test]
    fn same_sound_is_consistent() {
        let pipeline = AudioPipeline::new();
        let tone = sine_wave(440.0, 2.0, 0.5);
        let mem1 = pipeline.encode_samples(&tone, "A440").unwrap();
        let mem2 = pipeline.encode_samples(&tone, "A440").unwrap();
        let sim = cosine_similarity(&mem1.vector, &mem2.vector);
        assert!((sim - 1.0).abs() < 1e-4, "same input should produce identical vectors");
    }

    #[test]
    fn audio_memory_has_correct_wave_params() {
        let pipeline = AudioPipeline::new();
        let tone = sine_wave(440.0, 1.0, 0.5);
        let mem = pipeline.encode_samples(&tone, "test").unwrap();
        assert_eq!(mem.frequency, 0.05);
        assert!((mem.phase - PI / 4.0).abs() < 1e-6);
        assert_eq!(mem.decay_rate, 5e-7);
    }

    #[test]
    fn feature_vector_correct_dimension() {
        let tone = sine_wave(440.0, 2.0, 0.5);
        let features = extract_features(&tone).unwrap();
        assert_eq!(features.len(), AUDIO_FEATURE_DIM);
    }

    #[test]
    fn audio_orthogonal_to_text() {
        // Audio and text hypervectors should be nearly orthogonal
        // because they use different codebooks with different seeds
        let audio_pipeline = AudioPipeline::new();
        let text_codebook = Codebook::new(384, HYPERVECTOR_DIM, 42);

        let tone = sine_wave(440.0, 2.0, 0.5);
        let audio_mem = audio_pipeline.encode_samples(&tone, "A440").unwrap();

        // Project a random text-like embedding
        let text_embedding = vec![0.5f32; 384];
        let text_hv = text_codebook.project(&text_embedding);

        let sim = cosine_similarity(&audio_mem.vector, &text_hv);
        // With different codebooks and different input dims, these should be ~orthogonal
        assert!(sim.abs() < 0.2,
            "audio and text vectors should be nearly orthogonal, sim={}", sim);
    }
}
