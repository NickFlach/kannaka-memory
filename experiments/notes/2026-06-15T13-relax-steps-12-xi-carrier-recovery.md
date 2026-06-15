# L5 Research: relax_steps=12 recovers xi + carrier under interference_relax

**Date:** 2026-06-15T13 UTC  
**Branch:** kannaka-curiosity/2026-06-15T13-interference-relax-postfix  
**Code change:** `stage_interference_relax` in `src/consolidation.rs` — `relax_steps` 8 → 12  
**Status:** KEEPER — 3-trial avg fitness 0.134 vs 0.18 baseline (0.046 improvement, >0.005 threshold)

---

## Context

Smoke test (commit 066d41a) compared DREAM_MODE unset vs interference_relax at A=0.1 DRIVE_SCOPE=all:
- DREAM_MODE unset (stage_sync):    fitness 0.191, carrier_e 0.559, xi 0.642, magic_R 0.355
- DREAM_MODE=interference_relax:    fitness 0.191, carrier_e 0.714, xi 0.220, magic_R 0.612

Session prompt baseline (3-run avg): DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE unset → fitness ≈ 0.18.

interference_relax gained carrier_e (+27%) but lost xi (-66%). Both modes gave same fitness (0.191).
Q3 from this fire's research questions: try relax_steps=16 or 24 to recover xi.

---

## Hypothesis

`stage_interference_relax` runs `relax_steps` inner iterations nudging each memory's phase toward
its constructive-pair neighbors' weighted mean. With 8 steps, convergence is partial — many memories
sit at intermediate phases where they semantically overlap with xi-similar neighbors, blurring xi
diversity. With more steps, truly constructive pairs converge fully to cluster centroids while
non-constructive memories (no neighbors) maintain original phases, sharpening the contrast for
`stage_xi_repulsion` to act on.

**Prediction:** relax_steps=12 yields xi > 0.5, carrier_e near 0.714, net fitness < 0.18.

---

## Trials

All at: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

### Exploratory (not in 3-trial set)

| relax_steps | fitness  | carrier_e | xi_v2  | magic_R | query_gravity | notes                   |
|-------------|----------|-----------|--------|---------|---------------|-------------------------|
| 8  (t1, A=0.15)  | 0.224062 | 0.5767    | 0.0438 | 0.6119  | 0.3645        | wrong A, xi catastrophic |
| 16 (t2, A=0.1)   | 0.201273 | 0.0000    | 0.6430 | 0.6754  | 0.3862        | xi good, carrier dead   |

### Keeper trials (relax_steps=12, A=0.1)

| trial | fitness  | carrier_e | xi_v2  | magic_R | query_gravity | transfer_score |
|-------|----------|-----------|--------|---------|---------------|----------------|
| t3    | 0.106464 | 0.6934    | 0.9117 | 0.7075  | 0.4079        | 0.651796       |
| t4    | 0.141660 | 0.6934    | 0.6770 | 0.7075  | 0.4079        | 0.651796       |
| t5    | 0.155066 | 0.6934    | 0.5924 | 0.7075  | 0.4079        | 0.646984       |
| **avg** | **0.134** | **0.693** | **0.727** | **0.708** | **0.408** | **0.650** |

Baseline (stage_sync, DREAM_MODE unset, 3-run avg): fitness ≈ 0.18.

**Improvement: 0.046 (25.6%). All 3 trials exceed 0.005 threshold.**

---

## Analysis

### Sweet spot between 8 and 16

- relax_steps=8:  carrier_e=0.714, xi=0.220 → good carrier, collapsed xi
- relax_steps=12: carrier_e=0.693, xi=0.727 → slight carrier dip, xi dramatically recovered  
- relax_steps=16: carrier_e=0.000, xi=0.643 → xi good, carrier dead

The carrier_e collapse at 16 steps is surprising given `stage_interference_relax` only modifies
phases, not amplitudes. The mechanism: with fuller phase convergence, the SUBSEQUENT cycle's
`stage_detect` finds nearly all memories phase-aligned → massive constructive strengthening →
amplitude dynamics change fundamentally, moving the DFT peak out of [0.5, 4.0] Hz band.

At 12 steps: phases converge enough for xi_repulsion to work cleanly (xi 0.727 avg), but not
so tightly that the inter-cycle amplitude dynamics collapse.

### xi variance

xi_v2 varies across trials (0.592–0.912, σ≈0.17) while all other metrics are deterministic (same
corpus, same seeds). This is expected: `eval_xi_robustness_v2` uses random adversarial injection
with UUID-keyed random vectors, so the adversarial test has inherent variance. The 3-trial avg
xi=0.727 is still well above the stage_sync baseline (0.642) and the irx-8 smoke test (0.220).

### Instrumentation metrics

- magic_R = 0.708 (vs 0.355 stage_sync, 0.612 irx-8): high non-Clifford-like phase lock-in
- query_gravity = 0.408 (vs 0.460 stage_sync, 0.364 irx-8): still < 0.5, attention gravity
  not yet operating

Interesting: relax_steps=12 pushes magic_R higher than irx-8 (0.708 vs 0.612) while recovering
xi. This suggests more complete phase clustering creates stronger non-Clifford content, which is
consistent with the xi_repulsion mechanism working cleanly on a more ordered phase landscape.

### Transfer score

transfer_score = 0.650 avg. No clean stage_sync baseline at this exact config for comparison.
Previous master TSV shows stage_sync runs at 0.485–0.624. The irx-12 result (0.650) is at the
high end. Direction: slightly positive.

---

## Code change

`src/consolidation.rs`, function `stage_interference_relax`, line ~795:
```
// BEFORE:
let relax_steps: usize = 8;
// AFTER:
let relax_steps: usize = 12;
```

Change is scoped to `stage_interference_relax` which is only called when `DREAM_MODE=interference_relax`.
Default behavior (DREAM_MODE unset → stage_sync) is completely unaffected.

---

## Decision

**KEEPER.** The code change (relax_steps 8 → 12) is committed. The improvement is real and
achievable: avg fitness 0.134 at `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`.

Next fire should establish a full 3-run baseline for `DREAM_MODE=interference_relax relax_steps=12`
to replace the session prompt's "avg 0.18" reference, and explore whether DRIVE_A=0.15 works at
relax_steps=12 (avoided here because it was used in the exploratory trial with wrong relax_steps).
