# ADR-0050: Temporal / Confirmation-Weighted Recall — Un-Collapsing Resonance and Recency

**Status:** Accepted — mechanism **measured and supported** (L8, 2026-07-26),
shipping **OFF by default**. The half that handles contradiction is **blocked on a
writer**, not on ranking (see *Consequences*).
**Relates to:** ADR-0035 Cap 8 (the `effective_at` / `observed_at` / `expires_at`
temporal-truth fields this ADR is the first ranking consumer of), ADR-0048
(`KANNAKA_RECALL_ENERGY_EXP` — same shape, same off-by-default discipline),
ADR-0049 (facet encoding — reach; this ADR is ordering, and only ever re-orders a
pool that encoding made reachable).
**Evidence:** `experiments/results-L8.tsv`,
`experiments/notes/2026-07-26-L8-temporal-confirmation-recall.md`.

## Context

The question came from outside the ladder — a reply to the Product Hunt
announcement on Mastodon (@aiappsapi, 2026-07-26):

> "does recall strength depend on how recently a memory was confirmed, or only on
> how strongly it resonates? Those two come apart quickly once stored facts start
> contradicting each other."

The answer was **resonance only**, and the code was unambiguous about it:

- `Hemisphere::resonate_with_energy_exp` ranks `similarity × energy^e` and reads
  **no timestamp at all**.
- `energy` is pumped by **access**, not confirmation — the rich-get-richer bias
  ADR-0048 measured.
- The ADR-0035 temporal fields are persisted but read **only** as a post-recall
  filter in `swarm brief` (`bin/kannaka.rs:3622`), never in ranking.
- The `~693-day` amplitude decay (`medium/core.rs:54`) is on the FLAT path; the
  default CHIRAL path is age-blind.

So two axes that come apart under contradiction were collapsed into one. A
superseded fact and the fact that replaced it are *near-identical text* — they land
side by side in the candidate pool, and similarity separates them arbitrarily.

## Decision

Add a third factor to the ranking product, off by default:

```
resonance = similarity × energy^energy_exp × tweight^temporal_exp
```

`hemisphere::temporal_weight(meta, now, half_life_days, floor)` folds two things:

1. **Truth status** — past `expires_at` (superseded) or before `effective_at`
   collapses to a **floor**, never to zero.
2. **Confirmation recency** — a currently-true fact decays `0.5^(age/half_life)`
   from `observed_at`, falling back to `created_at`, clamped at the same floor.

Flags: `KANNAKA_RECALL_TEMPORAL_EXP` (**default 0.0 = off, byte-identical**),
`KANNAKA_RECALL_TEMPORAL_HALFLIFE_DAYS` (180),
`KANNAKA_RECALL_TEMPORAL_FLOOR` (0.25).

`resonate_with_energy_exp` now delegates to a new `resonate_with_weights` with
`temporal_exp = 0.0`, so every existing caller and test is unchanged by construction.

**Applied at BOTH scoring points** — the full-corpus pass and the xi re-rank — so it
governs which candidates *enter* the `2k` pool. This is the ADR-0048 review's lesson
carried forward: post-fetch re-ranking cannot promote what was never fetched.

**The floor is never zero.** Discounting a superseded fact is the point; making it
unretrievable is amnesia with a flag. An agent has to be able to answer "what did we
use *before*".

## Measurement (L8)

New research level `--level 8`. The fixture is built so the two axes *must* come
apart: fact families are near-identical sentences differing in one value token, all
answering the same cue, where version *v* `expires_at` exactly when version *v+1*
was `observed_at`. Similarity cannot separate them; only time can. 10 corpus seeds,
each arm measured **paired** against its own baseline on the same corpus.

Gates were pre-registered: P1 current-fact retrieval improves (> 0.5), P2
uncontradicted facts not degraded (≥ 0.95), P3 superseded facts stay reachable
(≥ 0.99), P4 instrument-live + default-byte-identical (hard PASS/FAIL, not folded
into fitness — a dead mechanism must fail loudly rather than score well).

At `exp = 0.25, floor = 0.25, half_life = 180d`: current-fact MRR **0.4774 → 0.8503**
(Δ **+0.3729**, 71% of available headroom), P2 = 0.9667, P3 = **1.0000**, fitness
0.151 vs 0.500 baseline. Revert-and-confirm-fail holds — at `exp = 0.0`, P1 is
exactly 0.0000.

Two findings outrank the win itself:

**The floor is the amnesia knob.** Floor 0.05 evicts the past (P3 0.9399, fails the
gate); the safe plateau is 0.15–0.50 (P3 = 1.0000).

**Freshness, not supersession, is most of the win.** Across a **120× half-life
range** (30d → 3650d) P1 moves only 0.699 → 0.743. At 3650d the recency term is
effectively inert and the supersession floor *alone* still delivers P1 = 0.7425 — but
P3 collapses to 0.9356. Both sub-mechanisms independently produce the win and differ
entirely in what they cost: freshness-promotion is nearly free, supersession-demotion
is what risks the past. Keep the floor generous; let recency do the work.

## Consequences

**The blocker moved from ranking to writing.** Code-verified: outside tests and the
L8 harness, the only writer of the three temporal fields is `HrmStore::set_temporal`,
reachable from exactly one place — `kannaka remember --effective/--observed/--expires`
(`bin/kannaka.rs:1557`). Nothing auto-populates them. On today's live HRM:

- `observed_at` is `None` everywhere → recency falls back to `created_at` and **still
  works**, as age-since-storage.
- `expires_at` is `None` everywhere → **the supersession half is completely inert**.

Enabling the flag on the live corpus today therefore buys age-decay ranking, not
contradiction handling. That is the honest scope of what this ADR ships.

**Follow-up, and it is the harder half:** something has to write supersession down.
Deciding that a new fact replaces an old one is a judgement, not a timestamp — it
needs its own design (and its own adversarial review) covering what counts as a
contradiction, what happens on partial disagreement, and who is allowed to expire a
memory. Single-writer discipline applies: only the writer may stamp; readers rank.

**Not addressed here:** auto-bumping `observed_at` on re-assertion (the
"confirmation" the original question named), inference of supersession, and any
interaction with dream consolidation, which may merge versions before ranking ever
sees them.

**Risk accepted:** the mechanism is validated on a synthetic fixture with clean,
single-attribute contradictions and disjoint vocabulary. Real contradictions are
messier and partial. This ships off by default, and the L8 harness stays in the
ladder so any future change to the ranking path is re-measured against the same
pre-registered gates.
