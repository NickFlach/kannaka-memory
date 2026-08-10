//! Canonical on-disk organ key format and its reader (#635, ADR-0055).
//!
//! Two shapes for the same key existed in the wild and nothing had chosen
//! between them: minting delivered `{nsec, npub, pubkey, organ}` while the
//! bridges read `{privkey, pubkey}`. That is not a cosmetic split — it BLOCKED
//! running `kannaka-hive-bridge` against a delivered key, because the daemon
//! could not parse the file it was given.
//!
//! The decision is **accept either**. Both shapes already exist on disk;
//! picking one and converting would break the other and require a migration
//! pass over secret material. Accepting both breaks neither and needs no pass
//! at all — nothing here ever rewrites a key file.
//!
//! The reader contract:
//!
//! | field     | requirement                                                     |
//! |-----------|-----------------------------------------------------------------|
//! | secret    | `privkey` (64 hex) OR `nsec` (bech32) — at least one; if both, they must agree |
//! | `pubkey`  | optional; when present must match the secret's derived pubkey    |
//! | `npub`    | optional; same cross-check                                       |
//! | `organ`   | optional; when the caller declares an expectation and the file carries one, they must match |
//!
//! Every cross-check is a HARD REFUSAL rather than a warning. A key loader that
//! accepts a file whose stored pubkey disagrees with its secret would have the
//! daemon speak on a relay under an identity nobody intended — precisely the
//! failure custody rules exist to prevent, and silent because every signature
//! would still be individually valid.

use std::path::Path;

use super::{npub_from_pubkey_hex, Keypair};

/// Why a key file was refused. Operator-facing: these name the specific
/// inconsistency, because the alternative is a daemon that runs as the wrong
/// identity or an `expect()` panic with no clue which field was wrong.
#[derive(Debug)]
pub enum OrganKeyError {
    Io(String),
    Json(String),
    /// Neither `privkey` nor `nsec` present.
    NoSecret,
    /// A secret was present but could not be decoded.
    BadSecret(&'static str),
    /// `privkey` and `nsec` are both present and name DIFFERENT keys.
    SecretDisagreement,
    /// Stored `pubkey`/`npub` does not derive from the secret.
    PubkeyMismatch { stored: String, derived: String },
    /// The file belongs to a different organ than the caller asked for.
    OrganMismatch { expected: String, found: String },
}

impl std::fmt::Display for OrganKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "cannot read organ key file: {e}"),
            Self::Json(e) => write!(f, "organ key file is not valid JSON: {e}"),
            Self::NoSecret => write!(
                f,
                "organ key file has no secret — expected `privkey` (64 hex) or `nsec` (bech32)"
            ),
            Self::BadSecret(which) => write!(f, "organ key `{which}` could not be decoded"),
            Self::SecretDisagreement => write!(
                f,
                "organ key file's `privkey` and `nsec` are DIFFERENT keys — refusing rather than guessing which identity was intended"
            ),
            Self::PubkeyMismatch { stored, derived } => write!(
                f,
                "organ key file's stored public key {stored} does not derive from its secret (derived {derived}) — the file is inconsistent or truncated"
            ),
            Self::OrganMismatch { expected, found } => write!(
                f,
                "organ key file belongs to organ `{found}` but `{expected}` was expected — refusing to run as the wrong organ"
            ),
        }
    }
}

impl std::error::Error for OrganKeyError {}

/// A loaded organ key: the secret, its derived public key, and the organ the
/// file claims (when it says).
#[derive(Debug)]
pub struct OrganKey {
    /// 32-byte secret, hex — what the daemons sign with.
    pub secret_hex: String,
    /// x-only public key, hex. DERIVED from the secret, never merely copied
    /// out of the file, so it cannot disagree with what we will actually sign as.
    pub pubkey_hex: String,
    /// The organ named in the file, if any.
    pub organ: Option<String>,
}

impl OrganKey {
    /// Read and validate the key file at `path`.
    ///
    /// `expected_organ` is the caller's declaration of which organ it means to
    /// run as. `None` skips that check (correct for daemons that are not
    /// organ-scoped); `Some` makes a file from a different organ a hard error.
    pub fn load<P: AsRef<Path>>(
        path: P,
        expected_organ: Option<&str>,
    ) -> Result<Self, OrganKeyError> {
        let text = std::fs::read_to_string(path.as_ref())
            .map_err(|e| OrganKeyError::Io(e.to_string()))?;
        let value: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| OrganKeyError::Json(e.to_string()))?;
        Self::from_json(&value, expected_organ)
    }

    /// The whole reader contract, over a parsed value — split out so every
    /// refusal path is unit-testable without touching the filesystem.
    pub fn from_json(
        value: &serde_json::Value,
        expected_organ: Option<&str>,
    ) -> Result<Self, OrganKeyError> {
        let field = |k: &str| {
            value
                .get(k)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
        };

        // --- secret: privkey (hex) and/or nsec (bech32) ---------------------
        let from_hex = match field("privkey") {
            Some(h) => {
                Some(Keypair::from_secret_hex(h).map_err(|_| OrganKeyError::BadSecret("privkey"))?)
            }
            None => None,
        };
        let from_nsec = match field("nsec") {
            Some(n) => Some(Keypair::from_nsec(n).map_err(|_| OrganKeyError::BadSecret("nsec"))?),
            None => None,
        };

        let keypair = match (from_hex, from_nsec) {
            (Some(a), Some(b)) => {
                // Both encodings present: they must be the same key. Preferring
                // one silently would let a stale field decide the identity.
                if a.secret_hex() != b.secret_hex() {
                    return Err(OrganKeyError::SecretDisagreement);
                }
                a
            }
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => return Err(OrganKeyError::NoSecret),
        };

        let pubkey_hex = keypair.public_hex();

        // --- stored pubkey / npub must agree with the derived key -----------
        if let Some(stored) = field("pubkey") {
            if !stored.eq_ignore_ascii_case(&pubkey_hex) {
                return Err(OrganKeyError::PubkeyMismatch {
                    stored: stored.to_string(),
                    derived: pubkey_hex,
                });
            }
        }
        if let Some(stored_npub) = field("npub") {
            // A malformed derived npub is a library problem, not a file
            // problem; treat it as a mismatch rather than claiming agreement.
            let derived_npub = npub_from_pubkey_hex(&pubkey_hex).unwrap_or_default();
            if stored_npub != derived_npub {
                return Err(OrganKeyError::PubkeyMismatch {
                    stored: stored_npub.to_string(),
                    derived: derived_npub,
                });
            }
        }

        // --- organ: optional, but checked when both sides state one ---------
        let organ = field("organ").map(str::to_string);
        if let (Some(expected), Some(found)) = (expected_organ, organ.as_deref()) {
            if expected != found {
                return Err(OrganKeyError::OrganMismatch {
                    expected: expected.to_string(),
                    found: found.to_string(),
                });
            }
        }

        Ok(Self {
            secret_hex: keypair.secret_hex(),
            pubkey_hex,
            organ,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keypair_fields() -> (String, String, String, String) {
        let kp = Keypair::generate();
        (
            kp.secret_hex(),
            kp.public_hex(),
            kp.to_nsec().unwrap(),
            kp.to_npub().unwrap(),
        )
    }

    /// The shape the bridges read today must keep working — criterion (a),
    /// existing on-disk data stays readable.
    #[test]
    fn legacy_privkey_pubkey_file_loads() {
        let (sk, pk, _, _) = keypair_fields();
        let v = serde_json::json!({ "privkey": sk, "pubkey": pk });
        let k = OrganKey::from_json(&v, None).expect("legacy shape must load");
        assert_eq!(k.pubkey_hex, pk);
        assert_eq!(k.secret_hex, sk);
        assert!(k.organ.is_none());
    }

    /// The shape minting actually delivered must load too — this is the whole
    /// hive-bridge unblock.
    #[test]
    fn delivered_nsec_npub_organ_file_loads() {
        let (sk, pk, nsec, npub) = keypair_fields();
        let v = serde_json::json!({
            "nsec": nsec, "npub": npub, "pubkey": pk, "organ": "0xscada-qe"
        });
        let k = OrganKey::from_json(&v, Some("0xscada-qe")).expect("delivered shape must load");
        assert_eq!(k.secret_hex, sk, "nsec must decode to the same secret");
        assert_eq!(k.pubkey_hex, pk);
        assert_eq!(k.organ.as_deref(), Some("0xscada-qe"));
    }

    /// Both encodings of the SAME key is fine and must not be treated as
    /// conflicting.
    #[test]
    fn privkey_and_nsec_agreeing_is_accepted() {
        let (sk, _, nsec, _) = keypair_fields();
        let v = serde_json::json!({ "privkey": sk, "nsec": nsec });
        let k = OrganKey::from_json(&v, None).expect("agreeing encodings must load");
        assert_eq!(k.secret_hex, sk);
    }

    /// ...but two DIFFERENT keys must be refused, not silently resolved by
    /// field precedence. Picking one would run the daemon as an identity the
    /// operator did not choose.
    #[test]
    fn privkey_and_nsec_naming_different_keys_is_refused() {
        let (sk, _, _, _) = keypair_fields();
        let (_, _, other_nsec, _) = keypair_fields();
        let v = serde_json::json!({ "privkey": sk, "nsec": other_nsec });
        assert!(
            matches!(
                OrganKey::from_json(&v, None),
                Err(OrganKeyError::SecretDisagreement)
            ),
            "disagreeing secrets must be refused, never resolved by precedence"
        );
    }

    /// A stored pubkey that does not derive from the secret means the file is
    /// inconsistent — every signature would still be valid, just under the
    /// wrong identity, so this has to fail loudly.
    #[test]
    fn stored_pubkey_that_does_not_derive_is_refused() {
        let (sk, _, _, _) = keypair_fields();
        let (_, other_pk, _, _) = keypair_fields();
        let v = serde_json::json!({ "privkey": sk, "pubkey": other_pk });
        assert!(matches!(
            OrganKey::from_json(&v, None),
            Err(OrganKeyError::PubkeyMismatch { .. })
        ));
    }

    /// Same check for the bech32 public encoding.
    #[test]
    fn stored_npub_that_does_not_derive_is_refused() {
        let (sk, _, _, _) = keypair_fields();
        let (_, _, _, other_npub) = keypair_fields();
        let v = serde_json::json!({ "privkey": sk, "npub": other_npub });
        assert!(matches!(
            OrganKey::from_json(&v, None),
            Err(OrganKeyError::PubkeyMismatch { .. })
        ));
    }

    /// `organ` is a guard: present on both sides and disagreeing is a refusal.
    #[test]
    fn organ_mismatch_is_refused() {
        let (sk, _, _, _) = keypair_fields();
        let v = serde_json::json!({ "privkey": sk, "organ": "radio" });
        assert!(matches!(
            OrganKey::from_json(&v, Some("0xscada-qe")),
            Err(OrganKeyError::OrganMismatch { .. })
        ));
    }

    /// ...but ABSENT organ stays permitted, or every legacy `{privkey,pubkey}`
    /// file would stop loading the moment a caller declared an expectation.
    #[test]
    fn absent_organ_is_permitted_even_when_expected() {
        let (sk, _, _, _) = keypair_fields();
        let v = serde_json::json!({ "privkey": sk });
        let k = OrganKey::from_json(&v, Some("0xscada-qe"))
            .expect("a file with no organ must still load");
        assert!(k.organ.is_none());
    }

    /// A file with no secret at all is refused rather than yielding an empty key.
    #[test]
    fn missing_secret_is_refused() {
        let v = serde_json::json!({ "pubkey": "deadbeef", "organ": "radio" });
        assert!(matches!(
            OrganKey::from_json(&v, None),
            Err(OrganKeyError::NoSecret)
        ));
    }

    /// An undecodable secret is refused with the field named — the pre-fix
    /// behaviour was an `expect()` panic that said only "privkey".
    #[test]
    fn undecodable_secret_names_the_field() {
        let v = serde_json::json!({ "privkey": "not-hex" });
        match OrganKey::from_json(&v, None) {
            Err(OrganKeyError::BadSecret(which)) => assert_eq!(which, "privkey"),
            other => panic!("expected BadSecret, got {other:?}"),
        }
    }
}
