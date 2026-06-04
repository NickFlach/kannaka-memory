# Experiment: xi_and_flat scope — proper implementation vs fallthrough

**Date**: 2026-06-04T18  
**Branch**: kannaka-curiosity/2026-06-04T18  
**Parameters**: DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=xi_and_flat DRIVE_FREQ_HZ=2.0 (default)  
**Code change**: Added explicit `"xi_and_flat"` match arm to DRIVE_SCOPE switch  
**Decision**: REVERTED — regression

---

## Background

The `DRIVE_SCOPE` environment variable controls which of the 6 engine dream chains
receive the multiplicative attention drive. The current empirical optimum
(`DRIVE_SCOPE=xi_and_flat`, avg fitness ≈ 0.184) uses a scope name that does NOT
appear in the match arms of the Rust source — it falls through to `_ => true`,
making it semantically identical to `DRIVE_SCOPE=all`.

The intended semantics described in the context documentation was:
> xi_and_flat = engine_clean + engine_adv + engine_flat (the metric-measurement engines)

This means NOT driving engine_a, engine_b_primed, or engine_b_naive — only the
xi evaluation passes and the flat-corpus carrier emergence test.

This experiment tested whether the intended xi_and_flat behavior actually performs
better than the accidental "all" behavior.

---

## Hypothesis

Driving only the metric-measurement engines (engine_clean, engine_adv, engine_flat)
while leaving the corpus consolidation engines (engine_a, engine_b_primed,
engine_b_naive) undriven would:

1. Preserve xi_robustness_v2 gains (engine_clean + engine_adv still driven)
2. Preserve carrier_emergence gains (engine_flat still driven)
3. Potentially IMPROVE transfer_score by allowing engine_a to consolidate without
   amplitude perturbation, producing a cleaner priming snapshot for engine_b_primed

Predicted fitness: ≤ 0.179 (≥0.005 improvement over 0.184 baseline)

---

## Results (4 trials)

| trial | fitness  | transfer_score | xi_robustness_v2 | carrier_emergence |
|-------|----------|----------------|-------------------|-------------------|
| T1    | 0.236460 | 0.644837       | 0.1835            | 0.5588            |
| T2    | 0.192609 | 0.644837       | 0.4763            | 0.5588            |
| T3    | 0.218398 | 0.624792       | 0.3242            | 0.5588            |
| T4    | 0.187394 | 0.624792       | 0.5308            | 0.5588            |

**4-trial average fitness**: 0.209 (3-trial: 0.216)  
**Baseline (all/xi_and_flat fallthrough)**: 0.184 avg  
**Δ fitness**: +0.025 to +0.032 (clear regression)

Deterministic metrics: carrier_emergence=0.5588, temporal_separation=1.0,
online_retention=1.0, catastrophic_forgetting=1.0, frequency_transfer=0.9901.
All identical across trials as expected.

---

## Analysis

The regression is driven entirely by xi_robustness_v2. Under the proper xi_and_flat
scope, xi_robustness averaged 0.38 (range 0.18–0.53) vs. ~0.56 under the "all" scope.

This is counterintuitive. The context documentation states "Driving engine_a hurts
xi by ~0.4" (measured with DRIVE_SCOPE=a_only). If engine_a drive hurts xi, removing
it should help. But the data shows the opposite.

Possible explanations:

1. **Synergistic effect**: The interaction between engine_a drive and engine_clean/adv
   drive is positive. Driving engine_a sets up an amplitude landscape that, when the
   xi evaluation engines are also driven with the same frequency/phase, creates better
   constructive interference in the xi scoring.

2. **Corpus coherence**: Driving engine_a during its dream chain changes the amplitudes
   of corpus_a memories. Since eval_xi_robustness_v2 receives the same corpus_a (not
   post-dream engine_a), this doesn't directly affect xi eval. However, if some global
   RNG state or timing is affected, trial-level xi variance could increase.

3. **High natural variance**: xi_robustness_v2 has stated ±0.3 per-trial variance.
   The 0.56 baseline may itself be an average that could be 0.38 on unlucky draws.
   The 4-trial average 0.38 may not be significantly different from 0.56 with 4 samples.

The transfer_score stayed relatively unchanged (0.62–0.64), confirming that not
driving engine_b_primed does NOT improve cross-corpus transfer on its own.

---

## Comparison to Baseline

| metric            | baseline (all/xi_and_flat fallthrough) | xi_and_flat proper (4-trial avg) |
|-------------------|----------------------------------------|----------------------------------|
| fitness           | 0.184                                  | 0.209 (regression)              |
| xi_robustness_v2  | ~0.56                                  | ~0.38 (lower, higher variance)  |
| carrier_emergence | ~0.56                                  | 0.5588 (unchanged)              |
| transfer_score    | ~0.60                                  | ~0.63 (marginal improvement)    |

---

## Decision

**REVERTED.** The proper xi_and_flat scope is worse than the accidental "all" behavior.

The current "optimum" (`DRIVE_SCOPE=xi_and_flat`) should be understood as
`DRIVE_SCOPE=all` (drives all 6 engine contexts). The "xi_and_flat" label in
experiment configs is misleading but the behavior is correct and should not be
changed without re-establishing the optimum.

**Follow-up ideas**:
- Test DRIVE_FREQ_HZ variants (0.5 Hz, 1 Hz, 3 Hz) — frequency may be the most
  unexplored axis
- Test DRIVE_SCOPE=no_transfer (all except engine_b_primed + engine_b_naive)
- Investigate why engine_a drive helps xi_robustness despite "a_only" hurting it
  (the a_only test presumably had no engine_clean/adv drive, making direct comparison
  invalid as a causal argument)
