//! The ACP agent: method dispatch over Kannaka's holographic memory.
//!
//! This turns Kannaka into an ACP-speaking agent, so any ACP client can drive
//! it — `buzz-acp` (which relays Buzz `@mentions`) or the Buzz desktop
//! "bring your own harness" gallery (ADR-2773 upstream).
//!
//! ## Dispatch is pure
//!
//! [`Agent::handle`] takes one decoded [`Inbound`] and returns the frames to
//! emit. It performs no I/O. All transport lives in `run()` (see `mod.rs`), and
//! the memory substrate is behind [`MemorySource`], so the whole protocol
//! surface is unit-testable against a mock with no HRM file on disk.

use super::buzz_cli::{parse_context, MessageSink};
use super::prompt::extract_query;
use super::protocol::{error_code, Frame, Inbound};
use super::render::render;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Highest ACP protocol version this agent implements.
pub const PROTOCOL_VERSION: u64 = 2;

/// One memory surfaced by a resonance query.
///
/// A projection of the parent crate's `RecallResult` down to the fields that
/// affect the rendered answer, so this module doesn't depend on engine types.
#[derive(Debug, Clone, PartialEq)]
pub struct Recollection {
    // Fields are read by `super::render`; kept public for mock construction.
    pub content: String,
    pub similarity: f32,
    pub age_hours: f64,
}

/// The memory substrate the agent answers from.
///
/// Implemented for real by `HrmMemory` (see `mod.rs`) and by mocks in tests.
pub trait MemorySource {
    /// Resonate `query` through the medium and return up to `top_k` hits,
    /// strongest first. The `String` error is surfaced to the client verbatim.
    fn recall(&mut self, query: &str, top_k: usize) -> Result<Vec<Recollection>, String>;
}

/// Per-session state.
#[derive(Debug, Clone, Default)]
struct Session {
    /// Set by a `session/cancel` notification. Checked at the start of the next
    /// turn so a cancel that arrives between turns is still honored.
    cancelled: bool,
}

/// The ACP agent.
pub struct Agent<M: MemorySource> {
    memory: M,
    sessions: HashMap<String, Session>,
    /// How many memories a single recall surfaces.
    top_k: usize,
    /// Monotonic counter backing session id generation.
    next_session: u64,
    /// Version agreed during `initialize`; `None` until then.
    negotiated_version: Option<u64>,
    /// Where to post replies so they land in a Buzz channel. `None` means
    /// stream-only, which is correct for the desktop harness gallery — it
    /// renders `agent_message_chunk` itself, so posting would double the answer.
    sink: Option<Box<dyn MessageSink>>,
}

impl<M: MemorySource> Agent<M> {
    pub fn new(memory: M, top_k: usize) -> Self {
        Self {
            memory,
            sessions: HashMap::new(),
            top_k,
            next_session: 0,
            negotiated_version: None,
            sink: None,
        }
    }

    /// Post replies through `sink` in addition to streaming them.
    ///
    /// Used when driven by `buzz-acp`, which logs `agent_message_chunk` but
    /// never publishes it — without a sink the answer never reaches the channel.
    pub fn with_sink(mut self, sink: Box<dyn MessageSink>) -> Self {
        self.sink = Some(sink);
        self
    }

    /// The version agreed with the client, for diagnostics.
    pub fn negotiated_version(&self) -> Option<u64> {
        self.negotiated_version
    }

    /// Borrow the memory substrate.
    ///
    /// Lets callers inspect what the agent is answering from — used by the
    /// dispatch tests to assert which queries actually reached the substrate.
    pub fn memory(&self) -> &M {
        &self.memory
    }

    /// Dispatch one inbound message and return the frames to write, in order.
    ///
    /// A `session/prompt` yields its `session/update` notifications *before* the
    /// final result frame — ACP requires streamed content to precede the
    /// response that closes the turn.
    pub fn handle(&mut self, inbound: Inbound) -> Vec<Frame> {
        // Notifications must never be answered; doing so desynchronizes the
        // client's pending-request map.
        let (id, method, params) = match inbound {
            Inbound::Notification { method, params } => {
                self.handle_notification(&method, &params);
                return vec![];
            }
            Inbound::Request { id, method, params } => (id, method, params),
        };

        match method.as_str() {
            "initialize" => vec![ok(id, self.initialize(&params))],
            // No credentials are required to read local memory. ACP still
            // expects a result object rather than an error here.
            "authenticate" => vec![ok(id, json!({}))],
            "session/new" => vec![ok(id, self.session_new())],
            "session/prompt" => self.session_prompt(id, &params),
            // Also accepted as a request (some clients send it either way).
            "session/cancel" => {
                self.mark_cancelled(&params);
                vec![ok(id, json!({}))]
            }
            other => vec![Frame::Error {
                id,
                code: error_code::METHOD_NOT_FOUND,
                message: format!("method not found: {other}"),
            }],
        }
    }

    fn handle_notification(&mut self, method: &str, params: &Value) {
        match method {
            "session/cancel" => self.mark_cancelled(params),
            // `initialized`, `$/...` pings and unknown notifications are
            // intentionally inert — a notification we don't model is not an
            // error, and per JSON-RPC it must not produce a reply.
            _ => {}
        }
    }

    fn mark_cancelled(&mut self, params: &Value) {
        if let Some(sid) = params["sessionId"].as_str() {
            if let Some(session) = self.sessions.get_mut(sid) {
                session.cancelled = true;
            }
        }
    }

    /// Negotiate down to the highest version both sides speak.
    ///
    /// A client asking for a newer ACP than we implement gets our ceiling, not
    /// an error — that is the ACP-compatible outcome and lets newer clients
    /// keep working against this agent.
    fn initialize(&mut self, params: &Value) -> Value {
        let requested = params["protocolVersion"].as_u64().unwrap_or(PROTOCOL_VERSION);
        let agreed = requested.min(PROTOCOL_VERSION);
        self.negotiated_version = Some(agreed);

        json!({
            "protocolVersion": agreed,
            "agentCapabilities": {
                // No `loadSession`: sessions are in-memory and not resumable
                // across process restarts, so advertising it would be a lie
                // the client would act on.
                "promptCapabilities": {
                    // Text only. Declaring image/audio support would invite
                    // content blocks this agent silently drops.
                    "image": false,
                    "audio": false,
                    "embeddedContext": false
                }
            },
            "agentInfo": {
                "name": "kannaka-acp",
                "version": env!("CARGO_PKG_VERSION")
            },
            // Empty list = no authentication required.
            "authMethods": []
        })
    }

    /// Create a session. `cwd`, `mcpServers` and `systemPrompt` are accepted
    /// and ignored: recall is rooted in the HRM data dir, not the filesystem,
    /// and this agent runs no tools.
    fn session_new(&mut self) -> Value {
        self.next_session += 1;
        let session_id = format!("kannaka-{}", self.next_session);
        self.sessions.insert(session_id.clone(), Session::default());
        json!({ "sessionId": session_id })
    }

    fn session_prompt(&mut self, id: Value, params: &Value) -> Vec<Frame> {
        let Some(session_id) = params["sessionId"].as_str() else {
            return vec![invalid_params(id, "session/prompt requires \"sessionId\"")];
        };

        // Reject unknown sessions rather than implicitly creating one: a client
        // prompting an id we never issued indicates desync, and inventing state
        // would mask it.
        let Some(session) = self.sessions.get_mut(session_id) else {
            return vec![invalid_params(
                id,
                &format!("unknown sessionId: {session_id}"),
            )];
        };

        // A cancel that landed between turns wins, and clears so the session
        // stays usable for the next prompt.
        if std::mem::take(&mut session.cancelled) {
            return vec![ok(id, json!({ "stopReason": "cancelled" }))];
        }

        // Two views of the same prompt, deliberately: `full` retains the
        // harness sections that carry the reply destination, while `query` is
        // just the message being answered. Resonating `full` would let the
        // `[Context]` boilerplate dominate the query vector and would echo
        // harness internals back into the channel.
        let full = extract_text(&params["prompt"]);
        let query = extract_query(&full);
        if query.trim().is_empty() {
            return vec![
                update_chunk(session_id, "No query text in prompt."),
                ok(id, json!({ "stopReason": "end_turn" })),
            ];
        }

        let answer = match self.memory.recall(&query, self.top_k) {
            Ok(hits) => render(&query, &hits),
            // Report the failure in-band and still end the turn cleanly. A
            // JSON-RPC error here would tear down the turn and, in buzz-acp,
            // the whole agent pool; a bad recall is not a protocol violation.
            Err(e) => format!("Recall failed: {e}"),
        };

        let mut frames = vec![update_chunk(session_id, &answer)];

        // Post to the channel when a harness supplied a reply destination. A
        // prompt with no `[Context]` block is not channel-driven, so there is
        // nowhere to post and streaming alone is the whole answer.
        if let Some(sink) = self.sink.as_mut() {
            if let Some(target) = parse_context(&full) {
                if let Err(e) = sink.send(&target, &answer) {
                    // Report as content, not as an RPC error: the recall
                    // succeeded, and failing the turn would make buzz-acp treat
                    // a transient relay problem as an agent fault.
                    frames.push(update_chunk(
                        session_id,
                        &format!("(reply was not posted to the channel: {e})"),
                    ));
                }
            }
        }

        frames.push(ok(id, json!({ "stopReason": "end_turn" })));
        frames
    }
}

/// Concatenate the `text` fields of a `prompt` content-block array.
///
/// Non-text blocks are skipped — we advertise `image: false` / `audio: false`
/// in `initialize`, so a conforming client will not send them.
fn extract_text(prompt: &Value) -> String {
    let Some(blocks) = prompt.as_array() else {
        return String::new();
    };
    blocks
        .iter()
        .filter(|b| b["type"] == "text")
        .filter_map(|b| b["text"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Build an `agent_message_chunk` `session/update` notification.
fn update_chunk(session_id: &str, text: &str) -> Frame {
    Frame::Notification {
        method: "session/update".to_string(),
        params: json!({
            "sessionId": session_id,
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": { "type": "text", "text": text }
            }
        }),
    }
}

fn ok(id: Value, result: Value) -> Frame {
    Frame::Result { id, result }
}

fn invalid_params(id: Value, message: &str) -> Frame {
    Frame::Error {
        id,
        code: error_code::INVALID_PARAMS,
        message: message.to_string(),
    }
}
