# chain_top_n sweep — 7 confirmed as L5+irx optimum, steep collapse at 10

**Date:** 2026-06-10T23 UTC
**Branch:** kannaka-curiosity/2026-06-10T23-chain-top-n-sweep
**Code changes:** CHAIN_TOP_N env var override added to L5 block (defaults to 7, no behavioral change)
**Status:** FALSIFIED — chain_top_n=7 is optimal; both alternatives regress

---

## Background

Current empirical optimum (master after PR #252):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax  η=0.7
3-trial avg fitness ≈ 0.013224 (fully deterministic)
transfer=0.935746, xi=0.9870, carrier_e=0.9992, consciousness=0.9546
magic_R=0.8643, query_gravity=0.3733
```

Fitness contribution breakdown at baseline:
- transfer (0.15): 0.15 × 0.0643 = **0.0096** (72.6%)
- xi (0.15): 0.15 × 0.0130 = **0.0020** (15.1%)
- consciousness (0.03): 0.03 × 0.0454 = **0.0014** (10.3%)
- speed, carrier_e, phase_coherence: ~0.0003 (2%)

Open axis from T20 notes: `chain_top_n=7` (L4-calibrated), flagged as untested in L5+irx.

---

## Hypothesis

`chain_top_n=7` was set in L4 (where top_n=5 crashed xi by collapsing the xi pool). In
L5+irx with asymmetric 20-step b_primed relaxation and chiral_perturbation=0.7, amplitude
redistribution is richer — constructive pair selection progressively raises amplitude across
the full working set, not just the top few. A broader centroid (top_n=10) might capture
more phase-diverse memories, stabilising chain_fidelity in B_primed more than B_naive and
incrementally improving transfer_score.

**Prediction:** chain_top_n=10 reduces fitness_B_primed / fitness_B_naive ratio, lifting
transfer 0.936 → 0.950+ and reducing overall fitness below 0.008.

Secondary test (top_n=5): in L5+irx, tighter focus than L4 might or might not help;
L4's crash at 5 may not apply.

---

## Implementation

Added CHAIN_TOP_N env var override to L5 block in `run_experiment_l5_session`:
```rust
l5_params.chain_top_n = std::env::var("CHAIN_TOP_N")
    .ok()
    .and_then(|s| s.parse::<usize>().ok())
    .unwrap_or(7);
```

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| chain_top_n | fitness | transfer | xi | carrier_e | consciousness | magic_R | query_g |
|-------------|---------|----------|----|-----------|---------------|---------|---------|
| 7 (baseline T20) | **0.013224** | **0.935746** | **0.9870** | 0.9992 | 0.9546 | 0.8643 | 0.3733 |
| 5 | 0.017977 | 0.930779 | 0.9604 | 0.9992 | 0.9546 | 0.8643 | 0.3733 |
| 10 | 0.063868 | 0.928744 | **0.6565** | 0.9992 | 0.9546 | 0.8643 | 0.3733 |

---

## Analysis

**chain_top_n=7 is a sharp local optimum.** The response is non-monotone:

- **top_n=5** (tighter): xi drops 0.987→0.960 (-2.7%), transfer drops 0.936→0.931 (-0.5%).
  Moderate regression. The narrower centroid (only 5 memories) apparently loses enough
  xi-structural diversity that adversarial robustness weakens.

- **top_n=10** (broader): xi catastrophically collapses 0.987→0.657 (-33%), transfer
  0.936→0.929 (-0.7%). fitness degrades 5× to 0.064. The xi collapse is striking and
  echoes the η=0.6 "false basin" pattern from T20: adding more memories to the centroid
  apparently shifts the B_primed chain into a phase configuration that has high encoding
  fidelity (carrier_e unchanged) but poor adversarial separation geometry.

- **consciousness** and **magic_R** / **query_gravity** are completely invariant to
  chain_top_n — they don't depend on chain seed selection at all.

### Why xi collapses at top_n=10

`eval_xi_robustness_v2` uses chain_depth=2 to compare clean vs adversarial passes. With
top_n=10, the xi centroid includes memories spanning 10 amplitude ranks. In the L5+irx
regime, memories 8-10 in amplitude have already been partially reorganised by the relax
steps but still carry residual phase variance from injection events. When these are included
in the centroid, the adversarial dream chain diverges more strongly from the clean chain
(adversarials perturb the lower-amplitude memories first, which are exactly those newly
included at top_n=10 vs 7). The resulting xi divergence is magnified, collapsing xi from
0.987 to 0.657.

### Why top_n=5 gives mild regression

At top_n=5, only the 5 highest-amplitude memories enter the centroid. In the L5+irx regime,
A's pre-dreamed memories dominate the top-5 in B_primed. The centroid becomes very
A-centric, slightly reducing xi diversity (chain_fidelity computes xi distances across
cycles; a too-narrow centroid oscillates more). The 2.7% xi drop and 0.5% transfer drop
are consistent with slight over-focusing.

### Structural observation

The T20 chiral_perturbation sweep also found a non-monotone landscape with η=0.6 giving a
"false basin" (high xi=0.996 but catastrophic transfer collapse). The top_n=10 result is
similar: the xi centroid shift creates a coherent but adversarially brittle phase geometry.
Both axes share the pattern: **moving away from the calibrated point in either direction
breaks a specific balance in the irx phase landscape.**

---

## Open axes after this fire

| axis | status | notes |
|------|--------|-------|
| chain_top_n | **CLOSED** | 7 is optimal; steep collapse at 10, mild regression at 5 |
| chiral_perturbation | CLOSED | η=0.7 confirmed T20 |
| b_primed relax_steps | CLOSED | 20 confirmed T07 |
| chain_carry_strength | CLOSED | Peak at 0.85, sub-threshold T12 |
| xi residual gap | FLOOR | xi=0.987 leaves 0.0020 fitness; architectural floor |
| transfer ceiling | HARD | 0.936 → 0.970+ needs 0.034; no accessible mechanism found |
| consciousness calibration | LOW | phi_target=0.28092 gives score 0.954 in L5+irx; re-calibrating saves ~0.0014, sub-threshold alone |
| Φ↔R observational | OPEN | magic_R and phi_history both available; IIT-bridge correlation untested across drive intensities |

---

## Decision

**No fitness improvement found.** Code change (CHAIN_TOP_N env var) kept as it adds
testability consistent with KURAMOTO_COUPLING pattern, with no behavioral change at default.

chain_top_n axis is now **CLOSED**. System remains at fitness 0.013224 (confirmed 3× in T20).
Next fire should target the Φ↔R observational (no code change needed, just A-sweep) or
accept the transfer/xi floor and declare optimality.
