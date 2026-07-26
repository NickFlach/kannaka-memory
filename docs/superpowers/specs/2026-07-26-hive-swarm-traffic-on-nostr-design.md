# Hive swarm traffic on `/nostr` — design

**Date:** 2026-07-26
**Status:** approved, ready for implementation planning
**Repos touched:** `flaukowski/kannaka-buzz`, `NickFlach/kannaka-memory`, `NickFlach/nats`
**Related:** ADR-0042 (NATS nervous system), ADR-0043 (Nostr interop membrane), ADR-0045 (kannaka-buzz Hive workspace), ADR-0046 (Hive membership unified auth)

## Problem

The `/nostr` page in the `nats` app monitors exactly one thing: the ADR-0043 DM
membrane. It subscribes to `KANNAKA.events.nostr.dm` and `.reply`, pairs them by
`rumor_id`, and backfills 30 days from the `KANNAKA_NOSTR` JetStream stream.

Swarm agents do not talk there. They talk in the Hive — `kannaka-buzz` — which is
a Nostr relay on a different layer, a different wire, and a different trust model.
Per the Hive's own `docs/KANNAKA.md`:

| Layer | System | Trust | Role |
|-------|--------|-------|------|
| Spine | NATS cluster | private, credentialed | organ ↔ organ nervous system |
| Room | kannaka-buzz | authenticated members | humans + agents collaborating |
| Skin | Nostr membrane | public relay | portable identity, DMs — *what `/nostr` shows today* |

`/nostr` should pick up Room traffic as well as Skin traffic. Nothing bridges the
two today: `kannaka-buzz` contains no NATS code at all.

## Decisions

Four decisions were settled during brainstorming; each is recorded with the
alternative that lost and why.

1. **Transport: bridge Hive → NATS**, rather than having the browser (or an app
   server route) speak Nostr directly to the buzz relay. The page keeps a single
   transport, JetStream history comes for free, and every other NATS consumer —
   pages, MCP tools, the TUI — gets the stream without further work. Cost: one
   new long-running daemon.

2. **Bridge home: `kannaka-memory`**, as a sibling binary to the existing
   `kannaka_nostr_bridge`. It reuses the NIP-01 signer in `src/nostr/`, the raw
   NATS publish helper, the env-config and systemd deploy pattern, and lands
   where the ADRs live. Cost: NIP-42 AUTH and NIP-29 `REQ` are hand-rolled over
   tungstenite instead of borrowing `buzz-ws-client`.

3. **Scope: broad bridging with an explicit per-channel opt-out.** Agent
   messages, human messages, the job lifecycle, workflow execution events, and
   the agent roster all cross. Channels marked no-bridge cross nothing. Observer
   frames and turn metrics are deferred (see "Deferred", below).

4. **Layout: a second panel, both membranes visible.** The left rail gains a
   `HIVE` stats block under `MEMBRANE`; the main column stacks `HIVE ROOMS`
   under `CONVERSATIONS`. The two streams have incompatible grouping — DMs fold
   by `rumor_id`, Hive folds by channel — so a unified timeline would destroy
   both structures. Existing DM code paths are untouched.

## Architecture

```
Hive (buzz relay)                     Spine (NATS)                    nats app
─────────────────                     ────────────                    ────────
kind 9 / 40002   messages   ─┐
kind 43001-43006 jobs       ─┤
kind 46001-46007 workflows  ─┼─ws──▶ kannaka_hive_bridge ──▶ KANNAKA.events.hive.msg       ─┐
kind 30177       agent roster─┤       NIP-42 AUTH                          .job             ├─▶ /nostr
kind 39000       channel meta─┘       NIP-29 REQ                           .workflow        │
                                      policy gate                          .agent          ─┘
                                                                                │
                                                                    KANNAKA_HIVE (JetStream, 90d)
                                                                                └──▶ getHiveHistory
```

One daemon, four subjects, one stream. The DM membrane path is unchanged.

**No NATS ACL change is required.** `config/nats-accounts.conf` already grants
`anon` subscribe on `KANNAKA.events.>`, so `KANNAKA.events.hive.*` is visible to
the browser's existing anon connection the moment the subjects enter the client
allowlist. The bridge publishes as `kannaka_internal`, exactly as
`kannaka_nostr_bridge` does today.

## Phase 1 — buzz relay changes

Both are framed as **generic upstream PRs to `block/buzz`**, not estate glue.
`KANNAKA.md` forbids patching `buzz-core`/`buzz-relay` internals as fork-local
changes but explicitly routes generic fixes upstream. Both qualify, so the fork
carries no permanent diff.

### 1a. Job lifecycle correlation

Commit `0398a39` mapped kinds 43001–43006 to `Scope::MessagesWrite` in
`crates/buzz-relay/src/handlers/ingest.rs`, making them writable and
channel-scoped via their `h` tag. It validates nothing else — there is no
enforced link from a 43002–43006 event back to the 43001 that started the job.
Any consumer must guess.

**Change:** in `required_scope_for_kind`'s neighbourhood in `ingest.rs`, add
validation that kinds 43002–43006 carry exactly one `e` tag resolving to a
stored kind-43001 event **in the same channel**. Reject otherwise with
`invalid: job event must reference its 43001 request`. Kind 43001 continues to
require only its `h` tag.

**Tests:** extend the existing `job_lifecycle_kinds_require_messages_write`
test module — accept a well-formed chain; reject missing `e`, multiple `e`,
`e` pointing at a non-43001, and `e` pointing at a 43001 in a different channel.

### 1b. Per-channel bridge policy

**Change:** a nullable `no_bridge` boolean on the channels table (migration),
settable via a kind-9002 group-metadata edit carrying a `no-bridge` tag, and
emitted on the relay-signed kind-39000 metadata event when set.

**Authz:** owner/admin only — the same tier as `name`/`about`, deliberately not
the any-member tier that `topic`/`purpose` use. Export control is not a casual
edit.

**Default:** absent means bridgeable. Broad bridging is the norm, per decision 3.

**Fail-closed:** the bridge does not bridge a channel whose policy it has never
resolved. An unreadable or unrefreshed policy means "do not export", never
"export by default". See the polling caveat in Phase 2.

## Phase 2 — `kannaka_hive_bridge`

New binary `src/bin/kannaka_hive_bridge.rs`, new library module `src/hive/`.
Same discipline as the DM bridge: **the binary is network plumbing, the logic is
library code with unit tests.**

### Startup sequence

The ordering is not incidental — two of these steps exist to close real races.

1. Connect; respond to the relay's `AUTH` challenge with a kind-22242 event
   (`relay` + `challenge` tags) signed via `Keypair::sign` from `src/nostr/`.
   No new crypto.
2. Historical `REQ` for kind 39000 → channel policy map + channel names → EOSE.
3. Historical `REQ` for kinds 30177 and 0 → agent roster (pubkey set + display
   names) and human display names → EOSE.
4. Live `REQ` for `{"kinds":[0,9,40002,43001..43006,46001..46007,30177]}`. No
   `#h` filter — the relay scopes by membership, so the bridge receives exactly
   the channels it belongs to. Kinds 0 and 30177 stay in the live filter so
   names and roster updates arrive without waiting for the next poll; neither is
   forwarded to NATS as its own subject except 30177, which feeds
   `.agent`.
5. Every `HIVE_POLICY_REFRESH_SECS` (default 60), repeat step 2.

**Why steps 2 and 3 precede step 4:** the roster is built from the same stream
it filters against, so opening the message subscription first would mean
mislabelling the first messages of an agent not yet learned. And the policy map
must be populated before any event is eligible to cross.

**Why step 5 polls rather than listens:** `NOSTR.md` states that channel-scoped
storage means live global subscriptions do **not** receive kind-39000 via
fan-out — clients discover groups by historical `REQ`. A bridge that subscribed
live and waited would never see a `no-bridge` flag set after startup. This is
the single most important correctness detail in the bridge.

### Per-event handling

```
resolve channel from `h` tag
  ├─ channel unknown, or no_bridge set  → drop (fail-closed)
  └─ otherwise
       ├─ dedupe by event id (crash-durable, reusing src/nostr/bridge.rs Dedup)
       ├─ rate-limit per author (reusing RateLimiter)
       ├─ map kind → subject + payload
       └─ publish to NATS
```

`is_agent` on message payloads is set from roster membership. Human messages
cross too (decision 3) — the `no-bridge` flag, not authorship, is the privacy
control.

### Config (env only, mirroring the DM bridge)

| Var | Default |
|---|---|
| `HIVE_RELAY_URL` | — (required) |
| `HIVE_KEY_FILE` | — (required, 0600 json `{privkey,pubkey}`) |
| `HIVE_DEDUPE_FILE` | `/var/lib/kannaka-hive-bridge/dedupe.log` |
| `HIVE_NATS_URL` / `_USER` / `_PASS` | `nats://127.0.0.1:4222` |
| `HIVE_SUBJECT_PREFIX` | `KANNAKA.events.hive` |
| `HIVE_POLICY_REFRESH_SECS` | `60` |
| `HIVE_RATE_CAP` / `_REFILL` | `20` / `1.0` |

### Deployment prerequisite

buzz stores channel events channel-scoped and enforces access control on read.
**The bridge's pubkey must be allowlisted and a member of every room to be
mirrored.** It cannot observe rooms it has not joined. This is Hive-side
membership administration, not code, and it gates any useful output.

## Phase 3 — the `/nostr` page

- **`src/lib/nats/schema.ts`** — four subjects appended to `SUBJECTS`, four
  payload interfaces (below).
- **`src/lib/nats/hive.ts`** *(new)* — pure fold,
  `(msgs, jobs, workflows, roster) → HiveRoom[]`. Each room holds a
  time-ordered mix of message rows, collapsed job cards, and workflow rows.
  Kept out of the component so it is testable and so `nostr.tsx` does not double
  in size.
- **`src/lib/nats/hive-history.functions.ts`** *(new)* — `getHiveHistory`, a
  near-copy of `getNostrHistory` against stream `KANNAKA_HIVE`: same
  `requireSupabaseAuth` gate, same ephemeral-consumer idiom, same best-effort
  posture where an absent stream yields `[]` and the live view still renders.
- **`src/routes/nostr.tsx`** — `HIVE` stats block in the left rail (agents,
  rooms, jobs active/done, workflows); `HIVE ROOMS` panel below
  `CONVERSATIONS`. Agent rows accented, human rows neutral, job cards showing
  the phase chain, workflow rows compact.

### Orphan job events

Once Phase 1a lands, a job event with no resolvable `job_id` means either a
legacy row or a relay bug. These render **dimmed, in an "unlinked" group at the
bottom of the room** rather than being dropped — silently discarding them would
make a relay defect look like an empty panel, which defeats the purpose of a
monitor.

### Empty state

Job and workflow kinds are new and may have no producers yet. The empty state
must say that plainly rather than reading as a broken page.

## Phase 4 — `hive_traffic` MCP tool

`src/lib/mcp/tools/hive-traffic.ts`, mirroring `nostr-traffic.ts`: short-lived
NATS connection, listens on the four hive subjects for a bounded window, folds
into rooms and job chains, returns `structuredContent`. Wrapped in `withAudit`,
gated by `scopeDenied(ctx, "mcp:read")`, `readOnlyHint: true`.

Live-window only, matching `nostr_traffic`'s shape. History access via the
`KANNAKA_HIVE` stream is a later addition.

## Subject and payload contract

```
KANNAKA.events.hive.msg       → HiveMsgEvent
KANNAKA.events.hive.job       → HiveJobEvent
KANNAKA.events.hive.workflow  → HiveWorkflowEvent
KANNAKA.events.hive.agent     → HiveAgentEvent
```

```ts
export interface HiveMsgEvent {
  type: "hive_msg";
  event_id: string;        // source nostr event id — provenance
  channel_id: string;      // h tag (uuid)
  channel_name?: string;   // resolved from kind 39000
  author_hex: string;
  author_npub: string;
  author_name?: string;    // 30177 roster for agents, kind-0 profile for humans
  is_agent: boolean;       // author is in the managed-agent roster
  content: string;
  reply_to?: string;       // e tag marked "reply" (NIP-10)
  created_at: number;      // seconds, from the nostr event
  ts: number;              // ms, bridge receive time
}

export interface HiveJobEvent {
  type: "hive_job";
  event_id: string;
  channel_id: string;
  channel_name?: string;
  phase: "request" | "accepted" | "progress" | "result" | "cancel" | "error";
  kind: number;            // 43001..43006
  job_id: string | null;   // the 43001 event id; null only for legacy/malformed
  author_hex: string;
  author_npub: string;
  author_name?: string;
  content: string;
  created_at: number;
  ts: number;
}

export interface HiveWorkflowEvent {
  type: "hive_workflow";
  event_id: string;
  channel_id?: string;
  phase: "triggered" | "step_started" | "step_completed"
       | "step_failed" | "completed" | "failed" | "cancelled";
  kind: number;            // 46001..46007
  run_id: string | null;   // first e tag resolving to the 46001; null if absent
  workflow?: string;       // workflow definition id, from its `d`/`a` tag
  step?: string;           // step name, on 46002-46004
  content: string;
  created_at: number;
  ts: number;
}

export interface HiveAgentEvent {
  type: "hive_agent";
  d_tag: string;           // NIP-33 identifier
  agent_hex: string;
  agent_npub: string;
  name?: string;
  persona?: string;
  ts: number;
}
```

Every payload carries `event_id` and the author pubkey so the page can show
provenance — see the forgeability risk below.

## Ops

`KANNAKA_HIVE` stream created on the cluster by hand, mirroring how PR #3
created `KANNAKA_NOSTR`: subjects `KANNAKA.events.hive.>`, R3, File, Limits/Old,
90-day retention, 2m dedup.

systemd unit for `kannaka-hive-bridge` cloned from the DM bridge's, with the
same 0600 key-file custody.

## Verification

**Rust (`src/hive/`)** — this is where the testable logic lives, following how
`src/nostr/bridge.rs` is CI-tested while its binary stays plumbing:

- kind → subject/payload mapping, one fixture event per kind
- roster gating: `is_agent` true/false resolution
- policy gating: known-bridgeable crosses; `no_bridge` set drops; **unknown
  channel drops** (fail-closed)
- job `job_id` extraction, including the `null` orphan path
- malformed / unsigned / wrong-channel events rejected
- dedupe across a simulated reconnect

**buzz** — ingest validation tests per Phase 1a.

**TypeScript** — the `nats` app has **no test runner**; `package.json` exposes
only `dev`, `build`, `lint`, `format`. Verification is `tsc --noEmit` plus a
fixture script for the `hive.ts` fold following the repo's existing
`scripts/test-agent-mcp.mjs` pattern. Adding a runner is out of scope here; if
one is wanted, that is its own change.

**End to end** — post a message and run a job chain in a bridged Hive channel;
confirm both appear on `/nostr` live and survive a reload via `getHiveHistory`.
Then set `no-bridge` on that channel and confirm traffic stops within
`HIVE_POLICY_REFRESH_SECS`.

## Risks

- **Subjects are forgeable.** `anon` holds publish on `KANNAKA.events.>`, so
  anyone with anon credentials can inject fabricated hive events — the same
  exposure `KANNAKA.events.nostr.*` already carries. Payloads include
  `event_id` and author pubkey for provenance. The real fix is a scoped bridge
  identity plus a tightened anon publish list, which is ADR-0042 Phase 1b work
  and out of scope here.
- **Agent and human text is untrusted input.** The page renders it as text, as
  the DM panel already does. No `dangerouslySetInnerHTML`.
- **Human messages reach the bus and 90-day retention.** This is intended, and
  the `no-bridge` flag is the control. It only works if channel owners know the
  flag exists — worth surfacing in the Hive UI, not only on the wire.
- **Fail-closed depends on the 60s poll.** A channel flagged `no-bridge` keeps
  exporting for up to one refresh interval. Lower the interval if that window
  matters; closing it entirely needs relay-side live delivery of 39000, which is
  noted upstream as a future enhancement.
- **Empty panels initially.** Job kinds became writable on 2026-07-25 and
  nothing in the fork emits them yet.

## Deferred to a follow-up spec

Observer frames (24200) and agent turn metrics (44200) are **NIP-44 encrypted to
the agent's owner**, per `buzz-core/src/observer.rs` and the
`KIND_AGENT_TURN_METRIC` doc comment. Reads are `p`-gated. The bridge cannot
decrypt either by being a room member; it needs the owner secret key — a key
that also authorises `OBSERVER_FRAME_CONTROL`, and therefore confers the ability
to *drive* agents, not merely observe them. Kind 24200 is additionally in the
ephemeral range (20000–29999) and is **never stored**, so it can only ever be
tailed live, never backfilled.

That combination — a custody decision, no history, and roughly 10–100× the event
volume — is a different design problem and gets its own spec.

## To confirm during planning

Facts asserted above were read from source; these were not, and should be
checked before the plan hardens:

- Exact emission site for kind 39000 (`crates/buzz-relay/src/handlers/side_effects.rs`
  is the likely file but was not read).
- Whether workflow execution events 46001–46007 are channel-scoped via `h` or
  stored globally. They appear in the stored-kinds list; scoping affects whether
  `channel_id` on `HiveWorkflowEvent` is reliably present.
- The actual tag shape of 46001–46007. `run_id`, `workflow`, and `step` on
  `HiveWorkflowEvent` are assumed to come from an `e` tag, a `d`/`a` tag, and a
  step-name tag respectively; the workflow engine's emitter
  (`crates/buzz-workflow/`) is authoritative and was not read.
- Whether kind-30177 managed-agent projections are readable by any authenticated
  member or are owner-scoped. `desktop/src/shared/constants/kinds.ts` describes
  them as secrets-stripped snapshots that an inbound sync hook subscribes to,
  which suggests broad readability — but if they are owner-scoped, roster
  resolution needs another source and `is_agent` degrades.
