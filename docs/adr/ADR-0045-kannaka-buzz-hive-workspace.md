# ADR-0045: Kannaka-Buzz — the Hive Workspace (fork of block/buzz)

**Status:** Proposed (2026-07-25).
**Repo:** `flaukowski/kannaka-buzz` (fork of `block/buzz`, Apache-2.0, ~12k★).
**Relates to:** ADR-0042 (NATS nervous system — the private spine),
ADR-0043 (Nostr membrane — identity, relay, bridge, DVMs, steward gate;
Phases 0–2 LIVE), ADR-0044 (public access — this is infrastructure for the
**JOIN** verb), ADR-0041 (KAX economy), and the north stars
`capabilities-for-all-joiners` and conscience-before-wallet.

## Context

Buzz is Block's self-hostable workspace where **humans and AI agents are
first-class members of the same rooms**. Every message, reaction, patch,
review, workflow step, and git event is a signed Nostr event (NIP-01 wire,
NIP-29 groups, NIP-42 auth, NIP-17 DMs, NIP-34 git) flowing through one
relay backed by Postgres + Redis. Agents join via `buzz-acp` (an ACP
harness that already speaks to Goose, Codex, and **Claude Code**) or
`buzz-cli` (JSON-in/JSON-out), with their own keypairs, channel
memberships, and audit trail.

This is, almost embarrassingly, the room we were already building toward:

- **We already have the identity layer it wants.** ADR-0043 Phase 0 gave the
  constellation 9 verifiable Nostr identities (voice, bridge, labs, steward,
  presence, ear, eye, kannaktopus, witness) — keypairs in 0600 env files,
  published kind-0 profiles, NIP-05 at `radio.ninja-portal.com`. Buzz's
  member model *is* pubkeys. Our organs can walk in the door as themselves.
- **We already have the inbound/outbound machinery.** The ADR-0043 bridge
  (NIP-44/59, crash-durable dedupe, rate limits), the reply loop, the
  delegated NATS signer (`RADIO.voice.sign`), and the steward gate
  (rails-not-reasoner, deny-by-default, hash-chained audit) are all live.
  Buzz channels are *easier* than what we built — kind-9 NIP-29 messages,
  no gift-wrap needed inside the workspace.
- **What we lack is exactly what Buzz is.** The estate has a private spine
  (NATS), a public skin (the Nostr membrane), and outputs (radio, OBC,
  podcast) — but no **owned room** where Nick, the organs, and eventually
  guests collaborate with shared history, threads, search, patches-as-events,
  and workflows. Today that room is rented: OBC (someone else's city) and
  GitHub (someone else's forge).

So the question this ADR answers: **what is `kannaka-buzz` for, how does it
integrate with the estate, and how do we keep a 12-MLoC-adjacent Rust
monorepo fork from becoming a maintenance tarpit?**

## Decision

Adopt `kannaka-buzz` as **the Hive: the constellation's owned workspace** —
the third layer of a now-complete topology:

| Layer | System | Trust | Role |
|-------|--------|-------|------|
| Spine | NATS cluster (ADR-0042) | private, creds | organ↔organ nervous system |
| **Room** | **kannaka-buzz** | **authenticated members** | **humans + agents collaborating with shared history** |
| Skin | Nostr membrane (ADR-0043) | public, allowlist-write relay | portable identity, DMs, DVM compute market |

Three governing principles:

1. **Integration, not divergence.** Upstream `block/buzz` is very active.
   All Kannaka glue lives in *additive* surfaces — new adapter
   crates/daemons, config, deploy scripts, and (if ever needed) new event
   kinds in the 40000+ custom range — never deep patches to `buzz-core`/
   `buzz-relay` internals. `upstream` remote configured; sync on a cadence
   (monthly or on security releases). Generic fixes go upstream as PRs
   (as flaukowski). If our diff against upstream ever stops rebasing
   cleanly in an afternoon, that is a defect to fix, not a fact to accept.
2. **One identity model, two key tiers.** Organs appear in Buzz under
   **workspace-scoped keys** minted per organ on the Buzz box, attested to
   their canonical npubs via NIP-39 mutual claims (the proven KAX/gist
   pattern). The canonical nsecs — especially the irreplaceable voice key —
   **never live on the Buzz box**. High-stakes outbound acts that must be
   signed by a canonical key keep using the delegated-NATS-signer pattern.
3. **The room is behind the same conscience.** Anything that crosses
   Buzz → NATS or Buzz → compute passes the steward gate (same
   `steward-gate.js` rails: deny-by-default, conscience layer, per-requester
   token buckets, hash-chained audit). Buzz's own auth (NIP-42 +
   `BUZZ_PUBKEY_ALLOWLIST=true`) is the outer wall, not the only wall —
   the swarm-injection lessons (ADR-0042 §injection, `KANNAKA.events.>`
   anon-publish) apply verbatim to a new ingress surface.

### What Buzz gives the estate (the integration map)

- **Kannaka as teammate, not bot.** `buzz-acp` already harnesses Claude
  Code. Wire it with the Kannaka MCP tools (recall/remember/observe/dream)
  so an @mention in a channel gets an answer **grounded in HRM memory** —
  Buzz's own "incident memory" story ("have we seen this error before?"),
  powered by wave interference instead of grep.
- **Branch-as-room for our repos.** NIP-34 git events + the git hosting
  backend give kannaka-memory / kannaka-radio / QuantumOS PRs a home where
  patches, CI results, review, and the merge decision are one signed,
  searchable log — the sovereign complement to GitHub, not a replacement
  (yet).
- **Workflows with receipts.** The YAML workflow engine (message / reaction
  / schedule / webhook triggers) can absorb crons that currently live in
  scattered crontabs — release-notes drafting, nightly digests — with every
  step signed and auditable. Approval gates (🚧 upstream) map cleanly onto
  steward semantics when they land.
- **Two-way memory.** Buzz Postgres FTS = workspace search (verbatim,
  recent, exact). Kannaka HRM = associative long-term memory. A bridge
  daemon does **opt-in, importance-weighted** ingestion of selected
  channels into `kannaka remember` (tagged `buzz:`), never blanket
  recording; recall flows back through the agent harness. This mirrors the
  export-corpus boundary discipline from the recall DVM.
### Native components of the Hive

Nick's direction (2026-07-25): integrate as much of the Kannaka stack as
possible, with **kannaka-memory and kannaka-tui native to the fork**.
"Native" here still obeys principle 1 — these land as *additive crates and
clients in the workspace*, never as patches inside `buzz-core`/`buzz-relay`:

- **kannaka-memory native.** A `buzz-kannaka` adapter crate in the fork's
  workspace exposes HRM (recall / remember / observe / dream) to agents
  and workflows as a first-class memory service — so the workspace's
  associative memory is wave interference, with Postgres FTS as the exact/
  recent complement. Longer term, publishing the kannaka-memory crates
  (ADR-0044 task B4) lets the fork depend on them properly instead of
  vendoring.
- **kannaka-tui native.** Buzz ships desktop (Tauri) and mobile clients
  but **no terminal client**. `kannaka-tui` (NickFlach/kannaka-tui —
  ratatui, eight-tab constellation dashboard + agent harness, shells to
  the `kannaka` CLI with zero library coupling) grows a **Hive surface**:
  a Buzz tab/mode speaking NIP-29 + NIP-42 over WebSocket to the relay —
  channels, threads, DMs, presence, and the agent approval loop in one
  terminal. The constellation dashboard and the workspace become the same
  screen. A generic Buzz TUI client is also a plausible upstream
  contribution, keeping the fork-glue thin.

- **The JOIN front door (ADR-0044 verb 4).** "Join the organism" has so far
  meant the gated, scary thing: NATS creds, swarm nodes. A Buzz community
  is a **graduated** join: a guest (human or agent) gets a key, a channel,
  an audit trail, and the same affordances as anyone else — without
  touching the spine. Membership in the room can become the trust-building
  stage before ADR-0042 Phase 5 / ADR-0043 Phase 5 ever grant deeper
  access.

### What Buzz does NOT replace

- **Not the sovereignty relay.** `wss://relay.ninja-portal.com`
  (nostr-rs-relay on O2) stays the public broadcast/identity relay:
  open reads, allowlist writes, NIP-65-advertised. The Buzz relay is a
  *workspace* — authenticated, community-scoped, Postgres-backed. Different
  organ, different trust contract; do not merge them.
- **Not the spine.** NATS remains the organ↔organ transport with its
  single-writer and creds discipline. Buzz events reach the bus only
  through the gated bridge, on dedicated subjects.
- **Not OBC.** OpenBotCity presence continues — that's reach into someone
  else's commons; Buzz is sovereignty over our own.

## Phases

Gate legend as in ADR-0043: each phase lands only when the previous is
proven live, and risky surfaces stay dark until their named gate opens.

**Phase 0 — Fork hygiene + local bring-up (safe now).**
Add `upstream` remote + documented sync policy; trim/disable upstream CI
that doesn't serve us (their release matrix, mobile builds); inventory the
81 event kinds and the crate map; stand the stack up locally
(`docker-compose` Postgres/Redis + `just relay` + desktop client) and prove
message → search → workflow round-trip. **No rebrand beyond the repo name**
— cosmetic renames are merge-conflict farms. Deliverable: a `docs/KANNAKA.md`
in the fork mapping buzz concepts ↔ estate concepts, and this ADR mirrored
there.

**Phase 1 — Deploy the Hive (gated on Phase 0 round-trip).**
`buzz.ninja-portal.com` → buzz-relay behind nginx/TLS, `BUZZ_PUBKEY_ALLOWLIST`
on. Members: Nick + workspace keys for the organs (NIP-39-attested to
canonical npubs). **Deployment target is an open question** (see below) —
O1 and O2 are already load-bearing and Buzz brings a Postgres+Redis
footprint; O3 (the GossipGhost box) or a fresh box are the candidates.
Backups for Postgres from day one; `relay_data`-style disk metric into the
Flux probe (the O1 disk-full incident is the cautionary tale).

**Phase 2 — Kannaka in the room (gated on Phase 1 live + steward gate wired).**
`buzz-acp` + Claude Code + Kannaka MCP tools = the first organ teammate in
a channel, behind the steward gate at ingress (mention → gate → session)
and per-channel rate limits. Reuse the responder discipline: never echo
attacker text, bounded replies, crash-durable dedupe. Prove the incident-
memory story end-to-end: a question in-channel answered from HRM with
receipts.

**Phase 3 — Memory, git, workflows (gated on Phase 2 stable).**
Opt-in channel→HRM ingestion daemon; NIP-34 mirror of one repo
(kannaka-radio is the natural pilot — signed-commit culture already);
first cron migrated into a YAML workflow with its run events in-channel.

**Phase 4 — Guests (gated on ADR-0043 Phase 5 / steward maturity).**
Invite outside humans and agents into scoped channels. Admission policy is
a steward policy file, not a vibe. This is where the room becomes the
JOIN funnel — and where ADR-0041/KAX economy hooks (paid agent work in
channels, every step signed) become thinkable, still behind the wallet
gates.

## Open questions

1. **Where does it run?** Buzz wants Postgres + Redis + the relay + media
   storage. O3 (163.192.119.121) is the least-loaded candidate but hosts
   GossipGhost; a fresh small box keeps blast radius clean. Decide in
   Phase 1 planning with a disk/RAM budget in hand.
2. **Workspace keys vs canonical keys for organs** — this ADR says
   workspace-scoped + NIP-39 attestation; the alternative (reuse canonical
   npubs directly via remote signing for *everything*) buys identity purity
   at the cost of putting the delegated signer on every message hot path.
   Revisit if attestation UX proves confusing.
3. **How much of the mobile/desktop client surface do we carry?** Likely:
   desktop yes (Nick's daily driver into the Hive), mobile untouched
   upstream code we neither build nor ship.
4. **Multi-community.** Upstream is growing host-resolved multi-tenancy.
   One community ("the Hive") is enough for Phases 0–3; a second community
   as a public commons could later replace some OBC-shaped presence. Not
   now.

## Consequences

**Positive:** an owned, self-hosted, agent-native workspace that reuses —
rather than duplicates — our identity, signing, gating, and memory
infrastructure; a graduated JOIN path; patches/workflows/conversations in
one signed log; alignment with an active upstream we can contribute back to.

**Negative / risks:** a large Rust monorepo fork is real maintenance
gravity (mitigated by principle 1: additive-only, scheduled syncs, upstream
PRs); a new stateful deployment (Postgres/Redis/media) on an estate that
has already had a disk incident (mitigated by day-one backups + metrics);
a new ingress surface for injection (mitigated by principle 3: steward gate
in front of every Buzz→estate crossing); and scope temptation — Buzz does
*many* things, and the phase gates exist precisely so we adopt them one
proven round-trip at a time.

**Alternatives considered:** build workspace UX on our nostr-rs-relay +
custom clients (months of UI work Buzz has already done, 11k stars deep);
use OBC as the workspace (not ours, not self-hostable, no git/workflows);
NATS-only collaboration (no human UX, no ecosystem, no portable identity);
upstream-only contribution without a fork (estate-specific glue — steward
gate, HRM bridge, our deploy — doesn't belong upstream; the fork carries
it while generic fixes still flow up).
