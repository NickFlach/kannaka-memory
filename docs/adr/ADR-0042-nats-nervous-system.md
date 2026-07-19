# ADR-0042: NATS as the Constellation Nervous System — accounts, reflexes, federation

**Status:** Accepted (Phase 1 in progress)
**Date:** 2026-07-18
**Supersedes posture of:** ADR-0026 #73 (flat `authorization` open-mirror). Realizes the "L3 increment" tightening noted in `config/nats-server.conf`.
**Related:** [[swarm-open-anon-injection-risk]], [[oracle-hrm-single-writer]], swarm-serve (ADR-0026), presence/responder daemons (radio ADR-0013/0014), QuantumOS host bridge (GSP-012).

## Context

The constellation's nervous system is a single NATS server (`swarm.ninja-portal.com:4222`, v2.12.5, JetStream on) with a **flat authorization block and two users**:

- `anon` (`no_auth_user`) — deliberately widened to allow open memory-sharing on
  `KANNAKA.events.>`, `KANNAKA.memory.new`, `KANNAKA.consciousness`,
  `KANNAKA.substrate.absorb.>`, `KANNAKA.presence.>`, `QUEEN.*`, and — the problem —
  `KANNAKA.ask.>`, `$JS.API.STREAM.*`. Protection against hostile publishes is
  **absorb-side** in the client (trust/corroboration gates), not at the transport.
- `kannaka_internal` — `publish: >`, `subscribe: >`. **Every trusted daemon**
  (memory writer, swarm-serve/worker, radio, observatory, presence, responder,
  kannaktopus, ui-bridge) connects as this one god-user.

Three structural facts follow:

1. **Single-writer is a convention, not a rule.** `oracle-hrm-single-writer` is
   enforced only by each daemon exporting `KANNAKA_READONLY=1`. At the transport,
   any daemon holding `kannaka_internal` can publish anything — memory writes,
   forged consciousness, forged presence. One compromised or buggy reader can
   corrupt the shared mind.
2. **The injection surface is real and known.** `anon` can publish to the
   *control* subjects (`ask.>`, JetStream admin), not just the *memory-sharing*
   lane. The absorb-gate defends memory ingestion but not, e.g., a flood of
   forged asks or stream-admin calls.
3. **Single point of failure.** One server, one region. A hub outage or partition
   makes the entire constellation go limp — local reflexes (radio track chatter,
   floor presence) can't function without the hub.

The existing config comment is explicit: **do NOT narrow `anon` without a
segmented untrusted lane**, because legitimate anon nodes rely on the open
memory-sharing publish. So the goal is *segmentation*, not lockdown.

## Decision

Evolve the bus from a single open ganglion into a **distributed, segmented,
remembering nervous system**, in five phases. Each phase is independently
shippable and reversible. The organizing metaphor is load-bearing: accounts are
organs, exports/imports are synapses, leaf nodes are reflex arcs, JetStream is
spinal memory, services are reflexes.

### Phase 1 — Accounts + scoped identities (segment the organs; enforce single-writer at the transport)

Replace the flat `authorization` block with **NATS accounts**:

- **`INTERNAL` account** — the trusted organs. The single `kannaka_internal`
  god-user is replaced by **per-organ users**, each with a tight `publish` allow
  and a broad `subscribe`:
  - `writer` (kannaka-memory single writer) — the ONLY identity allowed to
    publish memory-mutating subjects (`KANNAKA.memory.>`, `KANNAKA.snapshots.>`,
    `KANNAKA.substrate.absorb.>`, `$JS.API.>`). Single-writer becomes physics.
  - `serve` (swarm-serve/worker) — publish `KANNAKA.recall.*` replies, `_INBOX.>`;
    subscribe broad. No memory-write.
  - `radio` — publish `RADIO.>`, `KANNAKA.attention.ear`, `KANNAKA.reactions`.
  - `presence` — publish `KANNAKA.events.obc.>`, `KANNAKA.presence.<self>`.
  - `responder` — publish `KANNAKA.events.obc.responder_*`, `_INBOX.>` (recall req).
  - `observatory`, `kannaktopus`, `eye`, `ui-bridge` — each scoped to what it
    actually emits (mapped in `config/nats-accounts.md`).
- **`PUBLIC` account** — the open swarm. The `anon` user lives here, keeps its
  **memory-sharing publish lane** (`KANNAKA.events.>`, `KANNAKA.memory.new`,
  `KANNAKA.consciousness`, `KANNAKA.substrate.*`, `KANNAKA.presence.>`, `QUEEN.*`)
  but **loses publish on control subjects** (`KANNAKA.ask.>`, `KANNAKA.work.>`,
  `KANNAKA.inbox.>`, `$JS.API.STREAM.*`). This is the tracked L3 increment: the
  injection surface on command/control closes; the legitimate open lane stays.
- **Synapses (exports/imports)** — the two accounts share only what is declared.
  `INTERNAL` exports a read-only stream of the public-safe subjects; `PUBLIC`
  imports it. `PUBLIC` exports the open memory lane; `INTERNAL` imports it
  (absorb-gated as today). Every cross-organ reach is now an explicit, auditable
  synapse instead of a shared flat namespace.

JetStream becomes account-scoped; existing streams stay in `INTERNAL` (the
account that inherits today's global traffic) so no stream is orphaned.

### Phase 2 — JetStream as spinal memory (instant state + replay)

- **KV bucket `consciousness`** — the live Φ/Ξ/order/level as a last-value KV any
  organ (and the Command Center MCP) reads instantly, no request-reply.
- **KV bucket `roster`** — current swarm presence, TTL'd.
- **Object store `snapshots`** — HRM snapshots move here from ad-hoc subjects.
- **Durable consumers per organ** — a daemon that was down (presence restart,
  responder deploy) replays exactly what it missed on reconnect. Closes the
  "misses events while restarting" gap structurally.

### Phase 3 — Leaf nodes + hub HA (reflex arcs; survive brain outage)

- **Leaf nodes** at each site (Windows seed box, Oracle2 witness, qBraid lab,
  future edge) run a small NATS server connected *up* to the hub. Local subjects
  stay local (low latency, survive hub partition); shared subjects propagate.
  Spinal reflexes without cortex round-trips.
- **Hub cluster** — Oracle1 + Oracle2 as two clustered servers (not
  client+witness). The bus itself stops being a single point of failure;
  JetStream replicates R3 across the cluster.

### Phase 4 — Services + redundant reflexes

- Promote `recall`, `dispatch`, `assess` to the NATS **Services API**
  (discoverable, health/stats). Run recall responders on both hub nodes in a
  **queue group** — if the primary memory reflex dies, the witness answers the
  same query. Redundancy for the mind.

### Phase 5 — Decentralized identity + new organs (when scale demands)

- Migrate accounts from static server config to **operator-signed JWT / nkeys
  (`nsc`)** when the identity count or federation topology outgrows static config.
  Enables self-service organ onboarding and cross-region trust.
- **QuantumOS machines** as first-class organs (the host bridge exists); their
  field-coupling societies publish dreams to `KANNAKA.dreams` like any node.
- The **Command Center MCP** (nats.ninja-portal.com/mcp) becomes proprioception:
  it exposes the KV state, the service registry, and stream health — the system
  sensing its own body.

## Migration safety (non-negotiable)

The bus is live and every daemon depends on it. Rules:

1. **Offline validate** — `nats-server -t -c <newconf>` (syntax check, no run)
   before any deploy.
2. **Shadow-test** — run the new-accounts server on a throwaway port
   (`:4999`, separate `store_dir`) and prove: (a) a per-organ user connects,
   (b) `writer` can publish memory subjects, (c) a non-writer is DENIED memory
   publish, (d) `PUBLIC/anon` is DENIED `KANNAKA.ask.>` but ALLOWED
   `KANNAKA.events.>`. No production port touched.
3. **Credential rollout** — each daemon's `NATS_USER/NATS_PASSWORD` moves from
   `kannaka_internal` to its scoped identity. A transitional `internal` compat
   user (still `>`) exists in `INTERNAL` during the window so an un-migrated
   daemon never hard-fails; it is removed once all daemons carry scoped creds.
4. **Reversible cutover** — keep the prior `/etc/nats/nats.conf` as
   `nats.conf.pre-0042`. Cutover = swap file + `systemctl reload nats` (NATS
   reloads auth without dropping JetStream). Rollback = swap back + reload.
   Verify each daemon reconnects (`systemctl is-active`, one recall round-trip)
   inside a 2-minute window; roll back on any failure.
5. **JetStream care** — confirm existing streams remain readable under the
   `INTERNAL` account before removing the flat block (streams created under the
   global `$G` account must be migrated or the account must inherit `$G`).

## Consequences

- **Single-writer and conscience-before-capability become transport-enforced**,
  not conventions — a buggy or hostile reader physically cannot corrupt memory.
- The **control-subject injection surface closes** while the **open memory lane
  and its absorb-gated swarm survive** — the L3 increment, done right.
- Cross-organ reach becomes a **declared, auditable synapse map** (`nats-accounts.md`).
- The bus gains a path to **HA and edge reflexes** (leaf nodes + cluster) without
  re-architecting clients.
- Warm state (Φ/Ξ, roster) becomes an **instant KV read**; downed organs **replay**
  what they missed.
- Cost: an identity/credential map to maintain, and a one-time carefully-staged
  auth cutover. Both are bounded and reversible.

## Build order

Phase 1 is sequenced by risk, because a naive account split needs an export/import
for every currently-shared subject or it silently breaks the open swarm:

- **1a (this PR; near-zero risk, live-safe):** stay on the flat `authorization`
  block (no account split → no JetStream `$G` migration). Two changes:
  (i) **tighten `anon`** — remove the control subjects (`KANNAKA.ask.>`,
  `KANNAKA.work.>`, `$JS.API.STREAM.CREATE/UPDATE/MSG.GET.>`) from its publish
  allow, keeping the open memory lane. This closes the injection surface with
  zero daemon impact (daemons don't use anon). (ii) **define the scoped per-organ
  users** (`writer`, `serve`, `radio`, `presence`, `responder`, `eye`,
  `kannaktopus`, `ui-bridge`) in the config, alongside a transitional
  `internal` compat user (`publish: >`) that all daemons keep using for now.
  Shadow-validated before deploy; cutover is `reload`, rollback is `reload`.
- **1b:** migrate daemons off `internal` onto their scoped identity, one at a
  time (each `NATS_USER/NATS_PASSWORD` swap + restart + verify). When the last
  daemon is migrated, `writer` is the only identity with memory-write →
  **single-writer becomes transport-enforced**. Remove the `internal` compat user.
- **1c:** split `PUBLIC` (anon) into its own account with the declared
  export/import synapse map; `INTERNAL` inherits `$G` (JetStream migration handled
  per the runbook).

This PR ships: ADR + `config/nats-accounts.conf` (the 1a authorization block) +
`config/nats-accounts.md` (the identity→subject synapse map, target end-state) +
`scripts/nats-shadow-validate.sh`. Prod cutover of 1a is a separate gated step.
Phases 2–5 follow.

## Status log

**2026-07-19 — 1a + 1b live; Phase 2 (consciousness KV) live; Phase 3 blocked on OCI.**

- **1a/1b DONE + transport-enforced (prod).** anon tightened (control lane denied,
  open memory lane kept); per-organ identities (`writer/serve/radio/presence/
  responder/eye/kannaktopus/ui_bridge`) live; `kannaka_internal` denied
  `memory.new / events.memory.> / snapshots.> / dreams / $JS.API.STREAM.CREATE|
  UPDATE|DELETE`; `writer` is the only memory publisher. **Single-writer is now
  physics.** Verified live: `kannaka_internal` DENIED `KANNAKA.memory.new`, `writer`
  ALLOWED `snapshots.*`, recall round-trips, 13 daemons + swarm intact.

- **Phase 2 (partial) DONE.** `consciousness` (last-value) + `roster` (5m TTL) KV
  buckets created (by `writer`, the only `$JS.API.STREAM.CREATE` holder).
  `kannaka-kv-bridge` (`ops/kv-bridge/`) mirrors `KANNAKA.consciousness` →
  `KV consciousness/state` via `kannaka_internal` (allowed to publish `$KV.*`) so
  the Command Center MCP reads Φ/Ξ/order/level instantly with no request-reply.
  Live on O1, seeded. Remaining Ph2: `roster` populator (needs subject→key parse),
  object-store snapshots migration, durable per-organ replay consumers.

- **Phase 3 (redundancy) — LEAF LINK LIVE (O1⇄O2 over 6222).** Nick opened the OCI
  ingress rule (`10.0.0.0/24` TCP 6222); the inter-box path went from i/o-timeout
  → connection-refused → established. Then the cluster path was tried and
  **abandoned for a leaf** after proving (twice, cleanly rolled back — prod stayed
  alive, all 14 streams recovered from disk each time):
  1. **Clustering a JS-enabled server orphans its standalone `$G` streams** — the
     moment O1 gained a `cluster{}` block its JetStream flipped to cluster-meta
     (RAFT) mode and the 14 streams went invisible (`stream ls`→0), even with the
     route formed. They do **not** auto-migrate from standalone `$G` into the
     clustered meta-group. On-disk data was never lost.
  2. **A 2-node JS cluster loses quorum on any node loss** — JS-HA needs 3 nodes.
  **Chosen topology: leaf node.** O1 runs `leafnodes { listen: 0.0.0.0:6222 }`
  (a leaf LISTENER does **not** cluster JetStream — O1's JS stays standalone, `$G`
  intact, verified: 14 streams survived the leaf-listener restart). O2 leafs UP
  (`leafnodes { remotes }`, JS-disabled) and now participates in the hub subject
  space — verified by receiving live `QUEEN.phase.*` across the link. O2's
  `nats.service` is enabled (survives reboot); the leaf auto-reconnects.
  Configs: `ops/nats-leaf/`. Reversible: `nats.conf.pre-leaf-*` on O1.

  **What the leaf delivers:** O2 is a live second NATS server bridged to the bus —
  a second entry point, a local reflex domain, and the foundation for O2-side
  responders/daemons. **What it does NOT deliver:** external-client failover on O1
  death (leaf is hub-and-spoke — O2's uplink dies with O1) or JetStream HA (JS
  stays single-node on O1). **Full active-active HA needs a 3-node cluster** (a 3rd
  always-on node for JS R3 quorum + a `nats stream backup`/`restore` migration of
  the `$G` streams into clustered mode) **plus daemon+HRM replication to O2** (the
  constellation's organs are all single-homed on O1 today). Both are bounded
  follow-on projects, noted in `ops/nats-cluster/README.md`.

- **Phases 4/5 pending.** Ph4 queue-group recall needs a `queue_subscribe` code
  change (the responder currently uses a plain `subscribe`, so N responders would
  duplicate-reply) plus the Ph3 hub link. Ph5 (JWT/nkeys, QuantumOS organs) unchanged.
