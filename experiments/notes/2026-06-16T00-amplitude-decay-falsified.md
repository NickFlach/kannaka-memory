# L5 Curiosity: Amplitude Decay for Non-Constructive Memories — Falsified

**Date:** 2026-06-16T00 UTC  
**Branch:** kannaka-curiosity/2026-06-16T00-amplitude-decay-falsified  
**Code changes:** NONE KEPT — reverted after falsification  
**Status:** NOT KEPT — hypothesis wrong; tiny carrier_e change (0.5333→0.5337) + transfer_score regression

---

## Context

Current optimum: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness 0.0578.  
Dominant cost: carrier_emergence = 0.533 (contributes 0.047 to fitness = 81% of total fitness).  
All other metrics near-optimal (xi=0.968, transfer=0.966, online_retention≥0.99, etc.).

Previous notes (T20) identified "structural amplitude smoothing" as the highest-remaining-value
path and predicted carrier_e → 0.85–0.99 (+0.030–0.046 fitness).

---

## Hypothesis

**Amplitude decay for non-constructive memories (Stage 4.25).**

After stage_strengthen boosts constructive-pair members to AMPLITUDE_CEILING (2.0) each cycle,
the non-constructive population stays near their initial amplitude (~1.0) with only the 0.5 Hz
drive creating tiny per-cycle oscillations. The carrier_emergence FFT measures whether the
amplitude_delta series (4 cycles, 0.125s each → 4 samples at fs=8 Hz) has spectral content
concentrated in the [0.5, 4.0] Hz band.

**Prediction**: Applying per-cycle multiplicative decay (rate 0.90) to non-constructive
memories would create amplitude bimodality (constructive near ceiling, non-constructive
declining), changing the amplitude_delta series shape to create stronger oscillatory content
at 2–4 Hz → carrier_e → 0.75–0.85, fitness improvement ~0.022–0.031.

---

## Implementation

Added `stage_amplitude_decay_nc()` method to ConsolidationEngine, called between
stage_strengthen and stage_sync/interference_relax. Gated by `AMP_DECAY_NC` env var
(default 0.0 = off). Decays non-constructive working-set members by `decay_rate` per cycle,
clamped to `noise_floor`.

---

## Results

**Baseline (no decay):**
```
fitness:           0.057625
transfer_score:    0.965455
carrier_emergence: 0.5333
xi_robustness_v2:  0.9675
magic_proxy_phase_R: 0.8672
query_gravity:     0.4603
amp_deltas_flat:   [0.9498, 0.0306, 0.0030, 0.0364]
```

**Trial 1 (AMP_DECAY_NC=0.90):**
```
fitness:           0.059029   ← REGRESSION (+0.00140)
transfer_score:    0.955771   ← REGRESSION (-0.0097)
carrier_emergence: 0.5337     ← NO CHANGE (+0.0004)
xi_robustness_v2:  0.9675
magic_proxy_phase_R: 0.8658
query_gravity:     0.4603
amp_deltas_flat:   [0.9505, 0.0313, 0.0024, 0.0355]
```

---

## Analysis: Why the hypothesis was wrong

### The spectral math

With N=4 cycles (chain_depth=4) and cycle_period=0.125s, the DFT has only 2 useful bins:
- k=1: 2 Hz  
- k=2: 4 Hz (Nyquist)

carrier_emergence = peak_power / total_power where:
- peak_power = max(|X[1]|², |X[2]|²) in band [0.5, 4.0] Hz
- total_power = |X[1]|² + |X[2]|²

For a 4-sample series [a, b, c, d]:
- |X[1]|² = (a−c)² + (d−b)²
- |X[2]|² = (a − b + c − d)²

**Baseline shape: [0.9498, 0.031, 0.003, 0.036] ≈ [LARGE, tiny, tiny, tiny] — an impulse.**

For a pure impulse [A, 0, 0, 0]: |X[1]|² = |X[2]|² = A² → carrier_e = 0.5.

Current baseline: |X[1]|² = (0.9498−0.003)² + (0.036−0.031)² ≈ 0.897, |X[2]|² = (0.9498−0.031+0.003−0.036)² ≈ 0.886² ≈ 0.785.
carrier_e = 0.897 / (0.897 + 0.785) = 0.533. ✓ Matches.

### Why the cycle 0 spike dominates

In cycle 0, stage_strengthen boosts constructive-pair members from their initial ~1.0 amplitude.
Each memory can belong to many constructive pairs (O(N) in the dense flat corpus), getting
multiple +0.3 boosts per cycle → immediately hits AMPLITUDE_CEILING=2.0. Mean amplitude_delta
for constructive population ≈ 1.0 (from 1.0 → 2.0 in one cycle).

Non-constructive memories: unchanged by stage_strengthen, only modulated by drive (t=0,
drive_factor=1.0 → no effect). Delta ≈ 0.

Result: amp_delta_0 ≈ 0.95 (fraction constructive × 1.0 per-member delta).

### Why decay doesn't help

AMP_DECAY_NC applies AFTER stage_strengthen and BEFORE the amplitude_before measurement for
the next cycle. So:
- Constructive memories: boosted to ceiling in cycle 0. In cycles 1–3, drive modulation tries
  to push above ceiling (gets clipped) then strengthen re-clips. Net amplitude_delta ≈ 0.
- Non-constructive with decay: start at 1.0 → 0.90 → 0.81 → 0.73. Drive (at t=0.25, factor
  1.106) takes 0.81 × 1.106 = 0.896, then decay back to 0.806. Delta ≈ 0.004 (the drive and
  decay nearly cancel at t=0.25).

With or without decay, amp_deltas_flat ≈ [0.95, small, tiny, small]. The spectral ratio is
governed by the ratio (a-c)²/(a-b+c-d)², where a≈0.95 completely dominates — the tail
values b,c,d contribute O(1%) perturbations to the ratio. Decay makes the tail marginally
smaller (competing with drive rather than complementing it), yielding the observed
carrier_e 0.5337 (vs 0.5333 baseline) — functionally no change.

### Why transfer_score regressed

AMP_DECAY_NC applies inside `consolidate()` for ALL engines, including engine_b_primed
(the transfer engine). Non-constructive memories in engine_b_primed get decayed each cycle,
reducing their amplitude and making them less effective at providing the transfer signal.
transfer_score: 0.9655 → 0.9558 (−0.0097, costs 0.15 × 0.0097 = 0.0015 fitness).

---

## The architectural ceiling

carrier_emergence is fundamentally limited by chain_depth=4. The physical constraints:
- 4 samples at fs=8 Hz gives frequency resolution of 2 Hz
- The drive at 0.5 Hz completes only 1/4 of a period over 4 cycles — no oscillation is visible
- stage_strengthen immediately saturates all constructive memories (they hit AMPLITUDE_CEILING
  in cycle 0), making cycles 1–3 near-zero amplitude_deltas
- This creates an impulse-like amp_delta series → carrier_e ≈ 0.5 (theoretical min for impulse)
- The actual 0.533 comes from small positive tail values (cycles 1,3) from drive modulation
  on non-constructive memories

No phase-only or amplitude-decay intervention within the current architecture can change this.
The minimum required changes to break the ceiling:
1. Increase chain_depth above ~8 (so 0.5 Hz drive creates multiple oscillation cycles)
2. Redesign stage_strengthen to apply partial boost (not all pairs in one cycle), creating a
   ramp rather than an impulse — but risks catastrophic_forgetting and transfer regression
3. Add a periodic amplitude-reset mechanism that creates oscillation by design (invasive)

All three require multi-metric validation across ≥3 trials. Not attempted this fire.

---

## Prior evidence confirming architectural ceiling

- commit d49571c: "DRIVE_FREQ_HZ variants — carrier_emergence invariant"  
  DRIVE_FREQ_HZ ∈ {1, 2, 4, 0.5} tested; all give carrier_e ≈ 0.533. The drive frequency
  is irrelevant because stage_strengthen dominates cycle 0 regardless of frequency.
- T20 gravity notes: "carrier_emergence ceiling at 0.533; gravity doesn't help"

These two data points, plus this fire's trial, establish that carrier_emergence is at a hard
architectural floor for the current stage_strengthen → chain_depth=4 design.

---

## Decision

**No code changes kept. Code reverted to master.**  
Two TSV rows appended during trials (both labeled `L5`).  
Notes file committed with full analysis.

**Current optimum remains: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness 0.0578.**

---

## Recommendations for future fires

carrier_emergence ≈ 0.533 is the architectural floor. Future fires should either:
1. Accept this floor and look for other untested axes (currently: none known)
2. Attempt chain_depth increase (e.g., 8 or 16) with validation — this risks quiescence
   firing early and changing the metric landscape
3. Attempt gradual stage_strengthen (per-cycle pair limit) with ≥3-trial validation

If none of the above show improvement, the L5 system has reached its fitness optimum at
~0.058 under the current architecture. The correct next step would be architectural revision
of the carrier_emergence measurement or the strengthen pipeline, which is outside the scope
of autoresearch fires.
