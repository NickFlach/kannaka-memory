//! Color analysis utilities: RGB→HSV, histograms, dominant colors.

use super::HSV_BINS;

/// HSV color (H: 0-360, S: 0-1, V: 0-1).
#[derive(Debug, Clone, Copy)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

/// Convert RGB (0-255) to HSV.
pub fn rgb_to_hsv(r: u8, g: u8, b: u8) -> Hsv {
    let rf = r as f32 / 255.0;
    let gf = g as f32 / 255.0;
    let bf = b as f32 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;

    let v = max;
    let s = if max > 0.0 { delta / max } else { 0.0 };

    let h = if delta < 1e-6 {
        0.0
    } else if (max - rf).abs() < 1e-6 {
        60.0 * (((gf - bf) / delta) % 6.0)
    } else if (max - gf).abs() < 1e-6 {
        60.0 * ((bf - rf) / delta + 2.0)
    } else {
        60.0 * ((rf - gf) / delta + 4.0)
    };

    let h = if h < 0.0 { h + 360.0 } else { h };

    Hsv { h, s, v }
}

/// Compute HSV histogram from a frame's RGB pixels.
/// Returns `HSV_BINS * 3` bins (H, S, V each with HSV_BINS bins).
pub fn hsv_histogram(rgb: &[u8], width: u32, height: u32) -> Vec<f32> {
    let npx = (width * height) as usize;
    let mut h_hist = vec![0.0f32; HSV_BINS];
    let mut s_hist = vec![0.0f32; HSV_BINS];
    let mut v_hist = vec![0.0f32; HSV_BINS];

    for i in 0..npx {
        let base = i * 3;
        let hsv = rgb_to_hsv(rgb[base], rgb[base + 1], rgb[base + 2]);

        let h_bin = ((hsv.h / 360.0 * HSV_BINS as f32) as usize).min(HSV_BINS - 1);
        let s_bin = ((hsv.s * HSV_BINS as f32) as usize).min(HSV_BINS - 1);
        let v_bin = ((hsv.v * HSV_BINS as f32) as usize).min(HSV_BINS - 1);

        h_hist[h_bin] += 1.0;
        s_hist[s_bin] += 1.0;
        v_hist[v_bin] += 1.0;
    }

    // Normalize
    let total = npx as f32;
    for v in h_hist.iter_mut().chain(s_hist.iter_mut()).chain(v_hist.iter_mut()) {
        *v /= total;
    }

    let mut result = Vec::with_capacity(HSV_BINS * 3);
    result.extend_from_slice(&h_hist);
    result.extend_from_slice(&s_hist);
    result.extend_from_slice(&v_hist);
    result
}

/// Extract top-K dominant colors via simple k-means in HSV space.
/// Returns K * 3 values (H, S, V for each centroid), normalized to 0-1 range.
pub fn dominant_colors(rgb: &[u8], width: u32, height: u32, k: usize) -> Vec<f32> {
    let npx = (width * height) as usize;
    if npx == 0 {
        return vec![0.0; k * 3];
    }

    // Sample pixels for efficiency (max 1000)
    let step = (npx / 1000).max(1);
    let mut samples: Vec<Hsv> = Vec::new();
    for i in (0..npx).step_by(step) {
        let base = i * 3;
        samples.push(rgb_to_hsv(rgb[base], rgb[base + 1], rgb[base + 2]));
    }

    // Initialize centroids evenly spaced
    let mut centroids: Vec<Hsv> = (0..k)
        .map(|i| {
            let idx = i * samples.len() / k;
            samples[idx.min(samples.len() - 1)]
        })
        .collect();

    // K-means iterations
    for _ in 0..10 {
        let mut sums = vec![(0.0f32, 0.0f32, 0.0f32); k];
        let mut counts = vec![0usize; k];

        for s in &samples {
            let mut best = 0;
            let mut best_dist = f32::MAX;
            for (ci, c) in centroids.iter().enumerate() {
                // Simple Euclidean distance in HSV (H normalized to 0-1)
                let dh = (s.h / 360.0 - c.h / 360.0).abs().min(1.0 - (s.h / 360.0 - c.h / 360.0).abs());
                let ds = s.s - c.s;
                let dv = s.v - c.v;
                let dist = dh * dh + ds * ds + dv * dv;
                if dist < best_dist {
                    best_dist = dist;
                    best = ci;
                }
            }
            sums[best].0 += s.h;
            sums[best].1 += s.s;
            sums[best].2 += s.v;
            counts[best] += 1;
        }

        for i in 0..k {
            if counts[i] > 0 {
                let n = counts[i] as f32;
                centroids[i] = Hsv {
                    h: sums[i].0 / n,
                    s: sums[i].1 / n,
                    v: sums[i].2 / n,
                };
            }
        }
    }

    // Sort by count (most dominant first) — approximate by V value
    centroids.sort_by(|a, b| b.v.total_cmp(&a.v));

    let mut result = Vec::with_capacity(k * 3);
    for c in &centroids {
        result.push(c.h / 360.0); // normalize H to 0-1
        result.push(c.s);
        result.push(c.v);
    }
    // Pad if fewer than k clusters
    while result.len() < k * 3 {
        result.push(0.0);
    }
    result
}
