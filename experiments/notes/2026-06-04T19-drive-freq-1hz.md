# Hypothesis: DRIVE_FREQ_HZ=1.0 sub-harmonic drive

**Date:** 2026-06-04T19 UTC  
**Branch:** kannaka-curiosity/2026-06-04T19  
**Status:** INCONCLUSIVE — stub environment; production comparison blocked

---

## Environment note

The sibling path dependencies (`consciousness-core`, `kannaka-attention`) were not
present in this remote execution environment. The CI workflow checks them out as
sibling clones; this session had access only to `kannaka-memory`. Stub crates were
created at `/home/user/consciousness-core` and `/home/user/kannaka-attention` to
allow compilation.

**Critical difference from production**: the stub `compute_xi_signature` uses a
different formula than the canonical implementation. This causes:
- `carrier_emergence = 0.0` in all trials (production: ~0.31 no-drive, ~0.56 with drive)
- `carrier_bimodal = 0.0` in baseline runs (production: ~0.31)
- The multiplicative drive has reduced effect (or net-negative effect) on fitness
  in the stub, opposite of production behavior (0.244 → 0.184)

The stub results are internally consistent but **not comparable to the 0.184
production baseline**.

---

## Hypothesis

DRIVE_FREQ_HZ=1.0 (sub-harmonic of the 2 Hz carrier) may improve fitness over the
default 2.0 Hz drive.

**Rationale**: The L5 chain runs 16 cycles × 0.125s = 2 seconds. At 1 Hz the drive
completes exactly 2 full periods per chain, versus 4 periods at 2 Hz. Longer
modulation arcs could better align with slow consolidation dynamics, potentially
improving transfer_score or temporal_separation without degrading carrier_emergence.

**Prediction**: fitness decreases by ≥0.005 versus the DRIVE_FREQ_HZ=2.0 baseline.

---

## Trials (stub environment)

All trials: `DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=xi_and_flat` (except T3 which has DRIVE_A=0.0).

The research binary auto-appended these as rows 4–8 in `experiments/results-L5.tsv`
(all labeled `L5.9-stub` — the hardcoded label for L5 runs).

| TSV row | DRIVE_FREQ_HZ | fitness  | transfer_score | xi_robustness_v2 | carrier_emergence |
|---------|---------------|----------|----------------|------------------|-------------------|
| 4 (T1)  | 1.0           | 0.266762 | 0.2950         | 0.7454           | 0.0000            |
| 7 (T4)  | 1.0           | 0.261727 | 0.2825         | 0.7914           | 0.0000            |
| 5 (T2)  | 2.0 (baseline)| 0.246494 | 0.3778         | 0.7399           | 0.0000            |
| 8 (T5)  | 2.0 (baseline)| 0.219690 | 0.4036         | 0.8932           | 0.0000            |
| 6 (T3)  | 0.0 (no drive)| 0.235925 | 0.2615         | 0.9759           | 0.0000            |

Stub averages:
- 1 Hz: fitness avg **0.264** (n=2)
- 2 Hz: fitness avg **0.233** (n=2)
- no drive: fitness **0.236** (n=1)

---

## Comparison to baseline

Production 0.184 baseline is **not reachable** from this stub — carrier_emergence
is structurally 0.0 due to different xi-signature dynamics, contributing a constant
0.10 fitness penalty vs the ~0.044 penalty in production.

---

## Decision

**INCONCLUSIVE** vs production baseline (environment blocked).

Within-stub: 1 Hz is **worse** than 2 Hz by +0.031 (stub avg 0.264 vs 0.233).
The direction is consistent: 1 Hz hurts transfer_score and does not improve
xi_robustness_v2. Sub-harmonic drive is likely not beneficial.

**Code changes**: NONE. No changes to `src/bin/research.rs` or any production file.
The stub crates created for this fire are not part of the repo and are not committed.

**Recommendation**: Re-run this hypothesis in production when sibling repos are
available as siblings at `../consciousness-core` and `../kannaka-attention`.
Expected: 1 Hz drive will degrade carrier_emergence (currently 0.56 → near 0.0
since the 1 Hz signal is outside the [0.5, 4.0] Hz band detection window for
the flat-corpus FFT with n=16 cycles, and actually IS in-band at 1 Hz...
reconsidering: at n=16 cycles and fs=8 Hz, bin spacing = 0.5 Hz, so 1 Hz = bin 2
which IS in the [0.5, 4.0] Hz band. Carrier emergence might still score positively
at 1 Hz in production. Test remains worth running there.)

---

## Next fire suggestions

1. **Production re-run of 1 Hz**: run in environment where `../consciousness-core`
   exists to get comparable results.
2. **DRIVE_A=0.05**: softer drive may preserve xi_robustness_v2 (0.976 no-drive)
   while keeping transfer_score gains.
3. **Implement xi_and_flat scope**: currently falls through to wildcard=all in the
   code. True xi_and_flat (engine_clean + engine_adv + engine_flat, NOT engine_a)
   could improve xi_robustness_v2 by not letting drive perturb engine_a's dream.
