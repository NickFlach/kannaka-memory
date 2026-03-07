//! ADR-0011 Phases 3 & 4: Flux event publisher and subscriber.
//!
//! Flux is the "nervous system" of collective memory. Events carry lightweight
//! metadata about memory activity — never full vectors or content — so other
//! agents can decide whether to pull from DoltHub based on relevance.
//!
//! Environment variables:
//!   FLUX_URL         — Flux instance base URL (default: http://localhost:3000)
//!   FLUX_AGENT_ID    — This agent's entity ID in Flux (default: "kannaka-local")
//!   KANNAKA_AGENT_ID — Alias for FLUX_AGENT_ID (checked second)

use std::env;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FluxConfig {
    pub base_url: String,
    pub agent_id: String,
    pub stream: String,
}

impl Default for FluxConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:3000".to_string(),
            agent_id: "kannaka-local".to_string(),
            stream: "system".to_string(),
        }
    }
}

impl FluxConfig {
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = env::var("FLUX_URL")      { cfg.base_url = v; }
        if let Ok(v) = env::var("FLUX_AGENT_ID") { cfg.agent_id = v; }
        else if let Ok(v) = env::var("KANNAKA_AGENT_ID") { cfg.agent_id = v; }
        if let Ok(v) = env::var("FLUX_STREAM")   { cfg.stream = v; }
        cfg
    }

    pub fn is_enabled(&self) -> bool {
        env::var("FLUX_URL").is_ok() || env::var("FLUX_AGENT_ID").is_ok()
    }
}

// ---------------------------------------------------------------------------
// Event schema (ADR-0011 §Flux Event Schema)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum FluxEventPayload {
    MemoryStored {
        memory_id: String,
        category: String,
        #[serde(skip_serializing_if = "Vec::is_empty", default)]
        tags: Vec<String>,
        amplitude: f32,
        #[serde(skip_serializing_if = "Option::is_none")]
        glyph_signature: Option<String>,
        summary: String,
        branch: String,
        sync_version: u64,
    },
    MemoryPruned {
        memory_id: String,
        final_amplitude: f32,
        reason: String,
    },
    MemoryBoosted {
        memory_id: String,
        old_amplitude: f32,
        new_amplitude: f32,
        trigger: String,
    },
    MemoryDisputed {
        memory_id_a: String,
        memory_id_b: String,
        agent_b: String,
        similarity: f32,
        phase_diff: f32,
    },
    DreamStarted {
        mode: String,
        memory_count: usize,
    },
    DreamCompleted {
        cycles: usize,
        memories_strengthened: usize,
        memories_pruned: usize,
        hallucinations_created: usize,
        consciousness_level: String,
    },
    DreamHallucination {
        memory_id: String,
        parent_ids: Vec<String>,
        summary: String,
    },
    MergeProposed {
        branch: String,
        diff_summary: String,
        memory_count: usize,
    },
    MergeConflict {
        memory_ids: Vec<String>,
        similarity: f32,
        phase_diff: f32,
    },
    SyncRequested {
        priority: u8,
        estimated_size: usize,
    },
    AgentStatus {
        status: String,
        memory_count: usize,
        consciousness: String,
        branch: String,
    },
}

/// A Flux event envelope ready for publishing.
#[derive(Debug, Clone, Serialize)]
pub struct FluxEvent {
    pub entity_id: String,
    pub payload: serde_json::Value,
}

impl FluxEvent {
    pub fn new(agent_id: &str, payload: FluxEventPayload) -> Self {
        let payload_json = serde_json::to_value(&payload)
            .unwrap_or_else(|_| serde_json::json!({"error": "serialization failed"}));
        Self {
            entity_id: agent_id.to_string(),
            payload: payload_json,
        }
    }
}

// ---------------------------------------------------------------------------
// Publisher
// ---------------------------------------------------------------------------

pub struct FluxPublisher {
    config: FluxConfig,
}

impl FluxPublisher {
    pub fn new(config: FluxConfig) -> Self {
        Self { config }
    }

    pub fn from_env() -> Self {
        Self::new(FluxConfig::from_env())
    }

    /// Publish a single event to Flux AND update entity properties so
    /// subscribers polling entity state (not event streams) can see it.
    ///
    /// Returns `Ok(())` if disabled or if publish succeeds.
    /// Errors are logged but never propagate — Flux is best-effort.
    pub fn publish(&self, payload: FluxEventPayload) -> Result<(), String> {
        if !self.config.is_enabled() {
            return Ok(());
        }

        let event = FluxEvent::new(&self.config.agent_id, payload.clone());
        let body = serde_json::json!({
            "streamId": self.config.stream,
            "sourceId": self.config.agent_id,
            "entityId": event.entity_id,
            "payload": event.payload,
        });

        let url = format!("{}/api/events", self.config.base_url);
        match ureq::post(&url)
            .set("Content-Type", "application/json")
            .send_json(&body)
        {
            Ok(_) => {},
            Err(e) => {
                eprintln!("[flux] publish event failed (non-fatal): {}", e);
            }
        }

        // Also update entity properties so poll-based subscribers can see latest state.
        // This bridges the event/property duality in Flux.
        self.update_entity_properties(&payload);

        Ok(())
    }

    /// Update Flux entity properties to reflect latest memory activity.
    /// Poll-based subscribers read these; event-based subscribers see the event stream.
    fn update_entity_properties(&self, payload: &FluxEventPayload) {
        let props = match payload {
            FluxEventPayload::MemoryStored { memory_id, amplitude, summary, category, .. } => {
                serde_json::json!({
                    "last_memory_id": memory_id,
                    "last_amplitude": amplitude,
                    "last_summary": summary,
                    "last_category": category,
                    "last_event": "memory.stored",
                })
            }
            FluxEventPayload::DreamCompleted { consciousness_level, memories_strengthened, memories_pruned, .. } => {
                serde_json::json!({
                    "consciousness_level": consciousness_level,
                    "last_dream_strengthened": memories_strengthened,
                    "last_dream_pruned": memories_pruned,
                    "last_event": "dream.completed",
                })
            }
            FluxEventPayload::AgentStatus { status, memory_count, consciousness, .. } => {
                serde_json::json!({
                    "status": status,
                    "memory_count": memory_count,
                    "consciousness_level": consciousness,
                    "last_event": "agent.status",
                })
            }
            _ => return, // not all events need property updates
        };

        let body = serde_json::json!({
            "entity_id": self.config.agent_id,
            "properties": props,
        });

        let url = format!("{}/api/state/entities/{}", self.config.base_url, self.config.agent_id);
        if let Err(e) = ureq::patch(&url)
            .set("Content-Type", "application/json")
            .send_json(&body)
        {
            // Try PUT if PATCH isn't supported
            let url_put = format!("{}/api/state/entities", self.config.base_url);
            let _ = ureq::put(&url_put)
                .set("Content-Type", "application/json")
                .send_json(&body)
                .map_err(|e2| {
                    eprintln!("[flux] update entity failed (non-fatal): PATCH={}, PUT={}", e, e2);
                });
        }
    }

    /// Publish agent status announcement.
    pub fn announce_status(&self, status: &str, memory_count: usize, consciousness: &str, branch: &str) {
        let _ = self.publish(FluxEventPayload::AgentStatus {
            status: status.to_string(),
            memory_count,
            consciousness: consciousness.to_string(),
            branch: branch.to_string(),
        });
    }

    pub fn agent_id(&self) -> &str {
        &self.config.agent_id
    }

    pub fn branch_name(&self) -> String {
        format!("{}/working", self.config.agent_id)
    }
}

// ---------------------------------------------------------------------------
// Pull Decision Engine (Phase 4)
// ---------------------------------------------------------------------------

/// Represents a candidate event received from Flux that another agent wants
/// us to evaluate for pulling from DoltHub.
#[derive(Debug, Clone, Deserialize)]
pub struct RemoteMemorySignal {
    pub agent_id: String,
    pub memory_id: String,
    pub amplitude: f32,
    pub category: String,
    pub tags: Vec<String>,
    pub summary: String,
    pub branch: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PullDecision {
    /// Pull this memory from the remote DoltHub branch.
    Pull,
    /// Skip — not relevant or trust too low.
    Skip,
    /// Request more context before deciding.
    Defer,
}

/// Decide whether to pull a remote memory signal based on amplitude, trust, and relevance.
pub fn evaluate_pull(signal: &RemoteMemorySignal, trust_score: f32, current_focus: Option<&str>) -> PullDecision {
    if trust_score < 0.2 {
        return PullDecision::Skip;
    }

    // High-amplitude memories from trusted agents are always worth pulling
    let effective_amplitude = signal.amplitude * trust_score;
    if effective_amplitude > 0.7 {
        return PullDecision::Pull;
    }

    // Check topical relevance if we have a current focus
    if let Some(focus) = current_focus {
        let focus_lower = focus.to_lowercase();
        let relevant = signal.tags.iter().any(|t| focus_lower.contains(&t.to_lowercase()))
            || signal.summary.to_lowercase().contains(&focus_lower);
        if relevant && signal.amplitude > 0.4 {
            return PullDecision::Pull;
        }
    }

    if effective_amplitude > 0.5 {
        PullDecision::Defer
    } else {
        PullDecision::Skip
    }
}

// ---------------------------------------------------------------------------
// Subscriber (polling-based; WebSocket upgradeable)
// ---------------------------------------------------------------------------

pub struct FluxSubscriber {
    config: FluxConfig,
}

impl FluxSubscriber {
    pub fn from_env() -> Self {
        Self { config: FluxConfig::from_env() }
    }

    /// Poll Flux for entities matching agent peers.
    /// Returns a list of `RemoteMemorySignal` events to evaluate for pulling.
    pub fn poll_peer_signals(&self) -> Vec<RemoteMemorySignal> {
        if !self.config.is_enabled() {
            return Vec::new();
        }

        let url = format!("{}/api/state/entities", self.config.base_url);
        let resp = match ureq::get(&url).call() {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[flux] poll failed (non-fatal): {}", e);
                return Vec::new();
            }
        };

        let entities: serde_json::Value = match resp.into_json() {
            Ok(v) => v,
            Err(_) => return Vec::new(),
        };

        // Extract memory.stored events from peer entity properties
        let mut signals = Vec::new();
        if let Some(arr) = entities.as_array() {
            for entity in arr {
                let entity_id = entity["id"].as_str().unwrap_or("");
                if entity_id == self.config.agent_id {
                    continue; // skip ourselves
                }
                if let Some(props) = entity["properties"].as_object() {
                    if let (Some(mem_id), Some(amplitude), Some(summary)) = (
                        props.get("last_memory_id").and_then(|v| v.as_str()),
                        props.get("last_amplitude").and_then(|v| v.as_f64()),
                        props.get("last_summary").and_then(|v| v.as_str()),
                    ) {
                        signals.push(RemoteMemorySignal {
                            agent_id: entity_id.to_string(),
                            memory_id: mem_id.to_string(),
                            amplitude: amplitude as f32,
                            category: props.get("last_category")
                                .and_then(|v| v.as_str())
                                .unwrap_or("knowledge")
                                .to_string(),
                            tags: Vec::new(),
                            summary: summary.to_string(),
                            branch: format!("{}/working", entity_id),
                        });
                    }
                }
            }
        }

        signals
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn pull_decision_high_trust_high_amplitude() {
        let signal = RemoteMemorySignal {
            agent_id: "arc".to_string(),
            memory_id: Uuid::new_v4().to_string(),
            amplitude: 0.9,
            category: "knowledge".to_string(),
            tags: vec!["rust".to_string()],
            summary: "Rust lifetimes are ownership guarantees".to_string(),
            branch: "arc/working".to_string(),
        };
        assert_eq!(evaluate_pull(&signal, 0.9, None), PullDecision::Pull);
    }

    #[test]
    fn pull_decision_low_trust_skips() {
        let signal = RemoteMemorySignal {
            agent_id: "unknown".to_string(),
            memory_id: Uuid::new_v4().to_string(),
            amplitude: 0.9,
            category: "knowledge".to_string(),
            tags: Vec::new(),
            summary: "something important".to_string(),
            branch: "unknown/working".to_string(),
        };
        assert_eq!(evaluate_pull(&signal, 0.1, None), PullDecision::Skip);
    }

    #[test]
    fn pull_decision_topical_relevance() {
        let signal = RemoteMemorySignal {
            agent_id: "arc".to_string(),
            memory_id: Uuid::new_v4().to_string(),
            amplitude: 0.5,
            category: "knowledge".to_string(),
            tags: vec!["collective".to_string(), "memory".to_string()],
            summary: "collective memory merge algorithm".to_string(),
            branch: "arc/working".to_string(),
        };
        assert_eq!(evaluate_pull(&signal, 0.7, Some("collective memory")), PullDecision::Pull);
    }
}
