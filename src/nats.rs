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

/// A minimal synchronous NATS client.
pub struct SwarmTransport {
    stream: Arc<Mutex<TcpStream>>,
    url: String,
    next_sid: u64,
}

impl SwarmTransport {
    /// Connect to a NATS server at the given URL.
    pub fn connect(url: &str) -> Result<Self, NatsError> {
        let (host, port) = parse_nats_url(url)?;
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect_timeout(
            &addr.parse().map_err(|e| NatsError::Connect(format!("bad address {}: {}", addr, e)))?,
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

        Ok(Self {
            stream: Arc::new(Mutex::new(reader.into_inner())),
            url: url.to_string(),
            next_sid: 1,
        })
    }

    /// Connect to the default NATS URL.
    pub fn connect_default() -> Result<Self, NatsError> {
        Self::connect(DEFAULT_NATS_URL)
    }

    /// Get the URL this transport is connected to.
    pub fn url(&self) -> &str {
        &self.url
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

    /// Read all current agent phases by subscribing to `queen.phase.*`,
    /// collecting messages for a short window, then unsubscribing.
    pub fn get_all_phases(&self) -> Result<Vec<AgentPhase>, NatsError> {
        let mut stream = self.stream.lock().map_err(|e| {
            NatsError::Protocol(format!("lock poisoned: {}", e))
        })?;

        let sid = "phase_collect";
        // Subscribe
        write!(stream, "SUB queen.phase.* {}\r\n", sid)?;
        // Flush to request all retained messages
        write!(stream, "PING\r\n")?;
        stream.flush()?;

        // Set a short read timeout for collection
        stream.set_read_timeout(Some(Duration::from_millis(1500)))?;

        let mut phases: HashMap<String, AgentPhase> = HashMap::new();
        let mut reader = BufReader::new(stream.try_clone().map_err(NatsError::Io)?);

        // Read messages until timeout or PONG
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed == "PONG" {
                        // Got our PONG back, wait a tiny bit more for messages
                        continue;
                    }
                    if trimmed.starts_with("PING") {
                        // Reply to server PING
                        let mut s = reader.get_ref().try_clone().map_err(NatsError::Io)?;
                        write!(s, "PONG\r\n")?;
                        s.flush()?;
                        continue;
                    }
                    if trimmed.starts_with("MSG ") {
                        // MSG <subject> <sid> [reply-to] <#bytes>
                        let parts: Vec<&str> = trimmed.split_whitespace().collect();
                        if parts.len() >= 4 {
                            let nbytes: usize = parts.last().unwrap().parse().unwrap_or(0);
                            let mut payload = vec![0u8; nbytes];
                            reader.read_exact(&mut payload).ok();
                            // Read trailing \r\n
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

        // Unsubscribe
        drop(reader);
        write!(stream, "UNSUB {}\r\n", sid)?;
        stream.flush()?;

        // Restore default timeout
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
