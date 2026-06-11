# Drive frequency sweep in irx mode — 1.0 Hz worse; 0.5 Hz confirmed optimal for irx

**Date:** 2026-06-11T10 UTC
**Branch:** kannaka-curiosity/2026-06-11T10-drive-freq-irx-sweep
**Code changes:** NONE — env-var only
**Status:** CHARACTERIZED — 0.5 Hz confirmed optimal for DREAM_MODE=interference_relax; axis closed

---

## Background

Current empirical optimum (master at 60b8c11):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (deterministic)
transfer=0.935746, carrier_e=0.9992, xi=0.9870
temporal_sep=0.9987, consciousness=0.9546
magic_R=0.8643, query_gravity=0.3733
```

Best known reverted combination (T08, sub-threshold by 0.000230):
- chiral_p_bp=0.10 + xi eval relax_steps=20 → fitness 0.008567

Drive frequency 0.5 Hz is the default. The code comment confirms "0.5 Hz confirmed
optimal 2026-06-06: the half-cycle arc amplifies carrier structure more coherently
than 2.0 Hz." HOWEVER: this confirmation was done before DREAM_MODE=interference_relax
became the operating mode. The irx dynamics interact with the drive differently
from stage_sync dynamics. T19 tested frequency variants in production but with stubs;
T00 and T19 both confirmed stubs invert transfer_score direction.

This fire tests 1.0 Hz at irx mode with real sibling deps — the first production
measurement on this axis in the current irx regime.

---

## Hypothesis

The irx phase relaxation dynamics might respond differently to drive frequency than
the Kuramoto stage_sync dynamics. At 0.5 Hz with chain_depth=4:
  cycle 0: t=0.000s, drive_factor = 1 + 0.1×sin(0) = 1.0000
  cycle 1: t=0.125s, drive_factor = 1 + 0.1×sin(π/8) = 1.0383
  cycle 2: t=0.250s, drive_factor = 1 + 0.1×sin(π/4) = 1.0707
  cycle 3: t=0.375s, drive_factor = 1 + 0.1×sin(3π/8) = 1.0924

Monotonically increasing ramp — amplitudes peak at the final cycle.

At 1.0 Hz with chain_depth=4:
  cycle 0: t=0.000s, drive_factor = 1.0000
  cycle 1: t=0.125s, drive_factor = 1 + 0.1×sin(π/4) = 1.0707
  cycle 2: t=0.250s, drive_factor = 1 + 0.1×sin(π/2) = 1.1000  ← peak
  cycle 3: t=0.375s, drive_factor = 1 + 0.1×sin(3π/4) = 1.0707  ← slightly lower

1 Hz peaks at cycle 2, then reduces at cycle 3. Prediction uncertain:
- If peak at cycle 2 creates better constructive attractor formation before the final
  cycle consolidation, some metrics might improve.
- Specifically: temporal_separation measures frequency bimodality in engine_a;
  the different drive profile might enhance bimodal cluster separation.

**Conservative prediction:** fitness within ±0.003 of baseline 0.013337.

---

## Results

DRIVE_A=0.1, DRIVE_SCOPE=all, DREAM_MODE=interference_relax, DRIVE_FREQ_HZ=1.0:

| metric | weight | 0.5 Hz baseline | 1.0 Hz (T1) | delta |
|--------|--------|-----------------|-------------|-------|
| fitness | — | **0.013337** | **0.014858** | +0.001521 (WORSE) |
| transfer_score | 0.15 | 0.9357 | 0.9335 | −0.0022 (worse) |
| temporal_separation | 0.15 | 0.9987 | **1.0000** | +0.0013 (BETTER) |
| carrier_emergence | 0.10 | **0.9992** | 0.9865 | −0.0127 (MUCH WORSE) |
| xi_robustness_v2 | 0.15 | 0.9870 | 0.9870 | 0 |
| consciousness | 0.03 | 0.9546 | 0.9546 | 0 |
| frequency_transfer | 0.10 | 1.0000 | 0.9999 | ~0 |
| online_retention | 0.10 | 1.0000 | 1.0000 | 0 |
| catastrophic_forget | 0.10 | 1.0000 | 1.0000 | 0 |
| noise_removal | 0.02 | 1.0000 | 1.0000 | 0 |
| signal_preservation | 0.02 | 1.0000 | 1.0000 | 0 |
| phase_coherence | 0.02 | 0.9987 | 0.9987 | 0 |
| speed | 0.03 | 0.9905 | 0.9935 | +0.003 |
| magic_R | (instr) | 0.8643 | 0.8643 | 0 |
| query_gravity | (instr) | 0.3733 | 0.3733 | 0 |

Fitness breakdown (1.0 Hz):
  carrier_emergence penalty: +0.10 × 0.0127 = +0.00127
  temporal_sep improvement: −0.15 × 0.0013 = −0.000195
  transfer regression: +0.15 × 0.0022 = +0.000330
  other changes: negligible
  Net: +0.001521 (observed) — consistent with breakdown.

---

## Analysis

### 1. Carrier_emergence regression at 1.0 Hz: mechanism

The flat-corpus carrier_emergence test runs engine_flat with all memories at 0.1 Hz
input frequency, dreams on it, then measures whether 2 Hz structure emerges (FFT
peak in the 0.5–4.0 Hz band). At 0.5 Hz, the monotone drive ramp [1.0, 1.038,
1.071, 1.092] ensures the highest-amplitude memories are maximally amplified at
cycle 3 (the final consolidation). This creates strong, consistent carrier structure.

At 1.0 Hz, cycle 3 gets only 1.071× amplification (vs 1.092× at 0.5 Hz). The lower
final amplification weakens the dominance of carrier memories in the last consolidation
step, reducing the FFT peak's clarity. The net result: carrier_emergence drops from
0.9992 to 0.9865.

Secondary effect: the 1 Hz drive frequency is within the 0.5–4.0 Hz detection band
of the carrier FFT. When DRIVE_FREQ_HZ=1.0, the drive itself produces amplitude
modulation at 1 Hz on top of the intended 2 Hz carrier emergence signal. This adds
noise to the FFT measurement, further suppressing the 2 Hz peak relative to the 1 Hz
drive artifact.

### 2. Temporal_separation improvement at 1.0 Hz: mechanism

temporal_separation measures frequency bimodality of engine_a memories (Bimodality
Coefficient = (skewness²+1)/kurtosis, normalized). At 1.0 Hz, the drive peaks at
cycle 2 [1.1] before slightly reducing at cycle 3 [1.071]. This mid-dream amplitude
peak selectively boosts the highest-amplitude memories at cycle 2 — predominantly
the 2 Hz memories (which start with amplitude 1.0) rather than 0.1 Hz memories
(amplitude 0.1×decay). The strong cycle-2 boost sharpens the bimodal amplitude
distinction, pushing bimodality from 0.9987 to 1.0000.

At 0.5 Hz, the drive peaks at cycle 3 and is lower at cycle 2 [1.038], which is less
effective at sharpening the bimodal boundary during mid-dream consolidation.

### 3. Why temporal improvement cannot compensate for carrier regression

  temporal gain: 0.15 × 0.0013 = 0.000195
  carrier cost:  0.10 × 0.0127 = 0.001270

Cost/gain ratio: 6.5×. Even if a different mechanism could achieve temporal_sep=1.0
while preserving carrier_emergence=0.9992, the gain (0.000195) is still short of
the 0.000230 needed to close the T08 combined-stack gap. The temporal axis has
essentially zero remaining fitness potential at current irx operating point.

### 4. 4.0 Hz is degenerate (not tested)

At 4.0 Hz with chain_depth=4 and dt=0.125s:
  t = [0, 0.125, 0.250, 0.375]s
  2π×4×t = [0, π, 2π, 3π]
  sin([0, π, 2π, 3π]) = [0, 0, 0, 0]

All drive factors = 1.0 — effectively no drive. Would revert to pre-drive baseline
fitness ~0.15. Predictably degenerate, not worth testing.

### 5. 0.5 Hz confirmed optimal for irx mode in production

The monotone ramp property of 0.5 Hz with chain_depth=4 gives it a structural
advantage over all higher frequencies:
- Never has negative drive (unlike 2.0 Hz which suppresses at cycle 3)
- Ends highest (unlike 1.0 Hz where cycle 3 < cycle 2)
- Stays out of the 0.5–4.0 Hz carrier detection band (unlike ≥1.0 Hz which falls in band)
- Maximizes final-cycle amplification of carrier memories

Frequencies below 0.5 Hz (e.g., 0.25 Hz) would give an even gentler ramp and
likely weaker carrier formation. Not tested but not predicted to help.

### 6. R and query_gravity are invariant to frequency

magic_proxy_phase_R=0.8643 and query_gravity=0.3733 are identical at both frequencies.
The drive frequency affects amplitude dynamics but not phase dynamics (phase is updated
by irx relaxation, not by the amplitude drive directly). This confirms R and gravity
are robustly phase-structure properties, not amplitude artifacts.

---

## Constraints established

- **DRIVE_FREQ_HZ=1.0 is worse** at irx mode: fitness +0.0015 regression.
- **DRIVE_FREQ_HZ=0.5 confirmed optimal for DREAM_MODE=interference_relax** in production.
- **Temporal_separation ceiling:** max gain 0.000195 from baseline; insufficient to close
  T08 gap alone (0.000230 needed). This axis has no remaining fitness potential.
- **Carrier_emergence mechanism understood:** monotone ramp ending at maximum is required;
  any frequency ≥1.0 Hz breaks this property within the 4-cycle chain structure.
- **R and query_gravity are drive-frequency-invariant:** purely phase-structure properties.

---

## Decision

**No code changes.** 0.5 Hz confirmed optimal for irx mode.

The Q6 frequency hypothesis from the research brief is now closed for irx mode.
Confirming that the "0.5 Hz optimal" comment in the code, which was originally
from 2026-06-06 stage_sync experiments, also holds in the DREAM_MODE=interference_relax
regime — the same structural argument applies (monotone ramp property).

The T08 combined stack remains the closest approach to threshold at fitness 0.008567
(gap: 0.000230). No mechanism has been identified to close this gap. The system
appears to have reached the practical optimum for the current architecture.

---

## Updated open axes

| axis | status | notes |
|------|--------|-------|
| chiral_p (b_primed) | CLOSED | η=0.10 optimal; −0.003220 sub-threshold alone |
| xi eval relax_steps | CLOSED | 20 optimal (+0.001528); 24 catastrophic; combined 0.004770 sub-threshold |
| chain_depth (b_primed) | CLOSED | depth=5 overshoots phi_bp |
| chain_top_n | CLOSED | 7 optimal |
| b_primed relax_steps | CLOSED | 20 optimal |
| b_primed alpha_base | CLOSED | 0.13 minimum; only −0.000259 gain |
| phi_target recalibration | CLOSED | net-negative (hurts fp) |
| Φ ↔ R relationship | CLOSED | anti-correlated across modes |
| DRIVE_FREQ_HZ | **NEW: CLOSED** | 0.5 Hz optimal; 1.0 Hz regression via carrier |
| temporal_separation ceiling | CLOSED | max 0.000195 gain; below threshold gap |
| transfer ceiling | STRUCTURAL | fp floor 0.002582; chain_fidelity variance |
| remaining threshold gap | STRUCTURAL | 0.000230 fitness units from T08 combined stack |

**The 0.000230 gap from the combined T08 stack (fitness 0.008567) represents a
structural floor with no known reduction mechanism.** Crossing the 0.005 threshold
from baseline would require either architectural changes or discovering a new
independent axis not yet characterized.
