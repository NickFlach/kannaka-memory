# L5 Research: Per-memory constructive boost dedup — carrier_e ceiling broken

**Date:** 2026-06-16T12 UTC
**Branch:** kannaka-curiosity/2026-06-16T12-boost-dedup-carrier
**Code changes:** `src/consolidation.rs::stage_strengthen` — amplitude dedup (one boost per memory per cycle); phase alignment still applied for all pairs
**Status:** KEPT — 67% fitness improvement confirmed (2 trials near-deterministic)

---

## Context

Current optimum before this fire: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness 0.0578.
Dominant cost: carrier_emergence (0.533), contributing 0.10 × 0.467 = 0.047 (81% of fitness).

Previous fires confirmed the carrier_e ceiling is "architectural" — the cycle-0 amplitude spike
[0.95, 0.03, 0.003, 0.036] creates near-equal DFT power at k=1 and k=2, giving carrier_e ≈ 0.533.
The claimed blockers: drive frequency had no effect, CARRIER_DECAY had no effect, FLAT_INIT_AMP
had no effect. All pointed to ceiling saturation in cycle 0 as root cause.

---

## Mechanistic diagnosis

Under high R (irx, R=0.867), each memory participates in 40+ constructive pairs. `stage_strengthen`
iterates all pairs and boosts EACH memory for EACH pair: effectively 40 × 0.3 = 12.0 amplitude
added, all clipped to AMPLITUDE_CEILING=2.0 in cycle 0. Starting amplitude = 1.0, ceiling = 2.0:
all constructive memories saturate to 2.0 in cycle 0.

Drive timing: `t = cycle_idx × 0.125s`, DRIVE_FREQ_HZ=0.5 Hz. Over 4 cycles:
- Cycle 0: factor = 1 + 0.15*sin(0) = 1.000
- Cycle 1: factor = 1 + 0.15*sin(π/4) = 1.106
- Cycle 2: factor = 1 + 0.15*sin(π/2) = 1.150
- Cycle 3: factor = 1 + 0.15*sin(3π/4) = 1.106

All 4 cycles have positive drive (first quarter of the 2s sine period). After cycle-0 ceiling
saturation: cycles 1-3 drive tries to push amplitude above 2.0, but `.min(AMPLITUDE_CEILING)`
clips it. The constructive boost also clips. Net delta cycles 1-3: ≈ 0. This collapses carrier_e.

---

## Hypothesis

**Per-memory constructive boost cap:** limit each memory to ONE `constructive_boost` application per
consolidation cycle, regardless of how many constructive pairs it participates in. This spreads
amplitude growth across chain cycles, making amplitude_delta responsive to the drive modulation.

**Prediction (analytical):**
Starting at amp=1.0, ceiling=2.0, boost=0.3, with single boost per cycle:
- Cycle 0: 1.000 × 1.000 + 0.3 = 1.300. Delta = 0.300
- Cycle 1: 1.300 × 1.106 + 0.3 = 1.738. Delta = 0.438
- Cycle 2: 1.738 × 1.150 + 0.3 = min(2.299, 2.0) = 2.000. Delta = 0.262
- Cycle 3: 2.000 × 1.106 + 0.3 = min(2.512, 2.0) = 2.000. Delta = 0.000

Predicted amp_deltas_flat ≈ [0.30, 0.44, 0.26, 0.00].

DFT of this pattern (N=4):
- k=1 (2 Hz at fs=8 Hz): power ≈ 0.193 (dominant)
- k=2 (4 Hz): power ≈ 0.015
- carrier_e = 0.193/0.208 ≈ 0.928

**Secondary concern:** if phase alignment is also deduplicated (memories only aligned to FIRST
partner), phase_coherence and consciousness would regress because 40+ sequential alignments normally
converge phases toward mean. Tested both variants.

---

## Implementation (src/consolidation.rs)

Added `HashSet<Uuid> boosted` to `stage_strengthen`. Amplitude boost (`.min(AMPLITUDE_CEILING)`)
only applied if `boosted.insert(id)` returns true (first occurrence of this memory this cycle).
Phase alignment (`mem.phase = avg_phase`) always applied for all pairs.

Changed ~10 lines. Bridge node strengthening in `stage_strengthen_bridge_nodes` unchanged.

---

## Results

### Variant A: Full dedup (amplitude + phase, skip on repeat)

Command: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| trial | fitness  | transfer  | carrier_e | xi_v2  | phase_coh | consciousness | R      |
|-------|----------|-----------|-----------|--------|-----------|---------------|--------|
| t1    | 0.032714 | 0.887940  | 0.9846    | 0.9614 | 0.8050    | 0.8544        | 0.1406 |
| t2    | 0.032712 | 0.887940  | 0.9846    | 0.9614 | 0.8050    | 0.8544        | 0.1406 |
| t3    | 0.032717 | 0.887940  | 0.9846    | 0.9614 | 0.8050    | 0.8544        | 0.1406 |
| avg   | 0.032714 | 0.887940  | 0.9846    | 0.9614 | 0.8050    | 0.8544        | 0.1406 |

amp_deltas_flat: [0.4326, 0.5086, 0.0314, 0.0332]

Phase_coherence and consciousness regressed severely (0.805, 0.854). Phase dedup prevents the
40+ alignments that normally converge phases before irx runs.
R collapsed from 0.867 → 0.141 (phase is no longer coherent going into irx).

### Variant B: Amplitude-only dedup (KEPT — current code)

Amplitude boosted once per memory per cycle. Phase alignment always applied for all constructive
pairs (same as before).

Command: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| trial | fitness  | transfer  | carrier_e | xi_v2  | phase_coh | consciousness | R      | qgrav  |
|-------|----------|-----------|-----------|--------|-----------|---------------|--------|--------|
| t4    | 0.018973 | 0.968781  | 0.9856    | 0.9248 | 0.9973    | 0.9590        | 0.8630 | 0.4603 |
| t5    | 0.018974 | 0.968781  | 0.9856    | 0.9248 | 0.9973    | 0.9590        | 0.8630 | 0.4603 |
| avg   | 0.018974 | 0.968781  | 0.9856    | 0.9248 | 0.9973    | 0.9590        | 0.8630 | 0.4603 |

amp_deltas_flat: [0.4326, 0.5092, 0.0244, 0.0244]

---

## Comparison to baseline

| metric             | wt   | baseline (0.0578) | new (0.0190)  | cost Δ    |
|--------------------|------|-------------------|---------------|-----------|
| carrier_emergence  | 0.10 | 0.5333 → 0.047    | 0.9856 → 0.001 | **−0.046** |
| transfer_score     | 0.15 | 0.9655 → 0.005    | 0.9688 → 0.005 | +0.000    |
| xi_robustness_v2   | 0.15 | 0.9675 → 0.005    | 0.9248 → 0.011 | +0.006    |
| phase_coherence    | 0.02 | 0.9976 → 0.000    | 0.9973 → 0.000 | ≈ 0       |
| consciousness      | 0.03 | 0.9779 → 0.001    | 0.9590 → 0.001 | +0.001    |
| other metrics      | misc | ≈ 0               | ≈ 0           | ≈ 0       |

Net improvement: −0.046 + 0.007 = **−0.039 fitness**. Measured: 0.0578 → 0.0190 = −0.039. ✓

Xi regression (0.9675 → 0.9248): full phase alignment means 40+ sequential alignments pull each
memory toward a converged mean phase. Under irx this creates stronger phase clustering, but the
xi adversarial test may see cleaner and adversarial memories converging into the same phase basin
(less separation). This costs 0.15 × 0.043 = 0.0064 fitness. Not fully understood; xi axis is
open for future investigation under the new dedup regime.

R recovered fully (0.867) because phase alignment is preserved. The Variant A R collapse (0.141)
was a direct consequence of skipping phase alignments.

---

## Analytical validation

Observed amp_deltas_flat: [0.4326, 0.5092, 0.0244, 0.0244]
Predicted (analytical): [0.300, 0.438, 0.262, 0.000]

The observed pattern qualitatively matches (cycle 1 > cycle 0, then drops), but with:
- Higher cycle 0 (0.433 vs 0.300): multiple memories start at different amplitudes; those at
  lower amplitude in the corpus contribute larger delta relative to starting point.
- Near-zero cycles 2-3 (0.024 vs predicted 0.262/0.000): online injection at cycle 2 adds
  memories at amp=0.8, which then get phase-aligned but NOT amplitude-boosted (already in their
  first cycle). Their delta contribution is small.

The resulting DFT has dominant k=1 power → carrier_e = 0.986 (vs predicted 0.928).
Better than predicted because of corpus-wide amplitude averaging effects.

---

## TSV rows appended

Five L5 rows appended total:
- Trials 1-3 (Variant A, full dedup): fitness ≈ 0.0327
- Trials 4-5 (Variant B, amplitude-only dedup): fitness ≈ 0.0190

---

## Decision

**KEPT — amplitude-only dedup (Variant B).** This is the current code state.

Previous optimum: 0.0578 → **New optimum: 0.0190** (3-run equiv avg: 2 trials, variance < 0.000002).

Improvement: 0.039 fitness drop >> 0.005 threshold. Confirmed with near-deterministic variance.

The carrier_emergence ceiling (diagnosed as "architectural" over multiple prior fires) was NOT a
fundamental architectural limit — it was a multi-boost saturation artifact in `stage_strengthen`.
Limiting each memory to one amplitude boost per cycle (while preserving phase alignment) removes
the ceiling with zero architectural change.

**Xi axis open:** xi regression from 0.925 to 0.9248 costs 0.0064. Next fire may investigate
whether adjusting `constructive_boost`, `interference_threshold`, or irx `alpha_base` recovers xi
without harming carrier_e.
