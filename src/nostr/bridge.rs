//! Bridge inbound pipeline (ADR-0043 Phase 1): the pure, testable core that
//! turns a raw kind-1059 gift-wrap event into an accept/reject decision. The
//! network daemon (`bin/kannaka-nostr-bridge`, behind the `bridge` feature)
//! wraps this with a relay WebSocket + NATS routing; everything decision-making
//! lives here so it runs in CI.
//!
//! Order matters and is load-bearing: **verify+unwrap → dedupe → rate-limit**,
//! and the dedupe is crash-durable and applied BEFORE any side effect, so a
//! restart mid-processing can never double-handle a DM (review #11). Identity
//! and rate-limiting key on the INNER sender (`UnwrappedDm.sender`), never the
//! ephemeral gift-wrap key.

use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::nip59::{unwrap_gift_wrap, UnwrappedDm};
use super::Event;

/// What to do with an inbound gift wrap.
#[derive(Debug)]
pub enum Outcome {
    /// A fresh, verified DM to route onward.
    Accept(UnwrappedDm),
    /// Already processed (dedupe hit) — drop silently.
    Duplicate,
    /// Sender exceeded their rate budget — drop.
    RateLimited,
    /// Failed verification/decryption/consistency — drop (never disclosed).
    Invalid,
}

/// Crash-durable processed-id set. Appends each accepted rumor id to a file
/// (fsync'd) BEFORE the caller acts on the DM, and keeps a bounded in-memory
/// set for fast lookup. On restart the file is replayed so nothing is
/// reprocessed. The on-disk log is compacted when it grows past `max_ids`.
pub struct Dedup {
    path: PathBuf,
    seen: std::collections::HashSet<String>,
    order: VecDeque<String>,
    max_ids: usize,
}

impl Dedup {
    /// Open (and replay) the dedupe log at `path`, keeping at most `max_ids`.
    pub fn open<P: AsRef<Path>>(path: P, max_ids: usize) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut seen = std::collections::HashSet::new();
        let mut order = VecDeque::new();
        if let Ok(contents) = std::fs::read_to_string(&path) {
            for line in contents.lines() {
                let id = line.trim();
                if !id.is_empty() && seen.insert(id.to_string()) {
                    order.push_back(id.to_string());
                }
            }
        }
        Ok(Self {
            path,
            seen,
            order,
            max_ids,
        })
    }

    pub fn contains(&self, id: &str) -> bool {
        self.seen.contains(id)
    }

    /// Record `id` as processed, durably (append + fsync) before returning.
    /// Idempotent: recording a known id is a no-op.
    pub fn record(&mut self, id: &str) -> std::io::Result<()> {
        if !self.seen.insert(id.to_string()) {
            return Ok(());
        }
        self.order.push_back(id.to_string());
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(f, "{id}")?;
        f.sync_all()?;
        if self.order.len() > self.max_ids {
            self.compact()?;
        }
        Ok(())
    }

    /// Rewrite the log with only the most-recent `max_ids` ids.
    fn compact(&mut self) -> std::io::Result<()> {
        while self.order.len() > self.max_ids {
            if let Some(old) = self.order.pop_front() {
                self.seen.remove(&old);
            }
        }
        let tmp = self.path.with_extension("tmp");
        {
            let mut f = std::fs::File::create(&tmp)?;
            for id in &self.order {
                writeln!(f, "{id}")?;
            }
            f.sync_all()?;
        }
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// Per-sender token bucket. `capacity` tokens, refilled `refill_per_sec`. Keyed
/// on the inner sender pubkey. A spammer can burn only their own bucket.
pub struct RateLimiter {
    capacity: f64,
    refill_per_sec: f64,
    buckets: HashMap<String, (f64, i64)>, // sender -> (tokens, last_secs)
}

impl RateLimiter {
    pub fn new(capacity: f64, refill_per_sec: f64) -> Self {
        Self {
            capacity,
            refill_per_sec,
            buckets: HashMap::new(),
        }
    }

    /// Try to spend one token for `sender` at time `now_secs`. Returns true if
    /// allowed. Time is passed in (no wall-clock here → deterministic tests).
    pub fn allow(&mut self, sender: &str, now_secs: i64) -> bool {
        let (tokens, last) = self
            .buckets
            .entry(sender.to_string())
            .or_insert((self.capacity, now_secs));
        let elapsed = (now_secs - *last).max(0) as f64;
        *tokens = (*tokens + elapsed * self.refill_per_sec).min(self.capacity);
        *last = now_secs;
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Process one inbound gift wrap. `our_secret_hex` is the key DMs are addressed
/// to (Kannaka's voice key). Order: unwrap+verify, then dedupe (recorded
/// durably before Accept), then rate-limit on the inner sender.
pub fn process(
    our_secret_hex: &str,
    gift_wrap: &Event,
    dedup: &mut Dedup,
    limiter: &mut RateLimiter,
    now_secs: i64,
) -> Outcome {
    let dm = match unwrap_gift_wrap(our_secret_hex, gift_wrap) {
        Ok(dm) => dm,
        Err(_) => return Outcome::Invalid,
    };
    if dedup.contains(&dm.rumor_id) {
        return Outcome::Duplicate;
    }
    if !limiter.allow(&dm.sender, now_secs) {
        // Rate-limited BEFORE recording: a throttled message is not "handled",
        // so a later retry within budget can still be processed.
        return Outcome::RateLimited;
    }
    // Record durably before the caller acts — crash here = recorded, not
    // double-sent; crash just before = not recorded, reprocessed harmlessly
    // (the responder side is expected to be idempotent on rumor_id too).
    if dedup.record(&dm.rumor_id).is_err() {
        return Outcome::Invalid; // can't guarantee once-only → refuse
    }
    Outcome::Accept(dm)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nostr::{compute_id, nip44, Keypair};

    fn make_wrap(sender: &Keypair, recipient_pubkey: &str, text: &str, nonce: u8) -> Event {
        let eph = Keypair::generate();
        let now = 1_700_000_000i64;
        let spk = sender.public_hex();
        let tags = vec![vec!["p".to_string(), recipient_pubkey.to_string()]];
        let rid = compute_id(&spk, now, 14, &tags, text);
        let rumor = serde_json::json!({
            "id": rid, "pubkey": spk, "created_at": now, "kind": 14, "tags": tags, "content": text
        })
        .to_string();
        let cks = nip44::conversation_key(&sender.secret_hex(), recipient_pubkey).unwrap();
        let seal_c = nip44::encrypt_with_nonce(&rumor, &cks, &[nonce; 32]).unwrap();
        let seal = sender.sign_event(13, Vec::new(), &seal_c, now);
        let cke = nip44::conversation_key(&eph.secret_hex(), recipient_pubkey).unwrap();
        let wrap_c = nip44::encrypt_with_nonce(
            &serde_json::to_string(&seal).unwrap(),
            &cke,
            &[nonce.wrapping_add(1); 32],
        )
        .unwrap();
        eph.sign_event(
            1059,
            vec![vec!["p".into(), recipient_pubkey.into()]],
            &wrap_c,
            now,
        )
    }

    #[test]
    fn accept_then_dedupe() {
        let dir = std::env::temp_dir().join(format!("bridge-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ddpath = dir.join("dedupe.log");
        let recipient = Keypair::generate();
        let sender = Keypair::generate();
        let wrap = make_wrap(&sender, &recipient.public_hex(), "hello kannaka", 5);

        let mut dedup = Dedup::open(&ddpath, 1000).unwrap();
        let mut limiter = RateLimiter::new(10.0, 1.0);
        // First time → Accept.
        let out = process(
            &recipient.secret_hex(),
            &wrap,
            &mut dedup,
            &mut limiter,
            1000,
        );
        let rid = match out {
            Outcome::Accept(dm) => {
                assert_eq!(dm.sender, sender.public_hex());
                assert_eq!(dm.content, "hello kannaka");
                dm.rumor_id
            }
            _ => panic!("expected Accept"),
        };
        // Same wrap again → Duplicate.
        assert!(matches!(
            process(
                &recipient.secret_hex(),
                &wrap,
                &mut dedup,
                &mut limiter,
                1000
            ),
            Outcome::Duplicate
        ));
        // Crash-durable: a fresh Dedup replayed from disk still knows the id.
        let dedup2 = Dedup::open(&ddpath, 1000).unwrap();
        assert!(dedup2.contains(&rid));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn invalid_wrap_is_dropped() {
        let recipient = Keypair::generate();
        let sender = Keypair::generate();
        let mut wrap = make_wrap(&sender, &recipient.public_hex(), "hi", 7);
        wrap.content.push('Z'); // corrupt
        let dir = std::env::temp_dir().join(format!("bridge-inv-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut dedup = Dedup::open(dir.join("d.log"), 100).unwrap();
        let mut limiter = RateLimiter::new(10.0, 1.0);
        assert!(matches!(
            process(&recipient.secret_hex(), &wrap, &mut dedup, &mut limiter, 1),
            Outcome::Invalid
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rate_limiter_per_sender() {
        let mut rl = RateLimiter::new(2.0, 0.0); // 2 tokens, no refill
        assert!(rl.allow("alice", 0));
        assert!(rl.allow("alice", 0));
        assert!(!rl.allow("alice", 0)); // alice exhausted
        assert!(rl.allow("bob", 0)); // bob independent
                                     // refill after time
        let mut rl2 = RateLimiter::new(1.0, 1.0);
        assert!(rl2.allow("c", 0));
        assert!(!rl2.allow("c", 0));
        assert!(rl2.allow("c", 1)); // 1s → +1 token
    }
}
