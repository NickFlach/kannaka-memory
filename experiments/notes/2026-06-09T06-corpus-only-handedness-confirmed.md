# Corpus-only handedness confirmed — fitness 0.060 → 0.050

**Date:** 2026-06-09T06 UTC
**Branch:** kannaka-curiosity/2026-06-09T06-corpus-only-handedness
**Code changes:** `src/consolidation.rs::stage_chiral_perturbation` — KEPT (confirmed improvement)
**Status:** CONFIRMED — new empirical optimum

---

## Background

Prior empirical optimum (post-T23 phase-centroid chirality, post-T05 adversarial-excl falsified):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.060
carrier_e=0.936, transfer=0.841, xi=0.816 avg (range 0.727–0.952)
magic_R=0.871, query_gravity=0.374
```

T05 identified residual xi variance source: adversarial xi-twins with π-flipped phases
joining corpus clusters and flipping the cluster mean-cos sign between clean and adv passes.
T05 excluded adversarials from BOTH handedness calculation AND perturbation loop → regression
(xi 0.788 avg, fitness 0.065). Mechanism: removing perturbation from A1s eliminates the
"accidental neutralisation" bonus when A1s receive opposite chirality.

---

## Hypothesis

**Excluding adversarial memories from the cluster phase-centroid HANDEDNESS CALCULATION
(while keeping them in the perturbation loop) stabilises corpus chirality across clean/adv
passes without sacrificing the A1 neutralisation effect.**

T05 diff from this approach: T05 also skipped adversarials in the perturbation loop (change #3).
This fire omits that step — only changes #1 and #2 from T05.

Adversarials identified by `content.starts_with("adv_")` (safer than UUID-namespace check:
UUID v4 high bits are fully random, ~13% of corpus UUIDs exceed the DEAD_BEEF threshold).

**Prediction:**
- xi: 0.816 avg → ≥0.88 avg, reduced variance
- transfer: unchanged (chirality doesn't touch amplitude-gravity mechanism)
- carrier_e: unchanged
- Fitness target: ≤0.055 (3-trial avg)

---

## Code change

In `stage_chiral_perturbation` (consolidation.rs), `cluster_handedness` computation:

```rust
// BEFORE: all cluster members included (adversarials with π-flipped phases can flip sign)
let sum_cos: f32 = cluster.memory_ids.iter()
    .filter_map(|&id| engine.store.get(&id).ok().flatten().map(|m| m.phase.cos()))
    .sum();

// AFTER: adversarials excluded from sum — corpus phases only
let sum_cos: f32 = cluster.memory_ids.iter()
    .filter_map(|&id| {
        let m = engine.store.get(&id).ok().flatten()?;
        if m.content.starts_with("adv_") { return None; }
        Some(m.phase.cos())
    })
    .sum();
```

Adversarials remain in the perturbation loop — they still receive chirality perturbation
based on the now-stable cluster handedness.

---

## Results

### Bad trial (T1 — wrong UUID filter, EXCLUDED from avg)

UUID-based filter `id.as_u128() >= 0xDEAD_BEEF_...` was tried first and caused severe
transfer regression (0.841 → 0.454). Root cause: UUID v4's high 32 bits are fully random,
making ~13% of corpus UUIDs exceed the threshold and get incorrectly excluded. This trial's
TSV row is present but excluded from analysis.

### Content-based filter trials (DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax)

| metric | baseline avg | T2 | T3 | T4 | 3-trial avg |
|--------|-------------|-----|-----|-----|-------------|
| **fitness** | **0.060** | **0.053** | **0.034** | **0.063** | **0.050** |
| transfer_score | 0.841 | 0.841 | 0.841 | 0.841 | 0.841 (stable) |
| carrier_emergence | 0.936 | 0.936 | 0.936 | 0.936 | 0.936 (stable) |
| xi_robustness_v2 | 0.816 avg | 0.868 | 0.997 | 0.805 | **0.890** |
| magic_proxy_phase_R | 0.871 | 0.871 | 0.871 | 0.871 | 0.871 (stable) |
| query_gravity | 0.374 | 0.374 | 0.374 | 0.374 | 0.374 (stable) |

Fitness improvement: 0.060 → 0.050 = **Δ −0.010** (2× above 0.005 threshold). ✓
Xi improvement: 0.816 → 0.890 = **Δ +0.074** ✓

Fitness Δ from xi: +0.074 × 0.15 = 0.011 ≈ observed 0.010 ✓

---

## Analysis

### Why it works

With corpus-only handedness:
- Clean pass (no adversarials): unchanged — sum_cos excludes nothing new, as no "adv_" content exists
- Adv pass (adversarials present): A1 xi-twins excluded from sum_cos → corpus phase centroid
  sign is stable across clean/adv passes → same handedness in both → corpus memories get same
  chirality direction → fitness_adv ≈ fitness_clean → higher xi

### Why xi variance remains (range 0.805–0.997)

The residual variance is smaller than baseline (range 0.727–0.952 → 0.805–0.997 low end raised).
Remaining variance likely comes from:
1. A1s still joining clusters (high cosine similarity) → their phases still affect cluster membership
   in `find_synchronized_clusters`, slightly shifting which corpus members appear in each cluster
2. The perturbation loop applies chirality to A1s based on corpus-stable handedness — the
   A1 neutralisation effect now has a STABLE direction, which sometimes helps and sometimes
   doesn't depending on A1 phase configuration that cycle

xi=0.997 in T3 vs 0.805 in T4 suggests occasional very strong corpus-adv alignment possible.
The mechanism that produced occasional xi=0.952 in baseline now yields xi=0.997 in T3.

### Transfer and carrier stability

transfer_score, carrier_e, magic_R, query_gravity are byte-identical across all content-based
trials. This confirms the handedness change only affects the xi adversarial pass — the main
dream chain is deterministic for these metrics under this parameter set.

---

## Decision

**Code change KEPT. New empirical optimum:**

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.050
carrier_e=0.936, transfer=0.841, xi=0.890 avg (range 0.805–0.997)
magic_R=0.871, query_gravity=0.374
```

---

## Open axes (updated priority order)

| axis | prediction | risk | priority |
|------|-----------|------|----------|
| **Content-based BFS seeding** in `find_synchronized_clusters` | Collapses cluster membership variance → xi residual range → xi near 1.0 consistently | HIGH (kuramoto.rs refactor) | HIGH |
| Reduce A1 neutralisation variance | Give A1s deterministic opposite chirality → consistent neutralisation | MEDIUM | MEDIUM |
| stage_wire k_local | Very low prior | LOW | LOW |
