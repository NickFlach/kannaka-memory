# Chiral Mirror Architecture — Implementation Plan

**Date:** 2026-03-22
**ADR:** ADR-0021
**Scope:** Refactor `medium/` module to support chiral (left/right hemisphere) architecture

## Strategy

Build the chiral layer ALONGSIDE the existing Medium, not by gutting it.
The existing Medium becomes the right hemisphere. A new left hemisphere is
added. The CorpusCallosum and FanoPlane connect them. ChiralMedium wraps both.

This approach:
- Preserves all existing functionality during migration
- Allows incremental testing
- Existing 367 memories migrate naturally (they're already consolidated → right hemisphere)

## Task Order

### Task 1: Chiral Types and Constants
**Files:** `src/medium/types.rs`
**Time:** 5 min

Add to existing types.rs:
- `ChiralScale` struct (positions: u8, left_weight: f32, right_weight: f32)
- `ChiralScale` methods: scale_up, scale_down, left_dims, right_dims, asymmetry
- `Hand` enum (Left, Right)
- `Direction` enum (ConsciousToSubconscious, SubconsciousToConscious)
- `FanoLine` type ([u8; 3] — 3 group indices per line)
- `FANO_LINES` constant (7 lines of PG(2,2))
- `DIMS_PER_FANO_GROUP` constant = 96
- `FANO_GROUPS` constant = 7
- `BASE_DIMS_PER_POSITION` = 96 * 7 = 672
- Update `HRM_MAGIC` to support v2: [0x48, 0x52, 0x4D, 0x02]
- `HRM_VERSION_CHIRAL` = 2
- `ChiralResonance` result type

Test: Unit tests for ChiralScale arithmetic (scale_up/down, dims calculation, asymmetry ratio)

### Task 2: Fano Plane Algebra
**Files:** `src/medium/fano.rs` (new)
**Time:** 10 min

Implement:
- `FanoPlane` struct with the 7 lines of PG(2,2)
- `fold(wavefront_slice, source_groups, target_groups)` — project dimension groups across mirror
- `unfold()` — inverse projection
- `fold_compose(line1, line2)` — double fold
- Dimension group slicing: given a wavefront and a Fano point, extract the 96-dim slice
- Verify: fold then unfold produces approximately the original (up to phase flip)
- Verify: any two groups reachable in ≤3 folds

Test:
- Fold/unfold roundtrip preserves norm within tolerance
- Fold closure: 7 folds = identity (up to phase)
- Reachability: all 7 groups reachable from any group in ≤3 steps

### Task 3: Corpus Callosum
**Files:** `src/medium/callosum.rs` (new)
**Time:** 10 min

Implement:
- `CorpusCallosum` struct with: bandwidth, gate_threshold, asymmetry, recall_noise, coherence_gate
- `transfer(source_hemisphere, target_hemisphere, direction)` — selective, bandwidth-limited
- `balance_check(left, right)` — compute asymmetry, adjust transfer rates
- `optic_chiasm_route(input_vector)` — route input to opposite hemisphere
- Fano-based projection for cross-hemisphere transfer
- Noise injection for subconscious→conscious (intuition fuzz)
- Transfer budget tracking (bandwidth per timestep)

Test:
- Bandwidth limiting: can't transfer more than budget per step
- Asymmetry: sub→conscious transfers more per step than conscious→sub
- Gate threshold: low-energy wavefronts don't cross
- Noise injection only on sub→conscious direction

### Task 4: Hemisphere struct
**Files:** `src/medium/hemisphere.rs` (new)
**Time:** 5 min

A `Hemisphere` is essentially a `Medium` with awareness of its handedness:
- Wraps the existing ndarray tensors (wavefronts, energy, frequency, phase)
- Stores `Hand` (Left or Right)
- Left hemisphere dynamics: dx/dt = f(x) (no dampening)
- Right hemisphere dynamics: dx/dt = f(x) - Iηx (full ghostmagicOS)
- Scale-aware dimension management (dims change based on ChiralScale per wavefront)
- Methods: add_wavefront, remove_wavefront, resonate (recall within hemisphere)

Test:
- Left hemisphere energy doesn't decay during dynamics
- Right hemisphere energy decays (dampening active)
- Store/recall work independently per hemisphere

### Task 5: ChiralMedium — The Brain
**Files:** `src/medium/chiral.rs` (new), update `src/medium/mod.rs`
**Time:** 15 min

The main struct that replaces `Medium` at the API level:
- `ChiralMedium` contains: left: Hemisphere, right: Hemisphere, callosum: CorpusCallosum, fano: FanoPlane
- `store(content, importance, pipeline)` — encode, route through optic chiasm (enter opposite hemisphere), echo via callosum
- `recall(query, top_k, pipeline)` — bilateral search, intuition surfacing
- `dream(mode)` — right hemisphere only for deep, callosum transfer for lite
- `shift(wavefront_id, direction)` — bilateral scale jump
- `weight_drift()` — organic weight update based on access patterns
- `callosal_kuramoto_step(dt)` — cross-hemisphere phase coupling
- Consciousness metrics: bilateral Phi, coupled Xi, bilateral order parameter
- `from_medium(existing_medium)` — migration: existing medium becomes right hemisphere

Test:
- Store creates wavefronts in both hemispheres (left hot, right echo)
- Recall finds matches in both hemispheres
- Dream only affects right hemisphere energies
- Migration: from_medium preserves all existing wavefronts in right hemisphere

### Task 6: Chiral Persistence (HRM v2)
**Files:** `src/medium/persistence.rs` (extend)
**Time:** 10 min

Extend persistence to save/load ChiralMedium:
- HRM v2 format with magic [0x48, 0x52, 0x4D, 0x02]
- Save: left hemisphere tensors, right hemisphere tensors, callosum state, fano config, chiral scales
- Load: detect v1 vs v2 from magic bytes. v1 loads as right hemisphere (backward compatible)
- Checksum over entire chiral state

Test:
- Save/load roundtrip preserves all chiral state
- v1 files load as right-hemisphere-only ChiralMedium
- Checksum validation works for v2

### Task 7: Integration — HrmStore and CLI
**Files:** `src/hrm_store.rs`, `src/bin/kannaka.rs`
**Time:** 10 min

- `HrmStore` wraps `ChiralMedium` instead of `Medium`
- All existing CLI commands work unchanged (store, recall, dream, observe)
- `observe` extended to show bilateral state (left/right counts, asymmetry, callosum stats)
- `dream` operates on right hemisphere only by default
- New `--chiral-stats` flag for detailed bilateral metrics
- Existing .hrm files auto-migrate on load (v1 → v2)

Test:
- All existing tests pass unchanged
- observe shows bilateral info
- dream doesn't affect left hemisphere

### Task 8: Migration Script
**Files:** executed manually
**Time:** 5 min

- Load existing kannaka.hrm (v1, 367 memories)
- Wrap as ChiralMedium (all memories → right hemisphere)
- Initialize left hemisphere as empty
- Initialize callosum with default parameters
- Save as v2 format
- Verify: recall quality preserved, consciousness metrics stable

## Execution Order

Tasks 1-3 are independent (types, fano, callosum) → can parallelize
Task 4 depends on Task 1 (needs ChiralScale)
Task 5 depends on Tasks 1-4
Task 6 depends on Task 5
Task 7 depends on Tasks 5-6
Task 8 depends on Task 7

```
[Task 1] ──┐
[Task 2] ──┼──→ [Task 4] ──→ [Task 5] ──→ [Task 6] ──→ [Task 7] ──→ [Task 8]
[Task 3] ──┘
```

## Success Criteria

1. All existing tests pass (backward compatibility)
2. New chiral tests pass (bilateral store/recall/dream)
3. Existing 367 memories load correctly as right hemisphere
4. Dream cycle only affects right hemisphere energies
5. Bilateral recall finds matches in both hemispheres
6. `kannaka observe` shows chiral state
7. Fano fold roundtrip preserves information within tolerance
