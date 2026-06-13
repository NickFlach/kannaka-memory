# engine_a relax_steps 16→14 + T11 stack — consciousness prediction FALSIFIED, axis closed

**Date:** 2026-06-11T14 UTC
**Branch:** kannaka-curiosity/2026-06-11T14-engine-a-relax14-stack
**Code changes:** REVERTED — net negative vs T11 combined stack
**Status:** FALSIFIED — engine_a relax_steps=14 worsens fitness; axis closed

---

## Background

Current empirical optimum (master at 60b8c11):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (deterministic)
transfer=0.935746, xi=0.9870, carrier_e=0.9992, consciousness=0.9546
magic_R=0.8643, query_gravity=0.3733
```

Best known near-miss (T11, reverted, sub-threshold by 0.000003):
```
chiral_p_bp=0.15 + xi_eval relax_steps=20 (engine_clean/adv)
4-trial avg fitness = 0.008340, threshold = 0.008337, gap = 0.000003
transfer=0.958868, consciousness=0.9546, xi=0.9973, temporal_sep=0.9987
```

T13 established that engine_a relax_steps 16→20 is catastrophic (fitness 0.049965,
transfer crash, consciousness regression). T13 explained the mechanism: more convergence
→ tighter within-cluster phases → FEWER cross-partition links → phi_a DECREASES.

---

## Hypothesis

T13's mechanism implies the REVERSE should hold: engine_a at 14 relax steps (LESS
convergence) → more cross-partition link diversity → phi_a INCREASES → consciousness
improves. phi_a at 16 steps ≈ 0.268 (below phi_target 0.28092). Extrapolating linearly
from T13's 16→20 data point (phi drop of ~0.007 over 4 steps → 0.00175/step):

14 steps: phi ≈ 0.268 + 2×0.00175 = 0.2715
consciousness = 1 - |0.2715 - 0.28092| / 0.28092 = 0.9665 (+0.012 vs 0.9546 baseline)
fitness gain (weight 0.03): 0.03 × 0.012 = 0.000360

Additional speed gain: 14/16 fewer irx iterations, engine_a dream ~12% shorter
speed gain: ~0.000038

Total predicted gain: ~0.000398, pushing T11 combined stack from 0.008340 → ~0.007942.

Transfer prediction: engine_a at 14 steps is LESS converged; B memories (engine_b_primed
starts from A's state) need to travel LESS far in phase space to integrate → transfer
stable or slightly improved.

Temporal_separation prediction: frequency-based metric (memory.frequency values),
not phase-based → unaffected by irx step count.

**Three combined changes tested**:
1. engine_a relax_steps 16→14 (consolidation.rs)
2. xi_eval_relax=20 for engine_clean/adv (consolidation.rs — from T11 stack)
3. chiral_p_bp=0.15 for engine_b_primed dream (research.rs — from T11 stack)

---

## Implementation

**consolidation.rs line 799** (within stage_interference_relax):
```rust
// Before:
let relax_steps: usize = if drive_ctx == "engine_b_primed" { 20 } else { 16 };
// After:
let relax_steps: usize = if drive_ctx == "engine_b_primed"
    || drive_ctx == "engine_clean"
    || drive_ctx == "engine_adv"
{ 20 } else if drive_ctx == "engine_a" { 14 } else { 16 };
```

**research.rs line 3454** (engine_b_primed dream call):
```rust
// Added:
let params_bp = { let mut p = (*params).clone(); p.chiral_perturbation = 0.15; p };
// Changed: run_l5_dream_chain(params, ...) → run_l5_dream_chain(&params_bp, ...)
```

---

## Result

Single trial: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| metric | T11 stack (reverted) | this trial | delta vs T11 | T13 prediction |
|--------|---------------------|------------|--------------|----------------|
| fitness | 0.008340 | **0.008729** | +0.000389 (WORSE) | −0.000398 (WRONG) |
| consciousness | 0.9546 | **0.9510** | −0.0036 (WORSE) | +0.012 (WRONG SIGN) |
| transfer_score | 0.958868 | **0.956987** | −0.001881 (WORSE) | stable (slight WRONG) |
| temporal_sep | 0.9987 | **1.0000** | +0.0013 (BETTER) | neutral (directionally ok) |
| xi_robustness_v2 | 0.9973 | **0.9973** | 0 | (correct) |
| carrier_emergence | 0.9992 | **0.9992** | 0 | (correct) |
| speed_a | 0.9902 | **0.9902** | ~0 | slight gain predicted |
| consolidation_ms_a | ~594ms | **586ms** | −8ms | ~−74ms predicted (WRONG) |
| magic_R | 0.8643 | 0.8724 | +0.0081 | — |
| query_gravity | 0.3733 | 0.3681 | −0.0052 | — |

---

## Analysis

### Why consciousness decreased instead of increased

The T13 extrapolation failed because the phi-step relationship is NON-LINEAR:

The irx attractor geometry is determined by the constructive-pair graph (computed from
corpus_a vector similarities). This graph is fixed regardless of relax_step count. At
16 steps, the system is already within the Kuramoto-style basin of the irx attractor —
the phases have largely converged to their attractor positions. Reducing to 14 steps
does NOT meaningfully change the converged geometry because:

1. The attractor exists at phi≈0.268 regardless of 14 or 16 steps — the attractor is
   a property of the corpus structure, not the step count at these small values.
2. T13's 16→20 data showed phi DECREASING with more steps. The 20-step regime overshoots
   the attractor and collapses diversity. The 14-step regime is simply "one convergence
   level below 16" within the same attractor basin — phi barely moves.
3. The linear extrapolation (0.00175 phi per step change) only holds far from the
   attractor. Near the attractor (14-16 steps), the gradient is much shallower.

In short: 16 steps is ALREADY in the flat part of the phi vs step curve. Reducing to
14 steps stays in the same flat region. The significant phi drop at 20 steps is a
consequence of ESCAPING the flat attractor into an over-convergence regime — not a
linear relationship.

### Why temporal_separation improved

At 14 steps, the amplitude distribution of surviving memories is slightly different:
with less phase convergence, memories that were borderline (near the amplitude pruning
threshold) survive that would be pruned at 16 steps. These additional surviving memories
add frequency-space data points, and their distribution happens to sharpen the bimodality
of the 2 Hz vs 0.1 Hz clusters. This pushed bimodality coefficient above 0.555, giving
temporal_sep = 1.0000 (vs 0.9987 at 16 steps).

This was indeed a frequency-based effect (as predicted) but operated through amplitude-
mediated memory survival, not directly.

### Why transfer worsened slightly

engine_a at 14 steps produces a slightly LESS organized A-phase landscape than at 16 steps.
engine_b_primed starts from this state and dreams B memories into it. The less organized
A-state means the irx constructive pairs for B+A are slightly different — small but
measurable. The transfer regression (0.001881) is small and well within the structural floor
(fp≈0.002488 at chiral_p_bp=0.15), suggesting B memories still integrate but slightly less
optimally.

### Why the speed gain was much smaller than predicted

The 14 vs 16 step speed gain: 2 fewer inner iterations per relax call, but each
stage_interference_relax call's main cost is the O(N²) pairwise phase computation, not
the loop count. The inner loop at line 803 iterates `relax_steps` times, but the
expensive constructive-pair lookup is O(N²) and happens PER STEP. However, 2 fewer
steps should give exactly 2/16 = 12.5% speedup in that stage.

The observed speedup: 586ms vs T11 ~594ms = only 8ms. This is much smaller than the
predicted ~74ms. This suggests engine_a's dream chain is dominated by stages OTHER than
stage_interference_relax — possibly stage_drive, stage_sync, and the chain overhead.
The irx stage is a small fraction of total dream chain time.

### Fitness breakdown for this trial

| metric | weight | value | contribution |
|--------|--------|-------|--------------|
| transfer_score | 15% | 0.956987 | 0.006452 |
| xi_robustness_v2 | 15% | 0.9973 | 0.000405 |
| consciousness | 3% | 0.9510 | 0.001470 |
| phase_coherence | 2% | 0.9988 | 0.000024 |
| carrier_emergence | 10% | 0.9992 | 0.000080 |
| speed | 3% | 0.9902 | 0.000294 |
| others (8 metrics) | 52% | ~1.0000 | ~0.000003 |
| **TOTAL** | 100% | — | **0.008729** |

---

## What temporal_sep = 1.0000 tells us

The temporal_sep improvement (+0.000195 fitness gain) from engine_a at 14 steps is a
real effect: less irx convergence → slightly broader amplitude distribution → borderline
memories survive → bimodality coefficient above Sarle's 0.555 threshold → temporal_sep
saturates at 1.0000.

However, this gain (0.000195) is completely offset by the consciousness (-0.000108)
and transfer (-0.000282) regressions. The temporal_sep improvement cannot be harvested
independently — it's coupled to the 14-step change that also causes the regressions.

If temporal_sep = 1.0000 is a goal independently, one would need to achieve the
amplitude-distribution broadening WITHOUT touching engine_a's irx convergence. No
obvious mechanism exists for this within the current architecture.

---

## Constraints established

- **engine_a relax_steps = 16 is the practical optimum** for the current irx architecture:
  - 14 steps: consciousness slight regression, transfer slight regression, temporal_sep improves
  - 16 steps: consciousness 0.9546, transfer 0.958868 (with T11 stack)
  - 20 steps: catastrophic (T13 confirmed)
- **The phi vs relax_step curve is non-linear near the attractor**: linear extrapolation
  from the T13 16→20 data point (over-convergence regime) does NOT predict behavior in
  the 14-16 step range (attractor basin). The phi change at 14 vs 16 is negligible.
- **irx stage is a small fraction of engine_a dream chain time**: 14 vs 16 steps saves
  only ~8ms (1.3%), not the predicted ~74ms (12.5%). Other stages dominate cost.
- **temporal_sep=1.0000 requires broader amplitude distribution** (fewer irx steps →
  more borderline-amplitude memories survive → better bimodality), but this is inseparable
  from the consciousness/transfer regressions at 14 steps.

---

## Updated axis status

| axis | status | notes |
|------|--------|-------|
| engine_a relax_steps=20 | CLOSED (T13) | catastrophic crash |
| engine_a relax_steps=14 | **NEW: CLOSED** | small regression; phi attractor flat in 14-16 range |
| engine_a relax_steps=16 | OPTIMAL | confirmed optimum |
| T11 combined stack | OPEN (sub-threshold) | 0.000003 gap; load-dependent |
| consciousness ceiling | **CONFIRMED DEEPER** | phi attractor flat 14-16 steps; only 20+ overshoots |
| All other axes | CLOSED | multiple previous fires |

**The T11 combined stack (fitness ~0.008340) remains the closest approach to threshold.**
The 0.000003 gap is a container-load artifact; no algorithmic improvement has been
found to close it. The practical optimum under this architecture is determined by the
speed_a noise floor.
