# Drive Amplitude Sweep — L5 Curiosity Fire

**Date**: 2026-06-04T20 UTC  
**Branch**: kannaka-curiosity/2026-06-04T20  
**Runs used**: 5 / 5  

---

## Hypothesis

The documented empirical optimum (`DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=xi_and_flat`,
fitness ≈ 0.184) leaves room on the drive amplitude axis. Increasing `DRIVE_A` from 0.1 to 0.15
or 0.3 will create a stronger spectral signature in the amplitude-delta FFT, improving
`carrier_emergence` without destabilising consolidation dynamics.

**Prediction**: `carrier_emergence` 0.559 → 0.65+, fitness 0.115 → ~0.105.

---

## Pre-experiment discovery

Before testing the amplitude sweep, the no-drive baseline was measured to ground the fire:

| config | fitness | xi_robustness_v2 | transfer_score | carrier_emergence |
|--------|---------|-----------------|----------------|-------------------|
| no drive | 0.2204 | 0.457 | 0.625 | 0.315 |
| A=0.1 (run 1 of 2) | 0.1153 | 0.928 | 0.707 | 0.559 |

**Finding**: The previously documented "0.184 baseline with drive" is outdated. The current
system achieves ~0.112 avg with `DRIVE_A=0.1`. This is likely due to the carrier_emergence
Nyquist fix (commit cfc87f9) and xi_robustness_v2 improvements accumulating since those baselines
were recorded.

**Scope note**: `DRIVE_SCOPE=xi_and_flat` currently falls through to the wildcard `_ => true`
in the match statement (no `"xi_and_flat"` arm exists). It is functionally equivalent to
`DRIVE_SCOPE=all`. This appears to be the optimal configuration.

---

## Experimental results

### DRIVE_A=0.3 (severe regression)

| metric | value |
|--------|-------|
| fitness | **0.3175** |
| xi_robustness_v2 | 0.1064 |
| transfer_score | 0.3293 |
| carrier_emergence | 0.2861 |
| temporal_separation | 1.0000 |

The ±30% amplitude swing disrupts consolidation. xi_robustness_v2 collapses from ~0.93 to 0.11;
transfer_score drops from 0.707 to 0.329. carrier_emergence also regresses (not improves)
— the stronger drive creates irregular amplitude dynamics that spread FFT power across the
spectrum rather than concentrating it.

### DRIVE_A=0.15 (regression)

| metric | value |
|--------|-------|
| fitness | **0.2008** |
| xi_robustness_v2 | 0.4318 |
| transfer_score | 0.6323 |
| carrier_emergence | 0.5837 |
| temporal_separation | 1.0000 |

Even a 50% increase (0.1 → 0.15) causes xi_robustness_v2 to collapse from ~0.93 to 0.43.
carrier_emergence does improve marginally (0.559 → 0.584) but the xi loss (weight 0.15) far
outweighs the carrier gain (weight 0.10).

### DRIVE_A=0.1 confirmation (run 2 of 2)

| metric | value |
|--------|-------|
| fitness | **0.1104** |
| xi_robustness_v2 | 0.9609 |
| transfer_score | 0.7068 |
| carrier_emergence | 0.5588 |
| temporal_separation | 1.0000 |

---

## Drive amplitude summary

| DRIVE_A | fitness | xi_robustness_v2 | transfer_score | carrier_emergence |
|---------|---------|-----------------|----------------|-------------------|
| 0.0     | 0.2204  | 0.457           | 0.625          | 0.315             |
| 0.1     | 0.1128 avg | 0.93–0.96    | 0.707          | 0.559             |
| 0.15    | 0.2008  | 0.432           | 0.632          | 0.584             |
| 0.3     | 0.3175  | 0.106           | 0.329          | 0.286             |

`DRIVE_A=0.1` sits at a sharp minimum. The cliff between 0.1 and 0.15 is stark: xi drops by
~0.5 with only a 50% amplitude increase. This suggests a phase-transition-like threshold in
the consolidation dynamics around ±10-12% amplitude swing.

---

## Decision

**Revert**: No code changes were made (env-var only experiments). Nothing to revert.

**Keep**: A=0.1 remains the confirmed optimum. Update the documented baseline from 0.184 to
**0.112 avg** (2-trial mean: 0.115, 0.110).

**Next fire directions**:
1. Implement a proper `"xi_and_flat"` arm in the DRIVE_SCOPE match (engine_clean + engine_adv +
   engine_flat, excluding engine_a) and measure effect on xi_robustness_v2. Currently falls through
   to `_ => true` (all engines).
2. Explore `DRIVE_FREQ_HZ` variants (1.0, 3.0 Hz) at A=0.1 — the frequency axis is untested.
3. Investigate the residual transfer_score floor (0.707). This is the largest single contributor
   to remaining fitness (0.15 × 0.293 = 0.044).
4. Test `DRIVE_A` values in [0.08, 0.12] to tighten the optimum location — could be anywhere
   in that range.
