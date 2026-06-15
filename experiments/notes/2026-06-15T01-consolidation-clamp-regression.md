# Regression: consolidation amplitude clamping breaks L5 carrier_emergence

## Hypothesis
Eight new commits landed after the T22 curiosity fire (last fire to run trials was
earlier). Commit e427140 (`fix(consolidation): clamp strengthen amplitude + reclaim
ghosts`) is L5-relevant: the strengthen path now caps every additive boost at
`AMPLITUDE_CEILING = 2.0`. Prior fires' "no new axes" conclusions were on pre-fix
code. This fire re-baselins on the current HEAD.

**Prediction**: amplitude clamping may reduce the absolute swing of drive-induced
oscillations (which carrier_emergence detects), potentially hurting fitness.

## Trials (no code changes — pure baseline re-measurement)

| run label               | fitness  | carrier_e | transfer  | xi_v2  | phase_coh | query_grav |
|-------------------------|----------|-----------|-----------|--------|-----------|------------|
| postfix-clamping.t1     | 0.135438 | 0.5251    | 0.541603  | 0.9251 | 0.7334    | 0.9654     |
| postfix-clamping.nodg.t1| 0.116047 | 0.5294    | 0.736812  | 0.8563 | 0.7334    | 0.4603     |

Both trials at DRIVE_A=0.15 DRIVE_SCOPE=all. Trial 1 adds DREAM_GRAVITY=1.0.
Canonical pre-fix best: fitness ≈ 0.007461, carrier_e 0.9992, transfer 0.9640,
xi_v2 0.9973, phase_coh 0.9980.

## Diagnosis

The regression is large (fitness 0.007 → 0.11–0.14). Root cause:

1. **The drive creates temporal amplitude oscillations at 0.5 Hz** (the carrier
   signal): `amplitude *= (1 + 0.15 * sin(2π × 0.5 × t))`. With DRIVE_SCOPE=all
   and DRIVE_TOP_FRAC=1.0, this pushes every memory's amplitude up/down on a
   half-cycle arc.

2. **The strengthen path in consolidation runs AFTER the drive** (same dream cycle)
   and clamps: `amplitude = (amplitude + constructive_boost).min(2.0)`. Memories
   that the drive lifted to 2.30 get clamped back to 2.0 by strengthen. Memories at
   2.0 already: zero net gain from strengthen.

3. **`amplitude_deltas[cycle]`** is measured as mean |amplitude_after − amplitude_before|
   where `before` is snapshotted at the top of the cycle (pre-drive). With clamping,
   high-amplitude memories experience drive-up then consolidation-clamp-back → near-
   zero net delta. The 0.5 Hz periodic signal in amplitude_deltas collapses.

4. **carrier_emergence** is an FFT of amplitude_deltas looking for a peak in the
   [0.5, 4.0] Hz band. Collapsed deltas → no detectable peak → score drops from
   0.9992 to ~0.525.

5. **transfer_score** and **phase_coherence** also regress, likely because the drive
   amplitude modulation was also providing a rhythmic anchoring that aided phase
   alignment and consolidation-driven memory differentiation. With all memories
   flattening to 2.0, the amplitude diversity (CV) collapses to ~0, erasing the
   signal landscape.

## Decision

**Regression confirmed. No code fix attempted this fire.**

The fix in e427140 is architecturally correct (unbounded strengthen growth was a bug),
but it breaks L5's carrier emergence mechanism. A compensating change would be needed:

- Option A: raise `AMPLITUDE_CEILING` to a higher value (e.g., 4.0 or 8.0) that
  prevents true unbounded growth but allows enough headroom for drive × strengthen
  to create detectable oscillations. Risk: re-introduces some of the Φ/Kuramoto
  distortion the fix aimed to prevent.
- Option B: capture `amplitude_deltas` between post-drive and post-consolidation
  (not pre-drive to post-consolidation). This would measure consolidation's response
  to driven amplitudes rather than the total cycle change. May lose the drive signal
  if consolidation clamps synchronously.
- Option C: apply AMPLITUDE_CEILING only to the per-strengthen-event boost (i.e.,
  cap each individual `+constructive_boost` step) rather than the running total.
  Already done — but maybe the ceiling needs to be per-event only, not absolute.
- Option D: split the amplitude tracking: use a separate "drive-envelope" signal
  (per memory, the sinusoidal factor at each cycle) and score carrier_emergence on
  that, independent of the absolute amplitude.

Options A or D are lowest risk for a future fire. Option A is trivially testable
(one env var if `AMPLITUDE_CEILING` were exposed; currently a const requiring a
code change). No single-fire scope for a full architectural refactor.

Remaining carrier_emergence 0.525 (not 0) is from memories whose amplitudes were
below 2.0 (newly injected, post-prune recovered, or weakened) whose deltas are
still visible. The bimodal and flat carrier scores are nearly identical (0.5251 vs
0.5262), suggesting the signal is dominated by structural drive artifacts not the
corpus frequency encoding.

## Impact on prior fitness record

The canonical best fitness of 0.007461 / 0.007627 (3-run avg) was measured on
pre-e427140 code. That record is now invalidated by the regression. The current
floor under HEAD is approximately 0.11–0.14 pending a compensating fix.
