//! Working Memory — L2 conversation context layer.
//!
//! Maintains a ring buffer of conversation turns and rolling session state
//! that survives session compactions. Persists to both a fast JSON file
//! and periodic HyperMemory checkpoints in the main store.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::memory::HyperMemory;
use crate::store::MemoryEngine;

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

/// Serializable snapshot of the full working memory (for JSON persistence).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkingMemorySnapshot {
    turns: Vec<ConversationTurn>,
    session_state: SessionState,
    max_turns: usize,
    last_checkpoint: Option<DateTime<Utc>>,
    summary_model: String,
}

// ---------------------------------------------------------------------------
// WorkingMemory
// ---------------------------------------------------------------------------

const DEFAULT_MAX_TURNS: usize = 50;
const AUTO_SUMMARY_INTERVAL: usize = 10;
const SESSION_STATE_TAG: &str = "session-state";

pub struct WorkingMemory {
    turns: VecDeque<ConversationTurn>,
    session_state: SessionState,
    max_turns: usize,
    last_checkpoint: Option<DateTime<Utc>>,
    ollama_url: Option<String>,
    summary_model: String,
    /// Tracks turns since last auto-summary.
    turns_since_summary: usize,
}

impl WorkingMemory {
    /// Create a new empty working memory.
    pub fn new(ollama_url: Option<String>, summary_model: Option<String>) -> Self {
        Self {
            turns: VecDeque::with_capacity(DEFAULT_MAX_TURNS),
            session_state: SessionState::default(),
            max_turns: DEFAULT_MAX_TURNS,
            last_checkpoint: None,
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
                format!("{}…", &turn.content[..200])
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
                let preview = if turn.content.len() > 120 { &turn.content[..120] } else { &turn.content };
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
    // Context output
    // ------------------------------------------------------------------

    /// Format current state as a context block for prompt injection.
    pub fn get_context(&self) -> String {
        let mut out = String::new();
        out.push_str("## Working Memory Context\n\n");

        // Summary
        if !self.session_state.conversation_summary.is_empty() {
            out.push_str("### Conversation Summary\n");
            out.push_str(&self.session_state.conversation_summary);
            out.push_str("\n\n");
        }

        // Active tasks
        if !self.session_state.active_tasks.is_empty() {
            out.push_str("### Active Tasks\n");
            for task in &self.session_state.active_tasks {
                out.push_str(&format!("- [{}] {}\n", task.status, task.description));
            }
            out.push('\n');
        }

        // Pending questions
        if !self.session_state.pending_questions.is_empty() {
            out.push_str("### Pending Questions\n");
            for q in &self.session_state.pending_questions {
                out.push_str(&format!("- {}\n", q));
            }
            out.push('\n');
        }

        // Waiting on
        if !self.session_state.waiting_on.is_empty() {
            out.push_str("### Waiting On\n");
            for w in &self.session_state.waiting_on {
                out.push_str(&format!("- {}\n", w));
            }
            out.push('\n');
        }

        // Recent turns (last 5)
        let recent: Vec<&ConversationTurn> = self.turns.iter().rev().take(5).collect();
        if !recent.is_empty() {
            out.push_str("### Recent Turns\n");
            for turn in recent.into_iter().rev() {
                let preview = if turn.content.len() > 300 {
                    format!("{}…", &turn.content[..300])
                } else {
                    turn.content.clone()
                };
                out.push_str(&format!("**{}**: {}\n", turn.role, preview));
            }
        }

        out
    }

    // ------------------------------------------------------------------
    // Persistence — JSON fast path
    // ------------------------------------------------------------------

    fn json_path(data_dir: &Path) -> PathBuf {
        data_dir.join("working_memory.json")
    }

    /// Save to `working_memory.json` in the given data directory.
    pub fn save_json(&self, data_dir: &Path) -> Result<(), std::io::Error> {
        let snapshot = WorkingMemorySnapshot {
            turns: self.turns.iter().cloned().collect(),
            session_state: self.session_state.clone(),
            max_turns: self.max_turns,
            last_checkpoint: self.last_checkpoint,
            summary_model: self.summary_model.clone(),
        };
        let json = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(Self::json_path(data_dir), json)
    }

    /// Load from `working_memory.json`. Returns None if file doesn't exist or is corrupt.
    pub fn load_json(data_dir: &Path, ollama_url: Option<String>) -> Option<Self> {
        let path = Self::json_path(data_dir);
        let data = std::fs::read_to_string(&path).ok()?;
        let snap: WorkingMemorySnapshot = serde_json::from_str(&data).ok()?;
        Some(Self {
            turns: snap.turns.into_iter().collect(),
            session_state: snap.session_state,
            max_turns: snap.max_turns,
            last_checkpoint: snap.last_checkpoint,
            ollama_url,
            summary_model: snap.summary_model,
            turns_since_summary: 0,
        })
    }

    // ------------------------------------------------------------------
    // Persistence — HyperMemory checkpoint (safety net)
    // ------------------------------------------------------------------

    /// Checkpoint: saves JSON + stores a high-amplitude HyperMemory tagged "session-state".
    pub fn checkpoint(&mut self, data_dir: &Path, engine: &mut MemoryEngine) -> Result<(), std::io::Error> {
        // 1. Save JSON (fast path)
        self.save_json(data_dir)?;

        // 2. Build checkpoint content
        let checkpoint_content = self.get_context();
        let tagged_content = format!("[{}] {}", SESSION_STATE_TAG, checkpoint_content);

        // 3. Store as high-amplitude memory in engine
        let store_result = engine.remember(&tagged_content);
        if let Ok(id) = store_result {
            if let Ok(Some(mem)) = engine.get_memory_mut(&id) {
                mem.amplitude = 2.0; // High amplitude so it survives consolidation
                mem.layer_depth = 2; // Mark as higher layer
            }
        }

        self.last_checkpoint = Some(Utc::now());
        Ok(())
    }

    /// Restore working memory. Tries JSON first, then searches engine for session-state memories.
    pub fn restore(data_dir: &Path, engine: &MemoryEngine, ollama_url: Option<String>) -> Self {
        // Fast path: JSON file
        if let Some(wm) = Self::load_json(data_dir, ollama_url.clone()) {
            return wm;
        }

        // Fallback: search engine for session-state tagged memories
        let mut wm = Self::new(ollama_url, None);

        // Look for the most recent session-state memory
        if let Ok(results) = engine.store.all_memories() {
            let mut session_mems: Vec<&&HyperMemory> = results.iter()
                .filter(|m| m.content.starts_with(&format!("[{}]", SESSION_STATE_TAG)))
                .collect();
            session_mems.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            if let Some(mem) = session_mems.first() {
                // Parse the summary back into session state
                let content = mem.content.strip_prefix(&format!("[{}] ", SESSION_STATE_TAG)).unwrap_or(&mem.content);
                wm.session_state.conversation_summary = content.to_string();
                wm.session_state.last_updated = mem.created_at;
            }
        }

        wm
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_wm() -> WorkingMemory {
        WorkingMemory::new(None, None)
    }

    #[test]
    fn ring_buffer_overflow() {
        let mut wm = make_wm().with_max_turns(3);
        wm.add_turn("user", "one");
        wm.add_turn("assistant", "two");
        wm.add_turn("user", "three");
        assert_eq!(wm.turn_count(), 3);

        wm.add_turn("assistant", "four");
        assert_eq!(wm.turn_count(), 3);
        // Oldest ("one") should be gone
        let contents: Vec<&str> = wm.turns().map(|t| t.content.as_str()).collect();
        assert_eq!(contents, vec!["two", "three", "four"]);
    }

    #[test]
    fn turn_logging() {
        let mut wm = make_wm();
        wm.add_turn("user", "hello");
        wm.add_turn("assistant", "hi there");

        assert_eq!(wm.turn_count(), 2);
        let first = wm.turns().next().unwrap();
        assert_eq!(first.role, "user");
        assert_eq!(first.content, "hello");
    }

    #[test]
    fn task_management() {
        let mut wm = make_wm();
        wm.update_task("build feature", TaskStatus::InProgress);
        wm.update_task("write tests", TaskStatus::InProgress);
        assert_eq!(wm.session_state().active_tasks.len(), 2);

        // Update existing
        wm.update_task("build feature", TaskStatus::Done);
        assert_eq!(wm.session_state().active_tasks[0].status, TaskStatus::Done);

        // Clear completed
        wm.clear_completed();
        assert_eq!(wm.session_state().active_tasks.len(), 1);
        assert_eq!(wm.session_state().active_tasks[0].description, "write tests");
    }

    #[test]
    fn extractive_summary_fallback() {
        let mut wm = make_wm();
        wm.add_turn("user", "Can you help me with Rust?");
        wm.add_turn("assistant", "Sure! What do you need?");
        wm.add_turn("user", "I need to build a ring buffer");

        wm.summarize();
        let summary = &wm.session_state().conversation_summary;
        assert!(!summary.is_empty());
        assert!(summary.contains("ring buffer") || summary.contains("Rust"));
    }

    #[test]
    fn checkpoint_restore_roundtrip() {
        use crate::encoding::{EncodingPipeline, SimpleHashEncoder};
        use crate::codebook::Codebook;
        use crate::store::{MemoryEngine, InMemoryStore};

        let dir = std::env::temp_dir().join(format!("kannaka_wm_test_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let encoder = SimpleHashEncoder::new(384, 42);
        let codebook = Codebook::new(384, 10_000, 42);
        let pipeline = EncodingPipeline::new(Box::new(encoder), codebook);
        let mut engine = MemoryEngine::new(Box::new(InMemoryStore::new()), pipeline);

        let mut wm = WorkingMemory::new(None, None);
        wm.add_turn("user", "checkpoint test");
        wm.update_task("test task", TaskStatus::InProgress);
        wm.checkpoint(&dir, &mut engine).unwrap();

        // Restore from JSON
        let wm2 = WorkingMemory::restore(&dir, &engine, None);
        assert_eq!(wm2.turn_count(), 1);
        assert_eq!(wm2.session_state().active_tasks.len(), 1);
        assert_eq!(wm2.session_state().active_tasks[0].description, "test task");

        // Delete JSON, restore from engine fallback
        std::fs::remove_file(dir.join("working_memory.json")).unwrap();
        let wm3 = WorkingMemory::restore(&dir, &engine, None);
        // Should have recovered summary from engine
        assert!(!wm3.session_state().conversation_summary.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn get_context_formatting() {
        let mut wm = make_wm();
        wm.add_turn("user", "hello world");
        wm.update_task("do stuff", TaskStatus::InProgress);
        wm.session_state.pending_questions.push("what about X?".to_string());

        let ctx = wm.get_context();
        assert!(ctx.contains("Working Memory Context"));
        assert!(ctx.contains("do stuff"));
        assert!(ctx.contains("hello world"));
        assert!(ctx.contains("what about X?"));
    }

    #[test]
    fn auto_summary_triggers() {
        let mut wm = make_wm();
        // Add AUTO_SUMMARY_INTERVAL turns to trigger auto-summary
        for i in 0..AUTO_SUMMARY_INTERVAL {
            wm.add_turn("user", &format!("message {}", i));
        }
        // Summary should have been triggered
        assert!(!wm.session_state().conversation_summary.is_empty());
    }
}
