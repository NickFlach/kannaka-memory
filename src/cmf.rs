//! Conservative Memory Field Detection
//!
//! Inspired by the paper's Conservative Matrix Field (CMF) - a single mathematical 
//! object that generates families of related formulas. For memories, a CMF is a 
//! generative structure that multiple memories are "trajectories" through.

use crate::memory::HyperMemory;
use crate::wave::cosine_similarity;

/// Conservative Memory Field: a compact representation of a generative pattern
/// that multiple memories are trajectories through.
#[derive(Debug, Clone)]
pub struct ConservativeMemoryField {
    /// Primary basis vectors defining the field structure
    pub basis_vectors: Vec<Vec<f32>>,
    /// Trajectory parameters: how memories move through this field
    pub trajectory_params: TrajectoryParams,
    /// Path independence constraints ensuring this is a valid CMF
    pub path_constraints: PathConstraints,
    /// How many memories this CMF can explain (0-1)
    pub explanatory_power: f32,
    /// Unique identifier for this CMF
    pub id: String,
}

/// Parameters defining how memories move as trajectories through a CMF
#[derive(Debug, Clone)]
pub struct TrajectoryParams {
    /// Step size for trajectory generation
    pub step_size: f32,
    /// Curvature parameters for non-linear paths
    pub curvature: Vec<f32>,
    /// Boundary conditions
    pub bounds: (Vec<f32>, Vec<f32>), // (min_bounds, max_bounds)
}

/// Path independence constraints - different orderings through the basis 
/// should give consistent results (like a conservative vector field)
#[derive(Debug, Clone)]
pub struct PathConstraints {
    /// Maximum allowed deviation for path independence (smaller = more conservative)
    pub max_deviation: f32,
    /// Verified path combinations that are path-independent
    pub verified_paths: Vec<Vec<usize>>, // sequences of basis vector indices
    /// Constraint violation scores for diagnostics
    pub constraint_scores: Vec<f32>,
}

/// Result of CMF membership test
#[derive(Debug, Clone)]
pub struct CMFMembership {
    /// How well this memory fits the CMF (0-1)
    pub fitness: f32,
    /// Position parameters within the CMF trajectory space
    pub trajectory_position: Vec<f32>,
    /// Confidence in the membership assessment
    pub confidence: f32,
}

/// Detect Conservative Memory Field from a cluster of related memories.
/// 
/// Algorithm:
/// 1. Compute principal components (PCA-like) to find basis vectors
/// 2. Check path independence: do different orderings give consistent results?
/// 3. If path-independent, we found a CMF
pub fn detect_cmf(cluster: &[&HyperMemory]) -> Option<ConservativeMemoryField> {
    if cluster.len() < 3 {
        return None; // Need at least 3 memories to detect a field
    }
    
    let dim = cluster[0].vector.len();
    if dim == 0 {
        return None;
    }
    
    // Verify all memories have same dimension
    if !cluster.iter().all(|m| m.vector.len() == dim) {
        return None;
    }
    
    // Step 1: Compute principal components to find basis vectors
    let basis_vectors = compute_principal_components(cluster)?;
    
    // Step 2: Check path independence
    let path_constraints = check_path_independence(&basis_vectors, cluster)?;
    
    // Step 3: If path-independent, create CMF
    if path_constraints.max_deviation < 0.3 { // Threshold for path independence
        let trajectory_params = compute_trajectory_params(&basis_vectors, cluster);
        let explanatory_power = compute_explanatory_power(&basis_vectors, cluster);
        
        Some(ConservativeMemoryField {
            basis_vectors,
            trajectory_params,
            path_constraints,
            explanatory_power,
            id: format!("cmf_{}", &uuid::Uuid::new_v4().to_string()[..8]),
        })
    } else {
        None
    }
}

/// Compute principal components from memory vectors (simplified PCA)
fn compute_principal_components(cluster: &[&HyperMemory]) -> Option<Vec<Vec<f32>>> {
    let dim = cluster[0].vector.len();
    let n = cluster.len();
    
    if n == 0 || dim == 0 {
        return None;
    }
    
    // Compute mean vector
    let mut mean = vec![0.0; dim];
    for memory in cluster {
        for (i, &val) in memory.vector.iter().enumerate() {
            mean[i] += val;
        }
    }
    for val in &mut mean {
        *val /= n as f32;
    }
    
    // Center the data
    let mut centered_data: Vec<Vec<f32>> = Vec::new();
    for memory in cluster {
        let mut centered = memory.vector.clone();
        for (i, val) in centered.iter_mut().enumerate() {
            *val -= mean[i];
        }
        centered_data.push(centered);
    }
    
    // Simplified PCA: find directions of maximum variance
    let mut basis_vectors = Vec::new();
    
    // First principal component: direction of maximum variance
    let mut max_var_direction = vec![0.0; dim];
    let mut max_variance = 0.0;
    
    // Try different random directions and pick the one with max variance
    for trial in 0..20 {
        let mut direction = vec![0.0; dim];
        for i in 0..dim {
            direction[i] = (trial as f32 * 0.37 + i as f32 * 0.41).sin(); // pseudo-random
        }
        
        // Normalize direction
        let norm: f32 = direction.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut direction {
                *val /= norm;
            }
        }
        
        // Compute variance in this direction
        let mut variance = 0.0;
        for data in &centered_data {
            let projection: f32 = data.iter()
                .zip(direction.iter())
                .map(|(d, dir)| d * dir)
                .sum();
            variance += projection * projection;
        }
        variance /= n as f32;
        
        if variance > max_variance {
            max_variance = variance;
            max_var_direction = direction;
        }
    }
    
    if max_variance > 0.0 {
        basis_vectors.push(max_var_direction);
    }
    
    // Second principal component: orthogonal to first, max variance
    if basis_vectors.len() == 1 {
        let first = &basis_vectors[0];
        let mut second_direction = vec![0.0; dim];
        let mut max_second_var = 0.0;
        
        for trial in 0..20 {
            let mut direction = vec![0.0; dim];
            for i in 0..dim {
                direction[i] = (trial as f32 * 0.73 + i as f32 * 0.61).cos(); // different pseudo-random
            }
            
            // Make orthogonal to first component
            let dot: f32 = direction.iter()
                .zip(first.iter())
                .map(|(d, f)| d * f)
                .sum();
            for (i, val) in direction.iter_mut().enumerate() {
                *val -= dot * first[i];
            }
            
            // Normalize
            let norm: f32 = direction.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for val in &mut direction {
                    *val /= norm;
                }
                
                // Compute variance in this direction
                let mut variance = 0.0;
                for data in &centered_data {
                    let projection: f32 = data.iter()
                        .zip(direction.iter())
                        .map(|(d, dir)| d * dir)
                        .sum();
                    variance += projection * projection;
                }
                variance /= n as f32;
                
                if variance > max_second_var {
                    max_second_var = variance;
                    second_direction = direction;
                }
            }
        }
        
        if max_second_var > 0.0 {
            basis_vectors.push(second_direction);
        }
    }
    
    // Third component (if beneficial)
    if basis_vectors.len() == 2 && cluster.len() >= 5 {
        let first = &basis_vectors[0];
        let second = &basis_vectors[1];
        let mut third_direction = vec![0.0; dim];
        let mut max_third_var = 0.0;
        
        for trial in 0..10 {
            let mut direction = vec![0.0; dim];
            for i in 0..dim {
                direction[i] = (trial as f32 * 1.17 + i as f32 * 0.83).sin();
            }
            
            // Make orthogonal to first two components
            let dot1: f32 = direction.iter()
                .zip(first.iter())
                .map(|(d, f)| d * f)
                .sum();
            let dot2: f32 = direction.iter()
                .zip(second.iter())
                .map(|(d, s)| d * s)
                .sum();
            for i in 0..dim {
                direction[i] -= dot1 * first[i] + dot2 * second[i];
            }
            
            let norm: f32 = direction.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for val in &mut direction {
                    *val /= norm;
                }
                
                let mut variance = 0.0;
                for data in &centered_data {
                    let projection: f32 = data.iter()
                        .zip(direction.iter())
                        .map(|(d, dir)| d * dir)
                        .sum();
                    variance += projection * projection;
                }
                variance /= n as f32;
                
                if variance > max_third_var {
                    max_third_var = variance;
                    third_direction = direction;
                }
            }
        }
        
        if max_third_var > max_variance * 0.1 { // Only include if it explains meaningful variance
            basis_vectors.push(third_direction);
        }
    }
    
    if basis_vectors.is_empty() {
        None
    } else {
        Some(basis_vectors)
    }
}

/// Check path independence: different orderings through basis should give consistent results
fn check_path_independence(basis_vectors: &[Vec<f32>], cluster: &[&HyperMemory]) -> Option<PathConstraints> {
    if basis_vectors.len() < 2 {
        return Some(PathConstraints {
            max_deviation: 0.0,
            verified_paths: vec![vec![0]],
            constraint_scores: vec![1.0],
        });
    }
    
    let mut verified_paths = Vec::new();
    let mut constraint_scores = Vec::new();
    let mut max_deviation: f32 = 0.0;
    
    // Test different path orderings
    let num_basis = basis_vectors.len();
    for i in 0..num_basis {
        for j in 0..num_basis {
            if i == j {
                continue;
            }
            
            // Test path i -> j vs j -> i for each memory
            let mut total_deviation = 0.0;
            let mut valid_tests = 0;
            
            for memory in cluster {
                let path_ij = compute_path_result(memory, basis_vectors, &[i, j]);
                let path_ji = compute_path_result(memory, basis_vectors, &[j, i]);
                
                if let (Some(result_ij), Some(result_ji)) = (path_ij, path_ji) {
                    let deviation = cosine_distance(&result_ij, &result_ji);
                    total_deviation += deviation;
                    valid_tests += 1;
                }
            }
            
            if valid_tests > 0 {
                let avg_deviation = total_deviation / valid_tests as f32;
                max_deviation = max_deviation.max(avg_deviation);
                
                verified_paths.push(vec![i, j]);
                constraint_scores.push(1.0 - avg_deviation);
            }
        }
    }
    
    Some(PathConstraints {
        max_deviation,
        verified_paths,
        constraint_scores,
    })
}

/// Compute the result of following a specific path through the basis vectors
fn compute_path_result(memory: &HyperMemory, basis_vectors: &[Vec<f32>], path: &[usize]) -> Option<Vec<f32>> {
    if path.is_empty() || memory.vector.is_empty() {
        return None;
    }
    
    let mut result = memory.vector.clone();
    
    for &basis_idx in path {
        if basis_idx >= basis_vectors.len() {
            continue;
        }
        
        let basis = &basis_vectors[basis_idx];
        if basis.len() != result.len() {
            return None;
        }
        
        // Project onto this basis vector and apply transformation
        let projection: f32 = result.iter()
            .zip(basis.iter())
            .map(|(r, b)| r * b)
            .sum();
        
        // Transform the result (simplified)
        for (i, val) in result.iter_mut().enumerate() {
            *val += projection * basis[i] * 0.1; // Small step along basis direction
        }
    }
    
    Some(result)
}

/// Compute cosine distance (1 - cosine similarity)
fn cosine_distance(a: &[f32], b: &[f32]) -> f32 {
    1.0 - cosine_similarity(a, b)
}

/// Compute trajectory parameters for this CMF
fn compute_trajectory_params(basis_vectors: &[Vec<f32>], cluster: &[&HyperMemory]) -> TrajectoryParams {
    let dim = basis_vectors.get(0).map(|v| v.len()).unwrap_or(0);
    
    // Compute bounds from the memory cluster
    let mut min_bounds = vec![f32::INFINITY; dim];
    let mut max_bounds = vec![f32::NEG_INFINITY; dim];
    
    for memory in cluster {
        for (i, &val) in memory.vector.iter().enumerate() {
            if i < dim {
                min_bounds[i] = min_bounds[i].min(val);
                max_bounds[i] = max_bounds[i].max(val);
            }
        }
    }
    
    // Compute curvature from variance in trajectory directions
    let mut curvature = vec![0.1; basis_vectors.len()];
    for (i, basis) in basis_vectors.iter().enumerate() {
        let mut variance = 0.0;
        for memory in cluster {
            let projection: f32 = memory.vector.iter()
                .zip(basis.iter())
                .map(|(m, b)| m * b)
                .sum();
            variance += projection * projection;
        }
        variance /= cluster.len() as f32;
        curvature[i] = (variance.sqrt() * 0.01).min(1.0); // Cap curvature
    }
    
    TrajectoryParams {
        step_size: 0.05,
        curvature,
        bounds: (min_bounds, max_bounds),
    }
}

/// Compute how much of the cluster variance this CMF explains
fn compute_explanatory_power(basis_vectors: &[Vec<f32>], cluster: &[&HyperMemory]) -> f32 {
    if cluster.is_empty() || basis_vectors.is_empty() {
        return 0.0;
    }
    
    let dim = cluster[0].vector.len();
    
    // Compute total variance in cluster
    let mut mean = vec![0.0; dim];
    for memory in cluster {
        for (i, &val) in memory.vector.iter().enumerate() {
            mean[i] += val;
        }
    }
    for val in &mut mean {
        *val /= cluster.len() as f32;
    }
    
    let mut total_variance = 0.0;
    for memory in cluster {
        for (i, &val) in memory.vector.iter().enumerate() {
            let diff = val - mean[i];
            total_variance += diff * diff;
        }
    }
    
    // Compute variance explained by projection onto basis vectors
    let mut explained_variance = 0.0;
    for basis in basis_vectors {
        for memory in cluster {
            let projection: f32 = memory.vector.iter()
                .zip(basis.iter())
                .map(|(m, b)| m * b)
                .sum();
            explained_variance += projection * projection;
        }
    }
    
    if total_variance > 0.0 {
        (explained_variance / total_variance).min(1.0)
    } else {
        0.0
    }
}

/// Test how well a memory fits this CMF (0-1)
pub fn cmf_membership(memory: &HyperMemory, cmf: &ConservativeMemoryField) -> CMFMembership {
    if memory.vector.is_empty() || cmf.basis_vectors.is_empty() {
        return CMFMembership {
            fitness: 0.0,
            trajectory_position: Vec::new(),
            confidence: 0.0,
        };
    }
    
    let dim = memory.vector.len();
    let mut trajectory_position = Vec::new();
    let mut total_projection_strength = 0.0;
    
    // Project memory onto each basis vector
    for basis in &cmf.basis_vectors {
        if basis.len() == dim {
            let projection: f32 = memory.vector.iter()
                .zip(basis.iter())
                .map(|(m, b)| m * b)
                .sum();
            trajectory_position.push(projection);
            total_projection_strength += projection.abs();
        }
    }
    
    // Compute fitness based on how well the memory aligns with the CMF structure
    let memory_norm: f32 = memory.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
    let fitness = if memory_norm > 0.0 {
        (total_projection_strength / memory_norm).min(1.0)
    } else {
        0.0
    };
    
    // Confidence based on the strength of projections
    let avg_projection = if !trajectory_position.is_empty() {
        trajectory_position.iter().sum::<f32>() / trajectory_position.len() as f32
    } else {
        0.0
    };
    let confidence = (avg_projection.abs() * 2.0).min(1.0);
    
    CMFMembership {
        fitness,
        trajectory_position,
        confidence,
    }
}

/// Generate a new memory vector from the CMF using trajectory parameters
pub fn generate_trajectory(cmf: &ConservativeMemoryField, params: &[f32]) -> Option<Vec<f32>> {
    if cmf.basis_vectors.is_empty() || params.len() != cmf.basis_vectors.len() {
        return None;
    }
    
    let dim = cmf.basis_vectors[0].len();
    let mut result = vec![0.0; dim];
    
    // Linear combination of basis vectors weighted by parameters
    for (param, basis) in params.iter().zip(cmf.basis_vectors.iter()) {
        if basis.len() != dim {
            return None;
        }
        
        for (i, &basis_val) in basis.iter().enumerate() {
            result[i] += param * basis_val;
        }
    }
    
    // Apply trajectory curvature and bounds
    for (i, val) in result.iter_mut().enumerate() {
        if i < cmf.trajectory_params.bounds.0.len() && i < cmf.trajectory_params.bounds.1.len() {
            let min_bound = cmf.trajectory_params.bounds.0[i];
            let max_bound = cmf.trajectory_params.bounds.1[i];
            if min_bound < max_bound {
                *val = val.clamp(min_bound, max_bound);
            }
        }
    }
    
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::HyperMemory;
    
    #[test]
    fn detect_cmf_insufficient_memories() {
        let memory1 = HyperMemory::new(vec![1.0, 2.0], "test1".to_string());
        let memory2 = HyperMemory::new(vec![2.0, 3.0], "test2".to_string());
        let cluster = vec![&memory1, &memory2];
        
        let result = detect_cmf(&cluster);
        assert!(result.is_none(), "should not detect CMF with < 3 memories");
    }
    
    #[test]
    fn detect_cmf_with_valid_cluster() {
        let memory1 = HyperMemory::new(vec![1.0, 2.0, 3.0], "test1".to_string());
        let memory2 = HyperMemory::new(vec![2.0, 4.0, 6.0], "test2".to_string());
        let memory3 = HyperMemory::new(vec![1.5, 3.0, 4.5], "test3".to_string());
        let cluster = vec![&memory1, &memory2, &memory3];
        
        let result = detect_cmf(&cluster);
        assert!(result.is_some(), "should detect CMF with aligned memories");
    }
    
    #[test]
    fn cmf_membership_empty_vector() {
        let memory = HyperMemory::new(vec![], "empty".to_string());
        let cmf = ConservativeMemoryField {
            basis_vectors: vec![vec![1.0, 0.0]],
            trajectory_params: TrajectoryParams {
                step_size: 0.1,
                curvature: vec![0.1],
                bounds: (vec![0.0, 0.0], vec![1.0, 1.0]),
            },
            path_constraints: PathConstraints {
                max_deviation: 0.1,
                verified_paths: vec![vec![0]],
                constraint_scores: vec![1.0],
            },
            explanatory_power: 0.5,
            id: "test_cmf".to_string(),
        };
        
        let membership = cmf_membership(&memory, &cmf);
        assert_eq!(membership.fitness, 0.0);
    }
    
    #[test]
    fn generate_trajectory_valid_params() {
        let cmf = ConservativeMemoryField {
            basis_vectors: vec![
                vec![1.0, 0.0],
                vec![0.0, 1.0],
            ],
            trajectory_params: TrajectoryParams {
                step_size: 0.1,
                curvature: vec![0.1, 0.1],
                bounds: (vec![-2.0, -2.0], vec![2.0, 2.0]),
            },
            path_constraints: PathConstraints {
                max_deviation: 0.1,
                verified_paths: vec![vec![0, 1]],
                constraint_scores: vec![0.9],
            },
            explanatory_power: 0.8,
            id: "test_cmf".to_string(),
        };
        
        let params = vec![0.5, 0.3];
        let result = generate_trajectory(&cmf, &params);
        
        assert!(result.is_some());
        let trajectory = result.unwrap();
        assert_eq!(trajectory.len(), 2);
        assert!((trajectory[0] - 0.5).abs() < 1e-5);
        assert!((trajectory[1] - 0.3).abs() < 1e-5);
    }
}