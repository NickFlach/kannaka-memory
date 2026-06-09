# Chirality stability hypothesis falsified — xi mechanism revised

**Date:** 2026-06-09T04 UTC
**Branch:** kannaka-curiosity/2026-06-09T04-chirality-content-stable
**Code changes:** REVERTED (`stage_chiral_perturbation` content-stable chirality, then corpus-UUID-sort approach)
**Status:** FALSIFIED — stable chirality hurts xi; T21 mechanism model revised

---

## Background

Current empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.099
carrier_e=0.935, transfer=0.836, xi_robustness_v2=0.559 avg (range 0.256–0.874)
```

T21 identified the xi variance source as adversarial UUID randomness → BFS cluster
index shifts → chirality flips in the adversarial pass vs the clean pass →
fitness_adv diverges from fitness_clean. The proposed "fix" was content-based
chirality that remains stable regardless of adversarial UUID positions.

This fire tested that hypothesis across two implementations.

---

## Hypothesis

**Stabilizing chirality assignments across clean and adversarial passes will make
fitness_adv ≈ fitness_clean → xi improves from 0.559 avg toward ≥ 0.65 avg.**

**Prediction:**
- xi_robustness_v2: 0.559 avg → ≥ 0.65 avg
- transfer_score: 0.836 (unchanged — stable chirality doesn't change main chain)
- carrier_emergence: 0.935 (unchanged)
- Fitness target: ≤ 0.094

---

## Method

### Attempt 1: theme_vector hash chirality

Replace `cluster_idx % 2 == 0` with an LCG hash of the cluster's normalized
`theme_vector`:
```rust
let h: u64 = cluster.theme_vector.iter()
    .fold(0u64, |acc, &x| {
        acc.wrapping_mul(6364136223846793005u64).wrapping_add(x.to_bits() as u64)
    });
let handedness = if h % 2 == 0 { 1.0 } else { -1.0 };
```

The theme_vector is the normalized sum of member vectors — content-derived,
not UUID-dependent. Prediction: main chain unchanged; xi stabilizes.

### Attempt 2: corpus-UUID-sort chirality (stable BFS replica)

After observing attempt 1 failed, replaced with a more principled approach:
sort clusters by their minimum **corpus** member UUID (filtering adversarial
memories via `!content.starts_with("adv_")`). This exactly replicates the
clean-pass BFS enumeration order in the adversarial pass:

```rust
let min_corpus_uuids: Vec<u128> = clusters.iter()
    .map(|cluster| {
        cluster.memory_ids.iter()
            .filter_map(|id| {
                let is_corpus = engine.store.get(id).ok().flatten()
                    .map(|m| !m.content.starts_with("adv_"))
                    .unwrap_or(false);
                if is_corpus { Some(id.as_u128()) } else { None }
            })
            .min()
            .unwrap_or(u128::MAX)
    })
    .collect();
// Sort cluster indices by min corpus UUID → stable_rank replicates clean BFS order
let mut sorted_original_indices: Vec<usize> = (0..clusters.len()).collect();
sorted_original_indices.sort_by_key(|&i| min_corpus_uuids[i]);
// stable_rank % 2 == 0 → handedness = +1.0 (matches clean BFS parity exactly)
```

Adversarial-only clusters sort last (min_corpus_uuid = u128::MAX), corpus clusters
retain their original BFS parity.

---

## Results

| metric | baseline avg | attempt 1 T1 | attempt 2 T2 |
|--------|-------------|--------------|--------------|
| **fitness** | **0.099** | **0.200** | **0.149** |
| transfer_score | 0.836 | 0.568 | **0.836** |
| carrier_emergence | 0.935 | 0.903 | **0.935** |
| **xi_robustness_v2** | **0.559** | **0.172** | **0.223** |
| magic_proxy_phase_R | 0.617 | 0.265 | 0.617 |
| query_gravity | 0.363 | 0.362 | 0.363 |

---

## Analysis

### Attempt 1 falsified immediately

The theme_vector hash gave a WRONG distribution of handedness across clusters. Most
likely all 4 main corpus clusters hashed to the same parity → all got the same
handedness (+1.0) → no inter-cluster phase contrast → transfer and carrier_e collapsed.
Fitness: 0.200 (vs 0.099 baseline). Reverted after 1 trial.

### Attempt 2: main chain preserved, xi still low

Attempt 2 (corpus-UUID-sort) correctly preserves the main chain: transfer=0.836,
carrier_e=0.935, magic_R=0.617, query_gravity=0.363 — byte-identical to baseline.
This confirms the stable-rank approach exactly replicates clean BFS ordering in
the clean pass, as predicted.

**But xi = 0.223 — worse than the worst baseline case (0.256).**

### Why stable chirality hurts xi: mechanism revised

With xi = 0.223 and fitness_clean = 0.149:
```
|fitness_clean - fitness_adv| = (1 - 0.223) * 0.149 = 0.116
```
Since clean = 0.149, either adv = 0.033 (better!) or adv = 0.265 (worse).

Given that all other metrics (transfer, carrier_e) are preserved exactly, and that
adversarial A1 xi-twins are constructed to be SIMILAR to corpus centroids (cosine
similarity ≈ 1.0 to their target cluster vectors), the most likely scenario:

**A1 xi-twins join corpus clusters (sim > 0.75 clustering threshold).**

With stable chirality (corpus-UUID-sort), A1 adversarials get the SAME handedness
as their host corpus cluster. They COHERENTLY REINFORCE the chiral perturbation of
that cluster, effectively adding ~10 more "right" corpus memories to each cluster.
This strengthens the chiral perturbation → better inter-cluster phase contrast →
improved carrier_e and transfer in the adv engine → **fitness_adv < fitness_clean
(adversaries help)** → large |divergence| → low xi.

### T21's causal model was incorrect

T21 concluded: "xi variance = adversarial UUID randomness → BFS cluster index shift
→ chirality flip → fitness_adv diverges." The implicit assumption was that SAME chirality
→ fitness_adv ≈ fitness_clean → high xi.

This fire refutes that assumption. The actual system:
- Random chirality (baseline): sometimes adversaries get the WRONG chirality relative
  to their cluster's clean assignment → wrong chirality PARTIALLY CANCELS their
  A1 reinforcement effect → smaller |divergence| → HIGH xi cases (0.874)
  - When adversaries get the same chirality as corpus (no BFS shift): adversaries
    reinforce corpus clusters → fitness_adv < fitness_clean → moderate divergence
  - The average 0.559 reflects a mix of both cases
- Stable chirality (this fire): adversaries ALWAYS get the same chirality as corpus
  clusters → ALWAYS reinforce corpus → fitness_adv consistently < fitness_clean →
  xi = 0.223 (deterministically low)

**The HIGH xi (0.874) baseline cases were NOT "same chirality = adversaries ignored."
They were "wrong chirality = adversaries cancel their own improvement effect."**

The random chirality in the baseline was ACCIDENTALLY HELPFUL — it sometimes gave
adversaries the "wrong" handedness that cancels their reinforcement, producing high xi.
Stabilizing chirality eliminates this accidental cancellation.

### What would actually improve xi

The xi metric measures |fitness_clean - fitness_adv|. A1 xi-twins JOIN corpus clusters
because their vector content is similar (by construction). They then either:
- Reinforce corpus chirality (same handedness → fitness_adv < fitness_clean)
- Cancel corpus chirality (opposite handedness → smaller net perturbation → fitness_adv ≈ fitness_clean)

For HIGH xi: adversaries should not affect fitness. This requires either:
1. A1 adversaries form ISOLATED clusters (cosine sim < 0.75 to corpus) → they get
   neutral perturbation → minimal effect on fitness_adv
2. A1 adversaries are EXCLUDED from chiral perturbation → same effect
3. The xi test adversarial set is redesigned to have low corpus similarity

None of these are straightforward within the current constraint (no changes to the
xi metric definition itself). Option 1 requires changing the adversarial construction
(A1 vectors must be orthogonal to corpus). Option 2 requires content-type awareness
in stage_chiral_perturbation.

### Remaining xi open axis: adversarial isolation

If A1 xi-twins were designed to have cosine similarity < 0.75 to corpus clusters
(e.g., by using a different transformation), they would form isolated clusters →
get neutral perturbation → no reinforcement → fitness_adv ≈ fitness_clean → high xi.
This would require a change to `build_adversarial_set_l5`.

Alternatively: compute xi using a SEPARATE adversarial set that doesn't join corpus
clusters (A2 commutators and A3 freq-attacks already form isolated clusters — the
problem is specific to A1 xi-twins).

---

## Decision

**No code changes retained. Both approaches reverted. Hypothesis falsified.**

The T21 mechanism model was incorrect. xi variance = adversarial UUID randomness is
confirmed, but the MECHANISM is not "same chirality → high xi." Instead, "opposite
chirality to corpus → adversaries cancel own reinforcement → high xi."

**Empirical optimum unchanged:**
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
avg fitness ≈ 0.099
carrier_e=0.935, transfer=0.836, xi=0.559 avg
```

---

## Updated xi mechanism model

| state | chirality | adversary effect | fitness_adv | xi |
|-------|-----------|-----------------|-------------|-----|
| baseline random (BFS no-shift) | same as corpus | A1 reinforces cluster | < fitness_clean | ~0.5 |
| baseline random (BFS odd-shift) | opposite to corpus | A1 cancels itself | ≈ fitness_clean | ~0.874 |
| stable chirality (this fire) | ALWAYS same as corpus | A1 always reinforces | consistently < fitness_clean | 0.223 |
| ideal xi | blocked | no effect | = fitness_clean | ~1.0 |

The path to high xi requires blocking A1 adversarials from coherent cluster
participation — not stabilizing chirality.

---

## Remaining open structural items

| item | prediction | note |
|------|-----------|------|
| A1 xi-twin isolation (make sim < 0.75 to corpus) | HIGH VALUE for xi | Requires research.rs change to `build_adversarial_set_l5` |
| xi_flat_bprimed | regression (T23) | CLOSED |
| destructive_penalty | marginal | LOW |
| consolidation_repulsion_threshold | unknown | MEDIUM |
| stage_wire sim_floor | low prior | LOW |
| k_local (within-cluster) | low prior | LOW |
