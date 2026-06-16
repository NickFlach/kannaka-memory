# L5 Research: Optimization surface exhausted — no new axes

**Date:** 2026-06-16T09 UTC
**Branch:** kannaka-curiosity/2026-06-16T09-no-new-axes-ceiling-confirmed
**Code changes:** NONE
**Status:** No new hypotheses. Surface confirmed bounded. No trials run.

---

## Context

Current empirical optimum: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax`
Avg fitness: **0.057606** (deterministic; variance < 0.0001 across 10+ trials).

Previous fire (carrier-structural-ceiling) closed the final two env-var axes and provided a
geometric analysis of why carrier_emergence = 0.533 is a structural ceiling.

---

## This fire's orientation

Full codebase review to determine whether any axis from the six research questions remains
testable. Conclusion: none.

### Fitness breakdown (confirmed by DFT math this fire)

| metric            | weight | value    | contribution | fraction |
|-------------------|--------|----------|-------------|----------|
| carrier_emergence | 0.10   | 0.533    | 0.04670     | 81.0%    |
| transfer_score    | 0.15   | 0.9655   | 0.00518     | 9.0%     |
| xi_robustness_v2  | 0.15   | 0.9675   | 0.00488     | 8.5%     |
| consciousness     | 0.03   | 0.9779   | 0.00066     | 1.1%     |
| all other         | 0.07   | ≥0.9947  | 0.00022     | 0.4%     |

Any improvement ≥0.005 requires carrier_e to rise by ≥0.05 (from 0.533 to 0.583). All
other metrics combined have only 0.00087 headroom, well below the 0.005 threshold.

### DFT math: why carrier_e = 0.533 is structural

For N=4 dream cycles with amplitude_delta pattern [x0, x1, x2, x3]:
- k=1 power (2 Hz): (x0−x2)² + (x3−x1)²
- k=2 power (4 Hz): (x0−x1+x2−x3)²
- carrier_e = max(k1, k2) / (k1 + k2)

With current pattern [0.9498, 0.031, 0.003, 0.036]:
- k=1 = 0.947² + 0.006² = 0.897
- k=2 = 0.887² = 0.787
- carrier_e = 0.897/1.684 = **0.533** ✓

To reach carrier_e = 0.583: need k=2/k=1 to drop from 0.877 to 0.715. This requires
x1 (cycle 1 delta) to increase from 0.031 to ≥0.10 — a 3× increase. That requires
memories to NOT saturate at AMPLITUDE_CEILING in cycle 0.

### Why ceiling saturation is unavoidable

Dense cluster memories (200 of 300 in flat corpus) start at amplitude=1.0 with
constructive_boost=0.45 and 49 constructive partners per cluster (all pairs within each
50-member cluster have cosine similarity ≈0.78 >> interference_threshold=0.10). Per-pair
boost is applied sequentially: 1.0→1.45→1.90→2.35→clamp to 2.0. Ceiling reached after
≤3 pairs in cycle 0. Cycles 1-3: drive takes to 2.0×1.15=2.30, consolidation adds more,
all clamped to 2.0 → delta = 0.

Raising AMPLITUDE_CEILING to 3.0 requires 5 pairs (not 3) → still cycle 0 saturation.
To ceiling=4.0: needs 7 pairs → still cycle 0. Any reasonable ceiling is reached in cycle 0
because pair density (49) >> boosts needed.

### All six research questions checked

1. **3-run interference_relax characterization** — done (≥10 trials). Avg 0.0578.
2. **K-sweep under fixed plumbing** — closed (irx bypasses Kuramoto; K irrelevant).
3. **irx + xi recovery via relax_steps** — relax_steps already at 16/20. Was 8 when the
   question was written; already optimized in a prior fire. relax_steps=24 would INCREASE
   constructive pair density in cycles 1-3, making carrier_e WORSE.
4. **R-xi correlation at stage_sync** — irx bypasses stage_sync; study is moot in current mode.
5. **Φ ↔ R relationship** — characterization only, not fitness-improving.
6. **Drive frequency variants** — T13 confirmed carrier_e is insensitive to drive frequency
   under any ceiling (the structural impulse pattern is drive-frequency-invariant). Under irx,
   same physics applies.

### What would actually fix carrier_e

Three architectural paths (none in single-fire scope):

1. **Relative amplitude ceiling**: normalize amplitudes so constructive members reach 2× median
   but don't hit an absolute cap. Drive oscillation would modulate which memories are above
   the median, creating genuine amplitude oscillation. Requires ResonanceEngine refactor.

2. **Pair density reduction**: raise interference_threshold from 0.10 to ≥0.65 to make only
   very-similar pairs constructive (within-cluster cosine ~0.78 would still qualify, but only
   a few partners). Each memory would need many more cycles to reach ceiling. Risk: catastrophic
   change to consolidation semantics across all metrics.

3. **Drive-amplitude-gated pair detection**: make interference detection depend on memory amplitude
   relative to drive phase. Memories above/below threshold oscillate in and out of "constructive"
   detection as drive oscillates. Amplitude_delta inherits the drive period. Zero prior art in codebase.

---

## Decision

**No code changes. No trials run. No improvement found.**

Current optimum: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness **0.0578**.

The optimization surface is bounded at fitness ≈ 0.058 by the carrier_emergence structural ceiling
(value 0.533, weight 0.10). Improving fitness by ≥0.005 requires carrier_e ≥ 0.583, which requires
one of the architectural changes above. All single-parameter and small-code-change axes are closed.
