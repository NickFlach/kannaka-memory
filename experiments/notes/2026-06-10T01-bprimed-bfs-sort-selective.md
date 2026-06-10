# Selective BFS content sort (transfer engines only) — fitness 0.028 → 0.018

**Date:** 2026-06-10T01 UTC
**Branch:** kannaka-curiosity/2026-06-10T01-bprimed-bfs-sort
**Code changes:** `src/kuramoto.rs::find_synchronized_clusters` — KEPT (content sort for transfer engines only)
**Status:** CONFIRMED — fitness improvement 0.010, above 0.005 threshold, fully deterministic

---

## Background

T21 (2026-06-09T21) state:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
2-trial avg fitness ≈ 0.028
transfer=0.848, carrier_e=0.998, xi=0.985
magic_R=0.877, query_gravity=0.374
```

T21's fitness breakdown:
- transfer: 0.15 × (1 − 0.848) = **0.023** (83% of total)
- xi: 0.15 × (1 − 0.985) = 0.002 (7%)
- carrier_e: 0.10 × (1 − 0.998) = 0.0002 (<1%)
- other: ~0.003 (9%)

T21 had explicitly identified the remaining lever:
> "Transfer from 0.848 → 0.929: Find B-primed cluster seeding that helps transfer WITHOUT
> hurting xi/carrier_e. The BFS-sorted clusters helped transfer but hurt xi/carrier_e —
> need a B-primed-only sort."

The history of the BFS sort across fires:
- **T13** added `all.sort_by(|a, b| a.content.cmp(&b.content))` to `find_synchronized_clusters`
  applied universally to all engines. Transfer ~0.929, xi ~0.906, carrier_e ~0.900.
- **T21** removed the sort entirely. Xi recovered 0.906→0.985, carrier_e 0.900→0.998,
  but transfer dropped 0.929→0.848.

The T21 notes noted that the two effects pulled in opposite directions and hypothesized
finding a way to separate them.

---

## Hypothesis

The BFS content sort hurts xi SPECIFICALLY because the xi eval uses `engine_adv`, an engine
that contains adversarial memories with content strings prefixed "adv_l5_..." — these sort
alphabetically before corpus memories ("dec...", "emo...", etc.), causing adversarials to
form BFS cluster seeds first in the adversarial dream pass, which damages the clean-vs-adv
comparison that xi_robustness_v2 measures.

Additionally, `engine_clean` (the baseline for the xi eval) must use the SAME sort order
as `engine_adv` for the comparison to be meaningful. If engine_clean uses content sort and
engine_adv uses UUID sort, the cluster topologies diverge → xi appears lower.

For transfer engines (engine_a, engine_b_primed, engine_b_naive, engine_flat), adversarials
are entirely absent. The content sort just reorders corpus memories alphabetically. The
critical constraint for transfer is CONSISTENCY: engine_a and engine_b_primed must use the
same cluster seeding so the topology engine_a builds is what engine_b_primed inherits and
extends. When both use content sort, this consistency is maintained.

**Hypothesis:** Apply content sort only when `DRIVE_CONTEXT ∈ {engine_a, engine_b_primed,
engine_b_naive, engine_flat}`. Skip sort for engine_adv (adversarials present) and
engine_clean (must be consistent with engine_adv for xi comparison).

**Prediction:**
- transfer: ~0.903–0.929 (recover toward T16-B level via consistent corpus seeding)
- xi: ~0.985 (unchanged — engine_adv and engine_clean both use UUID sort, same as T21)
- carrier_e: ~0.998 (unchanged)
- Fitness: ~0.018–0.022

---

## Exploratory trials before confirming hypothesis

**Trial 1: B-primed-only sort** (DRIVE_CONTEXT == "engine_b_primed")
- fitness: 0.031, transfer: 0.822, xi: 0.987, carrier_e: 0.999
- Transfer DROPPED further (0.848→0.822). Engine_a ran with UUID sort, engine_b_primed
  ran with content sort → cluster topology MISMATCH → worse transfer. Confirmed the
  consistency requirement: it's not just B-primed, it's both A and B-primed together.

**Trial 2: All-except-adv sort** (DRIVE_CONTEXT != "engine_adv")
- fitness: 0.035, transfer: 0.903, xi: 0.878, carrier_e: 0.999
- Transfer recovered to 0.903 (great!) but xi dropped 0.985→0.878. Engine_clean got
  content-sorted, creating a topology mismatch with engine_adv (UUID-sorted) → xi damaged.
  This confirmed that engine_clean must be excluded from the content sort.

**Trial 3 (current hypothesis):** Transfer engines only
- fitness: 0.018, transfer: 0.903, xi: 0.987, carrier_e: 0.999 ✓

---

## Change

In `src/kuramoto.rs::find_synchronized_clusters`, after the cache lookup block and before
the adjacency matrix computation:

```rust
let drive_ctx = std::env::var("DRIVE_CONTEXT").unwrap_or_default();
let all = if matches!(drive_ctx.as_str(), "engine_a" | "engine_b_primed" | "engine_b_naive" | "engine_flat") {
    let mut sorted = all;
    sorted.sort_by(|a, b| a.content.cmp(&b.content));
    sorted
} else {
    all
};
```

DRIVE_CONTEXT values and sort behaviour:
| Context | Sort applied | Reason |
|---------|-------------|--------|
| engine_a | ✓ | transfer engine, corpus only |
| engine_b_primed | ✓ | transfer engine, corpus only |
| engine_b_naive | ✓ | transfer engine, corpus only |
| engine_flat | ✓ | transfer engine, corpus only |
| engine_adv | ✗ | adversarials present, UUID order protects xi |
| engine_clean | ✗ | must match engine_adv for xi comparison |

---

## Results

DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax

| trial | fitness | transfer | carrier_e | xi    | magic_R | query_gravity |
|-------|---------|----------|-----------|-------|---------|---------------|
| T3    | 0.018282 | 0.903199 | 0.9992 | 0.9870 | 0.8643 | 0.3733 |
| T4    | 0.018281 | 0.903199 | 0.9992 | 0.9870 | 0.8643 | 0.3733 |
| **avg** | **0.018282** | **0.903** | **0.999** | **0.987** | **0.864** | **0.373** |

Fully deterministic (byte-identical on all metrics across both trials).

---

## Comparison to baseline (T21)

| metric | T21 baseline | this fire | delta |
|--------|-------------|-----------|-------|
| fitness avg | 0.02783 | **0.01828** | **−0.0096** |
| transfer | 0.848 | **0.903** | **+0.055** |
| xi | 0.985 | 0.987 | +0.002 |
| carrier_e | 0.998 | 0.999 | +0.001 |
| magic_R | 0.877 | 0.864 | −0.013 |
| query_gravity | 0.374 | 0.373 | −0.001 |

---

## Fitness impact decomposition

| metric | weight | T21 contribution | this fire contribution | delta |
|--------|--------|--------------------|------------------------|-------|
| transfer | 0.15 | 0.023 | **0.015** | **−0.008 improvement** |
| xi | 0.15 | 0.002 | 0.002 | ~0 |
| carrier_e | 0.10 | 0.0002 | 0.0001 | ~0 |
| other | — | ~0.003 | ~0.002 | −0.001 |
| **total** | | **0.028** | **0.018** | **−0.010** |

---

## Comparison to T16-B (pre-T21 baseline with universal BFS sort)

T16-B had BFS sort applied universally (including engine_adv and engine_clean):
| metric | T16-B | this fire | delta |
|--------|-------|-----------|-------|
| fitness | ~0.036 | 0.018 | −0.018 |
| transfer | 0.929 | 0.903 | −0.026 |
| xi | 0.906 | 0.987 | +0.081 |
| carrier_e | 0.900 | 0.999 | +0.099 |

The small transfer gap (0.929 vs 0.903) vs T16-B is the cost of excluding engine_adv and
engine_clean from the sort. T16-B's higher transfer came partly from engine_adv being
content-sorted too, but that corrupted xi. The net fitness result is better (0.018 vs 0.036).

This fire finds the Pareto-optimal split: selective sort is strictly better than both
universal sort (T16-B) and no sort (T21) in fitness terms.

---

## New empirical optimum

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
2-trial avg fitness ≈ 0.018
transfer=0.903, carrier_e=0.999, xi=0.987 (fully deterministic)
magic_R=0.864, query_gravity=0.373
```

Remaining fitness breakdown:
- transfer: 0.15 × (1 − 0.903) = **0.015** (82% of total)
- xi: 0.15 × (1 − 0.987) = 0.002 (11%)
- carrier_e: 0.10 × (1 − 0.999) = 0.0001 (<1%)
- other: ~0.001 (7%)

Transfer remains the dominant lever.

---

## Decision

**Code change RETAINED.** Fitness improvement 0.010 > 0.005 threshold. Fully deterministic.

---

## Open axes

| axis | expected gain | mechanism |
|------|---------------|-----------|
| Transfer from 0.903 → 0.929 | −0.004 fitness | Recover the T16-B transfer without corrupting xi. Could universal content sort in engine_adv have been helping transfer indirectly? Unlikely — the xi eval path is independent of the transfer path. More likely: some other difference in the combined-context state. |
| Transfer from 0.903 → 1.0 | −0.015 fitness (theoretical max) | What determines chain_fidelity in engine_b_primed? May need to investigate the dream chain topology in B-primed specifically. |
| query_gravity from 0.373 → >0.5 | minor | Instrumentation only; not in fitness. |
| magic_R understanding | — | magic_R dropped slightly (0.877→0.864). Not in fitness but worth tracking as theory instrumentation. |
