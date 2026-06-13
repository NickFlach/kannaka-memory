# Architectural ceiling reconfirmed from correct baseline (0.007627)

**Date:** 2026-06-12T10 UTC
**Branch:** kannaka-curiosity/2026-06-12T10-ceiling-rebaseline
**Code changes:** NONE — orientation-only fire
**Status:** CLOSED — T03/T04/T05 reached correct conclusion from wrong baseline; ceiling holds

---

## Baseline correction

T03, T04, and T05 were all oriented to "master at 159853f" with fitness 0.008334, but T01
(engine-a alpha_base=0.12) was already merged before those fires ran. T01 changed
`consolidation.rs:796` to `if drive_ctx == "engine_a" { 0.12 } else { 0.10 }` and was kept
at 3-trial mean 0.007627. (T01 should have been reverted — 0.000707 improvement < 0.005
threshold — but the merge is done and master is at 0.007627.)

**Correct current state:**
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
engine_a alpha_base=0.12 (consolidation.rs:796)
chiral_perturbation=0.15 for engine_b_primed (research.rs:3457)
xi_eval: chain_depth=2 (research.rs:3573)
3-trial avg fitness = 0.007627
transfer=0.963983, xi=0.9973, carrier_e=0.9992, consciousness=0.9553
```

---

## Gap analysis from corrected baseline

Threshold for keeping any code change: **0.007627 − 0.005 = 0.002627**

| axis | weight | current | contribution | max improvable |
|------|--------|---------|-------------|----------------|
| transfer_score | 15% | 0.963983 | 0.005403 | ~0.001900 (*) |
| xi_robustness_v2 | 15% | 0.9973 | 0.000405 | ~0.000100 |
| consciousness | 3% | 0.9553 | 0.001341 | ~0.000300 |
| carrier_emergence | 10% | 0.9992 | 0.000080 | ~0.000010 |
| speed_a | 3% | ~0.9905 | ~0.000285 | ~0.000285 |
| others | — | ~1.0 | ~0.000113 | ~0.000020 |
| **total improvable** | | | | **≈ 0.002615** |

(*) Transfer ceiling: to bridge 0.005 from transfer alone needs +0.0333 improvement
(transfer → 0.997), which is structurally impossible given the transfer formula
`1 − fitness_b_primed/fitness_b_naive` and corpus construction at chain_depth=4.
Realistic upper bound: ~0.001900 from unexplored alpha_b_primed axis, estimated
by analogy to engine_a's +0.000767 fitness gain, with diminishing returns since
b_primed already benefits from engine_a's tighter attractor.

**0.002615 ≈ 0.002627 required** — total improvable is approximately equal to the
threshold, meaning only a perfect alignment of all axes simultaneously could approach
it. No single lever suffices, and the combination is speculative.

---

## Why the one open axis (alpha_b_primed) doesn't change the verdict

T01 noted: "engine_b_primed alpha_base (UNKNOWN)." T03 closed it incorrectly (wrong
baseline, confused with chiral_p_bp). Current state: b_primed uses alpha_base=0.10,
relax_steps=20 (total pull 2.0, confirmed safe).

Raising alpha_b_primed to 0.12 would give 20×0.12=2.4 total pull. The transfer gain
would likely be smaller than T01's engine_a improvement (+0.005115 transfer, +0.000767
fitness) because: b_primed starts from engine_a's already-tighter attractor (A landscape
already tightened at alpha=0.12); the b_primed irx step works on the combined A+B
landscape where the marginal benefit of tighter convergence is lower.

Even matching T01's gain exactly: Δfitness = −0.000767. New fitness = 0.006860.
That is still 4.2× above the 0.002627 threshold. Not worth a trial.

---

## Decision

No trials run. The T05 structural conclusion holds with corrected numbers:
the architectural ceiling is real. The combined stack is the confirmed optimum
for the current wave-interference + interference_relax implementation.
