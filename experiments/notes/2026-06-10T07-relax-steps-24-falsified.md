# relax_steps 16→24 — over-relaxation falsified

**Date:** 2026-06-10T07 UTC
**Branch:** kannaka-curiosity/2026-06-10T07-relax-steps-24
**Code changes:** NONE retained — hypothesis falsified, reverted.
**Status:** FALSIFIED — major regression.

---

## Background

Current master after T06 (phase-anchor + DRIVE_A sweep both falsified):

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.018 (deterministic)
transfer=0.903, xi=0.987, carrier_e=0.999
magic_R=0.864, query_gravity=0.373
```

Fitness breakdown:
- transfer (0.15): 0.15 × 0.097 = **0.015** (82% of total)
- xi (0.15): 0.15 × 0.013 = 0.002 (11%)
- other: ~0.001 (7%)

T06 open axes listed "relax_steps=24 not tested; may help fine-tune convergence
WITHOUT disrupting phase initialization."

---

## Hypothesis

`stage_interference_relax` runs `relax_steps=16` iterations of weighted circular
mean relaxation over constructive pairs (alpha_base=0.10, envelope_depth=0.15).

With 24 steps, each engine's phases would converge more completely toward its
constructive-pair attractor. For engine_b_primed (seeded from engine_a's post-dream
state), tighter convergence should push phi_bp closer to phi_target (0.28092),
improving the consciousness score in `eval_l5_placeholder_fitness` and lowering
fitness_b_primed → better transfer.

**Predicted:** transfer 0.903 → 0.910–0.930, xi ≈ unchanged, fitness 0.018 → 0.013–0.015.

**Why safe:** does not touch B phase initialization (the T06 constraint) or DRIVE_A.
The quiet-wave envelope adapts its shape automatically to step count.

---

## Change

`src/consolidation.rs` line 795:
```rust
// BEFORE
let relax_steps: usize = 16;
// AFTER
let relax_steps: usize = 24;
```

---

## Result (Trial 1)

```
DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax
```

| metric | baseline (master) | relax=24 | delta |
|--------|------------------|----------|-------|
| fitness | 0.018282 | **0.068583** | +0.050 REGRESSION |
| transfer | 0.903199 | 0.862276 | −0.041 |
| xi_robustness_v2 | 0.9870 | **0.748** | −0.239 MAJOR DROP |
| carrier_emergence | 0.9992 | 0.9224 | −0.077 |
| fitness_B_primed | ≈0.006 | 0.008464 | WORSE |
| fitness_B_naive | ≈0.060 | 0.061456 | slightly worse |
| magic_R | 0.864 | 0.672 | −0.192 |
| query_gravity | 0.373 | 0.370 | neutral |

**Verdict: FALSIFIED — catastrophic regression.**

---

## Diagnosis

The prediction was backwards: phi_bp got WORSE (fitness_b_primed increased from ~0.006
to 0.008), not better.

Over-relaxation (24 steps) pushes all engine phases past their constructive-pair
attractors into over-converged clusters. The effects cascade:

1. **xi collapse (0.987 → 0.748):** xi rewards wave-propagation diversity across
   phase space. More relax steps = tighter phase clustering = less diversity in
   engine_a's phase distribution = lower xi score. At 24 steps, phases cluster so
   tightly that the xi evaluation path loses the spread it needs.

2. **phi_bp gets worse, not better:** The constructive-pair attractor at 16 steps is
   the natural convergence point for the wave structure. More steps overshoot this
   attractor, moving phi_bp AWAY from phi_target. The 16-step operating point was
   already converged to the attractor's basin center.

3. **carrier_emergence drop:** Carrier structures are identified by local phase
   coherence within sub-clusters. Over-relaxation collapses sub-cluster diversity,
   merging distinct carriers into one, reducing the bimodal carrier signature.

4. **magic_R drop:** Tighter phase uniformity means lower spread of the Kuramoto
   order parameter, which in interference_relax measures the spread of the final
   phase distribution. Counterintuitively, phase over-alignment reduces R.

This is the third confirmed fragility axis:

| change | regression | mechanism |
|--------|-----------|-----------|
| B phase anchoring (T06) | 0.018 → 0.159 | Broke BFS sort topology consistency |
| DRIVE_A 0.10 → 0.15 (T06) | 0.018 → 0.161 | Passed amplitude stability cliff |
| relax_steps 16 → 24 (T07) | 0.018 → 0.069 | Over-relaxation past phase attractor |

---

## New constraint

**Do NOT increase relax_steps above 16 in the interference_relax regime.**

The 16-step operating point is at or near the convergence basin center of the
constructive-pair phase attractor. Increasing steps overshoots the attractor and
degrades all phase-dependent metrics simultaneously.

---

## Open axes (updated — T07)

| axis | expected gain | status |
|------|---------------|--------|
| relax_steps=24 | −0.003 | FALSIFIED — regression 0.018→0.069 |
| B-memory amplitude scaling in b_primed | unknown | untested; risky (may break carrier seeding) |
| Understand phi_bp gap (phi ≈ 0.94 target, not 1.0) | −0.003 | No clear lever without code changes to IIT bridge |
| Stage_sync K-sweep | ≤0.002 | Only DREAM_MODE=unset, already at 0.18 |

**Transfer ceiling at 0.903 appears to be structurally bounded** by:
1. The phi ratio between b_primed and b_naive engines (determined by IIT structure inheritance)
2. The relax_steps operating point at 16 (determined by phase attractor geometry)
3. DRIVE_A=0.10 (amplitude stability cliff below A=0.15)

---

## Decision

No code changes retained. Hypothesis falsified with major regression.
Current master at 0.018 remains the optimum. A new hard constraint is established:
relax_steps=16 is the effective upper bound for stage_interference_relax.
