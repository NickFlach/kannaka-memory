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
/// Returns the adjusted amplitude (0.5× of source agent's amplitude, weighted by trust).
pub fn hallucination_import_amplitude(source_amplitude: f32, trust_score: f32) -> f32 {
    (source_amplitude * 0.5 * trust_score).min(0.8)
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
    fn import_amplitude_caps_at_0_8() {
        assert!(hallucination_import_amplitude(1.0, 1.0) <= 0.8);
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
