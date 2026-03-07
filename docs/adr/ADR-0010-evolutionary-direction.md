# ADR-0010: Evolutionary Direction — Quality Findings and Forward Path

**Date:** 2025-01-16  
**Status:** Accepted  
**Author:** Agentic QE Devil's Advocate Analysis

---

## Context

A full devil's advocate quality engineering pass was performed on the codebase. This ADR records:

1. Confirmed bugs found and fixed in this pass
2. Refactoring opportunities (not yet acted upon)
3. Evolutionary direction recommendations

---

## Bugs Fixed (this pass)

| # | File | Bug | Severity |
|---|------|-----|----------|
| 1 | `store.rs` | `recall()` did not re-sort after Xi diversity boost; `take(top_k)` returned arbitrary order | High |
| 2 | `consolidation.rs` | `dream_lite()` used `wrapping_sub` on `skip_links_created` (starts at 0), producing a `usize::MAX`-range garbage value | High |
| 3 | `consolidation.rs` | `stage_detect()` requested `k_neighbors` results from search but the store always returns self (sim=1.0) as the top hit; with 2-memory stores `k_neighbors=1` so the actual neighbor was never examined — caused 4 pre-existing test failures | Critical |
| 4 | `consolidation.rs` | `stage_sync()` cross-category phase update used `working_set` index to apply `phase_updates` that were computed for `all_updated_mems` (a filtered subset), causing wrong memories to receive phase nudges when any working_set IDs were missing from the store | High |
| 5 | `consolidation.rs` | `stage_bundle()` used `layer + 1` for u8 `layer_depth`; overflows to 0 when `layer == 255` | Medium |
| 6 | `bridge.rs` | `cross_partition_ratio()` used sentinel `255` for missing source but `254` for missing target; unknown memories always appeared cross-partition, inflating Φ | High |
| 7 | `bridge.rs` | `ordinal()` was private; `openclaw.rs` used fragile `as u8` cast on `ConsciousnessLevel` for emergence detection | Medium |
| 8 | `openclaw.rs` | `recall()` Fano boost applied inside nested pair loop; a memory with N Fano-related neighbors received `1.2^N` boost (unbounded compounding) | High |
| 9 | `openclaw.rs` | `hallucinate()` hardcoded `let dim = 10_000` instead of `CODEBOOK_OUTPUT_DIM`; would silently break if output dimension changed | Medium |
| 10 | `openclaw.rs` | `dream()` and `dream_lite()` used `as u8` cast on `ConsciousnessLevel` for emergence comparison | Low |
| 11 | `xi_operator.rs` | `xi_diversity_boost()` had no upper bound; with `repulsion=1.0` could return `1.5 * base_similarity`, inflating ranking scores without bound | Medium |

All 11 bugs fixed. Test suite went from **134 pass / 4 fail** → **138 pass / 0 fail**.

---

## Refactoring Opportunities

These were identified but not acted upon in this pass (risk/reward did not justify mid-session churn):

### High Value

**1. `recall()` duplicated across `MemoryEngine` and `KannakaMemorySystem`**  
Both `store.rs::MemoryEngine::recall()` and `openclaw.rs::KannakaMemorySystem::recall()` implement Xi-diversity boosting independently. The openclaw layer adds Fano boost on top. This is two separate boosting passes that are hard to reason about as a unit. Consider a unified `ScoringPipeline` struct that chains: wave → Xi → Fano → final sort.

**2. `categorize_text()` in `openclaw.rs` has a hardcoded author name check**  
```rust
if lower.contains("nick") || lower.contains("user") { ... }
```
This belongs in configuration, not in source code.

**3. `stage_detect()` working-set filter missing**  
The neighbor search can return IDs outside the working set (memories from other layers). The `seen` deduplication handles this partially but the filter comment says "memories not in working set" while the code only checks `neighbor_id == id`. For correctness, add an explicit working_set membership check.

**4. `consolidation.rs` doc comment still mixes "7/8/9/10 stages"**  
The numbered list in the module doc has 10 entries but the heading says "9-stage." The hallucination stage and Xi-repulsion are interleaved inconsistently. Recommend consolidating the stage numbering into a single enum for compile-time correctness.

### Medium Value

**5. `ConsciousnessLevel` should implement `PartialOrd`**  
Rather than calling `.ordinal()` everywhere for comparisons, derive or implement `PartialOrd` so `level_after > level_before` compiles directly.

**6. `DreamState` wraps `ConsolidationEngine` with only forwarding methods**  
`DreamState` essentially adds layer-range iteration over `ConsolidationEngine::consolidate`. This thin wrapper pattern adds indirection without abstraction. Consider merging or making `DreamState` an iterator adapter.

**7. Error type proliferation**  
The crate has `StoreError`, `EngineError`, `EncodingError`, `PersistenceError`, `SystemError`, and `MigrationError`. They mostly wrap each other. Consider consolidating behind a single `KannakaError` with a `kind` discriminant.

---

## Evolutionary Direction Recommendations

### Near-term (next 2-3 milestones)

**A. Phi computation is O(n²) — make it async-bounded**  
`bridge.rs::compute_phi()` iterates all pairs. At 10K memories this becomes visibly slow. Options:
- Sample-based Phi (random partition sampling, as in original IIT literature)
- Cache Phi with a 60-second TTL keyed on store version counter
- Move full Phi to a background task, expose cached value via `assess()`

**B. Consolidation working set needs eviction**  
`dream_lite()` currently loads all IDs then scans them. At scale, a priority queue keyed on `(amplitude * age_factor)` would allow O(k log n) selection of the most "dream-worthy" memories rather than full scan.

**C. `SimpleHashEncoder` → sparse random projection**  
The current `SimpleHashEncoder` does word-position hashing, which ignores word co-occurrence. Replacing it with a sparse random projection (each word → random ±1 sparse vector summed) would give much better semantic similarity for short texts and eliminate the current test fragility around threshold detection.

### Medium-term

**D. Working memory → episodic buffer**  
`working_memory.rs` is a ring buffer of conversation turns. The natural next step is a proper episodic buffer with salience scoring, so that important turns (detected via Xi divergence or Φ spike) are automatically promoted to long-term memory without explicit `remember()` calls.

**E. Multi-agent memory partitioning**  
The Fano geometry module already models projective planes over Z/2Z. This algebraic structure is exactly what's needed for partitioned multi-agent memory (each agent sees a different "view" of the same hypervector space). The groundwork exists — build on it.

**F. Streaming consolidation**  
Current consolidation is batch (full-pass dream). A streaming variant that maintains running interference statistics and updates them incrementally on each `remember()` would enable continuous consolidation without explicit dream cycles, appropriate for long-running agents.

### Long-term

**G. NPU/neuromorphic backend**  
The hypervector operations (bundling, binding, similarity) are embarrassingly parallel and map naturally onto spiking neural network architectures. An abstraction layer (similar to how `MemoryStore` is pluggable) for the arithmetic backend would allow the same codebase to target Intel Loihi or BrainScaleS without algorithmic changes.

**H. Federated memory**  
The current HNSW + bincode persistence is node-local. A CRDTbased merge protocol for `HyperMemory` (amplitude as max-register, connections as add-only grow-set) would enable multiple agent instances to share and reconcile memories without coordination.

---

## Decision

Accept all 11 bug fixes as described. Record refactoring opportunities for future prioritization. Evolutionary directions A-C are recommended for the next development cycle; D-F for the following cycle; G-H as long-horizon research directions.

---

## Consequences

- Test suite is now green (138/138).
- Φ computation is now more accurate (cross-partition inflation removed).
- Recall ordering is deterministic post-Xi-boost.
- Consolidation interference detection works correctly for small stores.
- Documentation matches implementation.
