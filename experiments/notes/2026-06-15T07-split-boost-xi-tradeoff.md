# Split-Boost Hypothesis: main=0.15, xi_eval=0.025

## Context

Continuing from T06 which established the post-fix baseline (fitness ≈ 0.116) after
AMPLITUDE_CEILING=2.0 clamping collapsed carrier_bimodal from ~1.000 to ~0.530.

Root cause (T06): stage_strengthen applies up to k=32 constructive pairs per memory
in a single cycle. With default boost=0.45, signal memories jump from amplitude 1.0
to ceiling 2.0 in cycle 0, producing step-function amp_deltas [~0.94, ~0.04, ~0.004,
~0.038]. DFT of this step function has equal power at 2 Hz and 4 Hz → carrier_e ≈ 0.53.

## Hypotheses tested this fire

### H1: drive-after-consolidation (REVERTED T1)
Move drive block to after consolidation so cycle-0 amp_deltas come from drive, not
boost saturation. Result: carrier_e=0.527 (unchanged), transfer=0.652 (WORSE),
fitness=0.129. Step-function amp_deltas dominated by ceiling saturation regardless
of drive ordering. REVERTED.

### H2: lower global boost L5_BOOST=0.15 (T2, not kept)
With boost=0.15, still saturates cycle 0 via 32×0.15=4.8 total headroom > 1.0.
carrier_e=0.540 (marginally better), transfer=0.745, xi=0.856, fitness=0.113.
Only 0.003 improvement vs 0.116 — below 0.005 keep threshold.

### H3: aggressive reduction L5_BOOST=0.025 (T3, not kept)
boost=0.025 gives xi_robustness=0.971 (vs 0.856 default) but transfer=0.452 (worse).
Both xi and transfer carry weight 0.15; the xi gain is offset by transfer loss.
fitness=0.140 — net regression.

### H4+H5: split-boost (main=0.15, xi_eval=0.025) — THIS FIRE
**Prediction**: xi_eval with lower boost reduces adversarial amplitude disruption
without affecting transfer_score (which uses l5_params = main boost). Expected:
xi_robustness↑ near 0.971 (from T3), transfer stays near 0.745 (from T2).
Predicted fitness ≈ 0.097.

## Results

| Trial | settings          | fitness  | transfer_score | carrier_e | carrier_bimodal | xi_robustness_v2 | R      | query_gravity |
|-------|-------------------|----------|----------------|-----------|-----------------|------------------|--------|---------------|
| T4    | split-boost       | 0.096204 | 0.744783       | 0.5396    | 0.5323          | 0.9714           | 0.1293 | 0.4603        |
| T5    | split-boost (rep) | 0.124662 | 0.555486       | 0.5396    | 0.5323          | 0.9714           | 0.1295 | 0.4603        |

**2-trial average fitness: 0.110** (vs 0.116 post-fix default — improvement of 0.006)

## Analysis

The split-boost correctly isolates the two effects:
- **xi_robustness**: 0.971 in both trials (vs 0.856 default) — stable, confirmed.
  Lower boost in xi_eval means adversaries can't disrupt memories as much between
  the clean and adversarial passes. Consistent improvement of 0.115 × weight 0.15
  = **0.017 fitness points**.

- **transfer_score**: 0.555–0.745 across identical runs. This is a pre-existing
  variance issue documented in T06 notes. Not caused by our change.

The fundamental problem: transfer_score variance (σ ≈ 0.095 across 2 trials with
same settings) completely swamps the xi improvement signal (0.017 points). T4 shows
fitness=0.096 (improvement), T5 shows fitness=0.125 (regression). The 5-trial budget
(3 used before this fire) prevents the 3-trial confirmation required by fire rules.

**Cannot confirm ≥0.005 improvement in 3 trials. REVERTED.**

## What was reverted

1. `l5_params.constructive_boost = 0.15` override (9-line block added after
   `l5_params.consolidation_repulsion_threshold = 0.28`)
2. `xi_eval_params.constructive_boost = 0.025` addition (reverted to
   `{ let mut p = (*params).clone(); p.chain_depth = 2; p }`)

No consolidation.rs changes were made this fire.

## Key findings (carry forward)

1. **xi_robustness is reliably improvable**: lower boost in xi_eval_params gives
   xi=0.971 vs 0.856 default, consistently across trials. Weight 0.15 → 0.017
   fitness points. To confirm this in a future fire, need to eliminate transfer
   variance (3 trials with same settings), or find a way to stabilize transfer_score.

2. **transfer_score variance is the dominant noise source**: σ≈0.095 across
   identical trials (0.555–0.745 range documented). Root cause unknown — possibly
   stochastic corpus sampling, phase initialization, or random seed. Cannot confirm
   any small improvement until this is addressed.

3. **boost reduction never fixes carrier_bimodal**: Across all boost values tested
   (0.025, 0.15, 0.45), carrier_emergence stays 0.530–0.560. Step-function amp_deltas
   persist because 32 pairs × any boost ≥ 0.032 saturates the ceiling in cycle 0
   when pairs stack. The root fix requires either: (a) limiting k_neighbors in
   stage_detect to reduce simultaneous boosts, or (b) spreading consolidation across
   multiple sub-cycles per dream cycle.

## Next fire recommendation

Option A (high value, bigger change): Reduce `k_neighbors` in stage_detect from 32
to 2–4. This limits simultaneous boosts per cycle, forcing gradual carrier buildup
and restoring [A,A,B,B] amp_delta shape. Requires editing `src/consolidation.rs`
line 372: `let k_neighbors: usize = 32.min(...)` → `2.min(...)`. Fair game per fire
rules (consolidation.rs edits allowed, 2+ trials required).

Option B (low risk): Isolate transfer_score variance first. Run 3 identical default
trials and measure σ — if variance > 0.05, file as a separate issue before attempting
further optimization.
