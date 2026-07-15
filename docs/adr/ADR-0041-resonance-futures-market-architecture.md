# ADR-0041 — Resonance Futures: identity, settlement authority, and the path from play-credits to a real prediction market

- Status: Proposed (2026-07-14) — needs Nick's review. Adversarial design review COMPLETE (2026-07-14, 4-lens panel, 38 findings / 12 blockers). Reconciliation + forced changes recorded in the "Adversarial design review" section at the end; **Phase 0 is NOT yet complete** (two confirmed blockers remain — see below).
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

## Adversarial design review (2026-07-14)

A four-lens panel (identity/capability, ledger integrity & crash-consistency,
oracle/market economics, cross-service ops) attacked this design against the real
code before Phase 1. 38 findings (12 blocker, 22 major, 4 minor) + 8 confirmed-correct
guardrails. Reconciled verdicts below; the panel's raw output is archived in the
session transcript. The review **changed the plan** — Phase 0 is reopened, identity
fixes are promoted, ledger immutability is required, and corroboration moves ahead of
real-credit stakes.

### Refuted (with reason)

- **"The Phase-0 oracle-token gate does not exist in routes.js / GSHUB_ORACLE_TOKEN is
  never checked" (3 findings, filed as blockers).** Refuted against the *deployed*
  reality: `POST /api/markets/:id/resolve` and labs-tier creation return **403** to an
  unauthenticated caller in production (verified live), and the gate is present in
  `routes.js` on `master` (merged PR #93). The panel read a *stale local checkout*
  (origin/master had not been pulled after the merge). **But two real hazards survive
  the refutation** and are kept as blockers: (a) there is no automated regression test
  locking the gate — the ADR's own verification criterion ("a bare curl resolve must
  401/403") is not yet a CI check; (b) the gate is *incomplete* — see the TTL bypass
  blocker below.

### Confirmed blockers — Phase 0 (must fix before calling Phase 0 done)

- **TTL auto-resolver bypasses the oracle gate on labs markets.**
  `ghostsignals-hub.js::_resolveExpiredMarketsInner` selects *every* `resolved=0 AND
  expires_at < now` market with **no tag filter** and resolves each by max traded price
  (`method:'ttl'`). The Phase-0 gate only covers the HTTP resolve handler, so a labs
  market still auto-resolves by (sybil-pumpable) price at its TTL if the oracle's
  resolve has not landed — directly contradicting "resolve = oracle only". This is a
  **live exposure**: Prediction No 2's paired market `m_4584162dfc2d` has TTL near its
  2026-07-26 settle date. *Fix:* persist a `resolution_authority` at creation; the TTL
  loop excludes `oracle`-authoritative markets — they resolve only via an oracle-token
  call, and if the oracle misses the deadline the market is voided/refunded, never
  price-resolved.
- **Concurrent `resolveMarket` double-pays.** `resolveMarket` reads `market.resolved`
  from an awaited `getMarket()` *before* its transaction, then `UPDATE markets SET
  resolved=1 ... WHERE id=?` with **no `WHERE resolved=0` and no `changes()` check**. The
  oracle HTTP resolve racing the 10s TTL sweep (the in-memory `_resolving` guard only
  covers the sweep against itself) makes both paths read `resolved=0` in the async gap
  and pay every winning position twice — credits minted from nothing. *Fix:* `UPDATE ...
  WHERE id=? AND resolved=0`, read `changes()`, pay out only if this call flipped the
  flag; else roll back as "already resolved".
- **`placeTrade` check-then-act drives capital negative.** Reads `trader.capital`,
  checks `cost > capital` in JS, then unconditional `UPDATE traders SET capital =
  capital - ?`. Two concurrent trades both pass the check and both debit → negative
  balance / spend-what-you-don't-have. *Fix:* `UPDATE ... SET capital = capital - ? WHERE
  id=? AND capital >= ?`; verify `changes()==1` else roll back.
- **`predictions.json` non-atomic write + reset-to-`[]` erases the registry.**
  `persist()` is a single `fs.writeFileSync` of the whole array (no temp+rename, no
  fsync); a crash mid-write truncates the file, and `ensureLoaded()`'s bare
  `catch { predictions = []; }` then silently resets — the next write persists `[]`
  over the only copy. Because floor push + hub resolve happen *before* this fragile
  local write, a crash can leave credits paid and a floor row written with zero local
  record. *Fix:* serialize to `.tmp`, fsync, atomic rename, fsync dir; on parse failure
  rename to `.corrupt` and **fail closed** (never overwrite with `[]`); keep a rolling
  `.bak`.
- **Self-fulfilling measurement.** `runMeasurement` settles TRUE the instant any
  non-excluded bot claims a plot; a Yes-holder simply registers a fresh OBC bot (not on
  `excludeClaimants` — the proposer cannot enumerate future traders), claims a plot, and
  the next sweep pays them. `excludeClaimants` as a blocklist is the wrong mechanism.
  *Fix:* disqualify plots claimed after market creation and any claimant holding a
  position in the paired market; gate causable world-state conditions behind ADR-0039
  corroboration before any stake rides on them. (At play stakes this is a curiosity; on
  real money it is theft — this is why corroboration moves ahead of real-credit stakes.)

### Confirmed blockers — Phase 1 / 3 (design changes folded in)

- **Trader identity is an unverified free string** (`registerTrader`/`placeTrade` take
  `trader_id` from the body; each new id self-grants capital=100). Sybil-mint and
  grief-drain today; theft when credits carry value. *Fix (Phase 1):* derive
  `trader_id` from the verified JWT `sub`, never the body; capital becomes a
  ledger-derived balance (initial grant is a signed posting), not a per-id grant.
- **OBC agent-control challenge is spoofable.** Granting `agent` capability on a
  *harvested* row, or trusting a `dm.received` webhook's self-claimed sender bot_id, or
  a replayable/non-bound nonce, or treating the shared OBC JWT file as an identity
  proof — each lets an attacker claim another bot. *Fix:* verify the OBC partner webhook
  signature and use OBC's asserted sender (not payload-claimed); or verify a presented
  OBC JWT against OBC's key; single-use short-TTL bot_id-bound nonces; capability only
  on a persisted completed-challenge record.
- **Local JWT verification: alg-confusion / kid / revocation.** Pin an alg allowlist
  (reject `alg=none` and HMAC-against-the-public-key); require exp/iat/nbf with skew
  bounds; publish JWKS with `kid`, select by kid, overlapping rotation windows,
  **fail-closed** if the key endpoint is unreachable; add short TTL + a revoked-jti/kid
  list so a leaked token (especially oracle) can be killed before exp.
- **Real-money = single co-located oracle+custody (Phase 3).** One box holds the
  settlement token *and* could trigger custody release, with corroboration deferred.
  *Fix (hard Phase-3 preconditions):* N-of-M ADR-0039 corroboration before any
  settlement that releases funds; custody keys physically separated from the settlement
  oracle (different box, HSM/multisig); a continuous on-chain/off-chain reconciler that
  **halts** settlement on divergence.

### Confirmed major (folded into the phase they bite)

- **Floor ledger is upsert-mutable** (`onConflictDoUpdate` on `dealUuid` rewrites every
  column; `credits` is a float). A token holder — or the observatory's own crash-replay
  — can silently rewrite a "witnessed" settlement, and any Phase-2 hash chain over this
  table is invalidated by an in-place update. *Fix:* `onConflictDoNothing` +
  return-existing (immutable); corrections are new superseding rows; revoke UPDATE/DELETE
  on the table at the DB grant; integer minor units now, not float. **This reverses the
  earlier "the dealUuid re-push is fine" habit** used during the 07-14 recovery.
- **`settlePrediction` has no re-check after its awaits** → an auto-sweep clobbers a
  human's manual correction (and vice versa). *Fix:* set `status='settled'`
  synchronously before any await; re-check `if (status==='settled') return` — first
  settlement wins.
- **LMSR mints unbacked credits** (winners paid $1/share from nothing; no funded
  market-maker account), so the Phase-2 double-entry ledger cannot balance when these
  flows port onto it. *Fix:* model the AMM as a real account, escrow the bounded subsidy
  `b*ln(n)` at creation, pay winners from it; one canonical float→integer rounding rule
  with residual to the house account; assert conservation at commit.
- **Single-writer is assumed, not enforced** (no lockfile) → two observatory processes
  clobber each other's registry. *Fix:* exclusive lockfile at startup, or move the
  registry into the same DB as the rest of the pipeline.
- **Cross-service settlement is fire-and-forget** (floor push then hub resolve, both
  "never throws") → the two can permanently disagree, and the TTL loop then resolves the
  market to the opposite side. *Fix:* a durable outbox in the observatory — per-target
  delivery rows written inside the same persist, retried idempotently until confirmed;
  gate "settled" display on both targets confirmed or raise an inconsistency alarm.
- **Partial/paginated OBC read settles FALSE at deadline** (a 200 with a truncated
  `plots` array yields count=0, which is not a throw). *Fix:* follow pagination to
  exhaustion, treat any partial/ambiguous read as a measurement failure (stay open),
  require a confirming re-read before settling FALSE.
- **Sybil-manufactured price is cited as legitimizing signal** — this ADR itself cited
  "traders priced Yes at 0.82" as evidence of a real market; that number is trivially
  forgeable by one actor with N free registrations. *Fix:* never present raw
  open-registration play price as authoritative; weight by verified-identity
  participation; label play prices unaudited everywhere; per-principal position caps.
- **No trading halt before a deterministic settlement** → guaranteed front-running once
  the measurement is observably true. *Fix:* a `settling`/frozen lifecycle state; snapshot
  price at freeze, resolve at the snapshot.
- **KAX-IdP outage becomes a settlement outage; the auth flag-day can lock out the live
  oracle** (Prediction No 2 is running). *Fix:* cache/pin JWKS with long TTL so the oracle
  verifies offline and settlement never blocks on KAX reachability; migrate with the
  hub/floor accepting BOTH the legacy static token AND KAX-signed JWTs during a window,
  client creds first, then retire the static path; add a non-silent settlement-failure
  alert.
- **`settleEarlyOnTrue` assumes monotonicity but OBC claims are reversible** — a
  transient claim locks a permanent wrong settlement with no rollback. *Fix:* restrict
  early settle to verifiably-permanent facts, or require the condition to hold across K
  consecutive sweeps; keep every settlement reversible within the dispute window before
  floor/hub pushes are final.

### Confirmed minor

- Prediction `number` derived from `array.length` collides after a reset — use a
  persisted monotonic counter.
- Refundable-stake anti-spam is a manual-review DoS if "unmeasurable" refunds — forfeit
  the stake on any curation rejection; auto-reject non-self-validating specs before a
  human looks.
- Dispute window has no stake and undefined payout-freeze semantics — require a
  dispute stake, cap open disputes per principal, escrow with a hard time cap.
- Hub "already resolved" is treated as failure on retry — treat it as success when the
  existing outcome matches intent.

### Confirmed-correct — load-bearing, do not regress

- Asymmetric-signed JWTs verified **locally** (no per-request introspection) is the
  right architecture for the constellation — the fixes harden alg/kid/revocation, they
  do not argue for introspection.
- The **measurability gate** (no proposal opens without a machine spec or a named
  procedure+deadline) is correct and must not be weakened.
- `_resolveExpiredMarkets` **re-entrancy guard** (`_resolving`) is load-bearing against
  the sweep double-paying itself — keep it (it is simply not sufficient against the
  sweep-vs-HTTP race; that needs the DB-level single-flip fix).
- The `#43` **ISO-format expiry comparison** fix must not regress or expired markets
  silently strand.
- `placeTrade`'s single **BEGIN/COMMIT transaction** (q update + debit + trade + position
  in one atomic unit) is correct — preserve it when adding the guarded debit + auth.
- `requireAuth.ts` **re-reads the user row and checks `disabledAt` on every call**
  (never trusts the session blob) — keep this for the new principals.
- **`dealUuid` as the idempotency key** is the right concept — keep the key, just make
  the row immutable instead of upsert-mutable.
- **settle/resolve = oracle-service-only** is correct — the finding is only that it must
  be enforced per-endpoint with a *distinct* credential (a proposer/user/agent token
  must 403 at the settle handler), not that the rule is wrong.

### Resulting plan changes

1. **Phase 0 is reopened.** It is not complete until: the TTL loop excludes
   oracle-authoritative markets, `resolveMarket` and `placeTrade` are made atomic/
   guarded, the registry write is atomic + fail-closed, and a **capless-caller-denied
   regression test** exists in CI. The self-fulfilling-measurement class is documented
   as a settlement-safety rule (no auto-settle of participant-causable conditions).
2. **Corroboration (ADR-0039) moves from Phase 3 to a precondition of Phase 2** — no
   real-credit stake precedes N-measurer agree-to-settle.
3. **Ledger immutability is a Phase-2 hard requirement** — separate append-only,
   INSERT-only, integer, hash-chained table with a DB-enforced no-UPDATE/DELETE grant;
   `floor_ledger` becomes a display projection. The AMM is a funded ledger account.
4. **Identity binds authority to a verified principal end-to-end** — trader id from JWT
   sub, agent capability only on a verified-challenge record, alg/kid/revocation
   hardening, distinct oracle credential.
5. Real money keeps its legal gate **and** now additionally gates on corroboration +
   separated custody + a reconciler.
