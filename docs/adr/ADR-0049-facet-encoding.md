# ADR-0049: Facet Encoding — Atomic Wavefronts, Resolve-Only Parents

**Status:** Proposed — **v2, revised after adversarial-design-review**
(GO_WITH_CHANGES, 2026-07-26). The encoding fix is *proven*; v2 resolves how
facet+parent coexists with six live subsystems.
**Relates to:** ADR-0047 (attention), ADR-0048 (origin-partition + energy-neutral,
shipped), ADR-0036 (dream consolidation / resonance-merge — the sharpest
interaction), and the HRM `WavefrontMeta` serialization + `dream` determinism
(#521) contracts.

## Context

The origin failure was chased through six layers, each eliminated by measurement,
down to the encoder. **Proven on the live medium:** the lab fact as one *compound*
memory (identity+place+building-id+market+note superposed) is not in the top-6 for
`"where is Kannaka Labs"` at any energy exponent; the *same fact stored atomically*
surfaces at **rank 1, sim 0.763**. Compound memories encode to smeared wavefronts;
atomic facts resonate. Reach is an **encoding** property.

The fix — encode a compound memory as atomic **facet** wavefronts linked to a
full-context **parent** — is correct in physics. The review confirmed it and found
that a naive implementation breaks the pervasive **1-wavefront = 1-memory**
assumption. v2 is built around the structural decision that resolves most of it.

## Decision

### The spine: parents are resolve-only, facets are the active wavefronts

A compound memory becomes **N atomic facet wavefronts** (the scanned, recalled,
reachable surface) plus **one resolve-only parent record** (the full-context
memory the caller reads). The parent is **retained but not an active wavefront**:
it is **excluded** from the resonance scan, the resonance-merge grouping, the
coherence/eigendecomp matrix, energy prune/ghost, and every user-facing count
(memory totals, clusters, Φ, the belief absorb-fraction denominator). Facets are
scanned for recall but are **also excluded** from the merge grouping and the
counts. This one decision defuses four blockers at once: dream can't re-absorb
siblings into the blur, the O(n²) matrix and scan RAM don't inflate, a parent
can't out-rank its own facets, and metrics/belief-caps stay honest. **Parent
retention is an invariant** — never delete a parent to save space (that dangles
every facet link and fragments holistic recall).

### Serialization (load-bearing)

`parent_id` and `is_facet`/`decomposed` are appended as the **last** serialized
`WavefrontMeta` fields (after `provenance`), `#[serde(default)]`, with a new
`WavefrontMetaPreFacet` fallback at the head of the decode chain (mirroring the
`PreProvenance` pattern). Get the ordering wrong and all ~3800 records misdecode
behind a valid checksum with no load error — lock it with a committed pre-facet
`.hrm` round-trip fixture. Do **not** overload `HyperMemory.parents` (that is
hallucination lineage, exported in the glyph spec) — facet linkage is a distinct
named field.

### Decomposition is a pure, deterministic, once-only function

- **Deterministic on the dream/backfill path.** Extraction must be a pure function
  of content bytes (fixed clause/sentence split, fixed unicode boundaries) so the
  #521 byte-identical dream-determinism contract holds. An LLM extractor, if ever
  used, runs **once at wake `remember`-time only**, persisting the facet set;
  dream backfill is a pure read that never re-invokes a model.
- **Idempotent & once-only.** A persisted `decomposed` flag on the parent plus a
  reverse `parent_id` index; the pass skips any already-faceted parent and never
  decomposes a facet; a parent-id watermark makes it resumable. Test:
  **decompose-twice == decompose-once (no-op).**
- **Quality gate + compound test.** Only decompose *compound, lived-origin*
  content (≥2 independent clauses AND lived/session origin; exclude Audio/Visual
  perception and the `research:` corpus; never split across a causal/conditional/
  contrastive connective — because/so/if/but). Drop low-value facets (min word
  count, no bare-id/numeric/pronoun-lead facets, require ≥1 salient noun, prepend
  the parent subject so "it holds…" → "Kannaka Labs holds…"; handle v1.2 / 6ab.67
  decimal boundaries).

### Recall: resolve-then-dedup on an over-fetched pool, observe once

One shared `resolve_facets(ids)` called from **every** recall producer
(`recall_against`, `recall_vector`, `recall_against_ids`) so an unresolved path is
a compile-time impossibility. Over-fetch a pool (`top_k × max_facets_per_parent`,
reusing the existing 3× over-fetch idiom, chiral.rs:504) → map facet→parent →
**dedup by parent keeping the max facet score** → truncate to `top_k`. Resolution
rewrites `id→parent.id` and `content→parent.content` for holistic context; on a
resolution miss, surface the facet as its own full result — never drop, never
claim a dead parent. **Observation/`retrieval_count` fires exactly once per
surfaced constellation on the canonical target, after dedup** — otherwise an
8-facet parent takes 8 energy injections and re-introduces the exact ADR-0048
rich-get-richer bias just shipped to kill it. A linked constellation
(parent+facets) is exempt from energy-prune and ShortTerm-evict while linked
(Pinned semantics), so recall never ghosts a parent whose facets are alive.

### Cost discipline (1-core / 6GB hub)

Facets-per-parent cap (≤4–6) against an explicit RAM budget; exclude facet/parent
rows from the coherence eigendecomp and `apply_belief_coupling`'s `n`;
batch-decompose all facets against **one** snapshotted `corpus_mean` and call
`rebuild_cache` once; drop the flat-medium mirror duplication for facet rows;
consider quantizing facet storage (facets are resonance handles, not
full-precision records); resize every reader unit's `MemoryMax`; measure `.hrm`
size, resident RAM, and scan wall-time before/after against the cron budget.

## Falsifiable benchmark

Run through the **same chiral path the daemon uses** (a readonly variant of
`chiral.recall` on a chiral fixture, in the same process that wrote the facets —
the flat readonly mirror does not re-sync and would prove nothing). Assert: (1) a
specific-facet query **rank-wins** where the compound buried it (Δrank); (2) a
whole-memory query still surfaces the parent (facets don't fragment holistic
recall); (3) **mutating-path energy assertion** — recall a compound-with-8-facets
vs an atomic control N times, assert the parent received no more total injection
than the atomic memory and was observed exactly once per recall; (4) parent-dedup
asserted; 0-result = FAIL.

## Confirmed (do not regress)

- Parent retention is mandatory; resolve-only metadata record is fine, deletion is
  not.
- Read-side facet→parent resolution over the returned (id,score) list (post-scan,
  non-mutating) is the right seam — do not push resolution into the medium scan.
- `parent_id` as the last serialized field + `PreFacet` fallback is correct and
  load-bearing.
- Single-writer isolation holds for backfill vs concurrent recall (atomic
  tmp-write+rename; restart readers post-backfill so facets become visible).
- The over-fetch-then-truncate machinery already exists — reuse it, don't reinvent.

## Build order (flags default OFF; smallest proven slice first)

1. **Serialization** — `parent_id`/`is_facet`/`decomposed` trailing fields +
   `PreFacet` fallback + round-trip fixture.
2. **Deterministic decompose at `remember`** (wake path) + facet-quality/compound
   gate; parent retained resolve-only.
3. **`resolve_facets` + over-fetch/dedup/observe-once** across all recall producers.
4. **Benchmark** (chiral path, mutating-energy assert) against the proven
   compound-vs-atomic fixture.
5. **Backfill** existing compounds — idempotent, watermarked, cost-budgeted, in a
   dream pass with parents/facets excluded from merge/eigendecomp.

## Alternatives considered (and discarded by the review)

- **Backfill via a non-deterministic LLM extractor in dream** — breaks #521.
- **Delete the parent to save storage** — dangles every facet, fragments holistic
  recall.
- **Resolution pushed into the medium scan / benchmark on the flat readonly path**
  — wrong seam; proves nothing about the live chiral medium.
- **Salient-term weighting / bigger pool / leave-compound** — refuted across the
  arc.
