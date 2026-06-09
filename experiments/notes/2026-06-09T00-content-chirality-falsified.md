# Content-based chirality falsified — XOR assignment collapses transfer

**Date:** 2026-06-09T00 UTC
**Branch:** kannaka-curiosity/2026-06-09T00-content-chirality
**Code changes:** REVERTED (XOR content chirality in `stage_chiral_perturbation` added then removed)
**Status:** FALSIFIED — xi not stabilized, transfer catastrophically regressed

---

## Background

Current empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
avg fitness ≈ 0.099
carrier_e=0.935, transfer=0.836, xi=0.559 avg (range ~0.256–0.874)
```

Previous fire (T21) confirmed: xi variance is driven by adversarial UUID randomness changing
BFS cluster discovery ORDER in `find_synchronized_clusters`, which shifts cluster indices,
which changes `cluster_idx % 2` handedness assignments for corpus clusters. Proposed fix: use
XOR of cluster member UUIDs as a content-derived parity that is independent of BFS order.

---

## Hypothesis

**Replacing `cluster_idx % 2` with XOR of member UUID low-64 bits will stabilize corpus
cluster chirality across the clean and adversarial dream passes, reducing xi variance and
improving avg xi from 0.559 toward 0.8+.**

The key prediction was that adversarial memories either:
(a) form isolated clusters (sim < 0.75 coupling threshold) → corpus cluster XOR unchanged → high xi, or
(b) merge into corpus clusters but produce LESS variance than BFS-index approach (local vs global perturbation)

Main chain metrics predicted approximately stable: carrier_e and transfer rely on the PATTERN
of alternating chirality, not the specific left/right assignment.

---

## Method

Change in `stage_chiral_perturbation` (src/consolidation.rs):

```rust
// OLD: cluster index parity
let handedness = if cluster_idx % 2 == 0 { 1.0 } else { -1.0 };

// NEW: content-based XOR parity
let cluster_handedness: Vec<f32> = clusters.iter().map(|cluster| {
    let sig: u64 = cluster.memory_ids.iter()
        .map(|id| id.as_u128() as u64)
        .fold(0u64, |acc, x| acc ^ x);
    if sig % 2 == 0 { 1.0 } else { -1.0 }
}).collect();
// ...
let handedness = cluster_handedness[cluster_idx];
```

Two trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`.

---

## Results

| metric | baseline avg | T1 (xor-chirality) | T2 (xor-chirality) |
|--------|-------------|--------------------|--------------------|
| **fitness** | **0.099** | **0.167** (+0.068) | **0.210** (+0.111) |
| transfer_score | 0.836 | **0.345** (−0.491) | **0.312** (−0.524) |
| carrier_emergence | 0.935 | 0.946 (+0.011) | 0.946 (+0.011) |
| xi_robustness_v2 | 0.559 avg | 0.604 | 0.351 |
| magic_proxy_phase_R | 0.637 | 0.637 (det.) | 0.637 (det.) |
| query_gravity | 0.363 | 0.359 | 0.359 |

---

## Analysis

### Transfer catastrophically collapses under XOR chirality

Transfer drops from 0.836 to 0.312–0.345 (−0.5 magnitude). This is the dominant regression,
costing 0.15 × 0.5 ≈ +0.075 in fitness. carrier_e is fine (the amplitude dynamics are
preserved), but the A→B priming transfer is broken.

**Root cause:** The BFS-index parity assignment (`cluster_idx % 2`) produces a specific
left/right assignment for corpus clusters. Corpus cluster A (l4_dense memories) likely gets
index 0 in the clean pass (first discovered by BFS), receiving left-handed (+1.0) chirality.
Corpus cluster B (target memories) gets a complementary assignment.

The XOR parity assigns DIFFERENT handedness to some clusters. Since corpus UUID multiples
of C = 0x0123_4567_89AB_CDEF_0123_4567_89AB_CDEF, the XOR parity of each cluster is a
deterministic function of which corpus memories are in that cluster — but it's NOT the same
as the BFS-index parity. Some clusters that were left-handed under BFS-index become
right-handed under XOR, reversing their chiral perturbation.

The reversed perturbation for the carrier-dense cluster disrupts the phase structure that
drives A→B transfer. The amplitude gradient that `query_gravity` relies on appears less
sensitive (query_gravity drops only slightly: 0.363 → 0.359), but the transfer phase coherence
collapses completely.

### Xi is NOT stabilized by XOR approach

T1: xi=0.604, T2: xi=0.351 — still high variance (range ~0.25). Average 0.478, BELOW baseline
avg 0.559. The XOR approach does not reduce xi variance because:

- xi-twin adversaries (A1) target cluster CENTROIDS, so they have high cosine similarity to
  corpus clusters (above the 0.75 coupling threshold)
- A1 adversaries merge INTO corpus clusters, changing those clusters' XOR signatures
- The XOR parity of corpus clusters changes randomly based on adversarial UUID bit-0
- This produces similar variance to the old approach, but with a DIFFERENT absolute assignment
  that happens to be WORSE for transfer

### Fundamental insight

The BFS-index parity coincidentally produces the CORRECT left/right assignment for the
current corpus cluster structure. This assignment is what enables transfer=0.836 and
carrier_e=0.935. Any content-based assignment that differs from this "correct" assignment
will degrade transfer.

The correct fix for xi variance CANNOT change which corpus clusters are left vs right-handed
in the main chain. It must either:
1. Preserve the BFS-index assignment while preventing adversarial re-indexing, OR
2. Fix at the xi measurement level (make adversarial pass share the same cluster discovery
   results as the clean pass), OR
3. Prevent adversarial memories from affecting cluster membership at all

The XOR approach does none of these correctly.

### Instrumentation metrics are chirality-invariant

magic_proxy_phase_R (0.637) and query_gravity (0.359) are perfectly stable across both trials
and across the chirality change. These metrics are driven by amplitude distribution and phase
alignment, not by the specific left/right chirality assignment.

---

## Correct fix paths for xi variance

1. **Cache the clean-pass cluster assignments and reuse them in the adversarial pass.**
   If `find_synchronized_clusters` returns the same clusters for both passes (using the
   clean-pass engine's cluster cache), chirality is identical in both passes, and adversarial
   memories that join would be assigned to the same clusters with the same indices.
   **Risk**: This would require passing the clean-pass cluster results to the adversarial pass,
   which requires architectural changes to `eval_xi_robustness_v2`.

2. **Use the cluster CACHE from the clean pass in the adversarial pass.**
   The cluster cache (line 511 in kuramoto.rs) uses a `fingerprint_memories` key. If we
   could force the adversarial engine to use the clean engine's cluster cache, adversarial
   cluster indices would be identical. This is a narrow change but requires understanding
   the caching mechanism.

3. **Sort adversarial memories to isolated (near-zero similarity) positions.**
   Change `build_adversarial_set_l5` to generate adversarials with orthogonal vectors (sim ≈ 0
   to all corpus). Then adversarials form isolated clusters → corpus cluster indices unchanged
   → chirality unchanged → fitness_adv ≈ fitness_clean → high xi. Risk: orthogonal adversarials
   may not meaningfully test robustness.

---

## Decision

**No code changes retained. Hypothesis falsified.**

Empirical optimum unchanged:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
avg fitness ≈ 0.099
```

---

## Updated closed axes

| parameter | closed at | note |
|-----------|-----------|------|
| stage_chiral_perturbation XOR content chirality | CLOSED | Transfer −0.5, xi not improved |
| stage_chiral_perturbation BFS-index parity | KEEP | Coincidentally optimal for transfer |

## Remaining open structural items

1. **xi variance**: Clean-pass cluster cache reuse in adversarial pass is the mechanistically
   correct fix. Requires changes to `eval_xi_robustness_v2` and possibly `stage_chiral_perturbation`.
2. **stage_wire k_local**: untested. Very low prior.
3. **stage_wire sim_floor**: untested. Very low prior.
4. **destructive_penalty**: predicted marginal.
