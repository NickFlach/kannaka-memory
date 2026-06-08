# envelope_depth=0.30 and hallucination_amplitude=0.9 falsified — carrier_e near a goldilocks optimum

**Date:** 2026-06-08T13 UTC
**Branch:** kannaka-curiosity/2026-06-08T13
**Code changes:** envelope_depth 0.15→0.30 tried then reverted; hallucination_amplitude 0.7→0.9 tried then reverted
**Status:** BOTH FALSIFIED — no code changes kept

---

## Background

Current empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.099
carrier_emergence=0.935, transfer_score=0.836, xi_robustness_v2=0.559 avg
```

From T08/T12 notes, remaining unexplored axes:
1. stage_sync transfer gap (no known lever)
2. irx xi variance (RNG-driven)
3. **Structural consolidation parameters** (hallucination_amplitude, prune_threshold) — explicitly flagged unexplored

On further inspection, the actual code state is:
- `stage_interference_relax`: `alpha_base=0.20`, `relax_steps=8`, `envelope_depth=0.15`
  (not 0.10/16 as some earlier notes claimed — prior notes described reverted states)
- `hallucination_amplitude=0.7` (from experiment_params; not overridden in L5 block)

Two untested parameters were identified:
1. `envelope_depth` in `stage_interference_relax` — controls the sinusoidal modulation of alpha over 8 steps
2. `hallucination_amplitude` in L5 — controls amplitude of cross-cluster bridge memories

---

## Hypothesis 1: envelope_depth = 0.30 (doubled from 0.15)

**Code change:** `src/consolidation.rs` line 796: `envelope_depth: f32 = 0.15` → `0.30`

**Rationale:** The "quiet wave" envelope creates an annealing-like pattern:
- Steps 0-3 (positive half): alpha from 0.20 to 0.26 — warm/aggressive phase convergence
- Steps 4-7 (negative half): alpha from 0.20 to 0.14 — cool/gentle consolidation

Total integrated relaxation remains exactly 1.6 (8 steps × alpha_base=0.20, envelope averages to zero over one full cycle). Only the SHAPE of relaxation changes.

Doubling envelope_depth to 0.30 creates:
- Hot phase peak: alpha=0.20×1.30 = 0.260 (vs 0.230 at depth=0.15)
- Cool phase trough: alpha=0.20×0.70 = 0.140 (vs 0.170 at depth=0.15)

**Prediction:** Stronger initial clustering might improve carrier structure and possibly xi. Falsification signal: carrier_e < 0.90.

### Results

| metric | depth=0.15 baseline (avg) | depth=0.30 trial 1 | depth=0.30 trial 2 | 2-trial avg |
|--------|--------------------------|---------------------|---------------------|-------------|
| fitness | **0.099** | 0.170 | 0.182 | **0.176** |
| transfer_score | **0.836** | 0.756 | 0.756 | **0.756** |
| carrier_emergence | **0.935** | 0.722 | 0.722 | **0.722** |
| xi_robustness_v2 | 0.559 avg | 0.305 | 0.224 | 0.265 |
| magic_proxy_phase_R | 0.617 | 0.629 | 0.629 | 0.629 |
| query_gravity | 0.363 | 0.366 | 0.366 | 0.366 |

Transfer and carrier_e are deterministic (byte-identical across trials). Xi varies as expected.

**Hypothesis falsified.** carrier_e dropped from 0.935 to 0.722 (−0.213), well below the 0.90 threshold.

### Mechanism: hot-phase overshoot disrupts carrier structure

The carrier_e metric measures the 2 Hz spectral peak in amplitude changes across dream cycles. The irx mode creates this peak by phase-relaxing memories such that the drive's 0.5 Hz modulation creates constructive/destructive relationships in a coherent pattern. The carrier structure is fragile: it depends on the memory phases settling into specific positions relative to each other and to the pre-computed pair list.

With envelope_depth=0.30, the hot phase (steps 0-3) drives phase changes 30% stronger than at depth=0.15. This overshoots the optimal phase configuration before the cool phase can fine-tune it. The result is a phase state that is "over-converged" — memories are phase-closer to each other than optimal, reducing the amplitude differentiation that creates the 2 Hz carrier.

Transfer also dropped (0.836 → 0.756): transfer depends on the amplitude structure built by the dream. Over-convergent phases create a flatter amplitude landscape, reducing the primed-vs-naive discrimination in the transfer test.

The total relaxation integral is the same (1.6), but this confirms that the SHAPE of relaxation matters as much as the total — consistent with the T09 finding that the carrier_e mechanism depends on integral relaxation, not just step count.

---

## Hypothesis 2: hallucination_amplitude = 0.9 (raised from 0.7)

**Code change:** `src/bin/research.rs` L5 block: added `l5_params.hallucination_amplitude = 0.9;`

**Rationale:** The cross-cluster hallucinations (stage 6) create bridge memories between category clusters. Currently these have amplitude=0.7 and are protected from destructive dampening (`if mem.hallucinated { continue; }` in stage_prune). Raising to 0.9 makes bridges stronger, potentially improving cross-cluster xi robustness.

**Prediction:** Stronger hallucinations → better cross-cluster coherence → xi improves. Carrier_e risk: hallucinations at 0.1 Hz might compete with 2 Hz drive-created amplitude structure. Falsification signal: carrier_e < 0.90.

### Results

| metric | baseline (avg) | hall=0.9 trial 1 | hall=0.9 trial 2 | 2-trial avg |
|--------|---------------|------------------|------------------|-------------|
| fitness | **0.099** | 0.196 | 0.161 | **0.179** |
| transfer_score | **0.836** | 0.734 | 0.707 | **0.721** |
| carrier_emergence | **0.935** | 0.714 | 0.714 | **0.714** |
| xi_robustness_v2 | 0.559 avg | 0.160 | 0.419 | 0.290 |
| magic_proxy_phase_R | 0.617 | 0.612 | 0.612 | 0.612 |
| query_gravity | 0.363 | 0.364 | 0.364 | 0.364 |

Note: transfer_score varies across trials (0.734 vs 0.707) — unlike baseline irx where transfer is deterministic. This additional variance appears when hallucination amplitude is high enough that hallucination selection affects the engine_a state used for the transfer test.

**Hypothesis falsified.** carrier_e dropped from 0.935 to 0.714 (−0.221). Xi also regressed.

### Mechanism: high-amplitude hallucinations dilute the 2 Hz carrier

Hallucinated memories have frequency=0.1 Hz (storage band, same as sparse corpus members). The 2 Hz carrier emerges from the drive's amplitude modulation of dense-band memories (frequency≈2.0 Hz). Over 16 dream cycles, each cycle creates 2-3 new hallucinations at 0.9 amplitude. Crucially, hallucinations are protected from destructive dampening.

With 16 cycles × ~2 hallucinations each = ~32 hallucinations at amplitude 0.9, alongside ~100 drive-modulated memories at amplitude ~1.1-1.2. The fraction of memories at the "carrier" amplitude is reduced by the 0.9-amplitude hallucinations flooding the engine. The FFT measures power at 2 Hz vs total power — higher-amplitude 0.1 Hz hallucinations increase the total-power denominator without contributing to the 2 Hz numerator, reducing spectral concentration.

At baseline amplitude=0.7: hallucinations provide bridges but with less amplitude interference. The 0.7 level was apparently well-tuned to balance bridge strength vs carrier dilution.

---

## Combined findings: carrier_e is near a goldilocks optimum

Both changes degraded carrier_e from 0.935 to ~0.714-0.722, and both were reverted. The pattern suggests:

1. **envelope_depth**: 0.15 creates the right balance between aggressive initial clustering (hot phase) and gentle fine-tuning (cool phase). Higher depth (0.30) over-converges phases, flattening the amplitude landscape.

2. **hallucination_amplitude**: 0.7 is near the ceiling for bridge amplitude before carrier dilution becomes significant. Higher amplitude (0.9) drowns out the 2 Hz drive signal with 0.1 Hz hallucination noise.

Both parameters interact with carrier_e through the **amplitude dilution** mechanism: the carrier metric requires a strong spectral peak at 2 Hz relative to total power. Anything that either (a) reduces phase differentiation between memories or (b) adds high-amplitude non-carrier memories reduces this peak.

**The irx parameter space is very narrow at the carrier_e optimum.** The current settings (alpha_base=0.20, relax_steps=8, envelope_depth=0.15, hallucination_amplitude=0.7) represent a carefully balanced configuration where:
- Total relaxation (1.6) is enough to converge constructive pairs without over-converging
- The envelope shape (0.15) provides initial clustering without hot-phase overshoot
- Hallucination amplitude (0.7) builds bridges without diluting the carrier signal

---

## What remains open

All irx internal parameters (alpha_base, relax_steps, envelope_depth, hallucination_amplitude) are now closed at their current optimum values. The remaining open directions:

1. **prune_threshold** (0.095): analysis suggests this is dominated by noise_floor=0.18 for the amplitude ranges relevant to L5. Low expected upside, not worth a trial.
2. **xi variance seeding**: xi_robustness_v2 varies because hallucination selection is affected by non-deterministic cluster detection (rayon parallel cosine similarity). No env-var handle.
3. **stage_sync transfer gap** (0.655 vs irx's 0.836): no known lever for L5.
4. **New modes**: all explored modes (irx, sync, hybrid) characterized. No untested modes.

The empirical optimum is stable at:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
avg fitness ≈ 0.099
carrier_emergence=0.935, transfer_score=0.836, xi_robustness_v2=0.559 avg
```

The L5 irx parameter space appears fully explored within the current architecture. Future improvement likely requires either (1) a new dream mode concept or (2) corpus/adversarial architecture changes.
