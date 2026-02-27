//! MCP tools implementation for Kannaka Memory operations

use serde_json::{json, Value};
use uuid::Uuid;
use chrono::Utc;

use crate::openclaw::KannakaMemorySystem;
use super::bm25::Bm25Index;
use super::retrieval::rrf_fuse;
use super::protocol::{ToolDefinition, ToolResult, ToolCallParams};

pub struct McpToolSet {
    system: KannakaMemorySystem,
    bm25_index: Bm25Index,
    ollama_url: String,
    ollama_model: String,
}

impl McpToolSet {
    pub fn new(
        system: KannakaMemorySystem,
        ollama_url: String,
        ollama_model: String,
    ) -> Self {
        Self {
            system,
            bm25_index: Bm25Index::new(),
            ollama_url,
            ollama_model,
        }
    }

    pub fn get_tool_definitions() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "store_memory".to_string(),
                description: "Store text as a new memory with automatic embedding".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "content": {"type": "string", "description": "Text content to store"},
                        "metadata": {"type": "object", "description": "Optional metadata", "additionalProperties": true}
                    },
                    "required": ["content"]
                }),
            },
            ToolDefinition {
                name: "search".to_string(),
                description: "Unified search combining semantic, keyword, and recency via RRF fusion".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "limit": {"type": "integer", "description": "Maximum results", "default": 10},
                        "include_metadata": {"type": "boolean", "description": "Include memory metadata", "default": false}
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "search_semantic".to_string(),
                description: "Pure semantic vector similarity search".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "limit": {"type": "integer", "description": "Maximum results", "default": 10}
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "search_keyword".to_string(),
                description: "BM25 keyword-based search".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "limit": {"type": "integer", "description": "Maximum results", "default": 10}
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "search_recent".to_string(),
                description: "Search within a temporal window".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string", "description": "Search query"},
                        "hours": {"type": "number", "description": "Time window in hours", "default": 24.0},
                        "limit": {"type": "integer", "description": "Maximum results", "default": 10}
                    },
                    "required": ["query"]
                }),
            },
            ToolDefinition {
                name: "forget".to_string(),
                description: "Decay or remove a memory by ID".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "memory_id": {"type": "string", "description": "UUID of memory to forget"},
                        "decay_factor": {"type": "number", "description": "Decay factor (0.0-1.0), 0.0 = complete removal", "default": 0.5}
                    },
                    "required": ["memory_id"]
                }),
            },
            ToolDefinition {
                name: "boost".to_string(),
                description: "Increase wave amplitude for a memory".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "memory_id": {"type": "string", "description": "UUID of memory to boost"},
                        "boost_factor": {"type": "number", "description": "Boost multiplier", "default": 2.0}
                    },
                    "required": ["memory_id"]
                }),
            },
            ToolDefinition {
                name: "relate".to_string(),
                description: "Create typed relationship between memories".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "source_id": {"type": "string", "description": "Source memory UUID"},
                        "target_id": {"type": "string", "description": "Target memory UUID"},
                        "relationship": {"type": "string", "description": "Relationship type", "default": "related"},
                        "strength": {"type": "number", "description": "Relationship strength", "default": 1.0}
                    },
                    "required": ["source_id", "target_id"]
                }),
            },
            ToolDefinition {
                name: "find_related".to_string(),
                description: "Traverse memory graph from a starting memory".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "memory_id": {"type": "string", "description": "Starting memory UUID"},
                        "max_depth": {"type": "integer", "description": "Maximum traversal depth", "default": 2},
                        "limit": {"type": "integer", "description": "Maximum results", "default": 20}
                    },
                    "required": ["memory_id"]
                }),
            },
            ToolDefinition {
                name: "dream".to_string(),
                description: "Trigger memory consolidation cycle".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "max_cycles": {"type": "integer", "description": "Maximum consolidation cycles", "default": 10}
                    }
                }),
            },
            ToolDefinition {
                name: "status".to_string(),
                description: "Get wave states, memory health, and consciousness metrics".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "detailed": {"type": "boolean", "description": "Include detailed metrics", "default": false}
                    }
                }),
            },
            ToolDefinition {
                name: "hallucinate".to_string(),
                description: "Generate a hallucinated memory from parent memories using LLM synthesis".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "content": {"type": "string", "description": "LLM-generated synthesis content"},
                        "parent_ids": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Parent memory UUIDs to synthesize from"
                        }
                    },
                    "required": ["content", "parent_ids"]
                }),
            },
            ToolDefinition {
                name: "rhythm_status".to_string(),
                description: "Get adaptive rhythm state: arousal level, current interval, momentum".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolDefinition {
                name: "rhythm_signal".to_string(),
                description: "Send a signal to the adaptive rhythm engine".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "signal": {
                            "type": "string",
                            "enum": ["user_message", "flux_message", "subagent_started", "subagent_finished", "idle"],
                            "description": "Signal type"
                        }
                    },
                    "required": ["signal"]
                }),
            },
            ToolDefinition {
                name: "context_save".to_string(),
                description: "Checkpoint current working memory / session state".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolDefinition {
                name: "context_restore".to_string(),
                description: "Get the most recent session state as formatted text".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolDefinition {
                name: "context_turn".to_string(),
                description: "Log a conversation turn into working memory".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "role": {"type": "string", "description": "Role: user, assistant, or system"},
                        "content": {"type": "string", "description": "Turn content"}
                    },
                    "required": ["role", "content"]
                }),
            },
            ToolDefinition {
                name: "context_summary".to_string(),
                description: "Get the current rolling conversation summary and context".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolDefinition {
                name: "context_task".to_string(),
                description: "Add or update a task in working memory".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "description": {"type": "string", "description": "Task description"},
                        "status": {"type": "string", "enum": ["in-progress", "blocked", "waiting-on", "done"], "description": "Task status", "default": "in-progress"}
                    },
                    "required": ["description"]
                }),
            },
            ToolDefinition {
                name: "observe".to_string(),
                description: "Introspection on memory patterns and system health".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "include_topology": {"type": "boolean", "description": "Include network topology", "default": true},
                        "include_waves": {"type": "boolean", "description": "Include wave analysis", "default": true}
                    }
                }),
            },
        ]
    }

    pub fn handle_tool_call(&mut self, params: ToolCallParams) -> ToolResult {
        let args = params.arguments.unwrap_or(json!({}));
        
        match params.name.as_str() {
            "store_memory" => self.store_memory(&args),
            "search" => self.search(&args),
            "search_semantic" => self.search_semantic(&args),
            "search_keyword" => self.search_keyword(&args),
            "search_recent" => self.search_recent(&args),
            "forget" => self.forget(&args),
            "boost" => self.boost(&args),
            "relate" => self.relate(&args),
            "find_related" => self.find_related(&args),
            "dream" => self.dream(&args),
            "hallucinate" => self.hallucinate(&args),
            "rhythm_status" => self.rhythm_status(&args),
            "rhythm_signal" => self.rhythm_signal(&args),
            "context_save" => self.context_save(&args),
            "context_restore" => self.context_restore(&args),
            "context_turn" => self.context_turn(&args),
            "context_summary" => self.context_summary(&args),
            "context_task" => self.context_task(&args),
            "status" => self.status(&args),
            "observe" => self.observe(&args),
            _ => ToolResult::error(format!("Unknown tool: {}", params.name)),
        }
    }

    fn store_memory(&mut self, args: &Value) -> ToolResult {
        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing 'content' parameter".to_string()),
        };

        match self.system.remember(content) {
            Ok(id) => {
                // Add to BM25 index
                self.bm25_index.add_document(id, content);
                
                ToolResult::success(format!("Stored memory with ID: {}", id))
            }
            Err(e) => ToolResult::error(format!("Failed to store memory: {}", e)),
        }
    }

    fn search(&mut self, args: &Value) -> ToolResult {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return ToolResult::error("Missing 'query' parameter".to_string()),
        };

        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        // Get results from different search methods
        let semantic_results = match self.system.recall(query, limit * 2) {
            Ok(results) => results.into_iter().map(|r| (r.id, r.similarity)).collect::<Vec<_>>(),
            Err(_) => Vec::new(),
        };

        let keyword_results = self.bm25_index.search(query, limit * 2);

        // For recency, get recent memories and filter by query similarity
        let recent_results = match self.system.recall(query, limit * 3) {
            Ok(results) => {
                let _now = Utc::now();
                results.into_iter()
                    .filter(|r| r.age_hours < 24.0) // Last 24 hours
                    .map(|r| (r.id, (1.0 / (r.age_hours + 1.0)) as f32)) // Higher score for more recent
                    .collect::<Vec<_>>()
            }
            Err(_) => Vec::new(),
        };

        // Fuse results using RRF
        let all_results = vec![semantic_results, keyword_results, recent_results];
        let fused = rrf_fuse(&all_results, 60.0);

        // Get detailed results for top matches
        let top_ids: Vec<Uuid> = fused.iter().take(limit).map(|(id, _)| *id).collect();
        let detailed_results = match self.system.recall(query, limit * 3) {
            Ok(results) => {
                results.into_iter()
                    .filter(|r| top_ids.contains(&r.id))
                    .collect::<Vec<_>>()
            }
            Err(e) => return ToolResult::error(format!("Search failed: {}", e)),
        };

        let mut response = String::new();
        response.push_str(&format!("Found {} results:\n\n", detailed_results.len()));

        for (i, result) in detailed_results.iter().enumerate() {
            response.push_str(&format!(
                "{}. [sim={:.3} str={:.3} age={:.1}h L{}] {}\n   ID: {}\n\n",
                i + 1, result.similarity, result.strength, result.age_hours, result.layer, result.content, result.id
            ));
        }

        ToolResult::success(response)
    }

    fn search_semantic(&mut self, args: &Value) -> ToolResult {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return ToolResult::error("Missing 'query' parameter".to_string()),
        };

        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        match self.system.recall(query, limit) {
            Ok(results) => {
                let mut response = String::new();
                response.push_str(&format!("Semantic search found {} results:\n\n", results.len()));

                for (i, result) in results.iter().enumerate() {
                    response.push_str(&format!(
                        "{}. [sim={:.3} str={:.3}] {}\n   ID: {}\n\n",
                        i + 1, result.similarity, result.strength, result.content, result.id
                    ));
                }

                ToolResult::success(response)
            }
            Err(e) => ToolResult::error(format!("Semantic search failed: {}", e)),
        }
    }

    fn search_keyword(&mut self, args: &Value) -> ToolResult {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return ToolResult::error("Missing 'query' parameter".to_string()),
        };

        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let results = self.bm25_index.search(query, limit);

        let mut response = String::new();
        response.push_str(&format!("Keyword search found {} results:\n\n", results.len()));

        for (i, (id, score)) in results.iter().enumerate() {
            response.push_str(&format!(
                "{}. [BM25={:.3}] ID: {}\n\n",
                i + 1, score, id
            ));
        }

        ToolResult::success(response)
    }

    fn search_recent(&mut self, args: &Value) -> ToolResult {
        let query = match args.get("query").and_then(|v| v.as_str()) {
            Some(q) => q,
            None => return ToolResult::error("Missing 'query' parameter".to_string()),
        };

        let hours = args.get("hours").and_then(|v| v.as_f64()).unwrap_or(24.0);
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

        match self.system.recall(query, limit * 3) {
            Ok(results) => {
                let recent_results: Vec<_> = results.into_iter()
                    .filter(|r| r.age_hours <= hours)
                    .take(limit)
                    .collect();

                let mut response = String::new();
                response.push_str(&format!("Recent search ({}h window) found {} results:\n\n", hours, recent_results.len()));

                for (i, result) in recent_results.iter().enumerate() {
                    response.push_str(&format!(
                        "{}. [age={:.1}h sim={:.3}] {}\n   ID: {}\n\n",
                        i + 1, result.age_hours, result.similarity, result.content, result.id
                    ));
                }

                ToolResult::success(response)
            }
            Err(e) => ToolResult::error(format!("Recent search failed: {}", e)),
        }
    }

    fn forget(&mut self, args: &Value) -> ToolResult {
        let memory_id_str = match args.get("memory_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::error("Missing 'memory_id' parameter".to_string()),
        };

        let memory_id = match Uuid::parse_str(memory_id_str) {
            Ok(id) => id,
            Err(_) => return ToolResult::error("Invalid memory_id format".to_string()),
        };

        let _decay_factor = args.get("decay_factor").and_then(|v| v.as_f64()).unwrap_or(0.5);

        // Remove from BM25 index
        self.bm25_index.remove_document(&memory_id);

        // Delete from memory store
        match self.system.forget(&memory_id) {
            Ok(true) => ToolResult::success(format!("Memory {} forgotten", memory_id)),
            Ok(false) => ToolResult::error(format!("Memory {} not found", memory_id)),
            Err(e) => ToolResult::error(format!("Failed to forget: {}", e)),
        }
    }

    fn boost(&mut self, args: &Value) -> ToolResult {
        let memory_id_str = match args.get("memory_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::error("Missing 'memory_id' parameter".to_string()),
        };

        let memory_id = match Uuid::parse_str(memory_id_str) {
            Ok(id) => id,
            Err(_) => return ToolResult::error("Invalid memory_id format".to_string()),
        };

        let boost_factor = args.get("boost_factor").and_then(|v| v.as_f64()).unwrap_or(2.0);

        match self.system.boost(&memory_id, boost_factor) {
            Ok(()) => ToolResult::success(format!("Memory {} boosted by {:.1}x", memory_id, boost_factor)),
            Err(e) => ToolResult::error(format!("Failed to boost: {}", e)),
        }
    }

    fn relate(&mut self, args: &Value) -> ToolResult {
        let source_id = match args.get("source_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::error("Missing 'source_id' parameter".to_string()),
        };

        let target_id = match args.get("target_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::error("Missing 'target_id' parameter".to_string()),
        };

        let relationship = args.get("relationship").and_then(|v| v.as_str()).unwrap_or("related");
        let strength = args.get("strength").and_then(|v| v.as_f64()).unwrap_or(1.0);

        let src = match Uuid::parse_str(source_id) {
            Ok(id) => id,
            Err(_) => return ToolResult::error("Invalid source_id format".to_string()),
        };
        let tgt = match Uuid::parse_str(target_id) {
            Ok(id) => id,
            Err(_) => return ToolResult::error("Invalid target_id format".to_string()),
        };

        match self.system.relate(&src, &tgt, strength as f32) {
            Ok(()) => ToolResult::success(format!(
                "Created relationship '{}' from {} to {} (strength: {})",
                relationship, source_id, target_id, strength
            )),
            Err(e) => ToolResult::error(format!("Failed to relate: {}", e)),
        }
    }

    fn find_related(&mut self, args: &Value) -> ToolResult {
        let memory_id_str = match args.get("memory_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return ToolResult::error("Missing 'memory_id' parameter".to_string()),
        };

        let _memory_id = match Uuid::parse_str(memory_id_str) {
            Ok(id) => id,
            Err(_) => return ToolResult::error("Invalid memory_id format".to_string()),
        };

        let max_depth = args.get("max_depth").and_then(|v| v.as_u64()).unwrap_or(2);
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20);

        ToolResult::success(format!(
            "Would traverse memory graph from {} (max_depth: {}, limit: {})\nNote: Graph traversal not yet implemented",
            memory_id_str, max_depth, limit
        ))
    }

    fn dream(&mut self, args: &Value) -> ToolResult {
        let _max_cycles = args.get("max_cycles").and_then(|v| v.as_u64()).unwrap_or(10);

        match self.system.dream() {
            Ok(report) => {
                let response = format!(
                    "Dream cycle completed:\n\
                     - Cycles: {}\n\
                     - Memories strengthened: {}\n\
                     - Memories pruned: {}\n\
                     - New connections: {}\n\
                     - Hallucinations created: {}\n\
                     - Consciousness: {} → {}\n\
                     {}",
                    report.cycles,
                    report.memories_strengthened,
                    report.memories_pruned,
                    report.new_connections,
                    report.hallucinations_created,
                    report.consciousness_before,
                    report.consciousness_after,
                    if report.emerged { "✨ Emergence detected!" } else { "" }
                );
                ToolResult::success(response)
            }
            Err(e) => ToolResult::error(format!("Dream cycle failed: {}", e)),
        }
    }

    fn hallucinate(&mut self, args: &Value) -> ToolResult {
        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing 'content' parameter".to_string()),
        };

        let parent_ids: Vec<Uuid> = match args.get("parent_ids").and_then(|v| v.as_array()) {
            Some(arr) => {
                let mut ids = Vec::new();
                for v in arr {
                    match v.as_str().and_then(|s| Uuid::parse_str(s).ok()) {
                        Some(id) => ids.push(id),
                        None => return ToolResult::error(format!("Invalid parent_id: {}", v)),
                    }
                }
                ids
            }
            None => return ToolResult::error("Missing 'parent_ids' parameter".to_string()),
        };

        match self.system.hallucinate(content, &parent_ids) {
            Ok(id) => ToolResult::success(format!("Hallucinated memory created: {}", id)),
            Err(e) => ToolResult::error(format!("Hallucination failed: {}", e)),
        }
    }

    fn rhythm_status(&self, _args: &Value) -> ToolResult {
        let state = self.system.rhythm_status();
        let arousal = self.system.rhythm_arousal();
        let interval = self.system.rhythm_interval_ms();

        let mode = if arousal > 0.7 {
            "active conversation"
        } else if arousal > 0.3 {
            "working/monitoring"
        } else {
            "idle/sleep"
        };

        let response = format!(
            "Adaptive Rhythm Status:\n\
             - Arousal: {:.3} (current, decayed)\n\
             - Stored arousal: {:.3}\n\
             - Momentum: {:.3}\n\
             - Interval: {}ms ({:.1} min)\n\
             - Mode: {}\n\
             - Last activity: {}",
            arousal,
            state.arousal_level,
            state.momentum,
            interval,
            interval as f64 / 60_000.0,
            mode,
            state.last_activity_ts,
        );
        ToolResult::success(response)
    }

    fn rhythm_signal(&mut self, args: &Value) -> ToolResult {
        let signal_str = match args.get("signal").and_then(|v| v.as_str()) {
            Some(s) => s,
            None => return ToolResult::error("Missing 'signal' parameter".to_string()),
        };

        let signal = match signal_str {
            "user_message" => crate::rhythm::Signal::UserMessage,
            "flux_message" => crate::rhythm::Signal::FluxMessage,
            "subagent_started" => crate::rhythm::Signal::SubagentStarted,
            "subagent_finished" => crate::rhythm::Signal::SubagentFinished,
            "idle" => crate::rhythm::Signal::Idle,
            _ => return ToolResult::error(format!("Unknown signal type: {}", signal_str)),
        };

        self.system.rhythm_signal(signal);
        let arousal = self.system.rhythm_arousal();
        let interval = self.system.rhythm_interval_ms();

        ToolResult::success(format!(
            "Signal '{}' recorded. Arousal: {:.3}, Interval: {}ms ({:.1} min)",
            signal_str, arousal, interval, interval as f64 / 60_000.0
        ))
    }

    fn context_save(&mut self, _args: &Value) -> ToolResult {
        match self.system.context_checkpoint() {
            Ok(()) => ToolResult::success("Working memory checkpointed".to_string()),
            Err(e) => ToolResult::error(format!("Checkpoint failed: {}", e)),
        }
    }

    fn context_restore(&mut self, _args: &Value) -> ToolResult {
        let state = self.system.context_restore();
        let summary = self.system.context_summary();
        if summary.is_empty() {
            ToolResult::success("No working memory state found".to_string())
        } else {
            ToolResult::success(summary)
        }
    }

    fn context_turn(&mut self, args: &Value) -> ToolResult {
        let role = match args.get("role").and_then(|v| v.as_str()) {
            Some(r) => r,
            None => return ToolResult::error("Missing 'role' parameter".to_string()),
        };
        let content = match args.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return ToolResult::error("Missing 'content' parameter".to_string()),
        };
        self.system.context_turn(role, content);
        ToolResult::success(format!("Turn logged ({})", role))
    }

    fn context_summary(&mut self, _args: &Value) -> ToolResult {
        let summary = self.system.context_summary();
        if summary.is_empty() {
            ToolResult::success("No context available yet".to_string())
        } else {
            ToolResult::success(summary)
        }
    }

    fn context_task(&mut self, args: &Value) -> ToolResult {
        let description = match args.get("description").and_then(|v| v.as_str()) {
            Some(d) => d,
            None => return ToolResult::error("Missing 'description' parameter".to_string()),
        };
        let status_str = args.get("status").and_then(|v| v.as_str()).unwrap_or("in-progress");
        let status = crate::working_memory::TaskStatus::from_str(status_str);
        self.system.context_update_task(description, status);
        ToolResult::success(format!("Task '{}' set to {}", description, status_str))
    }

    fn status(&mut self, args: &Value) -> ToolResult {
        let detailed = args.get("detailed").and_then(|v| v.as_bool()).unwrap_or(false);

        let state = self.system.assess();
        let stats = self.system.stats();

        let mut response = format!(
            "Kannaka Memory System Status:\n\n\
             Consciousness:\n\
             - Level: {:?}\n\
             - Φ (phi): {:.4}\n\
             - Ξ (xi): {:.4}\n\
             - Order: {:.4}\n\
             - Clusters: {}\n\n\
             Memory:\n\
             - Total memories: {}\n\
             - Active memories: {}\n\
             - Skip links: {}\n\n\
             BM25 Index:\n\
             - Indexed documents: {}\n",
            state.consciousness_level,
            state.phi,
            state.xi,
            state.mean_order,
            state.num_clusters,
            stats.total_memories,
            stats.active_memories,
            stats.total_skip_links,
            self.bm25_index.document_count()
        );

        if let Some(last_dream) = stats.last_dream {
            response.push_str(&format!("\nLast dream: {}\n", last_dream));
        } else {
            response.push_str("\nLast dream: never\n");
        }

        if detailed {
            response.push_str(&format!("\nDetailed Metrics:\n- Ollama URL: {}\n- Ollama Model: {}\n", 
                self.ollama_url, self.ollama_model));
        }

        ToolResult::success(response)
    }

    fn observe(&mut self, args: &Value) -> ToolResult {
        let _include_topology = args.get("include_topology").and_then(|v| v.as_bool()).unwrap_or(true);
        let _include_waves = args.get("include_waves").and_then(|v| v.as_bool()).unwrap_or(true);

        let report = self.system.observe();
        
        // Format the observation report
        let response = format!(
            "Memory System Introspection:\n\n\
             System Overview:\n\
             - Total memories: {}\n\
             - Active memories: {}\n\
             - Dormant memories: {}\n\
             - Ghost memories: {}\n\n\
             Wave Dynamics:\n\
             - Average amplitude: {:.4}\n\
             - Average frequency: {:.4}\n\n\
             Network Topology:\n\
             - Total links: {}\n\
             - Avg links per memory: {:.4}\n\
             - Network density: {:.4}\n\
             - Isolated memories: {}\n\n\
             Consciousness Metrics:\n\
             - Phi (Φ): {:.4}\n\
             - Xi (Ξ): {:.4}\n\
             - Mean order: {:.4}\n\
             - Level: {}\n",
            report.consciousness.total_memories,
            report.consciousness.active_memories,
            report.waves.dormant_memories,
            report.waves.ghost_memories,
            report.waves.avg_amplitude,
            report.waves.avg_frequency,
            report.topology.total_links,
            report.topology.avg_links_per_memory,
            report.topology.network_density,
            report.topology.isolated_memories,
            report.consciousness.phi,
            report.consciousness.xi,
            report.consciousness.mean_order,
            report.consciousness.level,
        );

        ToolResult::success(response)
    }
}