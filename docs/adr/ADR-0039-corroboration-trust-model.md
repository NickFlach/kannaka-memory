# ADR-0039 — The corroboration trust model: identity says who, corroboration proves what

- Status: Accepted (2026-07-07)
- Date: 2026-07-07
- Repo: `kannaka-memory` (absorb-side gate + reputation engine + beacons)
- Related: ADR-0035 (swarm sensemaking), the increment-0 read-side gate (v0.10.6, PR #507/#508), the increment-1 corroboration substrate (v0.10.7, PRs #509–#513), the activate-gate/beacon helpers (PR #514). Threat: the 2026-07-06 anonymous NATS AIID injection.
- Code of record: `src/absorb_gate.rs` (admit chokepoint), `src/reputation.rs` (pubkey-keyed trust core), `src/provenance.rs` (ed25519 sign/verify + replay), `src/beacon.rs` (anti-eclipse heartbeats), `src/config.rs:202-267` (`SwarmTrustConfig` tunables), `src/bin/handlers/gate.rs` (seed-ceremony CLI).

## Context

The Kannaka swarm gossips over an **open** NATS server: `no_auth_user:anon` is
allowed to publish to `KANNAKA.events.>` on purpose (it keeps the swarm
zero-friction to join, and quantum/lab nodes come and go). That openness is a
standing injection surface.

### Threat model — the 2026-07-06 anon injection

On 2026-07-06 a single anonymous socket (traced to a Byteplus/Singapore VM)
published a burst of AIID-flavoured memories into `KANNAKA.events.>` while
**spoofing 48 distinct `agent_id` strings** on the one connection. The wire
`agent_id` is a free-text field an anonymous publisher fully controls, so "48
agents agree" was really "one attacker typed 48 names." Every downstream
consumer that trusted the wire `agent_id` — metric fan-in, absorb, sensemaking
corroboration counting — was, in principle, corruptible by one host.

Two lessons crystallised into the principle this ADR names:

- **Identity is not a wire string.** The only non-forgeable identity on an open
  bus is a cryptographic keypair: what a message proves is "the holder of
  private key *k* signed these exact bytes," never "the sender is who the
  `agent_id` field claims."
- **Agreement must be counted over distinct *keys*, not distinct *names*.** A
  claim is corroborated when **≥ K distinct trusted keypairs independently
  signed-and-remembered the same content** — a property one host cannot fake by
  renaming itself, because it does not hold the other keys.

> **identity says who, corroboration proves what.**
> The signature binds a memory to a keypair (who). Promotion past Quarantine
> requires independent signatures from other trusted lineages (what). Neither
> half alone admits content: an unknown key with 100 corroborations is still
> untrusted, and a trusted key asserting something alone is still only
> Quarantine.

We ship this in two increments, and the strong half stays **dormant by default**
so the open swarm keeps working until an operator deliberately arms it.

## Decision

### Increment-0 — the read-side gate (ACTIVE, v0.10.6)

inc-0 is a defensive read-side filter that assumes every wire field is
attacker-controlled. It does not need keys and is **on by default**
(`src/config.rs:202-231`, `SwarmTrustConfig`):

- **`trusted_agents`** — an allowlist of agent-ids (exact or `prefix*`, e.g.
  `qos-*`). Env `KANNAKA_TRUSTED_AGENTS` (comma-separated, REPLACES the list).
- **`metrics_trusted_only`** (default `true`) — only allowlisted phases (plus
  this node's own) feed swarm metrics; a phase from an unlisted id never drives
  the pairwise Kuramoto step. Escape hatch `KANNAKA_METRICS_TRUSTED_ONLY=0`.
- **`wire_trust_cap`** — every kept phase's attacker-supplied `trust_score` is
  clamped to this ceiling, so a wire message cannot assert its own high trust.

inc-0 is an **interim allowlist**: it limits blast radius (the 48-name spoof
stops mattering for metrics because none of the 48 names are allowlisted) but it
is still a name-based gate. inc-1 replaces the *trust decision* with keys while
inc-0 stays as the metric-fan-in filter.

### Increment-1 — the corroboration gate (DORMANT by default, v0.10.7)

inc-1 moves the trust decision onto cryptographic identity and independent
corroboration. It is composed at one write-side chokepoint,
`absorb_gate::admit()`, which **every** wire→store absorb path routes through.

**Substrate (inc-1a, `src/provenance.rs`).** ed25519 sign/verify over the
*canonical bytes* of a memory, with a replay LRU. The signature binds a fixed
`SIGN_AGENT_ID = "kannaka-swarm"` and a tier discriminant into the signed bytes
— the pubkey is the identity; the forgeable wire agent-id is deliberately **not**
bound (`src/absorb_gate.rs:32-35`). Every node always-signs its own
`memory.new`/exemplar emits, gated behind nothing: signing is additive and
harmless while the gate is off, and it is the ONLY way corroboration can accrue
*before* an operator flips the gate on.

**Reputation core (inc-1b, `src/reputation.rs`).** A pubkey-keyed trust store
rooted at operator-pinned **seeds**. A "corroboration" of a piece of content
(join key `blake3(normalize(content))`) is simply *another distinct trusted key
independently signing-and-remembering the same content* — there is no separate
endorsement message. `RepStore::decide()` promotes a content hash to `Live` when
enough distinct trusted lineages have signed it within a freshness window;
otherwise it lands in **Quarantine** (never dropped — legitimate content is
preserved and promotes later once corroboration arrives).

**Unconditional sanitization runs even while dormant.** `admit()` always clamps
`amplitude`/`phase`/`frequency` to finite ranges and **forces `hallucinated` to
the local default, never the wire value** — an attacker must not be able to
set or clear the immune-system flag over the wire (`src/absorb_gate.rs:10-14`).
Only the *promotion* half is conditional.

**Dormant contract.** With `corroboration_gate_enabled=false` and no seeds (both
defaults), `admit()` returns `Live` with the sanitized fields: byte-for-byte
inc-0 behaviour. The swarm keeps working until an operator runs the ceremony.

### Trust tunables (`SwarmTrustConfig`, `src/config.rs:219-267`)

| Field | Env | Default | Meaning |
|---|---|---|---|
| `trust_threshold` | `KANNAKA_TRUST_THRESHOLD` | (θ) | pubkey trust ≥ θ is Live-eligible; below ⇒ Quarantine |
| `theta_lo` | `KANNAKA_THETA_LO` | 0.4 | lower hysteresis: continuous weight `w(rep)=0` below this |
| `theta_hi` | `KANNAKA_THETA_HI` | 0.7 | upper hysteresis: `w` reaches 1.0 and a handle *arms* at/above |
| `accrual_alpha` | `KANNAKA_ACCRUAL_ALPHA` | 0.05 | per-promotion rep accrual coefficient α (also the per-epoch accrual cap) |
| `epoch_length_ms` | `KANNAKA_EPOCH_LENGTH_MS` | 60000 | corroboration freshness window (ms) |
| `beacon_grace_epochs` | `KANNAKA_BEACON_GRACE_EPOCHS` | 3 | epochs of missed seed beacons tolerated before failing CLOSED |
| `seed_pubkeys` | `KANNAKA_SEED_PUBKEYS` | *(empty)* | operator-pinned base64 ed25519 seed keys — the root of every lineage |
| `corroboration_gate_enabled` | `KANNAKA_CORROBORATION_GATE` | `false` | master switch for the promotion gate |

Hysteresis (θ_lo/θ_hi) exists so a key hovering near the threshold does not
flap Live/Quarantine epoch to epoch; α bounds how fast rep can be farmed.

### Activation contract — the seed ceremony

Arming the gate is a deliberate, guided operator action (`kannaka swarm
activate-gate`, `src/bin/handlers/gate.rs`), **reserved for the human operator**,
not any automatic path. Two invariants are enforced:

1. **≥ 2 distinct seed lineages.** `activate-gate` refuses to arm with fewer
   than two pinned seeds. A single-seed root can never corroborate — corroboration
   needs ≥ 2 distinct lineages — so a one-seed gate would freeze *every*
   promotion at Quarantine forever. `--force` bypasses the refusal ONLY for a
   deliberate single-seed bootstrap, and still requires `--yes` to write. The
   default is a **dry run**: it prints exactly what would change and writes
   nothing without `--yes`.

2. **Beacon liveness is fail-closed (anti-eclipse).** An armed gate additionally
   requires a **fresh seed beacon** to promote. Each seed runs a signed
   heartbeat emitter (`kannaka swarm beacon --loop`, one beacon per
   `epoch_length_ms`). Freshness rides inside the `QuarantineStaging`
   `BeaconTracker` that `admit()` already threads (`src/absorb_gate.rs:46-53`);
   the swarm receive path feeds it via `ingest_beacon`. If beacons go stale or
   absent for more than `beacon_grace_epochs` (default 3 × 60s = 180s) — an
   eclipse, a partition, or a dead beacon emitter — promotion **freezes to
   Quarantine, never Drop**. Content is held, not lost, and resumes promoting
   the moment beacons return. This is why the NATS subscription liveness fix
   (issues #499/#500) is load-bearing: a subscriber that hangs deaf on a
   silently-dead socket would stop seeing beacons and freeze all promotions.

Post-arming verification: `kannaka reputation list` (pinned seeds show
`STATUS=Seed`) and `kannaka swarm status`.

## Consequences

- **Positive.** The 48-name spoof is structurally defeated once armed: agreement
  is counted over keys the attacker does not hold. Fail-closed under eclipse
  means the *worst* an availability attack achieves is frozen promotions
  (degraded, not corrupted). Dormant-by-default means shipping the code changes
  no running behaviour.
- **Negative / cost.** Arming requires an operator ceremony and ≥ 2 live seed
  emitters; a partition that starves beacons degrades to Quarantine-only. These
  are deliberate trades: availability yields to integrity, and only when an
  operator opts in.
- **Interaction with inc-0.** inc-0 stays as the metric-fan-in filter even after
  inc-1 arms; the two are complementary (name-based blast-radius limit vs.
  key-based trust decision).

## Deferred work

Explicitly out of scope for v0.10.6/v0.10.7, tracked for the seed-ceremony pass
and beyond:

1. **`K_high` multi-lineage rule for high-impact memories** (`src/absorb_gate.rs:36-38`).
   `high_impact` is currently an amplitude proxy (clamped amplitude ≥
   `HIGH_IMPACT_AMPLITUDE`) and a high-impact memory should have to clear a
   *higher* distinct-lineage count `K_high` than ordinary content. There is no
   category taxonomy yet; the elevated bar is deferred to the seed-ceremony pass.

2. **`reserved_prefixes` enrollment alarm** (`src/config.rs:226-231`). Agent-id
   prefixes/exact-names reserved for operator enrollment: a first-sight or
   self-serve enrollment attempt for a matching id must raise an **alarm**, never
   an auto-pin. The field exists and is inert until the enrollment layer is wired.

3. **L3 server-side untrusted-lane split.** Today the gate is entirely
   client/read-side over one open subject tree. A stronger posture splits the
   NATS subject space (or adds server-side ACLs) so anonymous publishers land on
   a distinct *untrusted lane* that trusted consumers treat as suspect by
   construction, rather than every node re-deriving trust from scratch.

## Status of the trust config

The seed set (`seed_pubkeys`) and the master switch
(`corroboration_gate_enabled`) are **left empty/false in the shipped default and
in the deployed `~/.kannaka/config.toml`**. Arming them is the operator seed
ceremony (reserved for Nick) and is intentionally NOT performed by this ADR or by
any release.
