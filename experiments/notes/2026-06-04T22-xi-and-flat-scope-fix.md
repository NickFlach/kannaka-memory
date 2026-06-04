# Hyp: True `xi_and_flat` scope (engine_clean + engine_adv + engine_flat only)

**Date**: 2026-06-04T22  
**Branch**: kannaka-curiosity/2026-06-04T22  
**Code change**: Reverted (inconclusive)

## Discovery

During code orientation, found that `DRIVE_SCOPE=xi_and_flat` in the env falls through
to the `_ => true` arm in `run_l5_dream_chain`, making it identical to `DRIVE_SCOPE=all`.
The design intent (driving only engine_clean + engine_adv + engine_flat, excluding
engine_a and B engines) was never implemented.

## Hypothesis

Implementing true `xi_and_flat` (engine_clean + engine_adv + engine_flat only) would:
1. Preserve xi_robustness_v2 gains (engine_clean + engine_adv still driven)
2. Improve carrier_emergence (engine_flat still driven)
3. Avoid "engine_a drive hurts xi" effect (~0.4 reported impact)
4. Yield cleaner transfer_score (B engines untouched)

**Prediction**: fitness < 0.154 (below the 1-trial "all" reference with this build).

## Results

All runs use `DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 chain_depth=16`.

| trial | scope | fitness | transfer_score | carrier_emergence | xi_robustness_v2 |
|---|---|---|---|---|---|
| ref-0 | none (no drive) | 0.180920 | 0.485979 | 0.3235 | 0.8819 |
| ref-1 | all (= current xi_and_flat) | 0.154110 | 0.421814 | 0.5338 | 0.9791 |
| xi_and_flat-1 | true xi_and_flat | 0.164369 | 0.485979 | 0.5338 | 0.8516 |
| xi_and_flat-2 | true xi_and_flat | 0.162145 | 0.485979 | 0.5338 | 0.8670 |
| xi_and_flat-3 | true xi_and_flat | 0.151902 | 0.485979 | 0.5338 | 0.9356 |

**True xi_and_flat 3-trial avg**: 0.159472  
**"all" scope 1-trial reference**: 0.154110

## Analysis

### What the data shows

**transfer_score** is deterministic across trials: 0.485979 for true xi_and_flat,
0.421814 for "all". Driving engine_a changes transfer dynamics. Unexpectedly,
NOT driving engine_a produces a *higher* transfer_score (0.486 vs 0.422) — meaning
true xi_and_flat has lower fitness cost from transfer axis (0.0771 vs 0.0867).

**carrier_emergence** is deterministic at 0.5338 in both scopes: driving engine_flat
alone is sufficient to unlock carrier emergence improvement.

**xi_robustness_v2** is the volatile metric (±0.05 range across 3 trials). The
single "all" trial had 0.9791; true xi_and_flat trials ranged 0.85–0.94. With the
reported ±0.3 per-trial variance this difference is within noise range.

### Net fitness breakdown (avg true xi_and_flat vs "all")

| axis | "all" cost | xi_and_flat cost | delta |
|---|---|---|---|
| transfer_score (15%) | 0.0867 | 0.0771 | −0.0096 (xi_and_flat wins) |
| xi_robustness_v2 (15%) | 0.0031 | ~0.0172 | +0.0141 (all wins) |
| carrier_emergence (10%) | same | same | 0 |

Net: "all" is ahead by ~0.005 fitness points, but single-trial xi_robustness_v2
variance (±0.045 fitness) swamps this signal.

## Decision

**INCONCLUSIVE**. 3 trials of true xi_and_flat vs 1 trial of "all" does not
establish a reliable ordering. Code change reverted.

## What to test next

1. Run 3 trials of `DRIVE_SCOPE=all` to get a stable avg for fair comparison.
2. Test `DRIVE_SCOPE=no_transfer` — this drives engine_a + xi engines but NOT
   engine_b, potentially capturing xi robustness benefit without contaminating
   the B-primed transfer path.
3. Investigate what drives the transfer_score difference between "all" and baseline:
   is it engine_a amplitude changes affecting the snapshot_engine_for_plasticity path?
