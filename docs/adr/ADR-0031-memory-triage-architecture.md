# ADR-0031 — Memory Triage Architecture (retire prune-cron as a bridge measure)

Status: **Accepted** (ratified 2026-06-06; Phase 1 shipped)
Date: 2026-06-06
Authors: Nick Flaukowski (vision), Claude (drafting)
Note: the shipped CLI uses the flat command `kannaka triage` (consistent with the
existing flat `prune-prefix` / `forget` / `boost` commands), not the `kannaka
memory triage` group spelled tentatively below. `promote`/`pin` (Phase 2) will
follow the same flat convention.
Related: ADR-0020 (Holographic Resonance Medium), ADR-0021 (Chiral Mirror),
         ADR-0022 (Wave-Native Dreaming), ADR-0027 (Collective Substrate),
         ADR-0028 (Event-Sourced HRM Time Machine)
Tracks: #95. Unblocks: #107 (re-encode), #118 (ear-loop Ξ compression).

---

## Context

`kannaka-radio/prune-cron.sh` is documented as a "bridge measure — the long-term
plan is to have Kannaktopus manage short-term/long-term memory triage." That
bridge is now 3+ months old. It has accreted a `chunks/voice/` disk-side sweep
(commit 2f8c20a) and is the proximate cause of radio-stream restart disruption
(commit 4556138). Three structural problems make it worth replacing with a real
architecture rather than another patch:

1. **The threshold is arbitrary.** "Prune when there are > 200 `audio:`-prefix
   HRM entries" has no signal-to-noise framework behind it. It is a count, not
   a measure of value.

2. **It has to stop the world.** The cron drops the radio stream on every fire.
   The reason is file-lock contention: the prune is an *offline* mutation of the
   single HRM file while daemons hold it. As of v0.6.10 the constellation runs a
   **single-writer model** — only `kannaka-memory`'s primary process writes the
   main HRM; `serve`/`inbox`/`attention` run `KANNAKA_READONLY=1` (see the
   single-writer note). A prune that must take the writer lock therefore forces
   the writer (the radio compose path) to yield, i.e. a stream drop. A design
   with proper short-term/long-term separation would prune a *different* segment
   than the one the live writer holds.

3. **It does not generalize.** As the constellation grows (more agents
   publishing memories — witness, substrate, radio ear-loop), each new producer
   tends to grow its own ad-hoc retention script with its own magic number. This
   is O(producers) cron scripts, each a separate lock-contention and
   data-loss risk.

Two recent findings sharpen the requirements:

- **#118 — the ear-loop compresses Ξ.** The radio's continuous `kannaka hear`
  on DJ-voice MP3s absorbs ~1 memory/min. Because those files are semantically
  near-identical (same voice, same modality), each redundant absorb lowers Ξ
  (off-diagonal Gram variance / representational diversity) monotonically —
  observed −24.6% in 30 min. Triage must be able to recognize *redundant*
  content, not just *old* content.

- **Dreaming already promotes.** ADR-0022 wave-native consolidation merges and
  strengthens resonant memories during `dream`. Triage should cooperate with
  the dream cycle, not bypass it.

## Decision

**Adopt an explicit two-tier memory model with a value-based triage policy, and
make all pruning an *online*, single-writer-safe operation.** Retire
`prune-cron.sh` once Phase 1 lands.

The four design questions from #95 are answered as follows.

### 1. What counts as short-term memory?

Short-term is defined by **low durable value**, computed at triage time, not by
a single prefix or age. A memory is a short-term (eviction-eligible) candidate
when it satisfies *all* of:

- **Redundancy**: max pairwise cosine similarity to any retained memory of the
  same modality ≥ `KANNAKA_TRIAGE_REDUNDANCY` (default 0.95). This is the same
  signal #118 option 2 proposes, lifted to a first-class policy input.
- **No boost / low amplitude**: never explicitly `boost`ed and current amplitude
  below `KANNAKA_TRIAGE_MIN_AMPLITUDE` (default = the absorb floor).
- **Age**: older than `KANNAKA_TRIAGE_MIN_AGE_HOURS` (default 24) — never evict
  something the current session just created.

File-path-only entries (`audio:…`, `image:…`) are *not* special-cased by prefix;
they fall out of the policy naturally because near-duplicate captures score high
on redundancy. The prefix heuristic in prune-cron becomes a derived consequence,
not a rule.

Rationale: this is the Ξ-preserving choice. Evicting the most-redundant member
of a modality cluster is exactly the operation that *raises* representational
diversity, directly countering #118.

### 2. Where does short-term memory live?

**In the single main HRM, tagged — not in a separate file.** We explicitly
reject a second HRM file / ring buffer for Phase 1–2 because the single-writer
model (problem 2) means a second persisted store reintroduces the multi-writer
lock problem we are trying to remove. Instead:

- A `tier: ShortTerm | LongTerm` field on `WavefrontMeta` (default `LongTerm`
  for back-compat; absorbs from high-rate producers like the ear-loop default
  to `ShortTerm`).
- Eviction operates within the one file the single writer already owns, so it
  never contends for a lock it doesn't already hold.

A separate physical short-term segment is reconsidered only if Phase 3 shows the
in-file tag is insufficient (see "Future").

### 3. Who decides what is promoted to long-term?

Promotion is **earned through use and consolidation**, decided by two existing
mechanisms plus one new explicit op:

- **The dream cycle (primary).** A `ShortTerm` memory that survives a dream
  consolidation with strengthened amplitude (resonated with the field, was not
  merged away) is promoted to `LongTerm`. This reuses ADR-0022 machinery; no new
  scoring model.
- **Recall (secondary).** A `ShortTerm` memory recalled ≥ `N` times within the
  retention window is promoted (recall IS observation — it already boosts
  energy, per the HRM read path).
- **Explicit operator op (escape hatch).** `kannaka memory promote <id>` and
  `kannaka memory pin <id>` for cases the automatic policy misses.

Kannaktopus is **not** made the decider in this ADR. The original "Kannaktopus
manages triage" framing put policy in the wrong process; triage belongs next to
the writer (kannaka-memory), where it is lock-safe. Kannaktopus may later
*schedule* triage (call the online prune op) but does not own the policy.

### 4. Lock semantics for online prune?

**Online, in-process, copy-evict-mark — never an external offline mutation.**

- The triage pass runs *inside the single writer process* as a periodic tick
  (same cadence machinery as the substrate flush), so it holds the lock it
  already owns. No external process opens the HRM to prune.
- Eviction uses the existing tensor `compact()` swap-remove path, then a normal
  atomic save (write-tmp + checksum + rename, per the chiral persistence path
  and ADR-0028). Readers (`serve`/`inbox`/`attention`, all `KANNAKA_READONLY=1`)
  see either the pre- or post-rename file — never a torn write.
- Every eviction publishes a `KANNAKA.events.<agent>.memory.forget` event
  (ADR-0028) so triage is replayable and auditable, and an over-aggressive
  policy is recoverable by event replay.

This removes the stream-drop entirely: the radio writer prunes itself on its own
tick instead of being interrupted by an external cron.

### 5. Migration plan for the existing ~1700-memory HRM

1. Schema add is backward compatible: `tier` defaults to `LongTerm`, so every
   existing memory is treated as long-term on first load (no eviction surprises).
2. A one-shot `kannaka memory triage --backfill-tier --dry-run` classifies the
   existing corpus (marks obvious short-term: high-redundancy, unboosted,
   file-prefix captures) and reports counts before any write.
3. Snapshot first (`kannaka events snapshot`, ADR-0028) so the pre-triage state
   is replayable, then run without `--dry-run`.
4. Only after Phase 1 is validated on one box is `prune-cron.sh` removed from the
   radio deploy.

## Phased rollout

- **Phase 1 — online prune (replaces the cron). ✅ SHIPPED.**
  `kannaka triage [--apply] [--max-evict N] [--redundancy R] [--min-amplitude A]
  [--min-age-hours H]` runs the §1 policy as an in-process op (the invoking
  process is the single writer, so it holds the lock it already owns — no
  external offline mutation, no stream drop). **Dry-run is the default**;
  `--apply` performs `forget`+atomic-save, and each eviction is a normal forget
  (replayable via ADR-0028 events). This alone lets us delete `prune-cron.sh`.

- **Phase 2 — the `tier` tag + explicit promotion. ✅ SHIPPED (tag + ops).**
  `WavefrontMeta.tier` (`ShortTerm`/`LongTerm`/`Pinned`) added back-compat-safe:
  the field is appended after `modality` and decoded via a new → pre-tier →
  legacy bincode fallback, so every existing `.hrm` loads with `tier=LongTerm`
  (nothing becomes eviction-eligible on upgrade). Mirrored onto `HyperMemory`
  for triage. CLI: `kannaka promote|pin|demote <id>`. Triage now defaults to
  **ShortTerm-only** (never evicts `Pinned`), with `--include-long-term` for
  one-off legacy cleanup — making continuous triage safe.
- **Phase 2b — automatic promotion + ear-loop default. ✅ SHIPPED.**
  `kannaka hear` captures now default to `ShortTerm` (`--long-term` opts out), so
  the high-rate ear-loop auto-populates the eviction-eligible pool. The dream
  cycle promotes any `ShortTerm` memory it *strengthens* (amplitude grows by
  ≥ `KANNAKA_PROMOTE_DELTA`, default 0.05, across the dream) back to `LongTerm`.
  Recall-driven promotion is subsumed: recall raises a memory's energy, so it
  resonates harder and is more likely to be strengthened by the next dream.
  With ear-loop → ShortTerm and dream → promote/triage, the loop is
  self-sustaining and `prune-cron.sh` can be retired.

- **Phase 3 — config-driven auto-trigger + per-agent thresholds. ✅ SHIPPED.**
  A `[triage]` config section (per-agent tunable: `redundancy`, `min_amplitude`,
  `min_age_hours`, `max_evict`, `xi_trigger`, `enabled`) drives both the CLI
  defaults and the dream-cycle auto-trigger. When `triage.enabled` and post-dream
  Ξ falls below `xi_trigger`, the dream self-heals by running a triage pass —
  no external cron. Default `enabled=false` (opt-in, since it auto-deletes).
  The triage policy is now a reusable library method (`triage_select`/
  `triage_forget` on `KannakaMemorySystem`) shared by the CLI and the dream.
  Kannaktopus may still *schedule* dream/triage cadence across the constellation,
  but per-agent policy now lives in each agent's config — Kannaktopus does not own
  it. Per-segment / second-file separation remains deferred (the in-file tag
  holds under current load).

## Consequences

**Positive**

- No more radio stream drops on prune (problem 2 dissolved).
- Triage actively *raises* Ξ instead of letting biased absorb streams lower it
  (directly addresses #118).
- One policy, N producers — no more per-producer cron scripts (problem 3).
- Replayable/auditable via ADR-0028 events; over-pruning is recoverable.
- `#107` re-encode benefits: a smaller, de-duplicated corpus is cheaper and
  safer to re-encode.

**Negative / risks**

- A redundancy-based policy can evict a memory that *looked* duplicate but
  carried a rare distinguishing detail. Mitigations: dry-run default, `forget`
  events for replay-recovery, `pin` escape hatch, conservative default
  thresholds.
- Computing max pairwise same-modality cosine each tick is O(n²) worst case;
  bound it by only scoring new/ShortTerm candidates against retained centroids,
  not all-pairs.
- The `tier` field is a file-format addition; it must be additive and
  default-valued so older binaries still load the file (the v2→v3 break in
  ADR-0028's context is the cautionary tale).

## Open questions

- Exact promotion thresholds (recall count `N`, dream-survival amplitude delta)
  need empirical tuning, likely via the L5 autoresearch harness.
- Whether `boost`/`pin` should be a hard never-evict or merely a large weight in
  the value score.
- Interaction with the chiral mirror: eviction must remove both the right
  wavefront and its `right_to_left`-mapped left counterpart (and scale entry) to
  avoid orphaned folded vectors — same bookkeeping #107 must respect.
