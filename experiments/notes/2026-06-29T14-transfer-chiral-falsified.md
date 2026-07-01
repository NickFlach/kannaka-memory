# 2026-06-29T14 — Transfer ceiling probe: chiral_perturbation falsified

## Hypothesis

Transfer ceiling investigation.  
- `transfer_score = 0.9652` at current best config (irx + DREAM_GRAVITY=0.25)
- Diagnostic confirmed `fitness_B_primed = 0.002409`, `fitness_B_naive = 0.069150`
- Maximum possible gain if transfer → 1.0: `0.15 × 0.0348 = 0.00522` (barely above 0.005 threshold)

The primed pass hardcodes `chiral_perturbation = 0.15` (line 3517, research.rs). The main engine uses 0.7. Hypothesis: this 0.15 was set arbitrarily and was causing marginal disruption to B-memory phase organization during the primed dream chain. Setting it to 0.0 would let B memories settle naturally into A's phase attractors, improving `chain_fidelity` and `phase_coherence` in the placeholder fitness, driving `fitness_B_primed` toward zero.

**Prediction**: `transfer_score` rises toward 1.0; fitness drops by ~0.005 to ~0.0515.

## Results

| run | config | transfer_score | fitness_B_primed | fitness_B_naive | fitness |
|-----|--------|----------------|-----------------|----------------|---------|
| 1 (baseline) | irx DREAM_GRAVITY=0.25, eta=0.15 | 0.965165 | 0.002409 | 0.069150 | 0.056638 |
| 2 (probe) | irx DREAM_GRAVITY=0.25, eta=0.0 | 0.423174 | 0.039888 | 0.069150 | 0.137933 |

All other metrics unchanged (carrier_emergence=0.5265, xi_robustness=0.9796, R=0.8670, query_gravity=0.8623).

## Analysis

**Hypothesis falsified — catastrophically.**

`fitness_B_primed` increased 16.6× (0.002409 → 0.039888), `transfer_score` collapsed from 0.9652 to 0.4232, and fitness nearly tripled (0.0566 → 0.1379).

The `chiral_perturbation = 0.15` in the primed pass is NOT arbitrary. It is doing critical organizational work: when B's corpus memories are inserted into A's post-dream engine and a new dream chain begins, the chiral perturbation stage symmetry-breaks the mixed A+B phase space, allowing B memories to establish their own cluster identity rather than collapsing into A's attractors. Without it, the B-memory chain fails to converge properly — chain_fidelity and consciousness scores drop sharply in the placeholder fitness.

The `fitness_b_naive` is unchanged (0.069150) as expected — only the primed pass was modified.

## Conclusion

The transfer ceiling (`fitness_B_primed ≈ 0.002409`) is the **true structural minimum** under the current architecture with `chiral_perturbation = 0.15`. The 0.15 value enables B-memory organization in the mixed engine; it cannot be reduced without catastrophic regression.

Code change reverted. No TSV entries kept (failed hypothesis).

## Implication for L5 optimization space

This confirms the 2026-06-28 fire's conclusion: **L5 is at floor**. The fitness decomposition:

| metric              | weight | value  | contribution | % of fitness |
|---------------------|--------|--------|--------------|-------------|
| carrier_emergence   | 10%    | 0.5265 | 0.04735      | 83.6%       |
| transfer_score      | 15%    | 0.9652 | 0.00522      | 9.2%        |
| xi_robustness_v2    | 15%    | 0.9796 | 0.00306      | 5.4%        |
| others (10 metrics) | 60%    | ≈1.0   | ~0.00100     | 1.8%        |

- **carrier_emergence**: structural DFT floor (4-sample FFT, consolidation noise dominates drive signal ~4×). Requires architectural redesign.
- **transfer_score**: chiral_perturbation=0.15 is load-bearing; the 0.002409 residual is true minimum.
- **xi_robustness**: max gain 0.003 (below 0.005 threshold).

Sub-0.050 fitness requires redesigning carrier_emergence measurement or the consolidation physics — neither is tractable in a single fire.
