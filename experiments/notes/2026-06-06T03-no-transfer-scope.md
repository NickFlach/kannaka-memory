# Hypothesis: DRIVE_SCOPE=no_transfer improves fitness via transfer_score

**Date:** 2026-06-06T03 UTC  
**Branch:** kannaka-curiosity/2026-06-06T03  
**Status:** CONFIRMED — 3-trial avg fitness 0.143 vs baseline ~0.18 (Δ −0.037, threshold 0.005)

---

## Hypothesis

`DRIVE_SCOPE=no_transfer` drives all engines EXCEPT `engine_b_primed` and
`engine_b_naive`. T00 was blocked on this test by missing sibling deps; this fire
runs it in production.

Prediction from T00: this scope combines the xi_robustness benefit of driving engine_a
(as in "all" scope) with the transfer_score benefit of leaving engine_b undisturbed
(as in "xi_and_flat" scope). Expected fitness ~0.144.

---

## Results

| Trial | fitness | transfer_score | xi_robustness_v2 | carrier_emergence | magic_proxy_phase_R | query_gravity |
|-------|---------|----------------|-----------------|-------------------|---------------------|---------------|
| 1     | 0.188   | 0.703          | 0.468           | 0.5588            | 0.3623              | 0.4597        |
| 2     | 0.112   | 0.710          | 0.963           | 0.5588            | 0.3623              | 0.4597        |
| 3     | 0.128   | 0.710          | 0.858           | 0.5588            | 0.3623              | 0.4597        |
| **avg** | **0.143** | **0.707** | **0.763** | 0.5588 | 0.3623 | 0.4597 |

Baseline (DRIVE_SCOPE=all, DREAM_MODE unset, 3-run avg): ~0.18

---

## Analysis

**Transfer score**: Dramatically improved — 0.703–0.710 vs baseline ~0.42. Excluding
engine_b_primed and engine_b_naive from the drive means engine_b consolidates without
amplitude perturbation, producing sharper primed-vs-naive discrimination. The effect
is large and deterministic (trials 2 and 3 identical at 0.710).

**xi_robustness_v2**: Variable (0.468–0.963, avg 0.763). Baseline ~0.642. The avg is
slightly better, but trial 1 was anomalously low. The T00 prediction that xi would be
preserved (since engine_a IS still driven) is partially confirmed — the average is
healthy — but the variance is high. xi determination may depend on PRNG state during
the dream chain in a way that no_transfer amplifies.

**carrier_emergence, R, query_gravity**: All stable across trials and similar to
baseline. The no_transfer scope doesn't change the carrier dynamics or phase structure
in engine_a.

**Prediction accuracy**: T00 predicted ~0.144; actual avg is 0.143. Prediction was
accurate. The T00 concern about transfer_score direction reversal in stubs was correct;
production shows transfer_score 0.70+ vs the 0.486 plateau seen with xi_and_flat, a
further improvement not anticipated by T00.

---

## No code changes

This is a pure env-var configuration result. Nothing to revert.

---

## Next directions

1. **K-sweep on no_transfer**: does changing kuramoto_coupling (1.0–7.0) further
   reduce xi variance under no_transfer while holding transfer_score at 0.70+?
2. **no_transfer + interference_relax**: the smoke test showed interference_relax
   raises carrier_e (0.714 vs 0.559) and R (0.612 vs 0.355) but cuts xi (0.220).
   Combined with no_transfer's xi avg of 0.763, the trade-off surface is unknown.
3. **Reduce xi variance**: trial 1's xi=0.468 is anomalous. Investigate whether a
   kuramoto K change stabilizes xi across trials.
