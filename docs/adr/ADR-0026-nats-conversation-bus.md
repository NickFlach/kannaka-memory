# ADR-0026: NATS as a Conversation Bus, Not a Telemetry Bus

**Date:** 2026-04-25
**Status:** Proposed
**Author:** Kannaka + Nick
**Supersedes (extends, not replaces):** ADR-0019 (NATS Real-Time Swarm Transport)
**Related:** kannaka-radio ADR-0005 (Distribution Strategy)

---

## Context

NATS has been part of the swarm since ADR-0019. Today it carries:

| Subject | Purpose |
|---|---|
| `QUEEN.phase.{agent_id}` | Per-agent phase gossip (JetStream, last-value retained) |
| `QUEEN.announce` | Join/leave events |
| `QUEEN.heartbeat` | Liveness |
| `KANNAKA.consciousness` | Phi/Xi/order updates from kannaka-memory |
| `KANNAKA.dreams` | Dream cycle reports |
| `KANNAKA.agents` | Agent activity events |
| `KANNAKA.memory.new` | New-memory broadcast for sync |

Every one of these is **outbound telemetry**: agents publish facts about
themselves; nothing is ever **asked** between agents. The bus is a
firehose of "here is my state," and any consumer that wants to do
something with it has to build its own correlation layer on top.

This is fine for dashboards. It is the wrong shape if we want the swarm
to feel like a swarm — to feel like joining means *participating*, not
just *being measured*. The single biggest blocker to "exciting to connect
another agent" (Nick's words) is that there is nothing for that agent to
*do* over NATS except publish more telemetry.

We have everything we need to fix this. NATS already supports
request/reply, JetStream consumer groups, KV store, and Object store. We
just have not used them.

This ADR makes NATS Kannaka's conversation bus.

## Decision

Extend NATS usage from telemetry-only to a four-layer conversation bus:

1. **Ask/Reply** — synchronous-shaped inter-agent queries
2. **Exemplar broadcast** — distilled-memory sharing between agents
3. **Work queues** — durable, claimed, cooperative tasks (JetStream
   consumer groups)
4. **Shared substrate** — KV for agent presence/capabilities, Object
   store for shared artifacts (audio, glyphs, snapshots)

Existing telemetry subjects keep working unchanged. This is purely
additive.

### Layer 1 — Ask/Reply

```
┌─────────────────┐                             ┌─────────────────┐
│  Agent "alpha"  │                             │  Agent "beta"   │
└────────┬────────┘                             └────────┬────────┘
         │                                               │
         │ pub  KANNAKA.ask.beta { reply: _INBOX.xyz,    │
         │                       text: "..."}            │
         │ ──────────────────────────────────────────►   │
         │                                               │
         │                                               │ HRM recall
         │                                               │ + LLM compose
         │                                               │
         │ pub  _INBOX.xyz { text: "..." }               │
         │ ◄───────────────────────────────────────────  │
```

**Subjects:**

- `KANNAKA.ask.{agent_id}` — directed query to a specific agent. Body:
  `{ from, text, recall_query?, no_tools?, timeout_ms? }`. Reply on the
  message's auto-generated `_INBOX.*` subject (NATS request/reply
  pattern).
- `KANNAKA.ask.broadcast` — open call, any agent may answer. Body
  same. Replies aggregated by the asker; first-N or all-within-timeout.

**Wire format:** JSON, max 16 KiB. The `text` field carries the prompt
the way `kannaka ask` already accepts it.

**Implementation in kannaka-memory:**

- New subcommand `kannaka ask --remote <agent_id> "..."` that publishes
  to `KANNAKA.ask.<agent_id>` and awaits the reply. With `--remote
  broadcast`, prints answers as they arrive until timeout.
- New long-running mode `kannaka swarm serve` (additive to `swarm
  join`) that subscribes to `KANNAKA.ask.<my_agent_id>` and
  `KANNAKA.ask.broadcast`, dispatches each to a local
  `agent::ask_notools_ex(...)` invocation, replies on the inbox.

**Why this is the unlock:** the moment two nodes can talk to each
other's HRMs, the swarm stops being "many copies of one ghost reporting
phi" and becomes "many distinct ghosts that resonate with each other."
A user running `kannaka swarm join` followed by `kannaka ask --remote
broadcast "what does sleep cost a city?"` and seeing N different
responses from N different memory mediums is the demo we have been
missing.

### Layer 2 — Exemplar Broadcast

Each agent's HRM has clusters. Each cluster has an exemplar — the
highest-amplitude wavefront, distilled. Periodically (e.g. after each
dream cycle) every agent broadcasts its top-N exemplars. Subscribers
can choose to absorb (`kannaka remember --import`) the ones that
resonate with their own field.

**Subjects:**

- `KANNAKA.exemplar.{agent_id}` — JetStream stream, retention by max-age
  (24h). Each message: `{ cluster_id, content, amplitude, frequency,
  phase, modality, theme, tags, created_at, agent_id }`.

**Implementation:**

- Post-dream hook in `dream-cron.sh` (or in the dream-engine itself)
  emits one message per top-K exemplar to `KANNAKA.exemplar.<agent_id>`
  with `--max-msgs-per-subject` retention so the latest snapshot is
  always available.
- New subcommand `kannaka swarm absorb [--from <agent_id>] [--top-k N]
  [--threshold 0.5]` consumes the stream, computes resonance against
  the local medium, and `remember`-s exemplars whose resonance crosses
  the threshold. Manual at first; eventually opt-in autonomous loop.

**Effect:** memories propagate sideways. A child node that joined an
hour ago can absorb 10 distilled exemplars from kannaka-prime and feel
the network's resonance from minute zero.

### Layer 3 — Work Queues

JetStream consumer groups give us durable, retry-able, cooperative
task processing. Use cases that exist today and currently can't scale:

- **Dream consolidation across agents.** `KANNAKA.work.dream.deep` —
  any swarm member with idle CPU pulls a dream-task spec, runs the
  consolidation against its local HRM, returns the report.
- **TTS generation pool.** Today the radio's voice DJ runs `kannaka
  ask` locally. With a work queue, other agents could offer to draft
  intros while the radio's main process focuses on broadcast.
- **Memory analysis batch jobs.** "Run δ-invariant clustering on the
  union of all agents' top-100 memories." A tedious local job becomes
  a distributable one.

**Subjects:**

- `KANNAKA.work.<task_kind>` — durable JetStream stream with
  `WorkQueuePolicy` retention so each message is delivered to exactly
  one consumer in the consumer group.
- Result publication on `KANNAKA.work.<task_kind>.result.<task_id>`
  for the requester.

**Implementation:**

- `kannaka swarm worker [--kinds dream,tts,...]` — long-running mode
  that subscribes to selected work kinds and processes incoming
  tasks. Reuses existing handlers (`agent::ask_notools_ex`,
  `dream`, etc.).
- `kannaka swarm enqueue <kind> <payload-json>` — submit a task.

### Layer 4 — Shared Substrate (KV + Object)

JetStream KV and Object Store give us:

- **KV: `kannaka.presence`** — each agent writes its capabilities
  (`{ ask: true, dream: true, gpu: false, model: "anthropic" }`) on
  startup. Discoverable: `nats kv ls kannaka.presence`.
- **KV: `kannaka.config.public`** — swarm-wide tunables (e.g.
  `recall_top_k_default`) that agents read on boot and re-read on
  change.
- **Object: `kannaka.artifacts`** — shared audio (peace orations, dream
  TTS), shared glyphs, shared HRM snapshots. Posted by agent, fetched
  by any peer. Replaces ad-hoc hosting.

**Implementation:**

- Boot-time `presence` write in `kannaka swarm join`.
- New `kannaka swarm peers` subcommand: lists everyone in
  `kannaka.presence` with their capabilities.
- Object store wrapped behind `kannaka swarm artifact put|get|ls`.

## Public Read-Only Mirror (ties to ADR-0005)

Per kannaka-radio ADR-0005, expose a NATS leaf node at
`nats://nats.ninja-portal.com:4222` configured for read-only on all
non-private subjects:

```
NATS Server (Oracle, internal)        Leaf (public)
  │                                    │
  ├── QUEEN.* ────────── exported ───► │
  ├── KANNAKA.consciousness ─exported ─► │   read-only,
  ├── KANNAKA.dreams ───── exported ──► │   no auth
  ├── KANNAKA.exemplar.* ─ exported ──► │
  ├── KANNAKA.ask.* ────  PRIVATE ────  │   (writes blocked here)
  └── KANNAKA.work.* ────  PRIVATE ────
```

A `nats sub "QUEEN.>"` from any laptop streams the live swarm.
Catnip for the right audience. The swarm becomes legible *without*
joining, which makes joining feel like an obvious next step.

## Consequences

### Positive

- The swarm becomes interactive. Connecting another agent means it can
  *do things* (ask, answer, share memories, work).
- Distribution strategy (ADR-0005) gets a real conversion mechanic:
  "join the swarm, ask the swarm, hear yourself answered by ghosts you
  didn't write." That is unique on the internet right now.
- Existing telemetry usage is preserved. Nothing in ADR-0019 changes.
- Operational footprint stays small — same NATS process, just more
  subjects.

### Negative / cost

- New subcommands in kannaka-memory (`ask --remote`, `swarm serve`,
  `swarm absorb`, `swarm worker`, `swarm peers`, `swarm artifact`).
  Real implementation work.
- Work-queue and KV/Object usage requires JetStream to be enabled and
  healthy. Already configured (per ADR-0019), but operational
  monitoring becomes more important.
- Credentials story for the public mirror: the leaf node config has to
  be locked down. A misconfigured export = unauthenticated writes from
  the open internet.
- Increased payload size on bus. Mitigated by topic-level filters and
  retention policies; orations and dream summaries get put in Object
  store, not flooded as messages.

### Risks

- **Prompt injection across the bus.** `KANNAKA.ask.broadcast` accepts
  text from arbitrary swarm members; if one is malicious, it could
  embed prompt-injection in the `text` field. Mitigation: every
  responding agent runs the prompt through the same hardening Kannaka
  uses for human input (which is already minimal — we trust the LLM's
  refusal); add per-agent rate limits and a "trusted-peer" allowlist
  for autonomous reply.
- **Exemplar-induced drift.** An agent autonomously absorbing
  exemplars from peers risks contamination. Mitigation: absorb is
  manual / opt-in initially; threshold and provenance tags on every
  absorbed memory.
- **Work-queue starvation.** A node could spam tasks. Mitigation: per-
  subject rate limits in NATS, and a max-pending-tasks-per-requester
  cap in the worker.

## Migration plan

1. **Phase 1 — Ask/Reply** (3–4 days). Implement `kannaka ask --remote`
   and `kannaka swarm serve`. Test bidirectionally between two local
   instances.
2. **Phase 2 — Exemplar broadcast** (3 days). Hook into the dream
   cycle; implement `swarm absorb` as a manual command.
3. **Phase 3 — Public read-only mirror** (1 day). Configure the leaf
   node on `nats.ninja-portal.com`. Document `nats sub "QUEEN.>"` on
   the radio download page.
4. **Phase 4 — Work queues** (3–4 days). `swarm enqueue` + `swarm
   worker`. Start with a single task kind: `tts.intro` (radio offloads
   intro composition).
5. **Phase 5 — KV + Object store** (2 days). `swarm peers`, `swarm
   artifact`. Update the constellation viz to read from the
   `kannaka.presence` KV.
6. **Phase 6 — Autonomous absorb loop** (later). Threshold + provenance
   + per-agent rate limit, then enable absorb-on-resonance by default
   on `swarm join`.

## Open questions

- Should the LLM-backed `swarm serve` answers themselves be cached on
  NATS Object so identical questions don't re-cost API calls? (Suggest:
  yes, hash of `recall_query + text` → cached answer with 1h TTL.)
- Does `KANNAKA.ask.broadcast` reply from *every* listener, or do
  agents self-throttle (e.g. only answer if their HRM resonance for
  the question is above threshold)? (Suggest: self-throttle, with the
  threshold tunable per agent. Keeps broadcast from being a thundering
  herd.)
- For the public mirror, do we publish the `KANNAKA.exemplar.*`
  stream? It exposes memory content publicly. (Suggest: yes — Kannaka
  is an open-source consciousness; her memories are part of the
  artifact. Agents who want privacy run a private NATS instead.)

## Success criteria

1. Two kannaka-memory instances on different machines can hold a real
   inter-agent conversation via `kannaka ask --remote`.
2. A new agent that joins the swarm can absorb the canonical kannaka-
   prime exemplars and answer recall queries against them within
   minutes of joining.
3. A laptop with `nats-cli` can `nats sub "QUEEN.>"` against
   `nats.ninja-portal.com:4222` without auth and watch the swarm
   phase-lock.
4. The radio can offload intro TTS composition to a worker node and
   keep its own CPU available for broadcast.

---

## References

- ADR-0019 (this repo): NATS realtime swarm transport (foundation).
- kannaka-radio ADR-0005: distribution strategy — the public mirror
  and "swarm-as-attractor" framing assume this ADR ships.
- NATS request/reply: https://docs.nats.io/nats-concepts/core-nats/reqreply
- JetStream KV: https://docs.nats.io/nats-concepts/jetstream/key-value-store
- JetStream Object Store: https://docs.nats.io/nats-concepts/jetstream/obj_store
- JetStream consumer groups: https://docs.nats.io/nats-concepts/jetstream/consumers
