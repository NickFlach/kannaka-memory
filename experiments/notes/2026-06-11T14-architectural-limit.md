# Architectural limit confirmed — no new axis bridges the 0.000230 gap

**Date:** 2026-06-11T14 UTC
**Branch:** kannaka-curiosity/2026-06-11T14-architectural-limit
**Code changes:** NONE — orientation only
**Status:** CLOSED — system at practical architectural limit; no experiment run

---

## Current state

Master baseline: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`, fitness 0.013337.

Best known configuration (reverted, sub-threshold):
- chiral_p_bp=0.10 + xi_eval_relax_steps=20: fitness 0.008567 (single trial)
- Improvement: 0.004770; threshold: 0.005000; gap: 0.000230

---

## Candidates evaluated (no trials run)

### Triple-stack: chiral_bp=0.10 + xi_eval_relax=20 + b_primed_alpha_base=0.13

Predicted to regress vs double-stack.

From T06 (alpha_base sweep), alpha_base=0.13 for b_primed gave fp 0.003887→0.003826
(+0.000259 fitness) when tested WITHOUT chiral_bp. At that baseline, phi_bp ≈ 0.270 — below
target — so alpha_base=0.13 moved phi_bp toward target, reducing the consciousness term.

When chiral_bp=0.10 is already active, phi_bp is AT target (consciousness_bp ≈ 1.0). Adding
alpha_base=0.13 would push phi_bp ABOVE target — the same overshoot mechanism that made depth=5
regress when combined with chiral_bp=0.10 (T09). The chain_fidelity benefit of alpha_base=0.13
(≈+0.0001) would be cancelled or exceeded by the consciousness term regression. Net: neutral or
slightly negative. The triple-stack cannot bridge the 0.000230 gap.

Even if perfectly additive (optimistic): 0.004770 + 0.000259 = 0.005029 → fitness 0.008308
(just below threshold). But not additive due to phi_bp overshoot.

### K-sweep in interference_relax mode

Moot. DREAM_MODE=interference_relax causes `run_l5_dream_chain` to call
`stage_interference_relax` (line 270 of consolidation.rs) instead of `stage_sync`. The
Kuramoto coupling parameter K is never read in irx mode. ALL engines in the current L5
configuration — engine_a, engine_b_primed, engine_clean, engine_adv — use irx, not stage_sync.
K has identically zero effect on any metric. The 2026-06-06 K=0.5 "confirmation" was done
before irx mode existed; K is vestigial in the current architecture.

### xi_eval relax_steps = 21 or 22

Even if relax=21 preserved xi=0.9973 (neither improving nor worsening), combined with
chiral_bp=0.10 the gap would remain 0.000230. For threshold-crossing, xi needs to reach
0.9988 (a +0.0015 improvement over the relax=20 result). Given the sharp collapse at
relax=24 (xi 0.9973→0.748), the safe operating regime for xi_eval is narrow. Relax=21 is
unlikely to show meaningful xi improvement; the peak is at 20 ± 1 step.

### Online_retention (0.9905) and temporal_separation (0.9987)

No identified lever for either. Maximum combined gain if both reached 1.0:
0.10 × 0.0095 + 0.15 × 0.0013 = 0.001145 (theoretical ceiling). Without a knob, not testable.

---

## Why no single identified axis bridges the gap

The 0.000230 gap requires a metric improvement equivalent to one of:
- Transfer: fp drop from 0.002582 to 0.002489 — structural floor confirmed (T09)
- Xi: 0.9973 → 0.9988 — relax=20 is the sweet spot; 24 is catastrophic (T08)
- Consciousness: 0.9546 → 0.9623 — phi_target recalibration is net-negative (T07)
- Online retention: 0.9905 → 0.9928 — no mechanism identified
- Temporal separation: 0.9987 → 1.0 — small and no mechanism identified

---

## What would break through

Architectural changes, not parameter sweeps:
1. Remove or de-weight the cycle-2 injection disruption that creates the chain_fidelity floor
   (fp_structural = 0.002582 → cannot be reduced without changing corpus construction)
2. A new irx-compatible dream stage that reduces adversarial sensitivity without relax=24
   over-convergence (new algorithm for xi eval)
3. A phi_a reduction mechanism for engine_a that doesn't require fewer dream cycles
   (phi_a = 0.294, target = 0.281; every tested mechanism causes regressions elsewhere)
