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
use std::net::TcpStream;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kannaka_memory::hive_bridge::{map_event, MapContext, PolicyMap, Roster};
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

fn nats_hostport(url: &str) -> (String, u16) {
    let s = url.strip_prefix("nats://").unwrap_or(url);
    let mut it = s.splitn(2, ':');
    let host = it.next().unwrap_or("127.0.0.1").to_string();
    let port = it.next().and_then(|p| p.parse().ok()).unwrap_or(4222);
    (host, port)
}

/// Fire-and-forget NATS publish over a short-lived connection, matching the
/// DM bridge's approach. Hive volume is low enough that per-message connect
/// keeps the daemon stateless; revisit if throughput demands it.
fn nats_publish(cfg: &Config, subject: &str, payload: &str) -> std::io::Result<()> {
    let (host, port) = nats_hostport(&cfg.nats_url);
    let mut sock = TcpStream::connect((host.as_str(), port))?;
    sock.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut buf = [0u8; 2048];
    let _ = sock.read(&mut buf)?; // INFO line
    let connect = if !cfg.nats_user.is_empty() {
        format!(
            "CONNECT {{\"verbose\":false,\"pedantic\":false,\"name\":\"kannaka-hive-bridge\",\"user\":\"{}\",\"pass\":\"{}\"}}\r\n",
            cfg.nats_user.replace('"', "\\\""),
            cfg.nats_pass.replace('"', "\\\"")
        )
    } else {
        "CONNECT {\"verbose\":false,\"pedantic\":false,\"name\":\"kannaka-hive-bridge\"}\r\n".into()
    };
    sock.write_all(connect.as_bytes())?;
    sock.write_all(format!("PUB {} {}\r\n{}\r\n", subject, payload.len(), payload).as_bytes())?;
    sock.write_all(b"PING\r\n")?;
    let _ = sock.read(&mut buf)?;
    Ok(())
}

fn send_req<S: Read + Write>(ws: &mut tungstenite::WebSocket<S>, sub: &str, filter: &str) {
    let _ = ws.send(Message::Text(format!(r#"["REQ","{sub}",{filter}]"#).into()));
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

    let (mut ws, _) = tungstenite::connect(&cfg.relay_url).expect("connect to hive relay");
    eprintln!("[hive-bridge] connected to {}", cfg.relay_url);

    let mut authed = false;
    let mut subscribed = false;
    let mut last_policy_refresh = now_secs();

    loop {
        // Periodic policy refresh. buzz stores kind 39000 channel-scoped, so
        // live subscriptions never receive it via fan-out — a flag set after
        // startup is only ever seen by re-querying history.
        if authed && now_secs() - last_policy_refresh >= cfg.policy_refresh_secs as i64 {
            send_req(&mut ws, "policy", r#"{"kinds":[39000]}"#);
            last_policy_refresh = now_secs();
        }

        let msg = match ws.read() {
            Ok(m) => m,
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
                let _ = ws.send(Message::Text(format!(r#"["AUTH",{payload}]"#).into()));
                authed = true;
                // Resolve policy and roster BEFORE opening the content
                // subscription: the roster is built from the same stream it
                // filters, so subscribing first would mislabel the first
                // messages of an agent not yet learned.
                send_req(&mut ws, "policy", r#"{"kinds":[39000]}"#);
                send_req(&mut ws, "roster", r#"{"kinds":[0,10100]}"#);
            }
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

                let ctx = MapContext {
                    channel_name: None,
                    author_name: roster.display_name(&event.pubkey),
                    is_agent: roster.is_agent(&event.pubkey),
                    now_ms: now_ms(),
                };
                let Some(mapped) = map_event(&event, &ctx) else {
                    continue;
                };

                // Policy gate: everything except the agent roster is
                // channel-scoped and must clear its channel's policy.
                if mapped.subject != "agent" {
                    let channel_id = mapped.payload["channel_id"].as_str().unwrap_or("");
                    if !policy.is_bridgeable(channel_id) {
                        continue;
                    }
                }

                // Re-map with the resolved channel name now that policy has
                // confirmed the channel is known.
                let channel_id = mapped.payload["channel_id"].as_str().unwrap_or("");
                let ctx = MapContext {
                    channel_name: policy.channel_name(channel_id),
                    author_name: roster.display_name(&event.pubkey),
                    is_agent: roster.is_agent(&event.pubkey),
                    now_ms: now_ms(),
                };
                let Some(mapped) = map_event(&event, &ctx) else {
                    continue;
                };

                let subject = format!("{}.{}", cfg.subject_prefix, mapped.subject);
                let payload = mapped.payload.to_string();
                if let Err(e) = nats_publish(&cfg, &subject, &payload) {
                    eprintln!("[hive-bridge] publish failed: {e}");
                    continue;
                }
                let _ = dedup.record(&event.id);
            }
            _ => {}
        }
    }
}
