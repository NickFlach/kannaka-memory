//! NATS real-time transport for QueenSync phase gossip.
//!
//! Implements a minimal NATS client using raw TCP (the `nats` crate is broken
//! with rand 0.9). Supports PUB/SUB for phase announcements and uses
//! per-agent subjects for last-value semantics (simulating KV).
//!
//! Subject layout:
//! - `queen.phase.<agent_id>` — each agent's latest phase (publish per agent)
//! - `queen.phase.*` — wildcard subscribe to get all phases
//! - `queen.announce` — join/leave events

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::queen::AgentPhase;

pub const DEFAULT_NATS_URL: &str = "nats://swarm.ninja-portal.com:4222";
const STREAM_NAME: &str = "QUEEN_PHASES";

/// Minimal base64 decoder (standard alphabet, with padding).
fn base64_decode(input: &str) -> Result<Vec<u8>, NatsError> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &b in input.as_bytes() {
        if b == b'=' || b == b'\n' || b == b'\r' {
            continue;
        }
        let val = TABLE.iter().position(|&c| c == b)
            .ok_or_else(|| NatsError::Protocol(format!("invalid base64 char: {}", b as char)))? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(out)
}

/// Errors from the NATS transport layer.
#[derive(Debug)]
pub enum NatsError {
    Connect(String),
    Io(std::io::Error),
    Protocol(String),
    Serialize(String),
}

impl std::fmt::Display for NatsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(msg) => write!(f, "NATS connect: {}", msg),
            Self::Io(e) => write!(f, "NATS I/O: {}", e),
            Self::Protocol(msg) => write!(f, "NATS protocol: {}", msg),
            Self::Serialize(msg) => write!(f, "NATS serialize: {}", msg),
        }
    }
}

impl std::error::Error for NatsError {}

impl From<std::io::Error> for NatsError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Parse a NATS URL into (host, port).
fn parse_nats_url(url: &str) -> Result<(String, u16), NatsError> {
    let stripped = url
        .strip_prefix("nats://")
        .unwrap_or(url);
    let parts: Vec<&str> = stripped.split(':').collect();
    match parts.len() {
        1 => Ok((parts[0].to_string(), 4222)),
        2 => {
            let port = parts[1]
                .parse::<u16>()
                .map_err(|e| NatsError::Connect(format!("invalid port: {}", e)))?;
            Ok((parts[0].to_string(), port))
        }
        _ => Err(NatsError::Connect(format!("invalid NATS URL: {}", url))),
    }
}

/// Generate a unique inbox subject.
fn new_inbox(tag: &str) -> String {
    use std::time::SystemTime;
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("_INBOX.{}.{}", tag, nonce)
}

/// A minimal synchronous NATS client.
pub struct SwarmTransport {
    stream: Arc<Mutex<TcpStream>>,
    url: String,
    next_sid: u64,
    jetstream_ok: bool,
}

impl SwarmTransport {
    /// Connect to a NATS server at the given URL.
    pub fn connect(url: &str) -> Result<Self, NatsError> {
        let (host, port) = parse_nats_url(url)?;
        let addr = format!("{}:{}", host, port);
        // Resolve hostname to socket address (DNS lookup)
        use std::net::ToSocketAddrs;
        let socket_addr = addr
            .to_socket_addrs()
            .map_err(|e| NatsError::Connect(format!("DNS resolution failed for {}: {}", addr, e)))?
            .next()
            .ok_or_else(|| NatsError::Connect(format!("no addresses found for {}", addr)))?;
        let stream = TcpStream::connect_timeout(
            &socket_addr,
            Duration::from_secs(5),
        )
        .map_err(|e| NatsError::Connect(format!("failed to connect to {}: {}", addr, e)))?;

        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;

        // Read INFO line
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut info_line = String::new();
        reader.read_line(&mut info_line)?;
        if !info_line.starts_with("INFO ") {
            return Err(NatsError::Protocol(format!(
                "expected INFO, got: {}",
                info_line.trim()
            )));
        }

        // Send CONNECT
        let connect_payload = r#"{"verbose":false,"pedantic":false,"name":"kannaka","lang":"rust","version":"0.1.0","protocol":1}"#;
        let mut stream = reader.into_inner();
        write!(stream, "CONNECT {}\r\n", connect_payload)?;
        write!(stream, "PING\r\n")?;
        stream.flush()?;

        // Read PONG
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut pong_line = String::new();
        reader.read_line(&mut pong_line)?;
        if !pong_line.trim().starts_with("PONG") && !pong_line.trim().starts_with("+OK") {
            // Some servers send +OK before PONG, try reading one more line
            let mut pong2 = String::new();
            reader.read_line(&mut pong2)?;
            if !pong2.trim().starts_with("PONG") {
                return Err(NatsError::Protocol(format!(
                    "expected PONG, got: {} / {}",
                    pong_line.trim(),
                    pong2.trim()
                )));
            }
        }

        let mut transport = Self {
            stream: Arc::new(Mutex::new(reader.into_inner())),
            url: url.to_string(),
            next_sid: 1,
            jetstream_ok: false,
        };

        // Try to ensure JetStream stream exists
        transport.jetstream_ok = transport.ensure_stream().is_ok();

        Ok(transport)
    }

    /// Connect to the default NATS URL.
    pub fn connect_default() -> Result<Self, NatsError> {
        Self::connect(DEFAULT_NATS_URL)
    }

    /// Whether JetStream is available on this connection.
    pub fn has_jetstream(&self) -> bool {
        self.jetstream_ok
    }

    /// Get the URL this transport is connected to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Ensure the QUEEN_PHASES JetStream stream exists.
    fn ensure_stream(&self) -> Result<(), NatsError> {
        let inbox = new_inbox("jscreate");
        let mut stream = self.stream.lock().map_err(|e| {
            NatsError::Protocol(format!("lock poisoned: {}", e))
        })?;

        // Subscribe to inbox for reply
        write!(stream, "SUB {} 99\r\n", inbox)?;
        stream.flush()?;

        // Send stream create request
        let create_payload = serde_json::json!({
            "name": STREAM_NAME,
            "subjects": ["queen.phase.>"],
            "retention": "limits",
            "max_msgs_per_subject": 1,
            "storage": "file",
            "discard": "old",
            "num_replicas": 1
        });
        let payload_bytes = create_payload.to_string();
        let subject = format!("$JS.API.STREAM.CREATE.{}", STREAM_NAME);
        write!(stream, "PUB {} {} {}\r\n", subject, inbox, payload_bytes.len())?;
        stream.write_all(payload_bytes.as_bytes())?;
        write!(stream, "\r\n")?;
        stream.flush()?;

        // Read reply (with timeout)
        stream.set_read_timeout(Some(Duration::from_secs(3)))?;
        let mut reader = BufReader::new(stream.try_clone().map_err(NatsError::Io)?);
        let mut got_reply = false;

        for _ in 0..10 {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.starts_with("PING") {
                        let mut s = reader.get_ref().try_clone().map_err(NatsError::Io)?;
                        write!(s, "PONG\r\n")?;
                        s.flush()?;
                        continue;
                    }
                    if trimmed == "PONG" || trimmed == "+OK" {
                        continue;
                    }
                    if trimmed.starts_with("MSG ") {
                        let parts: Vec<&str> = trimmed.split_whitespace().collect();
                        if parts.len() >= 4 {
                            let nbytes: usize = parts.last().unwrap().parse().unwrap_or(0);
                            let mut payload = vec![0u8; nbytes];
                            reader.read_exact(&mut payload).ok();
                            let mut crlf = String::new();
                            reader.read_line(&mut crlf).ok();

                            // Check for error — code 10058 means stream already exists (OK)
                            if let Ok(resp) = serde_json::from_slice::<serde_json::Value>(&payload) {
                                if let Some(err) = resp.get("error") {
                                    let code = err.get("err_code").and_then(|c| c.as_u64()).unwrap_or(0);
                                    if code == 10058 {
                                        // Stream already exists — try UPDATE instead to ensure config
                                        got_reply = true;
                                        break;
                                    }
                                    return Err(NatsError::Protocol(format!(
                                        "JetStream stream create error: {}", err
                                    )));
                                }
                            }
                            got_reply = true;
                            break;
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(NatsError::Io(e)),
            }
        }

        // Unsubscribe
        drop(reader);
        write!(stream, "UNSUB 99\r\n")?;
        stream.flush()?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;

        if got_reply {
            Ok(())
        } else {
            Err(NatsError::Protocol("no JetStream reply for stream create".into()))
        }
    }

    /// Publish a raw message to a subject with an optional reply-to.
    fn publish_raw_reply(&self, subject: &str, reply_to: Option<&str>, payload: &[u8]) -> Result<(), NatsError> {
        let mut stream = self.stream.lock().map_err(|e| {
            NatsError::Protocol(format!("lock poisoned: {}", e))
        })?;
        match reply_to {
            Some(rt) => write!(stream, "PUB {} {} {}\r\n", subject, rt, payload.len())?,
            None => write!(stream, "PUB {} {}\r\n", subject, payload.len())?,
        }
        stream.write_all(payload)?;
        write!(stream, "\r\n")?;
        stream.flush()?;
        Ok(())
    }

    /// Publish a raw message to a subject.
    fn publish_raw(&self, subject: &str, payload: &[u8]) -> Result<(), NatsError> {
        let mut stream = self.stream.lock().map_err(|e| {
            NatsError::Protocol(format!("lock poisoned: {}", e))
        })?;
        write!(stream, "PUB {} {}\r\n", subject, payload.len())?;
        stream.write_all(payload)?;
        write!(stream, "\r\n")?;
        stream.flush()?;
        Ok(())
    }

    /// Publish this agent's phase state.
    pub fn publish_phase(&self, phase: &AgentPhase) -> Result<(), NatsError> {
        let subject = format!("queen.phase.{}", phase.agent_id);
        let payload = serde_json::to_vec(phase)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw(&subject, &payload)
    }

    /// Read all current agent phases.
    ///
    /// If JetStream is available, fetches stored phases from the QUEEN_PHASES
    /// stream (reliable, no timing dependency). Falls back to legacy PUB/SUB
    /// collection if JetStream is unavailable.
    pub fn get_all_phases(&self) -> Result<Vec<AgentPhase>, NatsError> {
        if self.jetstream_ok {
            self.get_all_phases_jetstream()
        } else {
            self.get_all_phases_legacy()
        }
    }

    /// Fetch all phases from JetStream by iterating stored messages.
    fn get_all_phases_jetstream(&self) -> Result<Vec<AgentPhase>, NatsError> {
        let mut stream = self.stream.lock().map_err(|e| {
            NatsError::Protocol(format!("lock poisoned: {}", e))
        })?;

        let inbox = new_inbox("phases");
        // Subscribe to inbox
        write!(stream, "SUB {} 98\r\n", inbox)?;
        stream.flush()?;

        let mut phases: HashMap<String, AgentPhase> = HashMap::new();
        let mut next_seq: u64 = 1;

        // Iterate through stored messages using next_by_subj
        loop {
            let req = serde_json::json!({
                "seq": next_seq,
                "next_by_subj": "queen.phase.>"
            });
            let req_bytes = req.to_string();
            let get_subject = format!("$JS.API.STREAM.MSG.GET.{}", STREAM_NAME);
            write!(stream, "PUB {} {} {}\r\n", get_subject, inbox, req_bytes.len())?;
            stream.write_all(req_bytes.as_bytes())?;
            write!(stream, "\r\n")?;
            stream.flush()?;

            // Read reply
            stream.set_read_timeout(Some(Duration::from_secs(3)))?;
            let mut reader = BufReader::new(stream.try_clone().map_err(NatsError::Io)?);
            let mut got_message = false;

            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if trimmed.starts_with("PING") {
                            let mut s = reader.get_ref().try_clone().map_err(NatsError::Io)?;
                            write!(s, "PONG\r\n")?;
                            s.flush()?;
                            continue;
                        }
                        if trimmed == "PONG" || trimmed == "+OK" {
                            continue;
                        }
                        if trimmed.starts_with("MSG ") {
                            let parts: Vec<&str> = trimmed.split_whitespace().collect();
                            if parts.len() >= 4 {
                                let nbytes: usize = parts.last().unwrap().parse().unwrap_or(0);
                                let mut payload = vec![0u8; nbytes];
                                reader.read_exact(&mut payload).ok();
                                let mut crlf = String::new();
                                reader.read_line(&mut crlf).ok();

                                // Parse the JS API response
                                if let Ok(resp) = serde_json::from_slice::<serde_json::Value>(&payload) {
                                    // Check for error (no more messages)
                                    if resp.get("error").is_some() {
                                        // No more messages in stream
                                        break;
                                    }
                                    // Extract the message data (base64 encoded)
                                    if let Some(msg) = resp.get("message") {
                                        if let Some(data_b64) = msg.get("data").and_then(|d| d.as_str()) {
                                            // Decode base64 payload
                                            if let Ok(decoded) = base64_decode(data_b64) {
                                                if let Ok(phase) = serde_json::from_slice::<AgentPhase>(&decoded) {
                                                    phases.insert(phase.agent_id.clone(), phase);
                                                }
                                            }
                                        }
                                        // Get the sequence number for next iteration
                                        if let Some(seq) = msg.get("seq").and_then(|s| s.as_u64()) {
                                            next_seq = seq + 1;
                                            got_message = true;
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut
                        || e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) => {
                        drop(reader);
                        write!(stream, "UNSUB 98\r\n")?;
                        stream.flush()?;
                        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
                        return Err(NatsError::Io(e));
                    }
                }
            }

            drop(reader);
            if !got_message {
                break;
            }
        }

        // Unsubscribe
        write!(stream, "UNSUB 98\r\n")?;
        stream.flush()?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;

        Ok(phases.into_values().collect())
    }

    /// Legacy PUB/SUB phase collection (fallback when JetStream is unavailable).
    fn get_all_phases_legacy(&self) -> Result<Vec<AgentPhase>, NatsError> {
        let mut stream = self.stream.lock().map_err(|e| {
            NatsError::Protocol(format!("lock poisoned: {}", e))
        })?;

        let sid = "phase_collect";
        write!(stream, "SUB queen.phase.* {}\r\n", sid)?;
        write!(stream, "PING\r\n")?;
        stream.flush()?;

        stream.set_read_timeout(Some(Duration::from_millis(1500)))?;

        let mut phases: HashMap<String, AgentPhase> = HashMap::new();
        let mut reader = BufReader::new(stream.try_clone().map_err(NatsError::Io)?);

        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed == "PONG" {
                        continue;
                    }
                    if trimmed.starts_with("PING") {
                        let mut s = reader.get_ref().try_clone().map_err(NatsError::Io)?;
                        write!(s, "PONG\r\n")?;
                        s.flush()?;
                        continue;
                    }
                    if trimmed.starts_with("MSG ") {
                        let parts: Vec<&str> = trimmed.split_whitespace().collect();
                        if parts.len() >= 4 {
                            let nbytes: usize = parts.last().unwrap().parse().unwrap_or(0);
                            let mut payload = vec![0u8; nbytes];
                            reader.read_exact(&mut payload).ok();
                            let mut crlf = String::new();
                            reader.read_line(&mut crlf).ok();

                            if let Ok(phase) = serde_json::from_slice::<AgentPhase>(&payload) {
                                phases.insert(phase.agent_id.clone(), phase);
                            }
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut
                    || e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(NatsError::Io(e)),
            }
        }

        drop(reader);
        write!(stream, "UNSUB {}\r\n", sid)?;
        stream.flush()?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;

        Ok(phases.into_values().collect())
    }

    /// Announce joining the swarm.
    pub fn announce_join(&self, agent_id: &str) -> Result<(), NatsError> {
        let payload = serde_json::json!({
            "event": "join",
            "agent_id": agent_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let bytes = serde_json::to_vec(&payload)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw("queen.announce", &bytes)
    }

    /// Announce leaving the swarm.
    pub fn announce_leave(&self, agent_id: &str) -> Result<(), NatsError> {
        let payload = serde_json::json!({
            "event": "leave",
            "agent_id": agent_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        let bytes = serde_json::to_vec(&payload)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw("queen.announce", &bytes)
    }

    /// Subscribe to phase updates. Returns a NatsSubscription that can be iterated.
    pub fn subscribe_phases(&self) -> Result<NatsSubscription, NatsError> {
        let stream_clone = {
            let stream = self.stream.lock().map_err(|e| {
                NatsError::Protocol(format!("lock poisoned: {}", e))
            })?;
            stream.try_clone()?
        };

        let sid = "phase_listen";
        {
            let mut stream = self.stream.lock().map_err(|e| {
                NatsError::Protocol(format!("lock poisoned: {}", e))
            })?;
            write!(stream, "SUB queen.phase.* {}\r\n", sid)?;
            write!(stream, "SUB queen.announce {}\r\n", sid)?;
            stream.flush()?;
        }

        Ok(NatsSubscription {
            reader: BufReader::new(stream_clone),
            sid: sid.to_string(),
        })
    }

    /// Send a PING to check connection health.
    pub fn ping(&self) -> Result<(), NatsError> {
        let mut stream = self.stream.lock().map_err(|e| {
            NatsError::Protocol(format!("lock poisoned: {}", e))
        })?;
        write!(stream, "PING\r\n")?;
        stream.flush()?;
        Ok(())
    }

    /// Check if connection is still alive.
    pub fn is_connected(&self) -> bool {
        self.ping().is_ok()
    }
}

/// A subscription that yields NATS messages.
pub struct NatsSubscription {
    reader: BufReader<TcpStream>,
    sid: String,
}

/// A received NATS message.
pub struct NatsMessage {
    pub subject: String,
    pub payload: Vec<u8>,
}

impl NatsMessage {
    /// Try to parse the payload as an AgentPhase.
    pub fn as_phase(&self) -> Option<AgentPhase> {
        serde_json::from_slice(&self.payload).ok()
    }

    /// Try to parse as a JSON value (for announce events).
    pub fn as_json(&self) -> Option<serde_json::Value> {
        serde_json::from_slice(&self.payload).ok()
    }
}

impl NatsSubscription {
    /// Block until the next message arrives. Returns None on connection close.
    pub fn next_message(&mut self) -> Option<NatsMessage> {
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => return None,
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.starts_with("PING") {
                        // Reply to server PING
                        if let Ok(mut s) = self.reader.get_ref().try_clone() {
                            let _ = write!(s, "PONG\r\n");
                            let _ = s.flush();
                        }
                        continue;
                    }
                    if trimmed.starts_with("MSG ") {
                        let parts: Vec<&str> = trimmed.split_whitespace().collect();
                        if parts.len() >= 4 {
                            let subject = parts[1].to_string();
                            let nbytes: usize = parts.last().unwrap().parse().unwrap_or(0);
                            let mut payload = vec![0u8; nbytes];
                            if self.reader.read_exact(&mut payload).is_err() {
                                return None;
                            }
                            // Consume trailing \r\n
                            let mut crlf = String::new();
                            let _ = self.reader.read_line(&mut crlf);
                            return Some(NatsMessage { subject, payload });
                        }
                    }
                    // Skip other messages (+OK, INFO, etc.)
                }
                Err(_) => return None,
            }
        }
    }

    /// Set read timeout for the subscription stream.
    pub fn set_timeout(&self, timeout: Option<Duration>) -> Result<(), NatsError> {
        self.reader.get_ref().set_read_timeout(timeout)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_nats_url_default_port() {
        let (host, port) = parse_nats_url("nats://localhost").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 4222);
    }

    #[test]
    fn parse_nats_url_custom_port() {
        let (host, port) = parse_nats_url("nats://swarm.ninja-portal.com:4222").unwrap();
        assert_eq!(host, "swarm.ninja-portal.com");
        assert_eq!(port, 4222);
    }

    #[test]
    fn parse_nats_url_no_scheme() {
        let (host, port) = parse_nats_url("localhost:4222").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 4222);
    }

    #[test]
    fn connect_default_graceful_failure() {
        // Should fail gracefully without a live NATS server
        // (CI/local dev may not have NATS running)
        match SwarmTransport::connect("nats://127.0.0.1:19999") {
            Ok(_) => panic!("should not connect to nonexistent server"),
            Err(e) => {
                // Expected: connection refused or timeout
                let msg = format!("{}", e);
                assert!(
                    msg.contains("connect") || msg.contains("Connect"),
                    "error should mention connect: {}",
                    msg
                );
            }
        }
    }

    #[test]
    fn phase_serialization_roundtrip() {
        use chrono::Utc;
        let phase = AgentPhase {
            id: "test-id".to_string(),
            agent_id: "agent-1".to_string(),
            phase: 1.5,
            frequency: 0.5,
            coherence: 0.8,
            phi: 3.2,
            order_parameter: 0.9,
            cluster_count: 3,
            memory_count: 42,
            xi_signature: None,
            protocol_version: "1.0".to_string(),
            timestamp: Utc::now(),
            trust_score: 0.5,
            handedness: crate::queen::Handedness::Achiral,
        };
        let bytes = serde_json::to_vec(&phase).unwrap();
        let decoded: AgentPhase = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.agent_id, "agent-1");
        assert!((decoded.phase - 1.5).abs() < 0.001);
    }

    /// Integration test — only runs if NATS is actually available.
    /// Skips gracefully otherwise.
    #[test]
    fn integration_publish_and_read() {
        let transport = match SwarmTransport::connect(DEFAULT_NATS_URL) {
            Ok(t) => t,
            Err(_) => {
                eprintln!("NATS not available, skipping integration test");
                return;
            }
        };

        let phase = AgentPhase {
            id: "int-test".to_string(),
            agent_id: "test-integration".to_string(),
            phase: 2.0,
            frequency: 0.5,
            coherence: 0.7,
            phi: 1.0,
            order_parameter: 0.0,
            cluster_count: 0,
            memory_count: 0,
            xi_signature: None,
            protocol_version: "1.0".to_string(),
            timestamp: chrono::Utc::now(),
            trust_score: 0.5,
            handedness: crate::queen::Handedness::Achiral,
        };

        // Publish
        transport.publish_phase(&phase).expect("publish should work");
        transport.announce_join("test-integration").expect("announce should work");

        // Note: get_all_phases may not return our message since plain NATS
        // doesn't retain messages. This test just verifies no errors.
        let phases = transport.get_all_phases().unwrap_or_default();
        eprintln!("Got {} phases from NATS", phases.len());
    }
}
