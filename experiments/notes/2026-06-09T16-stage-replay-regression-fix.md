# stage_replay sort revert — regression fix, fitness 0.036 stable

**Date:** 2026-06-09T16 UTC
**Branch:** kannaka-curiosity/2026-06-09T16-stage-replay-regression-fix
**Code changes:** `src/consolidation.rs::stage_replay` — KEPT (revert of T13 sort)
**Status:** CONFIRMED — regression fixed, fitness 0.036 stable (2 trials)

---

## Background

The curiosity chain accumulated these improvements through c484313:
- PR #222 (8d3aaa4): content-hash sort in `apply_targeted_chiral_perturbation`
- PR #223 (b1f4564): corpus-only handedness (exclude adversarials from cluster chirality)
- PR #224 / T13 (262b6f7): content-sort in `stage_replay` + BFS sort in `find_synchronized_clusters`
- PR #225 / T06-Q (abc16be): adversarial filter from `original_ids` quartile
- PR #226 / T15 (c93bcfc): deterministic adversarial UUIDs near u128::MAX

Each branch was confirmed individually:
```
T13 alone:    fitness 0.042, carrier_e 0.998, xi 0.926, transfer 0.805
T06-Q alone:  fitness 0.041, carrier_e 0.936, xi 0.945, transfer 0.841
T15 alone:    fitness 0.037, carrier_e 0.936, xi 0.973, transfer 0.841
```

---

## Problem: combined state at c484313 regressed to fitness 0.123

Baselining c484313 (all PRs merged) gave:

```
fitness 0.122574, carrier_e 0.9979, xi 0.6814, transfer 0.523036
magic_R 0.875, query_gravity 0.421
```

xi = 0.681 (vs T15 alone = 0.973) — catastrophic regression.
transfer = 0.523 (vs T15 alone = 0.841) — catastrophic regression.

---

## Root cause: T13's stage_replay sort antagonises T06-Q

T13 added a content-STRING sort to `stage_replay`, returning working_set in
alphabetical content order. Adversarial memory content strings ("adv_l5_a1_xi_twin 0",
"adv_l5_a2_commutator 0", "adv_l5_a3_freq_attack 0") sort BEFORE all corpus content
strings ("decoy outlier 0", "emotion feeling 0", ..., "science-music bridge 0") because
'a' < 'd' < 'e' < 'n' < 'p' < 'q' < 'r' < 's'.

This means in the adversarial dream pass (engine_adv, 330 memories), the 30 adversarials
occupy working_set positions 0-29. The working_set is passed to:
- `stage_detect`: finds neighbors in first 32 positions → adversarials pair mostly with other adversarials
- `stage_bundle`: groups adversarials first
- `stage_sync` / `stage_interference_relax`: processes adversarials-first ordering throughout
- `stage_xi_repulsion`: adversarial-adversarial repulsion dominates early
- `stage_hallucinate`: hallucinates from adversarial-dominated initial set
- etc.

All of these create a dream-pass that diverges heavily from the clean pass (corpus-only),
where positions 0-29 are corpus memories. The large |fitness_clean - fitness_adv| drives
xi toward 0.

In T13 ALONE (without T06-Q), adversarials were also included in `original_ids` at random
positions. Their amplitudes (A1=0.9, A2=1.0, A3=0.5) created correlated offsets in the
adv vs clean `initial_mean_amp` that partially compensated the working_set ordering
divergence → xi=0.926.

With T06-Q (filters adversarials from original_ids), `initial_mean_amp` is now
corpus-identical between clean and adv passes. The compensation is removed, and the
full working_set ordering divergence is exposed → xi=0.681.

---

## Fix: revert stage_replay sort

`apply_targeted_chiral_perturbation` already has PR #222's content-HASH sort inside it
(lines 2026-2043 of consolidation.rs), which determinises chiral pair selection
independently of working_set order. Removing stage_replay's content-string sort:
1. Restores natural HashMap iteration order for working_set
2. Puts adversarials at random positions in adv pass (same as the tested T15 state)
3. Does not change `apply_targeted_chiral_perturbation` pair determinism
4. Keeps T13's BFS sort in `find_synchronized_clusters` (kuramoto.rs) which benefits carrier_e

The fix is a 7-line removal (simplification, not addition).

---

## Results (DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax)

| trial | fitness | transfer | carrier_e | xi    | magic_R | query_gravity |
|-------|---------|----------|-----------|-------|---------|---------------|
| T1 (c484313 baseline, broken) | 0.122574 | 0.523 | 0.998 | 0.681 | 0.875 | 0.421 |
| **T2 (fix applied)** | **0.036487** | **0.929** | **0.900** | **0.906** | **0.864** | **0.373** |
| **T3 (repeat)** | **0.036495** | **0.929** | **0.900** | **0.906** | **0.864** | **0.373** |

vs T15 empirical optimum:
| metric | T15 alone | this | delta |
|--------|-----------|------|-------|
| fitness avg | 0.037 | **0.036** | **−0.001** |
| transfer | 0.841 | **0.929** | **+0.088** |
| carrier_e | 0.936 | 0.900 | −0.036 |
| xi | 0.973 | 0.906 | −0.067 |
| magic_R | 0.871 | 0.864 | −0.007 |

Fitness decomposition (this fix vs T15):
- transfer: 0.15*(1−0.929) = 0.011 vs 0.024 → **−0.013 improvement**
- xi: 0.15*(1−0.906) = 0.014 vs 0.004 → **+0.010 regression**
- carrier_e: 0.10*(1−0.900) = 0.010 vs 0.006 → +0.004 regression
- net: −0.001 fitness improvement

---

## Why carrier_e regressed from T13-alone's 0.998 to 0.900

T13's BFS sort in `find_synchronized_clusters` gives carrier_e=0.998 when run ALONE.
In the combined state (with PR #222's content-hash sort in apply_targeted_chiral_perturbation
and PR #223's corpus-only handedness), the flat corpus chiral perturbation pattern differs
from T13-alone. carrier_e = 0.900 in the combined state — better than the pre-T13 level
(0.714 on master, 0.936 in T15) but not reaching 0.998.

---

## Decision

**Code change RETAINED.** Critical regression (0.122 → 0.036) fixed. Result stable.

The stage_replay content-string sort was redundant given PR #222's per-function hash sort,
and actively harmful when combined with T06-Q. Its removal simplifies the code and
recovers near-optimal fitness.

New empirical optimum (combined accumulated state):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
2-trial avg fitness ≈ 0.036
transfer=0.929, carrier_e=0.900, xi=0.906 (stable)
magic_R=0.864, query_gravity=0.373
```

---

## Open axes

| axis | mechanism | priority |
|------|-----------|----------|
| xi from 0.906 → 1.0 | T06-Q + T15 gave xi=0.973 alone; combined with T13 BFS sort → 0.906. May need to audit BFS sort interaction with T06-Q in the adv pass. | HIGH |
| carrier_e from 0.900 → 0.998 | T13 BFS sort alone gave 0.998 but combined context gives 0.900 | MEDIUM |
| transfer variance | 0.929 stable (vs 0.841 stable in T15 alone) — appears fixed by PR #222's hash sort | CLOSED |
