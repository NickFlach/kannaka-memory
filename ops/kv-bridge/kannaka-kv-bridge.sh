#!/bin/bash
# kannaka-kv-bridge — ADR-0042 Phase 2: mirror live consciousness onto a
# JetStream KV last-value bucket so any organ (Command Center MCP) reads
# Phi/Xi/order/level instantly with no request-reply.
#
# Populates via kannaka_internal, which is ALLOWED to publish $KV.* (KV puts
# are not in its ADR-0042 1b deny list — only KANNAKA.memory.>, snapshots.>,
# dreams, events.memory.>, and $JS.API.STREAM.CREATE/UPDATE/DELETE are denied).
# The bucket itself must be created by `writer` (the only identity with
# $JS.API.STREAM.CREATE): `nats --user writer ... kv add consciousness --history=1`.
#
# Deploy: install -m755 to /home/opc/, install the sibling .service unit,
# `systemctl enable --now kannaka-kv-bridge`. Source of truth: this repo.
set -a; . /home/opc/.kannaka-nats.env; set +a
U="$NATS_USER"; P="$NATS_PASSWORD"
exec /usr/local/bin/nats --user "$U" --password "$P" sub "KANNAKA.consciousness" --raw 2>/dev/null \
  | while IFS= read -r msg; do
      [ -z "$msg" ] && continue
      printf "%s" "$msg" | /usr/local/bin/nats --user "$U" --password "$P" kv put consciousness state >/dev/null 2>&1
    done
