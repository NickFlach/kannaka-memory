//! kannaka-nostr-bridge — the ADR-0043 Phase 1 inbound membrane daemon.
//!
//! Connects to public relays, subscribes to kind-1059 gift wraps addressed to
//! Kannaka's voice key, and for each: unwraps + verifies (NIP-59/NIP-44),
//! dedupes crash-durably, rate-limits per inner sender, and routes the decoded
//! DM onto the NATS bus for the responder. All security logic lives in
//! `kannaka_memory::nostr::{nip44,nip59,bridge}` (CI-tested); this binary is the
//! network plumbing.
//!
//! Config (env):
//!   BRIDGE_VOICE_KEY_FILE   json {privkey,pubkey} the DMs are addressed to (0600)
//!   BRIDGE_RELAYS           comma-separated wss:// relay urls
//!   BRIDGE_DEDUPE_FILE      crash-durable processed-id log path
//!   BRIDGE_NATS_URL/_USER/_PASS   route target (nats://host:port + creds)
//!   BRIDGE_ROUTE_SUBJECT    default KANNAKA.events.nostr.dm
//!   BRIDGE_RATE_CAP/_REFILL per-sender token bucket (default 5 / 0.05 s⁻¹)

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kannaka_memory::nostr::bridge::{process, Dedup, Outcome, RateLimiter};
use kannaka_memory::nostr::{npub_from_pubkey_hex, Event};
use tungstenite::Message;

struct Config {
    privkey: String,
    pubkey: String,
    relays: Vec<String>,
    dedupe_file: String,
    nats_url: String,
    nats_user: String,
    nats_pass: String,
    route_subject: String,
    rate_cap: f64,
    rate_refill: f64,
}

fn env(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|s| !s.is_empty())
}

fn load_config() -> Config {
    let key_file = env("BRIDGE_VOICE_KEY_FILE").expect("BRIDGE_VOICE_KEY_FILE required");
    let key_json = std::fs::read_to_string(&key_file).expect("read voice key file");
    let key: serde_json::Value = serde_json::from_str(&key_json).expect("voice key json");
    let privkey = key["privkey"].as_str().expect("privkey").to_string();
    let pubkey = key["pubkey"].as_str().expect("pubkey").to_string();
    let relays = env("BRIDGE_RELAYS")
        .unwrap_or_else(|| "wss://relay.damus.io,wss://nos.lol".into())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    Config {
        privkey,
        pubkey,
        relays,
        dedupe_file: env("BRIDGE_DEDUPE_FILE")
            .unwrap_or_else(|| "/var/lib/kannaka-bridge/dedupe.log".into()),
        nats_url: env("BRIDGE_NATS_URL").unwrap_or_else(|| "nats://127.0.0.1:4222".into()),
        nats_user: env("BRIDGE_NATS_USER").unwrap_or_default(),
        nats_pass: env("BRIDGE_NATS_PASS").unwrap_or_default(),
        route_subject: env("BRIDGE_ROUTE_SUBJECT")
            .unwrap_or_else(|| "KANNAKA.events.nostr.dm".into()),
        rate_cap: env("BRIDGE_RATE_CAP")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5.0),
        rate_refill: env("BRIDGE_RATE_REFILL")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.05),
    }
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Minimal raw-NATS host:port parse from nats://host:port.
fn nats_hostport(url: &str) -> (String, u16) {
    let s = url.strip_prefix("nats://").unwrap_or(url);
    let mut it = s.splitn(2, ':');
    let host = it.next().unwrap_or("127.0.0.1").to_string();
    let port = it.next().and_then(|p| p.parse().ok()).unwrap_or(4222);
    (host, port)
}

/// Fire-and-forget NATS publish (short-lived connection). DMs are infrequent,
/// so per-message connect keeps the daemon simple and stateless.
fn nats_publish(cfg: &Config, payload: &str) -> std::io::Result<()> {
    let (host, port) = nats_hostport(&cfg.nats_url);
    let mut sock = TcpStream::connect((host.as_str(), port))?;
    sock.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut buf = [0u8; 2048];
    let _ = sock.read(&mut buf)?; // INFO line
    let connect = if !cfg.nats_user.is_empty() {
        format!(
            "CONNECT {{\"verbose\":false,\"pedantic\":false,\"name\":\"kannaka-nostr-bridge\",\"user\":\"{}\",\"pass\":\"{}\"}}\r\n",
            cfg.nats_user.replace('"', "\\\""),
            cfg.nats_pass.replace('"', "\\\"")
        )
    } else {
        "CONNECT {\"verbose\":false,\"pedantic\":false,\"name\":\"kannaka-nostr-bridge\"}\r\n"
            .into()
    };
    sock.write_all(connect.as_bytes())?;
    let bytes = payload.as_bytes();
    sock.write_all(format!("PUB {} {}\r\n", cfg.route_subject, bytes.len()).as_bytes())?;
    sock.write_all(bytes)?;
    sock.write_all(b"\r\n")?;
    sock.flush()?;
    Ok(())
}

/// One relay's connect→subscribe→process loop. Reconnects forever with backoff.
fn relay_loop(
    relay: String,
    cfg: Arc<Config>,
    dedup: Arc<Mutex<Dedup>>,
    limiter: Arc<Mutex<RateLimiter>>,
) {
    let sub_id = format!("kb-{}", &cfg.pubkey[..8]);
    // REQ for gift wraps p-tagged to our voice key. Relays return matching
    // stored events + stream new ones.
    let req = format!(
        "[\"REQ\",\"{sub_id}\",{{\"kinds\":[1059],\"#p\":[\"{}\"]}}]",
        cfg.pubkey
    );
    loop {
        match tungstenite::connect(&relay) {
            Ok((mut socket, _resp)) => {
                eprintln!("[bridge] connected {relay}");
                if socket.send(Message::Text(req.clone().into())).is_err() {
                    continue;
                }
                loop {
                    match socket.read() {
                        Ok(Message::Text(txt)) => {
                            handle_relay_message(&txt, &cfg, &dedup, &limiter)
                        }
                        Ok(Message::Ping(p)) => {
                            let _ = socket.send(Message::Pong(p));
                        }
                        Ok(Message::Close(_)) | Err(_) => break,
                        _ => {}
                    }
                }
                eprintln!("[bridge] disconnected {relay}");
            }
            Err(e) => eprintln!("[bridge] connect {relay} failed: {e}"),
        }
        std::thread::sleep(Duration::from_secs(10));
    }
}

fn handle_relay_message(
    txt: &str,
    cfg: &Config,
    dedup: &Arc<Mutex<Dedup>>,
    limiter: &Arc<Mutex<RateLimiter>>,
) {
    let msg: serde_json::Value = match serde_json::from_str(txt) {
        Ok(v) => v,
        Err(_) => return,
    };
    let arr = match msg.as_array() {
        Some(a) => a,
        None => return,
    };
    if arr.first().and_then(|v| v.as_str()) != Some("EVENT") || arr.len() < 3 {
        return; // EOSE / NOTICE / OK — ignore
    }
    let event: Event = match serde_json::from_value(arr[2].clone()) {
        Ok(e) => e,
        Err(_) => return,
    };
    let outcome = {
        let mut d = dedup.lock().unwrap();
        let mut l = limiter.lock().unwrap();
        process(&cfg.privkey, &event, &mut d, &mut l, now_secs())
    };
    match outcome {
        Outcome::Accept(dm) => {
            let sender_npub =
                npub_from_pubkey_hex(&dm.sender).unwrap_or_else(|_| dm.sender.clone());
            let routed = serde_json::json!({
                "type": "nostr_dm",
                "sender_hex": dm.sender,
                "sender_npub": sender_npub,
                "content": dm.content,
                "rumor_id": dm.rumor_id,
                "created_at": dm.created_at,
                "received_at": now_secs(),
            })
            .to_string();
            match nats_publish(cfg, &routed) {
                Ok(()) => eprintln!(
                    "[bridge] routed DM from {} ({} chars)",
                    sender_npub,
                    dm.content.len()
                ),
                Err(e) => eprintln!("[bridge] route publish failed: {e}"),
            }
        }
        Outcome::Duplicate => {}
        Outcome::RateLimited => eprintln!("[bridge] rate-limited a sender"),
        Outcome::Invalid => {} // silently drop — never disclose why
    }
}

fn main() {
    let cfg = Arc::new(load_config());
    if let Some(dir) = std::path::Path::new(&cfg.dedupe_file).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let dedup = Arc::new(Mutex::new(
        Dedup::open(&cfg.dedupe_file, 100_000).expect("open dedupe log"),
    ));
    let limiter = Arc::new(Mutex::new(RateLimiter::new(cfg.rate_cap, cfg.rate_refill)));
    eprintln!(
        "[bridge] up — voice {}… on {} relay(s), routing to {}",
        &cfg.pubkey[..12],
        cfg.relays.len(),
        cfg.route_subject
    );
    let mut handles = Vec::new();
    for relay in cfg.relays.clone() {
        let (c, d, l) = (cfg.clone(), dedup.clone(), limiter.clone());
        handles.push(std::thread::spawn(move || relay_loop(relay, c, d, l)));
    }
    for h in handles {
        let _ = h.join();
    }
}
