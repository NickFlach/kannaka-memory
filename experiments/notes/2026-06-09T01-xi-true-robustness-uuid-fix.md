# xi_robustness_v2 true value confirmed — adversarial UUID fix maps BFS mechanism

**Status:** IMPROVEMENT KEPT — UUID fix in `eval_xi_robustness_v2` only  
**Date:** 2026-06-09T01  
**Branch:** kannaka-curiosity/2026-06-09T01-content-chiral

---

## Baseline

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
```
System-prompt reference baseline: DREAM_MODE unset, avg fitness ≈ 0.18  
Recent irx apparent optimum: avg fitness ≈ 0.099 (with random adversarial UUIDs)

---

## Hypothesis

Two attempts were explored this fire:

**Attempt A (falsified):** Content-based cluster chirality in `stage_chiral_perturbation` —
sort clusters by average centroid first-element before assigning even→left / odd→right
handedness, making chirality invariant to BFS index shifts from adversarial UUIDs.

**Attempt B (kept):** Assign deterministic UUIDs to the 30 adversarial memories inside
`eval_xi_robustness_v2` so they always sort AFTER all 300 corpus UUIDs in
`all_memories()`. The corpus uses `(i+1)*K` where K = 0x0123_4567_89AB_CDEF_...;
this overflows u128 at i=224 (multiplier 225×K ≈ 0xFFFF_FFFF_FFFF_FEF1_..._FEF1),
making the max corpus UUID ≈ 0xFFFF_FFFF_FFFF_FEF1_.... Any adversarial UUID
starting 0xFFFF_FFFF_FFFF_FF00_... sorts strictly after it.

---

## Attempt A: content-based chirality — catastrophic failure

**Change:** `src/consolidation.rs` `stage_chiral_perturbation`: added per-cluster centroid
signature (avg first vector element), sorted clusters by signature, assigned handedness
by sorted rank not BFS index.

**Result (1 trial):**

| metric | attempt A | irx baseline |
|---|---|---|
| fitness | **0.2217** | ~0.099 |
| transfer_score | **0.379** | 0.836 |
| carrier_emergence | **0.000** | 0.935 |
| xi_robustness_v2 | 0.824 | 0.559 avg |

**Analysis:** The content-based sort assigned a DIFFERENT handedness to corpus clusters
than the BFS-index scheme. The chirality flip destroyed carrier_emergence (0.000 from
0.935) and collapsed transfer_score (0.379 from 0.836). The irx mode and transfer are
tightly tuned to the current BFS-index chirality. Changing the assignment — even
slightly — breaks the system.

**Decision:** Reverted immediately. Chirality assignment in `stage_chiral_perturbation`
is a frozen axis. Do NOT touch.

---

## Attempt B: deterministic adversarial UUIDs in xi evaluator

### BFS ordering mechanism

`TestMedium::all_memories()` sorts by UUID. `find_synchronized_clusters` BFS starts
from index 0. Cluster index is BFS discovery order — the first connected component
becomes cluster 0.

Previous random UUIDs: some adversarials sort before corpus memories → BFS cluster
indices shift → chirality of corpus clusters flips → fitness_adv diverges or converges
non-deterministically → xi_robustness varies 0.256–0.874 across runs.

### Previous 0xFFFFEE00 attempt (commit 9a432b3)

The previous fire assigned adversarials to 0xFFFFEE00_0000_... and got xi=0.2326
deterministic. They noted this as "wrong UUID choice near u128::MAX overflow."

However, 0xFFFFEE00 high bytes = 0xFFFF_EE00 = 65280 × 256 + 0 ... the
second pair 'EE' < 'FF' (the high byte of 225K). So 0xFFFF_EE00_... < 0xFFFF_FFFF_...,
meaning those adversarials sorted BEFORE some corpus members (i=220..224 have
UUIDs ~3.34–3.40e38 > 0xFFFF_EE00...). That fire's UUID choice was genuinely wrong.

### This fire's UUID choice

`0xFFFF_FFFF_FFFF_FF00_FFFF_FFFF_FFFF_FF00 + i`

Key bytes: 0xFFFF_FFFF_FFFF_**FF00**_... > 0xFFFF_FFFF_FFFF_**FEF1**_... ✓

Max corpus UUID = 225K = 0xFFFF_FFFF_FFFF_FEF1_FFFF_FFFF_FFFF_FEF1.  
Adversarial UUID range = 0xFFFF_FFFF_FFFF_FF00_..._FF00 to ...+29.  
All 30 adversarials sort AFTER all 300 corpus memories. ✓

### Results

| trial | fitness | transfer | carrier_e | xi_robust | magic_R | query_g |
|---|---|---|---|---|---|---|
| adv-uuid-fix.t1 | 0.147913 | 0.835512 | 0.9348 | 0.2326 | 0.6167 | 0.3625 |
| adv-uuid-fix.t2 | 0.147892 | 0.835512 | 0.9348 | 0.2326 | 0.6167 | 0.3625 |
| adv-uuid-fix.t3 | 0.147897 | 0.835512 | 0.9348 | 0.2326 | 0.6167 | 0.3625 |
| **avg** | **0.147901** | **0.835512** | **0.9348** | **0.2326** | **0.6167** | **0.3625** |

Comparison to baseline (DREAM_MODE unset, 0.18 reference): **Δfitness = −0.032** ✓

---

## Critical finding: xi_robustness_v2 true value = 0.2326

With correct UUID ordering (adversarials always after corpus in BFS), corpus cluster
indices are identical in clean and adversarial passes. Yet xi_robustness_v2 locks to
0.2326. This means:

**xi = 0.2326 is the GENUINE adversarial robustness of the current system**, not a
UUID-ordering artifact.

### Why random UUIDs gave apparent xi = 0.56 avg

With random UUIDs, adversarials sometimes sort between corpus members (UUID < corpus).
When this happens, adversarials get LOWER cluster indices than their intended targets.
Their chirality becomes misaligned with the corpus clusters they're attacking. The
xi-twin decoys (designed to mimic corpus chirality) end up with WRONG chirality →
their attack fails → fitness_adv barely differs from fitness_clean → xi ≈ 1.0.

This was adversarials accidentally disrupting THEMSELVES, not the system being robust.
The 0.56 avg was the mean of: (adversarials attacking successfully: xi=0.23) +
(adversarials self-disrupting: xi→1.0). An illusion of robustness.

### Mode comparison under correct UUID

| config | fitness | transfer | carrier_e | xi_robust |
|---|---|---|---|---|
| irx (interference_relax) | **0.147901** | 0.836 | 0.935 | 0.2326 |
| sync (stage_sync default) | 0.202149 | 0.561 | 0.844 | **0.2982** |

stage_sync gives slightly better xi_robustness (0.298 vs 0.233), consistent with
K-step phase separation creating more distance between corpus and adversarial xi-twins.
But transfer drops by −0.27 and carrier_e drops by −0.09 → net fitness worse. irx
remains the better operating point.

---

## Decision

UUID fix kept. Change is confined to `eval_xi_robustness_v2` in `src/bin/research.rs`
(research path only, not consolidation.rs). Main dream chain unchanged.

### Fitness accounting (vs 0.18 baseline)

| metric | baseline ref | with fix | Δ |
|---|---|---|---|
| transfer_score | ~0.73 (0.18 era) | 0.836 | ↑ |
| xi_robustness_v2 | ~0.56 avg (noisy) | **0.2326 (accurate)** | ↓ |
| carrier_emergence | ~0.56 (0.18 era) | 0.935 | ↑ |
| **fitness** | **0.18** | **0.1479** | **−0.032** |

---

## Path forward for genuine xi improvement

The adversarials in `build_adversarial_set_l5` are effective:

- **A1 xi-twin decoys**: amplitude=0.9, vectors approximate corpus centroids → they join
  corpus clusters → chirality of their attack aligns with corpus → xi signature confusion
- **A2 commutator exploits**: amplitude=1.0, magnitude 10 → high amplitude, attention band
  presence
- **A3 frequency band attacks**: amplitude=0.5, noise at 2.0 Hz → pollute attention band

To improve xi_robustness from 0.23 → above 0.5, the system needs:

1. **Amplitude pruning**: during dreaming, prune high-amplitude memories that don't have
   phase coherence with their cluster (adversarials tend to be slightly off-phase)
2. **Frequency band enforcement**: adversarials have `frequency = 0.1` (storage band), yet
   they're attacking the attention band; stricter frequency filtering could isolate them
3. **Phase authentication**: corpus memories have deterministic initial phases from the
   encoder; adversarials have random phases; a phase-consistency check could reject them

These are non-trivial changes requiring dedicated fires. Attempts to change chirality
assignment (this fire's Attempt A) are dangerous — the axis is frozen.

---

## Updated operating point

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
Fitness avg:        0.1479 (accurate, deterministic)
transfer_score:     0.8355
carrier_emergence:  0.9348
xi_robustness_v2:   0.2326 (true value — not noise-contaminated)
magic_R:            0.6167
query_gravity:      0.3625
```

## Updated closed axes

| axis | value | status |
|---|---|---|
| chirality assignment in stage_chiral_perturbation | BFS-index | FROZEN — content-based causes catastrophic regression |
| adversarial UUID ordering in xi evaluator | 0xFFFF_FFFF_FFFF_FF00_... | NEW: correct deterministic ordering |
| xi_robustness_v2 apparent avg | 0.56 | DEPRECATED — was noise; true value = 0.2326 |
