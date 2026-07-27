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

use super::protocol::{error_code, Frame, Inbound};
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
}

impl<M: MemorySource> Agent<M> {
    pub fn new(memory: M, top_k: usize) -> Self {
        Self {
            memory,
            sessions: HashMap::new(),
            top_k,
            next_session: 0,
            negotiated_version: None,
        }
    }

    /// The version agreed with the client, for diagnostics.
    pub fn negotiated_version(&self) -> Option<u64> {
        self.negotiated_version
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

        let query = extract_text(&params["prompt"]);
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

        vec![
            update_chunk(session_id, &answer),
            ok(id, json!({ "stopReason": "end_turn" })),
        ]
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

/// Render recalled memories as the agent's answer.
fn render(query: &str, hits: &[Recollection]) -> String {
    if hits.is_empty() {
        return format!("No memories resonated with \"{query}\".");
    }

    let mut out = format!(
        "{} {} for \"{}\":\n",
        hits.len(),
        if hits.len() == 1 { "memory" } else { "memories" },
        query
    );
    for (i, hit) in hits.iter().enumerate() {
        out.push_str(&format!(
            "\n{}. [{:.0}% · {}] {}",
            i + 1,
            hit.similarity * 100.0,
            format_age(hit.age_hours),
            hit.content.trim()
        ));
    }
    out
}

/// Human-readable age, coarsened by magnitude.
fn format_age(hours: f64) -> String {
    if hours < 1.0 {
        "just now".to_string()
    } else if hours < 24.0 {
        format!("{}h ago", hours.round() as i64)
    } else {
        format!("{}d ago", (hours / 24.0).round() as i64)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Scripted memory source: returns canned hits, or an error when set.
    #[derive(Default)]
    struct MockMemory {
        hits: Vec<Recollection>,
        fail: Option<String>,
        /// Records what was asked, to assert prompt-block assembly.
        seen: Vec<(String, usize)>,
    }

    impl MemorySource for MockMemory {
        fn recall(&mut self, query: &str, top_k: usize) -> Result<Vec<Recollection>, String> {
            self.seen.push((query.to_string(), top_k));
            match &self.fail {
                Some(e) => Err(e.clone()),
                None => Ok(self.hits.clone()),
            }
        }
    }

    fn hit(content: &str, similarity: f32, age_hours: f64) -> Recollection {
        Recollection {
            content: content.to_string(),
            similarity,
            age_hours,
        }
    }

    fn agent() -> Agent<MockMemory> {
        Agent::new(MockMemory::default(), 3)
    }

    fn request(id: i64, method: &str, params: Value) -> Inbound {
        Inbound::Request {
            id: json!(id),
            method: method.to_string(),
            params,
        }
    }

    /// Drive initialize + session/new and return the session id.
    fn open_session<M: MemorySource>(agent: &mut Agent<M>) -> String {
        agent.handle(request(1, "initialize", json!({"protocolVersion": 2})));
        let frames = agent.handle(request(2, "session/new", json!({"cwd": "."})));
        match &frames[0] {
            Frame::Result { result, .. } => result["sessionId"].as_str().unwrap().to_string(),
            other => panic!("expected result, got {other:?}"),
        }
    }

    fn result_of(frame: &Frame) -> &Value {
        match frame {
            Frame::Result { result, .. } => result,
            other => panic!("expected result frame, got {other:?}"),
        }
    }

    #[test]
    fn initialize_reports_version_and_capabilities() {
        let mut a = agent();
        let frames = a.handle(request(1, "initialize", json!({"protocolVersion": 2})));
        let r = result_of(&frames[0]);
        assert_eq!(r["protocolVersion"], 2);
        assert_eq!(r["agentInfo"]["name"], "kannaka-acp");
        // Empty authMethods signals "no auth required".
        assert_eq!(r["authMethods"], json!([]));
        assert!(r["agentCapabilities"].is_object());
    }

    #[test]
    fn initialize_negotiates_down_to_our_ceiling() {
        // A future client must get our max, not an error.
        let mut a = agent();
        let frames = a.handle(request(1, "initialize", json!({"protocolVersion": 99})));
        assert_eq!(result_of(&frames[0])["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(a.negotiated_version(), Some(PROTOCOL_VERSION));
    }

    #[test]
    fn initialize_honors_an_older_client() {
        let mut a = agent();
        let frames = a.handle(request(1, "initialize", json!({"protocolVersion": 1})));
        assert_eq!(result_of(&frames[0])["protocolVersion"], 1);
    }

    #[test]
    fn session_new_returns_unique_ids() {
        let mut a = agent();
        let first = open_session(&mut a);
        let frames = a.handle(request(3, "session/new", json!({"cwd": "."})));
        let second = result_of(&frames[0])["sessionId"].as_str().unwrap();
        assert_ne!(first, second);
    }

    #[test]
    fn prompt_streams_a_chunk_then_ends_the_turn() {
        let mut a = Agent::new(
            MockMemory {
                hits: vec![hit("the swarm hums at 72.83Hz", 0.91, 2.0)],
                ..Default::default()
            },
            3,
        );
        let sid = open_session(&mut a);
        let frames = a.handle(request(
            9,
            "session/prompt",
            json!({"sessionId": sid, "prompt": [{"type": "text", "text": "swarm"}]}),
        ));

        // Content must precede the frame that closes the turn.
        assert_eq!(frames.len(), 2);
        match &frames[0] {
            Frame::Notification { method, params } => {
                assert_eq!(method, "session/update");
                assert_eq!(params["update"]["sessionUpdate"], "agent_message_chunk");
                let text = params["update"]["content"]["text"].as_str().unwrap();
                assert!(text.contains("72.83Hz"), "got: {text}");
            }
            other => panic!("expected notification first, got {other:?}"),
        }
        assert_eq!(result_of(&frames[1])["stopReason"], "end_turn");
    }

    #[test]
    fn prompt_concatenates_all_text_blocks() {
        let mut a = agent();
        let sid = open_session(&mut a);
        a.handle(request(
            9,
            "session/prompt",
            json!({"sessionId": sid, "prompt": [
                {"type": "text", "text": "first"},
                {"type": "image", "data": "ignored"},
                {"type": "text", "text": "second"}
            ]}),
        ));
        // Non-text blocks are dropped; text blocks join in order.
        assert_eq!(a.memory.seen[0].0, "first\nsecond");
        assert_eq!(a.memory.seen[0].1, 3);
    }

    #[test]
    fn empty_recall_says_so_without_failing_the_turn() {
        let mut a = agent();
        let sid = open_session(&mut a);
        let frames = a.handle(request(
            9,
            "session/prompt",
            json!({"sessionId": sid, "prompt": [{"type": "text", "text": "nothing"}]}),
        ));
        assert_eq!(result_of(&frames[1])["stopReason"], "end_turn");
        match &frames[0] {
            Frame::Notification { params, .. } => {
                let text = params["update"]["content"]["text"].as_str().unwrap();
                assert!(text.contains("No memories resonated"), "got: {text}");
            }
            other => panic!("expected notification, got {other:?}"),
        }
    }

    #[test]
    fn recall_failure_is_reported_in_band_not_as_rpc_error() {
        // A failed recall must not tear down the turn — buzz-acp treats an RPC
        // error on session/prompt as an agent fault and recycles the process.
        let mut a = Agent::new(
            MockMemory {
                fail: Some("hrm locked".to_string()),
                ..Default::default()
            },
            3,
        );
        let sid = open_session(&mut a);
        let frames = a.handle(request(
            9,
            "session/prompt",
            json!({"sessionId": sid, "prompt": [{"type": "text", "text": "q"}]}),
        ));
        assert_eq!(result_of(&frames[1])["stopReason"], "end_turn");
        match &frames[0] {
            Frame::Notification { params, .. } => {
                let text = params["update"]["content"]["text"].as_str().unwrap();
                assert!(text.contains("hrm locked"), "got: {text}");
            }
            other => panic!("expected notification, got {other:?}"),
        }
    }

    #[test]
    fn empty_prompt_text_still_ends_the_turn_and_skips_recall() {
        let mut a = agent();
        let sid = open_session(&mut a);
        let frames = a.handle(request(
            9,
            "session/prompt",
            json!({"sessionId": sid, "prompt": []}),
        ));
        assert_eq!(result_of(&frames[1])["stopReason"], "end_turn");
        assert!(a.memory.seen.is_empty(), "must not query on empty prompt");
    }

    #[test]
    fn unknown_session_is_invalid_params() {
        let mut a = agent();
        a.handle(request(1, "initialize", json!({})));
        let frames = a.handle(request(
            9,
            "session/prompt",
            json!({"sessionId": "nope", "prompt": [{"type":"text","text":"q"}]}),
        ));
        match &frames[0] {
            Frame::Error { code, .. } => assert_eq!(*code, error_code::INVALID_PARAMS),
            other => panic!("expected error, got {other:?}"),
        }
    }

    #[test]
    fn missing_session_id_is_invalid_params() {
        let mut a = agent();
        let frames = a.handle(request(9, "session/prompt", json!({"prompt": []})));
        match &frames[0] {
            Frame::Error { code, .. } => assert_eq!(*code, error_code::INVALID_PARAMS),
            other => panic!("expected error, got {other:?}"),
        }
    }

    #[test]
    fn cancel_notification_produces_no_frames() {
        let mut a = agent();
        let sid = open_session(&mut a);
        let frames = a.handle(Inbound::Notification {
            method: "session/cancel".to_string(),
            params: json!({"sessionId": sid}),
        });
        // Answering a notification would desync the client.
        assert!(frames.is_empty());
    }

    #[test]
    fn cancel_between_turns_yields_cancelled_then_clears() {
        let mut a = agent();
        let sid = open_session(&mut a);
        a.handle(Inbound::Notification {
            method: "session/cancel".to_string(),
            params: json!({"sessionId": sid}),
        });

        let prompt = json!({"sessionId": sid, "prompt": [{"type":"text","text":"q"}]});
        let frames = a.handle(request(9, "session/prompt", prompt.clone()));
        assert_eq!(result_of(&frames[0])["stopReason"], "cancelled");
        assert!(a.memory.seen.is_empty(), "cancelled turn must not recall");

        // The flag is one-shot; the session stays usable.
        let frames = a.handle(request(10, "session/prompt", prompt));
        assert_eq!(result_of(&frames[1])["stopReason"], "end_turn");
    }

    #[test]
    fn unknown_method_is_method_not_found() {
        let mut a = agent();
        let frames = a.handle(request(1, "session/set_model", json!({})));
        match &frames[0] {
            Frame::Error { code, .. } => assert_eq!(*code, error_code::METHOD_NOT_FOUND),
            other => panic!("expected error, got {other:?}"),
        }
    }

    #[test]
    fn unknown_notification_is_silently_ignored() {
        let mut a = agent();
        let frames = a.handle(Inbound::Notification {
            method: "initialized".to_string(),
            params: json!({}),
        });
        assert!(frames.is_empty());
    }

    #[test]
    fn authenticate_succeeds_without_credentials() {
        let mut a = agent();
        let frames = a.handle(request(1, "authenticate", json!({"methodId": "x"})));
        assert!(matches!(frames[0], Frame::Result { .. }));
    }

    #[test]
    fn render_formats_rank_score_and_age() {
        let text = render(
            "q",
            &[hit("alpha", 0.9, 0.5), hit("beta", 0.5, 48.0)],
        );
        assert!(text.contains("2 memories"), "got: {text}");
        assert!(text.contains("1. [90% · just now] alpha"), "got: {text}");
        assert!(text.contains("2. [50% · 2d ago] beta"), "got: {text}");
    }

    #[test]
    fn render_uses_singular_for_one_hit() {
        let text = render("q", &[hit("only", 1.0, 3.0)]);
        assert!(text.contains("1 memory for"), "got: {text}");
        assert!(text.contains("3h ago"), "got: {text}");
    }
}
