# Hypothesis: interference_relax relax_steps=24, alpha_base=0.067

**Date:** 2026-06-07T00 UTC
**Branch:** kannaka-curiosity/2026-06-07T00
**Code change:** `src/consolidation.rs` stage_interference_relax: relax_steps 16→24, alpha_base 0.10→0.067 — **REVERTED** (no improvement)
**Status:** FALSIFIED — xi degraded, net fitness worse

---

## Background

T00 (2026-06-06T00) moved interference_relax from 8×0.20 to 16×0.10, keeping total
coupling at ~1.6. xi avg improved 0.083→0.607, fitness improved ~0.21→0.149 avg.

The T00 pattern (finer steps at same total coupling → better xi) suggested 24 steps
as the next point on the same curve. 24×0.067 keeps total coupling at ~1.6.

**Current empirical best:** K=1.0 all-scope DREAM_MODE unset, 3-trial avg fitness 0.138
**interference_relax at 16×0.10 (T00):** 3-trial avg fitness 0.149

---

## Hypothesis

More relaxation steps at lower per-step alpha → smoother convergence → xi improves.
The quiet-wave envelope sampled at 24 points vs 16 reduces oscillatory artifacts.

**Prediction:** xi avg rises from ~0.607 to ~0.68+. carrier_e unchanged at ~0.497.
Net: fitness avg drops from ~0.149 toward ~0.143 (still above 0.138 optimum, but closer).

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| trial | fitness | transfer_score | carrier_e | xi_robustness_v2 | magic_R | query_gravity |
|-------|---------|----------------|-----------|-----------------|---------|---------------|
| t1    | 0.144778 | 0.764839      | 0.7186    | 0.4713          | 0.5989  | 0.3651        |
| t2    | 0.184809 | 0.764839      | 0.7186    | 0.2048          | 0.5989  | 0.3651        |
| t3    | 0.171880 | 0.778987      | 0.7186    | 0.2764          | 0.5989  | 0.3651        |
| **avg** | **0.167** | **0.769** | **0.719** | **0.317** | **0.599** | **0.365** |

**Baseline (T00, 16×0.10, 3-trial avg):** fitness 0.149, xi 0.607, carrier_e 0.497, magic_R 0.617

---

## Analysis

The prediction was wrong on the main axis: **xi degraded from 0.607 avg to 0.317 avg.**

Carrier_e improved substantially (0.497→0.719), but at fitness weights (xi: 0.15,
carrier_e: 0.10) the xi degradation dominates:
- carrier gain: 0.10 × (0.719−0.497) = +0.022 fitness cost removed
- xi loss: 0.15 × (0.607−0.317) = −0.044 fitness cost added
- Net: +0.022 fitness (worse), consistent with observed 0.167 vs 0.149

**Why xi degraded with finer steps (opposite of T00):**

The T00 improvement (8→16 steps) was not purely about step count — it halved per-step
alpha from 0.20→0.10 simultaneously. The relevant variable is per-step alpha magnitude,
not step count alone. At alpha=0.10 per step, the relaxation lands phases in stable
attractor states that are hard for the adversary to perturb. At alpha=0.067, the phases
converge less strongly per step, leaving them in shallower attractor basins — more
vulnerable to adversarial xi attacks.

The carrier_e finding makes the same point from the other side: gentler steps (0.067)
preserve more phase diversity in the final state, creating amplitude beats (carrier).
Stronger steps (0.10) push phases to deeper attractors, erasing some carrier structure.
The carrier_e/xi trade-off is real, and alpha=0.10 sits on the xi-favorable side.

**Note on transfer_score variance:** t3 shows 0.778987 vs 0.764839 in t1/t2. transfer_score
has a small stochastic component under interference_relax (not just xi). Difference is
small (~0.014) and likely from ordering effects in the dream chain.

---

## Comparison to baselines

| config | fitness avg | xi avg | carrier_e | magic_R | transfer_score |
|--------|------------|--------|-----------|---------|----------------|
| K=1.0 all-scope (DREAM_MODE unset) | 0.138 | 0.863 | 0.568 | ~0.25 | 0.682 |
| interference_relax 16×0.10 (T00) | 0.149 | 0.607 | 0.497 | 0.617 | 0.750 |
| interference_relax 24×0.067 (this) | 0.167 | 0.317 | **0.719** | 0.599 | 0.769 |

24-step is strictly worse. carrier_e improvement does not compensate for xi degradation.

---

## Decision

**Code reverted.** No improvement over T00 16-step baseline.

16×0.10 remains the interference_relax optimum. Per-step alpha 0.10 sits at the sweet
spot: strong enough for xi-stabilizing phase convergence, gentle enough to preserve
carrier_e at 0.497. Going lower alpha (0.067) sacrifices xi while improving carrier_e —
wrong direction at current metric weights.

**Implication:** the gap between interference_relax (0.149 avg) and K=1.0 stage_sync
(0.138 avg) appears structural. interference_relax's constructive-pair-driven relaxation
does not create the deep xi attractor states that Kuramoto coupling at K=1.0 creates.
Further alpha/steps tuning will not close this gap. Future interference_relax work
should focus on a qualitative change to the phase update rule (e.g., deeper quiet-wave
envelope or hybrid coupling).
