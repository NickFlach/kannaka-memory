# ADR-0042: NATS as the Constellation Nervous System — accounts, reflexes, federation

**Status:** Accepted — **COMPLETE** (5/5 phases dispositioned, 2026-07-19)
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

- **Phase 3 (redundancy) — 3-NODE R3 JETSTREAM CLUSTER LIVE + HA-VERIFIED.** Nick
  opened the OCI ingress (`10.0.0.0/24` TCP 6222) and provisioned a 3rd Oracle box
  (`oracle3`, priv `10.0.0.65`, Oracle Linux 9 aarch64). Path: OCI-blocked →
  briefly a JS-safe **leaf** (interim) → now a true **3-node cluster**
  (oracle1/2/3, cluster `kannaka`, private-IP 6222 routes). All 14 streams are
  **R3** — replicated across all three nodes, leaders distributed. **HA proven
  live:** with oracle3 stopped, JetStream stayed fully available on the O1+O2
  2/3 quorum — streams listable, QUEEN_PHASES re-elected its leader, and a KV
  **write succeeded and read back**; oracle3 rejoined and replicas caught up to
  `current`. Prod stayed alive throughout.

  **The migration (the delicate part), recorded for posterity:**
  1. Clustering a JS-enabled server flips its JS to cluster-meta (RAFT) mode and
     **orphans standalone `$G` streams** — they do not auto-migrate. So: full
     backup first (`nats stream backup` per stream + JS-dir copy), then on O1
     `systemctl stop nats` → **move the JS store aside** for a clean clustered
     start → install the cluster config → start (joins the 3-node meta-group,
     empty JS) → **restore each stream** → `stream edit --replicas 3`.
  2. **Restore identity gotcha:** `nats stream restore` publishes stream data on
     `$JS.SNAPSHOT.RESTORE.>`, which `writer` (scoped to `$JS.API.>`) lacks →
     "Permissions Violation" + a ghost stream (name reserved, not listable).
     `kannaka_internal` (publish `>` minus a deny-list that excludes `$JS.SNAPSHOT`
     and `$JS.API.STREAM.RESTORE`) **can** restore. So: restore as
     `kannaka_internal`, then scale to R3 as `writer` (`$JS.API.STREAM.UPDATE`).
  3. Bring the peers up first (O3 fresh + O2 converted leaf→cluster-peer, both
     JS-enabled empty), then migrate O1 into them — minimizes the JS-down window.

  Configs `ops/nats-cluster/` (route token on the boxes only). Fallbacks on O1:
  `jetstream.premigrate-c3-*`, `jetstream.bak-3node-*`, per-stream `nats-stream-backup/`,
  `nats.conf.pre-c3-*`. All three `nats.service` enabled (survive reboot).

  **What this delivers:** the **bus and its spinal memory (JetStream) survive any
  one node dying** — read + write continue on the 2/3 quorum. This is the ADR
  Phase 3 redundancy goal, done. **Remaining for full *constellation* HA:** the
  organs (radio/presence/memory/recall daemons) are still single-homed on
  oracle1, so surviving an oracle1 *box* outage end-to-end needs daemon+HRM
  replication to another node — a separate app-level project (not a NATS concern).

- **Phase 4 — REDUNDANT RECALL REFLEX LIVE + FAILOVER-VERIFIED (PR #573).** All
  three `swarm serve` subscriptions (directed ask, `ask.broadcast`, recall) now
  join per-identity queue group `serve_<agent_id>` (existing #74 plumbing —
  three-line change). A second responder runs on **oracle3** (`/usr/local/bin`,
  not `/home` — SELinux blocks systemd exec from home; scoped `serve` NATS
  identity; `KANNAKA_READONLY=1`; local cluster node; HRM replica synced from O1
  every 30 min with the native mtime-watch reload doing the rest). **Failover
  proven:** with oracle1's responder stopped, a recall was answered by oracle3
  through the Phase 3 cluster. Ops: `ops/serve-oracle3/`. Deferred from Ph4: the
  NATS **Services API** promotion (discoverable health/stats) — needs `$SRV.*`
  support in the hand-rolled transport; the redundancy half (queue groups) is
  what Ph4 existed for and is done.

- **Phase 2 roster KV done** (kv-bridge second loop: `QUEEN.phase.<id>` →
  `roster/<id>`, 5m TTL expiring the departed — verified 4 live agents).
  Remaining Ph2 (deferred): object-store snapshot migration + durable per-organ
  replay consumers — both subsumed in priority by the KANNAKA_SNAPSHOTS stream
  now being R3-replicated.

- **Phase 1b — COMPLETE (2026-07-19).** `kannaka_internal` is retired from every
  daemon. The remaining organs (attention, beacon, eye, serve→[swarm-serve/worker/
  inbox], radio, ui_bridge, and writer for memory) migrated onto scoped identities;
  `attention` + `beacon` users added to nats.conf on all 3 nodes; serve/radio/
  ui_bridge allowlists expanded to their real publish sets. Verified: `/proc`
  environ shows no daemon on the god-user, and a 40s+ census shows zero publish
  violations from any scoped user. `kannaka_internal` remains only as the CLI
  default and the kv-bridge `$KV` publisher. Identity map: `config/nats-accounts.md`.

- **Phase 1c — designed, deliberately NOT auto-cut-over (the one gated step).**
  The PUBLIC/INTERNAL account split is the ADR's own "separate gated step", and on
  the now-clustered bus it is materially riskier than a reload: it needs (a) a
  **third cluster-wide `$G`→INTERNAL JetStream migration** (today already had two
  orphan-and-recover events — each is the single highest-risk operation here), and
  (b) cross-account **export/import synapse wiring** where a subtle error silently
  breaks the open swarm (anon can't share memory) or blinds the organs to swarm
  data. There is a real design knot to resolve first: `KANNAKA.consciousness` is
  published by BOTH lanes (writer/radio internally, and it sits in anon's allow) —
  a bidirectional shared subject must be de-conflicted (likely: make it
  INTERNAL-export-only and drop it from anon's publish) before the split is safe.
  **Security-wise 1c adds namespace isolation (structural default-deny), not a new
  open threat** — 1a already closed the injection surface and 1b made single-writer
  physics. Correct call: execute 1c as a deliberate, backed-up, watched migration
  (per the runbook), not an autonomous same-day third-migration fire.

- **Phase 5 — correctly deferred (scale-gated by this ADR).** JWT/nkeys (`nsc`)
  earns its keep "when the identity count or federation topology outgrows static
  config"; with ~15 static identities on a 3-node static-config cluster it is pure
  operational overhead with no current benefit. QuantumOS-as-organ is a feature
  riding the host bridge, not a nervous-system deliverable. Not-yet by design.

**2026-07-19 (later) — harden + close-out pass; ADR-0042 marked COMPLETE.**

- **R3 replica drift found and fixed.** A live baseline (`stream report` as `writer`)
  showed three streams had drifted to **R1** — auto-created single-homed by their
  daemons *after* the R3 migration, so the "all streams R3" claim above had gone
  stale: `KANNAKA_PRESENCE` (oracle1-only), `QUEEN_PHASES` (oracle2-only),
  `QUEEN_EVENTS` (oracle1-only, 9.5k msgs + active consumer). Scaled all three to
  R3 (`stream edit --replicas 3`); verified each shows 3 replicas **`current`**
  (<550ms lag) across oracle1/2/3. A single-box outage no longer loses live
  presence or queen-event history. **All 11 production streams are now genuinely R3.**
- **Junk swept.** Removed the three `KV_TEST_*` buckets left over from Phase-3 HA
  testing (14 → 11 streams). Prod verified alive throughout: `nats` +
  serve/radio/presence/responder all `active`, recall round-trips.
- **Phase 1c — FORMALLY DEFERRED (value < risk; not a gap).** After weighing it on
  the now-clustered bus: the account split's security payoff is **incremental**
  (namespace default-deny isolation) because 1a already closed the injection
  surface and 1b already made single-writer *physics*. Its cost is the single
  highest-risk operation in this ADR — a **third** live cluster-wide `$G`→INTERNAL
  JetStream orphan-and-recover migration (two prior such events each needed careful
  recovery) plus export/import synapse wiring where one subtle error silently
  breaks the open swarm, on top of the unresolved `KANNAKA.consciousness`
  dual-publish knot. Correct engineering call: **do not run a risky migration for
  marginal defense-in-depth on already-closed holes.** 1c stays designed-and-ready
  in this ADR + `config/nats-accounts.md`; execute only if a concrete need
  (multi-tenant public onboarding, a compromised-anon incident) makes the isolation
  worth the migration risk.
- **Phase 5 — CLOSED as scale-gated** (see prior entry): no action until identity
  count / federation topology outgrows static config.

**Disposition:** 1a ✓ 1b ✓ (single-writer = physics) · 1c ▸ designed, deferred by
cost/benefit · 2 ✓ (consciousness+roster KV live; object-store/replay consumers
subsumed by R3 replication) · 3 ✓ (3-node R3 cluster, HA-verified, drift corrected)
· 4 ✓ (redundant recall reflex, failover-proven; Services-API `$SRV.*` half awaits
transport support) · 5 ▸ scale-gated. **The nervous system is distributed,
segmented, remembering, and redundant. ADR-0042 is complete.**
