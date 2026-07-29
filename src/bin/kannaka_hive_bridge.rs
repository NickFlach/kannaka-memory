//! kannaka-hive-bridge — mirrors Hive (kannaka-buzz) room traffic onto NATS.
//!
//! Connects to the buzz relay, authenticates with NIP-42, subscribes to room
//! messages, the agent job lifecycle, and agent profiles, gates every event
//! through the per-channel bridge policy, and republishes onto
//! `KANNAKA.events.hive.*`. All decisions live in
//! `kannaka_memory::hive_bridge` (unit-tested); this binary is plumbing.
//!
//! Config (env):
//!   HIVE_RELAY_URL            wss:// url of the buzz relay
//!   HIVE_KEY_FILE             json {privkey,pubkey}, 0600 — an allowlisted member
//!   HIVE_DEDUPE_FILE          crash-durable processed-id log
//!   HIVE_NATS_URL/_USER/_PASS route target
//!   HIVE_SUBJECT_PREFIX       default KANNAKA.events.hive
//!   HIVE_POLICY_REFRESH_SECS  default 60
//!   HIVE_RATE_CAP/_REFILL     per-author token bucket (default 20 / 1.0)

use std::io::{Read, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kannaka_memory::hive_bridge::{OkVerdict, PolicyMap, Roster};
use kannaka_memory::nostr::bridge::{Dedup, RateLimiter};
use kannaka_memory::nostr::{Event, Keypair};
use tungstenite::Message;

struct Config {
    relay_url: String,
    privkey: String,
    dedupe_file: String,
    nats_url: String,
    nats_user: String,
    nats_pass: String,
    subject_prefix: String,
    policy_refresh_secs: u64,
    rate_cap: f64,
    rate_refill: f64,
}

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|s| !s.is_empty())
}

fn load_config() -> Config {
    let key_file = env("HIVE_KEY_FILE").expect("HIVE_KEY_FILE required");
    let key_json = std::fs::read_to_string(&key_file).expect("read hive key file");
    let key: serde_json::Value = serde_json::from_str(&key_json).expect("hive key json");
    Config {
        relay_url: env("HIVE_RELAY_URL").expect("HIVE_RELAY_URL required"),
        privkey: key["privkey"].as_str().expect("privkey").to_string(),
        dedupe_file: env("HIVE_DEDUPE_FILE")
            .unwrap_or_else(|| "/var/lib/kannaka-hive-bridge/dedupe.log".into()),
        nats_url: env("HIVE_NATS_URL").unwrap_or_else(|| "nats://127.0.0.1:4222".into()),
        nats_user: env("HIVE_NATS_USER").unwrap_or_default(),
        nats_pass: env("HIVE_NATS_PASS").unwrap_or_default(),
        subject_prefix: env("HIVE_SUBJECT_PREFIX")
            .unwrap_or_else(|| "KANNAKA.events.hive".into()),
        policy_refresh_secs: env("HIVE_POLICY_REFRESH_SECS")
            .and_then(|s| s.parse().ok())
            .unwrap_or(60),
        rate_cap: env("HIVE_RATE_CAP")
            .and_then(|s| s.parse().ok())
            .unwrap_or(20.0),
        rate_refill: env("HIVE_RATE_REFILL")
            .and_then(|s| s.parse().ok())
            .unwrap_or(1.0),
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

/// NIP-42: sign a kind-22242 event over the relay's challenge.
fn build_auth_event(kp: &Keypair, relay_url: &str, challenge: &str) -> Event {
    kp.sign_event(
        22242,
        vec![
            vec!["relay".to_string(), relay_url.to_string()],
            vec!["challenge".to_string(), challenge.to_string()],
        ],
        "",
        now_secs(),
    )
}

/// Minimum spacing between initial-connect attempts, so a bridge started
/// while NATS is down does not pay a connect timeout on every single event.
const CONNECT_RETRY: Duration = Duration::from_secs(15);

/// Bound on a single `ws.read()`.
///
/// Load-bearing for every timer in the event loop, not just the liveness
/// deadline below: `ws.read()` blocks, so on a silent socket the loop never
/// comes back around and nothing time-based can fire. The periodic policy
/// refresh had this latent too — it could only ever run on the back of an
/// unrelated inbound frame.
const WS_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// How long after connecting the content subscription may stay unopened
/// before we declare the connection dead.
///
/// Generous because it spans AUTH plus two history queries (policy, roster)
/// against a relay that may be paging; the failure it catches is indefinite,
/// so precision does not matter.
const SUBSCRIBE_DEADLINE_SECS: i64 = 60;

/// Bound reads on the underlying socket so the loop's timers can run.
fn bound_ws_reads(
    ws: &tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>,
) {
    use tungstenite::stream::MaybeTlsStream;
    let sock = match ws.get_ref() {
        MaybeTlsStream::Plain(s) => Some(s),
        MaybeTlsStream::Rustls(s) => Some(&s.sock),
        // `MaybeTlsStream` is #[non_exhaustive]; an unknown variant just means
        // no deadline enforcement, which is the old behaviour.
        _ => None,
    };
    match sock {
        Some(s) => {
            if let Err(e) = s.set_read_timeout(Some(WS_READ_TIMEOUT)) {
                eprintln!("[hive-bridge] WARN could not bound socket reads ({e}) — liveness deadline is inactive");
            }
        }
        None => eprintln!(
            "[hive-bridge] WARN unrecognised stream type — liveness deadline is inactive"
        ),
    }
}

/// Whether a websocket error is just "nothing arrived in time".
fn is_read_timeout(e: &tungstenite::Error) -> bool {
    match e {
        tungstenite::Error::Io(io) => matches!(
            io.kind(),
            // WouldBlock on unix, TimedOut on windows.
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
        ),
        _ => false,
    }
}

/// NATS output for the bridge, backed by the shared `SwarmTransport`.
///
/// This replaced a hand-rolled per-message client (one TCP connect + INFO +
/// CONNECT + PUB + PING for every event). That copy had drifted from
/// `nats::handshake` in four ways that mattered:
///
///   * **no connect timeout** — a plain `TcpStream::connect` to an unreachable
///     host blocks for the OS SYN timeout, and because it ran per message it
///     stalled the whole relay event loop that long for every event. This is
///     the liveness bug; `handshake` uses `connect_timeout`.
///   * credentials escaped `"` but not `\`, so a backslash in the password
///     produced malformed CONNECT JSON and an auth failure with no clue why.
///   * INFO was read into a fixed 2048-byte buffer with a single `read()` —
///     both truncatable and unbounded in the hostile direction.
///   * no `tls_required` detection, so a TLS-only server failed later with a
///     confusing protocol error instead of a clear message.
///
/// The shared transport also brings a persistent connection, ordered replay
/// of anything buffered across a drop, and rate-limited in-place revival.
struct NatsSink {
    url: String,
    /// The bridge authenticates as its own principal (`HIVE_NATS_*`). These are
    /// passed explicitly rather than exported as `NATS_USER`/`NATS_PASSWORD`,
    /// which on a box that also runs other kannaka units would connect the
    /// bridge as the shared swarm identity with the wrong ACLs.
    creds: Option<(String, String)>,
    transport: Option<kannaka_memory::nats::SwarmTransport>,
    last_attempt: Option<std::time::Instant>,
}

impl NatsSink {
    fn new(cfg: &Config) -> Self {
        let creds = if cfg.nats_user.is_empty() {
            None
        } else {
            Some((cfg.nats_user.clone(), cfg.nats_pass.clone()))
        };
        Self { url: cfg.nats_url.clone(), creds, transport: None, last_attempt: None }
    }

    /// Publish, connecting lazily on first use.
    ///
    /// `Ok(())` means the message is on the wire **or** in the transport's
    /// replay buffer — either way it reaches NATS without the relay re-sending
    /// it, so the caller may mark the event processed. `Err` means no
    /// connection exists at all and the event was genuinely dropped.
    ///
    /// Connecting lazily (rather than at startup) keeps the bridge's Hive-side
    /// work alive when NATS is down, which is what the old stateless publish
    /// gave us for free and is worth preserving.
    fn publish(&mut self, subject: &str, payload: &str) -> Result<(), String> {
        if self.transport.is_none() {
            if let Some(t) = self.last_attempt {
                if t.elapsed() < CONNECT_RETRY {
                    return Err("NATS unavailable (redial pending)".to_string());
                }
            }
            self.last_attempt = Some(std::time::Instant::now());
            match kannaka_memory::nats::SwarmTransport::connect_with_creds(
                &self.url,
                self.creds.clone(),
            ) {
                Ok(t) => {
                    eprintln!("[hive-bridge] NATS connected: {}", self.url);
                    self.transport = Some(t);
                }
                Err(e) => return Err(format!("NATS connect failed: {e}")),
            }
        }
        let transport = self.transport.as_ref().expect("connected above");
        match transport.publish(subject, payload.as_bytes()) {
            Ok(()) => Ok(()),
            // The transport buffered it and will replay in order once revived.
            Err(e) => {
                eprintln!("[hive-bridge] buffered for replay ({e})");
                Ok(())
            }
        }
    }
}

fn send_req<S: Read + Write>(ws: &mut tungstenite::WebSocket<S>, sub: &str, filter: &str) {
    let _ = ws.send(Message::Text(format!(r#"["REQ","{sub}",{filter}]"#)));
}

fn main() {
    let cfg = load_config();
    let kp = Keypair::from_secret_hex(&cfg.privkey).expect("valid hive privkey");
    if let Some(dir) = std::path::Path::new(&cfg.dedupe_file).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut dedup = Dedup::open(&cfg.dedupe_file, 100_000).expect("open dedupe log");
    let mut limiter = RateLimiter::new(cfg.rate_cap, cfg.rate_refill);
    let mut policy = PolicyMap::new();
    let mut roster = Roster::new();
    let mut nats = NatsSink::new(&cfg);

    let (mut ws, _) = tungstenite::connect(&cfg.relay_url).expect("connect to hive relay");
    eprintln!("[hive-bridge] connected to {}", cfg.relay_url);
    bound_ws_reads(&ws);

    let mut authed = false;
    let mut subscribed = false;
    let mut last_policy_refresh = now_secs();
    let connected_at = now_secs();
    // Id of the auth event we sent, so its OK frame can be told from every
    // other OK the relay emits.
    let mut auth_event_id: Option<String> = None;

    loop {
        // Periodic policy refresh. buzz stores kind 39000 channel-scoped, so
        // live subscriptions never receive it via fan-out — a flag set after
        // startup is only ever seen by re-querying history.
        if authed && now_secs() - last_policy_refresh >= cfg.policy_refresh_secs as i64 {
            send_req(&mut ws, "policy", r#"{"kinds":[39000]}"#);
            last_policy_refresh = now_secs();
        }

        // The connection is only useful once the content subscription is open.
        // Every way that can fail to happen — auth silently rejected, this
        // pubkey not allowlisted, a REQ that never returns EOSE — leaves a
        // live socket and a process that has logged nothing since "connected",
        // so it must be caught on a clock rather than by an error.
        if kannaka_memory::hive_bridge::subscribe_deadline_expired(
            subscribed,
            now_secs() - connected_at,
            SUBSCRIBE_DEADLINE_SECS,
        ) {
            eprintln!(
                "[hive-bridge] FATAL no content subscription {SUBSCRIBE_DEADLINE_SECS}s after connect \
                 (authed={authed}) — relay never completed the handshake; exiting for restart"
            );
            std::process::exit(1);
        }

        let msg = match ws.read() {
            Ok(m) => m,
            // Just an idle socket: go round again so the timers above run.
            Err(e) if is_read_timeout(&e) => continue,
            Err(e) => {
                eprintln!("[hive-bridge] socket error: {e}");
                break;
            }
        };
        let Message::Text(text) = msg else { continue };
        let Ok(frame) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let Some(verb) = frame.get(0).and_then(|v| v.as_str()) else {
            continue;
        };

        match verb {
            "AUTH" => {
                let Some(challenge) = frame.get(1).and_then(|v| v.as_str()) else {
                    continue;
                };
                let auth = build_auth_event(&kp, &cfg.relay_url, challenge);
                let payload = serde_json::to_string(&auth).expect("serialize auth event");
                let _ = ws.send(Message::Text(format!(r#"["AUTH",{payload}]"#)));
                auth_event_id = Some(auth.id.clone());
                // NB: this records only that auth was SENT. Confirmation is the
                // OK frame below; the name is kept because the policy-refresh
                // timer keys off "we have attempted auth".
                authed = true;
                // Resolve policy and roster BEFORE opening the content
                // subscription: the roster is built from the same stream it
                // filters, so subscribing first would mislabel the first
                // messages of an agent not yet learned.
                send_req(&mut ws, "policy", r#"{"kinds":[39000]}"#);
                send_req(&mut ws, "roster", r#"{"kinds":[0,10100]}"#);
            }
            // ["OK", <event-id>, <accepted>, <message>]. Previously discarded,
            // so a rejected auth ("auth-required: ...", pubkey not allowlisted)
            // was invisible: the REQs simply returned nothing forever.
            "OK" => match kannaka_memory::hive_bridge::classify_ok(&frame, auth_event_id.as_deref())
            {
                OkVerdict::AuthAccepted => {
                    eprintln!("[hive-bridge] auth accepted by {}", cfg.relay_url)
                }
                OkVerdict::AuthRejected(detail) => {
                    eprintln!(
                        "[hive-bridge] FATAL relay rejected auth: {detail} \
                         — check HIVE_KEY_FILE is an allowlisted member; exiting for restart"
                    );
                    std::process::exit(1);
                }
                // Not fatal, but silence here previously hid every relay-side
                // rejection of a published event.
                OkVerdict::EventRejected { id, detail } => {
                    eprintln!("[hive-bridge] relay rejected event {id}: {detail}")
                }
                OkVerdict::Ignored => {}
            },
            "EOSE" => {
                let sub = frame.get(1).and_then(|v| v.as_str()).unwrap_or("");
                if sub == "roster" && !subscribed {
                    send_req(
                        &mut ws,
                        "content",
                        r#"{"kinds":[0,9,40002,10100,43001,43002,43003,43004,43005,43006]}"#,
                    );
                    subscribed = true;
                    eprintln!(
                        "[hive-bridge] live: {} channels, {} agents",
                        policy.len(),
                        roster.agent_count()
                    );
                }
            }
            "EVENT" => {
                let Some(raw) = frame.get(2) else { continue };
                let Ok(event) = serde_json::from_value::<Event>(raw.clone()) else {
                    continue;
                };
                if event.verify().is_err() {
                    continue;
                }
                policy.apply_metadata(&event);
                roster.apply(&event);
                if dedup.contains(&event.id) {
                    continue;
                }
                if !limiter.allow(&event.pubkey, now_secs()) {
                    continue;
                }

                // map + policy gate + channel-name re-map, as one decision.
                // Lives in hive_bridge::export_decision so the suppression path
                // is testable rather than only reachable from this loop (#636).
                let Some(mapped) =
                    kannaka_memory::hive_bridge::export_decision(&event, &roster, &policy, now_ms())
                else {
                    continue;
                };

                let subject = format!("{}.{}", cfg.subject_prefix, mapped.subject);
                let payload = mapped.payload.to_string();
                if let Err(e) = nats.publish(&subject, &payload) {
                    eprintln!("[hive-bridge] publish failed: {e}");
                    continue;
                }
                let _ = dedup.record(&event.id);
            }
            _ => {}
        }
    }
}
