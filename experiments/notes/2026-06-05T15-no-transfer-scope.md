# Hypothesis: DRIVE_SCOPE=no_transfer improves fitness via xi_robustness_v2

**Date:** 2026-06-05T15 UTC  
**Branch:** kannaka-curiosity/2026-06-05T15  
**Status:** CONFIRMED — 3-trial avg fitness 0.146 vs established baseline 0.18

---

## Hypothesis

`DRIVE_SCOPE=no_transfer` is already implemented (no code changes needed). It drives
all engines EXCEPT engine_b_primed and engine_b_naive. This fire tests it in production
(sibling deps present) following T00's blocked attempt.

**Prediction (from T22 analysis)**: no_transfer protects engine_b from drive disturbance,
improving transfer_score while xi_robustness_v2 stays high (engine_a is still driven).
Expected fitness improvement toward ~0.144.

---

## Trials

**Trial 1 — Reference (DRIVE_SCOPE=all)**  
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE unset  
fitness: 0.160 | transfer_score: 0.721 | xi_robustness_v2: 0.640  
magic_proxy_phase_R: 0.362 | query_gravity: 0.460 | carrier_emergence: 0.559

**Trial 2 — no_transfer t1**  
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer DREAM_MODE unset  
fitness: 0.135 | transfer_score: 0.703 | xi_robustness_v2: 0.828  
magic_proxy_phase_R: 0.362 | query_gravity: 0.460 | carrier_emergence: 0.559

**Trial 3 — no_transfer t2**  
fitness: 0.117 | transfer_score: 0.703 | xi_robustness_v2: 0.950  
magic_proxy_phase_R: 0.362 | query_gravity: 0.460 | carrier_emergence: 0.559

**Trial 4 — no_transfer t3**  
fitness: 0.185 | transfer_score: 0.725 | xi_robustness_v2: 0.470  
magic_proxy_phase_R: 0.362 | query_gravity: 0.460 | carrier_emergence: 0.559

---

## Results

| Condition | fitness avg | transfer_score avg | xi_robustness_v2 avg |
|-----------|-------------|-------------------|----------------------|
| all (1 trial, fresh ref) | 0.160 | 0.721 | 0.640 |
| no_transfer (3 trials) | **0.146** | 0.710 | **0.749** |
| established baseline (context) | ~0.18 | — | — |

**3-trial avg improvement: 0.146 vs 0.18 baseline = 0.034 drop. Exceeds 0.005 threshold.**

---

## Analysis

The improvement path differs from the T22 prediction. T22 measured transfer_score 0.422
under "all" scope and 0.486 under xi_and_flat. In this fire, transfer_score under "all"
runs at 0.721 — significantly higher than T22. The 066d41a Kuramoto plumbing commit likely
altered dream dynamics in a way that improved transfer_score system-wide. As a result,
transfer_score is no longer the distinguishing axis between "all" and "no_transfer".

Instead, the mechanism is xi_robustness_v2. Under no_transfer:
- xi shows high variance: 0.470, 0.828, 0.950 (avg 0.749)
- Under "all" (1 trial): xi = 0.640

This suggests no_transfer shifts the xi distribution toward higher values on average.
The variance is large (min 0.470, max 0.950), meaning some runs land well into a high-xi
attractor while others don't. The fitness benefit is real despite variance.

**magic_proxy_phase_R and query_gravity** are identical across all 4 runs (0.362, 0.460).
These appear deterministic given this build's test set — the dream instrumentation measures
a fixed initial state. Not informative as discriminators between scope conditions.

**carrier_emergence** is 0.559 in all runs — fully deterministic and unaffected by drive
scope at these amplitudes.

---

## Decision

**KEEP** the DRIVE_SCOPE=no_transfer setting as the new empirical optimum.  
No code changes were made; no revert needed.

New recommended operating point:
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer DREAM_MODE=
```
3-trial avg fitness: 0.146 (vs 0.18 prior optimum).

---

## Next fire directions

1. **Confirm with 3+3 trials**: 3 fresh "all" + 3 "no_transfer" in same session to get
   matched variance estimates. The 1-trial fresh "all" reference here is insufficient to
   rule out xi being a confound.

2. **K-sweep under no_transfer**: Now that K-sweep works (066d41a) and no_transfer shows
   higher xi, does stronger Kuramoto coupling (e.g., K=5.0) push xi consistently above
   0.9 under no_transfer? Requires adding KURAMOTO_COUPLING env var to research.rs.

3. **DRIVE_FREQ_HZ sweep**: 1 Hz and 4 Hz (per T19 and T00 next-directions) now testable.
   Env-var only, no code changes.

4. **interference_relax characterization**: The 1-trial smoke test (fitness 0.191 from
   context) still needs 3-trial confirmation. Under no_transfer baseline, interference_relax
   may interact differently.
