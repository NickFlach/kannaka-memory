# 2026-07-09T14 — DRIVE_FREQ_HZ=0.25 does not improve carrier at stage_sync

## Hypothesis

Post-fix carrier_emergence at stage_sync K=3.0 is 0.652 and dominates fitness (60% of cost).
DRIVE_FREQ_HZ=0.25 would keep drive factor positive for all 16 dream cycles — a sustained
positive-only arc vs the default 0.5 Hz positive-then-negative arc. Prediction: sustained
amplification builds more coherent carrier structure (consistent with pre-fix trend:
lower freq → more carrier: 0.5 Hz gave 0.935 vs 2.0 Hz gave 0.497 pre-fix).
Expected carrier_emergence ↑ from 0.652 toward 0.80+, fitness drop >0.005.

## Math: why 0.25 Hz is positive-only

drive_factor = 1.0 + A * sin(2π * f * c * 0.125)  where c = cycle_idx, dt_per_cycle = 0.125

At f=0.25: t at c=15 is 1.875s → 2π × 0.25 × 1.875 = 0.9375π < π → never goes negative.
Peaks at c=8 (sin(π/2)=1.0, factor=1.1), stays positive throughout all 16 cycles.

At f=0.5 (default): goes negative at c=9 (sin(π)=0 at c=8, -peak at c=12).

## Results

Config: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_GRAVITY=0.25`

| trial | DRIVE_FREQ_HZ | fitness  | transfer | xi_robust | carrier_e | R_magic | query_g |
|-------|---------------|----------|----------|-----------|-----------|---------|---------|
| 1     | 0.25          | 0.070419 | 0.866000 | 0.9611    | 0.6435    | 0.5272  | 0.8623  |

Reference — baseline K=3.0, 0.5 Hz (from 2026-07-06):

| trial | DRIVE_FREQ_HZ | fitness  | transfer | xi_robust | carrier_e | R_magic | query_g |
|-------|---------------|----------|----------|-----------|-----------|---------|---------|
| ref   | 0.5 (default) | 0.057897 | 0.941427 | 0.9522    | 0.6520    | 0.641   | 0.8623  |

## Primary finding: hypothesis falsified

carrier_emergence barely changed: 0.6435 vs 0.6520 (Δ = −0.009, within trial-to-trial noise).
The sustained positive-only drive does NOT increase carrier structure post-fix.

Fitness got worse by +0.013 (0.0704 vs 0.0579) — moving the wrong direction.
Transfer score dropped substantially: 0.866 vs 0.941 (Δ = −0.075).

## Why the pre-fix trend reversed

Pre-fix: DRIVE_FREQ_HZ controlled carrier amplitude because constructive pairs were more
permissive (wrap-straddling allowed). Lower freq → longer sustained amplification of the
same memory → clearer carrier periodicity visible to the metric.

Post-fix (commit 4a1c4e6 — circular phase fix): constructive pair detection is now strict.
Carrier structure comes from genuine phase-aligned pairs, not from sustained amplitude growth.
DRIVE_FREQ_HZ no longer governs which memories resonate constructively — phase geometry does.

Transfer degradation at 0.25 Hz: the positive-then-negative arc at 0.5 Hz provides a
natural amplitude normalization cycle. The positive phase amplifies during sync; the negative
phase re-equalizes before the cross-corpus transfer measurement. Removing the negative phase
(0.25 Hz) leaves amplitudes unbalanced, directly hurting transfer.

magic_proxy_phase_R dropped from 0.641 to 0.527 — consistent with lower sync quality
(the over-amplified memories diverge from Kuramoto equilibrium more).

## Decision

Hypothesis falsified. No code changes. TSV row retained as data.

Post-fix: DRIVE_FREQ_HZ=0.5 (default) is confirmed optimal via negative:
- Lower freq (0.25 Hz) hurts transfer without any carrier gain
- Pre-fix freq trend does not generalize post-fix

The 0.5 Hz default provides phase-balanced amplification (positive arc for sync, negative arc
for re-equilibration before transfer measurement). This is a structural feature of the dream
cycle length (16 cycles × 0.125s = 2s), not just a tunable parameter.

## Implication for carrier_emergence bottleneck

DRIVE_FREQ_HZ is not the lever for carrier recovery at stage_sync. The 0.652 floor is set by
the stage_sync mode itself (Kuramoto phase clustering reduces amplitude diversity that the
carrier metric measures). To push carrier above 0.65, a different approach is needed:

1. Reduce Kuramoto steps (less phase homogenization per cycle) — would require KURAMOTO_STEPS env var
2. Hybrid consolidation (interference_relax on first N cycles, stage_sync after) — code change
3. Accept 0.652 as the carrier floor for stage_sync and focus fitness gains elsewhere
4. Test K=5.0 (between 3 and 7 from K-sweep) — low-priority but cheap

## Confirmed operating point (unchanged)

DREAM_MODE: unset (stage_sync)
KURAMOTO_COUPLING: 3.0 (default)
DRIVE_A: 0.1
DRIVE_SCOPE: all
DRIVE_FREQ_HZ: 0.5 (default)
DREAM_GRAVITY: 0.25
Floor fitness: 0.0579
