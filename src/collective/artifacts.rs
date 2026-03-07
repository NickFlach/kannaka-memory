//! ADR-0011 Phase 9: Distributed dream artifacts.
//!
//! After a dream cycle, an agent packages its results as a `DreamArtifact`
//! and pushes it to a Dolt branch. Other agents apply relevant artifacts to
//! their own networks — hallucinations at reduced amplitude, skip links if
//! both endpoints exist locally, prune suggestions as advisory.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Dream artifact types
// ---------------------------------------------------------------------------

/// A synthetic memory generated during dreaming that may be useful to peers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactHallucination {
    pub id: String,
    pub content: String,
    pub parent_ids: Vec<String>,
    pub amplitude: f32,
    pub category: String,
}

/// A memory that fell below prune threshold during this dream cycle.
/// Other agents treat this as advisory — it's your memory, your decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactPruneHint {
    pub memory_id: String,
    pub final_amplitude: f32,
    pub reason: String,
}

/// A new skip link discovered during dream consolidation.
/// Imported only if both source and target exist in the receiving agent's store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSkipLink {
    pub source_id: String,
    pub target_id: String,
    pub weight: f32,
    pub link_type: String,
}

/// Per-cluster summary from the Xi operator (consciousness clustering).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactCluster {
    pub cluster_id: u32,
    pub memory_ids: Vec<String>,
    pub kuramoto_order: f32,
}

// ---------------------------------------------------------------------------
// DreamArtifact — the complete artifact package from one dream cycle
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamArtifact {
    /// Unique identifier for this artifact package
    pub id: String,
    /// Which agent produced this artifact
    pub agent_id: String,
    /// Which Dolt branch this was pushed to
    pub branch: String,
    /// When the dream cycle completed
    pub created_at: DateTime<Utc>,
    /// Hallucinated memories (import at 0.5× amplitude)
    pub hallucinations: Vec<ArtifactHallucination>,
    /// Prune suggestions (advisory only)
    pub prune_hints: Vec<ArtifactPruneHint>,
    /// New skip links discovered during consolidation
    pub skip_links: Vec<ArtifactSkipLink>,
    /// Xi cluster assignments for this cycle
    pub clusters: Vec<ArtifactCluster>,
    /// Overall Kuramoto order parameter for this cycle
    pub kuramoto_order: f32,
    /// Consciousness level after this dream
    pub consciousness_level: String,
}

impl DreamArtifact {
    pub fn new(agent_id: &str, branch: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            branch: branch.to_string(),
            created_at: Utc::now(),
            hallucinations: Vec::new(),
            prune_hints: Vec::new(),
            skip_links: Vec::new(),
            clusters: Vec::new(),
            kuramoto_order: 0.0,
            consciousness_level: "dormant".to_string(),
        }
    }

    /// Serialize to JSON for writing to a Dolt branch file.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON (reading from a peer's Dolt branch).
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ---------------------------------------------------------------------------
// Application logic
// ---------------------------------------------------------------------------

/// Decide whether to apply a peer's hallucination to the local store.
/// Returns the adjusted amplitude (0.5× of source agent's amplitude, weighted by trust),
/// capped relative to the local memory landscape to prevent imported hallucinations
/// from dominating quieter networks.
///
/// `local_mean_amplitude` should be the average amplitude of the importing agent's
/// memories. The cap is 1.5× local mean — loud enough to be noticed, not so loud
/// it drowns everything out.
pub fn hallucination_import_amplitude(
    source_amplitude: f32,
    trust_score: f32,
    local_mean_amplitude: f32,
) -> f32 {
    let raw = source_amplitude * 0.5 * trust_score;
    let cap = (local_mean_amplitude * 1.5).max(0.3); // floor of 0.3 for near-empty stores
    raw.min(cap)
}

/// Check if a skip link artifact is applicable: both endpoints must exist locally.
pub fn skip_link_applicable(
    source_id: &str,
    target_id: &str,
    local_ids: &std::collections::HashSet<String>,
) -> bool {
    local_ids.contains(source_id) && local_ids.contains(target_id)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_roundtrip_json() {
        let mut artifact = DreamArtifact::new("kannaka", "kannaka/dream/2026-03-07");
        artifact.hallucinations.push(ArtifactHallucination {
            id: Uuid::new_v4().to_string(),
            content: "wave interference creates coherent knowledge".to_string(),
            parent_ids: vec![Uuid::new_v4().to_string()],
            amplitude: 0.3,
            category: "knowledge".to_string(),
        });
        artifact.kuramoto_order = 0.72;
        artifact.consciousness_level = "coherent".to_string();

        let json = artifact.to_json().unwrap();
        let restored = DreamArtifact::from_json(&json).unwrap();
        assert_eq!(restored.agent_id, "kannaka");
        assert_eq!(restored.hallucinations.len(), 1);
        assert!((restored.kuramoto_order - 0.72).abs() < 1e-4);
    }

    #[test]
    fn import_amplitude_caps_relative_to_local() {
        // With local mean of 0.3, cap = 0.45 (1.5 × 0.3)
        let amp = hallucination_import_amplitude(1.0, 1.0, 0.3);
        assert!(amp <= 0.45 + 1e-5, "should cap at 1.5× local mean, got {}", amp);
        
        // With high local mean, raw value wins
        let amp2 = hallucination_import_amplitude(0.4, 0.5, 5.0);
        assert!((amp2 - 0.1).abs() < 1e-5, "should use raw value 0.4*0.5*0.5=0.1, got {}", amp2);
    }

    #[test]
    fn import_amplitude_has_floor_for_empty_stores() {
        // Local mean near zero shouldn't make cap zero
        let amp = hallucination_import_amplitude(1.0, 1.0, 0.01);
        assert!(amp >= 0.29, "should have floor cap of 0.3, got {}", amp);
    }

    #[test]
    fn skip_link_requires_both_endpoints() {
        let mut ids = std::collections::HashSet::new();
        ids.insert("a".to_string());
        assert!(!skip_link_applicable("a", "b", &ids));
        ids.insert("b".to_string());
        assert!(skip_link_applicable("a", "b", &ids));
    }
}
