# 2026-06-25T14 — Drive frequency sweep: carrier_emergence floor confirmed

## Hypothesis

The current carrier_emergence (0.5330 post-bugfix) is the dominant remaining fitness
contributor: weight 0.10 × (1−0.533) = 0.047 = 84% of total fitness 0.0578.

The "0.5 Hz confirmed optimal (2026-06-06)" comment pre-dates:
1. The chain_depth=4 irx cap (introduced with interference_relax)
2. The 2026-06-22 bugfixes (geometry Cl(0,7), chiral cos>0.995, relate_wavefronts)

At chain_depth=4, `dt_per_cycle=0.125`, and t=[0, 0.125, 0.25, 0.375]:

- **0.5 Hz (current default)**: sin(2π×0.5×t) = [0, 0.383, 0.707, 0.924] — monotonic
  rising ramp, NOT an oscillation. No clear DFT peak.
- **1.0 Hz**: sin(2π×1.0×t) = [0, 0.707, 1.0, 0.707] — positive bell arch, always
  positive. DFT analysis predicts score ≈ 0.854 if drive dominates.
- **2.0 Hz**: sin(2π×2.0×t) = [0, 1, 0, −1] — exactly one full oscillation perfectly
  aligned with the 2 Hz DFT bin (k=1). DFT analysis predicts score → 1.0 if drive
  dominates.

Prediction: DRIVE_FREQ_HZ=1.0 or 2.0 will improve carrier_emergence above 0.533.

## Configuration

```
DRIVE_A=0.1 DRIVE_SCOPE=all DRIVE_TOP_FRAC=1.0 DREAM_MODE=interference_relax
```

## Results

| trial | DRIVE_FREQ_HZ | fitness  | carrier_e | transfer | xi_robust | magic_R | query_gravity |
|-------|---------------|----------|-----------|----------|-----------|---------|---------------|
| 0 (baseline, 0.5 Hz) | 0.5 | 0.057826 | 0.5330 | 0.9654 | 0.9675 | 0.867 | 0.4603 |
| 1 | 1.0           | 0.057770 | 0.5324    | 0.9661   | 0.9675    | 0.867   | 0.4603        |
| 2 | 2.0           | 0.057837 | 0.5320    | 0.9661   | 0.9675    | 0.870   | 0.4603        |

All differences within noise. **Hypothesis falsified.**

## Analysis

The analytical predictions assumed the amplitude_delta signal is dominated by the
drive. This assumption was wrong. The dream pipeline applies:
- `constructive_boost = 0.45` per constructive pair (large per-pair amplitude change)
- `destructive_penalty = 0.35` per destructive pair (large per-pair amplitude change)

These changes are O(0.35–0.45) × amplitude per pair, while the drive is only
`DRIVE_A=0.1` (10%). Dream dynamics dominate the amplitude_deltas by ~4×.

The resulting amplitude_delta signal is approximately stationary across cycles
(dream consolidation activity is roughly constant per-cycle at depth 4), with small
stochastic variation. A roughly constant signal distributes DFT power equally across
the 2 Hz and 4 Hz bins → carrier_emergence ≈ 0.5. The small deviations from 0.5
come from whatever asymmetric variation the dream dynamics produce, not from the
drive signal.

This explains why 1 Hz and 2 Hz give essentially identical scores to 0.5 Hz: all
three show the same dream-dynamics-dominated flat amplitude_delta signal.

## Why the old "0.5 Hz optimal" result gave 0.935

The pre-bugfix code had `|sin(dphi)|<0.1` for phase_locked_pairs, which accepted
anti-phase pairs as constructive. This massively inflated the constructive pair count,
causing huge amplitude boosts (many false-positive strengthen operations). The
resulting amplitude_delta variation was large and phase-correlated with the drive
(because more pairs → more boost → more variation), producing a strong spectral peak.

Post-bugfix (cos>0.995), far fewer pairs qualify as constructive. The amplitude
changes are smaller and less correlated with the drive, making the drive signal
invisible in the amplitude_delta sequence.

## Carrier_emergence is a hard floor under current constraints

To recover carrier_emergence above 0.533, options would include:
1. `DRIVE_A >> 0.1` — known bad (≥0.3 listed as known bad, and drive dominance
   would require A ≈ 0.45 to match constructive_boost scale)
2. Longer chain_depth for carrier measurement — "irx cap" at 4 exists for good reason
3. Changing constructive_boost or boost dynamics — would break consolidation semantics
4. Explicit oscillatory mechanism in the dream pipeline — out of scope for env-var fire

The carrier_emergence floor is **a consequence of correct physics** (proper constructive
pair detection), not a bug.

## Decision

**No code changes made. Nothing to revert.**

Fitness improvement: none (trials within noise of baseline 0.057826).

## New empirical optimum (unchanged)

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE=interference_relax
DRIVE_FREQ_HZ=0.5 (default)
```
3-trial avg fitness: **0.05783** (confirmed from previous fire 2026-06-24T14)

## Next fire candidates

1. **xi_robustness ceiling probe**: at 0.9675, it contributes 0.005 to fitness.
   What limits it? Could chain_depth=3 in xi_eval (vs 2) improve the adversarial
   discrimination? Risky (changes experiment structure) but worth understanding.
2. **Fitness is now ~97% determined by carrier_emergence (84%) and xi+transfer (16%).**
   The path to sub-0.050 fitness requires either fixing carrier_emergence or pushing
   xi+transfer to 1.0. Transfer is at 0.965 — understanding what prevents it from
   reaching 1.0 may be more tractable.
3. **query_gravity (0.4603)**: stuck across all tested conditions. May be a
   measurement ceiling or structural invariant — worth investigating the code.
