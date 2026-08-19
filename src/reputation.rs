//! Reputation — pubkey-keyed swarm-trust decision core (inc-1b).
//!
//! This is the **trust-decision CORE** of the Kannaka behavior/corroboration
//! model: the pure formulas (`w`, `g`, `lineage_weight`, [`promotion_weight`]
//! = `C(M)`, [`k_for`], [`accrual`]), the pubkey-keyed reputation store, the
//! append-only corroboration DAG, and a fail-closed append-only log. It adds a
//! module + config fields and **changes no runtime behaviour on its own** — the
//! absorb-path wiring and the `admit()` chokepoint land in a later pass.
//!
//! Design authority: inc-1 corroboration synthesis §2.1 (weight/corroboration),
//! §2.2 (promotion predicate + K formulas + vouch), §2.3 (reputation dynamics),
//! §3.2 (reputation.rs structure), §3.4 (bookkeeping).
//!
//! ## Load-bearing invariants (from the adversarial review — do not violate)
//!
//! - Trust is keyed on the **verified pubkey** `[u8; 32]`, NEVER a string id.
//! - A Sybil cluster sharing one `seed_root` saturates at **one** vote total
//!   ([`lineage_weight`] min-caps at 1). Wildcard / ⊥-lineage handles contribute
//!   **0**.
//! - Echoing already-Live content earns **zero** rep (dedup by `mem_hash =
//!   blake3(normalize(content))`, resolved by the caller).
//! - **No time-decay of the vote** — trust is event-driven. Penalty only via an
//!   operator hard-reject ([`RepStore::record_poison`]).
//! - Persistence is **append-only + fail-closed**: a log that fails to load or
//!   verify yields a store that refuses all promotion (everyone is rep 0).
//!
//! ## `w(rep)` arming model (judgment call)
//!
//! [`w`] is a PURE, monotone weight: `0` below `θ_lo`, clamped-linear to `1.0`
//! at `θ_hi`. Hysteresis needs state, so it is modeled as a per-record
//! [`RepRecord::armed`] flag updated on every rep change ([`RepRecord::update_armed`]):
//! it *arms* when rep reaches `θ_hi` and only *disarms* below `θ_lo`. `w` stays
//! pure (tests pin it); `armed` carries the hysteresis history separately.
//!
//! ## Log line integrity (judgment call)
//!
//! Each log line is **blake3-hash-chained** — `chain = blake3(prev_chain ‖
//! bincode(op))` — rather than ed25519-signed. The node key already lives in
//! [`crate::provenance`]; chaining keeps this core self-contained (no key
//! threading) while still making any in-place edit, truncation, or reordering of
//! the append-only log detectable → fail-closed. Swapping in a per-line signature
//! later is a drop-in change to [`RepStore::append_op`] / the load verifier.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::SwarmTrustConfig;
use crate::provenance;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A verified ed25519 public key. Trust is keyed on this, never a string id.
/// Mirrors the raw pubkey [`crate::provenance::verify_mem`] returns.
pub type PubKey = [u8; 32];

/// A blake3 content hash `blake3(normalize(content))`; the promotion key.
pub type MemHash = [u8; 32];

/// Operator hard-reject penalty `p` (inc-1 §2.3). A seed-quorum poison of a
/// promoted memory subtracts this from the origin's rep.
pub const P_POISON: f32 = 0.5;

/// Cold-start vouch stake `p_vouch` (inc-1 §2.2). A voucher whose probationary
/// memory is later rejected loses this.
pub const P_VOUCH: f32 = 0.5;

/// Float slack for the `C(M) >= K` comparison (both are effectively integral,
/// but `C` accumulates as `f32`).
const EPS: f32 = 1e-4;

/// Per-pubkey reputation record. **No vote-decay field** — trust is
/// event-driven (inc-1 §2.3); the only movements are accrual on distinct-lineage
/// promotion and the operator poison penalty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepRecord {
    /// The verified pubkey this record is keyed on.
    pub pubkey: PubKey,
    /// Reputation in `[0, 1]`. Unknown handles are 0; seeds are treated as 1.0
    /// (see [`RepStore::rep`]) without needing a record.
    pub rep: f32,
    /// The operator seed at the root of this handle's vouch chain, if resolved.
    pub seed_root: Option<PubKey>,
    /// The pubkey that vouched this handle into the web of trust, if any.
    pub vouched_by: Option<PubKey>,
    /// Count of promotions this handle originated (bookkeeping).
    pub promoted: u32,
    /// Count of operator poison hard-rejects against this handle (bookkeeping).
    pub poison: u32,
    /// Creation time, unix ms.
    pub created_at: i64,
    /// Hysteresis arming state (see the module `w(rep)` note). Armed once rep
    /// reaches `θ_hi`; disarmed only below `θ_lo`. Carries history so a handle
    /// hovering in the `[θ_lo, θ_hi)` band keeps its armed status.
    pub armed: bool,
}

impl RepRecord {
    /// A fresh rep-0, disarmed record for `pubkey`.
    pub fn new(pubkey: PubKey, created_at: i64) -> Self {
        Self {
            pubkey,
            rep: 0.0,
            seed_root: None,
            vouched_by: None,
            promoted: 0,
            poison: 0,
            created_at,
            armed: false,
        }
    }

    /// Update the hysteresis [`armed`](RepRecord::armed) flag from the current
    /// rep: arm at/above `θ_hi`, disarm below `θ_lo`, otherwise hold.
    pub fn update_armed(&mut self, cfg: &SwarmTrustConfig) {
        if self.rep >= cfg.theta_hi {
            self.armed = true;
        } else if self.rep < cfg.theta_lo {
            self.armed = false;
        }
    }
}

/// Enrollment status of a pubkey.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedStatus {
    /// An operator-pinned seed (rep 1.0, roots to itself).
    Seed,
    /// A non-seed handle that resolves to a seed via the vouch chain, or that
    /// carries a rep record.
    Enrolled,
    /// No seed lineage and no record — powerless (rep 0).
    Unknown,
}

/// The trust decision for a candidate memory (inc-1 §2.2 / §3.3).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Promotion {
    /// Promote to the live medium; `accrue` rep flows to the origin.
    Live { accrue: f32 },
    /// Seed-vouched cold-start: reduced-amplitude probationary tier, one
    /// lineage, out of dream/metrics. No rep accrues here.
    ProbationLive,
    /// Signed but under-corroborated: inert staging tier. Fail-safe default.
    Quarantine,
    /// Reject outright (reserved for the admit layer's invalid-signature drop;
    /// the pure decision here never emits it, but it is part of the shared
    /// vocabulary so the chokepoint has one enum).
    Drop,
}

// ---------------------------------------------------------------------------
// CorroborationDag
// ---------------------------------------------------------------------------

/// One promotion event: which pubkeys corroborated a `mem_hash` at an epoch.
/// `seen` dedups by pubkey so an echo (same pk twice) never double-counts.
#[derive(Debug, Default, Clone)]
struct PromotionEvent {
    epoch: u64,
    corroborators: Vec<(PubKey, Option<PubKey>)>,
    seen: HashSet<PubKey>,
}

/// Append-only corroboration DAG (inc-1 §3.2): vouch edges `vouchee -> voucher`
/// plus promotion events keyed by `mem_hash`. [`seed_root`](CorroborationDag::seed_root)
/// walks the vouch chain to a seed — cached and cycle-safe.
#[derive(Debug, Default)]
pub struct CorroborationDag {
    /// `vouchee -> voucher` — the reverse edge used to walk toward a seed.
    vouched_by: HashMap<PubKey, PubKey>,
    /// `mem_hash -> promotion event`.
    promotions: HashMap<MemHash, PromotionEvent>,
    /// Memoized `seed_root` results. Cleared on every new vouch edge (append-only
    /// growth can only turn a ⊥ into a `Some`, so a stale cache is never trusted).
    root_cache: RefCell<HashMap<PubKey, Option<PubKey>>>,
}

impl CorroborationDag {
    /// Record an enrollment vouch edge `voucher -> vouchee`. Invalidates the
    /// `seed_root` cache.
    pub fn record_vouch(&mut self, voucher: PubKey, vouchee: PubKey) {
        self.vouched_by.insert(vouchee, voucher);
        self.root_cache.borrow_mut().clear();
    }

    /// Record (or extend) a promotion event for `mem_hash`, deduping
    /// corroborators by pubkey. A repeat `(pk, mem_hash)` is a no-op.
    pub fn record_promotion(
        &mut self,
        mem_hash: MemHash,
        epoch: u64,
        corroborators: &[(PubKey, Option<PubKey>)],
    ) {
        let ev = self
            .promotions
            .entry(mem_hash)
            .or_insert_with(|| PromotionEvent { epoch, ..Default::default() });
        for (pk, root) in corroborators {
            if ev.seen.insert(*pk) {
                ev.corroborators.push((*pk, *root));
            }
        }
    }

    /// Whether `mem_hash` has already been promoted (used for echo dedup).
    pub fn is_promoted(&self, mem_hash: &MemHash) -> bool {
        self.promotions.contains_key(mem_hash)
    }

    /// Distinct corroborating pubkeys recorded for `mem_hash`.
    pub fn corroborator_count(&self, mem_hash: &MemHash) -> usize {
        self.promotions.get(mem_hash).map(|e| e.seen.len()).unwrap_or(0)
    }

    /// Walk the vouch chain from `pk` to the operator seed at its root. A seed
    /// roots to itself; a handle with no chain to a seed returns `None` (⊥
    /// lineage). Cycle-safe (visited set) and memoized.
    pub fn seed_root(&self, pk: &PubKey, seeds: &HashSet<PubKey>) -> Option<PubKey> {
        if let Some(cached) = self.root_cache.borrow().get(pk) {
            return *cached;
        }
        let mut visited: HashSet<PubKey> = HashSet::new();
        let mut cur = *pk;
        let result = loop {
            if seeds.contains(&cur) {
                break Some(cur);
            }
            if !visited.insert(cur) {
                break None; // cycle — terminate
            }
            match self.vouched_by.get(&cur) {
                Some(v) => cur = *v,
                None => break None, // ⊥ lineage
            }
        };
        self.root_cache.borrow_mut().insert(*pk, result);
        result
    }
}

// ---------------------------------------------------------------------------
// Pure formulas (inc-1 §2.1 / §2.2 / §2.3)
// ---------------------------------------------------------------------------

/// Continuous corroboration weight with hysteresis thresholds (inc-1 §2.1):
/// `0` for `rep < θ_lo`, clamped-linear ramp to `1.0` at `θ_hi`, saturating
/// above. Pure and monotone in `rep`. The *arming* half of the hysteresis lives
/// in [`RepRecord::armed`] (see the module note) — this function is stateless.
pub fn w(rep: f32, cfg: &SwarmTrustConfig) -> f32 {
    let lo = cfg.theta_lo;
    let hi = cfg.theta_hi;
    if rep < lo {
        return 0.0;
    }
    if rep >= hi || hi <= lo {
        return 1.0;
    }
    ((rep - lo) / (hi - lo)).clamp(0.0, 1.0)
}

/// Sub-linear, bounded diversity gain `g(D) = 1 - 1/(1+D)` (inc-1 §2.3). `D` is
/// the count of distinct seed-root lineages. `g(0)=0`, strictly increasing,
/// strictly `< 1`.
pub fn g(d: u32) -> f32 {
    1.0 - 1.0 / (1.0 + d as f32)
}

/// Saturating per-lineage weight: `min(1, Σ w(rep(pk)))` over the pubkeys of one
/// lineage that clear θ (inc-1 §2.2). A Sybil cluster in one lineage caps at 1.
pub fn lineage_weight(reps: &[f32], cfg: &SwarmTrustConfig) -> f32 {
    let sum: f32 = reps
        .iter()
        .copied()
        .filter(|&r| r >= cfg.trust_threshold)
        .map(|r| w(r, cfg))
        .sum();
    sum.min(1.0)
}

/// Promotion weight `C(M) = Σ over DISTINCT seed_roots of lineage_weight`
/// (inc-1 §2.2). Corroborators are `(pubkey, seed_root, rep)` triples. Rules:
/// dedup by pubkey (same pk counts once), drop rep `< θ`, drop `⊥`-lineage
/// (`seed_root == None`) handles, then group by seed_root and saturate each
/// lineage at 1. Pure so tests pin it.
pub fn promotion_weight<I>(corrs: I, cfg: &SwarmTrustConfig) -> f32
where
    I: IntoIterator<Item = (PubKey, Option<PubKey>, f32)>,
{
    let mut seen: HashSet<PubKey> = HashSet::new();
    let mut by_root: HashMap<PubKey, Vec<f32>> = HashMap::new();
    for (pk, root, rep) in corrs {
        if !seen.insert(pk) {
            continue; // same pk counts once
        }
        let Some(root) = root else {
            continue; // ⊥ lineage contributes 0
        };
        if rep < cfg.trust_threshold {
            continue;
        }
        by_root.entry(root).or_default().push(rep);
    }
    by_root.values().map(|reps| lineage_weight(reps, cfg)).sum()
}

/// Count of distinct seed-root lineages among valid corroborators — the `D`
/// that feeds [`accrual`] and `g(D)`. Same dedup/θ/⊥ rules as [`promotion_weight`].
pub fn distinct_lineage_count<I>(corrs: I, cfg: &SwarmTrustConfig) -> u32
where
    I: IntoIterator<Item = (PubKey, Option<PubKey>, f32)>,
{
    let mut seen: HashSet<PubKey> = HashSet::new();
    let mut roots: HashSet<PubKey> = HashSet::new();
    for (pk, root, rep) in corrs {
        if !seen.insert(pk) {
            continue;
        }
        if rep < cfg.trust_threshold {
            continue;
        }
        if let Some(root) = root {
            roots.insert(root);
        }
    }
    roots.len() as u32
}

/// Quorum sizes from the live seed count (inc-1 §2.2):
/// `K = clamp(ceil(seeds/2), 2, 4)` for normal writes, `K_high = ceil(2·seeds/3)`
/// for irreversible / high-impact writes. Integer-exact ceilings.
pub fn k_for(live_seed_count: u32) -> (u32, u32) {
    let k = ((live_seed_count + 1) / 2).clamp(2, 4); // ceil(n/2)
    let k_high = (2 * live_seed_count + 2) / 3; // ceil(2n/3)
    (k, k_high)
}

/// Per-promotion rep accrual `α · g(D)` (inc-1 §2.3). The per-epoch cap is
/// enforced by [`RepStore::record_promotion`], not here.
pub fn accrual(d: u32, cfg: &SwarmTrustConfig) -> f32 {
    cfg.accrual_alpha * g(d)
}

// ---------------------------------------------------------------------------
// Append-only log ops
// ---------------------------------------------------------------------------

/// A single append-only reputation-log operation. Deltas (`accrue`, `penalty`)
/// are stored so a replay reproduces rep exactly and independently of any later
/// config drift.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum LogOp {
    /// Enrollment vouch edge `voucher -> vouchee`.
    Vouch { voucher: PubKey, vouchee: PubKey },
    /// A promotion event (origin accrued `accrue`, already epoch-capped/echo-zeroed).
    Promotion {
        mem_hash: MemHash,
        origin: PubKey,
        epoch: u64,
        accrue: f32,
        corroborators: Vec<(PubKey, Option<PubKey>)>,
    },
    /// An operator hard-reject subtracting `penalty` from `pk`.
    Poison { pk: PubKey, penalty: f32 },
}

/// A hash-chained log line: `chain = blake3(prev_chain ‖ bincode(op))`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LogLine {
    op: LogOp,
    chain: [u8; 32],
}

/// Compact gzip snapshot payload (bincode inner, so `[u8;32]` map keys are fine).
#[derive(Debug, Serialize, Deserialize)]
struct Snapshot {
    log_head: [u8; 32],
    seeds: Vec<PubKey>,
    reps: Vec<RepRecord>,
    /// `(vouchee, voucher)` edges.
    vouch_edges: Vec<(PubKey, PubKey)>,
    /// `(mem_hash, epoch, corroborators)`.
    promotions: Vec<(MemHash, u64, Vec<(PubKey, Option<PubKey>)>)>,
    /// #10: per-pubkey `(epoch, accrued)` anti-burst cap state, so a compact
    /// that truncates the log doesn't reopen a fresh full α mid-epoch. `default`
    /// so a pre-#10 snapshot (no field) still deserializes (empty ⇒ rebuilt from
    /// the replayed log where possible).
    #[serde(default)]
    epoch_accrual: Vec<(PubKey, u64, f32)>,
}

// ---------------------------------------------------------------------------
// RepStore
// ---------------------------------------------------------------------------

/// Pubkey-keyed reputation store + corroboration DAG + fail-closed persistence
/// (inc-1 §3.2). Construct with [`in_memory`](RepStore::in_memory) (tests) or
/// [`load`](RepStore::load) (from `<data_dir>/reputation.{log,snapshot.gz}`).
#[derive(Debug)]
pub struct RepStore {
    reps: HashMap<PubKey, RepRecord>,
    seeds: HashSet<PubKey>,
    dag: CorroborationDag,
    /// Fail-closed flag: a load that failed to parse/verify sets this and every
    /// [`decide`](RepStore::decide) returns [`Promotion::Quarantine`].
    poisoned: bool,
    /// Running blake3 chain head over the append-only log.
    log_head: [u8; 32],
    /// `Some` when persistence is enabled; `None` for in-memory test stores.
    data_dir: Option<PathBuf>,
    /// Transient per-epoch accrual bookkeeping `pk -> (epoch, accrued)` — enforces
    /// the anti-burst cap. Not persisted (epochs advance; the cap is per-epoch).
    epoch_accrual: HashMap<PubKey, (u64, f32)>,
}

impl RepStore {
    /// An in-memory store seeded from `cfg.seed_pubkeys`, no persistence.
    pub fn in_memory(cfg: &SwarmTrustConfig) -> Self {
        Self {
            reps: HashMap::new(),
            seeds: seeds_from_cfg(cfg),
            dag: CorroborationDag::default(),
            poisoned: false,
            log_head: [0u8; 32],
            data_dir: None,
            epoch_accrual: HashMap::new(),
        }
    }

    /// A persistence-backed store rooted at `data_dir` (does not read existing
    /// state — use [`load`](RepStore::load) for that).
    pub fn new_at(cfg: &SwarmTrustConfig, data_dir: PathBuf) -> Self {
        let mut s = Self::in_memory(cfg);
        s.data_dir = Some(data_dir);
        s
    }

    /// A fail-closed store: poisoned, refuses all promotion.
    fn poisoned_store(cfg: &SwarmTrustConfig, data_dir: Option<PathBuf>) -> Self {
        let mut s = Self::in_memory(cfg);
        s.poisoned = true;
        s.data_dir = data_dir;
        s
    }

    /// True when the store is fail-closed and refuses every promotion (a corrupt
    /// or unverifiable log). Callers must treat everyone as rep 0.
    pub fn refuses_promotion(&self) -> bool {
        self.poisoned
    }

    /// Number of live (operator-pinned) seeds — the input to [`k_for`].
    pub fn live_seed_count(&self) -> u32 {
        self.seeds.len() as u32
    }

    /// Reputation of a pubkey: seeds are `1.0`, a record's rep otherwise, `0.0`
    /// for an unknown handle.
    pub fn rep(&self, pk: &PubKey) -> f32 {
        if self.seeds.contains(pk) {
            return 1.0;
        }
        self.reps.get(pk).map(|r| r.rep).unwrap_or(0.0)
    }

    /// Enrollment status of a pubkey.
    pub fn seed_status(&self, pk: &PubKey) -> SeedStatus {
        if self.seeds.contains(pk) {
            SeedStatus::Seed
        } else if self.seed_root_of(pk).is_some() || self.reps.contains_key(pk) {
            SeedStatus::Enrolled
        } else {
            SeedStatus::Unknown
        }
    }

    /// Authoritative seed_root for a pubkey (seed ⇒ itself, else DAG walk).
    pub fn seed_root_of(&self, pk: &PubKey) -> Option<PubKey> {
        self.dag.seed_root(pk, &self.seeds)
    }

    /// Read access to the corroboration DAG.
    pub fn dag(&self) -> &CorroborationDag {
        &self.dag
    }

    /// Read-only: the reputation record for `pk`, if one exists. Exposes the
    /// bookkeeping counters (`promoted`, `poison`, `armed`, `seed_root`) to the
    /// operator inspection CLI (`kannaka reputation show`). Pure — no state
    /// change and no effect on the (dormant) promotion gate.
    pub fn record(&self, pk: &PubKey) -> Option<&RepRecord> {
        self.reps.get(pk)
    }

    /// Read-only: iterate the operator-pinned seed pubkeys (CLI listing). A
    /// seed need not carry a [`RepRecord`], so enumerate this alongside
    /// [`records`](RepStore::records) to get every *known* pubkey.
    pub fn seeds(&self) -> impl Iterator<Item = &PubKey> {
        self.seeds.iter()
    }

    /// Read-only: iterate every reputation record (CLI listing). Does not
    /// include seeds that have no record — union with [`seeds`](RepStore::seeds).
    pub fn records(&self) -> impl Iterator<Item = &RepRecord> {
        self.reps.values()
    }

    // --- decision -------------------------------------------------------

    /// Decide the fate of a candidate memory (inc-1 §2.2). Pure w.r.t. store
    /// state — mutates nothing. Corroborators are `(pubkey, recorded_seed_root)`;
    /// the seed_root and rep are re-resolved authoritatively from this store, so
    /// caller-supplied lineage claims cannot inflate `C(M)`.
    ///
    /// - Fail-closed (poisoned) ⇒ [`Quarantine`](Promotion::Quarantine).
    /// - Already-Live `mem_hash` (echo) ⇒ `Live { accrue: 0.0 }`.
    /// - Seed origin, normal write ⇒ `Live`. Seed origin, `high_impact` ⇒
    ///   requires `K_high` DISTINCT lineages (a lone seed is 1 lineage ⇒
    ///   `Quarantine`).
    /// - Non-seed origin ⇒ `Live` iff `C(M) ≥ K_eff`. `resonance_ok` with ≥1
    ///   valid echo lowers `K` by 1 (floor 2); `high_impact` forces `K_high`
    ///   (never reduced). A seed among the corroborators without full quorum ⇒
    ///   `ProbationLive`; otherwise `Quarantine`.
    #[allow(clippy::too_many_arguments)]
    pub fn decide(
        &self,
        mem_hash: MemHash,
        origin_pk: PubKey,
        corroborators: &[(PubKey, Option<PubKey>)],
        epoch: u64,
        live_seed_count: u32,
        high_impact: bool,
        resonance_ok: bool,
        cfg: &SwarmTrustConfig,
    ) -> Promotion {
        let _ = epoch; // bound into corroboration envelopes for the admit layer;
                       // the pure predicate does not use it.
        if self.poisoned {
            return Promotion::Quarantine;
        }
        if self.dag.is_promoted(&mem_hash) {
            return Promotion::Live { accrue: 0.0 }; // echo — already Live, earns nothing
        }

        let (k, k_high) = k_for(live_seed_count);

        // Authoritative triples: recompute seed_root + rep from this store.
        let triples: Vec<(PubKey, Option<PubKey>, f32)> = corroborators
            .iter()
            .map(|(pk, _)| (*pk, self.seed_root_of(pk), self.rep(pk)))
            .collect();

        if self.seeds.contains(&origin_pk) {
            if high_impact {
                let mut roots: HashSet<PubKey> = triples
                    .iter()
                    .filter(|(_, _, rep)| *rep >= cfg.trust_threshold)
                    .filter_map(|(_, root, _)| *root)
                    .collect();
                roots.insert(origin_pk); // the origin seed is its own lineage
                // #9: floor K_high at 2 (mirrors the non-seed branch's
                // `k_high.max(2)`) so a LONE seed — a single lineage < 2 — can
                // never self-promote a high-impact write at n=1 (`k_for(1)`
                // yields raw K_high=0, which would otherwise admit `1 >= 0`).
                if roots.len() as u32 >= k_high.max(2) {
                    return Promotion::Live { accrue: 0.0 };
                }
                return Promotion::Quarantine;
            }
            return Promotion::Live { accrue: 0.0 };
        }

        // Non-seed origin: corroboration quorum.
        let c = promotion_weight(triples.iter().copied(), cfg);
        let d = distinct_lineage_count(triples.iter().copied(), cfg);

        let k_eff = if resonance_ok && d >= 1 {
            k.saturating_sub(1).max(2) // field resonance lowers K by ≤1, floor 2
        } else {
            k
        };
        let required = if high_impact { k_high.max(2) } else { k_eff };

        if c + EPS >= required as f32 {
            Promotion::Live { accrue: accrual(d, cfg) }
        } else if triples.iter().any(|(pk, _, _)| self.seeds.contains(pk)) {
            Promotion::ProbationLive // seed endorsement without full quorum
        } else {
            Promotion::Quarantine
        }
    }

    // --- bookkeeping (mutate + persist) ---------------------------------

    /// Record an enrollment vouch edge `voucher -> vouchee` and resolve the
    /// vouchee's seed_root. Appends to the log.
    pub fn record_vouch(&mut self, voucher_pk: PubKey, vouchee_pk: PubKey) {
        self.apply_vouch(voucher_pk, vouchee_pk);
        let _ = self.append_op(&LogOp::Vouch { voucher: voucher_pk, vouchee: vouchee_pk });
    }

    /// Record a promotion of `mem_hash` originated by `origin_pk`, accruing
    /// `α·g(D)` (per-epoch capped) to the origin — UNLESS `mem_hash` is already
    /// Live, in which case this is an echo: the corroboration edge is recorded
    /// but **no rep accrues and no memory is created**. Appends to the log.
    pub fn record_promotion(
        &mut self,
        mem_hash: MemHash,
        origin_pk: PubKey,
        epoch: u64,
        corroborators: Vec<(PubKey, Option<PubKey>)>,
        cfg: &SwarmTrustConfig,
    ) {
        let accrue = if self.dag.is_promoted(&mem_hash) {
            0.0 // echo earns nothing
        } else {
            let d = distinct_lineage_count(
                corroborators.iter().map(|(pk, root)| (*pk, *root, self.rep(pk))),
                cfg,
            );
            let raw = accrual(d, cfg);
            self.epoch_capped(origin_pk, epoch, raw, cfg)
        };
        self.apply_promotion(mem_hash, origin_pk, epoch, &corroborators, accrue, cfg);
        let _ = self.append_op(&LogOp::Promotion {
            mem_hash,
            origin: origin_pk,
            epoch,
            accrue,
            corroborators,
        });
    }

    /// Operator hard-reject: subtract `p` from `pk`'s rep and count the poison.
    /// The only rep-lowering lever (no peer-triggered penalty exists). Appends
    /// to the log.
    pub fn record_poison(&mut self, pk: PubKey, cfg: &SwarmTrustConfig) {
        self.apply_poison(pk, P_POISON, cfg);
        let _ = self.append_op(&LogOp::Poison { pk, penalty: P_POISON });
    }

    // --- state mutation (no persistence; also used by replay) -----------

    fn apply_vouch(&mut self, voucher: PubKey, vouchee: PubKey) {
        self.dag.record_vouch(voucher, vouchee);
        let sr = self.dag.seed_root(&vouchee, &self.seeds);
        let rec = self
            .reps
            .entry(vouchee)
            .or_insert_with(|| RepRecord::new(vouchee, now_ms()));
        rec.vouched_by = Some(voucher);
        rec.seed_root = sr;
    }

    fn apply_promotion(
        &mut self,
        mem_hash: MemHash,
        origin_pk: PubKey,
        epoch: u64,
        corroborators: &[(PubKey, Option<PubKey>)],
        accrue: f32,
        cfg: &SwarmTrustConfig,
    ) {
        let is_echo = self.dag.is_promoted(&mem_hash);
        self.dag.record_promotion(mem_hash, epoch, corroborators);
        if is_echo {
            return; // no rep change on echo
        }
        let sr = self.seed_root_of(&origin_pk);
        let rec = self
            .reps
            .entry(origin_pk)
            .or_insert_with(|| RepRecord::new(origin_pk, now_ms()));
        rec.rep = (rec.rep + accrue).clamp(0.0, 1.0);
        rec.promoted += 1;
        if rec.seed_root.is_none() {
            rec.seed_root = sr;
        }
        rec.update_armed(cfg);
    }

    fn apply_poison(&mut self, pk: PubKey, penalty: f32, cfg: &SwarmTrustConfig) {
        let rec = self.reps.entry(pk).or_insert_with(|| RepRecord::new(pk, now_ms()));
        rec.rep = (rec.rep - penalty).clamp(0.0, 1.0);
        rec.poison += 1;
        rec.update_armed(cfg);
    }

    /// Clamp `raw` accrual to what remains of this epoch's cap (`accrual_alpha`)
    /// for `pk`, so a burst of promotions in one epoch cannot spike rep.
    fn epoch_capped(&mut self, pk: PubKey, epoch: u64, raw: f32, cfg: &SwarmTrustConfig) -> f32 {
        let cap = cfg.accrual_alpha;
        let entry = self.epoch_accrual.entry(pk).or_insert((epoch, 0.0));
        if entry.0 != epoch {
            *entry = (epoch, 0.0);
        }
        let allowed = (cap - entry.1).max(0.0);
        let actual = raw.min(allowed);
        entry.1 += actual;
        actual
    }

    fn apply_op(&mut self, op: &LogOp, cfg: &SwarmTrustConfig) {
        match op {
            LogOp::Vouch { voucher, vouchee } => self.apply_vouch(*voucher, *vouchee),
            LogOp::Promotion { mem_hash, origin, epoch, accrue, corroborators } => {
                self.apply_promotion(*mem_hash, *origin, *epoch, corroborators, *accrue, cfg)
            }
            LogOp::Poison { pk, penalty } => self.apply_poison(*pk, *penalty, cfg),
        }
    }

    // --- persistence ----------------------------------------------------

    /// Append a hash-chained line to `<data_dir>/reputation.log`. A no-op for
    /// in-memory stores. Advances [`log_head`](RepStore::log_head) on success.
    fn append_op(&mut self, op: &LogOp) -> Result<(), String> {
        let Some(dir) = self.data_dir.clone() else {
            return Ok(()); // in-memory: no persistence
        };
        let op_bytes =
            bincode::serialize(op).map_err(|e| format!("reputation: encode op: {e}"))?;
        let mut h = blake3::Hasher::new();
        h.update(&self.log_head);
        h.update(&op_bytes);
        let chain = *h.finalize().as_bytes();
        let line = LogLine { op: op.clone(), chain };
        let json = serde_json::to_string(&line)
            .map_err(|e| format!("reputation: encode line: {e}"))?;

        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("reputation: mkdir {}: {e}", dir.display()))?;
        let path = dir.join("reputation.log");
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| format!("reputation: open {}: {e}", path.display()))?;
        f.write_all(json.as_bytes())
            .and_then(|_| f.write_all(b"\n"))
            .map_err(|e| format!("reputation: append: {e}"))?;
        self.log_head = chain;
        Ok(())
    }

    /// Load a store from `<data_dir>/reputation.snapshot.gz` (if present) + the
    /// hash-chained `reputation.log`. **Fail-closed**: any snapshot decode error,
    /// log read error, unparsable line, or broken hash chain yields a poisoned
    /// store that refuses all promotion (inc-1 §3.2). A missing file is a clean
    /// empty store, not a failure. Seeds are always taken from `cfg`.
    pub fn load(data_dir: &Path, cfg: &SwarmTrustConfig) -> RepStore {
        let mut store = RepStore::new_at(cfg, data_dir.to_path_buf());

        // 1. Optional compact snapshot. #4: the snapshot MUST carry a valid
        // signature by THIS node's own key. An unsigned, foreign, or tampered
        // snapshot is fail-closed (poisoned) rather than unioned — it must never
        // bypass the log's hash chain to inject seeds/reps/promotions.
        let snap_path = data_dir.join("reputation.snapshot.gz");
        if snap_path.exists() {
            match read_verified_snapshot(data_dir) {
                Some(snap) => store.apply_snapshot(snap),
                None => return RepStore::poisoned_store(cfg, Some(data_dir.to_path_buf())),
            }
        }

        // 2. Replay the hash-chained log on top.
        let log_path = data_dir.join("reputation.log");
        if log_path.exists() {
            let text = match std::fs::read_to_string(&log_path) {
                Ok(t) => t,
                Err(_) => return RepStore::poisoned_store(cfg, Some(data_dir.to_path_buf())),
            };
            let mut head = store.log_head;
            for line in text.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let ll: LogLine = match serde_json::from_str(line) {
                    Ok(l) => l,
                    Err(_) => return RepStore::poisoned_store(cfg, Some(data_dir.to_path_buf())),
                };
                let op_bytes = match bincode::serialize(&ll.op) {
                    Ok(b) => b,
                    Err(_) => return RepStore::poisoned_store(cfg, Some(data_dir.to_path_buf())),
                };
                let mut h = blake3::Hasher::new();
                h.update(&head);
                h.update(&op_bytes);
                let expect = *h.finalize().as_bytes();
                if expect != ll.chain {
                    return RepStore::poisoned_store(cfg, Some(data_dir.to_path_buf()));
                }
                head = expect;
                store.apply_op(&ll.op, cfg);
                // #10: rebuild the per-epoch accrual cap from each promotion so
                // a mid-epoch restart can't re-grant a fresh full α.
                if let LogOp::Promotion { origin, epoch, accrue, .. } = &ll.op {
                    store.reconstruct_epoch_accrual(*origin, *epoch, *accrue);
                }
            }
            store.log_head = head;
        }

        store
    }

    fn apply_snapshot(&mut self, snap: Snapshot) {
        self.log_head = snap.log_head;
        // Seeds already come from cfg; union in any snapshot-recorded seeds too.
        for s in snap.seeds {
            self.seeds.insert(s);
        }
        for (vouchee, voucher) in snap.vouch_edges {
            self.dag.record_vouch(voucher, vouchee);
        }
        for (mem_hash, epoch, corrs) in snap.promotions {
            self.dag.record_promotion(mem_hash, epoch, &corrs);
        }
        for rec in snap.reps {
            self.reps.insert(rec.pubkey, rec);
        }
        // #10: restore the per-epoch accrual cap captured at compact time.
        for (pk, epoch, accrued) in snap.epoch_accrual {
            self.epoch_accrual.insert(pk, (epoch, accrued));
        }
    }

    /// Rebuild the transient per-epoch accrual cap from a replayed `Promotion`
    /// op (finding #10). The log stores the ALREADY-capped `accrue` delta and
    /// its epoch; replaying in append order leaves each pk's entry at its most
    /// recent epoch's running total, so a mid-epoch restart cannot re-grant a
    /// fresh full α (the anti-burst cap survives the restart).
    fn reconstruct_epoch_accrual(&mut self, pk: PubKey, epoch: u64, accrue: f32) {
        let entry = self.epoch_accrual.entry(pk).or_insert((epoch, 0.0));
        if entry.0 != epoch {
            *entry = (epoch, 0.0);
        }
        entry.1 += accrue;
    }

    fn snapshot(&self) -> Snapshot {
        let mut vouch_edges = Vec::new();
        for (vouchee, voucher) in &self.dag.vouched_by {
            vouch_edges.push((*vouchee, *voucher));
        }
        let mut promotions = Vec::new();
        for (mem_hash, ev) in &self.dag.promotions {
            promotions.push((*mem_hash, ev.epoch, ev.corroborators.clone()));
        }
        Snapshot {
            log_head: self.log_head,
            seeds: self.seeds.iter().copied().collect(),
            reps: self.reps.values().cloned().collect(),
            vouch_edges,
            promotions,
            // #10: persist the per-epoch accrual cap so a compact (which
            // truncates the log) still preserves the anti-burst state.
            epoch_accrual: self
                .epoch_accrual
                .iter()
                .map(|(pk, (e, a))| (*pk, *e, *a))
                .collect(),
        }
    }

    /// Compact the append-only log into a gzip snapshot at
    /// `<data_dir>/reputation.snapshot.gz` and truncate the log, preserving the
    /// chain head so subsequent appends stay verifiable. No-op for in-memory
    /// stores. Mirrors the crate's atomic temp-write + rename discipline.
    pub fn compact(&self) -> Result<(), String> {
        let Some(dir) = self.data_dir.clone() else {
            return Ok(());
        };
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("reputation: mkdir {}: {e}", dir.display()))?;
        let bytes = bincode::serialize(&self.snapshot())
            .map_err(|e| format!("reputation: encode snapshot: {e}"))?;
        let mut enc =
            flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(&bytes)
            .map_err(|e| format!("reputation: gzip: {e}"))?;
        let gz = enc.finish().map_err(|e| format!("reputation: gzip finish: {e}"))?;
        // #4: sign the snapshot with THIS node's ed25519 key and store the
        // signature + signer pubkey in a sidecar, so a planted/foreign or
        // tampered snapshot fails verification on load (fail-closed).
        let seed = provenance::node_signing_key(&dir)?;
        let signer = provenance::verifying_key_bytes(&seed);
        let sig = provenance::sign_snapshot(&seed, &gz);
        let mut sidecar = Vec::with_capacity(96);
        sidecar.extend_from_slice(&signer); // signer pubkey[32] (diagnostic)
        sidecar.extend_from_slice(&sig); // ed25519 sig[64] over the gz bytes
        atomic_write(&dir.join("reputation.snapshot.gz"), &gz)?;
        atomic_write(&dir.join("reputation.snapshot.sig"), &sidecar)?;
        // Truncate the log — the snapshot captures state up to log_head; new
        // appends chain forward from the same head.
        atomic_write(&dir.join("reputation.log"), b"")?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Seed pubkey set from `cfg.seed_pubkeys` (base64). Invalid entries are
/// dropped — an empty set means the corroboration gate is dormant (no root yet).
fn seeds_from_cfg(cfg: &SwarmTrustConfig) -> HashSet<PubKey> {
    cfg.seed_pubkeys
        .iter()
        .filter_map(|s| b64_decode_32(s.trim()))
        .collect()
}

/// Current wall-clock in unix milliseconds.
fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

// #777: atomic write now lives in crate::fs_util — one implementation.
use crate::fs_util::atomic_write_bytes as atomic_write;
/// Verify (finding #4) + read + gunzip + bincode-decode a snapshot. `None` on
/// ANY failure (fail-closed): missing/short signature sidecar, no node key on
/// disk, a signature that does not verify against THIS node's own key, or a
/// decode error. The `.sig` sidecar is `signer_pubkey[32] ‖ sig[64]`; the
/// signature is checked over the on-disk gz bytes using the node's OWN
/// verifying key, so a foreign/planted snapshot (signed by a different key) or
/// a tampered one is refused.
fn read_verified_snapshot(data_dir: &Path) -> Option<Snapshot> {
    let gz = std::fs::read(data_dir.join("reputation.snapshot.gz")).ok()?;
    let sidecar = std::fs::read(data_dir.join("reputation.snapshot.sig")).ok()?;
    if sidecar.len() != 96 {
        return None; // unsigned / malformed sidecar ⇒ fail-closed
    }
    let mut sig = [0u8; 64];
    sig.copy_from_slice(&sidecar[32..96]);
    let vk = provenance::node_verifying_key(data_dir)?; // no key ⇒ can't verify
    if !provenance::verify_snapshot(&vk, &gz, &sig) {
        return None; // foreign/tampered ⇒ fail-closed
    }
    let mut dec = flate2::read::GzDecoder::new(&gz[..]);
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut dec, &mut bytes).ok()?;
    bincode::deserialize(&bytes).ok()
}

// --- base64 (standard alphabet) — decode-only, mirrors provenance's codec ---

fn b64_decode_32(s: &str) -> Option<PubKey> {
    let mut out: Vec<u8> = Vec::with_capacity(32);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &b in s.as_bytes() {
        if b == b'=' || b == b'\n' || b == b'\r' || b == b' ' {
            continue;
        }
        let val: u32 = match b {
            b'A'..=b'Z' => u32::from(b - b'A'),
            b'a'..=b'z' => u32::from(b - b'a') + 26,
            b'0'..=b'9' => u32::from(b - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    if out.len() != 32 {
        return None;
    }
    let mut a = [0u8; 32];
    a.copy_from_slice(&out);
    Some(a)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SwarmTrustConfig {
        SwarmTrustConfig::default() // theta_lo=0.4, theta_hi=0.7, theta=0.6, alpha=0.05
    }

    fn pk(n: u8) -> PubKey {
        [n; 32]
    }
    fn hash(n: u8) -> MemHash {
        [n; 32]
    }
    fn rr(p: PubKey, rep: f32) -> RepRecord {
        let mut r = RepRecord::new(p, 0);
        r.rep = rep;
        r
    }

    // (a) w(rep) hysteresis: 0 at 0.3, rises 0.4→0.7, 1.0 at ≥0.7; monotone.
    #[test]
    fn w_hysteresis_and_monotone() {
        let c = cfg();
        assert_eq!(w(0.3, &c), 0.0);
        assert_eq!(w(0.4, &c), 0.0); // ramp starts at θ_lo
        assert!((w(0.55, &c) - 0.5).abs() < 1e-4, "midpoint ramp");
        assert_eq!(w(0.7, &c), 1.0);
        assert_eq!(w(0.9, &c), 1.0);
        let mut prev = -1.0;
        for i in 0..=100 {
            let r = i as f32 / 100.0;
            let v = w(r, &c);
            assert!(v >= prev - 1e-6, "w must be monotone non-decreasing");
            prev = v;
        }
        // Arming hysteresis is carried on the record, not in w().
        let mut rec = rr(pk(1), 0.8);
        rec.update_armed(&c);
        assert!(rec.armed, "rep ≥ θ_hi arms");
        rec.rep = 0.5;
        rec.update_armed(&c); // in the band — holds armed
        assert!(rec.armed, "stays armed inside [θ_lo, θ_hi)");
        rec.rep = 0.3;
        rec.update_armed(&c); // below θ_lo — disarms
        assert!(!rec.armed, "disarms below θ_lo");
    }

    // (b) g(D) sub-linear + bounded (< 1).
    #[test]
    fn g_sublinear_bounded() {
        assert_eq!(g(0), 0.0);
        assert!((g(1) - 0.5).abs() < 1e-4);
        assert!((g(2) - 2.0 / 3.0).abs() < 1e-4);
        assert!(g(3) < 1.0 && g(1000) < 1.0, "bounded below 1");
        assert!(g(2) > g(1), "strictly increasing");
        // sub-linear: marginal gain shrinks.
        assert!(g(2) - g(1) < g(1) - g(0) + 1e-6);
    }

    // (c) lineage saturation: 5 pubkeys sharing ONE seed_root ⇒ C = 1, not 5.
    #[test]
    fn lineage_saturates_at_one() {
        let c = cfg();
        let root = pk(1);
        let corrs: Vec<_> = (10u8..15).map(|n| (pk(n), Some(root), 0.8f32)).collect();
        let cm = promotion_weight(corrs.iter().copied(), &c);
        assert!((cm - 1.0).abs() < 1e-4, "one lineage saturates at 1, got {cm}");
        assert_eq!(distinct_lineage_count(corrs.iter().copied(), &c), 1);
    }

    // (d) distinct lineages: 3 seed-disjoint lineages ⇒ C = 3 ⇒ K = 2 satisfied.
    #[test]
    fn distinct_lineages_sum() {
        let c = cfg();
        let corrs = vec![
            (pk(10), Some(pk(1)), 0.8f32),
            (pk(11), Some(pk(2)), 0.8f32),
            (pk(12), Some(pk(3)), 0.8f32),
        ];
        let cm = promotion_weight(corrs.iter().copied(), &c);
        assert!((cm - 3.0).abs() < 1e-4, "three lineages ⇒ C=3, got {cm}");
        let (k, _) = k_for(3);
        assert_eq!(k, 2);
        assert!(cm + EPS >= k as f32, "C=3 satisfies K=2");
    }

    // k_for schedule spot-checks (inc-1 §2.2 table).
    #[test]
    fn k_for_schedule() {
        assert_eq!(k_for(6), (3, 4)); // the 6-seed roster
        assert_eq!(k_for(3), (2, 2));
        assert_eq!(k_for(0), (2, 0)); // dormant floor
        assert_eq!(k_for(2), (2, 2));
    }

    fn store_with_6_seeds() -> (RepStore, Vec<PubKey>) {
        let c = cfg();
        let mut store = RepStore::in_memory(&c);
        let seeds: Vec<PubKey> = (1u8..=6).map(pk).collect();
        for s in &seeds {
            store.seeds.insert(*s);
        }
        (store, seeds)
    }

    // (e) decide across the branches.
    #[test]
    fn decide_branches() {
        let c = cfg();
        let (mut store, seeds) = store_with_6_seeds();
        let origin = pk(100); // non-seed

        // Two enrolled non-seed handles sharing ONE seed_root (seeds[0]).
        let a = pk(50);
        let b = pk(51);
        store.reps.insert(a, rr(a, 0.8));
        store.reps.insert(b, rr(b, 0.8));
        store.record_vouch(seeds[0], a);
        store.record_vouch(seeds[0], b);

        // C < K ⇒ Quarantine (one lineage = 1 < K=3), no seed among corroborators.
        let corr_ck = vec![(a, store.seed_root_of(&a)), (b, store.seed_root_of(&b))];
        let d1 = store.decide(hash(1), origin, &corr_ck, 1, 6, false, false, &c);
        assert_eq!(d1, Promotion::Quarantine, "C<K ⇒ Quarantine");

        // C ≥ K ⇒ Live with accrual > 0 (three seed-disjoint lineages).
        let corr_live: Vec<_> = seeds[0..3].iter().map(|s| (*s, Some(*s))).collect();
        let d2 = store.decide(hash(2), origin, &corr_live, 1, 6, false, false, &c);
        match d2 {
            Promotion::Live { accrue } => assert!(accrue > 0.0, "accrual must be > 0"),
            other => panic!("expected Live, got {other:?}"),
        }

        // Seed origin, normal write ⇒ Live.
        let d3 = store.decide(hash(3), seeds[0], &[], 1, 6, false, false, &c);
        assert_eq!(d3, Promotion::Live { accrue: 0.0 }, "seed normal ⇒ Live");

        // Seed origin, high-impact, ALONE ⇒ Quarantine (1 lineage < K_high=4).
        let d4 = store.decide(hash(4), seeds[0], &[], 1, 6, true, false, &c);
        assert_eq!(d4, Promotion::Quarantine, "lone seed high-impact ⇒ Quarantine");

        // Seed endorsement without full quorum ⇒ ProbationLive (cold-start).
        let d5 = store.decide(hash(5), origin, &[(seeds[0], Some(seeds[0]))], 1, 6, false, false, &c);
        assert_eq!(d5, Promotion::ProbationLive, "lone seed vouch ⇒ ProbationLive");
    }

    // (f) echo earns nothing: re-corroborating a recorded mem by the same pk
    // does not double-count and accrues no rep.
    #[test]
    fn echo_earns_nothing() {
        let c = cfg();
        let mut store = RepStore::in_memory(&c);
        let s1 = pk(1);
        store.seeds.insert(s1);
        let origin = pk(100);
        let mem = hash(7);

        store.record_promotion(mem, origin, 1, vec![(s1, Some(s1))], &c);
        let rep1 = store.rep(&origin);
        let cc1 = store.dag.corroborator_count(&mem);
        assert!(rep1 > 0.0, "first promotion accrues");
        assert_eq!(cc1, 1);

        // Echo: same mem, repeated corroborator.
        store.record_promotion(mem, origin, 1, vec![(s1, Some(s1)), (s1, Some(s1))], &c);
        let rep2 = store.rep(&origin);
        let cc2 = store.dag.corroborator_count(&mem);
        assert_eq!(rep1, rep2, "echo must not accrue");
        assert_eq!(cc1, cc2, "duplicate corroborator must not double-count");
    }

    // per-epoch accrual cap: a burst in one epoch cannot exceed alpha.
    #[test]
    fn accrual_epoch_capped() {
        let c = cfg();
        let mut store = RepStore::in_memory(&c);
        let s: Vec<PubKey> = (1u8..=6).map(pk).collect();
        for k in &s {
            store.seeds.insert(*k);
        }
        let origin = pk(100);
        // Many distinct promotions in the SAME epoch.
        for i in 0..20u8 {
            let corrs: Vec<_> = s[0..3].iter().map(|x| (*x, Some(*x))).collect();
            store.record_promotion(hash(50 + i), origin, 1, corrs, &c);
        }
        assert!(
            store.rep(&origin) <= c.accrual_alpha + 1e-4,
            "one epoch's accrual capped at alpha, got {}",
            store.rep(&origin)
        );
    }

    // (g) fail-closed load: a corrupt reputation.log ⇒ a store that refuses
    // promotion (and decide returns Quarantine for everyone).
    #[test]
    fn fail_closed_on_corrupt_log() {
        let c = cfg();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("reputation.log"), "not a valid json line\n").unwrap();
        let store = RepStore::load(dir.path(), &c);
        assert!(store.refuses_promotion(), "corrupt log ⇒ fail-closed");
        let d = store.decide(hash(1), pk(1), &[], 1, 6, false, false, &c);
        assert_eq!(d, Promotion::Quarantine, "poisoned store quarantines all");
    }

    // A broken hash chain (tampered line) is fail-closed — via the hash-chain
    // COMPARE, not the parse-error path. The tamper must stay length-preserving
    // ([u8;32] still deserializes) so it actually reaches the compare.
    #[test]
    fn fail_closed_on_broken_chain() {
        let c = cfg();
        let dir = tempfile::tempdir().unwrap();
        {
            let mut store = RepStore::new_at(&c, dir.path().to_path_buf());
            store.record_vouch(pk(1), pk(2));
        }
        let log_path = dir.path().join("reputation.log");
        let text = std::fs::read_to_string(&log_path).unwrap();

        // Positive control: the UNtampered log must load clean. This proves the
        // assertion below can only trip on the tamper, not on a load-path bug.
        assert!(
            !RepStore::load(dir.path(), &c).refuses_promotion(),
            "untampered log must load clean"
        );

        // Length-preserving tamper: flip one byte of the 32-element `chain`
        // array in place. The line still deserializes into [u8;32], so load
        // reaches the hash-chain compare — where the flipped byte != the
        // recomputed hash byte ⇒ poisoned.
        let mut line: serde_json::Value =
            serde_json::from_str(text.lines().next().unwrap()).unwrap();
        let first = line["chain"][0].as_u64().unwrap();
        line["chain"][0] = serde_json::json!((first ^ 0xFF) & 0xFF);
        std::fs::write(&log_path, serde_json::to_string(&line).unwrap() + "\n").unwrap();

        let store = RepStore::load(dir.path(), &c);
        assert!(
            store.refuses_promotion(),
            "tampered chain ⇒ fail-closed via hash-chain compare"
        );
    }

    // A clean append-only log round-trips through load and preserves rep.
    #[test]
    fn log_round_trip_preserves_rep() {
        let c = cfg();
        let dir = tempfile::tempdir().unwrap();
        // Seed via cfg so the loaded store agrees on seeds.
        let mut cfg2 = c.clone();
        // Use a base64 seed so seeds_from_cfg picks it up on reload.
        let seed_b64 = b64_encode_test(&pk(9));
        cfg2.seed_pubkeys = vec![seed_b64];
        let origin = pk(100);
        {
            let mut store = RepStore::load(dir.path(), &cfg2);
            let corrs = vec![(pk(9), Some(pk(9)))];
            store.record_promotion(hash(1), origin, 1, corrs, &cfg2);
            assert!(store.rep(&origin) > 0.0);
        }
        let reloaded = RepStore::load(dir.path(), &cfg2);
        assert!(!reloaded.refuses_promotion());
        assert!(reloaded.rep(&origin) > 0.0, "rep survived reload");
        assert!(reloaded.dag().is_promoted(&hash(1)));
    }

    // compact → snapshot → reload preserves state and clears the log.
    #[test]
    fn compact_snapshot_reload() {
        let c = cfg();
        let dir = tempfile::tempdir().unwrap();
        let mut cfg2 = c.clone();
        cfg2.seed_pubkeys = vec![b64_encode_test(&pk(9))];
        let origin = pk(100);
        {
            let mut store = RepStore::load(dir.path(), &cfg2);
            store.record_promotion(hash(1), origin, 1, vec![(pk(9), Some(pk(9)))], &cfg2);
            store.compact().unwrap();
        }
        assert!(dir.path().join("reputation.snapshot.gz").exists());
        let reloaded = RepStore::load(dir.path(), &cfg2);
        assert!(!reloaded.refuses_promotion());
        assert!(reloaded.rep(&origin) > 0.0, "rep survived compact+reload");
        assert!(reloaded.dag().is_promoted(&hash(1)));
    }

    // (#9) a LONE seed cannot self-promote a HIGH-IMPACT write at n=1: with one
    // live seed, K_high floors at 2 and one seed is a single lineage (1 < 2) ⇒
    // Quarantine. Guards the seed-branch `k_high.max(2)`.
    #[test]
    fn lone_seed_high_impact_quarantines() {
        let c = cfg();
        let mut store = RepStore::in_memory(&c);
        let s = pk(1);
        store.seeds.insert(s);
        assert_eq!(store.live_seed_count(), 1);
        assert_eq!(k_for(1), (2, 1)); // raw K_high=1 — the bug would admit `1 >= 1`
        // Lone seed, high-impact, no other lineages ⇒ Quarantine (1 < max(1,2)).
        let d = store.decide(hash(1), s, &[], 1, 1, /* high_impact = */ true, false, &c);
        assert_eq!(d, Promotion::Quarantine, "lone seed high-impact at n=1 ⇒ Quarantine");
        // Sanity: a NORMAL lone-seed write is still Live (inc-0 behaviour intact).
        let d_norm = store.decide(hash(2), s, &[], 1, 1, false, false, &c);
        assert_eq!(d_norm, Promotion::Live { accrue: 0.0 }, "normal lone-seed write ⇒ Live");
    }

    // (#10) the per-epoch accrual cap survives a restart: a mid-epoch crash must
    // NOT re-grant a fresh full α. Promote (accrue), reload from the log, promote
    // again in the SAME epoch — total accrual stays capped at α.
    #[test]
    fn epoch_cap_survives_reload() {
        let c = cfg();
        let dir = tempfile::tempdir().unwrap();
        let mut cfg2 = c.clone();
        // 3 seed-disjoint lineages so a promotion accrues real rep (D=3).
        cfg2.seed_pubkeys = vec![
            b64_encode_test(&pk(1)),
            b64_encode_test(&pk(2)),
            b64_encode_test(&pk(3)),
        ];
        let origin = pk(100);
        let epoch = 7u64;
        let corrs = || vec![(pk(1), Some(pk(1))), (pk(2), Some(pk(2))), (pk(3), Some(pk(3)))];
        {
            let mut store = RepStore::load(dir.path(), &cfg2);
            store.record_promotion(hash(1), origin, epoch, corrs(), &cfg2);
            assert!(store.rep(&origin) > 0.0, "first promotion accrues");
        }
        // Mid-epoch restart: the log replay must rebuild the epoch cap.
        let mut reloaded = RepStore::load(dir.path(), &cfg2);
        assert!(!reloaded.refuses_promotion());
        // Second promotion, SAME epoch, different content.
        reloaded.record_promotion(hash(2), origin, epoch, corrs(), &cfg2);
        assert!(
            reloaded.rep(&origin) <= cfg2.accrual_alpha + 1e-4,
            "epoch cap survived reload — no fresh α (rep={})",
            reloaded.rep(&origin)
        );
    }

    // (#4) a node-signed snapshot loads clean; a tampered or unsigned snapshot
    // fails the signature check ⇒ fail-closed (poisoned), never unioned.
    #[test]
    fn snapshot_signature_gates_load() {
        let c = cfg();
        let mut cfg2 = c.clone();
        cfg2.seed_pubkeys = vec![b64_encode_test(&pk(9))];
        let origin = pk(100);

        let make_snapshot = |dir: &Path| {
            let mut store = RepStore::load(dir, &cfg2);
            store.record_promotion(hash(1), origin, 1, vec![(pk(9), Some(pk(9)))], &cfg2);
            store.compact().unwrap();
        };

        // (i) clean: compact writes a node-signed snapshot; reload verifies + loads.
        {
            let dir = tempfile::tempdir().unwrap();
            make_snapshot(dir.path());
            assert!(dir.path().join("reputation.snapshot.gz").exists());
            assert!(dir.path().join("reputation.snapshot.sig").exists(), "snapshot is signed");
            let reloaded = RepStore::load(dir.path(), &cfg2);
            assert!(!reloaded.refuses_promotion(), "node-signed snapshot loads clean");
            assert!(reloaded.rep(&origin) > 0.0, "state survived signed compact+reload");
        }

        // (ii) tampered snapshot bytes ⇒ signature fails ⇒ poisoned.
        {
            let dir = tempfile::tempdir().unwrap();
            make_snapshot(dir.path());
            let gz_path = dir.path().join("reputation.snapshot.gz");
            let mut gz = std::fs::read(&gz_path).unwrap();
            let mid = gz.len() / 2;
            gz[mid] ^= 0xFF;
            std::fs::write(&gz_path, &gz).unwrap();
            let reloaded = RepStore::load(dir.path(), &cfg2);
            assert!(reloaded.refuses_promotion(), "tampered snapshot ⇒ fail-closed");
        }

        // (iii) unsigned snapshot (sig sidecar removed) ⇒ poisoned.
        {
            let dir = tempfile::tempdir().unwrap();
            make_snapshot(dir.path());
            std::fs::remove_file(dir.path().join("reputation.snapshot.sig")).unwrap();
            let reloaded = RepStore::load(dir.path(), &cfg2);
            assert!(reloaded.refuses_promotion(), "unsigned snapshot ⇒ fail-closed");
        }
    }

    // (h) seed_root DAG walk resolves a vouch chain to the seed; a cycle
    // terminates with None.
    #[test]
    fn seed_root_walk_and_cycle() {
        let mut dag = CorroborationDag::default();
        let mut seeds: HashSet<PubKey> = HashSet::new();
        let s = pk(1);
        seeds.insert(s);
        let a = pk(2);
        let b = pk(3);
        dag.record_vouch(s, a); // s vouches a
        dag.record_vouch(a, b); // a vouches b
        assert_eq!(dag.seed_root(&b, &seeds), Some(s), "chain resolves to seed");
        assert_eq!(dag.seed_root(&a, &seeds), Some(s));
        assert_eq!(dag.seed_root(&s, &seeds), Some(s), "seed roots to itself");
        assert_eq!(dag.seed_root(&pk(200), &seeds), None, "unknown ⇒ ⊥");

        // Cycle with no seed must terminate (not hang) and return None.
        let mut dag2 = CorroborationDag::default();
        let empty: HashSet<PubKey> = HashSet::new();
        let x = pk(10);
        let y = pk(11);
        dag2.record_vouch(x, y);
        dag2.record_vouch(y, x); // x ↔ y cycle
        assert_eq!(dag2.seed_root(&x, &empty), None, "cycle terminates ⇒ None");
        assert_eq!(dag2.seed_root(&y, &empty), None);
    }

    // Minimal base64 encoder for the persistence round-trip tests.
    fn b64_encode_test(data: &[u8]) -> String {
        const A: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in data.chunks(3) {
            let b0 = chunk[0];
            let b1 = *chunk.get(1).unwrap_or(&0);
            let b2 = *chunk.get(2).unwrap_or(&0);
            let n = (u32::from(b0) << 16) | (u32::from(b1) << 8) | u32::from(b2);
            out.push(A[((n >> 18) & 63) as usize] as char);
            out.push(A[((n >> 12) & 63) as usize] as char);
            out.push(if chunk.len() > 1 { A[((n >> 6) & 63) as usize] as char } else { '=' });
            out.push(if chunk.len() > 2 { A[(n & 63) as usize] as char } else { '=' });
        }
        out
    }
}
