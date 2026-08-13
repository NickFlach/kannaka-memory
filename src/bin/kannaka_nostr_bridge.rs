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
//!   BRIDGE_VOICE_KEY_FILE   organ key json the DMs are addressed to (0600).
//!                           Accepts {privkey,pubkey} or {nsec,npub,pubkey,organ} (#635)
//!   BRIDGE_ORGAN            optional; refuses a key from a different organ
//!   BRIDGE_RELAYS           comma-separated wss:// relay urls
//!   BRIDGE_DEDUPE_FILE      crash-durable processed-id log path
//!   BRIDGE_NATS_URL/_USER/_PASS   route target (nats://host:port + creds)
//!   BRIDGE_ROUTE_SUBJECT    default KANNAKA.events.nostr.dm
//!   BRIDGE_RATE_CAP/_REFILL per-sender token bucket (default 5 / 0.05 s⁻¹)

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kannaka_memory::nostr::bridge::{
    gift_wrap_req, process, Dedup, Outcome, RateLimiter, ReplayWatermark,
    DEFAULT_REPLAY_SLACK_SECS, NIP59_BACKDATE_WINDOW_SECS,
};
use kannaka_memory::nostr::{npub_from_pubkey_hex, Event};
use tungstenite::Message;

struct Config {
    privkey: String,
    pubkey: String,
    relays: Vec<String>,
    dedupe_file: String,
    dedupe_cap: usize,
    watermark_file: String,
    replay_slack_secs: i64,
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
    // #635: one reader for both on-disk shapes. `pubkey` is now DERIVED from
    // the secret rather than copied out of the file, so a file whose stored
    // pubkey disagrees is refused instead of having this daemon advertise one
    // identity while signing as another — every signature would still verify,
    // which is exactly what makes that failure silent.
    let key = kannaka_memory::nostr::organ_key::OrganKey::load(
        &key_file,
        env("BRIDGE_ORGAN").as_deref(),
    )
    .unwrap_or_else(|e| {
        eprintln!("[bridge] {key_file}: {e}");
        std::process::exit(1);
    });
    let privkey = key.secret_hex.clone();
    let pubkey = key.pubkey_hex.clone();
    let relays = env("BRIDGE_RELAYS")
        .unwrap_or_else(|| "wss://relay.damus.io,wss://nos.lol".into())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let cfg = Config {
        privkey,
        pubkey,
        relays,
        dedupe_file: env("BRIDGE_DEDUPE_FILE")
            .unwrap_or_else(|| "/var/lib/kannaka-bridge/dedupe.log".into()),
        // #687: the dedupe log is now the SECOND line of defence. Reconnects
        // are bounded by the replay watermark below, so the cap only has to
        // cover the slack window's worth of DMs rather than the account
        // lifetime. Still configurable (~250B of RSS per id).
        dedupe_cap: env("BRIDGE_DEDUPE_CAP")
            .and_then(|s| s.parse().ok())
            .unwrap_or(100_000),
        watermark_file: env("BRIDGE_WATERMARK_FILE")
            .unwrap_or_else(|| "/var/lib/kannaka-bridge/watermark.json".into()),
        // #687 first line of defence: bound reconnect replay to a window wide
        // enough to absorb NIP-59 backdating. 0 restores unbounded replay.
        replay_slack_secs: env("BRIDGE_REPLAY_SLACK_SECS")
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_REPLAY_SLACK_SECS),
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
    };
    // A slack narrower than the NIP-59 backdating window silently drops DMs —
    // the most expensive failure this daemon has, and an invisible one. Refuse
    // to let it be configured quietly.
    if cfg.replay_slack_secs > 0 && cfg.replay_slack_secs < NIP59_BACKDATE_WINDOW_SECS {
        eprintln!(
            "[bridge] WARNING: BRIDGE_REPLAY_SLACK_SECS={} is narrower than the NIP-59\nbackdating window ({}s). Gift wraps legitimately dated further back than the\nslack will be SKIPPED on reconnect — i.e. silently lost DMs. Use >= {}s, or 0\nto disable the cursor entirely. (#687)",
            cfg.replay_slack_secs, NIP59_BACKDATE_WINDOW_SECS, NIP59_BACKDATE_WINDOW_SECS
        );
    }
    cfg
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Minimum spacing between initial-connect attempts, so a bridge started while
/// NATS is down does not pay a connect timeout on every DM.
const CONNECT_RETRY: Duration = Duration::from_secs(15);

/// How often to reclaim refilled rate-limit buckets (see the sweep in
/// `handle_relay_message`). Cheap — one pass over a small map.
const PRUNE_INTERVAL_SECS: i64 = 300;

/// Unix-seconds of the last bucket prune. Shared across relay threads; the CAS
/// keeps two threads from sweeping back-to-back.
static LAST_PRUNE_SECS: AtomicI64 = AtomicI64::new(0);

/// NATS output for the DM bridge, backed by the shared `SwarmTransport`.
///
/// This replaced a hand-rolled per-message client that was the same code as the
/// hive bridge's — its comment even said "matching the DM bridge's approach" —
/// and carried the same four drifts from `nats::handshake` (#673):
///
///   * **no connect timeout.** `TcpStream::connect` to an unreachable host
///     blocks for the OS SYN timeout, and it ran per message, so an unreachable
///     NATS host stalled this relay thread that long for every DM.
///   * credentials escaped `"` but not `\`, so a backslash in the password
///     produced malformed CONNECT JSON and an auth rejection with no clue why.
///   * INFO was read into a fixed 2048-byte buffer with one `read()`.
///   * no `tls_required` detection.
///
/// It was also worse than the hive bridge's in one way: it never sent PING or
/// waited for PONG, so an `-ERR Authorization Violation` was never read. Every
/// DM "succeeded" while the server discarded it — a silent, total routing
/// failure that looked healthy in the log.
///
/// Shared across relay threads behind a mutex. Holding it across a publish
/// serialises DMs, which is fine at DM volume and is what `SwarmTransport` does
/// internally anyway.
struct NatsSink {
    url: String,
    /// The bridge authenticates as its own principal (`BRIDGE_NATS_*`), passed
    /// explicitly so an ambient `NATS_USER` on a box running other kannaka
    /// units cannot silently re-identify it.
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
    /// `Ok(())` means on the wire or in the transport's replay buffer. Lazy
    /// connect keeps the relay side working when NATS is down, which the old
    /// stateless publish gave for free and is worth preserving.
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
                    eprintln!("[bridge] NATS connected: {}", self.url);
                    self.transport = Some(t);
                }
                Err(e) => return Err(format!("NATS connect failed: {e}")),
            }
        }
        let transport = self.transport.as_ref().expect("connected above");
        match transport.publish(subject, payload.as_bytes()) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Buffered for ordered replay once the transport revives.
                eprintln!("[bridge] buffered for replay ({e})");
                Ok(())
            }
        }
    }
}

/// One relay's connect→subscribe→process loop. Reconnects forever with backoff.
fn relay_loop(
    relay: String,
    cfg: Arc<Config>,
    dedup: Arc<Mutex<Dedup>>,
    limiter: Arc<Mutex<RateLimiter>>,
    nats: Arc<Mutex<NatsSink>>,
    watermark: Arc<Mutex<ReplayWatermark>>,
) {
    let sub_id = format!("kb-{}", &cfg.pubkey[..8]);
    loop {
        match tungstenite::connect(&relay) {
            Ok((mut socket, _resp)) => {
                // #687: rebuild the REQ per connection so it carries THIS
                // relay's current cursor. Pre-fix the REQ was built once with
                // no `since`, so every reconnect re-requested the relay's whole
                // gift-wrap history and leaned on the dedupe log to suppress it
                // — which stops working the moment the cap evicts anything.
                let since = watermark
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .since_for(&relay, cfg.replay_slack_secs);
                let req = gift_wrap_req(&sub_id, &cfg.pubkey, since);
                match since {
                    Some(s) => eprintln!("[bridge] connected {relay} (replay since {s})"),
                    None => eprintln!("[bridge] connected {relay} (full history — no cursor yet)"),
                }
                if socket.send(Message::Text(req)).is_err() {
                    continue;
                }
                loop {
                    match socket.read() {
                        Ok(Message::Text(txt)) => {
                            handle_relay_message(&txt, &cfg, &dedup, &limiter, &nats);
                            mark_receiving(&watermark, &relay);
                        }
                        Ok(Message::Ping(p)) => {
                            let _ = socket.send(Message::Pong(p));
                            // A relay keepalive proves we are still receiving,
                            // so the cursor advances on an idle-but-healthy
                            // connection instead of only when DMs arrive.
                            mark_receiving(&watermark, &relay);
                        }
                        Ok(Message::Close(_)) | Err(_) => break,
                        _ => {}
                    }
                }
                eprintln!("[bridge] disconnected {relay}");
                // Persist before backing off: the mark we just built is what
                // bounds the NEXT connection's replay.
                let w = watermark.lock().unwrap_or_else(|p| p.into_inner());
                if let Err(e) = w.flush() {
                    eprintln!("[bridge] WARN: watermark flush failed ({e}) — next reconnect replays wider");
                }
            }
            Err(e) => eprintln!("[bridge] connect {relay} failed: {e}"),
        }
        std::thread::sleep(Duration::from_secs(10));
    }
}

/// Advance this relay's "receiving through here" mark. Best-effort: a failed
/// write only means the next reconnect replays a wider window, never a
/// narrower one, so it must not interrupt DM handling.
fn mark_receiving(watermark: &Arc<Mutex<ReplayWatermark>>, relay: &str) {
    let mut w = watermark.lock().unwrap_or_else(|p| p.into_inner());
    if let Err(e) = w.record(relay, now_secs()) {
        eprintln!("[bridge] WARN: watermark write failed ({e})");
    }
}

fn handle_relay_message(
    txt: &str,
    cfg: &Config,
    dedup: &Arc<Mutex<Dedup>>,
    limiter: &Arc<Mutex<RateLimiter>>,
    nats: &Arc<Mutex<NatsSink>>,
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
        // Recover from a poisoned lock rather than panicking. A panic in one
        // relay thread would otherwise poison these and take down every other
        // relay on its next DM; the guarded state is still coherent.
        let mut d = dedup.lock().unwrap_or_else(|p| p.into_inner());
        let mut l = limiter.lock().unwrap_or_else(|p| p.into_inner());
        let now = now_secs();

        // Reclaim refilled rate-limit buckets while we already hold the lock
        // (#678 did this for the hive bridge; the shared `RateLimiter` grows one
        // entry per sender ever seen and never shed any).
        //
        // The exposure is worse here than on the hive bridge. There, authors are
        // allowlisted relay members. Here the senders are arbitrary nostr keys —
        // anyone can DM the voice key, and each new stranger left a permanent
        // bucket entry. That is unbounded memory growth reachable by an
        // unauthenticated remote party, in a daemon meant to run indefinitely.
        //
        // Dropping a FULL bucket is lossless: `allow()` re-inserts exactly
        // `(capacity, now)` on the next sighting, so nobody escapes their limit.
        // Opportunistic rather than on a timer because this daemon is purely
        // event-driven — the relay threads block in `ws.read()`, so there is no
        // tick to hang a periodic sweep on.
        let last = LAST_PRUNE_SECS.load(Ordering::Relaxed);
        if now - last >= PRUNE_INTERVAL_SECS
            && LAST_PRUNE_SECS
                .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
        {
            let dropped = l.prune(now);
            if dropped > 0 {
                eprintln!(
                    "[bridge] pruned {dropped} idle rate-limit bucket(s), {} still tracked",
                    l.tracked()
                );
            }
        }

        process(&cfg.privkey, &event, &mut d, &mut l, now)
    };
    match outcome {
        Outcome::Accept(dm) => {
            let sender_npub =
                npub_from_pubkey_hex(&dm.sender).unwrap_or_else(|_| dm.sender.clone());
            // If this DM is a `propose:` message, file it to the market door as
            // nostr:<npub> (best-effort; the DM is still routed to the responder
            // below). Dormant unless NOSTR_PROPOSE_CHANNEL_TOKEN is set.
            maybe_file_proposal(&sender_npub, &dm.content);
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
            // Lock poisoning here means another relay thread panicked mid
            // publish; the transport itself is still usable, so recover rather
            // than propagating a panic into every remaining relay.
            let mut sink = nats.lock().unwrap_or_else(|p| p.into_inner());
            match sink.publish(&cfg.route_subject, &routed) {
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

/// If a decrypted DM is a `propose:` market proposal, file it to the observatory's
/// channel-scoped propose door as `nostr:<npub>` (the sender is the VERIFIED seal
/// signer, so this principal is sound). Best-effort and non-panicking — a failed
/// file is logged, never fatal, and the DM is routed to the responder regardless.
/// Dormant unless NOSTR_PROPOSE_CHANNEL_TOKEN is set. The door re-checks the token
/// against the pinned `nostr:` prefix, so a bug here fails closed, not open.
fn maybe_file_proposal(sender_npub: &str, content: &str) {
    // Prefix-match the trigger (never substring) so Kannaka's own outbound copy
    // can't retrigger: optional leading '!', then "propose" + ':' or whitespace.
    let t = content.trim_start();
    let t = t.strip_prefix('!').unwrap_or(t);
    let is_propose = t
        .to_ascii_lowercase()
        .strip_prefix("propose")
        .is_some_and(|rest| rest.starts_with(':') || rest.starts_with(char::is_whitespace));
    if !is_propose {
        return;
    }
    let token = match std::env::var("NOSTR_PROPOSE_CHANNEL_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => return, // feature off
    };
    let base = std::env::var("OBSERVATORY_BASE_URL")
        .unwrap_or_else(|_| "https://observatory.ninja-portal.com".to_string());
    let url = format!(
        "{}/api/predictions/propose-channel",
        base.trim_end_matches('/')
    );
    let principal = format!("nostr:{sender_npub}");
    let body = serde_json::json!({ "principal": principal, "text": content });
    match ureq::post(&url)
        .set("authorization", &format!("Bearer {token}"))
        .send_json(body)
    {
        Ok(resp) => eprintln!(
            "[bridge] filed nostr proposal from {sender_npub} → {}",
            resp.status()
        ),
        Err(ureq::Error::Status(code, _)) => {
            eprintln!("[bridge] nostr proposal rejected ({code}) from {sender_npub}")
        }
        Err(e) => eprintln!("[bridge] nostr proposal POST failed: {e}"),
    }
}

fn main() {
    let cfg = Arc::new(load_config());
    if let Some(dir) = std::path::Path::new(&cfg.dedupe_file).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Some(dir) = std::path::Path::new(&cfg.watermark_file).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let dedup = Arc::new(Mutex::new(
        Dedup::open(&cfg.dedupe_file, cfg.dedupe_cap).expect("open dedupe log"),
    ));
    // #687: flushed at most once a minute — a crash loses at most that much
    // watermark, which replays MORE on the next connect, never less.
    let watermark = Arc::new(Mutex::new(
        ReplayWatermark::open(&cfg.watermark_file, 60).expect("open replay watermark"),
    ));
    let limiter = Arc::new(Mutex::new(RateLimiter::new(cfg.rate_cap, cfg.rate_refill)));
    eprintln!(
        "[bridge] up — voice {}… on {} relay(s), routing to {}",
        &cfg.pubkey[..12],
        cfg.relays.len(),
        cfg.route_subject
    );
    match cfg.replay_slack_secs {
        0 => eprintln!("[bridge] reconnect replay: UNBOUNDED (cursor disabled)"),
        s => eprintln!("[bridge] reconnect replay bounded to {s}s of slack"),
    }
    let mut handles = Vec::new();
    // One transport shared by every relay thread. Lazy-connected on the first
    // DM so a bridge started while NATS is down still serves the relay side.
    let nats = Arc::new(Mutex::new(NatsSink::new(&cfg)));

    for relay in cfg.relays.clone() {
        let (c, d, l, n, w) = (
            cfg.clone(),
            dedup.clone(),
            limiter.clone(),
            nats.clone(),
            watermark.clone(),
        );
        handles.push(std::thread::spawn(move || relay_loop(relay, c, d, l, n, w)));
    }
    for h in handles {
        let _ = h.join();
    }
}
