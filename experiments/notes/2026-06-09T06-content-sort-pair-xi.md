# Content-sort pair selection eliminates xi variance — fitness 0.060 → 0.036

**Date:** 2026-06-09T06 UTC
**Branch:** kannaka-curiosity/2026-06-09T06-content-sort-pair-xi
**Code changes:** `src/consolidation.rs::apply_targeted_chiral_perturbation` — KEPT (confirmed improvement)
**Status:** CONFIRMED — new empirical optimum

---

## Background

Post-T23 empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.060
transfer_score=0.841, carrier_e=0.936, xi_robustness_v2=0.816 avg (range 0.727–0.952)
magic_R=0.871, query_gravity=0.374
```

T23 (phase-centroid chirality) reduced xi variance dramatically (0.256–0.874 → 0.727–0.952)
and improved avg xi from 0.559 to 0.816. T04 and T05 confirmed that stabilizing chirality
via UUID-sort approaches worsens xi; adversarial exclusion from stage_chiral_perturbation
also worsens xi.

T05 concluded the residual xi variance source was "adversarial UUIDs affecting
find_synchronized_clusters cluster MEMBERSHIP." This fire investigated further and found
a more specific and tractable root cause.

---

## Root cause analysis

`apply_targeted_chiral_perturbation` iterates pairs `(i, j)` with `j < i + 20` in
`working_set` order (UUID-sorted from `all_memories()`). Adversarial memories in the xi
adversarial pass are inserted via `engine_adv.store.insert(mem)` without explicit UUIDs
→ random UUID per run → different positions in `all_memories()` → different `working_set`
positions → different pairs selected → different chiral vector perturbations applied →
different fitness_adv → xi variance.

This is distinct from the T23 mechanism (cluster INDEX parity from BFS order). T23 fixed
the handedness assignment but not the pair selection ORDER, which remained UUID-dependent.

The pair selection limited to `j < i + 20` is a locality window that makes sense for
content-ordered data but is arbitrary/harmful when memories are UUID-ordered (UUIDs carry
no content information). When adversarials land between corpus memories in UUID space, they
form corpus-adversarial pairs → cross-contamination in vector perturbations → fitness_adv
diverges from fitness_clean → xi falls.

---

## Hypothesis

**Sorting `working_set` by content-string hash inside `apply_targeted_chiral_perturbation`
makes pair selection UUID-order-independent. Adversarial content strings (`"adv_l5_a1_xi_twin
0"` through `"adv_l5_a3_freq_attack 9"`) cluster together in sorted order. Inter-adversarial
cosine similarity is low (different xi-signature spaces) → adversarial-adversarial pairs
don't pass the 0.6 similarity threshold → adversarials effectively excluded from targeted
pair perturbation → corpus-adversarial vector cross-contamination eliminated → fitness_adv
more stable → xi higher and consistent.**

Secondary effect: corpus memories sorted by content string cluster within category
(`dense_a`, `dense_b`, etc.) → pairs are within-category → higher intra-category cosine
similarity → more pairs pass threshold → stronger, more targeted perturbation → improved
transfer and carrier_e.

**Prediction:**
- xi: 0.816 avg → ≥ 0.87 avg, variance ≤ 0.08 (narrowed from 0.225 range)
- transfer_score: 0.841 → improved (better pairs)
- carrier_emergence: 0.936 → unchanged or slight improvement
- fitness: avg ≤ 0.050

---

## Code change

In `apply_targeted_chiral_perturbation` (consolidation.rs), before the pair-selection loop:

```rust
// Sort by content string so pair selection is UUID-order-independent.
// working_set follows all_memories() insertion order; adversarial memories in
// the xi adversarial pass have random UUIDs and land at different positions
// each run → different pairs → different vector perturbations → xi variance.
let sorted_ids: Vec<Uuid> = {
    let mut pairs: Vec<(Uuid, u64)> = working_set.iter().map(|&id| {
        let h = engine.store.get(&id).ok().flatten()
            .map(|m| m.content.as_bytes().iter()
                .fold(0u64, |a, &b| a.wrapping_mul(31).wrapping_add(b as u64)))
            .unwrap_or(u64::MAX);
        (id, h)
    }).collect();
    pairs.sort_by_key(|&(_, h)| h);
    pairs.into_iter().map(|(id, _)| id).collect()
};
```

Then use `sorted_ids[i]` and `sorted_ids[j]` in the pair loop.

The immutable borrows in the `.map()` chain are released before any mutable borrows later.
No borrow-checker issues. O(n log n) sort cost ≈ negligible for n ≈ 550 memories.

---

## Results (3 trials, DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax)

| metric | baseline avg | T1 | T2 | T3 | 3-trial avg |
|--------|-------------|-----|-----|-----|-------------|
| **fitness** | **0.060** | **0.028** | **0.037** | **0.042** | **0.036** |
| transfer_score | 0.841 | 0.8803 | 0.8803 | 0.8803 | **0.880** |
| carrier_emergence | 0.936 | 0.9410 | 0.9410 | 0.9410 | **0.941** |
| xi_robustness_v2 | 0.816 avg | **0.990** | **0.929** | **0.893** | **0.937** |
| magic_proxy_phase_R | 0.871 | 0.755 | 0.755 | 0.755 | 0.755 |
| query_gravity | 0.374 | 0.363 | 0.363 | 0.363 | 0.363 |

Fitness improvement: 0.060 → 0.036 = **Δ −0.024** (4.8× above 0.005 threshold). ✓

Xi improvement: 0.816 avg → 0.937 avg = **Δ +0.121**, range narrowed from 0.225 → 0.097.

Transfer and carrier_e are byte-identical across all 3 trials. ✓

---

## Analysis

### Why it works

Content-string sorting places adversarials at deterministic positions regardless of their
random UUIDs. `"adv_l5_a1_xi_twin 0..9"`, `"adv_l5_a2_commutator 0..9"`,
`"adv_l5_a3_freq_attack 0..9"` each form compact groups in sorted order. Within each
group, inter-adversarial cosine similarity is low:
- A1s: vectors from xi-signature of different corpus centroids → orthogonal in xi space
- A2s: random large-magnitude vectors → low mutual similarity
- A3s: random low-magnitude vectors → low mutual similarity

No adversarial pairs pass the 0.6 similarity threshold → adversarials get no targeted
vector perturbation → corpus-adversarial cross-contamination eliminated → fitness_adv
tracks fitness_clean closely → high xi.

### Improved transfer from better pair selection

Corpus memories sorted by content string are within-category adjacent (`"dense_a 0..99"`
clustered together). Within-category pairs have HIGH cosine similarity (same dense cluster
centroid) → more pairs pass 0.6 threshold → more targeted perturbation applied → stronger
differentiation between clusters → improved transfer (0.841 → 0.880).

This is the same mechanism that makes phase-centroid chirality work: content-aligned
operations are more effective than UUID-random ones.

### Residual xi variance (0.893–0.990, range 0.097)

The remaining variance is much smaller than baseline (0.225) but not zero. Likely sources:
1. `stage_replay` still returns working_set in UUID order → other stages (stage_detect,
   stage_bundle, etc.) have mild order dependencies
2. Some stochastic element in the dream chain itself (quiescence threshold interaction
   with floating-point accumulation)

The remaining variance contributes only (1-0.893)*0.15 = 0.016 vs (1-0.727)*0.15 = 0.041
to fitness — so even worst-case xi now gives better fitness than baseline average.

### Fitness accounting

| component | Δ metric | weight | Δ fitness |
|-----------|----------|--------|-----------|
| xi_robustness_v2 | +0.121 | 0.15 | −0.018 |
| transfer_score | +0.039 | 0.15 | −0.006 |
| carrier_emergence | +0.005 | 0.10 | −0.001 |
| **net** | | | **−0.025** |

Predicted Δ −0.025 vs observed Δ −0.024. ✓

### magic_R change (0.871 → 0.755)

The phase-centroid handedness is unchanged. The magic_R decrease likely reflects that
the improved vector perturbation (better-targeted pairs) creates MORE phase separation
between clusters → lower global Kuramoto order parameter → lower R. Lower R means more
non-Clifford-like content (more quantum-magic-like phase structure). This is consistent
with improved transfer: stronger inter-cluster differentiation → better inter-engine
frequency transfer.

---

## Decision

**Code change KEPT. New empirical optimum:**

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.036
transfer_score=0.880, carrier_e=0.941, xi_robustness_v2=0.937 avg (range 0.893–0.990)
magic_R=0.755, query_gravity=0.363
```

---

## Updated open axes

| parameter | prediction | risk | priority |
|-----------|-----------|------|----------|
| **stage_replay sort** | Sorting working_set in stage_replay would propagate content-order to all dream stages; might further reduce xi variance | MEDIUM (affects all consolidation paths) | MEDIUM |
| xi residual variance | Source unclear; likely quiescence floating-point or other stage ordering | LOW (small impact) | LOW |
| stage_wire k_local | Low prior | LOW | LOW |
| destructive_penalty | Very low prior | LOW | LOW |
