#!/bin/bash
# oracle3 redundant recall responder (ADR-0042 Ph4). Serves the SAME identity
# (kannaka-prime) as oracle1's responder, in queue group serve_kannaka-prime —
# NATS delivers each request to exactly one of them. Read-only HRM replica,
# synced from oracle1 every 30 min (hrm-sync-o3.sh); the binary's native HRM
# mtime watch (#565) restart-to-reloads on each sync.
export KANNAKA_DATA_DIR=/home/opc/.kannaka
export KANNAKA_NATS_URL=nats://127.0.0.1:4222
export KANNAKA_READONLY=1
export KANNAKA_QUIET=1
set -a; . /home/opc/.kannaka-serve.env; set +a
exec /usr/local/bin/kannaka swarm serve --agent-id kannaka-prime --threshold 0.3
