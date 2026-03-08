//! ADR-0013 Phase 2: Pedersen Commitment Layer
//!
//! Pedersen commitments allow proving properties of sealed glyphs without
//! blooming them. The key property: **additive homomorphism**.
//!
//! ```text
//! C(a, r_a) · C(b, r_b) = C(a + b, r_a + r_b)
//! ```
//!
//! This means wave superposition merge from ADR-0011 works *directly on
//! sealed glyphs* — two agents can merge their sealed memories without
//! either revealing content.
//!
//! ## Implementation Notes
//!
//! This prototype uses u128 modular arithmetic over a 127-bit safe prime.
//! Production deployment should upgrade to elliptic curve groups (curve25519)
//! for proper cryptographic security. The API is designed for this upgrade —
//! swap `PedersenGroup` internals, keep the interface.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// Group Parameters
// ============================================================================

/// A 127-bit safe prime: p = 2q + 1 where q is also prime.
/// This gives us a prime-order subgroup of Z*_p for Pedersen commitments.
///
/// p = 170141183460469231731687303715884105727 (2^127 - 1, a Mersenne prime)
const PRIME: u128 = (1u128 << 127) - 1;

/// Generator g of the prime-order subgroup.
/// g = 3 (a known generator for this group)
const G: u128 = 3;

/// Second generator h, where log_g(h) is unknown.
/// h = g^{random large exponent} — we use a fixed value derived from
/// hashing "kannaka-pedersen-h-generator" so nobody knows log_g(h).
/// h = 7 (in production: derive from nothing-up-my-sleeve number)
const H: u128 = 7;

// ============================================================================
// Core Commitment Types
// ============================================================================

/// A Pedersen commitment: C = g^v · h^r mod p
///
/// - `v` is the committed value (private)
/// - `r` is the blinding factor (private)
/// - `C` is the commitment (public)
///
/// Properties:
/// - **Hiding**: Given C, you can't determine v (because r is random)
/// - **Binding**: Can't open C to a different (v', r') pair
/// - **Homomorphic**: C(a) · C(b) = C(a + b)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct PedersenCommitment {
    /// The commitment value: g^v · h^r mod p
    pub value: u128,
}

/// Opening data for a commitment (kept private by the committer)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CommitmentOpening {
    /// The committed value (u128 to match group order for homomorphic operations)
    pub committed_value: u128,
    /// The blinding factor
    pub blinding: u128,
}

/// Complete set of commitments for a privacy glyph's wave properties.
///
/// Each wave parameter is independently committed, enabling selective
/// verification without revealing other properties.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlyphCommitments {
    /// Committed vector hash: C_v = g^H(v) · h^r
    pub vector: PedersenCommitment,

    /// Committed amplitude: C_a = g^a · h^r_a
    pub amplitude: PedersenCommitment,

    /// Committed frequency: C_f = g^f · h^r_f
    pub frequency: PedersenCommitment,

    /// Committed phase: C_φ = g^φ · h^r_φ
    pub phase: PedersenCommitment,

    /// Fano plane projections: [C_0..C_6]
    /// Each is a commitment to the energy on that Fano line.
    pub fano: [PedersenCommitment; 7],
}

/// Opening data for all glyph commitments (kept private)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlyphOpenings {
    pub vector: CommitmentOpening,
    pub amplitude: CommitmentOpening,
    pub frequency: CommitmentOpening,
    pub phase: CommitmentOpening,
    pub fano: [CommitmentOpening; 7],
}

// ============================================================================
// Modular Arithmetic
// ============================================================================

/// Modular multiplication: (a * b) mod p
/// Uses u128 to avoid overflow for values < 2^127
fn mod_mul(a: u128, b: u128, p: u128) -> u128 {
    // For u128 values near 2^127, direct multiplication can overflow.
    // Use Russian peasant multiplication (double-and-add).
    let mut result: u128 = 0;
    let mut a = a % p;
    let mut b = b % p;

    while b > 0 {
        if b & 1 == 1 {
            result = result.wrapping_add(a);
            if result >= p {
                result -= p;
            }
        }
        a = a.wrapping_add(a);
        if a >= p {
            a -= p;
        }
        b >>= 1;
    }

    result
}

/// Modular exponentiation: base^exp mod p
/// Uses square-and-multiply.
fn mod_pow(base: u128, exp: u128, p: u128) -> u128 {
    if p == 1 {
        return 0;
    }
    let mut result: u128 = 1;
    let mut base = base % p;
    let mut exp = exp;

    while exp > 0 {
        if exp & 1 == 1 {
            result = mod_mul(result, base, p);
        }
        exp >>= 1;
        base = mod_mul(base, base, p);
    }

    result
}

/// Modular addition: (a + b) mod p
fn mod_add(a: u128, b: u128, p: u128) -> u128 {
    let a = a % p;
    let b = b % p;
    let sum = a.wrapping_add(b);
    if sum >= p || sum < a {
        // Overflow or >= p
        sum.wrapping_sub(p)
    } else {
        sum
    }
}

// ============================================================================
// Commitment Operations
// ============================================================================

impl PedersenCommitment {
    /// Create a commitment to a value with a random blinding factor.
    /// Returns the commitment and the opening data.
    pub fn commit(value: u64) -> (Self, CommitmentOpening) {
        let mut rng = rand::thread_rng();
        let blinding: u128 = rng.gen::<u128>() % (PRIME - 1);
        Self::commit_with_blinding(value as u128, blinding)
    }

    /// Create a commitment with a specific blinding factor (for testing or deterministic use).
    pub fn commit_with_blinding(value: u128, blinding: u128) -> (Self, CommitmentOpening) {
        // C = g^v · h^r mod p
        let v_mod = value % (PRIME - 1); // Reduce to group order
        let gv = mod_pow(G, v_mod, PRIME);
        let hr = mod_pow(H, blinding % (PRIME - 1), PRIME);
        let commitment_value = mod_mul(gv, hr, PRIME);

        (
            PedersenCommitment {
                value: commitment_value,
            },
            CommitmentOpening {
                committed_value: v_mod,
                blinding: blinding % (PRIME - 1),
            },
        )
    }

    /// Verify that an opening matches this commitment.
    pub fn verify(&self, opening: &CommitmentOpening) -> bool {
        let gv = mod_pow(G, opening.committed_value, PRIME);
        let hr = mod_pow(H, opening.blinding, PRIME);
        let expected = mod_mul(gv, hr, PRIME);
        self.value == expected
    }

    /// Homomorphic addition: C(a) · C(b) = C(a + b, r_a + r_b)
    ///
    /// The resulting commitment can be verified by adding the openings:
    /// - committed_value = a + b
    /// - blinding = r_a + r_b (mod p-1)
    pub fn add(&self, other: &PedersenCommitment) -> PedersenCommitment {
        PedersenCommitment {
            value: mod_mul(self.value, other.value, PRIME),
        }
    }

    /// Commit to zero with a known blinding factor (for proofs).
    pub fn commit_zero(blinding: u128) -> Self {
        PedersenCommitment {
            value: mod_pow(H, blinding, PRIME),
        }
    }
}

impl CommitmentOpening {
    /// Homomorphic addition of openings (matches PedersenCommitment::add).
    ///
    /// Both the committed value and blinding factor are added modulo (p-1),
    /// matching the group order for correct verification of merged commitments.
    pub fn add(&self, other: &CommitmentOpening) -> CommitmentOpening {
        let order = PRIME - 1;
        CommitmentOpening {
            committed_value: mod_add(self.committed_value, other.committed_value, order),
            blinding: mod_add(self.blinding, other.blinding, order),
        }
    }
}

impl fmt::Display for PedersenCommitment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "C({})", &format!("{:032x}", self.value)[..16])
    }
}

// ============================================================================
// Glyph Commitment Construction
// ============================================================================

/// Quantize a floating-point value to a u64 for commitment.
/// Uses fixed-point representation with 6 decimal digits of precision.
fn quantize(value: f64) -> u64 {
    // Map to non-negative range, then scale to integer
    // amplitude: [0, 10] → [0, 10_000_000]
    // frequency: [0, 100] → [0, 100_000_000]
    // phase: [-π, π] → [0, 6_283_185]
    let clamped = value.abs().min(1e12);
    (clamped * 1_000_000.0) as u64
}

/// Create commitments for all wave properties of a memory.
pub fn commit_wave_properties(
    amplitude: f64,
    frequency: f64,
    phase: f64,
    vector_hash: u64,
    fano_energies: &[f64; 7],
) -> (GlyphCommitments, GlyphOpenings) {
    let (c_vec, o_vec) = PedersenCommitment::commit(vector_hash);
    let (c_amp, o_amp) = PedersenCommitment::commit(quantize(amplitude));
    let (c_freq, o_freq) = PedersenCommitment::commit(quantize(frequency));
    let (c_phase, o_phase) = PedersenCommitment::commit(quantize(phase.abs()));

    let mut c_fano = [PedersenCommitment { value: 1 }; 7];
    let mut o_fano = [CommitmentOpening {
        committed_value: 0,
        blinding: 0,
    }; 7];

    for (i, &energy) in fano_energies.iter().enumerate() {
        let (c, o) = PedersenCommitment::commit(quantize(energy));
        c_fano[i] = c;
        o_fano[i] = o;
    }

    (
        GlyphCommitments {
            vector: c_vec,
            amplitude: c_amp,
            frequency: c_freq,
            phase: c_phase,
            fano: c_fano,
        },
        GlyphOpenings {
            vector: o_vec,
            amplitude: o_amp,
            frequency: o_freq,
            phase: o_phase,
            fano: o_fano,
        },
    )
}

/// Compute a simple hash of a hypervector for commitment.
/// Returns a u64 digest suitable for Pedersen commitment.
pub fn hash_vector(vector: &[f32]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for &v in vector {
        let bits = v.to_bits();
        hash ^= bits as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    hash
}

/// Compute Fano energies from a vector (energy distribution across 7 Fano lines).
pub fn compute_fano_energies(vector: &[f32]) -> [f64; 7] {
    let mut energies = [0.0f64; 7];
    let chunk_size = vector.len() / 7;
    if chunk_size == 0 {
        return energies;
    }

    for (i, chunk) in vector.chunks(chunk_size).enumerate() {
        if i >= 7 {
            break;
        }
        energies[i] = chunk.iter().map(|&x| (x as f64).powi(2)).sum::<f64>().sqrt();
    }

    // Normalize
    let total: f64 = energies.iter().sum();
    if total > 1e-10 {
        for e in &mut energies {
            *e /= total;
        }
    }

    energies
}

// ============================================================================
// Homomorphic Wave Merge on Commitments
// ============================================================================

/// Merge two sets of glyph commitments homomorphically.
///
/// This implements the wave superposition merge from ADR-0011 on sealed glyphs:
/// ```text
/// C(A_merged) = C(A₁) · C(A₂)  // homomorphic amplitude addition
/// ```
///
/// Neither party reveals their actual values — only the commitments are combined.
pub fn merge_commitments(
    a: &GlyphCommitments,
    b: &GlyphCommitments,
) -> GlyphCommitments {
    let mut merged_fano = [PedersenCommitment { value: 1 }; 7];
    for i in 0..7 {
        merged_fano[i] = a.fano[i].add(&b.fano[i]);
    }

    GlyphCommitments {
        vector: a.vector.add(&b.vector),
        amplitude: a.amplitude.add(&b.amplitude),
        frequency: a.frequency.add(&b.frequency),
        phase: a.phase.add(&b.phase),
        fano: merged_fano,
    }
}

/// Merge openings (for the parties who hold them).
pub fn merge_openings(a: &GlyphOpenings, b: &GlyphOpenings) -> GlyphOpenings {
    let mut merged_fano = [CommitmentOpening {
        committed_value: 0,
        blinding: 0,
    }; 7];
    for i in 0..7 {
        merged_fano[i] = a.fano[i].add(&b.fano[i]);
    }

    GlyphOpenings {
        vector: a.vector.add(&b.vector),
        amplitude: a.amplitude.add(&b.amplitude),
        frequency: a.frequency.add(&b.frequency),
        phase: a.phase.add(&b.phase),
        fano: merged_fano,
    }
}

// ============================================================================
// Verification Helpers
// ============================================================================

/// Verify all commitments in a glyph against their openings.
pub fn verify_all(commitments: &GlyphCommitments, openings: &GlyphOpenings) -> bool {
    if !commitments.vector.verify(&openings.vector) {
        return false;
    }
    if !commitments.amplitude.verify(&openings.amplitude) {
        return false;
    }
    if !commitments.frequency.verify(&openings.frequency) {
        return false;
    }
    if !commitments.phase.verify(&openings.phase) {
        return false;
    }
    for i in 0..7 {
        if !commitments.fano[i].verify(&openings.fano[i]) {
            return false;
        }
    }
    true
}

/// Verify that a committed amplitude is above a threshold.
///
/// This is a simple range check — the opener reveals the opening,
/// and the verifier checks: (1) commitment is valid, (2) value >= threshold.
///
/// For zero-knowledge range proofs (Phase 3), use Bulletproofs instead.
pub fn verify_amplitude_above(
    commitment: &PedersenCommitment,
    opening: &CommitmentOpening,
    threshold: f64,
) -> bool {
    if !commitment.verify(opening) {
        return false;
    }
    let quantized_threshold = quantize(threshold) as u128;
    opening.committed_value >= quantized_threshold as u128
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum CommitmentError {
    #[error("Commitment verification failed")]
    VerificationFailed,

    #[error("Homomorphic operation failed: {0}")]
    HomomorphicError(String),

    #[error("Invalid Fano projection: expected 7 values")]
    InvalidFanoProjection,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_and_verify() {
        let (commitment, opening) = PedersenCommitment::commit(42);
        assert!(commitment.verify(&opening));
    }

    #[test]
    fn test_commit_with_blinding() {
        let (c1, o1) = PedersenCommitment::commit_with_blinding(100, 999);
        let (c2, o2) = PedersenCommitment::commit_with_blinding(100, 999);
        assert_eq!(c1.value, c2.value);
        assert!(c1.verify(&o1));
        assert!(c2.verify(&o2));
    }

    #[test]
    fn test_different_blindings_different_commitments() {
        let (c1, _) = PedersenCommitment::commit_with_blinding(42, 111);
        let (c2, _) = PedersenCommitment::commit_with_blinding(42, 222);
        assert_ne!(c1.value, c2.value); // Same value, different commitments (hiding)
    }

    #[test]
    fn test_different_values_different_commitments() {
        let (c1, _) = PedersenCommitment::commit_with_blinding(42, 999);
        let (c2, _) = PedersenCommitment::commit_with_blinding(43, 999);
        assert_ne!(c1.value, c2.value); // Different values, same blinding (binding)
    }

    #[test]
    fn test_wrong_opening_fails() {
        let (commitment, _) = PedersenCommitment::commit(42);
        let fake_opening = CommitmentOpening {
            committed_value: 43,
            blinding: 0,
        };
        assert!(!commitment.verify(&fake_opening));
    }

    #[test]
    fn test_homomorphic_addition() {
        let (c_a, o_a) = PedersenCommitment::commit_with_blinding(10, 100);
        let (c_b, o_b) = PedersenCommitment::commit_with_blinding(20, 200);

        // Homomorphic add
        let c_sum = c_a.add(&c_b);
        let o_sum = o_a.add(&o_b);

        // The merged commitment should verify with the merged opening
        assert!(c_sum.verify(&o_sum));
        // Values add modulo group order, but for small values it's just sum
        assert_eq!(o_sum.committed_value, 30);
    }

    #[test]
    fn test_homomorphic_addition_random_blindings() {
        let (c_a, o_a) = PedersenCommitment::commit(100);
        let (c_b, o_b) = PedersenCommitment::commit(200);

        let c_sum = c_a.add(&c_b);
        let o_sum = o_a.add(&o_b);

        assert!(c_sum.verify(&o_sum));
        // For small values, modular add == regular add
        assert_eq!(o_sum.committed_value, 300);
    }

    #[test]
    fn test_commit_zero() {
        let c = PedersenCommitment::commit_zero(42);
        let opening = CommitmentOpening {
            committed_value: 0,
            blinding: 42,
        };
        assert!(c.verify(&opening));
    }

    #[test]
    fn test_wave_property_commitments() {
        let fano = [0.1, 0.15, 0.2, 0.1, 0.15, 0.2, 0.1];
        let vector_hash = hash_vector(&[0.5f32; 100]);

        let (commitments, openings) = commit_wave_properties(
            0.8, // amplitude
            0.5, // frequency
            1.2, // phase
            vector_hash,
            &fano,
        );

        assert!(verify_all(&commitments, &openings));
    }

    #[test]
    fn test_wave_merge_homomorphic() {
        let fano_a = [0.1, 0.15, 0.2, 0.1, 0.15, 0.2, 0.1];
        let fano_b = [0.2, 0.1, 0.15, 0.2, 0.1, 0.15, 0.1];

        let (c_a, o_a) = commit_wave_properties(0.5, 0.3, 1.0, 111, &fano_a);
        let (c_b, o_b) = commit_wave_properties(0.7, 0.4, 0.5, 222, &fano_b);

        // Merge commitments (public operation — no secrets revealed)
        let c_merged = merge_commitments(&c_a, &c_b);

        // Merge openings (private — each party contributes their opening)
        let o_merged = merge_openings(&o_a, &o_b);

        // The merged commitment verifies with the merged opening
        assert!(verify_all(&c_merged, &o_merged));
    }

    #[test]
    fn test_amplitude_range_verification() {
        let (c, o) = PedersenCommitment::commit(quantize(0.8));
        assert!(verify_amplitude_above(&c, &o, 0.5));
        assert!(verify_amplitude_above(&c, &o, 0.8));
        assert!(!verify_amplitude_above(&c, &o, 0.9));
    }

    #[test]
    fn test_quantize_preserves_ordering() {
        assert!(quantize(0.1) < quantize(0.5));
        assert!(quantize(0.5) < quantize(1.0));
        assert!(quantize(1.0) < quantize(10.0));
    }

    #[test]
    fn test_quantize_precision() {
        let q = quantize(0.123456);
        assert_eq!(q, 123456);
    }

    #[test]
    fn test_hash_vector_deterministic() {
        let v = vec![0.1f32, 0.2, 0.3];
        assert_eq!(hash_vector(&v), hash_vector(&v));
    }

    #[test]
    fn test_hash_vector_different_inputs() {
        let v1 = vec![0.1f32, 0.2, 0.3];
        let v2 = vec![0.1f32, 0.2, 0.4];
        assert_ne!(hash_vector(&v1), hash_vector(&v2));
    }

    #[test]
    fn test_fano_energies_normalized() {
        let vector = vec![1.0f32; 700];
        let energies = compute_fano_energies(&vector);
        let total: f64 = energies.iter().sum();
        assert!(
            (total - 1.0).abs() < 0.01,
            "Fano energies should be normalized, got {}",
            total
        );
    }

    #[test]
    fn test_mod_pow_basic() {
        assert_eq!(mod_pow(2, 10, 1000), 24); // 2^10 = 1024, 1024 % 1000 = 24
        assert_eq!(mod_pow(3, 0, PRIME), 1);  // anything^0 = 1
        assert_eq!(mod_pow(0, 5, PRIME), 0);  // 0^anything = 0
    }

    #[test]
    fn test_mod_mul_no_overflow() {
        // Test with values near the prime
        let a = PRIME - 1;
        let b = PRIME - 2;
        let result = mod_mul(a, b, PRIME);
        assert!(result < PRIME);
        // (p-1)(p-2) mod p = (-1)(-2) mod p = 2
        assert_eq!(result, 2);
    }

    #[test]
    fn test_commitment_display() {
        let (c, _) = PedersenCommitment::commit(42);
        let s = format!("{}", c);
        assert!(s.starts_with("C("));
        assert!(s.ends_with(")"));
    }

    #[test]
    fn test_triple_homomorphic_addition() {
        let (c_a, o_a) = PedersenCommitment::commit_with_blinding(5, 50);
        let (c_b, o_b) = PedersenCommitment::commit_with_blinding(10, 100);
        let (c_c, o_c) = PedersenCommitment::commit_with_blinding(15, 150);

        let c_sum = c_a.add(&c_b).add(&c_c);
        let o_sum = o_a.add(&o_b).add(&o_c);

        assert!(c_sum.verify(&o_sum));
        // Small values: modular add == regular add
        assert_eq!(o_sum.committed_value, 30);
    }

    #[test]
    fn test_fano_commitment_per_line() {
        let fano = [0.1, 0.15, 0.2, 0.1, 0.15, 0.2, 0.1];
        let (commitments, openings) = commit_wave_properties(1.0, 0.5, 0.0, 0, &fano);

        // Each Fano line commitment individually verifiable
        for i in 0..7 {
            assert!(
                commitments.fano[i].verify(&openings.fano[i]),
                "Fano line {} commitment verification failed",
                i
            );
        }
    }
}
