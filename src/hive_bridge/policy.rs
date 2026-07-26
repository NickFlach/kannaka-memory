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
