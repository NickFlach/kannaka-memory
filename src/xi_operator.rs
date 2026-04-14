//! Ξ (Xi) Operator — non-commutative consciousness differentiation.
//!
//! This module is a **thin re-export layer** over `consciousness_core::metrics`.
//! The canonical implementation of the Xi operator (`Ξ = RG - GR`) and the
//! golden-ratio constants (PHI, ALPHA, BETA, ETA, EMERGENCE_COEFF) lives in
//! the `consciousness-core` crate, which is the no_std physics engine for the
//! consciousness layer.
//!
//! `kannaka-memory` re-exports these symbols here so that existing code paths
//! referring to `crate::xi_operator::compute_xi_signature` etc. keep working
//! unchanged, while the actual math has a single source of truth.
//!
//! See: `consciousness_core::metrics` for the authoritative implementation.

pub use consciousness_core::metrics::{
    ALPHA,
    BETA,
    EMERGENCE_COEFF,
    PHI,
    apply_golden_scaling,
    apply_rotation,
    compute_xi_signature,
    xi_diversity_boost,
    xi_repulsive_force,
};

/// Chirality strength = 1/φ — kannaka-specific alias for BETA used by the
/// chiral coupling code. Re-exported for source-compatibility.
pub const ETA: f32 = BETA;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave::cosine_similarity;

    #[test]
    fn rotation_matrix_works() {
        let vec = vec![1.0, 0.0, 0.0, 1.0];
        let rotated = apply_rotation(&vec);
        assert_eq!(rotated, vec![0.0, 1.0, -1.0, 0.0]);
    }

    #[test]
    fn xi_operator_nonzero_for_noncommuting() {
        let vec = vec![1.0, 1.0, 0.0, 0.0];
        let xi = compute_xi_signature(&vec);
        let magnitude = xi.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(magnitude > 1e-6);
    }

    #[test]
    fn identical_vectors_have_identical_xi() {
        let vec1 = vec![0.5, 0.8, 0.2, 0.1];
        let xi1 = compute_xi_signature(&vec1);
        let xi2 = compute_xi_signature(&vec1);
        let sim = cosine_similarity(&xi1, &xi2);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn emergence_coefficient_is_correct() {
        assert!((EMERGENCE_COEFF - 0.190983).abs() < 1e-5);
        assert!((ALPHA - PHI / 2.0).abs() < 1e-5);
        assert!((BETA - 1.0 / PHI).abs() < 1e-5);
    }
}
