# L5 Research: Amplitude decay for carrier_emergence — falsified

**Date:** 2026-06-16T06 UTC
**Branch:** kannaka-curiosity/2026-06-16T06-amplitude-decay-carrier
**Code changes:** None kept — fully reverted
**Status:** FALSIFIED — no improvement; structural ceiling diagnosed and closed

---

## Hypothesis

Add per-cycle amplitude decay (×0.97) to non-constructive-pair memories after
`stage_strengthen`. Prediction from T20 notes: carrier_e → 0.85-0.99, decaying
non-constructive memories creates amplitude bimodality the FFT can detect, breaking
the |sin| symmetry that limits carrier_emergence. Expected fitness improvement:
+0.030–0.046 → overall fitness 0.012–0.027.

---

## Results

`DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax AMPLITUDE_DECAY=0.97`

| trial | fitness  | carrier_e | transfer_score | xi_v2  | R      | query_gravity |
|-------|----------|-----------|----------------|--------|--------|---------------|
| t1    | 0.057624 | 0.5334    | 0.965411       | 0.9675 | 0.8672 | 0.4603        |

Baseline (irx 3-trial avg): fitness 0.0578, carrier_e 0.5333

**Δfitness = −0.000165 (below 0.005 threshold). Carrier_e unchanged: 0.5333 → 0.5334.**

---

## Why it failed — structural diagnosis

The T20 prediction assumed 16 dream cycles, but `l5_params.chain_depth = 4` (hardcoded
at research.rs L3444, an irx-cap to prevent hallucination-driven over-consolidation).

Raw amplitude_deltas for engine_flat (4-sample DFT input):
```
[0.950, 0.031, 0.010, 0.030]
```

The signal is a **huge first-cycle spike then near-silence**:
- Cycle 0: constructive pairs boosted for the first time (+0.45 each) → large delta (0.95)
- Cycles 1-3: most memories already at AMPLITUDE_CEILING=2.0; drive can't push above
  ceiling, strengthen is a no-op → near-zero delta

The carrier_emergence = 0.533 is the 4-sample DFT artifact of this impulse pattern,
not a true carrier signal. DFT of [0.95, 0.03, 0.01, 0.03]:
- k=1 (2 Hz in 4-sample window at fs=8 Hz): |X(1)|² ≈ 0.884
- k=2 (4 Hz, boundary of [0.5, 4.0] Hz range): |X(2)|² ≈ 0.808
- carrier_emergence = 0.884 / (0.884 + 0.808) ≈ 0.522 (matches observed 0.533)

The DRIVE frequency (0.5 Hz) only covers **1/4 period** in 4 dream cycles, so it cannot
create a periodic carrier signal. The drive monotonically increases during cycles 0-3
(sin goes 0 → 0.924 over the first quarter-period), creating a small upward slope in
amplitude_deltas — but the first-cycle spike (0.95) dwarfs this.

**Why decay on non-constructive pairs doesn't help:**
- The first-cycle spike (0.95) comes from constructive pairs — decay is skipped for them
- Non-constructive pair amplitude changes are ≈0.03 per cycle; decay at ×0.97 changes
  these by ±0.001, too small to affect the DFT ratio
- Even aggressive decay (×0.90) would create a large initial spike (non-constructive at
  cycle 0 decayed 10%) then rapid approach to noise floor — worse, not better

---

## Why T20's prediction was wrong

T20 notes modeled 16 cycles of |-0.03 + 0.1455·sin_n| and computed a strong k=1
component. But with chain_depth=4, only cycles 0-3 exist. The signal is completely
different: a near-impulse at cycle 0 from constructive-pair initialization. No amount of
non-constructive decay can change the first-cycle constructive spike.

Previously noted: "T07 showed AMPLITUDE_CEILING sweep 2.0–6.0 all give carrier_e ≈ 0.530;
T13 confirmed drive frequency invariance." These are now fully explained: they all probe
the same 4-sample DFT artifact of first-cycle spike + silence. The artifact is stable
across ceiling values and drive frequencies because it's driven by the boost magnitude
(0.45) relative to the 4-cycle window.

---

## Structural ceiling — confirmed closed

As of this fire, **carrier_emergence ≈ 0.533 is the hard structural floor** for the
current architecture. Root cause:

1. `chain_depth = 4` (irx cap; T15 confirmed more cycles collapse xi/transfer)
2. `constructive_boost = 0.45` (first cycle boosts to near-ceiling for most pairs)
3. Drive at 0.5 Hz covers only 1/4 period → not periodic in the 4-cycle window

To improve carrier_emergence above 0.533 would require one of:
- Lower `constructive_boost` (0.10 or less) — reduces first-cycle spike but would
  significantly harm transfer_score (0.15 weight) and xi (0.15 weight), net regression
- Different drive frequency, e.g., 2.0 Hz → 1 full period in 4 cycles; but T13 confirmed
  2 Hz was worse than 0.5 Hz even before carrier_e was properly diagnosed
- Increase chain_depth — T15 found 4 cycles optimal; more cycles cause hallucination
  build-up that collapses xi and transfer
- Redesign the carrier_emergence metric to use a different signal (e.g., phase velocity
  over 4 cycles rather than amplitude_delta FFT)

The last option (metric redesign) is the most promising path, but it's an architectural
decision, not an L5 autoresearch knob sweep.

---

## Closed axes — complete list (post-fix, through this fire)

| axis | status | fitness floor |
|------|--------|---------------|
| DREAM_MODE=interference_relax | optimal | 0.058 |
| DRIVE_A=0.15 | optimal | — |
| DRIVE_SCOPE=all | optimal (irx) | — |
| AMPLITUDE_CEILING sweep | no-op (T07) | 0.533 carrier_e regardless |
| K-sweep (stage_sync) | no-op (irx doesn't use K) | — |
| DRIVE_FREQ_HZ | no-op (T13) | 0.533 carrier_e regardless |
| DREAM_GRAVITY=0.5 | sub-threshold Δ0.0005 (T20) | — |
| relax_steps sweep | no-op post-fix (T07) | — |
| chiral_bp sweep (T18) | regression | — |
| xi-repulsion threshold 0.28→0.22 (T14) | regression | — |
| no_transfer scope (T19) | 0.142 (worse than irx 0.058) | — |
| AMPLITUDE_DECAY on non-constructive (this) | no-op | 0.533 carrier_e |

**All known autoresearch axes are closed. The fitness floor is 0.058.**

---

## Decision

No code changes kept. Consolidation.rs fully reverted to master state.
2 TSV rows appended (labeled `L5`): trial 1 (grep output) and trial 2 (amplitude_deltas
diagnostic), both at AMPLITUDE_DECAY=0.97, DRIVE_A=0.15, DRIVE_SCOPE=all,
DREAM_MODE=interference_relax. Both showed fitness ≈ 0.0576 (below threshold vs 0.0578
baseline). Code is reverted; the AMPLITUDE_DECAY env var has no effect on reverted code.

**This fire closes the last open autoresearch axis. The 0.058 floor is structural.**

Architectural paths (outside autoresearch scope):
1. Carrier metric redesign (phase-velocity FFT over 4 cycles)
2. Adaptive constructive_boost that scales with proximity to ceiling
3. Chain depth increase with a new mechanism to prevent hallucination over-consolidation
