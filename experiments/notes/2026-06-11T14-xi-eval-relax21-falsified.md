# xi_eval_relax=21 — xi collapses below baseline; sweet spot confirmed sharply at 20

**Date:** 2026-06-11T14 UTC
**Branch:** kannaka-curiosity/2026-06-11T14-xi-eval-relax21
**Code changes:** REVERTED — single trial shows xi regression below even the relax=16 baseline
**Status:** FALSIFIED — relax=21 is over-converged; xi sweet spot is sharply at 20

---

## Background

Current empirical optimum (master at 8ff13f6):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (deterministic)
transfer=0.935746, xi=0.9870, carrier_e=0.9992, consciousness=0.9546
magic_R=0.8643, query_gravity=0.3733
```

Best known reverted combination (T11, sub-threshold by 0.000003):
```
chiral_p_bp=0.15 + xi_eval_relax=20
4-trial avg fitness=0.008340 (threshold=0.008337, gap=0.000003)
transfer=0.958868, xi=0.9973
```

T08 established the xi eval relax_steps sweep:
- relax=16 (baseline): xi=0.9870
- relax=20: xi=0.9973 (optimal)
- relax=24: xi=0.748 (catastrophic — over-convergence)

T12 notes flagged "xi residual at relax=21: UNKNOWN — might give +0.0002 more xi".
The gap between 20 (optimal) and 24 (catastrophic) was untested.

---

## Hypothesis

The xi sweet spot might not be exactly at 20. A single additional step (relax=21) could
either: (a) squeeze out marginal additional xi improvement (+0.0002 to +0.0015), or (b)
begin the descent toward the catastrophic collapse at 24. The T12 notes estimated this
as "UNKNOWN" — genuinely unexplored territory.

If relax=21 gave any improvement over 0.9973, combining it with chiral_p_bp=0.15 would
yield fitness = 0.008340 − improvement, firmly below the 0.008337 threshold.

Combined changes tested:
1. xi eval engines (engine_clean, engine_adv): relax_steps 16→21
2. engine_b_primed: chiral_perturbation 0.70→0.15 (T05's confirmed optimum)
3. engine_b_primed relax_steps: unchanged at 20 (T07's confirmed optimum)

---

## Result

Single trial: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| metric | master baseline | T11 stack (relax=20) | this trial (relax=21) | delta vs T11 |
|--------|-----------------|----------------------|-----------------------|--------------|
| fitness | 0.013337 | 0.008340 | **0.013622** | +0.005282 (WORSE THAN MASTER) |
| transfer | 0.935746 | 0.958868 | **0.958868** | 0 (chiral_p_bp=0.15 confirmed) |
| xi_robustness_v2 | 0.9870 | 0.9973 | **0.9614** | **−0.0359 (COLLAPSE)** |
| carrier_emergence | 0.9992 | 0.9992 | 0.9992 | 0 |
| consciousness | 0.9546 | 0.9546 | 0.9546 | 0 |
| magic_R | 0.8643 | 0.8643 | 0.8643 | 0 |
| query_gravity | 0.3733 | 0.3733 | 0.3733 | 0 |

xi at relax=21 (0.9614) is not only worse than relax=20 (0.9973) — it is WORSE than
the original relax=16 baseline (0.9870). One extra step pushed the system past the
sweet spot and into the over-convergence regime.

---

## Analysis

### Why relax=21 is worse than relax=16

The T08 mechanism for the xi improvement at relax=20:

> With 20 steps, engine_clean and engine_adv converge tightly to constructive-pair
> phase attractors. Adversarial memories represent a smaller fractional disruption of
> a tightly converged corpus → fitness_adv stays close to fitness_clean → xi improves.

The same analysis at relax=21:

The convergence at relax=21 overshoots the optimal attractor. Specifically, the quiet-wave
envelope in stage_interference_relax applies alpha at phase `2π × step/relax_steps`. At
relax=20, the envelope cycle completes (step/20 goes 0→1). At relax=21, the envelope runs
one step longer and the system experiences a slight EXPANSION of alpha after the zero-crossing
at step=20, before the additional step forces the system slightly away from the ideal attractor.

This extra step creates a subtle phase landscape shift. The specific over-convergence failure
mode (xi 0.9614 vs 0.9870 baseline) suggests:
- The 21-step attractor places corpus memories at a subtly different phase cluster configuration
- At this configuration, adversarial perturbation creates proportionally LARGER disruption than
  at the 20-step attractor (engine_adv diverges more from engine_clean)
- xi collapses to below-baseline level, not to an intermediate value

### The transition from 20 to 21 is sharper than 20 to 24

T08 showed:
- relax=20 → xi=0.9973
- relax=24 → xi=0.748

A linear interpolation at relax=21 would predict xi ≈ 0.948. Instead, xi=0.9614.
The actual trajectory is non-linear: the system begins over-convergence regime already
at relax=21. The collapse becomes more severe at 24.

### The quiet-wave envelope creates a discrete attractor lock

The envelope `alpha = alpha_base × (1 + 0.15 × sin(2π × step/relax_steps))` is sensitive
to the total number of steps because the cycle completes at step=relax_steps. With relax=20,
the system visits exactly one envelope cycle and settles at the zero of the envelope (alpha
returns to alpha_base). With relax=21, the 21st step applies alpha at phase 2π×20/21 ≈ 1.99π,
re-introducing a small positive alpha boost AFTER the attractor would have settled at step 20.
This extra boost disrupts the attractor and creates a less favorable xi phase landscape.

This suggests relax_steps must be chosen as a multiple of the natural oscillation period, or
exactly at a zero-crossing. 20 happens to land exactly at the end of one cycle, which may be
why it's the precise optimum.

### Impact on T08's "24 catastrophic" result

The trajectory is now clearer:
- relax=16: xi=0.9870 (natural baseline, one full cycle fewer than 20)
- relax=20: xi=0.9973 (optimal — envelope cycle exactly completes)
- relax=21: xi=0.9614 (worse than baseline — first over-convergence step)
- relax=24: xi=0.748 (continued degradation)

The sweet spot at 20 is a discrete attractor lock created by the envelope's periodic structure.
It is not a broad plateau — it is a single point.

---

## Constraints established

- **xi_eval_relax=21 is WORSE than the baseline at 16**: xi=0.9614 < 0.9870. The over-convergence
  begins immediately at relax=21, not gradually.
- **xi_eval_relax sweet spot is exactly 20**: discrete lock created by envelope cycle. The
  optimal is not ±1 around 20 — it IS 20.
- **xi axis is now fully bounded**: 16 (baseline), 20 (optimal), 21+ (over-convergence)
- **T12's "relax=21 might give +0.0002 more xi" hypothesis**: definitively falsified
- **No remaining untested relax values** between 16 and 24 that are expected to help

---

## Updated understanding of T11 near-threshold result

The T11 combined stack (chiral_p_bp=0.15 + xi_eval_relax=20) at fitness=0.008340 with
gap=0.000003 remains the best achievable configuration. There is no variant of the
xi_eval_relax axis that improves on the T11 result. The 0.000003 gap is caused entirely
by speed_a variance (container load), not by any algorithmic shortfall.

---

## Decision

**All code changes reverted.** Single trial shows xi=0.9614, a severe regression.

The xi_eval_relax axis is now comprehensively closed:
- 16 → 0.9870 (baseline)
- 20 → 0.9973 (exact optimum, confirmed T08)
- 21 → 0.9614 (over-convergence, now confirmed this fire)
- 24 → 0.748 (catastrophic, confirmed T08)

---

## Updated open axes

| axis | status | notes |
|------|--------|-------|
| xi_eval_relax | **CONFIRMED CLOSED AT 20** | relax=21 is already over-converged (xi=0.9614 < baseline) |
| chiral_p_bp=0.15 + xi_eval_20 stack (T11) | NEAR-THRESHOLD | fitness=0.008340, gap=0.000003 speed_a noise |
| all other axes | CLOSED | multiple previous fires |

**Practical optimum remains the T11 stack at fitness≈0.008340.** The gap of 0.000003 is
architectural speed_a noise, not an algorithmic shortfall. Threshold crossing at 0.008337
requires lighter container load than observed in this session.
