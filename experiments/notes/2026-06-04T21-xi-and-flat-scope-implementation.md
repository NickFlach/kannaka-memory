# Hypothesis: Implement `xi_and_flat` drive scope properly

**Date:** 2026-06-04T21  
**Branch:** kannaka-curiosity/2026-06-04T21

## Hypothesis

`DRIVE_SCOPE=xi_and_flat` is referenced in experiment context as the current empirical
optimum (fitness ≈ 0.184 avg), defined as driving engine_clean + engine_adv + engine_flat
while *excluding* engine_a. The claim is that "driving engine_a hurts xi by ~0.4."

However, inspection of the match arm in `run_l5_dream_chain` showed `xi_and_flat` was
never implemented — it fell through to `_ => true` (identical to "all"). Prediction:
implementing `xi_and_flat` as the 3-engine subset (engine_clean + engine_adv + engine_flat)
would improve xi_robustness_v2 and reduce fitness below the all-scope baseline.

## Code change tested

Added match arm to `src/bin/research.rs` (inside `run_l5_dream_chain`):

```rust
"xi_and_flat" => {
    drive_context == "engine_clean"
        || drive_context == "engine_adv"
        || drive_context == "engine_flat"
}
```

This excludes engine_a, engine_b_primed, engine_b_naive from the drive.

## Results

All runs: `DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0`.

| run | scope | fitness | transfer_score | carrier_bimodal | xi_robustness_v2 | carrier_emergence |
|-----|-------|---------|----------------|-----------------|------------------|-------------------|
| baseline | all | 0.1222 | 0.7210 | 0.7349 | 0.8928 | 0.5588 |
| trial-1 | xi_and_flat | 0.1592 | 0.6448 | 0.2517 | 0.7063 | 0.5588 |
| trial-2 | xi_and_flat | 0.1247 | 0.6248 | 0.2517 | 0.9566 | 0.5588 |
| trial-3 | xi_and_flat | 0.1919 | 0.6248 | 0.2517 | 0.5087 | 0.5588 |
| trial-4 | xi_and_flat | 0.2367 | 0.6448 | 0.2517 | 0.1897 | 0.5588 |

3-run avg (trials 1–3) fitness: **0.159** vs all-scope baseline **0.122**.

Additional full metrics from trial-4 (xi_and_flat):
noise_removal=1.0, signal_preservation=1.0, phase_coherence=0.7006, consciousness=0.8771,
encoding_entropy=1.0, frequency_transfer=0.9901, online_retention=1.0, temporal_separation=1.0,
total_ms=115521. (speed_a and catastrophic_forgetting not captured in trial-4 grep.)

## Analysis

The hypothesis was wrong in two ways:

1. **carrier_bimodal collapses from 0.735 → 0.252 (deterministic, consistent across all 4
   xi_and_flat runs).** Engine_a receives the drive in all-scope but not xi_and_flat. The
   carrier bimodality score depends on amplitude modulation of the main corpus engine (engine_a),
   not just the xi/flat engines. Removing the drive from engine_a loses this signal.

2. **xi_robustness_v2 did NOT improve.** The claim "engine_a drive hurts xi by ~0.4" is not
   supported. Avg xi for xi_and_flat (trials 1–3): (0.7063+0.9566+0.5087)/3 ≈ 0.724 vs
   all-scope 0.8928. Xi is actually better when engine_a is driven. The claim may have been
   derived from a different code version or parameter set.

3. **transfer_score regresses from 0.721 → ~0.635.** Engine_a modulation appears to prime
   cross-corpus transfer; removing it from the drive weakens the priming effect.

carrier_emergence is deterministic and identical (0.5588) in all runs regardless of scope —
it depends on amplitude deltas within each engine's chain, not cross-engine interactions.

## Decision

**REVERT** — code change reverted before commit. The `xi_and_flat` scope as defined here
(excluding engine_a) is a regression on all major axes: fitness +0.037 avg, carrier_bimodal
−0.48, transfer_score −0.086.

The `_  => true` wildcard (= "all") in the current code is the better behavior for this
parameter point. The "xi_and_flat" label should either be removed from documentation or
re-documented as an alias for "all".

## Next directions

- The all-scope "all" baseline itself (0.122) is already better than the context's claimed
  0.184 optimum — investigate whether the 0.184 baseline is stale or from a different build.
- The `transfer_score` is the largest residual axis (weight 0.15, value ~0.72, contributing
  ~0.042 to fitness). Exploring drive frequency (0.5 Hz, 1 Hz, 4 Hz) on all-scope may
  improve transfer_score further.
- carrier_bimodal (weight implicit via carrier_emergence) appears tightly coupled to engine_a
  amplitude dynamics — worth studying directly.
