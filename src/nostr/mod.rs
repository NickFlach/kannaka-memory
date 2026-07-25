//! NIP-01 event sign/verify + key encoding for the ADR-0043 Nostr membrane.
//!
//! This is the shared primitive Phase 0 calls for: a signer that ALSO verifies
//! inbound, computing the event id from the NIP-01-canonical byte serialization
//! rather than trusting a supplied `id` field or a non-canonical re-encode
//! (review blocker #4 — `JSON.stringify` is not canonical, and a verifier that
//! trusts the wire `id` accepts forged events whose `id`/`sig` agree but whose
//! `id` does not match the content).
//!
//! Crypto: pure-Rust BIP-340 schnorr over secp256k1 via `k256` (no C
//! toolchain), sha256 via `sha2`, bech32 (`nsec`/`npub`) via `bech32`. Nostr
//! pubkeys are the 32-byte x-only key, hex-encoded; signatures are 64-byte
//! BIP-340, hex-encoded — exactly what NIP-01 puts on the wire.

use k256::schnorr::{Signature, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

/// NIP-44 v2 encrypted payloads (bridge inbound DM crypto).
pub mod nip44;

/// NIP-59 gift wrap + NIP-17 DM unwrap (bridge inbound gate).
pub mod nip59;

/// Errors surfaced by the membrane's identity primitives. Verification errors
/// are deliberately coarse — a caller must not branch on *why* an event failed
/// to verify, only that it did.
#[derive(Debug, thiserror::Error)]
pub enum NostrError {
    #[error("invalid secret key")]
    InvalidSecretKey,
    #[error("invalid public key")]
    InvalidPublicKey,
    #[error("invalid signature encoding")]
    InvalidSignature,
    #[error("hex decode error")]
    Hex,
    #[error("bech32 error: {0}")]
    Bech32(String),
    /// The event's `id` does not match the sha256 of its canonical
    /// serialization — the content was altered after the id was computed.
    #[error("event id does not match canonical content")]
    IdMismatch,
    /// The schnorr signature does not verify against `pubkey` over `id`.
    #[error("signature verification failed")]
    BadSignature,
}

/// Fresh 32-byte aux_rand for BIP-340 signing (spec-recommended; hardens the
/// nonce against fault/side-channel even though the scheme is deterministic
/// without it). Drawn from the OS CSPRNG.
fn rand_aux() -> [u8; 32] {
    use rand_core::RngCore;
    let mut aux = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut aux);
    aux
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        s.push(char::from_digit((b & 0x0f) as u32, 16).unwrap());
    }
    s
}

fn from_hex(s: &str) -> Result<Vec<u8>, NostrError> {
    if s.len() % 2 != 0 {
        return Err(NostrError::Hex);
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| NostrError::Hex))
        .collect()
}

/// A secp256k1 keypair in its Nostr clothes. The secret never leaves as bytes
/// except through explicit `secret_hex`/`to_nsec` — callers persist it to a
/// 0600 env file (the NATS-creds custody pattern), never a repo.
pub struct Keypair {
    signing: SigningKey,
}

impl Keypair {
    /// Mint a fresh per-role key. Uses the OS CSPRNG via k256's `rand_core`.
    pub fn generate() -> Self {
        let signing = SigningKey::random(&mut rand_core::OsRng);
        Self { signing }
    }

    /// Load from a 32-byte secret, hex-encoded (as stored in the env file).
    pub fn from_secret_hex(hex: &str) -> Result<Self, NostrError> {
        let bytes = from_hex(hex)?;
        let signing = SigningKey::from_bytes(&bytes).map_err(|_| NostrError::InvalidSecretKey)?;
        Ok(Self { signing })
    }

    /// Load from an `nsec1…` bech32 secret.
    pub fn from_nsec(nsec: &str) -> Result<Self, NostrError> {
        let (hrp, data) = bech32::decode(nsec).map_err(|e| NostrError::Bech32(e.to_string()))?;
        if hrp.as_str() != "nsec" {
            return Err(NostrError::InvalidSecretKey);
        }
        let signing = SigningKey::from_bytes(&data).map_err(|_| NostrError::InvalidSecretKey)?;
        Ok(Self { signing })
    }

    /// 32-byte secret, hex-encoded. For writing the 0600 env file only.
    pub fn secret_hex(&self) -> String {
        to_hex(&self.signing.to_bytes())
    }

    /// 32-byte x-only public key, hex-encoded — the NIP-01 `pubkey`.
    pub fn public_hex(&self) -> String {
        to_hex(&self.signing.verifying_key().to_bytes())
    }

    /// `nsec1…` bech32 secret (NIP-19).
    pub fn to_nsec(&self) -> Result<String, NostrError> {
        let hrp = bech32::Hrp::parse("nsec").map_err(|e| NostrError::Bech32(e.to_string()))?;
        bech32::encode::<bech32::Bech32>(hrp, &self.signing.to_bytes())
            .map_err(|e| NostrError::Bech32(e.to_string()))
    }

    /// `npub1…` bech32 public key (NIP-19).
    pub fn to_npub(&self) -> Result<String, NostrError> {
        npub_from_pubkey_hex(&self.public_hex())
    }

    /// Sign the given event fields, returning a complete, wire-ready [`Event`]
    /// with a canonical `id` and BIP-340 `sig`.
    pub fn sign_event(
        &self,
        kind: u32,
        tags: Vec<Vec<String>>,
        content: &str,
        created_at: i64,
    ) -> Event {
        let pubkey = self.public_hex();
        let id_hex = compute_id(&pubkey, created_at, kind, &tags, content);
        let id_bytes = from_hex(&id_hex).expect("compute_id yields hex");
        // BIP-340 sign the 32-byte event id AS the raw message. `sign_raw`
        // (not the `Signer` trait, which SHA-256-prefixes the message first) —
        // the Nostr id is already a sha256, and signing sha256(id) would be
        // self-consistent but rejected by every standard BIP-340 verifier.
        let sig: Signature = self
            .signing
            .sign_raw(&id_bytes, &rand_aux())
            .expect("schnorr sign_raw over 32-byte id");
        Event {
            id: id_hex,
            pubkey,
            created_at,
            kind,
            tags,
            content: content.to_string(),
            sig: to_hex(&sig.to_bytes()),
        }
    }

    /// BIP-340 schnorr-sign an arbitrary 32-byte digest (raw message, no
    /// pre-hash), returning the 64-byte signature as hex. For protocol
    /// commitments outside NIP-01 — e.g. the KAX npub↔bot binding digest.
    pub fn sign_digest(&self, digest: &[u8; 32]) -> String {
        let sig: Signature = self
            .signing
            .sign_raw(digest, &rand_aux())
            .expect("schnorr sign_raw over 32-byte digest");
        to_hex(&sig.to_bytes())
    }
}

/// The KAX npub↔bot binding commitment digest (ADR-0043). MUST byte-match the
/// server's `npubBindDigest` in Agent-Kax (`artifacts/api-server/src/lib/
/// npubBind.ts`): sha256 of the compact canonical JSON array
/// `["kax:npub-bind:v1", domain, npub, botId, userId, nonce]`. Domain-separated
/// by the leading string tag so it can never collide with a NIP-01 event id
/// (whose array begins with the integer 0) — a signature gathered for a KAX
/// binding can never be replayed as a signed Nostr event.
pub fn kax_bind_digest(
    domain: &str,
    npub: &str,
    bot_id: &str,
    user_id: &str,
    nonce: &str,
) -> [u8; 32] {
    let value = serde_json::json!(["kax:npub-bind:v1", domain, npub, bot_id, user_id, nonce]);
    let canonical = serde_json::to_string(&value).expect("json array serializes");
    let digest = Sha256::digest(canonical.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Encode a 32-byte x-only pubkey hex as `npub1…`.
pub fn npub_from_pubkey_hex(pubkey_hex: &str) -> Result<String, NostrError> {
    let bytes = from_hex(pubkey_hex)?;
    let hrp = bech32::Hrp::parse("npub").map_err(|e| NostrError::Bech32(e.to_string()))?;
    bech32::encode::<bech32::Bech32>(hrp, &bytes).map_err(|e| NostrError::Bech32(e.to_string()))
}

/// A NIP-01 event. `id` and `sig` are hex; `pubkey` is 32-byte x-only hex.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub id: String,
    pub pubkey: String,
    pub created_at: i64,
    pub kind: u32,
    pub tags: Vec<Vec<String>>,
    pub content: String,
    pub sig: String,
}

impl Event {
    /// Verify the event end-to-end: recompute the id from the canonical
    /// serialization (NOT the wire `id`), reject on mismatch, then BIP-340
    /// verify `sig` against `pubkey` over that recomputed id. This is the
    /// membrane's inbound gate — every side effect must sit behind it.
    pub fn verify(&self) -> Result<(), NostrError> {
        let expected = compute_id(
            &self.pubkey,
            self.created_at,
            self.kind,
            &self.tags,
            &self.content,
        );
        // Constant-time-ish string compare is unnecessary here — the id is
        // public — but a mismatch is a hard reject before any crypto.
        if expected != self.id {
            return Err(NostrError::IdMismatch);
        }
        let id_bytes = from_hex(&self.id)?;
        schnorr_verify_raw(&self.pubkey, &id_bytes, &self.sig)
    }

    /// `npub1…` of the author.
    pub fn author_npub(&self) -> Result<String, NostrError> {
        npub_from_pubkey_hex(&self.pubkey)
    }
}

/// NIP-01 canonical serialization: the UTF-8 JSON array
/// `[0, pubkey, created_at, kind, tags, content]` with no insignificant
/// whitespace. serde_json's compact encoder escapes exactly the set NIP-01
/// mandates (`"`, `\\`, and the control chars, with `\n\r\t\b\f` as short
/// escapes) and nothing more (no forward-slash or non-ASCII escaping), so its
/// output is the canonical form. Building the array through `serde_json::Value`
/// keeps field order and encoding deterministic.
pub fn canonical_serialization(
    pubkey_hex: &str,
    created_at: i64,
    kind: u32,
    tags: &[Vec<String>],
    content: &str,
) -> String {
    let value = serde_json::json!([0, pubkey_hex, created_at, kind, tags, content]);
    serde_json::to_string(&value).expect("json array serializes")
}

/// BIP-340 verify `sig_hex` against `pubkey_hex` (32-byte x-only) over the
/// raw `msg` bytes — no message pre-hashing (Nostr's `m` is the event id,
/// already a sha256). This is the single point that must stay on `verify_raw`;
/// the official-vector test below guards it.
pub fn schnorr_verify_raw(pubkey_hex: &str, msg: &[u8], sig_hex: &str) -> Result<(), NostrError> {
    let pk_bytes = from_hex(pubkey_hex)?;
    let vk = VerifyingKey::from_bytes(&pk_bytes).map_err(|_| NostrError::InvalidPublicKey)?;
    let sig_bytes = from_hex(sig_hex)?;
    let sig =
        Signature::try_from(sig_bytes.as_slice()).map_err(|_| NostrError::InvalidSignature)?;
    vk.verify_raw(msg, &sig)
        .map_err(|_| NostrError::BadSignature)
}

/// sha256 of the canonical serialization, hex-encoded — the NIP-01 event id.
pub fn compute_id(
    pubkey_hex: &str,
    created_at: i64,
    kind: u32,
    tags: &[Vec<String>],
    content: &str,
) -> String {
    let canonical = canonical_serialization(pubkey_hex, created_at, kind, tags, content);
    let digest = Sha256::digest(canonical.as_bytes());
    to_hex(&digest)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Known NIP-01 vector. The canonical bytes and their sha256 were computed
    // by an independent implementation (Python `json.dumps(..., separators=
    // (',',':'), ensure_ascii=False)` + hashlib). A serialization regression —
    // a stray space, wrong escape, reordered field — changes the digest and
    // trips this.
    #[test]
    fn canonical_id_matches_known_vector() {
        let pubkey = "0000000000000000000000000000000000000000000000000000000000000001";
        let canonical = canonical_serialization(pubkey, 1_700_000_000, 1, &[], "hello");
        assert_eq!(
            canonical,
            r#"[0,"0000000000000000000000000000000000000000000000000000000000000001",1700000000,1,[],"hello"]"#
        );
        let id = compute_id(pubkey, 1_700_000_000, 1, &[], "hello");
        assert_eq!(
            id,
            "b8591d69d0638d47eb20e0505fdbaf565e52675fa998010df62813ad3d11b486"
        );
    }

    #[test]
    fn content_escaping_matches_nip01() {
        // Newline, quote, backslash must appear as \n \" \\ in the canonical
        // bytes — the exact NIP-01 escape set, nothing more.
        let s = canonical_serialization("ab", 1, 1, &[], "a\n\"b\\c");
        assert!(s.contains(r#""a\n\"b\\c""#), "got: {s}");
        // Forward slash must NOT be escaped (a common JSON encoder deviation).
        let s2 = canonical_serialization("ab", 1, 1, &[], "a/b");
        assert!(s2.contains("a/b"), "forward slash should be literal: {s2}");
    }

    // Authoritative BIP-340 spec test vector (index 1). This is the anti-
    // regression for the k256 double-hash trap: `Signer::sign`/`Verifier::verify`
    // SHA-256-prefix the message, which is self-consistent but rejects real
    // network signatures. Verifying a spec vector — where the signature was
    // produced over the RAW 32-byte message — only passes on the `verify_raw`
    // path. If someone "simplifies" back to the trait method, this fails.
    #[test]
    fn bip340_official_vector_verifies_raw() {
        let pubkey = "dff1d77f2a671c5f36183726db2341be58feae1da2deced843240f7b502ba659";
        let msg =
            from_hex("243f6a8885a308d313198a2e03707344a4093822299f31d0082efa98ec4e6c89").unwrap();
        let sig = "6896bd60eeae296db48a229ff71dfe071bde413e6d43f917dc8dcf8c78de33418906d11ac976abccb20b091292bff4ea897efcb639ea871cfa95f6de339e4b0a";
        schnorr_verify_raw(pubkey, &msg, sig).expect("official BIP-340 vector must verify");
        // A one-byte flip in the message must be rejected.
        let mut bad = msg.clone();
        bad[0] ^= 1;
        assert!(schnorr_verify_raw(pubkey, &bad, sig).is_err());
    }

    // The KAX bind digest must byte-match the Agent-Kax server's npubBindDigest.
    // Vector computed independently (Python json.dumps compact + hashlib). A
    // drift here means kannaka-signed bindings would be rejected by KAX.
    #[test]
    fn kax_bind_digest_matches_known_vector() {
        let d = kax_bind_digest(
            "kax.ninja-portal.com",
            "npub1j9t89fsgkpascqdezsrlw3p743jmkks084g6d0drzwuxaz3qaq6qx8w8dz",
            "0f05e10b-f8a1-46d6-b4a2-a7d4bae837f7",
            "user-abc",
            "0011223344556677",
        );
        assert_eq!(
            super::to_hex(&d),
            "6c9740647a3639d9ad72e3af25285714d73efb2b287679fa6cdfecaec08a476c"
        );
    }

    #[test]
    fn sign_digest_roundtrips_and_binds_pubkey() {
        let kp = Keypair::generate();
        let digest = kax_bind_digest("kax.ninja-portal.com", "npubX", "bot", "user", "nonce");
        let sig = kp.sign_digest(&digest);
        // Valid under this key over this exact digest.
        schnorr_verify_raw(&kp.public_hex(), &digest, &sig).expect("own sig verifies");
        // A one-field change to the commit invalidates it.
        let other = kax_bind_digest("kax.ninja-portal.com", "npubX", "bot", "user", "nonce2");
        assert!(schnorr_verify_raw(&kp.public_hex(), &other, &sig).is_err());
        // A different key does not verify.
        let kp2 = Keypair::generate();
        assert!(schnorr_verify_raw(&kp2.public_hex(), &digest, &sig).is_err());
    }

    #[test]
    fn sign_then_verify_roundtrips() {
        let kp = Keypair::generate();
        let ev = kp.sign_event(
            1,
            vec![vec!["t".into(), "kannaka".into()]],
            "the grid hums at 72.83Hz",
            1_700_000_000,
        );
        assert_eq!(ev.pubkey, kp.public_hex());
        ev.verify().expect("freshly signed event verifies");
    }

    #[test]
    fn tampered_content_is_rejected() {
        let kp = Keypair::generate();
        let mut ev = kp.sign_event(1, vec![], "original", 1_700_000_000);
        ev.content = "forged".into();
        // id no longer matches canonical content → reject BEFORE crypto.
        assert!(matches!(ev.verify(), Err(NostrError::IdMismatch)));
    }

    #[test]
    fn forged_id_matching_content_still_fails_on_sig() {
        // Attacker recomputes a valid id for altered content but cannot forge
        // the signature without the secret key.
        let kp = Keypair::generate();
        let mut ev = kp.sign_event(1, vec![], "original", 1_700_000_000);
        ev.content = "forged".into();
        ev.id = compute_id(&ev.pubkey, ev.created_at, ev.kind, &ev.tags, &ev.content);
        assert!(matches!(ev.verify(), Err(NostrError::BadSignature)));
    }

    #[test]
    fn wrong_pubkey_is_rejected() {
        let kp = Keypair::generate();
        let other = Keypair::generate();
        let mut ev = kp.sign_event(1, vec![], "hi", 1_700_000_000);
        // Swap in a different author but keep the signature: id changes with the
        // new pubkey, so this trips IdMismatch; if an attacker also recomputes
        // the id, the sig no longer matches the substituted key.
        ev.pubkey = other.public_hex();
        assert!(ev.verify().is_err());
    }

    #[test]
    fn nsec_npub_roundtrip() {
        let kp = Keypair::generate();
        let nsec = kp.to_nsec().unwrap();
        let npub = kp.to_npub().unwrap();
        assert!(nsec.starts_with("nsec1"));
        assert!(npub.starts_with("npub1"));
        let reloaded = Keypair::from_nsec(&nsec).unwrap();
        assert_eq!(reloaded.public_hex(), kp.public_hex());
        assert_eq!(reloaded.to_npub().unwrap(), npub);
    }

    #[test]
    fn secret_hex_roundtrip() {
        let kp = Keypair::generate();
        let reloaded = Keypair::from_secret_hex(&kp.secret_hex()).unwrap();
        assert_eq!(reloaded.public_hex(), kp.public_hex());
    }
}
