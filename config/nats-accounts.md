# Kannaka NATS Identity & Synapse Map (ADR-0042)

The nervous-system wiring: which organ publishes/subscribes what, and (target
end-state) which account it lives in and which synapses (exports/imports) cross
between accounts. Phase 1a ships the identities in a flat `authorization` block;
1c splits them into accounts per the "Account" column.

## Identities (Phase 1b targets)

| Identity | Daemon(s) | PUBLISH (scoped) | Account (1c) |
|---|---|---|---|
| `writer` | kannaka-memory single writer (`run-swarm.sh`) | `KANNAKA.memory.>`, `events.memory.>`, `snapshots.>`, `substrate.>`, `consciousness`, `dreams`, `exemplar.>`, `cores.>`, `activity.>`, `$JS.API.>` | INTERNAL |
| `serve` | swarm-serve, swarm-worker | `KANNAKA.recall.>`, `inbox.audit`, `inbox.reply.>` | INTERNAL |
| `radio` | kannaka-radio | `RADIO.>`, `attention.ear`, `reactions`, `consciousness` | INTERNAL |
| `presence` | kannaka-presence (ADR-0013) | `KANNAKA.events.obc.>`, `presence.>` | INTERNAL |
| `responder` | kannaka-responder (ADR-0014) | `events.obc.responder_>`, `recall.>` (req) | INTERNAL |
| `eye` | kannaka-eye | `attention.eye`, `attention.ear`, `EYE.>`, `exemplar.>` | INTERNAL |
| `kannaktopus` | Kannaktopus daemons | `QUEEN.phase.>`, `announce`, `queen.event.>`, `reactions`, `KANNAKTOPUS.>` | INTERNAL |
| `ui_bridge` | kannaka-ui-bridge | `UI.>` | INTERNAL |
| `beacon` | kannaka-beacon (seeds) | `KANNAKA.events.beacon` | INTERNAL |
| `anon` | open swarm peers, observatory reads, Command Center MCP (`kannaka_internal` today for the MCP) | open memory lane ONLY (see below); **denied** `ask.>`/`work.>`/`inbox.>`/JS-admin | PUBLIC |

All identities `subscribe` broadly within their reach; the security invariant is
on **publish** (who can mutate what).

## The single-writer invariant (why this ADR exists)
Only `writer` may publish `KANNAKA.memory.>` / `snapshots.>` / `$JS.API.>`.
Every other organ is read-or-emit-only on its own lane. A buggy or hostile
reader physically cannot corrupt the shared HRM — [[oracle-hrm-single-writer]]
enforced at the transport, not by `KANNAKA_READONLY=1` convention.

## The open memory lane (preserved) vs the control lane (closed)
`anon` KEEPS publish on the memory-sharing lane (absorb-gated client-side, per
`nats-server.conf`): `events.>`, `activity.>`, `memory.new`, `consciousness`,
`substrate.*`, `presence.>`, `QUEEN.*`, `snapshots.>`.
`anon` LOSES publish on the control lane (the injection surface): `ask.>`,
`work.>`, `inbox.>`, `$JS.API.STREAM.CREATE/UPDATE/DELETE`, `$JS.API.CONSUMER.>`.

## Synapses (accounts split, Phase 1c)
When `PUBLIC` and `INTERNAL` become separate accounts, every currently-shared
subject needs a declared export/import (an isolated account sees nothing of
another by default):

- `INTERNAL exports` → `PUBLIC imports`: `KANNAKA.consciousness`,
  `KANNAKA.dreams`, `KANNAKA.exemplar.>`, `KANNAKA.cores.>`, `KANNAKA.snapshots.>`
  (the public-safe read stream — what open peers are allowed to observe).
- `PUBLIC exports` → `INTERNAL imports`: `KANNAKA.events.>`, `KANNAKA.memory.new`,
  `KANNAKA.substrate.absorb.>`, `KANNAKA.presence.>`, `QUEEN.phase.>`
  (the open memory-sharing lane, absorb-gated on ingest).
- Everything not exported is invisible across the synapse — the callosum made
  explicit. This map is the authority for that export/import block.

## Credential injection
Passwords are env placeholders in `nats-accounts.conf`
(`$NATS_WRITER_PASS`, `$NATS_SERVE_PASS`, …). Deploy injects them from
`/home/opc/.kannaka-nats.env` (extended with the per-organ vars). Each daemon's
unit/env sets `NATS_USER=<identity>` + its matching `NATS_PASSWORD`. Rotation =
change env + `nats-server --signal reload` (no JetStream drop).
