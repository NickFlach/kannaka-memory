# Seat the `0xscada-qe` organ in the Hive — design

**Date:** 2026-07-26
**Status:** approved, ready for implementation planning
**Repos touched:** none — this is an operational change plus two published Nostr events
**Related:** ADR-0043 (Nostr interop membrane), ADR-0045 (kannaka-buzz Hive workspace),
ADR-0046 (Hive membership & unified auth), and the
`2026-07-26-hive-swarm-traffic-on-nostr` spec, whose bridge consumes what this
one publishes.

## Problem

A workspace-scoped Nostr keypair for the organ `0xscada-qe` was minted and
handed over as `~/Downloads/kannaka-hive-0xscada-qe-nostr.json`:

```json
{ "nsec": "...", "npub": "...", "pubkey": "<64 hex>", "organ": "0xscada-qe" }
```

This is the ADR-0045 principle-2 artifact — "organs appear in Buzz under
workspace-scoped keys minted per organ on the Buzz box." The key exists. The
organ is not yet seated behind it:

- The `nsec` is sitting in plaintext in a Downloads folder.
- Nothing on this box has ever connected to the Hive relay as this organ.
- The relay has no profile for the pubkey, so the organ would appear as a bare
  hex string.
- Critically, no kind-10100 agent profile exists, so nothing marks this pubkey
  as an **agent** rather than a human.

This box is `0xSCADA-QE`: `~/.kannaka/config.toml` sets `agent.id = "0xSCADA-QE"`
with `swarm.role = "queen"`. It has an identity on the Spine (NATS) and on
OpenBotCity, but none in the Room.

## Scope

**In:** seat this one organ. Custody the key, prove the relay accepts it, prove
what rooms it can see, and publish the two events that make it recognizable.

**Out:** the generalized per-organ identity layer (canonical on-disk key format,
a `kannaka` CLI path to load an organ key, `config.toml` wiring, the same rail
for radio/eye/staff). That is a real and probably next piece of work, but it is
a different spec. This one deliberately adds no mechanism.

## Decisions

Three decisions were settled during brainstorming; each is recorded with the
alternative that lost and why.

1. **Seating is defined as four ordered states, each with its own proof.** Not
   "it connected." See the table below. The point is that a partial result is
   legible — knowing we reached state 2 but not state 3 tells you exactly whose
   action unblocks it.

2. **Tooling is a throwaway operator script in the scratchpad**, not a new
   `kannaka` subcommand and not the Hive web UI.

   - *Rejected — `kannaka nostr publish` in Rust:* durable and it would reuse
     `Keypair::sign_event`, but it is a repo change, a rebuild, and a new CLI
     surface to maintain. That is the generalized identity layer, explicitly out
     of scope here.
   - *Rejected — paste the nsec into the Hive web SPA:* zero code, but manual,
     it puts the nsec through a browser, and the SPA publishes a kind-0 profile
     only. No kind-10100 means the organ seats as a *person*, which fails state
     4 — the one state that actually matters downstream.

   The durable artifact of this work is two signed events on the relay. The
   script is a wrench, and it is not committed to any repo.

3. **Verify allowlisting; do not assume it.** ADR-0045 says organ keys are
   minted on the Buzz box, which makes prior allowlisting likely — the key would
   not otherwise be useful. "Likely" is not a proof, and the failure is silent
   from this side (a closed socket), so state 2 gets an explicit check.

## The four states

The organ is seated when all four hold. They are ordered; each depends on the
one above it.

| # | State | Proof |
|---|-------|-------|
| 1 | Key in custody | nsec at `~/.secrets/kannaka-hive-0xscada-qe-nostr.json`, ACL'd to the owning user only, Downloads copy removed |
| 2 | Allowlisted | relay accepts a NIP-42 `AUTH` (kind 22242) instead of closing the socket |
| 3 | A member of rooms | `REQ {"kinds":[39000]}` returns ≥ 1 channel |
| 4 | Recognized as itself | kind-0 profile and kind-10100 agent profile published, then read back from the relay |

### Why kind 10100 is the load-bearing one

State 4's kind-10100 is not cosmetic. `buzz-core/src/kind.rs` documents it as
"Agent metadata + owner reference (replaceable, agent-authored)", keyed by the
agent's *own* pubkey. The hive-bridge's `roster.rs` — the module that decides
`is_agent` on every bridged message — reads exactly this kind, per the
`2026-07-26-hive-swarm-traffic-on-nostr` spec.

Publishing it now means that when the bridge daemon runs, this organ is
classified correctly from its first event, with no backfill. Publishing only
kind 0 would leave the organ permanently rendered as a human in the `/nostr`
HIVE ROOMS panel.

## Architecture

```
~/Downloads/kannaka-hive-0xscada-qe-nostr.json   (plaintext secret — remove)
        │
        │ state 1: move + ACL
        ▼
~/.secrets/kannaka-hive-0xscada-qe-nostr.json    (0600-equivalent)
        │
        │ read by
        ▼
scratchpad/seat-organ.mjs ──wss──▶ buzz.ninja-portal.com
        │                              │
        │  state 2: AUTH (kind 22242)  │
        │  state 3: REQ kinds [39000]  │
        │  state 4: EVENT kind 0       │──▶ relay storage
        │           EVENT kind 10100   │        │
        │                              │        │
        └──── read-back REQ ◀──────────┴────────┘
```

No repo is modified. No daemon is installed. Nothing runs after the script
exits — the state lives on the relay.

## Relay facts (verified, not assumed)

`GET https://buzz.ninja-portal.com` with `Accept: application/nostr+json`
returns HTTP 200 and a NIP-11 document:

- `"name": "Buzz Relay"`
- `supported_nips` includes **42** (auth) and **29** (relay-based groups)
- `h_grammar: "uuid-v4-lowercase"` — channel ids are lowercase UUID v4
- `max_content_len: 65536`, `max_authors: 20`

So the relay is live, speaks the two NIPs this work depends on, and the `h`-tag
grammar matches what the bridge spec's mapper expects.

## Custody

`~/.secrets/` is the established precedent on this box —
`~/.secrets/moltbook-0xscada-qe.json` holds the same organ's Moltbook API key in
the same shape (flat JSON, one service, named for the organ). This work follows
it rather than inventing a location.

The file keeps its delivered field names (`nsec`/`npub`/`pubkey`/`organ`). It is
**not** reshaped into the `{privkey, pubkey}` form that the hive-bridge's
`HIVE_KEY_FILE` expects.

That mismatch is real and is deliberately left alone. Reconciling it means
choosing a canonical on-disk organ-key format — a decision that should be made
once, across all organs, in the identity-layer spec. Making it here, for one
organ, on the way past, is how two conflicting conventions get born. The bridge
is not running yet, so nothing is blocked by deferring it.

Windows has no `chmod`. State 1's ACL is `icacls` with inheritance disabled and
a single grant to the owning user, which is the practical equivalent of 0600.

## Failure modes

Both of the plausible failures are *other people's actions*, not defects. Each
is a stop, not a workaround:

- **AUTH refused at state 2** — the pubkey is not on the relay's allowlist. That
  table is Postgres on the Buzz box (`flaukowski/kannaka-buzz`); there is no SSH
  access from this machine and no `ninja-portal` entry in `~/.ssh/known_hosts`.
  Stop, surface the npub, and ask for it to be allowlisted.
- **AUTH succeeds, state 3 returns zero rooms** — allowlisted but not invited to
  any channel. That is a kind-9000 invite from a room admin. Not issuable from
  here.

States 1 and 4 complete regardless of either, so a partial run is still
progress: the key ends up in custody and the identity events are published and
waiting. Neither failure should be papered over by, for example, retrying
without auth or publishing to a different relay.

## Verification

Every state is proven by observed output, never inferred from an absent error:

1. **Custody** — the target file exists and parses as JSON with the expected
   four fields; `icacls` output shows exactly one non-inherited user grant; the
   Downloads path no longer exists.
2. **Allowlisted** — the relay's `OK`/`AUTH` handshake completes and the socket
   stays open. A relay that closes on AUTH is a *failure*, and silence is not a
   pass.
3. **Rooms** — the channel list from the 39000 `REQ` is printed, with count. Zero
   is reported as zero, not smoothed over.
4. **Recognized** — after publishing, a fresh `REQ` for
   `{"kinds":[0,10100],"authors":["<pubkey>"]}` returns both events, and each
   one's `id` and signature verify. Read-back, not the publish `OK`, is the
   proof — an `OK` means accepted, not stored and served.

The script self-verifies every event it signs before sending, matching the
existing discipline in `src/bin/handlers/nostr.rs` ("never emit an event we
can't verify").

## Risks

- **The nsec is currently in plaintext in Downloads**, and has been since it was
  delivered. Moving it reduces future exposure but does not undo past exposure.
  If that key is considered compromised, the answer is a fresh mint on the Buzz
  box, not this spec.
- **The script handles the nsec in process memory and must never log it.** It
  prints npub and hex pubkey — public values — and nothing else about the key.
  This mirrors `kannaka nostr keygen`, which prints the nsec exactly once and
  never persists it.
- **kind 0 and kind 10100 are replaceable events**: the newest wins. If a
  profile already exists for this pubkey, publishing clobbers it. The run
  therefore reads existing kind-0/10100 for the pubkey *before* publishing, and
  reports what it found. `handlers/nostr.rs` already encodes this hazard in its
  `--content-json` flag, which exists precisely so a republish does not clobber
  fields it doesn't know about.
- **No NIP-39 attestation.** This workspace key is not cryptographically bound
  to any canonical `0xSCADA-QE` npub, so it carries no transferable reputation.
  ADR-0045 principle 2 wants that binding. It needs a canonical npub, and no
  evidence of one was found on this box. Deferred.

## Deferred

- **The generalized organ identity layer** — canonical on-disk key format,
  `kannaka` CLI load/sign/publish path, `config.toml` wiring, the same rail for
  the other organs. Its own spec.
- **NIP-39 mutual attestation** to a canonical npub (above).
- **Reconciling the key-file shape** with the bridge's `HIVE_KEY_FILE`
  `{privkey, pubkey}` expectation — belongs to the identity-layer spec, for the
  reason given under Custody.
- **Running the hive bridge itself.** That work has its own spec and plan, is
  partially implemented (`map.rs` and `policy.rs` committed, `roster.rs` still a
  stub, daemon not yet written), and is not gated on this one beyond wanting the
  10100 to exist first.
