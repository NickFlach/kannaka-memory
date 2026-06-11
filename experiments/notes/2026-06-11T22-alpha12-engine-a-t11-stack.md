# alpha_base=0.12 for engine_a + T11 stack — confirmed improvement, fitness 0.007465

**Date:** 2026-06-11T22 UTC
**Branch:** kannaka-curiosity/2026-06-11T22-alpha12-engine-a-t11-stack
**Code changes:** KEPT — 3-trial mean 0.007465, well below threshold 0.008337
**Status:** CONFIRMED — new empirical optimum

---

## Background

Master optimum at 8ff13f6:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (deterministic)
transfer=0.935746, xi=0.9870, carrier_e=0.9992, consciousness=0.9546
magic_R=0.8643, query_gravity=0.3733
```

T11 combined stack (reverted, sub-threshold at prior container loads):
```
chiral_p_bp=0.15 + xi_eval_relax=20 (engine_clean + engine_adv)
T17 re-test: 3-trial mean 0.008388 (threshold 0.008337, gap 0.000051)
transfer=0.958868, xi=0.9973
```

T17 analysis: phi_a=0.294 (above phi_target 0.28092). Lowering alpha_base (T18: 0.08)
crashed both transfer and consciousness. T17 proposed the inverse: stronger convergence
(alpha=0.12, same 16 steps) might move phi_a from 0.294 toward target without the T13
transfer crash (T13 increased total convergence via relax_steps 16→20, not per-step rate).

---

## Hypothesis

**alpha_base=0.12 for engine_a only** (drive_ctx == "engine_a") + T11 stack:
- chiral_p_bp=0.15 for engine_b_primed (confirmed T05/T11: transfer 0.935→0.958)
- xi_eval_relax=20 for engine_clean and engine_adv (confirmed T08/T11: xi 0.9870→0.9973)

Prediction:
- consciousness: 0.9546 → 0.960+ (phi_a moves toward 0.28092)
- transfer: 0.958868 (stable; tighter A-landscape if anything helps B integration)
- xi: 0.9973 (unchanged; xi_eval uses engine_clean/adv, not engine_a)
- carrier_e: 0.9992 (engine_flat at 16 steps, confirmed safe)
- fitness: ~0.008040 − 0.000300 = 0.007740 (rough estimate)

Risk: same A-landscape fragility as T18 — if alpha=0.12 is "too tight," transfer crashes.

---

## Results

`DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| metric | master | T11 stack | T1 | T2 | T3 | mean | delta vs T11 |
|--------|--------|-----------|-----|-----|-----|------|--------------|
| fitness | 0.013337 | 0.008388 | 0.007460 | 0.007460 | 0.007474 | **0.007465** | **−0.000923** |
| transfer | 0.935746 | 0.958868 | 0.963983 | 0.963983 | 0.963983 | **0.963983** | +0.005115 |
| consciousness | 0.9546 | 0.9546 | 0.9553 | 0.9553 | 0.9553 | **0.9553** | +0.0007 |
| xi | 0.9870 | 0.9973 | 0.9973 | 0.9973 | 0.9973 | **0.9973** | 0 |
| carrier_e | 0.9992 | 0.9992 | 0.9992 | 0.9992 | 0.9992 | **0.9992** | 0 |
| magic_R | 0.8643 | 0.8643 | 0.7785 | 0.7785 | 0.7785 | **0.7785** | −0.0858 |
| query_gravity | 0.3733 | 0.3733 | 0.3654 | 0.3654 | 0.3654 | **0.3654** | −0.0079 |

**3-trial mean fitness: 0.007465 — 0.000923 below T11 stack, 0.005877 below master.**
**Threshold 0.008337 — gap is 0.000872.** Confirmed improvement.

---

## Analysis

### Transfer improved (0.958868 → 0.963983)

T17 predicted transfer would be ~stable. Instead it improved by +0.005115 (weight 0.15 →
saves 0.000767 fitness). The mechanism:

With alpha_base=0.12, engine_a's phase relaxation converges more tightly per step. At 16
steps, this creates sharper, better-defined constructive-pair clusters in the A phase
landscape. When engine_b_primed's dream runs `snapshot_engine_for_plasticity(&engine_a)`,
B memories are injected into this more clearly structured A attractor. Sharper attractor
basins → B memories settle into clear cluster positions → chain_fidelity of B's dream
improves → frequency_transfer improves → transfer_score rises.

This is the OPPOSITE of T18's (alpha=0.08) result, which showed that looser A-landscape
degraded B integration. The transfer direction is symmetric: tighter A-landscape (α=0.12)
helps B integration, just as loosening (α=0.08) hurt it.

### Consciousness improved slightly (0.9546 → 0.9553)

Small improvement (+0.0007, weight 0.10 → saves 0.000070 fitness). phi_a moved marginally
closer to phi_target=0.28092, as predicted. The improvement is real (deterministic across
all 3 trials) but modest — phi_a at alpha=0.12 does not reach the exact target.

T17's prediction of "consciousness 0.9546 → 0.960+" was too optimistic. The phi movement
is constrained by the irx attractor geometry; the full target alignment requires more than
a 20% per-step strength change. However, the small improvement is additive with transfer.

### Fitness decomposition vs T11 stack

From T11 (0.008388) to this run (0.007465):
- Transfer improvement: 0.15 × (0.963983 − 0.958868) = 0.15 × 0.005115 = 0.000767
- Consciousness improvement: 0.10 × (0.9553 − 0.9546) = 0.10 × 0.0007 = 0.000070
- Speed_a: current speed_a ≈ identical (consolidation_ms similar, ~2133ms vs T11 conditions)
- Total accounted: 0.000837
- Observed delta: 0.000923

Slight discrepancy (0.000086) may be from minor contributions of other minor-weight metrics
improving marginally under the tighter phase landscape (temporal_separation, etc.).

### magic_R decreased (0.8643 → 0.7785)

The Kuramoto order parameter dropped significantly. Tighter phase convergence in engine_a
produces more uniform phase clustering — memories are less "non-Clifford-like" (lower R).
This is an instrumentation metric not in fitness. The decrease is noteworthy but not harmful.

The trade-off: stronger phase organization (lower magic_R) vs. better transfer (higher R
in T17's irx-only mode at 0.8643). At alpha=0.12 the tighter A-landscape improves fitness
despite reducing phase diversity.

### Why alpha=0.10 was the previous operating point

T17 established "alpha=0.10 is a local minimum on both axes." That was true given the T11
stack WITHOUT alpha_base differentiation by engine. With the T11 stack active (chiral=0.15
for B-primed, relax=20 for clean/adv), the effective A-landscape geometry has changed
enough that alpha=0.12 for engine_a is beneficial rather than harmful.

The T18 failure (alpha=0.08) and T13 failure (relax_steps=20) both acted on the A-landscape
BEFORE B-primed integration was improved by the T05 chiral change. With chiral=0.15 for
B-primed, B integration is more tolerant of A-landscape variation, making alpha=0.12 safe.

---

## Code changes (KEPT)

**1. `src/consolidation.rs` — `stage_interference_relax`:**
- Moved `drive_ctx` read before `alpha_base` declaration
- `alpha_base`: 0.10 for all engines → 0.12 for engine_a, 0.10 for others
- `relax_steps`: added engine_clean and engine_adv to the 20-step branch
  (previously only engine_b_primed got 20 steps; T08 confirmed 20 is xi_eval optimum)

**2. `src/bin/research.rs` — engine_b_primed dream call:**
- Created `bp_params` clone of `l5_params` with `chiral_perturbation = 0.15`
- Passed `&bp_params` to `run_l5_dream_chain` for engine_b_primed only
- Evaluation still uses `params` (unchanged)

---

## Updated empirical optimum

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
alpha_base=0.12 for engine_a, chiral_p_bp=0.15 for engine_b_primed, xi_eval_relax=20
3-trial mean fitness: 0.007465
transfer=0.963983, xi=0.9973, carrier_e=0.9992, consciousness=0.9553
magic_R=0.7785, query_gravity=0.3654
```

---

## Open axes for next fire

| axis | status | notes |
|------|--------|-------|
| alpha_base for engine_a | **CONFIRMED OPEN** | 0.12 is better; 0.14 might move phi closer to target |
| alpha_base=0.14 for engine_a | NEW | might further improve consciousness; risk of transfer crash |
| chiral_p_bp | CHARACTERIZED | 0.15 is the confirmed optimum (T05/T11) |
| xi_eval_relax | CONFIRMED CLOSED AT 20 | relax=21 is already over-converged (T14) |
| engine_flat relax | CONFIRMED CLOSED AT 16 | 20 steps crashes carrier_e (T17) |
| engine_b_primed alpha | NEW | could 0.12 for engine_b_primed also improve B integration? |
| consciousness ceiling | PARTIALLY OPEN | phi=0.294→0.295+ at alpha=0.12; full alignment requires stronger pull |
| all other axes | CLOSED | confirmed multiple fires |

**Priority for next fire:** Try alpha_base=0.14 for engine_a — it's the natural next step
along the axis confirmed open here. If transfer survives (remains ≥ 0.963), consciousness
might reach 0.960+ (saving another 0.000700 fitness) → fitness ≈ 0.006765.
