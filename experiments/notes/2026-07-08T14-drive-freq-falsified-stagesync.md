# 2026-07-08T14 — Drive frequency falsified under stage_sync; carrier floor confirmed

## Hypothesis

DRIVE_FREQ_HZ=1.0 would improve carrier_emergence from 0.652 to ~0.85 under
stage_sync (DREAM_MODE unset, K=3.0, DREAM_GRAVITY=0.25).

Reasoning: the 2026-06-25 drive-freq test ran under interference_relax where
constructive_boost (~0.45/pair) dominated amplitude_deltas by ~4× over the drive
(0.10). Stage_sync (step 4.5) is PHASE-ONLY — it does not directly modify amplitudes
— so the drive+gravity should be the main amplitude movers, making DRIVE_FREQ_HZ
visible in the DFT.

At 1.0 Hz, cycles 1-3 are all in the positive sine arch [sin(π/4), sin(π/2), sin(3π/4)]
= [0.707, 1.0, 0.707], giving a bell-arch amplitude pattern predicted to concentrate
DFT power at k=1 (2 Hz) with carrier_emergence ≈ 0.85.

## Configuration

```
DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE= (unset) DREAM_GRAVITY=0.25 DRIVE_FREQ_HZ=1.0
```

Baseline reference (DRIVE_FREQ_HZ=0.5, default, 2026-07-06):
- fitness: 0.0579, carrier: 0.652, transfer: 0.941, xi: 0.952

## Trial

| trial | DRIVE_FREQ_HZ | fitness  | carrier_e | transfer | xi_robust | magic_R | query_g |
|-------|---------------|----------|-----------|----------|-----------|---------|---------|
| 1     | 1.0           | 0.070351 | 0.6437    | 0.8660   | 0.9611    | 0.5272  | 0.8623  |

amp_deltas_flat: [0.19239, 0.00285, 0.03553, 0.01374]

## Analysis: secondary spike dominates, same as irx

The amp_deltas_flat reveals the carrier floor is NOT drive-dominated:

```
cycle 1 (t=0.125): 0.192 ← secondary consolidation spike (cycle 0's full-threshold
                           init creates massive amplitude reorganization; cycle 1 is
                           still settling, ~23× larger than drive at any freq)
cycle 2 (t=0.250): 0.003 ← nearly quiescent
cycle 3 (t=0.375): 0.036 ← medium, possibly gravity + small consolidation
cycle 4 (t=0.500): 0.014 ← small
```

DFT of [0.192, 0.003, 0.036, 0.014]:
- k=1 (2 Hz): (0.192-0.036)² + (0.014-0.003)² = 0.0244
- k=2 (4 Hz): (0.192-0.003+0.036-0.014)² = 0.0445

k=2 wins → carrier = 0.0445/0.0689 ≈ 0.646 (matches observed 0.6437)

The 1.0 Hz drive's contribution at cycle 1 is ≈ 0.1 × A_mean ≈ 0.07, compared to
secondary spike 0.192 — drive is swamped by a factor of ~3×.

The critical mechanism: even post-bugfix (cos>0.995 constructive pair threshold),
cycle 0's full-threshold consolidation (threshold_scale=1.0) still finds many
constructive pairs by chance (with 300 flat-corpus memories at random initial phases,
~5% are within 5.7° of each other → ~1435 pairs), generating a large amplitude
reorganization spike at cycle 0. Cycle 1's residual settling of this spike (0.192)
dominates all subsequent cycles.

Stage_sync is phase-only at step 4.5, but stages 4 (strengthen) and 6 (prune) still
modify amplitudes. The secondary spike at cycle 1 persists regardless of DREAM_MODE.

## Why stage_sync has carrier 0.652 vs irx's 0.533 (pre-fix)

Under irx pre-fix: cycle 0 spike was 4.17 → carrier 0.527 (primary spike dominates all 4 cycles)
Under stage_sync post-fix: cycle 1 secondary spike is 0.192 → carrier 0.646 (secondary spike + non-trivial delta at cycle 3)

The improvement from 0.527 to 0.652 came from:
1. The 4a1c4e6 circular phase fix reducing the constructive pair count → smaller cycle 0 spike → smaller cycle 1 residual
2. DREAM_GRAVITY=0.25 providing systematic phase-aligned amplitude modulation
3. The skip-cycle-0 code change (2026-07-02, chain_depth=5, all_deltas[1:])

None of these improvements created a drive-visible signal — carrier improvement came from the spike being smaller, not from drive frequency being effective.

## Why 1.0 Hz hurts transfer and xi

With DRIVE_FREQ_HZ=1.0, cycles 1-3 receive stronger amplitude boosts earlier:
- Cycle 1: +7.1% (vs +3.8% at 0.5 Hz)
- Cycle 2: +10.0% (vs +7.1% at 0.5 Hz)
- Cycle 3: +7.1% (vs +9.2% at 0.5 Hz)

The stronger early drive disrupts the amplitude landscape during Kuramoto sync (cycles 1-3),
apparently reducing the phase coherence that drives transfer from engine_a to engine_b.
magic_proxy_phase_R drops from 0.641 to 0.527 — indicating less phase synchronization —
consistent with earlier amplitude perturbation breaking the Kuramoto attractor.

## Decision

**No code changes. Nothing to revert.** TSV row appended for the 1.0 Hz trial.

Hypothesis FALSIFIED. DRIVE_FREQ_HZ=1.0 is strictly worse:
- carrier_emergence: 0.6437 vs 0.652 (essentially unchanged, within DFT noise)
- transfer_score: 0.866 vs 0.941 (−0.075, significant regression)
- xi_robustness_v2: 0.961 vs 0.952 (−0.009, slight regression)
- fitness: 0.070 vs 0.058 (+0.012, clear degradation)

## Env-var space under stage_sync: exhausted

The carrier_emergence floor is structural at 0.652 under stage_sync:
- Drive frequency: irrelevant (spike dominates by 3×)
- DREAM_GRAVITY: 0.25 is already optimal (sweep done 2026-06-27 under irx; mechanism
  same under stage_sync — v-shape in transfer recovers at exactly 0.25)
- DRIVE_A: sweep done; ≥0.3 known bad; 0.1 is optimal
- DRIVE_SCOPE=all: exhaustively confirmed
- KURAMOTO_COUPLING=3.0: sweep done (2026-07-06), k=3 is the optimum

## Structural floor analysis

| component          | weight | contribution | % of fitness |
|--------------------|--------|-------------|--------------|
| carrier_emergence  | 0.10   | 0.0348      | 60%          |
| transfer_score     | 0.15   | 0.0089      | 15%          |
| xi_robustness_v2   | 0.15   | 0.0072      | 12%          |
| other (11 metrics) | —      | ~0.007      | 12%          |
| **total**          |        | **0.0579**  | 100%         |

All three dominant components are at structural floors:
- carrier: 0.652 = spike-dominant DFT floor (see 2026-06-30 analysis)
- transfer: 0.941 (post-fix ceiling under stage_sync)
- xi: 0.952 (adversarial irreducible floor at chain_depth=2)

## Sub-0.050 fitness requires structural changes

Same conclusion as 2026-06-30, now re-confirmed for stage_sync:
1. **Amplitude ceiling removal**: replace abs_amplitude_cap(2.0) with relative normalization.
   Cycle 0 spike disappears; carrier would be drive-determined → ~0.85+.
2. **Carrier DFT redesign**: measure relative phase-to-amplitude correlation rather than
   spectral content of mean-abs-delta — a metric not dominated by initialization spikes.
3. **New research level (L6)**: metric arc that doesn't depend on the flat-corpus carrier signal.

## Current optimum (unchanged)

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset) DREAM_GRAVITY=0.25
KURAMOTO_COUPLING=3.0 (default, no env override needed)
DRIVE_FREQ_HZ=0.5 (default)
```

Stage_sync floor: **fitness = 0.0579** (2-trial avg from 2026-07-06, both 0.057897/0.057896)
carrier_emergence = 0.652, transfer = 0.941, xi = 0.952
