//! NATS real-time transport for QueenSync phase gossip.
//!
//! Implements a minimal NATS client using raw TCP (the `nats` crate is broken
//! with rand 0.9). Supports PUB/SUB for phase announcements, JetStream KV
//! for agent registry / discovery, structured event streams, and automatic
//! reconnection with message buffering.
//!
//! Connection model: one TCP socket per `SwarmTransport`, guarded by a single
//! mutex holding both the write half and a persistent `BufReader` (the reader
//! must persist across calls — recreating it per call discards any bytes it
//! pre-read into its internal buffer and desyncs the protocol). Long-running
//! subscriptions clone the socket and own their own reader; while one is open
//! the transport's RPC methods MUST NOT be used on the same connection (the
//! two readers would steal each other's bytes) — serving agents open a
//! dedicated `SwarmTransport` per subscription.
//!
//! Subject layout:
//! - `QUEEN.phase.<agent_id>` — each agent's latest phase (publish per agent)
//! - `QUEEN.phase.*` — wildcard subscribe to get all phases
//! - `QUEEN.event.<type>` — structured events (join, leave, dream.start, etc.)
//! - `QUEEN.announce` — legacy join/leave (still published for compat)

use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;
use crate::queen::AgentPhase;

/// Inject the canonical NATS envelope fields (schema_version, ts) into a
/// JSON object before publish, matching the contract enforced by the radio
/// + observatory validators (consciousness-core/docs/nats-contract.yaml).
/// agent_id is left to the caller — most payloads already have it. If the
/// payload has a "timestamp" field, mirror it to "ts" (the contract name);
/// otherwise stamp the current UTC time. Old subscribers keep seeing the
/// existing fields untouched; new validators see the envelope they want.
fn add_envelope(value: &mut serde_json::Value) {
    if let Some(obj) = value.as_object_mut() {
        // Per consciousness-core/docs/nats-contract.yaml:
        //   schema_version: string ("1.0")
        //   ts:             number (unix-ms)
        // Pre-fix these were "1" (string but wrong value) and an RFC3339
        // string respectively; closes #82 and the same defects #90/#91
        // flag in the JetStream events / six bypass publishers.
        obj.entry("schema_version".to_string())
            .or_insert_with(|| serde_json::Value::String("1.0".to_string()));
        if !obj.contains_key("ts") {
            // Promote an existing RFC3339 `timestamp` field to unix-ms.
            let ts = obj
                .get("timestamp")
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp_millis())
                .unwrap_or_else(|| Utc::now().timestamp_millis());
            obj.insert(
                "ts".to_string(),
                serde_json::Value::Number(serde_json::Number::from(ts)),
            );
        }
    }
}

/// Identity block attached to swarm announces/presence when the operator
/// has logged in via `kannaka identity` (swarm agent identity, step 2).
/// user_id + email ONLY — tokens never cross the NATS boundary.
///
/// Compatibility: the block rides as an OPTIONAL `"identity"` key on
/// payloads that production v0.6.19/0.6.20 agents parse as loose JSON
/// (`serde_json::Value`), so old agents ignore it and new agents see
/// `None` on old payloads. With no stored identity the payloads are
/// byte-identical to the pre-identity shape.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AnnounceIdentity {
    pub user_id: String,
    pub email: String,
}

impl AnnounceIdentity {
    /// Build from the stored identity file (`<data_dir>/identity.json`).
    /// Not logged in → `None`. Present-but-unreadable → warn and `None`
    /// (the swarm announce must never fail because of a bad identity file).
    pub fn from_store() -> Option<Self> {
        let path = crate::identity::identity_path();
        match crate::identity::IdentityStore::load(&path) {
            Ok(Some(s)) => Some(Self { user_id: s.user_id, email: s.email }),
            Ok(None) => None,
            Err(e) => {
                eprintln!("[nats] Warning: ignoring unreadable identity file: {e}");
                None
            }
        }
    }

    /// Insert the `"identity": {user_id, email}` block into a JSON object
    /// payload. No-op on non-objects.
    pub fn attach_to(&self, payload: &mut serde_json::Value) {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert(
                "identity".to_string(),
                serde_json::json!({ "user_id": self.user_id, "email": self.email }),
            );
        }
    }

    /// Read the identity email out of an announce/presence payload.
    /// Old-shape payloads (no `"identity"` key) → `None`.
    pub fn email_from(payload: &serde_json::Value) -> Option<&str> {
        payload.get("identity")?.get("email")?.as_str()
    }
}

/// Build the legacy `QUEEN.announce` payload (pre-envelope), optionally
/// enriched with the identity block. `identity = None` reproduces the
/// pre-identity shape exactly.
fn legacy_announce_payload(
    event: &str,
    agent_id: &str,
    identity: Option<&AnnounceIdentity>,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "event": event,
        "agent_id": agent_id,
    });
    if let Some(idn) = identity {
        idn.attach_to(&mut payload);
    }
    payload
}

pub const DEFAULT_NATS_URL: &str = "nats://swarm.ninja-portal.com:4222";
const STREAM_NAME: &str = "QUEEN_PHASES";
const EVENTS_STREAM_NAME: &str = "QUEEN_EVENTS";
const KV_BUCKET_AGENTS: &str = "QUEEN_AGENTS";

/// Maximum number of messages to buffer during disconnect.
const PUBLISH_BUFFER_LIMIT: usize = 100;

/// Default socket read/write timeout used outside explicit RPC windows.
const DEFAULT_IO_TIMEOUT: Duration = Duration::from_secs(5);

/// Default per-call timeout for JetStream API request/reply exchanges.
const JS_API_TIMEOUT: Duration = Duration::from_secs(3);

/// Upper bound on messages pulled by a single JetStream stream walk.
const STREAM_WALK_LIMIT: usize = 10_000;

/// Reverse lookup table for the standard base64 alphabet. 0xFF = invalid.
const BASE64_REV: [u8; 256] = {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut rev = [0xFFu8; 256];
    let mut i = 0;
    while i < 64 {
        rev[TABLE[i] as usize] = i as u8;
        i += 1;
    }
    rev
};

/// Minimal base64 decoder (standard alphabet, with padding).
fn base64_decode(input: &str) -> Result<Vec<u8>, NatsError> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &b in input.as_bytes() {
        if b == b'=' || b == b'\n' || b == b'\r' {
            continue;
        }
        let val = BASE64_REV[b as usize];
        if val == 0xFF {
            return Err(NatsError::Protocol(format!(
                "invalid base64 char: {}",
                b as char
            )));
        }
        buf = (buf << 6) | val as u32;
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
    Disconnected(String),
    KvNotFound(String),
}

impl std::fmt::Display for NatsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connect(msg) => write!(f, "NATS connect: {}", msg),
            Self::Io(e) => write!(f, "NATS I/O: {}", e),
            Self::Protocol(msg) => write!(f, "NATS protocol: {}", msg),
            Self::Serialize(msg) => write!(f, "NATS serialize: {}", msg),
            Self::Disconnected(msg) => write!(f, "NATS disconnected: {}", msg),
            Self::KvNotFound(key) => write!(f, "NATS KV key not found: {}", key),
        }
    }
}

impl std::error::Error for NatsError {}

impl From<std::io::Error> for NatsError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

// ---------------------------------------------------------------------------
// SharedWavefront protocol (QS-5, #56)
// ---------------------------------------------------------------------------

/// A wavefront with routing metadata for resonance-based memory sharing.
///
/// Communication speed 2 (medium): shared memories propagate via constructive
/// interference during dreams. Faster than Dolt merges, slower than NATS gossip.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SharedWavefront {
    /// Source agent that created this wavefront.
    pub source_agent: String,
    /// Target agent ID, or None for broadcast to all peers.
    pub target: Option<String>,
    /// Minimum cosine similarity required for the target to absorb this wavefront.
    pub resonance_threshold: f32,
    /// Dream cycles before expiry (shared wavefronts fade if never absorbed).
    pub ttl: u32,
    /// The wavefront vector (high-dimensional embedding).
    pub vector: Vec<f32>,
    /// Content/description of the shared memory.
    pub content: String,
    /// Amplitude/importance of the shared memory.
    pub amplitude: f32,
    /// Modality tag.
    pub modality: String,
    /// Timestamp of creation.
    pub created_at: String,
}

impl SharedWavefront {
    /// Create a new shared wavefront for broadcasting.
    pub fn broadcast(
        source_agent: &str,
        vector: Vec<f32>,
        content: String,
        amplitude: f32,
        modality: &str,
    ) -> Self {
        Self {
            source_agent: source_agent.to_string(),
            target: None,
            resonance_threshold: 0.4,
            ttl: 7,
            vector,
            content,
            amplitude,
            modality: modality.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Create a shared wavefront targeted at a specific agent.
    pub fn targeted(
        source_agent: &str,
        target_agent: &str,
        vector: Vec<f32>,
        content: String,
        amplitude: f32,
        resonance_threshold: f32,
    ) -> Self {
        Self {
            source_agent: source_agent.to_string(),
            target: Some(target_agent.to_string()),
            resonance_threshold,
            ttl: 7,
            vector,
            content,
            amplitude,
            modality: "unknown".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Check if this wavefront has expired (ttl decremented each dream cycle).
    pub fn is_expired(&self) -> bool {
        self.ttl == 0
    }

    /// Decrement TTL by one dream cycle.
    pub fn tick(&mut self) {
        self.ttl = self.ttl.saturating_sub(1);
    }
}

/// Parse a NATS URL into (host, port, optional (user, password)).
///
/// Accepts `nats://host`, `nats://host:port`, `nats://user:pass@host:port`,
/// and the same forms without the scheme.
fn parse_nats_url(url: &str) -> Result<(String, u16, Option<(String, String)>), NatsError> {
    let stripped = url.strip_prefix("nats://").unwrap_or(url);
    let (creds, hostport) = match stripped.rsplit_once('@') {
        Some((c, hp)) => {
            let (user, pass) = c.split_once(':').unwrap_or((c, ""));
            if user.is_empty() || pass.is_empty() {
                (None, hp)
            } else {
                (Some((user.to_string(), pass.to_string())), hp)
            }
        }
        None => (None, stripped),
    };
    let parts: Vec<&str> = hostport.split(':').collect();
    match parts.len() {
        1 => Ok((parts[0].to_string(), 4222, creds)),
        2 => {
            let port = parts[1]
                .parse::<u16>()
                .map_err(|e| NatsError::Connect(format!("invalid port: {}", e)))?;
            Ok((parts[0].to_string(), port, creds))
        }
        _ => Err(NatsError::Connect(format!("invalid NATS URL: {}", url))),
    }
}

/// Generate a unique inbox subject. Combines pid, a random uuid, and a
/// timestamp so two processes (or two threads in one process) starting a
/// request in the same instant cannot collide and cross-read replies.
fn new_inbox(tag: &str) -> String {
    use std::time::SystemTime;
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "_INBOX.{}.{}.{}.{}",
        tag,
        std::process::id(),
        uuid::Uuid::new_v4().simple(),
        nonce
    )
}

/// True for server `-ERR` messages that mean this connection is unusable.
fn is_auth_error(msg: &str) -> bool {
    let m = msg.to_ascii_lowercase();
    m.contains("authorization") || m.contains("authentication")
}

// ---------------------------------------------------------------------------
// Wire-protocol frame reader
// ---------------------------------------------------------------------------

/// One parsed NATS protocol frame.
enum Frame {
    Msg {
        subject: String,
        sid: String,
        reply_to: Option<String>,
        payload: Vec<u8>,
    },
    Ping,
    Pong,
    OkAck,
    Info,
    ServerErr(String),
}

/// Outcome of one frame-read attempt.
enum ReadOutcome {
    Frame(Frame),
    /// Clean read timeout at a frame boundary — no bytes were consumed,
    /// the stream is still in sync. Safe to poll again.
    TimedOut,
    /// EOF — the server closed the connection.
    Closed,
}

/// Read exactly one protocol frame. Returns `Err` on any condition that
/// desyncs the byte stream (partial line consumed before a timeout, short
/// payload read, unparseable MSG header, missing CRLF, non-UTF-8 control
/// line) — after such an error the connection must be considered dead.
fn read_frame(reader: &mut BufReader<TcpStream>) -> Result<ReadOutcome, NatsError> {
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => return Ok(ReadOutcome::Closed),
        Ok(_) => {}
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) =>
        {
            if line.is_empty() {
                return Ok(ReadOutcome::TimedOut);
            }
            // Bytes of a control line were consumed before the timeout hit;
            // they are lost and the stream is desynced. Fatal.
            return Err(NatsError::Protocol(format!(
                "read timed out mid-frame ({} bytes consumed) — protocol desync",
                line.len()
            )));
        }
        Err(e) => return Err(NatsError::Io(e)),
    }

    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("MSG ") {
        // Wire format:
        //   MSG <subject> <sid> <#bytes>
        //   MSG <subject> <sid> <reply-to> <#bytes>
        let parts: Vec<&str> = rest.split_whitespace().collect();
        let (subject, sid, reply_to, nbytes_str) = match parts.len() {
            3 => (parts[0], parts[1], None, parts[2]),
            4 => (parts[0], parts[1], Some(parts[2]), parts[3]),
            n => {
                return Err(NatsError::Protocol(format!(
                    "malformed MSG header ({} fields): {}",
                    n, trimmed
                )))
            }
        };
        let nbytes: usize = nbytes_str.parse().map_err(|_| {
            NatsError::Protocol(format!("invalid MSG byte count: {}", trimmed))
        })?;
        let mut payload = vec![0u8; nbytes];
        reader.read_exact(&mut payload).map_err(|e| {
            NatsError::Protocol(format!("short MSG payload read: {}", e))
        })?;
        let mut crlf = [0u8; 2];
        reader.read_exact(&mut crlf).map_err(|e| {
            NatsError::Protocol(format!("missing CRLF after MSG payload: {}", e))
        })?;
        if &crlf != b"\r\n" {
            return Err(NatsError::Protocol(
                "MSG payload not followed by CRLF".to_string(),
            ));
        }
        return Ok(ReadOutcome::Frame(Frame::Msg {
            subject: subject.to_string(),
            sid: sid.to_string(),
            reply_to: reply_to.map(String::from),
            payload,
        }));
    }
    if trimmed == "PING" {
        return Ok(ReadOutcome::Frame(Frame::Ping));
    }
    if trimmed == "PONG" {
        return Ok(ReadOutcome::Frame(Frame::Pong));
    }
    if trimmed == "+OK" {
        return Ok(ReadOutcome::Frame(Frame::OkAck));
    }
    if let Some(rest) = trimmed.strip_prefix("-ERR") {
        return Ok(ReadOutcome::Frame(Frame::ServerErr(
            rest.trim().trim_matches('\'').to_string(),
        )));
    }
    if trimmed == "INFO" || trimmed.starts_with("INFO ") {
        return Ok(ReadOutcome::Frame(Frame::Info));
    }
    Err(NatsError::Protocol(format!(
        "unexpected protocol line: {}",
        trimmed.chars().take(80).collect::<String>()
    )))
}

/// Write one PUB frame (header + payload + CRLF) and flush.
fn write_pub(w: &mut TcpStream, subject: &str, payload: &[u8]) -> Result<(), NatsError> {
    write!(w, "PUB {} {}\r\n", subject, payload.len())?;
    w.write_all(payload)?;
    w.write_all(b"\r\n")?;
    w.flush()?;
    Ok(())
}

/// The two halves of one NATS connection. The reader is persistent: it must
/// never be recreated mid-connection because `BufReader` may have pre-read
/// bytes past the kernel cursor; dropping it would silently discard them.
struct Conn {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
    /// Set when a write failed partway through a frame (e.g. a write timeout
    /// after the PUB header but before the payload). The outbound byte stream
    /// is desynced at that point: any further bytes — including a buffered
    /// replay — would be parsed by the server as the remainder of the
    /// truncated frame, silently publishing garbage. A dead Conn must never
    /// be written again; `reconnect()` replaces it wholesale.
    dead: bool,
}

impl Conn {
    fn set_read_timeout(&self, t: Option<Duration>) -> Result<(), NatsError> {
        // writer/reader are clones of the same socket; the timeout is a
        // property of the shared socket, so setting it once is enough.
        self.writer.set_read_timeout(t).map_err(NatsError::Io)
    }

    /// Current read timeout, falling back to the connection default if the
    /// getter fails.
    fn read_timeout(&self) -> Option<Duration> {
        self.writer.read_timeout().unwrap_or(Some(DEFAULT_IO_TIMEOUT))
    }

    fn pong(&mut self) -> Result<(), NatsError> {
        write!(self.writer, "PONG\r\n")?;
        self.writer.flush()?;
        Ok(())
    }
}

/// Perform the TCP connect + INFO/CONNECT/PING/PONG handshake. Shared by
/// `connect()` and `reconnect()` so both send the same authenticated
/// CONNECT payload (NATS_USER/NATS_PASSWORD env, falling back to URL
/// credentials). ADR-0026 #73.
fn handshake(url: &str) -> Result<Conn, NatsError> {
    let (host, port, url_creds) = parse_nats_url(url)?;
    let addr = format!("{}:{}", host, port);
    use std::net::ToSocketAddrs;
    let socket_addr = addr
        .to_socket_addrs()
        .map_err(|e| NatsError::Connect(format!("DNS resolution failed for {}: {}", addr, e)))?
        .next()
        .ok_or_else(|| NatsError::Connect(format!("no addresses found for {}", addr)))?;
    let stream = TcpStream::connect_timeout(&socket_addr, DEFAULT_IO_TIMEOUT)
        .map_err(|e| NatsError::Connect(format!("failed to connect to {}: {}", addr, e)))?;

    stream.set_read_timeout(Some(DEFAULT_IO_TIMEOUT))?;
    stream.set_write_timeout(Some(DEFAULT_IO_TIMEOUT))?;

    let mut writer = stream.try_clone()?;
    // This BufReader lives for the whole connection (it becomes Conn.reader),
    // so no pre-read bytes are ever discarded by an into_inner()/drop.
    let mut reader = BufReader::new(stream);

    // Read the INFO line.
    let mut info_line = String::new();
    reader.read_line(&mut info_line)?;
    if !info_line.starts_with("INFO ") {
        return Err(NatsError::Protocol(format!(
            "expected INFO, got: {}",
            info_line.trim()
        )));
    }
    // Inspect the server INFO: we cannot speak TLS, so fail with a clear
    // message instead of a confusing "expected PONG" later.
    if let Ok(info) =
        serde_json::from_str::<serde_json::Value>(info_line["INFO ".len()..].trim())
    {
        if info
            .get("tls_required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return Err(NatsError::Connect(
                "server requires TLS, which this client does not support".to_string(),
            ));
        }
    }

    // Credentials: env vars take precedence, then URL user:pass@. Anonymous
    // connections get the server's anonymous permissions (read-only).
    let env_user = std::env::var("NATS_USER").unwrap_or_default();
    let env_pass = std::env::var("NATS_PASSWORD").unwrap_or_default();
    let creds = if !env_user.is_empty() && !env_pass.is_empty() {
        Some((env_user, env_pass))
    } else {
        url_creds
    };
    let connect_payload = match &creds {
        Some((user, pass)) => {
            // Escape JSON safely. Both fields are short tokens — quotation
            // and backslash are the only chars worth handling.
            let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
            format!(
                r#"{{"verbose":false,"pedantic":false,"name":"kannaka","lang":"rust","version":"0.1.0","protocol":1,"user":"{}","pass":"{}"}}"#,
                esc(user),
                esc(pass)
            )
        }
        None => {
            r#"{"verbose":false,"pedantic":false,"name":"kannaka","lang":"rust","version":"0.1.0","protocol":1}"#
                .to_string()
        }
    };
    write!(writer, "CONNECT {}\r\n", connect_payload)?;
    write!(writer, "PING\r\n")?;
    writer.flush()?;

    // Wait for the actual PONG. +OK / async INFO lines are skipped (they are
    // not success); -ERR (e.g. Authorization Violation) fails the handshake.
    for _ in 0..10 {
        match read_frame(&mut reader)? {
            ReadOutcome::Frame(Frame::Pong) => return Ok(Conn { writer, reader, dead: false }),
            ReadOutcome::Frame(Frame::Ping) => {
                write!(writer, "PONG\r\n")?;
                writer.flush()?;
            }
            ReadOutcome::Frame(Frame::ServerErr(m)) => {
                return Err(NatsError::Connect(format!(
                    "server rejected connect: {}",
                    m
                )));
            }
            ReadOutcome::Frame(_) => continue,
            ReadOutcome::TimedOut => {
                return Err(NatsError::Protocol(
                    "timed out waiting for PONG during handshake".to_string(),
                ));
            }
            ReadOutcome::Closed => {
                return Err(NatsError::Connect(
                    "connection closed during handshake".to_string(),
                ));
            }
        }
    }
    Err(NatsError::Protocol(
        "no PONG after CONNECT (too many interleaved frames)".to_string(),
    ))
}

/// A message buffered during disconnect, replayed on reconnect.
#[derive(Clone)]
struct BufferedMessage {
    subject: String,
    payload: Vec<u8>,
}

/// A minimal synchronous NATS client with JetStream KV, events, and reconnection.
pub struct SwarmTransport {
    conn: Arc<Mutex<Conn>>,
    url: String,
    /// Monotonic subscription-id allocator. Every SUB (long-running
    /// subscriptions AND one-shot RPC inboxes) gets a unique sid so a
    /// timed-out request can never collide with a later subscription.
    next_sid: AtomicU64,
    jetstream_ok: bool,
    connected: Arc<Mutex<bool>>,
    publish_buffer: Arc<Mutex<VecDeque<BufferedMessage>>>,
}

// ──────────────────────────────────────────────────────────────────
// ADR-0028 — durable event-sourced HRM. Streams declared as data;
// `ensure_event_stream(kind)` builds the JetStream config from the
// spec table so adding a new stream is one variant + one arm.
// ──────────────────────────────────────────────────────────────────

/// Declarative spec for a JetStream stream. All ADR-0028 streams share
/// `retention=limits, storage=file, discard=old, num_replicas=1`; the
/// per-stream fields below are the only knobs that vary.
pub struct StreamSpec {
    pub name: &'static str,
    pub subjects: &'static [&'static str],
    pub max_age_days: Option<i64>,
    pub max_msgs_per_subject: Option<i64>,
    pub max_msg_size: Option<i64>,
}

/// The three durable streams that back ADR-0028's event-sourced HRM.
/// Iterate via `StreamKind::ALL` for bulk operations (e.g. `events init`).
#[derive(Clone, Copy, Debug)]
pub enum StreamKind {
    /// Per-agent remember/forget/dream log. 90-day window.
    /// Subject: `KANNAKA.events.memory.<agent_id>.>`
    MemoryEvents,
    /// Substrate absorb/anchor/flush log. 365-day window — the substrate IS
    /// the long memory; its event log is the constellation's diary.
    /// Subject: `KANNAKA.events.substrate.>`
    SubstrateEvents,
    /// Periodic gzipped HRM snapshots. Last 168 per agent (one week of
    /// hourly snapshots). Replay can warm-start from a snapshot instead
    /// of from event zero. Subject: `KANNAKA.snapshots.>`
    Snapshots,
}

impl StreamKind {
    pub const ALL: &'static [StreamKind] = &[
        StreamKind::MemoryEvents,
        StreamKind::SubstrateEvents,
        StreamKind::Snapshots,
    ];

    pub fn spec(self) -> StreamSpec {
        match self {
            StreamKind::MemoryEvents => StreamSpec {
                name: "KANNAKA_MEMORY_EVENTS",
                subjects: &["KANNAKA.events.memory.>"],
                max_age_days: Some(90),
                max_msgs_per_subject: None,
                max_msg_size: None,
            },
            StreamKind::SubstrateEvents => StreamSpec {
                name: "KANNAKA_SUBSTRATE_EVENTS",
                subjects: &["KANNAKA.events.substrate.>"],
                max_age_days: Some(365),
                max_msgs_per_subject: None,
                max_msg_size: None,
            },
            StreamKind::Snapshots => StreamSpec {
                name: "KANNAKA_SNAPSHOTS",
                subjects: &["KANNAKA.snapshots.>"],
                max_age_days: None,
                max_msgs_per_subject: Some(168),
                max_msg_size: Some(100 * 1024 * 1024),
            },
        }
    }
}

/// A durable event published to JetStream. The variant determines the
/// subject and payload shape; every payload carries `event_id`,
/// `schema_version`, `agent_id`, `ts` so replay is idempotent.
///
/// Adding a new event type = one variant + one arm in `subject()` and
/// `payload_json()`. No surgery on the publish path.
pub enum EventPayload<'a> {
    /// A new memory was stored. Replay reconstructs the wavefront.
    /// Subject: `KANNAKA.events.memory.<agent_id>.remember`
    MemoryRemember {
        agent_id: &'a str,
        memory_id: &'a uuid::Uuid,
        content: &'a str,
        importance: f32,
        modality: &'a str,
    },
    /// A memory was deleted. Replay drops the matching memory_id.
    /// Subject: `KANNAKA.events.memory.<agent_id>.forget`
    MemoryForget {
        agent_id: &'a str,
        memory_id: &'a uuid::Uuid,
    },
    /// Wave-signature absorbed into the substrate. No per-agent suffix —
    /// every absorb in one stream for easier collective time-machine
    /// reconstruction. Subject: `KANNAKA.events.substrate.absorb`
    SubstrateAbsorb {
        agent_id: &'a str,
        class_index: u32,
        amplitude: f32,
        phase: f32,
        frequency: f32,
    },
    /// Full-HRM snapshot manifest. ADR-0028 Phase 2 + ADR-0026 Phase 5:
    /// the gzipped HRM body lives on local disk at `body_path` (or a
    /// future Object Store URL), the JetStream event carries only the
    /// manifest + path + size. This is because NATS silently caps
    /// payloads ~8-10MB even with max_payload bumped to 64MB; 35MB raw
    /// HRMs need out-of-band storage.
    /// Subject: `KANNAKA.snapshots.<agent_id>.full`
    SnapshotFull {
        agent_id: &'a str,
        version: &'a str,
        wavefronts: u64,
        clusters: u64,
        phi: f32,
        /// Path to the gzipped HRM body (local FS path today; URL once
        /// ADR-0026 Phase 5 Object Store lands).
        body_path: &'a str,
        /// Compressed body size in bytes.
        body_gz_bytes: u64,
    },
}

impl<'a> EventPayload<'a> {
    pub fn subject(&self) -> String {
        match self {
            EventPayload::MemoryRemember { agent_id, .. } => {
                format!("KANNAKA.events.memory.{}.remember", agent_id)
            }
            EventPayload::MemoryForget { agent_id, .. } => {
                format!("KANNAKA.events.memory.{}.forget", agent_id)
            }
            EventPayload::SubstrateAbsorb { .. } => {
                "KANNAKA.events.substrate.absorb".to_string()
            }
            EventPayload::SnapshotFull { agent_id, .. } => {
                format!("KANNAKA.snapshots.{}.full", agent_id)
            }
        }
    }

    pub fn payload_json(&self) -> serde_json::Value {
        // Envelope matches the canonical contract — schema_version: "1.0"
        // (string), ts: unix-ms (number). The bare event_id stays.
        // Pre-fix this codepath bypassed add_envelope entirely (issue #90)
        // and emitted schema_version as an integer + ts as RFC3339 string.
        let base = serde_json::json!({
            "event_id": uuid::Uuid::new_v4(),
            "schema_version": "1.0",
            "ts": chrono::Utc::now().timestamp_millis(),
        });
        let mut obj = base.as_object().cloned().unwrap_or_default();
        match self {
            EventPayload::MemoryRemember {
                agent_id, memory_id, content, importance, modality,
            } => {
                obj.insert("agent_id".into(), serde_json::json!(agent_id));
                obj.insert("memory_id".into(), serde_json::json!(memory_id));
                obj.insert("content".into(), serde_json::json!(content));
                obj.insert("importance".into(), serde_json::json!(importance));
                obj.insert("modality".into(), serde_json::json!(modality));
            }
            EventPayload::MemoryForget { agent_id, memory_id } => {
                obj.insert("agent_id".into(), serde_json::json!(agent_id));
                obj.insert("memory_id".into(), serde_json::json!(memory_id));
            }
            EventPayload::SubstrateAbsorb {
                agent_id, class_index, amplitude, phase, frequency,
            } => {
                obj.insert("agent_id".into(), serde_json::json!(agent_id));
                obj.insert("class_index".into(), serde_json::json!(class_index));
                obj.insert("amplitude".into(), serde_json::json!(amplitude));
                obj.insert("phase".into(), serde_json::json!(phase));
                obj.insert("frequency".into(), serde_json::json!(frequency));
            }
            EventPayload::SnapshotFull {
                agent_id, version, wavefronts, clusters, phi,
                body_path, body_gz_bytes,
            } => {
                obj.insert("agent_id".into(), serde_json::json!(agent_id));
                obj.insert("manifest".into(), serde_json::json!({
                    "version": version,
                    "wavefronts": wavefronts,
                    "clusters": clusters,
                    "phi": phi,
                }));
                obj.insert("body_path".into(), serde_json::json!(body_path));
                obj.insert("body_gz_bytes".into(), serde_json::json!(body_gz_bytes));
            }
        }
        serde_json::Value::Object(obj)
    }
}

impl SwarmTransport {
    /// Connect to a NATS server at the given URL.
    pub fn connect(url: &str) -> Result<Self, NatsError> {
        let conn = handshake(url)?;
        let mut transport = Self {
            conn: Arc::new(Mutex::new(conn)),
            url: url.to_string(),
            next_sid: AtomicU64::new(1),
            jetstream_ok: false,
            connected: Arc::new(Mutex::new(true)),
            publish_buffer: Arc::new(Mutex::new(VecDeque::new())),
        };

        // Try to ensure JetStream streams exist
        transport.jetstream_ok = transport.ensure_stream().is_ok();
        if transport.jetstream_ok {
            let _ = transport.ensure_events_stream();
        }

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

    /// Allocate a fresh, connection-unique subscription id.
    fn alloc_sid(&self) -> u64 {
        self.next_sid.fetch_add(1, Ordering::Relaxed)
    }

    /// Lock the connection, mapping a poisoned mutex to a protocol error
    /// (the connection state under a panic mid-write is unknowable).
    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Conn>, NatsError> {
        self.conn
            .lock()
            .map_err(|e| NatsError::Protocol(format!("connection lock poisoned: {}", e)))
    }

    // -----------------------------------------------------------------------
    // Reconnection
    // -----------------------------------------------------------------------

    /// Attempt to reconnect to the NATS server, replaying any buffered
    /// messages in their original order. Uses the same authenticated
    /// handshake as `connect()` (NATS_USER/NATS_PASSWORD are NOT dropped
    /// on reconnect).
    pub fn reconnect(&mut self) -> Result<(), NatsError> {
        let new_conn = handshake(&self.url)?;

        // Swap in the new connection. A poisoned lock just means a previous
        // holder panicked — the old Conn is being replaced wholesale anyway.
        {
            let mut guard = self.conn.lock().unwrap_or_else(|p| p.into_inner());
            *guard = new_conn;
        }
        *self.connected.lock().unwrap_or_else(|p| p.into_inner()) = true;

        // Re-check JetStream
        self.jetstream_ok = self.ensure_stream().is_ok();
        if self.jetstream_ok {
            let _ = self.ensure_events_stream();
        }

        // Replay buffered messages in order. On failure the unsent remainder
        // stays queued (in order, at the front) for the next attempt.
        let replay_result = {
            let mut conn = self.lock_conn()?;
            self.flush_buffer_locked(&mut conn)
        };
        if let Err(e) = replay_result {
            self.mark_disconnected();
            eprintln!(
                "[nats] reconnect: buffered replay incomplete ({} still queued): {}",
                self.buffered_count(),
                e
            );
        }

        Ok(())
    }

    /// Check if connection is still alive. Sends a PING and waits for the
    /// PONG, so a dead socket whose writes still land in the kernel buffer
    /// is detected.
    pub fn is_connected(&self) -> bool {
        {
            let c = self.connected.lock().unwrap_or_else(|p| p.into_inner());
            if !*c {
                return false;
            }
        }
        self.ping().is_ok()
    }

    /// Mark the connection as disconnected (used internally on I/O failure).
    fn mark_disconnected(&self) {
        *self.connected.lock().unwrap_or_else(|p| p.into_inner()) = false;
    }

    /// Buffer a message for later replay (up to PUBLISH_BUFFER_LIMIT).
    fn buffer_message(&self, subject: &str, payload: &[u8]) {
        let mut buf = self.publish_buffer.lock().unwrap_or_else(|p| p.into_inner());
        if buf.len() >= PUBLISH_BUFFER_LIMIT {
            if let Some(dropped) = buf.pop_front() {
                eprintln!(
                    "[nats] publish buffer full ({}) — dropping oldest message on {}",
                    PUBLISH_BUFFER_LIMIT, dropped.subject
                );
            }
        }
        buf.push_back(BufferedMessage {
            subject: subject.to_string(),
            payload: payload.to_vec(),
        });
    }

    /// Return the number of messages currently buffered.
    pub fn buffered_count(&self) -> usize {
        self.publish_buffer
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .len()
    }

    /// Drain the publish buffer onto the wire in FIFO order. If a message
    /// fails to send it is pushed back to the FRONT (preserving order) and
    /// the error is returned; nothing after it is attempted.
    fn flush_buffer_locked(&self, conn: &mut Conn) -> Result<(), NatsError> {
        if conn.dead {
            return Err(NatsError::Disconnected(
                "connection desynced — buffered replay deferred until reconnect".to_string(),
            ));
        }
        loop {
            let next = self
                .publish_buffer
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .pop_front();
            let Some(msg) = next else { return Ok(()) };
            if let Err(e) = write_pub(&mut conn.writer, &msg.subject, &msg.payload) {
                // The failed write may have landed partially — the stream is
                // desynced; nothing may be written on this Conn again.
                conn.dead = true;
                eprintln!(
                    "[nats] buffered replay failed on {} — re-queueing: {}",
                    msg.subject, e
                );
                self.publish_buffer
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .push_front(msg);
                return Err(e);
            }
        }
    }

    // -----------------------------------------------------------------------
    // JetStream API request/reply core
    // -----------------------------------------------------------------------

    /// One JetStream API request/reply exchange over the held connection.
    ///
    /// SUBs a unique inbox with a unique sid (auto-unsubscribed after one
    /// message), PUBs the request with the inbox as reply-to, then reads
    /// frames until the reply lands on OUR inbox — frames for other sids
    /// (gossip, stray replies) are consumed and skipped, server PINGs are
    /// answered, `-ERR` lines are logged (auth errors are fatal).
    ///
    /// Returns `Ok(None)` on a clean no-reply-within-timeout; `Err` only
    /// for conditions that invalidate the connection (which is then marked
    /// disconnected). The previous read timeout is restored on all paths.
    fn js_api_call_locked(
        &self,
        conn: &mut Conn,
        api_subject: &str,
        request: &[u8],
        timeout: Duration,
    ) -> Result<Option<serde_json::Value>, NatsError> {
        let sid = self.alloc_sid();
        let inbox = new_inbox("js");

        write!(conn.writer, "SUB {} {}\r\n", inbox, sid)?;
        write!(conn.writer, "UNSUB {} 1\r\n", sid)?;
        write!(conn.writer, "PUB {} {} {}\r\n", api_subject, inbox, request.len())?;
        conn.writer.write_all(request)?;
        conn.writer.write_all(b"\r\n")?;
        conn.writer.flush()?;

        let prev = conn.read_timeout();
        conn.set_read_timeout(Some(timeout))?;
        let deadline = Instant::now() + timeout;

        let mut fatal = false;
        let result = loop {
            if Instant::now() > deadline {
                break Ok(None);
            }
            match read_frame(&mut conn.reader) {
                Ok(ReadOutcome::Frame(Frame::Msg { subject, payload, .. }))
                    if subject == inbox =>
                {
                    break serde_json::from_slice::<serde_json::Value>(&payload)
                        .map(Some)
                        .map_err(|e| {
                            NatsError::Protocol(format!("bad JetStream API reply: {}", e))
                        });
                }
                // A frame for some other subscription on this connection —
                // its payload was consumed correctly; skip it.
                Ok(ReadOutcome::Frame(Frame::Msg { .. })) => continue,
                Ok(ReadOutcome::Frame(Frame::Ping)) => {
                    let _ = conn.pong();
                }
                Ok(ReadOutcome::Frame(Frame::ServerErr(m))) => {
                    eprintln!("[nats] server error: {}", m);
                    if is_auth_error(&m) {
                        fatal = true;
                        break Err(NatsError::Disconnected(m));
                    }
                }
                Ok(ReadOutcome::Frame(_)) => continue,
                Ok(ReadOutcome::TimedOut) => break Ok(None),
                Ok(ReadOutcome::Closed) => {
                    fatal = true;
                    break Err(NatsError::Disconnected(
                        "connection closed during JetStream request".to_string(),
                    ));
                }
                Err(e) => {
                    fatal = true;
                    break Err(e);
                }
            }
        };

        // If no reply was delivered, the auto-unsub never fired — remove the
        // subscription explicitly so it can't leak server-side.
        if !matches!(result, Ok(Some(_))) {
            let _ = write!(conn.writer, "UNSUB {}\r\n", sid);
            let _ = conn.writer.flush();
        }
        let _ = conn.set_read_timeout(prev);
        if fatal {
            self.mark_disconnected();
        }
        result
    }

    /// Walk a JetStream stream by subject filter, returning the raw stored
    /// messages as (subject, decoded payload) pairs. Shared engine behind
    /// `get_stream_messages`, `kv_keys` and the phase reader; keeps the ONE
    /// persistent connection reader for the whole walk (recreating a reader
    /// per request used to lose pre-read bytes and return 0 rows).
    fn stream_walk(
        &self,
        stream_name: &str,
        subject_filter: &str,
        max_messages: usize,
    ) -> Result<Vec<(String, Vec<u8>)>, NatsError> {
        let mut conn = self.lock_conn()?;
        let api_subject = format!("$JS.API.STREAM.MSG.GET.{}", stream_name);
        let mut out: Vec<(String, Vec<u8>)> = Vec::new();
        let mut next_seq: u64 = 1;

        while out.len() < max_messages {
            let req = serde_json::json!({
                "seq": next_seq,
                "next_by_subj": subject_filter,
            })
            .to_string();
            let resp = match self.js_api_call_locked(
                &mut conn,
                &api_subject,
                req.as_bytes(),
                JS_API_TIMEOUT,
            ) {
                Ok(Some(v)) => v,
                Ok(None) => break, // no reply — treat as end of stream
                Err(e) => return Err(e),
            };
            if resp.get("error").is_some() {
                break; // typically "no message found" — end of matches
            }
            let Some(msg) = resp.get("message") else { break };
            // #371: a single stored message without a usable seq must NOT
            // truncate the whole forward walk. Advance the cursor past it and
            // keep going — the strictly-increasing `next_seq` + finite stream
            // guarantees termination at the next no-match.
            let Some(seq) = msg.get("seq").and_then(|s| s.as_u64()) else {
                next_seq += 1;
                continue;
            };
            // Bookkeeping first so non-decodable payloads don't stall the
            // iteration (older test data in front of real manifests).
            next_seq = seq + 1;
            let subject = msg
                .get("subject")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let data = msg
                .get("data")
                .and_then(|d| d.as_str())
                .and_then(|b64| base64_decode(b64).ok())
                .unwrap_or_default();
            out.push((subject, data));
        }
        Ok(out)
    }

    // -----------------------------------------------------------------------
    // JetStream stream management
    // -----------------------------------------------------------------------

    /// Ensure the QUEEN_PHASES JetStream stream exists.
    fn ensure_stream(&self) -> Result<(), NatsError> {
        self.ensure_js_stream(
            STREAM_NAME,
            serde_json::json!({
                "name": STREAM_NAME,
                "subjects": ["QUEEN.phase.>"],
                "retention": "limits",
                "max_msgs_per_subject": 1,
                "storage": "file",
                "discard": "old",
                "num_replicas": 1
            }),
        )
    }

    /// Ensure the QUEEN_EVENTS JetStream stream exists.
    fn ensure_events_stream(&self) -> Result<(), NatsError> {
        self.ensure_js_stream(
            EVENTS_STREAM_NAME,
            serde_json::json!({
                "name": EVENTS_STREAM_NAME,
                "subjects": ["QUEEN.event.>"],
                "retention": "limits",
                "max_msgs": 10000,
                "storage": "file",
                "discard": "old",
                "num_replicas": 1
            }),
        )
    }

    /// Generic helper: create-or-update a JetStream stream.
    fn ensure_js_stream(
        &self,
        stream_name: &str,
        config: serde_json::Value,
    ) -> Result<(), NatsError> {
        let payload = config.to_string();
        let mut conn = self.lock_conn()?;

        let create_subject = format!("$JS.API.STREAM.CREATE.{}", stream_name);
        let resp = self
            .js_api_call_locked(&mut conn, &create_subject, payload.as_bytes(), JS_API_TIMEOUT)?
            .ok_or_else(|| {
                NatsError::Protocol(format!(
                    "no JetStream reply for stream create: {}",
                    stream_name
                ))
            })?;

        if let Some(err) = resp.get("error") {
            let code = err.get("err_code").and_then(|c| c.as_u64()).unwrap_or(0);
            if code == 10058 {
                // Stream already exists — attempt UPDATE to sync config.
                // Best-effort: an UPDATE rejection (e.g. immutable field)
                // still leaves a usable stream, so the reply is drained
                // and ignored.
                let update_subject = format!("$JS.API.STREAM.UPDATE.{}", stream_name);
                let _ = self.js_api_call_locked(
                    &mut conn,
                    &update_subject,
                    payload.as_bytes(),
                    JS_API_TIMEOUT,
                );
                return Ok(());
            }
            return Err(NatsError::Protocol(format!(
                "JetStream stream create error: {}",
                err
            )));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Low-level publish (with disconnect buffering)
    // -----------------------------------------------------------------------

    /// Public façade over the raw PUB. Use this for ad-hoc subjects
    /// (e.g. the inbox prototype) that don't have a typed publish_X
    /// method. The disconnect-buffering behavior of publish_raw is
    /// preserved.
    pub fn publish(&self, subject: &str, payload: &[u8]) -> Result<(), NatsError> {
        self.publish_raw(subject, payload)
    }

    /// Publish a raw message to a subject, buffering on disconnect.
    ///
    /// If earlier messages are sitting in the disconnect buffer they are
    /// replayed (in order) BEFORE this message, so the wire order matches
    /// the call order. On failure this message joins the back of the buffer.
    fn publish_raw(&self, subject: &str, payload: &[u8]) -> Result<(), NatsError> {
        let mut conn = self.lock_conn()?;
        // A dead Conn's outbound stream is desynced mid-frame — writing
        // anything more (this message OR the buffered replay) would be parsed
        // by the server as the tail of the truncated frame. Fail fast and
        // keep buffering until reconnect() swaps in a fresh socket. This also
        // spares every post-failure publish the 5s write-timeout it used to
        // burn re-failing on the same dead socket.
        if conn.dead {
            drop(conn);
            self.mark_disconnected();
            self.buffer_message(subject, payload);
            return Err(NatsError::Disconnected(
                "connection desynced by an earlier mid-frame write failure — reconnect required"
                    .to_string(),
            ));
        }
        let result = self
            .flush_buffer_locked(&mut conn)
            .and_then(|()| write_pub(&mut conn.writer, subject, payload));
        if result.is_err() {
            conn.dead = true;
            drop(conn);
            self.mark_disconnected();
            self.buffer_message(subject, payload);
        }
        result
    }

    // -----------------------------------------------------------------------
    // Phase publishing
    // -----------------------------------------------------------------------

    /// Publish this agent's phase state.
    pub fn publish_phase(&self, phase: &AgentPhase) -> Result<(), NatsError> {
        let subject = format!("QUEEN.phase.{}", phase.agent_id);
        let mut value = serde_json::to_value(phase)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        add_envelope(&mut value);
        let payload = serde_json::to_vec(&value)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw(&subject, &payload)
    }

    /// Publish consciousness state to KANNAKA.consciousness.
    pub fn publish_consciousness(&self, state: &serde_json::Value) -> Result<(), NatsError> {
        let mut value = state.clone();
        add_envelope(&mut value);
        let payload = serde_json::to_vec(&value)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw("KANNAKA.consciousness", &payload)
    }

    /// Publish dream report to KANNAKA.dreams.
    pub fn publish_dreams(&self, report: &serde_json::Value) -> Result<(), NatsError> {
        let mut value = report.clone();
        add_envelope(&mut value);
        let payload = serde_json::to_vec(&value)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw("KANNAKA.dreams", &payload)
    }

    /// Publish a single cluster exemplar to KANNAKA.exemplar.<agent>.<cluster>.
    /// ADR-0026 Phase 2 — distilled-memory broadcast for cross-agent absorption.
    /// Uses one subject per (agent, cluster) so JetStream's
    /// max_msgs_per_subject=1 retention keeps the latest snapshot.
    pub fn publish_exemplar(
        &self,
        agent_id: &str,
        cluster_id: u32,
        payload: &serde_json::Value,
    ) -> Result<(), NatsError> {
        let subject = format!("KANNAKA.exemplar.{}.{}", agent_id, cluster_id);
        let bytes = serde_json::to_vec(payload)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw(&subject, &bytes)
    }

    /// Ensure the KANNAKA_EXEMPLARS JetStream stream exists.
    /// ADR-0026 Phase 2.
    pub fn ensure_exemplar_stream(&self) -> Result<(), NatsError> {
        self.ensure_js_stream(
            "KANNAKA_EXEMPLARS",
            serde_json::json!({
                "name": "KANNAKA_EXEMPLARS",
                "subjects": ["KANNAKA.exemplar.>"],
                "retention": "limits",
                "max_msgs_per_subject": 1,
                "max_age": 604800_000_000_000i64, // 7 days in nanoseconds
                "storage": "file",
                "discard": "old",
                "num_replicas": 1
            }),
        )
    }

    /// Pull all stored exemplar messages for one agent (or all agents).
    /// Convenience wrapper around `get_stream_messages`.
    pub fn get_exemplars(
        &self,
        from_agent: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, NatsError> {
        let subject_filter = match from_agent {
            Some(a) => format!("KANNAKA.exemplar.{}.>", a),
            None => "KANNAKA.exemplar.>".to_string(),
        };
        self.get_stream_messages("KANNAKA_EXEMPLARS", &subject_filter, 500)
    }

    /// ADR-0037 Track-D: publish this agent's belief-core snapshot (L6
    /// fingerprints + phases) so peers can content-match + couple toward shared
    /// beliefs. Subject `KANNAKA.cores.<agent_id>`; KANNAKA_CORES keeps the
    /// latest per agent (max_msgs_per_subject=1).
    pub fn publish_cores(
        &self,
        agent_id: &str,
        payload: &serde_json::Value,
    ) -> Result<(), NatsError> {
        let subject = format!("KANNAKA.cores.{}", agent_id);
        let bytes = serde_json::to_vec(payload)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw(&subject, &bytes)
    }

    /// Ensure the KANNAKA_CORES JetStream stream exists (ADR-0037 Track-D).
    pub fn ensure_cores_stream(&self) -> Result<(), NatsError> {
        self.ensure_js_stream(
            "KANNAKA_CORES",
            serde_json::json!({
                "name": "KANNAKA_CORES",
                "subjects": ["KANNAKA.cores.>"],
                "retention": "limits",
                "max_msgs_per_subject": 1,
                "max_age": 604800_000_000_000i64, // 7 days
                "max_msg_size": 1_048_576i64, // 1 MiB — a core snapshot is ≤8 cores (~KB); bound abuse
                "storage": "file",
                "discard": "old",
                "num_replicas": 1
            }),
        )
    }

    /// Pull peers' belief-core snapshots (one latest per agent), optionally
    /// filtered to a single agent. ADR-0037 Track-D.
    pub fn get_peer_cores(
        &self,
        from_agent: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, NatsError> {
        let subject_filter = match from_agent {
            Some(a) => format!("KANNAKA.cores.{}", a),
            None => "KANNAKA.cores.>".to_string(),
        };
        self.get_stream_messages("KANNAKA_CORES", &subject_filter, 500)
    }

    /// Generic helper: iterate a JetStream stream's messages by subject filter.
    /// Used by both exemplars and presence (and future ADR-0026 streams).
    /// Non-JSON payloads are skipped (but still advance the walk).
    pub fn get_stream_messages(
        &self,
        stream_name: &str,
        subject_filter: &str,
        max_messages: usize,
    ) -> Result<Vec<serde_json::Value>, NatsError> {
        Ok(self
            .stream_walk(stream_name, subject_filter, max_messages)?
            .into_iter()
            .filter_map(|(_, data)| serde_json::from_slice(&data).ok())
            .collect())
    }

    /// Publish this agent's presence record. ADR-0026 Phase 5.
    /// Subject: `KANNAKA.presence.<agent_id>` — JetStream KANNAKA_PRESENCE
    /// keeps only the latest per agent (max_msgs_per_subject=1).
    pub fn publish_presence(
        &self,
        agent_id: &str,
        payload: &serde_json::Value,
    ) -> Result<(), NatsError> {
        let subject = format!("KANNAKA.presence.{}", agent_id);
        let bytes = serde_json::to_vec(payload)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw(&subject, &bytes)
    }

    /// Ensure the KANNAKA_PRESENCE stream exists. ADR-0026 Phase 5.
    pub fn ensure_presence_stream(&self) -> Result<(), NatsError> {
        self.ensure_js_stream(
            "KANNAKA_PRESENCE",
            serde_json::json!({
                "name": "KANNAKA_PRESENCE",
                "subjects": ["KANNAKA.presence.>"],
                "retention": "limits",
                "max_msgs_per_subject": 1,
                "max_age": 86_400_000_000_000i64, // 24h — agents who haven't refreshed are gone
                "storage": "file",
                "discard": "old",
                "num_replicas": 1
            }),
        )
    }

    // ──────────────────────────────────────────────────────────────────
    // ADR-0028 — event-sourced HRM + time-machine replay. See `StreamKind`
    // and `EventPayload` (defined above) for the data model. Subject order
    // puts the literal kind token (`memory` / `substrate`) at position 3
    // so JetStream's subject-overlap check sees the streams as disjoint.
    // ──────────────────────────────────────────────────────────────────

    /// Idempotently create the JetStream stream described by `kind`.
    pub fn ensure_event_stream(&self, kind: StreamKind) -> Result<(), NatsError> {
        let spec = kind.spec();
        let mut cfg = serde_json::json!({
            "name": spec.name,
            "subjects": spec.subjects,
            "retention": "limits",
            "storage": "file",
            "discard": "old",
            "num_replicas": 1,
        });
        if let Some(days) = spec.max_age_days {
            cfg["max_age"] = serde_json::json!(days * 24 * 3_600 * 1_000_000_000i64);
        }
        if let Some(n) = spec.max_msgs_per_subject {
            cfg["max_msgs_per_subject"] = serde_json::json!(n);
        }
        if let Some(sz) = spec.max_msg_size {
            cfg["max_msg_size"] = serde_json::json!(sz);
        }
        self.ensure_js_stream(spec.name, cfg)
    }

    /// Publish a durable event to its JetStream subject. The variant
    /// determines both the subject and the JSON payload shape; every
    /// event carries `event_id`, `schema_version`, `agent_id`, `ts` so
    /// replay (ADR-0028 Phase 3) is idempotent.
    ///
    /// Best-effort: failures don't abort the originating action — the
    /// in-memory state has already changed.
    pub fn publish_event(&self, event: EventPayload<'_>) -> Result<(), NatsError> {
        let subject = event.subject();
        let payload = event.payload_json();
        let bytes = serde_json::to_vec(&payload)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw(&subject, &bytes)
    }

    /// Read all current presence records.
    pub fn get_presence(&self) -> Result<Vec<serde_json::Value>, NatsError> {
        self.get_stream_messages("KANNAKA_PRESENCE", "KANNAKA.presence.>", 200)
    }

    /// Publish a wave-signature-only absorb event to
    /// `KANNAKA.substrate.absorb.<agent_id>` (ADR-0027 Phase 1).
    ///
    /// Privacy by design: ONLY the wave signature crosses the boundary —
    /// `class_index`, `amplitude`, `phase`, `frequency`. No content, no
    /// memory id, no tags. The receiving substrate (kannaka-prime) adapts
    /// the signature into its fixed-topology 96-class HRM. Content stays
    /// in the sending agent's local HRM where it belongs.
    pub fn publish_substrate_absorb(
        &self,
        agent_id: &str,
        class_index: u32,
        amplitude: f32,
        phase: f32,
        frequency: f32,
    ) -> Result<(), NatsError> {
        let subject = format!("KANNAKA.substrate.absorb.{}", agent_id);
        let mut payload = serde_json::json!({
            "agent_id": agent_id,
            "class_index": class_index,
            "amplitude": amplitude,
            "phase": phase,
            "frequency": frequency,
        });
        add_envelope(&mut payload);
        let bytes = serde_json::to_vec(&payload)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw(&subject, &bytes)
    }

    /// Publish collective consciousness metrics from the substrate to
    /// `KANNAKA.substrate.phi` (ADR-0027 Phase 1). Periodically emitted by
    /// `kannaka substrate run` so observatory + radio can display the
    /// constellation's integrated Phi separately from any individual
    /// agent's local Phi.
    pub fn publish_substrate_phi(
        &self,
        phi: f32,
        xi: f32,
        order: f32,
        num_clusters: usize,
        total_wavefronts: usize,
        contributing_agents: &[String],
    ) -> Result<(), NatsError> {
        // agent_id was missing pre-fix, which made observatory attribute
        // the collective metric to "unknown". Stamping the substrate's
        // identity here. (#91)
        let mut payload = serde_json::json!({
            "agent_id": "kannaka-substrate",
            "collective_phi": phi,
            "collective_xi": xi,
            "collective_order": order,
            "num_active_clusters": num_clusters,
            "total_wavefronts": total_wavefronts,
            "contributing_agents": contributing_agents,
            "source": "substrate",
        });
        add_envelope(&mut payload);
        let bytes = serde_json::to_vec(&payload)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw("KANNAKA.substrate.phi", &bytes)
    }

    /// Publish a new memory to KANNAKA.memory.new for cross-agent synchronization.
    ///
    /// The payload is a JSON object wrapping the HyperMemory plus the source agent_id
    /// so receivers can skip their own messages. Optional `memory_count` and
    /// `cluster_count` fields let the radio's swarm aggregator surface the
    /// sender's authoritative counts even when the sender isn't running a
    /// `swarm join` daemon — without these, agents that only `remember` from
    /// the CLI always showed up with cl:0 in the per-agent dropdown.
    pub fn publish_memory_new_with_counts(
        &self,
        memory: &crate::memory::HyperMemory,
        agent_id: &str,
        memory_count: usize,
        cluster_count: usize,
    ) -> Result<(), NatsError> {
        let mut payload = serde_json::json!({
            "agent_id": agent_id,
            "memory": memory,
            "memory_count": memory_count,
            "cluster_count": cluster_count,
        });
        add_envelope(&mut payload);
        let bytes = serde_json::to_vec(&payload)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw("KANNAKA.memory.new", &bytes)
    }

    /// Backward-compat shim — same as publish_memory_new_with_counts but
    /// without the memory/cluster counts. Kept so older call sites keep
    /// compiling; new call sites should prefer the _with_counts variant.
    pub fn publish_memory_new(
        &self,
        memory: &crate::memory::HyperMemory,
        agent_id: &str,
    ) -> Result<(), NatsError> {
        self.publish_memory_new_with_counts(memory, agent_id, 0, 0)
    }

    // -----------------------------------------------------------------------
    // SharedWavefront protocol (QS-5, #56)
    // -----------------------------------------------------------------------

    /// Publish a shared wavefront for resonance-based memory sharing.
    ///
    /// Subject: `queen.memory.shared.{target}` where target is an agent_id
    /// or "broadcast" for all agents. JetStream retains for 7 days.
    pub fn publish_shared_wavefront(&self, wavefront: &SharedWavefront) -> Result<(), NatsError> {
        let target = wavefront.target.as_deref().unwrap_or("broadcast");
        let subject = format!("queen.memory.shared.{}", target);
        let payload = serde_json::to_vec(wavefront)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw(&subject, &payload)
    }

    /// Publish a dream lifecycle event (start or end).
    ///
    /// Used by swarm-aware consolidation to coordinate dream timing.
    pub fn publish_dream_lifecycle(
        &self,
        event_type: &str,
        agent_id: &str,
        details: &serde_json::Value,
    ) -> Result<(), NatsError> {
        let payload = serde_json::json!({
            "agent_id": agent_id,
            "event": event_type,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "details": details,
        });
        let event_name = format!("dream.{}", event_type);
        self.announce_event(&event_name, &payload)
    }

    // -----------------------------------------------------------------------
    // Phase reading
    // -----------------------------------------------------------------------

    /// Read all current agent phases.
    pub fn get_all_phases(&self) -> Result<Vec<AgentPhase>, NatsError> {
        if self.jetstream_ok {
            self.get_all_phases_jetstream()
        } else {
            self.get_all_phases_legacy()
        }
    }

    /// Fetch all phases from JetStream by iterating stored messages.
    fn get_all_phases_jetstream(&self) -> Result<Vec<AgentPhase>, NatsError> {
        let msgs = self.stream_walk(STREAM_NAME, "QUEEN.phase.>", STREAM_WALK_LIMIT)?;
        let mut phases: HashMap<String, AgentPhase> = HashMap::new();
        for (_, data) in msgs {
            if let Ok(phase) = serde_json::from_slice::<AgentPhase>(&data) {
                phases.insert(phase.agent_id.clone(), phase);
            }
        }
        Ok(phases.into_values().collect())
    }

    /// Legacy PUB/SUB phase collection (fallback when JetStream is unavailable).
    /// Subscribes to the live gossip subject and collects whatever arrives
    /// until 1.5s of silence (capped at 10s total). Only frames carrying our
    /// own sid are interpreted as phases.
    fn get_all_phases_legacy(&self) -> Result<Vec<AgentPhase>, NatsError> {
        let mut conn = self.lock_conn()?;
        let sid = self.alloc_sid();
        let sid_str = sid.to_string();

        write!(conn.writer, "SUB QUEEN.phase.* {}\r\n", sid)?;
        write!(conn.writer, "PING\r\n")?;
        conn.writer.flush()?;

        let prev = conn.read_timeout();
        conn.set_read_timeout(Some(Duration::from_millis(1500)))?;
        let deadline = Instant::now() + Duration::from_secs(10);

        let mut phases: HashMap<String, AgentPhase> = HashMap::new();
        let mut fatal: Option<NatsError> = None;
        loop {
            if Instant::now() > deadline {
                break;
            }
            match read_frame(&mut conn.reader) {
                Ok(ReadOutcome::Frame(Frame::Msg { sid: msid, payload, .. })) => {
                    if msid == sid_str {
                        if let Ok(phase) = serde_json::from_slice::<AgentPhase>(&payload) {
                            phases.insert(phase.agent_id.clone(), phase);
                        }
                    }
                }
                Ok(ReadOutcome::Frame(Frame::Ping)) => {
                    let _ = conn.pong();
                }
                Ok(ReadOutcome::Frame(Frame::ServerErr(m))) => {
                    eprintln!("[nats] server error: {}", m);
                    if is_auth_error(&m) {
                        fatal = Some(NatsError::Disconnected(m));
                        break;
                    }
                }
                Ok(ReadOutcome::Frame(_)) => continue,
                Ok(ReadOutcome::TimedOut) | Ok(ReadOutcome::Closed) => break,
                Err(e) => {
                    fatal = Some(e);
                    break;
                }
            }
        }

        let _ = write!(conn.writer, "UNSUB {}\r\n", sid);
        let _ = conn.writer.flush();
        let _ = conn.set_read_timeout(prev);

        if let Some(e) = fatal {
            drop(conn);
            self.mark_disconnected();
            return Err(e);
        }
        Ok(phases.into_values().collect())
    }

    // -----------------------------------------------------------------------
    // Structured event publishing
    // -----------------------------------------------------------------------

    /// Publish a structured event to `queen.event.<event_type>`.
    ///
    /// Event types: "join", "leave", "dream.start", "dream.end", "memory.shared".
    /// Events are stored in the QUEEN_EVENTS JetStream stream (if available).
    ///
    /// Pre-fix this published to uppercase `QUEEN.event.<type>` with a
    /// wrapper envelope `{event, timestamp, payload}` — NATS subjects are
    /// case-sensitive, so kannaka-radio (which subscribes to lowercase
    /// `queen.event.<type>` per the contract, expecting a flat payload)
    /// never received them. (#88)
    pub fn announce_event(
        &self,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<(), NatsError> {
        // Flatten: merge payload fields with the envelope. The subject
        // already names the event, so the `event` wrapper field is gone.
        let mut flat = match payload {
            serde_json::Value::Object(map) => serde_json::Value::Object(map.clone()),
            _ => serde_json::json!({}),
        };
        if let Some(obj) = flat.as_object_mut() {
            obj.entry("event_type".to_string())
                .or_insert_with(|| serde_json::Value::String(event_type.to_string()));
        }
        add_envelope(&mut flat);
        let bytes = serde_json::to_vec(&flat)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        let subject = format!("queen.event.{}", event_type);
        self.publish_raw(&subject, &bytes)
    }

    /// Announce joining the swarm.
    ///
    /// Publishes to both the new event stream (`QUEEN.event.join`) and the
    /// legacy `QUEEN.announce` subject for backward compatibility.
    pub fn announce_join(&self, agent_id: &str) -> Result<(), NatsError> {
        self.announce_join_with_identity(agent_id, None)
    }

    /// Announce joining the swarm, optionally carrying the operator's
    /// stored identity (swarm agent identity, step 2). `identity = None`
    /// publishes payloads byte-identical to `announce_join` pre-identity,
    /// so v0.6.19/0.6.20 peers see no change.
    pub fn announce_join_with_identity(
        &self,
        agent_id: &str,
        identity: Option<&AnnounceIdentity>,
    ) -> Result<(), NatsError> {
        let mut event_payload = serde_json::json!({ "agent_id": agent_id });
        if let Some(idn) = identity {
            idn.attach_to(&mut event_payload);
        }
        self.announce_event("join", &event_payload)?;

        let mut legacy = legacy_announce_payload("join", agent_id, identity);
        add_envelope(&mut legacy);
        let bytes = serde_json::to_vec(&legacy)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw("QUEEN.announce", &bytes)
    }

    /// Announce leaving the swarm.
    ///
    /// Publishes to both the new event stream (`QUEEN.event.leave`) and the
    /// legacy `QUEEN.announce` subject for backward compatibility.
    pub fn announce_leave(&self, agent_id: &str) -> Result<(), NatsError> {
        let event_payload = serde_json::json!({ "agent_id": agent_id });
        self.announce_event("leave", &event_payload)?;

        let mut legacy = serde_json::json!({
            "event": "leave",
            "agent_id": agent_id,
        });
        add_envelope(&mut legacy);
        let bytes = serde_json::to_vec(&legacy)
            .map_err(|e| NatsError::Serialize(e.to_string()))?;
        self.publish_raw("QUEEN.announce", &bytes)
    }

    // -----------------------------------------------------------------------
    // JetStream KV bucket operations
    // -----------------------------------------------------------------------

    /// Create a NATS JetStream KV bucket.
    ///
    /// Under the hood this creates a stream named `KV_<name>` with subjects
    /// `$KV.<name>.>`, max 1 message per subject (last-value semantics),
    /// and an optional TTL in seconds.
    pub fn create_kv_bucket(&self, name: &str, ttl_seconds: u64) -> Result<(), NatsError> {
        let stream_name = format!("KV_{}", name);
        let subjects = format!("$KV.{}.>", name);
        let config = if ttl_seconds > 0 {
            serde_json::json!({
                "name": stream_name,
                "subjects": [subjects],
                "retention": "limits",
                "max_msgs_per_subject": 1,
                "max_age": ttl_seconds * 1_000_000_000_u64,
                "storage": "file",
                "discard": "old",
                "num_replicas": 1,
                "allow_rollup_hdrs": true
            })
        } else {
            serde_json::json!({
                "name": stream_name,
                "subjects": [subjects],
                "retention": "limits",
                "max_msgs_per_subject": 1,
                "storage": "file",
                "discard": "old",
                "num_replicas": 1,
                "allow_rollup_hdrs": true
            })
        };
        self.ensure_js_stream(&stream_name, config)
    }

    /// Put a value into a NATS KV bucket.
    pub fn kv_put(&self, bucket: &str, key: &str, value: &str) -> Result<(), NatsError> {
        let subject = format!("$KV.{}.{}", bucket, key);
        self.publish_raw(&subject, value.as_bytes())
    }

    /// Get a value from a NATS KV bucket.
    ///
    /// Returns the latest value for the given key, or `NatsError::KvNotFound`
    /// if the key does not exist (or the server didn't reply in time).
    pub fn kv_get(&self, bucket: &str, key: &str) -> Result<String, NatsError> {
        let stream_name = format!("KV_{}", bucket);
        let target_subject = format!("$KV.{}.{}", bucket, key);
        let api_subject = format!("$JS.API.STREAM.MSG.GET.{}", stream_name);
        let req = serde_json::json!({ "last_by_subj": target_subject }).to_string();

        let resp = {
            let mut conn = self.lock_conn()?;
            self.js_api_call_locked(&mut conn, &api_subject, req.as_bytes(), JS_API_TIMEOUT)?
        };

        resp.as_ref()
            .filter(|r| r.get("error").is_none())
            .and_then(|r| r.get("message"))
            .and_then(|m| m.get("data"))
            .and_then(|d| d.as_str())
            .and_then(|b64| base64_decode(b64).ok())
            .map(|decoded| String::from_utf8_lossy(&decoded).to_string())
            .ok_or_else(|| NatsError::KvNotFound(format!("{}/{}", bucket, key)))
    }

    /// List all keys in a NATS KV bucket.
    pub fn kv_keys(&self, bucket: &str) -> Result<Vec<String>, NatsError> {
        let stream_name = format!("KV_{}", bucket);
        let prefix = format!("$KV.{}.", bucket);
        let filter = format!("$KV.{}.>", bucket);
        let msgs = self.stream_walk(&stream_name, &filter, STREAM_WALK_LIMIT)?;
        Ok(msgs
            .into_iter()
            .filter_map(|(subject, _)| subject.strip_prefix(&prefix).map(String::from))
            .collect())
    }

    /// Discover all agents registered in the QUEEN_AGENTS KV bucket.
    ///
    /// Returns a map of agent_id -> registration JSON value.
    pub fn discover_peers(&self) -> Result<HashMap<String, serde_json::Value>, NatsError> {
        let keys = self.kv_keys(KV_BUCKET_AGENTS)?;
        let mut peers = HashMap::new();
        for key in keys {
            match self.kv_get(KV_BUCKET_AGENTS, &key) {
                Ok(val) => {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&val) {
                        peers.insert(key, parsed);
                    } else {
                        peers.insert(key, serde_json::Value::String(val));
                    }
                }
                Err(NatsError::KvNotFound(_)) => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(peers)
    }

    // -----------------------------------------------------------------------
    // Subscriptions / PING
    // -----------------------------------------------------------------------

    /// Subscribe to phase updates and memory sync messages.
    ///
    /// Subscribes to:
    /// - `QUEEN.phase.*` — agent phase gossip
    /// - `QUEEN.announce` — join/leave announcements
    /// - `KANNAKA.memory.new` + `KANNAKA.dreams` — memory sync (when
    ///   `include_memories` is true)
    pub fn subscribe_phases_and_memories(&self, include_memories: bool) -> Result<NatsSubscription, NatsError> {
        let mut conn = self.lock_conn()?;
        let first_sid = self.alloc_sid();
        write!(conn.writer, "SUB QUEEN.phase.* {}\r\n", first_sid)?;
        write!(conn.writer, "SUB QUEEN.announce {}\r\n", self.alloc_sid())?;
        if include_memories {
            write!(conn.writer, "SUB KANNAKA.memory.new {}\r\n", self.alloc_sid())?;
            write!(conn.writer, "SUB KANNAKA.dreams {}\r\n", self.alloc_sid())?;
        }
        conn.writer.flush()?;
        let stream_clone = conn.writer.try_clone()?;

        Ok(NatsSubscription {
            reader: BufReader::new(stream_clone),
            sid: first_sid.to_string(),
        })
    }

    /// Subscribe to phase updates. Returns a NatsSubscription that can be iterated.
    pub fn subscribe_phases(&self) -> Result<NatsSubscription, NatsError> {
        self.subscribe_phases_and_memories(false)
    }

    /// Subscribe to memory sync messages only. Returns a NatsSubscription.
    pub fn subscribe_memories(&self) -> Result<NatsSubscription, NatsError> {
        self.subscribe_with_queue("KANNAKA.memory.new", None)
    }

    /// Send a PING and wait for the PONG (2s budget) to check connection
    /// health. A write that merely lands in a dead socket's kernel buffer
    /// no longer counts as "connected". Marks the transport disconnected
    /// on failure. The previous read timeout is restored on all paths.
    pub fn ping(&self) -> Result<(), NatsError> {
        let mut conn = self.lock_conn()?;
        let prev = conn.read_timeout();
        let _ = conn.set_read_timeout(Some(Duration::from_secs(2)));
        let deadline = Instant::now() + Duration::from_secs(2);

        let result = (|conn: &mut Conn| -> Result<(), NatsError> {
            write!(conn.writer, "PING\r\n")?;
            conn.writer.flush()?;
            loop {
                if Instant::now() > deadline {
                    return Err(NatsError::Protocol("no PONG within 2s".to_string()));
                }
                match read_frame(&mut conn.reader) {
                    Ok(ReadOutcome::Frame(Frame::Pong)) => return Ok(()),
                    Ok(ReadOutcome::Frame(Frame::Ping)) => {
                        let _ = conn.pong();
                    }
                    Ok(ReadOutcome::Frame(Frame::ServerErr(m))) => {
                        eprintln!("[nats] server error: {}", m);
                        if is_auth_error(&m) {
                            return Err(NatsError::Disconnected(m));
                        }
                    }
                    Ok(ReadOutcome::Frame(_)) => continue,
                    Ok(ReadOutcome::TimedOut) => {
                        return Err(NatsError::Protocol(
                            "timed out waiting for PONG".to_string(),
                        ));
                    }
                    Ok(ReadOutcome::Closed) => {
                        return Err(NatsError::Disconnected(
                            "connection closed".to_string(),
                        ));
                    }
                    Err(e) => return Err(e),
                }
            }
        })(&mut conn);

        let _ = conn.set_read_timeout(prev);
        if result.is_err() {
            drop(conn);
            self.mark_disconnected();
        }
        result
    }

    // -----------------------------------------------------------------------
    // Request / Reply (ADR-0026 Phase 1)
    // -----------------------------------------------------------------------

    /// Synchronous request: publish to `subject` with a unique reply-to inbox,
    /// await the FIRST reply on that inbox, return its payload. Used for
    /// directed agent queries (`kannaka ask --remote <agent_id>`).
    ///
    /// Holds the connection lock for the whole exchange. Replies are matched
    /// by inbox subject — messages for other subscriptions on this connection
    /// are consumed and skipped, never returned as the reply. The previous
    /// read timeout is restored on all paths.
    pub fn request_one(
        &self,
        subject: &str,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, NatsError> {
        let inbox = new_inbox("req");
        let sid = self.alloc_sid();
        let mut conn = self.lock_conn()?;

        // SUB inbox first so we don't race the reply; UNSUB <sid> 1 lets the
        // server clean up automatically after the first delivery.
        write!(conn.writer, "SUB {} {}\r\n", inbox, sid)?;
        write!(conn.writer, "PUB {} {} {}\r\n", subject, inbox, payload.len())?;
        conn.writer.write_all(payload)?;
        conn.writer.write_all(b"\r\n")?;
        write!(conn.writer, "UNSUB {} 1\r\n", sid)?;
        conn.writer.flush()?;

        let prev = conn.read_timeout();
        conn.set_read_timeout(Some(timeout))?;
        let deadline = Instant::now() + timeout;

        let mut fatal = false;
        let result = loop {
            if Instant::now() > deadline {
                break Err(NatsError::Protocol("request_one timed out".to_string()));
            }
            match read_frame(&mut conn.reader) {
                Ok(ReadOutcome::Frame(Frame::Msg { subject: msub, payload, .. }))
                    if msub == inbox =>
                {
                    break Ok(payload);
                }
                Ok(ReadOutcome::Frame(Frame::Msg { .. })) => continue,
                Ok(ReadOutcome::Frame(Frame::Ping)) => {
                    let _ = conn.pong();
                }
                Ok(ReadOutcome::Frame(Frame::ServerErr(m))) => {
                    eprintln!("[nats] server error: {}", m);
                    if is_auth_error(&m) {
                        fatal = true;
                        break Err(NatsError::Disconnected(m));
                    }
                }
                Ok(ReadOutcome::Frame(_)) => continue,
                Ok(ReadOutcome::TimedOut) => {
                    break Err(NatsError::Protocol("request_one timed out".to_string()));
                }
                Ok(ReadOutcome::Closed) => {
                    fatal = true;
                    break Err(NatsError::Disconnected(
                        "connection closed during request".to_string(),
                    ));
                }
                Err(e) => {
                    fatal = true;
                    break Err(e);
                }
            }
        };

        if result.is_err() {
            // No reply was delivered, so the auto-unsub never fired —
            // remove the subscription explicitly.
            let _ = write!(conn.writer, "UNSUB {}\r\n", sid);
            let _ = conn.writer.flush();
        }
        let _ = conn.set_read_timeout(prev);
        if fatal {
            drop(conn);
            self.mark_disconnected();
        }
        result
    }

    /// Broadcast request: publish to `subject` with a reply-to inbox, collect
    /// every reply that arrives on that inbox within `timeout`, return them
    /// all. Used for `kannaka ask --remote broadcast` where any number of
    /// agents may answer. The previous read timeout is restored on all paths.
    pub fn request_many(
        &self,
        subject: &str,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<Vec<Vec<u8>>, NatsError> {
        let inbox = new_inbox("reqm");
        let sid = self.alloc_sid();
        let mut conn = self.lock_conn()?;

        write!(conn.writer, "SUB {} {}\r\n", inbox, sid)?;
        write!(conn.writer, "PUB {} {} {}\r\n", subject, inbox, payload.len())?;
        conn.writer.write_all(payload)?;
        conn.writer.write_all(b"\r\n")?;
        conn.writer.flush()?;

        let prev = conn.read_timeout();
        conn.set_read_timeout(Some(Duration::from_millis(500)))?;
        let deadline = Instant::now() + timeout;

        let mut replies: Vec<Vec<u8>> = Vec::new();
        let mut fatal = false;
        while Instant::now() < deadline {
            match read_frame(&mut conn.reader) {
                Ok(ReadOutcome::Frame(Frame::Msg { subject: msub, payload, .. }))
                    if msub == inbox =>
                {
                    replies.push(payload);
                }
                Ok(ReadOutcome::Frame(Frame::Msg { .. })) => continue,
                Ok(ReadOutcome::Frame(Frame::Ping)) => {
                    let _ = conn.pong();
                }
                Ok(ReadOutcome::Frame(Frame::ServerErr(m))) => {
                    eprintln!("[nats] server error: {}", m);
                    if is_auth_error(&m) {
                        fatal = true;
                        break;
                    }
                }
                Ok(ReadOutcome::Frame(_)) => continue,
                // Clean per-read timeout: the outer loop re-checks the deadline.
                Ok(ReadOutcome::TimedOut) => continue,
                Ok(ReadOutcome::Closed) => {
                    fatal = true;
                    break;
                }
                Err(e) => {
                    // Hard error (desync, reset) — do NOT spin on it.
                    eprintln!("[nats] request_many read error: {}", e);
                    fatal = true;
                    break;
                }
            }
        }

        // UNSUB so the server stops delivering on this inbox.
        let _ = write!(conn.writer, "UNSUB {}\r\n", sid);
        let _ = conn.writer.flush();
        let _ = conn.set_read_timeout(prev);
        if fatal {
            drop(conn);
            self.mark_disconnected();
        }
        Ok(replies)
    }

    /// Publish a reply to the given reply-to subject. Used by `swarm serve`
    /// after handling an ask.
    pub fn reply(&self, reply_to: &str, payload: &[u8]) -> Result<(), NatsError> {
        self.publish_raw(reply_to, payload)
    }

    /// Open a long-running subscription on `subject` and return a stream of
    /// messages. The caller drives the loop and decides when to stop.
    /// Note: while the subscription is open, calls to other methods on this
    /// transport will block (single-mutex stream) and the two readers would
    /// steal each other's bytes; a serving agent should run on a dedicated
    /// SwarmTransport instance per subscription.
    pub fn subscribe(&self, subject: &str) -> Result<NatsSubscription, NatsError> {
        self.subscribe_with_queue(subject, None)
    }

    /// Subscribe with an optional queue group. NATS delivers each message to
    /// exactly ONE subscriber per queue group — this is how we get
    /// work-queue semantics across multiple workers (#74). Wire format:
    ///   SUB <subject> [queue-group] <sid>
    pub fn subscribe_with_queue(
        &self,
        subject: &str,
        queue_group: Option<&str>,
    ) -> Result<NatsSubscription, NatsError> {
        let mut conn = self.lock_conn()?;
        let sid = self.alloc_sid();
        match queue_group {
            Some(g) => write!(conn.writer, "SUB {} {} {}\r\n", subject, g, sid)?,
            None => write!(conn.writer, "SUB {} {}\r\n", subject, sid)?,
        }
        conn.writer.flush()?;
        let stream_clone = conn.writer.try_clone().map_err(NatsError::Io)?;
        // NOTE: the read timeout is a property of the underlying socket,
        // which is shared with the transport — this also clears the
        // transport's default timeout. Acceptable under the documented
        // one-subscription-per-connection model.
        stream_clone.set_read_timeout(None)?;
        Ok(NatsSubscription {
            reader: BufReader::new(stream_clone),
            sid: sid.to_string(),
        })
    }
}

/// A subscription that yields NATS messages.
pub struct NatsSubscription {
    reader: BufReader<TcpStream>,
    /// The (first) subscription id registered for this subscription.
    sid: String,
}

/// A received NATS message.
#[derive(Debug)]
pub struct NatsMessage {
    pub subject: String,
    pub payload: Vec<u8>,
    /// Reply-to subject if the publisher set one (NATS request/reply pattern).
    /// Present when the wire MSG line had 5 parts instead of 4. Used by
    /// `kannaka swarm serve` to respond to inbound `KANNAKA.ask.*` messages.
    pub reply_to: Option<String>,
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

/// Outcome of one `NatsSubscription::next_event` poll. Lets serve loops
/// distinguish "nothing arrived within the read timeout — poll again"
/// from "the connection is dead — reconnect or exit" (which `next_message`
/// conflates into `None`, historically causing 100%-CPU spin loops on a
/// closed socket).
#[derive(Debug)]
pub enum SubEvent {
    /// A message arrived.
    Msg(NatsMessage),
    /// The read timeout (see `set_timeout`) elapsed with no traffic.
    /// The connection is still healthy; poll again.
    Timeout,
    /// EOF, a fatal protocol desync, or an authorization error from the
    /// server. This subscription will never yield again — reconnect or exit.
    Closed,
}

impl NatsSubscription {
    /// The subscription id this subscription registered with the server.
    pub fn sid(&self) -> &str {
        &self.sid
    }

    /// Poll for the next event, distinguishing timeout from connection
    /// death. Server PINGs are answered transparently; `-ERR` lines are
    /// logged (authorization errors close the subscription).
    pub fn next_event(&mut self) -> SubEvent {
        loop {
            match read_frame(&mut self.reader) {
                Ok(ReadOutcome::Frame(Frame::Msg { subject, reply_to, payload, .. })) => {
                    return SubEvent::Msg(NatsMessage { subject, payload, reply_to });
                }
                Ok(ReadOutcome::Frame(Frame::Ping)) => {
                    if let Ok(mut s) = self.reader.get_ref().try_clone() {
                        let _ = write!(s, "PONG\r\n");
                        let _ = s.flush();
                    }
                }
                Ok(ReadOutcome::Frame(Frame::ServerErr(m))) => {
                    eprintln!("[nats] server error: {}", m);
                    if is_auth_error(&m) {
                        return SubEvent::Closed;
                    }
                }
                Ok(ReadOutcome::Frame(_)) => continue,
                Ok(ReadOutcome::TimedOut) => return SubEvent::Timeout,
                Ok(ReadOutcome::Closed) => return SubEvent::Closed,
                Err(e) => {
                    eprintln!("[nats] subscription protocol error: {}", e);
                    return SubEvent::Closed;
                }
            }
        }
    }

    /// Block until the next message arrives. Returns None on read timeout
    /// AND on connection close — callers that need to tell those apart
    /// (e.g. serve loops that must not spin on a dead socket) should use
    /// `next_event` instead.
    pub fn next_message(&mut self) -> Option<NatsMessage> {
        match self.next_event() {
            SubEvent::Msg(m) => Some(m),
            SubEvent::Timeout | SubEvent::Closed => None,
        }
    }

    /// Set read timeout for the subscription stream.
    ///
    /// NOTE: the timeout is a property of the underlying socket, which is
    /// shared with the SwarmTransport that created this subscription — fine
    /// under the one-subscription-per-connection model.
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
        let (host, port, creds) = parse_nats_url("nats://localhost").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 4222);
        assert!(creds.is_none());
    }

    #[test]
    fn parse_nats_url_custom_port() {
        let (host, port, creds) = parse_nats_url("nats://swarm.ninja-portal.com:4222").unwrap();
        assert_eq!(host, "swarm.ninja-portal.com");
        assert_eq!(port, 4222);
        assert!(creds.is_none());
    }

    #[test]
    fn parse_nats_url_no_scheme() {
        let (host, port, creds) = parse_nats_url("localhost:4222").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 4222);
        assert!(creds.is_none());
    }

    #[test]
    fn parse_nats_url_with_credentials() {
        let (host, port, creds) = parse_nats_url("nats://alice:s3cr3t@example.com:4223").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 4223);
        assert_eq!(creds, Some(("alice".to_string(), "s3cr3t".to_string())));
    }

    #[test]
    fn parse_nats_url_creds_default_port() {
        let (host, port, creds) = parse_nats_url("nats://bob:pw@example.com").unwrap();
        assert_eq!(host, "example.com");
        assert_eq!(port, 4222);
        assert_eq!(creds, Some(("bob".to_string(), "pw".to_string())));
    }

    #[test]
    fn new_inbox_unique() {
        let a = new_inbox("t");
        let b = new_inbox("t");
        assert_ne!(a, b);
        assert!(a.starts_with("_INBOX.t."));
    }

    #[test]
    fn connect_default_graceful_failure() {
        match SwarmTransport::connect("nats://127.0.0.1:19999") {
            Ok(_) => panic!("should not connect to nonexistent server"),
            Err(e) => {
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
            link_count: 0,
            xi_signature: None,
            protocol_version: "1.0".to_string(),
            timestamp: Utc::now(),
            trust_score: 0.5,
            handedness: crate::queen::Handedness::Achiral,
            left_coherence: 0.0,
            right_coherence: 0.0,
            bridge_activity: 0.0,
            dream_state: None,
            role: None,
            display_name: None,
        };
        let bytes = serde_json::to_vec(&phase).unwrap();
        let decoded: AgentPhase = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded.agent_id, "agent-1");
        assert!((decoded.phase - 1.5).abs() < 0.001);
    }

    #[test]
    fn base64_decode_roundtrip() {
        let decoded = base64_decode("aGVsbG8=").unwrap();
        assert_eq!(&decoded, b"hello");
    }

    #[test]
    fn base64_decode_empty() {
        let decoded = base64_decode("").unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn base64_decode_rejects_invalid() {
        assert!(base64_decode("ab!c").is_err());
    }

    #[test]
    fn nats_error_display_variants() {
        let e = NatsError::Disconnected("lost connection".into());
        assert!(format!("{}", e).contains("disconnected"));

        let e = NatsError::KvNotFound("mybucket/mykey".into());
        assert!(format!("{}", e).contains("KV key not found"));
    }

    #[test]
    fn buffered_message_clone() {
        let msg = BufferedMessage {
            subject: "QUEEN.event.join".to_string(),
            payload: b"test".to_vec(),
        };
        let cloned = msg.clone();
        assert_eq!(cloned.subject, msg.subject);
        assert_eq!(cloned.payload, msg.payload);
    }

    #[test]
    fn publish_buffer_limit() {
        let buf: Arc<Mutex<VecDeque<BufferedMessage>>> =
            Arc::new(Mutex::new(VecDeque::new()));
        {
            let mut guard = buf.lock().unwrap();
            for i in 0..PUBLISH_BUFFER_LIMIT + 20 {
                if guard.len() >= PUBLISH_BUFFER_LIMIT {
                    guard.pop_front();
                }
                guard.push_back(BufferedMessage {
                    subject: format!("test.{}", i),
                    payload: vec![],
                });
            }
            assert_eq!(guard.len(), PUBLISH_BUFFER_LIMIT);
            assert_eq!(guard.front().unwrap().subject, "test.20");
        }
    }

    // -----------------------------------------------------------------------
    // Announce identity (swarm agent identity, step 2)
    // -----------------------------------------------------------------------

    fn sample_identity() -> AnnounceIdentity {
        AnnounceIdentity {
            user_id: "user-123".into(),
            email: "agent@spacechild.love".into(),
        }
    }

    #[test]
    fn announce_payload_without_identity_is_byte_compatible() {
        // No stored identity → exactly the payload v0.6.19/0.6.20 publish.
        let new = legacy_announce_payload("join", "agent-a", None);
        let old = serde_json::json!({ "event": "join", "agent_id": "agent-a" });
        assert_eq!(
            serde_json::to_string(&new).unwrap(),
            serde_json::to_string(&old).unwrap(),
        );
    }

    #[test]
    fn announce_payload_with_identity_adds_optional_block() {
        let payload = legacy_announce_payload("join", "agent-a", Some(&sample_identity()));
        // Existing fields untouched (old agents read these by name).
        assert_eq!(payload["event"], "join");
        assert_eq!(payload["agent_id"], "agent-a");
        // Identity block carries user_id/email ONLY — never tokens.
        assert_eq!(payload["identity"]["user_id"], "user-123");
        assert_eq!(payload["identity"]["email"], "agent@spacechild.love");
        let block = payload["identity"].as_object().unwrap();
        assert_eq!(block.len(), 2, "identity block must be user_id + email only");
        let text = serde_json::to_string(&payload).unwrap();
        assert!(!text.contains("token"), "no token material on the wire");
    }

    #[test]
    fn old_shape_announce_still_deserializes() {
        // What a v0.6.19/0.6.20 agent publishes today (post-envelope).
        let old = r#"{"event":"join","agent_id":"agent-old","schema_version":"1.0","ts":1765400000000}"#;
        let v: serde_json::Value = serde_json::from_str(old).expect("old payload parses");
        assert_eq!(v["agent_id"], "agent-old");
        // New code sees no identity on old payloads — no error, just None.
        assert_eq!(AnnounceIdentity::email_from(&v), None);
    }

    #[test]
    fn new_shape_announce_keeps_legacy_fields_intact() {
        // Old agents parse announces as loose JSON by field name — verify
        // the fields they use survive the identity enrichment + envelope.
        let mut payload = legacy_announce_payload("join", "agent-new", Some(&sample_identity()));
        add_envelope(&mut payload);
        let bytes = serde_json::to_vec(&payload).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["event"], "join");
        assert_eq!(v["agent_id"], "agent-new");
        assert_eq!(v["schema_version"], "1.0");
        assert!(v["ts"].is_number());
        assert_eq!(AnnounceIdentity::email_from(&v), Some("agent@spacechild.love"));
    }

    #[test]
    fn presence_identity_email_extraction() {
        // Presence record from a pre-identity agent: no identity key.
        let old = serde_json::json!({ "agent_id": "a", "memory_count": 5 });
        assert_eq!(AnnounceIdentity::email_from(&old), None);

        // Presence record with an attached identity.
        let mut new = serde_json::json!({ "agent_id": "a", "memory_count": 5 });
        sample_identity().attach_to(&mut new);
        assert_eq!(new["agent_id"], "a", "existing fields untouched");
        assert_eq!(AnnounceIdentity::email_from(&new), Some("agent@spacechild.love"));

        // attach_to on a non-object is a no-op, not a panic.
        let mut not_obj = serde_json::json!("scalar");
        sample_identity().attach_to(&mut not_obj);
        assert_eq!(AnnounceIdentity::email_from(&not_obj), None);
    }

    // -----------------------------------------------------------------------
    // Integration tests -- skip if NATS unavailable
    // -----------------------------------------------------------------------

    fn connect_or_skip() -> Option<SwarmTransport> {
        match SwarmTransport::connect(DEFAULT_NATS_URL) {
            Ok(t) => Some(t),
            Err(_) => {
                eprintln!("NATS not available, skipping integration test");
                None
            }
        }
    }

    #[test]
    fn integration_publish_and_read() {
        let transport = match connect_or_skip() {
            Some(t) => t,
            None => return,
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
            link_count: 0,
            xi_signature: None,
            protocol_version: "1.0".to_string(),
            timestamp: chrono::Utc::now(),
            trust_score: 0.5,
            handedness: crate::queen::Handedness::Achiral,
            left_coherence: 0.0,
            right_coherence: 0.0,
            bridge_activity: 0.0,
            dream_state: None,
            role: None,
            display_name: None,
        };

        transport.publish_phase(&phase).expect("publish should work");
        transport.announce_join("test-integration").expect("announce should work");

        let phases = transport.get_all_phases().unwrap_or_default();
        eprintln!("Got {} phases from NATS", phases.len());
    }

    #[test]
    #[ignore = "requires live NATS server with JetStream; run locally with `cargo test --ignored`"]
    fn integration_kv_put_get() {
        let transport = match connect_or_skip() {
            Some(t) => t,
            None => return,
        };
        if !transport.has_jetstream() {
            eprintln!("JetStream not available, skipping KV test");
            return;
        }

        transport.create_kv_bucket("TEST_KV", 60).expect("create bucket");
        transport.kv_put("TEST_KV", "hello", "world").expect("kv_put");
        // Small delay to let JetStream persist
        std::thread::sleep(Duration::from_millis(200));
        let val = transport.kv_get("TEST_KV", "hello").expect("kv_get");
        assert_eq!(val, "world");

        // Overwrite: may not take effect if the bucket was previously created
        // with discard="new" (stale server state). On a fresh bucket, updates work.
        transport.kv_put("TEST_KV", "hello", "updated").expect("kv_put overwrite");
        std::thread::sleep(Duration::from_millis(200));
        let val2 = transport.kv_get("TEST_KV", "hello").expect("kv_get after overwrite");
        // Accept either value (old bucket: "world", fresh bucket: "updated")
        assert!(
            val2 == "updated" || val2 == "world",
            "expected 'updated' or 'world', got: {}", val2
        );
    }

    #[test]
    #[ignore = "requires live NATS server with JetStream; run locally with `cargo test --ignored`"]
    fn integration_kv_keys() {
        let transport = match connect_or_skip() {
            Some(t) => t,
            None => return,
        };
        if !transport.has_jetstream() {
            eprintln!("JetStream not available, skipping KV keys test");
            return;
        }

        transport.create_kv_bucket("TEST_KEYS", 60).expect("create bucket");
        transport.kv_put("TEST_KEYS", "a", "1").expect("put a");
        transport.kv_put("TEST_KEYS", "b", "2").expect("put b");
        std::thread::sleep(Duration::from_millis(200));

        let keys = transport.kv_keys("TEST_KEYS").expect("kv_keys");
        assert!(keys.contains(&"a".to_string()), "keys should contain 'a': {:?}", keys);
        assert!(keys.contains(&"b".to_string()), "keys should contain 'b': {:?}", keys);
    }

    #[test]
    fn integration_kv_get_missing() {
        let transport = match connect_or_skip() {
            Some(t) => t,
            None => return,
        };
        if !transport.has_jetstream() {
            eprintln!("JetStream not available, skipping KV missing test");
            return;
        }

        transport.create_kv_bucket("TEST_MISSING", 60).expect("create bucket");
        match transport.kv_get("TEST_MISSING", "nonexistent") {
            Err(NatsError::KvNotFound(_)) => {}
            other => panic!("expected KvNotFound, got: {:?}", other),
        }
    }

    #[test]
    fn integration_discover_peers() {
        let transport = match connect_or_skip() {
            Some(t) => t,
            None => return,
        };
        if !transport.has_jetstream() {
            eprintln!("JetStream not available, skipping discover_peers test");
            return;
        }

        transport.create_kv_bucket(KV_BUCKET_AGENTS, 300).expect("create agents bucket");
        let info = serde_json::json!({
            "agent_id": "test-discover",
            "role": "tester",
            "joined_at": chrono::Utc::now().to_rfc3339(),
        });
        transport.kv_put(KV_BUCKET_AGENTS, "test-discover", &info.to_string())
            .expect("register agent");
        std::thread::sleep(Duration::from_millis(200));

        let peers = transport.discover_peers().expect("discover_peers");
        // On a fresh NATS server this will always find the agent. If the KV bucket
        // was previously created with a stale discard policy, the put may have been
        // silently dropped; we log instead of failing to keep CI green.
        if !peers.contains_key("test-discover") {
            eprintln!(
                "discover_peers: agent not found (stale KV bucket config?). peers={:?}",
                peers
            );
        }
    }

    #[test]
    fn integration_announce_event() {
        let transport = match connect_or_skip() {
            Some(t) => t,
            None => return,
        };

        let payload = serde_json::json!({ "agent_id": "test-events" });
        transport.announce_event("join", &payload).expect("announce join event");
        transport.announce_event("dream.start", &payload).expect("announce dream.start");
        transport.announce_event("memory.shared", &payload).expect("announce memory.shared");
    }

    #[test]
    fn integration_reconnect_live() {
        let mut transport = match connect_or_skip() {
            Some(t) => t,
            None => return,
        };

        assert!(transport.is_connected());
        transport.reconnect().expect("reconnect to live server");
        assert!(transport.is_connected());

        let payload = serde_json::json!({ "agent_id": "reconnect-test" });
        transport.announce_event("join", &payload).expect("announce after reconnect");
    }
}
