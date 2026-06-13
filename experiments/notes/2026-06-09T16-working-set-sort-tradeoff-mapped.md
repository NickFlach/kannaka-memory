# Working-set sort (Path 2) tradeoff fully mapped — no fitness improvement

**Date:** 2026-06-09T16 UTC
**Branch:** kannaka-curiosity/2026-06-09T16-drop-path2-working-set-sort
**Code changes:** REVERTED — both variants (no sort, amplitude sort) reverted to T15 baseline
**Status:** FALSIFIED — neither alternative sort improves fitness beyond 0.005 threshold

---

## Background

Current empirical optimum (T15):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.037
carrier_e=0.936, transfer=0.841, xi=0.973 (stable)
magic_proxy_phase_R=0.871, query_gravity=0.374
```

Fitness breakdown:
- transfer_score (0.15 weight): 0.15*(1-0.841) = 0.0239 → 64% of total cost
- carrier_emergence (0.10): 0.10*(1-0.936) = 0.0064 → 17%
- xi_robustness_v2 (0.15): 0.15*(1-0.973) = 0.0041 → 11%
- consciousness (0.03): 0.03*(1-0.917) = 0.0025 → 7%

Transfer dominates at 64% of fitness cost. Improving it is the main lever.

Current code has two sorting changes from T13:
- **Path 1**: content-string BFS sort in `find_synchronized_clusters` (kuramoto.rs)
- **Path 2**: content-string working_set sort in `stage_replay` (consolidation.rs, line 344)

T13 showed: Path 1 alone gave transfer=0.948; Path 1+Path 2 gave transfer=0.816.
This implies Path 2 is actively HURTING transfer (0.948→0.816, then UUID fix partially
recovered it to 0.841).

---

## Hypothesis 1: Remove Path 2 (no working-set sort)

**Prediction**: transfer recovers toward 0.948, xi may drop but UUID fix stabilizes it,
net fitness improvement > 0.005.

**Code change**: Simplified stage_replay to return memories in HashMap iteration order
(no sort).

### Results (2 trials)

| metric | T15 baseline | no-sort T1 | no-sort T2 |
|--------|-------------|-----------|-----------|
| **fitness** | **0.037121** | **0.036542** | **0.036539** |
| transfer_score | 0.840641 | **0.928729** | **0.928729** |
| carrier_emergence | 0.9360 | 0.9003 | 0.9003 |
| xi_robustness_v2 | **0.9733** | 0.9057 | 0.9057 |
| consciousness | 0.9172 | 0.9546 | 0.9546 |
| magic_proxy_phase_R | 0.8709 | 0.8643 | 0.8643 |
| query_gravity | 0.3738 | 0.3733 | 0.3733 |

**All metrics byte-identical between no-sort T1 and T2 → fully deterministic.**

### Analysis of tradeoff

Removing Path 2 shifts the working-set order from content-adjacent to HashMap-random.
The `apply_targeted_chiral_perturbation` sliding window then targets DIFFERENT pairs:

| effect | direction | weight | fitness delta |
|--------|-----------|--------|---------------|
| transfer: 0.841 → 0.929 | improvement | 0.15 | +0.01324 |
| carrier_e: 0.936 → 0.900 | regression  | 0.10 | −0.00357 |
| xi: 0.973 → 0.906 | regression  | 0.15 | −0.01005 |
| consciousness: 0.917 → 0.955 | improvement | 0.03 | +0.00114 |
| **net** | | | **+0.00076** |

Net fitness improvement: **0.000585** (baseline 0.037126 → no-sort 0.036541).
This is far below the 0.005 threshold.

**The transfer gain is almost exactly offset by xi + carrier_e regressions.**

---

## Hypothesis 2: Amplitude-descending working-set sort

**Prediction**: High-amplitude memories targeted first → different pair selection,
potentially better carrier_e + xi without hurting transfer as much as content sort.

**Code change**: Sort working_set by amplitude descending (`total_cmp`).

### Result (1 trial)

| metric | T15 baseline | amplitude sort |
|--------|-------------|----------------|
| fitness | 0.037121 | **0.097220** |
| transfer_score | 0.840641 | **0.493804** |
| carrier_emergence | 0.9360 | 0.9587 |
| xi_robustness_v2 | 0.9733 | 0.9274 |
| consciousness | 0.9172 | 0.8652 |

**Catastrophically worse.** Amplitude sort destroys transfer (0.841→0.494) and
consciousness (0.917→0.865). Reverted immediately after 1 trial.

---

## Decision

**Both code changes REVERTED.** Content-sort (T15 baseline) is the best discovered
working_set ordering.

---

## What was learned

### The transfer/xi/carrier_e tension

`apply_targeted_chiral_perturbation` uses a sliding-window over working_set. The
working_set order determines which memory pairs get targeted. This creates a
three-way tradeoff:

| working_set sort | transfer | xi | carrier_e | fitness |
|-----------------|---------|-----|-----------|---------|
| none (random) | **0.929** | 0.906 | 0.900 | 0.0365 |
| content (current) | 0.841 | **0.973** | **0.936** | 0.0371 |
| amplitude desc | 0.494 | 0.927 | 0.959 | 0.0972 |

Content sort gives the best OVERALL fitness (0.0371 ≈ 0.0365 for no-sort, but
0.097 for amplitude sort). The working_set sort primarily determines the
transfer/xi tradeoff.

### Why content-adjacent pairs hurt transfer

Content sort pairs memories that are alphabetically adjacent in content string.
This concentrates the chiral perturbation on semantically-similar pairs. The
transfer metric measures how well corpus B benefits from corpus A's dream. If
the chiral perturbation targets same-corpus semantically-similar pairs, it may
reinforce corpus A's specific structure at the expense of generalizable patterns
that enable B transfer.

### Why removal gives stable results despite "random" HashMap order

With content-sort BFS (Path 1) in place, the cluster STRUCTURE is fully
deterministic. `apply_targeted_chiral_perturbation` uses a sliding window, but
the B-corpus transfer depends more on cluster-level generalization (determined
by Path 1) than on pair-level perturbation (determined by working_set order).
Transfer is 0.928729 byte-identical across both no-sort trials, confirming
transfer stability without Path 2.

---

## Open axes for future fires

| axis | mechanism | expected delta | priority |
|------|-----------|----------------|----------|
| **Working-set sort to break transfer/xi tension** | Find a sort that targets diverse-content pairs (improves transfer) while maintaining determinism (keeps xi) | uncertain | HIGH |
| **Transfer architecture** | B_primed construction or priming step improvement | ~0.020 potential | HIGH |
| **carrier_emergence** | Currently at 0.936; 0.998 was achievable in T13 Path1+Path2 but now dropped. Mechanism unclear with UUID fix present. | 0.006 | MEDIUM |
| **Drive frequency variants** | 1 Hz, 4 Hz, 0.5 Hz at A=0.1 with current optimal config | unknown | MEDIUM |

### Concrete next hypothesis

**Reverse-complement working_set sort**: sort by content REVERSED (z→a). This
creates content-opposite-adjacent pairs instead of content-similar-adjacent pairs.
If transfer degradation comes from same-semantics pairs being targeted, reverse
order would target semantically-distant pairs → potentially better transfer.
Prediction: transfer rises toward 0.929, xi slightly lower (but deterministic).

If reverse sort gives transfer > 0.88 while xi > 0.950, net fitness would be
0.15*(1-0.88) + 0.10*(1-0.93) + 0.15*(1-0.950) ≈ 0.018+0.007+0.0075 ≈ 0.033,
which would be a 0.004 improvement. Borderline.

More promising: combine reverse-sort working_set with a Phase 1 variant that
more strongly separates corpus A and B cluster structures.
