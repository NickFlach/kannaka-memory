# ADR-0042 Phase 4 — oracle3 redundant recall responder (LIVE)

A second `swarm serve` instance on oracle3 answers `KANNAKA.recall.kannaka-prime`
(and directed/broadcast asks) for the same identity as oracle1's responder, in
NATS queue group `serve_kannaka-prime` (PR #573) — each request is delivered to
exactly one responder; if either dies, the other keeps answering.

**Failover proven live 2026-07-19:** with oracle1's `kannaka-swarm-serve`
stopped, a recall from oracle1 was answered by oracle3 through the Phase 3
cluster (attribution by elimination — O3 held the only subscription).

## Pieces (all on oracle3 unless noted)

| Piece | Where | Notes |
|---|---|---|
| Binary | `/usr/local/bin/kannaka` | Synced from O1's `target/release/kannaka` after each deploy (aarch64). **NOT `/home` — SELinux blocks systemd exec from home dirs** (`selinux-home-bin-systemd-trap`). |
| Run script | `/usr/local/bin/run-kannaka-serve.sh` (source: `run-kannaka-serve.sh` here) | `KANNAKA_READONLY=1`, local cluster node (`nats://127.0.0.1:4222`), scoped `serve` NATS identity from `/home/opc/.kannaka-serve.env` (0600, on-box only). |
| Unit | `/etc/systemd/system/kannaka-swarm-serve.service` (source: `kannaka-swarm-serve.service` here) | `Restart=always`, enabled. |
| HRM replica | `/home/opc/.kannaka/kannaka.hrm` | Synced from O1 every 30 min (`hrm-sync-o3.sh` here, runs on **O1** via cron `12,42 * * * *`, dedicated key `~/.ssh/o3-sync`, private VCN). The binary's native HRM mtime watch (#565) restart-to-reloads after each sync. |

## Deploy / update

On O1 after a `kannaka` build:
```sh
rsync -az -e "ssh -i ~/.ssh/o3-sync" ~/kannaka-memory/target/release/kannaka opc@10.0.0.65:/tmp/kannaka.new
ssh -i ~/.ssh/o3-sync opc@10.0.0.65 'sudo install -m755 /tmp/kannaka.new /usr/local/bin/kannaka && sudo systemctl restart kannaka-swarm-serve'
```

## Notes

- The replica lags O1 by ≤30 min — acceptable for the recall reflex (research/
  identity memories, not real-time state). Tighten the cron if needed.
- Benign log noise: `Permissions Violation ... $JS.API.STREAM.CREATE.QUEEN_PHASES`
  — the client tries to ensure streams exist on connect; non-writer identities
  are correctly denied (ADR-0042 1b working as intended).
- The queue group is per-identity (`serve_<agent_id>`), so this pattern extends
  to any organ: run the same daemon on two nodes, same agent-id, done.
