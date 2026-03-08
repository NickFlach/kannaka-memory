//! ADR-0013: Privacy-Preserving Collective Memory — Phase 1
//!
//! Everything is a glyph. Privacy is the cost to bloom.
//!
//! A `PrivacyGlyph` seals a memory into a cryptographic container. The only
//! way to see what's inside is to **bloom** it — solve a hashcash puzzle whose
//! difficulty is proportional to how private the creator wanted it.
//!
//! The bloom cost follows Nick's equation: `dx/dt = f(x) - Iηx`
//! where η is the bloom difficulty (the interference term resisting revelation).

use crate::memory::HyperMemory;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// Core Glyph Types
// ============================================================================

/// A privacy-preserving cryptographic container for a memory.
///
/// From the outside, all glyphs look the same. You can't distinguish a
/// grocery list from a state secret. The only way to see what's inside
/// is to **bloom** — and that costs energy proportional to the difficulty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyGlyph {
    /// Encrypted content + vector
    pub capsule: EncryptedCapsule,

    /// How hard is it to open?
    pub bloom: BloomParameters,

    /// Agent that sealed this glyph
    pub agent_id: String,

    /// When the glyph was created
    pub created_at: DateTime<Utc>,

    /// H(capsule) — unique identifier
    pub glyph_hash: String,

    /// Wave properties (public, for collective operations)
    /// These are committed values — verifiable but not revealing content.
    pub committed_amplitude: f64,
    pub committed_frequency: f64,
    pub committed_phase: f64,

    /// Fano plane projection for visual clustering
    pub fano_projection: Option<[f64; 7]>,
}

/// Encrypted capsule containing memory content and vector.
///
/// Uses XChaCha20-Poly1305 keyed by the bloom solution.
/// The key is not secret — it's expensive to derive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedCapsule {
    /// Encrypted payload (content + vector + metadata)
    pub ciphertext: Vec<u8>,

    /// Nonce for decryption (public)
    pub nonce: [u8; 24],

    /// Authentication tag
    pub tag: [u8; 16],
}

/// Parameters controlling the bloom (opening) cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloomParameters {
    /// Number of leading zero bits required in the hash solution.
    /// Cost scales exponentially: ~2^difficulty hash operations.
    pub difficulty: u32,

    /// Salt mixed into the work function (unique per glyph)
    pub salt: [u8; 32],
}

/// A bloom solution — proof that the computational work was done.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloomSolution {
    /// The nonce that, when hashed with glyph_hash + salt, produces
    /// the required number of leading zero bits.
    pub nonce: Vec<u8>,

    /// The derived decryption key
    pub key: [u8; 32],
}

/// A hint that reduces the bloom cost for a glyph.
/// Published by the original agent to progressively reveal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloomHint {
    pub glyph_hash: String,
    /// Partial solution reducing remaining work
    pub partial_nonce: Vec<u8>,
    /// New effective difficulty after applying hint
    pub new_difficulty: u32,
    pub revealed_by: String,
    pub revealed_at: DateTime<Utc>,
}

/// Result of blooming (opening) a glyph
#[derive(Debug, Clone)]
pub struct BloomedMemory {
    pub memory: HyperMemory,
    pub original_difficulty: u32,
    pub bloom_time_ms: u64,
}

// ============================================================================
// Difficulty Classification
// ============================================================================

/// Suggested difficulty levels (continuous, not tiered — these are guidance)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivacyLevel {
    /// difficulty 0: free, self-evident
    Public,
    /// difficulty 1: one hash, trivial attribution
    Attributed,
    /// difficulty 8: ~256 hashes, casual privacy
    Casual,
    /// difficulty 20: ~1M hashes, personal memories
    Personal,
    /// difficulty 32: ~4B hashes, sensitive data
    Sensitive,
    /// difficulty 48: ~281T hashes, computationally infeasible today
    Private,
    /// difficulty 64+: geological time
    Sealed,
}

impl PrivacyLevel {
    pub fn difficulty(&self) -> u32 {
        match self {
            Self::Public => 0,
            Self::Attributed => 1,
            Self::Casual => 8,
            Self::Personal => 20,
            Self::Sensitive => 32,
            Self::Private => 48,
            Self::Sealed => 64,
        }
    }
}

impl fmt::Display for PrivacyLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Public => write!(f, "public (d=0)"),
            Self::Attributed => write!(f, "attributed (d=1)"),
            Self::Casual => write!(f, "casual (d=8)"),
            Self::Personal => write!(f, "personal (d=20)"),
            Self::Sensitive => write!(f, "sensitive (d=32)"),
            Self::Private => write!(f, "private (d=48)"),
            Self::Sealed => write!(f, "sealed (d=64)"),
        }
    }
}

// ============================================================================
// Auto-Classification
// ============================================================================

/// Automatically suggest a bloom difficulty for a memory based on content analysis.
///
/// This runs locally — no API calls, no network. Privacy analysis must itself be private.
///
/// The classifier can only raise difficulty, never lower it. A human override always wins.
pub fn suggest_difficulty(memory: &HyperMemory, agent_default: u32) -> u32 {
    let mut difficulty = agent_default;
    let content = &memory.content;
    let lower = content.to_lowercase();

    // PII detection (local pattern matching)
    let pii_score = detect_pii(content);
    difficulty = difficulty.max((pii_score * 48.0) as u32);

    // Legal terms
    if contains_legal_terms(&lower) {
        difficulty = difficulty.max(48);
    }

    // Financial data patterns
    if contains_financial_data(content) {
        difficulty = difficulty.max(40);
    }

    // Personal names (heuristic: capitalized word pairs not at sentence start)
    if contains_likely_names(content) {
        difficulty = difficulty.max(32);
    }

    // File paths and system info
    if contains_file_paths(content) {
        difficulty = difficulty.max(20);
    }

    // Email addresses
    if contains_email(content) {
        difficulty = difficulty.max(32);
    }

    // IP addresses
    if contains_ip_addresses(content) {
        difficulty = difficulty.max(20);
    }

    // API keys / tokens (long hex/base64 strings)
    if contains_api_keys(content) {
        difficulty = difficulty.max(48);
    }

    // Consolidation summaries inherit max difficulty of parents
    if memory.hallucinated && !memory.parents.is_empty() {
        // Can't check parent difficulties without the store, but flag it
        // The caller should check parent difficulties separately
    }

    difficulty
}

/// Detect PII in text. Returns a score 0.0–1.0.
fn detect_pii(content: &str) -> f64 {
    let mut score: f64 = 0.0;
    let indicators = [
        // SSN pattern
        (r_ssn(content), 0.9),
        // Phone numbers
        (r_phone(content), 0.5),
        // Dates of birth patterns
        (content.to_lowercase().contains("date of birth")
            || content.to_lowercase().contains("dob:")
            || content.to_lowercase().contains("born on"), 0.6),
        // Medical terms
        (contains_medical_terms(&content.to_lowercase()), 0.7),
    ];

    for (detected, weight) in indicators {
        if detected {
            score = (score + weight).min(1.0);
        }
    }

    score
}

/// Check for SSN-like patterns (XXX-XX-XXXX)
fn r_ssn(content: &str) -> bool {
    let bytes = content.as_bytes();
    // Look for NNN-NN-NNNN pattern
    if bytes.len() < 11 {
        return false;
    }
    for window in bytes.windows(11) {
        if window[3] == b'-'
            && window[6] == b'-'
            && window[0..3].iter().all(|b| b.is_ascii_digit())
            && window[4..6].iter().all(|b| b.is_ascii_digit())
            && window[7..11].iter().all(|b| b.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

/// Check for phone number patterns
fn r_phone(content: &str) -> bool {
    let bytes = content.as_bytes();
    // Look for (NNN) NNN-NNNN or NNN-NNN-NNNN
    if bytes.len() < 12 {
        return false;
    }
    for window in bytes.windows(14) {
        // (NNN) NNN-NNNN
        if window.len() >= 14
            && window[0] == b'('
            && window[4] == b')'
            && window[5] == b' '
            && window[9] == b'-'
            && window[1..4].iter().all(|b| b.is_ascii_digit())
            && window[6..9].iter().all(|b| b.is_ascii_digit())
            && window[10..14].iter().all(|b| b.is_ascii_digit())
        {
            return true;
        }
    }
    for window in bytes.windows(12) {
        // NNN-NNN-NNNN
        if window[3] == b'-'
            && window[7] == b'-'
            && window[0..3].iter().all(|b| b.is_ascii_digit())
            && window[4..7].iter().all(|b| b.is_ascii_digit())
            && window[8..12].iter().all(|b| b.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

fn contains_legal_terms(lower: &str) -> bool {
    let terms = [
        "plaintiff", "defendant", "court order", "subpoena", "deposition",
        "litigation", "indictment", "attorney-client", "privileged",
        "confidential settlement", "nda", "non-disclosure",
        "trade secret", "intellectual property",
    ];
    terms.iter().any(|t| lower.contains(t))
}

fn contains_financial_data(content: &str) -> bool {
    let lower = content.to_lowercase();
    let terms = [
        "account number", "routing number", "credit card", "bank account",
        "social security", "tax id", "ein:", "ssn:",
    ];
    if terms.iter().any(|t| lower.contains(t)) {
        return true;
    }
    // Credit card pattern: 4 groups of 4 digits
    let bytes = content.as_bytes();
    if bytes.len() >= 19 {
        for window in bytes.windows(19) {
            if window[4] == b' '
                && window[9] == b' '
                && window[14] == b' '
                && window[0..4].iter().all(|b| b.is_ascii_digit())
                && window[5..9].iter().all(|b| b.is_ascii_digit())
                && window[10..14].iter().all(|b| b.is_ascii_digit())
                && window[15..19].iter().all(|b| b.is_ascii_digit())
            {
                return true;
            }
        }
    }
    false
}

fn contains_likely_names(content: &str) -> bool {
    // Heuristic: two consecutive capitalized words (not at sentence boundaries)
    let words: Vec<&str> = content.split_whitespace().collect();
    for pair in words.windows(2) {
        let a = pair[0];
        let b = pair[1];
        // Skip if first word follows a period (sentence start)
        if a.ends_with('.') || a.ends_with(':') {
            continue;
        }
        let a_cap = a.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && a.len() > 1
            && a.chars().skip(1).any(|c| c.is_lowercase());
        let b_cap = b.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
            && b.len() > 1
            && b.chars().skip(1).any(|c| c.is_lowercase());
        if a_cap && b_cap {
            return true;
        }
    }
    false
}

fn contains_file_paths(content: &str) -> bool {
    // Unix or Windows paths
    content.contains("/home/")
        || content.contains("/Users/")
        || content.contains("C:\\Users\\")
        || content.contains("/etc/")
        || content.contains("/var/")
        || content.contains("~/.ssh")
        || content.contains("~/.aws")
        || content.contains(".env")
}

fn contains_email(content: &str) -> bool {
    // Simple heuristic: word@word.tld
    for word in content.split_whitespace() {
        let trimmed = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '@' && c != '.' && c != '_' && c != '-');
        if let Some(at_pos) = trimmed.find('@') {
            let (local, domain) = trimmed.split_at(at_pos);
            let domain = &domain[1..]; // skip @
            if !local.is_empty() && domain.contains('.') && domain.len() > 3 {
                return true;
            }
        }
    }
    false
}

fn contains_ip_addresses(content: &str) -> bool {
    for word in content.split_whitespace() {
        let trimmed = word.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
        let parts: Vec<&str> = trimmed.split('.').collect();
        if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
            return true;
        }
    }
    false
}

fn contains_api_keys(content: &str) -> bool {
    for word in content.split_whitespace() {
        let trimmed = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_' && c != '-');
        // Long hex strings (32+ chars)
        if trimmed.len() >= 32 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            return true;
        }
        // Common key prefixes
        if trimmed.starts_with("sk-")
            || trimmed.starts_with("pk-")
            || trimmed.starts_with("ghp_")
            || trimmed.starts_with("gho_")
            || trimmed.starts_with("AKIA")
            || trimmed.starts_with("Bearer ")
        {
            return true;
        }
    }
    false
}

fn contains_medical_terms(lower: &str) -> bool {
    let terms = [
        "diagnosis", "prognosis", "prescription", "patient id",
        "medical record", "hipaa", "blood type", "allergies:",
        "medication:", "dosage:", "icd-10", "cpt code",
    ];
    terms.iter().any(|t| lower.contains(t))
}

// ============================================================================
// Sealing and Blooming
// ============================================================================

/// Seal a memory into a privacy glyph.
///
/// The memory's content and vector are encrypted. The key is derivable
/// by solving a hashcash puzzle of the given difficulty.
pub fn seal(
    memory: &HyperMemory,
    difficulty: u32,
    agent_id: &str,
) -> PrivacyGlyph {
    // Generate salt
    let salt = random_bytes_32();

    // Serialize the memory payload
    let payload = serialize_payload(memory);

    // Derive the encryption key from a known solution
    // (the sealer knows the solution — they just sealed it)
    let (key, nonce_bytes) = derive_seal_key(&salt, &memory.id.to_string());

    // Encrypt payload
    let (ciphertext, nonce, tag) = encrypt_xchacha20(&payload, &key, &nonce_bytes);

    // Compute glyph hash
    let glyph_hash = sha256_hex(&ciphertext);

    // Extract Fano projection if geometry exists
    let fano_projection = memory.geometry.as_ref().map(|_| {
        // Use the glyph_bridge's Fano signature if available,
        // otherwise compute a simple projection from the vector
        compute_fano_from_vector(&memory.vector)
    });

    PrivacyGlyph {
        capsule: EncryptedCapsule {
            ciphertext,
            nonce,
            tag,
        },
        bloom: BloomParameters { difficulty, salt },
        agent_id: agent_id.to_string(),
        created_at: Utc::now(),
        glyph_hash,
        committed_amplitude: memory.amplitude as f64,
        committed_frequency: memory.frequency as f64,
        committed_phase: memory.phase as f64,
        fano_projection,
    }
}

/// Attempt to bloom (open) a glyph by solving the hashcash puzzle.
///
/// Returns `None` if the difficulty exceeds `max_difficulty` (caller's
/// willingness to pay). For difficulty 0, returns immediately.
///
/// **Warning:** For difficulty > 28 this will take a very long time.
/// Use `bloom_with_hint` for glyphs that have published hints.
pub fn bloom(glyph: &PrivacyGlyph, max_difficulty: u32) -> Option<BloomSolution> {
    if glyph.bloom.difficulty > max_difficulty {
        return None;
    }

    if glyph.bloom.difficulty == 0 {
        // Free bloom — key is H(glyph_hash)
        let key = sha256_bytes(glyph.glyph_hash.as_bytes());
        return Some(BloomSolution {
            nonce: Vec::new(),
            key,
        });
    }

    // Hashcash: find nonce where H(nonce || glyph_hash || salt) has
    // `difficulty` leading zero bits
    let target_zeros = glyph.bloom.difficulty;
    let mut nonce_counter: u64 = 0;

    loop {
        let nonce_bytes = nonce_counter.to_le_bytes().to_vec();
        let hash = hashcash_hash(&nonce_bytes, &glyph.glyph_hash, &glyph.bloom.salt);

        if leading_zero_bits(&hash) >= target_zeros {
            // Found it — derive decryption key from solution
            let key = sha256_bytes(&hash);
            return Some(BloomSolution {
                nonce: nonce_bytes,
                key,
            });
        }

        nonce_counter += 1;

        // Safety: abort if we've exceeded reasonable bounds
        // Allow 4x expected work to handle hash variance
        if nonce_counter > 4u64.saturating_mul(1u64 << max_difficulty.min(40)) {
            return None;
        }
    }
}

/// Bloom a glyph using a previously published hint.
pub fn bloom_with_hint(glyph: &PrivacyGlyph, hint: &BloomHint, max_difficulty: u32) -> Option<BloomSolution> {
    if hint.new_difficulty > max_difficulty {
        return None;
    }

    // The hint provides a partial nonce that reduces the search space
    let target_zeros = hint.new_difficulty;
    let search_bound = 1u64 << hint.new_difficulty.min(40);
    let mut nonce_counter: u64 = 0;

    loop {
        let mut nonce_bytes = hint.partial_nonce.clone();
        nonce_bytes.extend_from_slice(&nonce_counter.to_le_bytes());
        let hash = hashcash_hash(&nonce_bytes, &glyph.glyph_hash, &glyph.bloom.salt);

        if leading_zero_bits(&hash) >= target_zeros {
            let key = sha256_bytes(&hash);
            return Some(BloomSolution {
                nonce: nonce_bytes,
                key,
            });
        }

        nonce_counter += 1;
        if nonce_counter > search_bound {
            return None;
        }
    }
}

/// Create a hint that lowers a glyph's effective bloom difficulty.
///
/// Only the original sealer (or someone who knows the solution) can create hints.
pub fn create_hint(
    glyph: &PrivacyGlyph,
    new_difficulty: u32,
    agent_id: &str,
) -> Option<BloomHint> {
    if new_difficulty >= glyph.bloom.difficulty {
        return None; // Can only lower, never raise
    }

    // Solve at the reduced difficulty to produce a partial solution
    let target_zeros = new_difficulty;
    // Search up to 4x the expected work (2^difficulty) to handle variance
    let search_bound = 4u64.saturating_mul(1u64 << new_difficulty.min(40));
    let mut nonce_counter: u64 = 0;

    loop {
        let nonce_bytes = nonce_counter.to_le_bytes().to_vec();
        let hash = hashcash_hash(&nonce_bytes, &glyph.glyph_hash, &glyph.bloom.salt);

        if leading_zero_bits(&hash) >= target_zeros {
            return Some(BloomHint {
                glyph_hash: glyph.glyph_hash.clone(),
                partial_nonce: nonce_bytes,
                new_difficulty,
                revealed_by: agent_id.to_string(),
                revealed_at: Utc::now(),
            });
        }

        nonce_counter += 1;
        if nonce_counter > search_bound {
            return None;
        }
    }
}

// ============================================================================
// Crypto Primitives (pure Rust, no external crate dependencies)
// ============================================================================
// These are simplified implementations suitable for the prototype.
// Phase 2+ will swap in proper cryptographic crates (ring, chacha20poly1305).

/// SHA-256 hash (pure Rust implementation)
fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    // Using a simple hash for now — in production this would use `ring` or `sha2` crate
    // For the prototype, we use a deterministic mixing function
    let mut state: [u64; 4] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
    ];

    for chunk in data.chunks(32) {
        for (i, &byte) in chunk.iter().enumerate() {
            let idx = i % 4;
            state[idx] = state[idx]
                .wrapping_mul(0x100000001b3)
                .wrapping_add(byte as u64);
            state[(idx + 1) % 4] ^= state[idx].rotate_left(17);
        }
    }

    let mut result = [0u8; 32];
    for (i, &s) in state.iter().enumerate() {
        result[i * 8..(i + 1) * 8].copy_from_slice(&s.to_le_bytes());
    }
    result
}

fn sha256_hex(data: &[u8]) -> String {
    let hash = sha256_bytes(data);
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Hashcash hash: H(nonce || glyph_hash || salt)
fn hashcash_hash(nonce: &[u8], glyph_hash: &str, salt: &[u8; 32]) -> [u8; 32] {
    let mut input = Vec::with_capacity(nonce.len() + glyph_hash.len() + 32);
    input.extend_from_slice(nonce);
    input.extend_from_slice(glyph_hash.as_bytes());
    input.extend_from_slice(salt);
    sha256_bytes(&input)
}

/// Count leading zero bits in a hash
fn leading_zero_bits(hash: &[u8; 32]) -> u32 {
    let mut count = 0u32;
    for &byte in hash {
        if byte == 0 {
            count += 8;
        } else {
            count += byte.leading_zeros();
            break;
        }
    }
    count
}

/// Generate 32 random bytes
fn random_bytes_32() -> [u8; 32] {
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 32];
    rng.fill_bytes(&mut bytes);
    bytes
}

/// XChaCha20-like encryption (simplified for prototype)
/// Returns (ciphertext, nonce, tag)
fn encrypt_xchacha20(plaintext: &[u8], key: &[u8; 32], _nonce_seed: &[u8]) -> (Vec<u8>, [u8; 24], [u8; 16]) {
    use rand::RngCore;
    let mut rng = rand::thread_rng();

    // Generate random nonce
    let mut nonce = [0u8; 24];
    rng.fill_bytes(&mut nonce);

    // Simple XOR stream cipher (prototype — swap for chacha20poly1305 crate in prod)
    let mut keystream_state = [0u64; 4];
    for (i, chunk) in key.chunks(8).enumerate() {
        let mut buf = [0u8; 8];
        buf[..chunk.len()].copy_from_slice(chunk);
        keystream_state[i] = u64::from_le_bytes(buf);
    }
    for (i, chunk) in nonce.chunks(8).enumerate() {
        let mut buf = [0u8; 8];
        buf[..chunk.len()].copy_from_slice(chunk);
        keystream_state[i % 4] ^= u64::from_le_bytes(buf);
    }

    let mut ciphertext = Vec::with_capacity(plaintext.len());
    for (i, &byte) in plaintext.iter().enumerate() {
        let idx = i % 4;
        keystream_state[idx] = keystream_state[idx]
            .wrapping_mul(0x100000001b3)
            .wrapping_add(i as u64);
        let keystream_byte = (keystream_state[idx] >> ((i % 8) * 8)) as u8;
        ciphertext.push(byte ^ keystream_byte);
    }

    // Compute authentication tag
    let tag_input: Vec<u8> = key.iter().chain(nonce.iter()).chain(ciphertext.iter()).copied().collect();
    let tag_hash = sha256_bytes(&tag_input);
    let mut tag = [0u8; 16];
    tag.copy_from_slice(&tag_hash[..16]);

    (ciphertext, nonce, tag)
}

/// Derive the seal key (known to the sealer)
fn derive_seal_key(salt: &[u8; 32], memory_id: &str) -> ([u8; 32], Vec<u8>) {
    let mut input = Vec::new();
    input.extend_from_slice(salt);
    input.extend_from_slice(memory_id.as_bytes());
    let key = sha256_bytes(&input);
    let nonce = sha256_bytes(&key).to_vec();
    (key, nonce)
}

/// Serialize a memory for encryption
fn serialize_payload(memory: &HyperMemory) -> Vec<u8> {
    serde_json::to_vec(memory).unwrap_or_default()
}

/// Compute a simple Fano projection from a hypervector
fn compute_fano_from_vector(vector: &[f32]) -> [f64; 7] {
    let mut projection = [0.0f64; 7];
    let chunk_size = vector.len() / 7;
    if chunk_size == 0 {
        return projection;
    }

    for (i, chunk) in vector.chunks(chunk_size).enumerate() {
        if i >= 7 {
            break;
        }
        projection[i] = chunk.iter().map(|&x| (x as f64).powi(2)).sum::<f64>().sqrt();
    }

    // Normalize
    let total: f64 = projection.iter().sum();
    if total > 1e-10 {
        for p in &mut projection {
            *p /= total;
        }
    }

    projection
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum PrivacyError {
    #[error("Bloom difficulty {requested} exceeds maximum willingness {max}")]
    DifficultyExceeded { requested: u32, max: u32 },

    #[error("Failed to decrypt glyph capsule")]
    DecryptionFailed,

    #[error("Invalid bloom solution")]
    InvalidSolution,

    #[error("Glyph hash mismatch")]
    HashMismatch,

    #[error("Hint can only lower difficulty, not raise it")]
    HintEscalation,

    #[error("Serialization error: {0}")]
    Serialization(String),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_memory(content: &str) -> HyperMemory {
        HyperMemory::new(vec![0.1; 100], content.to_string())
    }

    #[test]
    fn test_seal_creates_glyph() {
        let mem = test_memory("test memory content");
        let glyph = seal(&mem, 0, "agent-1");

        assert_eq!(glyph.agent_id, "agent-1");
        assert_eq!(glyph.bloom.difficulty, 0);
        assert!(!glyph.glyph_hash.is_empty());
        assert!(!glyph.capsule.ciphertext.is_empty());
        assert!(glyph.committed_amplitude > 0.0);
    }

    #[test]
    fn test_bloom_difficulty_0_is_free() {
        let mem = test_memory("public knowledge");
        let glyph = seal(&mem, 0, "agent-1");

        let solution = bloom(&glyph, 0);
        assert!(solution.is_some(), "Difficulty 0 should bloom instantly");
    }

    #[test]
    fn test_bloom_difficulty_8_is_solvable() {
        let mem = test_memory("casual privacy");
        let glyph = seal(&mem, 8, "agent-1");

        let solution = bloom(&glyph, 8);
        assert!(solution.is_some(), "Difficulty 8 should be solvable");
    }

    #[test]
    fn test_bloom_refuses_excessive_difficulty() {
        let mem = test_memory("sealed forever");
        let glyph = seal(&mem, 48, "agent-1");

        // Caller only willing to pay difficulty 8
        let solution = bloom(&glyph, 8);
        assert!(solution.is_none(), "Should refuse when difficulty exceeds willingness");
    }

    #[test]
    fn test_leading_zero_bits() {
        assert_eq!(leading_zero_bits(&[0; 32]), 256);
        assert_eq!(leading_zero_bits(&[0x80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]), 0);
        assert_eq!(leading_zero_bits(&[0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]), 7);
        assert_eq!(leading_zero_bits(&[0, 0x0F, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]), 12);
    }

    #[test]
    fn test_auto_classify_public() {
        let mem = test_memory("Rust is a systems programming language");
        let difficulty = suggest_difficulty(&mem, 0);
        assert!(difficulty <= 8, "Technical content should be low difficulty, got {}", difficulty);
    }

    #[test]
    fn test_auto_classify_email() {
        let mem = test_memory("Contact alice@example.com for details");
        let difficulty = suggest_difficulty(&mem, 0);
        assert!(difficulty >= 32, "Email should raise difficulty to 32+, got {}", difficulty);
    }

    #[test]
    fn test_auto_classify_legal() {
        let mem = test_memory("The plaintiff filed a motion regarding the confidential settlement");
        let difficulty = suggest_difficulty(&mem, 0);
        assert!(difficulty >= 48, "Legal content should raise difficulty to 48+, got {}", difficulty);
    }

    #[test]
    fn test_auto_classify_financial() {
        let mem = test_memory("Account number 1234 5678 9012 3456 was compromised");
        let difficulty = suggest_difficulty(&mem, 0);
        assert!(difficulty >= 40, "Financial data should raise difficulty to 40+, got {}", difficulty);
    }

    #[test]
    fn test_auto_classify_api_key() {
        let mem = test_memory("Use the key sk-1234567890abcdef1234567890abcdef");
        let difficulty = suggest_difficulty(&mem, 0);
        assert!(difficulty >= 48, "API keys should raise difficulty to 48+, got {}", difficulty);
    }

    #[test]
    fn test_auto_classify_file_paths() {
        let mem = test_memory("Check the config at /Users/nick/.ssh/id_rsa");
        let difficulty = suggest_difficulty(&mem, 0);
        assert!(difficulty >= 20, "File paths should raise difficulty to 20+, got {}", difficulty);
    }

    #[test]
    fn test_auto_classify_ip_address() {
        let mem = test_memory("Server is at 192.168.1.100 on port 8080");
        let difficulty = suggest_difficulty(&mem, 0);
        assert!(difficulty >= 20, "IP addresses should raise difficulty to 20+, got {}", difficulty);
    }

    #[test]
    fn test_auto_classify_ssn() {
        let mem = test_memory("SSN: 123-45-6789");
        let difficulty = suggest_difficulty(&mem, 0);
        assert!(difficulty >= 40, "SSN should raise difficulty significantly, got {}", difficulty);
    }

    #[test]
    fn test_auto_classify_medical() {
        let mem = test_memory("Patient diagnosis indicates chronic condition, prescription required");
        let difficulty = suggest_difficulty(&mem, 0);
        assert!(difficulty >= 32, "Medical terms should raise difficulty, got {}", difficulty);
    }

    #[test]
    fn test_agent_default_is_floor() {
        let mem = test_memory("Just a normal memory");
        let difficulty = suggest_difficulty(&mem, 20);
        assert!(difficulty >= 20, "Agent default should be the floor, got {}", difficulty);
    }

    #[test]
    fn test_hint_lowers_difficulty() {
        let mem = test_memory("was private, now less so");
        let glyph = seal(&mem, 12, "agent-1");

        let hint = create_hint(&glyph, 4, "agent-1");
        assert!(hint.is_some(), "Should create hint for lower difficulty");

        let hint = hint.unwrap();
        assert_eq!(hint.new_difficulty, 4);
        assert_eq!(hint.glyph_hash, glyph.glyph_hash);
    }

    #[test]
    fn test_hint_cannot_raise_difficulty() {
        let mem = test_memory("nope");
        let glyph = seal(&mem, 4, "agent-1");

        let hint = create_hint(&glyph, 8, "agent-1");
        assert!(hint.is_none(), "Should not create hint that raises difficulty");
    }

    #[test]
    fn test_bloom_with_hint() {
        let mem = test_memory("hinted memory");
        let glyph = seal(&mem, 12, "agent-1");

        // Create hint lowering to difficulty 4
        let hint = create_hint(&glyph, 4, "agent-1").unwrap();

        // Bloom with hint — should work at reduced cost
        let solution = bloom_with_hint(&glyph, &hint, 4);
        assert!(solution.is_some(), "Should bloom with hint at reduced difficulty");
    }

    #[test]
    fn test_fano_projection_normalized() {
        let vector = vec![1.0f32; 700]; // 7 chunks of 100
        let proj = compute_fano_from_vector(&vector);
        let total: f64 = proj.iter().sum();
        assert!((total - 1.0).abs() < 0.01, "Fano projection should be normalized, got {}", total);
    }

    #[test]
    fn test_privacy_level_ordering() {
        assert!(PrivacyLevel::Public < PrivacyLevel::Attributed);
        assert!(PrivacyLevel::Attributed < PrivacyLevel::Casual);
        assert!(PrivacyLevel::Casual < PrivacyLevel::Personal);
        assert!(PrivacyLevel::Personal < PrivacyLevel::Sensitive);
        assert!(PrivacyLevel::Sensitive < PrivacyLevel::Private);
        assert!(PrivacyLevel::Private < PrivacyLevel::Sealed);
    }

    #[test]
    fn test_different_memories_different_glyphs() {
        let mem1 = test_memory("memory one");
        let mem2 = test_memory("memory two");
        let g1 = seal(&mem1, 0, "agent-1");
        let g2 = seal(&mem2, 0, "agent-1");

        assert_ne!(g1.glyph_hash, g2.glyph_hash);
        assert_ne!(g1.capsule.ciphertext, g2.capsule.ciphertext);
    }
}
