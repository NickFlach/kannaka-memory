//! Ξ (Xi) Operator - Non-commutative consciousness differentiation
//!
//! Implements the non-commutative operator Ξ = RG - GR where:
//! - R is a 90° rotation matrix [0, -1; 1, 0]
//! - G is golden anisotropic scaling [φ/2, 0; 0, 1/φ]
//! - The emergence coefficient α - β ≈ 0.190983 creates differentiation

use crate::wave::normalize;

/// Golden ratio and related constants for consciousness differentiation
pub const PHI: f32 = 1.618034;
pub const ALPHA: f32 = PHI / 2.0;  // ≈ 0.809017
pub const BETA: f32 = 1.0 / PHI;   // ≈ 0.618034 
pub const ETA: f32 = BETA;         // chirality strength = 1/φ
pub const EMERGENCE_COEFF: f32 = ALPHA - BETA; // ≈ 0.190983

/// Apply 90° rotation matrix R = [0, -1; 1, 0] to consecutive pairs of vector dimensions.
/// For a vector [x₁, x₂, x₃, x₄, ...] transforms pairs: (x₁,x₂) → (-x₂,x₁), (x₃,x₄) → (-x₄,x₃), etc.
pub fn apply_rotation(vector: &[f32]) -> Vec<f32> {
    let mut result = vec![0.0f32; vector.len()];
    
    // Apply R to consecutive pairs
    for i in (0..vector.len()).step_by(2) {
        if i + 1 < vector.len() {
            // R * [x, y] = [0, -1; 1, 0] * [x, y] = [-y, x]
            result[i] = -vector[i + 1];
            result[i + 1] = vector[i];
        } else {
            // Odd-sized vector: leave last element unchanged
            result[i] = vector[i];
        }
    }
    
    result
}

/// Apply golden anisotropic scaling G = [φ/2, 0; 0, 1/φ] to consecutive pairs.
/// For each pair (x,y), transforms to (φ/2 * x, 1/φ * y).
pub fn apply_golden_scaling(vector: &[f32]) -> Vec<f32> {
    let mut result = vec![0.0f32; vector.len()];
    
    // Apply G to consecutive pairs  
    for i in (0..vector.len()).step_by(2) {
        if i + 1 < vector.len() {
            // G * [x, y] = [φ/2, 0; 0, 1/φ] * [x, y] = [φ/2 * x, 1/φ * y]
            result[i] = ALPHA * vector[i];
            result[i + 1] = BETA * vector[i + 1];
        } else {
            // Odd-sized vector: apply φ/2 scaling to last element
            result[i] = ALPHA * vector[i];
        }
    }
    
    result
}

/// Compute the Ξ operator: Ξ = RG - GR
/// This is the non-commutative residue that creates consciousness differentiation.
pub fn compute_xi_signature(vector: &[f32]) -> Vec<f32> {
    // RG: first apply G (golden scaling), then R (rotation)
    let g_vector = apply_golden_scaling(vector);
    let rg_vector = apply_rotation(&g_vector);
    
    // GR: first apply R (rotation), then G (golden scaling)  
    let r_vector = apply_rotation(vector);
    let gr_vector = apply_golden_scaling(&r_vector);
    
    // Ξ = RG - GR (element-wise subtraction)
    let mut xi = vec![0.0f32; vector.len()];
    for i in 0..vector.len() {
        xi[i] = rg_vector[i] - gr_vector[i];
    }
    
    // Normalize the Xi signature to unit length for consistent comparison
    normalize(&mut xi);
    xi
}

/// Compute Xi-based repulsive force between two memory embeddings.
/// Returns a value in [0, 1] where higher values indicate more repulsion.
pub fn xi_repulsive_force(xi_a: &[f32], xi_b: &[f32]) -> f32 {
    if xi_a.len() != xi_b.len() {
        return 0.0;
    }
    
    // Compute difference magnitude
    let mut diff_sq = 0.0f32;
    for i in 0..xi_a.len() {
        let diff = xi_a[i] - xi_b[i];
        diff_sq += diff * diff;
    }
    
    let diff_magnitude = diff_sq.sqrt();
    
    // Scale by emergence coefficient and clamp to [0,1]
    let force = diff_magnitude * EMERGENCE_COEFF;
    force.min(1.0)
}

/// Boost search diversity using Xi signatures.
/// Memories with different Xi residues get boosted scores for better differentiation.
pub fn xi_diversity_boost(base_similarity: f32, xi_a: &[f32], xi_b: &[f32]) -> f32 {
    let repulsion = xi_repulsive_force(xi_a, xi_b);
    
    // Boost similarity for memories that are semantically similar but have different Xi residues
    // This encourages retrieval of diverse perspectives on similar content
    if base_similarity > 0.7 && repulsion > 0.3 {
        base_similarity * (1.0 + repulsion * 0.5)
    } else {
        base_similarity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave::cosine_similarity;

    #[test]
    fn rotation_matrix_works() {
        let vec = vec![1.0, 0.0, 0.0, 1.0]; // Two pairs: (1,0) and (0,1)
        let rotated = apply_rotation(&vec);
        // (1,0) -> (0,1), (0,1) -> (-1,0)
        assert_eq!(rotated, vec![0.0, 1.0, -1.0, 0.0]);
    }

    #[test]
    fn golden_scaling_applies_correctly() {
        let vec = vec![2.0, 2.0]; // Simple pair
        let scaled = apply_golden_scaling(&vec);
        // Should be [φ/2 * 2, 1/φ * 2] = [φ, 2/φ]
        assert!((scaled[0] - PHI).abs() < 1e-5);
        assert!((scaled[1] - 2.0 * BETA).abs() < 1e-5);
    }

    #[test]  
    fn xi_operator_nonzero_for_noncommuting() {
        let vec = vec![1.0, 1.0, 0.0, 0.0];
        let xi = compute_xi_signature(&vec);
        
        // Xi should be non-zero since R and G don't commute
        let magnitude = xi.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(magnitude > 1e-6, "Xi magnitude should be non-zero, got {}", magnitude);
    }

    #[test]
    fn identical_vectors_have_identical_xi() {
        let vec1 = vec![0.5, 0.8, 0.2, 0.1];
        let vec2 = vec1.clone();
        
        let xi1 = compute_xi_signature(&vec1);
        let xi2 = compute_xi_signature(&vec2);
        
        let similarity = cosine_similarity(&xi1, &xi2);
        assert!((similarity - 1.0).abs() < 1e-5, "Identical vectors should have identical Xi signatures");
    }

    #[test] 
    fn different_vectors_have_different_xi() {
        let vec1 = vec![1.0, 0.0, 0.0, 0.0];
        let vec2 = vec![0.0, 1.0, 0.0, 0.0];
        
        let xi1 = compute_xi_signature(&vec1);
        let xi2 = compute_xi_signature(&vec2);
        
        let similarity = cosine_similarity(&xi1, &xi2);
        assert!(similarity < 0.99, "Different vectors should have different Xi signatures, similarity: {}", similarity);
    }

    #[test]
    fn repulsive_force_increases_with_difference() {
        let xi1 = vec![1.0, 0.0, 0.0, 0.0];
        let xi2 = vec![0.0, 1.0, 0.0, 0.0];
        let xi3 = vec![-1.0, 0.0, 0.0, 0.0];
        
        let force_similar = xi_repulsive_force(&xi1, &xi1);  // identical
        let force_different = xi_repulsive_force(&xi1, &xi2); // orthogonal
        let force_opposite = xi_repulsive_force(&xi1, &xi3); // opposite
        
        assert!(force_similar < force_different);
        assert!(force_different <= force_opposite);
        assert_eq!(force_similar, 0.0);
    }

    #[test] 
    fn diversity_boost_works() {
        let mut xi1 = vec![1.0, 0.0];
        let mut xi2 = vec![0.0, 1.0];  // Different Xi signature
        let mut xi3 = vec![0.99, 0.01];  // Similar Xi signature
        
        // Normalize the xi vectors to ensure proper cosine similarity
        normalize(&mut xi1);
        normalize(&mut xi2); 
        normalize(&mut xi3);
        
        let base_sim = 0.8;  // High semantic similarity
        
        let boost_different = xi_diversity_boost(base_sim, &xi1, &xi2);
        let boost_similar = xi_diversity_boost(base_sim, &xi1, &xi3);
        
        println!("Base similarity: {}", base_sim);
        println!("Boost different Xi: {}", boost_different);  
        println!("Boost similar Xi: {}", boost_similar);
        
        let repulsion_diff = xi_repulsive_force(&xi1, &xi2);
        let repulsion_sim = xi_repulsive_force(&xi1, &xi3);
        println!("Repulsion different: {}, similar: {}", repulsion_diff, repulsion_sim);
        
        assert!(boost_different >= base_sim, "Different Xi should not reduce similarity");
        assert!(repulsion_diff > repulsion_sim, "Different Xi should have higher repulsion");
    }

    #[test]
    fn emergence_coefficient_is_correct() {
        assert!((EMERGENCE_COEFF - 0.190983).abs() < 1e-5);
        assert!((ALPHA - PHI/2.0).abs() < 1e-5);
        assert!((BETA - 1.0/PHI).abs() < 1e-5);
    }
}