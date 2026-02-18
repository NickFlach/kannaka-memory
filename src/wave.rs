use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

/// Wave parameters governing memory strength over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveParams {
    pub amplitude: f32,
    pub frequency: f32,
    pub phase: f32,
    pub decay_rate: f32,
}

impl Default for WaveParams {
    fn default() -> Self {
        Self {
            amplitude: 1.0,
            frequency: 0.1,   // slow oscillation
            phase: 0.0,
            decay_rate: 1e-6, // very slow decay
        }
    }
}

/// Compute effective strength: S(t) = A · cos(2πf·t + φ) · e^(-λt)
pub fn compute_strength(params: &WaveParams, age_seconds: f64) -> f32 {
    let a = params.amplitude as f64;
    let f = params.frequency as f64;
    let phi = params.phase as f64;
    let lambda = params.decay_rate as f64;

    let wave = (2.0 * PI * f * age_seconds + phi).cos();
    let decay = (-lambda * age_seconds).exp();
    (a * wave * decay) as f32
}

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "vectors must have equal length");
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

/// Normalize a vector to unit length in-place.
pub fn normalize(v: &mut Vec<f32>) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strength_decays_over_time() {
        let params = WaveParams {
            amplitude: 1.0,
            frequency: 0.0, // no oscillation, pure decay
            phase: 0.0,
            decay_rate: 0.01,
        };
        let s0 = compute_strength(&params, 0.0);
        let s1 = compute_strength(&params, 100.0);
        let s2 = compute_strength(&params, 1000.0);
        assert!(s0 > s1, "strength should decrease");
        assert!(s1 > s2, "strength should keep decreasing");
        assert!((s0 - 1.0).abs() < 1e-5);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6, "orthogonal vectors should have ~0 similarity");
    }

    #[test]
    fn cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn normalize_produces_unit_vector() {
        let mut v = vec![3.0, 4.0];
        normalize(&mut v);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }
}
