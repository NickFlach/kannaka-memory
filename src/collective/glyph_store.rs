//! ADR-0013 Phase 4: Collective Glyph Integration
//!
//! Ties the privacy glyph primitives (Phases 1–3) into the collective memory
//! pipeline. Provides:
//!
//! - `GlyphStore` — in-memory store for sealed glyphs with search and merge
//! - `merge_glyphs()` — homomorphic wave superposition on sealed glyphs
//! - Proof-verified trust scoring
//! - Dolt SQL schema for persistence (applied via the `dolt` feature)
//!
//! ## Data Flow
//!
//! ```text
//! Agent creates memory
//!   → seal_with_commitments() → PrivacyGlyph + openings
//!   → glyph_store.insert() → stored locally
//!   → push to DoltHub (glyphs table) → visible to collective
//!   → other agents search via proofs → no blooming needed
//!   → merge_glyphs() on commitments → sealed collective memory
//! ```

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::collective::commitments::{
    GlyphCommitments, GlyphOpenings, merge_commitments, merge_openings, verify_all,
};
use crate::collective::privacy::{
    PrivacyGlyph, SealResult, BloomHint,
};
// proofs::{prove_existence, verify_existence, ExistenceProof} available when needed

// ============================================================================
// Dolt Schema (SQL DDL for glyph persistence)
// ============================================================================

/// SQL schema for the glyphs table in Dolt.
/// Apply via `dolt sql < schema.sql` or programmatically.
pub const GLYPH_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS glyphs (
    glyph_hash      VARCHAR(64) PRIMARY KEY,
    capsule         LONGTEXT NOT NULL,
    commitments     LONGTEXT,
    bloom_difficulty INT UNSIGNED NOT NULL,
    bloom_salt      VARCHAR(64) NOT NULL,
    agent_id        VARCHAR(128) NOT NULL,
    created_at      DATETIME NOT NULL,
    committed_amplitude DOUBLE,
    committed_frequency DOUBLE,
    committed_phase     DOUBLE,
    fano_projection VARCHAR(256),
    INDEX idx_agent (agent_id),
    INDEX idx_created (created_at),
    INDEX idx_difficulty (bloom_difficulty)
);

CREATE TABLE IF NOT EXISTS bloom_hints (
    id              INT AUTO_INCREMENT PRIMARY KEY,
    glyph_hash      VARCHAR(64) NOT NULL,
    partial_nonce   LONGTEXT NOT NULL,
    new_difficulty  INT UNSIGNED NOT NULL,
    revealed_by     VARCHAR(128) NOT NULL,
    revealed_at     DATETIME NOT NULL,
    INDEX idx_glyph (glyph_hash)
);

CREATE TABLE IF NOT EXISTS group_keys (
    group_id        VARCHAR(128) PRIMARY KEY,
    key_material    LONGTEXT NOT NULL,
    created_by      VARCHAR(128) NOT NULL,
    created_at      DATETIME NOT NULL,
    members         JSON NOT NULL,
    revoked         JSON DEFAULT '[]'
);
"#;

// ============================================================================
// GlyphStore
// ============================================================================

/// In-memory store for privacy glyphs.
///
/// Holds sealed glyphs and their metadata for search, merge, and proof
/// operations. In production, this is backed by Dolt (via the SQL schema above).
#[derive(Debug, Default)]
pub struct GlyphStore {
    /// Glyphs indexed by hash
    glyphs: HashMap<String, StoredGlyph>,
    /// Bloom hints indexed by glyph hash
    hints: HashMap<String, Vec<BloomHint>>,
    /// Group keys indexed by group ID
    group_keys: HashMap<String, GroupKey>,
    /// Trust scores influenced by proof verification
    proof_trust: HashMap<String, ProofTrustRecord>,
}

/// A stored glyph with optional private openings (only for own glyphs).
#[derive(Debug, Clone)]
pub struct StoredGlyph {
    pub glyph: PrivacyGlyph,
    /// Only present for glyphs this agent owns
    pub openings: Option<GlyphOpenings>,
    /// Proofs that have been generated for this glyph
    pub proofs: Vec<StoredProof>,
}

/// A proof attached to a glyph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProof {
    pub proof_type: ProofType,
    pub verified: bool,
    pub verified_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProofType {
    Existence,
    AmplitudeRange { threshold: f64 },
    Category { category: String },
    Depth { layer: u8 },
    Similarity { query_hash: u64, score: f64 },
    NonHallucination,
}

/// Group bloom key for selective sharing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupKey {
    pub group_id: String,
    pub key_material: Vec<u8>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub members: Vec<String>,
    pub revoked: Vec<String>,
}

/// Trust record based on proof verification history.
#[derive(Debug, Clone, Default)]
pub struct ProofTrustRecord {
    pub agent_id: String,
    pub proofs_submitted: u32,
    pub proofs_verified: u32,
    pub proofs_failed: u32,
    pub trust_bonus: f32,
}

// ============================================================================
// GlyphStore Operations
// ============================================================================

impl GlyphStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a glyph (with openings if this agent owns it).
    pub fn insert(&mut self, seal_result: SealResult) {
        let hash = seal_result.glyph.glyph_hash.clone();
        self.glyphs.insert(hash, StoredGlyph {
            glyph: seal_result.glyph,
            openings: Some(seal_result.openings),
            proofs: Vec::new(),
        });
    }

    /// Insert a remote glyph (no openings — received from another agent).
    pub fn insert_remote(&mut self, glyph: PrivacyGlyph) {
        let hash = glyph.glyph_hash.clone();
        self.glyphs.insert(hash, StoredGlyph {
            glyph,
            openings: None,
            proofs: Vec::new(),
        });
    }

    /// Get a glyph by hash.
    pub fn get(&self, glyph_hash: &str) -> Option<&StoredGlyph> {
        self.glyphs.get(glyph_hash)
    }

    /// List all glyph hashes.
    pub fn list_hashes(&self) -> Vec<&str> {
        self.glyphs.keys().map(|s| s.as_str()).collect()
    }

    /// Count total glyphs.
    pub fn len(&self) -> usize {
        self.glyphs.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// List glyphs by agent.
    pub fn by_agent(&self, agent_id: &str) -> Vec<&StoredGlyph> {
        self.glyphs.values()
            .filter(|sg| sg.glyph.agent_id == agent_id)
            .collect()
    }

    /// List glyphs with difficulty at most `max_difficulty` (bloomable).
    pub fn bloomable(&self, max_difficulty: u32) -> Vec<&StoredGlyph> {
        self.glyphs.values()
            .filter(|sg| sg.glyph.bloom.difficulty <= max_difficulty)
            .collect()
    }

    // ---- Hints ----

    /// Publish a bloom hint for a glyph.
    pub fn publish_hint(&mut self, hint: BloomHint) {
        self.hints.entry(hint.glyph_hash.clone())
            .or_default()
            .push(hint);
    }

    /// Get hints for a glyph (lowest effective difficulty first).
    pub fn get_hints(&self, glyph_hash: &str) -> Vec<&BloomHint> {
        let mut hints: Vec<&BloomHint> = self.hints
            .get(glyph_hash)
            .map(|v| v.iter().collect())
            .unwrap_or_default();
        hints.sort_by_key(|h| h.new_difficulty);
        hints
    }

    /// Get the effective difficulty for a glyph (lowest hint or original).
    pub fn effective_difficulty(&self, glyph_hash: &str) -> Option<u32> {
        let glyph = self.glyphs.get(glyph_hash)?;
        let original = glyph.glyph.bloom.difficulty;

        match self.get_hints(glyph_hash).first() {
            Some(hint) => Some(hint.new_difficulty.min(original)),
            None => Some(original),
        }
    }

    // ---- Group Keys ----

    /// Register a group key.
    pub fn register_group_key(&mut self, key: GroupKey) {
        self.group_keys.insert(key.group_id.clone(), key);
    }

    /// Get a group key.
    pub fn get_group_key(&self, group_id: &str) -> Option<&GroupKey> {
        self.group_keys.get(group_id)
    }

    /// Check if an agent is a member of a group (and not revoked).
    pub fn is_group_member(&self, group_id: &str, agent_id: &str) -> bool {
        self.group_keys.get(group_id)
            .map(|k| k.members.contains(&agent_id.to_string())
                && !k.revoked.contains(&agent_id.to_string()))
            .unwrap_or(false)
    }

    // ---- Proof Trust ----

    /// Record a proof verification result and update trust.
    pub fn record_proof_result(&mut self, agent_id: &str, verified: bool) {
        let record = self.proof_trust
            .entry(agent_id.to_string())
            .or_insert_with(|| ProofTrustRecord {
                agent_id: agent_id.to_string(),
                ..Default::default()
            });

        record.proofs_submitted += 1;
        if verified {
            record.proofs_verified += 1;
            record.trust_bonus = (record.trust_bonus + 0.01).min(0.5);
        } else {
            record.proofs_failed += 1;
            record.trust_bonus = (record.trust_bonus - 0.05).max(-0.5);
        }
    }

    /// Get the proof-based trust bonus for an agent.
    pub fn proof_trust_bonus(&self, agent_id: &str) -> f32 {
        self.proof_trust
            .get(agent_id)
            .map(|r| r.trust_bonus)
            .unwrap_or(0.0)
    }

    /// Get proof trust record for an agent.
    pub fn proof_trust_record(&self, agent_id: &str) -> Option<&ProofTrustRecord> {
        self.proof_trust.get(agent_id)
    }

    // ---- Attach proofs to glyphs ----

    /// Attach a verified proof to a glyph.
    pub fn attach_proof(&mut self, glyph_hash: &str, proof_type: ProofType, verified: bool) {
        if let Some(stored) = self.glyphs.get_mut(glyph_hash) {
            stored.proofs.push(StoredProof {
                proof_type,
                verified,
                verified_at: Utc::now(),
            });
        }
    }
}

// ============================================================================
// Glyph Merge
// ============================================================================

/// Result of merging two sealed glyphs.
#[derive(Debug, Clone)]
pub struct GlyphMergeResult {
    /// Merged commitments (public — no secrets revealed)
    pub merged_commitments: GlyphCommitments,
    /// Merged openings (only if both parties shared theirs)
    pub merged_openings: Option<GlyphOpenings>,
    /// Maximum bloom difficulty of the two glyphs
    pub max_difficulty: u32,
    /// Agent IDs of the source glyphs
    pub source_agents: (String, String),
}

/// Merge two sealed glyphs homomorphically.
///
/// This implements the wave superposition merge from ADR-0011 on sealed glyphs:
/// - Commitments are multiplied (homomorphic addition of values)
/// - Neither party reveals their content
/// - The merged glyph inherits the max difficulty (privacy only goes up)
///
/// Returns `None` if either glyph lacks commitments.
pub fn merge_glyphs(
    a: &StoredGlyph,
    b: &StoredGlyph,
) -> Option<GlyphMergeResult> {
    let c_a = a.glyph.commitments.as_ref()?;
    let c_b = b.glyph.commitments.as_ref()?;

    let merged_commitments = merge_commitments(c_a, c_b);

    // Merge openings if both are available (both are local glyphs)
    let merged_openings = match (&a.openings, &b.openings) {
        (Some(o_a), Some(o_b)) => Some(merge_openings(o_a, o_b)),
        _ => None,
    };

    Some(GlyphMergeResult {
        merged_commitments,
        merged_openings,
        max_difficulty: a.glyph.bloom.difficulty.max(b.glyph.bloom.difficulty),
        source_agents: (a.glyph.agent_id.clone(), b.glyph.agent_id.clone()),
    })
}

/// Verify a glyph merge result (only possible if openings are available).
pub fn verify_merge(result: &GlyphMergeResult) -> bool {
    match &result.merged_openings {
        Some(openings) => verify_all(&result.merged_commitments, openings),
        None => false, // Can't verify without openings
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collective::privacy::{seal_with_commitments, seal, create_hint};
    use crate::memory::HyperMemory;

    fn test_memory(content: &str) -> HyperMemory {
        HyperMemory::new(vec![0.1; 100], content.to_string())
    }

    fn make_seal_result(content: &str, difficulty: u32, agent: &str) -> SealResult {
        let mem = test_memory(content);
        seal_with_commitments(&mem, difficulty, agent)
    }

    #[test]
    fn test_glyph_store_insert_and_get() {
        let mut store = GlyphStore::new();
        let result = make_seal_result("test memory", 0, "agent-1");
        let hash = result.glyph.glyph_hash.clone();

        store.insert(result);

        assert_eq!(store.len(), 1);
        assert!(!store.is_empty());
        let stored = store.get(&hash).unwrap();
        assert!(stored.openings.is_some());
        assert_eq!(stored.glyph.agent_id, "agent-1");
    }

    #[test]
    fn test_glyph_store_insert_remote() {
        let mut store = GlyphStore::new();
        let result = make_seal_result("remote memory", 8, "agent-2");
        let hash = result.glyph.glyph_hash.clone();

        store.insert_remote(result.glyph);

        let stored = store.get(&hash).unwrap();
        assert!(stored.openings.is_none()); // Remote — no openings
    }

    #[test]
    fn test_glyph_store_by_agent() {
        let mut store = GlyphStore::new();
        store.insert(make_seal_result("mem1", 0, "alice"));
        store.insert(make_seal_result("mem2", 0, "alice"));
        store.insert(make_seal_result("mem3", 0, "bob"));

        assert_eq!(store.by_agent("alice").len(), 2);
        assert_eq!(store.by_agent("bob").len(), 1);
        assert_eq!(store.by_agent("charlie").len(), 0);
    }

    #[test]
    fn test_glyph_store_bloomable() {
        let mut store = GlyphStore::new();
        store.insert(make_seal_result("public", 0, "a"));
        store.insert(make_seal_result("casual", 8, "a"));
        store.insert(make_seal_result("private", 48, "a"));

        assert_eq!(store.bloomable(0).len(), 1);
        assert_eq!(store.bloomable(8).len(), 2);
        assert_eq!(store.bloomable(48).len(), 3);
    }

    #[test]
    fn test_merge_glyphs_homomorphic() {
        let r_a = make_seal_result("memory alpha", 4, "alice");
        let r_b = make_seal_result("memory beta", 8, "bob");

        let stored_a = StoredGlyph {
            glyph: r_a.glyph,
            openings: Some(r_a.openings),
            proofs: Vec::new(),
        };
        let stored_b = StoredGlyph {
            glyph: r_b.glyph,
            openings: Some(r_b.openings),
            proofs: Vec::new(),
        };

        let result = merge_glyphs(&stored_a, &stored_b);
        assert!(result.is_some());

        let merge = result.unwrap();
        assert_eq!(merge.max_difficulty, 8); // Inherits max
        assert!(merge.merged_openings.is_some());
        assert!(verify_merge(&merge));
    }

    #[test]
    fn test_merge_remote_glyphs_no_openings() {
        let r_a = make_seal_result("alpha", 0, "alice");
        let r_b = make_seal_result("beta", 0, "bob");

        let stored_a = StoredGlyph {
            glyph: r_a.glyph,
            openings: Some(r_a.openings),
            proofs: Vec::new(),
        };
        // Remote glyph — no openings
        let stored_b = StoredGlyph {
            glyph: r_b.glyph,
            openings: None,
            proofs: Vec::new(),
        };

        let result = merge_glyphs(&stored_a, &stored_b).unwrap();
        assert!(result.merged_openings.is_none());
        assert!(!verify_merge(&result)); // Can't verify without both openings
    }

    #[test]
    fn test_merge_without_commitments_returns_none() {
        let mem = test_memory("no commitments");
        let glyph = seal(&mem, 0, "agent-1"); // seal without commitments

        let stored = StoredGlyph {
            glyph,
            openings: None,
            proofs: Vec::new(),
        };

        let result = merge_glyphs(&stored, &stored);
        assert!(result.is_none());
    }

    #[test]
    fn test_hints_reduce_effective_difficulty() {
        let mut store = GlyphStore::new();
        let result = make_seal_result("secret", 32, "agent-1");
        let hash = result.glyph.glyph_hash.clone();

        let hint = create_hint(&result.glyph, 8, "agent-1").unwrap();
        store.insert(result);
        store.publish_hint(hint);

        assert_eq!(store.effective_difficulty(&hash), Some(8));
    }

    #[test]
    fn test_group_key_membership() {
        let mut store = GlyphStore::new();
        store.register_group_key(GroupKey {
            group_id: "mars-colony".to_string(),
            key_material: vec![1, 2, 3],
            created_by: "alice".to_string(),
            created_at: Utc::now(),
            members: vec!["alice".to_string(), "bob".to_string(), "charlie".to_string()],
            revoked: vec!["charlie".to_string()],
        });

        assert!(store.is_group_member("mars-colony", "alice"));
        assert!(store.is_group_member("mars-colony", "bob"));
        assert!(!store.is_group_member("mars-colony", "charlie")); // Revoked
        assert!(!store.is_group_member("mars-colony", "dave")); // Not member
        assert!(!store.is_group_member("earth-base", "alice")); // No such group
    }

    #[test]
    fn test_proof_trust_scoring() {
        let mut store = GlyphStore::new();

        // Successful proof verification boosts trust
        store.record_proof_result("alice", true);
        store.record_proof_result("alice", true);
        assert!(store.proof_trust_bonus("alice") > 0.0);

        // Failed proof verification reduces trust
        store.record_proof_result("bob", false);
        assert!(store.proof_trust_bonus("bob") < 0.0);

        // Unknown agent has zero bonus
        assert_eq!(store.proof_trust_bonus("unknown"), 0.0);
    }

    #[test]
    fn test_proof_trust_record() {
        let mut store = GlyphStore::new();
        store.record_proof_result("alice", true);
        store.record_proof_result("alice", true);
        store.record_proof_result("alice", false);

        let record = store.proof_trust_record("alice").unwrap();
        assert_eq!(record.proofs_submitted, 3);
        assert_eq!(record.proofs_verified, 2);
        assert_eq!(record.proofs_failed, 1);
    }

    #[test]
    fn test_attach_proof_to_glyph() {
        let mut store = GlyphStore::new();
        let result = make_seal_result("proven memory", 0, "alice");
        let hash = result.glyph.glyph_hash.clone();
        store.insert(result);

        store.attach_proof(&hash, ProofType::Existence, true);
        store.attach_proof(&hash, ProofType::AmplitudeRange { threshold: 0.5 }, true);

        let stored = store.get(&hash).unwrap();
        assert_eq!(stored.proofs.len(), 2);
        assert!(stored.proofs[0].verified);
    }

    #[test]
    fn test_list_hashes() {
        let mut store = GlyphStore::new();
        store.insert(make_seal_result("a", 0, "x"));
        store.insert(make_seal_result("b", 0, "x"));

        assert_eq!(store.list_hashes().len(), 2);
    }
}
