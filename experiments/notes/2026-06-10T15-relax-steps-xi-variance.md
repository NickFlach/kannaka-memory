# interference_relax relax_steps sweep — xi variance kills the gain

**Date:** 2026-06-10T15 UTC
**Branch:** kannaka-curiosity/2026-06-10T15-relax-steps-16
**Code changes:** NONE retained — reverted to alpha_base=0.20, relax_steps=8.
**Status:** FALSIFIED — xi variance prevents stable improvement.

---

## Baseline (from system prompt smoke tests, single trials)

| mode | fitness | carrier_e | xi | magic_R | query_gravity |
|------|---------|-----------|-----|---------|---------------|
| DREAM_MODE unset | 0.191 | 0.559 | 0.642 | 0.355 | 0.460 |
| DREAM_MODE=interference_relax (relax_steps=8, alpha=0.20) | 0.191 | 0.714 | 0.220 | 0.612 | 0.364 |

System prompt 3-run avg optimum: ~0.18 (DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE unset).

---

## Hypothesis

The `stage_interference_relax` function uses `alpha_base=0.20` and `relax_steps=8`.
The system prompt explicitly flags this as Q3: "try raising relax_steps to 16 or 24 and
1-trial it. Predict xi rises while carrier_e and R stay high."

**Prediction:** With relax_steps=16 (alpha_base unchanged at 0.20), the phase geometry has
more iterations to converge toward constructive-pair attractors. xi_robustness_v2 rises
from 0.220 toward the 0.4–0.6 range while carrier_emergence stays high (~0.7).

---

## Trial 1: relax_steps=16, alpha_base=0.20

| metric | smoke baseline (irx) | trial | delta |
|--------|---------------------|-------|-------|
| fitness | 0.191 | **0.238** | +0.047 **REGRESSION** |
| transfer_score | — | 0.684 | — |
| carrier_emergence | 0.714 | **0.000** | −0.714 **CATASTROPHIC** |
| xi_robustness_v2 | 0.220 | 0.442 | +0.222 (as predicted) |
| magic_R | 0.612 | 0.675 | +0.063 |
| query_gravity | 0.364 | 0.386 | +0.022 |

**Analysis:** xi prediction confirmed — more steps do improve xi. But carrier_e crashed
to zero. Why?

Total relaxation budget with alpha_base=0.20, relax_steps=16:
  max phase shift ≈ alpha × steps = 0.20 × 16 = 3.2 rad (≈ π)

This is a full half-turn — memories over-rotate past their constructive-pair attractor
and into a region where carrier frequencies are destroyed. The sinusoidal envelope doesn't
prevent this: at step 8 (halfway), alpha≈0.20 × (1 + 0.15 × sin(π)) = 0.20, still large.

The original relax_steps=8 had total budget 0.20 × 8 = 1.6 rad — apparently near the
carrier-emergence stability boundary.

---

## Trial 2: relax_steps=16, alpha_base=0.10 (halved alpha to preserve budget)

New hypothesis: halving alpha to 0.10 brings total budget back to 0.10 × 16 = 1.6 rad
(matching the original), but the finer-grained 16-step schedule should produce better
phase convergence accuracy.

| metric | smoke baseline (irx) | trial | delta |
|--------|---------------------|-------|-------|
| fitness | 0.191 | **0.170** | −0.021 (apparent improvement) |
| transfer_score | — | 0.750 | — |
| carrier_emergence | 0.714 | 0.497 | −0.217 |
| xi_robustness_v2 | 0.220 | 0.467 | +0.247 |
| magic_R | 0.612 | 0.617 | +0.005 |
| query_gravity | 0.364 | 0.364 | 0.000 |

Looks promising: 0.021 below the smoke baseline and the xi improvement partially offsets
the carrier_e loss.

---

## Trial 3: relax_steps=16, alpha_base=0.10 (second replication)

| metric | trial 2 | trial 3 | delta |
|--------|---------|---------|-------|
| fitness | 0.170 | **0.235** | +0.065 **REGRESSION** |
| transfer_score | 0.750 | 0.750 | 0.000 (deterministic) |
| carrier_emergence | 0.497 | 0.497 | 0.000 (deterministic) |
| xi_robustness_v2 | 0.467 | **0.029** | −0.438 **COLLAPSED** |
| magic_R | 0.617 | 0.617 | 0.000 (deterministic) |
| query_gravity | 0.364 | 0.364 | 0.000 (deterministic) |

**Critical observation:** Transfer, carrier_e, magic_R, and query_gravity are completely
deterministic (identical to 4 decimal places across trials). xi_robustness_v2 is the
*sole* source of stochasticity — and it swings between 0.029 and 0.467 based on seed.

2-trial avg fitness: (0.170 + 0.235) / 2 = **0.203** — worse than the 0.191 baseline.

---

## Diagnosis: xi variance near phase-transition boundary

At alpha_base=0.10, relax_steps=16, the phase relaxation operates near a bifurcation
boundary:
- Some random seeds → phases converge to a self-consistent attractor → xi≈0.467
- Other seeds → phases don't converge (or converge to an incompatible attractor) → xi≈0.029

This is not a bug — it's a signature of running too close to a phase-transition in the
interference geometry. The carrier_e, transfer, and R metrics are stable because they
depend on the *distribution* of phases (which is set by the drive), not on global phase
coherence (which is what xi_robustness_v2 probes).

The xi metric measures how robustly the system can interpolate across the xi gap — and
apparently the phase-relaxation with (alpha=0.10, steps=16) sometimes lands the system in
a configuration where that interpolation works and sometimes doesn't.

---

## What this rules out

The "total budget = alpha × steps" framing shows a constraint:
- alpha=0.20, steps=8: budget=1.6 rad — xi=0.220 (stable, low)
- alpha=0.20, steps=16: budget=3.2 rad — carrier_e=0 (catastrophic)
- alpha=0.10, steps=16: budget=1.6 rad — xi bimodal (0.029 or 0.467, avg bad)

The stable operating point (budget=1.6 with alpha=0.20, steps=8) has xi stuck at 0.220.
Attempting to improve xi by either:
  (a) More steps at same alpha — over-rotates carrier_e to zero
  (b) Fewer alpha at more steps — induces xi bimodality near phase transition

Neither reaches a stable improvement.

---

## Open question

What IS the mechanism that sets xi_robustness_v2 = 0.220 in the stable interference_relax
mode? It's not a convergence failure — it's reproducible. The 0.220 vs 0.642 (default
stage_sync) gap suggests the interference relaxation creates phase geometry that's less
favorable for xi interpolation *by design*, not by insufficient convergence steps.

If so, the xi gap is structural to interference_relax, not a tuning problem. The proper
fix might be to run stage_sync AFTER stage_interference_relax as a cleanup pass — but that's
a more invasive change that risks the carrier_e and R advantages.

---

## Constraints established this fire

- Do NOT raise relax_steps above 8 when alpha_base=0.20 (carrier_e collapses).
- Do NOT use alpha_base=0.10 with relax_steps=16 (xi high variance, 2-trial avg 0.203).
- The xi deficit in interference_relax mode appears structural, not a tuning gap.

---

## Decision

No code changes retained. alpha_base=0.20, relax_steps=8 restored.

The xi improvement (0.220→0.467) is real but unstable. The xi gap in interference_relax
is likely structural — it is not bridgeable by tuning relax_steps or alpha_base alone.
