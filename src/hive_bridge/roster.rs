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
