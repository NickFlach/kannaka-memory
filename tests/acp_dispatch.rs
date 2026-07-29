//! Dispatch tests for the ACP agent (`kannaka_memory::acp::server`).
//!
//! Lives here rather than inline so `src/acp/server.rs` stays under the
//! 500-line limit, and because the whole surface under test is public: the
//! agent is driven exactly as a real ACP client drives it — decoded messages
//! in, frames out — against a scripted memory substrate.

mod dispatch {
    use kannaka_memory::acp::protocol::{error_code, Frame, Inbound};
    use kannaka_memory::acp::server::{Agent, MemorySource, Recollection, PROTOCOL_VERSION};
    use serde_json::{json, Value};

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
        assert_eq!(a.memory().seen[0].0, "first\nsecond");
        assert_eq!(a.memory().seen[0].1, 3);
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
        assert!(a.memory().seen.is_empty(), "must not query on empty prompt");
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
        assert!(a.memory().seen.is_empty(), "cancelled turn must not recall");

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

}

/// Tests for channel posting: the agent must send its answer through the sink
/// when — and only when — a harness supplied a `[Context]` reply destination.
mod channel_posting {
    use kannaka_memory::acp::buzz_cli::{MessageSink, ReplyTarget};
    use kannaka_memory::acp::protocol::{Frame, Inbound};
    use kannaka_memory::acp::server::{Agent, MemorySource, Recollection};
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};

    /// Records what was posted; optionally fails to exercise the error path.
    struct MockSink {
        sent: Arc<Mutex<Vec<(ReplyTarget, String)>>>,
        fail: Option<String>,
    }

    impl MessageSink for MockSink {
        fn send(&mut self, target: &ReplyTarget, body: &str) -> Result<(), String> {
            self.sent
                .lock()
                .unwrap()
                .push((target.clone(), body.to_string()));
            match &self.fail {
                Some(e) => Err(e.clone()),
                None => Ok(()),
            }
        }
    }

    struct OneHit;

    impl MemorySource for OneHit {
        fn recall(&mut self, _q: &str, _k: usize) -> Result<Vec<Recollection>, String> {
            Ok(vec![Recollection {
                content: "the swarm hums".to_string(),
                similarity: 0.9,
                age_hours: 1.0,
            }])
        }
    }

    const CHANNEL: &str = "8f14e45f-ceea-467a-9c1e-1b2c3d4e5f60";

    fn prompt_with_context() -> String {
        format!(
            "[Context]\nScope: channel\nChannel: general (#{CHANNEL})\n\n[Event]\nwhat's up?"
        )
    }

    /// Build an agent wired to a recording sink; returns the agent, the log, and
    /// an opened session id.
    fn wired(fail: Option<&str>) -> (Agent<OneHit>, Arc<Mutex<Vec<(ReplyTarget, String)>>>, String) {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let sink = MockSink {
            sent: Arc::clone(&sent),
            fail: fail.map(str::to_string),
        };
        let mut a = Agent::new(OneHit, 3).with_sink(Box::new(sink));
        a.handle(Inbound::Request {
            id: json!(1),
            method: "initialize".to_string(),
            params: json!({"protocolVersion": 2}),
        });
        let frames = a.handle(Inbound::Request {
            id: json!(2),
            method: "session/new".to_string(),
            params: json!({"cwd": "."}),
        });
        let sid = match &frames[0] {
            Frame::Result { result, .. } => result["sessionId"].as_str().unwrap().to_string(),
            other => panic!("expected result, got {other:?}"),
        };
        (a, sent, sid)
    }

    fn prompt(a: &mut Agent<OneHit>, sid: &str, text: &str) -> Vec<Frame> {
        a.handle(Inbound::Request {
            id: json!(9),
            method: "session/prompt".to_string(),
            params: json!({"sessionId": sid, "prompt": [{"type": "text", "text": text}]}),
        })
    }

    fn result_of(frame: &Frame) -> &Value {
        match frame {
            Frame::Result { result, .. } => result,
            other => panic!("expected result frame, got {other:?}"),
        }
    }

    #[test]
    fn answer_is_posted_to_the_channel_from_context() {
        let (mut a, sent, sid) = wired(None);
        let frames = prompt(&mut a, &sid, &prompt_with_context());

        let log = sent.lock().unwrap();
        assert_eq!(log.len(), 1, "exactly one post per turn");
        assert_eq!(log[0].0.channel, CHANNEL);
        assert!(log[0].1.contains("the swarm hums"), "got: {}", log[0].1);
        // Streaming still happens — the desktop and the harness log rely on it.
        assert!(matches!(frames[0], Frame::Notification { .. }));
        assert_eq!(result_of(frames.last().unwrap())["stopReason"], "end_turn");
    }

    #[test]
    fn posted_body_matches_the_streamed_chunk() {
        let (mut a, sent, sid) = wired(None);
        let frames = prompt(&mut a, &sid, &prompt_with_context());
        let streamed = match &frames[0] {
            Frame::Notification { params, .. } => params["update"]["content"]["text"]
                .as_str()
                .unwrap()
                .to_string(),
            other => panic!("expected notification, got {other:?}"),
        };
        assert_eq!(sent.lock().unwrap()[0].1, streamed);
    }

    #[test]
    fn nothing_is_posted_without_a_context_block() {
        // Desktop harness gallery: it renders the chunk itself, so posting
        // would duplicate the answer.
        let (mut a, sent, sid) = wired(None);
        let frames = prompt(&mut a, &sid, "just asking directly");
        assert!(sent.lock().unwrap().is_empty());
        assert_eq!(result_of(frames.last().unwrap())["stopReason"], "end_turn");
    }

    #[test]
    fn post_failure_is_reported_as_content_and_still_ends_the_turn() {
        let (mut a, _sent, sid) = wired(Some("relay unreachable"));
        let frames = prompt(&mut a, &sid, &prompt_with_context());

        // answer chunk, failure chunk, result
        assert_eq!(frames.len(), 3);
        match &frames[1] {
            Frame::Notification { params, .. } => {
                let text = params["update"]["content"]["text"].as_str().unwrap();
                assert!(text.contains("relay unreachable"), "got: {text}");
                assert!(text.contains("not posted"), "got: {text}");
            }
            other => panic!("expected failure notification, got {other:?}"),
        }
        // A failed post must not surface as an RPC error: buzz-acp would treat
        // that as an agent fault and recycle the process.
        assert_eq!(result_of(&frames[2])["stopReason"], "end_turn");
    }

    #[test]
    fn cancelled_turn_posts_nothing() {
        let (mut a, sent, sid) = wired(None);
        a.handle(Inbound::Notification {
            method: "session/cancel".to_string(),
            params: json!({"sessionId": sid}),
        });
        let frames = prompt(&mut a, &sid, &prompt_with_context());
        assert_eq!(result_of(&frames[0])["stopReason"], "cancelled");
        assert!(sent.lock().unwrap().is_empty());
    }
}
