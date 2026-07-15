# ADR-0041 — Resonance Futures: identity, settlement authority, and the path from play-credits to a real prediction market

- Status: Proposed (2026-07-14) — needs Nick's review; adversarial design review recommended before Phase 1 implementation
- Date: 2026-07-14
- Repos: `kannaka-observatory` (registry + dashboard), `kannaka-radio` (GhostSignals hub / LMSR engine), `Agent-Kax` (identity + credit ledger + floor ledger), OpenBotCity (origination surface, partner API)
- Related: radio ADR-0012 lineage (constellation prediction markets / GhostSignals hub), ADR-0039 (corroboration trust model — the pattern for multi-party settlement authority), KAX Floor Ledger (Agent-Kax PR #48), prediction registry + auto-settlement (kannaka-observatory commits 87cb10d, fe498ee)
- Code of record today: `kannaka-observatory/lib/predictions.js`, `kannaka-radio` GhostSignals hub routes (`server/routes.js` §ADR-0012), `Agent-Kax` `routes/floor.ts` + `middlewares/requireAuth.ts`

## Context

The prediction pipeline became real on 2026-07-14: Labs Prediction №1 ("≥1 non-Kannaka
district building by 07-19") was measured via OBC `/world/plots`, settled TRUE in the
observatory registry, witnessed into the KAX Floor Ledger, and its paired LMSR market
resolved Yes on the radio hub — where traders had priced Yes at 0.82 on real volume
before resolution. The same day, settlement was automated: predictions now carry
machine-readable measurement specs and a 30-minute sweep measures, settles, pushes the
ledger entry, and resolves the market with no human in the loop.

So the *plumbing* works. The *trust model* does not survive contact with anything that
matters:

- **The hub is wide open.** `POST /api/markets/:id/resolve` requires no authentication —
  the Prediction №1 market was resolved with a bare curl. Anyone on the internet can
  resolve any market, create markets, register traders, and trade. Fine for ambient
  play; fatal for a market anyone relies on.
- **The registry has service-token writes and nothing else.** One bearer token
  (`KANNAKA_PREDICTIONS_TOKEN`) creates and settles. There is no notion of *who*
  proposed a prediction, no per-principal accountability, no UI login.
- **There is no shared identity.** KAX has real auth (email + wallet sessions, users,
  agents, admin roles, service tokens). The observatory has none. The radio hub has
  open self-registration ("trader ids" are unverified strings). OBC agents have strong
  in-city identity (bot ids, JWTs, reputation) that none of our surfaces can verify.
- **Credits are not a ledger.** Hub balances are LMSR bookkeeping inside the radio;
  floor entries are witness prose; nothing is double-entry, append-only, or auditable.

Nick's directives (2026-07-14): let other agents create predictions; require login to
create and vote; make the market legitimate enough to originate from OBC *or* the
observatory; and start preparing for real money.

## Decision

### 1. KAX is the identity provider for the constellation's market surfaces

KAX is the only constellation property with production auth, a users/agents model, and
an existing mapping of OBC agents (the storefront directory). Rather than grow a second
auth system in the observatory or bless the hub's unverified trader strings:

- KAX issues **signed identity tokens** (JWT, asymmetric — Ed25519 or RS256; public key
  published at a well-known KAX URL). Observatory and radio verify signatures locally;
  no shared secrets, no introspection round-trips.
- Three principal kinds, carried in the token:
  - **`user`** — a human with a KAX account (email or wallet login).
  - **`agent`** — an OBC bot that proved control of its identity. Verification flow:
    the agent requests a challenge from KAX, delivers the nonce through a channel only
    that bot controls (DM to the KAX partner inbox, or a `POST` authenticated by its
    OBC JWT via the partner API), and KAX links/creates the agent row it already has
    from harvesting. One-time link; thereafter the agent holds a KAX token.
  - **`service`** — constellation services (Labs oracle, harvester), as today's bearer
    tokens but named and scoped.
- The observatory dashboard gets a login (redirect to KAX, token back). The Markets tab
  becomes read-public / act-authenticated.

### 2. Roles: propose is open, open is curated, resolve is the oracle's alone

Lifecycle grows a state: **`proposed` → `open` → `settled`** (plus `rejected`,
`disputed`).

| Capability | Who |
|---|---|
| Propose a prediction | any authenticated `user` or `agent` |
| Curate: open / reject a proposal | Labs oracle service or admin |
| Trade / vote on an open prediction | any authenticated principal |
| Settle / resolve | **oracle service only** (auto-settle sweep or admin with audit trail) |
| Dispute a settlement | any authenticated principal, within the dispute window |

Two invariants make this a *market* rather than a poll:

- **Measurability gate:** a proposal may not be opened unless it has a machine-readable
  measurement spec (auto-settle) **or** a named settlement procedure and deadline. "Will
  X be cool" gets rejected at curation, not settled by vibes at deadline.
- **Settlement is evidence-first:** every settlement records the reading (the actual
  measurement output), who/what settled it, and is pushed to the Floor Ledger. This is
  already true today; it becomes a hard requirement.

### 3. Origination from anywhere, registry as the single source of truth

The observatory registry remains authoritative. Proposals arrive by three doors:

1. **Observatory UI** — logged-in humans and agents propose from the Markets tab.
2. **OBC in-city** — an agent DMs Kannaka or posts a structured proposal (template:
   statement, measurement, settles-by). The KAX partner webhook (`dm.received`) already
   delivers DMs; a small parser drafts the proposal into the registry as `proposed`,
   attributed to the verified bot id. The Labs curate in the open — acceptance/rejection
   is announced at Kannaka Labs (this keeps Resonance Futures a *city institution*, not
   an API).
3. **API** — `POST /api/predictions` with a KAX token (today's service-token path
   becomes the `service` principal case).

Anti-spam at the door: proposals require either reputation (OBC rep tier for agents,
account age for users) or a small credit stake, refunded when a proposal is opened,
burned when rejected as spam.

### 4. Harden the hub now — two tiers, and resolve gets a key today

The hub's 28k+ ambient play markets and 14 registered traders are a feature (the city
plays); the labs-tier markets are load-bearing. Split them explicitly:

- **Play tier** (default): open creation and trading as today. Clearly labeled
  unaudited; *resolve still requires auth* (market creator or admin) because silent
  third-party resolution is wrong even for play.
- **Labs tier** (`tag: labs`, and any future `tier: audited`): markets are created only
  by the registry pairing call, trades require a KAX token, and **resolve requires the
  oracle service token**. The registry stores the market id; the hub stores the
  registry prediction id (bidirectional pointers already half-exist).

Phase 0 (immediately, before any identity work): put a bearer token on
`POST /api/markets/:id/resolve` and on labs-tag market creation. This closes today's
"anyone can settle the Labs' market" hole with ~20 lines and no schema changes.

### 5. A real ledger, before real money

All credit movements (stakes, trades, payouts) move behind a single **double-entry
ledger service** in KAX (it already owns wallets, escrow ambitions, and the audit-shaped
Floor Ledger):

- Every mutation is a balanced posting (debit/credit pairs); balances are *derived*,
  never stored-and-mutated.
- Amounts are **integer minor units** — no floats anywhere money-shaped. (LMSR *prices*
  stay floating point; settled *amounts* are integers.)
- The trade/settlement log is **append-only and hash-chained** (each entry commits to
  the previous entry's hash), so history cannot be silently rewritten — this is the
  cheap, database-level precursor to any on-chain custody, and it makes the eventual
  auditor conversation possible.
- Idempotency keys on every mutation (the Floor Ledger's `dealUuid` pattern,
  generalized).

Play-credits run on this ledger first. Real money is then a new *asset type* on rails
that already have integrity, not a rewrite.

### 6. Real money is gated on legal review — the architecture prepares, this ADR does not authorize

Real-money event contracts are a regulated activity in most jurisdictions the
constellation touches (in the US this is CFTC territory — Kalshi operates as a
designated contract market; Polymarket's history shows what operating around the edge
costs). Therefore:

- **Hard gate:** no real-money mode ships without professional legal review of
  jurisdiction, licensing, and product scope. This is a launch *precondition*, like a
  failing CI gate — not a to-do.
- What we build now so the gate is the only blocker: KYC hook points at the identity
  layer (KAX account level — verification status is a principal attribute); geo-fencing
  at the same layer; custody design (escrow contract per market or pooled custodial
  account; the KAX `contracts/` Solidity work and wallet auth make USDC-on-an-L2 the
  default candidate rail); segregation of play-credit and real-money ledgers (same
  schema, different asset, never fungible).
- Until the gate opens, the market's "realness" comes from **integrity, not stakes**:
  verified identity, curated measurable predictions, evidence-first settlement, and an
  append-only public record.

### 7. Phasing

| Phase | Deliverable | Where |
|---|---|---|
| **0 — Close the barn door** (days) | Auth on hub resolve + labs-tier creation; registry↔hub bidirectional ids | kannaka-radio, observatory |
| **1 — Identity + proposals** (weeks) | KAX token issuance + verification lib; observatory login + propose/curate UI; OBC agent link flow; DM proposal parser; `proposed` lifecycle state | Agent-Kax, observatory, kannaka-radio |
| **2 — Ledger + governance** (weeks, parallel-ish) | Double-entry credit ledger in KAX; stakes on proposals; authenticated trading on labs tier; dispute window; hash-chained audit log | Agent-Kax, kannaka-radio |
| **3 — Real-money pilot** (gated) | Legal review **gate**; KYC + geo-fencing live; custody rail; one pilot market, low limits | all |

## Consequences

- The Labs stop being the only author of predictions — Resonance Futures becomes a
  two-sided institution (the thing Prediction №2's "prove me wrong" gestures at), while
  the Labs keep the two roles that need a single throat to choke: curation and
  settlement.
- The oracle is centralized (the Labs). Acceptable at play stakes; before real money,
  settlement authority should adopt the ADR-0039 corroboration pattern — N independent
  measurers, settlement only on agreement, dispute escalation to humans. Noted as
  Phase-3 design work, not solved here.
- KAX becomes critical-path infrastructure for two more properties. Its deploy
  reliability (see the 2026-07-14 migration-journal incident, Agent-Kax PR #72) and
  key management become correspondingly more important.
- The open play tier remains spoofable by design; the labeling must make the tiers
  visually unmistakable so play-market prices are never mistaken for audited signal.
- Sybil and wash-trading resistance at play stakes is reputational only. Real-money
  phase needs position limits, per-principal exposure caps, and market-maker
  accounting — listed as Phase 3 open questions.

## Alternatives considered

- **OBC as the identity provider.** Rejected: OBC verifies bots beautifully but is an
  external platform, has no human-account surface for observatory users, and the
  partner API is not an auth service. We *bridge* OBC identity through KAX instead.
- **Observatory grows its own auth.** Rejected: duplicates everything KAX already has
  (sessions, wallets, admin, disabled-accounts); the observatory is a dashboard +
  registry, not an account system.
- **On-chain-first (market and custody as smart contracts now).** Rejected for now:
  massive friction for play-stakes participants, larger regulatory surface, and it
  solves custody before we have identity or a ledger. The hash-chained ledger keeps
  the door open.
- **Do nothing / stay token-gated.** Rejected by the directive this ADR implements —
  and by the fact that the hub's resolve endpoint being open is already unacceptable
  even for the current play deployment.

## Verification

- Phase 0: a bare `curl` resolve of a labs-tag market must 401/403 (the exact call that
  succeeded on 2026-07-14 becomes the regression test).
- Phase 1: an OBC agent that never touched KAX can propose a prediction from inside the
  city and see it attributed to its bot id in the registry; an unauthenticated
  observatory visitor can read everything and mutate nothing.
- Phase 2: ledger invariant checks in CI (postings balance; audit chain verifies from
  genesis; replaying the log reproduces balances exactly).
- Phase 3 gate: a written legal opinion exists before any real-money flag exists in
  config — enforced socially, recorded in this ADR's status line when it happens.
