//! kannaka-eye: Video perception module.
//!
//! Decodes video files (via ffmpeg subprocess) → extracts spatial + temporal
//! features → projects through a dedicated video Codebook → HyperMemory.
//!
//! The video codebook uses seed 0xEYE (15966), orthogonal to text (42)
//! and audio (0xEA5 = 3749) in 10,000-dimensional space.

mod color;
mod decode;
mod motion;
mod shot;
mod spatial;
mod temporal;

pub use decode::{decode_video, VideoFrames, FrameInfo};
pub use spatial::{extract_frame_features, aggregate_spatial, SpatialFeatures};
pub use temporal::{extract_temporal_features, TemporalFeatures};

use std::f32::consts::PI;
use std::path::Path;

use crate::codebook::Codebook;
use crate::memory::HyperMemory;
use crate::xi_operator::compute_xi_signature;

// ── Constants ──────────────────────────────────────────────

/// Default frames per second for analysis (2 fps = efficient).
pub const DEFAULT_FPS: f32 = 2.0;
/// Target width for decoded frames (height scales proportionally).
pub const TARGET_WIDTH: u32 = 320;
/// Spatial feature vector dimension (per-frame, aggregated to sequence stats).
pub const SPATIAL_FEATURE_DIM: usize = 192;
/// Temporal feature vector dimension (sequence-level).
pub const TEMPORAL_FEATURE_DIM: usize = 128;
/// Total video feature vector dimension (spatial + temporal).
pub const VIDEO_FEATURE_DIM: usize = SPATIAL_FEATURE_DIM + TEMPORAL_FEATURE_DIM; // 320
/// Codebook seed for video modality.
/// Mnemonic: "EYE" → 0x3E5E = 15966, orthogonal to text (42) and audio (0xEA5).
pub const VIDEO_CODEBOOK_SEED: u64 = 0x3E5E;
/// Hypervector output dimension (matches text + audio pipelines).
pub const HYPERVECTOR_DIM: usize = 10_000;
/// Number of HSV histogram bins per channel.
pub const HSV_BINS: usize = 16;
/// Number of edge orientation bins.
pub const EDGE_BINS: usize = 36;
/// Spatial grid for region statistics (rows × cols).
pub const REGION_GRID: (usize, usize) = (4, 5);
/// Number of spatial frequency bands (DCT energy).
pub const FREQ_BANDS: usize = 32;
/// Number of optical flow radial bins.
pub const FLOW_BINS: usize = 32;

// ── Error ──────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum EyeError {
    #[error("failed to decode video: {0}")]
    Decode(String),
    #[error("empty video (no frames decoded)")]
    EmptyVideo,
    #[error("video too short for analysis (need >= 2 frames)")]
    TooShort,
    #[error("feature extraction failed: {0}")]
    Feature(String),
    #[error("ffmpeg not found — install ffmpeg and ensure it's in PATH")]
    FfmpegNotFound,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// ── VideoFeatures ──────────────────────────────────────────

/// Full video feature vector + metadata.
#[derive(Debug, Clone)]
pub struct VideoFeatures {
    /// Concatenated feature vector [spatial_agg(192) + temporal(128)] = 320 dims.
    pub vector: Vec<f32>,
    /// Spatial features aggregated across frames.
    pub spatial: SpatialFeatures,
    /// Temporal features computed over frame sequence.
    pub temporal: TemporalFeatures,
    /// Duration in seconds.
    pub duration_secs: f32,
    /// Number of frames analyzed.
    pub frame_count: usize,
    /// Frames per second used for analysis.
    pub analysis_fps: f32,
    /// Detected shot boundaries (frame indices).
    pub shot_boundaries: Vec<usize>,
}

// ── VideoPipeline ──────────────────────────────────────────

/// Top-level API: video file → HyperMemory.
pub struct VideoPipeline {
    codebook: Codebook,
    fps: f32,
}

impl VideoPipeline {
    /// Create a new pipeline with the dedicated video codebook.
    pub fn new() -> Self {
        Self {
            codebook: Codebook::new(VIDEO_FEATURE_DIM, HYPERVECTOR_DIM, VIDEO_CODEBOOK_SEED),
            fps: DEFAULT_FPS,
        }
    }

    /// Create a pipeline with custom FPS.
    pub fn with_fps(fps: f32) -> Self {
        Self {
            codebook: Codebook::new(VIDEO_FEATURE_DIM, HYPERVECTOR_DIM, VIDEO_CODEBOOK_SEED),
            fps,
        }
    }

    /// Encode a video file into a HyperMemory.
    pub fn encode_file(&self, path: &Path) -> Result<(HyperMemory, VideoFeatures), EyeError> {
        // Decode frames via ffmpeg
        let frames = decode_video(path, self.fps, TARGET_WIDTH)?;
        if frames.frames.is_empty() {
            return Err(EyeError::EmptyVideo);
        }
        if frames.frames.len() < 2 {
            return Err(EyeError::TooShort);
        }

        // Extract per-frame spatial features
        let per_frame_spatial: Vec<Vec<f32>> = frames
            .frames
            .iter()
            .map(|f| spatial::extract_frame_features(f))
            .collect();

        // Detect shot boundaries
        let shot_boundaries = shot::detect_shots(&per_frame_spatial);

        // Aggregate spatial features across frames
        let spatial_agg = spatial::aggregate_spatial(&per_frame_spatial);

        // Extract temporal features from the frame sequence
        let temporal = temporal::extract_temporal_features(&per_frame_spatial, &shot_boundaries);

        // Concatenate into full feature vector
        let mut vector = spatial_agg.vector.clone();
        vector.extend_from_slice(&temporal.vector);
        assert_eq!(vector.len(), VIDEO_FEATURE_DIM);

        // Project through codebook
        let hv = self.codebook.project(&vector);

        let vf = VideoFeatures {
            vector: vector.clone(),
            spatial: spatial_agg,
            temporal,
            duration_secs: frames.duration_secs,
            frame_count: frames.frames.len(),
            analysis_fps: self.fps,
            shot_boundaries: shot_boundaries.clone(),
        };

        let mut mem = HyperMemory::new(hv, format!("video:{}", path.display()));
        // Video-specific wave params (from ADR-0008)
        mem.frequency = 0.03; // slower than audio (0.05) — visual memories more stable
        mem.phase = PI / 2.0; // 90° offset from audio (π/4) and text (0)
        mem.decay_rate = 3e-7; // slower decay than audio (5e-7)
        mem.xi_signature = compute_xi_signature(&mem.vector);

        Ok((mem, vf))
    }

    /// Access the underlying codebook (for tests).
    pub fn codebook(&self) -> &Codebook {
        &self.codebook
    }
}

impl Default for VideoPipeline {
    fn default() -> Self {
        Self::new()
    }
}
