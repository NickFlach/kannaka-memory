//! Retrieval fusion using Reciprocal Rank Fusion (RRF)

use std::collections::HashMap;
use uuid::Uuid;

/// Combine multiple ranked lists using Reciprocal Rank Fusion
pub fn rrf_fuse(results: &[Vec<(Uuid, f32)>], k: f32) -> Vec<(Uuid, f32)> {
    if results.is_empty() {
        return Vec::new();
    }

    let mut scores = HashMap::new();
    
    for result_list in results {
        for (rank, (id, _score)) in result_list.iter().enumerate() {
            let rrf_score = 1.0 / (k + (rank + 1) as f32);
            *scores.entry(*id).or_insert(0.0) += rrf_score;
        }
    }

    // Convert to sorted vector
    let mut combined: Vec<_> = scores.into_iter().collect();
    combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    combined
}

/// Weighted fusion where different result lists have different importance
pub fn weighted_rrf_fuse(results: &[(Vec<(Uuid, f32)>, f32)], k: f32) -> Vec<(Uuid, f32)> {
    if results.is_empty() {
        return Vec::new();
    }

    let mut scores = HashMap::new();
    
    for (result_list, weight) in results {
        for (rank, (id, _score)) in result_list.iter().enumerate() {
            let rrf_score = weight * (1.0 / (k + (rank + 1) as f32));
            *scores.entry(*id).or_insert(0.0) += rrf_score;
        }
    }

    // Convert to sorted vector
    let mut combined: Vec<_> = scores.into_iter().collect();
    combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    combined
}

/// Combine results preserving original scores but using RRF for ranking conflicts
pub fn hybrid_fuse(results: &[Vec<(Uuid, f32)>], k: f32) -> Vec<(Uuid, f32)> {
    if results.is_empty() {
        return Vec::new();
    }

    let mut rrf_scores = HashMap::new();
    let mut original_scores = HashMap::new();
    
    for result_list in results {
        for (rank, (id, original_score)) in result_list.iter().enumerate() {
            let rrf_score = 1.0 / (k + (rank + 1) as f32);
            *rrf_scores.entry(*id).or_insert(0.0) += rrf_score;
            
            // Keep the best original score seen for each ID
            let current_best = original_scores.get(id).copied().unwrap_or(0.0);
            if *original_score > current_best {
                original_scores.insert(*id, *original_score);
            }
        }
    }

    // Create combined results with both RRF and original scores
    let mut combined: Vec<_> = rrf_scores
        .into_iter()
        .map(|(id, rrf_score)| {
            let original_score = original_scores.get(&id).copied().unwrap_or(0.0);
            // Combine RRF score (for ranking) with original score (for interpretation)
            let combined_score = rrf_score + original_score * 0.1; // Small weight on original
            (id, combined_score)
        })
        .collect();
        
    combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    combined
}

/// Calculate rank-based metrics for evaluation
pub fn calculate_ndcg(relevant_ids: &[Uuid], results: &[(Uuid, f32)], k: usize) -> f32 {
    if relevant_ids.is_empty() || results.is_empty() {
        return 0.0;
    }

    let results_top_k = &results[..results.len().min(k)];
    
    // DCG calculation
    let mut dcg = 0.0;
    for (i, (id, _score)) in results_top_k.iter().enumerate() {
        if relevant_ids.contains(id) {
            dcg += 1.0 / (i as f32 + 2.0).log2(); // +2 because log2(1) = 0
        }
    }

    // IDCG calculation (perfect ranking)
    let mut idcg = 0.0;
    let num_relevant = relevant_ids.len().min(k);
    for i in 0..num_relevant {
        idcg += 1.0 / (i as f32 + 2.0).log2();
    }

    if idcg == 0.0 {
        0.0
    } else {
        dcg / idcg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rrf_basic() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        let list1 = vec![(id1, 1.0), (id2, 0.8)];
        let list2 = vec![(id2, 0.9), (id3, 0.7)];
        let results = vec![list1, list2];

        let fused = rrf_fuse(&results, 60.0);
        
        // id2 should be first (appears in both lists at good positions)
        assert_eq!(fused[0].0, id2);
        assert!(fused[0].1 > fused[1].1);
    }

    #[test]
    fn test_weighted_rrf() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let list1 = vec![(id1, 1.0)];
        let list2 = vec![(id2, 1.0)];
        
        // Give list1 higher weight
        let results = vec![(list1, 2.0), (list2, 1.0)];
        let fused = weighted_rrf_fuse(&results, 60.0);
        
        // id1 should win due to higher weight
        assert_eq!(fused[0].0, id1);
    }

    #[test]
    fn test_ndcg() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        let relevant = vec![id1, id2];
        let results = vec![(id1, 1.0), (id3, 0.9), (id2, 0.8)];
        
        let ndcg = calculate_ndcg(&relevant, &results, 3);
        assert!(ndcg > 0.0 && ndcg <= 1.0);
    }
}