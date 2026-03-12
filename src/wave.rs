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
    compute_strength_with_retrieval(params, age_seconds, 0)
}

/// Compute effective strength with retrieval energy (EXP-003):
/// S(t) = (A + retrieval_energy) · cos(2πf·t + φ) · e^(-λt)
///
/// Each retrieval adds diminishing energy: energy = 0.05 · ln(1 + retrieval_count)
/// This makes retrieval a generative f(x) term in the dx/dt = f(x) - λx system.
pub fn compute_strength_with_retrieval(params: &WaveParams, age_seconds: f64, retrieval_count: u32) -> f32 {
    let retrieval_energy = 0.05 * (1.0 + retrieval_count as f64).ln();
    let a = params.amplitude as f64 + retrieval_energy;
    let f = params.frequency as f64;
    let phi = params.phase as f64;
    let lambda = params.decay_rate as f64;

    let wave = (2.0 * PI * f * age_seconds + phi).cos();
    let decay = (-lambda * age_seconds).exp();
    (a * wave * decay) as f32
}

/// Cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() {
        if a.is_empty() || b.is_empty() {
            eprintln!("[warn] cosine_similarity called with empty vector (missing embeddings?)");
        }
        return 0.0;
    }
    if a.len() != b.len() {
        return 0.0;
    }
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

    // EXP-003: Retrieval energy tests

    #[test]
    fn retrieval_energy_boosts_strength() {
        let params = WaveParams {
            amplitude: 1.0,
            frequency: 0.0,
            phase: 0.0,
            decay_rate: 0.001,
        };
        let s_no_retrieval = compute_strength_with_retrieval(&params, 100.0, 0);
        let s_with_retrieval = compute_strength_with_retrieval(&params, 100.0, 10);
        assert!(s_with_retrieval > s_no_retrieval,
            "retrieval should boost strength: {} vs {}", s_with_retrieval, s_no_retrieval);
    }

    #[test]
    fn retrieval_energy_has_diminishing_returns() {
        let params = WaveParams {
            amplitude: 1.0,
            frequency: 0.0,
            phase: 0.0,
            decay_rate: 0.0,
        };
        let boost_1_to_10 = compute_strength_with_retrieval(&params, 0.0, 10)
            - compute_strength_with_retrieval(&params, 0.0, 1);
        let boost_100_to_110 = compute_strength_with_retrieval(&params, 0.0, 110)
            - compute_strength_with_retrieval(&params, 0.0, 100);
        assert!(boost_1_to_10 > boost_100_to_110,
            "later retrievals should have less effect: {} vs {}", boost_1_to_10, boost_100_to_110);
    }

    #[test]
    fn zero_retrieval_matches_original() {
        let params = WaveParams {
            amplitude: 1.0,
            frequency: 0.1,
            phase: 0.5,
            decay_rate: 0.001,
        };
        let s_original = compute_strength(&params, 500.0);
        let s_zero = compute_strength_with_retrieval(&params, 500.0, 0);
        assert!((s_original - s_zero).abs() < 1e-6,
            "zero retrievals should match original: {} vs {}", s_original, s_zero);
    }
}
