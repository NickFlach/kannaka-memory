//! Heartbeat beacons — eclipse / partition fail-closed defense (inc-1b PART A).
//!
//! An eclipse or network partition can starve a node of the corroborations it
//! needs and, worse, feed it a manufactured local "consensus" (synthesis §4.2).
//! The corroboration gate ([`crate::absorb_gate`]) defends against this by
//! requiring a FRESH signed heartbeat from at least one operator SEED before it
//! will promote wire content to the live medium. If beacons go stale (the node
//! stops hearing from the seed set), promotion **fails closed** — content is not
//! dropped, it is frozen in the Quarantine staging tier until beacons resume.
//!
//! This module is the substrate:
//!
//! - [`Beacon`] — a `{ epoch, prev_reject_root, pubkey, sig }` statement signed
//!   over [`crate::provenance::canonical_beacon`] (`DOMAIN_BEACON ‖ epoch ‖
//!   prev_reject_root`). Only a SEED's beacon counts.
//! - [`roll_reject_root`] — the lightweight blake3 rolling hash that stands in
//!   for a full Merkle root of recently operator-rejected mem-hashes. Documented
//!   simplification (synthesis §3.4): good enough to *bind* a reject summary into
//!   the signed beacon so it can't be replayed with a swapped payload; a real
//!   Merkle tree with membership proofs is a later drop-in.
//! - [`BeaconTracker`] — records, per seed pubkey, the latest VERIFIED beacon
//!   epoch seen; [`BeaconTracker::fresh`] answers the gate's freshness question.
//!
//! **Dormant by default:** nothing here runs unless the corroboration gate is
//! armed (seeds pinned + `corroboration_gate_enabled`). With no seeds there are
//! no beacons and no behavior change.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

use crate::provenance::{canonical_beacon, VerifyErr};
use crate::reputation::PubKey;

/// The rolling reject-root before any reject has been folded in.
pub const EMPTY_REJECT_ROOT: [u8; 32] = [0u8; 32];

/// NATS subject a signed heartbeat [`Beacon`] is published on (PART A
/// anti-eclipse). Deliberately under `KANNAKA.events.>` so it rides the swarm's
/// existing anon publish/subscribe ACL — no server ACL change is needed for
/// seeds to emit or peers to receive. Seeds publish here (`publish_beacon`) and
/// the `swarm listen --auto-sync` loop subscribes + ingests.
pub const BEACON_SUBJECT: &str = "KANNAKA.events.beacon";

/// Fold one operator-rejected mem-hash into the rolling reject-root:
/// `root' = blake3(root ‖ rejected)`. A lightweight, order-sensitive stand-in
/// for a full Merkle root of recent rejects (synthesis §3.4) — enough to bind a
/// reject summary into a signed [`Beacon`]; not yet a proof-carrying tree.
pub fn roll_reject_root(prev: &[u8; 32], rejected_mem_hash: &[u8; 32]) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(prev);
    h.update(rejected_mem_hash);
    *h.finalize().as_bytes()
}

// ---------------------------------------------------------------------------
// Beacon
// ---------------------------------------------------------------------------

/// A signed heartbeat: proof that a seed was live at `epoch`, plus a summary of
/// the rejects it has seen. `epoch ‖ prev_reject_root` is signed under
/// `DOMAIN_BEACON`, so a signature minted for any other statement type (mem,
/// phase, …) fails [`Beacon::verify`] by domain separation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Beacon {
    /// Corroboration epoch this beacon attests liveness for.
    pub epoch: u64,
    /// Rolling blake3 summary of recently operator-rejected mem-hashes.
    pub prev_reject_root: [u8; 32],
    /// The signer's ed25519 verifying (public) key — must be a pinned SEED to count.
    pub pubkey: [u8; 32],
    /// The 64-byte ed25519 signature over the canonical beacon bytes.
    pub sig: [u8; 64],
}

impl Beacon {
    /// Sign a beacon for `epoch` with a raw 32-byte ed25519 seed.
    pub fn sign(signing_seed: &[u8; 32], epoch: u64, prev_reject_root: [u8; 32]) -> Beacon {
        let sk = SigningKey::from_bytes(signing_seed);
        let msg = canonical_beacon(epoch, &prev_reject_root);
        let sig = sk.sign(&msg);
        Beacon {
            epoch,
            prev_reject_root,
            pubkey: sk.verifying_key().to_bytes(),
            sig: sig.to_bytes(),
        }
    }

    /// Verify the beacon's signature over its canonical bytes. PURE — reads no
    /// clock and no seed set (the caller decides trust). Returns the verified
    /// signer pubkey on success, [`VerifyErr::BadSig`] otherwise.
    pub fn verify(&self) -> Result<PubKey, VerifyErr> {
        let vk = VerifyingKey::from_bytes(&self.pubkey).map_err(|_| VerifyErr::BadSig)?;
        let signature = Signature::from_bytes(&self.sig);
        let msg = canonical_beacon(self.epoch, &self.prev_reject_root);
        vk.verify_strict(&msg, &signature).map_err(|_| VerifyErr::BadSig)?;
        Ok(self.pubkey)
    }
}

// ---------------------------------------------------------------------------
// Beacon JSON wire form
// ---------------------------------------------------------------------------

/// On-wire JSON shape of a [`Beacon`] — the fixed-width byte fields base64
/// encoded (standard alphabet), exactly mirroring how
/// [`crate::provenance::ProvenanceSig`] is put on the NATS wire. `epoch` is a
/// plain integer. Unknown extra keys (e.g. the `schema_version`/`ts` envelope a
/// publisher may add) are ignored on decode — no `deny_unknown_fields`.
#[derive(Serialize, Deserialize)]
struct BeaconWire {
    epoch: u64,
    prev_reject_root: String,
    pubkey: String,
    sig: String,
}

impl Serialize for Beacon {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        BeaconWire {
            epoch: self.epoch,
            prev_reject_root: crate::provenance::b64_encode(&self.prev_reject_root),
            pubkey: crate::provenance::b64_encode(&self.pubkey),
            sig: crate::provenance::b64_encode(&self.sig),
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for Beacon {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use crate::provenance::b64_decode_arr;
        let w = BeaconWire::deserialize(d)?;
        Ok(Beacon {
            epoch: w.epoch,
            prev_reject_root: b64_decode_arr::<32>(&w.prev_reject_root)
                .map_err(serde::de::Error::custom)?,
            pubkey: b64_decode_arr::<32>(&w.pubkey).map_err(serde::de::Error::custom)?,
            sig: b64_decode_arr::<64>(&w.sig).map_err(serde::de::Error::custom)?,
        })
    }
}

// ---------------------------------------------------------------------------
// BeaconTracker
// ---------------------------------------------------------------------------

/// Why a beacon was not counted by [`BeaconTracker::ingest`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeaconReject {
    /// The ed25519 signature did not verify over the canonical beacon bytes.
    BadSig,
    /// The signer is not an operator-pinned seed — a non-seed beacon never counts.
    NotSeed,
    /// `epoch` is implausibly far in the future (> `now_epoch + 1`) — refusing
    /// this stops an attacker from future-dating a beacon to stay "fresh" forever.
    FutureEpoch,
    /// Not newer than the latest epoch already recorded for this seed — a
    /// replayed or stale beacon can never re-arm freshness.
    Stale,
}

impl std::fmt::Display for BeaconReject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::BadSig => "beacon signature verification failed",
            Self::NotSeed => "beacon signer is not a pinned seed",
            Self::FutureEpoch => "beacon epoch is implausibly far in the future",
            Self::Stale => "beacon epoch is not newer than the last seen (replay/stale)",
        };
        f.write_str(s)
    }
}

impl std::error::Error for BeaconReject {}

/// Per-seed latest-verified-beacon-epoch tracker. Every recorded entry is a
/// SEED (non-seed beacons are rejected at [`ingest`](BeaconTracker::ingest)), so
/// [`fresh`](BeaconTracker::fresh) can answer "did any seed beacon recently?"
/// without re-checking the seed set.
#[derive(Debug, Default, Clone)]
pub struct BeaconTracker {
    /// `seed pubkey -> latest VERIFIED beacon epoch` (monotone per seed).
    latest: HashMap<PubKey, u64>,
}

impl BeaconTracker {
    /// An empty tracker (no beacons seen ⇒ NOT fresh ⇒ promotion fails closed).
    pub fn new() -> Self {
        Self::default()
    }

    /// Verify a beacon and, if it is a fresh SEED beacon, record its epoch.
    ///
    /// Rejects, in order: a bad signature ([`BeaconReject::BadSig`]); a non-seed
    /// signer ([`BeaconReject::NotSeed`]); an epoch more than one ahead of
    /// `now_epoch` ([`BeaconReject::FutureEpoch`]); and an epoch not strictly
    /// newer than the last one recorded for this seed ([`BeaconReject::Stale`] —
    /// this is what defeats a replayed old-epoch beacon). On success returns the
    /// seed pubkey and advances its latest epoch.
    pub fn ingest(
        &mut self,
        beacon: &Beacon,
        seeds: &HashSet<PubKey>,
        now_epoch: u64,
    ) -> Result<PubKey, BeaconReject> {
        let pk = beacon.verify().map_err(|_| BeaconReject::BadSig)?;
        if !seeds.contains(&pk) {
            return Err(BeaconReject::NotSeed);
        }
        if beacon.epoch > now_epoch.saturating_add(1) {
            return Err(BeaconReject::FutureEpoch);
        }
        if let Some(&prev) = self.latest.get(&pk) {
            if beacon.epoch <= prev {
                return Err(BeaconReject::Stale);
            }
        }
        self.latest.insert(pk, beacon.epoch);
        Ok(pk)
    }

    /// Fail-closed freshness: true iff at least one recorded SEED beacon is
    /// within `grace` epochs of `now_epoch` (i.e. `now_epoch <= epoch + grace`).
    /// An empty tracker is NOT fresh — with no live beacon the gate freezes
    /// promotion, which is the safe direction under eclipse/partition.
    pub fn fresh(&self, now_epoch: u64, grace: u32) -> bool {
        let g = grace as u64;
        self.latest
            .values()
            .any(|&epoch| now_epoch <= epoch.saturating_add(g))
    }

    /// The latest verified beacon epoch for a seed, if any (inspection/tests).
    pub fn latest_epoch(&self, pk: &PubKey) -> Option<u64> {
        self.latest.get(pk).copied()
    }

    /// Number of seeds with a recorded beacon.
    pub fn len(&self) -> usize {
        self.latest.len()
    }

    /// True when no beacons have been recorded.
    pub fn is_empty(&self) -> bool {
        self.latest.is_empty()
    }

    /// Persist the tracker to `path` (best-effort JSON). A lost or corrupt file
    /// simply reloads empty — which is the fail-closed direction (freeze until a
    /// fresh beacon is re-ingested), so no atomic/fsync ceremony is needed.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let wire = BeaconTrackerWire {
            entries: self.latest.iter().map(|(pk, e)| (*pk, *e)).collect(),
        };
        let bytes =
            serde_json::to_vec(&wire).map_err(|e| format!("beacon: encode: {e}"))?;
        std::fs::write(path, bytes).map_err(|e| format!("beacon: write {}: {e}", path.display()))
    }

    /// Load the tracker from `path`. Missing or corrupt ⇒ empty (fail-closed).
    pub fn load(path: &Path) -> Self {
        let parsed = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<BeaconTrackerWire>(&b).ok());
        match parsed {
            Some(w) => BeaconTracker {
                latest: w.entries.into_iter().collect(),
            },
            None => BeaconTracker::new(),
        }
    }
}

/// On-disk shape of [`BeaconTracker`]. `[u8; 32]` is serde-native (serde ships
/// array impls up to length 32), so no base64 wire is needed here.
#[derive(Serialize, Deserialize, Default)]
struct BeaconTrackerWire {
    entries: Vec<([u8; 32], u64)>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// A seed's raw signing seed and its derived verifying pubkey.
    fn seed_key(n: u8) -> ([u8; 32], PubKey) {
        let signing = [n; 32];
        let pk = crate::provenance::verifying_key_bytes(&signing);
        (signing, pk)
    }

    fn seed_set(pks: &[PubKey]) -> HashSet<PubKey> {
        pks.iter().copied().collect()
    }

    // sign -> verify round-trip returns the signer pubkey.
    #[test]
    fn sign_verify_round_trip() {
        let (signing, pk) = seed_key(1);
        let b = Beacon::sign(&signing, 42, EMPTY_REJECT_ROOT);
        assert_eq!(b.verify(), Ok(pk));
        assert_eq!(b.pubkey, pk);
    }

    // Tampering the reject-root breaks the signature.
    #[test]
    fn tampered_reject_root_is_bad_sig() {
        let (signing, _pk) = seed_key(1);
        let mut b = Beacon::sign(&signing, 42, EMPTY_REJECT_ROOT);
        b.prev_reject_root[0] ^= 0xFF;
        assert_eq!(b.verify(), Err(VerifyErr::BadSig));
    }

    // The JSON wire form round-trips exactly, and a tampered wire byte (a
    // flipped signature byte carried across the wire) fails verify.
    #[test]
    fn wire_round_trip_and_tampered_byte_fails_verify() {
        let (signing, pk) = seed_key(1);
        let b = Beacon::sign(&signing, 77, EMPTY_REJECT_ROOT);

        // to_wire -> from_wire equal, and the round-tripped beacon still verifies.
        let json = serde_json::to_string(&b).expect("encode");
        let back: Beacon = serde_json::from_str(&json).expect("decode");
        assert_eq!(b, back, "wire round-trip is the identity");
        assert_eq!(back.verify(), Ok(pk), "round-tripped beacon still verifies");

        // Tamper ONE signature byte, carry it across the wire, and verify fails.
        let mut tampered = back.clone();
        tampered.sig[0] ^= 0xFF;
        let j2 = serde_json::to_string(&tampered).expect("encode tampered");
        let back2: Beacon = serde_json::from_str(&j2).expect("decode tampered");
        assert_eq!(
            back2.verify(),
            Err(VerifyErr::BadSig),
            "a tampered wire byte fails verify"
        );
    }

    // roll_reject_root is deterministic and order-sensitive.
    #[test]
    fn roll_reject_root_is_order_sensitive() {
        let a = [1u8; 32];
        let bb = [2u8; 32];
        let r_ab = roll_reject_root(&roll_reject_root(&EMPTY_REJECT_ROOT, &a), &bb);
        let r_ba = roll_reject_root(&roll_reject_root(&EMPTY_REJECT_ROOT, &bb), &a);
        assert_eq!(
            r_ab,
            roll_reject_root(&roll_reject_root(&EMPTY_REJECT_ROOT, &a), &bb),
            "deterministic"
        );
        assert_ne!(r_ab, r_ba, "reject order changes the root");
    }

    // A fresh seed beacon makes the tracker fresh within grace.
    #[test]
    fn fresh_seed_beacon_is_fresh() {
        let (signing, pk) = seed_key(1);
        let seeds = seed_set(&[pk]);
        let mut t = BeaconTracker::new();
        assert!(!t.fresh(10, 3), "empty tracker is never fresh");
        let b = Beacon::sign(&signing, 10, EMPTY_REJECT_ROOT);
        assert_eq!(t.ingest(&b, &seeds, 10), Ok(pk));
        assert!(t.fresh(10, 3), "same epoch is fresh");
        assert!(t.fresh(13, 3), "within grace (10..=13) is fresh");
        assert!(!t.fresh(14, 3), "beyond grace freezes");
    }

    // A NON-seed beacon does not count toward freshness.
    #[test]
    fn non_seed_beacon_rejected() {
        let (_s1, pk1) = seed_key(1); // the only pinned seed
        let (signing2, _pk2) = seed_key(2); // a non-seed signer
        let seeds = seed_set(&[pk1]);
        let mut t = BeaconTracker::new();
        let b = Beacon::sign(&signing2, 10, EMPTY_REJECT_ROOT);
        assert_eq!(t.ingest(&b, &seeds, 10), Err(BeaconReject::NotSeed));
        assert!(!t.fresh(10, 3), "a non-seed beacon leaves the tracker stale");
    }

    // A replayed old-epoch beacon is rejected (monotone per seed).
    #[test]
    fn replayed_old_epoch_rejected() {
        let (signing, pk) = seed_key(1);
        let seeds = seed_set(&[pk]);
        let mut t = BeaconTracker::new();
        let fresh = Beacon::sign(&signing, 10, EMPTY_REJECT_ROOT);
        assert_eq!(t.ingest(&fresh, &seeds, 10), Ok(pk));
        // Replay the same beacon: epoch 10 is not newer than the recorded 10.
        assert_eq!(t.ingest(&fresh, &seeds, 12), Err(BeaconReject::Stale));
        // An even older beacon is stale too.
        let old = Beacon::sign(&signing, 5, EMPTY_REJECT_ROOT);
        assert_eq!(t.ingest(&old, &seeds, 12), Err(BeaconReject::Stale));
        assert_eq!(t.latest_epoch(&pk), Some(10), "latest epoch unchanged by replays");
    }

    // A future-dated beacon is refused (can't buy permanent freshness).
    #[test]
    fn future_dated_beacon_rejected() {
        let (signing, pk) = seed_key(1);
        let seeds = seed_set(&[pk]);
        let mut t = BeaconTracker::new();
        let future = Beacon::sign(&signing, 100, EMPTY_REJECT_ROOT);
        assert_eq!(t.ingest(&future, &seeds, 10), Err(BeaconReject::FutureEpoch));
        assert!(t.is_empty());
    }

    // Persistence round-trips the latest-epoch map.
    #[test]
    fn tracker_persist_reload() {
        let (signing, pk) = seed_key(1);
        let seeds = seed_set(&[pk]);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("beacons.json");
        let mut t = BeaconTracker::new();
        t.ingest(&Beacon::sign(&signing, 7, EMPTY_REJECT_ROOT), &seeds, 7).unwrap();
        t.save(&path).unwrap();
        let loaded = BeaconTracker::load(&path);
        assert_eq!(loaded.latest_epoch(&pk), Some(7));
        assert!(loaded.fresh(9, 3));
        // A missing file loads empty (fail-closed).
        let empty = BeaconTracker::load(&dir.path().join("nope.json"));
        assert!(empty.is_empty());
    }
}
