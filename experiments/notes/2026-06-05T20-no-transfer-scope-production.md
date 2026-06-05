# Hypothesis: DRIVE_SCOPE=no_transfer — production test

**Date:** 2026-06-05T20 UTC  
**Branch:** kannaka-curiosity/2026-06-05T20  
**Status:** NEGATIVE — hypothesis rejected; no code changes to revert

---

## Hypothesis

`DRIVE_SCOPE=no_transfer` drives all engines EXCEPT engine_b_primed and engine_b_naive.
Compared to "all" scope, engine_b is undisturbed during the dream chain.

**Prediction (from T00 notes):**
- transfer_score improves (~0.422 → ~0.486) because engine_b is unperturbed
- xi_robustness_v2 stays as high as "all" because engine_a IS still driven
- Expected fitness: ~0.144 (improvement over "all" ~0.18 baseline)

This fire: first time running in production with sibling deps available (T00 was blocked by
missing consciousness-core and kannaka-attention path deps).

---

## Results

All runs: `DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=<scope> --level 5`

| run | scope | fitness | transfer_score | xi_robustness_v2 | carrier_emergence | magic_proxy_phase_R | query_gravity |
|-----|-------|---------|----------------|-------------------|-------------------|---------------------|---------------|
| ref | all   | 0.161   | 0.707          | 0.641             | 0.559             | 0.362               | 0.460         |
| t1  | no_transfer | 0.193 | 0.710       | 0.422             | 0.559             | 0.362               | 0.460         |
| t2  | no_transfer | 0.165 | 0.710       | 0.609             | 0.559             | 0.362               | 0.460         |
| t3  | no_transfer | 0.135 | 0.710       | 0.813             | 0.559             | 0.362               | 0.460         |
| **avg** | **no_transfer** | **0.164** | **0.710** | **0.615** | 0.559 | 0.362 | 0.460 |

---

## Analysis

**Prediction wrong on both counts.**

1. **transfer_score**: Improved by exactly 0.003 (0.707 → 0.710), not the ~0.064 gain T00 predicted.
   Notably transfer_score is **deterministic per scope** — identical across all three no_transfer trials.
   This means the scope directly determines the transfer evaluation context and
   noise cancels out. The gain is real but tiny.

2. **xi_robustness_v2**: NOT maintained at "all" levels. Average 0.615 vs 0.641 reference, with
   substantially higher variance (range 0.422–0.813 vs the "all" ref at 0.641). The T00 reasoning
   was that "engine_a IS still driven, so xi should be unaffected." In practice, removing the
   engine_b drive reduces the scope's effectiveness for xi recovery.

3. **Fitness**: 3-run avg 0.164 vs "all" ref 0.161 — marginally worse, not distinguishable from
   noise. The predicted ~0.144 did not materialize.

**Secondary observation**: magic_proxy_phase_R (0.362) and query_gravity (0.460) are identical
across all runs regardless of scope. These are stable under scope changes — the stage_sync
behavior and attention-as-gravity dynamics are not sensitive to which engines are driven.

---

## Decision

No improvement. No code changes. Hypothesis rejected.

The T00 prediction was based on T22 production results, but those results appear not to have
accounted for the xi variance or the transfer mechanism correctly.

---

## Next fire directions

1. **K-sweep under fixed stage_sync plumbing** (question 2 from system context): vary
   `kuramoto_coupling` in {1.0, 2.0, 3.0, 5.0, 7.0} at DRIVE_A=0.1 DRIVE_SCOPE=all.
   Now that K reaches stage_sync (since commit 066d41a), this is a real signal.
   Look for xi peak and R correlation. Env-var only, no code changes.

2. **interference_relax 3-run characterization** (question 1): smoke test (T from 066d41a)
   was 1 trial each. Run 3 trials at DREAM_MODE=interference_relax DRIVE_A=0.1 DRIVE_SCOPE=all
   for a stable avg. xi=0.220 in smoke test is poor; is that stable or high-variance like xi
   under stage_sync?

3. **relax_steps tuning** (question 3): if interference_relax xi stays low, raise relax_steps
   8→16 and check if xi recovers. Alpha_base + envelope_depth are conserved.
