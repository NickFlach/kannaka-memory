//! kannaka-ear: Audio perception module.
//!
//! Decodes audio files (WAV, MP3) → extracts perceptual features →
//! projects through a dedicated audio Codebook → HyperMemory.
//!
//! The audio codebook uses a different seed (0xEAR = 3755) from the text
//! codebook (42), ensuring audio and text hypervectors occupy orthogonal
//! subspaces in the 10,000-dimensional memory space.

mod decode;
mod features;
mod mel;

pub use decode::decode_audio;
pub use features::{extract_features, AudioFeatures};

use std::f32::consts::PI;
use std::path::Path;

use crate::codebook::Codebook;
use crate::memory::HyperMemory;
use crate::xi_operator::compute_xi_signature;

// ── Constants ──────────────────────────────────────────────

/// Target sample rate for all analysis (mono).
pub const SAMPLE_RATE: u32 = 22050;
/// FFT window size.
pub const FFT_SIZE: usize = 2048;
/// Hop between FFT frames.
pub const HOP_SIZE: usize = 512;
/// Number of mel filter bands.
pub const N_MELS: usize = 128;
/// Number of MFCC coefficients.
pub const N_MFCC: usize = 13;
/// Total perceptual feature vector dimension (see ADR for breakdown).
pub const AUDIO_FEATURE_DIM: usize = 296;
/// Codebook seed for audio modality — distinct from text seed (42).
/// Mnemonic: "EAR" → 0xEA5 (3749).
pub const AUDIO_CODEBOOK_SEED: u64 = 0xEA5;
/// Hypervector output dimension (matches text pipeline).
pub const HYPERVECTOR_DIM: usize = 10_000;

// ── Error ──────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum EarError {
    #[error("failed to decode audio: {0}")]
    Decode(String),
    #[error("empty audio")]
    EmptyAudio,
    #[error("audio too short for analysis (need ≥ {FFT_SIZE} samples)")]
    TooShort,
    #[error("feature extraction failed: {0}")]
    Feature(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ── AudioPipeline ──────────────────────────────────────────

/// Top-level API: audio file → HyperMemory.
pub struct AudioPipeline {
    codebook: Codebook,
}

impl AudioPipeline {
    /// Create a new pipeline with the dedicated audio codebook.
    pub fn new() -> Self {
        Self {
            codebook: Codebook::new(AUDIO_FEATURE_DIM, HYPERVECTOR_DIM, AUDIO_CODEBOOK_SEED),
        }
    }

    /// Encode an audio file into a HyperMemory.
    pub fn encode_file(&self, path: &Path) -> Result<(HyperMemory, AudioFeatures), EarError> {
        let samples = decode_audio(path)?;
        if samples.is_empty() {
            return Err(EarError::EmptyAudio);
        }
        if samples.len() < FFT_SIZE {
            return Err(EarError::TooShort);
        }

        let af = extract_features(&samples)?;
        let hv = self.codebook.project(&af.vector);

        let mut mem = HyperMemory::new(hv, format!("audio:{}", path.display()));
        // Audio-specific wave params (from ADR)
        mem.frequency = 0.05;
        mem.phase = PI / 4.0;
        mem.decay_rate = 5e-7;
        mem.xi_signature = compute_xi_signature(&mem.vector);

        Ok((mem, af))
    }

    /// Encode raw mono f32 samples at [`SAMPLE_RATE`] Hz.
    pub fn encode_samples(&self, samples: &[f32], label: &str) -> Result<(HyperMemory, AudioFeatures), EarError> {
        if samples.is_empty() {
            return Err(EarError::EmptyAudio);
        }
        if samples.len() < FFT_SIZE {
            return Err(EarError::TooShort);
        }

        let af = extract_features(samples)?;
        let hv = self.codebook.project(&af.vector);

        let mut mem = HyperMemory::new(hv, format!("audio:{}", label));
        mem.frequency = 0.05;
        mem.phase = PI / 4.0;
        mem.decay_rate = 5e-7;
        mem.xi_signature = compute_xi_signature(&mem.vector);

        Ok((mem, af))
    }

    /// Access the underlying codebook (for tests).
    pub fn codebook(&self) -> &Codebook {
        &self.codebook
    }
}

impl Default for AudioPipeline {
    fn default() -> Self {
        Self::new()
    }
}
