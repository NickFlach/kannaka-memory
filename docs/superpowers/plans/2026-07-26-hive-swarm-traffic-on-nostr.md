# Hive Swarm Traffic on `/nostr` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bridge agent and human room traffic from the Hive (`kannaka-buzz`) onto the NATS spine so the `nats` app's `/nostr` page monitors swarm agent communication alongside the existing ADR-0043 DM membrane.

**Architecture:** A new daemon in `kannaka-memory` holds a NIP-42-authenticated WebSocket to the buzz relay, subscribes to room messages, the agent job lifecycle, and agent profiles, gates every event through a per-channel bridge policy, and republishes onto three `KANNAKA.events.hive.*` subjects. The browser picks those up over its existing NATS connection; a JetStream stream gives 90-day scrollback. Two upstreamable relay fixes in `kannaka-buzz` make the job lifecycle correlatable and give channel owners an opt-out.

**Tech Stack:** Rust (buzz relay + bridge daemon, `tungstenite` 0.24, `k256` schnorr), PostgreSQL (buzz), NATS + JetStream, TypeScript / React / TanStack Start (the `nats` app).

**Spec:** `docs/superpowers/specs/2026-07-26-hive-swarm-traffic-on-nostr-design.md`

## Global Constraints

- **Three repos.** Tasks 1–4 are in `flaukowski/kannaka-buzz`; tasks 5–8 and 14 in `NickFlach/kannaka-memory`; tasks 9–13 in `NickFlach/nats`. Never mix repos in one commit.
- **buzz changes must be upstreamable to `block/buzz`.** No Kannaka-specific naming, no estate endpoints, no references to NATS in relay code. The feature is "channel export policy" and "job lifecycle correlation", both generic. Per `KANNAKA.md`: no secrets, keys, credentials, or estate-internal endpoints in that repository.
- **Bridge module is named `hive_bridge`, not `hive`.** `kannaka-memory/src/lib.rs:54` already declares `pub mod hive_formation;` (a swarm-cohesion concept, unrelated). A module named `hive` beside it would be actively misleading.
- **The bridge daemon reuses the existing `bridge` Cargo feature** (`Cargo.toml:111`, `bridge = ["nostr", "tungstenite"]`). Do not add a new feature.
- **Subject names, exactly:** `KANNAKA.events.hive.msg`, `KANNAKA.events.hive.job`, `KANNAKA.events.hive.agent`.
- **No NATS ACL change.** `config/nats-accounts.conf` already grants `anon` subscribe on `KANNAKA.events.>`. The bridge publishes as `kannaka_internal`.
- **The `nats` app has no test runner.** `package.json` exposes only `dev`, `build`, `lint`, `format`. Verification there is `npx tsc --noEmit` plus fixture scripts run with bare `node`, following the existing `scripts/test-agent-mcp.mjs` precedent.
- **`src/lib/nats/hive.ts` must use erasable-only TypeScript** — type annotations and `interface` only; no `enum`, no parameter properties, no `namespace`. Node 24's native type stripping runs the fixture script directly against the `.ts` source, and non-erasable syntax breaks it.
- **Fail closed.** Any channel whose bridge policy the daemon has not resolved is not bridged. Absence of information is never permission.

---

## File Structure

**`flaukowski/kannaka-buzz`**

| File | Responsibility |
|---|---|
| `crates/buzz-relay/src/handlers/ingest.rs` | Job-lifecycle correlation validation (structural + referential) |
| `migrations/0025_channel_no_bridge.sql` | `no_bridge` column on `channels` |
| `crates/buzz-db/src/channel.rs` | `ChannelRecord.no_bridge`, `ChannelUpdate.no_bridge` |
| `crates/buzz-relay/src/handlers/side_effects.rs` | Parse `no-bridge` on kind 9002; emit it on kind 39000 |

**`NickFlach/kannaka-memory`**

| File | Responsibility |
|---|---|
| `src/hive_bridge/mod.rs` | Module root, shared payload types, re-exports |
| `src/hive_bridge/map.rs` | Pure `Event` → (subject, JSON payload) mapping |
| `src/hive_bridge/policy.rs` | Channel policy map built from kind 39000; fail-closed gate |
| `src/hive_bridge/roster.rs` | Agent set from kind 10100; display names from kind 0 |
| `src/bin/kannaka_hive_bridge.rs` | Network plumbing: NIP-42 AUTH, NIP-29 REQ, NATS publish |

**`NickFlach/nats`**

| File | Responsibility |
|---|---|
| `src/lib/nats/schema.ts` | Three subjects + three payload interfaces |
| `src/lib/nats/hive.ts` | Pure fold: flat events → `HiveRoom[]` |
| `src/lib/nats/hive-history.functions.ts` | `getHiveHistory` server fn over the `KANNAKA_HIVE` stream |
| `src/routes/nostr.tsx` | `HIVE` rail block + `HIVE ROOMS` panel |
| `src/lib/mcp/tools/hive-traffic.ts` | `hive_traffic` MCP tool |
| `scripts/test-hive-fold.mjs` | Fixture test for the fold |

---

## Task 1: Structural job-lifecycle correlation (buzz)

Kinds 43002–43006 currently carry no enforced link to the 43001 that opened the job. Require exactly one `e` tag.

**Files:**
- Modify: `crates/buzz-relay/src/handlers/ingest.rs` (near `required_scope_for_kind`, ~line 304)
- Test: same file, existing `mod tests` (~line 2846)

**Interfaces:**
- Produces: `fn validate_job_correlation(kind: u32, event: &Event) -> Result<(), &'static str>`

- [ ] **Step 1: Write the failing test**

Add to the `mod tests` block in `crates/buzz-relay/src/handlers/ingest.rs`:

```rust
#[test]
fn job_request_needs_no_e_tag() {
    let ev = make_dummy_event();
    assert!(validate_job_correlation(KIND_JOB_REQUEST, &ev).is_ok());
}

#[test]
fn job_followups_require_exactly_one_e_tag() {
    let none = make_dummy_event();
    for kind in [
        KIND_JOB_ACCEPTED,
        KIND_JOB_PROGRESS,
        KIND_JOB_RESULT,
        KIND_JOB_CANCEL,
        KIND_JOB_ERROR,
    ] {
        assert!(
            validate_job_correlation(kind, &none).is_err(),
            "kind {kind} with no e tag must be rejected"
        );
    }
}

#[test]
fn non_job_kinds_are_unaffected() {
    let ev = make_dummy_event();
    assert!(validate_job_correlation(KIND_STREAM_MESSAGE, &ev).is_ok());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p buzz-relay job_followups_require_exactly_one_e_tag`
Expected: FAIL — `cannot find function 'validate_job_correlation' in this scope`

- [ ] **Step 3: Write the minimal implementation**

Add above `required_scope_for_kind` in the same file:

```rust
/// Agent job-lifecycle events 43002–43006 must reference the kind-43001 that
/// opened the job, via exactly one `e` tag. Upstream defines these kinds but
/// never constrained their shape, which leaves every consumer guessing at the
/// correlation. Kind 43001 opens a job and references nothing.
fn validate_job_correlation(kind: u32, event: &Event) -> Result<(), &'static str> {
    if !matches!(
        kind,
        KIND_JOB_ACCEPTED
            | KIND_JOB_PROGRESS
            | KIND_JOB_RESULT
            | KIND_JOB_CANCEL
            | KIND_JOB_ERROR
    ) {
        return Ok(());
    }
    let e_count = event
        .tags
        .iter()
        .filter(|t| t.kind().to_string() == "e")
        .count();
    match e_count {
        1 => Ok(()),
        0 => Err("invalid: job event must reference its 43001 request via an e tag"),
        _ => Err("invalid: job event must carry exactly one e tag"),
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p buzz-relay job_`
Expected: PASS — including the pre-existing `job_lifecycle_kinds_require_messages_write`

- [ ] **Step 5: Wire it into the ingest path**

In `ingest_event`, immediately after the `required_scope_for_kind` match resolves (~line 1546, before the relay-admin checks):

```rust
    if let Err(msg) = validate_job_correlation(kind_u32, &event) {
        return Err(IngestError::Rejected(msg.into()));
    }
```

- [ ] **Step 6: Verify the crate still builds and all relay tests pass**

Run: `cargo test -p buzz-relay`
Expected: PASS, no new failures

- [ ] **Step 7: Commit**

```bash
git add crates/buzz-relay/src/handlers/ingest.rs
git commit -m "fix(relay): require job lifecycle events to reference their request

Kinds 43002-43006 were accepted with no enforced link back to the
43001 that opened the job, leaving consumers to guess the correlation.
Require exactly one e tag on the follow-up kinds."
```

---

## Task 2: Referential job validation (buzz)

Structural validation proves an `e` tag exists. This proves it points at a real 43001 in the same channel.

**Files:**
- Modify: `crates/buzz-relay/src/handlers/ingest.rs`

**Interfaces:**
- Consumes: `validate_job_correlation` from Task 1
- Produces: referential check inside `ingest_event`; no new public surface

- [ ] **Step 1: Write the failing test**

Add to `mod tests`:

```rust
#[test]
fn job_e_tag_target_is_extracted() {
    let mut ev = make_dummy_event();
    ev.tags = vec![Tag::parse(["e", "aa".repeat(32).as_str()]).unwrap()];
    assert_eq!(
        job_referenced_request(&ev).as_deref(),
        Some("aa".repeat(32).as_str())
    );
}

#[test]
fn job_e_tag_absent_yields_none() {
    let ev = make_dummy_event();
    assert!(job_referenced_request(&ev).is_none());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p buzz-relay job_e_tag`
Expected: FAIL — `cannot find function 'job_referenced_request'`

- [ ] **Step 3: Implement the extractor**

Beside `validate_job_correlation`:

```rust
/// The event id a job follow-up references, from its single `e` tag.
/// Returns `None` when absent — callers run after `validate_job_correlation`,
/// which already rejects that case for the follow-up kinds.
fn job_referenced_request(event: &Event) -> Option<String> {
    event
        .tags
        .iter()
        .find(|t| t.kind().to_string() == "e")
        .and_then(|t| t.content().map(|v| v.to_string()))
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p buzz-relay job_e_tag`
Expected: PASS

- [ ] **Step 5: Add the referential check to the async ingest path**

In `ingest_event`, after the structural check added in Task 1 and after `channel_id` has been resolved:

```rust
    if matches!(
        kind_u32,
        KIND_JOB_ACCEPTED
            | KIND_JOB_PROGRESS
            | KIND_JOB_RESULT
            | KIND_JOB_CANCEL
            | KIND_JOB_ERROR
    ) {
        let target = job_referenced_request(&event)
            .ok_or_else(|| IngestError::Rejected("invalid: job event missing e tag".into()))?;
        let referenced = state
            .db
            .get_event(tenant.community(), &target)
            .await
            .map_err(|e| IngestError::Internal(e.to_string()))?;
        match referenced {
            Some(req) if event_kind_u32(&req.event) == KIND_JOB_REQUEST
                && req.channel_id == channel_id =>
            {
                // correlated
            }
            Some(_) => {
                return Err(IngestError::Rejected(
                    "invalid: job e tag must reference a 43001 in the same channel".into(),
                ))
            }
            None => {
                return Err(IngestError::Rejected(
                    "invalid: job e tag references an unknown event".into(),
                ))
            }
        }
    }
```

- [ ] **Step 6: Confirm the DB accessor name**

`get_event` is assumed. Confirm the actual single-event lookup on `state.db`:

Run: `grep -n "pub async fn get_event\|pub async fn get_event_by_id" crates/buzz-db/src/*.rs`
Expected: one match. If the name or signature differs, adjust the call above to match — do not add a new DB method.

- [ ] **Step 7: Run the full relay test suite**

Run: `cargo test -p buzz-relay`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add crates/buzz-relay/src/handlers/ingest.rs
git commit -m "fix(relay): verify job e tag resolves to a 43001 in the same channel

Structural validation proved an e tag was present; this proves it points
at a real job request in the same channel, so a correlated lifecycle
cannot be forged by referencing an arbitrary event."
```

---

## Task 3: `no_bridge` channel column (buzz)

**Files:**
- Create: `migrations/0025_channel_no_bridge.sql`
- Modify: `crates/buzz-db/src/channel.rs`

**Interfaces:**
- Produces: `ChannelRecord.no_bridge: bool`, `ChannelUpdate.no_bridge: Option<bool>`

- [ ] **Step 1: Write the migration**

Create `migrations/0025_channel_no_bridge.sql`:

```sql
-- Channel export policy: when true, this channel's events must not be
-- replicated off this relay by any bridge or exporter. Defaults to false so
-- existing channels are unaffected.
ALTER TABLE channels ADD COLUMN no_bridge BOOLEAN NOT NULL DEFAULT FALSE;
```

- [ ] **Step 2: Add the field to `ChannelRecord`**

In `crates/buzz-db/src/channel.rs`, inside `pub struct ChannelRecord` (~line 21), after `visibility`:

```rust
    /// Channel export policy — when true, no bridge may replicate this
    /// channel's events off the relay.
    pub no_bridge: bool,
```

- [ ] **Step 3: Add the field to `ChannelUpdate`**

In the same file, inside `pub struct ChannelUpdate` (~line 1033):

```rust
    /// New export policy, or `None` to leave unchanged.
    pub no_bridge: Option<bool>,
```

- [ ] **Step 4: Fix every resulting compile error**

Run: `cargo build -p buzz-db 2>&1 | grep -E "^error" -A 5 | head -60`

Add `no_bridge` to each `SELECT` column list, each `ChannelRecord { .. }` construction, each row mapping, and the `UPDATE` builder that consumes `ChannelUpdate`. Follow exactly how the adjacent `visibility` field is handled in each site.

- [ ] **Step 5: Verify the workspace builds**

Run: `cargo build --workspace`
Expected: success, no errors

- [ ] **Step 6: Run migrations against a dev database and verify the column**

Run: `just relay` in one shell to apply migrations, then:

```bash
PGPASSWORD=buzz_dev psql -h localhost -U buzz -d buzz -c "\d channels" | grep no_bridge
```

Expected: `no_bridge | boolean | not null default false`

- [ ] **Step 7: Commit**

```bash
git add migrations/0025_channel_no_bridge.sql crates/buzz-db/src/channel.rs
git commit -m "feat(db): add no_bridge channel export policy column

Lets a channel declare that its events must not be replicated off the
relay by any bridge or exporter. Defaults false; existing channels are
unaffected."
```

---

## Task 4: `no-bridge` on the wire (buzz)

Owners set the policy with a kind-9002 metadata edit; the relay reflects it on kind 39000 so any consumer can honour it.

**Files:**
- Modify: `crates/buzz-relay/src/handlers/side_effects.rs` (9002 handling; `emit_group_discovery_events` at ~line 960)

**Interfaces:**
- Consumes: `ChannelUpdate.no_bridge` from Task 3
- Produces: a `["no-bridge"]` tag on kind-39000 events for flagged channels

- [ ] **Step 1: Write the failing test**

Add to the tests module in `side_effects.rs`:

```rust
#[test]
fn no_bridge_tag_parsed_from_metadata_edit() {
    let tags = vec![
        Tag::parse(["h", "11111111-1111-1111-1111-111111111111"]).unwrap(),
        Tag::parse(["no-bridge", "true"]).unwrap(),
    ];
    assert_eq!(parse_no_bridge_tag(&tags), Some(true));
}

#[test]
fn no_bridge_tag_false_is_parsed() {
    let tags = vec![Tag::parse(["no-bridge", "false"]).unwrap()];
    assert_eq!(parse_no_bridge_tag(&tags), Some(false));
}

#[test]
fn absent_no_bridge_tag_leaves_policy_unchanged() {
    let tags = vec![Tag::parse(["name", "ops"]).unwrap()];
    assert_eq!(parse_no_bridge_tag(&tags), None);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p buzz-relay no_bridge_tag`
Expected: FAIL — `cannot find function 'parse_no_bridge_tag'`

- [ ] **Step 3: Implement the parser**

```rust
/// Read a `["no-bridge", "true"|"false"]` tag off a kind-9002 metadata edit.
/// `None` means the tag was absent — leave the stored policy unchanged.
fn parse_no_bridge_tag(tags: &[Tag]) -> Option<bool> {
    tags.iter()
        .find(|t| t.kind().to_string() == "no-bridge")
        .and_then(|t| t.content())
        .map(|v| v == "true")
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p buzz-relay no_bridge_tag`
Expected: PASS

- [ ] **Step 5: Apply the parsed policy in the 9002 handler**

Find where kind 9002 builds its `ChannelUpdate` and set the field, gated to owner/admin — the same authorization tier `name` and `about` already use, deliberately *not* the any-member tier that `topic`/`purpose` use:

```rust
    // Export policy is an owner/admin decision, same tier as name/about.
    if is_owner_or_admin {
        update.no_bridge = parse_no_bridge_tag(&event.tags);
    }
```

- [ ] **Step 6: Emit the tag on kind 39000**

In `emit_group_discovery_events`, in the 39000 tag block, immediately after the existing `private` push:

```rust
        if channel.no_bridge {
            tags.push(Tag::parse(["no-bridge"])?);
        }
```

- [ ] **Step 7: Verify the full suite**

Run: `cargo test -p buzz-relay`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add crates/buzz-relay/src/handlers/side_effects.rs
git commit -m "feat(relay): expose channel export policy on kind 39000

Owners and admins set no-bridge with a kind 9002 metadata edit; the
relay reflects it as a no-bridge tag on the group metadata event so
bridges and exporters can honour it without out-of-band configuration."
```

---

## Task 5: Event mapping (kannaka-memory)

Pure translation from a verified Nostr `Event` to a NATS subject and JSON payload. No IO.

**Files:**
- Create: `src/hive_bridge/mod.rs`
- Create: `src/hive_bridge/map.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `kannaka_memory::nostr::{Event, npub_from_pubkey_hex}`
- Produces:
  - `pub struct Mapped { pub subject: &'static str, pub payload: serde_json::Value }`
  - `pub fn map_event(event: &Event, ctx: &MapContext) -> Option<Mapped>`
  - `pub struct MapContext<'a> { pub channel_name: Option<&'a str>, pub author_name: Option<&'a str>, pub is_agent: bool, pub now_ms: i64 }`

- [ ] **Step 1: Register the module**

In `src/lib.rs`, after `pub mod hive_formation;` (line 54):

```rust
/// Hive (kannaka-buzz) → NATS bridge: pure mapping, policy, and roster logic.
/// Network plumbing lives in `src/bin/kannaka_hive_bridge.rs`.
#[cfg(feature = "bridge")]
pub mod hive_bridge;
```

Create `src/hive_bridge/mod.rs`:

```rust
//! Hive → NATS bridge logic (ADR-0045). The binary is plumbing; everything
//! testable lives here, mirroring how `nostr::bridge` relates to
//! `kannaka_nostr_bridge`.

pub mod map;
pub mod policy;
pub mod roster;

pub use map::{map_event, MapContext, Mapped};
pub use policy::PolicyMap;
pub use roster::Roster;
```

Create placeholder `src/hive_bridge/policy.rs` and `src/hive_bridge/roster.rs` each containing only a doc comment, so the module tree compiles. Tasks 6 and 7 fill them.

- [ ] **Step 2: Write the failing test**

Create `src/hive_bridge/map.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::nostr::Event;

    fn ev(kind: u32, tags: Vec<Vec<String>>, content: &str) -> Event {
        Event {
            id: "a".repeat(64),
            pubkey: "b".repeat(64),
            created_at: 1_800_000_000,
            kind,
            tags,
            content: content.to_string(),
            sig: "c".repeat(128),
        }
    }

    fn tag(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    fn ctx() -> MapContext<'static> {
        MapContext { channel_name: Some("ops"), author_name: Some("scribe"), is_agent: true, now_ms: 1_800_000_000_000 }
    }

    #[test]
    fn room_message_maps_to_msg_subject() {
        let e = ev(9, vec![tag(&["h", "chan-1"])], "hello");
        let m = map_event(&e, &ctx()).expect("kind 9 maps");
        assert_eq!(m.subject, "msg");
        assert_eq!(m.payload["type"], "hive_msg");
        assert_eq!(m.payload["channel_id"], "chan-1");
        assert_eq!(m.payload["content"], "hello");
        assert_eq!(m.payload["is_agent"], true);
        assert_eq!(m.payload["channel_name"], "ops");
    }

    #[test]
    fn job_request_carries_its_own_id_as_job_id() {
        let e = ev(43001, vec![tag(&["h", "chan-1"])], "build it");
        let m = map_event(&e, &ctx()).expect("kind 43001 maps");
        assert_eq!(m.subject, "job");
        assert_eq!(m.payload["phase"], "request");
        assert_eq!(m.payload["job_id"], e.id);
    }

    #[test]
    fn job_followup_takes_job_id_from_e_tag() {
        let e = ev(43004, vec![tag(&["h", "chan-1"]), tag(&["e", &"d".repeat(64)])], "done");
        let m = map_event(&e, &ctx()).expect("kind 43004 maps");
        assert_eq!(m.payload["phase"], "result");
        assert_eq!(m.payload["job_id"], "d".repeat(64));
    }

    #[test]
    fn job_followup_without_e_tag_yields_null_job_id() {
        let e = ev(43006, vec![tag(&["h", "chan-1"])], "boom");
        let m = map_event(&e, &ctx()).expect("kind 43006 maps");
        assert!(m.payload["job_id"].is_null());
    }

    #[test]
    fn agent_profile_maps_to_agent_subject() {
        let e = ev(10100, vec![], r#"{"channel_add_policy":"any","owner":"eeee"}"#);
        let m = map_event(&e, &ctx()).expect("kind 10100 maps");
        assert_eq!(m.subject, "agent");
        assert_eq!(m.payload["agent_hex"], e.pubkey);
        assert_eq!(m.payload["owner_hex"], "eeee");
    }

    #[test]
    fn message_without_h_tag_is_dropped() {
        let e = ev(9, vec![], "orphan");
        assert!(map_event(&e, &ctx()).is_none());
    }

    #[test]
    fn unhandled_kind_is_dropped() {
        let e = ev(7, vec![tag(&["h", "chan-1"])], "+");
        assert!(map_event(&e, &ctx()).is_none());
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test --features bridge hive_bridge::map`
Expected: FAIL — `cannot find function 'map_event'`

- [ ] **Step 4: Implement the mapper**

Prepend to `src/hive_bridge/map.rs`:

```rust
//! Pure `Event` → (subject suffix, JSON payload) mapping. No IO, no policy.

use crate::nostr::{npub_from_pubkey_hex, Event};
use serde_json::{json, Value};

/// Context the mapper needs but cannot derive from the event alone.
pub struct MapContext<'a> {
    pub channel_name: Option<&'a str>,
    pub author_name: Option<&'a str>,
    pub is_agent: bool,
    pub now_ms: i64,
}

/// A mapped event: the subject suffix (appended to the configured prefix)
/// and the JSON payload to publish.
pub struct Mapped {
    pub subject: &'static str,
    pub payload: Value,
}

fn tag_value<'a>(event: &'a Event, name: &str) -> Option<&'a str> {
    event
        .tags
        .iter()
        .find(|t| t.first().map(String::as_str) == Some(name))
        .and_then(|t| t.get(1))
        .map(String::as_str)
}

fn job_phase(kind: u32) -> Option<&'static str> {
    match kind {
        43001 => Some("request"),
        43002 => Some("accepted"),
        43003 => Some("progress"),
        43004 => Some("result"),
        43005 => Some("cancel"),
        43006 => Some("error"),
        _ => None,
    }
}

/// Map a verified event onto its NATS payload. Returns `None` for kinds the
/// bridge does not forward, and for message/job events with no `h` tag —
/// without a channel there is no policy to check, so they cannot be bridged.
pub fn map_event(event: &Event, ctx: &MapContext) -> Option<Mapped> {
    let npub = npub_from_pubkey_hex(&event.pubkey).ok()?;

    if event.kind == 10100 {
        let owner = serde_json::from_str::<Value>(&event.content)
            .ok()
            .and_then(|v| v.get("owner").and_then(Value::as_str).map(str::to_string));
        return Some(Mapped {
            subject: "agent",
            payload: json!({
                "type": "hive_agent",
                "event_id": event.id,
                "agent_hex": event.pubkey,
                "agent_npub": npub,
                "name": ctx.author_name,
                "owner_hex": owner,
                "ts": ctx.now_ms,
            }),
        });
    }

    let channel_id = tag_value(event, "h")?;

    if let Some(phase) = job_phase(event.kind) {
        let job_id = if event.kind == 43001 {
            Some(event.id.clone())
        } else {
            tag_value(event, "e").map(str::to_string)
        };
        return Some(Mapped {
            subject: "job",
            payload: json!({
                "type": "hive_job",
                "event_id": event.id,
                "channel_id": channel_id,
                "channel_name": ctx.channel_name,
                "phase": phase,
                "kind": event.kind,
                "job_id": job_id,
                "author_hex": event.pubkey,
                "author_npub": npub,
                "author_name": ctx.author_name,
                "content": event.content,
                "created_at": event.created_at,
                "ts": ctx.now_ms,
            }),
        });
    }

    if matches!(event.kind, 9 | 40002) {
        let reply_to = event
            .tags
            .iter()
            .find(|t| {
                t.first().map(String::as_str) == Some("e")
                    && t.get(3).map(String::as_str) == Some("reply")
            })
            .and_then(|t| t.get(1))
            .cloned();
        return Some(Mapped {
            subject: "msg",
            payload: json!({
                "type": "hive_msg",
                "event_id": event.id,
                "channel_id": channel_id,
                "channel_name": ctx.channel_name,
                "author_hex": event.pubkey,
                "author_npub": npub,
                "author_name": ctx.author_name,
                "is_agent": ctx.is_agent,
                "content": event.content,
                "reply_to": reply_to,
                "created_at": event.created_at,
                "ts": ctx.now_ms,
            }),
        });
    }

    None
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test --features bridge hive_bridge::map`
Expected: PASS — 7 tests

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/hive_bridge/
git commit -m "feat(hive-bridge): pure Nostr event to NATS payload mapping

Translates buzz room messages, the agent job lifecycle, and agent
profiles into the KANNAKA.events.hive.* payload contract. No IO, so the
whole surface is unit tested."
```

---

## Task 6: Channel policy, fail-closed (kannaka-memory)

**Files:**
- Modify: `src/hive_bridge/policy.rs`

**Interfaces:**
- Produces:
  - `pub struct PolicyMap` with `pub fn new() -> Self`, `pub fn apply_metadata(&mut self, event: &Event)`, `pub fn is_bridgeable(&self, channel_id: &str) -> bool`, `pub fn channel_name(&self, channel_id: &str) -> Option<&str>`, `pub fn len(&self) -> usize`

- [ ] **Step 1: Write the failing test**

Replace `src/hive_bridge/policy.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::nostr::Event;

    fn meta(channel: &str, name: &str, no_bridge: bool) -> Event {
        let mut tags = vec![
            vec!["d".to_string(), channel.to_string()],
            vec!["name".to_string(), name.to_string()],
        ];
        if no_bridge {
            tags.push(vec!["no-bridge".to_string()]);
        }
        Event {
            id: "a".repeat(64),
            pubkey: "b".repeat(64),
            created_at: 1_800_000_000,
            kind: 39000,
            tags,
            content: String::new(),
            sig: "c".repeat(128),
        }
    }

    #[test]
    fn unknown_channel_is_not_bridgeable() {
        let p = PolicyMap::new();
        assert!(!p.is_bridgeable("never-seen"));
    }

    #[test]
    fn known_open_channel_is_bridgeable() {
        let mut p = PolicyMap::new();
        p.apply_metadata(&meta("chan-1", "ops", false));
        assert!(p.is_bridgeable("chan-1"));
        assert_eq!(p.channel_name("chan-1"), Some("ops"));
    }

    #[test]
    fn no_bridge_channel_is_not_bridgeable() {
        let mut p = PolicyMap::new();
        p.apply_metadata(&meta("chan-2", "secrets", true));
        assert!(!p.is_bridgeable("chan-2"));
    }

    #[test]
    fn policy_flip_to_no_bridge_is_honoured() {
        let mut p = PolicyMap::new();
        p.apply_metadata(&meta("chan-3", "ops", false));
        assert!(p.is_bridgeable("chan-3"));
        p.apply_metadata(&meta("chan-3", "ops", true));
        assert!(!p.is_bridgeable("chan-3"));
    }

    #[test]
    fn non_metadata_kinds_are_ignored() {
        let mut p = PolicyMap::new();
        let mut e = meta("chan-4", "ops", false);
        e.kind = 9;
        p.apply_metadata(&e);
        assert_eq!(p.len(), 0);
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --features bridge hive_bridge::policy`
Expected: FAIL — `cannot find type 'PolicyMap'`

- [ ] **Step 3: Implement the policy map**

Prepend to `src/hive_bridge/policy.rs`:

```rust
//! Per-channel bridge policy, built from relay-signed kind-39000 group
//! metadata.
//!
//! Fail-closed by construction: `is_bridgeable` answers `false` for any
//! channel not present in the map. buzz stores 39000 channel-scoped, so live
//! subscriptions do not receive it via fan-out — the daemon refreshes this map
//! by periodic historical REQ. A channel the daemon has never resolved must
//! never be exported on the assumption that it is probably fine.

use crate::nostr::Event;
use std::collections::HashMap;

struct ChannelPolicy {
    name: Option<String>,
    no_bridge: bool,
}

/// Channel id → policy. Rebuilt/refreshed from kind-39000 events.
#[derive(Default)]
pub struct PolicyMap {
    channels: HashMap<String, ChannelPolicy>,
}

impl PolicyMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold a kind-39000 group-metadata event into the map. Non-39000 events
    /// are ignored.
    pub fn apply_metadata(&mut self, event: &Event) {
        if event.kind != 39000 {
            return;
        }
        let Some(channel_id) = event
            .tags
            .iter()
            .find(|t| t.first().map(String::as_str) == Some("d"))
            .and_then(|t| t.get(1))
            .cloned()
        else {
            return;
        };
        let name = event
            .tags
            .iter()
            .find(|t| t.first().map(String::as_str) == Some("name"))
            .and_then(|t| t.get(1))
            .cloned();
        let no_bridge = event
            .tags
            .iter()
            .any(|t| t.first().map(String::as_str) == Some("no-bridge"));
        self.channels
            .insert(channel_id, ChannelPolicy { name, no_bridge });
    }

    /// True only for channels the map has resolved AND that are not flagged.
    pub fn is_bridgeable(&self, channel_id: &str) -> bool {
        self.channels
            .get(channel_id)
            .map(|c| !c.no_bridge)
            .unwrap_or(false)
    }

    pub fn channel_name(&self, channel_id: &str) -> Option<&str> {
        self.channels
            .get(channel_id)
            .and_then(|c| c.name.as_deref())
    }

    pub fn len(&self) -> usize {
        self.channels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --features bridge hive_bridge::policy`
Expected: PASS — 5 tests

- [ ] **Step 5: Commit**

```bash
git add src/hive_bridge/policy.rs
git commit -m "feat(hive-bridge): fail-closed per-channel bridge policy

Builds the channel policy map from relay-signed kind 39000 metadata.
An unresolved channel is never bridgeable, so a refresh failure cannot
silently widen what leaves the relay."
```

---

## Task 7: Agent roster and display names (kannaka-memory)

**Files:**
- Modify: `src/hive_bridge/roster.rs`

**Interfaces:**
- Produces:
  - `pub struct Roster` with `pub fn new() -> Self`, `pub fn apply(&mut self, event: &Event)`, `pub fn is_agent(&self, pubkey_hex: &str) -> bool`, `pub fn display_name(&self, pubkey_hex: &str) -> Option<&str>`, `pub fn agent_count(&self) -> usize`

- [ ] **Step 1: Write the failing test**

Replace `src/hive_bridge/roster.rs` with only this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::nostr::Event;

    fn ev(kind: u32, pubkey: &str, content: &str) -> Event {
        Event {
            id: "a".repeat(64),
            pubkey: pubkey.to_string(),
            created_at: 1_800_000_000,
            kind,
            tags: vec![],
            content: content.to_string(),
            sig: "c".repeat(128),
        }
    }

    #[test]
    fn agent_profile_marks_author_as_agent() {
        let mut r = Roster::new();
        r.apply(&ev(10100, &"a".repeat(64), r#"{"channel_add_policy":"any"}"#));
        assert!(r.is_agent(&"a".repeat(64)));
        assert_eq!(r.agent_count(), 1);
    }

    #[test]
    fn unknown_pubkey_is_not_an_agent() {
        let r = Roster::new();
        assert!(!r.is_agent(&"f".repeat(64)));
    }

    #[test]
    fn kind0_supplies_display_name() {
        let mut r = Roster::new();
        r.apply(&ev(0, &"b".repeat(64), r#"{"display_name":"Nick","name":"nf"}"#));
        assert_eq!(r.display_name(&"b".repeat(64)), Some("Nick"));
    }

    #[test]
    fn kind0_falls_back_to_name_field() {
        let mut r = Roster::new();
        r.apply(&ev(0, &"c".repeat(64), r#"{"name":"scribe"}"#));
        assert_eq!(r.display_name(&"c".repeat(64)), Some("scribe"));
    }

    #[test]
    fn kind0_does_not_confer_agent_status() {
        let mut r = Roster::new();
        r.apply(&ev(0, &"d".repeat(64), r#"{"name":"human"}"#));
        assert!(!r.is_agent(&"d".repeat(64)));
    }

    #[test]
    fn malformed_kind0_content_is_ignored() {
        let mut r = Roster::new();
        r.apply(&ev(0, &"e".repeat(64), "not json"));
        assert_eq!(r.display_name(&"e".repeat(64)), None);
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --features bridge hive_bridge::roster`
Expected: FAIL — `cannot find type 'Roster'`

- [ ] **Step 3: Implement the roster**

Prepend to `src/hive_bridge/roster.rs`:

```rust
//! Who is an agent, and what everyone is called.
//!
//! Agent identity comes from kind 10100 (`KIND_AGENT_PROFILE`) — "Agent
//! metadata + owner reference (replaceable, agent-authored)" — which is keyed
//! by the agent's own pubkey and is therefore a direct signal. Kind 30177 was
//! considered and rejected: it is owner-authored and keyed by
//! `(owner_pubkey, kind, d_tag)`, so it needs a second dereference to answer
//! the same question.
//!
//! Display names come from kind 0 for agents and humans alike.

use crate::nostr::Event;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct Roster {
    agents: HashSet<String>,
    names: HashMap<String, String>,
}

impl Roster {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fold a kind-10100 or kind-0 event into the roster. Other kinds are
    /// ignored.
    pub fn apply(&mut self, event: &Event) {
        match event.kind {
            10100 => {
                self.agents.insert(event.pubkey.clone());
            }
            0 => {
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&event.content) else {
                    return;
                };
                let name = v
                    .get("display_name")
                    .and_then(serde_json::Value::as_str)
                    .filter(|s| !s.is_empty())
                    .or_else(|| v.get("name").and_then(serde_json::Value::as_str))
                    .filter(|s| !s.is_empty());
                if let Some(name) = name {
                    self.names.insert(event.pubkey.clone(), name.to_string());
                }
            }
            _ => {}
        }
    }

    pub fn is_agent(&self, pubkey_hex: &str) -> bool {
        self.agents.contains(pubkey_hex)
    }

    pub fn display_name(&self, pubkey_hex: &str) -> Option<&str> {
        self.names.get(pubkey_hex).map(String::as_str)
    }

    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --features bridge hive_bridge::roster`
Expected: PASS — 6 tests

- [ ] **Step 5: Run the whole module's tests together**

Run: `cargo test --features bridge hive_bridge`
Expected: PASS — 18 tests across map, policy, roster

- [ ] **Step 6: Commit**

```bash
git add src/hive_bridge/roster.rs
git commit -m "feat(hive-bridge): agent roster from kind 10100, names from kind 0

Agent identity comes from the agent-authored 10100 profile, keyed by the
agent's own pubkey — a direct signal, unlike the owner-authored 30177
managed-agent definition."
```

---

## Task 8: The bridge daemon (kannaka-memory)

Network plumbing only. All decisions live in Tasks 5–7.

**Files:**
- Create: `src/bin/kannaka_hive_bridge.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: `hive_bridge::{map_event, MapContext, PolicyMap, Roster}`, `nostr::{Event, Keypair}`, `nostr::bridge::{Dedup, RateLimiter}`

- [ ] **Step 1: Register the binary**

In `Cargo.toml`, after the `kannaka-nostr-bridge` block (~line 126):

```toml
[[bin]]
name = "kannaka-hive-bridge"
path = "src/bin/kannaka_hive_bridge.rs"
required-features = ["bridge"]
```

- [ ] **Step 2: Write the config loader and NIP-42 AUTH helper**

Create `src/bin/kannaka_hive_bridge.rs`:

```rust
//! kannaka-hive-bridge — mirrors Hive (kannaka-buzz) room traffic onto NATS.
//!
//! Connects to the buzz relay, authenticates with NIP-42, subscribes to room
//! messages, the agent job lifecycle, and agent profiles, gates every event
//! through the per-channel bridge policy, and republishes onto
//! `KANNAKA.events.hive.*`. All decisions live in
//! `kannaka_memory::hive_bridge` (unit-tested); this binary is plumbing.
//!
//! Config (env):
//!   HIVE_RELAY_URL            wss:// url of the buzz relay
//!   HIVE_KEY_FILE             json {privkey,pubkey}, 0600 — an allowlisted member
//!   HIVE_DEDUPE_FILE          crash-durable processed-id log
//!   HIVE_NATS_URL/_USER/_PASS route target
//!   HIVE_SUBJECT_PREFIX       default KANNAKA.events.hive
//!   HIVE_POLICY_REFRESH_SECS  default 60
//!   HIVE_RATE_CAP/_REFILL     per-author token bucket (default 20 / 1.0)

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kannaka_memory::hive_bridge::{map_event, MapContext, PolicyMap, Roster};
use kannaka_memory::nostr::bridge::{Dedup, RateLimiter};
use kannaka_memory::nostr::{Event, Keypair};
use tungstenite::Message;

struct Config {
    relay_url: String,
    privkey: String,
    dedupe_file: String,
    nats_url: String,
    nats_user: String,
    nats_pass: String,
    subject_prefix: String,
    policy_refresh_secs: u64,
    rate_cap: f64,
    rate_refill: f64,
}

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|s| !s.is_empty())
}

fn load_config() -> Config {
    let key_file = env("HIVE_KEY_FILE").expect("HIVE_KEY_FILE required");
    let key_json = std::fs::read_to_string(&key_file).expect("read hive key file");
    let key: serde_json::Value = serde_json::from_str(&key_json).expect("hive key json");
    Config {
        relay_url: env("HIVE_RELAY_URL").expect("HIVE_RELAY_URL required"),
        privkey: key["privkey"].as_str().expect("privkey").to_string(),
        dedupe_file: env("HIVE_DEDUPE_FILE")
            .unwrap_or_else(|| "/var/lib/kannaka-hive-bridge/dedupe.log".into()),
        nats_url: env("HIVE_NATS_URL").unwrap_or_else(|| "nats://127.0.0.1:4222".into()),
        nats_user: env("HIVE_NATS_USER").unwrap_or_default(),
        nats_pass: env("HIVE_NATS_PASS").unwrap_or_default(),
        subject_prefix: env("HIVE_SUBJECT_PREFIX")
            .unwrap_or_else(|| "KANNAKA.events.hive".into()),
        policy_refresh_secs: env("HIVE_POLICY_REFRESH_SECS")
            .and_then(|s| s.parse().ok())
            .unwrap_or(60),
        rate_cap: env("HIVE_RATE_CAP").and_then(|s| s.parse().ok()).unwrap_or(20.0),
        rate_refill: env("HIVE_RATE_REFILL").and_then(|s| s.parse().ok()).unwrap_or(1.0),
    }
}

fn now_secs() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}

/// NIP-42: sign a kind-22242 event over the relay's challenge.
fn build_auth_event(kp: &Keypair, relay_url: &str, challenge: &str) -> Event {
    kp.sign_event(
        22242,
        vec![
            vec!["relay".to_string(), relay_url.to_string()],
            vec!["challenge".to_string(), challenge.to_string()],
        ],
        "",
        now_secs(),
    )
}
```

- [ ] **Step 3: Verify it compiles so far**

Run: `cargo check --features bridge --bin kannaka-hive-bridge`
Expected: errors only about the missing `main` function

- [ ] **Step 4: Add the NATS publish helper**

Append to the same file — this mirrors `kannaka_nostr_bridge.rs`'s short-lived-connection publish:

```rust
fn nats_hostport(url: &str) -> (String, u16) {
    let s = url.strip_prefix("nats://").unwrap_or(url);
    let mut it = s.splitn(2, ':');
    let host = it.next().unwrap_or("127.0.0.1").to_string();
    let port = it.next().and_then(|p| p.parse().ok()).unwrap_or(4222);
    (host, port)
}

/// Fire-and-forget NATS publish over a short-lived connection, matching the
/// DM bridge's approach. Hive volume is low enough that per-message connect
/// keeps the daemon stateless; revisit if throughput demands it.
fn nats_publish(cfg: &Config, subject: &str, payload: &str) -> std::io::Result<()> {
    let (host, port) = nats_hostport(&cfg.nats_url);
    let mut sock = TcpStream::connect((host.as_str(), port))?;
    sock.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut buf = [0u8; 2048];
    let _ = sock.read(&mut buf)?; // INFO line
    let connect = if !cfg.nats_user.is_empty() {
        format!(
            "CONNECT {{\"verbose\":false,\"pedantic\":false,\"name\":\"kannaka-hive-bridge\",\"user\":\"{}\",\"pass\":\"{}\"}}\r\n",
            cfg.nats_user.replace('"', "\\\""),
            cfg.nats_pass.replace('"', "\\\"")
        )
    } else {
        "CONNECT {\"verbose\":false,\"pedantic\":false,\"name\":\"kannaka-hive-bridge\"}\r\n".into()
    };
    sock.write_all(connect.as_bytes())?;
    sock.write_all(
        format!("PUB {} {}\r\n{}\r\n", subject, payload.len(), payload).as_bytes(),
    )?;
    sock.write_all(b"PING\r\n")?;
    let _ = sock.read(&mut buf)?;
    Ok(())
}
```

- [ ] **Step 5: Add the main loop**

Append:

```rust
fn send_req(ws: &mut tungstenite::WebSocket<impl Read + Write>, sub: &str, filter: &str) {
    let _ = ws.send(Message::Text(format!(r#"["REQ","{sub}",{filter}]"#)));
}

fn main() {
    let cfg = load_config();
    let kp = Keypair::from_secret_hex(&cfg.privkey).expect("valid hive privkey");
    let mut dedup = Dedup::open(&cfg.dedupe_file, 100_000).expect("open dedupe log");
    let mut limiter = RateLimiter::new(cfg.rate_cap, cfg.rate_refill);
    let mut policy = PolicyMap::new();
    let mut roster = Roster::new();

    let (mut ws, _) = tungstenite::connect(&cfg.relay_url).expect("connect to hive relay");
    eprintln!("[hive-bridge] connected to {}", cfg.relay_url);

    let mut authed = false;
    let mut subscribed = false;
    let mut last_policy_refresh = now_secs();

    loop {
        // Periodic policy refresh. buzz stores kind 39000 channel-scoped, so
        // live subscriptions never receive it via fan-out — a flag set after
        // startup is only ever seen by re-querying history.
        if authed && now_secs() - last_policy_refresh >= cfg.policy_refresh_secs as i64 {
            send_req(&mut ws, "policy", r#"{"kinds":[39000]}"#);
            last_policy_refresh = now_secs();
        }

        let msg = match ws.read() {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[hive-bridge] socket error: {e}");
                break;
            }
        };
        let Message::Text(text) = msg else { continue };
        let Ok(frame) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        let Some(verb) = frame.get(0).and_then(|v| v.as_str()) else { continue };

        match verb {
            "AUTH" => {
                let Some(challenge) = frame.get(1).and_then(|v| v.as_str()) else { continue };
                let auth = build_auth_event(&kp, &cfg.relay_url, challenge);
                let payload = serde_json::to_string(&auth).expect("serialize auth event");
                let _ = ws.send(Message::Text(format!(r#"["AUTH",{payload}]"#)));
                authed = true;
                // Resolve policy and roster BEFORE opening the content
                // subscription: the roster is built from the same stream it
                // filters, so subscribing first would mislabel the first
                // messages of an agent not yet learned.
                send_req(&mut ws, "policy", r#"{"kinds":[39000]}"#);
                send_req(&mut ws, "roster", r#"{"kinds":[0,10100]}"#);
            }
            "EOSE" => {
                let sub = frame.get(1).and_then(|v| v.as_str()).unwrap_or("");
                if sub == "roster" && !subscribed {
                    send_req(
                        &mut ws,
                        "content",
                        r#"{"kinds":[0,9,40002,10100,43001,43002,43003,43004,43005,43006]}"#,
                    );
                    subscribed = true;
                    eprintln!(
                        "[hive-bridge] live: {} channels, {} agents",
                        policy.len(),
                        roster.agent_count()
                    );
                }
            }
            "EVENT" => {
                let Some(raw) = frame.get(2) else { continue };
                let Ok(event) = serde_json::from_value::<Event>(raw.clone()) else { continue };
                if event.verify().is_err() {
                    continue;
                }
                policy.apply_metadata(&event);
                roster.apply(&event);
                if dedup.contains(&event.id) {
                    continue;
                }
                if !limiter.allow(&event.pubkey, now_secs()) {
                    continue;
                }

                let ctx = MapContext {
                    channel_name: None,
                    author_name: roster.display_name(&event.pubkey),
                    is_agent: roster.is_agent(&event.pubkey),
                    now_ms: now_ms(),
                };
                let Some(mapped) = map_event(&event, &ctx) else { continue };

                // Policy gate: everything except the agent roster is
                // channel-scoped and must clear its channel's policy.
                if mapped.subject != "agent" {
                    let channel_id = mapped.payload["channel_id"].as_str().unwrap_or("");
                    if !policy.is_bridgeable(channel_id) {
                        continue;
                    }
                }

                // Re-map with the resolved channel name now that policy has
                // confirmed the channel is known.
                let channel_id = mapped.payload["channel_id"].as_str().unwrap_or("");
                let ctx = MapContext {
                    channel_name: policy.channel_name(channel_id),
                    author_name: roster.display_name(&event.pubkey),
                    is_agent: roster.is_agent(&event.pubkey),
                    now_ms: now_ms(),
                };
                let Some(mapped) = map_event(&event, &ctx) else { continue };

                let subject = format!("{}.{}", cfg.subject_prefix, mapped.subject);
                let payload = mapped.payload.to_string();
                if let Err(e) = nats_publish(&cfg, &subject, &payload) {
                    eprintln!("[hive-bridge] publish failed: {e}");
                    continue;
                }
                let _ = dedup.record(&event.id);
            }
            _ => {}
        }
    }
}
```

- [ ] **Step 6: Verify it builds**

Run: `cargo build --features bridge --bin kannaka-hive-bridge`
Expected: success

- [ ] **Step 7: Verify clippy is clean**

Run: `cargo clippy --features bridge --bin kannaka-hive-bridge -- -D warnings`
Expected: no warnings. Fix any that appear.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml src/bin/kannaka_hive_bridge.rs
git commit -m "feat(hive-bridge): kannaka-hive-bridge daemon

NIP-42 authenticated WebSocket to the buzz relay, NIP-29 REQ over room
messages, the agent job lifecycle, and agent profiles, gated by the
fail-closed channel policy and republished onto KANNAKA.events.hive.*.

Policy and roster resolve before the content subscription opens, and the
policy map is refreshed by periodic historical REQ because buzz stores
kind 39000 channel-scoped and never fans it out live."
```

---

## Task 9: Subjects and payload types (nats app)

**Files:**
- Modify: `src/lib/nats/schema.ts`

**Interfaces:**
- Produces: `HiveMsgEvent`, `HiveJobEvent`, `HiveAgentEvent`; three new entries in `SUBJECTS`

- [ ] **Step 1: Add the subjects**

In `src/lib/nats/schema.ts`, inside the `SUBJECTS` array, after the two `KANNAKA.events.nostr.*` entries:

```ts
  // Hive room traffic (ADR-0045) — bridged from kannaka-buzz by
  // kannaka-hive-bridge. Agent + human messages, the agent job lifecycle,
  // and the agent roster.
  "KANNAKA.events.hive.msg",
  "KANNAKA.events.hive.job",
  "KANNAKA.events.hive.agent",
```

- [ ] **Step 2: Add the payload interfaces**

At the end of the same file, after `NostrReplyEvent`:

```ts
// Hive room traffic (ADR-0045).
export interface HiveMsgEvent {
  type: "hive_msg";
  event_id: string;        // source nostr event id — provenance
  channel_id: string;      // h tag (uuid)
  channel_name?: string;   // resolved from kind 39000
  author_hex: string;
  author_npub: string;
  author_name?: string;    // kind-0 profile; agents and humans alike
  is_agent: boolean;       // author published a kind-10100 agent profile
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

export interface HiveAgentEvent {
  type: "hive_agent";      // from kind 10100, agent-authored
  event_id: string;
  agent_hex: string;       // the event pubkey — the agent's own key
  agent_npub: string;
  name?: string;
  owner_hex?: string;
  ts: number;
}
```

- [ ] **Step 3: Verify types compile**

Run: `npx tsc --noEmit`
Expected: no errors

- [ ] **Step 4: Commit**

```bash
git add src/lib/nats/schema.ts
git commit -m "feat(hive): add KANNAKA.events.hive.* subjects and payload types"
```

---

## Task 10: The room fold (nats app)

**Files:**
- Create: `src/lib/nats/hive.ts`
- Create: `scripts/test-hive-fold.mjs`

**Interfaces:**
- Consumes: `HiveMsgEvent`, `HiveJobEvent`, `HiveAgentEvent` from Task 9
- Produces:
  - `export interface HiveJob { job_id: string | null; phases: HiveJobEvent[]; lastTs: number; done: boolean }`
  - `export interface HiveRow { kind: "msg" | "job"; ts: number; msg?: HiveMsgEvent; job?: HiveJob }`
  - `export interface HiveRoom { channel_id: string; channel_name?: string; rows: HiveRow[]; orphanJobs: HiveJobEvent[]; lastTs: number }`
  - `export function foldHive(msgs: HiveMsgEvent[], jobs: HiveJobEvent[]): HiveRoom[]`

- [ ] **Step 1: Write the failing test**

Create `scripts/test-hive-fold.mjs`:

```js
// Fixture test for the /nostr Hive fold. The app has no test runner; this
// follows the scripts/test-agent-mcp.mjs precedent and relies on Node's
// native TypeScript type stripping to import the .ts source directly.
import assert from "node:assert/strict";
import { foldHive } from "../src/lib/nats/hive.ts";

const msg = (over = {}) => ({
  type: "hive_msg", event_id: "m1", channel_id: "c1", channel_name: "ops",
  author_hex: "a1", author_npub: "npub1a", author_name: "scribe",
  is_agent: true, content: "hello", created_at: 1800000000, ts: 1000, ...over,
});
const job = (over = {}) => ({
  type: "hive_job", event_id: "j1", channel_id: "c1", channel_name: "ops",
  phase: "request", kind: 43001, job_id: "j1", author_hex: "a1",
  author_npub: "npub1a", content: "build", created_at: 1800000000, ts: 1100, ...over,
});

// One room, one message
let rooms = foldHive([msg()], []);
assert.equal(rooms.length, 1);
assert.equal(rooms[0].channel_id, "c1");
assert.equal(rooms[0].channel_name, "ops");
assert.equal(rooms[0].rows.length, 1);

// Job phases collapse into one card
rooms = foldHive([], [
  job(),
  job({ event_id: "j2", phase: "accepted", kind: 43002, ts: 1200 }),
  job({ event_id: "j3", phase: "result", kind: 43004, ts: 1300 }),
]);
assert.equal(rooms[0].rows.length, 1, "three phases collapse to one card");
assert.equal(rooms[0].rows[0].job.phases.length, 3);
assert.equal(rooms[0].rows[0].job.done, true, "result marks the job done");

// An in-flight job is not done
rooms = foldHive([], [job(), job({ event_id: "j2", phase: "progress", kind: 43003, ts: 1200 })]);
assert.equal(rooms[0].rows[0].job.done, false);

// An error also closes the job
rooms = foldHive([], [job(), job({ event_id: "j2", phase: "error", kind: 43006, ts: 1200 })]);
assert.equal(rooms[0].rows[0].job.done, true);

// Orphans are separated, not dropped
rooms = foldHive([], [job({ job_id: null, phase: "result", kind: 43004 })]);
assert.equal(rooms[0].rows.length, 0);
assert.equal(rooms[0].orphanJobs.length, 1, "orphans surface, never dropped");

// Rooms sort by most recent activity
rooms = foldHive([msg({ channel_id: "c1", ts: 500 }), msg({ event_id: "m2", channel_id: "c2", ts: 9000 })], []);
assert.equal(rooms[0].channel_id, "c2", "most recent room first");

// Rows within a room sort newest first
rooms = foldHive([msg({ ts: 100 }), msg({ event_id: "m2", ts: 900 })], []);
assert.equal(rooms[0].rows[0].msg.event_id, "m2");

// Duplicate event ids collapse
rooms = foldHive([msg(), msg()], []);
assert.equal(rooms[0].rows.length, 1, "same event_id must not double-render");

console.log("hive fold: all assertions passed");
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node scripts/test-hive-fold.mjs`
Expected: FAIL — `Cannot find module '../src/lib/nats/hive.ts'`

- [ ] **Step 3: Implement the fold**

Create `src/lib/nats/hive.ts`:

```ts
// Pure fold from flat Hive events to rooms. No React, no NATS — kept out of
// the route component so it is testable and so nostr.tsx stays readable.
//
// Erasable-only TypeScript: type annotations and interfaces only. The fixture
// script runs this file directly under Node's type stripping, which rejects
// enums, parameter properties, and namespaces.
import type { HiveJobEvent, HiveMsgEvent } from "./schema";

export interface HiveJob {
  job_id: string | null;
  phases: HiveJobEvent[];
  lastTs: number;
  done: boolean;
}

export interface HiveRow {
  kind: "msg" | "job";
  ts: number;
  msg?: HiveMsgEvent;
  job?: HiveJob;
}

export interface HiveRoom {
  channel_id: string;
  channel_name?: string;
  rows: HiveRow[];
  orphanJobs: HiveJobEvent[];
  lastTs: number;
}

const TERMINAL_PHASES = new Set(["result", "error", "cancel"]);

export function foldHive(msgs: HiveMsgEvent[], jobs: HiveJobEvent[]): HiveRoom[] {
  const rooms = new Map<string, HiveRoom>();
  const seen = new Set<string>();

  const room = (channel_id: string, channel_name?: string): HiveRoom => {
    let r = rooms.get(channel_id);
    if (!r) {
      r = { channel_id, channel_name, rows: [], orphanJobs: [], lastTs: 0 };
      rooms.set(channel_id, r);
    }
    if (!r.channel_name && channel_name) r.channel_name = channel_name;
    return r;
  };

  for (const m of msgs) {
    if (!m?.event_id || seen.has(m.event_id)) continue;
    seen.add(m.event_id);
    const r = room(m.channel_id, m.channel_name);
    r.rows.push({ kind: "msg", ts: m.ts, msg: m });
    r.lastTs = Math.max(r.lastTs, m.ts);
  }

  // Job phases collapse into one card per job_id. A job with no resolvable
  // id is an orphan: surfaced separately rather than dropped, so a relay
  // defect reads as a defect instead of an empty panel.
  const byJob = new Map<string, HiveJob>();
  for (const j of jobs) {
    if (!j?.event_id || seen.has(j.event_id)) continue;
    seen.add(j.event_id);
    const r = room(j.channel_id, j.channel_name);
    r.lastTs = Math.max(r.lastTs, j.ts);
    if (!j.job_id) {
      r.orphanJobs.push(j);
      continue;
    }
    const key = `${j.channel_id}:${j.job_id}`;
    let job = byJob.get(key);
    if (!job) {
      job = { job_id: j.job_id, phases: [], lastTs: 0, done: false };
      byJob.set(key, job);
      r.rows.push({ kind: "job", ts: j.ts, job });
    }
    job.phases.push(j);
    job.lastTs = Math.max(job.lastTs, j.ts);
    if (TERMINAL_PHASES.has(j.phase)) job.done = true;
  }

  for (const job of byJob.values()) {
    job.phases.sort((a, b) => a.ts - b.ts);
  }

  const out = [...rooms.values()];
  for (const r of out) {
    for (const row of r.rows) {
      if (row.kind === "job" && row.job) row.ts = row.job.lastTs;
    }
    r.rows.sort((a, b) => b.ts - a.ts);
    r.orphanJobs.sort((a, b) => b.ts - a.ts);
  }
  out.sort((a, b) => b.lastTs - a.lastTs);
  return out;
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node scripts/test-hive-fold.mjs`
Expected: `hive fold: all assertions passed`

- [ ] **Step 5: Verify types compile**

Run: `npx tsc --noEmit`
Expected: no errors

- [ ] **Step 6: Commit**

```bash
git add src/lib/nats/hive.ts scripts/test-hive-fold.mjs
git commit -m "feat(hive): pure room fold with job-phase collapsing

Groups Hive events by channel, collapses 43001-43006 phases into one
card per job, and surfaces uncorrelated job events as orphans rather
than dropping them."
```

---

## Task 11: History backfill (nats app)

**Files:**
- Create: `src/lib/nats/hive-history.functions.ts`

**Interfaces:**
- Produces: `getHiveHistory` server fn returning `HiveHistoryItem[]`, where `HiveHistoryItem = { kind: "msg"; ts: number; ev: HiveMsgEvent } | { kind: "job"; ts: number; ev: HiveJobEvent }`

- [ ] **Step 1: Write the server function**

Create `src/lib/nats/hive-history.functions.ts`, mirroring `nostr-history.functions.ts` exactly in structure:

```ts
import { createServerFn } from "@tanstack/react-start";
import { requireSupabaseAuth } from "@/integrations/supabase/auth-middleware";
import {
  AckPolicy,
  DeliverPolicy,
  StringCodec,
  connect,
  type JetStreamManager,
  type NatsConnection,
} from "nats.ws";
import type { HiveJobEvent, HiveMsgEvent } from "./schema";

const STREAM = "KANNAKA_HIVE";
const SUBJECT = "KANNAKA.events.hive.>";
const LOOKBACK_MS = 30 * 24 * 60 * 60 * 1000; // last 30 days
const MAX = 400;
const sc = StringCodec();

export type HiveHistoryItem =
  | { kind: "msg"; ts: number; ev: HiveMsgEvent }
  | { kind: "job"; ts: number; ev: HiveJobEvent };

function parseObj(raw: string): Record<string, unknown> | null {
  try {
    const v = JSON.parse(raw);
    return v && typeof v === "object" ? (v as Record<string, unknown>) : null;
  } catch {
    return null;
  }
}

/**
 * Backfill recent Hive room traffic from the KANNAKA_HIVE JetStream stream
 * (ADR-0045). Operator-gated — bridged rooms carry members' messages. Returns
 * the last ~30 days of msg/job events (up to 400) so the /nostr HIVE panel has
 * scrollback before live traffic takes over. Empty if the stream or creds are
 * absent — the live view still works.
 */
export const getHiveHistory = createServerFn({ method: "GET" })
  .middleware([requireSupabaseAuth])
  .handler(async (): Promise<HiveHistoryItem[]> => {
    const url = process.env.NATS_WS_URL;
    const pass = process.env.NATS_INTERNAL_PASSWORD;
    if (!url || !pass) return [];

    let nc: NatsConnection | null = null;
    let jsm: JetStreamManager | null = null;
    let consumerName: string | null = null;
    const items: HiveHistoryItem[] = [];

    try {
      nc = await connect({
        servers: url,
        user: "kannaka_internal",
        pass,
        timeout: 3000,
        reconnect: false,
        pingInterval: 60_000,
      });
      jsm = await nc.jetstreamManager();
      const js = nc.jetstream();

      // Stream may not exist yet — return empty so the live view still renders.
      try {
        await jsm.streams.info(STREAM);
      } catch {
        return [];
      }

      consumerName = `hive_hist_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
      await jsm.consumers.add(STREAM, {
        durable_name: consumerName,
        ack_policy: AckPolicy.None,
        deliver_policy: DeliverPolicy.StartTime,
        opt_start_time: new Date(Date.now() - LOOKBACK_MS).toISOString(),
        filter_subject: SUBJECT,
        inactive_threshold: 30_000_000_000, // 30s in ns
      });

      const consumer = await js.consumers.get(STREAM, consumerName);
      const deadline = Date.now() + 5000;

      while (items.length < MAX && Date.now() < deadline) {
        const iter = await consumer.fetch({
          max_messages: Math.min(50, MAX - items.length),
          expires: Math.max(300, Math.min(2000, deadline - Date.now())),
        });
        let gotAny = false;
        for await (const m of iter) {
          gotAny = true;
          const tsNs = m.info?.timestampNanos;
          const ts = tsNs ? Number(BigInt(tsNs) / 1_000_000n) : Date.now();
          const obj = parseObj(sc.decode(m.data));
          if (obj) {
            if (m.subject.endsWith(".job")) {
              items.push({ kind: "job", ts, ev: obj as unknown as HiveJobEvent });
            } else if (m.subject.endsWith(".msg")) {
              items.push({ kind: "msg", ts, ev: obj as unknown as HiveMsgEvent });
            }
          }
          if (items.length >= MAX) break;
        }
        if (!gotAny) break;
      }
    } catch {
      // best-effort backfill; live view is the source of truth
    } finally {
      try { if (jsm && consumerName) await jsm.consumers.delete(STREAM, consumerName); } catch { /* noop */ }
      try { await nc?.drain(); } catch { /* noop */ }
      try { await nc?.close(); } catch { /* noop */ }
    }

    return items;
  });
```

- [ ] **Step 2: Verify types compile**

Run: `npx tsc --noEmit`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add src/lib/nats/hive-history.functions.ts
git commit -m "feat(hive): 30-day JetStream backfill for the HIVE panel"
```

---

## Task 12: The `/nostr` panels (nats app)

**Files:**
- Modify: `src/routes/nostr.tsx`

**Interfaces:**
- Consumes: `foldHive`, `HiveRoom` (Task 10); `getHiveHistory` (Task 11); `HiveMsgEvent`, `HiveJobEvent`, `HiveAgentEvent` (Task 9)

- [ ] **Step 1: Add imports and subscriptions**

In `src/routes/nostr.tsx`, extend the existing imports:

```tsx
import type { HiveAgentEvent, HiveJobEvent, HiveMsgEvent, NostrDmEvent, NostrReplyEvent } from "@/lib/nats/schema";
import { foldHive, type HiveRoom } from "@/lib/nats/hive";
import { getHiveHistory, type HiveHistoryItem } from "@/lib/nats/hive-history.functions";
```

Inside `function Nostr()`, after the existing `dms` / `replies` subscriptions:

```tsx
  const hiveMsgs = useSubject<HiveMsgEvent>("KANNAKA.events.hive.msg", 300);
  const hiveJobs = useSubject<HiveJobEvent>("KANNAKA.events.hive.job", 200);
  const hiveAgents = useSubject<HiveAgentEvent>("KANNAKA.events.hive.agent", 100);
  const fetchHiveHistory = useServerFn(getHiveHistory);
  const [hiveHistory, setHiveHistory] = useState<HiveHistoryItem[]>([]);
```

- [ ] **Step 2: Load Hive history on mount**

After the existing history `useEffect`:

```tsx
  useEffect(() => {
    let cancelled = false;
    fetchHiveHistory()
      .then((items) => { if (!cancelled) setHiveHistory(items ?? []); })
      .catch(() => { /* live view still works without backfill */ });
    return () => { cancelled = true; };
  }, [fetchHiveHistory]);
```

- [ ] **Step 3: Build the rooms**

After the existing `exchanges` memo:

```tsx
  const rooms: HiveRoom[] = useMemo(() => {
    const msgs: HiveMsgEvent[] = [];
    const jobs: HiveJobEvent[] = [];
    // History first (older), then live — foldHive dedupes by event_id.
    for (const h of hiveHistory) {
      if (h.kind === "msg") msgs.push(h.ev);
      else jobs.push(h.ev);
    }
    for (const m of hiveMsgs) if (m.payload) msgs.push(m.payload);
    for (const j of hiveJobs) if (j.payload) jobs.push(j.payload);
    return foldHive(msgs, jobs);
  }, [hiveMsgs, hiveJobs, hiveHistory]);

  const agentCount = new Set(
    hiveAgents.map((a) => a.payload?.agent_hex).filter(Boolean),
  ).size;
  const allJobs = rooms.flatMap((r) => r.rows.filter((x) => x.kind === "job"));
  const jobsDone = allJobs.filter((x) => x.job?.done).length;
```

- [ ] **Step 4: Add the HIVE rail panel**

In the left rail column, after the closing `</Panel>` of `MEMBRANE`:

```tsx
        <Panel title="HIVE" right={<Tag tone="signal">ADR-0045</Tag>}>
          <div className="grid gap-1.5">
            <KV k="rooms" v={rooms.length} accent="signal" />
            <KV k="agents" v={agentCount} accent="hemi-r" />
            <KV k="jobs" v={`${allJobs.length - jobsDone} ▸ / ${jobsDone} ✓`} />
          </div>
          <p className="mt-3 text-[10px] leading-relaxed text-muted-foreground">
            Bridged from the Hive relay. Channels flagged no-bridge never
            appear here. Room messages include members' text; operator eyes only.
          </p>
        </Panel>
```

- [ ] **Step 5: Add the HIVE ROOMS panel**

After the closing `</Panel>` of `CONVERSATIONS`, inside the same right-hand column — wrap both panels in a `<div className="grid gap-4 content-start">` if they are not already siblings in one:

```tsx
      <Panel
        title="HIVE ROOMS"
        right={<Tag tone="muted">{`${rooms.length} rooms`}</Tag>}
      >
        {rooms.length === 0 ? (
          <div className="py-10 text-center text-xs text-muted-foreground">
            <p>No Hive room traffic yet.</p>
            <p className="mt-1">
              Agent job kinds are newly writable and may have no producers —
              an empty panel with a live connection is expected, not an error.
            </p>
          </div>
        ) : (
          <div className="grid gap-4">
            {rooms.map((room) => (
              <div key={room.channel_id} className="grid gap-2">
                <div className="flex items-center justify-between gap-2">
                  <span className="font-mono text-[11px] text-signal">
                    #{room.channel_name ?? room.channel_id.slice(0, 8)}
                  </span>
                  <span className="text-[10px] text-muted-foreground">{ago(room.lastTs)}</span>
                </div>

                {room.rows.map((row) =>
                  row.kind === "msg" && row.msg ? (
                    <div key={row.msg.event_id} className="border-l-2 border-border/60 pl-2.5">
                      <div className="flex items-center gap-1.5">
                        <span
                          className={`font-mono text-[10px] ${row.msg.is_agent ? "text-hemi-r" : "text-muted-foreground"}`}
                        >
                          {row.msg.author_name ?? row.msg.author_npub.slice(0, 12)}
                        </span>
                        {row.msg.is_agent && <Tag tone="signal">agent</Tag>}
                      </div>
                      <div className="mt-1 text-xs leading-relaxed">{row.msg.content}</div>
                    </div>
                  ) : row.job ? (
                    <div key={row.job.job_id ?? row.ts} className="border border-border/60 p-2.5">
                      <div className="flex items-center justify-between gap-2">
                        <span className="font-mono text-[10px] text-muted-foreground">
                          job {row.job.job_id?.slice(0, 8) ?? "—"}
                        </span>
                        <Tag tone={row.job.done ? "muted" : "signal"}>
                          {row.job.done ? "closed" : "in flight"}
                        </Tag>
                      </div>
                      <div className="mt-1.5 flex flex-wrap gap-1">
                        {row.job.phases.map((p) => (
                          <Tag key={p.event_id} tone="muted">{p.phase}</Tag>
                        ))}
                      </div>
                    </div>
                  ) : null,
                )}

                {room.orphanJobs.length > 0 && (
                  <div className="mt-1 border-l-2 border-border/40 pl-2.5 opacity-50">
                    <div className="text-[9px] uppercase tracking-[0.2em] text-muted-foreground">
                      unlinked ({room.orphanJobs.length})
                    </div>
                    {room.orphanJobs.map((j) => (
                      <div key={j.event_id} className="text-[10px] text-muted-foreground">
                        {j.phase} · {j.event_id.slice(0, 8)}
                      </div>
                    ))}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </Panel>
```

- [ ] **Step 6: Verify types compile**

Run: `npx tsc --noEmit`
Expected: no errors

- [ ] **Step 7: Verify lint passes**

Run: `npm run lint`
Expected: no new errors

- [ ] **Step 8: Verify it renders**

Run: `npm run dev`, open `/nostr`, sign in as operator.
Expected: `MEMBRANE` and `CONVERSATIONS` unchanged; `HIVE` and `HIVE ROOMS` render; with no bridge running, `HIVE ROOMS` shows the explanatory empty state and the page does not error.

- [ ] **Step 9: Commit**

```bash
git add src/routes/nostr.tsx
git commit -m "feat(hive): HIVE rail stats and HIVE ROOMS panel on /nostr

Adds Hive room traffic beside the DM membrane. Each keeps its own
grouping: DMs fold by rumor id, rooms fold by channel with job phases
collapsed into cards. Uncorrelated job events render dimmed rather than
being dropped."
```

---

## Task 13: `hive_traffic` MCP tool (nats app)

**Files:**
- Create: `src/lib/mcp/tools/hive-traffic.ts`

**Interfaces:**
- Consumes: `foldHive` (Task 10), the schema types (Task 9)

- [ ] **Step 1: Write the tool**

Create `src/lib/mcp/tools/hive-traffic.ts`:

```ts
import { withAudit } from "@/lib/mcp/audit";
import { scopeDenied } from "@/lib/mcp/scope";
import { defineTool, type ToolContext } from "@lovable.dev/mcp-js";
import { z } from "zod";
import { connect, StringCodec, type NatsConnection, type Subscription } from "nats.ws";
import { foldHive } from "@/lib/nats/hive";
import type { HiveJobEvent, HiveMsgEvent } from "@/lib/nats/schema";

const sc = StringCodec();
const MSG_SUBJECT = "KANNAKA.events.hive.msg";
const JOB_SUBJECT = "KANNAKA.events.hive.job";

function tryParse(raw: string): unknown {
  try { return JSON.parse(raw); } catch { return null; }
}

export default defineTool({
  name: "hive_traffic",
  title: "Monitor Hive room traffic",
  description:
    "Sample live swarm agent traffic in the Kannaka Hive (ADR-0045) and return it grouped by room. Opens a short-lived NATS connection with the operator's kannaka_internal credentials, listens on KANNAKA.events.hive.msg (room messages from agents and humans) and KANNAKA.events.hive.job (the 43001-43006 agent job lifecycle), groups by channel, and collapses each job's phases into one record. Channels flagged no-bridge on the relay never reach this stream. Room messages include members' text — treat as sensitive. NATS core is ephemeral, so this captures only traffic during the listening window; call again to keep watching.",
  inputSchema: {
    durationMs: z
      .number().int().min(500).max(8000).default(4000)
      .describe("How long to listen, in milliseconds (500 - 8000)."),
    maxMessages: z
      .number().int().min(1).max(300).default(150)
      .describe("Return at most this many raw messages (1 - 300) across both subjects."),
  },
  annotations: { readOnlyHint: true, idempotentHint: false, openWorldHint: true },
  handler: withAudit(
    "hive_traffic",
    async (args: { durationMs: number; maxMessages: number }, ctx: ToolContext) => {
      if (!ctx.isAuthenticated()) {
        return { content: [{ type: "text", text: "Not authenticated" }], isError: true };
      }
      const denied = scopeDenied(ctx, "mcp:read");
      if (denied) return denied;

      const url = process.env.NATS_WS_URL;
      const pass = process.env.NATS_INTERNAL_PASSWORD;
      if (!url) {
        return { content: [{ type: "text", text: "NATS_WS_URL is not configured" }], isError: true };
      }

      let nc: NatsConnection | null = null;
      const subs: Subscription[] = [];
      const msgs: HiveMsgEvent[] = [];
      const jobs: HiveJobEvent[] = [];

      try {
        nc = await connect({
          servers: url,
          user: pass ? "kannaka_internal" : "anon",
          pass: pass || "anon",
          timeout: 3000,
          reconnect: false,
          pingInterval: 60_000,
        });

        let total = 0;
        const consume = async (sub: Subscription, kind: "msg" | "job") => {
          for await (const m of sub) {
            const obj = tryParse(sc.decode(m.data));
            if (obj && typeof obj === "object") {
              if (kind === "msg") msgs.push(obj as HiveMsgEvent);
              else jobs.push(obj as HiveJobEvent);
              total++;
            }
            if (total >= args.maxMessages) break;
          }
        };

        const msgSub = nc.subscribe(MSG_SUBJECT);
        const jobSub = nc.subscribe(JOB_SUBJECT);
        subs.push(msgSub, jobSub);

        await Promise.race([
          Promise.all([consume(msgSub, "msg"), consume(jobSub, "job")]),
          new Promise<void>((resolve) => setTimeout(resolve, args.durationMs)),
        ]);
      } catch (e) {
        return {
          content: [{ type: "text", text: `NATS error: ${e instanceof Error ? e.message : String(e)}` }],
          isError: true,
        };
      } finally {
        for (const s of subs) { try { s.unsubscribe(); } catch { /* noop */ } }
        try { await nc?.drain(); } catch { /* noop */ }
        try { await nc?.close(); } catch { /* noop */ }
      }

      const rooms = foldHive(msgs, jobs);
      const agents = new Set(msgs.filter((m) => m.is_agent).map((m) => m.author_hex)).size;
      const result = {
        window_ms: args.durationMs,
        msgCount: msgs.length,
        jobCount: jobs.length,
        roomCount: rooms.length,
        agentCount: agents,
        rooms,
      };
      const summary =
        msgs.length + jobs.length === 0
          ? "No Hive traffic during the listening window (NATS core is live-only; try again while a room is active)."
          : `${msgs.length} message(s), ${jobs.length} job event(s) across ${rooms.length} room(s) from ${agents} agent(s).`;
      return {
        content: [{ type: "text", text: `${summary}\n\n${JSON.stringify(result, null, 2)}` }],
        structuredContent: result,
      };
    },
  ),
});
```

- [ ] **Step 2: Register the tool**

Check how `nostr-traffic.ts` is registered:

Run: `grep -rn "nostr-traffic\|nostrTraffic" src/lib/mcp/`

Add `hive-traffic` to the same registry in the same style. If tools are auto-discovered by directory scan, no change is needed — confirm which by reading `src/lib/mcp/index.ts`.

- [ ] **Step 3: Verify types compile**

Run: `npx tsc --noEmit`
Expected: no errors

- [ ] **Step 4: Verify the tool is listed**

Run: `npm run dev`, then in another shell:

```bash
curl -s localhost:3000/.mcp/list-tools | grep hive_traffic
```

Expected: `hive_traffic` appears in the tool list

- [ ] **Step 5: Commit**

```bash
git add src/lib/mcp/tools/hive-traffic.ts src/lib/mcp/index.ts
git commit -m "feat(mcp): hive_traffic tool for sampling Hive room traffic"
```

---

## Task 14: Provisioning and deployment (kannaka-memory)

**Files:**
- Create: `ops/hive-bridge/README.md`
- Create: `ops/hive-bridge/kannaka-hive-bridge.service`

- [ ] **Step 1: Create the JetStream stream**

Mirrors how `KANNAKA_NOSTR` was created (nats PR #3):

```bash
nats stream add KANNAKA_HIVE \
  --subjects 'KANNAKA.events.hive.>' \
  --storage file --replicas 3 \
  --retention limits --discard old \
  --max-age 90d --dupe-window 2m \
  --defaults
```

- [ ] **Step 2: Verify the stream exists**

Run: `nats stream info KANNAKA_HIVE`
Expected: subjects `KANNAKA.events.hive.>`, R3, file storage, 90-day max age

- [ ] **Step 3: Write the systemd unit**

Create `ops/hive-bridge/kannaka-hive-bridge.service`:

```ini
[Unit]
Description=Kannaka Hive bridge (buzz relay -> NATS)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/kannaka-hive-bridge
Restart=always
RestartSec=5
EnvironmentFile=/etc/kannaka/hive-bridge.env
StateDirectory=kannaka-hive-bridge
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true

[Install]
WantedBy=multi-user.target
```

- [ ] **Step 4: Write the ops README**

Create `ops/hive-bridge/README.md`:

```markdown
# kannaka-hive-bridge

Mirrors Hive (kannaka-buzz) room traffic onto NATS as `KANNAKA.events.hive.*`.
See `docs/superpowers/specs/2026-07-26-hive-swarm-traffic-on-nostr-design.md`.

## Prerequisite: relay membership

The bridge's pubkey must be **allowlisted on the relay and a member of every
room to be mirrored**. buzz stores channel events channel-scoped and enforces
access control on read — the bridge sees nothing in rooms it has not joined.
This is the most common reason for an empty `/nostr` HIVE panel.

```sql
INSERT INTO pubkey_allowlist (pubkey) VALUES (decode('<bridge-pubkey-hex>', 'hex'));
```

## Config

`/etc/kannaka/hive-bridge.env`, mode 0600, never committed:

    HIVE_RELAY_URL=wss://<hive-relay-host>
    HIVE_KEY_FILE=/etc/kannaka/hive-bridge.key.json
    HIVE_DEDUPE_FILE=/var/lib/kannaka-hive-bridge/dedupe.log
    HIVE_NATS_URL=nats://127.0.0.1:4222
    HIVE_NATS_USER=kannaka_internal
    HIVE_NATS_PASS=<password>
    HIVE_SUBJECT_PREFIX=KANNAKA.events.hive
    HIVE_POLICY_REFRESH_SECS=60

The key file is `{"privkey":"<hex>","pubkey":"<hex>"}`, mode 0600 — the same
custody the ADR-0043 voice key gets.

## Channel opt-out

A channel owner or admin excludes a room by publishing a kind-9002 metadata
edit with a `["no-bridge","true"]` tag. The relay reflects it as a `no-bridge`
tag on kind 39000 and the bridge stops exporting that room within
`HIVE_POLICY_REFRESH_SECS`. Channels the bridge has never resolved are never
exported.

## Install

    cargo build --release --features bridge --bin kannaka-hive-bridge
    sudo install -m755 target/release/kannaka-hive-bridge /usr/local/bin/
    sudo install -m644 ops/hive-bridge/kannaka-hive-bridge.service /etc/systemd/system/
    sudo systemctl daemon-reload && sudo systemctl enable --now kannaka-hive-bridge
```

- [ ] **Step 5: End-to-end verification**

1. Start the bridge; confirm the log line `[hive-bridge] live: N channels, M agents` with N > 0.
2. Post a message in a bridged room as an agent. Confirm it appears on `/nostr` under `HIVE ROOMS` within ~1s.
3. Run a job chain (43001 → 43002 → 43004) and confirm it renders as one card marked `closed`.
4. Reload `/nostr` and confirm both survive via `getHiveHistory`.
5. Set `no-bridge` on that channel; confirm traffic stops within `HIVE_POLICY_REFRESH_SECS` and the room's existing rows remain (history is not retroactively purged).
6. Confirm a room the bridge is *not* a member of never appears.

- [ ] **Step 6: Commit**

```bash
git add ops/hive-bridge/
git commit -m "ops(hive-bridge): systemd unit, stream provisioning, and runbook"
```

---

## Self-Review

**Spec coverage:**

| Spec section | Task |
|---|---|
| Phase 1a — job correlation | 1, 2 |
| Phase 1b — channel bridge policy | 3, 4 |
| Phase 2 — bridge, startup ordering, policy poll, fail-closed | 5, 6, 7, 8 |
| Phase 3 — schema, fold, history, panels, orphans, empty state | 9, 10, 11, 12 |
| Phase 4 — `hive_traffic` MCP tool | 13 |
| Ops — stream, systemd, membership prerequisite | 14 |
| Verification — Rust units, TS fixture, end-to-end | 5, 6, 7 (units), 10 (fixture), 14 (e2e) |

No spec section is unimplemented.

**Type consistency checked:** `map_event`/`MapContext`/`Mapped` (Task 5) are consumed with matching signatures in Task 8. `PolicyMap::{new, apply_metadata, is_bridgeable, channel_name, len}` (Task 6) and `Roster::{new, apply, is_agent, display_name, agent_count}` (Task 7) match their call sites in Task 8. `foldHive` (Task 10) returns `HiveRoom[]` as consumed in Tasks 12 and 13. `HiveMsgEvent`/`HiveJobEvent`/`HiveAgentEvent` (Task 9) are used unchanged throughout. The Rust payload keys emitted in Task 5 match the TypeScript interfaces in Task 9 field-for-field.

**Known soft spots**, flagged rather than papered over:

- **Task 2, Step 6** requires confirming the real DB accessor name (`get_event` is assumed). This is the one place the plan asks the implementer to check a signature rather than asserting it — `buzz-db`'s single-event lookup was not read.
- **Task 4, Step 5** says "find where kind 9002 builds its `ChannelUpdate`" without a line number; the 9002 handler body was not read in detail. The surrounding authorization pattern (`name`/`about` are owner/admin) is documented in `NOSTR.md` and must be matched.
- **Task 13, Step 2** branches on how MCP tools are registered, which was not read.

Everything else in this plan was verified against source at `kannaka-buzz@00c5120` and `kannaka-memory@origin/master`.
