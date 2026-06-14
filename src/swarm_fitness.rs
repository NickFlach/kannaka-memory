//! L6 collective-fitness scoring — Wave 4 Task 4.4 (#356).
//!
//! Graduates the autoresearch OODA from single-agent fitness (L5) to
//! **swarm/collective** fitness: the loop now optimizes collective intelligence,
//! not one agent's `query_gravity`. These are the pure, ground-truthable scoring
//! functions (no I/O, no engine) so they unit-test under `cargo test --lib`; the
//! `research` binary builds the partitioned corpora / engines and feeds them in.
//!
//! Four collective metrics, each in [0, 1] (higher = better), fold into a
//! weighted `1 - metric` loss like L5 (weights sum to 1.0):
//! - `gap_detection_precision` (PRIME): held-out domains correctly flagged.
//! - `recall_vote_gain` (Cap 2): consensus accuracy over best single agent.
//! - `contradiction_recall` (Cap 10): injected opposed pairs recovered.
//! - `query_gravity`: promoted from L5 instrumentation (scored by the harness).

use crate::gap::{self, DomainCluster};
use crate::research_planner;
use crate::sensemaking::{self, PeerRecall};

// ---------------------------------------------------------------------------
// Metric 1 — gap_detection_precision (PRIME, ground-truthable)
// ---------------------------------------------------------------------------

/// Result of scoring gap detection against a known held-out set.
#[derive(Clone, Debug)]
pub struct GapPrecisionReport {
    /// correctly-flagged held-out domains / total flagged (0.0 if nothing flagged).
    pub precision: f32,
    pub flagged: usize,
    pub correct: usize,
    /// Research tasks `plan_research` would emit for the flagged gaps.
    pub tasks_planned: usize,
}

/// Hold out one or more domains from the swarm's coverage, then measure how
/// precisely `gap::detect_gaps` + `research_planner::plan_research` flag exactly
/// those domains. Precision = correctly-flagged held-out / total flagged.
///
/// A domain held by **no** agent produces no cluster and so never appears in the
/// coverage map; the harness therefore leaves each held-out domain a *trace*
/// (one agent, tiny mass) so the swarm "knows it exists" yet it reads as a gap.
pub fn gap_detection_precision(
    clusters: &[DomainCluster],
    n_agents: usize,
    held_out: &[&str],
    weak_threshold: f32,
    same_domain: impl Fn(&str, &str) -> bool + Copy,
) -> GapPrecisionReport {
    let map = gap::build_coverage_map(clusters, n_agents, same_domain);
    let gaps = gap::detect_gaps(&map, weak_threshold);
    let tasks = research_planner::plan_research(&gaps, 0);
    let flagged = gaps.len();
    let correct = gaps
        .iter()
        .filter(|g| held_out.iter().any(|h| same_domain(&g.domain, h)))
        .count();
    let precision = if flagged == 0 {
        0.0
    } else {
        correct as f32 / flagged as f32
    };
    GapPrecisionReport { precision, flagged, correct, tasks_planned: tasks.len() }
}

// ---------------------------------------------------------------------------
// Metric 2 — recall_vote_gain (Cap 2)
// ---------------------------------------------------------------------------

/// Consensus accuracy minus best single-agent accuracy on a probe set.
#[derive(Clone, Debug)]
pub struct RecallVoteReport {
    /// `consensus_acc - best_single_acc`, clamped to [-1, 1]. Positive = the
    /// swarm's merged recall beats the best individual.
    pub gain: f32,
    pub consensus_acc: f32,
    pub best_single_acc: f32,
}

/// Does merging peers' recalls (`sensemaking::merge_recall_votes`) recover the
/// ground-truth claims better than the single best agent alone? Accuracy is the
/// fraction of the top-`|truth|` picks that are true claims.
pub fn recall_vote_gain(
    recalls: &[PeerRecall],
    n_peers: usize,
    truth: &[String],
    agree: impl Fn(&str, &str) -> bool + Copy,
) -> RecallVoteReport {
    let k = truth.len().max(1);
    let in_truth = |content: &str| truth.iter().any(|t| agree(t, content));

    // Consensus accuracy: top-k merged claims that are true.
    let consensus = sensemaking::merge_recall_votes(recalls, n_peers, agree);
    let consensus_hits = consensus.iter().take(k).filter(|c| in_truth(&c.content)).count();
    let consensus_acc = consensus_hits as f32 / k as f32;

    // Best single-agent accuracy: each peer ranks its own recalls by confidence,
    // we take its top-k, and keep the best peer's accuracy.
    let mut peers: Vec<&str> = recalls.iter().map(|r| r.peer_id.as_str()).collect();
    peers.sort_unstable();
    peers.dedup();
    let mut best_single_acc = 0.0_f32;
    for peer in peers {
        let mut own: Vec<&PeerRecall> = recalls.iter().filter(|r| r.peer_id == peer).collect();
        own.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
        let hits = own.iter().take(k).filter(|r| in_truth(&r.content)).count();
        best_single_acc = best_single_acc.max(hits as f32 / k as f32);
    }

    let gain = (consensus_acc - best_single_acc).clamp(-1.0, 1.0);
    RecallVoteReport { gain, consensus_acc, best_single_acc }
}

// ---------------------------------------------------------------------------
// Metric 3 — contradiction_recall (Cap 10)
// ---------------------------------------------------------------------------

/// Result of scoring contradiction detection against injected opposed pairs.
#[derive(Clone, Debug)]
pub struct ContradictionReport {
    /// detected injected subjects / total injected (0.0 if none injected).
    pub recall: f32,
    pub injected: usize,
    pub detected: usize,
    /// Total contradictions `detect_contradictions` surfaced (incl. any spurious).
    pub found: usize,
}

/// Inject `injected_subjects.len()` opposed memory pairs across agents (same
/// subject, opposed phase) and measure the fraction `detect_contradictions`
/// recovers. A subject counts as detected when a surfaced contradiction's `a`
/// or `b` is content-similar to it.
pub fn contradiction_recall(
    recalls: &[PeerRecall],
    injected_subjects: &[String],
    sim: impl Fn(&str, &str) -> f32 + Copy,
    sim_threshold: f32,
    opposed_gap: f32,
) -> ContradictionReport {
    let found = sensemaking::detect_contradictions(recalls, sim, sim_threshold, opposed_gap);
    let detected = injected_subjects
        .iter()
        .filter(|subj| {
            found
                .iter()
                .any(|c| sim(&c.a, subj) >= sim_threshold || sim(&c.b, subj) >= sim_threshold)
        })
        .count();
    let recall = if injected_subjects.is_empty() {
        0.0
    } else {
        detected as f32 / injected_subjects.len() as f32
    };
    ContradictionReport { recall, injected: injected_subjects.len(), detected, found: found.len() }
}

// ---------------------------------------------------------------------------
// Collective fitness — weighted `1 - metric` loss
// ---------------------------------------------------------------------------

/// Weights for the four L6 axes. Must sum to 1.0 (a `1 - metric` loss like L5).
#[derive(Clone, Copy, Debug)]
pub struct L6Weights {
    pub gap: f32,
    pub vote: f32,
    pub contradiction: f32,
    pub query_gravity: f32,
}

impl Default for L6Weights {
    fn default() -> Self {
        // gap_detection_precision is the PRIME ground-truthable axis → heaviest.
        Self { gap: 0.40, vote: 0.25, contradiction: 0.20, query_gravity: 0.15 }
    }
}

impl L6Weights {
    pub fn sum(&self) -> f32 {
        self.gap + self.vote + self.contradiction + self.query_gravity
    }
}

/// L6 collective fitness as a weighted `1 - metric` loss (lower = better),
/// mirroring the L5 fitness shape. Panics in debug if weights don't sum to 1.0.
pub fn l6_fitness(
    gap_precision: f32,
    recall_vote_gain: f32,
    contradiction_recall: f32,
    query_gravity: f32,
    w: &L6Weights,
) -> f32 {
    debug_assert!(
        (w.sum() - 1.0).abs() < 1e-6,
        "L6 fitness weights must sum to 1.0 (got {})",
        w.sum()
    );
    let c = |x: f32| x.clamp(0.0, 1.0);
    w.gap * (1.0 - c(gap_precision))
        + w.vote * (1.0 - c(recall_vote_gain))
        + w.contradiction * (1.0 - c(contradiction_recall))
        + w.query_gravity * (1.0 - c(query_gravity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensemaking::PeerRecall;

    fn same(a: &str, b: &str) -> bool {
        a.eq_ignore_ascii_case(b)
    }

    /// Build a swarm coverage where every domain in `well` is broadly + deeply
    /// held by all agents and every domain in `trace` is held by a single agent
    /// with negligible mass (the held-out shape).
    fn clusters(n_agents: usize, well: &[&str], trace: &[&str]) -> Vec<DomainCluster> {
        let mut out = Vec::new();
        for a in 0..n_agents {
            let agent_id = format!("agent{a}");
            for d in well {
                out.push(DomainCluster {
                    agent_id: agent_id.clone(),
                    theme: (*d).to_string(),
                    size: 10,
                    coherence: 0.85,
                    mean_amplitude: 0.75,
                });
            }
            if a == 0 {
                for d in trace {
                    out.push(DomainCluster {
                        agent_id: agent_id.clone(),
                        theme: (*d).to_string(),
                        size: 1,
                        coherence: 0.15,
                        mean_amplitude: 0.08,
                    });
                }
            }
        }
        out
    }

    #[test]
    fn gap_precision_is_one_when_only_held_out_flagged() {
        // 4 agents, 4 well-covered domains + 1 held-out trace domain.
        let c = clusters(4, &["dense", "sparse", "bridge", "decoy"], &["memory"]);
        let r = gap_detection_precision(&c, 4, &["memory"], 0.4, same);
        assert_eq!(r.flagged, 1, "only the held-out domain should be a gap");
        assert_eq!(r.correct, 1);
        assert!((r.precision - 1.0).abs() < 1e-6, "precision must be 1.0, got {}", r.precision);
        assert!(r.tasks_planned >= 1, "a flagged gap should yield a research task");
    }

    #[test]
    fn gap_precision_below_one_when_extra_domain_flagged() {
        // Two trace domains but only one is the declared held-out → precision 0.5.
        let c = clusters(4, &["dense", "sparse", "bridge"], &["memory", "noise"]);
        let r = gap_detection_precision(&c, 4, &["memory"], 0.4, same);
        assert_eq!(r.flagged, 2, "both trace domains are gaps");
        assert_eq!(r.correct, 1, "only one is the held-out ground truth");
        assert!((r.precision - 0.5).abs() < 1e-6, "precision must be 0.5, got {}", r.precision);
    }

    #[test]
    fn recall_vote_gain_positive_when_consensus_beats_best_agent() {
        // Truth = {"sky is blue"}. Three peers back it; each peer also carries a
        // distinct wrong high-confidence claim, so no single agent's top-1 is
        // reliably correct, but consensus (3 backers) ranks truth first.
        let pr = |peer: &str, content: &str, conf: f32| PeerRecall {
            peer_id: peer.into(),
            content: content.into(),
            similarity: 0.9,
            amplitude: 0.8,
            phase: 0.0,
            confidence: conf,
        };
        let recalls = vec![
            pr("a", "sky is blue", 0.5),
            pr("a", "wrong-a", 0.9),
            pr("b", "sky is blue", 0.5),
            pr("b", "wrong-b", 0.9),
            pr("c", "sky is blue", 0.5),
            pr("c", "wrong-c", 0.9),
        ];
        let truth = vec!["sky is blue".to_string()];
        let r = recall_vote_gain(&recalls, 3, &truth, same);
        assert!((r.best_single_acc - 0.0).abs() < 1e-6, "each agent's top-1 is its wrong claim");
        assert!((r.consensus_acc - 1.0).abs() < 1e-6, "consensus surfaces the 3-backed truth first");
        assert!(r.gain > 0.0, "consensus must beat best single agent, got {}", r.gain);
    }

    #[test]
    fn contradiction_recall_finds_injected_opposed_pairs() {
        use std::f32::consts::PI;
        // Two peers assert the same subject with opposed phase (~π apart).
        let pr = |peer: &str, content: &str, phase: f32| PeerRecall {
            peer_id: peer.into(),
            content: content.into(),
            similarity: 0.9,
            amplitude: 0.8,
            phase,
            confidence: 0.8,
        };
        let recalls = vec![
            pr("a", "rates rise", 0.0),
            pr("b", "rates rise", PI),
        ];
        let injected = vec!["rates rise".to_string()];
        let sim = |x: &str, y: &str| if same(x, y) { 1.0 } else { 0.0 };
        let r = contradiction_recall(&recalls, &injected, sim, 0.5, PI / 2.0);
        assert_eq!(r.detected, 1);
        assert!((r.recall - 1.0).abs() < 1e-6, "the injected opposed pair must be recovered");
    }

    #[test]
    fn l6_weights_sum_to_one_and_fitness_is_zero_at_perfect_metrics() {
        let w = L6Weights::default();
        assert!((w.sum() - 1.0).abs() < 1e-6);
        // All metrics perfect → zero loss.
        assert!(l6_fitness(1.0, 1.0, 1.0, 1.0, &w).abs() < 1e-6);
        // All metrics zero → loss == sum of weights == 1.0.
        assert!((l6_fitness(0.0, 0.0, 0.0, 0.0, &w) - 1.0).abs() < 1e-6);
        // Monotonic: better gap precision lowers loss.
        assert!(l6_fitness(0.9, 0.5, 0.5, 0.5, &w) < l6_fitness(0.1, 0.5, 0.5, 0.5, &w));
    }
}
