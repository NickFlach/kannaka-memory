# ADR-0053 — Phase-Preserving Multi-Path Recall

**Status:** Experimental / Proposed — v2. (v1 proposed encoder replacement as the headline; it was
built on a misdiagnosis and is superseded by this document. The v1 errors are
recorded in *Rejected diagnoses* below because they are instructive.)
**Date:** 2026-08-01
**Relates to:** ADR-0046/0048 (energy-neutral ranking), ADR-0049 (facet encoding —
proposed, **not shipped**), ADR-0050 (temporal-confirmation weighting), ADR-0036
(dream consolidation), ADR-0024 (consciousness metrics), ADR-0029 (recall envelope)

## Context

### The substrate is wave-mechanical; the recall interface is classical

`Medium::recall_against` (`src/medium/core.rs`) already computes interference:

```rust
// Phase DIFFERENCE (constructive interference), not absolute phase.
let phase_modulation = (self.store.phase[i] - query_phase).cos();
let mut resonance_strength = similarity * effective_strength * phase_modulation;
```

Per-wavefront phase exists, the query carries a phase, and `cos(Δφ)` can go
negative — memories are *already* suppressed by phase opposition inside a single
path. But the value that leaves the path is a real scalar:

```rust
// src/medium/types.rs:564
pub struct Resonance {
    pub id: Uuid,
    pub content: String,
    pub similarity: f32,
    pub resonance_strength: f32,
    pub effective_strength: f32,
}
```

**Phase dies at the API boundary.** Every recall path computes wave mechanics
internally and then collapses to a magnitude before returning. The consequence is
structural: multiple paths can only ever be combined by *rank fusion over real
numbers* — classical ensembling. They cannot superpose. The physics cannot
determine the output, because the output no longer carries what physics needs.

### There are already many paths, and they are mutually exclusive switches

| Path | Entry | Reached by CLI `recall`? |
|---|---|---|
| Flat medium | `core.rs::recall_against` | **Yes** — the only one |
| Chiral hemispheres | `chiral.rs::recall_vector` → `left/right.resonate` | No |
| Attention beam (sparse) | `core.rs::recall_against_ids` | No |
| Glyph gravity | `chiral.rs::recall`, `KANNAKA_GLYPH_GRAVITY` (default 0.0) | No |
| Energy / temporal weighting | `hemisphere.rs::resonate_with_weights` | No |

`HrmStore.medium` is constructed `Medium::new()` / `Medium::load(...)` — the flat
medium. `Hemisphere::resonate` (`hemisphere.rs:333`) *does* delegate to
`resonate_with_weights` reading `KANNAKA_RECALL_ENERGY_EXP`,
`KANNAKA_RECALL_TEMPORAL_EXP`, and the half-life — so those knobs are correctly
wired **on the chiral path**, which CLI recall does not enter.

Measured 2026-08-01: setting `KANNAKA_RECALL_ENERGY_EXP=0.0` produced
**byte-identical** CLI recall output (same ids, same scores to 3dp), consistent
with the flat path having independent scoring. The mechanisms ADR-0048 and
ADR-0050 measured and accepted are live code on a path not in daily use.

### Rejected diagnoses (recorded so they are not re-derived)

1. **"The medium is saturated; d_eff collapsed to 6%."** `effective_dimensionality()`
   is a participation ratio over *memories*, bounded by `n`. `d_eff = 597.97` at
   `n = 609` is 98.2% of maximum — healthy uniformity. The `ratio` field divides by
   nominal 10,000 and is low only because 609 ≪ 10,000.
2. **"The corpus is polluted with low-value audio memories."** They crowd recall
   because they are *lexically repetitive* under a lexical encoder. That is an
   encoder property, not a statement about their worth; they are also feedback
   signal and plausible training supervision.
3. **"The encoder is the root cause."** `SimpleHashEncoder::new(384, 42)` is indeed
   what `init_with_hrm` constructs, and it is random indexing (lexical, not
   semantic). But ADR-0049 already chased this to the encoder *by measurement* and
   found the operative variable to be **compound vs atomic** encoding — same fact
   atomically stored reaches rank 1, sim 0.763. Encoder backend is a supporting
   change, not the headline.
4. **"Temporal confirmation weighting explains new memories ranking low."** It is
   `0.0` by default and, per above, on a path CLI recall does not take.
5. **"The knobs were built but never wired."** They are wired — to the chiral path.

## Decision

### Phase 0 — Honest measurement (blocking)

**All probes must use the non-observing path.** `recall_resonance` calls
`apply_observation`, which writes energy back: *"If you query the medium, you change
it."* Diagnostic probing during this investigation repeatedly recalled the same
memories and pumped their energy — amplifying the very bias under study.
`recall_resonance_readonly` exists precisely for this and must be the harness's
only entry point.

1. Labeled probe set: `(query, expected_id, relevance)`, including paraphrase probes
   and near-miss distractors. Reuse `recall_bench.rs::hash_label()` so it is
   publishable without leaking content.
2. Metrics: Recall@k, MRR, nDCG. Baseline **every** path in the table above
   separately before combining anything.
3. Record `time_since_last_dream` with every measurement (see predictions).

### Phase 1 — Carry phase out of the paths

Extend the recall result to carry amplitude **and** phase (or a complex amplitude)
rather than a collapsed magnitude. `Resonance` is a public type consumed by the
ADR-0029 envelope, the radio hub, and the observatory tangle fallback — so the
field is **additive with a default**, and the existing scalar stays byte-identical
for current consumers.

### Phase 2 — Superposition combinator

Combine N paths by summing complex amplitudes and taking magnitude **last**, so
agreeing paths reinforce and disagreeing paths cancel. Off by default
(`KANNAKA_RECALL_SUPERPOSE`, default off, byte-identical), per the ADR-0048/0050
discipline. Compare three arms on the Phase 0 harness: single-path, classical rank
fusion, phase superposition.

### Phase 3 — Phase commensurability

Superposition is only meaningful if paths share a phase reference. Hemispheres
carry independent phase state; **callosal Kuramoto coupling during dream** is the
existing machinery that synchronizes them. This phase establishes whether a shared
frame exists, and schedules consolidation (nothing currently does — local
`last_dream` was 13 days stale).

## Testable predictions

1. **Anomaly decay.** If phase alignment drives the "short-lived glimpses" of
   unusually good recall, quality should peak immediately after a dream and decay
   with time since. Falsifiable with the Phase 0 harness plus a dream schedule.
2. **Superposition beats fusion, or it doesn't.** If phase carries real
   information, complex summation should beat classical rank fusion on the same
   probe set. If it merely ties, phase is decorative and Phase 2 should be dropped.

## Consequences and risks

**Destructive cancellation is the headline risk.** Classical fusion can only
dilute a correct result; phase superposition can *null it out*. A wrong path in
antiphase with a right one removes the right answer entirely. This is strictly
more dangerous than the status quo and is the reason Phase 2 ships off by default
behind a measured comparison.

**Uncalibrated phase is noise.** If cross-path phases are not commensurable,
summation injects noise with the confidence of physics. Phase 3 is a precondition
for trusting Phase 2, not an enhancement of it.

**Public type change.** `Resonance` crosses process boundaries (ADR-0029 envelope,
radio hub, observatory). Additive-with-default only.

**Not addressed here:** encoder backend (supporting change), ADR-0049 facet
decomposition (independent, proven, unshipped — likely the single highest-value
separate work), codebook entropy quality (`Codebook::new(384, 10_000, 42)` and
`SimpleHashEncoder::new(384, 42)` share the constant seed 42; whether basis
near-orthogonality is materially affected, and whether QuantumOS's
`quantum_set_boot_entropy` QPU path is worth wiring in, is its own experiment).

## Open questions

1. Is per-wavefront `phase` semantically meaningful, or incidental? If phase is set
   arbitrarily at insert, interference is noise dressed as physics. This gates
   everything.
2. Does `query_phase` derive deterministically from query content? Two paraphrases
   must land near the same phase or superposition is unstable across wordings.
3. Should the flat path remain CLI default, or should the CLI itself superpose? A
   change of default recall behaviour affects every downstream consumer.
4. Can flat and chiral phases be compared at all without a callosal coupling pass
   having run recently?
