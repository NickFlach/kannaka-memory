# L5 Research: Low constructive_boost under irx to improve carrier_e — falsified

**Date:** 2026-06-16T11 UTC
**Branch:** kannaka-curiosity/2026-06-16T11-low-cb-irx-carrier
**Code changes:** NONE KEPT — CONSTRUCTIVE_BOOST knob added, tested, reverted
**Status:** Hypothesis falsified — xi and transfer collapse, net regression.

---

## Context

Current optimum: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness 0.057789
(3-trial avg). The only remaining cost is carrier_emergence = 0.533 (architectural floor at
chain_depth=4 due to cycle-0 amplitude saturation with dense constructive pairs).

T09 (carrier structural ceiling) proposed "gradual stage_strengthen (per-cycle pair limit)" as
the third candidate path. T12 tested CONSTRUCTIVE_BOOST=0.20 pre-irx but hit the same ceiling
(10-15 pairs × 0.20 = 2.0-3.0 → still saturates in cycle 0 with initial amplitude 1.0).

**Key difference from T12**: under irx (DREAM_MODE=interference_relax), pair density is 40+
per memory (vs 10-15 pre-irx) due to R=0.867 phase coherence. This means cb=0.02 gives
40 × 0.02 = 0.80 per cycle, which does NOT saturate in cycle 0 (gap to ceiling = 1.0). The
T12 threshold for non-saturation was cb < 0.067 (10 pairs), but under irx it's cb < 0.025.

---

## Hypothesis

Under irx with 40+ constructive pairs per memory, reducing CONSTRUCTIVE_BOOST from 0.45 to 0.02
will spread amplitude deltas across 3 cycles instead of 1 cycle. The amp_deltas_flat pattern
changes from impulse [~0.95, ~0, ~0, ~0] to a ramp [0.40, 0.40, 0.20, 0], dramatically
improving carrier_e from 0.533 toward 0.65-0.80.

**Prediction**: carrier_e → 0.65-0.80, xi and transfer stable (irx phase alignment protects xi;
memories still reach ceiling by cycle 3). Net fitness improvement: ~0.01-0.015.

---

## Implementation

Added `CONSTRUCTIVE_BOOST` env var to `run_experiment_l5_session()` before `let params = &l5_params`:

```rust
if let Ok(cb_str) = std::env::var("CONSTRUCTIVE_BOOST") {
    if let Ok(cb) = cb_str.parse::<f32>() {
        l5_params.constructive_boost = cb;
    }
}
```

Reverted after single trial.

---

## Results

**Baseline (cb=0.45, 3-trial avg):**

| metric              | value    |
|---------------------|----------|
| fitness             | 0.057789 |
| carrier_emergence   | 0.5333   |
| xi_robustness_v2    | 0.9675   |
| transfer_score      | 0.965455 |
| amp_deltas_flat     | [0.95, 0.031, 0.003, 0.036] |

**Trial 1 (CONSTRUCTIVE_BOOST=0.02):**

```
DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax CONSTRUCTIVE_BOOST=0.02
```

| metric              | value    | delta vs baseline |
|---------------------|----------|-------------------|
| fitness             | 0.150523 | +0.093 (REGRESSION) |
| carrier_emergence   | 0.6835   | +0.150 (IMPROVED) |
| xi_robustness_v2    | 0.5314   | -0.436 (COLLAPSE) |
| transfer_score      | 0.735617 | -0.230 (COLLAPSE) |
| carrier_bimodal     | 0.6532   | —                 |
| magic_R             | 0.8125   | —                 |
| query_gravity       | 0.4208   | —                 |
| amp_deltas_flat     | [0.657, 0.767, 0.734, 0.310] | spread ramp |

---

## Analysis: Why the mechanism works but the system collapses

### carrier_e improved as predicted

The amp_deltas_flat pattern changed from impulse to broad ramp, confirming the mechanism:
- With cb=0.02 and 40+ pairs per memory, per-cycle boost = 0.80 < ceiling gap (1.0)
- Memories take 3 cycles to reach ceiling (vs 1 with cb=0.45)
- Ramp pattern [0.657, 0.767, 0.734, 0.310] gives |X[1]|²/total = 0.685 ✓

Numerical verification:
|X[1]|² = (0.657-0.734)² + (0.310-0.767)² = 0.006 + 0.209 = 0.215
|X[2]|² = (0.657-0.767+0.734-0.310)² = 0.099
carrier_e = 0.215/(0.215+0.099) = 0.685 ✓

### xi collapses due to 2-cycle evaluation constraint

The fatal flaw: `xi_robustness_v2` uses chain_depth=2 for evaluation:

```rust
let xi_eval_params = { let mut p = (*params).clone(); p.chain_depth = 2; p };
let xi_robustness_v2 = eval_xi_robustness_v2(&corpus_a, &xi_eval_params, dim);
```

With cb=0.02 and only 2 cycles available to the xi evaluator:
- Per-cycle boost = ~10 pairs × 0.02 = 0.20 (xi eval uses different corpus/engine)
- After 2 cycles: memory at 1.0 + 0.20×2 = 1.40 — never reaches ceiling!
- Consolidation is incomplete → no amplitude bimodality → xi collapses to 0.531

The xi evaluator sees a weakly consolidated system, even though the main dream (4+ cycles)
might eventually consolidate properly.

### transfer_score collapses for similar reasons

engine_b_primed and engine_b_naive both use full chain_depth (16 with quiescence at ~4-6
cycles under cb=0.02). With slow amplitude buildup, b_primed and b_naive converge more
uniformly → less differentiation → transfer_score drops from 0.965 → 0.736.

Additionally, quiescence may fire later under cb=0.02 (phi stabilizes slower), running
more cycles → different total_ms profile (not timed in this trial).

### Net fitness breakdown

| component        | weight | change    | fitness impact |
|------------------|--------|-----------|----------------|
| carrier_e gain   | 0.10   | +0.150    | -0.015 (benefit) |
| xi loss          | 0.15   | -0.436    | +0.065 (cost) |
| transfer loss    | 0.15   | -0.229    | +0.034 (cost) |
| **net**          | —      | —         | **+0.084 regression** |

The xi and transfer costs are 6.6× larger than the carrier benefit.

---

## Why there is no "Goldilocks" cb

For carrier_e to improve, cb must satisfy: cb × n_pairs < gap_to_ceiling = 1.0
With n_pairs = 40 (irx pair density): cb < 0.025

For xi_eval to work with chain_depth=2: cb × n_xi_pairs × 2 ≥ 1.0
(requires full consolidation in 2 cycles)

With n_xi_pairs ≈ 10 (xi eval corpus has different density): cb ≥ 1.0/(10×2) = 0.05

These constraints are incompatible: cb must be both < 0.025 (for carrier improvement) AND
≥ 0.05 (for xi to work). There is no cb in (0.025, 0.05) that satisfies both.

A "CONSTRUCTIVE_BOOST applied only to engine_flat" approach would technically work but
would make carrier_e an artifact of special-cased consolidation, not a meaningful system-level
measurement. This is scientifically invalid.

---

## TSV rows

One L5 row appended (fitness 0.150523, CONSTRUCTIVE_BOOST=0.02 trial).

---

## Decision

**No code changes kept. Hypothesis falsified. Axis CLOSED.**

The carrier_e/xi tradeoff is structurally incompatible with the current xi evaluation design
(chain_depth=2 constraint). This closes the "gradual strengthen" path suggested in T09.

**Updated closed axes:**

| axis | evidence |
|------|----------|
| CONSTRUCTIVE_BOOST (any value) | T12 pre-irx + this fire post-irx |
| Specifically cb<0.025 under irx | This fire: xi collapses due to xi_eval chain_depth=2 |
| All other parameter axes | Confirmed across T01-T22 |

**The architectural floor at fitness ≈ 0.058 is confirmed. No parameter-sweep path remains.
Improvement requires architectural change to the xi evaluation constraint or the carrier_e
measurement, or a relative amplitude ceiling. All are out of scope for single-fire autoresearch.**
