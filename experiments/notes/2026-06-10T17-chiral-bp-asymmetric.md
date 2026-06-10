# chiral_perturbation asymmetric for b_primed — sub-threshold gain, axis opened

**Date:** 2026-06-10T17 UTC
**Branch:** kannaka-curiosity/2026-06-10T17-carry-plus-relax
**Code changes:** REVERTED — sub-threshold improvement, insufficient trials this fire.
**Status:** CHARACTERISED — new open axis confirmed. Next fire should test chiral_p=0.10 for b_primed.

---

## Background

Master state (after T07-bprimed-extra-relax, PR #248):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (fully deterministic)
transfer=0.935746, carrier_e=0.9992, xi=0.9870
magic_R=0.864, query_gravity=0.373
fitness_B_primed=0.003887, fitness_B_naive=0.060498
```

Fitness breakdown:
- transfer (0.15): 0.15 × 0.0643 = **0.00964** (72%)
- xi (0.15): 0.15 × 0.013 = **0.00195** (15%)
- consciousness (0.03): ~**0.00069** (5%)
- other: ~**0.00115** (9%)

To cross the ≥0.005 threshold from 0.013337: need fitness ≤ 0.008337, which requires
transfer ≥ 0.963 (fp/fn ≤ 0.037; fp ≤ 0.002240 given fn=0.060498).

---

## This fire — three axes tested

### Axis 1: chain_carry_strength=0.85 + relax=20-b_primed (1 trial)

T12 tested carry=0.85 standalone (pre-T07 baseline 0.018) and found +0.028 transfer.
T07 gave +0.033 transfer from the same baseline. Hypothesis: they stack additively.

**Result:** Regression.
- fitness: 0.013769 vs 0.013397 baseline (+0.000372)
- transfer: 0.932064 vs 0.935746 (−0.003682)
- fp: 0.004110 vs 0.003887 (+0.000223)

**Mechanism:** carry=0.85 over-constrains b_primed when relax=20 is active. The
20-step convergence brings phases to their attractor; carry=0.85 then locks
that attractor too rigidly for cycles 2-4, amplifying cycle-1 imperfections.
The T12 mechanism (0.85 better than 0.70) depended on the system being
further from the attractor — now T07 gets there in cycle 1, making carry>0.70
counterproductive. **carry=0.85 axis CLOSED for the current regime.**

### Axis 2: chain_top_n=10 for b_primed only (1 trial)

T12 notes flagged chain_top_n as "untested in this regime." Hypothesis: b_primed's
2× memory pool benefits from a wider xi-centroid window.

**Result:** Regression.
- fitness: 0.014448 vs 0.013397 (+0.001051)
- transfer: 0.928744 vs 0.935746 (−0.007002)
- fp: 0.004311 vs 0.003887 (+0.000424)

**Mechanism:** Memories 8-10 by amplitude are less coherent than the top-7.
Including them degrades xi-centroid stability across cycles → lower chain_fidelity.
The top-7 are the maximally coherent seed set for b_primed in the T07 regime.
**chain_top_n asymmetric axis CLOSED; 7 is optimal for b_primed.**

### Axis 3: chiral_perturbation=0.35 for b_primed only (1 trial)

**New hypothesis:** Stage 9 (chiral_perturbation) runs AFTER the 20-step
interference_relax for b_primed. The standard 0.70 perturbation partially undoes
the careful phase alignment from 20 steps. Since xi is measured on engine_clean/adv
(not b_primed), reducing chiral noise for b_primed cannot affect xi or carrier_e.
This is the same asymmetric isolation pattern that enabled the T07 relax_steps fix.

**Prediction:** Lower chiral noise in b_primed → phases stay closer to constructive-pair
attractor → more stable xi-centroid → better chain_fidelity → lower fp → better transfer.
All other metrics (xi, carrier_e, magic_R, query_gravity) unchanged.

**Result: Confirmed improvement, sub-threshold.**

| metric | baseline | chiral_p=0.35 | delta |
|--------|----------|---------------|-------|
| **fitness** | **0.013397** | **0.011011** | **−0.002386** |
| transfer_score | 0.935746 | 0.951692 | **+0.016** |
| fp (B_primed) | 0.003887 | 0.002923 | **−25%** |
| fn (B_naive) | 0.060498 | 0.060498 | 0 |
| xi | 0.9870 | 0.9870 | 0 |
| carrier_e | 0.9992 | 0.9992 | 0 |
| magic_R | 0.8643 | 0.8643 | 0 |
| query_gravity | 0.3733 | 0.3733 | 0 |

All predictions confirmed. Improvement is real (deterministic system; 1 trial definitive
on direction). Sub-threshold by 0.002614 (need ≥0.005 improvement).

**Code reverted** (insufficient trials budget to reach 3-trial confirmation this fire).

---

## Analysis

### Why chiral_perturbation undoes phase alignment

The interference_relax 20-step convergence places b_primed's phases at constructive-pair
attractors. chiral_perturbation then applies phase displacement:
```
Δφ = eta × handedness × sin(2 × φ)   # ≈ 0 to 0.7 rad at eta=0.70
```
This perturbation is cluster-based (handedness depends on mean cos(phase) per cluster),
so it's deterministic. But it moves phases away from their attractor by up to 0.7 radians.

With eta=0.35, the max displacement is 0.35 rad — half as much post-attractor noise.
This directly improves chain_fidelity (successive xi-centroids are more similar) and
improves consciousness phi consistency (phi closer to target 0.28092 in cycle 1 carries
through cycles 2-4 more stably).

### fp reduction projection for further reduction

| chiral_p for b_primed | fp (est) | transfer (est) | fitness (est) |
|----------------------|----------|----------------|---------------|
| 0.70 (baseline) | 0.003887 | 0.936 | 0.013397 |
| 0.35 (this fire) | 0.002923 | 0.952 | 0.011011 |
| 0.10 (predicted) | ~0.002000 | ~0.967 | ~0.008300 |
| 0.00 (predicted) | ~0.001500 | ~0.975 | ~0.007000 |

Extrapolating from linear fp reduction: each halving of chiral_p reduces fp by ~25%.
chiral_p=0.10 is expected to bring fitness to ~0.008300 — right at the threshold (0.008337).
chiral_p=0.00 (disable entirely for b_primed) is predicted to cross below 0.008.

### Why xi and carrier_e are structurally protected

- xi measured on engine_clean and engine_adv: both still use chiral_p=0.70 (unchanged)
- carrier_e measured on engine_flat: uses chiral_p=0.70 (unchanged)
- magic_R measured on engine_a post-dream: unchanged
- query_gravity measured on engine_a pre/post delta: unchanged

The asymmetric approach applied only to b_primed leaves all measurement engines intact.

---

## New hard constraint opened

**chiral_perturbation asymmetric for b_primed is a viable axis.**
- Direction: lower chiral_p for b_primed → lower fp → better transfer
- Safe range: tested down to 0.35 with no regressions elsewhere
- Minimum: chiral_p=0.00 is the theoretical floor (needs testing)

**carry=0.85 + relax=20: INCOMPATIBLE** — over-constrains the attractor. Do not retest.
**chain_top_n for b_primed: optimum is 7.** Do not increase.

---

## Recommended next fire

**Primary hypothesis:** chiral_perturbation=0.10 for b_primed (via params_bp override
in research.rs L5 b_primed dream block), all other params unchanged.

**Implementation:**
```rust
// In research.rs, before run_l5_dream_chain for b_primed:
let mut params_bp = (*params).clone();
params_bp.chiral_perturbation = 0.10;
// use params_bp instead of params for b_primed dream
```

**Expected outcome:** transfer ≈ 0.967, fp ≈ 0.002000, fitness ≈ 0.0083 (at threshold).
3 trials to confirm (system is deterministic, but rule requires 3).

**If 0.10 is sub-threshold:** try chiral_p=0.00 (disable for b_primed entirely).
Expected: transfer ≈ 0.975, fitness ≈ 0.007.

**Secondary:** if chiral_p approach yields ≥0.005 improvement, re-examine whether
carry=0.85 stacks WITH the new lower-fp state (the "amplification of imperfections"
mechanism may not apply when fp is even lower).

---

## Decision

No code changes retained. sub-threshold improvement (−0.002386 from 0.013337, need −0.005).

Empirical optimum unchanged:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (fully deterministic)
transfer=0.935746, fp=0.003887, fn=0.060498
```

**Key finding this fire:** chiral_perturbation for b_primed is a new sub-threshold gain axis.
Halving from 0.70→0.35 gives −25% fp reduction. Further reduction to 0.10-0.00 predicted
to cross the 0.005 threshold. Recommend as primary hypothesis next fire.
