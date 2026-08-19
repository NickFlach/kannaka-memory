//! Knowledge gap detection — ADR-0035 Capability 1 / Wave 3 Task 3.3.
//!
//! Pure coverage mapping: given the memory clusters held across the swarm, bin
//! them into domains and compute per-domain coverage (breadth × depth ×
//! confidence), then flag the weakly-represented and low-confidence domains. No
//! I/O — the CLI/NATS layer feeds local + peer cluster summaries.
//!
//! This is the collective successor to the single-agent curiosity loop: the
//! ranked [`CoverageGap`] list is "the swarm developing curiosity," and is the
//! direct input to Wave 4's autonomous research planning. `gap_detection_precision`
//! (does it flag held-out domains?) is a candidate L6 fitness metric.
//!
//! BlindSpot detection (domains held by zero agents but adjacent in xi/phase
//! space to populated ones) needs a domain prior + adjacency model and is left
//! for a follow-up; this ships WeaklyRepresented + LowConfidence.

/// One memory cluster held by one agent (the input unit).
#[derive(Clone, Debug)]
pub struct DomainCluster {
    pub agent_id: String,
    pub theme: String,
    pub size: usize,
    /// Kuramoto cluster coherence in [0, 1].
    pub coherence: f32,
    /// Mean amplitude of the cluster's memories in [0, 1].
    pub mean_amplitude: f32,
}

/// Aggregated coverage of one domain across the swarm.
#[derive(Clone, Debug)]
pub struct DomainCoverage {
    pub domain: String,
    /// Distinct agents holding this domain.
    pub peers_holding: usize,
    pub total_size: usize,
    /// breadth = peers_holding / n_agents, in [0, 1].
    pub breadth: f32,
    /// depth = saturating function of total cluster mass, in [0, 1].
    pub depth: f32,
    /// confidence = mean cluster coherence, in [0, 1].
    pub confidence: f32,
    /// coverage = breadth × depth × confidence, in [0, 1].
    pub coverage: f32,
}

/// Why a domain is a gap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GapKind {
    /// Few agents / little mass hold it.
    WeaklyRepresented,
    /// Held with breadth but incoherent / contradicted.
    LowConfidence,
}

/// A flagged coverage gap.
#[derive(Clone, Debug)]
pub struct CoverageGap {
    pub domain: String,
    pub coverage: f32,
    pub kind: GapKind,
    pub peers_holding: usize,
}

/// `depth` saturates: a domain with ~`SATURATION` total memory-mass counts as
/// full depth, so more isn't disproportionately rewarded.
const DEPTH_SATURATION: f32 = 40.0;

/// Group clusters into domains (themes merged by `same_domain`) and compute the
/// coverage map. `n_agents` is the swarm size (≥1). Sorted by coverage ascending
/// (weakest first).
pub fn build_coverage_map(
    clusters: &[DomainCluster],
    n_agents: usize,
    same_domain: impl Fn(&str, &str) -> bool,
) -> Vec<DomainCoverage> {
    let denom = n_agents.max(1) as f32;
    // Group clusters whose themes are the "same domain". O(n × d) by nature:
    // an arbitrary predicate cannot be bucketed. Callers whose predicate is
    // key-reducible (every CLI caller: trim + case-fold) should use
    // `build_coverage_map_keyed`, which does the same job in O(n) (#771).
    let mut groups: Vec<(String, Vec<&DomainCluster>)> = Vec::new();
    for c in clusters {
        match groups.iter_mut().find(|(rep, _)| same_domain(rep, &c.theme)) {
            Some((_, members)) => members.push(c),
            None => groups.push((c.theme.clone(), vec![c])),
        }
    }

    finish_coverage_map(groups, denom)
}

/// Shared scoring/sort tail for both grouping strategies, so the keyed fast
/// path (#771) and the predicate path cannot drift apart.
fn finish_coverage_map(
    groups: Vec<(String, Vec<&DomainCluster>)>,
    denom: f32,
) -> Vec<DomainCoverage> {
    let mut map: Vec<DomainCoverage> = groups
        .into_iter()
        .map(|(domain, members)| {
            let mut agents: Vec<&str> = members.iter().map(|m| m.agent_id.as_str()).collect();
            agents.sort_unstable();
            agents.dedup();
            let peers_holding = agents.len();
            let total_size: usize = members.iter().map(|m| m.size).sum();
            // depth weights size by mean amplitude (active mass), then saturates.
            let mass: f32 = members
                .iter()
                .map(|m| m.size as f32 * m.mean_amplitude.clamp(0.0, 1.0))
                .sum();
            let breadth = (peers_holding as f32 / denom).clamp(0.0, 1.0);
            let depth = (mass / DEPTH_SATURATION).clamp(0.0, 1.0);
            let confidence = (members.iter().map(|m| m.coherence.clamp(0.0, 1.0)).sum::<f32>()
                / members.len() as f32)
                .clamp(0.0, 1.0);
            let coverage = breadth * depth * confidence;
            DomainCoverage {
                domain,
                peers_holding,
                total_size,
                breadth,
                depth,
                confidence,
                coverage,
            }
        })
        .collect();

    map.sort_by(|a, b| a.coverage.total_cmp(&b.coverage));
    map
}

/// `build_coverage_map` for key-reducible domain predicates: clusters whose
/// `key(theme)` are equal share a domain. Single HashMap pass — O(n) expected
/// — instead of the predicate version's O(n × d) scan, which degrades to
/// O(n²) when themes are largely distinct (#771: ~25M comparisons for 5,000
/// unique-theme observations on the `kannaka gaps` hot path). First-seen
/// theme string stays the domain representative, matching the predicate
/// version's behavior exactly.
pub fn build_coverage_map_keyed(
    clusters: &[DomainCluster],
    n_agents: usize,
    key: impl Fn(&str) -> String,
) -> Vec<DomainCoverage> {
    let denom = n_agents.max(1) as f32;
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut groups: Vec<(String, Vec<&DomainCluster>)> = Vec::new();
    for c in clusters {
        let k = key(&c.theme);
        match index.get(&k) {
            Some(&gi) => groups[gi].1.push(c),
            None => {
                index.insert(k, groups.len());
                groups.push((c.theme.clone(), vec![c]));
            }
        }
    }
    finish_coverage_map(groups, denom)
}

/// Flag domains whose coverage is below `weak_threshold`. A weak domain that is
/// nonetheless broadly held (breadth ≥ 0.5) but incoherent (confidence < 0.5) is
/// `LowConfidence`; otherwise `WeaklyRepresented`. Sorted by coverage ascending.
pub fn detect_gaps(map: &[DomainCoverage], weak_threshold: f32) -> Vec<CoverageGap> {
    let mut gaps: Vec<CoverageGap> = map
        .iter()
        .filter(|d| d.coverage < weak_threshold)
        .map(|d| {
            let kind = if d.breadth >= 0.5 && d.confidence < 0.5 {
                GapKind::LowConfidence
            } else {
                GapKind::WeaklyRepresented
            };
            CoverageGap {
                domain: d.domain.clone(),
                coverage: d.coverage,
                kind,
                peers_holding: d.peers_holding,
            }
        })
        .collect();
    gaps.sort_by(|a, b| a.coverage.total_cmp(&b.coverage));
    gaps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cl(agent: &str, theme: &str, size: usize, coh: f32, amp: f32) -> DomainCluster {
        DomainCluster {
            agent_id: agent.into(),
            theme: theme.into(),
            size,
            coherence: coh,
            mean_amplitude: amp,
        }
    }

    #[test]
    fn narrowly_held_domain_has_lower_coverage_than_widely_held() {
        // "wide" held by 5 agents; "narrow" by 1, same size/coherence each.
        let mut clusters = vec![cl("a1", "narrow", 10, 0.9, 0.8)];
        for i in 0..5 {
            clusters.push(cl(&format!("b{i}"), "wide", 10, 0.9, 0.8));
        }
        let map = build_coverage_map(&clusters, 5, |a, b| a == b);
        let narrow = map.iter().find(|d| d.domain == "narrow").unwrap();
        let wide = map.iter().find(|d| d.domain == "wide").unwrap();
        assert!(narrow.coverage < wide.coverage, "{} !< {}", narrow.coverage, wide.coverage);
        assert_eq!(narrow.peers_holding, 1);
        assert_eq!(wide.peers_holding, 5);
    }

    #[test]
    fn coherent_well_covered_domain_is_not_flagged() {
        let mut clusters = Vec::new();
        for i in 0..4 {
            clusters.push(cl(&format!("a{i}"), "solid", 30, 0.95, 0.9));
        }
        let map = build_coverage_map(&clusters, 4, |a, b| a == b);
        let gaps = detect_gaps(&map, 0.25);
        assert!(gaps.is_empty(), "well-covered domain should not be a gap: {gaps:?}");
    }

    #[test]
    fn incoherent_broad_domain_is_low_confidence() {
        // Held by 3 of 3 agents (broad) but incoherent (low coherence).
        let clusters = vec![
            cl("a", "shaky", 30, 0.2, 0.9),
            cl("b", "shaky", 30, 0.2, 0.9),
            cl("c", "shaky", 30, 0.2, 0.9),
        ];
        let map = build_coverage_map(&clusters, 3, |a, b| a == b);
        let gaps = detect_gaps(&map, 0.5);
        let g = gaps.iter().find(|g| g.domain == "shaky").unwrap();
        assert_eq!(g.kind, GapKind::LowConfidence);
    }

    #[test]
    fn rare_domain_is_weakly_represented() {
        let mut clusters = vec![cl("a", "rare", 3, 0.9, 0.5)];
        for i in 0..4 {
            clusters.push(cl(&format!("b{i}"), "common", 30, 0.9, 0.9));
        }
        let map = build_coverage_map(&clusters, 5, |a, b| a == b);
        let gaps = detect_gaps(&map, 0.3);
        let g = gaps.iter().find(|g| g.domain == "rare").unwrap();
        assert_eq!(g.kind, GapKind::WeaklyRepresented);
    }
}

#[cfg(test)]
mod keyed_parity_tests {
    use super::*;

    fn cluster(agent: &str, theme: &str, size: usize) -> DomainCluster {
        DomainCluster {
            agent_id: agent.into(),
            theme: theme.into(),
            size,
            coherence: 0.8,
            mean_amplitude: 0.5,
        }
    }

    /// #771: the keyed fast path must be BEHAVIORALLY IDENTICAL to the
    /// predicate path for the predicate every CLI caller passes
    /// (trim + eq_ignore_ascii_case ⇔ key = trim + to_ascii_lowercase) —
    /// including first-seen representative naming and ordering.
    #[test]
    fn keyed_matches_predicate_for_the_cli_predicate() {
        let clusters = vec![
            cluster("a", "Rust Perf", 10),
            cluster("b", "  rust perf ", 5),
            cluster("a", "Sleep Science", 20),
            cluster("c", "RUST PERF", 1),
            cluster("b", "sleep science", 2),
            cluster("c", "Lone Topic", 3),
        ];
        let via_pred = build_coverage_map(&clusters, 3, |a, b| {
            a.trim().eq_ignore_ascii_case(b.trim())
        });
        let via_key = build_coverage_map_keyed(&clusters, 3, |t| t.trim().to_ascii_lowercase());
        assert_eq!(via_pred.len(), via_key.len());
        for (p, k) in via_pred.iter().zip(via_key.iter()) {
            assert_eq!(p.domain, k.domain, "first-seen representative must match");
            assert_eq!(p.peers_holding, k.peers_holding);
            assert_eq!(p.total_size, k.total_size);
            assert!((p.breadth - k.breadth).abs() < 1e-6);
            assert!((p.depth - k.depth).abs() < 1e-6);
            assert!((p.confidence - k.confidence).abs() < 1e-6);
        }
    }
}
