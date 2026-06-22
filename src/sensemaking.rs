//! Swarm sensemaking primitives — ADR-0035 Phase 1 / Wave 1.
//!
//! Pure, transport-agnostic logic for the Distributed Sensemaking Engine:
//! peer expertise scoring (Cap 3), collective recall merging + confidence
//! (Cap 2), contradiction detection (Cap 10), and brief composition (Cap 5).
//!
//! These functions hold NO I/O. The NATS/CLI layer fans a query out to peers,
//! collects [`PeerRecall`] responses, and feeds them here. Keeping the cognition
//! pure makes it unit-testable and lets the same logic run single-agent (local
//! recall only) or swarm-wide. The L6 autoresearch scores these outputs.

/// One recalled item from a peer in response to a query.
#[derive(Clone, Debug)]
pub struct PeerRecall {
    pub peer_id: String,
    pub content: String,
    /// Semantic similarity of this item to the query, in [0, 1].
    pub similarity: f32,
    /// Recall amplitude (strength) on the responding peer.
    pub amplitude: f32,
    /// Wave phase in radians — used as a stance proxy for contradiction detection.
    pub phase: f32,
    /// The peer's local confidence in this item, in [0, 1].
    pub confidence: f32,
}

/// Signals for scoring a peer's expertise on a topic (Cap 3, Task 1.1).
#[derive(Clone, Debug)]
pub struct ExpertiseSignals {
    pub query_similarity: f32,
    pub coverage: f32,
    pub historical_success: f32,
    pub phi: f32,
    pub local_confidence: f32,
}

/// Weighted expertise score in [0, 1]. Topical fit (similarity, coverage)
/// dominates; the global consciousness metric (phi) contributes least so a
/// merely "highly conscious" but off-topic peer does not outrank an on-topic one.
pub fn score_peer_expertise(s: &ExpertiseSignals) -> f32 {
    // sim, coverage, history, phi, confidence — weights sum to 1.0.
    const W: [f32; 5] = [0.35, 0.25, 0.20, 0.05, 0.15];
    let v = [
        s.query_similarity,
        s.coverage,
        s.historical_success,
        s.phi,
        s.local_confidence,
    ];
    W.iter()
        .zip(v)
        .map(|(w, x)| w * x.clamp(0.0, 1.0))
        .sum::<f32>()
        .clamp(0.0, 1.0)
}

/// A merged claim across peers (Cap 2, Task 1.2).
#[derive(Clone, Debug)]
pub struct ConsensusItem {
    pub content: String,
    /// Number of peers that returned ~this claim.
    pub support: usize,
    /// Consensus confidence in [0, 1].
    pub confidence: f32,
    pub mean_similarity: f32,
}

/// Merge peer recalls into consensus claims. Two items are the "same claim" when
/// `agree` returns true (e.g. cosine over content embeddings, thresholded).
///
/// Consensus confidence blends agreement breadth (how many of the `n_peers`
/// support it) with the mean per-item similarity — so a claim returned by many
/// peers with high similarity scores near 1.0, while a lone outlier (support 1 of
/// many) is kept but flagged with low confidence rather than silently merged.
/// Results are sorted by confidence (desc).
pub fn merge_recall_votes(
    recalls: &[PeerRecall],
    n_peers: usize,
    agree: impl Fn(&str, &str) -> bool,
) -> Vec<ConsensusItem> {
    let denom = n_peers.max(1) as f32;
    let mut groups: Vec<(String, Vec<&PeerRecall>)> = Vec::new();
    for r in recalls {
        match groups.iter_mut().find(|(rep, _)| agree(rep, &r.content)) {
            Some((_, members)) => members.push(r),
            None => groups.push((r.content.clone(), vec![r])),
        }
    }

    let mut items: Vec<ConsensusItem> = groups
        .into_iter()
        .map(|(content, members)| {
            // Distinct peers backing this claim (a peer is counted once).
            let mut peers: Vec<&str> = members.iter().map(|m| m.peer_id.as_str()).collect();
            peers.sort_unstable();
            peers.dedup();
            let support = peers.len();
            let mean_similarity =
                members.iter().map(|m| m.similarity).sum::<f32>() / members.len() as f32;
            let mean_conf =
                members.iter().map(|m| m.confidence).sum::<f32>() / members.len() as f32;
            // Agreement breadth in [0,1] × evidence quality (similarity × peer confidence).
            let breadth = (support as f32 / denom).clamp(0.0, 1.0);
            let confidence = (breadth * mean_similarity * mean_conf).clamp(0.0, 1.0);
            ConsensusItem {
                content,
                support,
                confidence,
                mean_similarity,
            }
        })
        .collect();

    items.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    items
}

/// A detected disagreement: two claims about the same subject with opposed stance.
#[derive(Clone, Debug)]
pub struct Contradiction {
    pub a: String,
    pub b: String,
    pub similarity: f32,
    /// Phase gap in radians (0..π); near π = opposed stance.
    pub phase_gap: f32,
}

/// Find pairs that are semantically close (`sim(a, b) >= sim_threshold`) but
/// phase-opposed (phase gap > `opposed_gap`, near π) — the wave-native signal for
/// "same subject, opposite stance" (Cap 10). Disagreement is surfaced, not hidden.
pub fn detect_contradictions(
    items: &[PeerRecall],
    sim: impl Fn(&str, &str) -> f32,
    sim_threshold: f32,
    opposed_gap: f32,
) -> Vec<Contradiction> {
    let two_pi = 2.0 * std::f32::consts::PI;
    let mut out = Vec::new();
    for i in 0..items.len() {
        for j in (i + 1)..items.len() {
            let s = sim(&items[i].content, &items[j].content);
            if s < sim_threshold {
                continue;
            }
            let raw = (items[i].phase - items[j].phase).abs();
            let gap = raw.min(two_pi - raw); // 0..π
            if gap > opposed_gap {
                out.push(Contradiction {
                    a: items[i].content.clone(),
                    b: items[j].content.clone(),
                    similarity: s,
                    phase_gap: gap,
                });
            }
        }
    }
    out.sort_by(|x, y| y.phase_gap.total_cmp(&x.phase_gap));
    out
}

/// A composed answer to `kannaka swarm brief "<topic>"` (Cap 5, Task 1.4).
#[derive(Clone, Debug)]
pub struct SwarmBrief {
    pub topic: String,
    pub known: Vec<ConsensusItem>,
    pub contradictions: Vec<Contradiction>,
    /// (peer_id, expertise) sorted by expertise desc.
    pub relevant_peers: Vec<(String, f32)>,
    /// Overall brief confidence in [0, 1] (mean of the top consensus items,
    /// penalized by unresolved contradictions).
    pub confidence: f32,
}

/// Assemble a brief from already-merged consensus, detected contradictions, and
/// scored peers. Confidence is the mean confidence of the top-k known items,
/// reduced when contradictions remain unresolved (each shaves a small penalty).
pub fn compose_brief(
    topic: &str,
    known: Vec<ConsensusItem>,
    contradictions: Vec<Contradiction>,
    mut relevant_peers: Vec<(String, f32)>,
) -> SwarmBrief {
    relevant_peers.sort_by(|a, b| b.1.total_cmp(&a.1));
    let k = 3.min(known.len());
    let base = if k == 0 {
        0.0
    } else {
        known[..k].iter().map(|c| c.confidence).sum::<f32>() / k as f32
    };
    let penalty = (contradictions.len() as f32 * 0.1).min(0.5);
    let confidence = (base - penalty).clamp(0.0, 1.0);
    SwarmBrief {
        topic: topic.to_string(),
        known,
        contradictions,
        relevant_peers,
        confidence,
    }
}

/// A candidate hypothesis produced by a dream cycle (a hallucinated/synthesized
/// memory), to be cross-checked against the swarm (Cap 6 / Wave 3 Task 3.1).
#[derive(Clone, Debug)]
pub struct Hypothesis {
    pub content: String,
    pub phase: f32,
}

/// A peer's cluster summary, broadcast via exemplar exchange.
#[derive(Clone, Debug)]
pub struct PeerCluster {
    pub agent_id: String,
    pub summary: String,
    pub phase: f32,
}

/// Whether a dreamed hypothesis is corroborated across the swarm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HypothesisStatus {
    /// Resonates with ≥ k distinct peers → keep full amplitude.
    Reinforced,
    /// Only local support → down-rank (Wave 2.2), don't delete.
    Speculative,
}

/// The verdict on one hypothesis.
#[derive(Clone, Debug)]
pub struct HypothesisVerdict {
    pub content: String,
    pub status: HypothesisStatus,
    pub supporting_peers: usize,
}

/// Cross-agent dreaming (Cap 6): a hypothesis is **Reinforced** when it resonates
/// — content-similar (`sim >= sim_threshold`) AND phase-coherent (|Δφ| <=
/// `phase_tol`) — with at least `k_peers` *distinct* peers' cluster summaries;
/// otherwise **Speculative**. Empty peer set → everything Speculative (degrades to
/// today's local-only dream). Pure: the consolidation/NATS layer feeds it absorbed
/// peer exemplars and applies the down-rank to Speculative results.
pub fn reinforce_hypotheses(
    hypotheses: &[Hypothesis],
    peer_clusters: &[PeerCluster],
    sim: impl Fn(&str, &str) -> f32,
    sim_threshold: f32,
    phase_tol: f32,
    k_peers: usize,
) -> Vec<HypothesisVerdict> {
    let two_pi = 2.0 * std::f32::consts::PI;
    hypotheses
        .iter()
        .map(|h| {
            let mut peers: Vec<&str> = peer_clusters
                .iter()
                .filter(|c| {
                    let raw = (h.phase - c.phase).abs();
                    let dphi = raw.min(two_pi - raw);
                    dphi <= phase_tol && sim(&h.content, &c.summary) >= sim_threshold
                })
                .map(|c| c.agent_id.as_str())
                .collect();
            peers.sort_unstable();
            peers.dedup();
            let supporting_peers = peers.len();
            let status = if supporting_peers >= k_peers.max(1) {
                HypothesisStatus::Reinforced
            } else {
                HypothesisStatus::Speculative
            };
            HypothesisVerdict {
                content: h.content.clone(),
                status,
                supporting_peers,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pr(peer: &str, content: &str, sim: f32, phase: f32, conf: f32) -> PeerRecall {
        PeerRecall {
            peer_id: peer.into(),
            content: content.into(),
            similarity: sim,
            amplitude: 1.0,
            phase,
            confidence: conf,
        }
    }

    #[test]
    fn expertise_prefers_on_topic_over_high_phi() {
        let on_topic = score_peer_expertise(&ExpertiseSignals {
            query_similarity: 0.9,
            coverage: 0.8,
            historical_success: 0.6,
            phi: 0.2,
            local_confidence: 0.7,
        });
        let high_phi_off_topic = score_peer_expertise(&ExpertiseSignals {
            query_similarity: 0.1,
            coverage: 0.1,
            historical_success: 0.2,
            phi: 1.0,
            local_confidence: 0.5,
        });
        assert!(on_topic > high_phi_off_topic, "{on_topic} !> {high_phi_off_topic}");
        assert!((0.0..=1.0).contains(&on_topic));
    }

    #[test]
    fn consensus_beats_outlier() {
        // 3 of 4 peers agree on "X"; one lone peer says "Y".
        let recalls = vec![
            pr("a", "X", 0.9, 0.0, 0.9),
            pr("b", "X", 0.85, 0.0, 0.8),
            pr("c", "X", 0.88, 0.0, 0.85),
            pr("d", "Y", 0.6, 3.1, 0.7),
        ];
        let agree = |a: &str, b: &str| a == b;
        let merged = merge_recall_votes(&recalls, 4, agree);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].content, "X"); // highest confidence first
        assert_eq!(merged[0].support, 3);
        let y = merged.iter().find(|c| c.content == "Y").unwrap();
        assert!(merged[0].confidence > y.confidence, "consensus should beat outlier");
        assert_eq!(y.support, 1);
    }

    #[test]
    fn detects_opposed_stance_not_agreement() {
        // Same subject (sim=1.0), opposite phase (≈π apart) -> contradiction.
        // Same subject, same phase -> NOT flagged.
        let items = vec![
            pr("a", "rates rise", 1.0, 0.0, 0.9),
            pr("b", "rates rise", 1.0, std::f32::consts::PI, 0.9), // opposed
            pr("c", "rates rise", 1.0, 0.05, 0.9),                // agrees with a
        ];
        let sim = |a: &str, b: &str| if a == b { 1.0 } else { 0.0 };
        let found = detect_contradictions(&items, sim, 0.8, std::f32::consts::FRAC_PI_2);
        // `b` (phase π) is opposed to BOTH `a` (0.0) and `c` (0.05) -> 2 pairs.
        // The agreeing pair (a, c) at gap 0.05 must NOT be flagged.
        assert_eq!(found.len(), 2, "b is opposed to both a and c");
        assert!(
            found.iter().all(|c| c.phase_gap > std::f32::consts::FRAC_PI_2),
            "only opposed pairs flagged; the a–c agreement is excluded"
        );
    }

    #[test]
    fn brief_confidence_penalized_by_contradictions() {
        let known = vec![ConsensusItem {
            content: "X".into(),
            support: 3,
            confidence: 0.8,
            mean_similarity: 0.9,
        }];
        let clean = compose_brief("t", known.clone(), vec![], vec![("a".into(), 0.9)]);
        let conflicted = compose_brief(
            "t",
            known,
            vec![Contradiction {
                a: "X".into(),
                b: "notX".into(),
                similarity: 0.9,
                phase_gap: 3.0,
            }],
            vec![("a".into(), 0.9)],
        );
        assert!(clean.confidence > conflicted.confidence);
    }

    #[test]
    fn hypothesis_reinforced_by_multiple_peers_else_speculative() {
        let hyps = vec![
            Hypothesis { content: "rates rise".into(), phase: 0.0 },
            Hypothesis { content: "local fancy".into(), phase: 0.0 },
        ];
        let peers = vec![
            PeerCluster { agent_id: "a".into(), summary: "rates rise".into(), phase: 0.05 },
            PeerCluster { agent_id: "b".into(), summary: "rates rise".into(), phase: 0.1 },
            PeerCluster { agent_id: "c".into(), summary: "rates rise".into(), phase: std::f32::consts::PI }, // anti-phase, excluded
        ];
        let sim = |a: &str, b: &str| if a == b { 1.0 } else { 0.0 };
        let v = reinforce_hypotheses(&hyps, &peers, sim, 0.8, std::f32::consts::FRAC_PI_2, 2);
        // "rates rise" supported by a,b (c is anti-phase) -> Reinforced.
        assert_eq!(v[0].status, HypothesisStatus::Reinforced);
        assert_eq!(v[0].supporting_peers, 2);
        // "local fancy" has no peer support -> Speculative.
        assert_eq!(v[1].status, HypothesisStatus::Speculative);
        assert_eq!(v[1].supporting_peers, 0);
    }

    #[test]
    fn empty_peers_makes_everything_speculative() {
        let hyps = vec![Hypothesis { content: "x".into(), phase: 0.0 }];
        let v = reinforce_hypotheses(&hyps, &[], |_, _| 1.0, 0.8, 1.0, 1);
        assert_eq!(v[0].status, HypothesisStatus::Speculative);
    }
}
