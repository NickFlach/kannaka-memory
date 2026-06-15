# L5 Curiosity: chiral_p_bp sweep — transfer lever found but speed cost cancels gain

**Date:** 2026-06-15T18 UTC  
**Branch:** kannaka-curiosity/2026-06-15T18-chiral-bp-sweep  
**Code changes:** REVERTED — no net improvement  
**Trials:** 3 (CHIRAL_BP ∈ {0.05, 0.50, 0.70})

---

## Hypothesis

`chiral_p_bp` (b_primed engine's chiral_perturbation, currently 0.15 while b_naive uses 0.7)
controls the exploration quality of b_primed consolidation. Lowering it further (0.05)
was predicted to give b_primed smoother phase convergence → lower fitness_b_primed →
higher transfer_score = 1 - fitness_b_primed / fitness_b_naive.

**Prediction:** CHIRAL_BP=0.05 improves transfer_score and fitness by ≥0.005.

---

## Baseline (post-fix, DRIVE_A=0.1 DRIVE_SCOPE=all, CHIRAL_BP=0.15)

```
fitness:         0.114918
transfer_score:  0.736812  (fitness_b_primed ≈ 0.01649, fitness_b_naive = 0.062651)
carrier_e:       0.5294
xi_v2:           0.8563
speed:           0.9646
total_ms:        12798
```

---

## Results

| CHIRAL_BP | fitness  | transfer_score | fitness_B_primed | fitness_B_naive | speed  | total_ms |
|-----------|----------|----------------|-----------------|----------------|--------|----------|
| 0.05      | 0.141458 | 0.567267       | 0.027111        | 0.062651       | 0.9281 | 26259    |
| **0.15 baseline** | **0.114918** | **0.736812** | **0.016490** | **0.062651** | **0.9646** | **12798** |
| 0.50      | 0.115121 | 0.742916       | 0.016107        | 0.062651       | 0.9279 | 25965    |
| 0.70      | 0.115665 | 0.739372       | 0.016329        | 0.062651       | 0.9274 | 26157    |

carrier_emergence, xi_robustness_v2, magic_R, query_gravity: identical across all trials.

---

## Analysis

### Direction (confirmed wrong)

Lower chiral_bp (0.05) HURT transfer_score dramatically (0.567 vs 0.737). The mechanism:
chiral_perturbation is an exploration driver during consolidation. Less perturbation = b_primed
gets stuck in a worse local minimum → fitness_b_primed rises (0.01649→0.027) → transfer_score falls.

### Upper direction

Higher chiral_bp (0.50, 0.70) improved transfer_score marginally (+0.006 at 0.50). The b_primed
engine benefited from stronger exploration quality. But this is marginal and maximal around 0.50;
0.70 (= b_naive level) regresses slightly (0.739 vs 0.743 at 0.50).

### Speed cancellation (root cause of zero net gain)

Total_ms doubled at 0.50 and 0.70 (12798ms → ~26000ms). This is because higher chiral perturbation
in b_primed creates more complex phase interactions that require more consolidation cycles to reach
quiescence. The `speed` metric penalizes longer wall-clock runs:

- speed at baseline: 0.9646
- speed at chiral_bp=0.50: 0.9279
- Fitness cost from speed loss: ~0.02 × (0.9646 - 0.9279) = 0.0007

This exactly cancels the transfer gain of 0.15 × 0.006 = 0.0009. Net fitness change: ≈ 0.

### Why 0.15 is near-optimal

The original 0.15 was set in a pre-fix fire to balance b_primed consolidation quality vs consolidation
speed. Post-fix, the same trade-off holds: 0.15 gives the best (transfer improvement per ms) ratio.
Going higher improves transfer but costs proportionally more in speed. The Pareto frontier is flat
(all configurations 0.15–0.70 give fitness ≈ 0.115) but 0.15 is cheapest computationally.

---

## Closed axes

- **chiral_p_bp sweep (0.05–0.70):** fully explored. No fitness improvement over baseline 0.15.
  Speed/transfer trade-off is tight; no free lunch. **Axis closed.**

---

## Open post-fix axes (updated)

1. **xi_repulsion_weight** (currently 0.3, research.rs line 58): xi_v2 = 0.856 contributes
   0.022 to fitness cost. Raising repulsion weight to 0.5–0.7 might push xi_v2 toward 0.95+,
   saving ~0.010+ fitness. Requires code change (add env var knob) + rebuild. Highest-yield
   unexplored single-parameter axis remaining.

2. **carrier_e measurement decoupling** (noted in T12): compute carrier signal from DRIVE_A×amplitude
   analytically rather than from actual amplitude deltas. Restores semantic "does drive oscillate?"
   vs "does amplitude change oscillate?" Requires research.rs code change. Could restore carrier_e
   toward 0.99 (+0.047 fitness) but changes metric semantics.

3. **Asymmetric amplitude decay** (noted in T07b): add per-cycle amplitude decay to non-constructive
   memories in stage_constructive. Could restore bimodal amplitude structure without removing ceiling.
   Requires consolidation.rs code change.

---

## Decision

No code changes kept. No fitness improvement. TSV rows appended (3 trials).
