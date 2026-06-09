# Working-set content-sort reveals transfer-xi-carrier tradeoff — neutral net fitness

**Date:** 2026-06-09T17 UTC
**Branch:** kannaka-curiosity/2026-06-09T17-revert-workingset-sort
**Code changes:** REVERTED — stage_replay content-sort restored
**Status:** FALSIFIED (fitness improvement < 0.005 threshold) — tradeoff documented

---

## Background

Current empirical optimum (post-T15):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
2-trial avg fitness ≈ 0.037
transfer_score=0.841 (stable), carrier_emergence=0.936, xi_robustness_v2=0.973 (stable)
magic_R=0.871, query_gravity=0.374
```

Fitness breakdown:
- transfer_score: 0.15 × (1 − 0.841) = **0.024** (64% of total fitness)
- xi_robustness_v2: 0.15 × (1 − 0.973) = 0.004 (11%)
- carrier_emergence: 0.10 × (1 − 0.936) = 0.006 (17%)
- other: ~0.003

Transfer is the dominant remaining lever.

---

## Hypothesis

The T13 fire added a content-sort of `working_set` in `stage_replay` (consolidation.rs).
The T13 notes claimed this was needed because `apply_targeted_chiral_perturbation` uses
a "sliding window over working_set." However, inspection of the current code shows that
`apply_targeted_chiral_perturbation` (lines 2026–2036) **re-sorts by content hash
internally** before any pair selection, making working_set order irrelevant for
chiral targeting.

Additionally, `TestMedium::all_memories()` already returns memories sorted by UUID.
Since all corpus/adversarial UUIDs are deterministic (corpus: `(i+1)*STRIDE`, adversarials:
`u128::MAX − k*stride` via T15), the pre-sort working_set is already fully deterministic
without any explicit content-sort.

T13 "Path 1 only" trials (BFS sort only, NO working_set sort) gave transfer=0.9477
with xi varying pre-T14/T15. With T14 (adversarial filtering) and T15 (det UUIDs)
providing xi stability via independent mechanisms, removing the working_set sort
should now recover transfer without destabilizing xi.

**Prediction:**
- Transfer: ~0.930–0.950 (recovery toward T13 Path-1-only level)
- Xi: ~0.965–0.973 (T14+T15 mechanisms maintain stability)
- Carrier_e: ~0.930–0.940 (mild change expected)
- Fitness: ~0.022–0.030 (significant improvement over 0.037)

---

## Implementation

One-line change: replaced the `sort_by_key(|(content, _)| *content)` in `stage_replay`
with a direct `m.id` map (UUID-sorted, which is what `all_memories()` already returns).

---

## Results

DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax (UUID-sorted working_set)

| trial | fitness | transfer | carrier_e | xi    | R     | query_gravity |
|-------|---------|----------|-----------|-------|-------|---------------|
| T1    | 0.036498 | 0.928729 | 0.9003   | 0.9057 | 0.8643 | 0.3733 |
| T2    | 0.036484 | 0.928729 | 0.9003   | 0.9057 | 0.8643 | 0.3733 |
| **avg** | **0.0365** | **0.929** | **0.900** | **0.906** | **0.864** | **0.373** |

Results are **fully deterministic** (byte-identical between T1 and T2 on all core metrics).
Xi stability confirmed: no variance, 0.906 stable.

---

## Comparison to baseline (content-sorted working_set)

| metric | content-sort (current) | UUID-sort (this fire) | delta |
|--------|------------------------|----------------------|-------|
| fitness avg | 0.0371 | **0.0365** | **−0.0006** |
| transfer | 0.841 | **0.929** | **+0.088** |
| xi | 0.973 | 0.906 | −0.067 |
| carrier_e | 0.936 | 0.900 | −0.036 |
| magic_R | 0.871 | 0.864 | −0.007 |
| query_gravity | 0.374 | 0.373 | −0.001 |

---

## Fitness impact decomposition

The content-sort in `stage_replay` makes a **near-zero net fitness trade**:
- Transfer gain: 0.15 × 0.088 = +0.013 fitness improvement
- Xi loss: 0.15 × 0.067 = −0.010 fitness regression
- Carrier_e loss: 0.10 × 0.036 = −0.004 fitness regression
- **Net: −0.001 fitness** (i.e., 0.001 improvement without sort)

This is well below the 0.005 improvement threshold. The content-sort is essentially
a **neutral tradeoff**: it sacrifices transfer to gain xi + carrier_e.

---

## Why transfer improves with UUID-sort

UUID-sort (= insertion order for corpus A, then B) puts all A memories before all B
memories in the B-primed dream engine's working_set. Content-sort partially interleaves
them (B's sparse memories sort before A's sparse memories alphabetically: "l5b_sparse_e"
< "sparse_e"). This interleaving in content-sort causes A and B memories to compete
differently in sequential stages (stage_xi_repulsion, stage_hallucinate), with cascading
effects on chain_fidelity in the B-primed dream → lower fitness_B_primed → lower transfer.

---

## Why xi/carrier_e are better with content-sort

The content-sort groups same-category memories closer together (within-category
lexicographic order clusters "dense_a 0/1/10/11..." etc.). This grouping affects:
1. **carrier_e**: `engine_flat` uses UUID-sort too; the flat corpus dream benefits from
   category-grouped working_set, producing cleaner 2 Hz carrier emergence.
2. **xi**: the adversarial pass dream (in `eval_xi_robustness_v2`) processes adversarials
   FIRST in content-sort ("adv_l5_..." < "dense_..."). UUID-sort puts adversarials last
   (`u128::MAX − k*stride` > corpus UUIDs). This changes which sequential stage updates
   adversarials experience, altering fitness_adv slightly and reducing xi by 0.067.

---

## Mechanism of current xi stability (clarification)

The T13 notes claimed the working_set sort was needed for xi stability via
`apply_targeted_chiral_perturbation`'s sliding window. This is **incorrect**:
`apply_targeted_chiral_perturbation` re-sorts by content hash internally (lines 2026–2036).
The actual xi stability comes from:
1. **T14**: filters adversarials from `original_ids` in `run_l5_dream_chain`
2. **T15**: adversarial UUIDs deterministic near `u128::MAX` — but since `all_memories()`
   sorts by UUID, this actually puts adversarials LAST in UUID-sorted order (not relevant
   to content-sort BFS), affecting amplitude-tie-breaking in `compute_chain_seed` and
   sequential ordering in other UUID-order-dependent operations.
3. **T13 Path 1**: BFS in `find_synchronized_clusters` (kuramoto.rs) content-sorts —
   adversarials sort FIRST in adv pass ("adv_l5_" < "dense_"), but this is consistent
   across all runs → no per-run xi variance.

The working_set content-sort (T13 Path 2) is NOT needed for xi stability. Removing it
gives xi=0.906 (stable, not varying) while T13 Path 1 + T14 + T15 maintain determinism.

---

## Dual-engine sort attempt (also tried, also reverted)

After establishing the tradeoff, a second approach was attempted: use UUID-sort ONLY for
`engine_b_primed` (via `DRIVE_CONTEXT == "engine_b_primed"` check in `stage_replay`), while
keeping content-sort for all other engines. Prediction: transfer recovers to 0.929 while
xi and carrier_e stay at baseline (xi eval and engine_flat would still use content-sort).

**Result:** Catastrophic — fitness 0.114, transfer=0.578, xi=0.681, carrier_e=0.998. The
hypothesis that "only engine_b_primed is affected" was wrong. Carrier_e improved (0.936→0.998)
while transfer and xi collapsed, suggesting the engine_b_primed UUID-sort interacted with the
cluster cache or some other shared process state in ways that affected the xi evaluation. The
failure mode is not fully understood but the result is clear: the dual-engine approach is NOT
a safe improvement.

Both changes fully reverted. No net code change from this fire.

---

## Decision

**Code change REVERTED.** Net fitness improvement (0.0006) < 0.005 threshold.
**Dual-engine approach also REVERTED** — catastrophic failure, cause unclear.

The transfer–xi–carrier_e tradeoff is essentially neutral. Neither ordering is strictly
better in fitness terms. The dual-engine approach produced unexpected interactions between
the UUID-sorted engine_b_primed dream and the xi evaluation passes.

---

## Open axes this finding reveals

| axis | description | priority |
|------|-------------|----------|
| Break the tradeoff | Find working_set order that improves transfer WITHOUT hurting xi/carrier_e | HIGH |
| Cluster-phase sort | Sort working_set by phase sextant — might preserve xi while helping transfer | MEDIUM |
| Dual-engine sort | Use content-sort for engine_a/flat/xi-eval, UUID-sort for engine_b_primed only | HIGH |
| Transfer-dedicated fix | Make B-primed dream more effective without changing working_set order | MEDIUM |

### Highest-priority next fire: dual-engine sort

The key insight: the SAME working_set sort is used for ALL engines
(engine_a, engine_b_primed, engine_b_naive, engine_flat, engine_clean, engine_adv).
Content-sort helps xi (via engine_adv ordering in eval_xi_robustness_v2) and carrier_e
(via engine_flat ordering). UUID-sort helps transfer (via engine_b_primed ordering).

**If we could use content-sort for xi/carrier_e engines and UUID-sort for B-primed,
we might recover both high transfer AND high xi/carrier_e.**

This would require adding a runtime flag or a new sort mode to `stage_replay`, gated
on `std::env::var("DRIVE_CONTEXT")` (already available: "engine_b_primed", "engine_flat",
"engine_adv", "engine_clean", "engine_b_naive"). Estimated fitness improvement:
0.15 × 0.088 (full transfer recovery) ≈ **0.013 fitness points** if xi/carrier_e hold.
