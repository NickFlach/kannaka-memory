//! JSON-RPC 2.0 framing for the ACP (Agent Client Protocol) stdio transport.
//!
//! ACP frames are newline-delimited JSON ("NDJSON") — one complete JSON value
//! per line, no `Content-Length` headers. This matches the framing used by the
//! reference client (`buzz-acp`'s `write_ndjson`), by `goose acp`, and by the
//! `claude-agent-acp` adapter.
//!
//! ## Why hand-rolled instead of a protocol crate
//!
//! The parent crate deliberately dropped `tokio` (see Cargo.toml: "tokio +
//! async-trait removed") when the old MCP server went away. ACP over stdio is
//! strictly request/response plus server-initiated notifications, so a blocking
//! line loop is sufficient and keeps the dependency surface at `serde_json`.
//!
//! ## Protocol invariant: stdout carries frames ONLY
//!
//! Anything written to stdout that is not a JSON-RPC frame corrupts the stream
//! and the client will fail with a parse error. All diagnostics must go to
//! stderr. [`Frame::write`] is the only sanctioned stdout writer.

use serde_json::{json, Value};
use std::io::{BufRead, Write};

/// JSON-RPC 2.0 error codes used by this agent.
///
/// The negative values below are the reserved codes from the JSON-RPC 2.0
/// specification, section 5.1. We do not define application-specific codes:
/// a failed recall is reported as `INTERNAL_ERROR` with a human-readable
/// message rather than inventing a private code space the client can't read.
pub mod error_code {
    /// Malformed JSON was received.
    pub const PARSE_ERROR: i64 = -32700;
    /// The JSON is valid but is not a well-formed request object.
    pub const INVALID_REQUEST: i64 = -32600;
    /// The requested method does not exist.
    pub const METHOD_NOT_FOUND: i64 = -32601;
    /// The method exists but the params are unusable.
    pub const INVALID_PARAMS: i64 = -32602;
    /// The method exists and params were valid, but execution failed.
    pub const INTERNAL_ERROR: i64 = -32603;
}

/// A single decoded inbound JSON-RPC message.
///
/// ACP clients send both requests (which carry an `id` and demand a response)
/// and notifications (no `id`, must NOT be answered). Conflating the two is the
/// classic ACP bug: replying to a notification desynchronizes the client's
/// pending-request map and it will mis-attribute the next response.
#[derive(Debug, Clone, PartialEq)]
pub enum Inbound {
    /// A request expecting exactly one response with a matching `id`.
    Request {
        /// Correlation id. Per JSON-RPC this may be a number, string, or null,
        /// so it is kept as a raw `Value` and echoed back verbatim.
        id: Value,
        method: String,
        params: Value,
    },
    /// A fire-and-forget notification. Must not be answered.
    Notification { method: String, params: Value },
}

impl Inbound {
    /// The method name, regardless of variant.
    pub fn method(&self) -> &str {
        match self {
            Inbound::Request { method, .. } | Inbound::Notification { method, .. } => method,
        }
    }

    /// The params object, regardless of variant.
    pub fn params(&self) -> &Value {
        match self {
            Inbound::Request { params, .. } | Inbound::Notification { params, .. } => params,
        }
    }
}

/// Why a line could not be turned into an [`Inbound`].
#[derive(Debug, Clone, PartialEq)]
pub enum DecodeError {
    /// The line was not valid JSON. No `id` is recoverable, so a response
    /// cannot be correlated — reply with a null-id error per JSON-RPC 2.0.
    Parse(String),
    /// Valid JSON, but not a usable request object. The `id` (if any) is
    /// carried so the error response can still be correlated.
    Invalid { id: Value, message: String },
}

/// Decode one NDJSON line into an [`Inbound`].
///
/// Absent-vs-null `id` is the request/notification discriminator. Note that an
/// explicit `"id": null` is treated as a *request* here: JSON-RPC 2.0 permits a
/// null id, and answering it is harmless, whereas silently swallowing it would
/// hang a client that is waiting on it.
pub fn decode(line: &str) -> Result<Inbound, DecodeError> {
    let value: Value =
        serde_json::from_str(line).map_err(|e| DecodeError::Parse(e.to_string()))?;

    let obj = value.as_object().ok_or_else(|| DecodeError::Invalid {
        id: Value::Null,
        message: "request must be a JSON object".to_string(),
    })?;

    let id = obj.get("id").cloned();

    let method = obj
        .get("method")
        .and_then(|m| m.as_str())
        .ok_or_else(|| DecodeError::Invalid {
            id: id.clone().unwrap_or(Value::Null),
            message: "missing or non-string \"method\"".to_string(),
        })?
        .to_string();

    // Params are optional in JSON-RPC. Normalizing absent params to `{}` lets
    // handlers use plain indexing (`params["sessionId"]`) without unwrapping.
    let params = obj.get("params").cloned().unwrap_or_else(|| json!({}));

    match id {
        Some(id) => Ok(Inbound::Request { id, method, params }),
        None => Ok(Inbound::Notification { method, params }),
    }
}

/// An outbound JSON-RPC frame.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// A successful response to a request.
    Result { id: Value, result: Value },
    /// An error response to a request.
    Error {
        id: Value,
        code: i64,
        message: String,
    },
    /// A server-initiated notification (e.g. `session/update`).
    Notification { method: String, params: Value },
}

impl Frame {
    /// Render this frame as a JSON-RPC 2.0 object.
    pub fn to_value(&self) -> Value {
        match self {
            Frame::Result { id, result } => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result,
            }),
            Frame::Error { id, code, message } => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": code, "message": message },
            }),
            Frame::Notification { method, params } => json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params,
            }),
        }
    }

    /// Write this frame to `out` as one NDJSON line and flush.
    ///
    /// Flushing per frame is required, not an optimization slip: the client
    /// blocks reading our stdout, so a buffered `session/update` that never
    /// flushes presents to the client as an idle agent and trips its idle
    /// timeout mid-turn.
    pub fn write<W: Write>(&self, out: &mut W) -> std::io::Result<()> {
        // `to_value` produces a plain object, which cannot fail to serialize.
        let line = serde_json::to_string(&self.to_value())
            .expect("JSON-RPC frame is always serializable");
        out.write_all(line.as_bytes())?;
        out.write_all(b"\n")?;
        out.flush()
    }
}

/// Build the error [`Frame`] that answers a [`DecodeError`].
pub fn decode_error_frame(err: &DecodeError) -> Frame {
    match err {
        DecodeError::Parse(message) => Frame::Error {
            id: Value::Null,
            code: error_code::PARSE_ERROR,
            message: format!("parse error: {message}"),
        },
        DecodeError::Invalid { id, message } => Frame::Error {
            id: id.clone(),
            code: error_code::INVALID_REQUEST,
            message: message.clone(),
        },
    }
}

/// Read NDJSON lines from `input`, yielding each non-blank line.
///
/// Blank lines are skipped rather than reported as parse errors — some clients
/// emit a trailing newline on shutdown, and answering that with a spurious
/// `PARSE_ERROR` frame is noise the client then has to discard.
pub fn lines<R: BufRead>(input: R) -> impl Iterator<Item = std::io::Result<String>> {
    input
        .lines()
        .filter(|r| !matches!(r, Ok(line) if line.trim().is_empty()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_request_with_id() {
        let got = decode(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"a":1}}"#);
        assert_eq!(
            got,
            Ok(Inbound::Request {
                id: json!(1),
                method: "initialize".to_string(),
                params: json!({"a": 1}),
            })
        );
    }

    #[test]
    fn decodes_notification_without_id() {
        let got = decode(r#"{"jsonrpc":"2.0","method":"session/cancel","params":{}}"#);
        assert_eq!(
            got,
            Ok(Inbound::Notification {
                method: "session/cancel".to_string(),
                params: json!({}),
            })
        );
    }

    #[test]
    fn absent_params_normalize_to_empty_object() {
        let got = decode(r#"{"jsonrpc":"2.0","id":7,"method":"initialize"}"#).unwrap();
        assert_eq!(got.params(), &json!({}));
    }

    #[test]
    fn explicit_null_id_is_a_request_not_a_notification() {
        // A client waiting on a null-id request must still get a response.
        let got = decode(r#"{"jsonrpc":"2.0","id":null,"method":"initialize"}"#).unwrap();
        assert!(matches!(got, Inbound::Request { .. }));
    }

    #[test]
    fn malformed_json_is_a_parse_error() {
        let err = decode("{not json").unwrap_err();
        assert!(matches!(err, DecodeError::Parse(_)));
        // Unparseable input has no recoverable id, so the reply must use null.
        match decode_error_frame(&err) {
            Frame::Error { id, code, .. } => {
                assert_eq!(id, Value::Null);
                assert_eq!(code, error_code::PARSE_ERROR);
            }
            other => panic!("expected error frame, got {other:?}"),
        }
    }

    #[test]
    fn missing_method_keeps_id_for_correlation() {
        let err = decode(r#"{"jsonrpc":"2.0","id":42}"#).unwrap_err();
        match err {
            DecodeError::Invalid { ref id, .. } => assert_eq!(id, &json!(42)),
            other => panic!("expected invalid request, got {other:?}"),
        }
        match decode_error_frame(&err) {
            Frame::Error { id, code, .. } => {
                assert_eq!(id, json!(42));
                assert_eq!(code, error_code::INVALID_REQUEST);
            }
            other => panic!("expected error frame, got {other:?}"),
        }
    }

    #[test]
    fn non_object_json_is_invalid() {
        assert!(matches!(
            decode("[1,2,3]").unwrap_err(),
            DecodeError::Invalid { .. }
        ));
    }

    #[test]
    fn frames_serialize_as_single_ndjson_lines() {
        let mut buf = Vec::new();
        Frame::Result {
            id: json!(1),
            result: json!({"protocolVersion": 2}),
        }
        .write(&mut buf)
        .unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(text.ends_with('\n'));
        // Exactly one newline: embedded newlines would split one frame in two.
        assert_eq!(text.matches('\n').count(), 1);
        let back: Value = serde_json::from_str(text.trim()).unwrap();
        assert_eq!(back["jsonrpc"], "2.0");
        assert_eq!(back["result"]["protocolVersion"], 2);
    }

    #[test]
    fn notification_frame_omits_id() {
        let value = Frame::Notification {
            method: "session/update".to_string(),
            params: json!({}),
        }
        .to_value();
        // An `id` here would make the client treat this as a response and try
        // to match it against a pending request.
        assert!(value.get("id").is_none());
    }

    #[test]
    fn multiline_text_stays_one_frame() {
        // Recall content legitimately contains newlines; JSON escaping must
        // keep the frame on a single line.
        let mut buf = Vec::new();
        Frame::Notification {
            method: "session/update".to_string(),
            params: json!({"text": "line one\nline two"}),
        }
        .write(&mut buf)
        .unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert_eq!(text.matches('\n').count(), 1);
    }

    #[test]
    fn blank_lines_are_skipped() {
        let input = "{\"id\":1,\"method\":\"a\"}\n\n   \n{\"id\":2,\"method\":\"b\"}\n";
        let got: Vec<String> = lines(input.as_bytes()).map(|r| r.unwrap()).collect();
        assert_eq!(got.len(), 2);
    }
}
