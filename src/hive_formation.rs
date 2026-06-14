//! Emergent hive formation — ADR-0035 Phase 4 / Wave 4 Task 4.2.
//!
//! Pure resonance-clustering of peers into *purposive* hives. `queen::detect_hives`
//! already clusters peers by phase alone; this layer makes the clustering
//! resonance-aware: two peers join a hive only when they are BOTH phase-coherent
//! AND share knowledge domains (their cluster themes resonate) — so a hive forms
//! because its members *know the same things*, not merely because they phase-locked.
//! No I/O — the CLI/NATS layer supplies peer phases (presence) + domains (exemplars).

/// A peer to be clustered: its phase + the domains (cluster themes) it holds.
#[derive(Clone, Debug)]
pub struct PeerNode {
    pub agent_id: String,
    pub phase: f32,
    pub domains: Vec<String>,
}

/// A formed hive.
#[derive(Clone, Debug)]
pub struct Hive {
    pub members: Vec<String>,
    /// The domain most members share (the hive's specialization), if any.
    pub focus_domain: Option<String>,
    /// Mean intra-hive membership affinity in [0, 1].
    pub cohesion: f32,
}

/// Membership affinity between two peers in [0, 1] = phase coherence × domain
/// overlap. Phase coherence is 1.0 when aligned and falls to 0.0 at `phase_tol`
/// (and beyond). Domain overlap is the best `sim` over any pair of their domains.
/// Both factors are required: phase-locked-but-off-topic OR same-domain-but-
/// anti-phase peers score ~0 and do not co-hive.
pub fn score_hive_membership(
    a: &PeerNode,
    b: &PeerNode,
    sim: impl Fn(&str, &str) -> f32,
    phase_tol: f32,
) -> f32 {
    let two_pi = 2.0 * std::f32::consts::PI;
    let raw = (a.phase - b.phase).abs();
    let dphi = raw.min(two_pi - raw);
    let phase_coh = if phase_tol <= 0.0 || dphi >= phase_tol {
        0.0
    } else {
        (1.0 - dphi / phase_tol).clamp(0.0, 1.0)
    };
    if phase_coh == 0.0 {
        return 0.0;
    }
    let mut domain_overlap = 0.0f32;
    for da in &a.domains {
        for db in &b.domains {
            domain_overlap = domain_overlap.max(sim(da, db).clamp(0.0, 1.0));
        }
    }
    phase_coh * domain_overlap
}

/// Cluster peers into hives via union-find: peers are unioned when their
/// membership affinity ≥ `threshold` (transitive). Singletons (peers with no
/// qualifying neighbor) form their own hive. `focus_domain` = the domain held by
/// the most members; `cohesion` = mean affinity over intra-hive qualifying edges.
pub fn form_hives(
    peers: &[PeerNode],
    sim: impl Fn(&str, &str) -> f32,
    phase_tol: f32,
    threshold: f32,
) -> Vec<Hive> {
    let n = peers.len();
    if n == 0 {
        return Vec::new();
    }
    // Union-find.
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while parent[r] != r {
            r = parent[r];
        }
        // path compression
        let mut c = x;
        while parent[c] != r {
            let next = parent[c];
            parent[c] = r;
            c = next;
        }
        r
    }

    // Record qualifying edges + their scores for cohesion.
    let mut edges: Vec<(usize, usize, f32)> = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            let s = score_hive_membership(&peers[i], &peers[j], &sim, phase_tol);
            if s >= threshold {
                edges.push((i, j, s));
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri] = rj;
                }
            }
        }
    }

    // Group members by root.
    use std::collections::HashMap;
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }

    let mut hives: Vec<Hive> = groups
        .into_values()
        .map(|idxs| {
            let members: Vec<String> = idxs.iter().map(|&i| peers[i].agent_id.clone()).collect();
            // focus_domain = most frequent domain across members.
            let mut counts: HashMap<&str, usize> = HashMap::new();
            for &i in &idxs {
                for d in &peers[i].domains {
                    *counts.entry(d.as_str()).or_default() += 1;
                }
            }
            let focus_domain = counts
                .into_iter()
                .max_by_key(|(_, c)| *c)
                .map(|(d, _)| d.to_string());
            // cohesion = mean score over intra-group qualifying edges.
            let group: std::collections::HashSet<usize> = idxs.iter().copied().collect();
            let intra: Vec<f32> = edges
                .iter()
                .filter(|(i, j, _)| group.contains(i) && group.contains(j))
                .map(|(_, _, s)| *s)
                .collect();
            let cohesion = if intra.is_empty() {
                1.0 // singleton hive
            } else {
                intra.iter().sum::<f32>() / intra.len() as f32
            };
            Hive {
                members,
                focus_domain,
                cohesion,
            }
        })
        .collect();
    // Stable order: larger hives first, then by focus domain.
    hives.sort_by(|a, b| {
        b.members
            .len()
            .cmp(&a.members.len())
            .then_with(|| a.focus_domain.cmp(&b.focus_domain))
    });
    hives
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, phase: f32, domains: &[&str]) -> PeerNode {
        PeerNode {
            agent_id: id.into(),
            phase,
            domains: domains.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn exact(a: &str, b: &str) -> f32 {
        if a == b {
            1.0
        } else {
            0.0
        }
    }

    #[test]
    fn same_domain_close_phase_co_hive() {
        let peers = vec![
            node("a", 0.0, &["ml"]),
            node("b", 0.3, &["ml"]),
        ];
        let hives = form_hives(&peers, exact, std::f32::consts::FRAC_PI_2, 0.3);
        assert_eq!(hives.len(), 1);
        assert_eq!(hives[0].members.len(), 2);
        assert_eq!(hives[0].focus_domain.as_deref(), Some("ml"));
    }

    #[test]
    fn off_topic_but_phase_close_does_not_co_hive() {
        // Phase-close but no shared domain -> separate singleton hives.
        let peers = vec![
            node("a", 0.0, &["ml"]),
            node("b", 0.1, &["cooking"]),
        ];
        let hives = form_hives(&peers, exact, std::f32::consts::FRAC_PI_2, 0.3);
        assert_eq!(hives.len(), 2);
    }

    #[test]
    fn same_domain_anti_phase_does_not_co_hive() {
        let peers = vec![
            node("a", 0.0, &["ml"]),
            node("b", std::f32::consts::PI, &["ml"]),
        ];
        let hives = form_hives(&peers, exact, std::f32::consts::FRAC_PI_2, 0.3);
        assert_eq!(hives.len(), 2);
    }

    #[test]
    fn transitive_hive_via_shared_domain() {
        // a~b and b~c (chain) -> all three in one hive even if a,c are a bit apart.
        let peers = vec![
            node("a", 0.0, &["ml"]),
            node("b", 0.4, &["ml"]),
            node("c", 0.8, &["ml"]),
        ];
        let hives = form_hives(&peers, exact, std::f32::consts::FRAC_PI_2, 0.2);
        assert_eq!(hives.len(), 1);
        assert_eq!(hives[0].members.len(), 3);
    }

    #[test]
    fn empty_yields_no_hives() {
        let hives = form_hives(&[], exact, 1.0, 0.3);
        assert!(hives.is_empty());
    }
}
