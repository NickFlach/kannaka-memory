//! ADR-0011 Phase 5: Wave interference merge algorithm.
//!
//! When two agents have memories about the same topic, wave physics determines
//! the merge outcome. The amplitude superposition formula is literal:
//!
//!   A = √(A₁² + A₂² + 2·A₁·A₂·cos(Δφ))
//!
//! Classification:
//! - Constructive (Δφ < π/4):         memories agree — merge amplitudes
//! - Destructive  (Δφ > 3π/4):        memories disagree — quarantine both
//! - Partial      (π/4 ≤ Δφ ≤ 3π/4): ambiguous — keep both, link them

use std::f32::consts::PI;

use chrono::Utc;
use uuid::Uuid;

use crate::memory::{HyperMemory, MergeRecord};
use crate::wave::cosine_similarity;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Cosine similarity below this value → memories are about different topics; no merge.
pub const INDEPENDENCE_THRESHOLD: f32 = 0.6;

/// Phase difference (rad) below this → constructive interference.
pub const CONSTRUCTIVE_THRESHOLD: f32 = PI / 4.0;

/// Phase difference (rad) above this → destructive interference.
pub const DESTRUCTIVE_THRESHOLD: f32 = 3.0 * PI / 4.0;

/// Amplitude reduction factor when memories are in destructive interference.
pub const DESTRUCTIVE_PENALTY: f32 = 0.4;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum MergeKind {
    /// Similarity below INDEPENDENCE_THRESHOLD — topics differ; no action.
    Independent,
    /// Phase-aligned agreement — amplitudes superpose; produce single merged memory.
    Constructive,
    /// Phase-opposed disagreement — both memories dampened; quarantine for review.
    Destructive,
    /// Ambiguous phase — keep both independently; link with partial_agreement skip link.
    Partial,
}

#[derive(Debug, Clone)]
pub struct MergeResult {
    pub kind: MergeKind,
    pub similarity: f32,
    pub phase_diff: f32,
    /// Merged amplitude (constructive) or post-penalty amplitude (destructive).
    pub resulting_amplitude: f32,
}

// ---------------------------------------------------------------------------
// Core algorithm
// ---------------------------------------------------------------------------

/// Classify two memories and compute the merge result.
///
/// Does NOT mutate the memories — callers apply the result.
pub fn classify_merge(local: &HyperMemory, remote: &HyperMemory) -> MergeResult {
    let similarity = cosine_similarity(&local.vector, &remote.vector);

    if similarity < INDEPENDENCE_THRESHOLD {
        return MergeResult {
            kind: MergeKind::Independent,
            similarity,
            phase_diff: 0.0,
            resulting_amplitude: local.amplitude,
        };
    }

    let raw_diff = (local.phase - remote.phase).abs() % (2.0 * PI);
    let phase_diff = if raw_diff > PI { 2.0 * PI - raw_diff } else { raw_diff };

    let kind = if phase_diff < CONSTRUCTIVE_THRESHOLD {
        MergeKind::Constructive
    } else if phase_diff > DESTRUCTIVE_THRESHOLD {
        MergeKind::Destructive
    } else {
        MergeKind::Partial
    };

    let resulting_amplitude = match kind {
        MergeKind::Constructive => {
            let a1 = local.amplitude;
            let a2 = remote.amplitude;
            (a1 * a1 + a2 * a2 + 2.0 * a1 * a2 * phase_diff.cos()).sqrt()
        }
        MergeKind::Destructive => {
            local.amplitude * (1.0 - DESTRUCTIVE_PENALTY * similarity)
        }
        MergeKind::Partial => local.amplitude,
        MergeKind::Independent => local.amplitude,
    };

    MergeResult { kind, similarity, phase_diff, resulting_amplitude }
}

/// Apply a constructive merge: blend local memory in-place with remote.
///
/// - Amplitude: wave superposition formula
/// - Vector: amplitude-weighted average
/// - Phase: amplitude-weighted circular mean
/// - Appends a MergeRecord to merge_history
/// - Bumps sync_version
pub fn apply_constructive(local: &mut HyperMemory, remote: &HyperMemory, result: &MergeResult) {
    let a1 = local.amplitude;
    let a2 = remote.amplitude;
    let total = a1 + a2;

    // Amplitude-weighted vector average
    if !local.vector.is_empty() && local.vector.len() == remote.vector.len() {
        for (lv, rv) in local.vector.iter_mut().zip(remote.vector.iter()) {
            *lv = (*lv * a1 + *rv * a2) / total;
        }
    }

    // Amplitude-weighted circular mean phase
    let sin_mean = (a1 * local.phase.sin() + a2 * remote.phase.sin()) / total;
    let cos_mean = (a1 * local.phase.cos() + a2 * remote.phase.cos()) / total;
    local.phase = sin_mean.atan2(cos_mean);

    local.amplitude = result.resulting_amplitude;
    local.sync_version += 1;

    local.merge_history.push(MergeRecord {
        merged_at: Utc::now(),
        source_agent: remote.origin_agent.clone(),
        source_memory_id: remote.id.to_string(),
        merge_type: "constructive".to_string(),
        phase_diff: result.phase_diff,
        amplitude_before: a1,
        amplitude_after: result.resulting_amplitude,
    });
}

/// Apply a destructive merge: dampen local memory and mark as disputed.
pub fn apply_destructive(local: &mut HyperMemory, remote: &HyperMemory, result: &MergeResult) {
    let amplitude_before = local.amplitude;
    local.amplitude = result.resulting_amplitude;
    local.disputed = true;
    local.sync_version += 1;

    local.merge_history.push(MergeRecord {
        merged_at: Utc::now(),
        source_agent: remote.origin_agent.clone(),
        source_memory_id: remote.id.to_string(),
        merge_type: "destructive".to_string(),
        phase_diff: result.phase_diff,
        amplitude_before,
        amplitude_after: result.resulting_amplitude,
    });
}

/// Apply a partial merge: add a merge record without modifying amplitude.
pub fn apply_partial(local: &mut HyperMemory, remote: &HyperMemory, result: &MergeResult) {
    local.merge_history.push(MergeRecord {
        merged_at: Utc::now(),
        source_agent: remote.origin_agent.clone(),
        source_memory_id: remote.id.to_string(),
        merge_type: "partial".to_string(),
        phase_diff: result.phase_diff,
        amplitude_before: local.amplitude,
        amplitude_after: local.amplitude,
    });
}

// ---------------------------------------------------------------------------
// Trust-weighted amplitude
// ---------------------------------------------------------------------------

/// Apply trust weighting to an effective amplitude for merge comparisons.
///
/// effective_amplitude = raw_amplitude × trust_score × recency_factor
pub fn trust_weighted_amplitude(amplitude: f32, trust_score: f32, age_days: f64, half_life_days: f64) -> f32 {
    let recency = (-(age_days / half_life_days) * std::f64::consts::LN_2) as f32;
    amplitude * trust_score * recency.exp()
}

// ---------------------------------------------------------------------------
// Quarantine entry
// ---------------------------------------------------------------------------

/// Describes a disputed memory pair that should be added to the quarantine table.
#[derive(Debug, Clone)]
pub struct QuarantineEntry {
    pub id: Uuid,
    pub memory_id_a: Uuid,
    pub memory_id_b: Uuid,
    pub agent_a: String,
    pub agent_b: String,
    pub similarity: f32,
    pub phase_diff: f32,
}

impl QuarantineEntry {
    pub fn new(local: &HyperMemory, remote: &HyperMemory, result: &MergeResult) -> Self {
        Self {
            id: Uuid::new_v4(),
            memory_id_a: local.id,
            memory_id_b: remote.id,
            agent_a: local.origin_agent.clone(),
            agent_b: remote.origin_agent.clone(),
            similarity: result.similarity,
            phase_diff: result.phase_diff,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn mem(amplitude: f32, phase: f32, content: &str) -> HyperMemory {
        let mut m = HyperMemory::new(vec![1.0; 64], content.to_string());
        m.amplitude = amplitude;
        m.phase = phase;
        m
    }

    #[test]
    fn constructive_merge_boosts_amplitude() {
        let local = mem(0.8, 0.0, "fact A");
        let remote = mem(0.7, 0.1, "fact A rephrased");
        let result = classify_merge(&local, &remote);
        assert_eq!(result.kind, MergeKind::Constructive);
        assert!(result.resulting_amplitude > local.amplitude);
    }

    #[test]
    fn destructive_merge_dampens_amplitude() {
        let local = mem(0.8, 0.0, "fact A");
        let remote = mem(0.7, PI, "fact A contradicted");
        let result = classify_merge(&local, &remote);
        assert_eq!(result.kind, MergeKind::Destructive);
        assert!(result.resulting_amplitude < local.amplitude);
    }

    #[test]
    fn low_similarity_is_independent() {
        let mut local = mem(0.8, 0.0, "cats");
        let mut remote = mem(0.7, 0.0, "quantum gravity");
        // make them dissimilar by randomizing vectors
        for (i, v) in local.vector.iter_mut().enumerate() { *v = if i % 2 == 0 { 1.0 } else { -1.0 }; }
        for (i, v) in remote.vector.iter_mut().enumerate() { *v = if i % 2 == 0 { -1.0 } else { 1.0 }; }
        let result = classify_merge(&local, &remote);
        assert_eq!(result.kind, MergeKind::Independent);
    }

    #[test]
    fn constructive_apply_records_history() {
        let mut local = mem(0.8, 0.0, "fact A");
        local.origin_agent = "kannaka".to_string();
        let mut remote = mem(0.7, 0.1, "fact A rephrased");
        remote.origin_agent = "arc".to_string();
        let result = classify_merge(&local, &remote);
        if result.kind == MergeKind::Constructive {
            apply_constructive(&mut local, &remote, &result);
            assert_eq!(local.merge_history.len(), 1);
            assert_eq!(local.merge_history[0].source_agent, "arc");
            assert_eq!(local.sync_version, 1);
        }
    }
}
