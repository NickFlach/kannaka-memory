//! Hive → NATS bridge logic (ADR-0045). The binary is plumbing; everything
//! testable lives here, mirroring how `nostr::bridge` relates to
//! `kannaka_nostr_bridge`.

pub mod map;
pub mod policy;
pub mod roster;

pub use map::{map_event, MapContext, Mapped};
pub use policy::PolicyMap;
pub use roster::Roster;

/// Decide whether an event crosses to NATS, and with what payload.
///
/// This composes the two halves that were previously only wired together
/// inside the daemon's event loop: `map_event` (pure shape) and the
/// `PolicyMap` gate (privacy). Extracted so the EXPORT DECISION itself can be
/// tested, not just its parts.
///
/// That distinction is the point of #636. `no-bridge` is the only privacy
/// control on the bridge — human messages otherwise cross to the bus and into
/// 90-day JetStream retention — and the unit tests covered `PolicyMap` and
/// `map_event` separately while nothing demonstrated that a flagged channel
/// actually stops producing output.
///
/// Returns `None` when the event must not be exported: unmappable, or
/// channel-scoped and its channel is not bridgeable (including a channel whose
/// policy has never been resolved — fail-closed).
///
/// The agent roster is exempt because it is not channel-scoped, so there is no
/// channel policy for it to clear.
pub fn export_decision(
    event: &crate::nostr::Event,
    roster: &roster::Roster,
    policy: &policy::PolicyMap,
    now_ms: i64,
) -> Option<map::Mapped> {
    let first = map::map_event(
        event,
        &map::MapContext {
            channel_name: None,
            author_name: roster.display_name(&event.pubkey),
            is_agent: roster.is_agent(&event.pubkey),
            now_ms,
        },
    )?;

    // Everything except the agent roster is channel-scoped and must clear its
    // channel's policy before anything is emitted.
    if first.subject != "agent" {
        let channel_id = first.payload["channel_id"].as_str().unwrap_or("");
        if !policy.is_bridgeable(channel_id) {
            return None;
        }
    }

    // Re-map with the resolved channel name now that policy has confirmed the
    // channel is known.
    let channel_id = first.payload["channel_id"].as_str().unwrap_or("");
    map::map_event(
        event,
        &map::MapContext {
            channel_name: policy.channel_name(channel_id),
            author_name: roster.display_name(&event.pubkey),
            is_agent: roster.is_agent(&event.pubkey),
            now_ms,
        },
    )
}

#[cfg(test)]
mod export_tests {
    use super::*;
    use crate::nostr::Event;

    const NOW: i64 = 1_800_000_000_000;

    fn ev(kind: u32, pubkey: &str, tags: Vec<Vec<String>>, content: &str) -> Event {
        Event {
            id: "a".repeat(64),
            pubkey: pubkey.to_string(),
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

    /// kind-39000 channel metadata, optionally carrying the `no-bridge` flag.
    fn channel_meta(channel_id: &str, name: &str, no_bridge: bool) -> Event {
        let mut tags = vec![tag(&["d", channel_id]), tag(&["name", name])];
        if no_bridge {
            tags.push(tag(&["no-bridge"]));
        }
        ev(39000, &"e".repeat(64), tags, "")
    }

    fn human_message(channel_id: &str, body: &str) -> Event {
        ev(9, &"b".repeat(64), vec![tag(&["h", channel_id])], body)
    }

    /// #636 — the risky half. A channel flagged `no-bridge` must produce NO
    /// export. Previously PolicyMap and map_event were each unit-tested while
    /// nothing exercised the two together, so the suppression path itself was
    /// unproven.
    #[test]
    fn no_bridge_channel_produces_no_export() {
        let roster = Roster::default();
        let mut policy = PolicyMap::new();
        policy.apply_metadata(&channel_meta("chan-private", "backchannel", true));

        let msg = human_message("chan-private", "something private");
        assert!(
            export_decision(&msg, &roster, &policy, NOW).is_none(),
            "a no-bridge channel must not cross to NATS — this is the only \
             privacy control on the bridge"
        );
    }

    /// The other direction: suppression must not be achieved by exporting
    /// nothing at all.
    #[test]
    fn bridgeable_channel_still_exports() {
        let roster = Roster::default();
        let mut policy = PolicyMap::new();
        policy.apply_metadata(&channel_meta("chan-open", "ops", false));

        let msg = human_message("chan-open", "hello");
        let mapped = export_decision(&msg, &roster, &policy, NOW)
            .expect("an unflagged channel must still export");
        assert_eq!(mapped.subject, "msg");
        assert_eq!(mapped.payload["channel_id"], "chan-open");
        // The re-map step is what resolves the human-readable channel name.
        assert_eq!(mapped.payload["channel_name"], "ops");
    }

    /// Fail-closed: a channel whose policy has never been resolved is not
    /// exported. This is the case that matters on a cold start, before the
    /// metadata sweep has completed.
    #[test]
    fn unresolved_channel_is_fail_closed() {
        let roster = Roster::default();
        let policy = PolicyMap::new();

        let msg = human_message("chan-unknown", "arrives before metadata");
        assert!(
            export_decision(&msg, &roster, &policy, NOW).is_none(),
            "an unresolved channel must be treated as not bridgeable"
        );
    }

    /// Flipping a channel to `no-bridge` must take effect — the refresh path
    /// is what stops an already-exporting channel within
    /// HIVE_POLICY_REFRESH_SECS.
    #[test]
    fn flagging_an_open_channel_stops_export() {
        let roster = Roster::default();
        let mut policy = PolicyMap::new();
        policy.apply_metadata(&channel_meta("chan-flip", "ops", false));
        let msg = human_message("chan-flip", "before");
        assert!(export_decision(&msg, &roster, &policy, NOW).is_some());

        // Newer metadata carrying the flag. created_at must advance, or the
        // ordering guard rejects it as stale.
        let mut later = channel_meta("chan-flip", "ops", true);
        later.created_at += 1;
        policy.apply_metadata(&later);

        assert!(
            export_decision(&msg, &roster, &policy, NOW).is_none(),
            "flipping a channel to no-bridge must stop its export"
        );
    }

    /// The agent roster is exempt because it is not channel-scoped — there is
    /// no channel policy for it to clear. Pinned so the exemption is not
    /// widened by accident into "anything without a channel bypasses policy".
    #[test]
    fn agent_subject_bypasses_the_channel_gate() {
        let roster = Roster::default();
        let policy = PolicyMap::new(); // deliberately empty
        let profile = ev(
            10100,
            &"b".repeat(64),
            vec![],
            r#"{"channel_add_policy":"any","owner":"eeee"}"#,
        );
        let mapped = export_decision(&profile, &roster, &policy, NOW)
            .expect("agent roster is not channel-scoped");
        assert_eq!(mapped.subject, "agent");
    }

    /// An unmappable event yields nothing regardless of policy.
    #[test]
    fn unmappable_event_is_not_exported() {
        let roster = Roster::default();
        let mut policy = PolicyMap::new();
        policy.apply_metadata(&channel_meta("chan-open", "ops", false));
        // kind 9 with no `h` tag has no channel at all.
        let orphan = ev(9, &"b".repeat(64), vec![], "orphan");
        assert!(export_decision(&orphan, &roster, &policy, NOW).is_none());
    }
}
