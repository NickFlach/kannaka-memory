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
    created_at: i64,
    event_id: String,
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
    /// are ignored. Enforces ordering: if an older event arrives after a newer
    /// one, it is silently rejected. On a tie in created_at, lexicographically
    /// greater event IDs win to ensure convergence across nodes.
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

        // Check ordering guard: reject if we already have a channel policy
        // for this id and the incoming event is strictly older
        if let Some(existing) = self.channels.get(&channel_id) {
            if event.created_at < existing.created_at {
                // Incoming event is older, silently reject
                return;
            }
            if event.created_at == existing.created_at {
                // Tie: keep the lexicographically greater event ID.
                // Only reject if incoming ID is strictly less than existing.
                if event.id < existing.event_id {
                    return;
                }
            }
        }

        self.channels.insert(
            channel_id,
            ChannelPolicy {
                name,
                no_bridge,
                created_at: event.created_at,
                event_id: event.id.clone(),
            },
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nostr::Event;

    fn meta(channel: &str, name: &str, no_bridge: bool) -> Event {
        meta_with_created_at(channel, name, no_bridge, 1_800_000_000, "a")
    }

    fn meta_with_created_at(
        channel: &str,
        name: &str,
        no_bridge: bool,
        created_at: i64,
        id_char: &str,
    ) -> Event {
        let mut tags = vec![
            vec!["d".to_string(), channel.to_string()],
            vec!["name".to_string(), name.to_string()],
        ];
        if no_bridge {
            tags.push(vec!["no-bridge".to_string()]);
        }
        Event {
            id: id_char.repeat(64),
            pubkey: "b".repeat(64),
            created_at,
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

    #[test]
    fn older_event_does_not_reopen_no_bridge_channel() {
        let mut p = PolicyMap::new();
        // Apply newer event that sets no-bridge
        p.apply_metadata(&meta_with_created_at("chan-5", "secrets", true, 2_000_000_000, "a"));
        assert!(!p.is_bridgeable("chan-5"));
        // Apply older event that tries to clear no-bridge
        p.apply_metadata(&meta_with_created_at("chan-5", "secrets", false, 1_900_000_000, "b"));
        // Channel should still not be bridgeable (older event ignored)
        assert!(!p.is_bridgeable("chan-5"));
    }

    #[test]
    fn newer_event_does_unflag_no_bridge() {
        let mut p = PolicyMap::new();
        // Apply older event that sets no-bridge
        p.apply_metadata(&meta_with_created_at("chan-6", "ops", true, 1_900_000_000, "a"));
        assert!(!p.is_bridgeable("chan-6"));
        // Apply newer event that clears no-bridge
        p.apply_metadata(&meta_with_created_at("chan-6", "ops", false, 2_000_000_000, "b"));
        // Channel should now be bridgeable
        assert!(p.is_bridgeable("chan-6"));
    }
}
