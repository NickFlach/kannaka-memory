//! Sequence-level temporal feature extraction (128 dims).
//!
//! Computed over the full frame sequence, capturing how the video evolves.
//! Features: shot statistics (16), color flow (24), motion trajectory (24),
//! visual tempo (16), complexity evolution (16), brightness arc (16),
//! stillness ratio (8), entropy flow (8).

use super::shot::{shot_statistics, ShotStats};
use super::TEMPORAL_FEATURE_DIM;

/// Temporal features computed over a video's frame sequence.
#[derive(Debug, Clone)]
pub struct TemporalFeatures {
    /// 128-dim temporal feature vector.
    pub vector: Vec<f32>,
    /// Visual tempo in "beats per minute" (frame-difference peaks).
    pub visual_tempo_bpm: f32,
    /// Number of detected shots.
    pub shot_count: usize,
    /// Brightness arc description.
    pub brightness_trend: f32,
}

/// Extract temporal features from per-frame spatial features and shot boundaries.
pub fn extract_temporal_features(
    per_frame: &[Vec<f32>],
    shot_boundaries: &[usize],
) -> TemporalFeatures {
    let n = per_frame.len();
    let mut features = Vec::with_capacity(TEMPORAL_FEATURE_DIM);

    // 1. Shot statistics (16 dims)
    let shot_stats = shot_statistics(shot_boundaries, n);
    features.extend_from_slice(&encode_shot_stats(&shot_stats, n));

    // 2. Color flow — temporal derivative of color histograms (24 dims)
    features.extend_from_slice(&color_flow(per_frame));

    // 3. Motion trajectory from spatial features (24 dims)
    features.extend_from_slice(&motion_trajectory(per_frame));

    // 4. Visual tempo (16 dims)
    let (tempo_features, tempo_bpm) = visual_tempo(per_frame);
    features.extend_from_slice(&tempo_features);

    // 5. Complexity evolution (16 dims)
    features.extend_from_slice(&complexity_evolution(per_frame));

    // 6. Brightness arc (16 dims)
    let (brightness_features, brightness_trend) = brightness_arc(per_frame);
    features.extend_from_slice(&brightness_features);

    // 7. Stillness ratio (8 dims)
    features.extend_from_slice(&stillness_ratio(per_frame));

    // 8. Entropy flow (8 dims)
    features.extend_from_slice(&entropy_flow(per_frame));

    // Ensure exact dimension
    features.truncate(TEMPORAL_FEATURE_DIM);
    while features.len() < TEMPORAL_FEATURE_DIM {
        features.push(0.0);
    }

    TemporalFeatures {
        vector: features,
        visual_tempo_bpm: tempo_bpm,
        shot_count: shot_stats.count,
        brightness_trend,
    }
}

/// Encode shot statistics into 16 dims.
fn encode_shot_stats(stats: &ShotStats, total_frames: usize) -> Vec<f32> {
    let tf = total_frames as f32;
    vec![
        stats.count as f32 / tf.max(1.0),        // normalized shot count
        stats.mean_length / tf.max(1.0),          // normalized mean length
        stats.std_length / tf.max(1.0),           // normalized std
        stats.max_length / tf.max(1.0),           // normalized max
        stats.regularity,                          // cut rhythm regularity
        (stats.count as f32).ln().max(0.0) / 5.0, // log shot count
        if stats.count > 1 { 1.0 } else { 0.0 },  // has cuts
        stats.mean_length.recip().min(1.0),        // cut frequency
        // Pad to 16
        0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ]
}

/// Color flow: temporal derivative of HSV histograms (24 dims).
/// Measures how the color palette evolves over time.
fn color_flow(per_frame: &[Vec<f32>]) -> Vec<f32> {
    let hist_dim = 48.min(per_frame[0].len());
    let n = per_frame.len();

    if n < 2 {
        return vec![0.0; 24];
    }

    // Compute frame-to-frame histogram differences
    let mut diffs: Vec<Vec<f32>> = Vec::with_capacity(n - 1);
    for i in 1..n {
        let diff: Vec<f32> = (0..hist_dim)
            .map(|d| per_frame[i][d] - per_frame[i - 1][d])
            .collect();
        diffs.push(diff);
    }

    // Mean and std of color velocity per HSV channel (6 dims each, 18 total)
    let mut result = Vec::with_capacity(24);
    let channels = [0..16, 16..32, 32..48.min(hist_dim)]; // H, S, V

    for ch_range in &channels {
        let ch_velocities: Vec<f32> = diffs
            .iter()
            .map(|d| {
                ch_range.clone()
                    .filter(|&i| i < d.len())
                    .map(|i| d[i].abs())
                    .sum::<f32>()
            })
            .collect();

        let mean = ch_velocities.iter().sum::<f32>() / ch_velocities.len() as f32;
        let var = ch_velocities.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / ch_velocities.len() as f32;
        result.push(mean);
        result.push(var.sqrt());
    }

    // Autocorrelation at 3 lags (6 dims)
    let total_velocity: Vec<f32> = diffs
        .iter()
        .map(|d| d.iter().map(|v| v.abs()).sum::<f32>())
        .collect();

    for lag in 1..=3 {
        result.push(autocorrelation(&total_velocity, lag));
        result.push(autocorrelation(&total_velocity, lag * 2));
    }

    result.truncate(24);
    while result.len() < 24 {
        result.push(0.0);
    }
    result
}

/// Motion trajectory from spatial features (24 dims).
fn motion_trajectory(per_frame: &[Vec<f32>]) -> Vec<f32> {
    // Use the frame-to-frame L2 difference of the full spatial vector as a proxy for motion
    let n = per_frame.len();
    if n < 2 {
        return vec![0.0; 24];
    }

    let dim = per_frame[0].len();
    let frame_diffs: Vec<f32> = (1..n)
        .map(|i| {
            let d: f32 = (0..dim)
                .map(|j| (per_frame[i][j] - per_frame[i - 1][j]).powi(2))
                .sum();
            d.sqrt()
        })
        .collect();

    let mean = frame_diffs.iter().sum::<f32>() / frame_diffs.len() as f32;
    let var = frame_diffs.iter().map(|d| (d - mean).powi(2)).sum::<f32>() / frame_diffs.len() as f32;
    let max = frame_diffs.iter().cloned().fold(0.0f32, f32::max);

    // Acceleration: differences of differences
    let accel: Vec<f32> = (1..frame_diffs.len())
        .map(|i| (frame_diffs[i] - frame_diffs[i - 1]).abs())
        .collect();
    let accel_mean = if accel.is_empty() { 0.0 } else { accel.iter().sum::<f32>() / accel.len() as f32 };
    let accel_max = accel.iter().cloned().fold(0.0f32, f32::max);

    // Per-quarter dominant direction (simplified: mean diff per quarter)
    let quarter = frame_diffs.len() / 4;
    let quarter_means: Vec<f32> = (0..4)
        .map(|q| {
            let start = q * quarter;
            let end = ((q + 1) * quarter).min(frame_diffs.len());
            if end > start {
                frame_diffs[start..end].iter().sum::<f32>() / (end - start) as f32
            } else {
                0.0
            }
        })
        .collect();

    let mut result = vec![
        mean, var.sqrt(), max,
        accel_mean, accel_max,
    ];
    result.extend_from_slice(&quarter_means);

    // Pad to 24
    while result.len() < 24 {
        result.push(0.0);
    }
    result.truncate(24);
    result
}

/// Visual tempo: onset detection on frame-difference signal (16 dims).
fn visual_tempo(per_frame: &[Vec<f32>]) -> (Vec<f32>, f32) {
    let n = per_frame.len();
    if n < 4 {
        return (vec![0.0; 16], 0.0);
    }

    // Frame-to-frame total difference (the "visual energy" signal)
    let energy: Vec<f32> = (1..n)
        .map(|i| {
            let dim = per_frame[0].len().min(per_frame[i].len());
            (0..dim)
                .map(|j| (per_frame[i][j] - per_frame[i - 1][j]).abs())
                .sum::<f32>()
        })
        .collect();

    // Peak detection: find local maxima
    let mut peaks: Vec<usize> = Vec::new();
    let mean_energy = energy.iter().sum::<f32>() / energy.len() as f32;
    for i in 1..energy.len() - 1 {
        if energy[i] > energy[i - 1] && energy[i] > energy[i + 1] && energy[i] > mean_energy {
            peaks.push(i);
        }
    }

    // Inter-peak intervals
    let intervals: Vec<f32> = (1..peaks.len())
        .map(|i| (peaks[i] - peaks[i - 1]) as f32)
        .collect();

    let mean_interval = if intervals.is_empty() {
        0.0
    } else {
        intervals.iter().sum::<f32>() / intervals.len() as f32
    };

    // Visual BPM (based on analysis FPS, default 2)
    let bpm = if mean_interval > 0.0 {
        60.0 * 2.0 / mean_interval // assumes 2 fps
    } else {
        0.0
    };

    // Regularity of intervals
    let interval_var = if intervals.len() > 1 {
        let var = intervals.iter().map(|i| (i - mean_interval).powi(2)).sum::<f32>() / intervals.len() as f32;
        var.sqrt()
    } else {
        0.0
    };
    let regularity = if mean_interval > 0.0 {
        (1.0 - interval_var / mean_interval).clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Energy statistics
    let e_mean = energy.iter().sum::<f32>() / energy.len() as f32;
    let e_var = energy.iter().map(|e| (e - e_mean).powi(2)).sum::<f32>() / energy.len() as f32;
    let e_max = energy.iter().cloned().fold(0.0f32, f32::max);

    let mut result = vec![
        bpm / 200.0,                // normalized BPM
        regularity,                  // tempo regularity
        peaks.len() as f32 / n as f32, // peak density
        mean_interval / n as f32,    // normalized interval
        e_mean,                      // mean energy
        e_var.sqrt(),                // energy std
        e_max,                       // peak energy
        autocorrelation(&energy, 1), // lag-1 autocorrelation
    ];

    // Pad to 16
    while result.len() < 16 {
        result.push(0.0);
    }
    result.truncate(16);

    (result, bpm)
}

/// Complexity evolution (16 dims) — how visual complexity changes over time.
fn complexity_evolution(per_frame: &[Vec<f32>]) -> Vec<f32> {
    // Use edge density (indices 48+32=80 to 80+36=116) as complexity proxy
    let edge_start = 80.min(per_frame[0].len());
    let edge_end = 116.min(per_frame[0].len());

    if edge_start >= edge_end {
        return vec![0.0; 16];
    }

    let complexity: Vec<f32> = per_frame
        .iter()
        .map(|f| f[edge_start..edge_end].iter().sum::<f32>())
        .collect();

    let n = complexity.len() as f32;
    let mean = complexity.iter().sum::<f32>() / n;
    let var = complexity.iter().map(|c| (c - mean).powi(2)).sum::<f32>() / n;

    // Linear trend
    let trend = linear_trend(&complexity);

    // Quartile means
    let q = complexity.len() / 4;
    let q_means: Vec<f32> = (0..4)
        .map(|i| {
            let s = i * q;
            let e = ((i + 1) * q).min(complexity.len());
            if e > s { complexity[s..e].iter().sum::<f32>() / (e - s) as f32 } else { 0.0 }
        })
        .collect();

    let mut result = vec![
        mean, var.sqrt(), trend,
        complexity.first().copied().unwrap_or(0.0),
        complexity.last().copied().unwrap_or(0.0),
    ];
    result.extend_from_slice(&q_means);

    while result.len() < 16 {
        result.push(0.0);
    }
    result.truncate(16);
    result
}

/// Brightness arc (16 dims) — luminance trajectory over time.
fn brightness_arc(per_frame: &[Vec<f32>]) -> (Vec<f32>, f32) {
    // Mean brightness is at index 168 (after HSV+freq+edges+regions+flow)
    let bright_idx = 168.min(per_frame[0].len().saturating_sub(1));

    let brightness: Vec<f32> = per_frame
        .iter()
        .map(|f| if bright_idx < f.len() { f[bright_idx] } else { 0.5 })
        .collect();

    let n = brightness.len() as f32;
    let mean = brightness.iter().sum::<f32>() / n;
    let var = brightness.iter().map(|b| (b - mean).powi(2)).sum::<f32>() / n;
    let min = brightness.iter().cloned().fold(f32::MAX, f32::min);
    let max = brightness.iter().cloned().fold(f32::MIN, f32::max);
    let trend = linear_trend(&brightness);
    let opening = brightness.first().copied().unwrap_or(0.5);
    let closing = brightness.last().copied().unwrap_or(0.5);

    let mut result = vec![
        opening, closing, mean, var.sqrt(),
        min, max, max - min, trend,
    ];

    // Quartile means
    let q = brightness.len() / 4;
    for i in 0..4 {
        let s = i * q;
        let e = ((i + 1) * q).min(brightness.len());
        result.push(if e > s {
            brightness[s..e].iter().sum::<f32>() / (e - s) as f32
        } else {
            0.0
        });
    }

    while result.len() < 16 {
        result.push(0.0);
    }
    result.truncate(16);

    (result, trend)
}

/// Stillness ratio (8 dims) — fraction of low-motion frames.
fn stillness_ratio(per_frame: &[Vec<f32>]) -> Vec<f32> {
    let n = per_frame.len();
    if n < 2 {
        return vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    }

    let dim = per_frame[0].len();
    let diffs: Vec<f32> = (1..n)
        .map(|i| {
            (0..dim)
                .map(|j| (per_frame[i][j] - per_frame[i - 1][j]).abs())
                .sum::<f32>()
        })
        .collect();

    let mean_diff = diffs.iter().sum::<f32>() / diffs.len() as f32;
    let threshold = mean_diff * 0.3; // "still" = less than 30% of mean motion

    let still_count = diffs.iter().filter(|&&d| d < threshold).count();
    let still_ratio = still_count as f32 / diffs.len() as f32;

    // Longest still segment
    let mut max_still = 0usize;
    let mut current_still = 0usize;
    for &d in &diffs {
        if d < threshold {
            current_still += 1;
            max_still = max_still.max(current_still);
        } else {
            current_still = 0;
        }
    }

    vec![
        still_ratio,
        max_still as f32 / n as f32,
        mean_diff,
        diffs.iter().cloned().fold(0.0f32, f32::max),
        diffs.iter().cloned().fold(f32::MAX, f32::min),
        threshold,
        0.0, 0.0,
    ]
}

/// Entropy flow (8 dims) — Shannon entropy of frame features over time.
fn entropy_flow(per_frame: &[Vec<f32>]) -> Vec<f32> {
    // Compute per-frame entropy of the HSV histogram (first 48 dims)
    let hist_dim = 48.min(per_frame[0].len());

    let entropies: Vec<f32> = per_frame
        .iter()
        .map(|f| {
            let hist = &f[..hist_dim];
            let sum: f32 = hist.iter().sum();
            if sum <= 0.0 {
                return 0.0;
            }
            -hist.iter()
                .map(|&p| {
                    let prob = p / sum;
                    if prob > 0.0 { prob * prob.ln() } else { 0.0 }
                })
                .sum::<f32>()
        })
        .collect();

    let n = entropies.len() as f32;
    let mean = entropies.iter().sum::<f32>() / n;
    let var = entropies.iter().map(|e| (e - mean).powi(2)).sum::<f32>() / n;
    let trend = linear_trend(&entropies);
    let range = entropies.iter().cloned().fold(f32::MIN, f32::max)
        - entropies.iter().cloned().fold(f32::MAX, f32::min);

    vec![
        mean, var.sqrt(), trend, range,
        entropies.first().copied().unwrap_or(0.0),
        entropies.last().copied().unwrap_or(0.0),
        0.0, 0.0,
    ]
}

// ── Helpers ──────────────────────────────────────────────

/// Simple autocorrelation at a given lag.
fn autocorrelation(signal: &[f32], lag: usize) -> f32 {
    let n = signal.len();
    if lag >= n || n < 2 {
        return 0.0;
    }

    let mean = signal.iter().sum::<f32>() / n as f32;
    let var: f32 = signal.iter().map(|s| (s - mean).powi(2)).sum();

    if var < 1e-10 {
        return 0.0;
    }

    let cov: f32 = (0..n - lag)
        .map(|i| (signal[i] - mean) * (signal[i + lag] - mean))
        .sum();

    cov / var
}

/// Linear trend (slope of best-fit line, normalized).
fn linear_trend(values: &[f32]) -> f32 {
    let n = values.len() as f32;
    if n < 2.0 {
        return 0.0;
    }

    let x_mean = (n - 1.0) / 2.0;
    let y_mean = values.iter().sum::<f32>() / n;

    let mut num = 0.0f32;
    let mut den = 0.0f32;

    for (i, &v) in values.iter().enumerate() {
        let xi = i as f32 - x_mean;
        num += xi * (v - y_mean);
        den += xi * xi;
    }

    if den.abs() < 1e-10 {
        0.0
    } else {
        num / den
    }
}
