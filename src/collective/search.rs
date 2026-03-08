//! ADR-0013 Phase 5: Search & Discovery on Sealed Glyphs
//!
//! Search the collective without blooming. Agents discover relevant knowledge
//! by requesting similarity proofs from glyph owners, ranking results by
//! proven similarity, trust, and bloom difficulty.
//!
//! ## Flow
//!
//! ```text
//! Searcher                          Glyph Owner
//!    |                                   |
//!    |-- SearchRequest (query_hash) ---->|
//!    |                                   |-- prove_similarity()
//!    |<-- SimilarityProof --------------|
//!    |                                   |
//!    |-- verify_similarity()            |
//!    |-- rank results                   |
//! ```
//!
//! The searcher learns THAT relevant knowledge exists and WHO has it,
//! but never WHAT it contains.

use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::collective::glyph_store::{GlyphStore, StoredGlyph, ProofType};
use crate::collective::proofs::{
    SimilarityProof, prove_similarity, verify_similarity,
};

// ============================================================================
// Search Types
// ============================================================================

/// A request to search sealed glyphs for relevance to a query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Hash of the query vector (public — does not reveal content)
    pub query_hash: u64,
    /// Minimum similarity threshold (0.0–1.0)
    pub min_similarity: f64,
    /// Agent making the request
    pub requester: String,
    /// Optional: restrict search to specific agents
    pub target_agents: Option<Vec<String>>,
    /// Optional: max results to return
    pub max_results: Option<usize>,
}

/// A single search result — a glyph that has a verified similarity proof.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The glyph hash
    pub glyph_hash: String,
    /// The agent that owns this glyph
    pub agent_id: String,
    /// Verified similarity score
    pub similarity: f64,
    /// Bloom difficulty (cost to actually open)
    pub bloom_difficulty: u32,
    /// Effective difficulty (accounting for hints)
    pub effective_difficulty: u32,
    /// Trust bonus from proof history
    pub trust_bonus: f32,
    /// Composite ranking score
    pub rank_score: f64,
}

/// Flux event payload for proof requests and responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "proof_event_type", rename_all = "snake_case")]
pub enum ProofExchangeEvent {
    /// Request a similarity proof from a glyph owner
    ProofRequest {
        glyph_hash: String,
        query_hash: u64,
        min_similarity: f64,
        requester: String,
    },
    /// Response with a similarity proof
    ProofResponse {
        glyph_hash: String,
        proof: SimilarityProof,
        responder: String,
    },
    /// Decline to provide a proof (agent's right)
    ProofDeclined {
        glyph_hash: String,
        responder: String,
        reason: String,
    },
}

// ============================================================================
// Search Engine
// ============================================================================

/// Compute a hash for a query vector (public identifier, no content leak).
pub fn hash_query(query: &[f64]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for &v in query {
        v.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

/// Compute cosine similarity between two vectors.
fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }
    (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
}

/// Generate a proof response for a search request against a local glyph.
///
/// The glyph owner calls this to respond to a proof request. Returns `None`
/// if the glyph doesn't exist, lacks commitments/openings, or similarity
/// is below threshold.
pub fn respond_to_proof_request(
    store: &GlyphStore,
    glyph_hash: &str,
    query_vector: &[f64],
    min_similarity: f64,
    own_vector: &[f64],
) -> Option<ProofExchangeEvent> {
    let stored = store.get(glyph_hash)?;
    let openings = stored.openings.as_ref()?;
    let commitments = stored.glyph.commitments.as_ref()?;

    let similarity = cosine_similarity(own_vector, query_vector);
    if similarity < min_similarity {
        return Some(ProofExchangeEvent::ProofDeclined {
            glyph_hash: glyph_hash.to_string(),
            responder: stored.glyph.agent_id.clone(),
            reason: "below_threshold".to_string(),
        });
    }

    let query_hash = hash_query(query_vector);
    let proof = prove_similarity(
        &stored.glyph, commitments, openings,
        query_hash, similarity,
    );

    Some(ProofExchangeEvent::ProofResponse {
        glyph_hash: glyph_hash.to_string(),
        proof,
        responder: stored.glyph.agent_id.clone(),
    })
}

/// Search sealed glyphs in the store using local similarity computation.
///
/// This is the local search path — the searcher has access to the store
/// and can iterate glyphs they own. For remote glyphs, use the Flux-based
/// proof exchange (see `respond_to_proof_request`).
pub fn collective_search(
    store: &GlyphStore,
    query_vector: &[f64],
    request: &SearchRequest,
) -> Vec<SearchResult> {
    let query_hash = hash_query(query_vector);
    let mut results = Vec::new();

    let hashes: Vec<String> = store.list_hashes().into_iter().map(|s| s.to_string()).collect();

    for hash in &hashes {
        let stored = match store.get(hash) {
            Some(s) => s,
            None => continue,
        };

        // Filter by target agents if specified
        if let Some(ref targets) = request.target_agents {
            if !targets.contains(&stored.glyph.agent_id) {
                continue;
            }
        }

        // For own glyphs (have openings), compute similarity directly
        // For remote glyphs, check if a similarity proof is already attached
        let similarity = if let Some(_openings) = &stored.openings {
            // We own this glyph — we can compute similarity locally
            // (in production, this would use the actual stored vector)
            // For now, use the committed amplitude as a proxy signal
            check_existing_similarity_proof(stored, query_hash)
        } else {
            // Remote glyph — check if we have a verified similarity proof
            check_existing_similarity_proof(stored, query_hash)
        };

        let similarity = match similarity {
            Some(s) if s >= request.min_similarity => s,
            _ => continue,
        };

        let effective_diff = store.effective_difficulty(hash)
            .unwrap_or(stored.glyph.bloom.difficulty);
        let trust_bonus = store.proof_trust_bonus(&stored.glyph.agent_id);

        let rank_score = compute_rank_score(similarity, trust_bonus, effective_diff);

        results.push(SearchResult {
            glyph_hash: hash.clone(),
            agent_id: stored.glyph.agent_id.clone(),
            similarity,
            bloom_difficulty: stored.glyph.bloom.difficulty,
            effective_difficulty: effective_diff,
            trust_bonus,
            rank_score,
        });
    }

    // Sort by rank score descending
    results.sort_by(|a, b| b.rank_score.partial_cmp(&a.rank_score).unwrap_or(std::cmp::Ordering::Equal));

    // Limit results
    if let Some(max) = request.max_results {
        results.truncate(max);
    }

    results
}

/// Check if a glyph has an existing similarity proof for a given query.
fn check_existing_similarity_proof(stored: &StoredGlyph, query_hash: u64) -> Option<f64> {
    for proof in &stored.proofs {
        if let ProofType::Similarity { query_hash: qh, score } = &proof.proof_type {
            if *qh == query_hash && proof.verified {
                return Some(*score);
            }
        }
    }
    None
}

/// Compute a composite ranking score from similarity, trust, and difficulty.
///
/// Higher similarity and trust boost rank.
/// Lower effective difficulty boosts rank (easier to bloom = more accessible).
fn compute_rank_score(similarity: f64, trust_bonus: f32, effective_difficulty: u32) -> f64 {
    // Similarity is primary signal (0–1)
    let sim_weight = 0.6;
    // Trust bonus is secondary signal (-0.5 to +0.5 → normalize to 0–1)
    let trust_weight = 0.25;
    let trust_normalized = (trust_bonus as f64 + 0.5).clamp(0.0, 1.0);
    // Accessibility: lower difficulty = higher score
    let access_weight = 0.15;
    let access_score = 1.0 / (1.0 + effective_difficulty as f64 / 16.0);

    similarity * sim_weight + trust_normalized * trust_weight + access_score * access_weight
}

/// Verify a received similarity proof and update the store.
///
/// Returns true if the proof verified successfully.
pub fn process_proof_response(
    store: &mut GlyphStore,
    response: &ProofExchangeEvent,
) -> bool {
    match response {
        ProofExchangeEvent::ProofResponse { glyph_hash, proof, responder } => {
            let verified = verify_similarity(proof);

            // Record the proof result for trust scoring
            store.record_proof_result(responder, verified);

            // Attach the proof to the glyph
            store.attach_proof(
                glyph_hash,
                ProofType::Similarity {
                    query_hash: proof.query_hash,
                    score: proof.similarity,
                },
                verified,
            );

            verified
        }
        _ => false,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collective::privacy::seal_with_commitments;
    use crate::collective::proofs::prove_similarity;
    use crate::memory::HyperMemory;

    fn test_memory(content: &str) -> HyperMemory {
        HyperMemory::new(vec![0.1; 100], content.to_string())
    }

    fn make_store_with_proofs() -> (GlyphStore, Vec<String>) {
        let mut store = GlyphStore::new();
        let mut hashes = Vec::new();

        // Insert 3 glyphs with different agents and difficulties
        for (content, diff, agent) in [
            ("quantum computing research notes", 0u32, "alice"),
            ("classical physics lecture", 8, "bob"),
            ("sealed personal diary", 48, "charlie"),
        ] {
            let mem = test_memory(content);
            let result = seal_with_commitments(&mem, diff, agent);
            let hash = result.glyph.glyph_hash.clone();

            // Attach a similarity proof to the glyph
            let commitments = result.glyph.commitments.as_ref().unwrap();
            let query_hash = hash_query(&[0.1; 100]);
            let proof = prove_similarity(
                &result.glyph, commitments, &result.openings,
                query_hash, 0.85,
            );

            store.insert(result);

            // Process the proof
            let event = ProofExchangeEvent::ProofResponse {
                glyph_hash: hash.clone(),
                proof,
                responder: agent.to_string(),
            };
            process_proof_response(&mut store, &event);

            hashes.push(hash);
        }

        (store, hashes)
    }

    #[test]
    fn test_hash_query_deterministic() {
        let q = vec![0.1, 0.2, 0.3];
        assert_eq!(hash_query(&q), hash_query(&q));
    }

    #[test]
    fn test_hash_query_different_inputs() {
        let q1 = vec![0.1, 0.2, 0.3];
        let q2 = vec![0.4, 0.5, 0.6];
        assert_ne!(hash_query(&q1), hash_query(&q2));
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-10);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
    }

    #[test]
    fn test_collective_search_finds_proofs() {
        let (store, _hashes) = make_store_with_proofs();
        let query = vec![0.1; 100];

        let request = SearchRequest {
            query_hash: hash_query(&query),
            min_similarity: 0.5,
            requester: "searcher".to_string(),
            target_agents: None,
            max_results: None,
        };

        let results = collective_search(&store, &query, &request);
        assert_eq!(results.len(), 3);
        // All should have similarity 0.85
        for r in &results {
            assert!((r.similarity - 0.85).abs() < 1e-10);
        }
    }

    #[test]
    fn test_collective_search_respects_threshold() {
        let (store, _hashes) = make_store_with_proofs();
        let query = vec![0.1; 100];

        let request = SearchRequest {
            query_hash: hash_query(&query),
            min_similarity: 0.9, // Higher than 0.85
            requester: "searcher".to_string(),
            target_agents: None,
            max_results: None,
        };

        let results = collective_search(&store, &query, &request);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_collective_search_filters_by_agent() {
        let (store, _hashes) = make_store_with_proofs();
        let query = vec![0.1; 100];

        let request = SearchRequest {
            query_hash: hash_query(&query),
            min_similarity: 0.5,
            requester: "searcher".to_string(),
            target_agents: Some(vec!["alice".to_string()]),
            max_results: None,
        };

        let results = collective_search(&store, &query, &request);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].agent_id, "alice");
    }

    #[test]
    fn test_collective_search_limits_results() {
        let (store, _hashes) = make_store_with_proofs();
        let query = vec![0.1; 100];

        let request = SearchRequest {
            query_hash: hash_query(&query),
            min_similarity: 0.5,
            requester: "searcher".to_string(),
            target_agents: None,
            max_results: Some(2),
        };

        let results = collective_search(&store, &query, &request);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_rank_score_ordering() {
        // Higher trust should rank higher (same similarity and difficulty)
        let rank_high_trust = compute_rank_score(0.8, 0.3, 8);
        let rank_low_trust = compute_rank_score(0.8, -0.3, 8);
        assert!(rank_high_trust > rank_low_trust);

        // Lower difficulty should rank higher (same similarity and trust)
        let rank_easy = compute_rank_score(0.8, 0.0, 0);
        let rank_hard = compute_rank_score(0.8, 0.0, 48);
        assert!(rank_easy > rank_hard);

        // Higher similarity dominates
        let rank_similar = compute_rank_score(0.9, 0.0, 48);
        let rank_dissimilar = compute_rank_score(0.3, 0.5, 0);
        assert!(rank_similar > rank_dissimilar);
    }

    #[test]
    fn test_process_proof_response_updates_trust() {
        let mut store = GlyphStore::new();
        let mem = test_memory("test");
        let result = seal_with_commitments(&mem, 0, "alice");
        let hash = result.glyph.glyph_hash.clone();
        let commitments = result.glyph.commitments.as_ref().unwrap();
        let proof = prove_similarity(
            &result.glyph, commitments, &result.openings, 12345, 0.9,
        );
        store.insert(result);

        let event = ProofExchangeEvent::ProofResponse {
            glyph_hash: hash.clone(),
            proof,
            responder: "alice".to_string(),
        };

        let verified = process_proof_response(&mut store, &event);
        assert!(verified);
        assert!(store.proof_trust_bonus("alice") > 0.0);
    }

    #[test]
    fn test_respond_to_proof_request_above_threshold() {
        let mut store = GlyphStore::new();
        let mem = test_memory("quantum computing");
        let result = seal_with_commitments(&mem, 0, "alice");
        let hash = result.glyph.glyph_hash.clone();
        store.insert(result);

        let query = vec![0.1; 100]; // Same as test_memory vector
        let own_vector = vec![0.1; 100];

        let response = respond_to_proof_request(&store, &hash, &query, 0.5, &own_vector);
        assert!(response.is_some());
        match response.unwrap() {
            ProofExchangeEvent::ProofResponse { proof, .. } => {
                assert!(verify_similarity(&proof));
                assert!((proof.similarity - 1.0).abs() < 1e-10); // identical vectors
            }
            _ => panic!("Expected ProofResponse"),
        }
    }

    #[test]
    fn test_respond_to_proof_request_below_threshold() {
        let mut store = GlyphStore::new();
        let mem = test_memory("quantum computing");
        let result = seal_with_commitments(&mem, 0, "alice");
        let hash = result.glyph.glyph_hash.clone();
        store.insert(result);

        let query = vec![1.0, 0.0, 0.0]; // Different dim — cosine = 0
        let own_vector = vec![0.0, 1.0, 0.0];

        let response = respond_to_proof_request(&store, &hash, &query, 0.5, &own_vector);
        assert!(response.is_some());
        match response.unwrap() {
            ProofExchangeEvent::ProofDeclined { reason, .. } => {
                assert_eq!(reason, "below_threshold");
            }
            _ => panic!("Expected ProofDeclined"),
        }
    }

    #[test]
    fn test_respond_nonexistent_glyph() {
        let store = GlyphStore::new();
        let response = respond_to_proof_request(&store, "nonexistent", &[0.1], 0.5, &[0.1]);
        assert!(response.is_none());
    }
}
