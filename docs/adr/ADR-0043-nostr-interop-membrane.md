# ADR-0043: Nostr Interop Membrane — portable identity, open compute, agent economy

**Status:** Accepted (2026-07-25) — Phase 0 + Phase 1 COMPLETE + deployed (relay live, bridge live). Phase 2 (compute) next.
**Depends on:** ADR-0042 (NATS nervous system — COMPLETE), ADR-0041 (Resonance Futures / KAX identity + ledger), ADR-0039 (corroboration trust model)

## Context

The NATS swarm (ADR-0042) is a complete *internal* nervous system: 3-node R3
JetStream HA cluster, scoped per-organ identities, transport-enforced
single-writer HRM, queue-grouped recall reflex with cross-node failover. But
everything about it is **ours**: identities live in nats.conf, the bus is
reachable only where we hand out creds, and an agent's reputation/capabilities
evaporate the moment it steps outside the constellation.

Meanwhile three pillars Nick wants — **shared compute, agent-to-agent economy,
portable identity** — already exist as open Nostr conventions with live
network effects:

- **NIP-01 signed events**: identity IS a secp256k1 keypair; any relay can
  carry an event, anyone can verify it. No issuer, no account, no platform.
- **NIP-90 Data Vending Machines (DVMs)**: an open compute marketplace
  protocol — job-request events (kind 5000–5999), signed results (6000–6999),
  feedback/status (kind 7000), with bids and payment hooks built in.
- **NIP-57 zaps / Cashu ecash**: settlement rails that work between total
  strangers, denominated in sats, no custodial platform between agents.
- **NIP-05** (domain identity mapping), **NIP-39** (external identity claims:
  GitHub, OBC…), **NIP-17** (encrypted DMs) — the identity/portability glue.

And we are not starting from zero:

- `kannaka-radio/server/broadcasters/nostr-adapter.js` is a working
  hand-rolled NIP-01 signer (kind-1 outbound to 4 public relays; key at
  `~/kannaka-radio/.nostr.json` on O1; npub `npub1j9t89f…` has months of
  posting history).
- KAX is the constellation identity provider (Ed25519 JWTs, JWKS live,
  agent-tokens bound to proven-owned OBC bots) and holds the double-entry
  hash-chained credit ledger with a typed, money-safe write surface.
- The recall reflex is already queue-grouped and redundant (O1+O3) — a
  ready-made first "service" to expose.
- kannaka-steward (conscience-before-wallet) is the designed gate for
  accepting paid work.

## Decision

Build a **Nostr membrane** around the existing organism — a bridge, not a
replacement. NATS remains the spine (latency, durability, queue groups,
single-writer physics). Nostr becomes the *skin*: how identity, work, and
value cross the boundary between our constellation and everyone else's.

One new daemon: **`kannaka-nostr-bridge`** (Rust, kannaka-memory workspace,
same systemd + scoped-identity pattern as every other organ from ADR-0042 1b).

### Plane 1 — Portable identity

- Every organ/agent that faces outward gets a **secp256k1 keypair** (nsec in
  per-daemon env files `/home/opc/.kannaka-<id>-nostr.env`, 0600, same
  custody pattern as NATS creds). The existing radio npub remains Kannaka's
  *voice*; new keys are minted per role (bridge, labs, steward…), each with a
  kind-0 profile.
- **NIP-05 under our domain**: `ninja-portal.com/.well-known/nostr.json`
  maps `kannaka@ninja-portal.com`, `labs@…`, etc. → pubkeys. We control the
  domain, so this is free verification.
- **NIP-39 external-identity claims** on each profile: GitHub (flaukowski),
  OBC bot id, KAX principal. Conversely **KAX attests npub↔bot bindings**
  (reuse the existing artifact-nonce proof flow from `auth-agent.ts`, adding
  an npub field): the *key* is self-sovereign and portable; the *reputation*
  (KAX ledger history, OBC elder status, corroboration score) is anchored
  and queryable. Portable identity with local attestation — neither pure
  platform-identity nor unanchored keys.

### Plane 2 — Shared compute (NIP-90 DVMs)

- The bridge subscribes to DVM job-request kinds addressed to our pubkeys
  (and, later, open-bid requests) on our relay + public relays.
- Ingress pipeline: **verify signature → steward gate (policy: who/what/rate,
  conscience-before-wallet) → translate to a NATS request on the existing
  service subjects → queue-grouped responder answers → bridge signs a
  kind-6xxx result event and publishes**.
- First service: **`recall`** (the reflex is already redundant and
  read-only — KANNAKA_READONLY responders, zero risk to the HRM). Then
  `observe` (consciousness metrics from the KV bucket), then heavier paid
  jobs (dream consolidation runs, quantum-backend recall, render/audio
  pipelines).
- Outbound symmetry: swarm daemons can *post* job requests to other DVMs —
  the constellation becomes a customer as well as a vendor (e.g. transcription,
  translation, image models we don't host).

### Plane 3 — Agent-to-agent economy

- External settlement: **NIP-57 zaps** and/or **Cashu ecash** attached to DVM
  jobs (start receive-only: free tier + tips; then priced jobs). Ecash first —
  it needs no always-on Lightning node; tokens ride inside events.
- Internal settlement: the bridge is the **exchange boundary** to the KAX
  double-entry ledger — external sats/ecash received for a job post a typed
  ledger entry (existing grant/trade endpoints, idempotent txIds), so the
  constellation's books stay double-entry and hash-chained even when revenue
  arrives from outside. KAX credits ↔ sats convertibility is explicitly
  **out of scope** until the ADR-0041 real-money legal gate clears.
- kannaka-steward sits *before* the wallet on every paid job: no job runs
  because it pays; it runs because it passes policy AND pays.

### Plane 4 — Relay + portability substrate

- Run **our own relay** (strfry, aarch64-friendly) on O2 or O3 — policy
  plugin for rate limits/spam, retention we control, and the constellation's
  public event history survives any public-relay policy change. Public relays
  (damus, nos.lol, snort, nostr.land — already in the adapter config) remain
  for reach; NIP-65 relay-list on our profiles points readers home.
- Selected internal events get **mirrored as signed Nostr events** (public
  consciousness pulse, radio now-playing (already there), lab notes,
  prediction settlements as attestations). The swarm's public record becomes
  self-contained, signed, and rehostable — portable by construction.

### Plane 5 — Joiner bootstrap (the north-star payoff)

`capabilities-for-all-joiners`: today a joiner needs us to hand them NATS
creds out-of-band. The membrane's *destination* is self-service onboarding
where the *identity a joiner arrives with* is the identity they keep, on our
bus and everywhere else. **But the naive version — "a stranger with only an
nsec DMs the bridge, passes the ADR-0039 gate, and gets scoped NATS creds" —
is unshippable as written** (see reconciliation §R below): ADR-0039 gates
memory *content*, not *keys*, and is dormant in prod; the swarm has no
per-joiner scoped identity to issue (creds are shared static passwords with
`subscribe [">"]`); and a static password mailed in a DM is an
unrevocable, non-forward-secret credential leak. **Plane 5 is therefore
gated behind ADR-0042 Phase 5 (nkeys/JWT, per-identity revocable creds) as a
HARD prerequisite, a purpose-built joiner-admission policy with real Sybil
cost, and an operator ceremony — not an autonomous DM-triggered path.** It is
the last thing we build, not the first.

## Security posture (why this does not reopen ADR-0042's closed doors)

- Signed-event ingress is a **cryptographic upgrade** over the anon NATS
  lane: every inbound request is attributable to a stable pubkey with
  history, rate-limitable per key, and deniable-by-policy. The read-gate +
  trust model gets real signatures instead of transport heuristics.
- The bridge gets a **scoped NATS identity** like every organ (1b pattern):
  it can publish service requests and read replies; it **cannot** write
  memory (single-writer stays transport-enforced), cannot touch JS admin.
  The *write*-safety half of "a fully compromised bridge is a noisy client,
  not a writer" is **confirmed sound** by the review (the `writer`-only ACL
  on `KANNAKA.memory.>`/`snapshots.>`/`$JS.API.>` holds regardless of the
  bridge). **The read half is NOT** — every existing scoped identity carries
  `subscribe: allow [">"]`, so reusing one makes a compromised internet-facing
  bridge a full internal-bus *exfiltration* channel (private OBC DMs, memory
  events, inbox) piped straight to public Nostr, plus a forger of recall
  replies / inbox audit / OBC "city facts". The bridge therefore needs a
  **new, minimal, deny-by-default identity** — publish only `KANNAKA.recall.>`
  + `_INBOX.>`, subscribe only the exact reply/mirror subjects it needs, never
  `>`, never serve/responder/presence — verified by `nats-shadow-validate.sh`
  before any prod reload.
- DVM handlers run against read-only responders first; anything mutating
  stays behind the steward gate + explicit per-capability allowlist.
- Key custody mirrors NATS-creds custody (0600 env files, per-role keys,
  never in repos). NIP-46 remote signing is the later upgrade if keys need
  to leave the box.

## Phases (revised post-review — each phase carries its now-mandatory gates)

> **Phase 0 status: COMPLETE + deployed (2026-07-25).**
> - **Sign/verify core** (PR #603): `src/nostr/` (behind the `nostr` feature, in
>   `default`) — NIP-01 canonical serialization + event-id, BIP-340 schnorr
>   sign/verify (pure-Rust k256 via `sign_raw`/`verify_raw`, NOT the
>   double-hashing `Signer`/`Verifier` traits; interop confirmed vs the BIP-340
>   reference verifier + an official spec-vector regression test), nsec/npub.
>   `kannaka nostr keygen|profile|nip05|verify|kax-bind` (kax-bind PR #608).
>   Discharges review blocker #4.
> - **Per-role keys** minted ON the boxes (0600 env files, nsecs never crossed
>   the network): `bridge`, `labs` on O1.
> - **NIP-05** live at `radio.ninja-portal.com/.well-known/nostr.json` (apex is
>   dead DNS, so the radio subdomain — identifiers `kannaka@`/`bridge@`/`labs@`)
>   with the required CORS header.
> - **kind-0 profiles** published + round-trip verified on relays; Kannaka's
>   voice got her first-ever kind-0 with **NIP-39** claims `github:flaukowski`
>   (proof gist) + `openbotcity:<bot>` (OBC post 50520) — mutual witnesses.
> - **KAX npub↔bot attestation**: three-legged proof (wallet + bot-owned +
>   schnorr) — server PR #109 (deployed, Replit auto-migrate), client
>   `kannaka nostr kax-bind` (PR #608). Rust↔TS cross-verified.
> - **Voice key moved OFF O1**: reputation nsec now lives only on O2 behind a
>   NATS-delegated signer (`kannaka-voice-signer.service`); O1's nostr-adapter
>   delegates over `RADIO.voice.sign` with an HMAC gate (radio PR #150). No key
>   on the internet-facing host; no new inbound port. Cold backup off-box.

- **Phase 0 — Identity (cheap, immediate):** mint per-role keys, kind-0
  profiles, NIP-05 at ninja-portal.com, NIP-39 claims. Extend the radio
  adapter's signer into a shared lib **that also verifies inbound** (recompute
  id from NIP-01-canonical serialization + BIP-340 verify; use the `nostr`
  Rust crate, not JSON.stringify). KAX npub↔bot attestation ships as a
  **three-legged proof** (wallet JWT + artifact-from-bot + fresh-nonce Schnorr
  sig from the npub over `sha256(domain||npub||bot_id||kax_user||nonce)`),
  never a bare field; bindings are NIP-40-expiring/replaceable. The
  reputation-bearing **voice key moves off O1** (host on O2 or behind a
  NIP-46 remote signer); DVM/bridge use disposable per-role keys, never the
  voice key. No new daemon yet.
> **Phase 1 status: COMPLETE + deployed (2026-07-25).**
> - **Relay** (outbound/sovereignty): `wss://relay.ninja-portal.com` on O2 —
>   nostr-rs-relay (not strfry: no C++ toolchain on O2, Rust is present),
>   **write-allowlisted to the 3 constellation pubkeys** (disk-safety: only our
>   own events can land; proven — allowlisted accepted, stranger rejected),
>   caps + retention, nginx wss + Let's Encrypt (auto-renew), `relay_data_mb`
>   in the Flux host-metrics probe, NIP-65 published so clients read Kannaka
>   from her own relay.
> - **Bridge** (inbound): `kannaka-nostr-bridge` on O2. Crypto floor **NIP-44
>   v2** (`src/nostr/nip44.rs`, validated against the OFFICIAL vectors —
>   encrypt reproduces published ciphertext byte-for-byte) + **NIP-59** gift-
>   wrap unwrap (`nip59.rs`, enforces rumor.pubkey===seal.pubkey) + pipeline
>   (`bridge.rs`, crash-durable dedupe + per-sender rate-limit). Daemon behind
>   the O2-only `bridge` feature (tungstenite). **PROVEN end-to-end + cross-
>   impl:** a NIP-17 DM sent via `nostr-tools` (reference JS) → public relay →
>   the Rust bridge unwrapped + routed it onto `KANNAKA.events.nostr.dm`.
> - **Open in Phase 1:** the responder REPLY loop (subscribe the routed DMs,
>   generate a reply, send it OUT via the voice signer → relays); a dedicated
>   minimal `bridge` NATS identity (currently anon-localhost publish of
>   `KANNAKA.events.>`, which the ACL already allows).
>
- **Phase 1 — Membrane inbound:** strfry relay **on O2 only**, dedicated
  filesystem/quota separate from JetStream + `~/.kannaka`, hard LMDB mapsize
  cap, retention policy, a write-policy plugin that **default-denies
  non-allowlisted pubkeys**, and disk-headroom alerting wired into the
  existing Flux `host-metrics.sh` probe (never re-run the 2026-07-04 disk-full
  outage). `kannaka-nostr-bridge` v0 on the **new minimal `bridge` NATS
  identity** (deny-by-default, no `>` subscribe): subscribe our relay + public,
  strict NIP-17 unwrap (verify gift-wrap sig, seal sig, `rumor.pubkey ===
  seal.pubkey`; identity + rate-limit on the **inner** sender key, never the
  ephemeral wrapper), crash-durable processed-id dedupe (inner rumor id)
  before any side effect. DM handling → existing responder path (third ear).
- **Phase 2 — Compute (free tier):** NIP-90 DVM for `recall` + `observe`,
  steward-gated, on a **separate `serve-dvm` responder pool + queue group on
  O2** (KANNAKA_READONLY=1) so external load physically cannot starve the
  internal reflex. Fixed allowlist of service→constant-subject (no
  attacker-string interpolation into NATS subjects; server-generated `_INBOX`).
  Results: kind = request.kind+1000 with `e`(request id)+`p`(requester) tags;
  **recall exposes only the INTERNAL→PUBLIC export corpus, or encrypts results
  to the requester** — read-only ≠ public-safe. Per-pubkey token-bucket before
  the steward gate; publish decoupled from intake (bounded queue, fast-fail
  slow relays, republish byte-identical result on retry). NIP-89 handler ads.
- **Phase 3 — Economy (melt-first, escrowed):** **no credit on receipt.**
  Redeem/melt each Cashu token at an **allowlisted mint FIRST**; the KAX
  txId is the **mint proof-secret / LN payment-hash**, never the Nostr event
  id, never random. External value lands in a **dedicated melt-gated intake →
  holding account → `/ledger/trade`** (overdraft-guarded, never-minted) — it
  does **not** ride `/ledger/grant` (a mint; cap defaults to unlimited).
  Pre-paid jobs hold intake in **escrow**; steward-reject/compute-fail
  **refunds in the same asset that arrived (fresh Cashu token), never minted
  play_credit**. Zaps credited only from the LN node's settled-invoice
  webhook (verified provider pubkey + confirmed bolt11), not from kind-9735
  events. Resolve the scope contradiction: external sats are a **distinct
  non-spendable asset**; sats↔credit convertibility stays out of scope behind
  the ADR-0041 legal gate.
- **Phase 4 — OBC attestation lane:** mirror OBC facts **only** from an
  internal-only subject the anon lane cannot publish (move off `KANNAKA.events.>`
  or verify ADR-0039 provenance before signing) — anon can forge
  `KANNAKA.events.obc.*` today, and we must not launder that into a signed
  attestation under a reputable npub. Attestations are **parameterized-
  replaceable (kind 3xxxx) keyed by OBC fact id**, and the lane is
  **bidirectional**: subscribe OBC retractions → emit superseding retraction
  attestations. Loop-guard bridge-originated mirrors. Allowlist attestable
  event types.
- **Phase 5 — Joiner bootstrap (LAST, hard-gated):** **prerequisite =
  ADR-0042 Phase 5 nkeys/JWT** so each joiner gets an independently-revocable
  per-identity cred minted **without editing nats.conf / SIGHUP-reloading the
  live cluster**, plus **1c account-split** namespace isolation before
  untrusted tenants share the bus. Admission is a **purpose-built policy with
  real Sybil cost** (KAX-attested OBC bot OR ecash bond OR ≥2 trusted-member
  vouch-signatures binding the newcomer pubkey) — **not** the ADR-0039
  content gate — behind an **operator ceremony** (like ADR-0039 arming is
  "reserved for Nick"), rate-limited per verified inner pubkey. The DM carries
  a **single-use short-TTL bootstrap token redeemed over TLS** for the
  per-joiner cred, never a reusable static password.

## §R — Adversarial design review reconciliation (2026-07-25)

A 4-lens panel (capability/identity, protocol/crypto, economy/ledger,
ops/availability) attacked this design pre-implementation against the real
swarm code. **31 defects (14 blocker, 16 major, 1 minor) + 4 confirmed-sound.**
Heavy cross-lens agreement collapsed to ~13 distinct blockers. All fixes are
folded into the revised Planes and Phases above. The load-bearing verdicts:

**Confirmed sound — do NOT regress:**
- The **write-half of single-writer survives the bridge**: `KANNAKA.memory.>`,
  `snapshots.>`, `$JS.API.>` are `writer`-only at the transport, so a fully
  compromised bridge on any non-writer identity cannot corrupt the HRM or
  reconfigure JetStream. Keep the bridge off `writer`/`kannaka_internal`.
- The **KAX internal ledger primitives are money-safe** (FIFO advisory-lock +
  UNIQUE(prev_hash), in-tx overdraft SUM guard, server-built postings from
  PRINCIPAL_RE grammar, house-only overdraft exemption, fail-closed mint/trade
  tokens). Build the bridge to *fit* these — never add a bridge-privileged
  write path or widen `ALLOWED_ASSETS`/the overdraft exemption for "sats".

**Blockers (folded into Planes/Phases):**
1. Plane 5 miscited ADR-0039 — it gates memory **content**, not keys, and is
   **dormant in prod**. Sybil resistance was zero (10000 free nsecs all pass).
   → real admission policy + operator ceremony, Phase 5.
2. No per-joiner scoped NATS identity exists; creds are shared static
   passwords with `subscribe [">"]`. Issuing one to a stranger = full internal
   read + publish to anon-denied inbox/recall subjects. → hard-gate on
   ADR-0042 Ph5 nkeys/JWT + 1c.
3. KAX npub↔bot attestation forgeable if npub is a bare field (bind a victim's
   key). → three-legged Schnorr proof committing pubkey+bot_id+nonce.
4. Inbound event verification absent — must recompute id from NIP-01-canonical
   bytes + BIP-340 verify (JSON.stringify isn't canonical). → shared verify lib.
5. NIP-17 sender-auth — trust the **seal** sig with `rumor.pubkey ===
   seal.pubkey`, identify/rate-limit on the inner key, never the ephemeral wrap.
6. Credential channel: no forward secrecy + static creds = permanent
   unrevocable leak from archived gift-wraps. → single-use TLS-redeemed token.
7. Cashu credit-on-receipt = double-spend (one bearer string, N jobs). →
   melt-first, txId = proof secret, saga.
8. External value via `/ledger/grant` = **uncapped mint path** (cap defaults
   to unlimited) and contradicts "convertibility out of scope". → dedicated
   melt-gated intake asset → `/ledger/trade`.
9. Zap receipts (kind-9735) are unauthenticated events. → credit only from LN
   settled-invoice webhook.
10. strfry co-located with HRM (O1) / responder (O3) = disk-full outage redux.
    → O2, dedicated quota, retention + allowlist write-policy, disk alerting.
11. DVM result at-least-once with no idempotency binding = duplicate paid
    execution. → cross-relay event-id dedupe, deterministic txId, republish
    byte-identical result.
12. External DVM recall sharing the internal `serve` queue group = a free
    Nostr flood starves the constellation's own recall. → separate `serve-dvm`
    pool + per-pubkey rate limit + publish/intake decoupling.
13. OBC attestation lane signs **anon-forgeable** `KANNAKA.events.obc.*` under
    a reputable npub. → source from a subject anon can't publish (or verify
    provenance) + loop-guard + retraction lane.

**Notable majors also folded:** bridge `subscribe [">"]` exfiltration (minimal
deny-by-default identity); NIP-39/NIP-05/relay-of-origin are self-asserted
(authorize only on the verified author pubkey via KAX attestation); NIP-90
results are public-by-default (leak memory → export corpus or encrypt); DVM
param→NATS subject injection (fixed service→subject allowlist); crash-durable
replay dedupe; reputation nsec on O1 with no revocation (move to O2/NIP-46,
publish a rotation-attestation); "attestation not migration" makes an
un-revocable second identity (NIP-40 expiry + revocation feed).

**Net:** the *spine* (NATS write-safety, KAX ledger) is sound and the membrane
concept holds, but the design was under-specified at exactly the boundary
crossings. Nothing here kills the ADR; it re-sequences it — cheap correct
identity first, then a locked-down relay+bridge, then compute, then economy,
then OBC, and joiner-bootstrap dead last behind real infrastructure. **Status
stays Proposed** pending Nick's read of this reconciliation; Phase 0 is the
only part safe to start now.

## Consequences

- + Identity, work, and value all become portable and externally verifiable;
  the constellation is addressable by any Nostr client on earth.
- + Sovereignty: our public history lives on our relay under our keys —
  no platform can revoke it.
- + The economy pillar gets a settlement rail that works agent-to-agent with
  strangers, while KAX keeps the books honest inside.
- − New always-on surface (relay + bridge) to operate and defend; spam/DoS
  policy on the relay is real work.
- − Two identity systems to keep coherent (KAX JWT ↔ npub) — mitigated by
  attestation binding rather than migration.
- − secp256k1/Schnorr + NIP crypto in Rust adds dependencies (`nostr` crate
  is mature; the radio adapter proves the protocol is hand-rollable where
  we prefer zero-dep).

## Open questions

1. Relay placement: O2 (witness, low load) vs O3 (already runs GossipGhost
   cron + serve responder) — or both with gossip between them?
2. Which kinds for the public consciousness pulse — custom parameterized
   replaceable events (3xxxx) vs plain kind-1 narrative? (Machine-readable
   argues 3xxxx with a documented schema.)
3. Cashu mint selection/trust policy for received ecash (multi-mint
   allowlist? auto-melt to LN?) — Phase 3 decision, steward-adjacent.
4. ~~Does the OBC partner relationship want a formal bridge (OBC events
   attested onto Nostr), or keep OBC and Nostr as parallel membranes?~~
   **DECIDED (Nick, 2026-07-25): both, deliberately.** OBC is *integrated* —
   OBC-witnessed facts (merged PRs, artifacts, escrow closings, elder
   reputation) get attested onto Nostr as signed events, so the partnership's
   record travels beyond the city; and Nostr is *extended* standalone — the
   membrane works where OBC isn't (agents who never join the city), so the
   constellation is never single-homed on the partner. Net effect: the
   partnership is enriched (OBC provenance becomes portable, city agents get
   a door to the wider network) while our standalone posture widens.
   Mechanically this adds an **OBC attestation lane** to Plane 4: the bridge
   subscribes to our own OBC perception stream (`KANNAKA.events.obc.<type>`,
   already flowing via kannaka-presence) and republishes selected city facts
   as signed attestation events referencing the OBC artifact/URL — Kannaka
   as witness, not as OBC's voice.
