# 2026-07-19T14 — CHAIN_TOP_N sweep falsified: transfer_score inert to chain seed width

## Context

Current confirmed optimum (Jul 18 fire):
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.35 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3
```
fitness 0.019249 (3-trial avg, Jul 17). Remaining fitness dominated by transfer_score
(0.938, 48%), consciousness (0.883, 18%), xi_robustness_v2 (0.978, 17%).

## Hypothesis — CHAIN_TOP_N sweep for transfer improvement

### Reasoning

`CHAIN_TOP_N` controls how many top-amplitude non-noise memories are selected per
dream cycle to form the `ChainSeed.carry_xi_centroid` used in chain_fidelity evaluation.
Default is 7.

In the transfer measurement, `eval_l5_placeholder_fitness` runs on both `engine_b_primed`
and `engine_b_naive`, including `chain_fidelity` at weight 0.10. The B-primed engine
carries A's post-dream memories (higher amplitude, more consolidated) alongside B's
newly inserted memories. Narrowing the seed (CHAIN_TOP_N=5) could favour A's dominant
memories in the B-primed centroid more strongly than in B-naive, potentially creating
a larger primed vs naive distinction → higher transfer_score.

Widening the seed (CHAIN_TOP_N=10) was predicted to dilute the centroid signal and
potentially allow adversarial memories (30 injected in xi eval) into the top-N more
easily, hurting xi_robustness_v2.

**Prediction**: CHAIN_TOP_N=5 → transfer_score 0.938 → ~0.945, fitness ~0.018.
**Risk**: transfer inert to CHAIN_TOP_N (both primed and naive see the same structural
change, cancelling the ratio).

### Configuration

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_GRAVITY=0.35
KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
```

Baseline code changes applied (reverted before commit):
1. `xi_eval_params.chain_depth` 2→3
2. `CARRIER_KURAMOTO_COUPLING` env var override in `flat_params` block

CHAIN_TOP_N is pure env var — no code change required.

## Results

| trial | CHAIN_TOP_N | fitness  | transfer | xi_robust | carrier_e | consciousness | speed  | query_g |
|-------|-------------|----------|----------|-----------|-----------|---------------|--------|---------|
| 1     | 5           | 0.019206 | 0.938415 | 0.9783    | 1.0000    | 0.8830        | 0.9642 | 0.8962  |
| 2     | 10          | 0.022148 | 0.938415 | 0.9586    | 1.0000    | 0.8830        | 0.9645 | 0.8962  |

Baseline reference (CHAIN_TOP_N=7, Jul 17 3-trial avg): fitness 0.019249.

## Analysis

**Hypothesis FALSIFIED.**

### CHAIN_TOP_N=5

Transfer_score byte-identical at 0.938415. All metrics (xi, carrier, consciousness,
phase_coherence) byte-identical to baseline. Fitness 0.019206 is within noise of the
baseline avg 0.019249 — the small difference is timing variance in speed_a
(14760ms vs ~15200ms).

The transfer ratio (1 - fitness_b_primed/fitness_b_naive) is completely insensitive
to CHAIN_TOP_N=5. This confirms the theoretical concern: narrowing the chain seed
affects chain_fidelity identically in both B-primed and B-naive evaluations. The
ratio cancels and transfer_score is unchanged.

### CHAIN_TOP_N=10

Fitness degrades: 0.019206 → 0.022148 (+0.002942 relative to trial 1).
The degradation is entirely in xi_robustness_v2: 0.9783 → 0.9586 (−0.0197).

**Mechanism**: The xi eval builds a clean engine and an adversarial engine (30 injected
memories) then computes `xi = 1 - |fitness_clean - fitness_adv| / max(fitness_clean, 0.05)`.
With CHAIN_TOP_N=10, the adversarial memories can enter the top-10 chain seed more
easily (they start at amplitude 0.15 but are not noise-tagged and may accumulate
amplitude during the dream). A wider seed amplifies the xi-centroid divergence between
clean and adversarial engines, raising |fitness_clean - fitness_adv| and lowering xi.

At CHAIN_TOP_N=5–7, adversarial memories cannot competitively enter the top-5 or
top-7 (corpus signal memories dominate). The xi_robustness score is insensitive in
this range. At CHAIN_TOP_N=10, there is just enough adversarial leakage to measurably
corrupt the centroid comparison.

### CHAIN_TOP_N sensitivity range

| CHAIN_TOP_N | fitness  | xi_robust | transfer | notes                              |
|-------------|----------|-----------|----------|------------------------------------|
| 5           | 0.019206 | 0.9783    | 0.938415 | byte-identical to baseline on xi   |
| 7 (default) | 0.019249 | 0.9783    | 0.938415 | 3-trial avg baseline               |
| 10          | 0.022148 | 0.9586    | 0.938415 | xi degradation from adv leakage    |

Transfer is inert across all tested CHAIN_TOP_N values. xi is robust at ≤7, fragile at 10.
CHAIN_TOP_N=7 is already at the correct operating point.

## Decision

**No improvement found.** CHAIN_TOP_N is not a lever for transfer improvement.
- CHAIN_TOP_N=5: essentially identical to baseline (all metrics byte-identical)
- CHAIN_TOP_N=10: regressive (xi degradation, fitness +0.003)

Code changes reverted. No source modifications committed.

## Confirmed operating point (unchanged from Jul 17/18)

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.35 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3
```
fitness = 0.019249 (3-trial avg, Jul 17)

## Next fire recommendations

The transfer floor (0.938) appears structurally locked under the current dream topology:
- K-sweep (tested Jul 12): K=2.0 is optimal for transfer
- DREAM_GRAVITY sweep (tested Jul 17): 0.35 is optimal, gravity does not affect transfer
- CHAIN_TOP_N sweep (this fire): transfer inert to 5/7/10
- chain_depth (tested Jul 18): chain_depth=5 collapses transfer

**Transfer is unreachable via the known env-var knobs.** New approaches to consider:

1. **chiral_perturbation for B_primed**: `params_bp.chiral_perturbation` is hard-coded
   to 0.15 (vs default 0.7 for B_naive). Reducing further to 0.05 or 0.0 would minimize
   phase disruption in the primed pass, potentially improving B_primed's fitness_sub
   and raising the transfer ratio. This requires a code change in the L5 code path.
   Risk: too little perturbation might over-fit B to A's phase structure, reducing
   the clean vs adversarial xi signal.

2. **DRIVE_FREQ_HZ variants** (1.0, 0.25 Hz): at chain_depth=4, changing the drive
   frequency changes which cycles receive positive vs negative amplitude modulation.
   At 0.25 Hz, all 4 cycles see a more gradual rising arc (gentler than 0.5 Hz).
   At 1.0 Hz, cycles 1-2 get full positive drive, cycle 4 gets negative suppression.
   These haven't been tested at current dynamics (fitness 0.019, DREAM_GRAVITY=0.35).

3. **consciousness mechanism**: phi diverges from phi_target (0.883 vs 1.0 at target).
   phi_target=0.28092 is set as a literal constant in L5 params. Adjusting phi_target
   to match the empirical landing value (either 0.248 or 0.314, whichever it settles
   to) would give consciousness≈1.0 — though this games the metric rather than
   improving the system. Genuinely improving phi requires understanding what drives
   the undershoot/overshoot.

4. **phase_coherence floor (0.8939)**: contributes 11% of fitness (0.00212). Not tested
   directly as a lever. kuramoto_steps (currently 50) or kuramoto_dt (0.05) might
   affect phase clustering without requiring new env-var plumbing. Worth 1 trial at
   kuramoto_steps=100 (100 steps vs 50 at same K=2.0).

## TSV rows appended

- Trial 1: CHAIN_TOP_N=5, DREAM_GRAVITY=0.35, all baseline changes — fitness 0.019206
- Trial 2: CHAIN_TOP_N=10, DREAM_GRAVITY=0.35, all baseline changes — fitness 0.022148
