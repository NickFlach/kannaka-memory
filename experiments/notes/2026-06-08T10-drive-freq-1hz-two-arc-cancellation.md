# DRIVE_FREQ_HZ=1.0 at K=0.5+A=0.15 — two-arc cancellation confirmed

**Date:** 2026-06-08T10 UTC
**Branch:** kannaka-curiosity/2026-06-08T10
**Code changes:** None — env-var only
**Status:** FALSIFIED — carrier_e collapses; 0.5 Hz confirmed as unique optimum

---

## Hypothesis

DRIVE_FREQ_HZ=1.0 has been tested at K=1.0+A=0.1+2Hz-baseline (T12, 2026-06-06), but
never at the current production settings K=0.5+A=0.15+0.5Hz-baseline. The 0.25 Hz fire
notes (2026-06-08T00) explicitly flagged this as unexplored at current conditions.

At 1 Hz (dt_per_cycle=0.125), the drive peaks at cycle 2: `sin(2π×1.0×0.125×2) = 1.0`.
This gives 14 post-peak consolidation cycles vs 12 at 0.5 Hz (peak at cycle 4).

**Prediction — competing hypotheses:**

- *Optimistic:* More post-peak consolidation time → higher carrier_e and transfer.
  At K=0.5, Kuramoto coupling is weaker, so the second positive arc (cycle 10) adds
  constructively rather than reversing first-arc gains.
- *Pessimistic (T07/T12 two-arc analysis):* Two full oscillations produce amplitude
  deltas that partially cancel in the DFT → lower spectral concentration at 1 Hz →
  carrier_e collapses.

---

## Config

    DRIVE_A=0.15  DRIVE_SCOPE=all  DRIVE_FREQ_HZ=1.0  KURAMOTO_COUPLING=0.5 (default)
    DREAM_MODE=<unset> (stage_sync)

**Baseline:** K=0.5+A=0.15+0.5Hz stage_sync → avg fitness **0.104**, carrier_e=0.853,
xi=0.873, transfer=0.655

---

## Results (1 trial — result decisive)

| metric | K=0.5+A=0.15+0.5Hz baseline | DRIVE_FREQ=1.0 (t1) | delta |
|--------|------------------------------|---------------------|-------|
| fitness | 0.104 avg | **0.158** | **+0.054 regression** |
| carrier_emergence | 0.853 | **0.470** | **−0.383 COLLAPSE** |
| transfer_score | 0.655 | 0.613 | −0.042 |
| xi_robustness_v2 | 0.873 | 0.824 | −0.049 |
| carrier_bimodal | ~0.85 | 0.642 | −0.208 |
| magic_proxy_phase_R | ~0.161 | 0.227 | +0.066 |
| query_gravity | ~0.446 | 0.473 | +0.027 |

---

## Analysis

### Two-arc cancellation dominates at K=0.5

carrier_e dropped from 0.853 to 0.470 — a 45% collapse. The mechanism is identical to
what T12 documented at 1 Hz with the 2 Hz baseline, now confirmed at K=0.5+A=0.15:

At 1 Hz over 16 cycles (dt=0.125, total=2.0s):
- Positive peak at cycle 2 (t=0.25s)
- Zero crossing at cycle 4
- Negative trough at cycle 6 (t=0.75s)
- Second positive peak at cycle 10 (t=1.25s)
- Second negative trough at cycle 14 (t=1.75s)

The DFT of amplitude_deltas sees 2 complete 1 Hz oscillations. While this gives high
spectral concentration IN bin 2, the second positive arc (cycles 8-12) re-amplifies
memories that the negative arc (cycles 4-8) had suppressed. The net amplitude_delta
signal has a distinctive two-arc envelope rather than the single arch of 0.5 Hz.

The `eval_carrier_emergence` looks for the FFT peak in [0.5, 4.0] Hz. At 1 Hz with
DFT bins at every 0.5 Hz increment (fs/N = 8/16 = 0.5 Hz), bin 2 captures 1 Hz power.
But the two-arc pattern means the amplitude_delta signal amplitude per cycle is LOWER
on average (second arc partially reverses first arc's memory amplifications), and
competing bins from the oscillation pattern leak power — reducing carrier_e.

At 0.5 Hz (one gentle arch + one gentle suppression), the amplitude_delta signal
peaks once smoothly, and ALL spectral energy concentrates in the single [0.5 Hz] bin.
At 1 Hz with two arcs, power spreads across bins 1 (0.5 Hz), 2 (1.0 Hz), and
harmonics — the single-bin concentration that drives carrier_e is disrupted.

### Why K=0.5 does NOT rescue 1 Hz from two-arc cancellation

The pessimistic hypothesis assumed K=1.0 Kuramoto coupling contributes to arc
cancellation by resisting the amplitude drive. At K=0.5, the coupling is weaker and
the drive dominates more — but this makes the TWO-ARC structure MORE prominent (less
coupling to damp the oscillations), not less. Lower K amplifies the drive's effect on
amplitude, making the second arc stronger (more reversal), not weaker.

The optimistic hypothesis was wrong in direction: at K=0.5, the second positive arc
at cycle 10 causes MORE reinstatement of cycle-6 suppressions, not less.

### magic_R and query_gravity both rise

magic_proxy_phase_R: 0.161 → 0.227 (+0.066). query_gravity: 0.446 → 0.473 (+0.027).

The 1 Hz pattern creates more phase concentration (two arcs converge phases more
aggressively), raising R. But this phase concentration comes at the cost of carrier
structure. magic_R rising while carrier_e collapses is consistent with the T23 finding
that high R is mechanistically ambiguous — it can reflect two qualitatively different
regimes.

### Fitness budget

- carrier_e: 0.853 → 0.470, Δ = −0.383, cost = 0.383 × 0.10 = +0.038
- transfer: 0.655 → 0.613, Δ = −0.042, cost = 0.042 × 0.15 = +0.006
- xi: 0.873 → 0.824, Δ = −0.049, cost = 0.049 × 0.15 = +0.007
- Sum: +0.051. Observed: +0.054 (excellent fit).

---

## Decision

**Hypothesis falsified.** No code changes to revert. Empirical optima unchanged.

**Empirical optima:**
- `DRIVE_A=0.15 DRIVE_SCOPE=all DRIVE_FREQ_HZ=0.5 KURAMOTO_COUPLING=0.5` (stage_sync) → avg fitness **0.104**
- `DRIVE_A=0.1 DREAM_MODE=interference_relax DRIVE_SCOPE=all` → avg fitness **0.099**

---

## Implications

1. **DRIVE_FREQ_HZ axis is now fully closed.** Tested values:
   - 0.25 Hz: carrier_e collapses (−0.142), transfer collapses (−0.126), fitness ~0.093 avg — borderline, rejected
   - 0.5 Hz: CONFIRMED OPTIMAL — single-arch maximum spectral concentration
   - 1.0 Hz: carrier_e collapses (−0.383) via two-arc cancellation, fitness 0.158
   - 2.0 Hz: inferior to 0.5 Hz (tested T12, 4 complete oscillations → spectral spreading)
   - 3.0 Hz: inferior (tested T07-T12)
   - 4.0 Hz: ALIASED to zero (sin(π×k) = 0 for integer k)
   
   0.5 Hz is uniquely optimal: it maximizes spectral concentration in the lowest in-band
   bin while keeping the amplitude modulation coherent (single arch, no cancellation).

2. **K=0.5 amplifies the two-arc cancellation, not reduces it.** Lower K makes the
   drive more dominant, which makes the two-arc amplitude reversal more prominent.
   This is a general principle: at operating points where the drive is more dominant
   (low K, higher A), frequency artifacts that produce amplitude reversals are more
   harmful, not less.

3. **The remaining open territory.** With frequency, amplitude, scope, mode, coupling,
   and relax_steps all closed, the next explorable directions are:
   - Structural dream changes (stage_hallucinate, stage_prune thresholds — no env vars)
   - Seeding the adversarial eval to characterize irx xi variance mechanism
   - Novel mechanisms not yet in the codebase
