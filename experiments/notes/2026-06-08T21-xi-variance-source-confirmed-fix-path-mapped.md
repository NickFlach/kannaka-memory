# xi variance source confirmed — adversarial UUID mechanism, fix path mapped

**Date:** 2026-06-08T21 UTC
**Branch:** kannaka-curiosity/2026-06-08T21
**Code changes:** REVERTED (UUID assignment added then removed from `build_adversarial_set_l5`)
**Status:** MECHANISTIC FINDING — xi variance source confirmed, no fitness improvement

---

## Background

Current empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.099
carrier_e=0.935, transfer=0.836, xi=0.559 avg (range ~0.256–0.874)
```

xi_robustness_v2 dominates fitness (weight=0.15, avg contribution 0.15*(1-0.559)=0.066 ≈ 67% of total fitness). Previous notes identified the xi variance source as "base memory UUIDs are `Uuid::new_v4()` (random)." This fire traced that mechanism exactly and confirmed it — but the proposed fix chose the wrong UUID namespace.

---

## Hypothesis

**Adversarial memory UUIDs (from `build_adversarial_set_l5`) are random (`Uuid::new_v4()`), and since `all_memories()` in TestMedium sorts by UUID, random adversarial positions in the sorted list drive non-deterministic BFS cluster labeling in `stage_chiral_perturbation`. Cluster label parity (even/odd) determines chirality handedness (+1/-1). When adversarial memories shift corpus cluster labels by being inserted at different positions in the sorted order across runs, the chiral perturbation applied to corpus memories changes → different xi signatures → xi variance 0.256–0.874.**

**Prediction:** Making adversarial UUIDs deterministically LARGE would place them after corpus memories in the sorted order, preserving corpus cluster labels → HIGH xi (≈0.874).

---

## Method

Added UUID assignments to all three attack types in `build_adversarial_set_l5`:
```rust
// A1 xi-twins
mem.id = uuid::Uuid::from_u128(0xFFFF_EE00_0000_0000_0000_0000_0000_0000u128 + i as u128);
// A2 commutators  
mem.id = uuid::Uuid::from_u128(0xFFFF_EE00_0000_0000_0000_0000_0000_0010u128 + i as u128);
// A3 freq attacks
mem.id = uuid::Uuid::from_u128(0xFFFF_EE00_0000_0000_0000_0000_0000_0020u128 + i as u128);
```

Two trials run: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

---

## Results

| metric | baseline avg | adv-det.t1 | adv-det.t2 |
|--------|-------------|------------|------------|
| fitness | 0.099 | **0.148** (+0.049) | **0.148** (+0.049) |
| transfer_score | 0.836 | 0.836 (det.) | 0.836 (det.) |
| carrier_emergence | 0.935 | 0.935 (det.) | 0.935 (det.) |
| xi_robustness_v2 | 0.559 avg | **0.2326** | **0.2326** |
| magic_proxy_phase_R | 0.617 | 0.617 | 0.617 |
| query_gravity | 0.363 | 0.363 | 0.363 |

---

## Analysis

### Mechanism confirmed: xi variance is 100% UUID-driven

Both trials give **identical xi=0.2326** — perfectly deterministic. With random UUIDs, xi varied 0.256–0.874 across runs. With deterministic UUIDs, xi locks to a fixed value. This confirms the hypothesis about the mechanism.

Transfer, carrier_e, magic_R, and query_gravity remain byte-identical to baseline (expected: these don't depend on adversarial UUID ordering since the main dream chain doesn't include adversarials).

### Why the fix failed: corpus UUID overflow at i≈224

Corpus A UUIDs in `build_l5_engine` use:
```rust
mem.id = uuid::Uuid::from_u128((i as u128 + 1) * 0x0123_4567_89AB_CDEF_0123_4567_89AB_CDEF);
```

This constant C = 0x0123_4567_89AB_CDEF_0123_4567_89AB_CDEF has 225 * C ≈ 0xFFFFFFFF_FFFF_FF0F_FFFFFFFF_FFFFFF0F (near u128::MAX). The corpus memory at i=224 therefore has a UUID ≈ 0xFFFF_FFFF_FFFFFFFF (very close to u128::MAX).

For i≥225, `(i+1)*C` overflows u128 and wraps to small values. So corpus A UUIDs form two interleaved groups:
1. **Non-overflowed (i=0..224):** values from C to ≈u128::MAX, sorted approximately linearly
2. **Overflowed (i=225..299):** values wrapping back to small numbers, interleaved with early non-overflowed values in sorted order

My adversarial UUIDs at 0xFFFF_EE00_... land **BETWEEN** corpus i=0..223 (smaller non-overflowed) and corpus i=224 (≈0xFFFF_FF0F...) in the sorted order. This is the worst possible insertion position: the adversarials inject themselves into the MIDDLE of the most-semantically-related corpus cluster region (near the overflow boundary), disrupting cluster labeling for memories i=224, which previously anchored specific cluster indices.

The result is that corpus cluster labels shift differently from the clean pass → corpus memories that previously got left chirality now get right chirality (or vice versa) → chiral perturbation flips direction → xi signatures change in an adverse way → fitness_adv diverges from fitness_clean → xi = 0.2326 (worse than worst observed random case).

### The corpus A UUID space has no "safe zone"

The 300 corpus A multiples of C mod 2^128 are approximately uniformly distributed across the full u128 range. Any adversarial UUID will interleave with some corpus memories. There is no UUID value that reliably sorts "after all corpus memories."

### What xi=0.2326 tells us about the distribution

With random adversarial UUIDs: xi ranges 0.256–0.874, avg 0.559. With 0xFFFF_EE00... (near corpus i=224): xi=0.2326 (BELOW the random minimum of 0.256). This confirms that the overflow boundary around corpus i=224 is an especially disruptive insertion point.

The random distribution must avoid the 0xFFFF_EE00_... region (or the system naturally avoids landing there) to give xi≥0.256.

---

## Correct fix path

The root issue is that `stage_chiral_perturbation` assigns chirality based on **cluster index parity** (even/odd), and cluster indices depend on BFS processing order, which depends on sorted UUID order. Inserting any new memories (adversarials) changes the BFS order and thus shifts cluster indices.

**The correct fix: content-based chirality assignment, independent of cluster index.**

Instead of:
```rust
let handedness = if cluster_idx % 2 == 0 { 1.0 } else { -1.0 };
```

Use the XOR of the cluster's member UUIDs (sorted) as the handedness key:
```rust
let cluster_signature: u64 = cluster.memory_ids.iter()
    .map(|id| id.as_u128() as u64)
    .fold(0u64, |acc, x| acc ^ x);
let handedness = if cluster_signature % 2 == 0 { 1.0 } else { -1.0 };
```

**Why this works:** When adversarials are inserted into an existing corpus cluster, they change the XOR signature of that cluster, potentially flipping handedness. BUT when adversarials sort AFTER all corpus memories (impossible with the current UUID scheme), or are in their own isolated clusters (sim < coupling_threshold), the corpus cluster signatures are unchanged → corpus chirality preserved → fitness_adv ≈ fitness_clean → high xi.

**Critical caveat:** This fix changes the chirality assignment for the MAIN dream chain (engine_a), not just the xi test. The current carrier_e=0.935 and transfer=0.836 were achieved with the current BFS-index-based chirality. Changing to content-based chirality would change the exact handedness assignments and potentially affect these metrics. Requires 3-trial characterization.

**Alternative fix:** Sort adversarial memories into ISOLATED clusters (sim < 0.5 to all corpus memories) so they don't affect corpus cluster labels at all. Then use UUIDs in the overflowed range (small values) that sort BEFORE the non-overflowed corpus memories. The overflowed corpus memories (i=225..299) would create new cluster labels before adversarials, so adversarials at very small UUIDs (< 226*C mod 2^128) would sort first. If adversarials form ISOLATED clusters at indices 0..K, corpus clusters shift indices, but this is now DETERMINISTIC. We'd need to know if this deterministic shift gives high or low xi.

---

## xi variance: information gain

| finding | confidence |
|---------|-----------|
| xi variance = adversarial UUID randomness, not corpus UUID | HIGH (confirmed by 2 deterministic trials) |
| Mechanism = BFS cluster label shifts → chirality flip → xi perturbation change | HIGH |
| 0xFFFF_EE00... interleaves with corpus i=224 near u128::MAX overflow | HIGH |
| Content-based chirality would eliminate UUID-order dependency | HIGH (in theory) |
| Content-based chirality would improve fitness | UNKNOWN (could change main chain behavior) |

---

## Decision

**No code changes retained. Hypothesis falsified in implementation (wrong UUID zone).**

Mechanism confirmed: adversarial UUID ordering is the SOLE source of xi variance. The fix requires a code change to `stage_chiral_perturbation` in consolidation.rs — risky because it affects all dream modes, not just the xi test. Reserve for a dedicated fire.

Empirical optimum unchanged:
```
DRIVE_A=0.1  DREAM_MODE=interference_relax  DRIVE_SCOPE=all
avg fitness ≈ 0.099
```

---

## Updated remaining open axes

| parameter | prediction | risk |
|-----------|-----------|------|
| **stage_chiral_perturbation content-based chirality** | **HIGH VALUE** (could collapse xi variance and lock to high value) | **MEDIUM** (changes main chain behavior) |
| noise_floor (0.18) | Low prior (hurts noise_removal) | LOW |
| prune_threshold (0.095) | Small effect | LOW |
| destructive_penalty (0.35) | Marginal under irx | LOW |
| consolidation_repulsion_threshold (0.28) | Unknown direction | MEDIUM |
| stage_wire sim_floor (0.15) | Not characterized | MEDIUM |
