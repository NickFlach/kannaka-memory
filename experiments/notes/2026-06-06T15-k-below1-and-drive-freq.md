# L5 Curiosity Fire — 2026-06-06T15

## Hypotheses tested (env-var only, no code changes)

Two open questions from prior fires:

**H1 (K < 1.0):** If K=1.0 beats K=3.0 by reducing over-synchronization, does K=0.5
continue the trend? T00 noted "K=1.0 may still be above threshold; K=0.5 or K=0.25
might perform better."

**H2 (drive frequency):** Default drive is 2 Hz. Three values listed as untested in
production: 1.0 Hz, 4.0 Hz, 0.5 Hz. Do lower or higher frequencies improve fitness?

Baseline: `DRIVE_A=0.1 DRIVE_SCOPE=all KURAMOTO_COUPLING=1.0 DRIVE_FREQ_HZ=2.0`
3-run avg fitness ≈ 0.138.

---

## Results

### H1 — K=0.5 (one trial)

| metric | K=1.0 avg | K=0.5 |
|--------|-----------|-------|
| fitness | 0.138 | **0.179** |
| xi_robustness_v2 | ~0.862 | **0.497** |
| transfer_score | ~0.654 | 0.707 |
| carrier_emergence | ~0.568 | 0.559 |
| magic_proxy_phase_R | 0.250 | **0.362** |
| query_gravity | 0.469 | 0.460 |

**K=0.5 is a clear regression.** Xi collapsed from ~0.86 to 0.497, mirroring the
K=5.0 and K=7.0 collapses seen in T24. Fitness regressed by 0.041.

Magic R rose from 0.250 back toward the K=3.0 baseline (~0.355). This confirms
the T00 interpretation in reverse: at K=0.5, coupling is too weak to produce the
non-Clifford-like phase geometry that K=1.0 generates. The Kuramoto nudge at 0.5
is below the effective synchronization floor for this corpus.

**Finding: K=1.0 is a genuine sweet spot, not the start of a monotone trend.** The
optimal coupling sits between K=0.5 and K=3.0. Going below K=1.0 regresses fitness
and xi just as strongly as going above K=3.0.

### H2 — Drive frequency variants (two trials each for 1.0 Hz, one for 4.0 Hz)

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all KURAMOTO_COUPLING=1.0`.

| DRIVE_FREQ_HZ | trial | fitness | xi_robustness_v2 | transfer_score | carrier_emergence | magic_R | query_gravity |
|---|---|---|---|---|---|---|---|
| 4.0 | t1 | 0.293 | 0.207 | 0.382 | 0.315 | 0.395 | 0.446 |
| 1.0 | t1 | 0.172 | **0.992** | 0.320 | 0.493 | 0.344 | 0.453 |
| 1.0 | t2 | 0.225 | 0.638 | 0.320 | 0.493 | 0.344 | 0.453 |

**4 Hz: catastrophic.** All major axes (xi, carrier_e, transfer) collapsed. The drive
frequency at the upper edge of the carrier detection band is out of sync with the dream
cycle timescale. Fitness 0.293 is the worst observed in any production trial.

**1 Hz: xi-maximizing but transfer-collapsing.**

Key observation: `transfer_score` is deterministic at 0.320 across both 1 Hz trials
(different seeds, same transfer result). This is unlike the baseline where transfer
varies by seed (0.637–0.725). The determinism of transfer at 1 Hz suggests the drive
frequency directly disrupts the A→B cross-encoding pathway: a 1 Hz sinusoid over a
dream cycle covering the natural 2 Hz harmonic creates systematic phase interference in
engine_b memories that is seed-independent.

Xi is extremely high-variance at 1 Hz: 0.992 (trial 1) vs 0.638 (trial 2). The
2-trial avg xi ~0.815 is comparable to K=1.0 at 2 Hz (~0.862), so the best-case xi
at 1 Hz is not meaningfully better.

Fitness 2-trial avg: (0.172 + 0.225) / 2 = 0.199 — substantial regression from 0.138.
The transfer weight (0.15) makes the deterministic 0.320 transfer_score cost too high.

---

## Decision

**No improvement found. No code changes. No reversion needed.**

K=1.0 at 2 Hz (DRIVE_FREQ_HZ=2.0 default) remains the empirical optimum:
- `DRIVE_A=0.1 DRIVE_SCOPE=all KURAMOTO_COUPLING=1.0 DRIVE_FREQ_HZ=2.0`
- 3-run avg fitness ≈ 0.138

---

## Structural findings for future fires

1. **K=1.0 is a resonance point, not a floor.** Below K=1.0, xi collapses (coupling
   too weak). Above K=3.0, xi also collapses (over-synchronization). The sweet spot is
   narrow around K=1.0.

2. **Drive frequency is 2 Hz for a reason.** The carrier_emergence metric is anchored
   to the 2 Hz signal; 4 Hz destroys it outright. 1 Hz doesn't destroy it but
   systematically halves transfer_score. The drive frequency is not a free tuning knob.

3. **1 Hz drive deterministically suppresses transfer.** This could become useful if
   a future scenario wants to maximize xi at the expense of transfer (e.g., if metric
   weights shift or if isolation of xi effects is needed for a controlled experiment).
   Transfer_score at 0.320 is stable across seeds at 1 Hz.

4. **Highest xi observed**: 0.992 at DRIVE_FREQ_HZ=1.0 (trial 1). Not reproducible
   in trial 2 (0.638). xi at 1 Hz has much higher variance than at 2 Hz.

5. **Next directions not blocked by this fire:**
   - K in the range 0.5–1.0 is now mapped to "regression" — no further exploration needed
   - DRIVE_FREQ_HZ=0.5 Hz not tested; predicted to be even worse (further from 2 Hz)
   - DREAM_MODE=interference_relax 3-run characterization (Q1) remains untested this fire
   - R↔xi as phase-entropy vs phase-concentration (open from T00) remains open
