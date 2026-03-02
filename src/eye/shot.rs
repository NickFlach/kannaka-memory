//! Shot boundary detection via histogram difference between consecutive frames.

/// Detect shot boundaries from per-frame spatial feature vectors.
///
/// Uses the color histogram portion (first 48 dims) of each frame's features.
/// A shot boundary is detected when the histogram difference exceeds a threshold.
pub fn detect_shots(per_frame_features: &[Vec<f32>]) -> Vec<usize> {
    if per_frame_features.len() < 2 {
        return vec![];
    }

    // Use first 48 dims (HSV histogram) for shot detection
    let hist_dim = 48.min(per_frame_features[0].len());
    let mut diffs: Vec<f32> = Vec::with_capacity(per_frame_features.len() - 1);

    for i in 1..per_frame_features.len() {
        let diff: f32 = (0..hist_dim)
            .map(|d| (per_frame_features[i][d] - per_frame_features[i - 1][d]).abs())
            .sum();
        diffs.push(diff);
    }

    if diffs.is_empty() {
        return vec![];
    }

    // Adaptive threshold: mean + 2*std of histogram differences
    let n = diffs.len() as f32;
    let mean = diffs.iter().sum::<f32>() / n;
    let variance = diffs.iter().map(|d| (d - mean).powi(2)).sum::<f32>() / n;
    let std = variance.sqrt();
    let threshold = mean + 2.0 * std;

    let mut boundaries = Vec::new();
    for (i, &diff) in diffs.iter().enumerate() {
        if diff > threshold {
            boundaries.push(i + 1); // boundary is at the frame AFTER the change
        }
    }

    boundaries
}

/// Compute shot-level statistics from detected boundaries.
pub struct ShotStats {
    /// Number of shots.
    pub count: usize,
    /// Mean shot length in frames.
    pub mean_length: f32,
    /// Std of shot lengths.
    pub std_length: f32,
    /// Max shot length.
    pub max_length: f32,
    /// Regularity of cut rhythm (0=irregular, 1=perfectly regular).
    pub regularity: f32,
}

pub fn shot_statistics(boundaries: &[usize], total_frames: usize) -> ShotStats {
    if boundaries.is_empty() {
        return ShotStats {
            count: 1,
            mean_length: total_frames as f32,
            std_length: 0.0,
            max_length: total_frames as f32,
            regularity: 1.0,
        };
    }

    // Shot lengths
    let mut lengths: Vec<f32> = Vec::new();
    let mut prev = 0;
    for &b in boundaries {
        lengths.push((b - prev) as f32);
        prev = b;
    }
    lengths.push((total_frames - prev) as f32); // last shot

    let count = lengths.len();
    let mean = lengths.iter().sum::<f32>() / count as f32;
    let variance = lengths.iter().map(|l| (l - mean).powi(2)).sum::<f32>() / count as f32;
    let std = variance.sqrt();
    let max = lengths.iter().cloned().fold(0.0f32, f32::max);

    // Regularity: 1 - (std / mean), clamped to [0, 1]
    let regularity = if mean > 0.0 {
        (1.0 - std / mean).clamp(0.0, 1.0)
    } else {
        0.0
    };

    ShotStats {
        count,
        mean_length: mean,
        std_length: std,
        max_length: max,
        regularity,
    }
}
