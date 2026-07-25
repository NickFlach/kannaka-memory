//! NIP-44 v2 encrypted payloads — the crypto floor for the bridge's inbound
//! DM path (ADR-0043 Phase 1). Hand-built on RustCrypto primitives (no C
//! toolchain), validated against the OFFICIAL nip44 test vectors (see the
//! `official_vectors_*` tests, which run every published valid/invalid case).
//!
//! Scheme (v2): conversation_key = HKDF-extract("nip44-v2", ecdh_x); per
//! message, HKDF-expand(conversation_key, info=nonce, 76B) → ChaCha20 key(32)
//! + nonce(12) + HMAC key(32). Plaintext is length-prefixed + zero-padded to a
//! bucketed length, ChaCha20-encrypted, then HMAC-SHA256(nonce ‖ ciphertext).
//! Payload = base64(0x02 ‖ nonce ‖ ciphertext ‖ mac).
//!
//! Decryption is fail-closed and constant-time on the MAC: a bad MAC, wrong
//! version, or malformed padding returns Err and never yields plaintext.

use base64::Engine;
use hmac::{Hmac, Mac};
use k256::ecdh::diffie_hellman;
use k256::{PublicKey, SecretKey};
use sha2::Sha256;

#[cfg(test)]
use super::to_hex;
use super::{from_hex, NostrError};

type HmacSha256 = Hmac<Sha256>;
const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;
const MIN_PLAINTEXT: usize = 1;
const MAX_PLAINTEXT: usize = 65535;

/// ECDH conversation key between our secret (hex) and their x-only pubkey
/// (hex). `conversation_key = HMAC-SHA256(key="nip44-v2", msg = ecdh_x)`.
pub fn conversation_key(secret_hex: &str, pubkey_x_hex: &str) -> Result<[u8; 32], NostrError> {
    let sec = from_hex(secret_hex)?;
    let sk = SecretKey::from_slice(&sec).map_err(|_| NostrError::InvalidSecretKey)?;
    // Lift the x-only pubkey to the even-Y point (BIP-340 / NIP-44 convention)
    // by prefixing the SEC1 compressed even tag.
    let x = from_hex(pubkey_x_hex)?;
    if x.len() != 32 {
        return Err(NostrError::InvalidPublicKey);
    }
    let mut sec1 = Vec::with_capacity(33);
    sec1.push(0x02);
    sec1.extend_from_slice(&x);
    let pk = PublicKey::from_sec1_bytes(&sec1).map_err(|_| NostrError::InvalidPublicKey)?;
    let shared = diffie_hellman(sk.to_nonzero_scalar(), pk.as_affine());
    let shared_x = shared.raw_secret_bytes(); // 32-byte x coordinate
    let mut mac = HmacSha256::new_from_slice(b"nip44-v2").expect("hmac key");
    mac.update(shared_x);
    let out = mac.finalize().into_bytes();
    let mut ck = [0u8; 32];
    ck.copy_from_slice(&out);
    Ok(ck)
}

/// HKDF-expand(PRK=conversation_key, info=nonce, L). SHA-256.
fn hkdf_expand(prk: &[u8; 32], info: &[u8], out_len: usize) -> Vec<u8> {
    let mut okm = Vec::with_capacity(out_len + 32);
    let mut t: Vec<u8> = Vec::new();
    let mut i: u8 = 1;
    while okm.len() < out_len {
        let mut mac = HmacSha256::new_from_slice(prk).expect("hmac key");
        mac.update(&t);
        mac.update(info);
        mac.update(&[i]);
        t = mac.finalize().into_bytes().to_vec();
        okm.extend_from_slice(&t);
        i += 1;
    }
    okm.truncate(out_len);
    okm
}

/// Per-message keys from (conversation_key, 32-byte nonce): (chacha_key[32],
/// chacha_nonce[12], hmac_key[32]).
fn message_keys(conversation_key: &[u8; 32], nonce: &[u8]) -> ([u8; 32], [u8; 12], [u8; 32]) {
    let okm = hkdf_expand(conversation_key, nonce, 76);
    let mut ck = [0u8; 32];
    let mut cn = [0u8; 12];
    let mut hk = [0u8; 32];
    ck.copy_from_slice(&okm[0..32]);
    cn.copy_from_slice(&okm[32..44]);
    hk.copy_from_slice(&okm[44..76]);
    (ck, cn, hk)
}

/// NIP-44 padded length for an unpadded plaintext byte length.
pub fn calc_padded_len(unpadded_len: usize) -> usize {
    if unpadded_len <= 32 {
        return 32;
    }
    // next power of two: 1 << bit_length(len - 1)
    let bitlen = (usize::BITS - (unpadded_len as u64 - 1).leading_zeros()) as usize;
    let next_power = 1usize << bitlen;
    let chunk = if next_power <= 256 {
        32
    } else {
        next_power / 8
    };
    chunk * ((unpadded_len - 1) / chunk + 1)
}

fn chacha20(key: &[u8; 32], nonce: &[u8; 12], buf: &mut [u8]) {
    use chacha20::cipher::{KeyIvInit, StreamCipher};
    let mut cipher = chacha20::ChaCha20::new(&(*key).into(), &(*nonce).into());
    cipher.apply_keystream(buf);
}

/// Encrypt with an explicit nonce (deterministic — for the test vectors and
/// callers that supply their own CSPRNG nonce). Returns the base64 payload.
pub fn encrypt_with_nonce(
    plaintext: &str,
    conversation_key: &[u8; 32],
    nonce: &[u8; 32],
) -> Result<String, NostrError> {
    let pt = plaintext.as_bytes();
    if pt.len() < MIN_PLAINTEXT || pt.len() > MAX_PLAINTEXT {
        return Err(NostrError::BadSignature); // length out of range → reject
    }
    let (ck, cn, hk) = message_keys(conversation_key, nonce);
    let padded_len = calc_padded_len(pt.len());
    let mut buf = Vec::with_capacity(2 + padded_len);
    buf.extend_from_slice(&(pt.len() as u16).to_be_bytes());
    buf.extend_from_slice(pt);
    buf.resize(2 + padded_len, 0);
    chacha20(&ck, &cn, &mut buf);
    let mut mac = HmacSha256::new_from_slice(&hk).expect("hmac key");
    mac.update(nonce);
    mac.update(&buf);
    let tag = mac.finalize().into_bytes();
    let mut payload = Vec::with_capacity(1 + 32 + buf.len() + 32);
    payload.push(2);
    payload.extend_from_slice(nonce);
    payload.extend_from_slice(&buf);
    payload.extend_from_slice(&tag);
    Ok(B64.encode(&payload))
}

/// Decrypt a base64 NIP-44 v2 payload with the conversation key. Fail-closed:
/// wrong version, bad MAC, or malformed padding → Err, never plaintext.
pub fn decrypt(payload_b64: &str, conversation_key: &[u8; 32]) -> Result<String, NostrError> {
    if payload_b64.starts_with('#') {
        // Reserved future-version marker per NIP-44 — not decryptable here.
        return Err(NostrError::BadSignature);
    }
    let data = B64
        .decode(payload_b64.as_bytes())
        .map_err(|_| NostrError::BadSignature)?;
    // 1 version + 32 nonce + >=1 ciphertext (>=32 padded+2) + 32 mac.
    if data.len() < 1 + 32 + 34 + 32 || data.len() > 1 + 32 + (2 + 65536) + 32 {
        return Err(NostrError::BadSignature);
    }
    if data[0] != 2 {
        return Err(NostrError::BadSignature);
    }
    let nonce = &data[1..33];
    let ct_end = data.len() - 32;
    let ciphertext = &data[33..ct_end];
    let their_mac = &data[ct_end..];
    let (ck, cn, hk) = message_keys(conversation_key, nonce);
    // Constant-time MAC verify over nonce ‖ ciphertext.
    let mut mac = HmacSha256::new_from_slice(&hk).expect("hmac key");
    mac.update(nonce);
    mac.update(ciphertext);
    mac.verify_slice(their_mac)
        .map_err(|_| NostrError::BadSignature)?;
    let mut buf = ciphertext.to_vec();
    let mut cn12 = [0u8; 12];
    cn12.copy_from_slice(&cn);
    chacha20(&ck, &cn12, &mut buf);
    if buf.len() < 2 {
        return Err(NostrError::BadSignature);
    }
    let unpadded_len = u16::from_be_bytes([buf[0], buf[1]]) as usize;
    if unpadded_len < MIN_PLAINTEXT
        || unpadded_len > buf.len() - 2
        || buf.len() != 2 + calc_padded_len(unpadded_len)
    {
        return Err(NostrError::BadSignature);
    }
    let plaintext = &buf[2..2 + unpadded_len];
    String::from_utf8(plaintext.to_vec()).map_err(|_| NostrError::BadSignature)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn vectors() -> Value {
        serde_json::from_str(include_str!("nip44.vectors.json")).expect("vectors json")
    }
    fn hx(s: &str) -> Vec<u8> {
        super::from_hex(s).unwrap()
    }

    #[test]
    fn official_conversation_keys() {
        let v = vectors();
        let cases = v["v2"]["valid"]["get_conversation_key"].as_array().unwrap();
        assert!(cases.len() >= 30);
        for c in cases {
            let ck = conversation_key(c["sec1"].as_str().unwrap(), c["pub2"].as_str().unwrap())
                .expect("conv key");
            assert_eq!(super::to_hex(&ck), c["conversation_key"].as_str().unwrap());
        }
    }

    #[test]
    fn official_calc_padded_len() {
        let v = vectors();
        for pair in v["v2"]["valid"]["calc_padded_len"].as_array().unwrap() {
            let a = pair[0].as_u64().unwrap() as usize;
            let b = pair[1].as_u64().unwrap() as usize;
            assert_eq!(calc_padded_len(a), b, "padded_len({a})");
        }
    }

    #[test]
    fn official_message_keys() {
        let v = vectors();
        let mk = &v["v2"]["valid"]["get_message_keys"];
        let ck: [u8; 32] = hx(mk["conversation_key"].as_str().unwrap())
            .try_into()
            .unwrap();
        for c in mk["keys"].as_array().unwrap() {
            let nonce = hx(c["nonce"].as_str().unwrap());
            let (cek, cen, hk) = message_keys(&ck, &nonce);
            assert_eq!(super::to_hex(&cek), c["chacha_key"].as_str().unwrap());
            assert_eq!(super::to_hex(&cen), c["chacha_nonce"].as_str().unwrap());
            assert_eq!(super::to_hex(&hk), c["hmac_key"].as_str().unwrap());
        }
    }

    #[test]
    fn official_encrypt_decrypt() {
        let v = vectors();
        for c in v["v2"]["valid"]["encrypt_decrypt"].as_array().unwrap() {
            let ck: [u8; 32] = hx(c["conversation_key"].as_str().unwrap())
                .try_into()
                .unwrap();
            let nonce: [u8; 32] = hx(c["nonce"].as_str().unwrap()).try_into().unwrap();
            let plaintext = c["plaintext"].as_str().unwrap();
            let expected_payload = c["payload"].as_str().unwrap();
            // Encrypt with the fixed nonce → exact published payload.
            let payload = encrypt_with_nonce(plaintext, &ck, &nonce).expect("encrypt");
            assert_eq!(payload, expected_payload, "encrypt mismatch");
            // Decrypt the published payload → original plaintext.
            assert_eq!(decrypt(expected_payload, &ck).expect("decrypt"), plaintext);
        }
    }

    #[test]
    fn official_invalid_decrypt_rejected() {
        let v = vectors();
        for c in v["v2"]["invalid"]["decrypt"].as_array().unwrap() {
            let ck: [u8; 32] = hx(c["conversation_key"].as_str().unwrap())
                .try_into()
                .unwrap();
            let payload = c["payload"].as_str().unwrap();
            assert!(
                decrypt(payload, &ck).is_err(),
                "invalid payload must reject: {}",
                c["note"].as_str().unwrap_or("")
            );
        }
    }

    #[test]
    fn tampered_mac_is_rejected() {
        let ck = conversation_key(
            "0000000000000000000000000000000000000000000000000000000000000001",
            "0000000000000000000000000000000000000000000000000000000000000002",
        )
        .unwrap();
        let nonce = [7u8; 32];
        let payload = encrypt_with_nonce("secret message", &ck, &nonce).unwrap();
        // Flip a byte in the middle → MAC fails.
        let mut raw = B64.decode(&payload).unwrap();
        let mid = raw.len() / 2;
        raw[mid] ^= 1;
        let tampered = B64.encode(&raw);
        assert!(decrypt(&tampered, &ck).is_err());
        // Untampered still decrypts.
        assert_eq!(decrypt(&payload, &ck).unwrap(), "secret message");
    }
}
