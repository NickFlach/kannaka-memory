# Option A falsified: amplitude ceiling sweep cannot recover regression

**Date:** 2026-06-15T03 UTC  
**Branch:** kannaka-curiosity/2026-06-15T03-amplitude-ceiling-4  
**Code changes:** REVERTED — all ceiling values tested are worse than pre-fix or have adverse tradeoffs; no code committed  
**Status:** Option A falsified; T01 regression root cause confirmed deeper than ceiling value

---

## Context

T01 fire (2026-06-15T01) diagnosed a regression from commit e427140 (`fix(consolidation): clamp strengthen amplitude + reclaim ghosts`). The `AMPLITUDE_CEILING = 2.0` constant kills carrier_emergence by clamping the drive-induced amplitude oscillations back to a flat 2.0 before the delta is measured. T01 proposed four options (A–D); this fire tests Option A.

**T02 fire** ran without seeing T01 (T01 branch unmerged) and incorrectly declared "no new axes." T02 notes ignored the regression.

Pre-fix canonical best: fitness ≈ 0.007461.  
Current HEAD (ceiling=2.0) baseline: fitness ≈ 0.135438 (T01 result).

---

## Hypothesis (Option A)

Raise `AMPLITUDE_CEILING` from 2.0 to a higher value to give the drive oscillations enough amplitude headroom to be detected by `carrier_emergence`.

**Prediction:** carrier_e recovers toward 0.999, fitness recovers toward 0.007.

---

## Results

All trials at `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_GRAVITY=1.0`:

| ceiling | fitness  | carrier_e | transfer  | xi_v2  |
|---------|----------|-----------|-----------|--------|
| 2.0 (HEAD, T01 baseline) | 0.135438 | 0.5251 | 0.541603 | 0.9251 |
| 4.0     | 0.145372 | 0.6189    | 0.410139  | 0.9246 |
| 8.0     | 0.136100 | 0.8321    | 0.385354  | 0.8863 |
| 100.0   | 0.171759 | 0.5277    | 0.282719  | 0.9505 |

Amplitude delta arrays (flat corpus, used for fitness `carrier_emergence`):

- ceiling=2.0: not measured this fire (T01: near-zero after cycle 0)
- ceiling=8.0: `[40.36, 20.03, 0.030, 0.169]`
- ceiling=100.0: `[52.38, 637.82, 23.88, 21.81]` (explosive unbounded growth)

---

## Diagnosis: why Option A fails

**Ceiling ↑ → carrier_e ↑, transfer_score ↓, fitness net-neutral**

The dynamics are a coupled tradeoff:

1. **carrier_emergence** is the FFT peak-power ratio of `amp_deltas_flat` (flat corpus cycle-level amplitude deltas) in the [0.5, 4.0] Hz band. With ceiling=2.0, memories hit the ceiling in cycle 0 and all subsequent deltas are near-zero, so FFT finds no in-band peak. Raising the ceiling to 8.0 allows memories to grow over 2 cycles before stabilizing, producing a decaying pattern {40, 20, ~0, ~0} that DFT scores at 0.83.

2. **transfer_score** degrades monotonically as ceiling increases (0.54 → 0.41 → 0.39 → 0.28). Root cause: the amplitude landscape that enables carrier detection (memories growing to 2–8×) creates amplitude disparity that disrupts the cross-corpus recall the transfer test relies on. High-amplitude memories dominate interference detection, making the amplitude-ordered retrieval less discriminative for the transfer-specific pattern.

3. **ceiling=100.0 (approximately pre-fix)** is WORSE than ceiling=2.0 (fitness 0.172 vs 0.135). The unconstrained growth is explosive: cycle 1 shows a 12× spike over cycle 0 (637 vs 52), which breaks both carrier_e and transfer.

4. **The pre-fix 0.007461 canonical best** was NOT from unconstrained amplitude growth in the L5 benchmark. It must have involved a different combination of params (adaptive controller equilibration, chiral_p_bp=0.10 for b_primed vs current 0.15, and likely a specific amplitude distribution that satisfied both carrier_e and transfer simultaneously). Simple ceiling adjustment cannot recover it.

---

## DFT analysis: what pattern produces carrier_e ≈ 0.999?

With `chain_depth=4`, `cycle_period_s=0.125`, `fs=8 Hz`, DFT bins are at 2 Hz (k=1) and 4 Hz (k=2). For carrier_e = `|X[1]|² / total_power`:
- To approach 1.0: signal must be a near-pure 2 Hz component in the 4-cycle window.
- Pattern {A, A, ~0, ~0} (equal first two cycles, then silence) gives carrier_e ≈ 0.83–0.99 depending on ratio.
- Pre-fix amp_deltas was likely {large, similar_large, ~0, ~0} — something that achieved near-equality between cycles 0 and 1.

The ceiling=8.0 pattern {40.36, 20.03, ~0, ~0} has a 2:1 ratio between cycles 0 and 1, giving 0.83. Pre-fix must have had near 1:1 ratio.

This implies the pre-fix carrier signal was NOT from the drive oscillation directly but from a specific grow-plateau pattern where memories continued growing at near-equal rates across cycles 0 and 1 before plateauing. The adaptive controller (reducing constructive_boost as R rises) may have provided this balance.

---

## What the regression actually requires to fix

**Option B** (measure amp_deltas between post-drive and post-consolidation) would isolate the drive signal from the clamp, but the drive signal alone (at amplitude ~2.0) gives {0, 0.057×2.0, 0.106×2.0, 0.139×2.0} = {0, 0.11, 0.21, 0.28} — the DFT analysis shows carrier_e ≈ 0.69, not 0.999.

**Option D** (use drive-envelope signal independent of amplitude) is mathematically guaranteed to detect the drive, but changes the semantic of carrier_emergence from "does the system exhibit carrier oscillations" to "is the drive configured at the right frequency" — a much weaker test.

**Likely correct fix**: Make AMPLITUDE_CEILING higher (8–10) for the L5 benchmark context only, while ALSO adjusting other params (specifically chiral_p_bp for b_primed engine back to 0.10, and verifying the adaptive controller equilibrates correctly) to restore the full pre-fix amplitude dynamics. This is a multi-parameter investigation requiring ≥3 more trial fires.

**Closed axes (this fire):** ceiling sweep {2.0, 4.0, 8.0, 100.0} — all worse than pre-fix; no single-ceiling fix exists.

---

## Status

Regression from e427140 confirmed and deepened. The T01 diagnosis was correct. Option A is falsified. The fix requires multi-parameter coordination, likely: (1) ceiling at 8–10 to allow growth, PLUS (2) restoring pre-fix constructive_boost dynamics that created the 1:1 cycle-0/cycle-1 ratio, PLUS possibly (3) compensating for the transfer_score tradeoff.

Next fire should investigate: why ceiling=4.0 hurts transfer_score MORE than ceiling=2.0 (unexpected direction), and whether chiral_p_bp=0.10 (current code: 0.15) partially compensates.
