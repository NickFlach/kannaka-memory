# b_primed alpha_base sweep — shallow minimum at 0.13, sub-threshold

**Date:** 2026-06-11T06 UTC
**Branch:** kannaka-curiosity/2026-06-11T06-bprimed-alpha-base-sweep
**Code changes:** REVERTED — sub-threshold improvement, max gain 0.000259
**Status:** FALSIFIED — alpha_base axis closed; non-monotonic with very shallow minimum at 0.13

---

## Background

Current empirical optimum (master at ed008c0):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (deterministic)
transfer=0.935746, carrier_e=0.9992, xi=0.9870
fitness_B_primed=0.003887, fitness_B_naive=0.060498
magic_R=0.8643, query_gravity=0.3733
```

T04 (chiral-bp-010-sweep) closed all known stacking axes, leaving:
- transfer ceiling: fp floor ~0.003887 (at chiral_p=0.7 default), fp=0.002582 with reverted
  chiral_p=0.10 for b_primed (sub-threshold improvement of 0.003166)
- xi residual: near architectural limit, no open lever

The global `alpha_base` axis was closed at 0.10 by T09 (2026-06-08) — raising it to 0.15
globally caused carrier_e 0.935→0.833. But T09 tested GLOBAL alpha, affecting engine_a's
carrier formation. Engine_b_primed's carrier_e is not measured (carrier is measured on
engine_a only). A b_primed-specific alpha override has never been tested.

---

## Hypothesis

B's new memories in engine_b_primed start at corpus-assigned {0, π/2} phases while A's
memories are already at their post-dream irx attractor. The irx update formula is:

```
new_phase = cur + alpha × sin(target - cur)
```

For A's at-attractor memories: target ≈ cur, so sin(target-cur) ≈ 0 — they barely move
regardless of alpha magnitude. For B's far-from-attractor memories: sin is significant,
so higher alpha pushes them faster toward the constructive attractor.

**Prediction:** b_primed alpha_base=0.13 (30% increase) selectively accelerates B-memory
convergence without disturbing A's settled phases. Result: fp decreases, transfer rises,
fitness drops.

**Implementation:**
```rust
let alpha_base: f32 = if drive_ctx == "engine_b_primed" {
    BPRIMED_ALPHA_BASE env (default 0.13)
} else {
    0.10
};
```

**Prediction at 0.13:** fp ~0.003600, transfer ~0.940, fitness ~0.012800 (Δ ≈ −0.0005).
**Falsification signal:** fp rises above 0.003887 (over-relaxation).

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| alpha_base (b_primed) | fitness | transfer | fp (B_primed) | fn (B_naive) | xi | carrier_e | magic_R | query_g |
|------------------------|---------|----------|---------------|--------------|-----|-----------|---------|---------|
| 0.10 (baseline) | 0.013337 | 0.935746 | 0.003887 | 0.060498 | 0.9870 | 0.9992 | 0.8643 | 0.3733 |
| **0.13 (T1)** | **0.013078** | **0.936756** | **0.003826** | 0.060498 | 0.9870 | 0.9992 | 0.8643 | 0.3733 |
| 0.15 (T2) | 0.013735 | 0.932385 | 0.004091 | 0.060498 | 0.9870 | 0.9992 | 0.8643 | 0.3733 |

---

## Analysis

### Non-monotonic curve with shallow minimum at alpha≈0.13

| alpha | fp | fitness | Δ fitness |
|-------|----|---------|-----------|
| 0.10 | 0.003887 | 0.013337 | — |
| **0.13** | **0.003826** | **0.013078** | **−0.000259** |
| 0.15 | 0.004091 | 0.013735 | +0.000398 |

The minimum is at 0.13 with a tiny improvement (fp 0.003887→0.003826, −1.6% reduction).
At 0.15, fp reverses and worsens by +5.2% vs baseline.

### Why the gain is negligible and the regression appears quickly

The "selective acceleration" argument is partially correct but overestimated. A's memories
in engine_b_primed are NOT perfectly at equilibrium — they were transferred from engine_a
(which itself used the constructive-pair attractor for corpus_a), but engine_b_primed has
a DIFFERENT working set (corpus_a + corpus_b). The constructive pairs in engine_b_primed
are recomputed from scratch on this mixed engine; A's memories' neighbors now include B
members, shifting the local attractor. At alpha=0.10, A's memories gently re-converge to
the mixed attractor. At alpha=0.13, they re-converge slightly harder without overshooting.
At alpha=0.15, they begin to overshoot the mixed attractor → fp worsens.

The "sin(Δφ)≈0 for A at equilibrium" assumption was wrong: A's memories are NOT at the
mixed-engine's attractor, only at engine_a's single-corpus attractor. The cross-corpus
constructive pairs create a new attractor geometry that A's memories are also trying to reach.

The gain at alpha=0.13 reflects a marginal improvement in this re-convergence, but the
total budget constraint (20 × 0.13 = 2.6 vs 20 × 0.10 = 2.0) is already close to the
2.4 budget that T09 showed was at the carrier_e cliff for the global case.

### Why xi, carrier_e, magic_R, query_gravity are all unchanged

All four metrics are measured on engine_a only. The b_primed alpha_base change does not
touch engine_a's relaxation at all. This confirms the proposed isolation — the change is
purely a b_primed dynamics modification.

### Gap to threshold

Maximum achievable gain from b_primed alpha_base axis: **0.000259** (at alpha=0.13).
Threshold gap from baseline: **−0.005000** (need to reach ≤0.008337).
This axis contributes **5.2% of the required gap** — negligible.

Even stacking with the best known reverted improvement (chiral_p=0.10 → −0.003166),
the combined improvement would be −0.003425, still 0.001575 short of threshold.

---

## Constraints established

- **b_primed alpha_base = 0.13 is the minimum** — but the minimum is only 0.000259 better
  than the default 0.10. Not worth special-casing.
- **alpha > 0.13 for b_primed regresses** — A's mixed-engine re-convergence overshoots.
- **b_primed alpha_base axis is now closed** — the gain is sub-threshold by 19×.

---

## Decision

**Code reverted.** b_primed alpha_base axis yields at most 0.000259 improvement — well
below the 0.005 threshold. No stacking candidate exists to make up the 0.004741 gap.

---

## Updated open axes

| axis | status | notes |
|------|--------|-------|
| chiral_p (b_primed) | CLOSED | η=0.10 is optimum; −0.003166 improvement, sub-threshold |
| b_primed alpha_base | **NEW: CLOSED** | 0.13 is optimum; −0.000259 improvement, sub-threshold |
| chain_top_n | CLOSED | 7 confirmed optimal |
| chiral_perturbation | CLOSED | η=0.7 confirmed optimal |
| b_primed relax_steps | CLOSED | 20 confirmed optimal |
| chain_carry_strength | CLOSED | neutral in η≤0.10 regime |
| transfer ceiling | **OPEN** | fp floor ~0.003826 (0.003887 at default); need fp ≤ 0.001815 via unknown mechanism |
| xi residual gap | LOW | xi=0.987 leaves 0.00195 fitness; near architectural limit |

**Speculation:** The transfer score ceiling is increasingly structural. fp=0.003826
represents B memories' irreducible adjustment cost when integrated into engine_b_primed's
mixed-corpus attractor landscape. No single-parameter irx tuning reduces this further by
meaningful amounts. New breakthrough would require corpus-level changes or a fundamentally
different b_primed integration strategy.
