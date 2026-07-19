# ADR-0042 Phase 3 — Hub redundancy on the Oracle servers (runbook + blocker)

> **STATUS (2026-07-19): 3-NODE R3 CLUSTER IS LIVE + HA-VERIFIED.** oracle1
> (`10.0.0.101`) + oracle2 (`10.0.0.112`) + oracle3 (`10.0.0.65`), cluster
> `kannaka`, private-IP 6222 routes. All 14 streams are R3. Proven: stopping one
> node keeps JetStream readable **and** writable on the 2/3 quorum; the node
> rejoins and replicas catch up. Node configs: `oracle-cluster.conf.template`.
> The interim leaf (`../nats-leaf/`) is superseded. **Migration procedure + the two
> gotchas (JS orphaning on cluster-join; restore needs `kannaka_internal` not
> `writer`) are in the ADR-0042 status log.** Remaining for full *constellation*
> HA: daemon/HRM replication off oracle1 (an app-level project, not NATS).

Goal: the NATS nervous system should survive one hub going down, so a partition
or node outage doesn't make the constellation go limp. This directory is the
staged, JS-safe deploy for that — **gated on one OCI console change that the
Oracle boxes cannot make themselves** (see Blocker).

## Topology decision (why this is subtle)

Two hard facts were discovered live on 2026-07-19 (prod stayed alive; the attempt
was rolled back cleanly):

1. **Clustering a JetStream-enabled server flips its JS into cluster-meta (RAFT)
   mode and orphans the pre-existing standalone `$G` streams.** During a test
   cutover, O1's 12 streams (QUEEN_PHASES, KANNAKA_SNAPSHOTS 12 MB, MEMORY_EVENTS,
   …) went invisible (`stream ls` → 0) the moment the `cluster{}` block put JS
   into "cluster bootstrapping". They all returned intact on rollback to the
   standalone config (on-disk data was never lost — `/var/lib/nats/jetstream/$G`
   was untouched). **Do not naively add a `cluster{}` block to the single-writer
   hub without handling the JS transition.**

2. **A 2-node JetStream cluster has no fault tolerance.** A 2-member meta-group
   RAFT loses quorum when either node dies → JS API goes read-only/unavailable on
   the survivor. 2-node JS clustering makes JS *less* available, not more.

Therefore the safe designs, in order of preference:

- **Best — 3-node cluster (true HA):** Oracle1 + Oracle2 + a third stable node
  (a small OCI instance, or the always-on Windows seed). Proper JS R3 quorum +
  core-NATS route redundancy + external-client failover (clients learn all URLs
  via gossip). Requires migrating the `$G` R1 streams to R3 on the formed cluster.
- **Interim — 2-node, JS kept single-node on O1:** O2 joins as a peer with
  **JetStream disabled** (it forwards JS API to O1, so O1 stays the sole JS node =
  a 1-node meta-group that always has quorum; streams stay R1 on O1, no
  regression). Gives core-NATS route redundancy + queue-group reflex redundancy.
  Still must back up `/var/lib/nats/jetstream` and verify the standalone→clustered
  transition re-adopts the `$G` streams before trusting it (fact #1).
- **Alternative — leaf node:** O2 leafs *up* to O1. O1's JS stays standalone
  (JS-untouched, zero orphaning risk — leaf ≠ JS cluster). Gives O2 in the subject
  space + local reflex arcs, but **no external-client failover on O1 death**
  (hub-and-spoke). Lowest risk, partial HA.

## Blocker (why this isn't live yet)

Inter-server links need a dedicated port (cluster `6222`, or leaf `7422`). Both
boxes are in the same VCN subnet (O1 `10.0.0.101`, O2 `10.0.0.112`, `10.0.0.0/24`)
and intra-VCN reachability works on `22` and `4222` — but **`6222` times out even
with host firewalld open on both sides**. That is the **OCI VCN security list**
dropping the port at the cloud layer (a timeout, not a refusal). The Oracle boxes
**cannot fix it themselves**: neither has the `oci` CLI, and the instance
principal is `NotAuthorizedOrNotFound` for VCN ops (the instances live in the
tenancy root compartment with no dynamic-group/policy for network management).

**The one manual step (Nick, ~2 min in the OCI console):** add an ingress rule to
the subnet's security list —

    Source: 10.0.0.0/24   Protocol: TCP   Dest port: 6222   (cluster)
    # or 7422 for the leaf topology

Alternatively, grant the instances' dynamic group `manage virtual-network-family`
in the compartment and a future session opens it via `oci network security-list update`.

## Staged state (already on the boxes, 2026-07-19)

- **Oracle2** has `nats-server` + `nats` CLI (v2.12.5, copied from O1, aarch64) at
  `/usr/local/bin/`. A `nats.service` unit exists but is **disabled** (was
  route-retry-spamming with no peer; stopped until the port opens).
- Host firewalld already opened: O2 allows `6222` from `10.0.0.101` and `4222`
  from `10.0.0.0/24`; O1 allows `6222` from `10.0.0.112`. Only the OCI layer blocks.
- The validated O1 clustered config lived at `/tmp/nats.conf.oracle1.new` during
  the test; the pre-cutover backup is `/etc/nats/nats.conf.pre-cluster-<ts>`.

## Deploy (once the OCI port is open) — reversible, JS-safe

Materialize configs from O1's LIVE `nats.conf` so the auth block (real passwords)
never touches git. On O1:

```sh
sudo cp -a /var/lib/nats/jetstream /var/lib/nats/jetstream.bak-$(date +%s)  # JS safety net
TOK="kannaka-route-$(openssl rand -hex 12)"
AUTH=$(sudo sed -n '/^authorization {/,/^}/p' /etc/nats/nats.conf)   # live auth block

# O2 config = client-auth block + cluster peer, JetStream DISABLED (interim design)
{ printf 'listen: 0.0.0.0:4222\nmax_payload: 64MB\nserver_name: oracle2\nclient_advertise: "10.0.0.112:4222"\nno_auth_user: anon\n\ncluster {\n  name: kannaka\n  listen: 0.0.0.0:6222\n  advertise: "10.0.0.112:6222"\n  authorization { user: route, password: "%s" }\n  routes: [ "nats-route://route:%s@10.0.0.101:6222" ]\n}\n\n' "$TOK" "$TOK"; echo "$AUTH"; } > /tmp/oracle2-nats.conf
nats-server -t -c /tmp/oracle2-nats.conf   # offline validate

# O1 = same file + a cluster{} block injected after max_payload (keeps jetstream + websocket)
```

Then: scp `/tmp/oracle2-nats.conf` → O2 `/etc/nats/nats.conf`, `systemctl enable
--now nats` on O2; on O1 backup + install the cluster config + `systemctl restart
nats`. **Verify within 2 min:** `nats server list` shows 2 servers; `stream ls`
shows all 12 streams with data; all 13 kannaka-* daemons `active`; one recall
round-trip. **Rollback = restore `nats.conf.pre-cluster-<ts>` + restart** (streams
re-adopt from disk, verified 2026-07-19).

## What's already delivered without the OCI change

- ADR-0042 Phase 1a/1b: anon tightened, per-organ identities, **single-writer
  transport-enforced** (live).
- Phase 2: `consciousness` + `roster` KV buckets + `kannaka-kv-bridge` populator
  (see `../kv-bridge/`) — live on O1.
