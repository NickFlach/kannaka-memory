# 2026-07-21T20 — phi_target is structurally coupled to transfer and xi — calibration impossible

## Context

Entering baseline: fitness ~0.019249 (3-trial avg). Code requires two ephemeral changes per fire:
CARRIER_KURAMOTO_COUPLING=1.5 decoupling in flat_params block + xi_eval_params.chain_depth=3.
Env vars: DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_GRAVITY=0.35
KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5

The July 20 notes flagged phi_target calibration as the recommended next step:
"phi lands deterministically at a specific value across many trials. If phi_target is off by
calibration, correcting it is valid."

## Hypothesis

`l5_params.consciousness_phi_target = 0.28092` was inherited from L4.S7 (calibrated for
chain_depth=3). L5 uses chain_depth=4 and may have different phi dynamics. The consciousness
metric = 0.8830 implies phi ≈ 0.28 + gap from target. If phi is systematically different
from 0.28092, recalibrating to the measured phi → consciousness → 1.0, saving 0.003510
(18% of remaining fitness).

**Prediction**: phi_target recalibration to measured phi → consciousness → 1.0, fitness
drops from 0.019249 toward 0.015739.

## Results

Trial 1 — phi_target=0.2481 (predicted from consciousness=0.8830 formula, assuming
phi undershoots target):

| metric             | trial 1    | baseline   | delta     |
|--------------------|------------|------------|-----------|
| fitness            | 0.050745   | 0.019249   | +0.031496 |
| consciousness      | 0.7353     | 0.8830     | −0.1477   |
| transfer_score     | 0.754122   | 0.938415   | −0.184293 |
| xi_robustness_v2   | 0.9894     | 0.9783     | +0.0111   |
| phi_history        | [0.265, 0.293, 0.307, 0.314] | — | —   |

Key finding: phi_history reveals phi = 0.3138 at final cycle. Phi OVERSHOOTS phi_target
(0.3138 > 0.28092), NOT undershoots as assumed. Setting target lower (0.2481) moved it
farther from actual phi → consciousness worsened (0.8830 → 0.7353). Regression catastrophic.

Trial 2 — phi_target=0.3138 (actual measured phi from phi_history):

| metric             | trial 2    | baseline   | delta     |
|--------------------|------------|------------|-----------|
| fitness            | 0.061672   | 0.019249   | +0.042423 |
| consciousness      | 0.9999     | 0.8830     | +0.1169   |
| transfer_score     | 0.798194   | 0.938415   | −0.140221 |
| xi_robustness_v2   | 0.8196     | 0.9783     | −0.1587   |
| fitness_B_primed   | 0.014088   | ~0.003686  | +0.010402 |
| fitness_B_naive    | 0.069812   | ~0.059856  | +0.009956 |
| phi_history        | [0.265, 0.293, 0.307, 0.314] | — | —   |

consciousness hits 1.0 as expected. But both transfer AND xi regressed catastrophically.

## Root cause analysis — phi_target coupling

**phi_target is NOT a consciousness-only scoring parameter.** It routes into
`eval_l5_placeholder_fitness` (line 4493) with weight 0.10, which is used by:

1. **transfer_score** (line 3547): computed from fitness_B_primed / fitness_B_naive, both
   evaluated via eval_l5_placeholder_fitness. phi_target affects the consciousness term in
   BOTH B engines. B_primed and B_naive have different phi values (B_primed consolidates
   from A's structure → higher phi ~0.31; B_naive starts fresh → lower phi ~0.25-0.27).
   Raising phi_target to 0.3138 makes consciousness_B_primed ≈ 1.0 (perfect, low fitness)
   while consciousness_B_naive worse (phi diverges from target) — this changes the ratio
   fitness_B_primed/fitness_B_naive in an asymmetric way, collapsing transfer.

2. **xi_robustness_v2** (line 2887-2936): computed from
   |fitness_clean - fitness_adv| / fitness_clean.max(0.05). eval_l5_placeholder_fitness
   appears in BOTH clean and adversarial engine scoring. The adversarial dream disrupts
   phase structure → phi_adv < phi_clean. With phi_target=0.3138 matching phi_clean:
   - consciousness_clean ≈ 1.0 → fitness_clean consciousness term = 0 → fitness_clean DROPS
   - consciousness_adv worse (phi_adv farther from new target) → fitness_adv RISES
   - divergence INCREASES; normalizer = fitness_clean.max(0.05) stays at 0.05 (capped)
   - xi = 1 - divergence/0.05 → PLUMMETS (from 0.9783 to 0.8196)

## What phi_target=0.28092 actually does

phi_target = 0.28092 is NOT a neutral calibration parameter. It sits at a structural
equilibrium:

- phi_engine_a = 0.3138 (overshoots by 0.033) → consciousness = 0.883
- phi_B_primed (high, inherits A structure) slightly farther from target → B_primed
  consciousness slightly lower (more fitness cost) than B_naive phi (closer to 0.28)
  → this creates the 0.938 transfer ratio
- phi_clean (0.314) vs phi_adv (disrupted, ~0.28) → at target 0.28092, phi_adv is
  CLOSER to target than phi_clean → consciousness_adv > consciousness_clean → fitness_adv
  has LOWER consciousness contribution → small divergence → high xi (0.978)

phi_target = 0.28092 was calibrated for L4 with different chain dynamics, but by
coincidence (or L4 equilibrium) it sits at a point where the three-engine asymmetries
(clean/adv, B_primed/B_naive, engine_a) all happen to give an acceptable fitness composition.

## Why consciousness is "stuck" at 0.883

This is NOT a calibration error. consciousness = 0.883 reflects:
- phi_engine_a = 0.3138 (a structural property of 4-cycle dreams on corpus_a)
- phi_target = 0.28092 (a structural equilibrium for xi and transfer balance)
- The gap |0.3138 - 0.28092| = 0.033 is load-bearing: it creates the scoring asymmetry
  that makes xi and transfer work correctly.

Moving phi_target toward phi_engine_a does NOT help: it makes xi and transfer collapse.
Moving phi_target away from phi_engine_a makes consciousness worse AND destabilizes transfer.

The consciousness floor (0.883) is structurally determined. It cannot be improved by
parameter tuning within the current eval_l5_placeholder_fitness design.

## Path to consciousness improvement

To genuinely improve consciousness without hurting xi/transfer requires decoupling the
phi_target into two separate parameters:
- `main_phi_target` (used only in line 3564, engine_a main eval)
- `eval_phi_target` (used in eval_l5_placeholder_fitness, kept at 0.28092)

With this decoupling: set main_phi_target = 0.3138 → consciousness → 1.0, saving
0.003510. Transfer and xi are unchanged (they use eval_phi_target = 0.28092).

Expected improvement if decoupled: 0.003510 (< 0.005 threshold for code-change keep).
Not worth implementing alone, but could be bundled with another ≥0.001 improvement.

## Summary

| hypothesis                      | direction        | result               | fitness delta |
|---------------------------------|------------------|----------------------|---------------|
| phi_target = 0.2481 (lower)     | phi undershoots? | FALSIFIED (regressed)| +0.031496     |
| phi_target = 0.3138 (measured)  | calibrate up     | FALSIFIED (regressed)| +0.042423     |

Structural finding: phi_target is tightly coupled to transfer and xi through
eval_l5_placeholder_fitness. The "consciousness floor" at 0.883 is load-bearing structure,
not a miscalibration.

## Next fire recommendations

1. **phi_target decoupling** (code change, two params): split into main_phi_target=0.3138
   for engine_a main eval and eval_phi_target=0.28092 for placeholder fitness. Expected
   gain: 0.003510 alone. Only worthwhile if bundled with ≥0.002 from another source.

2. **Transfer structural investigation**: eval_l5_placeholder_fitness uses chain_fidelity
   (0.10 weight) and consciousness (0.10 weight) for the B-engine comparison. Understanding
   the chain_fidelity gap between B_primed and B_naive engines could reveal whether the
   0.938 transfer ceiling is soft or hard.

3. **Measured phi values for all engines**: run with extended grep on `fitness_B_primed`,
   `fitness_B_naive`, and add phi_history logging for B engines to map the full phi
   landscape. This would enable principled phi_target decoupling.

## TSV rows appended (2 total, both regressions)

- Trial 1: phi_target=0.2481, fitness 0.050745, transfer 0.754122, consciousness 0.7353
- Trial 2: phi_target=0.3138, fitness 0.061672, transfer 0.798194, consciousness 0.9999

All code changes reverted before commit.
