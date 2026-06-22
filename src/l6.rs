//! ADR-0037 L6 instrument — belief-core *identity* across dreams.
//!
//! The spiral telemetry counts cores per dream, but "a belief = a stabilized
//! spiral core" is only falsifiable if we can follow an *individual* core over
//! time (lifetime / position drift / content-correspondence). The hard part is
//! that each dream rebuilds the 2-D PCA embedding from scratch, so a core's
//! `(x, y)` is NOT comparable across dreams (sign/rotation ambiguity).
//!
//! The fix is a **frame-invariant content fingerprint**: a deterministic random
//! projection of the core's neighbourhood centroid (a high-dim wavefront vector,
//! which lives in the stable embedding space, not the unstable 2-D PCA). Cosine
//! of two fingerprints is preserved under PCA flips, so cores can be matched by
//! *what they organize* rather than *where they landed*. `build_tracks` chains
//! per-dream observations into tracks; a long track = a persistent belief.

use serde::{Deserialize, Serialize};

/// One spiral core observed in a single dream. `fp` is the frame-invariant
/// content fingerprint (see `fingerprint`); `(x, y)` is the per-dream 2-D
/// position (only meaningful within that dream's embedding); `phase` is the
/// core's holistic phase at its anchor (the value a peer node couples toward in
/// Track-D — see `ChiralMedium::couple_toward_peer_cores`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreObs {
    pub x: f32,
    pub y: f32,
    pub charge: i32,
    pub fp: Vec<f32>,
    /// Holistic phase at the core anchor (radians). `#[serde(default)]` so
    /// pre-Track-D `l6-cores.jsonl` snapshots (no phase field) still parse.
    #[serde(default)]
    pub phase: f32,
}

/// Deterministic random projection of a content `vector` to `dims` (a compact,
/// frame-invariant fingerprint). Cosine-preserving (Johnson–Lindenstrauss) so
/// `cosine` on two fingerprints approximates cosine of the source vectors. The
/// projection weights come from a SplitMix64 finalizer keyed by (dimension,
/// index) — no stored matrix, identical across processes/runs. Output is L2-
/// normalized so `cosine` reduces to a dot product.
pub fn fingerprint(vector: &[f32], dims: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; dims.max(1)];
    for (d, o) in out.iter_mut().enumerate() {
        let seed = 0x9E37_79B9_7F4A_7C15u64.wrapping_add((d as u64).wrapping_mul(0xD1B5_4A32_D192_ED03));
        let mut acc = 0.0f32;
        for (i, &v) in vector.iter().enumerate() {
            let mut h = seed ^ (i as u64).wrapping_mul(0x2545_F491_4F6C_DD1D);
            h ^= h >> 30;
            h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
            h ^= h >> 27;
            h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
            h ^= h >> 31;
            let w = (h as f32 / u64::MAX as f32) * 2.0 - 1.0; // ~[-1, 1)
            acc += v * w;
        }
        *o = acc;
    }
    let norm = out.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for o in out.iter_mut() {
            *o /= norm;
        }
    }
    out
}

/// Cosine similarity of two fingerprints. Assumes L2-normalized inputs (as
/// produced by `fingerprint`), so this is a dot product; falls back to 0 on a
/// length mismatch.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Index + cosine of the core in `cores` whose fingerprint best matches `fp`.
/// If `charge` is `Some(q)`, only cores of charge `q` are considered. Returns
/// `None` when no eligible core exists. Used by Track-D peer-core coupling +
/// the cross-node shared-core metric.
pub fn nearest_core(fp: &[f32], cores: &[CoreObs], charge: Option<i32>) -> Option<(usize, f32)> {
    let mut best: Option<(usize, f32)> = None;
    for (j, c) in cores.iter().enumerate() {
        if let Some(q) = charge {
            if c.charge != q {
                continue;
            }
        }
        let s = cosine(fp, &c.fp);
        if best.map(|(_, bs)| s > bs).unwrap_or(true) {
            best = Some((j, s));
        }
    }
    best
}

/// Count how many `own` cores have a same-charge `peer` core within fingerprint
/// cosine ≥ `min_cos` — the cross-node "shared beliefs" tally (Track-D's
/// falsifiable "shared cores ⇒ swarm agreement" measurement). Asymmetric: it
/// counts own cores that are mirrored by the peer.
pub fn shared_cores(own: &[CoreObs], peer: &[CoreObs], min_cos: f32) -> usize {
    own.iter()
        .filter(|c| {
            nearest_core(&c.fp, peer, Some(c.charge))
                .map(|(_, s)| s >= min_cos)
                .unwrap_or(false)
        })
        .count()
}

/// A core followed across dreams: same charge + fingerprint-matched neighbours.
#[derive(Debug, Clone, Serialize)]
pub struct Track {
    pub id: usize,
    pub charge: i32,
    /// Snapshot index where the track first appeared.
    pub first: usize,
    /// Snapshot index where it was last seen.
    pub last: usize,
    /// Number of snapshots (dreams) the core appeared in.
    pub appearances: usize,
    /// last_seen - first_seen + 1 (the span it was alive over).
    pub span: usize,
    /// Most recent 2-D position (frame-local; for a rough drift read only).
    pub last_pos: (f32, f32),
}

struct ActiveTrack {
    id: usize,
    charge: i32,
    fp: Vec<f32>,
    first: usize,
    last: usize,
    appearances: usize,
    last_pos: (f32, f32),
}

/// Chain per-dream core observations into tracks. Greedy bipartite match between
/// each snapshot's cores and the running tracks: a (core, track) pair is eligible
/// iff same `charge` and fingerprint cosine ≥ `min_cos`; pairs are assigned
/// highest-cosine-first. Matched tracks extend (and adopt the latest fingerprint,
/// to follow slow content drift); unmatched cores start new tracks. Tracks are
/// never force-closed (a core may flicker between dreams), so `appearances`
/// counts real sightings and `span` the birth→last range.
pub fn build_tracks(snapshots: &[Vec<CoreObs>], min_cos: f32) -> Vec<Track> {
    let mut active: Vec<ActiveTrack> = Vec::new();
    let mut next_id = 0usize;

    for (snap_idx, snap) in snapshots.iter().enumerate() {
        // Candidate (cosine, core_idx, active_idx) pairs, eligible by charge+threshold.
        let mut pairs: Vec<(f32, usize, usize)> = Vec::new();
        for (ci, core) in snap.iter().enumerate() {
            for (aj, at) in active.iter().enumerate() {
                if at.charge != core.charge {
                    continue;
                }
                let c = cosine(&core.fp, &at.fp);
                if c >= min_cos {
                    pairs.push((c, ci, aj));
                }
            }
        }
        pairs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut core_used = vec![false; snap.len()];
        let mut track_used = vec![false; active.len()];
        for (_c, ci, aj) in pairs {
            if core_used[ci] || track_used[aj] {
                continue;
            }
            core_used[ci] = true;
            track_used[aj] = true;
            let core = &snap[ci];
            active[aj].last = snap_idx;
            active[aj].appearances += 1;
            active[aj].last_pos = (core.x, core.y);
            active[aj].fp = core.fp.clone(); // follow drift
        }

        for (ci, core) in snap.iter().enumerate() {
            if !core_used[ci] {
                active.push(ActiveTrack {
                    id: next_id,
                    charge: core.charge,
                    fp: core.fp.clone(),
                    first: snap_idx,
                    last: snap_idx,
                    appearances: 1,
                    last_pos: (core.x, core.y),
                });
                next_id += 1;
            }
        }
    }

    let mut tracks: Vec<Track> = active
        .into_iter()
        .map(|a| Track {
            id: a.id,
            charge: a.charge,
            first: a.first,
            last: a.last,
            appearances: a.appearances,
            span: a.last - a.first + 1,
            last_pos: a.last_pos,
        })
        .collect();
    // Longest-lived first.
    tracks.sort_by(|a, b| b.appearances.cmp(&a.appearances).then(b.span.cmp(&a.span)));
    tracks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(charge: i32, fp: Vec<f32>) -> CoreObs {
        CoreObs { x: 0.0, y: 0.0, charge, fp, phase: 0.0 }
    }

    #[test]
    fn fingerprint_is_deterministic_and_normalized() {
        let v = vec![0.3, -0.1, 0.7, 0.2, -0.5];
        let a = fingerprint(&v, 16);
        let b = fingerprint(&v, 16);
        assert_eq!(a, b, "fingerprint must be deterministic");
        let norm = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4 || norm == 0.0, "fingerprint must be normalized");
    }

    #[test]
    fn fingerprint_preserves_similarity() {
        // Similar source vectors → high fingerprint cosine; dissimilar → low.
        let a = fingerprint(&[1.0, 0.0, 0.0, 0.0, 0.5, 0.2], 32);
        let a2 = fingerprint(&[1.0, 0.05, 0.0, 0.0, 0.5, 0.18], 32);
        let b = fingerprint(&[0.0, 0.0, 1.0, -0.8, 0.0, 0.0], 32);
        assert!(cosine(&a, &a2) > 0.9, "near-identical content should match");
        assert!(cosine(&a, &b) < cosine(&a, &a2), "different content should be less similar");
    }

    #[test]
    fn build_tracks_follows_a_persistent_core() {
        // A stable +1 core present in all 3 dreams, plus a one-off −1 core.
        let stable = fingerprint(&[1.0, 0.2, -0.3, 0.4], 16);
        let transient = fingerprint(&[-0.5, 0.9, 0.1, 0.0], 16);
        let snaps = vec![
            vec![obs(1, stable.clone())],
            vec![obs(1, stable.clone()), obs(-1, transient.clone())],
            vec![obs(1, stable.clone())],
        ];
        let tracks = build_tracks(&snaps, 0.85);
        // The persistent core is the longest-lived track.
        let top = &tracks[0];
        assert_eq!(top.charge, 1);
        assert_eq!(top.appearances, 3, "stable core seen in all 3 dreams");
        assert_eq!(top.span, 3);
        // The transient core is its own short track.
        assert!(tracks.iter().any(|t| t.charge == -1 && t.appearances == 1));
    }

    #[test]
    fn opposite_charges_never_match() {
        let fp = fingerprint(&[1.0, 0.2, -0.3, 0.4], 16);
        let snaps = vec![vec![obs(1, fp.clone())], vec![obs(-1, fp.clone())]];
        let tracks = build_tracks(&snaps, 0.5);
        // Same fingerprint but opposite charge ⇒ two distinct tracks, not one.
        assert_eq!(tracks.len(), 2);
        assert!(tracks.iter().all(|t| t.appearances == 1));
    }

    // ── Track-D cross-node matching ──────────────────────────────────────
    #[test]
    fn nearest_core_respects_charge_and_picks_best() {
        let a = fingerprint(&[1.0, 0.0, 0.0, 0.2], 16);
        let a2 = fingerprint(&[1.0, 0.03, 0.0, 0.21], 16); // ~a
        let b = fingerprint(&[0.0, 1.0, -0.5, 0.0], 16); // different
        let cores = vec![obs(1, b.clone()), obs(1, a2.clone()), obs(-1, a.clone())];
        // Best +1 match for `a` is index 1 (a2), not the -1 core (excluded by charge).
        let (j, s) = nearest_core(&a, &cores, Some(1)).unwrap();
        assert_eq!(j, 1);
        assert!(s > 0.9);
        // No +1 core at all ⇒ None.
        assert!(nearest_core(&a, &[obs(-1, a.clone())], Some(1)).is_none());
    }

    #[test]
    fn shared_cores_counts_mirrored_beliefs() {
        let shared = fingerprint(&[1.0, 0.2, -0.3, 0.4], 16);
        let mine_only = fingerprint(&[-0.7, 0.1, 0.9, 0.0], 16);
        let own = vec![obs(1, shared.clone()), obs(1, mine_only.clone())];
        let peer = vec![obs(1, shared.clone()), obs(-1, fingerprint(&[0.0, 0.0, 1.0, 1.0], 16))];
        // Only the shared +1 belief is mirrored by the peer.
        assert_eq!(shared_cores(&own, &peer, 0.85), 1);
        // A peer with no overlap shares nothing.
        assert_eq!(shared_cores(&own, &[obs(1, fingerprint(&[0.0, -1.0, 0.0, 0.5], 16))], 0.85), 0);
    }
}
