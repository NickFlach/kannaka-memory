//! Attention Field — session-scoped attention tracking over HRM wavefronts.
//!
//! The HRM IS the persistence layer. This module provides a transient
//! ring buffer for the current conversation window and structured
//! attention projection over the highest-energy wavefronts.
//!
//! No JSON persistence. No checkpoint-driven survival logic. The medium
//! is the sole truth store.

use std::collections::VecDeque;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::medium::Modality;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Task progress status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    InProgress,
    Blocked,
    WaitingOn,
    Done,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::InProgress => write!(f, "in-progress"),
            TaskStatus::Blocked => write!(f, "blocked"),
            TaskStatus::WaitingOn => write!(f, "waiting-on"),
            TaskStatus::Done => write!(f, "done"),
        }
    }
}

impl TaskStatus {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "blocked" => TaskStatus::Blocked,
            "waiting-on" | "waitingon" | "waiting" => TaskStatus::WaitingOn,
            "done" | "complete" | "completed" => TaskStatus::Done,
            _ => TaskStatus::InProgress,
        }
    }
}

/// A single conversation turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub embedding: Option<Vec<f32>>,
}

/// A tracked task item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskItem {
    pub description: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
}

/// Structured rolling session state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub active_tasks: Vec<TaskItem>,
    pub pending_questions: Vec<String>,
    pub waiting_on: Vec<String>,
    pub conversation_summary: String,
    pub last_updated: DateTime<Utc>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            active_tasks: Vec::new(),
            pending_questions: Vec::new(),
            waiting_on: Vec::new(),
            conversation_summary: String::new(),
            last_updated: Utc::now(),
        }
    }
}

/// HRM-native attention projection -- returns highest-energy wavefronts
/// instead of formatted markdown text.
pub struct AttentionProjection {
    pub active_wavefronts: Vec<(Uuid, f32, String)>,  // (id, energy, content_preview)
    pub dominant_modality: Modality,
    pub attention_coherence: f32,
    pub recent_switch_points: usize,
}

// ---------------------------------------------------------------------------
// AttentionField (formerly WorkingMemory)
// ---------------------------------------------------------------------------

const DEFAULT_MAX_TURNS: usize = 50;
const AUTO_SUMMARY_INTERVAL: usize = 10;

pub struct AttentionField {
    turns: VecDeque<ConversationTurn>,
    session_state: SessionState,
    max_turns: usize,
    ollama_url: Option<String>,
    summary_model: String,
    /// Tracks turns since last auto-summary.
    turns_since_summary: usize,
}

impl AttentionField {
    /// Create a new empty attention field.
    pub fn new(ollama_url: Option<String>, summary_model: Option<String>) -> Self {
        Self {
            turns: VecDeque::with_capacity(DEFAULT_MAX_TURNS),
            session_state: SessionState::default(),
            max_turns: DEFAULT_MAX_TURNS,
            ollama_url,
            summary_model: summary_model.unwrap_or_else(|| "phi3:mini".to_string()),
            turns_since_summary: 0,
        }
    }

    /// Create with a custom max-turns limit.
    pub fn with_max_turns(mut self, max: usize) -> Self {
        self.max_turns = max;
        self
    }

    // ------------------------------------------------------------------
    // Turn management
    // ------------------------------------------------------------------

    /// Add a conversation turn. Evicts oldest if ring buffer is full.
    /// Triggers auto-summary every `AUTO_SUMMARY_INTERVAL` turns.
    pub fn add_turn(&mut self, role: &str, content: &str) {
        let turn = ConversationTurn {
            id: Uuid::new_v4(),
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            embedding: None,
        };
        if self.turns.len() >= self.max_turns {
            self.turns.pop_front();
        }
        self.turns.push_back(turn);
        self.turns_since_summary += 1;

        if self.turns_since_summary >= AUTO_SUMMARY_INTERVAL {
            self.summarize();
            self.turns_since_summary = 0;
        }
    }

    /// Number of turns currently stored.
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Iterate over turns (oldest first).
    pub fn turns(&self) -> impl Iterator<Item = &ConversationTurn> {
        self.turns.iter()
    }

    // ------------------------------------------------------------------
    // Task management
    // ------------------------------------------------------------------

    /// Add or update a task. If a task with the same description exists, update its status.
    pub fn update_task(&mut self, description: &str, status: TaskStatus) {
        if let Some(task) = self.session_state.active_tasks.iter_mut().find(|t| t.description == description) {
            task.status = status;
        } else {
            self.session_state.active_tasks.push(TaskItem {
                description: description.to_string(),
                status,
                created_at: Utc::now(),
            });
        }
        self.session_state.last_updated = Utc::now();
    }

    /// Remove all tasks with status `Done`.
    pub fn clear_completed(&mut self) {
        self.session_state.active_tasks.retain(|t| t.status != TaskStatus::Done);
        self.session_state.last_updated = Utc::now();
    }

    /// Access the current session state.
    pub fn session_state(&self) -> &SessionState {
        &self.session_state
    }

    // ------------------------------------------------------------------
    // Summarization
    // ------------------------------------------------------------------

    /// Produce a rolling summary. Tries Ollama first, falls back to extractive.
    pub fn summarize(&mut self) {
        if let Some(ref url) = self.ollama_url {
            if let Some(summary) = self.try_ollama_summary(url) {
                self.session_state.conversation_summary = summary;
                self.session_state.last_updated = Utc::now();
                return;
            }
        }
        self.extractive_summary();
    }

    /// Build an extractive (fallback) summary from recent turns.
    fn extractive_summary(&mut self) {
        let recent: Vec<&ConversationTurn> = self.turns.iter().rev().take(5).collect();
        let mut parts: Vec<String> = Vec::new();
        // Reverse so oldest-first
        for turn in recent.into_iter().rev() {
            let preview = if turn.content.len() > 200 {
                format!("{}…", &turn.content[..turn.content.floor_char_boundary(200)])
            } else {
                turn.content.clone()
            };
            parts.push(format!("[{}] {}", turn.role, preview));
        }

        // Extract task-like lines by keyword
        for turn in self.turns.iter().rev().take(20) {
            let lower = turn.content.to_lowercase();
            if lower.contains("todo") || lower.contains("task") || lower.contains("need to") || lower.contains("should") {
                let existing: Vec<&str> = self.session_state.active_tasks.iter().map(|t| t.description.as_str()).collect();
                let preview = if turn.content.len() > 120 { &turn.content[..turn.content.floor_char_boundary(120)] } else { &turn.content };
                if !existing.iter().any(|e| e == &preview) {
                    // Don't auto-add; just note it in summary
                    parts.push(format!("[task-hint] {}", preview));
                }
            }
        }

        self.session_state.conversation_summary = parts.join("\n");
        self.session_state.last_updated = Utc::now();
    }

    /// Try to summarize via Ollama. Returns None on any failure.
    fn try_ollama_summary(&self, base_url: &str) -> Option<String> {
        let recent: Vec<String> = self.turns.iter().rev().take(15).rev().map(|t| {
            format!("[{}] {}", t.role, t.content)
        }).collect();

        let conversation = recent.join("\n");
        let prompt = format!(
            "Summarize this conversation concisely. Focus on: what was discussed, \
             any decisions made, open questions, and pending tasks.\n\n\
             Conversation:\n{}\n\nSummary:",
            conversation
        );

        let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.summary_model,
            "prompt": prompt,
            "stream": false,
        });

        let resp = ureq::post(&url)
            .timeout(std::time::Duration::from_secs(30))
            .send_json(&body)
            .ok()?;

        let json: serde_json::Value = resp.into_json().ok()?;
        json.get("response").and_then(|v| v.as_str()).map(|s| s.trim().to_string())
    }

    // ------------------------------------------------------------------
    // HRM-native attention projection
    // ------------------------------------------------------------------

    /// Project attention over the HRM store -- returns highest-energy wavefronts
    /// as structured data instead of formatted markdown.
    pub fn project_attention(&self, store: &dyn crate::store::MediumBackend) -> AttentionProjection {
        let mut wavefronts = Vec::new();

        if let Ok(all) = store.all_memories() {
            // Sort by amplitude (energy) descending, take top N
            let mut sorted: Vec<&crate::memory::HyperMemory> = all.into_iter().collect();
            sorted.sort_by(|a, b| b.amplitude.total_cmp(&a.amplitude));

            for mem in sorted.into_iter().take(10) {
                let preview = if mem.content.len() > 200 {
                    format!("{}...", &mem.content[..mem.content.floor_char_boundary(200)])
                } else {
                    mem.content.clone()
                };
                wavefronts.push((mem.id, mem.amplitude, preview));
            }
        }

        // Determine dominant modality from top wavefronts
        let dominant_modality = if wavefronts.is_empty() {
            Modality::Unknown
        } else {
            // Count modalities in top wavefronts
            let mut counts = std::collections::HashMap::new();
            if let Ok(all) = store.all_memories() {
                let mut sorted: Vec<&crate::memory::HyperMemory> = all.into_iter().collect();
                sorted.sort_by(|a, b| b.amplitude.total_cmp(&a.amplitude));
                for mem in sorted.into_iter().take(10) {
                    *counts.entry(mem.modality).or_insert(0u32) += 1;
                }
            }
            counts.into_iter()
                .max_by_key(|(_, c)| *c)
                .map(|(m, _)| m)
                .unwrap_or(Modality::Unknown)
        };

        // Compute attention coherence from amplitude variance
        let attention_coherence = if wavefronts.len() >= 2 {
            let energies: Vec<f32> = wavefronts.iter().map(|(_, e, _)| *e).collect();
            let mean = energies.iter().sum::<f32>() / energies.len() as f32;
            let variance = energies.iter().map(|e| (e - mean).powi(2)).sum::<f32>() / energies.len() as f32;
            // Low variance = high coherence (attention focused)
            1.0 / (1.0 + variance)
        } else {
            0.0
        };

        AttentionProjection {
            active_wavefronts: wavefronts,
            dominant_modality,
            attention_coherence,
            recent_switch_points: 0, // TODO: derive from NCS switch detection
        }
    }
}

/// Backward-compatible type alias.
pub type WorkingMemory = AttentionField;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_af() -> AttentionField {
        AttentionField::new(None, None)
    }

    #[test]
    fn ring_buffer_overflow() {
        let mut af = make_af().with_max_turns(3);
        af.add_turn("user", "one");
        af.add_turn("assistant", "two");
        af.add_turn("user", "three");
        assert_eq!(af.turn_count(), 3);

        af.add_turn("assistant", "four");
        assert_eq!(af.turn_count(), 3);
        // Oldest ("one") should be gone
        let contents: Vec<&str> = af.turns().map(|t| t.content.as_str()).collect();
        assert_eq!(contents, vec!["two", "three", "four"]);
    }

    #[test]
    fn turn_logging() {
        let mut af = make_af();
        af.add_turn("user", "hello");
        af.add_turn("assistant", "hi there");

        assert_eq!(af.turn_count(), 2);
        let first = af.turns().next().unwrap();
        assert_eq!(first.role, "user");
        assert_eq!(first.content, "hello");
    }

    #[test]
    fn task_management() {
        let mut af = make_af();
        af.update_task("build feature", TaskStatus::InProgress);
        af.update_task("write tests", TaskStatus::InProgress);
        assert_eq!(af.session_state().active_tasks.len(), 2);

        // Update existing
        af.update_task("build feature", TaskStatus::Done);
        assert_eq!(af.session_state().active_tasks[0].status, TaskStatus::Done);

        // Clear completed
        af.clear_completed();
        assert_eq!(af.session_state().active_tasks.len(), 1);
        assert_eq!(af.session_state().active_tasks[0].description, "write tests");
    }

    #[test]
    fn extractive_summary_fallback() {
        let mut af = make_af();
        af.add_turn("user", "Can you help me with Rust?");
        af.add_turn("assistant", "Sure! What do you need?");
        af.add_turn("user", "I need to build a ring buffer");

        af.summarize();
        let summary = &af.session_state().conversation_summary;
        assert!(!summary.is_empty());
        assert!(summary.contains("ring buffer") || summary.contains("Rust"));
    }

    #[test]
    fn attention_projection_empty_store() {
        let af = make_af();
        let store = crate::store::TestMedium::new();
        let proj = af.project_attention(&store);
        assert!(proj.active_wavefronts.is_empty());
        assert_eq!(proj.attention_coherence, 0.0);
    }

    #[test]
    fn auto_summary_triggers() {
        let mut af = make_af();
        // Add AUTO_SUMMARY_INTERVAL turns to trigger auto-summary
        for i in 0..AUTO_SUMMARY_INTERVAL {
            af.add_turn("user", &format!("message {}", i));
        }
        // Summary should have been triggered
        assert!(!af.session_state().conversation_summary.is_empty());
    }
}
