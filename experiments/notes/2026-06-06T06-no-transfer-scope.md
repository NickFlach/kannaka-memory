# Hypothesis: DRIVE_SCOPE=no_transfer — engine_b exclusion isolates transfer vs xi

**Date:** 2026-06-06T06 UTC  
**Branch:** kannaka-curiosity/2026-06-06T06  
**Code changes:** None — env-var only test  
**Status:** INCONCLUSIVE for fitness improvement; reveals structural engine_b effect

---

## Hypothesis

`DRIVE_SCOPE=no_transfer` drives engine_a + engine_clean + engine_adv + engine_flat,
but NOT engine_b_primed or engine_b_naive. This was the primary recommendation from
T00 (2026-06-05), which was blocked by missing sibling deps. Sibling deps now present.

**Prediction (from T22 analysis):**
- transfer_score ≈ 0.486 (like xi_and_flat — engine_b unperturbed)
- xi_robustness_v2 ≈ 0.979 (like "all" — engine_a still driven)
- carrier_emergence ≈ 0.534 (engine_flat still driven)
- Expected fitness ≈ 0.144 (improvement over 0.154 "all" ref)

---

## Results

All runs: `DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer`, default DREAM_MODE.

| trial | fitness | transfer_score | xi_robustness_v2 | carrier_emergence | magic_R | query_gravity |
|---|---|---|---|---|---|---|
| t1 | 0.167313 | 0.718530 | 0.5901 | 0.5588 | 0.3623 | 0.4597 |
| t2 | 0.165858 | 0.725206 | 0.5904 | 0.5588 | 0.3623 | 0.4597 |
| t3 | 0.160240 | 0.725206 | 0.6276 | 0.5588 | 0.3623 | 0.4597 |
| **avg** | **0.164470** | **0.722981** | **0.6027** | **0.5588** | **0.3623** | **0.4597** |

**Reference baselines (from T22, production):**

| scope | fitness | transfer_score | xi_robustness_v2 | carrier_emergence |
|---|---|---|---|---|
| all (1 trial) | 0.154110 | 0.421814 | 0.9791 | 0.5338 |
| xi_and_flat (3-trial avg) | 0.159472 | 0.485979 | 0.8690 | 0.5338 |
| **no_transfer (3-trial avg)** | **0.164470** | **0.722981** | **0.6027** | **0.5588** |

---

## Analysis

### Prediction mismatch

Both predicted values were wrong in opposite directions:

- **transfer_score**: predicted 0.486, got 0.723. Not driving engine_b improves
  transfer far more than not driving engine_a does. The improvement from engine_b
  exclusion is +0.301 over "all"; the improvement from engine_a exclusion alone
  (xi_and_flat) was only +0.064.

- **xi_robustness_v2**: predicted 0.979, got 0.603. Driving engine_a alone is not
  sufficient to maintain xi. engine_b drive (absent in no_transfer) appears to be
  a prerequisite for high xi, not neutral to it.

### The engine_b drive effect on xi

The data implies engine_b drive (primed + naive) is load-bearing for xi_robustness_v2:

| engine_b driven? | engine_a driven? | xi avg |
|---|---|---|
| YES ("all") | YES | 0.979 |
| NO (xi_and_flat) | NO | 0.869 |
| NO (no_transfer) | YES | 0.603 |

Excluding engine_b while keeping engine_a driven yields the *lowest* xi (0.603), worse
than excluding both (xi_and_flat: 0.869). This is counterintuitive: the engine_b dream
chain consolidation, even though engine_b is the "transfer" path, seems to stabilize the
xi measurement path indirectly.

### Fitness arithmetic on key axes (weight 0.15 each)

| scope | xi cost | transfer cost | xi+transfer total |
|---|---|---|---|
| all | 0.0032 | 0.0867 | 0.0899 |
| xi_and_flat | 0.0197 | 0.0771 | 0.0968 |
| no_transfer | 0.0596 | 0.0416 | 0.1012 |

no_transfer has the best transfer cost but the worst xi cost, netting the worst combined
score of the three. The "all" scope wins because xi gains from engine_b drive (0.0564)
outweigh the transfer penalty from engine_b drive (0.0451).

### Instrumentation metrics

magic_proxy_phase_R (0.3623) and query_gravity (0.4597) are deterministic across all 3
trials and nearly identical to the prior "all" smoke-test baseline (~0.355, ~0.460).
No DREAM_MODE change, so this is expected.

---

## Decision

**Not an improvement.** 3-trial avg fitness 0.164 > 0.154 "all" reference. No code
changes to revert.

The key structural finding: engine_b drive is positively correlated with xi_robustness,
not negatively. The prior rationale for xi_and_flat (protect xi by excluding engine_a)
was partially wrong — engine_a drive helps xi, AND engine_b drive also helps xi. The
transfer_score penalty from driving engine_b (0.301 pts) is outweighed by the xi
benefit (0.376 pts) in the fitness formula.

---

## Implications for future fires

1. **Why "all" is still the empirical optimum**: engine_b dream consolidation is doing
   useful work for xi that the simplified scope tests disrupted. The "all" scope is not
   just a baseline — it's genuinely optimal given current metric weights.

2. **The xi↔transfer tradeoff is more complex than initially modeled.** engine_b exclusion
   massively helps transfer (0.301 pts) but massively hurts xi (0.376 pts). Any future
   scope experiment must account for engine_b's dual role.

3. **High-transfer regime**: if a future metric reweighting elevated transfer_score weight
   above xi_robustness weight, no_transfer (0.723 transfer) would become competitive.
   The crossover is at weight(transfer) > weight(xi) × (0.376/0.301) ≈ weight(xi) × 1.25.

4. **DREAM_MODE=interference_relax + xi recovery (Q3)**: this still unexplored. Under
   interference_relax, xi drops to 0.220. If relax_steps=16+ recovers xi while keeping
   carrier_e high (0.714), the carrier↔xi tradeoff inside the mode becomes tractable.
   Worth a future code-change test.

5. **K-sweep under fixed plumbing (Q2)**: now that stage_sync reads kuramoto params,
   sweeping kuramoto_coupling may reveal the K where xi peaks. The R↔xi correlation
   prediction from the magic-gives-it-gravity doc is also testable.
