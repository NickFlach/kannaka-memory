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
// No `Eq`: the payload is a serde_json::Value, whose float variant has no
// total equality.
#[derive(Debug, Clone, PartialEq)]
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
        // An empty tag value is absence, not a value. Without this an
        // `["h", ""]` message claimed channel id "" — which, paired with a
        // degenerate `["d", ""]` metadata event, made every such message
        // bridgeable and bypassed the no-bridge gate entirely. (#643)
        .filter(|s| !s.is_empty())
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
