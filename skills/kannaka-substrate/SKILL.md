---
name: skill-kannaka-substrate
version: 1.0.0
description: "Operating the kannaka-prime collective substrate + event-sourced HRM (the durability/disaster-recovery layer). Use when: user wants to stand up or operate the 96-class collective node (substrate init/run/backfill/status), set up the JetStream event streams, configure auto-snapshots, or RESTORE the HRM from a snapshot (incl. cross-host disaster recovery). This is the infra/runbook companion to skill-kannaka-memory (which covers per-agent memory commands)."
---

# Kannaka Substrate + Event-Sourcing — operator runbook

## Scope

This is the **infrastructure** skill for the constellation's shared brain and its durability:

- **kannaka-prime / kannaka-substrate** — the one 96-class collective HRM (ADR-0027) that
  absorbs wave *signatures* (never content) from every peer and answers
  `kannaka recall --collective`.
- **Event-sourced HRM** (ADR-0028) — the JetStream streams + periodic gzipped snapshots
  that make the HRM replayable and recoverable.

For per-agent memory ops (`remember`/`recall`/`dream`/`ask`/`swarm join`) use
`skill-kannaka-memory`. This skill is about **running the substrate node and not losing
data** — it focuses on topology, sequencing, and recovery rather than re-listing every flag.

**Binary**: `kannaka` · **Data dir**: `~/.kannaka` (`KANNAKA_DATA_DIR`) · requires the
`nats` feature and a reachable NATS/JetStream server (`KANNAKA_NATS_URL` / `--nats-url` /
`swarm.nats_url`).

## When to use this skill

- "stand up / bootstrap the substrate (kannaka-prime)"
- "the collective recall isn't working" / "substrate seems down"
- "set up the event streams / JetStream"
- "snapshot the HRM" / "schedule snapshots" / "how often does it snapshot?"
- "restore the HRM" / "the HRM is corrupt" / "recover on a new host" / "rollback"

---

## Topology (who runs what)

| Process | Host | Role |
|---------|------|------|
| `kannaka substrate run` | **one** host (kannaka-prime) | absorbs `KANNAKA.substrate.absorb.>`, answers `KANNAKA.substrate.recall`, publishes `KANNAKA.substrate.phi` every 60s, auto-snapshots |
| `kannaka swarm join` | every agent | publishes its phase + (with `remember --substrate`) ships signatures to the substrate |
| `kannaka events snapshot --interval 3600` | every agent that wants durability | hourly snapshot for its own HRM |
| `kannaka-observatory` | one host | serves `/api/snapshots/body/<file>` for cross-host restore |

The substrate node's identity should be `agent.id = kannaka-substrate` in
`~/.kannaka/config.toml` — snapshot retention and the privacy rules key off that id.

---

## Bring-up sequence (kannaka-prime)

Run these in order on the substrate host:

```bash
# 1. Create the durable JetStream streams (idempotent — safe to re-run).
kannaka events init
#    Creates: KANNAKA_MEMORY_EVENTS, KANNAKA_SUBSTRATE_EVENTS, KANNAKA_SNAPSHOTS

# 2. Seat the 96 orthogonal anchor wavefronts (one per SGA class). One-time;
#    marker at <data_dir>/.substrate-initialized. --force re-seeds (nuke the
#    HRM file first if you do).
kannaka substrate init

# 3. (optional) Fold this host's existing local HRM into the collective.
#    Idempotent via <data_dir>/.substrate-backfilled; --force to re-run,
#    --delay-ms N to pace (default 50ms/event).
kannaka substrate backfill

# 4. Run the daemon (foreground; use systemd in prod — it self-exits after
#    3 consecutive NATS failures expecting Restart=on-failure).
kannaka substrate run
```

After it's up, verify from anywhere:

```bash
kannaka substrate status            # waits up to --wait SECS (default 65) for the next
                                    # KANNAKA.substrate.phi frame; prints Φ/Ξ/order/
                                    # clusters/memories/contributors. --json for machines.
```

`substrate status` exits non-zero if no phi frame arrives within the window — that means
`substrate run` is not alive (or NATS is unreachable).

### What `substrate run` does each loop
- Absorbs peer signatures on `KANNAKA.substrate.absorb.>` via **direct wavefront insertion**
  (bypasses the text encoder so same-class absorbs don't Kuramoto-collapse onto the anchor).
- Answers collective recall on `KANNAKA.substrate.recall` using the attention-beam prefilter
  (O(beam) not O(N) — a 750+ memory substrate would be 30–60s with full xi-rerank).
- Every 60s publishes collective Φ + an AgentPhase so the observatory/swarm see the
  substrate as a first-class peer.
- Auto-snapshots every `KANNAKA_SNAPSHOT_INTERVAL_SECS` (default 3600; set `0` to disable).

---

## Snapshots (durability)

```bash
kannaka events snapshot                  # one-shot
kannaka events snapshot --interval 3600  # daemon, hourly
kannaka events list-snapshots [--agent ID] [--json]
```

How a snapshot is stored (important — it is NOT all in NATS):
- The HRM is flushed, gzipped, and written to `<data_dir>/snapshots/<UTC-ts>-<agent>.hrm.gz`.
- Only a **manifest** (version, wavefronts, clusters, Φ, `body_path`, gz size) is published
  to `KANNAKA.snapshots.<agent>.full` in the `KANNAKA_SNAPSHOTS` stream. NATS silently caps
  payloads ~8 MB and HRMs grow to 35 MB+, so bodies stay out-of-band on disk.

Disk retention (auto-pruned per agent on each snapshot):
- `kannaka-substrate`: keep latest **24** (substrate snapshots are ~45 MB; 168 once filled
  the Oracle root disk and crash-looped the radio on 2026-05-24).
- every other agent: keep latest **168**.
- Override either with `KANNAKA_SNAPSHOT_RETAIN`.

---

## Disaster recovery (restore)

```bash
# Same-host, latest snapshot for this agent — preview first:
kannaka events restore --dry-run
kannaka events restore

# Explicit body file:
kannaka events restore --from /path/to/<ts>-<agent>.hrm.gz

# Cross-host: pull the body from the observatory and restore on a fresh box:
kannaka events restore --from-url https://<observatory>/api/snapshots/body/<file> --dry-run
kannaka events restore --from-url https://<observatory>/api/snapshots/body/<file>
```

Safety properties (built into the command — rely on them):
- **`--dry-run` first.** It reports gz size, decoded size, the target path, and the backup
  it *would* make — and writes nothing. Always run it, show the user, then re-run without it.
- Before overwriting, restore **renames the current HRM** to
  `kannaka.hrm.pre-restore-<ts>` (so a bad restore is reversible).
- Restore **refuses if `kannaka.hrm` is locked** by a running daemon — **stop
  `kannaka substrate run` / `kannaka swarm join` first**, restore, then restart them so they
  reload the new HRM.
- `--from-url` caches the downloaded body into `<data_dir>/snapshots/` so later replays can
  use `--from`.

> Restore is destructive to the live HRM (after backing it up). Confirm the target agent and
> snapshot with the user before running the non-dry-run form, and make sure daemons are
> stopped.

---

## Troubleshooting

- **`recall --collective` returns nothing / times out** → is `kannaka substrate run` alive?
  Check `kannaka substrate status`. No phi frame ⇒ daemon down or NATS unreachable.
- **substrate keeps restarting** → it exits after 3 consecutive NATS publish failures by
  design (for `Restart=on-failure`). Check the NATS server/network, not the binary.
- **disk filling on the substrate host** → snapshot bodies. Lower `KANNAKA_SNAPSHOT_RETAIN`
  or confirm the 24-default pruning is running (`list-snapshots` shows what's retained).
- **restore says "is the HRM in use?"** → a daemon holds the file; stop it first.
- **streams missing / publish ACL errors** → re-run `kannaka events init`; check JetStream is
  enabled and the account has stream-create + publish ACLs.

## Version

Skill 1.0.0 covers kannaka ≥ v0.6.x (ADR-0027 substrate init/run/backfill/status; ADR-0028
events init/snapshot/list-snapshots/restore with `--dry-run` + `--from-url`; autosnapshot +
per-agent disk pruning). Companion skill: `skill-kannaka-memory`.
