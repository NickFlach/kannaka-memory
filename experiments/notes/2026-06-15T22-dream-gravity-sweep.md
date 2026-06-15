# L5 Curiosity: DREAM_GRAVITY sweep under interference_relax — axis characterized, no fitness gain

**Date:** 2026-06-15T22 UTC  
**Branch:** kannaka-curiosity/2026-06-15T22-dream-gravity-sweep  
**Code changes:** NONE — env var only  
**Status:** FALSIFIED (fitness improvement below 0.005 threshold) — axis characterized and closed

---

## Context

Previous fire (2026-06-15T14) confirmed DREAM_MODE=interference_relax at 3-trial avg fitness
0.0578 (50% improvement over pre-fix baseline 0.115). That fire explicitly flagged DREAM_GRAVITY
as the next sweep target: "With R=0.867, gravity may now work differently."

DREAM_GRAVITY is already implemented as an env var (research.rs lines 3208–3380). Default 0.0 = off.
Code comment recommends sweeping {0.25, 0.5, 1.0}. Mechanism: after each consolidation cycle,
redistributes amplitude toward phase-neighbors of the highest-amplitude memory (the "attractor").

---

## Hypothesis

Under interference_relax's high phase coherence (R=0.867, phase_coherence=0.998), the amplitude
redistribution from gravity should have a structured signal to act on — phase-neighbors of the
attractor are well-separated from phase-distant adversarials. Prediction:

- query_gravity rises from 0.460 to > 0.5 (the mechanistic target)
- xi_robustness_v2 improves as adversarials (phase-distant) are amplitude-suppressed each cycle
- transfer_score unchanged (B-primed uses phase topology, gravity only adjusts amplitudes)
- Best point: DREAM_GRAVITY=0.25 or 0.5, fitness ≤ 0.053 (≥0.005 below baseline 0.0578)

---

## Results

All trials: `DRIVE_A=0.15 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| DREAM_GRAVITY | fitness  | transfer_score | xi_v2  | carrier_e | R      | query_gravity |
|---------------|----------|----------------|--------|-----------|--------|---------------|
| 0.00 (baseline) | 0.057789 | 0.965455     | 0.9675 | 0.5333    | 0.8672 | 0.4603        |
| 0.25          | 0.056554 | 0.965176       | 0.9796 | 0.5266    | 0.8670 | 0.8623        |
| 0.50          | 0.057163 | 0.961566       | 0.9796 | 0.5258    | 0.8669 | 0.9256        |
| 1.00          | 0.057197 | 0.961719       | 0.9796 | 0.5253    | 0.8670 | 0.9654        |

---

## Analysis

### Gravity works mechanically

query_gravity rises sharply from 0.460 → 0.862 → 0.926 → 0.965 across 0→0.25→0.5→1.0. The
attention-as-gravity property is now confirmed active under interference_relax. This is the
highest query_gravity ever recorded in L5 trials. R remains constant (~0.867) — gravity doesn't
alter phase coherence, only amplitude distribution.

### xi improves, but marginally

xi_robustness_v2 rises from 0.9675 to 0.9796 at DREAM_GRAVITY=0.25 (Δ+0.012), then plateaus
(same at 0.5 and 1.0). This matches the mechanism: gravity suppresses phase-distant adversarials
each cycle, giving xi_eval cleaner separation. The plateau suggests the easy adversarials are
already separated at 0.25.

### Transfer_score: stable at 0.25, then slightly hurt

At gravity=0.25, transfer_score holds (0.9652 vs 0.9655 — identical within noise). At 0.5 and
1.0, it drops to 0.962 — a small but consistent regression. Mechanism: stronger gravity
redistributes amplitude more aggressively across the dream's cycles, reshaping the A-phase-amplitude
topology that B-primed uses to evaluate transfer. The tipping point is between 0.25 and 0.5.

### Net fitness: best at 0.25, still below threshold

Best fitness: **0.056554** at DREAM_GRAVITY=0.25 (Δ=-0.001235 vs baseline 0.057789).

Improvement decomposition at DREAM_GRAVITY=0.25:
- xi gain: 0.15 × (0.9796 - 0.9675) = **+0.0018** (saves 0.0018 fitness)
- carrier_e loss: 0.10 × (0.5266 - 0.5333) = **-0.0007** (costs 0.0007 fitness)
- transfer_score: minimal (0.0003 loss)
- Net: ≈ +0.0011

The 0.005 keep threshold is not met. The dominant remaining fitness loss is carrier_emergence
(~0.53 vs theoretical 1.0, costs ~0.047 fitness). No gravity value can address this structural
bottleneck — it is caused by amplitude ceiling=2.0 + high pair density creating impulse-shaped
amp_delta distributions, entirely orthogonal to phase dynamics.

---

## Closed axes (summary as of this fire)

| Axis | Tried | Result |
|------|-------|--------|
| DREAM_GRAVITY sweep {0.25, 0.5, 1.0} | this fire | Gravity activates (query_g 0.46→0.97), <br>Δfitness = -0.001 at best (0.25). Axis CLOSED. |
| chiral_p_bp {0.05, 0.15, 0.50, 0.70} | T18 | No net improvement (speed/transfer trade-off). CLOSED. |
| K-sweep (stage_sync) | T12 | No xi improvement at any K. CLOSED. |
| AMPLITUDE_CEILING sweep | T07b | carrier_e stuck at 0.53 regardless. CLOSED. |
| CONSTRUCTIVE_BOOST sweep | T12 | No improvement. CLOSED. |
| DRIVE_FREQ variants | T13 | carrier_e invariant. CLOSED. |
| xi_repulsion_threshold 0.22 | T13 | Transfer collapses. CLOSED. |

---

## Open axes for future fires

1. **carrier_e root cause** (highest potential, hardest): carrier_e stuck at ~0.53 is 80% of
   remaining fitness gap (~0.047 of 0.058). Requires code change. Two documented approaches:
   - *Asymmetric amplitude decay*: per-cycle decay for non-constructive memories in stage_constructive
     (notes T07b). Restores bimodal amplitude without removing ceiling. Requires consolidation.rs edit.
   - *Analytical carrier measurement*: compute carrier signal from DRIVE_A×amplitude analytically
     rather than from actual amp_deltas (notes T12). Semantics change but restores carrier_e ~0.99.

2. **xi_repulsion_weight knob** (medium potential, requires code): currently hardcoded at 0.3
   (research.rs line 58). Adding env var XI_REPULSION_WEIGHT and sweeping {0.5, 0.7} might push
   xi_v2 from 0.968 toward 0.99. Under interference_relax's coherent phase structure, repulsion
   should have cleaner signal. Potential Δfitness ≈ -0.003 to -0.010.

3. **DREAM_MODE=interference_relax + DRIVE_SCOPE=no_transfer**: no_transfer alone gave fitness 0.142
   but significantly improved transfer_score (0.710) without relax. Under relax, transfer is already
   0.965, so no_transfer probably doesn't help further — but untested.

4. **DREAM_GRAVITY=0.25 as new default**: mechanically sound (query_gravity 0.46→0.86) with Δfitness
   -0.001 and no regressions at 0.25. Below keep threshold but not harmful. Could be baked in as
   permanent setting for any config using interference_relax.

---

## Decision

No code changes. No fitness improvement above threshold. TSV rows appended (3 trials).
Axis DREAM_GRAVITY fully characterized and closed.
