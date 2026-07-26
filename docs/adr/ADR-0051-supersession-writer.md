# ADR-0051: The Supersession Writer — Recording That One Fact Replaced Another

**Status:** Proposed — **pending adversarial-design-review before any build.**
**Relates to:** ADR-0050 (temporal/confirmation-weighted recall — this ADR unblocks
its inert half), ADR-0049 (facet encoding — the dependency that makes detection
tractable), ADR-0035 Cap 8 (the fields being written), ADR-0036 (dream
consolidation — the conflict), `sensemaking::detect_contradictions` (Cap 10 — the
existing detector, measured here and rejected for this job).

## Context

ADR-0050 shipped temporal ranking and closed the ranking question: a superseded fact
can now be demoted without being evicted, measured, off by default. It also moved the
blocker rather than removing it. Code-verified there: outside tests, the only writer
of `effective_at` / `observed_at` / `expires_at` is `HrmStore::set_temporal`, reachable
from exactly one place — `kannaka remember --effective/--observed/--expires`. Nothing
auto-populates them. On the live HRM `expires_at` is `None` everywhere, so **the
supersession half of ADR-0050 is completely inert.**

Something has to write supersession down. That is this ADR.

## The obvious reuse does not work, and we measured it

`sensemaking::detect_contradictions` (Cap 10) already finds "same subject, opposed
stance": pairs with `sim(a,b) >= threshold` AND **phase gap near π**. It is used by
`immune::classify_batch`. Reusing it was the first candidate.

It cannot do this job, and the reason is structural. `content_born_phase` is
deliberately content-**smooth** — its own doc comment says it maps "identical content
to identical phase", specifically so belief-cores cluster instead of scattering like a
hash. A superseded fact differs from its replacement by a *single value token*. **The
pair that most needs detecting is the pair that looks most alike.**

Probe (`chiral::tests::supersession_pairs_are_not_phase_opposed`, kept as a
regression test):

| pair | phase gap |
|---|---|
| `"...channel is twelve"` vs `"...channel is twentyseven"` (supersession) | **0.6341 rad** |
| `"...channel is twelve"` vs `"mycelium spreads beneath the forest floor"` (unrelated) | **0.5415 rad** |
| opposed-stance threshold (`π/2`) | 1.5708 rad |

Two findings, and the second is the stronger one:

1. A supersession pair is nowhere near phase-opposed (0.63 ≪ 1.57), so
   `detect_contradictions` will **never** flag it.
2. **Phase does not even order these correctly.** The supersession pair has a *larger*
   phase gap than a completely unrelated pair. Phase is not merely insufficient here —
   it is non-monotonic with respect to the distinction we need, so **no threshold on
   phase gap can work**, tuned or otherwise.

This is the same shape as the ADR-0047 refutation (phase is content-born and unread by
recall). Phase keeps being the intuitive answer and keeps not being the answer.

## Decision

Supersession is written by a **staged writer**, where each stage must earn the next by
measurement. Detection rides on **facets**, not phase, and not on whole compound
memories.

**Stage 0 — explicit declaration. No judgement, ships first.**
`kannaka remember "<text>" --supersedes <id>` stamps `expires_at = now` on the named
memory and `observed_at = now` on the new one, through the existing
`HrmStore::set_temporal`. Zero inference, zero risk of a wrong call, and it makes
ADR-0050's supersession half genuinely usable today for any caller that already knows
the relationship (the swarm, the nostr membrane, and any tool with a source of truth
do). This is the whole of what ships without further review.

**Stage 1 — propose, do not apply.**
A detector emits supersession *candidates* to an observation channel and writes
nothing. Candidate signal, keyed on ADR-0049 facets rather than compound memories:

> two facets whose **subject+attribute** resonate above a high threshold but whose
> **value** differs, where the newer facet's `observed_at` is later.

Facets are the right unit precisely because the encoder arc proved compound memories
smear — you cannot reliably read "same subject, different value" out of a blurred
wavefront. **This makes ADR-0051 depend on ADR-0049 being built.** Stage 1 cannot be
implemented before facets exist; attempting it on compound memories is the mistake
this ADR exists to avoid.

**Stage 2 — auto-apply, behind an off-by-default flag, only if Stage 1 measures well.**
Same discipline as ADR-0050 and ADR-0048: a flag, a default that is byte-identical to
not having the feature, and pre-registered gates.

## Measurement (proposed L9)

A new research level, gates registered before running, following L8's shape (paired
arms, ≥10 corpus seeds, hard liveness gate not folded into fitness):

- **G1 precision** — of the pairs the detector calls supersession, what fraction truly
  are? This is the gate that matters. A false positive expires a *true* memory, which
  is the one genuinely harmful outcome here.
- **G2 recall** — of the true supersessions, what fraction are found? Deliberately
  weighted *below* precision: missing a supersession leaves today's behaviour, calling
  a false one degrades a true fact.
- **G3 negative control** — near-duplicate but *compatible* facts (elaborations,
  restatements, partial overlap) must NOT be called supersessions. This is where a
  naive similarity threshold will fail, and it must be measured, not assumed.
- **G4 non-contradiction control** — a corpus with no contradictions must produce zero
  candidates. Zero, not "few".

L8's lesson applies directly and is not optional: **stamp the whole fixture.** L8's
first run failed every gate because unstamped distractors were silently dated to
`now`. Any L9 fixture must set timestamps on every memory it creates, including
controls.

## Consequences and open risks

**Dream consolidation may destroy the evidence first.** Two versions of a fact are
near-identical, which makes them prime candidates for ADR-0036 resonance-merge. If
dream merges them before supersession is recorded, the contradiction is not resolved —
it is *blurred into one wavefront*, which is the worst outcome available: a memory that
asserts both values weakly. This interaction must be settled before Stage 2, and it may
force facets to be merge-exempt (ADR-0049 already exempts them from merge grouping for
a different reason — that exemption may be load-bearing here too).

**Single-writer discipline is binding.** Only the writer may stamp. `attention serve`
and every other daemon run `KANNAKA_READONLY=1`, where `save_medium` early-returns —
a detector running there would compute candidates that silently never persist, which
is exactly the ADR-0047 review's finding about `reinforce_link`. Stage 1's detector
therefore *emits* candidates; only the single writer consumes them.

**Wrongly expiring a true memory is recoverable but not free.** ADR-0050's floor is
non-zero, so a wrongly-expired memory is demoted, not deleted, and `set_temporal` can
clear the stamp. That is the safety margin that makes Stage 2 thinkable at all — but
it argues for a conservative detector and for keeping the floor generous, which is
also what L8's half-life sweep independently concluded (freshness-promotion is nearly
free; supersession-demotion is what costs).

**Not addressed here:** auto-refreshing `observed_at` when a fact is merely re-asserted
without contradiction (the "confirmation" half of the original question — cheap and
probably worth doing before any of this), partial/graded disagreement, and who is
allowed to expire a memory that arrived over the swarm rather than locally.
