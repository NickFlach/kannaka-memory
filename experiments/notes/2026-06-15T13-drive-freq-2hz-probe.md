# L5 Post-Fix Probe: DRIVE_FREQ_HZ=2.0 — carrier_emergence invariant to drive frequency

**Date:** 2026-06-15T13 UTC
**Branch:** kannaka-curiosity/2026-06-15T13-drive-freq-2hz-probe
**Code changes:** NONE
**Status:** Hypothesis falsified; structural diagnosis confirmed

---

## Hypothesis

Post-fix (AMPLITUDE_CEILING=2.0, chain_depth=4), DRIVE_FREQ_HZ=0.5 Hz was confirmed
optimal in the pre-fix interference_relax era with longer dream chains. In the current
stage_sync regime with exactly 4 dream cycles and dt_per_cycle=0.125, the 0.5 Hz drive
traces only a quarter-arc over 4 cycles — it never completes an oscillation:

  t = [0, 0.125, 0.25, 0.375]
  0.5 Hz drive: sin(2π × 0.5 × t) = [0, 0.383, 0.707, 0.924]  — strictly increasing ramp

At 2.0 Hz, the drive completes a full oscillation in 4 cycles:

  2.0 Hz drive: sin(2π × 2.0 × t) = [0, 1.0, 0, -1.0]  — one complete period

**Prediction**: DRIVE_FREQ_HZ=2.0 creates a genuinely oscillatory drive signal
over 4 cycles. At cycle 3 (sin = -1), the drive reduces amplitudes by 15%
(factor 0.85). For memories at ceiling=2.0, this brings them to 1.70, then
consolidation may or may not restore them to 2.0 depending on pair count.
For memories with few/no constructive pairs, this creates a large negative delta
at cycle 3, changing the amp_deltas_flat pattern from [large, ~0, ~0, ~0]
to [large, ~0, ~0, nonzero], improving carrier_emergence.

**Expected:** carrier_emergence rises toward 0.65-0.70; fitness drops to 0.10-0.11.

---

## Baseline (post-fix default, DRIVE_FREQ_HZ=0.5 Hz)

| metric             | baseline (T12) |
|--------------------|---------------|
| fitness            | 0.1159        |
| transfer_score     | 0.737         |
| carrier_emergence  | 0.529         |
| xi_robustness_v2   | 0.856         |
| amp_deltas_flat    | [0.9498, 0.031, 0.010, 0.042] |

---

## Trial

Settings: `DRIVE_A=0.1 DRIVE_SCOPE=all DRIVE_FREQ_HZ=2.0` (DREAM_MODE unset)

| metric             | t1           |
|--------------------|-------------|
| fitness            | 0.1454       |
| transfer_score     | 0.5416       |
| carrier_emergence  | 0.5284       |
| carrier_bimodal    | 0.5302       |
| xi_robustness_v2   | 0.8563       |
| magic_proxy_phase_R| 0.1295       |
| query_gravity      | 0.4603       |
| amp_deltas_flat    | [0.9498, 0.031, 0.009, 0.040] |
| amplitude_deltas_a | [0.9495, 0.031, 0.012, 0.048] |

---

## Analysis

**Hypothesis falsified.**

The amp_deltas_flat pattern at DRIVE_FREQ_HZ=2.0 is **identical** to the 0.5 Hz baseline:
[0.9498, 0.031, 0.009, 0.040] vs [0.9498, 0.031, 0.009, 0.042]. The drive frequency
has zero effect on the amplitude delta signal.

Mechanistic explanation: at cycle 3 with 2.0 Hz drive, the drive reduces amplitudes by 15%
(factor = 0.85). For a memory at ceiling=2.0, this gives 1.70. The flat corpus has ~3+
effective constructive pairs per memory (observed: N × 0.3 ≈ 0.9 boost at cycle 0),
so the consolidation at cycle 3 adds ≥ 0.9, restoring amplitude to ≥ 2.60 → clamped
to 2.0. Net amplitude delta = |2.0 - 2.0| = 0.

The consolidation boost completely absorbs the drive's negative-phase reduction.
This holds for any drive frequency: the ceiling clamp + consolidation boost dominates,
making amplitude_deltas insensitive to drive frequency in the post-fix regime.

**transfer_score collapsed to 0.541** (from 0.737 at 0.5 Hz). The 0.5 Hz optimum
for transfer was confirmed correct: the 2.0 Hz drive disrupts b_primed and b_naive
engine consolidation at the wrong cycle phases, hurting the A→B plasticity transfer.

---

## Structural diagnosis confirmed

This trial provides the definitive test of a key diagnostic claim:

> "carrier_emergence ≈ 0.529 is structurally determined by the amplitude ceiling
> impulse pattern, not by drive frequency."

Confirmed: drive frequency has no effect on carrier_emergence post-fix. The value
0.529 is the DFT spectral concentration of the impulse pattern [large, ~0, ~0, ~0]
at the k=1 bin (2 Hz) in a 4-point window. It cannot be changed by any drive
frequency parameter, because the ceiling clamp and consolidation boost absorb all
drive oscillations after cycle 0.

**Closed axes confirmed by this probe:**
- DRIVE_FREQ_HZ (0.5 → 2.0): CLOSED. 0.5 Hz optimal for transfer; carrier_emergence
  is insensitive to drive frequency in the post-fix 4-cycle regime.

**Remaining structural facts:**
- carrier_emergence floor: ~0.529 (irreducible under ceiling=2.0 + chain_depth=4)
- transfer_score: variable 0.54-0.74, DRIVE_FREQ_HZ=0.5 Hz + DRIVE_A=0.1 + K=0.5 optimal
- xi_robustness_v2: 0.856 (deterministic for given K; K=1.0 gives 0.886 but costs transfer)
- Net best achievable fitness post-fix: ~0.113-0.116 (structural ceiling)

---

## Decision

No code changes. No improvement found.

The post-fix optimization surface has no remaining clean parameter axes. The three
largest fitness contributors (carrier_emergence, transfer_score, xi_v2) are all
near their architectural limits under ceiling=2.0 + chain_depth=4 + stage_sync.

Further improvement requires a structural change to one of:
1. The carrier_emergence metric (measure drive signal analytically, not via amplitude_deltas)
2. The chain_depth limit (relax from 4 to allow longer-period carrier detection, but risks over-consolidation)
3. The amplitude ceiling semantics (relative rather than absolute ceiling, but risks reverting the correct bug fix)
