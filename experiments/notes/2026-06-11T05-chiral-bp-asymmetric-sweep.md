# chiral_perturbation asymmetric b_primed sweep — eta=0.15 is optimum, sub-threshold

**Date:** 2026-06-11T05 UTC
**Branch:** kannaka-curiosity/2026-06-11T05-chiral-bp-to-zero
**Code changes:** NONE retained — sub-threshold improvement, code reverted
**Status:** CHARACTERIZED — eta=0.15 for b_primed is the asymmetric optimum; improvement sub-threshold

---

## Background

Current empirical optimum (master, post PR #253):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013224 (deterministic)
transfer=0.935746, xi=0.9870, carrier_e=0.9992
magic_R=0.864, query_gravity=0.373
fitness_B_primed=0.003887, fitness_B_naive=0.060498
```

Fitness breakdown:
- transfer (0.15): 0.15 × 0.064254 = **0.009638** (72.8%)
- xi (0.15): 0.15 × 0.013 = **0.001950** (14.7%)
- consciousness (0.03): 0.03 × 0.0454 = **0.001362** (10.3%)
- other: ~0.000274 (2.1%)

**T17 (2026-06-10T17) established**: `chiral_perturbation=0.35` for b_primed only gives
fp reduction: 0.003887 → 0.002923 (−25%), transfer 0.936 → 0.952, fitness 0.013 → 0.011.
Recommended follow-up: test eta=0.10 → predicted threshold crossing (~0.008300).

T18-T20 pivoted to testing GLOBAL chiral sweeps (not asymmetric b_primed), which all
regressed. This fire returns to T17's recommended asymmetric b_primed axis.

---

## Hypothesis

`chiral_perturbation=0.7` for b_primed partially undoes the 20-step interference_relax
convergence by adding up to 0.7 rad phase displacement. An intermediate value
preserves more of the relaxation work → better chain_fidelity in b_primed → lower fp
→ better transfer. All measurement engines (engine_a, engine_flat, engine_clean, engine_adv)
use the global eta=0.7 (unaffected).

T17's linear extrapolation predicted a monotonic fp reduction down to eta=0.00 (fp→0.0015,
transfer→0.975). This fire tests whether the extrapolation holds and finds the true minimum.

**Prediction**: eta=0.10 crosses the 0.005 threshold; eta=0.00 approaches fp→0. 

---

## Implementation

Added `CHIRAL_BP` env var override to `run_experiment_l5_session` in a params_bp clone,
passing `&params_bp` instead of `params` to the b_primed dream chain only. The placeholder
fitness eval (`eval_l5_placeholder_fitness`) still uses original `params` (only
`consciousness_phi_target` matters there, which is unchanged). Reverted after sweep.

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| eta (b_primed) | fp | fn | transfer | fitness | xi | carrier_e | magic_R | query_g |
|----------------|----|----|----------|---------|-----|-----------|---------|---------|
| 0.70 (baseline) | 0.003887 | 0.060498 | 0.935746 | **0.013224** | 0.987 | 0.9992 | 0.8643 | 0.3733 |
| 0.35 (T17) | 0.002923 | 0.060498 | 0.951692 | 0.011011 | 0.987 | 0.9992 | 0.8643 | 0.3733 |
| 0.20 | 0.002651 | 0.060498 | 0.956182 | 0.010283 | 0.987 | 0.9992 | 0.864 | 0.373 |
| **0.15** | **0.002488** | **0.060498** | **0.958868** | **0.009873** | **0.987** | **0.9992** | **0.864** | **0.373** |
| 0.10 | 0.002582 | 0.060498 | 0.957321 | 0.010121 | 0.987 | 0.9992 | 0.864 | 0.373 |
| 0.05 | 0.002718 | 0.060498 | 0.955073 | 0.010444 | 0.987 | 0.9992 | 0.864 | 0.373 |
| 0.00 | 0.023822 | 0.060498 | 0.606234 | 0.062771 | 0.987 | 0.9992 | 0.864 | 0.373 |

**Optimum: eta=0.15** — fitness 0.009873, Δ from baseline = −0.003351.

---

## Analysis

### eta=0.15 is the asymmetric b_primed chiral optimum

The fp response is non-monotonic with a clear minimum at eta=0.15:
- Decreasing from 0.70 → 0.20 → 0.15: fp decreases monotonically (less noise, better attractor)
- Decreasing from 0.15 → 0.10 → 0.05: fp INCREASES — chiral perturbation below 0.15 is
  underpowered and leaves b_primed in a suboptimal phase configuration
- eta=0.00: **catastrophic failure** — fp explodes to 0.024, transfer collapses to 0.606

### Why eta=0.00 catastrophically fails

`stage_chiral_perturbation` (Stage 9) does more than add noise — it actively sorts
memories into cluster-based chiral attractors (±π/4 for CW/CCW clusters). With eta=0.00,
this sorting is disabled, and memories after the 20-step interference_relax are not pushed
into the stable cluster structure. Without this push:
- chain_fidelity degrades (xi-centroids are not reliably cluster-anchored)
- The 4-cycle chain builds on increasingly incoherent centroids
- fp explodes by 6×

This shows chiral_perturbation is not "just noise" — it's a structural organizer for
cluster handedness. The interference_relax convergence toward constructive-pair attractors
is necessary but not sufficient for stable chiral cluster formation.

### Why 0.15 is optimal

At eta=0.15 (vs 0.70):
- Phase displacement reduced from ≤0.70 rad to ≤0.15 rad → less disruption of
  the 20-step interference_relax attractor
- Chiral cluster formation still fully functional — memories pushed ±0.15 rad toward
  handedness attractors, sufficient for cluster separation
- fp reduced from 0.003887 → 0.002488 (−36% from baseline)

At eta=0.10 (too low): The phase push toward handedness attractors is weaker. For
some memories that are between cluster attractors after interference_relax, the weaker
push leaves them less committed to one cluster → slightly worse chain_fidelity → fp rises.

### T17's linear extrapolation was wrong

T17 predicted fp → 0.0015 at eta=0.00. The actual behavior is highly non-linear:
- 0.70 → 0.15: fp decreases smoothly (−36%)
- 0.15 → 0.00: fp explodes (614% increase)

The bifurcation point is between 0.05 and 0.00. Below some minimum phase push, the
chiral cluster organization fails entirely.

### Why the improvement is sub-threshold

Best improvement: Δ = −0.003351 (0.013224 → 0.009873). Threshold: −0.005.

The remaining fitness at eta=0.15 breaks down as:
- transfer (0.15): 0.15 × 0.041132 = **0.006170** (62%)
- xi (0.15): 0.15 × 0.013 = **0.001950** (20%)
- consciousness (0.03): 0.03 × 0.0454 = **0.001362** (14%)
- other: ~0.000391 (4%)

The transfer gain is real and substantial, but xi and consciousness costs (combined 0.003312,
34%) prevent threshold crossing. These two costs are near architectural limits:
- xi=0.9870: characterized as "near architectural limit" in prior notes
- consciousness=0.9546: determined by phi_bp=0.270 relative to target=0.281 (T10 structural)

To cross the threshold from eta=0.15 (fitness=0.009873), we'd need an additional -0.001649
improvement. With transfer already at 0.9589 (further improvement marginal), the most
realistic path would be reducing either xi cost or consciousness cost.

---

## Basin map: eta={0.00, 0.05, 0.10, 0.15, 0.20, 0.35, 0.70}

| eta | fp | Δfp from baseline |
|-----|-----|-------------------|
| 0.70 | 0.003887 | 0 |
| 0.35 | 0.002923 | −25% |
| 0.20 | 0.002651 | −32% |
| **0.15** | **0.002488** | **−36%** ← minimum |
| 0.10 | 0.002582 | −34% |
| 0.05 | 0.002718 | −30% |
| 0.00 | 0.023822 | +513% ← catastrophic |

Basin is asymmetric: graceful improvement from 0.70→0.15, cliff from 0.15→0.00.

---

## Constraints established

- **eta=0.15 is the L5+irx asymmetric b_primed chiral optimum** (CLOSED — basin mapped)
- eta=0.00 is catastrophically destructive to b_primed chain_fidelity
- The chiral_perturbation minimum for cluster organization is between 0.05 and 0.15
- Global chiral reduction regresses all metrics (T18-T20); asymmetric b_primed-only reduction
  is the only safe axis for this parameter
- Sub-threshold by 0.001649 at the optimum; no plausible combination of xi reduction and
  consciousness improvement can close this gap without a new structural mechanism

---

## Decision

**No code changes retained.** eta=0.15 for b_primed is the best achievable on this axis
but sub-threshold (−0.003351 < −0.005).

**Open axes:**

| axis | status | expected gain |
|------|--------|---------------|
| chiral_bp asymmetric | **CLOSED** | −0.003351 at eta=0.15, sub-threshold |
| xi residual | CLOSED (near limit) | −0.001950 if xi → 1.0; no known mechanism |
| consciousness phi_bp | STRUCTURAL (T10) | −0.001362 if phi_bp → phi_target; B-phase init closed (T06) |
| transfer via other mechanism | UNKNOWN | 0.9589 is the new ceiling for this architecture |

**The system appears to be approaching a practical architectural limit.** The remaining
fitness (0.009873 achievable via eta=0.15) is dominated by structural costs (xi, consciousness)
that have no accessible lever. A new architectural mechanism would be required to push below
0.008 (the -0.005 threshold from current master).

**If a future fire finds a mechanism to reduce xi below 0.970 or push phi_bp above 0.278,
combining with eta=0.15 for b_primed would be worth revisiting.**
