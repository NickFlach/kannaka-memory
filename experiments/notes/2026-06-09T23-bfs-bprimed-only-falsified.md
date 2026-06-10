# BFS content-sort for engine_b_primed only — falsified, transfer regression

**Date:** 2026-06-09T23 UTC
**Branch:** kannaka-curiosity/2026-06-09T23-bfs-bprimed-only
**Code changes:** REVERTED — no net code change
**Status:** FALSIFIED — transfer regression, fitness 0.027→0.031

---

## Context: session orientation discovery

This fire began with the session pointer at d2ee426 (pre-T18/T20/T21). The actual
remote master was at 48cd8e8, already incorporating:
- T18: xi-engine depth=2 isolation (fitness 0.043→0.030)
- T20: adversarial post-dream filter (xi 0.681→0.805)
- T21: BFS sort revert in find_synchronized_clusters (xi 0.906→0.985, fitness 0.036→0.028)

Three orientation trials (consumed 3 of 5 budget runs) re-discovered T18's xi-engine
depth=2 result before recognizing the branch was stale. After pulling master, one
baseline run confirmed the actual current optimum.

**Actual current optimum (post-T18+T20+T21):**
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
fitness ≈ 0.027573 (1-trial baseline)
transfer=0.848, carrier_e=0.998, xi=0.987 (stable)
magic_R=0.877, query_gravity=0.374
```

Remaining fitness breakdown:
- transfer: 0.15 × (1 − 0.848) = **0.023** (83% of total)
- xi: 0.15 × (1 − 0.987) = 0.002 (7%)
- carrier_e: 0.10 × (1 − 0.998) = 0.0002 (1%)
- other: ~0.003 (9%)

Transfer is the overwhelmingly dominant remaining lever.

---

## Hypothesis

T21's BFS sort revert (removing content-sort from `find_synchronized_clusters`)
improved xi from 0.906→0.985 and carrier_e from 0.900→0.998 but dropped transfer
from 0.929→0.848. The transfer benefit of content-sort came from corpus memories
forming better-structured BFS clusters, which flows into stage_wire_cross_cluster
bridge creation and B memory integration.

The proposed fix: apply content-sort in `find_synchronized_clusters` ONLY when
DRIVE_CONTEXT == "engine_b_primed". Other engines (engine_clean, engine_adv)
keep UUID sort, preserving xi=0.985. Engine_a uses UUID sort (cluster topology
consistent with T21 baseline).

**Prediction:**
- transfer: ~0.90 (partial recovery toward T16-B's 0.929)
- xi: ~0.987 (unchanged — xi engines keep UUID sort)
- carrier_e: ~0.998 (unchanged — engine_flat keeps UUID sort)
- fitness: ~0.022–0.024 (improvement of ~0.005)

---

## Implementation

In `src/kuramoto.rs::find_synchronized_clusters`, conditionally sort by content
string when DRIVE_CONTEXT == "engine_b_primed" (before fingerprint computation,
so cache correctly separates per-sort results).

---

## Results (1 trial — budget constraint)

DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax (b_primed content-sort only)

| metric | baseline (T21+) | this trial | delta |
|--------|-----------------|------------|-------|
| fitness | **0.02757** | 0.03144 | **+0.004 REGRESSION** |
| transfer | 0.848 | 0.822 | **−0.026 WORSE** |
| xi | 0.987 | 0.987 | 0 |
| carrier_e | 0.998 | 0.998 | 0 |
| magic_R | 0.877 | 0.877 | 0 |
| query_gravity | 0.374 | 0.374 | 0 |

xi and carrier_e held exactly as predicted. Transfer got WORSE, not better.

---

## Why transfer got worse (post-mortem)

T16-B's transfer=0.929 benefit came from ALL engines using content-sort — engine_a
AND engine_b_primed formed clusters with the SAME sort order. This consistency matters:
engine_a's dream creates A's phase topology using content-sorted cluster structure.
When engine_b_primed subsequently integrates B memories, it inherits A's cluster
boundaries and builds on them. With engine_a using UUID-sorted clusters and
engine_b_primed using content-sorted clusters, the cluster topology is *inconsistent*
between the two engines. B memories try to integrate into A's UUID-structured network
using content-sorted cluster bridges → the mismatch is worse than both using UUID order
→ transfer 0.848→0.822.

---

## Correct next hypothesis: engine_a + engine_b_primed both content-sorted

Based on this result, the transfer recovery requires engine_a AND engine_b_primed
to BOTH use content-sort, while xi engines (clean/adv) and engine_flat keep UUID sort.

The mechanism for each engine:
- engine_a + engine_b_primed: content-sort → consistent cluster topology between A-primed
  network and B-primed integration → transfer recovery toward 0.929
- engine_clean + engine_adv: UUID sort → adversarials seed clusters last → xi stays at 0.987
- engine_flat: UUID sort → carrier_e stays at 0.998

Implementation: check `drive_ctx == "engine_a" || drive_ctx == "engine_b_primed"`.

Estimated gain:
- transfer: 0.848 → ~0.929: improvement = 0.15 × 0.081 = **0.012**
- fitness: 0.027 → ~0.015

This would be the highest remaining gain in the L5 system. Budget exceeded for this fire;
reserved as highest-priority hypothesis for next fire.

---

## Caveats

- Only 1 trial run (budget = 5 runs, 4 consumed on orientation/baseline before this)
- Code fully reverted; TSV row from trial is appended but notes it as FALSIFIED
- The transfer regression with b_primed-only sort is informative: cluster sort MUST
  be consistent between engine_a and engine_b_primed for transfer to benefit

---

## Decision

**REVERTED.** Transfer regression, not improvement. Fitness 0.031 > baseline 0.027.
