//! inc-1 corroboration gate — end-to-end runtime tests (synthesis §3.5).
//!
//! These drive the REAL library surface — [`admit`] over a [`QuarantineStaging`]
//! + [`RepStore`], with a pinned-seed set and ed25519-signed wire memories — and
//! assert a concrete observable per scenario (a decision variant, a rep value, a
//! store/staging count, a promotion record). No binary spawn: the library API is
//! the same code path the wired swarm callers use, and driving it directly keeps
//! the assertions deterministic (synthesis's "prefer admit/RepStore" guidance).
//!
//! Everything here runs with the gate ARMED (seeds pinned +
//! `corroboration_gate_enabled`). The dormant/default path (no seeds ⇒ no
//! behaviour change) is covered by the in-crate `absorb_gate` unit tests.

use std::collections::HashSet;

use uuid::Uuid;

use kannaka_memory::absorb_gate::{
    admit, commit_promotion, content_hash, epoch_now, AdmitDecision, QuarantineStaging, PROV_TIER,
    SIGN_AGENT_ID, SUBJECT_MEMORY_NEW,
};
use kannaka_memory::beacon::Beacon;
use kannaka_memory::config::KannakaConfig;
use kannaka_memory::provenance::{amp_to_q16, sign_mem, verifying_key_bytes, ProvenanceSig};
use kannaka_memory::reputation::{distinct_lineage_count, k_for, Promotion, PubKey, RepStore};

// ---------------------------------------------------------------------------
// Fixtures / helpers
// ---------------------------------------------------------------------------

const NON_SEED: [u8; 32] = [200u8; 32];

/// Standard-alphabet base64 (matches the crate's provenance/reputation codec).
fn b64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
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

/// The base64 pubkeys for a set of raw signing seeds — the operator seed roster.
fn seed_pubkeys(signing: &[[u8; 32]]) -> Vec<String> {
    signing.iter().map(|s| b64(&verifying_key_bytes(s))).collect()
}

/// The pubkey set (for beacon ingest) for a set of raw signing seeds.
fn seed_set(signing: &[[u8; 32]]) -> HashSet<PubKey> {
    signing.iter().map(|s| verifying_key_bytes(s)).collect()
}

/// A gate-armed config over the given seed roster.
fn cfg(signing: &[[u8; 32]]) -> KannakaConfig {
    let mut c = KannakaConfig::default();
    c.swarm_trust.corroboration_gate_enabled = true;
    c.swarm_trust.seed_pubkeys = seed_pubkeys(signing);
    c.swarm_trust.epoch_length_ms = 60_000;
    c
}

/// Sign `content` exactly as `admit` will verify it (fixed agent-id/tier/subject).
fn sign(signing: &[u8; 32], mem_id: Uuid, content: &str, amp: f32, now: i64) -> ProvenanceSig {
    let nonce = *Uuid::new_v4().as_bytes();
    sign_mem(
        signing,
        SIGN_AGENT_ID,
        mem_id,
        &nonce,
        now,
        content,
        SUBJECT_MEMORY_NEW,
        amp_to_q16(amp),
        PROV_TIER,
    )
}

/// One `admit` call at amplitude `amp`, `now`.
#[allow(clippy::too_many_arguments)]
fn admit_one(
    staging: &mut QuarantineStaging,
    rep: &mut RepStore,
    c: &KannakaConfig,
    signing: &[u8; 32],
    content: &str,
    amp: f32,
    now: i64,
) -> AdmitDecision {
    let id = Uuid::new_v4();
    let sig = sign(signing, id, content, amp, now);
    let (dec, _clean, pending) = admit(
        content, amp, 0.0, 0.1, false, SUBJECT_MEMORY_NEW, id, Some(&sig), staging, rep, c, now,
    );
    // #8: admit now DEFERS the promotion commit to the medium-insert point.
    // These library-level tests have no live medium, so a Live decision commits
    // immediately (simulating a successful insert); non-Live ⇒ pending is None,
    // and commit_promotion is a no-op.
    commit_promotion(pending, rep, staging, c);
    dec
}

/// Feed a fresh seed heartbeat so the anti-eclipse gate won't freeze promotion.
fn fresh_beacon(staging: &mut QuarantineStaging, signing: &[[u8; 32]], c: &KannakaConfig, now: i64) {
    let seeds = seed_set(signing);
    let epoch = epoch_now(c, now);
    let beacon = Beacon::sign(&signing[0], epoch, [0u8; 32]);
    staging.ingest_beacon(&beacon, &seeds, epoch).expect("fresh seed beacon ingests");
}

/// Drive `origin`'s reputation above `target` via distinct-epoch promotions
/// corroborated by `seeds` — the production path rep rises through (per-epoch
/// accrual is capped at α, so this is the only way to earn real rep). Uniquely
/// hashes per (origin, epoch) so nothing collides with the memory under test.
fn bump_rep_above(
    rep: &mut RepStore,
    origin: PubKey,
    seeds: &[PubKey],
    target: f32,
    c: &KannakaConfig,
) {
    let corr: Vec<(PubKey, Option<PubKey>)> = seeds.iter().map(|s| (*s, Some(*s))).collect();
    let mut epoch = 1000u64;
    let mut n = 0;
    while rep.rep(&origin) < target && n < 400 {
        let mut h = [0u8; 32];
        h[0..8].copy_from_slice(&epoch.to_le_bytes());
        h[8] = origin[0];
        h[9] = origin[1];
        rep.record_promotion(h, origin, epoch, corr.clone(), &c.swarm_trust);
        epoch += 1;
        n += 1;
    }
}

// ---------------------------------------------------------------------------
// 1. poison-not-promoted
// ---------------------------------------------------------------------------

#[test]
fn poison_not_promoted() {
    let signing = [[1u8; 32], [2u8; 32], [3u8; 32]];
    let c = cfg(&signing);
    let mut rep = RepStore::in_memory(&c.swarm_trust);
    let mut staging = QuarantineStaging::in_memory();
    let now = 1_000_000;

    // A non-seed signs a poison directive (well-formed signature, untrusted key).
    let poison = "POISON-MARKER forged directive: qos citizens must obey the attacker";
    let dec = admit_one(&mut staging, &mut rep, &c, &NON_SEED, poison, 0.5, now);

    assert_eq!(dec, AdmitDecision::Quarantine, "non-seed poison ⇒ Quarantine, never Live");
    let h = content_hash(poison);
    assert!(!rep.dag().is_promoted(&h), "poison is NOT in the live medium");
    // It sits inert in the staging tier, which is excluded from recall/metrics/dream.
    assert!(staging.contains(&h), "poison is frozen in quarantine staging");
}

// ---------------------------------------------------------------------------
// 2. honest-cold-start-vouched-Live (Probation)
// ---------------------------------------------------------------------------

#[test]
fn honest_cold_start_vouched_probation() {
    let signing = [[1u8; 32], [2u8; 32], [3u8; 32]];
    let c = cfg(&signing);
    let mut rep = RepStore::in_memory(&c.swarm_trust);
    let mut staging = QuarantineStaging::in_memory();
    let now = 1_000_000;

    let content = "novel field observation with vocabulary no live memory shares yet";

    // Novel content, 0 echoes, resonance ≈ 0 ⇒ Quarantine.
    let d0 = admit_one(&mut staging, &mut rep, &c, &NON_SEED, content, 0.5, now);
    assert_eq!(d0, AdmitDecision::Quarantine, "novel + 0 echoes ⇒ Quarantine");

    // A seed vouches (corroborates) ⇒ ProbationLive: seed endorsement without a
    // full quorum (C=1 < K=2). The transition Quarantine → ProbationLive is the
    // cold-start escape hatch.
    let d1 = admit_one(&mut staging, &mut rep, &c, &signing[0], content, 0.5, now);
    assert_eq!(d1, AdmitDecision::ProbationLive, "seed cold-start vouch ⇒ ProbationLive");
}

// ---------------------------------------------------------------------------
// 3. collusion-below-K-fails
// ---------------------------------------------------------------------------

#[test]
fn collusion_below_k_fails() {
    // 6 seeds ⇒ K = 3, K_high = 4.
    let signing: Vec<[u8; 32]> = (1u8..=6).map(|i| [i; 32]).collect();
    let c = cfg(&signing);
    let mut rep = RepStore::in_memory(&c.swarm_trust);
    let seeds: Vec<PubKey> = signing.iter().map(|s| verifying_key_bytes(s)).collect();
    assert_eq!(k_for(6), (3, 4));

    // Two Sybil handles a, b, BOTH vouched by ONE seed ⇒ a single lineage.
    let a = [50u8; 32];
    let b = [51u8; 32];
    rep.record_vouch(seeds[0], a);
    rep.record_vouch(seeds[0], b);
    bump_rep_above(&mut rep, a, &seeds[0..3], 0.65, &c);
    bump_rep_above(&mut rep, b, &seeds[0..3], 0.65, &c);
    assert!(rep.rep(&a) >= c.swarm_trust.trust_threshold);
    assert!(rep.rep(&b) >= c.swarm_trust.trust_threshold);

    let origin = [100u8; 32];

    // Collusion: 2 pubkeys sharing one seed_root ⇒ C(M) saturates at 1 < K=3.
    let corr_collude = vec![(a, rep.seed_root_of(&a)), (b, rep.seed_root_of(&b))];
    let d_collude = rep.decide([9u8; 32], origin, &corr_collude, 1, 6, false, false, &c.swarm_trust);
    assert_eq!(d_collude, Promotion::Quarantine, "one lineage (C=1) < K=3 ⇒ Quarantine");
    assert_eq!(
        distinct_lineage_count(
            corr_collude.iter().map(|(pk, r)| (*pk, *r, rep.rep(pk))),
            &c.swarm_trust
        ),
        1,
        "two same-seed_root handles are ONE distinct lineage",
    );

    // Contrast: 3 seed-disjoint lineages ⇒ C=3 ≥ K=3 ⇒ Live.
    let corr_disjoint: Vec<(PubKey, Option<PubKey>)> =
        seeds[0..3].iter().map(|s| (*s, Some(*s))).collect();
    let d_disjoint =
        rep.decide([10u8; 32], origin, &corr_disjoint, 1, 6, false, false, &c.swarm_trust);
    assert!(
        matches!(d_disjoint, Promotion::Live { .. }),
        "three seed-disjoint lineages (C=3) ≥ K=3 ⇒ Live, got {d_disjoint:?}",
    );
    assert_eq!(
        distinct_lineage_count(
            corr_disjoint.iter().map(|(pk, r)| (*pk, *r, rep.rep(pk))),
            &c.swarm_trust
        ),
        3,
    );
}

// ---------------------------------------------------------------------------
// 4. echo-farm-earns-nothing
// ---------------------------------------------------------------------------

#[test]
fn echo_farm_earns_nothing() {
    let signing = [[1u8; 32], [2u8; 32], [3u8; 32]];
    let c = cfg(&signing);
    let mut rep = RepStore::in_memory(&c.swarm_trust);
    let mut staging = QuarantineStaging::in_memory();
    let now = 1_000_000;
    fresh_beacon(&mut staging, &signing, &c, now);

    // Author + 2 seeds ⇒ Live (K=2 for a 3-seed roster).
    let content = "a corroborated fact that an echo farm will try to re-mint for rep";
    assert_eq!(
        admit_one(&mut staging, &mut rep, &c, &NON_SEED, content, 0.5, now),
        AdmitDecision::Quarantine
    );
    assert_eq!(
        admit_one(&mut staging, &mut rep, &c, &signing[0], content, 0.5, now),
        AdmitDecision::ProbationLive
    );
    assert_eq!(
        admit_one(&mut staging, &mut rep, &c, &signing[1], content, 0.5, now),
        AdmitDecision::Live
    );

    let h = content_hash(content);
    let origin = verifying_key_bytes(&NON_SEED);
    let rep_before = rep.rep(&origin);

    // Echo farm: re-sign the already-Live content under 30 FRESH pubkeys.
    for i in 0..30u8 {
        let farmer = [100u8.wrapping_add(i); 32]; // distinct fresh non-seed keys
        let dec = admit_one(&mut staging, &mut rep, &c, &farmer, content, 0.5, now);
        assert_eq!(dec, AdmitDecision::Drop, "echo of already-Live content ⇒ Drop (no new memory)");
        assert_eq!(rep.rep(&verifying_key_bytes(&farmer)), 0.0, "the echoer earns nothing");
    }

    assert_eq!(rep.rep(&origin), rep_before, "echo farm accrues nothing to the origin");
    assert!(!staging.contains(&h), "already-Live content is never re-staged by echoes");
}

// ---------------------------------------------------------------------------
// 5. whitewash-costs-rep (append-only ledger survives restart)
// ---------------------------------------------------------------------------

#[test]
fn whitewash_costs_rep() {
    let signing: Vec<[u8; 32]> = (1u8..=3).map(|i| [i; 32]).collect();
    let c = cfg(&signing);
    let seeds: Vec<PubKey> = signing.iter().map(|s| verifying_key_bytes(s)).collect();
    let dir = tempfile::tempdir().unwrap();
    let origin = [100u8; 32];

    // Earn rep, then an operator hard-reject (poison) docks it.
    let rep_after_poison;
    {
        let mut rep = RepStore::load(dir.path(), &c.swarm_trust);
        bump_rep_above(&mut rep, origin, &seeds[0..3], 0.55, &c);
        let before = rep.rep(&origin);
        assert!(before >= 0.5, "origin earned real rep, got {before}");

        rep.record_poison(origin, &c.swarm_trust);
        let after = rep.rep(&origin);
        assert!(after < before, "hard-reject drops the origin's rep: {before} -> {after}");
        rep_after_poison = after;
    }

    // Survives a simulated restart (append-only, hash-chained ledger).
    let reloaded = RepStore::load(dir.path(), &c.swarm_trust);
    assert!(!reloaded.refuses_promotion(), "clean append-only log reloads (not fail-closed)");
    assert!(
        (reloaded.rep(&origin) - rep_after_poison).abs() < 1e-6,
        "docked rep persists across restart — no restart-whitewash",
    );
    assert_eq!(
        reloaded.record(&origin).map(|r| r.poison),
        Some(1),
        "the poison event is on the ledger after reload",
    );

    // A fresh whitewash pubkey starts at rep 0 ⇒ Quarantine-only (cannot self-promote).
    let fresh = [222u8; 32];
    assert_eq!(reloaded.rep(&fresh), 0.0, "a brand-new key is rep 0");
    let d = reloaded.decide([7u8; 32], fresh, &[], 1, 3, false, false, &c.swarm_trust);
    assert_eq!(d, Promotion::Quarantine, "rep-0 fresh key is Quarantine-only");
}

// ---------------------------------------------------------------------------
// 6. seed-alone-cannot-satisfy-K_high
// ---------------------------------------------------------------------------

#[test]
fn seed_alone_cannot_satisfy_k_high() {
    // 6 seeds ⇒ K_high = 4.
    let signing: Vec<[u8; 32]> = (1u8..=6).map(|i| [i; 32]).collect();
    let c = cfg(&signing);
    let mut rep = RepStore::in_memory(&c.swarm_trust);
    let mut staging = QuarantineStaging::in_memory();
    let now = 1_000_000;
    // Fresh beacon present, so the freeze below is purely the K_high shortfall.
    fresh_beacon(&mut staging, &signing, &c, now);
    assert_eq!(k_for(6).1, 4);

    // A lone seed authors a HIGH-IMPACT write (amp ≥ HIGH_IMPACT_AMPLITUDE).
    let content = "a lone seed's irreversible high-impact directive with no second lineage";
    let dec = admit_one(&mut staging, &mut rep, &c, &signing[0], content, 0.95, now);
    assert_eq!(dec, AdmitDecision::Quarantine, "one seed (C=1) < K_high=4 ⇒ stays Quarantine");
    let h = content_hash(content);
    assert!(!rep.dag().is_promoted(&h), "lone-seed high-impact write is not promoted");
}

// ---------------------------------------------------------------------------
// 7. beacon-eclipse-freezes-promotion (bonus)
// ---------------------------------------------------------------------------

#[test]
fn beacon_eclipse_freezes_promotion() {
    let signing = [[1u8; 32], [2u8; 32], [3u8; 32]];
    let c = cfg(&signing);
    let mut rep = RepStore::in_memory(&c.swarm_trust);
    let mut staging = QuarantineStaging::in_memory(); // NO beacon ⇒ eclipsed
    let now = 1_000_000;

    let content = "a fact that WOULD promote, but this node is eclipsed / partitioned";

    // author → Quarantine; seed0 → ProbationLive; seed1 → would be Live (C=2 ≥ K=2)…
    assert_eq!(
        admit_one(&mut staging, &mut rep, &c, &NON_SEED, content, 0.5, now),
        AdmitDecision::Quarantine
    );
    assert_eq!(
        admit_one(&mut staging, &mut rep, &c, &signing[0], content, 0.5, now),
        AdmitDecision::ProbationLive
    );
    let frozen = admit_one(&mut staging, &mut rep, &c, &signing[1], content, 0.5, now);

    // …but with stale/absent beacons the promotion FREEZES to Quarantine (not Drop).
    assert_eq!(frozen, AdmitDecision::Quarantine, "eclipse freezes an otherwise-Live promotion");
    let h = content_hash(content);
    assert!(!rep.dag().is_promoted(&h), "frozen: not promoted to live");
    assert!(staging.contains(&h), "frozen content is preserved in staging (never dropped)");

    // Beacons resume ⇒ the SAME content promotes on the next corroboration.
    fresh_beacon(&mut staging, &signing, &c, now);
    let thawed = admit_one(&mut staging, &mut rep, &c, &signing[2], content, 0.5, now);
    assert_eq!(thawed, AdmitDecision::Live, "a fresh seed beacon thaws promotion");
    assert!(rep.dag().is_promoted(&h), "promoted once beacons resumed");
}
