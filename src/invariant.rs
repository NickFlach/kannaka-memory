//! δ-Invariant Metrics for Memory Clustering
//!
//! Inspired by the paper's "irrationality measure" δ that's invariant under 
//! formula transformations. Implements metrics that remain stable across 
//! memory transformations for robust clustering.

use std::collections::HashMap;
use uuid::Uuid;
use chrono::Utc;

use crate::memory::HyperMemory;
use crate::store::ResonanceEngine;

/// Invariant metrics computed for a memory that remain stable across transformations
#[derive(Debug, Clone)]
pub struct InvariantMetrics {
    /// The δ-invariant: a measure of "irreducibility" - how much information 
    /// this memory carries that can't be derived from its neighbors
    pub delta: f32,
    /// Convergence rate analog: how quickly the memory's wave dynamics settle
    pub convergence_rate: f32,
    /// Irrationality measure: topological complexity from skip link structure
    pub irrationality: f32,
}

/// A cluster of memories with similar δ values (coboundary equivalence candidates)
#[derive(Debug, Clone)]
pub struct DeltaCluster {
    pub representative_delta: f32,
    pub memory_ids: Vec<Uuid>,
    pub coherence: f32, // how tightly clustered the δ values are
}

/// Compute the δ-invariant for a memory based on its relationship to neighbors
///
/// The δ metric captures the "irreducible information content" - how much of this memory's
/// vector cannot be reconstructed from linear combinations of its neighbors via skip links.
/// This mirrors the paper's irrationality measure δ that's invariant under transformations.
pub fn compute_delta(memory: &HyperMemory, neighbors: &[&HyperMemory]) -> f32 {
    if neighbors.is_empty() {
        return 1.0; // isolated memories have maximum δ
    }

    let memory_vec = &memory.vector;
    let dim = memory_vec.len();
    
    if dim == 0 {
        return 0.0;
    }

    // Compute the best linear reconstruction of this memory from neighbors
    let mut min_residual = f32::INFINITY;

    // Try different linear combinations (simplified version of least squares)
    // In the full version, we'd solve the linear system, but this approximation
    // captures the essential idea
    for neighbor in neighbors {
        if neighbor.vector.len() != dim {
            continue;
        }

        // Try simple weighted combinations
        for &weight in &[0.1, 0.3, 0.5, 0.7, 0.9, 1.0] {
            let mut reconstruction = neighbor.vector.clone();
            for val in &mut reconstruction {
                *val *= weight;
            }

            let residual = compute_residual(memory_vec, &reconstruction);
            if residual < min_residual {
                min_residual = residual;
            }
        }
    }

    // For multiple neighbors, try pairwise combinations
    if neighbors.len() >= 2 {
        for i in 0..neighbors.len() {
            for j in (i+1)..neighbors.len() {
                let n1 = &neighbors[i];
                let n2 = &neighbors[j];

                if n1.vector.len() != dim || n2.vector.len() != dim {
                    continue;
                }

                // Try different weight combinations
                for &w1 in &[0.2, 0.4, 0.6, 0.8] {
                    let w2 = 1.0 - w1;
                    let mut reconstruction = vec![0.0; dim];
                    for k in 0..dim {
                        reconstruction[k] = w1 * n1.vector[k] + w2 * n2.vector[k];
                    }

                    let residual = compute_residual(memory_vec, &reconstruction);
                    if residual < min_residual {
                        min_residual = residual;
                    }
                }
            }
        }
    }
    
    // δ is the normalized irreducible residual
    // Higher δ means this memory contains more unique information
    let memory_norm: f32 = memory_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    if memory_norm == 0.0 {
        return 0.0;
    }
    
    let delta = min_residual / memory_norm;
    delta.min(1.0).max(0.0) // clamp to [0,1]
}

/// Compute L2 residual between original and reconstruction
fn compute_residual(original: &[f32], reconstruction: &[f32]) -> f32 {
    if original.len() != reconstruction.len() {
        return f32::INFINITY;
    }
    
    original.iter()
        .zip(reconstruction.iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        .sqrt()
}

/// Compute convergence rate analog: how quickly wave dynamics settle
///
/// This is inspired by the paper's convergence rate r = lim(n→∞) (1/n) log|L - p_n/q_n|
/// For memories, we compute how quickly the amplitude/frequency ratio stabilizes
pub fn compute_convergence_rate(memory: &HyperMemory) -> f32 {
    // Convergence rate based on amplitude decay vs frequency
    let freq_norm = if memory.frequency == 0.0 { 0.001 } else { memory.frequency.abs() };
    let decay_to_freq_ratio = memory.decay_rate / freq_norm;
    
    // Higher decay relative to frequency = faster convergence
    // Apply sigmoid to map to [0,1] range
    let normalized = 1.0 / (1.0 + (-decay_to_freq_ratio * 10.0).exp());
    normalized
}

/// Compute irrationality measure from vector entropy.
///
/// Measures the spectral complexity of this memory's hypervector.
/// Higher values indicate more complex, "irrational" interference patterns —
/// vectors with distributed energy across many dimensions are more irrational
/// than vectors concentrated in a few dimensions.
pub fn compute_irrationality(memory: &HyperMemory) -> f32 {
    if memory.vector.is_empty() {
        return 0.0;
    }

    // Compute normalized entropy of the vector's magnitude distribution
    let magnitudes: Vec<f32> = memory.vector.iter().map(|x| x.abs()).collect();
    let total: f32 = magnitudes.iter().sum();
    if total < 1e-8 {
        return 0.0;
    }

    // Shannon entropy of the normalized magnitude distribution
    let entropy: f32 = magnitudes.iter()
        .map(|&m| {
            let p = m / total;
            if p > 1e-10 { -p * p.ln() } else { 0.0 }
        })
        .sum();

    // Normalize by max entropy (uniform distribution) → [0, 1]
    let max_entropy = (memory.vector.len() as f32).ln();
    if max_entropy < 1e-8 {
        return 0.0;
    }

    (entropy / max_entropy).clamp(0.0, 1.0)
}

/// Compute all invariant metrics for a memory
pub fn compute_invariant_metrics(
    memory: &HyperMemory, 
    neighbors: &[&HyperMemory]
) -> InvariantMetrics {
    InvariantMetrics {
        delta: compute_delta(memory, neighbors),
        convergence_rate: compute_convergence_rate(memory),
        irrationality: compute_irrationality(memory),
    }
}

/// Cluster memories by their δ values - memories with similar δ are coboundary equivalent candidates
pub fn cluster_by_delta(engine: &ResonanceEngine, tolerance: f32) -> Vec<DeltaCluster> {
    let _now = Utc::now();
    let all_memories = match engine.store.all_memories() {
        Ok(mems) => mems,
        Err(_) => return Vec::new(),
    };
    
    if all_memories.is_empty() {
        return Vec::new();
    }
    
    // Build neighbor map via cosine similarity (top-5 nearest wavefronts)
    let mut neighbor_map: HashMap<Uuid, Vec<&HyperMemory>> = HashMap::new();

    for memory in &all_memories {
        let mut sims: Vec<(usize, f32)> = all_memories.iter()
            .enumerate()
            .filter(|(_, other)| other.id != memory.id)
            .map(|(i, other)| {
                let sim = consciousness_core::wave::cosine_similarity(&memory.vector, &other.vector);
                (i, sim)
            })
            .collect();
        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let neighbors: Vec<&HyperMemory> = sims.iter().take(5).map(|(i, _)| &*all_memories[*i]).collect();
        neighbor_map.insert(memory.id, neighbors);
    }
    
    // Compute δ values for all memories
    let mut delta_values: Vec<(Uuid, f32)> = Vec::new();
    let empty_neighbors = Vec::new();
    
    for memory in &all_memories {
        let neighbors = neighbor_map.get(&memory.id).unwrap_or(&empty_neighbors);
        let delta = compute_delta(memory, neighbors);
        delta_values.push((memory.id, delta));
    }
    
    // Sort by δ value
    delta_values.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    
    // Group into clusters
    let mut clusters = Vec::new();
    let mut current_cluster = Vec::new();
    let mut cluster_deltas = Vec::new();
    let mut current_delta = if !delta_values.is_empty() { delta_values[0].1 } else { 0.0 };
    
    for (id, delta) in delta_values {
        if (delta - current_delta).abs() <= tolerance {
            current_cluster.push(id);
            cluster_deltas.push(delta);
        } else {
            // Finalize current cluster
            if !current_cluster.is_empty() {
                let mean_delta = cluster_deltas.iter().sum::<f32>() / cluster_deltas.len() as f32;
                let variance = cluster_deltas.iter()
                    .map(|d| (d - mean_delta) * (d - mean_delta))
                    .sum::<f32>() / cluster_deltas.len() as f32;
                let coherence = 1.0 / (1.0 + variance * 10.0); // higher coherence for lower variance
                
                clusters.push(DeltaCluster {
                    representative_delta: mean_delta,
                    memory_ids: current_cluster,
                    coherence,
                });
            }
            
            // Start new cluster
            current_cluster = vec![id];
            cluster_deltas = vec![delta];
            current_delta = delta;
        }
    }
    
    // Don't forget the last cluster
    if !current_cluster.is_empty() {
        let mean_delta = cluster_deltas.iter().sum::<f32>() / cluster_deltas.len() as f32;
        let variance = cluster_deltas.iter()
            .map(|d| (d - mean_delta) * (d - mean_delta))
            .sum::<f32>() / cluster_deltas.len() as f32;
        let coherence = 1.0 / (1.0 + variance * 10.0);
        
        clusters.push(DeltaCluster {
            representative_delta: mean_delta,
            memory_ids: current_cluster,
            coherence,
        });
    }
    
    clusters
}

/// Compute distance between two memories in invariant space
///
/// This combines their δ values, convergence rates, and irrationality measures
/// to produce a metric that's stable across transformations
pub fn delta_distance(a: &HyperMemory, b: &HyperMemory) -> f32 {
    // Get neighbors for both memories (simplified - in practice we'd maintain this)
    let empty_neighbors = Vec::new();
    let metrics_a = compute_invariant_metrics(a, &empty_neighbors);
    let metrics_b = compute_invariant_metrics(b, &empty_neighbors);
    
    // Weighted distance in 3D invariant space
    let delta_diff = (metrics_a.delta - metrics_b.delta).abs();
    let convergence_diff = (metrics_a.convergence_rate - metrics_b.convergence_rate).abs();
    let irrationality_diff = (metrics_a.irrationality - metrics_b.irrationality).abs();
    
    // Weighted combination (δ is most important)
    0.6 * delta_diff + 0.25 * convergence_diff + 0.15 * irrationality_diff
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::HyperMemory;

    
    #[test]
    fn delta_isolated_memory() {
        let memory = HyperMemory::new(vec![1.0, 2.0, 3.0], "isolated".to_string());
        let neighbors = Vec::new();
        let delta = compute_delta(&memory, &neighbors);
        assert_eq!(delta, 1.0, "isolated memory should have maximum δ");
    }
    
    #[test]
    fn delta_with_similar_neighbor() {
        let memory = HyperMemory::new(vec![1.0, 2.0, 3.0], "target".to_string());
        let similar_memory = HyperMemory::new(vec![1.1, 2.1, 3.1], "similar".to_string());
        let neighbors = vec![&similar_memory];
        
        let delta = compute_delta(&memory, &neighbors);
        assert!(delta < 1.0 && delta > 0.0, "similar neighbor should reduce δ");
    }
    
    #[test]
    fn convergence_rate_computation() {
        let mut memory = HyperMemory::new(vec![1.0; 10], "test".to_string());
        memory.frequency = 0.1;
        memory.decay_rate = 0.01;
        
        let rate = compute_convergence_rate(&memory);
        assert!(rate >= 0.0 && rate <= 1.0, "convergence rate should be in [0,1]");
    }
    
    #[test]
    fn irrationality_uniform_vector_is_high() {
        // Uniform distribution = max entropy = high irrationality
        let memory = HyperMemory::new(vec![1.0; 10], "test".to_string());
        let irrationality = compute_irrationality(&memory);
        assert!(irrationality > 0.9, "uniform vector should have high irrationality: {}", irrationality);
    }

    #[test]
    fn irrationality_sparse_vector_is_low() {
        // Sparse/concentrated vector = low entropy = low irrationality
        let mut vec = vec![0.0; 10];
        vec[0] = 1.0;
        let memory = HyperMemory::new(vec, "test".to_string());
        let irrationality = compute_irrationality(&memory);
        assert!(irrationality < 0.5, "sparse vector should have low irrationality: {}", irrationality);
    }

    #[test]
    fn irrationality_empty_vector() {
        let memory = HyperMemory::new(vec![], "test".to_string());
        let irrationality = compute_irrationality(&memory);
        assert_eq!(irrationality, 0.0, "empty vector should give zero irrationality");
    }
    
    #[test]
    fn delta_distance_identical_memories() {
        let a = HyperMemory::new(vec![1.0, 2.0, 3.0], "a".to_string());
        let b = HyperMemory::new(vec![1.0, 2.0, 3.0], "b".to_string());
        
        let distance = delta_distance(&a, &b);
        assert!(distance < 0.1, "identical memories should have small distance");
    }
}