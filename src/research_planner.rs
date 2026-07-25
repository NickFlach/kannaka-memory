//! Autonomous research planner — ADR-0035 Capability (Phase 4) / Wave 4 Task 4.1.
//!
//! Pure: turn the Wave 3.3 coverage-gap map into a ranked set of directed
//! research tasks — the collective generalization of the single-agent curiosity
//! loop (`research-suggest` → `research --ingest` → curiosity PR). No I/O; the
//! CLI/NATS layer enqueues the tasks onto the work queue where a worker ingests
//! literature into exactly the swarm's blind spots.

use crate::gap::{CoverageGap, GapKind};

/// A directed research task derived from a coverage gap.
#[derive(Clone, Debug)]
pub struct ResearchTask {
    pub domain: String,
    /// The query handed to `kannaka research <theme_query> --ingest`.
    pub theme_query: String,
    pub kind: GapKind,
    /// Higher = more urgent.
    pub priority: f32,
    pub peers_holding: usize,
    pub rationale: String,
}

/// Severity weight per gap kind. A weakly-represented (thin) domain is a bigger
/// research opportunity than one that's broadly held but merely incoherent. (A
/// `BlindSpot` kind would outrank both, but `gap::detect_gaps` doesn't emit it
/// yet — see gap.rs.)
fn kind_weight(k: GapKind) -> f32 {
    match k {
        GapKind::WeaklyRepresented => 1.0,
        GapKind::LowConfidence => 0.7,
    }
}

/// Turn gaps into ranked research tasks. `priority = (1 - coverage) × kind_weight`,
/// sorted descending. `dedupe_min_peers` drops domains already held by at least
/// that many peers (no point researching what the collective already covers);
/// pass 0 to keep all.
pub fn plan_research(gaps: &[CoverageGap], dedupe_min_peers: usize) -> Vec<ResearchTask> {
    let mut tasks: Vec<ResearchTask> = gaps
        .iter()
        .filter(|g| dedupe_min_peers == 0 || g.peers_holding < dedupe_min_peers)
        .map(|g| {
            let priority = (1.0 - g.coverage).clamp(0.0, 1.0) * kind_weight(g.kind);
            let rationale = match g.kind {
                GapKind::WeaklyRepresented => format!(
                    "weakly represented (coverage {g.coverage:.2}, held by {g.peers_holding} peer(s)) — ingest to deepen"
                ),
                GapKind::LowConfidence => format!(
                    "low confidence (coverage {g.coverage:.2}) — ingest to corroborate / resolve"
                ),
            };
            ResearchTask {
                domain: g.domain.clone(),
                theme_query: g.domain.clone(),
                kind: g.kind,
                priority,
                peers_holding: g.peers_holding,
                rationale,
            }
        })
        .collect();
    tasks.sort_by(|a, b| b.priority.total_cmp(&a.priority));
    tasks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap(domain: &str, coverage: f32, kind: GapKind, peers: usize) -> CoverageGap {
        CoverageGap {
            domain: domain.into(),
            coverage,
            kind,
            peers_holding: peers,
        }
    }

    #[test]
    fn weakly_represented_outranks_low_confidence_at_equal_coverage() {
        let gaps = vec![
            gap("a", 0.2, GapKind::LowConfidence, 3),
            gap("b", 0.2, GapKind::WeaklyRepresented, 1),
        ];
        let tasks = plan_research(&gaps, 0);
        assert_eq!(tasks[0].domain, "b"); // weakly-represented wins
        assert!(tasks[0].priority > tasks[1].priority);
    }

    #[test]
    fn lower_coverage_is_higher_priority() {
        let gaps = vec![
            gap("a", 0.3, GapKind::WeaklyRepresented, 1),
            gap("b", 0.05, GapKind::WeaklyRepresented, 1),
        ];
        let tasks = plan_research(&gaps, 0);
        assert_eq!(tasks[0].domain, "b");
    }

    #[test]
    fn dedupe_drops_well_covered_domains() {
        let gaps = vec![
            gap("covered", 0.1, GapKind::WeaklyRepresented, 4),
            gap("rare", 0.1, GapKind::WeaklyRepresented, 1),
        ];
        let tasks = plan_research(&gaps, 3); // drop domains held by >= 3 peers
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].domain, "rare");
    }

    #[test]
    fn empty_gaps_yield_no_tasks() {
        assert!(plan_research(&[], 0).is_empty());
    }
}
