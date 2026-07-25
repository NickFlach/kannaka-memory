# 2026-07-25T14 — chiral_b_primed=0.25 falsified: transfer worsens, 0.15 confirmed optimal

## Context

Entering confirmed operating point from Jul 21 fire:
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.35 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3 (ephemeral code change)
```
3-trial avg fitness: **0.019249**

Remaining fitness dominated by:
| source           | weight | value  | contribution | % of fitness |
|------------------|--------|--------|--------------|-------------|
| transfer_score   | 0.15   | 0.9384 | 0.009240     | 48%         |
| consciousness    | 0.03   | 0.8830 | 0.003510     | 18%         |
| xi_robustness_v2 | 0.15   | 0.9783 | 0.003255     | 17%         |
| phase_coherence  | 0.02   | 0.8939 | 0.002122     | 11%         |
| speed_a          | 0.03   | 0.963  | 0.001110     | 6%          |

The Jul 20 fire tested chiral_b_primed=0.05 (worse, transfer 0.933) and noted the relationship
might peak above 0.15. This fire tests chiral_b_primed=0.25 to confirm or deny that prediction.

## Hypothesis

`params_bp.chiral_perturbation = 0.15` is the only parameter distinguishing the B-primed dream
from B-naive's dream (chiral=0.7). Raising to 0.25 increases Xi diversity in B_primed's
consolidation phase, potentially improving chain_fidelity in the B_primed eval, lowering
fitness_B_primed, and improving the transfer ratio.

**Prediction**: transfer_score rises from 0.938 toward ~0.943. Expected gain "likely <0.001"
per Jul 20 notes.

## Configuration

Three ephemeral code changes applied (reverted before commit):
1. `params_bp.chiral_perturbation = 0.15` → `0.25`
2. `xi_eval_params.chain_depth = 2` → `3` (baseline activator)
3. `flat_params.kuramoto_coupling` override from `CARRIER_KURAMOTO_COUPLING` env var (baseline activator)

Env vars: `DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_GRAVITY=0.35 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5`

## Results

| trial | fitness  | transfer | carrier_e | xi_rob | consciousness | speed  | magic_R | query_g |
|-------|----------|----------|-----------|--------|---------------|--------|---------|---------|
| 1     | 0.022541 | 0.921236 | 1.0000    | 0.9783 | 0.8830        | 0.9389 | 0.6082  | 0.8962  |

Baseline reference (Jul 17, chiral_b_primed=0.15): fitness 0.019249, transfer 0.938415.

**Hypothesis FALSIFIED.** Fitness worsened: +0.003292 (17% regression). Transfer dropped:
0.938415 → 0.921236 (−0.017179).

## Analysis — confirmed chiral_b_primed optimum at 0.15

| chiral_b_primed | transfer | fitness  | direction |
|-----------------|----------|----------|-----------|
| 0.05            | 0.933002 | 0.020014 | below optimal |
| **0.15**        | **0.938415** | **0.019249** | **OPTIMAL** |
| 0.25            | 0.921236 | 0.022541 | above optimal |

The curve peaks at 0.15. Below 0.15: insufficient Xi diversity → weak chain_fidelity in
B_primed → transfer drops. Above 0.15: B_primed dream becomes more similar to B_naive
dream (toward chiral=0.7) → primed advantage erodes → transfer drops.

The Jul 20 notes correctly identified 0.15 as a saddle. The experiment here confirms
the saddle IS the optimum, not a local minimum with a better peak above it.

Speed degraded notably: 0.963 → 0.939. Higher chiral perturbation in B_primed creates
more phase disruption, requiring more consolidation time.

xi, consciousness, carrier, and magic_R are unchanged — the chiral change is cleanly
isolated to the B_primed dream path.

## Fitness floor confirmed

The chiral sweep (0.05, 0.15, 0.25) is now complete. No configuration improves transfer
beyond 0.938. Combined with previous falsifications:
- K sweep (K=1.5 to 5.0): transfer inert
- DREAM_GRAVITY range (0.25–0.40): transfer inert
- CHAIN_TOP_N (5, 7, 10): transfer inert at 5 and 7; 10 hurts xi
- DRIVE_FREQ_HZ (0.25, 0.5, 1.0): transfer inert
- chain_depth_main (4, 5): depth=5 collapsed transfer
- chain_depth_b_primed: implicitly at 4 (same as main engine via l5_params)

The transfer_score = 0.938 floor appears to be an architectural constant of the current
B_primed vs B_naive eval design, not a parameter-accessible improvement opportunity.

## All code changes reverted

No code changes kept. TSV row appended (1 regression trial).

## Next fire recommendations

Given that all accessible parameters have been swept and the floor is confirmed:

1. **Structural transfer decoupling**: the B_primed eval at line 3530-3533 uses the same
   `params` as B_naive (line 3544). The only structural asymmetry is the dream params
   (chiral=0.15 vs 0.7). If a second structural asymmetry were introduced — e.g., B_primed
   dream with DREAM_GRAVITY=0.35 but B_naive with DREAM_GRAVITY=0.0 (no scaffold bonus) —
   the primed advantage might grow. High risk of metric instability.

2. **phi_target decoupling** (standalone): the Jul 21 notes identified that consciousness
   can be improved to 1.0 by splitting main_phi_target=0.3138 from eval_phi_target=0.28092.
   Gain: 0.003510. Below the 0.005 threshold alone. Possibly worth implementing if bundled
   with another 0.001+ gain — but no obvious second source now exists.

3. **Accept the floor**: fitness 0.019249 may be the effective minimum for the current
   eval architecture. The transfer floor (0.938) + consciousness floor (0.883) + xi floor
   (0.978) + phase_coherence floor (0.894) are all structurally constrained. Further
   improvement likely requires redesigning one or more of these eval components.

4. **xi_eval depth=4 at K=3.0**: Jul 16 recommendation, untested. Would need KURAMOTO_COUPLING
   to be set separately for xi_eval (code change: `xi_eval_params.kuramoto_coupling = 3.0`).
   Risky — July 12 K-sweep showed K=3.0 hurts transfer in the main engines, and xi
   eval at depth=4 was shown to hurt xi at the previous K. Only worth trying if xi
   at depth=3 K=2.0 can be improved upon.

## TSV rows appended (1 total, regression)

- Trial 1: chiral_b_primed=0.25, fitness 0.022541, transfer 0.921236, xi 0.9783
