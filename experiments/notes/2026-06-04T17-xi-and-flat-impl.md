# L5 Curiosity Fire — 2026-06-04T17

## Hypothesis

`DRIVE_SCOPE=xi_and_flat` was not implemented as a dedicated match arm in the
scope router — it fell through to `_ => true` (equivalent to "all" engines).
This meant the documented "empirical optimum" (A=0.1, TOP_FRAC=1.0,
SCOPE=xi_and_flat, fitness ≈ 0.184) was actually driving **all six** engine
contexts: engine_a, engine_b_primed, engine_b_naive, engine_flat, engine_clean,
engine_adv.

**Prediction**: Implementing xi_and_flat correctly — targeting only
engine_clean + engine_adv + engine_flat (the xi measurement engines and the
flat-corpus carrier emergence engine) while skipping engine_a,
engine_b_primed, engine_b_naive — should improve xi_robustness_v2. The xi
measurement engines (engine_clean, engine_adv) still receive the drive, keeping
divergence between clean and adversarial passes small. engine_a no longer
receives the drive, removing whatever perturbation it was adding to the xi score.
carrier_emergence should be unaffected (engine_flat still driven).

## Preliminary probe: DRIVE_SCOPE=flat_only (no code change)

Before implementing xi_and_flat, tested DRIVE_SCOPE=flat_only as a sanity check
(drives only engine_flat, leaving engine_clean and engine_adv at baseline).

| run | fitness | xi_robustness_v2 | carrier_emergence |
|-----|---------|-----------------|-------------------|
| flat_only | 0.258415 | 0.0205 | 0.5588 |

Result: **regression**. xi_robustness_v2 collapsed to 0.02 without driving the
xi measurement engines (engine_clean, engine_adv). This showed that the drive
on engine_clean + engine_adv is load-bearing for xi. The flat_only hypothesis
is falsified.

## Code change

Added `"xi_and_flat"` match arm to the `DRIVE_SCOPE` router in
`run_l5_dream_chain` (`src/bin/research.rs`):

```rust
"xi_and_flat" => {
    drive_context == "engine_clean"
        || drive_context == "engine_adv"
        || drive_context == "engine_flat"
}
```

This targets the three xi/carrier measurement engines while excluding
engine_a, engine_b_primed, engine_b_naive.

## Results: corrected xi_and_flat

`DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=xi_and_flat`

| trial | fitness | xi_robustness_v2 | carrier_emergence | transfer_score |
|-------|---------|-----------------|-------------------|----------------|
| 1     | 0.145950 | 0.7908 | 0.5588 | 0.624792 |
| 2     | 0.119502 | 0.9668 | 0.5588 | 0.624792 |
| 3     | 0.133725 | 0.8522 | 0.5588 | 0.644837 |
| 4 (full metrics) | 0.173779 | 0.6049 | 0.5588 | 0.624792 |

**4-trial average fitness: 0.143** (range 0.120–0.174)
**Baseline (all-engine drive): 0.184**
**Improvement: −0.041 fitness** (well above the 0.005 threshold)

xi_robustness_v2 improved from ~0.56 (all-engine drive) to avg **0.810** (high
per-trial variance ±0.15, consistent with known ±0.3 metric variance).

carrier_emergence: 0.5588 across all runs (deterministic given seed, maintained).
temporal_separation, online_retention: 1.0000 (saturated, unchanged).
transfer_score: stable near 0.625 (slight decrease from pre-drive baseline ~0.675,
likely from not driving engine_b_primed).

Full metrics captured on trial 4:
- noise_removal: 1.0000, signal_preservation: 1.0000
- phase_coherence: 0.7006, speed: 0.8853, consciousness: 0.8771
- encoding_entropy: 1.0000, frequency_transfer: 0.9901
- total_ms: 56602

## Comparison to baseline

| config | avg fitness | xi | carrier_emergence |
|--------|------------|-----|-------------------|
| no drive | ~0.244 | ~0.795 | ~0.310 |
| all engines (xi_and_flat = _ => true) | ~0.184 | ~0.560 | ~0.559 |
| xi_and_flat correct (this fire) | ~0.143 | ~0.810 | ~0.559 |

The correct xi_and_flat recovers xi toward the no-drive baseline while retaining
the full carrier_emergence benefit from driving engine_flat. The engine_a drive
penalty is removed.

## Decision

**KEEP** the code change. The improvement is consistent across all 4 trials
(all below baseline 0.184). New empirical optimum: fitness ≈ 0.143 with
`DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=xi_and_flat` (correctly implemented).

## Next directions

- Test higher DRIVE_A (0.2, 0.3) with correct xi_and_flat to push carrier further
- Test DRIVE_FREQ_HZ variants (1 Hz, 4 Hz) — the drive frequency hasn't been swept yet
- transfer_score is the largest remaining residual (~0.625, weight 0.15); investigate
  whether driving engine_b_primed alone can improve it without hurting xi
- xi_robustness_v2 still has high per-trial variance (±0.15) — source unclear
