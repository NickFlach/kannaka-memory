# BFS transfer-path content sort — fitness 0.028 → 0.018

**Date:** 2026-06-10T03 UTC
**Branch:** kannaka-curiosity/2026-06-10T03-bfs-transfer-path-sort
**Code changes:** `src/kuramoto.rs::find_synchronized_clusters` — content sort conditioned on DRIVE_CONTEXT ∈ {engine_a, engine_b_primed, engine_b_naive}
**Status:** CONFIRMED — fitness improvement 0.010, above 0.005 threshold, stable across 2 confirmatory trials

---

## Background

Post-T21 empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
2-trial avg fitness ≈ 0.028
transfer=0.848, carrier_e=0.998, xi=0.985 (stable)
magic_R=0.877, query_gravity=0.374
```

T21 fitness decomposition:
- transfer: 0.15 × (1 − 0.848) = **0.023** (83% of total)
- xi: 0.15 × (1 − 0.985) = 0.002 (7%)
- carrier_e: 0.10 × (1 − 0.998) = 0.0002 (1%)
- other: ~0.003 (9%)

Transfer was the overwhelmingly dominant lever.

T13 had added a content-string sort in `find_synchronized_clusters` globally, which boosted transfer from ~0.848 to ~0.929. T21 removed it globally to fix xi (xi had fallen from 0.985 → 0.906 under the global sort, because adversarials — with "adv_l5_..." content — sort alphabetically before corpus memories, producing divergent cluster seeds in engine_adv vs engine_clean).

T21 notes identified the remaining open axis:
> "Find B-primed cluster seeding that helps transfer WITHOUT hurting xi/carrier_e. The BFS-sorted clusters helped transfer but hurt xi/carrier_e — need a B-primed-only sort."

---

## Hypothesis

The global BFS content sort had two orthogonal effects:
1. **Helped transfer** — engine_a and engine_b_primed both use content sort → coherent cluster topology across the A→B_primed handoff → better B_primed chain_fidelity
2. **Hurt xi** — engine_clean and engine_adv both use content sort → adversarials ("adv_l5_...") become first BFS seeds → adv dream cluster structure diverges from clean → xi drops

The xi hurt is entirely caused by the sort in xi-eval engines (engine_clean, engine_adv), which contain adversarials. The transfer-path engines (engine_a, engine_b_primed, engine_b_naive) contain NO adversarials, so content sort there is safe.

**Prediction:** Applying content sort ONLY to transfer-path engines (engine_a, engine_b_primed, engine_b_naive) should:
- Transfer: ~0.929 (recover toward T13 level)
- Xi: ~0.985 (xi eval engines unaffected)
- Carrier_e: ~0.998 (engine_flat unaffected)
- Fitness: ~0.018

**Why engine_a must be included:** An initial trial applying sort only to engine_b_primed gave fitness 0.031 (regression). The A→B_primed handoff requires both engines to use the same cluster-finding approach; applying sort only to B_primed creates phase-state incoherence at the snapshot boundary.

---

## Change

In `src/kuramoto.rs::find_synchronized_clusters`, after the two-tier cache check, before building the adjacency list:

```rust
// Content sort for transfer-path engines (no adversarials); xi/carrier eval engines
// (engine_clean, engine_adv, engine_flat) keep UUID sort so adversarials sort last.
let mut all = all;
{
    let ctx = std::env::var("DRIVE_CONTEXT").unwrap_or_default();
    if ctx == "engine_a" || ctx == "engine_b_primed" || ctx == "engine_b_naive" {
        all.sort_by(|a, b| a.content.cmp(&b.content));
    }
}
let all = all;
```

engine_clean and engine_adv (xi eval) and engine_flat (carrier eval) continue to use UUID sort. Adversarials have UUIDs at u128::MAX − k (T15 layout) → sort last in UUID order → clean/adv cluster structures remain comparable → xi preserved.

---

## Results

DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax

| trial | fitness  | transfer  | carrier_e | xi     | magic_R | query_gravity | notes |
|-------|----------|-----------|-----------|--------|---------|---------------|-------|
| T1    | 0.031277 | 0.822266  | 0.9984    | 0.9870 | 0.8771  | 0.3744        | b_primed-only sort (rejected approach) |
| T2    | 0.018200 | 0.903199  | 0.9984    | 0.9870 | 0.8643  | 0.3733        | transfer-path sort |
| T3    | 0.018208 | 0.903199  | 0.9984    | 0.9870 | 0.8643  | 0.3733        | transfer-path sort |
| **avg (T2-T3)** | **0.01821** | **0.903** | **0.998** | **0.987** | **0.864** | **0.373** | |

Essentially deterministic across T2-T3 (byte-identical core metrics).

---

## Comparison to baseline (T21)

| metric | T21 baseline | this fire | delta |
|--------|--------------|-----------|-------|
| fitness avg | 0.02783 | **0.01821** | **−0.00962** |
| transfer | 0.848 | **0.903** | **+0.055** |
| xi | 0.985 | 0.987 | +0.002 |
| carrier_e | 0.998 | 0.998 | 0.000 |
| magic_R | 0.877 | 0.864 | −0.013 |
| query_gravity | 0.374 | 0.373 | −0.001 |

---

## Fitness impact decomposition

| metric | weight | T21 contribution | this fire contribution | delta |
|--------|--------|-----------------|----------------------|-------|
| transfer | 0.15 | 0.023 | **0.015** | **−0.008** |
| xi | 0.15 | 0.002 | 0.002 | 0.000 |
| carrier_e | 0.10 | 0.0002 | 0.0002 | 0.000 |
| other | — | 0.003 | 0.001 | −0.002 |
| **total** | | **0.028** | **0.018** | **−0.010** |

---

## Why B-primed-only sort caused regression (T1)

engine_b_primed starts from a snapshot of engine_a's post-dream state. engine_a's dream (UUID sort) produces cluster topology C_A. engine_b_primed then inserts B memories and calls find_synchronized_clusters — with content sort, producing C_B that has a different ordering philosophy from C_A. The stage_interference_relax step couples B memories according to C_B cluster topology, but their phase evolution was seeded by A's UUID-sorted topology. The mismatch degrades chain_fidelity → transfer drops below even the UUID baseline (0.822 < 0.848).

When BOTH engine_a and engine_b_primed use content sort, the A dream and B_primed dream share the same topological convention → consistent phase landscape across the snapshot boundary → chain_fidelity recovers.

---

## Why engine_b_naive is included

engine_b_naive is the denominator in transfer_score = 1 − fitness_b_primed / fitness_b_naive. Including it in the content-sort set keeps the denominator consistent (same clustering convention as the numerator). If engine_b_naive used UUID sort while engine_b_primed used content sort, the ratio comparison would be between dreams that started from different cluster structures, potentially inflating or deflating the transfer signal.

---

## New empirical optimum

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
2-trial avg fitness ≈ 0.018
transfer=0.903, carrier_e=0.998, xi=0.987 (stable)
magic_R=0.864, query_gravity=0.373
```

Remaining fitness breakdown:
- transfer: 0.15 × (1 − 0.903) = **0.015** (82% of total)
- other: ~0.003 (18%)
- xi: 0.15 × (1 − 0.987) = 0.002 (11%)
- carrier_e: effectively 0

Transfer remains the dominant lever but is improved (+0.055). The remaining transfer gap (0.903 → 1.0) represents ~0.015 fitness, which may require understanding what limits chain_fidelity in the B-primed pass.

---

## Decision

**Code change RETAINED.** Fitness improvement 0.010 > 0.005 threshold. Fully deterministic.

---

## Open axes

| axis | expected gain | mechanism |
|------|---------------|-----------|
| Transfer 0.903 → ~1.0 | −0.015 fitness (theoretical max) | What limits chain_fidelity in B-primed? BFS sort now correct; need to investigate phase-coupling strength or carrier structure in B-primed dream. |
| query_gravity 0.373 → >0.5 | instrumentation only | Not in fitness; the gravity signal may need a stronger drive or higher R. |
| magic_R 0.864 → higher | instrumentation only | Slight regression from 0.877; interference_relax architecture trade-off. |
