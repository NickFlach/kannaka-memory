# Hypothesis: DRIVE_SCOPE=no_transfer — first production test

**Date:** 2026-06-05T21 UTC  
**Branch:** kannaka-curiosity/2026-06-05T21  
**Status:** NULL RESULT — avg fitness 0.183, no improvement over 0.18 baseline

---

## Hypothesis

T00 proposed that `DRIVE_SCOPE=no_transfer` (drives all engines except
engine_b_primed and engine_b_naive) would combine two advantages:
- xi_robustness advantage of driving engine_a (as in "all" scope)
- transfer_score advantage of leaving engine_b undisturbed (as in xi_and_flat)

Predicted fitness: ~0.144 (improvement over xi_and_flat ref of ~0.154 from T22).

T00 was blocked by missing sibling deps. This fire runs in production with
consciousness-core and kannaka-attention available as siblings.

---

## Configuration

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer
DREAM_MODE unset (stage_sync path)
```

---

## Results (3 trials, rows appended to results-L5.tsv as label "L5")

| trial | fitness  | transfer | xi        | carrier_e | R      | query_gravity |
|-------|----------|----------|-----------|-----------|--------|---------------|
| t1    | 0.150874 | 0.709696 | 0.7058    | 0.5588    | 0.3623 | 0.4597        |
| t2    | 0.199247 | 0.725206 | 0.3679    | 0.5588    | 0.3623 | 0.4597        |
| t3    | 0.199482 | 0.702644 | 0.3914    | 0.5588    | 0.3623 | 0.4597        |
| **avg** | **0.183** | **0.712** | **0.489** | 0.5588 | 0.3623 | 0.4597 |

Baseline (DRIVE_SCOPE=all, 3-run context avg): fitness ≈ 0.18, transfer ~0.71

---

## Analysis

**Null result.** 3-run avg fitness 0.183 does not improve over the 0.18 baseline by
≥0.005. No code changes made; nothing to revert.

**The transfer_score prediction was wrong**: transfer at no_transfer (~0.712) is
essentially identical to "all" scope (~0.71 from context), not an improvement.
The T22 ref-all result showing transfer 0.422 (which drove the T00 prediction) was
anomalous or from a different code era — post-066d41a production "all" already
achieves transfer ~0.71.

**xi variance is unchanged**: xi is bimodal (0.37–0.71) under no_transfer, same
pattern as under "all" scope. Protecting engine_b from drive doesn't reduce xi
variance.

**magic_proxy_phase_R and query_gravity are constant** across all 3 trials (R=0.362,
qg=0.460), suggesting these are deterministic given the same DRIVE_SCOPE/DREAM_MODE
and only xi varies stochastically.

---

## Why transfer_score is already high under "all" scope

The T22 ref-all result (transfer 0.422) was labelled `hyp-xi-flat.ref-all` and
preceded the Kuramoto-plumbing commit 066d41a. After 066d41a plumbed params through
stage_sync (new behavior even for default K), the baseline transfer dynamics appear
to have shifted. The no_transfer vs all distinction may have mattered under the old
hard-coded stage_sync but no longer does under the plumbed version.

---

## Next fire directions

1. **K-sweep (priority)**: now that K actually reaches stage_sync, sweep
   kuramoto_coupling in {1.0, 2.0, 3.0, 5.0, 7.0} to find where xi peaks
   (question 2 from fire context). This is the most informative untested axis.
2. **interference_relax characterization**: 3-run characterization of
   DREAM_MODE=interference_relax to get stable xi, R, query_gravity averages
   (single-trial smoke test showed xi=0.220, R=0.612 — high variance likely).
3. **relax_steps sweep**: try relax_steps=16 or 24 in stage_interference_relax
   to test whether xi recovers while R stays high (question 3 from context).
