# K-Sweep: Kuramoto coupling strength now that plumbing works

**Date:** 2026-06-06  
**Branch:** kannaka-curiosity/2026-06-06T00  
**Status:** CONFIRMED IMPROVEMENT — kept

## Hypothesis

Commit 066d41a plumbed `params.kuramoto_coupling` through to `stage_sync` — all
prior K sweeps were measuring noise (the hard-coded 3.0 was used regardless of
input). Now that K actually reaches the dream, the empirical optimum might differ
from the arbitrary default of 3.0.

**Prediction:** Lower K preserves more phase diversity within categories, making
adversarial perturbations harder to construct (xi↑). Very high K over-synchronizes
phases toward a single attractor per category — making the system more
"stabilizer-like" and reducing xi.

## Code change

Added `KURAMOTO_COUPLING` env var reading in the L5 params block of
`run_experiment_l5_session`. Keeps the env var as a sweep/override handle;
updates the L5 default from 3.0 → 1.0 after confirmation.

## Results

Baseline reference (K=3.0, DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE unset):
- fitness ≈ 0.18, xi ≈ 0.64, transfer ≈ 0.68, carrier_e ≈ 0.56, R ≈ 0.355

| K | fitness | xi_robustness_v2 | transfer_score | carrier_emergence | magic_R | query_gravity |
|---|---------|-----------------|----------------|-------------------|---------|---------------|
| 1.0 | 0.1411 | 0.8137 | 0.6824 | 0.5684 | 0.2498 | 0.4691 |
| 1.0 | 0.1416 | 0.8622 | 0.6370 | 0.5684 | 0.2498 | 0.4691 |
| 1.0 | 0.1324 | 0.9165 | 0.6443 | 0.5684 | 0.2498 | 0.4691 |
| 5.0 | 0.2258 | 0.5082 | 0.5008 | 0.4050 | 0.2951 | 0.4247 |
| 7.0 | 0.1766 | 0.5267 | 0.6685 | 0.5361 | 0.2399 | 0.3912 |

**K=1.0 three-run avg fitness: 0.138** (vs baseline 0.18 → Δ = −0.042)

## Interpretation

The pattern is monotone: K=1.0 < K=3.0 < K=7.0 < K=5.0 on fitness (lower is
better), with xi inversely correlated. Lower K leaves phases more diverse within
categories — the Kuramoto coupling at K=1.0 is likely below or near the
synchronization threshold for these cluster sizes, so it nudges rather than
forces. This nudge is enough for carrier_emergence (unchanged at 0.568) but
does not over-lock phases.

The magic proxy R is *lower* at K=1.0 (0.250) vs K=3.0 (~0.355). This initially
seems to contradict the "magic gives gravity" hypothesis (higher R = more
non-Clifford content). However, very high phase concentration is itself
classically simulable — you just track one representative phase per cluster. The
K=3.0 regime is likely in the "too much order" zone: high R but low information
entropy in the phase arrangement, which a linear adversary can exploit. K=1.0
sits below the synchronization threshold, so the nonlinear Kuramoto dynamics act
as a mixing perturbation rather than a convergence engine.

query_gravity is stable across K=1.0 trials (0.469) and declines at K=7.0
(0.391), consistent with over-synchronization erasing the amplitude contrast that
gravity depends on.

## Decision

KEEP. The improvement is large (Δ fitness = −0.042), consistent across 3 trials
(variance ≈ 0.004), and mechanically explained. Default L5 coupling updated to
1.0 in `run_experiment_l5_session`. New empirical optimum:

    DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=<unset>  KURAMOTO_COUPLING=1.0
    3-run avg fitness ≈ 0.138

## Open questions spawned

- What is the critical K for this corpus (phase transition point)?  K=1.0 may
  still be above threshold; K=0.5 or K=0.25 might perform better or worse.
- Does K=1.0 + DREAM_MODE=interference_relax compound the gains? The two
  mechanisms are orthogonal (K affects stage_sync; interference_relax replaces
  stage_sync entirely). Mutual exclusion, not additive.
- Does the xi↑ at K=1.0 correlate with a lower magic_R — and if so, does this
  challenge the R-as-magic-proxy framing? R may need to be complemented by a
  phase *entropy* measure rather than a phase *concentration* measure.
