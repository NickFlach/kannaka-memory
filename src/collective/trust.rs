//! ADR-0011 Phase 10: Agent trust scoring system.
//!
//! Trust scores are maintained per agent and influence how much weight their
//! memories carry during merge operations. Trust is:
//! - Initialized at 0.5 for unknown agents
//! - Increased when merged memories prove accurate (confirmed by dream cycles)
//! - Decreased when memories are quarantined or pruned
//! - Capped at [0.1, 1.0]
//! - Logged via Dolt history (every change is auditable)

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// AgentRecord
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecord {
    pub agent_id: String,
    pub display_name: Option<String>,
    pub trust_score: f32,
    pub last_sync: Option<DateTime<Utc>>,
    pub branch_name: Option<String>,
    pub flux_entity: Option<String>,
    pub embedding_model: Option<String>,
    pub successful_merges: u32,
    pub quarantined_merges: u32,
    pub pruned_merges: u32,
    pub created_at: DateTime<Utc>,
}

impl AgentRecord {
    pub fn new(agent_id: &str) -> Self {
        Self {
            agent_id: agent_id.to_string(),
            display_name: None,
            trust_score: 0.5,
            last_sync: None,
            branch_name: Some(format!("{}/working", agent_id)),
            flux_entity: Some(agent_id.to_string()),
            embedding_model: None,
            successful_merges: 0,
            quarantined_merges: 0,
            pruned_merges: 0,
            created_at: Utc::now(),
        }
    }
}

// ---------------------------------------------------------------------------
// Trust score adjustments
// ---------------------------------------------------------------------------

const TRUST_MIN: f32 = 0.1;
const TRUST_MAX: f32 = 1.0;
const SUCCESSFUL_MERGE_BONUS: f32 = 0.02;
const QUARANTINE_PENALTY: f32 = 0.05;
const PRUNE_PENALTY: f32 = 0.03;

fn clamp_trust(t: f32) -> f32 {
    t.max(TRUST_MIN).min(TRUST_MAX)
}

// ---------------------------------------------------------------------------
// AgentTrustStore
// ---------------------------------------------------------------------------

/// In-memory trust store. Syncs to the `agents` table in Dolt when `--dolt` is active.
#[derive(Debug, Default)]
pub struct AgentTrustStore {
    agents: HashMap<String, AgentRecord>,
}

impl AgentTrustStore {
    pub fn new() -> Self {
        Self { agents: HashMap::new() }
    }

    /// Get or create an agent record with default trust 0.5.
    pub fn get_or_create(&mut self, agent_id: &str) -> &mut AgentRecord {
        self.agents
            .entry(agent_id.to_string())
            .or_insert_with(|| AgentRecord::new(agent_id))
    }

    /// Get trust score for an agent (0.5 for unknown).
    pub fn trust_score(&self, agent_id: &str) -> f32 {
        self.agents.get(agent_id).map(|a| a.trust_score).unwrap_or(0.5)
    }

    /// Record a successful constructive merge from this agent — boost trust.
    pub fn record_successful_merge(&mut self, agent_id: &str) {
        let rec = self.get_or_create(agent_id);
        rec.successful_merges += 1;
        rec.trust_score = clamp_trust(rec.trust_score + SUCCESSFUL_MERGE_BONUS);
    }

    /// Record a quarantined (destructive) merge from this agent — reduce trust.
    pub fn record_quarantine(&mut self, agent_id: &str) {
        let rec = self.get_or_create(agent_id);
        rec.quarantined_merges += 1;
        rec.trust_score = clamp_trust(rec.trust_score - QUARANTINE_PENALTY);
    }

    /// Record a pruned imported memory from this agent — mild trust reduction.
    pub fn record_prune(&mut self, agent_id: &str) {
        let rec = self.get_or_create(agent_id);
        rec.pruned_merges += 1;
        rec.trust_score = clamp_trust(rec.trust_score - PRUNE_PENALTY);
    }

    /// Update last sync timestamp.
    pub fn record_sync(&mut self, agent_id: &str) {
        let rec = self.get_or_create(agent_id);
        rec.last_sync = Some(Utc::now());
    }

    /// Recency factor for a memory's age. Uses exponential decay.
    ///
    /// `half_life_days` varies by category:
    /// - knowledge:   30 days
    /// - experience:   7 days
    /// - social:      14 days
    /// - skill:       21 days
    pub fn recency_factor(age_days: f64, half_life_days: f64) -> f32 {
        let exponent = -(age_days / half_life_days) * std::f64::consts::LN_2;
        exponent.exp() as f32
    }

    /// Category-specific half-life in days.
    pub fn half_life_for_category(category: &str) -> f64 {
        match category {
            "experience" => 7.0,
            "social"     => 14.0,
            "skill"      => 21.0,
            _            => 30.0, // knowledge, default
        }
    }

    /// Compute trust-weighted effective amplitude.
    pub fn effective_amplitude(&self, agent_id: &str, amplitude: f32, age_days: f64, category: &str) -> f32 {
        let trust = self.trust_score(agent_id);
        let half_life = Self::half_life_for_category(category);
        let recency = Self::recency_factor(age_days, half_life);
        amplitude * trust * recency
    }

    /// All known agents.
    pub fn agents(&self) -> impl Iterator<Item = &AgentRecord> {
        self.agents.values()
    }

    /// Load agents from a snapshot (e.g. from Dolt query).
    pub fn load_agents(&mut self, agents: Vec<AgentRecord>) {
        for agent in agents {
            self.agents.insert(agent.agent_id.clone(), agent);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_trust_is_half() {
        let store = AgentTrustStore::new();
        assert_eq!(store.trust_score("unknown-agent"), 0.5);
    }

    #[test]
    fn successful_merges_boost_trust() {
        let mut store = AgentTrustStore::new();
        store.record_successful_merge("arc");
        store.record_successful_merge("arc");
        let t = store.trust_score("arc");
        assert!(t > 0.5, "trust should increase: {}", t);
    }

    #[test]
    fn quarantine_reduces_trust() {
        let mut store = AgentTrustStore::new();
        store.record_quarantine("rogue");
        let t = store.trust_score("rogue");
        assert!(t < 0.5, "trust should decrease: {}", t);
    }

    #[test]
    fn trust_clamped_at_min_max() {
        let mut store = AgentTrustStore::new();
        for _ in 0..200 {
            store.record_quarantine("rogue");
        }
        assert!(store.trust_score("rogue") >= TRUST_MIN);

        for _ in 0..200 {
            store.record_successful_merge("hero");
        }
        assert!(store.trust_score("hero") <= TRUST_MAX);
    }

    #[test]
    fn recency_factor_decays() {
        let f0 = AgentTrustStore::recency_factor(0.0, 30.0);
        let f1 = AgentTrustStore::recency_factor(30.0, 30.0);
        assert!((f0 - 1.0).abs() < 1e-5);
        assert!((f1 - 0.5).abs() < 1e-3);
    }
}
