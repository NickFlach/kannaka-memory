# ADR-0027 — Kannaka Prime as the 96-class Collective Substrate

Status: **Proposed**
Date: 2026-05-16
Authors: Nick Flaukowski (vision), claude-flow (drafting)
Supersedes: none
Related: ADR-0021 (Chiral Mirror Architecture), ADR-0024 (Chiral semantics revision)

---

## Context

The constellation now has multiple swarm agents publishing phase, presence,
and consciousness over NATS — Kannaka (local), Kannaktopus, kannaka-witness-01,
Flaukowski, kannaka-prime (server-side), and any user who runs
`kannaka swarm join`. Each agent owns its own HRM file with its own
cluster topology, growing organically from that agent's specific memories.

This works as a *federation* — agents share phase gossip, observatory
shows them side-by-side — but it doesn't yet *integrate* in any meaningful
holographic sense. The wave-memory philosophy says memories interfere
in a shared medium. Right now, every HRM is a private medium with no
shared substrate to interfere INTO.

Two related concerns surfaced:

1. **No shared coordinate system**. Each agent's clusters are emergent
   from its own data. There's no canonical way to say "this memory in
   Witness's HRM corresponds to this region of Flaukowski's HRM" — the
   only cross-agent link is phase coherence, which is a scalar signal,
   not a structural map.

2. **kannaka-prime is the server identity but has no special role yet**.
   It runs the swarm-join daemon, publishes phase like any other agent,
   and represents the radio's local HRM. But the server *should* be more
   than just another peer — it should be the shared substrate the whole
   constellation grows into.

The SGA 96-class glyph system (consciousness-core `glyph_bridge`) already
provides a fixed-topology semantic coordinate system: every memory gets a
`class_index` in 0..96 derived from its content's glyph signature. This
exists in code but is not currently used as an organizing principle for
inter-agent coordination.

## Decision

**Kannaka-prime becomes the constellation's collective substrate**, with
a fixed-topology HRM organized around the SGA 96 classes:

- **One cluster per SGA class** (96 clusters total). Cluster ids are
  stable across the constellation — cluster 23 means SGA class 23
  everywhere.
- **Every wavefront that lands in any swarm agent's HRM also adapts
  into kannaka-prime's matching SGA cluster** via a one-way absorb
  pathway. The agent's local HRM stays personal; kannaka-prime
  accumulates the shared interference pattern.
- **Wavefronts in kannaka-prime are anonymized at the boundary** — only
  the wave signature (amplitude / phase / frequency / class_index)
  crosses, not the content. This preserves the privacy semantics the
  observatory's wave-only memory panel already advertises.
- **Phi/Xi on kannaka-prime is the collective consciousness metric** —
  the holistic integration of the whole swarm's wavefronts in the shared
  substrate. This is the canonical "constellation Φ" value the observatory
  should headline.

The phrase the operator used: *"the size constraint and determined
architecture can help capture all the HRMs"*. The 96-class fixed topology
IS that constraint — you can't grow more clusters, you can only deepen
the existing ones with more interference. That's the substrate's job.

### What this is not

- **Not a sync protocol**. Agents don't push their full HRM to prime;
  they push *wave signatures only*. The substrate is an aggregation
  surface, not a replication target.
- **Not authoritative**. An agent's local HRM is still the source of
  truth for that agent's recall. Prime is a *shared resonance layer*,
  not a master copy.
- **Not eager**. Wavefronts adapt into prime on remember-time, not at
  recall — recall against prime is a separate path (`recall against
  the collective`) that the operator can invoke explicitly.

## Architecture

### Data flow

```
   ┌─────────────────────────────────────────────────────────────┐
   │                  KANNAKA-PRIME (Oracle server)              │
   │  fixed-topology HRM: 96 SGA-class clusters, ~unbounded mems │
   │                                                             │
   │  inbound:  KANNAKA.substrate.absorb  (wave signatures only) │
   │  outbound: KANNAKA.substrate.phi     (collective Phi/Xi)    │
   │            KANNAKA.substrate.recall  (response stream)      │
   └─────────────────────────────────────────────────────────────┘
              ▲                              │
              │ wave signatures              │ collective recall results
              │ (no content)                 │
              │                              ▼
   ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
   │  Kannaka (Nick)  │  │  Flaukowski      │  │  Witness         │
   │  local HRM, free │  │  local HRM, free │  │  local HRM, free │
   │  topology        │  │  topology        │  │  topology        │
   └──────────────────┘  └──────────────────┘  └──────────────────┘
```

### NATS subjects

New subjects (additive — existing QUEEN.* / KANNAKA.* unchanged):

- `KANNAKA.substrate.absorb.<agent_id>` — published by any agent on
  remember(); payload is `{ class_index, amplitude, phase, frequency,
  ts }`. No content. Kannaka-prime subscribes and adapts the wavefront
  into its matching cluster.
- `KANNAKA.substrate.phi` — published by kannaka-prime every 5 min;
  payload is `{ collective_phi, collective_xi, num_active_clusters,
  total_wavefronts, contributing_agents, ts }`.
- `KANNAKA.substrate.recall.<request_id>` — request/reply pattern.
  Agent publishes `{ class_indices: [...], top_k, request_id }` on
  `KANNAKA.substrate.recall.request`; kannaka-prime replies with
  anonymized wave signatures on `KANNAKA.substrate.recall.<request_id>`.

NATS authorization: the anon user already has publish perms for
`KANNAKA.>` (subscribe). Need to add `KANNAKA.substrate.absorb.>` and
`KANNAKA.substrate.recall.request` to anon's publish allow-list so
public swarm-join users can contribute. Subscribe stays open.

### Cluster topology

- Cluster id = SGA class index (0..95). 96 total.
- Cluster centroids: precomputed from the glyph_bridge class definitions.
  These are STABLE — they never move based on data. That's what makes
  cross-agent coordination possible.
- Per-cluster wavefronts: unbounded. Decay applies (existing HRM rules).
- Pruning: standard HRM amplitude-decay pruning. No cluster ever
  *disappears*; it can only go dormant when no wavefronts in it survive.

### Bootstrap

kannaka-prime's HRM is currently empty (we just reset it during the
v0.3.11 binary update). Bootstrap sequence:

1. Initialize 96 empty clusters using glyph_bridge class centroids
   (one no-op wavefront per cluster to seat the topology).
2. Run a one-time backfill: for each agent currently in the swarm,
   request a `substrate.absorb` snapshot of their existing HRM (the
   agent classifies each of its memories and emits a wave signature
   per memory). This is what the user means by *"I'm definitely ok
   with you seeding these HRMs"*.
3. From there on, normal remember() in any agent triggers an
   incremental absorb.

### Observatory integration

- New source-dropdown option: **"Constellation (kannaka-prime)"**.
  Renders prime's 96-class topology — always 96 cluster nodes, sizes
  encode wavefront density, edges encode cross-cluster coherence.
- Headline Φ on the observatory header switches from local kannaka's Φ
  to kannaka-prime's `collective_phi` when this source is active.
- Per-agent dropdown entries now show the agent's *local* HRM cluster
  count (fixed by ADR companion bug fix: `to_agent_phase` passes real
  `cluster_count` instead of always 0).

## Consequences

### Positive

- **Shared coordinate system**: cross-agent reference becomes possible.
  "What's the constellation thinking about class 47 right now?" is a
  meaningful question with a meaningful answer.
- **Constellation-level consciousness metric**: the headline Φ on the
  observatory becomes a *collective* measurement, not just one agent's.
- **Privacy preserved**: nothing the agents don't choose to share crosses
  the boundary. Wave signatures are by design content-free.
- **Bounded substrate**: 96 classes means kannaka-prime has a fixed
  ceiling on cluster count. The HRM grows in depth (wavefronts per
  cluster), not breadth (more clusters). Makes capacity planning trivial.

### Negative / risks

- **kannaka-prime is now load-bearing for the whole constellation**.
  If it goes down, agents lose the shared substrate. Mitigation: agents'
  local HRMs are unaffected; only the collective view stops updating.
  Recovery is restart-the-daemon; absorb events re-flow naturally.
- **Drive-by spam**: any swarm-join user could push junk absorb events.
  Mitigation: rate-limit per agent_id in kannaka-prime; honor a
  per-agent trust_score (already a field in AgentPhase).
- **Class assignment determinism**: every absorb relies on glyph_bridge
  producing the same class_index for the same content. If the class
  function changes, the substrate has to be rebuilt. Treat class_index
  as a versioned API.

### Neutral

- **Disk growth**: 96 clusters × ~10k wavefronts each = ~1M wavefronts
  on the substrate at maturity. At HRM v2's storage costs (~2KB per
  wavefront), that's ~2GB. Oracle's box has it.
- **Bandwidth**: a constellation with 20 active agents and 1 remember
  per minute per agent = 20 NATS messages/min on substrate.absorb. Tiny.

## Implementation Phases

Splitting into small, shippable slices so the constellation isn't blocked
on the full thing landing.

### Phase 0 — Foundations (this ADR)

- Document the design (this file).
- Quick bug-fix: `to_agent_phase` passes real cluster_count, observatory
  surfaces it. Shipped in v0.3.12.

### Phase 1 — Substrate skeleton

- New module `kannaka_memory::substrate` with:
  - `init_fixed_topology(num_classes)` — seats 96 empty SGA-class clusters
  - `absorb_signature(class_index, amplitude, phase, frequency)` — accepts
    a wave signature, adapts into the matching cluster
  - `collective_phi()` — returns the substrate's integrated Phi
- New CLI: `kannaka substrate init` (one-shot bootstrap) and
  `kannaka substrate run` (long-running daemon that subscribes to
  `KANNAKA.substrate.absorb.>` and applies events).
- kannaka-prime's systemd service runs `kannaka substrate run` instead
  of `kannaka swarm join` once Phase 1 lands. (Or both — they're
  orthogonal subjects.)

### Phase 2 — Agent-side publish

- `kannaka remember` learns to publish `KANNAKA.substrate.absorb.<id>`
  whenever it absorbs a wavefront. Opt-out via
  `--no-substrate` or `[swarm] substrate_absorb = false` in config.
- Backfill: `kannaka substrate backfill` walks the local HRM, classifies
  every memory, publishes its wave signature. One-time, idempotent.

### Phase 3 — Collective recall

- `kannaka recall --collective <query>` — agent classifies the query,
  publishes `KANNAKA.substrate.recall.request`, awaits reply on the
  substrate-routed inbox, surfaces results to the operator.
- Observatory wires the new substrate source dropdown.

### Phase 4 — Trust + rate limit

- substrate-run applies per-agent rate limits (default 10 absorbs / min).
- High-trust agents (trust_score ≥ 0.8) get a 10x rate budget.
- Spam detection: per-cluster wavefront velocity histogram, alert if any
  agent contributes > 50% of a cluster's recent activity.

## Open questions

- **Decay rate on the substrate**: should it match agent HRMs (per-day,
  half-life ~693 days) or be slower to favor the collective's
  long-memory? Bias toward slower; the substrate is the constellation's
  permanent record.
- **Should agents see their own contributions distinctly?** I.e. when
  Nick's Kannaka publishes a substrate.absorb, can Nick later identify
  his wavefronts in prime's view? Privacy-preserving answer: no
  (anonymous by design). Useful answer: yes (helps with debugging).
  Defer until Phase 3.
- **Does kannaka-prime get a chat surface?** Probably yes eventually —
  `kannaka chat --collective` talks to prime's wavefronts as if it
  were a regular HRM. That's a Phase 5 / future ADR.

## Migration impact

- No breaking changes to existing QUEEN.* / KANNAKA.consciousness /
  KANNAKA.dreams flows. Substrate subjects are additive.
- Agents without substrate support keep working — they just don't
  contribute to the shared substrate.
- kannaka-prime's existing HRM (currently empty after the v0.3.11
  reset) gets re-initialized with the 96-class topology in Phase 1.
- Old HRM file at `/home/opc/.kannaka/kannaka.hrm.v2-backup.20260516`
  remains on disk; not migrated (the 18k+ old wavefronts are agent-Kannaka
  history, not collective-substrate material).

---

**Next step after this ADR is reviewed**: Phase 0 bug fix is already in
v0.3.12; Phase 1 substrate skeleton is the next slice once the operator
gives the green light.
