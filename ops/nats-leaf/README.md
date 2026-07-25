# ADR-0042 Phase 3 — Leaf-node hub link (LIVE)

Oracle2 leafs up to Oracle1 over port **6222** (the OCI-opened port). This is the
**JS-safe** redundancy topology, chosen after clustering was proven to orphan the
hub's JetStream streams (see `../nats-cluster/README.md`).

## Why leaf, not cluster

A `cluster{}` block on a JetStream-enabled server flips its JS into cluster-meta
(RAFT) mode; the hub's existing standalone `$G` streams (QUEEN_PHASES,
KANNAKA_SNAPSHOTS, MEMORY_EVENTS, the KV buckets, …) do **not** auto-migrate and
go invisible. A **`leafnodes{}` listener does not cluster JetStream** — the hub's
JS stays standalone and its streams stay intact (verified: all 14 streams survived
the leaf-listener restart). So the two Oracle NATS servers link via leaf, not
cluster, until a proper 3-node cluster + stream migration is done.

## Topology (live 2026-07-19)

```
  Oracle1 (hub, 10.0.0.101)                 Oracle2 (10.0.0.112)
  ┌───────────────────────────┐            ┌──────────────────────────┐
  │ nats :4222 (clients)       │            │ nats :4222 (clients)     │
  │ jetstream (STANDALONE, $G) │  leaf 6222 │ jetstream DISABLED       │
  │ leafnodes.listen :6222 ◄───┼────────────┤ leafnodes.remotes ──────►│
  └───────────────────────────┘            └──────────────────────────┘
         all 13 daemons + kv-bridge                 witness/staff/beacon
```

- **O1** = current standalone config **+** `leafnodes { listen: 0.0.0.0:6222,
  authorization { user: leaf, password: <LEAF_TOKEN> } }`. JS untouched.
- **O2** = client-auth block (mirrored from O1) **+** JetStream disabled **+**
  `leafnodes { remotes: [ nats-leaf://leaf:<LEAF_TOKEN>@10.0.0.101:6222 ] }`.
  `nats.service` enabled (survives reboot); the remote auto-reconnects on drop.

`<LEAF_TOKEN>` is a shared secret held only on the boxes (`/etc/nats/nats.conf`),
not in git. Regenerate with `openssl rand -hex 12` and set it identically in both
files.

## Verified

- Leaf connection: `Leafnode connection created for account: $G` (O2 log);
  TCP `ESTAB 10.0.0.101:6222 ↔ 10.0.0.112` (O1 `ss`).
- Propagation: O2 subscribed `QUEEN.phase.>` and received live phase gossip
  published on O1 — the hub subject space reaches O2.
- Hub unharmed: 14/14 streams, 14/14 daemons, single-writer enforced, recall OK.

## What it delivers / doesn't

**Delivers:** O2 is a live second NATS server on the bus — a second entry point, a
local reflex domain, and the base for O2-side responders/daemons (ADR Phase 4).

**Does not deliver:** external-client failover if O1 dies (leaf is hub-and-spoke —
O2's uplink dies with O1), nor JetStream HA (JS is single-node on O1). Those need a
**3-node cluster** (3rd always-on node for JS R3 quorum + a `nats stream backup`/
`restore` migration of `$G` into clustered mode) **and** daemon+HRM replication to
O2 (the organs are single-homed on O1 today). See `../nats-cluster/README.md`.

## Rollback

O1: restore `/etc/nats/nats.conf.pre-leaf-<ts>` + `systemctl restart nats`
(streams re-adopt from disk). O2: `systemctl disable --now nats` (its daemons
connect to O1 directly, unaffected).
