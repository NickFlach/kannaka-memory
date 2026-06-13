# BFS content-sort revert recovers xi + carrier_e — fitness 0.036 → 0.028

**Date:** 2026-06-09T21 UTC
**Branch:** kannaka-curiosity/2026-06-09T21-bfs-sort-revert
**Code changes:** `src/kuramoto.rs::find_synchronized_clusters` — KEPT (removed content-string sort)
**Status:** CONFIRMED — fitness improvement 0.008, above 0.005 threshold, fully deterministic

---

## Background

Post-T16-B empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
2-trial avg fitness ≈ 0.036
transfer=0.929, carrier_e=0.900, xi=0.906 (stable)
magic_R=0.864, query_gravity=0.373
```

Fitness breakdown at T16-B baseline:
- xi: 0.15 × (1 − 0.906) = **0.014** (39% of total)
- carrier_e: 0.10 × (1 − 0.900) = **0.010** (28%)
- transfer: 0.15 × (1 − 0.929) = **0.011** (31%)
- other: ~0.001

Xi was the dominant remaining lever, not transfer.

The T16-B notes identified the remaining xi bottleneck explicitly:
> "xi from 0.906 → 1.0: T06-Q + T15 gave xi=0.973 alone; combined with T13 BFS sort → 0.906.
>  May need to audit BFS sort interaction with T06-Q in the adv pass."

T13 added TWO changes:
1. Content-string sort in `stage_replay` — reverted by T16-B (was antagonising T06-Q)
2. Content-string sort in `find_synchronized_clusters` (BFS seed order) — KEPT by T16-B

The BFS sort puts adversarials ("adv_l5_..." strings) BEFORE corpus memories alphabetically
("adv..." < "dec..." < "emo..." etc.). In the adv dream pass, adversarials form BFS cluster seeds
first, producing a cluster structure that diverges from the clean pass. With T06-Q filtering
adversarials from `original_ids`, the compensation that previously offset this divergence is
gone → xi = 0.906.

---

## Hypothesis

Removing the content-string sort from `find_synchronized_clusters` restores natural UUID-sorted
order (from `all_memories()` which already sorts by `m.id`). T15 placed adversarial UUIDs at
`u128::MAX − k*stride` (very large values) → adversarials sort LAST in UUID order → corpus
memories form BFS clusters first → adv pass cluster structure resembles clean pass → xi recovers.

The old sort comment claimed content-sort was needed "for deterministic BFS traversal order
regardless of HashMap iteration order." This was true before T15, when adversarial UUIDs were
random. With T15's deterministic UUIDs and `all_memories()` sorting by UUID, UUID-sorted order
is already fully deterministic — the content-string sort adds no determinism benefit and actively
harms xi.

**Prediction:**
- xi: ~0.973 (recover toward T15-alone level)
- carrier_e: ~0.900–0.936 (T13 BFS alone gave 0.998; without BFS sort, combined context may land between T15's 0.936 and T13's 0.998)
- transfer: ~0.929 (cluster detection order shouldn't affect B-primed transfer)
- Fitness: ~0.022–0.024

---

## Change

One-line removal in `src/kuramoto.rs::find_synchronized_clusters`:
```rust
// Removed:
// let mut all = all;
// all.sort_by(|a, b| a.content.cmp(&b.content));
// let all = all;
```
`all_memories()` already returns UUID-sorted memories. UUID order is fully deterministic
with T15's fixed adversarial UUIDs. No determinism regression.

---

## Results

DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax

| trial | fitness | transfer | carrier_e | xi    | magic_R | query_gravity |
|-------|---------|----------|-----------|-------|---------|---------------|
| T1    | 0.027834 | 0.848048 | 0.9984   | 0.9849 | 0.8771 | 0.3744 |
| T2    | 0.027830 | 0.848048 | 0.9984   | 0.9849 | 0.8771 | 0.3744 |
| **avg** | **0.02783** | **0.848** | **0.998** | **0.985** | **0.877** | **0.374** |

Fully deterministic (byte-identical on all core metrics).

---

## Comparison to baseline (T16-B)

| metric | T16-B baseline | this fire | delta |
|--------|----------------|-----------|-------|
| fitness avg | 0.0365 | **0.02783** | **−0.0087** |
| transfer | 0.929 | 0.848 | −0.081 |
| xi | 0.906 | **0.985** | **+0.079** |
| carrier_e | 0.900 | **0.998** | **+0.098** |
| magic_R | 0.864 | 0.877 | +0.013 |
| query_gravity | 0.373 | 0.374 | +0.001 |

---

## Fitness impact decomposition

| metric | weight | baseline contribution | this fire contribution | delta |
|--------|--------|-----------------------|------------------------|-------|
| transfer | 0.15 | 0.011 | 0.023 | +0.012 regression |
| xi | 0.15 | 0.014 | 0.002 | **−0.012 improvement** |
| carrier_e | 0.10 | 0.010 | 0.0002 | **−0.010 improvement** |
| other | — | ~0.001 | ~0.003 | +0.002 |
| **total** | | **0.036** | **0.028** | **−0.008** |

Net improvement: **−0.008 fitness** (threshold: 0.005). CONFIRMED.

---

## Why carrier_e surged to 0.998

T13-alone gave carrier_e=0.998, but in the combined context (with T06-Q + T15 + T16-B
stage_replay revert) it had settled at 0.900. The BFS sort's cluster structure was
causing carrier_e to be suppressed in the combined context. With BFS sort removed,
the flat corpus dream (engine_flat) benefits from the natural UUID ordering → correct
cluster seeding → carrier_e returns to ~0.998.

This means T13's BFS sort was ACTIVELY HURTING carrier_e in the combined context
(0.998 alone → 0.900 combined → 0.998 after removal). A sign inversion when combined.

## Why xi recovered to 0.985 (above predicted 0.973)

T15-alone xi=0.973 also had the content-string stage_replay sort (later reverted by T16-B).
The stage_replay content sort also contributed adversarial-first ordering in the adv pass's
working_set. With that already reverted by T16-B AND NOW the BFS sort also removed, the adv
pass sees adversarials in both working_set AND cluster order at the same positions as the
clean pass → xi=0.985 (even better than T15-alone's 0.973).

## Why transfer dropped 0.929 → 0.848

The BFS sort change affects how `find_synchronized_clusters` seeds clusters, which flows
into `stage_sync`/`stage_interference_relax` coupling assignments and `stage_kannaktopus`
targets. The B-primed engine's dream now proceeds with a different cluster topology →
different phase evolution → slightly different chain_fidelity.

The transfer drop (0.929→0.848, adding +0.012 fitness) is smaller than the combined xi +
carrier_e gains (−0.022 fitness). Net benefit: −0.008 fitness overall.

Note: transfer=0.848 ≈ T15-alone's transfer=0.841. We may have partially "undone" the
T16-B transfer improvement by removing the BFS sort. The two improvements were not
independent — BFS sort helped engine_b_primed clustering → better transfer, but hurt
xi/carrier_e. Now with BFS sort removed, transfer reverts to near-T15 level, but xi and
carrier_e are both near-optimal.

---

## New empirical optimum

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
2-trial avg fitness ≈ 0.028
transfer=0.848, carrier_e=0.998, xi=0.985 (stable)
magic_R=0.877, query_gravity=0.374
```

Remaining fitness breakdown:
- transfer: 0.15 × (1 − 0.848) = **0.023** (83% of total)
- xi: 0.15 × (1 − 0.985) = 0.002 (7%)
- carrier_e: 0.10 × (1 − 0.998) = 0.0002 (1%)
- other: ~0.003 (9%)

Transfer is now the overwhelmingly dominant lever (83% of fitness). Xi and carrier_e
are essentially solved.

---

## Decision

**Code change RETAINED.** Fitness improvement 0.008 > 0.005 threshold. Fully deterministic.

---

## Open axes

| axis | expected gain | mechanism |
|------|---------------|-----------|
| Transfer from 0.848 → 0.929 | −0.012 fitness | Find B-primed cluster seeding that helps transfer WITHOUT hurting xi/carrier_e. The BFS-sorted clusters helped transfer but hurt xi/carrier_e — need a B-primed-only sort. |
| Transfer from 0.848 → 1.0 | −0.023 fitness (theoretical max) | What drives chain_fidelity? May need separate investigation. |
| query_gravity from 0.374 → >0.5 | minor | Instrumentation only; not in fitness. |
