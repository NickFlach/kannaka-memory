//! Absorb gate — the inc-1b corroboration `admit()` chokepoint.
//!
//! This is the single write-side chokepoint every wire→store absorb path routes
//! through. It composes the committed inc-1a/1b substrate ([`crate::provenance`]
//! sign/verify + replay, [`crate::reputation`] pubkey-keyed trust core) into one
//! decision + one field-sanitization step.
//!
//! ## Two responsibilities
//!
//! 1. **UNCONDITIONAL sanitization ([`CleanFields`]).** Runs even when the gate
//!    is dormant. Clamps `amplitude`/`phase`/`frequency` to sane finite ranges
//!    and — critically — **forces `hallucinated` to the local default, NEVER the
//!    wire value** (an attacker must not be able to set/clear the immune flag
//!    over the wire). This is the inc-1b fix for the raw-insert gap.
//! 2. **CONDITIONAL promotion gate.** Only when
//!    `cfg.swarm_trust.corroboration_gate_enabled` is true AND seeds are pinned
//!    does [`admit`] require a valid signature and run the corroboration
//!    predicate. Otherwise it returns [`AdmitDecision::Live`] with the sanitized
//!    fields — i.e. **inc-0 behaviour is preserved** and the swarm keeps working
//!    until an operator pins seeds. **DORMANT BY DEFAULT.**
//!
//! ## Corroboration model (join key = `blake3(normalize(content))`)
//!
//! A "corroboration" is simply **another distinct trusted node independently
//! signing-and-remembering the SAME content** — there is no separate endorsement
//! message. The gate accumulates, per content-hash, the set of DISTINCT signer
//! pubkeys that have signed-remembered it, and promotes when
//! [`RepStore::decide`](crate::reputation::RepStore::decide) says `Live`.
//!
//! ## Judgment calls (documented for the reviewer)
//!
//! - **Fixed signed agent-id.** The pubkey is the real identity; the wire
//!   agent-id string is forgeable and untrusted. So both sign and verify bind a
//!   fixed [`SIGN_AGENT_ID`] into the canonical bytes, which keeps `admit()` free
//!   of an agent-id parameter while still reusing the committed `canonical_mem`.
//! - **`high_impact` = amplitude proxy.** There is no category taxonomy yet, so a
//!   wire memory whose (clamped) amplitude ≥ [`HIGH_IMPACT_AMPLITUDE`] must clear
//!   `K_high` distinct lineages. Deferred to the seed-ceremony pass.
//! - **"already Live" = `RepStore::dag().is_promoted(h)`.** The reputation DAG is
//!   the authoritative record of gate promotions; an echo re-records the edge and
//!   accrues nothing.
//! - **Always-sign on publish** (see the publish call sites): every node signs
//!   its own `memory.new`/exemplar emits, gated behind nothing. Signing is
//!   additive and harmless when the gate is off, and it is the ONLY way
//!   corroboration can accrue before an operator flips the gate on.
//! - **Beacon freshness rides in [`QuarantineStaging`].** The anti-eclipse
//!   [`BeaconTracker`] (PART A) lives inside the staging store that `admit`
//!   already threads, so the chokepoint signature is unchanged. When the gate is
//!   ACTIVE, a `Live` promotion additionally requires a FRESH seed beacon; if
//!   beacons are stale/absent (eclipse or partition) the promotion **freezes to
//!   Quarantine** (never Drop — legit content is preserved and promotes once
//!   beacons resume). Feeding the tracker is [`QuarantineStaging::ingest_beacon`]
//!   (called from the swarm receive path / `kannaka identity beacon`).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::beacon::{Beacon, BeaconReject, BeaconTracker};
use crate::config::KannakaConfig;
use crate::memory::HyperMemory;
use crate::provenance::{amp_to_q16, now_ms, verify_mem, ProvenanceSig, ReplayLru};
use crate::reputation::{Promotion, PubKey, RepStore};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Fixed agent-id bound into the signed canonical bytes on BOTH sign and verify.
/// The pubkey ([`ProvenanceSig::pubkey`]) is the real identity; the wire agent-id
/// string is forgeable, so it is deliberately not bound. Using a constant keeps
/// [`admit`] free of an agent-id parameter while reusing `canonical_mem`.
pub const SIGN_AGENT_ID: &str = "kannaka-swarm";

/// Fixed tier discriminant bound into the signed canonical bytes.
pub const PROV_TIER: u8 = 0;

/// Subject bound into a `KANNAKA.memory.new` provenance signature (sign + verify).
pub const SUBJECT_MEMORY_NEW: &str = "KANNAKA.memory.new";

/// Subject bound into a `KANNAKA.exemplar.*` provenance signature (sign + verify).
pub const SUBJECT_EXEMPLAR: &str = "KANNAKA.exemplar";

/// Hard ceiling on a wire memory's amplitude (inc-1b sanitization).
pub const MAX_WIRE_AMPLITUDE: f32 = 0.95;

/// Frequency clamp ceiling for a wire memory.
pub const MAX_WIRE_FREQUENCY: f32 = 1000.0;

/// Fallback frequency for a non-finite wire value (mirrors `WaveParams::default`).
const DEFAULT_FREQUENCY: f32 = 0.1;

/// (Clamped) amplitude at/above which a wire memory is treated as high-impact and
/// must clear `K_high` distinct lineages. Judgment call — amplitude proxy until a
/// category taxonomy lands.
pub const HIGH_IMPACT_AMPLITUDE: f32 = 0.9;

/// Max staged content-hashes retained (disk-full defense).
pub const STAGING_CAP: usize = 10_000;

/// TTL after which a staged (uncorroborated) entry is evicted, in ms (7 days).
pub const STAGING_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

// ---------------------------------------------------------------------------
// Decision + cleaned fields
// ---------------------------------------------------------------------------

/// The fate of a candidate wire memory at the absorb chokepoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmitDecision {
    /// Reject: invalid/absent signature, replay, or an echo of already-Live
    /// content. The caller inserts NO memory.
    Drop,
    /// Signed but under-corroborated: goes to the staging tier (persisted,
    /// excluded from recall/metrics/dream), NOT the live medium.
    Quarantine,
    /// Seed-endorsed cold-start without full quorum: probation tier (same
    /// staging treatment as Quarantine for this pass — not live).
    ProbationLive,
    /// Promote into the live medium — the caller inserts the memory with the
    /// sanitized [`CleanFields`].
    Live,
}

/// Sanitized wave fields the caller MUST apply before inserting a wire memory.
/// Produced on EVERY [`admit`] call, dormant or active.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CleanFields {
    /// `wire_amp` clamped to `[0, MAX_WIRE_AMPLITUDE]` (non-finite → 0).
    pub amplitude: f32,
    /// `wire_phase` reduced to `[0, 2π)` (non-finite → 0).
    pub phase: f32,
    /// `wire_freq` clamped to `[0, MAX_WIRE_FREQUENCY]` (non-finite → default).
    pub frequency: f32,
    /// ALWAYS the local default (`false`) — the wire immune flag is never trusted.
    pub hallucinated: bool,
}

impl CleanFields {
    /// Overwrite a memory's wire-controlled fields with the sanitized values.
    pub fn apply(&self, mem: &mut HyperMemory) {
        mem.amplitude = self.amplitude;
        mem.phase = self.phase;
        mem.frequency = self.frequency;
        mem.hallucinated = self.hallucinated;
    }
}

fn sanitize(wire_amp: f32, wire_phase: f32, wire_freq: f32) -> CleanFields {
    let amplitude = if wire_amp.is_finite() {
        wire_amp.clamp(0.0, MAX_WIRE_AMPLITUDE)
    } else {
        0.0
    };
    let phase = if wire_phase.is_finite() {
        wire_phase.rem_euclid(std::f32::consts::TAU)
    } else {
        0.0
    };
    let frequency = if wire_freq.is_finite() {
        wire_freq.clamp(0.0, MAX_WIRE_FREQUENCY)
    } else {
        DEFAULT_FREQUENCY
    };
    // SECURITY (inc-1b): `hallucinated` is FORCED to the local default; the wire
    // immune flag is never trusted (an attacker must not pre-set/clear it).
    CleanFields { amplitude, phase, frequency, hallucinated: false }
}

// ---------------------------------------------------------------------------
// Small pure helpers
// ---------------------------------------------------------------------------

/// True when the corroboration promotion gate is armed: master switch on AND at
/// least one seed pinned. Everything else is dormant (inc-0 fallback).
pub fn gate_active(cfg: &KannakaConfig) -> bool {
    cfg.swarm_trust.corroboration_gate_enabled && !cfg.swarm_trust.seed_pubkeys.is_empty()
}

/// Corroboration epoch for `now_ms` (`now / epoch_length_ms`, guarded).
pub fn epoch_now(cfg: &KannakaConfig, now_ms: i64) -> u64 {
    let len = cfg.swarm_trust.epoch_length_ms.max(1);
    (now_ms.max(0) / len) as u64
}

/// The corroboration join key: `blake3(normalize(content))`. Whitespace is
/// collapsed so cosmetic differences don't split a corroboration set.
pub fn content_hash(content: &str) -> [u8; 32] {
    let normalized = content.split_whitespace().collect::<Vec<_>>().join(" ");
    *blake3::hash(normalized.as_bytes()).as_bytes()
}

// ---------------------------------------------------------------------------
// admit() — the chokepoint
// ---------------------------------------------------------------------------

/// The absorb chokepoint. Returns the promotion decision plus the sanitized
/// fields the caller must apply before any insert.
///
/// - **Dormant** (`!gate_active` or no live seeds): returns `(Live, cleaned)` —
///   inc-0 behaviour preserved. The caller still applies `cleaned` (the
///   unconditional field bug-fix) and keeps whatever inc-0 source gating it had.
/// - **Active**: a valid, fresh signature is required (`Drop` otherwise). An echo
///   of already-Live content records the corroboration edge and returns `Drop`
///   (no new memory, no accrual). Otherwise the signer is accumulated into the
///   per-content staging set and [`RepStore::decide`] governs.
#[allow(clippy::too_many_arguments)]
pub fn admit(
    content: &str,
    wire_amp: f32,
    wire_phase: f32,
    wire_freq: f32,
    _wire_hallucinated: bool, // deliberately ignored — never trust the wire immune flag
    subject: &str,
    memory_id: Uuid,
    sig: Option<&ProvenanceSig>,
    staging: &mut QuarantineStaging,
    rep: &mut RepStore,
    cfg: &KannakaConfig,
    now_ms: i64,
) -> (AdmitDecision, CleanFields) {
    // (a) ALWAYS sanitize — unconditional inc-1b bug fix.
    let clean = sanitize(wire_amp, wire_phase, wire_freq);

    // (b) Dormant: no promotion logic, no signature requirement.
    if !gate_active(cfg) || rep.live_seed_count() == 0 {
        return (AdmitDecision::Live, clean);
    }

    // (c) Active: a valid + fresh signature is a hard precondition.
    let amp_q16 = amp_to_q16(wire_amp);
    let pk: PubKey = match sig {
        Some(s) => match verify_mem(
            s,
            SIGN_AGENT_ID,
            memory_id,
            content,
            subject,
            amp_q16,
            PROV_TIER,
            now_ms,
        ) {
            Ok(pk) => {
                // Freshness: single-use (memory_id, nonce); fail-closed if poisoned.
                if staging.replay.refuses_live(now_ms)
                    || staging
                        .replay
                        .check_and_insert(memory_id, &s.nonce, now_ms)
                        .is_err()
                {
                    return (AdmitDecision::Drop, clean);
                }
                pk
            }
            Err(_) => return (AdmitDecision::Drop, clean),
        },
        None => return (AdmitDecision::Drop, clean),
    };

    let epoch = epoch_now(cfg, now_ms);
    let h = content_hash(content);
    let high_impact = clean.amplitude >= HIGH_IMPACT_AMPLITUDE;

    // (d) Echo: content already Live ⇒ record the corroboration edge, insert NO
    // new memory, accrue nothing.
    if rep.dag().is_promoted(&h) {
        let root = rep.seed_root_of(&pk);
        rep.record_promotion(h, pk, epoch, vec![(pk, root)], &cfg.swarm_trust);
        let _ = staging.save();
        return (AdmitDecision::Drop, clean);
    }

    // (e) Accumulate this distinct signer, then decide.
    staging.add_corroborator(h, content, &clean, pk, now_ms);
    let origin = staging.origin(&h).unwrap_or(pk);
    let corroborators: Vec<(PubKey, Option<PubKey>)> = staging
        .corroborators(&h)
        .into_iter()
        .map(|c| (c, rep.seed_root_of(&c)))
        .collect();

    let decision = rep.decide(
        h,
        origin,
        &corroborators,
        epoch,
        rep.live_seed_count(),
        high_impact,
        /* resonance_ok = */ false,
        &cfg.swarm_trust,
    );

    match decision {
        Promotion::Live { .. } => {
            // (f) PART A anti-eclipse fail-closed: a Live promotion additionally
            // requires a FRESH seed heartbeat beacon. If beacons are stale/absent
            // (eclipse or partition), FREEZE — return Quarantine so the content
            // stays staged (never Drop legit content) and promotes once beacons
            // resume. Corroboration is NOT recorded while frozen, so the same
            // content re-decides Live the moment a fresh beacon arrives.
            if !staging.beacons.fresh(epoch, cfg.swarm_trust.beacon_grace_epochs) {
                let _ = staging.save();
                return (AdmitDecision::Quarantine, clean);
            }
            // Promote: accrue to the origin + mark Live in the DAG, then drop the
            // staged copy (the caller inserts the current memory into the medium).
            rep.record_promotion(h, origin, epoch, corroborators, &cfg.swarm_trust);
            let _ = staging.promote(&h);
            let _ = staging.save();
            (AdmitDecision::Live, clean)
        }
        Promotion::ProbationLive => {
            let _ = staging.save();
            (AdmitDecision::ProbationLive, clean)
        }
        Promotion::Quarantine => {
            let _ = staging.save();
            (AdmitDecision::Quarantine, clean)
        }
        // decide() never emits Drop, but map it for completeness.
        Promotion::Drop => {
            let _ = staging.promote(&h);
            (AdmitDecision::Drop, clean)
        }
    }
}

// ---------------------------------------------------------------------------
// QuarantineStaging — persisted, recall/metrics/dream-excluded staging tier
// ---------------------------------------------------------------------------

/// A staged (signed-but-under-corroborated) memory. Lives on disk under
/// `<data_dir>/quarantine/`, NEVER in the live `.hrm`, so it is excluded from
/// recall / metrics / dream by construction.
#[derive(Debug, Clone)]
pub struct StagedMemory {
    /// Raw content of the (first) staging message.
    pub content: String,
    /// Sanitized fields to apply on eventual promotion.
    pub cleaned: CleanFields,
    /// First time this content-hash was seen locally (ms) — TTL anchor + origin.
    pub first_seen: i64,
    /// The first signer of this content — the promotion-accrual origin.
    pub origin: PubKey,
    /// Distinct signer pubkeys accumulated for this content-hash.
    pub corroborators: HashSet<PubKey>,
}

/// Persisted content-hash-keyed staging store with a replay-freshness set.
/// Bounded by [`STAGING_CAP`] entries + [`STAGING_TTL_MS`] TTL.
#[derive(Debug)]
pub struct QuarantineStaging {
    /// `<data_dir>/quarantine/`; `None` for in-memory test stores.
    dir: Option<PathBuf>,
    entries: HashMap<[u8; 32], StagedMemory>,
    /// (memory_id, nonce) replay freshness for [`admit`].
    replay: ReplayLru,
    /// PART A anti-eclipse: per-seed latest verified heartbeat epoch. A Live
    /// promotion requires this to be fresh (see [`admit`]). Rides here so the
    /// chokepoint signature stays unchanged.
    beacons: BeaconTracker,
    cap: usize,
    ttl_ms: i64,
}

impl QuarantineStaging {
    /// An in-memory (no-persistence) staging store for tests.
    pub fn in_memory() -> Self {
        Self {
            dir: None,
            entries: HashMap::new(),
            replay: ReplayLru::new(),
            beacons: BeaconTracker::new(),
            cap: STAGING_CAP,
            ttl_ms: STAGING_TTL_MS,
        }
    }

    /// Load the staging tier from `<data_dir>/quarantine/` (staging.json +
    /// replay.json). Missing files ⇒ empty store; a corrupt staging.json is
    /// treated as empty (the corroboration record is reconstructable from the
    /// append-only reputation log). The replay set fails closed on corruption.
    pub fn load(data_dir: &Path) -> Self {
        let dir = data_dir.join("quarantine");
        let mut s = Self {
            replay: ReplayLru::load(&dir.join("replay.json"), now_ms()),
            beacons: BeaconTracker::load(&dir.join("beacons.json")),
            dir: Some(dir.clone()),
            entries: HashMap::new(),
            cap: STAGING_CAP,
            ttl_ms: STAGING_TTL_MS,
        };
        if let Ok(bytes) = std::fs::read(dir.join("staging.json")) {
            if let Ok(wire) = serde_json::from_slice::<StagingWire>(&bytes) {
                for e in wire.entries {
                    s.entries.insert(
                        e.hash,
                        StagedMemory {
                            content: e.content,
                            cleaned: CleanFields {
                                amplitude: e.amplitude,
                                phase: e.phase,
                                frequency: e.frequency,
                                hallucinated: e.hallucinated,
                            },
                            first_seen: e.first_seen,
                            origin: e.origin,
                            corroborators: e.corroborators.into_iter().collect(),
                        },
                    );
                }
            }
        }
        s
    }

    /// Number of staged content-hashes.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when nothing is staged.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether `h` is currently staged.
    pub fn contains(&self, h: &[u8; 32]) -> bool {
        self.entries.contains_key(h)
    }

    /// PART A: verify a heartbeat beacon and, if it is a fresh SEED beacon,
    /// record its epoch in the anti-eclipse tracker. Non-seed / replayed /
    /// future-dated beacons are rejected (see [`BeaconTracker::ingest`]). This is
    /// the receive path a swarm serve/join loop (or `kannaka identity beacon`
    /// out-of-band) calls; `seeds` is the pinned seed set (`RepStore::seeds`).
    pub fn ingest_beacon(
        &mut self,
        beacon: &Beacon,
        seeds: &HashSet<PubKey>,
        now_epoch: u64,
    ) -> Result<PubKey, BeaconReject> {
        self.beacons.ingest(beacon, seeds, now_epoch)
    }

    /// PART A: whether at least one seed beacon is fresh within `grace` epochs of
    /// `now_epoch` — the freshness predicate [`admit`] gates Live promotion on.
    pub fn beacons_fresh(&self, now_epoch: u64, grace: u32) -> bool {
        self.beacons.fresh(now_epoch, grace)
    }

    /// The accrual origin (first signer) of a staged content-hash.
    pub fn origin(&self, h: &[u8; 32]) -> Option<PubKey> {
        self.entries.get(h).map(|e| e.origin)
    }

    /// The distinct signer set accumulated for a staged content-hash.
    pub fn corroborators(&self, h: &[u8; 32]) -> Vec<PubKey> {
        self.entries
            .get(h)
            .map(|e| e.corroborators.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Record a distinct signer of `content` under its hash, creating the entry
    /// on first sight (the first signer is the origin). Enforces TTL + cap.
    fn add_corroborator(
        &mut self,
        h: [u8; 32],
        content: &str,
        cleaned: &CleanFields,
        pk: PubKey,
        now_ms: i64,
    ) {
        self.evict_expired(now_ms);
        if !self.entries.contains_key(&h) && self.entries.len() >= self.cap {
            self.evict_oldest();
        }
        let entry = self.entries.entry(h).or_insert_with(|| StagedMemory {
            content: content.to_string(),
            cleaned: *cleaned,
            first_seen: now_ms,
            origin: pk,
            corroborators: HashSet::new(),
        });
        entry.corroborators.insert(pk);
    }

    /// Remove and return the staged memory for `h` (used on promotion).
    pub fn promote(&mut self, h: &[u8; 32]) -> Option<StagedMemory> {
        self.entries.remove(h)
    }

    /// Drop a staged entry without returning it.
    pub fn remove(&mut self, h: &[u8; 32]) {
        self.entries.remove(h);
    }

    fn evict_expired(&mut self, now_ms: i64) {
        let ttl = self.ttl_ms;
        self.entries
            .retain(|_, e| now_ms.saturating_sub(e.first_seen) <= ttl);
    }

    fn evict_oldest(&mut self) {
        if let Some(oldest) = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.first_seen)
            .map(|(h, _)| *h)
        {
            self.entries.remove(&oldest);
        }
    }

    /// Persist the staging tier + replay set (atomic). No-op for in-memory stores.
    pub fn save(&self) -> Result<(), String> {
        let Some(dir) = &self.dir else {
            return Ok(());
        };
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("quarantine: mkdir {}: {e}", dir.display()))?;
        let wire = StagingWire {
            entries: self
                .entries
                .iter()
                .map(|(h, m)| StagedWire {
                    hash: *h,
                    content: m.content.clone(),
                    amplitude: m.cleaned.amplitude,
                    phase: m.cleaned.phase,
                    frequency: m.cleaned.frequency,
                    hallucinated: m.cleaned.hallucinated,
                    first_seen: m.first_seen,
                    origin: m.origin,
                    corroborators: m.corroborators.iter().copied().collect(),
                })
                .collect(),
        };
        let bytes = serde_json::to_vec(&wire)
            .map_err(|e| format!("quarantine: encode: {e}"))?;
        atomic_write_bytes(&dir.join("staging.json"), &bytes)?;
        // Replay persistence is best-effort — a lost replay set only re-opens the
        // skew window, and the set fails closed on a corrupt reload.
        let _ = self.replay.persist(&dir.join("replay.json"));
        // Beacon freshness is best-effort — a lost/corrupt file reloads empty,
        // which freezes promotion (the fail-closed direction) until a fresh
        // beacon is re-ingested.
        let _ = self.beacons.save(&dir.join("beacons.json"));
        Ok(())
    }
}

// --- persistence wire types -------------------------------------------------

#[derive(Serialize, Deserialize)]
struct StagedWire {
    hash: [u8; 32],
    content: String,
    amplitude: f32,
    phase: f32,
    frequency: f32,
    hallucinated: bool,
    first_seen: i64,
    origin: [u8; 32],
    corroborators: Vec<[u8; 32]>,
}

#[derive(Serialize, Deserialize, Default)]
struct StagingWire {
    entries: Vec<StagedWire>,
}

/// Atomic write: temp sibling → `write_all` → `sync_all` → rename over target.
/// Mirrors the sidecar writers elsewhere in the crate.
fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&dir).map_err(|e| format!("quarantine: mkdir {}: {e}", dir.display()))?;
    let tmp = dir.join(format!(".kannaka-staging-tmp-{}", Uuid::new_v4()));
    {
        use std::io::Write as _;
        let mut f = std::fs::File::create(&tmp).map_err(|e| format!("quarantine: tmp: {e}"))?;
        if let Err(e) = f.write_all(bytes) {
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("quarantine: write: {e}"));
        }
        if let Err(e) = f.sync_all() {
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("quarantine: sync: {e}"));
        }
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("quarantine: rename: {e}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beacon::Beacon;
    use crate::provenance::{sign_mem, verifying_key_bytes};
    use crate::reputation::RepStore;

    // --- fixtures -----------------------------------------------------------

    /// A small set of pinned-seed SIGNING keys + their base64 pubkeys.
    struct Seeds {
        signing: Vec<[u8; 32]>,
    }
    impl Seeds {
        fn new(n: u8) -> Self {
            Seeds { signing: (1..=n).map(|i| [i; 32]).collect() }
        }
        fn seed(&self, i: usize) -> [u8; 32] {
            self.signing[i]
        }
        fn pubkeys_b64(&self) -> Vec<String> {
            self.signing.iter().map(|s| b64(&verifying_key_bytes(s))).collect()
        }
    }

    fn test_cfg(seed_pubkeys: Vec<String>, gate: bool) -> KannakaConfig {
        let mut cfg = KannakaConfig::default();
        cfg.swarm_trust.corroboration_gate_enabled = gate;
        cfg.swarm_trust.seed_pubkeys = seed_pubkeys;
        cfg.swarm_trust.epoch_length_ms = 60_000;
        cfg
    }

    /// Sign `content` with a signing seed, matching what `admit` will verify.
    fn signed(
        signing: &[u8; 32],
        mem_id: Uuid,
        content: &str,
        subject: &str,
        amp: f32,
        now: i64,
    ) -> ProvenanceSig {
        let nonce = *Uuid::new_v4().as_bytes();
        sign_mem(
            signing,
            SIGN_AGENT_ID,
            mem_id,
            &nonce,
            now,
            content,
            subject,
            amp_to_q16(amp),
            PROV_TIER,
        )
    }

    /// Standard-alphabet base64 (matches `reputation::b64_decode_32`).
    fn b64(data: &[u8]) -> String {
        const A: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in data.chunks(3) {
            let b0 = chunk[0];
            let b1 = *chunk.get(1).unwrap_or(&0);
            let b2 = *chunk.get(2).unwrap_or(&0);
            let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
            out.push(A[((n >> 18) & 63) as usize] as char);
            out.push(A[((n >> 12) & 63) as usize] as char);
            out.push(if chunk.len() > 1 { A[((n >> 6) & 63) as usize] as char } else { '=' });
            out.push(if chunk.len() > 2 { A[(n & 63) as usize] as char } else { '=' });
        }
        out
    }

    const NON_SEED: [u8; 32] = [200u8; 32];

    // --- (a) dormant passthrough sanitizes ---------------------------------
    #[test]
    fn dormant_passthrough_sanitizes() {
        let cfg = test_cfg(vec![], false); // gate off, no seeds
        let mut rep = RepStore::in_memory(&cfg.swarm_trust);
        let mut staging = QuarantineStaging::in_memory();
        let (dec, clean) = admit(
            "hello world",
            5.0,   // absurd amplitude
            100.0, // absurd phase
            9e9,   // absurd frequency
            true,  // wire immune flag set — must be ignored
            SUBJECT_MEMORY_NEW,
            Uuid::new_v4(),
            None, // unsigned — but dormant, so accepted
            &mut staging,
            &mut rep,
            &cfg,
            1_000_000,
        );
        assert_eq!(dec, AdmitDecision::Live, "dormant unsigned ⇒ Live (inc-0 preserved)");
        assert!(clean.amplitude <= MAX_WIRE_AMPLITUDE, "amplitude clamped");
        assert!(!clean.hallucinated, "hallucinated forced to local default");
        assert!(clean.phase >= 0.0 && clean.phase < std::f32::consts::TAU, "phase reduced");
        assert!(clean.frequency <= MAX_WIRE_FREQUENCY, "frequency clamped");
    }

    // --- (b) active + unsigned ⇒ Drop --------------------------------------
    #[test]
    fn active_unsigned_drops() {
        let seeds = Seeds::new(3);
        let cfg = test_cfg(seeds.pubkeys_b64(), true);
        let mut rep = RepStore::in_memory(&cfg.swarm_trust);
        let mut staging = QuarantineStaging::in_memory();
        let (dec, _clean) = admit(
            "novel content here that is long enough",
            0.5,
            0.0,
            0.1,
            false,
            SUBJECT_MEMORY_NEW,
            Uuid::new_v4(),
            None,
            &mut staging,
            &mut rep,
            &cfg,
            1_000_000,
        );
        assert_eq!(dec, AdmitDecision::Drop, "active + unsigned ⇒ Drop");
    }

    // --- (c) active + signed non-seed, C<K ⇒ Quarantine --------------------
    #[test]
    fn active_signed_nonseed_quarantines() {
        let seeds = Seeds::new(3);
        let cfg = test_cfg(seeds.pubkeys_b64(), true);
        let mut rep = RepStore::in_memory(&cfg.swarm_trust);
        let mut staging = QuarantineStaging::in_memory();
        let now = 1_000_000;
        let content = "an uncorroborated claim from an unknown handle";
        let mem_id = Uuid::new_v4();
        let sig = signed(&NON_SEED, mem_id, content, SUBJECT_MEMORY_NEW, 0.5, now);
        let (dec, _c) = admit(
            content, 0.5, 0.0, 0.1, false, SUBJECT_MEMORY_NEW, mem_id, Some(&sig),
            &mut staging, &mut rep, &cfg, now,
        );
        assert_eq!(dec, AdmitDecision::Quarantine, "signed non-seed, C<K ⇒ Quarantine");
        let h = content_hash(content);
        assert!(staging.contains(&h), "stays in staging");
        assert!(!rep.dag().is_promoted(&h), "not promoted to live");
    }

    /// Drive `content` to Live: non-seed author, then K seed-disjoint lineages.
    /// K=2 for a 3-seed roster. Returns (rep, staging, cfg, h, origin_pk).
    fn promote_to_live(
        seeds: &Seeds,
        content: &str,
        now: i64,
    ) -> (RepStore, QuarantineStaging, KannakaConfig, [u8; 32], PubKey) {
        let cfg = test_cfg(seeds.pubkeys_b64(), true);
        let mut rep = RepStore::in_memory(&cfg.swarm_trust);
        let mut staging = QuarantineStaging::in_memory();

        // PART A: seed a fresh heartbeat so the Live promotion below isn't frozen
        // by the anti-eclipse gate.
        let seed_set: HashSet<PubKey> = (0..seeds.signing.len())
            .map(|i| verifying_key_bytes(&seeds.seed(i)))
            .collect();
        let now_epoch = epoch_now(&cfg, now);
        let beacon = Beacon::sign(&seeds.seed(0), now_epoch, [0u8; 32]);
        staging.ingest_beacon(&beacon, &seed_set, now_epoch).unwrap();

        // author (non-seed) → Quarantine
        let id0 = Uuid::new_v4();
        let s0 = signed(&NON_SEED, id0, content, SUBJECT_MEMORY_NEW, 0.5, now);
        let (d0, _) = admit(content, 0.5, 0.0, 0.1, false, SUBJECT_MEMORY_NEW, id0, Some(&s0),
            &mut staging, &mut rep, &cfg, now);
        assert_eq!(d0, AdmitDecision::Quarantine);

        // seed 0 corroborates → ProbationLive (C=1 < K=2, seed present)
        let id1 = Uuid::new_v4();
        let s1 = signed(&seeds.seed(0), id1, content, SUBJECT_MEMORY_NEW, 0.5, now);
        let (d1, _) = admit(content, 0.5, 0.0, 0.1, false, SUBJECT_MEMORY_NEW, id1, Some(&s1),
            &mut staging, &mut rep, &cfg, now);
        assert_eq!(d1, AdmitDecision::ProbationLive);

        // seed 1 corroborates → Live (C=2 ≥ K=2)
        let id2 = Uuid::new_v4();
        let s2 = signed(&seeds.seed(1), id2, content, SUBJECT_MEMORY_NEW, 0.5, now);
        let (d2, _) = admit(content, 0.5, 0.0, 0.1, false, SUBJECT_MEMORY_NEW, id2, Some(&s2),
            &mut staging, &mut rep, &cfg, now);
        assert_eq!(d2, AdmitDecision::Live);

        let h = content_hash(content);
        (rep, staging, cfg, h, verifying_key_bytes(&NON_SEED))
    }

    // --- (d) active + K distinct seed lineages ⇒ Live ----------------------
    #[test]
    fn active_k_seed_lineages_promote() {
        let seeds = Seeds::new(3);
        let now = 1_000_000;
        let content = "a fact corroborated by two independent operator seeds";
        let (rep, staging, _cfg, h, _origin) = promote_to_live(&seeds, content, now);
        assert!(rep.dag().is_promoted(&h), "promoted to live");
        assert!(!staging.contains(&h), "moved out of staging on promotion");
    }

    // --- PART A: stale/absent beacon freezes an otherwise-promotable memory --
    #[test]
    fn stale_beacon_freezes_then_fresh_beacon_thaws() {
        let seeds = Seeds::new(3);
        let cfg = test_cfg(seeds.pubkeys_b64(), true);
        let now = 1_000_000;
        let content = "an otherwise-promotable fact observed during an eclipse";
        let now_epoch = epoch_now(&cfg, now);

        let mut rep = RepStore::in_memory(&cfg.swarm_trust);
        let mut staging = QuarantineStaging::in_memory(); // NO beacon ⇒ eclipsed

        // author → Quarantine; seed0 → ProbationLive; seed1 → would be Live (C=2≥K=2).
        let id0 = Uuid::new_v4();
        let s0 = signed(&NON_SEED, id0, content, SUBJECT_MEMORY_NEW, 0.5, now);
        admit(content, 0.5, 0.0, 0.1, false, SUBJECT_MEMORY_NEW, id0, Some(&s0),
            &mut staging, &mut rep, &cfg, now);
        let id1 = Uuid::new_v4();
        let s1 = signed(&seeds.seed(0), id1, content, SUBJECT_MEMORY_NEW, 0.5, now);
        admit(content, 0.5, 0.0, 0.1, false, SUBJECT_MEMORY_NEW, id1, Some(&s1),
            &mut staging, &mut rep, &cfg, now);
        let id2 = Uuid::new_v4();
        let s2 = signed(&seeds.seed(1), id2, content, SUBJECT_MEMORY_NEW, 0.5, now);
        let (frozen, _) = admit(content, 0.5, 0.0, 0.1, false, SUBJECT_MEMORY_NEW, id2, Some(&s2),
            &mut staging, &mut rep, &cfg, now);

        let h = content_hash(content);
        assert_eq!(frozen, AdmitDecision::Quarantine, "stale/absent beacons freeze promotion");
        assert!(!rep.dag().is_promoted(&h), "frozen: not promoted to live");
        assert!(staging.contains(&h), "frozen content preserved in staging (never dropped)");

        // Beacons resume ⇒ the next corroboration promotes the SAME content.
        let seed_set: HashSet<PubKey> = (0..seeds.signing.len())
            .map(|i| verifying_key_bytes(&seeds.seed(i)))
            .collect();
        let beacon = Beacon::sign(&seeds.seed(0), now_epoch, [0u8; 32]);
        staging.ingest_beacon(&beacon, &seed_set, now_epoch).unwrap();
        let id3 = Uuid::new_v4();
        let s3 = signed(&seeds.seed(2), id3, content, SUBJECT_MEMORY_NEW, 0.5, now);
        let (thawed, _) = admit(content, 0.5, 0.0, 0.1, false, SUBJECT_MEMORY_NEW, id3, Some(&s3),
            &mut staging, &mut rep, &cfg, now);
        assert_eq!(thawed, AdmitDecision::Live, "a fresh seed beacon thaws promotion");
        assert!(rep.dag().is_promoted(&h), "promoted once beacons resumed");
    }

    // --- (e) echo: already-Live content ⇒ no new memory, accrues nothing ---
    #[test]
    fn echo_earns_nothing() {
        let seeds = Seeds::new(3);
        let now = 1_000_000;
        let content = "a fact corroborated then echoed";
        let (mut rep, mut staging, cfg, h, origin) = promote_to_live(&seeds, content, now);

        let rep_before = rep.rep(&origin);
        // A NEW seed (index 2) re-signs the already-Live content.
        let id3 = Uuid::new_v4();
        let s3 = signed(&seeds.seed(2), id3, content, SUBJECT_MEMORY_NEW, 0.5, now);
        let (dec, _c) = admit(content, 0.5, 0.0, 0.1, false, SUBJECT_MEMORY_NEW, id3, Some(&s3),
            &mut staging, &mut rep, &cfg, now);

        assert_eq!(dec, AdmitDecision::Drop, "echo ⇒ no new memory");
        assert!(rep.dag().is_promoted(&h), "content stays Live");
        assert_eq!(rep.rep(&origin), rep_before, "echo accrues nothing to the origin");
        assert!(!staging.contains(&h), "echo creates no staging entry");
    }

    // --- (f) wire immune/amplitude injection is neutralized ----------------
    #[test]
    fn wire_injection_is_sanitized() {
        let cfg = test_cfg(vec![], false); // dormant is sufficient for sanitization
        let mut rep = RepStore::in_memory(&cfg.swarm_trust);
        let mut staging = QuarantineStaging::in_memory();

        // Task (f): wire hallucinated=false, amplitude=1.0.
        let (_d, clean) = admit("x", 1.0, 0.0, 0.1, false, SUBJECT_MEMORY_NEW, Uuid::new_v4(),
            None, &mut staging, &mut rep, &cfg, 1);
        assert!(clean.amplitude <= MAX_WIRE_AMPLITUDE, "amplitude clamped ≤0.95");
        assert!(!clean.hallucinated, "hallucinated is the local default");

        // Stronger: attacker sets the wire immune flag TRUE — must NOT be taken.
        let (_d2, clean2) = admit("y", 1.0, 0.0, 0.1, true, SUBJECT_MEMORY_NEW, Uuid::new_v4(),
            None, &mut staging, &mut rep, &cfg, 1);
        assert!(!clean2.hallucinated, "the wire immune flag is never trusted");

        // And it applies onto a real memory.
        let mut mem = HyperMemory::new(vec![0.0; 4], "y".to_string());
        mem.hallucinated = true;
        mem.amplitude = 42.0;
        clean2.apply(&mut mem);
        assert!(!mem.hallucinated && mem.amplitude <= MAX_WIRE_AMPLITUDE);
    }

    // --- persistence round-trip (staging survives reload) ------------------
    #[test]
    fn staging_persist_reload() {
        let seeds = Seeds::new(3);
        let cfg = test_cfg(seeds.pubkeys_b64(), true);
        let dir = tempfile::tempdir().unwrap();
        let now = 1_000_000;
        let content = "a claim awaiting corroboration";
        let h = content_hash(content);
        {
            let mut rep = RepStore::in_memory(&cfg.swarm_trust);
            let mut staging = QuarantineStaging::load(dir.path());
            let id = Uuid::new_v4();
            let sig = signed(&NON_SEED, id, content, SUBJECT_MEMORY_NEW, 0.5, now);
            let (dec, _c) = admit(content, 0.5, 0.0, 0.1, false, SUBJECT_MEMORY_NEW, id, Some(&sig),
                &mut staging, &mut rep, &cfg, now);
            assert_eq!(dec, AdmitDecision::Quarantine);
        }
        let reloaded = QuarantineStaging::load(dir.path());
        assert!(reloaded.contains(&h), "staged entry survived reload");
    }
}
