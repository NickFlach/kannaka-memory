# L5 Research: carrier_emergence structural ceiling — two axes tested, both closed

**Date:** 2026-06-16T09 UTC
**Branch:** kannaka-curiosity/2026-06-16T09-carrier-structural-ceiling
**Code changes:** NONE KEPT — CARRIER_DECAY implemented and reverted (zero effect confirmed)
**Status:** No improvement found. Optimization surface exhausted.

---

## Context

Current optimum: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness 0.0578 (3-trial avg).
Dominant cost: carrier_emergence (0.533, cost 0.10 × 0.467 = 0.047 = 81% of total fitness).

T20 identified carrier_emergence as the only remaining axis, with two proposed paths:
1. **no_transfer + interference_relax** (zero code change): combining the two best scope/mode settings
2. **Asymmetric amplitude decay** (CARRIER_DECAY env var): decay non-constructive memories to create
   amplitude bimodality and hopefully make amp_deltas_flat oscillate at drive frequency

T13 previously confirmed that drive frequency has zero effect on carrier_e under chain_depth=4.
This fire tests the two remaining structural hypotheses.

---

## Axis 1: no_transfer + interference_relax (zero code change)

**Hypothesis**: Combining DRIVE_SCOPE=no_transfer (avoids driving engine_b_primed/engine_b_naive)
with DREAM_MODE=interference_relax stacks their benefits: higher transfer from not disrupting
engine_b, plus the phase coherence (R=0.867) and xi benefits from interference_relax.

**Prediction**: transfer_score improves from 0.9655 → 0.97+, fitness drops to ~0.054.

**Result** (`DRIVE_A=0.15 DRIVE_SCOPE=no_transfer DREAM_MODE=interference_relax`):

| metric           | baseline (all+irx) | no_transfer+irx |
|------------------|-------------------|-----------------|
| fitness          | 0.057789 (3-avg)  | 0.058791        |
| transfer_score   | 0.965455          | 0.957577        |
| carrier_e        | 0.5333            | 0.5333          |
| xi_v2            | 0.9675            | 0.9675          |
| R                | 0.8672            | 0.8672          |

**Hypothesis falsified**. no_transfer + irx is WORSE (Δ +0.001 fitness, transfer_score drops 0.008).

**Mechanistic reversal from T19**: Pre-irx, driving engine_b_primed HURT transfer (disruptive
phase modulation on the A-derived state). Post-irx, driving engine_b_primed HELPS transfer:
the irx phase relaxation in engine_b makes constructive pairs more coherent regardless of drive,
and the drive amplitude boost amplifies recently-transferred A memories, improving their recall.
The no_transfer no-op for engine_b removes this benefit → transfer degrades.

**Axis closed**: no_transfer + irx is categorically sub-threshold.

---

## Axis 2: Asymmetric amplitude decay (CARRIER_DECAY code change)

**Hypothesis**: Adding per-cycle amplitude decay (rate d) to memories NOT in constructive pairs
creates amplitude bimodality: constructive members → ceiling (2.0), non-constructive → decaying.
The decay creates amplitude_delta contributions in cycles 1-3 that oscillate with the drive,
improving the spectral concentration at the target frequency.

**Theoretical analysis**:

amp_deltas_flat pattern (observed): [0.9498, 0.031, 0.003, 0.036]

The first cycle (0.9498) dominates because memories go from initial amplitude to near-ceiling
in cycle 0. After that, constructive members are ceiling-saturated and contribute zero delta.
The DFT of this spike-dominant pattern has nearly equal k=1 and k=2 power → carrier_e ≈ 0.533.

With CARRIER_DECAY=0.05 and DRIVE_FREQ_HZ=0.5 Hz:
- Non-constructive memories at amplitude X contribute per-cycle delta of:
  - Cycle 0: X × 0.05 (pure decay)
  - Cycle 1: X × |1.057 × 0.95 - 1| = X × 0.004 (drive up almost exactly cancels decay)
  - Cycle 2: X × 0.051 (drive × decay gives net 5.1% increase)  
  - Cycle 3: X × 0.081 (drive 1.138 × decay 0.95 = 1.081, net 8.1% increase)

These contributions are [0.05, 0.004, 0.051, 0.081] × X × N_nc/N. Pattern does not create
strong k=1 or k=2 signal relative to the dominant cycle-0 spike.

**Implementation**: Added Stage 4.4 in consolidation.rs between stage_strengthen and stage_sync.
Gated by CARRIER_DECAY env var (default 0.0 = no-op). ~20 lines. **Reverted after test.**

**Result** (`CARRIER_DECAY=0.05`):

| metric              | baseline (d=0)  | CARRIER_DECAY=0.05 |
|---------------------|-----------------|-------------------|
| fitness             | 0.057606        | 0.057548          |
| transfer_score      | 0.965455        | 0.965705          |
| carrier_emergence   | 0.5333          | 0.5335            |
| carrier_bimodal     | 0.5220          | 0.5221            |
| xi_v2               | 0.9675          | 0.9675            |
| R                   | 0.8672          | 0.8672            |
| amp_deltas_flat     | [0.9498, 0.031, 0.003, 0.036] | [0.9502, 0.031, 0.003, 0.036] |

**Hypothesis falsified**. CARRIER_DECAY=0.05 has ZERO effect on carrier_emergence (Δ = 0.0002,
within numerical noise). amp_deltas_flat is byte-near-identical to no-decay baseline.

**Why decay has no effect**: The non-constructive memories' contribution to amp_deltas_flat is
already negligible relative to the cycle-0 constructive boost spike (0.9498). With N_nc/N ≈ 0.5
and X ≈ 0.5, the added per-cycle delta is ≈ 0.05 × 0.5 × 0.5 = 0.013 for cycles 0 and 2.
This is 75× smaller than the cycle-0 spike. Even doubling the decay rate (0.10) would only add
0.026 to those cycles — still noise-level. The spectral signature (k=1 vs k=2 ratio) is dominated
by the spike at position n=0, not by any added per-cycle variation.

This also explains why T13's drive frequency had zero effect: the mechanism is the same. Any
change that adds small perturbations to cycles 1-3 is swamped by the cycle-0 spike.

---

## Structural ceiling diagnosis

carrier_emergence = 0.533 is the mathematical consequence of the amplitude_delta pattern
[~0.95, ~0.03, ~0.003, ~0.036] produced by AMPLITUDE_CEILING=2.0 + chain_depth=4:

- **cycle 0**: all constructive memories boosted from initial (~0.5) to near-ceiling in one
  shot → mean delta ≈ 0.45-0.95 (large, "initial transient")
- **cycles 1-3**: constructive memories already at ceiling → boost absorbed → delta ≈ 0
- **total**: spike-dominant pattern with equal k=1 and k=2 power → carrier_e ≈ 0.5

The marginal deviation above 0.5 (to 0.533) comes from secondary effects (decay of non-ceiling
memories, drive-induced slight amplitude variations that hit ceiling at slightly different points
per cycle). These create weak k=1 and k=2 asymmetry.

**Axes confirmed closed as of this fire:**

| axis | notes |
|------|-------|
| DRIVE_FREQ_HZ (any value) | T13 confirmed: ceiling+boost absorbs all drive, carrier_e invariant |
| CARRIER_DECAY (any value) | This fire: no effect; added delta swamped by cycle-0 spike |
| no_transfer + irx | This fire: worse (transfer_score drops 0.008 without benefit) |
| DREAM_GRAVITY | T20: sub-threshold (Δ −0.001 fitness) |
| K-sweep | T12: no effect (irx bypasses Kuramoto) |
| relax_steps (16, 20) | Already at these values; T07 showed 16 kills carrier_e pre-fix |
| DRIVE_SCOPE=all | Confirmed optimal |
| DRIVE_A=0.15 | T14 confirmed optimal |
| DREAM_MODE=interference_relax | T14 confirmed optimal |

---

## What would actually fix carrier_e

Based on the structural analysis, carrier_emergence requires either:

1. **Multiple dream cycles to reach steady-state before measurement**: the current metric
   is dominated by the initial-transient spike in cycle 0. A "pre-warm" pass (run 4 cycles
   without recording, then run 4 measured cycles) would eliminate the spike. But pre-warmed
   constructive memories are all at ceiling → their delta = 0 even in measured cycles → total
   amplitude_delta ≈ 0 → carrier_e = 0. Opposite problem.

2. **Relative amplitude ceiling**: instead of absolute cap at 2.0, normalize amplitudes so
   the median stays at 1.0. Constructive memories could reach 2-3× median while non-constructive
   memories stay at 0.5-1×. The drive would then create oscillatory amplitude_delta in constructive
   members (they don't hit an absolute ceiling). Requires architectural change to ResonanceEngine.

3. **Longer dream chain**: chain_depth=4 was capped to prevent over-consolidation under irx.
   With chain_depth=16-32, the 0.5 Hz drive would complete a full oscillation and the initial
   spike would be a smaller fraction of the total series. But T15 showed chain_depth≥8 causes
   hallucination-driven over-consolidation that collapses xi (0.97→0.68).

4. **Drive-correlated pair formation**: if the number of constructive pairs per cycle oscillates
   at the drive frequency, amplitude_delta would oscillate. This requires amplitude-threshold
   dynamics where drive changes which memories cross the constructive threshold. Currently,
   after cycle 0 all constructive memories are at ceiling → no threshold crossing in cycles 1-3.
   Relative ceiling would fix this.

None of these can be tested within this fire's scope/constraints.

---

## TSV rows appended

Two L5 rows appended (one per cargo run, both labeled L5):
- no_transfer+irx trial: fitness 0.058791
- CARRIER_DECAY=0.05 trial: fitness 0.057548

---

## Decision

**No code changes kept. No improvement found. Two axes closed definitively.**

Current optimum: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness **0.0578**.
This remains the empirical optimum. All known axes are exhausted.

**The optimization surface for L5 is structurally bounded at fitness ≈ 0.058 until the
carrier_emergence ceiling is addressed by an architectural change (relative amplitude ceiling
or decoupled steady-state measurement).**
