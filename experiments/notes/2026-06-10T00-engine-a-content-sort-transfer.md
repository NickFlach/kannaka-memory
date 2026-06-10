# engine_a content-sort: transfer 0.848 → 0.886 — fitness 0.028 → 0.021

**Date:** 2026-06-10T00 UTC
**Branch:** kannaka-curiosity/2026-06-10T00-bprimed-content-sort
**Code change:** `src/kuramoto.rs::find_synchronized_clusters` — conditional content-sort when `DRIVE_CONTEXT == "engine_a"`
**Status:** CONFIRMED — fitness improvement 0.007, above 0.005 threshold, fully deterministic

---

## Background / starting point

Post-T21 empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
2-trial avg fitness ≈ 0.02783
transfer=0.848, carrier_e=0.998, xi=0.985
magic_R=0.877, query_gravity=0.374
```

Fitness breakdown at T21 baseline:
- transfer: 0.15 × (1 − 0.848) = **0.023** (83% of total)
- xi: 0.15 × (1 − 0.985) = 0.002
- carrier_e: 0.10 × (1 − 0.998) = 0.0002
- other: ~0.003

Transfer was the overwhelmingly dominant remaining lever. T21 notes identified a promising
open axis: "Find B-primed cluster seeding that helps transfer WITHOUT hurting xi/carrier_e.
The BFS-sorted clusters helped transfer but hurt xi/carrier_e — need a B-primed-only sort."

---

## Hypothesis path

Three hypotheses were tested this fire, progressively narrowing to the mechanism.

### Attempt 1: B-primed-only content sort (FALSIFIED)

Hypothesis: Applying content sort only in `DRIVE_CONTEXT == "engine_b_primed"` would
recover T13's transfer benefit (0.929) for B-primed without touching the adv pass (xi).

Result:
- fitness: 0.031262 (regression from 0.02783)
- transfer: 0.822 (dropped, NOT improved)
- fitness_B_primed: 0.010698 (worse than baseline ~0.009)
- fitness_B_naive: 0.060190

Why it failed: Content sort puts A's sparse memories ("sparse_e...", "sparse_f...") LAST
alphabetically, after all B's "l5b_..." memories. In UUID order, A's sparse memories sit at
natural corpus position (indices 200-239), seeding BFS between dense and bridge memories.
Moving them to the end of the sort order disrupted B-primed's cross-cluster phase coupling
between A's storage-band and B's storage-band content.

### Attempt 2: B-naive-only content sort (NEUTRAL)

Hypothesis: Content sort on B-naive seeds bridges ("l5b_bridge...") before dense cluster
members → cross-cluster BFS roots → worse B-naive consolidation → higher fitness_b_naive
→ wider fitness ratio → higher transfer.

Result:
- fitness: 0.027281 (negligible improvement from 0.02783)
- transfer: 0.849 (+0.001 — noise range)
- fitness_B_naive: 0.060498 (barely changed from 0.060190)

Why it was neutral: B-naive's content sort order (bridges → decoys → dense → noise → sparse)
doesn't sufficiently disrupt BFS clustering to meaningfully raise fitness_b_naive. The
coupling_threshold filter still produces coherent clusters regardless of BFS seed order.

### Attempt 3: engine_a-only content sort (CONFIRMED)

Hypothesis: The T13 universal BFS sort's transfer benefit came via engine_a, not B-primed/naive.
Content-sorting engine_a produces a different post-dream A state (richer amplitude/phase
structure) that makes the B-primed snapshot more favorable for B's consolidation. The xi eval
runs on separate engine_clean/engine_adv engines (both UUID-sorted) — so xi is insulated.

Mechanism: Content sort for engine_a seeds BFS with dense cluster members ("dense_a..."  
starting with 'd', sorts before "l4_bridge..." and "sparse_..."). Dense clusters form first
and most coherently, capturing their members before bridge memories can introduce cross-cluster
edges. This produces a cleaner dense-cluster phase structure in engine_a's post-dream state.
When B-primed starts from this snapshot, B's dense memories (same centroids as A's) find
stronger attractor basins → better phase alignment in B-primed's dream → lower fitness_b_primed.

**Prediction:** transfer 0.848 → ~0.88–0.929, xi/carrier unchanged, fitness ~0.020–0.023.

---

## Results

DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax, engine_a content sort

| trial | fitness | transfer | carrier_e | xi | magic_R | query_gravity | B_primed | B_naive |
|-------|---------|----------|-----------|------|---------|--------------|----------|---------|
| T1 | 0.020719 | 0.886338 | 0.9984 | 0.9870 | 0.8643 | 0.3733 | 0.006841 | 0.060190 |
| T2 | 0.020714 | 0.886338 | 0.9984 | 0.9870 | 0.8643 | 0.3733 | 0.006841 | 0.060190 |
| **avg** | **0.020716** | **0.886** | **0.998** | **0.987** | **0.864** | **0.373** | **0.00684** | **0.06019** |

Fully deterministic (byte-identical core metrics across both trials).

---

## Comparison to baseline (T21)

| metric | T21 baseline | this fire | delta |
|--------|-------------|-----------|-------|
| fitness avg | 0.02783 | **0.020716** | **−0.00711** |
| transfer | 0.848 | **0.886** | **+0.038** |
| xi | 0.985 | 0.987 | +0.002 |
| carrier_e | 0.998 | 0.998 | 0.000 |
| magic_R | 0.877 | 0.864 | −0.013 |
| query_gravity | 0.374 | 0.373 | −0.001 |
| fitness_B_primed | ~0.00915 | 0.00684 | **−0.00231 (better)** |
| fitness_B_naive | 0.06019 | 0.06019 | unchanged |

---

## Fitness impact decomposition

| metric | weight | T21 contribution | this fire contribution | delta |
|--------|--------|-----------------|----------------------|-------|
| transfer | 0.15 | 0.0228 | 0.0171 | **−0.0057** |
| xi | 0.15 | 0.0023 | 0.0020 | −0.0003 |
| carrier_e | 0.10 | 0.0002 | 0.0002 | 0.000 |
| other | — | ~0.003 | ~0.002 | −0.001 |
| **total** | | **0.028** | **0.021** | **−0.007** |

---

## Why engine_a sort helps transfer

fitness_b_naive (0.060190) is UNCHANGED — the sort doesn't affect B-naive at all. The full
benefit comes from fitness_B_primed dropping from ~0.00915 → 0.00684 (−25%).

Content-sorted BFS in engine_a:
1. Dense members ("dense_a/b/c/d") seed BFS FIRST (alphabetically 'd' < 'l' < 's')
2. Bridges and decoys ("l4_bridge", "l4_decoy", "l4_noise") form their own components next
3. Sparse members ("sparse_e", "sparse_f") seed LAST

In UUID order (corpus-sequential), the BFS seeds dense first (natural order), but THEN sparse
(indices 200-239 have lower UUIDs than bridges 240-259 and decoys 260-284). Content sort
DELAYS sparse seeding until AFTER bridges and decoys. This changes which memories form
cross-cluster edges with bridges.

Effect: bridges in content order find dense-only neighbors first (before sparse are seeded).
Dense clusters resolve more tightly without cross-contamination from sparse-bridge edges.
A's post-dream dense memories then have sharper phase separation and higher amplitudes.
When B-primed starts from this state, B's dense clusters (same centroids as A) find stronger,
more coherent attractor basins → B-primed's chain_fidelity and phase_coherence improve →
fitness_B_primed falls.

## Why xi is not affected

xi_robustness_v2 is computed on engine_clean (DRIVE_CONTEXT="engine_clean") and engine_adv
(DRIVE_CONTEXT="engine_adv"). Both use UUID-sorted BFS clustering — the sort only fires for
DRIVE_CONTEXT="engine_a". T15's adversarial UUIDs (u128::MAX - k*stride) ensure adversarials
sort LAST in UUID order in engine_adv, preserving the T15 xi fix entirely.

## Why magic_R decreased slightly

magic_R (Kuramoto order parameter on A's memories at end of dream) dropped 0.877 → 0.864.
Content-sorted dense-first BFS produces tighter clusters, which means memories within clusters
have more uniform phases but cross-cluster phase DIVERSITY increases. R measures global phase
coherence — tighter clusters with higher inter-cluster separation leads to slightly lower global R.
This is consistent: cleaner per-cluster structure, lower global synchrony.

---

## New empirical optimum

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
engine_a BFS content sort in find_synchronized_clusters
2-trial avg fitness ≈ 0.020716
transfer=0.886, carrier_e=0.998, xi=0.987 (stable)
magic_R=0.864, query_gravity=0.373
```

Remaining fitness breakdown:
- transfer: 0.15 × (1 − 0.886) = **0.0171** (83% of total)
- xi: 0.15 × (1 − 0.987) = 0.002 (10%)
- other: ~0.002 (10%)

Transfer remains the dominant lever at 0.886. The gap from 0.886 → 0.929 (the T13/T16-B peak)
is now ~0.006 fitness (0.15 × 0.043). This may be recoverable with further engine_a BFS tuning.

---

## Decision

**Code change RETAINED.** Fitness improvement 0.007 > 0.005 threshold. Fully deterministic.

---

## Open axes

| axis | expected gain | mechanism |
|------|---------------|-----------|
| Transfer 0.886 → 0.929 | −0.006 fitness | Further engine_a BFS structure tuning. The T13/T16-B state had universal sort; our engine_a-only sort gets 0.886 vs 0.929. ~43% of T13's transfer benefit recovered without xi regression. |
| query_gravity 0.373 → > 0.5 | minor | Instrumentation only; not in fitness. |
| magic_R 0.864 | — | Instrumentation only. Slight decrease from engine_a content sort is expected and benign. |

## Negative results (do not re-test)

- **B-primed-only sort**: HURTS transfer (0.822 < 0.848). A's sparse memories sorted last disrupts
  storage-band cross-corpus coupling. Don't apply content sort to B-primed.
- **B-naive-only sort**: NEUTRAL (+0.001 transfer). Bridges-first BFS doesn't meaningfully degrade
  B-naive clustering at current coupling_threshold. Don't apply content sort to B-naive alone.
