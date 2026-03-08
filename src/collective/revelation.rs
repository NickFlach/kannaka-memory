//! ADR-0013 Phase 6: Progressive Revelation
//!
//! Mechanisms for controlled privacy reduction over time:
//!
//! - **Time-based declassification**: Automatic difficulty reduction after a duration
//! - **Group bloom key workflows**: Manage selective sharing via group keys
//! - **Selective sharing**: Share specific glyphs with specific agents/groups
//! - **Revelation policies**: Configurable rules for when and how privacy decreases
//!
//! ## Design Principle
//!
//! Privacy only goes *down* via revelation — never up. The original sealed state
//! is always recorded in DoltHub history. A glyph can become more accessible
//! over time, but its creation privacy is immutable.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::collective::glyph_store::{GlyphStore, GroupKey};
use crate::collective::privacy::{BloomHint, create_hint};

// ============================================================================
// Revelation Policies
// ============================================================================

/// A policy that governs when and how a glyph's difficulty should decrease.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevelationPolicy {
    /// Which glyph this policy applies to
    pub glyph_hash: String,
    /// The agent that created this policy (must own the glyph)
    pub agent_id: String,
    /// The rule for revelation
    pub rule: RevelationRule,
    /// Whether this policy has been executed
    pub executed: bool,
    /// When this policy was created
    pub created_at: DateTime<Utc>,
}

/// Rules that trigger difficulty reduction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RevelationRule {
    /// Reduce difficulty after a fixed duration from creation
    TimeBased {
        /// Duration after glyph creation to trigger
        after_days: u32,
        /// New difficulty to set
        new_difficulty: u32,
    },
    /// Reduce difficulty for members of a specific group only
    GroupOnly {
        /// Group that gets reduced difficulty
        group_id: String,
        /// Difficulty for group members (typically 0)
        group_difficulty: u32,
    },
    /// Step-wise declassification: reduce difficulty in stages
    Staged {
        /// Stages: (days_after_creation, new_difficulty)
        stages: Vec<(u32, u32)>,
    },
    /// Manual — agent explicitly triggers the revelation
    Manual {
        new_difficulty: u32,
    },
}

/// Result of evaluating revelation policies against the current time.
#[derive(Debug, Clone)]
pub struct RevelationAction {
    pub glyph_hash: String,
    pub new_difficulty: u32,
    pub reason: String,
}

// ============================================================================
// Policy Evaluation
// ============================================================================

/// Evaluate a single policy against the current time.
///
/// Returns a `RevelationAction` if the policy should trigger now.
pub fn evaluate_policy(
    policy: &RevelationPolicy,
    store: &GlyphStore,
    now: DateTime<Utc>,
) -> Option<RevelationAction> {
    if policy.executed {
        return None;
    }

    let stored = store.get(&policy.glyph_hash)?;
    let current_diff = store.effective_difficulty(&policy.glyph_hash)?;

    match &policy.rule {
        RevelationRule::TimeBased { after_days, new_difficulty } => {
            let threshold = stored.glyph.created_at + Duration::days(*after_days as i64);
            if now >= threshold && *new_difficulty < current_diff {
                Some(RevelationAction {
                    glyph_hash: policy.glyph_hash.clone(),
                    new_difficulty: *new_difficulty,
                    reason: format!("time_based: {} days elapsed", after_days),
                })
            } else {
                None
            }
        }

        RevelationRule::Staged { stages } => {
            // Find the most aggressive stage that has triggered
            let mut best_action: Option<RevelationAction> = None;
            for (days, diff) in stages {
                let threshold = stored.glyph.created_at + Duration::days(*days as i64);
                if now >= threshold && *diff < current_diff {
                    let action = RevelationAction {
                        glyph_hash: policy.glyph_hash.clone(),
                        new_difficulty: *diff,
                        reason: format!("staged: {} days, difficulty → {}", days, diff),
                    };
                    match &best_action {
                        Some(existing) if existing.new_difficulty <= *diff => {}
                        _ => { best_action = Some(action); }
                    }
                }
            }
            best_action
        }

        RevelationRule::Manual { new_difficulty } => {
            // Manual policies are triggered externally, not by time
            // This variant exists for `execute_manual_revelation()`
            if *new_difficulty < current_diff {
                Some(RevelationAction {
                    glyph_hash: policy.glyph_hash.clone(),
                    new_difficulty: *new_difficulty,
                    reason: "manual".to_string(),
                })
            } else {
                None
            }
        }

        RevelationRule::GroupOnly { .. } => {
            // Group-based revelation doesn't use time — it's handled via
            // group key membership checks in the bloom path
            None
        }
    }
}

/// Evaluate all pending policies and return actions that should fire now.
pub fn evaluate_pending_policies(
    policies: &[RevelationPolicy],
    store: &GlyphStore,
    now: DateTime<Utc>,
) -> Vec<RevelationAction> {
    policies.iter()
        .filter_map(|p| evaluate_policy(p, store, now))
        .collect()
}

/// Execute a revelation action by publishing a bloom hint to the store.
///
/// Returns the generated `BloomHint`, or `None` if the glyph doesn't exist
/// or the hint creation fails.
pub fn execute_revelation(
    store: &mut GlyphStore,
    action: &RevelationAction,
    agent_id: &str,
) -> Option<BloomHint> {
    let stored = store.get(&action.glyph_hash)?;
    let hint = create_hint(&stored.glyph, action.new_difficulty, agent_id)?;
    store.publish_hint(hint.clone());
    Some(hint)
}

// ============================================================================
// Group Key Management Workflows
// ============================================================================

/// Create a new group for selective sharing.
pub fn create_group(
    store: &mut GlyphStore,
    group_id: &str,
    creator: &str,
    initial_members: Vec<String>,
) -> GroupKey {
    // Generate deterministic key material from group_id + creator
    let key_material = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        group_id.hash(&mut hasher);
        creator.hash(&mut hasher);
        let h = hasher.finish();
        h.to_le_bytes().to_vec()
    };

    let mut members = initial_members;
    if !members.contains(&creator.to_string()) {
        members.push(creator.to_string());
    }

    let key = GroupKey {
        group_id: group_id.to_string(),
        key_material,
        created_by: creator.to_string(),
        created_at: Utc::now(),
        members,
        revoked: Vec::new(),
    };

    store.register_group_key(key.clone());
    key
}

/// Add a member to an existing group.
///
/// Only the group creator can add members. Returns `true` if successful.
pub fn add_group_member(
    store: &mut GlyphStore,
    group_id: &str,
    new_member: &str,
    requester: &str,
) -> bool {
    // Get a clone to check creator, since we need mutable access after
    let key = match store.get_group_key(group_id) {
        Some(k) => k.clone(),
        None => return false,
    };

    if key.created_by != requester {
        return false;
    }

    if key.members.contains(&new_member.to_string()) {
        return true; // Already a member
    }

    let mut updated = key;
    updated.members.push(new_member.to_string());
    // Un-revoke if previously revoked
    updated.revoked.retain(|r| r != new_member);
    store.register_group_key(updated);
    true
}

/// Revoke a member from a group.
///
/// Only the group creator can revoke. Returns `true` if successful.
pub fn revoke_group_member(
    store: &mut GlyphStore,
    group_id: &str,
    member: &str,
    requester: &str,
) -> bool {
    let key = match store.get_group_key(group_id) {
        Some(k) => k.clone(),
        None => return false,
    };

    if key.created_by != requester {
        return false;
    }

    if member == requester {
        return false; // Can't revoke yourself (creator)
    }

    let mut updated = key;
    if !updated.revoked.contains(&member.to_string()) {
        updated.revoked.push(member.to_string());
    }
    store.register_group_key(updated);
    true
}

/// Check if an agent should get reduced difficulty for a glyph via group membership.
///
/// Looks through all policies for `GroupOnly` rules where the agent is a member.
/// Returns the lowest group difficulty, or `None` if no group grants apply.
pub fn group_effective_difficulty(
    policies: &[RevelationPolicy],
    store: &GlyphStore,
    glyph_hash: &str,
    agent_id: &str,
) -> Option<u32> {
    let mut best: Option<u32> = None;

    for policy in policies {
        if policy.glyph_hash != glyph_hash {
            continue;
        }
        if let RevelationRule::GroupOnly { group_id, group_difficulty } = &policy.rule {
            if store.is_group_member(group_id, agent_id) {
                match best {
                    Some(d) if d <= *group_difficulty => {}
                    _ => { best = Some(*group_difficulty); }
                }
            }
        }
    }

    best
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collective::privacy::seal_with_commitments;
    use crate::memory::HyperMemory;
    use chrono::Duration;

    fn test_memory(content: &str) -> HyperMemory {
        HyperMemory::new(vec![0.1; 100], content.to_string())
    }

    fn setup_store_with_glyph(difficulty: u32) -> (GlyphStore, String) {
        let mut store = GlyphStore::new();
        let mem = test_memory("classified data");
        let result = seal_with_commitments(&mem, difficulty, "alice");
        let hash = result.glyph.glyph_hash.clone();
        store.insert(result);
        (store, hash)
    }

    #[test]
    fn test_time_based_policy_triggers() {
        let (store, hash) = setup_store_with_glyph(32);
        let policy = RevelationPolicy {
            glyph_hash: hash.clone(),
            agent_id: "alice".to_string(),
            rule: RevelationRule::TimeBased {
                after_days: 30,
                new_difficulty: 8,
            },
            executed: false,
            created_at: Utc::now(),
        };

        // Not triggered yet (now)
        let action = evaluate_policy(&policy, &store, Utc::now());
        assert!(action.is_none());

        // Triggered after 31 days
        let future = Utc::now() + Duration::days(31);
        let action = evaluate_policy(&policy, &store, future);
        assert!(action.is_some());
        assert_eq!(action.unwrap().new_difficulty, 8);
    }

    #[test]
    fn test_time_based_policy_skips_if_already_lower() {
        let (store, hash) = setup_store_with_glyph(4); // Already low
        let policy = RevelationPolicy {
            glyph_hash: hash.clone(),
            agent_id: "alice".to_string(),
            rule: RevelationRule::TimeBased {
                after_days: 1,
                new_difficulty: 8, // Higher than current!
            },
            executed: false,
            created_at: Utc::now(),
        };

        let future = Utc::now() + Duration::days(2);
        let action = evaluate_policy(&policy, &store, future);
        assert!(action.is_none()); // Won't increase difficulty
    }

    #[test]
    fn test_staged_revelation() {
        let (store, hash) = setup_store_with_glyph(48);
        let policy = RevelationPolicy {
            glyph_hash: hash.clone(),
            agent_id: "alice".to_string(),
            rule: RevelationRule::Staged {
                stages: vec![
                    (30, 32),  // After 30 days → difficulty 32
                    (90, 16),  // After 90 days → difficulty 16
                    (365, 0),  // After 1 year → public
                ],
            },
            executed: false,
            created_at: Utc::now(),
        };

        // At 60 days: first stage triggers
        let day60 = Utc::now() + Duration::days(60);
        let action = evaluate_policy(&policy, &store, day60).unwrap();
        assert_eq!(action.new_difficulty, 32);

        // At 100 days: second stage triggers (lower difficulty wins)
        let day100 = Utc::now() + Duration::days(100);
        let action = evaluate_policy(&policy, &store, day100).unwrap();
        assert_eq!(action.new_difficulty, 16);

        // At 400 days: final stage — public
        let day400 = Utc::now() + Duration::days(400);
        let action = evaluate_policy(&policy, &store, day400).unwrap();
        assert_eq!(action.new_difficulty, 0);
    }

    #[test]
    fn test_executed_policy_skips() {
        let (store, hash) = setup_store_with_glyph(32);
        let policy = RevelationPolicy {
            glyph_hash: hash.clone(),
            agent_id: "alice".to_string(),
            rule: RevelationRule::TimeBased {
                after_days: 1,
                new_difficulty: 8,
            },
            executed: true, // Already done
            created_at: Utc::now(),
        };

        let future = Utc::now() + Duration::days(10);
        assert!(evaluate_policy(&policy, &store, future).is_none());
    }

    #[test]
    fn test_manual_revelation() {
        let (store, hash) = setup_store_with_glyph(32);
        let policy = RevelationPolicy {
            glyph_hash: hash.clone(),
            agent_id: "alice".to_string(),
            rule: RevelationRule::Manual { new_difficulty: 0 },
            executed: false,
            created_at: Utc::now(),
        };

        let action = evaluate_policy(&policy, &store, Utc::now());
        assert!(action.is_some());
        assert_eq!(action.unwrap().new_difficulty, 0);
    }

    #[test]
    fn test_execute_revelation_publishes_hint() {
        let (mut store, hash) = setup_store_with_glyph(32);
        let action = RevelationAction {
            glyph_hash: hash.clone(),
            new_difficulty: 8,
            reason: "test".to_string(),
        };

        let hint = execute_revelation(&mut store, &action, "alice");
        assert!(hint.is_some());
        assert_eq!(store.effective_difficulty(&hash), Some(8));
    }

    #[test]
    fn test_create_group() {
        let mut store = GlyphStore::new();
        let key = create_group(
            &mut store,
            "mars-colony",
            "alice",
            vec!["bob".to_string(), "charlie".to_string()],
        );

        assert_eq!(key.group_id, "mars-colony");
        assert_eq!(key.created_by, "alice");
        assert!(key.members.contains(&"alice".to_string()));
        assert!(key.members.contains(&"bob".to_string()));
        assert!(store.is_group_member("mars-colony", "alice"));
        assert!(store.is_group_member("mars-colony", "bob"));
    }

    #[test]
    fn test_add_group_member() {
        let mut store = GlyphStore::new();
        create_group(&mut store, "team-1", "alice", vec![]);

        assert!(add_group_member(&mut store, "team-1", "dave", "alice"));
        assert!(store.is_group_member("team-1", "dave"));

        // Non-creator can't add
        assert!(!add_group_member(&mut store, "team-1", "eve", "bob"));
    }

    #[test]
    fn test_revoke_group_member() {
        let mut store = GlyphStore::new();
        create_group(&mut store, "team-2", "alice", vec!["bob".to_string()]);

        assert!(store.is_group_member("team-2", "bob"));
        assert!(revoke_group_member(&mut store, "team-2", "bob", "alice"));
        assert!(!store.is_group_member("team-2", "bob"));
    }

    #[test]
    fn test_creator_cant_self_revoke() {
        let mut store = GlyphStore::new();
        create_group(&mut store, "team-3", "alice", vec![]);

        assert!(!revoke_group_member(&mut store, "team-3", "alice", "alice"));
        assert!(store.is_group_member("team-3", "alice"));
    }

    #[test]
    fn test_group_effective_difficulty() {
        let (mut store, hash) = setup_store_with_glyph(48);
        create_group(&mut store, "mars-colony", "alice", vec!["bob".to_string()]);

        let policies = vec![
            RevelationPolicy {
                glyph_hash: hash.clone(),
                agent_id: "alice".to_string(),
                rule: RevelationRule::GroupOnly {
                    group_id: "mars-colony".to_string(),
                    group_difficulty: 0,
                },
                executed: false,
                created_at: Utc::now(),
            },
        ];

        // Bob is in mars-colony → difficulty 0
        let diff = group_effective_difficulty(&policies, &store, &hash, "bob");
        assert_eq!(diff, Some(0));

        // Charlie is NOT in mars-colony → None
        let diff = group_effective_difficulty(&policies, &store, &hash, "charlie");
        assert!(diff.is_none());
    }

    #[test]
    fn test_evaluate_pending_policies() {
        let (store, hash) = setup_store_with_glyph(48);
        let policies = vec![
            RevelationPolicy {
                glyph_hash: hash.clone(),
                agent_id: "alice".to_string(),
                rule: RevelationRule::TimeBased {
                    after_days: 1,
                    new_difficulty: 16,
                },
                executed: false,
                created_at: Utc::now(),
            },
            RevelationPolicy {
                glyph_hash: hash.clone(),
                agent_id: "alice".to_string(),
                rule: RevelationRule::TimeBased {
                    after_days: 365,
                    new_difficulty: 0,
                },
                executed: false,
                created_at: Utc::now(),
            },
        ];

        let future = Utc::now() + Duration::days(2);
        let actions = evaluate_pending_policies(&policies, &store, future);
        assert_eq!(actions.len(), 1); // Only first policy triggers
        assert_eq!(actions[0].new_difficulty, 16);
    }
}
