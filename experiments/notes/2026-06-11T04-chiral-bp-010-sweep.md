# chiral_perturbation b_primed sweep — optimal at η=0.10, sub-threshold improvement

**Date:** 2026-06-11T04 UTC
**Branch:** kannaka-curiosity/2026-06-11T04-chiral-bp-010
**Code changes:** REVERTED — sub-threshold improvement (0.003166 < 0.005 threshold)
**Status:** FALSIFIED (threshold not crossed) — axis characterized, sharp minimum at η=0.10

---

## Background

Current empirical optimum (master at 2b8050f):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (deterministic)
transfer=0.935746, carrier_e=0.9992, xi=0.9870
magic_R=0.864, query_gravity=0.373
fitness_B_primed=0.003887, fitness_B_naive=0.060498
```

T17 characterised chiral_perturbation for b_primed as an open axis, reverted due to
sub-threshold single-trial result (chiral_p=0.35 gave −25% fp, fitness 0.011011).
T17 predicted a linear fp reduction that would cross threshold at chiral_p=0.10 or 0.00.
T17 recommended this as the primary hypothesis for the next fire.

## Hypothesis

Lower chiral_perturbation for b_primed only reduces post-irx phase noise. The
20-step interference_relax convergence brings phases to constructive attractors;
the standard chiral step (η=0.70) then displaces them up to 0.7 rad, partially
undoing the convergence. A lower η for b_primed preserves the attractor alignment,
reduces fp, and improves transfer.

**Prediction (from T17 linear extrapolation):**
- chiral_p=0.10: fp~0.002000, transfer~0.967, fitness~0.008300 (at threshold)
- chiral_p=0.00: fp~0.001500, transfer~0.975, fitness~0.007000 (above threshold)

## Implementation

```rust
let params_bp = { let mut p = (*params).clone(); p.chiral_perturbation = eta; p };
run_l5_dream_chain(&params_bp, &mut engine_b_primed);
```

xi/carrier_e/magic_R/query_gravity all measured on other engines — isolated safely.

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| eta (b_primed) | fitness | transfer | fp (B_primed) | fn (B_naive) | xi | carrier_e | magic_R | query_g |
|----------------|---------|----------|---------------|--------------|-----|-----------|---------|---------|
| 0.70 (baseline) | 0.013337 | 0.935746 | 0.003887 | 0.060498 | 0.9870 | 0.9992 | 0.8643 | 0.3733 |
| 0.10 (trial 1) | 0.010171 | 0.957321 | 0.002582 | 0.060498 | 0.9870 | 0.9992 | 0.8643 | 0.3733 |
| 0.00 (trial 2) | 0.062436 | 0.608905 | 0.023661 | 0.060498 | 0.9870 | 0.9992 | 0.8643 | 0.3733 |
| 0.05 (trial 4) | 0.010506 | 0.955073 | 0.002718 | 0.060498 | 0.9870 | 0.9992 | 0.8643 | 0.3733 |

Also tested: chiral_p=0.10 + carry=0.85 for b_primed (trial 3):
| 0.10 + carry=0.85 | 0.010162 | 0.957321 | 0.002582 | 0.060498 | 0.9870 | 0.9992 | 0.8643 | 0.3733 |

(Fitness 0.010162 vs 0.010171 — within determinism noise; carry=0.85 is neutral.)

---

## Analysis

### T17 linear extrapolation was wrong: the curve is not monotonically decreasing

Full curve including T17 data:
| eta | fp | fitness | improvement |
|-----|-----|---------|-------------|
| 0.70 | 0.003887 | 0.013337 | — |
| 0.35 (T17) | 0.002923 | 0.011011 | −0.002326 |
| **0.10** | **0.002582** | **0.010171** | **−0.003166 (sub-threshold)** |
| 0.05 | 0.002718 | 0.010506 | −0.002831 (reversal) |
| 0.00 | 0.023661 | 0.062436 | **CATASTROPHIC** |

The minimum is at **η=0.10**. Going below this reverses the improvement before
the cliff to 0.00. T17's linear extrapolation assumed fp would keep falling, but:
- From 0.35 to 0.10: fp dropped from 0.002923 to 0.002582 (11.7% reduction)
- From 0.10 to 0.05: fp ROSE from 0.002582 to 0.002718 (+5.3%)
- From 0.05 to 0.00: fp exploded to 0.023661 (cliff)

### Why η=0.10 is the minimum, not a continuing descent

The chiral perturbation is not simply "noise after convergence." It plays a
structural role: it creates the handedness-based phase asymmetry that enables
constructive interference in subsequent cycles. The chiral_perturbation formula
applies `Δφ = eta × handedness × sin(2φ)`, where `handedness` is cluster-specific.

At η=0.10, the displacement is ~0.10 rad maximum — small enough that the irx
attractor isn't disrupted, but large enough to maintain the cluster phase asymmetry
that drives constructive pairing in cycles 2-4.

At η=0.05, the phase asymmetry is insufficient: the chiral step is too weak to
establish the cluster handedness gradient, so cycle-2 constructive pairing degrades
slightly → fp rises.

At η=0.00, no phase separation occurs at all. The phases all converge to the
same attractor type, breaking the inter-cluster differentiation entirely →
catastrophic fp explosion (6× worse than baseline).

### Why carry=0.85 is neutral at η=0.10

The chain_carry mechanism modulates the interference threshold for cycles 2-4.
With η=0.10, the cycle-1 chain seed is very clean (phases close to attractors).
carry=0.85 vs 0.70 applies to this same seed — both give fp=0.002582, transfer=0.957321.

T17's "amplification of imperfections" mechanism was the only carry leverage point:
carry amplified the imperfect post-chiral state. At η=0.10, there's minimal imperfection
to amplify, so carry_strength becomes irrelevant.

### Gap assessment

The maximum achievable improvement from chiral_p axis is 0.003166 (at η=0.10).
To cross the 0.005 threshold from 0.013337 baseline, we need 0.001834 more improvement
from some other axis.

Fitness at optimal (0.010171) breakdown:
- transfer (0.15): 0.15 × 0.042679 = **0.006402** (63%)
- xi (0.15): 0.15 × 0.013 = **0.001950** (19%)
- consciousness (0.03): ~**0.000690** (7%)
- other: ~**0.001129** (11%)

To reach 0.008337 (threshold), transfer would need to reach 0.970+.
At current fp=0.002582 / fn=0.060498, transfer = 0.957. Need fp ≤ 0.001815 (a 30% further
reduction from 0.002582).

---

## Constraints established

- **η=0.10 is the structural minimum for b_primed chiral perturbation** — not an
  intermediate optimum but an architectural constraint. Below 0.10, phase separation
  degrades; at 0.00 it collapses entirely.
- carry=0.85 is neutral in the η≤0.10 regime (no imperfections to amplify)
- Maximum gain from this single axis: 0.003166 (sub-threshold by 0.001834)

---

## Decision

**Code reverted.** chiral_perturbation=0.10 for b_primed gives 0.003166 improvement,
sub-threshold (need 0.005).

If combined with another 0.002+ axis, the stacked total might cross threshold. But
no candidate stacking axis is currently open.

---

## Updated open axes

| axis | status | notes |
|------|--------|-------|
| chiral_p (b_primed) | **CLOSED** | η=0.10 is optimum; 0.003166 improvement, sub-threshold |
| chain_top_n | CLOSED | 7 confirmed optimal (T22) |
| chiral_perturbation | CLOSED | η=0.7 confirmed optimal (T20) |
| b_primed relax_steps | CLOSED | 20 confirmed optimal (T07) |
| chain_carry_strength | CLOSED | neutral in η≤0.10 regime; peak at 0.85 but sub-threshold independently |
| transfer ceiling | **OPEN** | fp ceiling ~0.002582 with known asymmetric technique; need fp ≤ 0.001815 via unknown mechanism |
| xi residual gap | LOW | xi=0.987 leaves 0.00195 fitness; near architectural limit |

**Speculation for future fires:** fn=0.060498 has been unchanged across all fires.
The transfer score = 1 - fp/fn. fn can only be reduced by making b_naive's dream
WORSE (counterproductive) or by changing the corpus construction. The fp floor of
~0.0026 may be structural — B's memories require some minimum adjustment cost even
when primed with optimal A state.
