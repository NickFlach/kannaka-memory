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

/// What an `["OK", <id>, <accepted>, <detail>]` frame means for the bridge.
///
/// Extracted from the relay loop for the same reason as `export_decision`:
/// the interesting outcome kills the process, which is not reachable from a
/// test if the decision lives inline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OkVerdict {
    /// Our AUTH event was accepted — the connection is genuinely usable.
    AuthAccepted,
    /// Our AUTH event was rejected. Fatal: every later REQ silently returns
    /// nothing, so there is nothing to degrade to.
    AuthRejected(String),
    /// Some other event was rejected. Worth logging, not fatal.
    EventRejected { id: String, detail: String },
    /// An accepted non-auth event, or a malformed frame.
    Ignored,
}

/// Classify an OK frame. `auth_event_id` is `None` before we have sent AUTH.
pub fn classify_ok(frame: &serde_json::Value, auth_event_id: Option<&str>) -> OkVerdict {
    let Some(id) = frame.get(1).and_then(|v| v.as_str()) else {
        return OkVerdict::Ignored;
    };
    // A missing/non-bool accepted flag is treated as rejection: this frame
    // exists to carry a failure, and defaulting to "fine" would restore the
    // exact silence being fixed.
    let accepted = frame.get(2).and_then(|v| v.as_bool()).unwrap_or(false);
    let detail = frame.get(3).and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Only match the auth id when we actually have one — `None == None` would
    // otherwise make an id-less frame look like our auth result.
    if auth_event_id.is_some() && auth_event_id == Some(id) {
        return if accepted {
            OkVerdict::AuthAccepted
        } else {
            OkVerdict::AuthRejected(detail)
        };
    }
    if accepted {
        OkVerdict::Ignored
    } else {
        OkVerdict::EventRejected { id: id.to_string(), detail }
    }
}

/// Whether to give up on a connection that has not opened its content
/// subscription yet.
///
/// Split out because the dangerous direction is the false positive: a bug that
/// let this return true for an already-subscribed bridge would kill a healthy
/// daemon on a timer.
pub fn subscribe_deadline_expired(subscribed: bool, elapsed_secs: i64, deadline_secs: i64) -> bool {
    !subscribed && elapsed_secs >= deadline_secs
}

#[cfg(test)]
mod ok_frame_tests {
    use super::*;
    use serde_json::json;

    const AUTH_ID: &str = "aaaa1111";

    #[test]
    fn rejected_auth_is_fatal_and_carries_the_reason() {
        let f = json!(["OK", AUTH_ID, false, "auth-required: not an allowlisted member"]);
        assert_eq!(
            classify_ok(&f, Some(AUTH_ID)),
            OkVerdict::AuthRejected("auth-required: not an allowlisted member".to_string()),
        );
    }

    #[test]
    fn accepted_auth_is_recognised() {
        let f = json!(["OK", AUTH_ID, true, ""]);
        assert_eq!(classify_ok(&f, Some(AUTH_ID)), OkVerdict::AuthAccepted);
    }

    #[test]
    fn another_events_rejection_is_not_mistaken_for_auth() {
        // The bug this guards: killing the bridge because some unrelated
        // event was rejected.
        let f = json!(["OK", "bbbb2222", false, "rate-limited"]);
        assert_eq!(
            classify_ok(&f, Some(AUTH_ID)),
            OkVerdict::EventRejected {
                id: "bbbb2222".to_string(),
                detail: "rate-limited".to_string()
            },
        );
    }

    #[test]
    fn ok_before_auth_is_never_read_as_an_auth_result() {
        let f = json!(["OK", "bbbb2222", false, "nope"]);
        assert!(!matches!(
            classify_ok(&f, None),
            OkVerdict::AuthRejected(_) | OkVerdict::AuthAccepted
        ));
    }

    #[test]
    fn a_missing_accepted_flag_is_not_treated_as_success() {
        let f = json!(["OK", AUTH_ID]);
        assert!(matches!(classify_ok(&f, Some(AUTH_ID)), OkVerdict::AuthRejected(_)));
    }

    #[test]
    fn malformed_frames_are_ignored() {
        assert_eq!(classify_ok(&json!(["OK"]), Some(AUTH_ID)), OkVerdict::Ignored);
    }

    #[test]
    fn deadline_only_fires_before_the_subscription_opens() {
        assert!(subscribe_deadline_expired(false, 60, 60), "should give up once past the deadline");
        assert!(!subscribe_deadline_expired(false, 59, 60), "not yet");
        // The one that matters: a live bridge must never be killed on a timer.
        assert!(
            !subscribe_deadline_expired(true, 100_000, 60),
            "a subscribed bridge must survive indefinitely"
        );
    }
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


    /// #643 — the degenerate-metadata bypass, end to end.
    ///
    /// `["d", ""]` used to register a channel whose id was the empty string,
    /// and `["h", ""]` used to resolve to that same id. Together, ONE such
    /// metadata event made every h-empty message clear the policy gate —
    /// defeating the only privacy control on the bridge without any channel
    /// ever being flagged.
    #[test]
    fn degenerate_empty_channel_metadata_cannot_make_traffic_bridgeable() {
        let roster = Roster::default();
        let mut policy = PolicyMap::new();

        // An unflagged metadata event with an EMPTY d tag.
        let degenerate = ev(
            39000,
            &"e".repeat(64),
            vec![tag(&["d", ""]), tag(&["name", "nowhere"])],
            "",
        );
        policy.apply_metadata(&degenerate);
        assert_eq!(policy.len(), 0, "an empty channel id must not be registered");

        // A message claiming the empty channel.
        let msg = ev(9, &"b".repeat(64), vec![tag(&["h", ""])], "should not cross");
        assert!(
            export_decision(&msg, &roster, &policy, NOW).is_none(),
            "an empty channel id must never be bridgeable"
        );
    }

    /// Even with a legitimately bridgeable channel present, an h-empty message
    /// must not borrow its clearance.
    #[test]
    fn empty_h_tag_does_not_inherit_another_channels_clearance() {
        let roster = Roster::default();
        let mut policy = PolicyMap::new();
        policy.apply_metadata(&channel_meta("chan-open", "ops", false));

        let msg = ev(9, &"b".repeat(64), vec![tag(&["h", ""])], "orphan");
        assert!(export_decision(&msg, &roster, &policy, NOW).is_none());
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
