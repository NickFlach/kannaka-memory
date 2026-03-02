//! Per-frame spatial feature extraction (192 dims) and aggregation.
//!
//! Features: HSV histogram (48), spatial frequency (32), edge orientation (36),
//! region statistics (20), optical flow magnitude (32), contrast/brightness (8),
//! dominant colors (16).

use super::color::{hsv_histogram, dominant_colors};
use super::decode::FrameInfo;
use super::{HSV_BINS, EDGE_BINS, FREQ_BANDS, REGION_GRID, FLOW_BINS, SPATIAL_FEATURE_DIM};

/// Aggregated spatial features across all frames.
#[derive(Debug, Clone)]
pub struct SpatialFeatures {
    /// 192-dim vector (mean of per-frame features).
    pub vector: Vec<f32>,
    /// Mean brightness (0-255).
    pub mean_brightness: f32,
    /// Mean contrast.
    pub mean_contrast: f32,
}

/// Extract per-frame spatial features (192 dims).
pub fn extract_frame_features(frame: &FrameInfo) -> Vec<f32> {
    let mut features = Vec::with_capacity(SPATIAL_FEATURE_DIM);

    // 1. HSV histogram (48 dims: 16 bins × 3 channels)
    let hsv_hist = hsv_histogram(&frame.rgb, frame.width, frame.height);
    features.extend_from_slice(&hsv_hist);

    // 2. Spatial frequency via simple block variance (32 dims)
    let freq = spatial_frequency(frame);
    features.extend_from_slice(&freq);

    // 3. Edge orientation histogram (36 dims)
    let edges = edge_orientation(frame);
    features.extend_from_slice(&edges);

    // 4. Region statistics (20 dims: 4×5 grid mean luminance)
    let regions = region_statistics(frame);
    features.extend_from_slice(&regions);

    // 5. Optical flow placeholder (32 dims — filled with zeros for single frame)
    // Actual flow computed between frame pairs in temporal features
    features.extend(std::iter::repeat(0.0f32).take(FLOW_BINS));

    // 6. Contrast/brightness (8 dims)
    let cb = contrast_brightness(frame);
    features.extend_from_slice(&cb);

    // 7. Dominant colors (16 dims: 4 colors × HSV + weight)
    let dc = dominant_colors(&frame.rgb, frame.width, frame.height, 4);
    // dc is 12 dims (4 × 3 HSV), pad to 16
    features.extend_from_slice(&dc);
    let remaining = 16 - dc.len().min(16);
    features.extend(std::iter::repeat(0.0f32).take(remaining));

    // Ensure exact dimension
    features.truncate(SPATIAL_FEATURE_DIM);
    while features.len() < SPATIAL_FEATURE_DIM {
        features.push(0.0);
    }

    features
}

/// Spatial frequency approximation using block variance of luminance.
/// Divides frame into blocks and computes variance per block, then
/// bins into frequency bands.
fn spatial_frequency(frame: &FrameInfo) -> Vec<f32> {
    let block_size = 8u32;
    let bx = (frame.width / block_size).max(1);
    let by = (frame.height / block_size).max(1);

    let mut variances: Vec<f32> = Vec::new();

    for by_idx in 0..by {
        for bx_idx in 0..bx {
            let ox = bx_idx * block_size;
            let oy = by_idx * block_size;

            let mut sum = 0.0f32;
            let mut sum_sq = 0.0f32;
            let mut count = 0.0f32;

            for py in 0..block_size.min(frame.height - oy) {
                for px in 0..block_size.min(frame.width - ox) {
                    let l = frame.luminance(ox + px, oy + py);
                    sum += l;
                    sum_sq += l * l;
                    count += 1.0;
                }
            }

            if count > 1.0 {
                let mean = sum / count;
                let var = (sum_sq / count) - mean * mean;
                variances.push(var.max(0.0));
            }
        }
    }

    // Bin variances into FREQ_BANDS
    bin_values(&variances, FREQ_BANDS)
}

/// Sobel edge orientation histogram (36 bins × 10°).
fn edge_orientation(frame: &FrameInfo) -> Vec<f32> {
    let mut hist = vec![0.0f32; EDGE_BINS];
    let w = frame.width;
    let h = frame.height;

    if w < 3 || h < 3 {
        return hist;
    }

    // Sample every 2nd pixel for speed
    let step = 2u32;
    let mut count = 0.0f32;

    for y in (1..h - 1).step_by(step as usize) {
        for x in (1..w - 1).step_by(step as usize) {
            // Sobel gradients
            let gx = -frame.luminance(x - 1, y - 1) - 2.0 * frame.luminance(x - 1, y) - frame.luminance(x - 1, y + 1)
                   + frame.luminance(x + 1, y - 1) + 2.0 * frame.luminance(x + 1, y) + frame.luminance(x + 1, y + 1);

            let gy = -frame.luminance(x - 1, y - 1) - 2.0 * frame.luminance(x, y - 1) - frame.luminance(x + 1, y - 1)
                   + frame.luminance(x - 1, y + 1) + 2.0 * frame.luminance(x, y + 1) + frame.luminance(x + 1, y + 1);

            let mag = (gx * gx + gy * gy).sqrt();
            if mag > 10.0 {
                // Angle in [0, π) → bin
                let angle = gy.atan2(gx); // [-π, π]
                let angle_pos = if angle < 0.0 { angle + std::f32::consts::PI } else { angle };
                let bin = ((angle_pos / std::f32::consts::PI * EDGE_BINS as f32) as usize).min(EDGE_BINS - 1);
                hist[bin] += mag;
                count += mag;
            }
        }
    }

    // Normalize
    if count > 0.0 {
        for h in hist.iter_mut() {
            *h /= count;
        }
    }

    hist
}

/// Region statistics: mean luminance in a REGION_GRID spatial grid.
fn region_statistics(frame: &FrameInfo) -> Vec<f32> {
    let (rows, cols) = REGION_GRID;
    let rh = (frame.height / rows as u32).max(1);
    let rw = (frame.width / cols as u32).max(1);

    let mut stats = Vec::with_capacity(rows * cols);

    for r in 0..rows {
        for c in 0..cols {
            let ox = c as u32 * rw;
            let oy = r as u32 * rh;
            let mut sum = 0.0f32;
            let mut count = 0.0f32;

            for py in 0..rh.min(frame.height - oy) {
                for px in 0..rw.min(frame.width - ox) {
                    sum += frame.luminance(ox + px, oy + py);
                    count += 1.0;
                }
            }

            stats.push(if count > 0.0 { sum / count / 255.0 } else { 0.0 });
        }
    }

    stats
}

/// Contrast and brightness statistics (8 dims).
fn contrast_brightness(frame: &FrameInfo) -> Vec<f32> {
    let npx = frame.pixel_count();
    if npx == 0 {
        return vec![0.0; 8];
    }

    let mut min_l = f32::MAX;
    let mut max_l = f32::MIN;
    let mut sum = 0.0f32;
    let mut sum_sq = 0.0f32;

    // Sample for speed
    let step = (npx / 5000).max(1);
    let mut count = 0.0f32;

    for i in (0..npx).step_by(step) {
        let x = (i % frame.width as usize) as u32;
        let y = (i / frame.width as usize) as u32;
        let l = frame.luminance(x, y);
        sum += l;
        sum_sq += l * l;
        if l < min_l { min_l = l; }
        if l > max_l { max_l = l; }
        count += 1.0;
    }

    let mean = sum / count;
    let variance = (sum_sq / count) - mean * mean;
    let std = variance.max(0.0).sqrt();

    // Local contrast: std of luminance in center region
    let cx = frame.width / 4;
    let cy = frame.height / 4;
    let cw = frame.width / 2;
    let ch = frame.height / 2;
    let mut center_sum = 0.0f32;
    let mut center_sq = 0.0f32;
    let mut center_count = 0.0f32;
    let center_step = ((cw * ch) as usize / 1000).max(1);

    for i in (0..(cw * ch) as usize).step_by(center_step) {
        let lx = cx + (i as u32 % cw);
        let ly = cy + (i as u32 / cw);
        if lx < frame.width && ly < frame.height {
            let l = frame.luminance(lx, ly);
            center_sum += l;
            center_sq += l * l;
            center_count += 1.0;
        }
    }

    let center_mean = if center_count > 0.0 { center_sum / center_count } else { mean };
    let center_var = if center_count > 1.0 { (center_sq / center_count) - center_mean * center_mean } else { 0.0 };
    let local_contrast = center_var.max(0.0).sqrt();

    vec![
        mean / 255.0,           // normalized mean brightness
        std / 128.0,            // normalized std
        min_l / 255.0,          // min
        max_l / 255.0,          // max
        (max_l - min_l) / 255.0, // range
        local_contrast / 128.0, // local contrast
        center_mean / 255.0,    // center brightness
        variance / (128.0 * 128.0), // normalized variance
    ]
}

/// Aggregate per-frame spatial features into a single 192-dim vector.
/// Uses mean across all frames.
pub fn aggregate_spatial(per_frame: &[Vec<f32>]) -> SpatialFeatures {
    let dim = per_frame[0].len();
    let n = per_frame.len() as f32;
    let mut mean_vec = vec![0.0f32; dim];

    for frame_features in per_frame {
        for (i, &v) in frame_features.iter().enumerate() {
            mean_vec[i] += v;
        }
    }
    for v in mean_vec.iter_mut() {
        *v /= n;
    }

    // Extract brightness/contrast from the aggregated contrast_brightness section
    // Indices: after HSV(48) + freq(32) + edges(36) + regions(20) + flow(32) = 168
    let brightness_idx = 168;
    let mean_brightness = if dim > brightness_idx {
        mean_vec[brightness_idx] * 255.0
    } else {
        128.0
    };
    let mean_contrast = if dim > brightness_idx + 5 {
        mean_vec[brightness_idx + 5] * 128.0
    } else {
        0.0
    };

    SpatialFeatures {
        vector: mean_vec,
        mean_brightness,
        mean_contrast,
    }
}

/// Bin a slice of values into `n_bins` using histogram.
fn bin_values(values: &[f32], n_bins: usize) -> Vec<f32> {
    if values.is_empty() {
        return vec![0.0; n_bins];
    }

    let max = values.iter().cloned().fold(0.0f32, f32::max).max(1e-6);
    let mut hist = vec![0.0f32; n_bins];

    for &v in values {
        let bin = ((v / max * n_bins as f32) as usize).min(n_bins - 1);
        hist[bin] += 1.0;
    }

    // Normalize
    let total: f32 = hist.iter().sum();
    if total > 0.0 {
        for h in hist.iter_mut() {
            *h /= total;
        }
    }

    hist
}
