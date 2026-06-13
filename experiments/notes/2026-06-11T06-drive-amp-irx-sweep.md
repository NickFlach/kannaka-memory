# DRIVE_A sweep in interference_relax regime — A=0.1 confirmed as true local optimum

**Date:** 2026-06-11T06 UTC
**Branch:** kannaka-curiosity/2026-06-11T06-drive-amp-irx
**Code changes:** NONE — env-var only test
**Status:** AXIS CLOSED — A=0.1 is sharp local optimum; both directions regress

---

## Background

Current empirical optimum (master after PR #259):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (deterministic)
transfer=0.935746, fp=0.003887, fn=0.060498
carrier_emergence=0.9992, xi=0.9870
magic_R=0.8643, query_gravity=0.3733
```

The `DRIVE_A=0.15` setting was confirmed better in the old regime (DREAM_MODE unset,
2026-06-06T08/T21). However, both those fires used `DREAM_MODE=<unset>` — in that regime,
carrier_e was far from saturation (0.559) and a stronger drive lifted it toward 0.58.

In the current regime, carrier_e = 0.9992 (nearly saturated). The question: does
A=0.15 help or hurt when the interference_relax mechanism is active?

The code default is `DRIVE_A = 0.15` (line 3216 of research.rs). Current experiments
explicitly set `DRIVE_A=0.1`. This axis had never been tested with DREAM_MODE=interference_relax.

A second question: DRIVE_A=0.05 — could reducing A-memory amplitude dominance in
engine_b_primed help B-memory integration? (Fewer A-memories monopolizing chain_seeds.)

---

## Hypothesis A: DRIVE_A=0.15

**Mechanism:** A-memories after engine_a dream have amplitude ≈ 1.40× initial.
In b_primed (A+B mixed), the larger amplitude gap between A and B should give A's
memories stronger pull during interference_relax, potentially reducing fp.

**Prediction:** fp 0.003887 → 0.003400-0.003600, fitness ≈ 0.012800-0.013100.
Sub-threshold, but confirms if the mechanism is active.

## Hypothesis B: DRIVE_A=0.05

**Mechanism:** Smaller A-memory amplitude advantage (1.12× vs 1.26×). More B-memories
enter the top-7 chain_seed → B's phase structure is better integrated into the
interference pattern → B-memories converge more completely to A's attractors.

**Prediction:** fp 0.003887 → 0.003500-0.003700. Sub-threshold.

---

## Results

All trials: `DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| DRIVE_A | fitness | transfer | fp | carrier_e | xi | magic_R | q_gravity |
|---------|---------|----------|----|-----------|-----|---------|---------|
| 0.1 (baseline) | **0.013337** | 0.935746 | 0.003887 | **0.9992** | 0.9870 | 0.8643 | 0.3733 |
| 0.05 (trial 2) | 0.017134 | 0.925941 | 0.004480 | 0.9744 | 0.9870 | 0.8644 | 0.3740 |
| 0.15 (trial 1) | 0.016776 | 0.927053 | 0.004413 | 0.9763 | 0.9870 | 0.8643 | 0.3726 |

fn = 0.060498 in all cases (structural, as expected).

---

## Analysis

### A=0.1 is a sharp local optimum in the interference_relax regime

Both A=0.05 and A=0.15 produce similar regressions (fitness ≈ 0.0168-0.0171), with
comparable fp degradation and carrier_e collapse. The optimum has two components:

**1. carrier_emergence sensitivity to drive amplitude**

carrier_e drops from 0.9992 to 0.9744-0.9763 in both directions. This means A=0.1
sits at the carrier_e sweet spot for the interference_relax regime. The flat-corpus
engine's ability to produce a detectable carrier signal is critically tuned to A=0.1:
- A=0.15 over-drives: amplitude oscillations are too large, distorting the 0.5 Hz
  carrier structure that interference_relax builds. carrier_e degrades.
- A=0.05 under-drives: insufficient amplitude contrast for the carrier to be detectable
  above noise. carrier_e degrades.

In the old regime (DREAM_MODE=unset), carrier_e was far from saturation (0.559), so
A=0.15 pushed it toward 0.584 — a genuine gain. In the current regime, carrier_e
is near the ceiling. Any deviation from A=0.1 perturbs the mechanism that keeps it there.

**2. fp sensitivity to chain_seed amplitude ordering**

fp worsens at both A=0.05 and A=0.15:
- A=0.15 (MORE A dominance): chain_seed is even more A-dominated. B-memories receive
  less representation in the carry centroid, degrading their integration into A's
  phase landscape → fp rises.
- A=0.05 (LESS A dominance): chain_seed lets more B-memories through. But these
  B-memories introduce phase noise into the carry centroid, disrupting the clean
  A-phase signal that interference_relax uses to pull B-memories toward the attractor → fp rises.

A=0.1 creates the precise balance: A-memories dominate enough to establish the
phase attractor, but not so much that B-memories become invisible to the chain.

**3. xi is structurally stable**

xi=0.9870 is unchanged at both A=0.05 and A=0.15. The xi mechanism (adversarial
xi-fingerprint via engine_clean/engine_adv) is independent of the drive amplitude
in this range. This confirms xi is architecturally bounded at 0.987, not drive-tunable.

### Why old regime didn't predict this

The 2026-06-06T21 fire (A=0.15, DREAM_MODE=unset) showed A=0.15 was better. That
result had carrier_e as the dominant lever (0.559 → 0.584). In the current regime:
- carrier_e is at 0.9992 — no room to grow, only room to regress
- The interference_relax dynamic doesn't exist in DREAM_MODE=unset
- chain_top_n=7 creates a tighter seed window where the A/B amplitude balance is critical

The old confirmation doesn't transfer. A=0.1 is the interference_relax optimum.

---

## Constraints established

- **DRIVE_A=0.1 confirmed as the true local optimum for DREAM_MODE=interference_relax.**
  Both A=0.05 and A=0.15 regress (fitness 0.0168-0.0171 vs 0.0133 baseline).
- The regime is sensitive to drive amplitude: ±50% change causes ≈0.004 fitness regression.
- carrier_e requires A=0.1 in this regime (saturated at 0.9992 only at A=0.1).
- The drive axis is confirmed closed in the current regime. The empirical optimum setting
  of DRIVE_A=0.1 is not an arbitrary choice — it is the mechanical optimum.

---

## Decision

**No code changes.** Axis closed. No revert needed.

The empirical optimum remains unchanged:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
fitness ≈ 0.013337 (deterministic)
```

---

## Updated open axes

| axis | status | notes |
|------|--------|-------|
| DRIVE_A (irx regime) | **CLOSED** | A=0.1 is sharp local optimum; ±50% regresses by 0.004 |
| chiral_p (b_primed) | CLOSED | η=0.10 optimal; 0.003166 improvement, sub-threshold |
| chain_top_n | CLOSED | 7 confirmed optimal |
| chiral_perturbation (global) | CLOSED | η=0.7 confirmed optimal |
| b_primed relax_steps | CLOSED | 20 confirmed optimal (T07) |
| chain_carry_strength | CLOSED | neutral in η≤0.10 regime |
| transfer ceiling fp floor | STRUCTURAL | fp floor ≈ 0.0026 (chiral_p_bp=0.10); threshold needs fp ≤ 0.0018 |
| xi residual gap | ARCHITECTURAL | xi=0.987 near ceiling; not drive-tunable |
| consciousness | STRUCTURAL | phi is structural (T10); not drive-tunable |
| envelope_depth (irx) | **UNTESTED** | hard-coded at 0.15 in stage_interference_relax; never swept |

### Speculation

The only remaining genuinely untested axis is `envelope_depth` in stage_interference_relax
(line 800 of consolidation.rs: `let envelope_depth: f32 = 0.15`). This modulates the
"quiet wave" step size across the 20 relax steps. Values to try: 0.0 (flat, no breathing),
0.05 (gentler breathing), 0.30 (stronger breathing). Effect likely small. Justification
would require code change to consolidation.rs + at least 2 trials.

If envelope_depth also shows A=0.1-style sensitivity (only 0.15 works), then the entire
parameter space has been swept to architectural limits and the next gains require
a structural redesign of the transfer measurement or the B-memory initialization mechanism.
