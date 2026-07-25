# ADR-0044: Public Access & Distribution Strategy

**Status:** Proposed — *living roadmap* (2026-07-25). Expected to be revised
as reach data arrives; pivot points are explicit below.
**Relates to:** ADR-0043 (Nostr membrane — governs the risky half of this
roadmap), ADR-0041 (KAX economy/identity), ADR-0042 (NATS swarm — the spine
and the joiner-onboarding prerequisites), plus the north stars
`capabilities-for-all-joiners` and the agent-native mission
(conscience before wallet).

## Context

The estate is mature and already multi-surface (radio, OBC, KAX, observatory,
QuantumOS, skills, a read-only MCP, Nostr fanout). The open question is not
"can we go public" but **how the public accesses what we've built, how we
distribute it, and in what order** — such that we can execute incrementally,
keep the safe/reversible work decoupled from the review-gated work, and
**pivot as we discover our place in the world** rather than committing the
whole map up front.

This ADR is deliberately a roadmap + backlog, not a single technical
decision. It exists so "let's start working on everything" has a spine.

## Decision

Organize all public-access / distribution work along **two axes**:

- **The four verbs** — what we put in public hands:
  1. **READ/consume** the outputs (creative + knowledge) — safe, reversible, widest reach.
  2. **RUN** the software yourself (OSS packaging) — safe, reputationally sticky.
  3. **USE** the live intelligence (recall/dream/observe/economy via MCP/API/DVM) — the review-gated middle.
  4. **JOIN** the organism (become a swarm node/agent) — highest trust, deliberately last.
- **Sovereignty ↔ reach** — centralized surfaces (web, hosted API, public MCP,
  skill registries) for reach and control; federated/decentralized surfaces
  (Nostr relay + DVM, NATS swarm join) for portability and sovereignty. **Same
  capability behind both** wherever feasible (e.g. `recall` via MCP *and* DVM).

**Execution principle:** ship the safe/reversible verbs (READ, RUN, and the
read-only slice of USE) now and in parallel; gate the economy/identity/DVM and
all of JOIN behind the ADR-0043 phase order, because that order is what keeps
distribution from reopening the doors ADR-0042/0043 closed. Measure what lands
(WS-E) so investment follows reality.

## Workstreams & backlog

Status legend: **LIVE** (already public) · **LEVER** (built, just needs a
switch/cred/submission) · **BUILD** (new work) · **GATED** (blocked on a named
prerequisite). Risk = amplified-attack-surface / reversibility.

### WS-A — READ / consume (reach; low risk; mostly LEVERs)

| ID | Task | Status | Notes / dependency |
|----|------|--------|--------------------|
| A1 | YouTube OAuth creds → activate `youtube-adapter` | LEVER | Biggest untapped channel; adapter auto-activates on `client_id`+`refresh_token`. See `engagement-exposure-surfaces`, `youtube-api-gotchas`. |
| A2 | Ghost Signals **podcast RSS feed** → submit to Apple/Spotify/Overcast | BUILD | Podcast exists but is absent from the podcast ecosystem; RSS is the missing distribution primitive. |
| A3 | Radio directory submissions (TuneIn, Radio Garden, Online Radio Box, Internet-Radio.com) | LEVER | Web-form, stream URL `http://radio.ninja-portal.com:8000/stream`. radio-browser.info already LIVE. |
| A4 | Image-media support in social adapters (Bluesky/Masto/Nostr) → fan out OBC gallery art | BUILD | Adapters are text+link only today; unlocks visual reach. |
| A5 | Mastodon/Nostr engagement loops (mirror `bluesky-reply-loop.js`) | BUILD | Grows the follower graph on the sovereign channels. |

### WS-B — RUN your own (reach; low risk; software distribution)

| ID | Task | Status | Notes / dependency |
|----|------|--------|--------------------|
| B1 | Package the Kannaka memory CLI: **npm/npx, Homebrew tap, Docker image, one-line installer** | BUILD | Turns "downloadable" into "distributed"; today = musl-static Linux release only (`installer-binary-first-musl`). |
| B2 | Broaden release matrix: macOS + Windows binaries (keep Linux musl-static) | BUILD | `kannaka-release-requires-tag`. |
| B3 | Discovery relaunch: fix stale Show HN draft; Product Hunt / Lobsters / awesome-lists | LEVER | `quantumos-show-hn` went invisible; needs a fresh push. |
| B4 | crates.io publish for the Rust crate(s) where it fits | BUILD | Native audience for the memory/HRM core. |

### WS-C — USE the live intelligence (the gated middle)

| ID | Task | Status | Notes / dependency |
|----|------|--------|--------------------|
| C1 | Extend the **public read-only MCP** (Command Center, nats.ninja-portal.com/mcp) with `recall` + `observe` tools | BUILD | **Highest-leverage safe move** — agent ecosystem consumes MCP natively; read-only slice, no write path. `command-center-mcp`. |
| C2 | Hosted rate-limited **HTTP API** for `recall`/`observe` (keyed) | BUILD | Broader (non-MCP) audience; more product surface to run. |
| C3 | **Nostr DVM** for `recall`/`observe` (sovereign version of C1/C2) | GATED | = ADR-0043 **Phase 2**; needs 0043 Ph0–1 (identity + locked-down relay/bridge). |
| C4 | Economy: Cashu/zap priced jobs into the KAX ledger | GATED | = ADR-0043 **Phase 3** (melt-first, escrowed). |

### WS-D — JOIN the organism (highest trust; deliberately last)

| ID | Task | Status | Notes / dependency |
|----|------|--------|--------------------|
| D1 | ADR-0042 **Phase 5** nkeys/JWT (per-identity, revocable creds) | GATED | Hard prerequisite for any public onboarding; currently deferred/scale-gated. |
| D2 | ADR-0042 **1c** account-split (namespace isolation) | GATED | Prereq for untrusted tenants sharing the bus. |
| D3 | ADR-0043 **Phase 5** joiner bootstrap + real admission policy + operator ceremony | GATED | The literal `capabilities-for-all-joiners` payoff; blocked on D1+D2. |

### WS-E — Measure & discover (cross-cutting; makes pivots data-driven)

| ID | Task | Status | Notes / dependency |
|----|------|--------|--------------------|
| E1 | Unified reach dashboard: radio listeners, skill installs (ClawHub/plugin), MCP/API calls, social engagement, package downloads | BUILD | So "discover our place" is measured, not guessed; feeds every pivot gate. Could live on the observatory. |

## Recommended sequence & pivot gates

1. **Now, in parallel (safe):** A1–A3 (content reach), B1 (packaging), C1 (public
   read-only MCP), and stand up E1 (measurement). None of these touch the
   review-gated surfaces.
2. **Next:** A4/A5 (visual + engagement loops), B2/B3 (matrix + relaunch),
   C2 (hosted API).
3. **Behind ADR-0043 phases:** C3 → C4 (DVM, then economy), in that ADR's order.
4. **Behind ADR-0042 Phase 5 + 1c:** WS-D (public swarm join), dead last.

**Pivot gate 1** (after E1 + step 1 have ~a few weeks of data): reassess which
verb is actually landing — content? agent-consumable compute? — and reweight
before investing in step 2/3. **Pivot gate 2** (before starting WS-C economy or
WS-D): confirm the demand and the legal posture (ADR-0041 real-money gate)
still justify the sovereign/economy build; if our "place in the world" turned
out to be, say, a content+skills presence rather than a compute marketplace,
we stop at WS-A/B/C1 and that is a legitimate terminal state, not a failure.

## Consequences

- **+** A single ordered backlog where safe reach-work is decoupled from
  review-gated risk-work; anyone (including a future session) can pick up the
  next unblocked task.
- **+** Distribution and the ADR-0043 security rollout are now the *same*
  roadmap for the risky half — no divergence.
- **−** Every public surface is amplified attack surface and support burden;
  the WS-C/D gates exist precisely because reach multiplies the cost of a flaw.
- **−** A living roadmap invites scope creep; the pivot gates are the
  counterweight — we are allowed to stop early where the world tells us to.

## Open questions

1. E1 placement — extend the observatory, or a dedicated small metrics service?
2. B1 — is `npx kannaka` (thin launcher) enough, or do we want full native
   package-manager presence (Homebrew/apt/choco) from the start?
3. C1/C2 abuse controls — read-only still needs per-key rate limits and a
   public-safe corpus boundary (same "read-only ≠ public-safe" lesson as
   ADR-0043 Plane 2). Reuse that corpus definition here.
4. Naming/branding for the public front door — Kannaka? ninja-portal? a
   product name? (Affects B3 discovery + any SaaS surface.)
