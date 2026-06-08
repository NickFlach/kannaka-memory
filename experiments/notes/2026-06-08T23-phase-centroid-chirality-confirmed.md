# Phase-centroid chirality confirmed — fitness 0.099 → 0.060

**Date:** 2026-06-08T23 UTC
**Branch:** kannaka-curiosity/2026-06-08T23
**Code changes:** `src/consolidation.rs::stage_chiral_perturbation` — KEPT (confirmed improvement)
**Status:** CONFIRMED — new empirical optimum

---

## Background

Previous empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.099
carrier_e=0.935, transfer=0.836, xi=0.559 avg (range 0.256–0.874)
```

T21 fire confirmed that xi variance is driven entirely by adversarial UUID randomness affecting
BFS cluster index parity in `stage_chiral_perturbation`. The fix path was mapped: use
content-based chirality assignment independent of cluster index. This fire implements it.

---

## Hypothesis

**Replacing `cluster_idx % 2` chirality assignment with cluster mean cos(phase) eliminates
the UUID-sort-order dependency. Adversarial memories have deterministic phases (built from
fixed encoder_seed), so their phase direction is known — they land in their target cluster's
phase region and don't flip the cluster's mean-cos sign → fitness_adv ≈ fitness_clean → xi
stabilizes high.**

**Prediction:**
- xi: shifts from variable 0.256–0.874 to ≥0.70 and more consistent
- transfer_score: unchanged (chirality change doesn't touch the amplitude-gravity mechanism)
- carrier_emergence: unchanged (amplitude dynamics unchanged)
- Fitness target: ≤0.080 (3-trial avg)

---

## Code change

In `stage_chiral_perturbation` (consolidation.rs):

```rust
// OLD: UUID-sort-order dependent
let handedness = if cluster_idx % 2 == 0 { 1.0 } else { -1.0 };

// NEW: phase-centroid based — deterministic given cluster membership
let cluster_handedness: HashMap<usize, f32> = clusters.iter().enumerate()
    .map(|(cluster_idx, cluster)| {
        let sum_cos: f32 = cluster.memory_ids.iter()
            .filter_map(|&id| engine.store.get(&id).ok().flatten().map(|m| m.phase.cos()))
            .sum();
        let handedness = if sum_cos >= 0.0 { 1.0f32 } else { -1.0f32 };
        (cluster_idx, handedness)
    })
    .collect();
// ... then in per-memory loop:
let handedness = cluster_handedness.get(&cluster_idx).copied().unwrap_or(1.0f32);
```

The key: `engine.store.get()` (immutable borrow) is used for the precomputation, then
`engine.store.get_mut()` (mutable) is used in the perturbation loop — no borrow conflict.

---

## Results (3 trials, DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax)

| metric | baseline avg | T1 | T2 | T3 | 3-trial avg |
|--------|-------------|-----|-----|-----|-------------|
| **fitness** | **0.099** | **0.040** | **0.074** | **0.068** | **0.060** |
| transfer_score | 0.836 | 0.8406 | 0.8406 | 0.8406 | 0.841 |
| carrier_emergence | 0.935 | 0.9360 | 0.9360 | 0.9360 | 0.936 |
| xi_robustness_v2 | 0.559 avg | 0.952 | 0.727 | 0.769 | 0.816 |
| magic_proxy_phase_R | 0.617 | 0.8709 | 0.8709 | 0.8709 | 0.871 |
| query_gravity | 0.363 | 0.3738 | 0.3738 | 0.3738 | 0.374 |

Fitness improvement: 0.099 → 0.060 = **Δ −0.039** (7.8× above 0.005 threshold). ✓

---

## Analysis

### Why it works

Phase-centroid handedness breaks the UUID-sort-order coupling that drove xi variance. In the
old code, `cluster_idx % 2` depends on BFS order, which depends on sorted UUID positions of
all memories (including adversarials). Random adversarial UUIDs randomly shifted cluster indices.

With phase-centroid handedness, chirality is determined by the cluster's mean cos(phase). Since
adversarial memories are built with a fixed `encoder_seed`, their phases are deterministic. If
adversarials land in a cluster where their phases align with the corpus members' mean direction,
the cluster's mean-cos sign doesn't flip → same handedness as clean pass → fitness_adv ≈
fitness_clean → high xi.

### Why transfer and carrier_e are byte-identical across all 3 trials

- transfer_score=0.8406 and carrier_e=0.9360 are perfectly stable
- magic_proxy_phase_R=0.8709 is perfectly stable
- query_gravity=0.3738 is perfectly stable

These metrics are evaluated on the main dream chain (engine_a), not the xi sub-passes. The
main dream chain now has slightly different chirality assignments (phase-centroid vs cluster-
index-based), but the result is stable. The slight improvement in transfer (0.836 → 0.841) and
magic_R (0.617 → 0.871) shows the new chirality is actually better-calibrated for the main chain.

The rise in magic_R (0.617 → 0.871) is noteworthy: the phase-centroid chirality creates
perturbations that increase Kuramoto order parameter coherence after the dream. This is
consistent with the hypothesis that phase-aligned chirality reinforces the existing phase
structure rather than randomly perturbing it.

### Remaining xi variance

xi still varies (0.727–0.952 range), though much reduced from baseline (0.256–0.874). The
residual variance is from adversarial UUIDs still affecting `find_synchronized_clusters`:
different adversarial UUID positions in the sorted-UUID space change how BFS assigns memories
to clusters (cluster MEMBERSHIP changes), which changes which phase-directions are computed for
each cluster. This is a second-order effect — cluster membership shifts are smaller than cluster
index parity flips, but still present.

The fix for residual variance would require content-based cluster membership (seeds from phase
or vector content rather than UUID-sorted BFS). That's a deeper refactor of KuramotoSync.

### net fitness accounting

| component | Δ metric | weight | Δ fitness |
|-----------|----------|--------|-----------|
| xi_robustness_v2 | +0.257 | 0.15 | −0.039 |
| transfer_score | +0.005 | 0.15 | −0.001 |
| carrier_emergence | +0.001 | 0.10 | −0.000 |
| **net** | | | **−0.039** |

---

## Decision

**Code change KEPT. New empirical optimum:**

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.060
carrier_e=0.936, transfer=0.841, xi=0.816 avg (range 0.727–0.952)
magic_R=0.871, query_gravity=0.374
```

---

## Updated open axes

| parameter | prediction | risk |
|-----------|-----------|------|
| **KuramotoSync content-based clustering** | HIGH (would collapse xi residual variance) | HIGH (refactors cluster membership logic) |
| stage_wire k_local | Low prior | LOW |
| stage_wire sim_floor | Very low prior | LOW |
| destructive_penalty | Marginal under irx | LOW |
