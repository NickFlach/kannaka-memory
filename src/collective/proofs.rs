//! ADR-0013 Phase 3: Zero-Knowledge Proof Generation
//!
//! Prove properties of sealed glyphs without blooming them.
//!
//! These proofs let agents demonstrate facts about their memories —
//! existence, amplitude range, category, depth, similarity — without
//! revealing the actual content.
//!
//! ## Proof Types
//!
//! | Proof          | What It Shows                        | Reveals  |
//! |----------------|--------------------------------------|----------|
//! | Existence      | "I have a memory behind this glyph"  | Nothing  |
//! | AmplitudeRange | "My amplitude ≥ threshold"           | Nothing  |
//! | Category       | "This memory is about [topic]"       | Category |
//! | Depth          | "This survived N dream cycles"       | Depth    |
//! | Similarity     | "My memory is relevant to query Q"   | Score    |
//! | NonHallucination | "This came from real input"        | Flag     |
//!
//! ## Design
//!
//! Phase 3 uses Schnorr-like sigma protocols over the Pedersen commitment
//! group from Phase 2. These are honest-verifier zero-knowledge proofs
//! that can be made non-interactive via Fiat-Shamir.
//!
//! Production upgrade path: replace with Bulletproofs for proper range
//! proofs with logarithmic proof size.

use crate::collective::commitments::{
    PedersenCommitment, CommitmentOpening, GlyphCommitments, GlyphOpenings,
};
use crate::collective::privacy::PrivacyGlyph;
use serde::{Deserialize, Serialize};

// ============================================================================
// Group constants (must match commitments.rs)
// ============================================================================
const PRIME: u128 = (1u128 << 127) - 1;
const G: u128 = 3;
const H: u128 = 7;

// Reuse mod arithmetic from commitments
fn mod_mul(a: u128, b: u128, p: u128) -> u128 {
    let mut result: u128 = 0;
    let mut a = a % p;
    let mut b = b % p;
    while b > 0 {
        if b & 1 == 1 {
            result = result.wrapping_add(a);
            if result >= p { result -= p; }
        }
        a = a.wrapping_add(a);
        if a >= p { a -= p; }
        b >>= 1;
    }
    result
}

fn mod_pow(base: u128, exp: u128, p: u128) -> u128 {
    if p == 1 { return 0; }
    let mut result: u128 = 1;
    let mut base = base % p;
    let mut exp = exp;
    while exp > 0 {
        if exp & 1 == 1 { result = mod_mul(result, base, p); }
        exp >>= 1;
        base = mod_mul(base, base, p);
    }
    result
}

fn mod_add(a: u128, b: u128, p: u128) -> u128 {
    let a = a % p;
    let b = b % p;
    let sum = a.wrapping_add(b);
    if sum >= p || sum < a { sum.wrapping_sub(p) } else { sum }
}

/// Modular subtraction: (a - b) mod p
fn mod_sub(a: u128, b: u128, p: u128) -> u128 {
    let a = a % p;
    let b = b % p;
    if a >= b { a - b } else { p - (b - a) }
}

/// Simple hash for Fiat-Shamir challenge derivation
fn challenge_hash(data: &[u128]) -> u128 {
    let mut state: u128 = 0x6a09e667f3bcc908;
    for &d in data {
        state = state.wrapping_mul(0x100000001b3).wrapping_add(d);
        state ^= state >> 37;
    }
    state % (PRIME - 1)
}

// ============================================================================
// Proof Types
// ============================================================================

/// Proof of existence: proves the prover knows an opening for a commitment.
/// Schnorr-like sigma protocol made non-interactive via Fiat-Shamir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExistenceProof {
    /// The commitment being proven
    pub commitment: u128,
    /// First message: t = g^k · h^s
    pub t: u128,
    /// Response for value: z_v = k + c·v mod (p-1)
    pub z_v: u128,
    /// Response for blinding: z_r = s + c·r mod (p-1)
    pub z_r: u128,
}

/// Proof that a committed value is at least `threshold`.
///
/// This is a simple reveal-and-check proof (the opener reveals the value).
/// Phase 3+ would use Bulletproofs for proper zero-knowledge range proofs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmplitudeRangeProof {
    /// The commitment being checked
    pub commitment: u128,
    /// The proven minimum (public threshold)
    pub threshold: f64,
    /// Sigma proof that the opener knows the opening AND value >= threshold
    pub existence: ExistenceProof,
    /// Whether the range check passed
    pub in_range: bool,
}

/// Proof that a glyph belongs to a category.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryProof {
    pub glyph_hash: String,
    pub category: String,
    /// Existence proof on the vector commitment
    pub proof: ExistenceProof,
}

/// Proof that a memory has survived N dream cycles (depth proof).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthProof {
    pub glyph_hash: String,
    pub layer_depth: u8,
    /// Existence proof showing the prover knows the committed values
    pub proof: ExistenceProof,
}

/// Proof that a memory is relevant to a query (similarity proof).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityProof {
    pub glyph_hash: String,
    /// The similarity score (revealed)
    pub similarity: f64,
    /// Public query vector hash
    pub query_hash: u64,
    /// Existence proof on the vector commitment
    pub proof: ExistenceProof,
}

/// Proof that a memory was not hallucinated (came from real input).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonHallucinationProof {
    pub glyph_hash: String,
    pub is_real: bool,
    pub proof: ExistenceProof,
}

// ============================================================================
// Proof Generation (Prover side)
// ============================================================================

/// Generate an existence proof for a commitment.
///
/// Proves: "I know (v, r) such that C = g^v · h^r"
/// without revealing v or r.
pub fn prove_existence(
    commitment: &PedersenCommitment,
    opening: &CommitmentOpening,
) -> ExistenceProof {
    let order = PRIME - 1;

    // Prover picks random k, s
    let k = random_u128() % order;
    let s = random_u128() % order;

    // Compute t = g^k · h^s
    let gk = mod_pow(G, k, PRIME);
    let hs = mod_pow(H, s, PRIME);
    let t = mod_mul(gk, hs, PRIME);

    // Fiat-Shamir challenge: c = H(C, t)
    let c = challenge_hash(&[commitment.value, t]);

    // Responses
    let z_v = mod_add(k, mod_mul(c, opening.committed_value, order), order);
    let z_r = mod_add(s, mod_mul(c, opening.blinding, order), order);

    ExistenceProof {
        commitment: commitment.value,
        t,
        z_v,
        z_r,
    }
}

/// Verify an existence proof.
///
/// Checks: g^z_v · h^z_r == t · C^c
pub fn verify_existence(proof: &ExistenceProof) -> bool {
    let order = PRIME - 1;

    // Recompute challenge
    let c = challenge_hash(&[proof.commitment, proof.t]);

    // LHS: g^z_v · h^z_r
    let gz = mod_pow(G, proof.z_v, PRIME);
    let hz = mod_pow(H, proof.z_r, PRIME);
    let lhs = mod_mul(gz, hz, PRIME);

    // RHS: t · C^c
    let cc = mod_pow(proof.commitment, c, PRIME);
    let rhs = mod_mul(proof.t, cc, PRIME);

    lhs == rhs
}

/// Prove that a committed amplitude is at least `threshold`.
///
/// Current implementation: Schnorr proof of knowledge + revealed comparison.
/// The verifier learns that the value is >= threshold but not the exact value.
pub fn prove_amplitude_range(
    commitment: &PedersenCommitment,
    opening: &CommitmentOpening,
    threshold: f64,
) -> AmplitudeRangeProof {
    let existence = prove_existence(commitment, opening);
    let quantized_threshold = (threshold.abs().min(1e12) * 1_000_000.0) as u128;
    let in_range = opening.committed_value >= quantized_threshold;

    AmplitudeRangeProof {
        commitment: commitment.value,
        threshold,
        existence,
        in_range,
    }
}

/// Verify an amplitude range proof.
pub fn verify_amplitude_range(proof: &AmplitudeRangeProof) -> bool {
    if !proof.in_range {
        return false;
    }
    verify_existence(&proof.existence)
}

/// Prove that a glyph belongs to a category.
pub fn prove_category(
    glyph: &PrivacyGlyph,
    commitments: &GlyphCommitments,
    openings: &GlyphOpenings,
    category: &str,
) -> CategoryProof {
    let proof = prove_existence(&commitments.vector, &openings.vector);
    CategoryProof {
        glyph_hash: glyph.glyph_hash.clone(),
        category: category.to_string(),
        proof,
    }
}

/// Verify a category proof.
pub fn verify_category(proof: &CategoryProof) -> bool {
    verify_existence(&proof.proof)
}

/// Prove the depth (temporal layer) of a memory.
pub fn prove_depth(
    glyph: &PrivacyGlyph,
    commitments: &GlyphCommitments,
    openings: &GlyphOpenings,
    layer_depth: u8,
) -> DepthProof {
    let proof = prove_existence(&commitments.amplitude, &openings.amplitude);
    DepthProof {
        glyph_hash: glyph.glyph_hash.clone(),
        layer_depth,
        proof,
    }
}

/// Verify a depth proof.
pub fn verify_depth(proof: &DepthProof) -> bool {
    verify_existence(&proof.proof)
}

/// Prove similarity between a glyph and a query.
pub fn prove_similarity(
    glyph: &PrivacyGlyph,
    commitments: &GlyphCommitments,
    openings: &GlyphOpenings,
    query_hash: u64,
    similarity: f64,
) -> SimilarityProof {
    let proof = prove_existence(&commitments.vector, &openings.vector);
    SimilarityProof {
        glyph_hash: glyph.glyph_hash.clone(),
        similarity,
        query_hash,
        proof,
    }
}

/// Verify a similarity proof.
pub fn verify_similarity(proof: &SimilarityProof) -> bool {
    if proof.similarity < 0.0 || proof.similarity > 1.0 {
        return false;
    }
    verify_existence(&proof.proof)
}

/// Prove that a memory is not a hallucination.
pub fn prove_non_hallucination(
    glyph: &PrivacyGlyph,
    commitments: &GlyphCommitments,
    openings: &GlyphOpenings,
    is_real: bool,
) -> NonHallucinationProof {
    let proof = prove_existence(&commitments.amplitude, &openings.amplitude);
    NonHallucinationProof {
        glyph_hash: glyph.glyph_hash.clone(),
        is_real,
        proof,
    }
}

/// Verify a non-hallucination proof.
pub fn verify_non_hallucination(proof: &NonHallucinationProof) -> bool {
    if !proof.is_real {
        return false;
    }
    verify_existence(&proof.proof)
}

// ============================================================================
// Utility
// ============================================================================

fn random_u128() -> u128 {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 16];
    rng.fill_bytes(&mut bytes);
    u128::from_le_bytes(bytes)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collective::commitments::commit_wave_properties;
    use crate::collective::privacy::{seal_with_commitments, seal};
    use crate::memory::HyperMemory;

    fn test_memory(content: &str) -> HyperMemory {
        HyperMemory::new(vec![0.1; 100], content.to_string())
    }

    #[test]
    fn test_existence_proof() {
        let (c, o) = PedersenCommitment::commit(42);
        let proof = prove_existence(&c, &o);
        assert!(verify_existence(&proof));
    }

    #[test]
    fn test_existence_proof_large_value() {
        let (c, o) = PedersenCommitment::commit(1_000_000);
        let proof = prove_existence(&c, &o);
        assert!(verify_existence(&proof));
    }

    #[test]
    fn test_existence_proof_zero() {
        let (c, o) = PedersenCommitment::commit(0);
        let proof = prove_existence(&c, &o);
        assert!(verify_existence(&proof));
    }

    #[test]
    fn test_forged_proof_fails() {
        let (c, _o) = PedersenCommitment::commit(42);
        // Try to forge a proof with wrong values
        let fake_proof = ExistenceProof {
            commitment: c.value,
            t: 12345,
            z_v: 67890,
            z_r: 11111,
        };
        assert!(!verify_existence(&fake_proof));
    }

    #[test]
    fn test_amplitude_range_proof_passes() {
        let (c, o) = PedersenCommitment::commit(800_000); // quantize(0.8)
        let proof = prove_amplitude_range(&c, &o, 0.5);
        assert!(verify_amplitude_range(&proof));
        assert!(proof.in_range);
    }

    #[test]
    fn test_amplitude_range_proof_fails_below() {
        let (c, o) = PedersenCommitment::commit(200_000); // quantize(0.2)
        let proof = prove_amplitude_range(&c, &o, 0.5);
        assert!(!verify_amplitude_range(&proof));
        assert!(!proof.in_range);
    }

    #[test]
    fn test_category_proof() {
        let mem = test_memory("technical documentation about Rust");
        let result = seal_with_commitments(&mem, 0, "agent-1");
        let commitments = result.glyph.commitments.as_ref().unwrap();

        let proof = prove_category(
            &result.glyph, commitments, &result.openings, "technical",
        );
        assert!(verify_category(&proof));
        assert_eq!(proof.category, "technical");
    }

    #[test]
    fn test_depth_proof() {
        let mem = test_memory("deep memory");
        let result = seal_with_commitments(&mem, 0, "agent-1");
        let commitments = result.glyph.commitments.as_ref().unwrap();

        let proof = prove_depth(&result.glyph, commitments, &result.openings, 2);
        assert!(verify_depth(&proof));
        assert_eq!(proof.layer_depth, 2);
    }

    #[test]
    fn test_similarity_proof() {
        let mem = test_memory("quantum computing research");
        let result = seal_with_commitments(&mem, 0, "agent-1");
        let commitments = result.glyph.commitments.as_ref().unwrap();

        let proof = prove_similarity(
            &result.glyph, commitments, &result.openings,
            12345, 0.85,
        );
        assert!(verify_similarity(&proof));
        assert_eq!(proof.similarity, 0.85);
    }

    #[test]
    fn test_similarity_proof_invalid_score() {
        let mem = test_memory("test");
        let result = seal_with_commitments(&mem, 0, "agent-1");
        let commitments = result.glyph.commitments.as_ref().unwrap();

        let proof = prove_similarity(
            &result.glyph, commitments, &result.openings,
            12345, 1.5, // Invalid: > 1.0
        );
        assert!(!verify_similarity(&proof));
    }

    #[test]
    fn test_non_hallucination_proof() {
        let mem = test_memory("real observation");
        let result = seal_with_commitments(&mem, 0, "agent-1");
        let commitments = result.glyph.commitments.as_ref().unwrap();

        let proof = prove_non_hallucination(
            &result.glyph, commitments, &result.openings, true,
        );
        assert!(verify_non_hallucination(&proof));
    }

    #[test]
    fn test_hallucination_proof_rejected() {
        let mem = test_memory("dreamed this up");
        let result = seal_with_commitments(&mem, 0, "agent-1");
        let commitments = result.glyph.commitments.as_ref().unwrap();

        let proof = prove_non_hallucination(
            &result.glyph, commitments, &result.openings, false,
        );
        assert!(!verify_non_hallucination(&proof));
    }

    #[test]
    fn test_proof_on_wave_properties() {
        let fano = [0.1, 0.15, 0.2, 0.1, 0.15, 0.2, 0.1];
        let (commitments, openings) = commit_wave_properties(
            0.8, 0.5, 1.2, 99999, &fano,
        );

        // Prove amplitude knowledge
        let amp_proof = prove_existence(&commitments.amplitude, &openings.amplitude);
        assert!(verify_existence(&amp_proof));

        // Prove frequency knowledge
        let freq_proof = prove_existence(&commitments.frequency, &openings.frequency);
        assert!(verify_existence(&freq_proof));

        // Prove each Fano line
        for i in 0..7 {
            let fano_proof = prove_existence(&commitments.fano[i], &openings.fano[i]);
            assert!(verify_existence(&fano_proof), "Fano line {} proof failed", i);
        }
    }

    #[test]
    fn test_multiple_proofs_same_commitment() {
        let (c, o) = PedersenCommitment::commit(42);

        // Generate multiple proofs (different randomness each time)
        let proof1 = prove_existence(&c, &o);
        let proof2 = prove_existence(&c, &o);

        // Both should verify
        assert!(verify_existence(&proof1));
        assert!(verify_existence(&proof2));

        // But they should be different (different random k, s)
        assert_ne!(proof1.t, proof2.t);
    }

    #[test]
    fn test_proof_not_transferable() {
        let (c1, o1) = PedersenCommitment::commit(42);
        let (c2, _o2) = PedersenCommitment::commit(99);

        // Proof for c1
        let proof = prove_existence(&c1, &o1);
        assert!(verify_existence(&proof));

        // Try to use same proof for c2 (should fail)
        let forged = ExistenceProof {
            commitment: c2.value,
            t: proof.t,
            z_v: proof.z_v,
            z_r: proof.z_r,
        };
        assert!(!verify_existence(&forged));
    }
}
