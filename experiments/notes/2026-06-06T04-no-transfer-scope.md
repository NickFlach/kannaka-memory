# Hypothesis: DRIVE_SCOPE=no_transfer improves fitness

**Date:** 2026-06-06T04 UTC  
**Branch:** kannaka-curiosity/2026-06-06T04  
**Status:** HYPOTHESIS NOT CONFIRMED — no code changes, nothing to revert

---

## Hypothesis

`DRIVE_SCOPE=no_transfer` (already implemented: drives all engines EXCEPT
engine_b_primed and engine_b_naive) should combine:
- xi_robustness_v2 advantage of "all" scope (engine_a driven → better xi)
- transfer_score advantage of xi_and_flat scope (engine_b undisturbed)

**Prediction from T00 notes**: fitness ~0.144 (improvement over "all" ref).

This hypothesis was blocked last fire (T00) by missing sibling deps. Sibling deps
(`../consciousness-core`, `../kannaka-attention`) are present in this environment.
No code changes required.

---

## Experimental setup

- `DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DREAM_MODE=` (unset)
- Compared `DRIVE_SCOPE=all` (2 ref trials) vs `DRIVE_SCOPE=no_transfer` (3 trials)
- `magic_proxy_phase_R` and `query_gravity` logged but invariant to DRIVE_SCOPE
  (both depend on the dream sync step, not which engines are driven)

---

## Results

| condition     | trial | fitness  | xi_robust | transfer_score | carrier_e | R     | gravity |
|---------------|-------|----------|-----------|----------------|-----------|-------|---------|
| all (ref)     | 1     | 0.175137 | 0.5478    | 0.706831       | 0.5588    | 0.362 | 0.460   |
| all (ref)     | 2     | 0.142625 | 0.7634    | 0.706831       | 0.5588    | 0.362 | 0.460   |
| **all avg**   |       | **0.159** | **0.656** | **0.707**      | 0.559     | 0.362 | 0.460   |
| no_transfer   | 1     | 0.200633 | 0.3588    | 0.725206       | 0.5588    | 0.362 | 0.460   |
| no_transfer   | 2     | 0.157457 | 0.6467    | 0.725206       | 0.5588    | 0.362 | 0.460   |
| no_transfer   | 3     | 0.147940 | 0.7248    | 0.709696       | 0.5588    | 0.362 | 0.460   |
| **no_transfer avg** | | **0.169** | **0.577** | **0.720**  | 0.559     | 0.362 | 0.460   |

---

## Analysis

1. **transfer_score is consistently higher** under no_transfer (0.720 vs 0.707, Δ+1.3%).
   This matches the T22 observation that leaving engine_b undisturbed helps transfer.

2. **xi_robustness_v2 is consistently lower** under no_transfer (0.577 vs 0.656, Δ−7.9%).
   The prediction that driving only engine_a would preserve xi did not hold. Something
   in the engine_b drive interaction appears to help xi_robustness, not hurt it.

3. **Net fitness is worse**: 0.169 vs 0.159 — no_transfer does not improve on "all" scope.

4. **R, query_gravity, carrier_emergence are scope-invariant**: identical across
   conditions (R=0.362, gravity=0.460, carrier_e=0.559). These metrics depend on the
   dream sync mode (stage_sync vs interference_relax), not on the drive scope.

5. **xi_robustness_v2 variance is very high** (0.359–0.763 within a single condition).
   This makes per-trial comparisons unreliable; 3-run averages are the minimum viable
   sample size for this metric.

---

## Side observation: baseline drift

The system prompt cites "all" scope 3-run avg fitness ≈ 0.18 (pre-066d41a).
Today's "all" trials average 0.159. The Kuramoto plumbing fix (066d41a) brought
`coupling_strength=3.0` to stage_sync (was falling back to 1.0). This 3× coupling
increase likely accounts for the ~0.02 baseline improvement. The stated ~0.18 baseline
is now stale; future fires should treat ~0.16 as the current "all" reference.

---

## Decision

no_transfer fitness avg (0.169) does not beat "all" avg (0.159). No code changes
made. Nothing to revert.

**The transfer_score directional effect is real** (+1.3%) and worth noting, but the xi
cost outweighs it at the current fitness weights (xi: 0.15, transfer: 0.15 — equal
weight, and xi loss exceeds transfer gain).

---

## Next fire directions

1. **K-sweep under fixed plumbing** (question #2): `kuramoto_coupling` in {1.0, 2.0,
   5.0, 7.0} — current default 3.0 already tested. With plumbing fixed, this is now
   a real sweep instead of noise. Asks: where does xi peak?

2. **interference_relax characterization** (question #1): 3-run avg at
   `DREAM_MODE=interference_relax DRIVE_A=0.1 DRIVE_SCOPE=all`. One smoke-test
   trial per mode was run at 066d41a. Need stable averages.

3. **interference_relax relax_steps=16** (question #3): predict xi rises while R and
   carrier_e stay high. Quick 2-trial test with 1 code change.
