# ADR-0055 — Canonical Organ Key Format

**Status:** Accepted (decision delegated by Nick 2026-08-10; see #635)
**Date:** 2026-08-10
**Relates to:** ADR-0043 (Nostr membrane), ADR-0045 principle 2 (workspace-scoped
keys minted per organ), #635 (the blocking issue)

## Context

Two on-disk shapes for the same organ key existed, and nothing had chosen
between them.

What minting delivered, and what now sits at
`~/.secrets/kannaka-hive-0xscada-qe-nostr.json`:

```json
{ "nsec": "nsec1…", "npub": "npub1…", "pubkey": "<64 hex>", "organ": "0xscada-qe" }
```

What the daemons read:

```json
{ "privkey": "<64 hex>", "pubkey": "<64 hex>" }
```

Different field names, and a bech32 `nsec` against a raw hex `privkey`. This was
not cosmetic: `kannaka-hive-bridge` **could not load the key it had been handed**,
which is why #635 was filed as a blocker rather than a tidy-up.

The reason it was not fixed in passing is sound — reconciling it while seating a
single organ would have set a convention for every organ (`radio`, `eye`,
`staff`, …) on the way past. That is a decision to make once, deliberately.

## Decision

**Readers accept either shape. The canonical file to WRITE is the superset
already delivered. Nothing rewrites an existing file.**

Canonical write shape:

```json
{ "nsec": "nsec1…", "npub": "npub1…", "pubkey": "<64 hex>", "organ": "<organ>" }
```

Reader contract, implemented once in `src/nostr/organ_key.rs`:

| field | requirement |
|---|---|
| secret | `privkey` (64 hex) **or** `nsec` (bech32) — at least one; if both are present they must name the same key |
| `pubkey` | optional; when present must match the pubkey derived from the secret |
| `npub` | optional; same cross-check |
| `organ` | optional; when the caller declares an expected organ *and* the file carries one, they must match |

Location convention: `~/.secrets/<service>-<organ>-nostr.json`, mode 0600 —
the existing de-facto pattern, matching `moltbook-0xscada-qe.json`.

Every cross-check is a **hard refusal**, not a warning.

## Rationale

Judged against the criteria in priority order.

**(a) Existing on-disk data stays readable — decisive.** Both shapes already
exist in the wild. Picking one and converting breaks the other and requires a
migration pass over *secret material*. Accepting both breaks neither and needs
no pass at all. Because nothing is ever rewritten, the one-way-format hazard
that governs `.hrm` changes has no analogue here — there is no first write to
snapshot before.

**(b) Stable across restarts and hosts.** Both encodings carry the same 32
secret bytes; `nsec` is bech32 over exactly those. Identity is therefore
invariant under either encoding, and copying a file between hosts changes
nothing.

**(c) Unblocks `kannaka-hive-bridge` immediately.** The daemon reads the
delivered file as-is: no conversion, no re-mint, no operator ceremony.

Accept-either is also nearly free to build: `Keypair::from_nsec` already exists
and `bech32` is already a dependency under the `nostr` feature. This is a
loader, not a new primitive.

### Why `pubkey` is derived, not trusted

The loader derives the public key from the secret and treats any stored
`pubkey`/`npub` as an assertion to *check*, never as the value to use. A file
whose stored pubkey disagrees with its secret is refused.

This matters more than it looks. If a daemon advertised one identity while
signing as another, every signature it produced would still be individually
valid — nothing downstream would flag it. The failure is silent by
construction, so it has to be caught at load.

### Why `organ` is optional but checked

Making it **required** would break the legitimate `{privkey,pubkey}` files that
lack it, failing criterion (a). Making it **ignored** would waste the only
safeguard against pointing a daemon at the wrong organ's key — a mistake that
puts the wrong identity on a relay, which is the failure custody rules exist to
prevent.

So it is optional, and enforced only when both the caller's expectation
(`HIVE_ORGAN` / `BRIDGE_ORGAN`) and the file's value are present. Absent stays
permitted and unremarkable.

## Consequences

- `kannaka-hive-bridge` and `kannaka-nostr-bridge` share one loader; both lose
  the `key["privkey"].as_str().expect("privkey")` pattern, which panicked with
  no indication of which field was wrong.
- Malformed key files now exit(1) with an operator-readable message naming the
  specific inconsistency.
- No migration. No conversion script. Existing deployments keep working
  untouched, in both shapes.
- Minting should emit the canonical superset going forward; it already does.

## Not decided here

The `kannaka` CLI path to load an organ key (`kannaka nostr` currently mints
keys but deliberately never persists or reloads one) remains open, and belongs
with the wider organ identity layer alongside the rest of the custody story.
