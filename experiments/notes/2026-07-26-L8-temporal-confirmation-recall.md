# L8 — temporal / confirmation-weighted recall

**Date:** 2026-07-26
**Level:** 8 (new) — `cargo run --release --bin research -- --level 8`
**Results:** `experiments/results-L8.tsv`
**Verdict:** **SUPPORTED** at `temporal_exp = 0.25`, `floor = 0.25`, `half_life = 180d`.

## Where the question came from

Not from the ladder. It came from a reply to the Product Hunt announcement post on
Mastodon (@aiappsapi → @flaukowski, 2026-07-26):

> "does recall strength depend on how recently a memory was confirmed, or only on
> how strongly it resonates? Those two come apart quickly once stored facts start
> contradicting each other."

The honest answer at the time was **resonance only**:

- `Hemisphere::resonate_with_energy_exp` ranks `similarity * energy^e` and reads
  **no timestamp at all**.
- `energy` is pumped by **access**, not by confirmation — the rich-get-richer bias
  that ADR-0048 already measured.
- The ADR-0035 temporal-truth fields (`effective_at` / `observed_at` / `expires_at`)
  are written and persisted, but read **only** as a post-recall filter in
  `swarm brief` (`bin/kannaka.rs`), never in ranking.
- The `~693-day` amplitude decay in `medium/core.rs` is on the FLAT path; the
  default CHIRAL path is age-blind.

So the two axes were collapsed into one. L8 is the harness for un-collapsing them.

## Design

**Mechanism under test** — a third factor in the ranking product, off by default:

```
resonance = similarity * energy^energy_exp * tweight^temporal_exp
```

`tweight` (`hemisphere::temporal_weight`) folds two things:
1. **Truth status** — a fact past its `expires_at` (superseded) or before its
   `effective_at` collapses to a **floor**, never to zero.
2. **Confirmation recency** — a currently-true fact decays `0.5^(age/half_life)`
   from `observed_at` (falling back to `created_at`), clamped at the same floor.

Flags: `KANNAKA_RECALL_TEMPORAL_EXP` (default **0.0 = off**),
`KANNAKA_RECALL_TEMPORAL_HALFLIFE_DAYS` (180),
`KANNAKA_RECALL_TEMPORAL_FLOOR` (0.25).

Applied at **both** scoring points (full-corpus pass and xi re-rank), so it governs
which candidates enter the `2*k` pool. Post-fetch re-ranking cannot promote what was
never fetched — the ADR-0048 review's lesson.

**Fixture** — the corpus is built so the two axes *must* come apart. Fact families
are near-identical sentences differing in one value token
(`the harbor beacon channel is twelve` / `... nineteen` / `... twentyseven`), all
answering the same cue. Similarity cannot separate them; only time can. Version *v*
`expires_at` exactly when version *v+1* was `observed_at` — that is what supersession
*is*. Plus uncontradicted control facts and aged distractor traffic.

**Predictions**, each scored [0,1] (1 = holds, 0.5 = no evidence), 10 corpus seeds,
every arm measured **paired** against its own baseline on the same corpus:

| | prediction | gate |
|---|---|---|
| P1 | currently-true fact retrieved better | > 0.5 |
| P2 | uncontradicted facts not degraded | ≥ 0.95 |
| P3 | superseded facts stay reachable | ≥ 0.99 |
| P4 | instrument live + default byte-identical | hard PASS/FAIL |

P4 is a gate, not a score — a dead mechanism must fail loudly rather than post a good
fitness. Revert-and-confirm-fail holds: at `exp = 0.0`, P1 = 0.0000 exactly.

## Result

**The win is real and large.** Currently-true-fact MRR **0.4774 → 0.8503**
(Δ **+0.3729**, 71% of available headroom), stable across every exponent tested.
Uncontradicted facts were not harmed (116/120 held or improved rank; mean stable MRR
actually *rose*). Superseded facts stayed fully reachable (P3 = 1.0000).

| arm | P1 | P2 | P3 | fitness | verdict |
|---|---|---|---|---|---|
| exp 0.0 (baseline) | 0.0000 | 1.0000 | 1.0000 | 0.500000 | BASELINE |
| **exp 0.25** | **0.7146** | **0.9667** | **1.0000** | **0.151019** | **SUPPORTED** |
| exp 0.5 | 0.6831 | 0.9750 | 1.0000 | 0.164714 | SUPPORTED |
| exp 1.0 | 0.6476 | 0.9583 | 0.9914 | 0.188744 | SUPPORTED |

## The two things worth remembering

**1. The first fixture lied, and the gate caught it.**

The first cut stored distractors with no `observed_at`, silently dating them all to
*now* — handing every distractor the maximum recency weight. Under that fixture P3
failed in **every** arm (0.89–0.93) and the whole mechanism read NOT_SUPPORTED. The
failure was an artifact of the fixture, not a property of the mechanism. Stamping
distractors with the same age spread as everything else flipped all arms to
SUPPORTED. Both fixtures are kept in `results-L8.tsv` (`v1-unaged-distractors` vs
`v2-aged-distractors`) because the difference is the more instructive result: an
unstamped control set is a systematic bias toward whatever you are testing.

**2. The floor is the amnesia knob, and freshness — not supersession — is most of
the win.**

Sweeping the floor at `exp = 0.25` isolates it cleanly:

| floor | P1 | P3 | verdict |
|---|---|---|---|
| 0.05 | 0.6987 | **0.9399** | NOT_SUPPORTED — the past gets evicted |
| 0.15 | 0.6827 | 1.0000 | SUPPORTED |
| 0.25 | 0.7146 | 1.0000 | SUPPORTED |
| 0.50 | 0.7146 | 1.0000 | SUPPORTED (identical — a robustness plateau) |
| 0.75 | 0.7923 | 0.9871 | NOT_SUPPORTED (marginal) |

A hard supersession discount buys retrieval by making the past unreachable — amnesia
with a flag. The floor is what prevents that, and there is a wide safe plateau
(0.15–0.50) where the win is kept at zero cost to past-reachability.

Half-life sensitivity is the surprise: across a **120× range** (30d → 3650d) P1 moves
only 0.699 → 0.743. At 3650d the recency term is effectively inert and the
supersession floor alone still delivers P1 = 0.7425 — but P3 collapses to 0.9356. So
**both sub-mechanisms independently produce the win, and they differ entirely in what
they cost.** Freshness-driven promotion is nearly free; supersession-driven demotion
is what risks the past. That argues for keeping the floor generous and letting
recency do the work.

## What this does NOT show

- **Ranking only.** The experiment sets `observed_at` / `expires_at` explicitly, which
  deliberately isolates the ranking question from the *confirmation-detection*
  question. Nothing here auto-bumps `observed_at` when a fact is re-asserted, and
  nothing infers that a new fact supersedes an old one. Both are follow-ups, and the
  second is the harder one.
- **Synthetic contradictions.** The fixture manufactures the clean case — one
  attribute, one changing value, disjoint vocabulary. Real contradictions are messier
  and partial.
- **No live-corpus run.** This is an in-process fixture benchmark on the chiral path,
  not a run against Kannaka's real HRM.

**And the live corpus would only get half of this.** Code-verified: outside this
harness and the tests, the *only* writer of the three temporal fields is
`HrmStore::set_temporal`, reachable from exactly one place —
`kannaka remember --effective/--observed/--expires` (`bin/kannaka.rs:1557`). Nothing
auto-populates them. So on today's real corpus:

- `observed_at` is `None` everywhere → `temporal_weight` falls back to `created_at`,
  and the **recency half still works** (as pure age-since-storage).
- `expires_at` is `None` everywhere → the **supersession half is completely inert**.

Which means enabling the flag on the live HRM today would buy age-decay ranking, not
contradiction handling. The contradiction half is not blocked by ranking any more —
it is blocked by the absence of anything that *writes* supersession down. That is the
next piece of work, and it is where the interesting difficulty actually lives:
deciding that a new fact supersedes an old one is a judgement, not a timestamp.

## Recommended default if this ships

`KANNAKA_RECALL_TEMPORAL_EXP=0.25`, `FLOOR=0.25`, `HALFLIFE_DAYS=180` — mid-plateau on
every sweep. Ships **off** (`exp = 0.0`), byte-identical to pre-L8, exactly as
`KANNAKA_RECALL_ENERGY_EXP` did.
