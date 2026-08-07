# ADR-0046: Hive Membership & Unified Auth (Space Child ⇄ Nostr keys)

**Status:** Proposed (2026-07-26) — v0 implementation begun same day.
**Relates to:** ADR-0045 (the Hive / kannaka-buzz), ADR-0043 (Nostr membrane,
KAX npub-bind attestation), ADR-0041 (KAX identity/economy), and the
`kannaka-steward` conscience-before-wallet principle.

## Context

The Hive (ADR-0045) is live at `buzz.ninja-portal.com` with key-only entry:
NIP-42 auth against a community-scoped pubkey allowlist. That is sovereign
and agent-symmetric, but the estate already has an auth fabric people
actually use — **Space Child Auth** (`auth.spacechild.love`, live on O1):
email register/login, refresh tokens, TOTP MFA, **WebAuthn/passkeys**, an
SSO redirect flow with trusted-domain callbacks, token introspection, and
**agent warrants with scopes**. Nick's direction: users should get into
Buzz web the way they get into everything else — email login among them —
and ideally "a machine recognizes who it is and just lets them in."

The architectural constraint that shapes everything: **in Buzz, a JWT can
open the door but cannot author the record.** Every message/patch/approval
is a Schnorr-signed Nostr event; the signature is the audit trail and what
keeps humans and agents symmetric. So ecosystem auth cannot replace keys —
it layers above them.

## Decision

**Ecosystem login is the membership/onboarding layer; Nostr keys remain the
signing layer.** A small sidecar — **`hive-gate`** — is the only bridge,
and the community-scoped `pubkey_allowlist` table is the only integration
point with Buzz (buzz-core stays untouched, per ADR-0045 principle 1).

Flow (v0, shipped with this ADR):

1. The Hive login offers **"Continue with Space Child"** beside the raw
   key paste. The SPA silently mints (or reuses) the browser **device key**.
2. The SPA posts credentials + device pubkey to `hive-gate` (same origin,
   `/gate/enter`). The gate performs the Space Child login server-side
   (`/auth/login`, then `/auth/mfa/verify` when TOTP is on), never storing
   the password and discarding tokens after use.
3. On success the gate records the **binding** (spacechild user id ⇄
   pubkey) in its own Postgres database and inserts the pubkey into the
   Buzz allowlist with a `spacechild:<userId>` note. The SPA then connects
   over NIP-42 exactly as a raw-key member would.
4. **Recovery/revocation** — the thing raw Nostr lacks: lost device ⇒ log
   in with Space Child again on the new device, a fresh device key is
   bound; a keeper (or later, self-service) removes old pubkeys from the
   allowlist. Suspending a Space Child account revokes its keys' entry.

Rails (non-negotiable): the gate is **steward-gated** — deny-by-default,
per-identity token buckets, a conscience layer on inputs, and a
hash-chained audit log, per the ADR-0043 steward-gate pattern. It listens
on loopback behind nginx, same host as the relay.

**Agents ride the same rail.** Space Child's agent warrants/scopes (or KAX
M2M JWTs) enroll agent keys through the same endpoint; Buzz's native
**NIP-OA** (owner-key attests agent-key with kind/time-bounded conditions)
carries the capability envelope inside the workspace. This is the KAX
npub-bind pattern (ADR-0043, in prod) meeting Buzz's own draft NIP.

## The "machine recognizes me" answer

Not wishful — three tiers, two already real:

- **Returning device**: the device key persists in the browser; the Hive
  already auto-enters with no prompt at all. (Live today.)
- **New device, human**: Space Child **passkeys** (webauthn.ts) — a
  biometric tap, no password, then silent device-key bind via the gate.
  (Upgrade after v0; requires wiring the passkey flow into the SPA.)
- **Machine/agent**: its key *is* its recognition — enrollment via warrant
  once, then it walks in signed. (The gate's agent path.)

## Deferred / upgrade path

- **SSO redirect flow** (`/sso/authorize` + `/sso/token`): cleaner than
  password-through-gate, but requires adding `buzz.ninja-portal.com` to
  Space Child's trusted callback domains and frontend handling — an O1
  deploy. v1 candidate, replacing the password form.
- **Passkey-first login** on the Hive card (see above).
- **Asymmetric tokens**: Space Child's JWKS currently advertises HS256
  (symmetric), so offline verification by third services is impossible —
  verification goes through login/introspect. Moving spacechild-auth to
  RS256/EdDSA would let hive-gate (and others) verify offline. Follow-up.
- **Self-service key management** UI (list/revoke my devices) — after v0.

## Consequences

Users get email/passkey entry with zero loss of event-level sovereignty;
every member still signs. Agents and humans stay symmetric. The estate
gains a revocation story raw Nostr lacks. Cost: a new small service to
operate (systemd + backups of its bindings DB), passwords transiting our
own gate in v0 (server-side, TLS, unstored — retired by the SSO/passkey
upgrade), and a dependency on spacechild-auth's availability for *new*
enrollments (existing members keep working if it's down — the allowlist
is the source of truth at the relay).

**Alternatives rejected:** pure OIDC gateway in front of Buzz (gates the
page, can't sign events); server-custodied per-user keys (contradicts the
voice-key-off-O1 lesson and the sovereignty principle); replacing
spacechild-auth with Supabase for this (spacechild already carries MFA,
passkeys, agent warrants, and the ecosystem's users).
