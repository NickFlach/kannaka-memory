# AMPLITUDE_CEILING Sweep — Hypothesis Falsified

## Context

The previous fire (T06) identified that `AMPLITUDE_CEILING = 2.0` (added in correctness fix PR #360) caused a ~17x fitness regression (0.007 → ~0.13). The T06 notes hypothesized a "sweet spot" ceiling (3.0–5.0) that would restore carrier bimodal structure while preventing unbounded growth.

## Hypothesis

Raising `AMPLITUDE_CEILING` above 2.0 will restore `carrier_bimodal` toward its pre-fix value of ~1.000, because higher-amplitude carriers can differentiate from non-carriers within the dream cycle. Predicting carrier_bimodal > 0.7 somewhere in range {3.0, 5.0, 8.0}.

## Method

Made `AMPLITUDE_CEILING` runtime-configurable via env var (read via `OnceLock`, default 2.0), replacing the `const` at consolidation.rs:39. Ran 3 trials at DRIVE_A=0.1 DRIVE_SCOPE=all (standard baseline config), varying ceiling.

## Results

| ceiling | fitness  | transfer_score | carrier_bimodal | carrier_emergence | xi_robustness | R      | query_gravity |
|---------|----------|----------------|-----------------|-------------------|---------------|--------|---------------|
| 2.0     | ~0.116   | ~0.737         | ~0.530          | ~0.529            | ~0.856        | ~0.129 | 0.460         |
| 3.0     | 0.1120   | 0.754          | 0.531           | 0.533             | 0.856         | 0.129  | 0.460         |
| 5.0     | 0.1441   | 0.543          | 0.531           | 0.538             | 0.850         | 0.132  | 0.460         |
| 8.0     | 0.1600   | 0.482          | 0.525           | 0.537             | 0.823         | 0.219  | 0.460         |

*(ceiling=2.0 row from T06 for reference)*

## Analysis

**Hypothesis falsified.** `carrier_bimodal` does not improve with higher ceilings — it stays within 0.525–0.533 across the full range tested. This refutes the T06 prediction.

The ceiling is NOT the binding constraint for bimodal structure. Two other patterns are visible:

1. **Higher ceilings worsen transfer_score and xi_robustness**: transfer drops from 0.754 at ceiling=3.0 to 0.482 at ceiling=8.0. The higher amplitudes appear to distort the resonance ranking used in transfer evaluation. The correctness fix was right to worry about this.

2. **R rises monotonically with ceiling** (0.129 → 0.132 → 0.219). Higher-amplitude memories drive stronger Kuramoto phase coupling, producing over-synchronization. This is the distortion the original fix was designed to prevent.

**Why bimodal doesn't emerge**: The pre-fix bimodal structure (~5x–20x amplitude ratio between carriers and non-carriers) accumulated across *hundreds of L5 trial runs* whose inflated values persisted in the on-disk store. Each L5 trial starts from a fresh store. Within a single trial's dream cycles, with constructive_boost=0.3 and starting amplitudes near 1.0, even ceiling=8.0 doesn't produce enough differentiation fast enough.

**Root cause reassessment**: The post-fix `carrier_bimodal` degradation reflects a structural mismatch between:
- The bimodal detection algorithm (calibrated for a ×5–20 amplitude ratio)
- The within-trial amplitude dynamics (limited by modest boost rates and few dream cycles)

## Decision

Code change reverted. No fitness improvement (ceiling=3.0 is 0.004 better than default, below the 0.005 threshold; carrier_bimodal unchanged).

## Next fire recommendations

1. **Increase `constructive_boost`**: Default is 0.3 (in `AdaptiveParams::default()`). If doubled to 0.6, more differentiation per cycle. Try 0.45, 0.6, 0.9 via env var or code change. Risk: triggers the R-distortion that ceiling was meant to prevent — pair with ceiling=2.0 to stay safe.

2. **Increase dream cycle count in L5 eval**: If L5 runs few cycles, more cycles = more accumulation. Need to check how many cycles L5 uses and whether there's a knob.

3. **Recalibrate bimodal detection**: The detection thresholds may simply be miscalibrated for the new amplitude regime. The metric may need updating to detect bimodality in [1.0, 2.0] rather than [1.0, 20.0].

4. **Focus on transfer_score variance**: Trial-to-trial variance in transfer_score (0.48–0.75 with identical settings) is large and is driving most of the fitness variance. Understanding this stochasticity is prerequisite to meaningful optimization.
