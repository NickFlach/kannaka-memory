//! NIP-59 gift wrap + NIP-17 private DM unwrap (ADR-0043 Phase 1). The bridge's
//! inbound gate for encrypted DMs.
//!
//! Three nested events (outer→inner):
//!   - kind 1059 **gift wrap** — signed by a throwaway EPHEMERAL key, hides the
//!     real sender; its content is a NIP-44 seal encrypted to us.
//!   - kind 13 **seal** — signed by the sender's REAL key; content is a NIP-44
//!     rumor encrypted to us.
//!   - kind 14 **rumor** — the actual DM, UNSIGNED by design.
//!
//! The load-bearing checks (review items #4/#5): verify the gift-wrap sig, then
//! the seal sig, then **enforce `rumor.pubkey === seal.pubkey`** — the sender
//! identity is the SEAL's key, and a rumor claiming a different author is a
//! spoof. Identify + rate-limit downstream on `sender` (the inner key), NEVER
//! the ephemeral wrapper. Fail-closed throughout: any decrypt/sig/consistency
//! failure returns Err and yields nothing.

use super::nip44;
use super::{compute_id, Event, NostrError};

/// A verified inbound DM, unwrapped from a gift wrap. `sender` is the real
/// author (seal == rumor pubkey). `rumor_id` is the dedupe key.
#[derive(Debug, Clone)]
pub struct UnwrappedDm {
    pub sender: String,
    pub content: String,
    pub rumor_id: String,
    pub created_at: i64,
    pub kind: u32,
}

/// The inner rumor. Deliberately a distinct type from [`Event`] because a
/// rumor is UNSIGNED — it carries no `sig`, so it must never flow through
/// signature verification.
#[derive(serde::Deserialize)]
struct Rumor {
    id: String,
    pubkey: String,
    created_at: i64,
    kind: u32,
    #[serde(default)]
    tags: Vec<Vec<String>>,
    content: String,
}

/// Unwrap a NIP-59 gift wrap addressed to us (`our_secret_hex`). Returns the
/// verified inner DM or an error if any layer fails to verify/decrypt or the
/// sender identity is inconsistent. A gift wrap not meant for us fails at the
/// NIP-44 MAC check and returns Err — no partial disclosure.
pub fn unwrap_gift_wrap(
    our_secret_hex: &str,
    gift_wrap: &Event,
) -> Result<UnwrappedDm, NostrError> {
    // 1. Outer gift wrap: kind + ephemeral signature.
    if gift_wrap.kind != 1059 {
        return Err(NostrError::BadSignature);
    }
    gift_wrap.verify()?;

    // 2. Decrypt gift wrap → seal, via ECDH(our_sk, ephemeral_pubkey).
    let ck_wrap = nip44::conversation_key(our_secret_hex, &gift_wrap.pubkey)?;
    let seal_json = nip44::decrypt(&gift_wrap.content, &ck_wrap)?;
    let seal: Event = serde_json::from_str(&seal_json).map_err(|_| NostrError::BadSignature)?;

    // 3. Seal: kind + the REAL sender's signature.
    if seal.kind != 13 {
        return Err(NostrError::BadSignature);
    }
    seal.verify()?;

    // 4. Decrypt seal → rumor, via ECDH(our_sk, seal.pubkey).
    let ck_seal = nip44::conversation_key(our_secret_hex, &seal.pubkey)?;
    let rumor_json = nip44::decrypt(&seal.content, &ck_seal)?;
    let rumor: Rumor = serde_json::from_str(&rumor_json).map_err(|_| NostrError::BadSignature)?;

    // 5. Sender identity IS the seal key. A rumor claiming another author is a
    // spoof — reject before trusting anything in it.
    if rumor.pubkey != seal.pubkey {
        return Err(NostrError::BadSignature);
    }
    // 6. Integrity: the rumor's own id must match its content (rumors are
    // unsigned, so the id is the only self-consistency check available).
    let recomputed = compute_id(
        &rumor.pubkey,
        rumor.created_at,
        rumor.kind,
        &rumor.tags,
        &rumor.content,
    );
    if recomputed != rumor.id {
        return Err(NostrError::IdMismatch);
    }

    Ok(UnwrappedDm {
        sender: seal.pubkey,
        content: rumor.content,
        rumor_id: rumor.id,
        created_at: rumor.created_at,
        kind: rumor.kind,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nostr::Keypair;

    // Build a full gift wrap (rumor → seal → wrap) the way a sender would, so
    // we can prove the recipient unwraps it to the original sender + message.
    fn wrap_dm(
        sender: &Keypair,
        ephemeral: &Keypair,
        recipient_pubkey_hex: &str,
        text: &str,
    ) -> Event {
        let now = 1_700_000_000i64;
        // Rumor (kind 14, UNSIGNED): id computed, sig empty.
        let sender_pk = sender.public_hex();
        let tags = vec![vec!["p".to_string(), recipient_pubkey_hex.to_string()]];
        let rumor_id = compute_id(&sender_pk, now, 14, &tags, text);
        let rumor_json = serde_json::json!({
            "id": rumor_id, "pubkey": sender_pk, "created_at": now,
            "kind": 14, "tags": tags, "content": text,
        })
        .to_string();
        // Seal (kind 13) signed by the SENDER, content = rumor encrypted to recipient.
        let ck_sender =
            nip44::conversation_key(&sender.secret_hex(), recipient_pubkey_hex).unwrap();
        let seal_content = nip44::encrypt_with_nonce(&rumor_json, &ck_sender, &[9u8; 32]).unwrap();
        let seal = sender.sign_event(13, Vec::new(), &seal_content, now);
        let seal_json = serde_json::to_string(&seal).unwrap();
        // Gift wrap (kind 1059) signed by the EPHEMERAL key, content = seal encrypted to recipient.
        let ck_eph =
            nip44::conversation_key(&ephemeral.secret_hex(), recipient_pubkey_hex).unwrap();
        let wrap_content = nip44::encrypt_with_nonce(&seal_json, &ck_eph, &[3u8; 32]).unwrap();
        let wrap_tags = vec![vec!["p".to_string(), recipient_pubkey_hex.to_string()]];
        ephemeral.sign_event(1059, wrap_tags, &wrap_content, now)
    }

    #[test]
    fn round_trip_unwrap() {
        let sender = Keypair::generate();
        let ephemeral = Keypair::generate();
        let recipient = Keypair::generate();
        let wrap = wrap_dm(
            &sender,
            &ephemeral,
            &recipient.public_hex(),
            "the grid hums",
        );
        let dm = unwrap_gift_wrap(&recipient.secret_hex(), &wrap).expect("unwrap");
        assert_eq!(dm.sender, sender.public_hex()); // real sender, not ephemeral
        assert_ne!(dm.sender, ephemeral.public_hex());
        assert_eq!(dm.content, "the grid hums");
        assert_eq!(dm.kind, 14);
    }

    #[test]
    fn wrong_recipient_cannot_unwrap() {
        let sender = Keypair::generate();
        let ephemeral = Keypair::generate();
        let recipient = Keypair::generate();
        let eavesdropper = Keypair::generate();
        let wrap = wrap_dm(&sender, &ephemeral, &recipient.public_hex(), "for her eyes");
        // A different key's ECDH yields a different conversation key → MAC fails.
        assert!(unwrap_gift_wrap(&eavesdropper.secret_hex(), &wrap).is_err());
    }

    #[test]
    fn spoofed_rumor_author_is_rejected() {
        // Attacker seals a rumor whose inner pubkey claims to be someone else.
        let attacker = Keypair::generate();
        let ephemeral = Keypair::generate();
        let recipient = Keypair::generate();
        let victim = Keypair::generate();
        let now = 1_700_000_000i64;
        let tags = vec![vec!["p".to_string(), recipient.public_hex()]];
        // Rumor claims VICTIM as author, but the seal is signed by ATTACKER.
        let rumor_id = compute_id(&victim.public_hex(), now, 14, &tags, "trust me");
        let rumor_json = serde_json::json!({
            "id": rumor_id, "pubkey": victim.public_hex(), "created_at": now,
            "kind": 14, "tags": tags, "content": "trust me",
        })
        .to_string();
        let ck_att =
            nip44::conversation_key(&attacker.secret_hex(), &recipient.public_hex()).unwrap();
        let seal_content = nip44::encrypt_with_nonce(&rumor_json, &ck_att, &[1u8; 32]).unwrap();
        let seal = attacker.sign_event(13, Vec::new(), &seal_content, now);
        let seal_json = serde_json::to_string(&seal).unwrap();
        let ck_eph =
            nip44::conversation_key(&ephemeral.secret_hex(), &recipient.public_hex()).unwrap();
        let wrap_content = nip44::encrypt_with_nonce(&seal_json, &ck_eph, &[2u8; 32]).unwrap();
        let wrap = ephemeral.sign_event(
            1059,
            vec![vec!["p".to_string(), recipient.public_hex()]],
            &wrap_content,
            now,
        );
        // rumor.pubkey (victim) != seal.pubkey (attacker) → reject.
        assert!(matches!(
            unwrap_gift_wrap(&recipient.secret_hex(), &wrap),
            Err(NostrError::BadSignature)
        ));
    }

    #[test]
    fn tampered_seal_signature_is_rejected() {
        let sender = Keypair::generate();
        let ephemeral = Keypair::generate();
        let recipient = Keypair::generate();
        let mut wrap = wrap_dm(&sender, &ephemeral, &recipient.public_hex(), "hi");
        // Corrupt the gift-wrap content so the seal fails to decrypt/parse.
        wrap.content.push('A');
        // id no longer matches → outer verify fails first.
        assert!(unwrap_gift_wrap(&recipient.secret_hex(), &wrap).is_err());
    }
}
