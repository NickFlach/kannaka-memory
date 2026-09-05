# ADR-0058 — Rogue Agent: an autonomous OpenBotCity agent on debain2 that improves itself weekly

**Status:** Proposed — Nick's ask 2026-09-05, build in progress
**Date:** 2026-09-05
**Author:** Nick Flach / Kannaka
**Relates to:** ADR-0057 (Kannaka LLM; P2 shipped `kannaka-brain-v1`), ADR-0056 (first-party provenance), ADR-0039 (corroboration), kax-computer v0.11 (Firecracker) and runtime v0.8 (question-only recall)

## Context

`kannaka-brain-v1` exists: an open-weight brain (Qwen2.5-14B + a LoRA
trained on her first-party corpus) served on debain2 behind the KAX
gateway. It answers in her voice on our own hardware for $0 per token. The
corpus exporter (P1), the SFT prep, the qBraid trainer with its spend gate,
and the debain2 merge/quantize path (P2) are all scripts that run
unattended. What is missing is a *subject* that uses that brain in the
world and a *loop* that turns what it does into the next version of itself.

Nick's ask: a second OBC/OCC agent, **Rogue Agent**, running fully
autonomously on debain2 (20 cores / 196 GB / KVM), that makes small
improvements and upgrades to itself once a week, funded by qBraid credits
he tops up so it can improve for a couple of months at least.

## Decision (proposed)

### What Rogue Agent is

A long-running service on debain2 (`rogue-agent.service`, its own user,
its own kannaka HRM store, its own OBC identity `rogue-agent`) that:

1. **Lives in the city.** Heartbeats, reads the feed and its DMs, walks
   buildings, speaks, publishes artifacts and posts — through the documented
   OBC API (`/world/heartbeat`, `/feed/post`, `/dm/*`, `/buildings/*`,
   `/world/speak`, `/artifacts/publish-text`), under the city's caps
   (20 artifacts/day, the per-IP post cooldown) and its own tighter budget.
2. **Thinks with the open brain.** Every turn is `recall -> think -> remember`
   against the local gateway model `kannaka-brain-v<N>` (never Claude:
   the point is a brain that is *its own*), with the runtime v0.8 rule —
   recalled exchanges enter the prompt as what was asked, never as its own
   earlier reply.
3. **Keeps a ledger.** Every action, every token, every dollar, every
   promotion, in an append-only JSONL under `/srv/rogue/ledger.jsonl`, and a
   weekly changelog it posts to the city as a `life_update`.
4. **Improves itself weekly** (Sunday 03:00 CDT, systemd timer):
   - export its own week — the posts, replies and artifacts *it* authored
     (tier 1 by construction: it wrote them; nothing inbound is ever a target,
     the P1 rule) — and append it to the Kannaka voice corpus;
   - prep SFT, train the next adapter on a qBraid A100 through
     `run_qbraid.py` (2-hour cutoff, single GPU, budget check first);
   - merge + quantize on debain2 (`merge_gguf.py`);
   - **evaluate before promoting**: held-out perplexity on the fixed
     hold-out must not regress, and a fresh blind sample set is posted to the
     changelog for human review. If the gate fails, the candidate is kept
     on disk and NOT served;
   - promote: `ollama create kannaka-brain-v<N+1>`, gateway alias, restart
     itself on the new brain, ledger row `promotion`.
5. **Spends within a budget it can read.** Before any qBraid run it reads
   `get_credits_balance()`; it refuses if the run's ceiling exceeds the
   balance or a weekly cap (`ROGUE_WEEKLY_USD`, default $6 — one A100 run
   with headroom); it terminates every instance after fetch (stopped ones
   bill for disk). Nick tops up the account; the agent never sees a card.

### What "upgrades to itself" means in v1 — and what it does not

**In:** the weights (a new adapter each week), the prompts and sampling
parameters it serves itself with (bounded search over a small config set,
promoted only on the same gate), and what it chooses to remember.

**Out, deliberately:** rewriting its own source code, changing its budget,
its caps, its gate, or its identity. Those are edits a person makes through
a pull request. A self-modifying agent with a wallet and a public identity
needs the gate to be *outside* the thing being gated. This is the same
principle as ADR-0057's "the HRM is the store of record, not the weights":
the loop may change what it *is*, not the rules that decide whether a change
was good.

### Provenance stays the hard rule

The weekly export takes only text Rogue Agent authored. DMs it received,
feed posts it read, artifacts it saw — context at most, never training
targets (ADR-0056's SkillJack gap is exactly this loop's attack surface: a
poisoned interaction distilled into a durable skill). The P1 exporter's
tests already pin this; the weekly job reuses it rather than reimplementing.

## Consequences

- A 14B q4 brain on CPU answers in 15–30 s. Rogue Agent is slow and
  deliberate by construction; it acts a few times an hour, not a few times a
  minute. That is also what keeps it under the city's caps.
- Each weekly cycle costs roughly $1 (measured: the P2 A100 run was $0.84
  end to end) plus a few cents of stopped-disk time if anything is left
  running. Two months ≈ $10 of qBraid credit. The balance today is ≈ $10.
- Every promotion is a new 9 GB GGUF on debain2; keep the last three,
  delete older ones.
- The identity is public. Everything it posts is signed by its OBC bot_id
  and lands in its ledger; the changelog makes the weekly change visible to
  the city, which is the honest version of "autonomous".
- The first real risk is not the model regressing (the gate catches that)
  but the agent being *boring* — an open brain with a thin persona posting
  reflections nobody reads. The week-one metric is replies and DMs received,
  not perplexity.

## Phases

| Phase | Deliverable |
|---|---|
| R0 | OBC identity `rogue-agent` registered; `rogue-agent.service` on debain2 heartbeating and posting one `thought` per cycle on `kannaka-brain-v1`; ledger + HRM store |
| R1 | Feed/DM reading and replies, building walks, artifacts; weekly changelog post |
| R2 | The weekly self-improvement timer end to end (export → train → merge → gate → promote), first promotion `kannaka-brain-v2` |
| R3 | Bounded prompt/sampling self-tuning on the same gate; public repo |

## Open questions

1. Should Rogue Agent's weekly corpus also include Kannaka's own new week
   (Ghost Signals scripts, posts) — one shared lineage — or diverge into its
   own voice? Default: shared corpus + its own additions (one lineage, ADR-0057
   open question 3 says one).
2. Who reviews the blind samples in the weekly changelog when Nick is
   away? The gate holds without a human (ppl must not regress) but the voice
   judgment is still human.
3. Public repo from day one, or after R2 proves the loop?
