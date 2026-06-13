# chiral_perturbation=0.10 for b_primed — axis floor found, sub-threshold

**Date:** 2026-06-11T01 UTC
**Branch:** kannaka-curiosity/2026-06-11T01-chiral-bp-zero
**Code changes:** REVERTED — sub-threshold improvement
**Status:** FALSIFIED (chiral_bp=0.00) + CHARACTERISED (chiral_bp=0.10 is axis floor)

---

## Background

Current empirical optimum (master after PR #253):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (fully deterministic)
transfer=0.935746, carrier_e=0.9992, xi=0.9870
magic_R=0.864, query_gravity=0.373
fitness_B_primed=0.003887, fitness_B_naive=0.060498
```

Open axis from T17 (chiral-bp-asymmetric fire):
> "Halving chiral_p from 0.70→0.35 gives −25% fp reduction (fp: 0.003887→0.002923,
> fitness: 0.013337→0.011011). Next fire: test chiral_p=0.10 for b_primed.
> Expected fp ≈ 0.002000, fitness ≈ 0.0083 (at threshold).
> chiral_p=0.00 predicted to give fp ≈ 0.001500, fitness ≈ 0.007."

The T17 open axis was never followed up — subsequent fires (T18-T20) tested global
chiral_perturbation sweeps, and T21-T22 tested chain_top_n.

---

## Hypothesis

`stage_chiral_perturbation` (η=0.7) runs as Stage 9 AFTER the 20-step
interference_relax for b_primed. It applies phase perturbation:
```
Δφ = eta × handedness × sin(2φ)   # up to 0.7 rad at eta=0.70
```
This partial undoing of the 20-step phase alignment is the dominant mechanism.
Lower eta → less phase disruption → phases stay closer to constructive-pair attractor
→ better chain_fidelity in b_primed → lower fp → higher transfer.

T17 tested 0.35 and found −25% fp reduction but sub-threshold. Linear extrapolation
predicted fp would reach ~0.002000 at eta=0.10 and ~0.001500 at eta=0.00.

**Prediction:** chiral_bp=0.10 → fp ≈ 0.002000, transfer ≈ 0.967, fitness ≈ 0.0083
(at threshold). chiral_bp=0.00 → fp ≈ 0.001500, fitness ≈ 0.007 (crosses threshold).

---

## Implementation

In `src/bin/research.rs` L5 block, before `run_l5_dream_chain` for b_primed:
```rust
let mut params_bp = params.clone();
params_bp.chiral_perturbation = std::env::var("CHIRAL_BP")
    .ok()
    .and_then(|s| s.parse::<f32>().ok())
    .unwrap_or(0.0);
// pass &params_bp instead of params to run_l5_dream_chain and eval_l5_placeholder_fitness
```

xi, carrier_e, magic_R, query_gravity are all structurally protected (measured on
clean/adv/flat/a engines, not b_primed).

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| chiral_bp | fitness | transfer | fp | xi | carrier_e | magic_R | query_g |
|-----------|---------|----------|----|----|-----------|---------|---------|
| 0.70 (baseline) | 0.013337 | 0.935746 | 0.003887 | 0.9870 | 0.9992 | 0.8643 | 0.3733 |
| **0.10** | **0.010137** | **0.957321** | **0.002582** | 0.9870 | 0.9992 | 0.8643 | 0.3733 |
| 0.05 | 0.010446 | 0.955073 | 0.002718 | 0.9870 | 0.9992 | 0.8643 | 0.3733 |
| 0.00 | 0.062371 | 0.608905 | 0.023661 | 0.9870 | 0.9992 | 0.8643 | 0.3733 |

**Best result:** chiral_bp=0.10 → fitness 0.010137 (Δ −0.003200 from baseline)

---

## Analysis

### The cliff at chiral_bp=0.00

The T17 linear extrapolation was wrong in the chiral_bp=0.00 direction.
fp at 0.00 = 0.023661 — **6× WORSE** than baseline, not 60% better as predicted.

The mechanism: `eval_l5_placeholder_fitness` includes `encoding_entropy` (0.05 weight).
`eval_encoding_entropy` measures vector diversity across surviving memories. With
chiral_p=0.00, the vector perturbation component (`eta × similarity` scaling) is
completely disabled. B memories in b_primed retain their fresh, unperturbed vectors
from corpus B generation — these are close to the dense centroids of corpus B,
creating a near-degenerate set. The encoding_entropy collapses → fp spikes.

The T17 notes actually identified this mechanism but assumed the phase-stability
improvement would dominate. The data shows the vector-diversity mechanism dominates
at very low chiral values.

### The optimum at chiral_bp=0.10

At 0.10, two competing mechanisms:
1. **Phase stability** (favors lower η): less phase disruption → chain_fidelity improved
2. **Vector diversity** (favors higher η): more chiral → better encoding_entropy in fp

At 0.10 vs 0.05:
- 0.10 is better than 0.05 (fp: 0.002582 vs 0.002718)
- The phase-stability benefit still slightly outweighs the diversity cost
- At 0.05, the diversity cost has grown to exceed the phase-stability benefit

The optimum is at chiral_bp ≈ 0.10. The T17 point (0.35) was on the "phase stability
dominates" side of the curve. The true optimum is shifted left toward 0.10.

### Curve summary

| chiral_bp | fp | transfer | fitness | notes |
|-----------|-----|----------|---------|-------|
| 0.70 | 0.003887 | 0.9357 | 0.01334 | global baseline |
| 0.35 | 0.002923 | 0.9517 | 0.01101 | T17 (−25% fp) |
| **0.10** | **0.002582** | **0.9573** | **0.01014** | **this fire (−34% fp, best)** |
| 0.05 | 0.002718 | 0.9551 | 0.01045 | slightly worse than 0.10 |
| 0.00 | 0.023661 | 0.6089 | 0.06237 | catastrophic cliff |

The response is non-monotone with:
- Decreasing fp from 0.70 → 0.10 (phase stability wins)
- A cliff between 0.10 and 0.00 (encoding_entropy collapse dominates)
- Optimum clearly at **chiral_bp ≈ 0.10**

### Why the threshold is not crossed

Best achievable: fp=0.002582 at chiral_bp=0.10.
Threshold-crossing fp: ≤0.002240 (requires transfer ≥ 0.963, fitness ≤ 0.008337).
Gap: 0.000342 (13% further fp reduction needed).

The chiral axis cannot close this gap:
- 0.10 is the axis optimum
- Lower values increase fp (cliff toward 0.00)
- Higher values approach baseline

### What controls the fp floor at 0.10

fp = sum of placeholder fitness components for b_primed:
- consciousness (0.10 weight): phi convergence toward 0.28092
- chain_fidelity (0.10 weight): xi-centroid cosine similarity across 4 dream cycles
- encoding_entropy (0.05 weight): vector diversity
- noise_removal (0.05), signal_preservation (0.05), phase_coherence (0.05)

At chiral_bp=0.10, the fp floor of 0.002582 is driven by structural limits:
- chain_fidelity: the 4-cycle chain with top_n=7 and relax_steps=20 reaches a
  deterministic attractor; further chiral reduction increases fp (diversity loss)
- consciousness: phi trajectory through 4 cycles is near the target; small residual
- encoding_entropy: marginal degradation at 0.10 vs 0.70 chiral

The 0.000342 gap to threshold-crossing fp appears architectural: the current
memory insertion scheme (B at fresh phases), combined with the 4-cycle chain depth,
has a structural floor for fitness_b_primed that chiral manipulation cannot bridge.

---

## Constraints established

| constraint | status | value |
|-----------|--------|-------|
| chiral_bp axis floor | NEW | 0.10 (cliff below, baseline above) |
| chiral_bp = 0.00 | CLOSED | catastrophic collapse (fp +508%) |
| chiral_bp = 0.10 | CONFIRMED | −34% fp, sub-threshold improvement |
| threshold-crossing fp | REQUIRES | fp ≤ 0.002240; current best 0.002582 |

---

## Transfer ceiling: architectural analysis

At the current best (chiral_bp=0.10, not kept):
- fp=0.002582, fn=0.060498, transfer=0.957321
- Remaining transfer gap: 1 − 0.957 = 0.043 → 0.15 × 0.043 = 0.0065 fitness contribution

To reach threshold (fitness ≤ 0.008337) from current baseline (0.013337):
- Need Δ ≥ 0.005000 total
- Best single-axis gain available: chiral_bp=0.10 gives Δ = 0.003200
- Remaining gap after chiral_bp=0.10: 0.001800

The 0.001800 gap has no clear single-axis path given:
- chain_top_n: closed at 7
- relax_steps: closed at 20 for b_primed
- alpha_base: global 0.15 tested and falsified (2026-06-08); asymmetric b_primed
  version untested but risky given the integral budget constraint
- chain_carry_strength: closed (regresses in T07 regime)
- chiral_perturbation: this fire confirmed the axis floor

**Possible combination path:** chiral_bp=0.10 (−0.003200) + a second axis that contributes
≥0.001800 more. The only untested mechanism is asymmetric alpha_base for b_primed:
raising alpha_base from 0.10 to ~0.12 specifically for engine_b_primed (same isolation
as T07 relax_steps fix). This was not attempted this fire due to trial budget.

---

## Decision

**No code changes retained.**

chiral_bp axis is characterized: floor at 0.10, cliff below, sub-threshold gain only.
Transfer ceiling now better understood: fp structural floor at ~0.002582 with current
architecture. The 0.001800 gap to threshold likely requires an architectural insight
or a new combination.

---

## Updated open axes

| axis | status | notes |
|------|--------|-------|
| chiral_bp | CHARACTERISED | floor at 0.10 (−34% fp, sub-threshold); cliff at <0.10 |
| alpha_base for b_primed | OPEN (UNTESTED) | global 0.15 failed (2026-06-08); asymmetric b_primed untested |
| transfer combination path | SPECULATIVE | chiral_bp=0.10 + alpha_bp=0.12 could stack; needs 1 trial |
| xi residual gap | CLOSED | 0.987, architectural limit |

**Candidate next fire hypothesis:** Combine chiral_bp=0.10 + asymmetric alpha_base for
b_primed (e.g. 0.12). Combined prediction: fp ≈ 0.002000, transfer ≈ 0.967, fitness ≈ 0.0083
(at threshold). Requires code change to alpha_base selection in stage_interference_relax.
