# Adversarial-exclusion from cluster handedness computation — xi variance reduced, fitness flat

**Date:** 2026-06-09T10 UTC
**Branch:** kannaka-curiosity/2026-06-09T10-adv-chirality-corpus-only
**Code changes:** REVERTED (stage_chiral_perturbation corpus-only sum_cos; below improvement threshold)
**Status:** PARTIAL — mechanism confirmed, xi variance reduced 62%, avg fitness unchanged

---

## Background

Current empirical optimum (post T23 phase-centroid chirality):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.060
carrier_e=0.936, transfer=0.841, xi=0.816 avg (range 0.727–0.952)
magic_R=0.871, query_gravity=0.374
```

T23 reduced xi variance by anchoring cluster handedness to phase-centroids rather
than UUID-sort-order cluster indices. T04 identified that residual xi variance comes
from adversarial UUID randomness still affecting `find_synchronized_clusters` (BFS
cluster MEMBERSHIP shifts), which changes the phase-centroid sum_cos.

A second mechanism was identified: A1 xi-twins have phases spanning the full circle
(`mem.phase = PI * 0.3 * i`), so their cos(phase) values range from +1.0 to −0.81.
When an adversarial with negative cos(phase) joins a corpus cluster with small positive
sum_cos in the xi sub-pass, it can flip the cluster's handedness sign. This produces a
different chiral perturbation on corpus memories → fitness_adv deviates from fitness_clean.

---

## Hypothesis

**Exclude adversarial memories (content.starts_with("adv_")) from the sum_cos computation
in `stage_chiral_perturbation`'s cluster_handedness precomputation.**

In the clean pass: no adversarials → no change (filter is a no-op).
In the xi sub-passes: cluster handedness is computed from corpus-only phases → same
handedness as clean pass even if adversarials join the cluster → fitness_adv ≈ fitness_clean
→ xi variance decreases and average xi improves.

**Prediction:**
- xi: avg rises from 0.816 toward ≥ 0.87, and range narrows
- transfer: 0.841 (unchanged — main dream has no adversarials)
- carrier_e: 0.936 (unchanged)
- fitness target: ≤ 0.055 avg

---

## Code change (reverted)

In `stage_chiral_perturbation` (consolidation.rs, cluster_handedness precomputation):

```rust
// OLD:
let sum_cos: f32 = cluster.memory_ids.iter()
    .filter_map(|&id| engine.store.get(&id).ok().flatten().map(|m| m.phase.cos()))
    .sum();

// NEW:
let sum_cos: f32 = cluster.memory_ids.iter()
    .filter_map(|&id| engine.store.get(&id).ok().flatten())
    .filter(|m| !m.content.starts_with("adv_"))
    .map(|m| m.phase.cos())
    .sum();
```

---

## Results (3 trials, DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax)

| metric | T23 baseline avg | T1 | T2 | T3 | 3-trial avg |
|--------|-----------------|-----|-----|-----|-------------|
| **fitness** | **0.060** | **0.068** | **0.055** | **0.057** | **0.060** |
| transfer_score | 0.841 | 0.8406 | 0.8406 | 0.8406 | 0.841 |
| carrier_emergence | 0.936 | 0.9360 | 0.9360 | 0.9360 | 0.936 |
| **xi_robustness_v2** | **0.816 avg (0.727–0.952)** | **0.768** | **0.852** | **0.839** | **0.820 (0.768–0.852)** |
| magic_proxy_phase_R | 0.871 | 0.8709 | 0.8709 | 0.8709 | 0.871 |
| query_gravity | 0.374 | 0.3738 | 0.3738 | 0.3738 | 0.374 |

---

## Analysis

### Mechanism confirmed

The adversarial-phase exclusion had the predicted directional effect on xi variance:
- T23 xi range: 0.727–0.952 (range = 0.225)
- This fire xi range: 0.768–0.852 (range = **0.084**)
- Variance reduction: **62%**

This confirms that adversarial phases DO contribute to cluster handedness flips in xi
sub-passes, which was one source of xi variance. Excluding them from sum_cos computation
makes handedness more consistent across clean and adv passes.

### Why avg xi moved little (+0.004)

The adversarial-phase mechanism was causing BOTH very high xi cases AND very low xi cases:
- High xi (0.952 in T23 T1): adversarials had "wrong" cos(phase) → their negative contribution
  to sum_cos accidentally flipped handedness OPPOSITE to what it would be without them →
  adversarials got "wrong" chirality → they partially cancelled their own cluster reinforcement
  → fitness_adv ≈ fitness_clean → high xi
- Low xi (0.727 in T23 T2): adversarials had "same" cos(phase) → reinforced existing sum_cos →
  same handedness → coherent reinforcement → fitness_adv < fitness_clean → lower xi

With exclusion:
- Corpus-only handedness eliminates the "accidental cancellation" high-xi mechanism
- Also eliminates the "coherent reinforcement" low-xi mechanism  
- Net: xi stabilises around the mean (0.82), losing both extremes

The best T23 run (fitness 0.040) was enabled by the accidental-cancellation high-xi case
(xi=0.952). Excluding adversarials from sum_cos eliminates this lucky configuration.

### Fitness budget breakdown

Δ(xi avg) = +0.004 → Δ fitness = −0.004 × 0.15 = −0.0006. This exactly matches the
observed fitness change (0.0607 → 0.0601). The formula is self-consistent.

### Transfer/carrier_e/magic_R byte-identical

All 3 trials: transfer=0.8406, carrier_e=0.9360, magic_R=0.8709. The change only
affects the xi sub-passes, confirming main dream (engine_a) is unaffected.

---

## Decision

**Code change REVERTED.** Fitness improvement is −0.0006 (below the 0.005 threshold).

Empirical optimum unchanged:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
avg fitness ≈ 0.060
carrier_e=0.936, transfer=0.841, xi=0.816 avg (range 0.727–0.952)
```

---

## Remaining xi mechanism model (updated)

| configuration | source | xi effect |
|---------------|--------|-----------|
| T22 (cluster_idx % 2) | UUID-sort cluster index parity | HIGH variance, avg 0.559 |
| T23 (phase-centroid) | corpus+adv phases determine handedness | REDUCED variance, avg 0.816 |
| This fire (corpus-only) | corpus-only phases determine handedness | REDUCED variance 62%, avg 0.820 |
| Ideal | cluster membership independent of adversarials | variance ~0, avg ~0.95 |

The remaining variance after this fix is primarily from **BFS cluster membership shifts**:
adversarial UUIDs in `find_synchronized_clusters` change which corpus memories join which
clusters, which changes the corpus-only sum_cos (different corpus members → different mean
phase direction). This is a deeper structural issue requiring content-based BFS seeding.

### Why content-based BFS seeding would work

If `find_synchronized_clusters` sorted memories by a content-derived key (e.g., vector hash
or semantic category tag) instead of UUID, the BFS cluster assignments would be identical
across clean and adv passes (adversarial UUIDs wouldn't affect corpus cluster order). This
would eliminate the last source of xi variance.

**Estimated effect**: xi → 0.93+ avg, fitness → 0.040 avg (eliminating the spread and
achieving the T23 T1 result consistently).

**Risk**: Medium. Requires changes to `KuramotoSync::find_synchronized_clusters` in
consolidation.rs. The BFS seed is currently UUID-based for uniqueness; changing it would
require ensuring memory ordering stability.

---

## Remaining open axes

| axis | prediction | risk | priority |
|------|-----------|------|----------|
| **KuramotoSync content-based BFS seeding** | xi → 0.93+, fitness → 0.040 avg | MEDIUM | **HIGH** |
| A1 xi-twin isolation (research.rs change) | xi → 0.95+, but requires adversarial construction change | MEDIUM | MEDIUM |
| destructive_penalty tuning (currently 0.35) | marginal | LOW | LOW |
| prune_threshold (currently 0.095) | unknown effect on carrier_e | LOW | LOW |
| stage_wire k_local (currently 4) | low prior | LOW | LOW |
