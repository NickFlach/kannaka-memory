//! Optical flow estimation — simple block-matching approach.
//!
//! No OpenCV dependency. We compute approximate motion vectors between
//! consecutive frames using a block-matching algorithm.

use super::decode::FrameInfo;
use super::FLOW_BINS;

/// Motion vector (dx, dy) in pixels.
#[derive(Debug, Clone, Copy)]
pub struct MotionVector {
    pub dx: f32,
    pub dy: f32,
}

impl MotionVector {
    pub fn magnitude(&self) -> f32 {
        (self.dx * self.dx + self.dy * self.dy).sqrt()
    }

    pub fn angle(&self) -> f32 {
        self.dy.atan2(self.dx)
    }
}

/// Compute global motion between two frames using block matching.
///
/// Divides frame into blocks and finds best match in search window.
/// Returns per-block motion vectors.
pub fn block_motion(prev: &FrameInfo, curr: &FrameInfo) -> Vec<MotionVector> {
    let block_size: u32 = 16;
    let search_range: i32 = 8;

    let bx = prev.width / block_size;
    let by = prev.height / block_size;
    let mut vectors = Vec::with_capacity((bx * by) as usize);

    for by_idx in 0..by {
        for bx_idx in 0..bx {
            let ox = bx_idx * block_size;
            let oy = by_idx * block_size;

            let mut best_dx: i32 = 0;
            let mut best_dy: i32 = 0;
            let mut best_sad = u64::MAX;

            // Search window
            for dy in -search_range..=search_range {
                for dx in -search_range..=search_range {
                    let sx = ox as i32 + dx;
                    let sy = oy as i32 + dy;

                    // Bounds check
                    if sx < 0 || sy < 0
                        || (sx + block_size as i32) > curr.width as i32
                        || (sy + block_size as i32) > curr.height as i32
                    {
                        continue;
                    }

                    // Sum of absolute differences
                    let mut sad: u64 = 0;
                    for py in 0..block_size {
                        for px in 0..block_size {
                            let prev_lum = prev.luminance(ox + px, oy + py) as u64;
                            let curr_lum = curr.luminance(
                                (sx + px as i32) as u32,
                                (sy + py as i32) as u32,
                            ) as u64;
                            sad += if prev_lum > curr_lum {
                                prev_lum - curr_lum
                            } else {
                                curr_lum - prev_lum
                            };
                        }
                    }

                    if sad < best_sad {
                        best_sad = sad;
                        best_dx = dx;
                        best_dy = dy;
                    }
                }
            }

            vectors.push(MotionVector {
                dx: best_dx as f32,
                dy: best_dy as f32,
            });
        }
    }

    vectors
}

/// Compute optical flow magnitude histogram from motion vectors.
/// Returns `FLOW_BINS` bins of radially-binned motion energy.
pub fn flow_histogram(vectors: &[MotionVector]) -> Vec<f32> {
    if vectors.is_empty() {
        return vec![0.0; FLOW_BINS];
    }

    let max_mag = vectors.iter().map(|v| v.magnitude()).fold(0.0f32, f32::max).max(1.0);
    let mut hist = vec![0.0f32; FLOW_BINS];

    for v in vectors {
        let mag = v.magnitude();
        let bin = ((mag / max_mag * FLOW_BINS as f32) as usize).min(FLOW_BINS - 1);
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

/// Compute aggregate motion statistics from a sequence of frame-pair motion vectors.
pub fn motion_stats(all_vectors: &[Vec<MotionVector>]) -> (f32, f32, f32) {
    // mean, std, max of per-frame average magnitude
    let per_frame_mag: Vec<f32> = all_vectors
        .iter()
        .map(|vecs| {
            if vecs.is_empty() {
                0.0
            } else {
                let sum: f32 = vecs.iter().map(|v| v.magnitude()).sum();
                sum / vecs.len() as f32
            }
        })
        .collect();

    let n = per_frame_mag.len() as f32;
    if n < 1.0 {
        return (0.0, 0.0, 0.0);
    }

    let mean = per_frame_mag.iter().sum::<f32>() / n;
    let variance = per_frame_mag.iter().map(|m| (m - mean).powi(2)).sum::<f32>() / n;
    let std = variance.sqrt();
    let max = per_frame_mag.iter().cloned().fold(0.0f32, f32::max);

    (mean, std, max)
}
