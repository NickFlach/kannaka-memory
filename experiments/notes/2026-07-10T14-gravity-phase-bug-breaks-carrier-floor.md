# 2026-07-10T14 — Gravity phase normalization bug was the carrier_emergence floor

## Hypothesis

Primary (FALSIFIED en route): Setting `flat_params.interference_threshold = 1.5` in the
flat-corpus eval block would suppress the cycle 1 secondary spike by zeroing constructive
pairs, letting the 0.5 Hz drive dominate the DFT at k=1 and push carrier_emergence to ~0.85.

Secondary (CONFIRMED, dominant): The `DREAM_GRAVITY` phase computation had a latent bug
in `run_l5_dream_chain` that caused runaway amplitude gain on memories with unnormalized
phases (l4_noise: phase = π×i×0.7, up to ~220 rad; l4_decoy: π×i×0.31, up to ~100 rad).
Fixing the normalization eliminates artificial amplitude spikes in engine_flat, letting
the DFT signal emerge naturally.

## Bug description

### Buggy code (all prior sessions):
```rust
let raw = (phase0 - gravity_query_phase).abs();
let dphi = raw.min(two_pi - raw); // 0..pi
```

When `raw > 2π` (as for any l4_noise or l4_decoy memory):
  - `two_pi - raw` becomes negative
  - `raw.min(negative)` = negative value
  - `dphi` is negative → `align = 1.0 - dphi/π > 1.0` → `g >> 1.125`
  - For raw=219.5 rad: g ≈ 18× → amplitude explosion in engine_flat

In normal operation this was masked: constructive pair strengthening at cycle 0 pushes
memories to AMPLITUDE_CEILING=2.0, which also cap-limits gravity's effect. But when
pairs were disabled (threshold=1.5, trial 1) the explosion became visible.

### Fixed code:
```rust
let raw = (phase0 - gravity_query_phase).abs();
let raw_norm = raw % two_pi;                    // normalize to [0, 2π)
let dphi = raw_norm.min(two_pi - raw_norm);     // true circular distance in [0, π]
```

## Configuration

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_GRAVITY=0.25 DREAM_MODE=unset KURAMOTO_COUPLING=3.0`

## Trials

| trial | change vs baseline          | fitness  | carrier_e | transfer | xi_robust | amp_deltas_flat (cycles 1-4)          |
|-------|-----------------------------|----------|-----------|----------|-----------|---------------------------------------|
| 1     | threshold=1.5, no grav fix  | 0.082586 | 0.5213    | 0.8660   | 0.9611    | [35.7, 757, 16346, 366102] — explosion|
| 2     | threshold=1.5 + grav fix    | 0.079619 | 0.5005    | 0.8996   | 0.9611    | [0.881, 0.018, 0.020, 0.022]          |
| 3     | grav fix only               | 0.033676 | 0.9602    | 0.8996   | 0.9611    | [0.041, 0.003, 0.036, 0.014]          |
| 4     | grav fix only (confirm)     | 0.033704 | 0.9602    | 0.8996   | 0.9611    | [0.041, 0.003, 0.036, 0.014]          |
| 5     | grav fix only (full output) | 0.033653 | 0.9602    | 0.8996   | 0.9611    | [0.041, 0.003, 0.036, 0.014]          |

Baseline (2026-07-06, DREAM_MODE unset K=3): fitness=0.0579, carrier=0.652, transfer=0.941, xi=0.952

## Why threshold=1.5 made things worse even with the grav fix (trial 2)

With zero constructive pairs, some non-pair amplitude process (likely stage_strengthen_bridge_nodes
and/or gravity) produces a cycle 1 delta of 0.881 — larger than the baseline 0.192. This
is the opposite of the prediction.

Root cause: in normal operation (threshold=0.03), cycle 0 pair-strengthening pre-organizes
amplitudes (many pushed to AMPLITUDE_CEILING=2.0). Cycle 1's delta is the moderate residual
of that organized state. Without any pair-strengthening (threshold=1.5), cycle 1 sees a
more disordered amplitude landscape and produces a larger reorganization spike from other
sources. The secondary spike is not caused by pair-strengthening overflow — it is REDUCED
by it.

## Why the grav fix alone dramatically improves carrier (trial 3+)

With the bug active, l4_noise memories in engine_flat received gravity gains up to 18×
per cycle. After cycle 0, these memories dominated the amplitude landscape. Cycle 1's
massive reorganization of this artificially inflated state produced delta=0.192 regardless
of drive frequency.

With the bug fixed, l4_noise gains are correctly bounded by DREAM_GRAVITY=0.25:
  g ∈ [0.875, 1.125]. Cycle 0 produces a modest reorganization; cycles 1-4 carry
  mostly drive+gravity signal → amp_deltas_flat = [0.041, 0.003, 0.036, 0.014].

DFT of [0.041, 0.003, 0.036, 0.014]: k=1 (2 Hz) dominates → carrier_emergence = 0.9602.

## Impact on query_gravity metric

query_gravity dropped from 0.8623 to 0.4933. The bug was artificially amplifying l4_noise
memories toward the gravity attractor, inflating the query_gravity score. The fixed value
(0.4933) reflects true phase-aligned retrieval quality. The metric is logged separately
from the 13 fitness components, so this does not worsen fitness.

## Fitness breakdown (trial 3 vs baseline)

| metric             | weight | baseline | trial 3 | delta (fitness contribution) |
|--------------------|--------|----------|---------|------------------------------|
| carrier_emergence  | 0.10   | 0.652    | 0.9602  | saves 0.031                  |
| transfer_score     | 0.15   | 0.941    | 0.8996  | costs  0.006                 |
| xi_robustness_v2   | 0.15   | 0.952    | 0.9611  | saves  0.001                 |
| all others         | —      | —        | same    | ~0                           |
| **total fitness**  |        | 0.0579   | 0.0337  | **−0.024 (42% reduction)**   |

## Decision

**Keep the gravity phase normalization fix. Revert threshold=1.5 (not in codebase).**

Code change: `src/bin/research.rs` — one line added (`let raw_norm = raw % two_pi;`) and
one line modified (use `raw_norm` instead of `raw` in `raw.min(two_pi - raw)`).

New confirmed optimum:
```
DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE= (unset) DREAM_GRAVITY=0.25
KURAMOTO_COUPLING=3.0 (default) DRIVE_FREQ_HZ=0.5 (default)
```
New floor: **fitness ≈ 0.0337** (avg trials 3-5: 0.0338)
carrier_emergence = 0.960, transfer = 0.900, xi = 0.961

## New carrier floor analysis

The carrier_emergence floor under stage_sync is now at 0.960 (not 0.652).
All prior carrier_emergence analysis (2026-06-30, 2026-07-08) was done with the gravity
bug active and should be treated as invalid for understanding the mechanism.

The bug was introduced when l4_noise was added to build_l5_engine with phase = π×i×0.7
(unnormalized). The gravity attractor code assumed phases in [0, 2π).

## Sub-0.034 fitness directions

Transfer_score (0.900) is now the next dominant fitness driver:
- 0.15 × (1 - 0.900) = 0.015 (43% of total fitness)
- Previous env-var sweeps for transfer are partially confounded by the gravity bug
- KURAMOTO_COUPLING sweep (2026-07-06) showed K=3 optimal — reconfirm?
- DREAM_GRAVITY could be re-swept now that the bug is fixed (previous 0.25 optimum
  was found under the bug; the corrected gravity may have a different optimum)
- xi_robustness_v2 (0.961): was 0.952 under the bug; slightly improved

## Current overall best

```
fitness ≈ 0.0337  carrier=0.960  transfer=0.900  xi=0.961
```
Previous best: 0.0579. New record confirmed in 3 trials.
