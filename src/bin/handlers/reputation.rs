//! `kannaka reputation` — operator inspection of the inc-1 corroboration trust
//! ledger.
//!
//! Read the pubkey-keyed reputation store (`show`, `list`) plus the single
//! operator hard-reject lever (`hard-reject`). These verbs change NO automatic
//! runtime behaviour: the corroboration gate stays dormant until an operator
//! pins a seed AND sets `swarm_trust.corroboration_gate_enabled`. Follows the
//! handler-extraction pattern in `handlers/substrate.rs`; base64 (standard
//! alphabet, matching the provenance wire codec) lives in `bin/kannaka.rs` as
//! `crate::b64_{encode_std,decode_32}` and is shared with the identity handler.
//!
//! config-vs-store split: seeds are read from config (`swarm_trust.seed_pubkeys`
//! → `RepStore` seed set); rep/promoted/poison/lineage come from the store at
//! `<data_dir>/reputation.{log,snapshot.gz}`.

use std::process;

use kannaka_memory::config::KannakaConfig;
use kannaka_memory::reputation::{RepStore, SeedStatus};

const USAGE: &str = "Usage: kannaka reputation \
    <show <b64-pubkey> [--json] | list [--json] | hard-reject <b64-mem-hash> --origin <b64-pubkey>>";

pub(crate) fn handle_reputation(args: &[String]) {
    match args.get(1).map(|s| s.as_str()) {
        Some("show") => handle_show(args),
        Some("list") => handle_list(args),
        Some("hard-reject") => handle_hard_reject(args),
        _ => {
            eprintln!("{USAGE}");
            process::exit(1);
        }
    }
}

/// Enrollment status → the stable label the CLI + JSON expose.
fn status_str(s: SeedStatus) -> &'static str {
    match s {
        SeedStatus::Seed => "Seed",
        SeedStatus::Enrolled => "Enrolled",
        SeedStatus::Unknown => "Unknown",
    }
}

/// `kannaka reputation show <b64-pubkey> [--json]` — rep, seed_status,
/// seed_root, and the promoted/poison bookkeeping counters for one pubkey. An
/// unknown key is rep 0 / Unknown / seed_root "-".
fn handle_show(args: &[String]) {
    let usage = "Usage: kannaka reputation show <base64-pubkey> [--json]";
    let b64 = require_positional(args, 2, usage);
    let pk = match crate::b64_decode_32(&b64) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("  reputation show: malformed pubkey — {e}");
            process::exit(1);
        }
    };
    let json = args.iter().any(|a| a == "--json");

    let cfg = KannakaConfig::load();
    let dir = KannakaConfig::data_dir();
    let store = RepStore::load(&dir, &cfg.swarm_trust);

    let rep = store.rep(&pk);
    let status = store.seed_status(&pk);
    let seed_root = store.seed_root_of(&pk).map(|r| crate::b64_encode_std(&r));
    let (promoted, poison) = store
        .record(&pk)
        .map(|r| (r.promoted, r.poison))
        .unwrap_or((0, 0));

    if json {
        let v = serde_json::json!({
            "pubkey": b64,
            "rep": rep,
            "status": status_str(status),
            "seed_root": seed_root,
            "promoted": promoted,
            "poison": poison,
            "fail_closed": store.refuses_promotion(),
        });
        println!("{v}");
    } else {
        println!("  pubkey:    {b64}");
        println!("  rep:       {rep:.4}");
        println!("  status:    {}", status_str(status));
        println!("  seed_root: {}", seed_root.as_deref().unwrap_or("-"));
        println!("  promoted:  {promoted}");
        println!("  poison:    {poison}");
        if store.refuses_promotion() {
            println!(
                "  note:      store is fail-closed (corrupt/unverifiable log) — all promotion refused"
            );
        }
    }
}

/// `kannaka reputation list [--json]` — every known pubkey (pinned seeds ∪
/// pubkeys carrying a rep record) with its rep + status.
fn handle_list(args: &[String]) {
    let json = args.iter().any(|a| a == "--json");

    let cfg = KannakaConfig::load();
    let dir = KannakaConfig::data_dir();
    let store = RepStore::load(&dir, &cfg.swarm_trust);

    // Known pubkeys = pinned seeds ∪ pubkeys that carry a rep record.
    // BTreeSet keeps the listing deterministic (byte order).
    let mut keys: std::collections::BTreeSet<[u8; 32]> = std::collections::BTreeSet::new();
    for s in store.seeds() {
        keys.insert(*s);
    }
    for r in store.records() {
        keys.insert(r.pubkey);
    }

    #[allow(clippy::type_complexity)]
    let rows: Vec<(String, f32, SeedStatus, Option<String>, u32, u32)> = keys
        .iter()
        .map(|pk| {
            let (promoted, poison) = store
                .record(pk)
                .map(|r| (r.promoted, r.poison))
                .unwrap_or((0, 0));
            (
                crate::b64_encode_std(pk),
                store.rep(pk),
                store.seed_status(pk),
                store.seed_root_of(pk).map(|r| crate::b64_encode_std(&r)),
                promoted,
                poison,
            )
        })
        .collect();

    if json {
        let arr: Vec<serde_json::Value> = rows
            .iter()
            .map(|(pk, rep, status, root, promoted, poison)| {
                serde_json::json!({
                    "pubkey": pk,
                    "rep": rep,
                    "status": status_str(*status),
                    "seed_root": root,
                    "promoted": promoted,
                    "poison": poison,
                })
            })
            .collect();
        println!("{}", serde_json::Value::Array(arr));
        return;
    }

    if rows.is_empty() {
        println!("  (no known pubkeys — no seeds pinned and no reputation records yet)");
        println!("  Pin a seed with: kannaka identity enroll-seed <base64-pubkey>");
        return;
    }
    println!(
        "  {:<44}  {:>7}  {:<9}  {:>8}  {:>6}",
        "PUBKEY(b64)", "REP", "STATUS", "PROMOTED", "POISON"
    );
    for (pk, rep, status, _root, promoted, poison) in &rows {
        println!(
            "  {:<44}  {:>7.4}  {:<9}  {:>8}  {:>6}",
            pk,
            rep,
            status_str(*status),
            promoted,
            poison
        );
    }
    println!(
        "  {} known pubkey(s); {} pinned seed(s)",
        rows.len(),
        store.live_seed_count()
    );
}

/// `kannaka reputation hard-reject <b64-mem-hash> --origin <b64-pubkey>` —
/// operator: dock the origin's reputation (`record_poison`, rep -= P_POISON)
/// for a rejected memory.
///
/// The committed store keys reputation on the origin PUBKEY; resolving an
/// origin *from* a mem_hash is the (not-yet-wired) admit layer's job, so the
/// operator supplies `--origin` explicitly for now. The mem_hash is validated
/// and echoed for the audit trail.
fn handle_hard_reject(args: &[String]) {
    let usage =
        "Usage: kannaka reputation hard-reject <base64-mem-hash> --origin <base64-pubkey>";
    let mem_b64 = require_positional(args, 2, usage);
    let _mem_hash = match crate::b64_decode_32(&mem_b64) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("  hard-reject: malformed mem_hash — {e}");
            process::exit(1);
        }
    };
    // TODO(inc-1): M-of-N operator quorum — a hard-reject should require M
    // operator signatures before it docks rep; today it is single-operator.
    let origin_b64 = match flag_value(args, "--origin") {
        Some(v) => v,
        None => {
            eprintln!("{usage}");
            eprintln!("  (need --origin <base64-pubkey>: the memory origin whose rep to dock)");
            process::exit(1);
        }
    };
    let origin = match crate::b64_decode_32(&origin_b64) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("  hard-reject: malformed --origin pubkey — {e}");
            process::exit(1);
        }
    };

    let cfg = KannakaConfig::load();
    let dir = KannakaConfig::data_dir();
    let mut store = RepStore::load(&dir, &cfg.swarm_trust);

    let before = store.rep(&origin);
    store.record_poison(origin, &cfg.swarm_trust);
    let after = store.rep(&origin);

    println!("  \u{2713} hard-reject recorded for mem {mem_b64}");
    println!("  origin {origin_b64}: rep {before:.4} -> {after:.4}");
    if store.seed_status(&origin) == SeedStatus::Seed {
        println!(
            "  note: origin is a pinned seed — rep() reports 1.0 for seeds regardless; the poison \
             is logged, but a seed's authority comes from the pin. Use `kannaka identity revoke` to unpin."
        );
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A required positional at `idx` (rejects a missing value or a `--flag`).
fn require_positional(args: &[String], idx: usize, usage: &str) -> String {
    match args.get(idx) {
        Some(s) if !s.is_empty() && !s.starts_with("--") => s.clone(),
        _ => {
            eprintln!("{usage}");
            process::exit(1);
        }
    }
}

/// The value following `flag` anywhere in `args`, if present.
fn flag_value(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // `reputation show` on an unknown key resolves to rep 0 / Unknown — the
    // read-side the handler formats.
    #[test]
    fn show_unknown_key_is_rep_zero_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = KannakaConfig::default(); // no seeds pinned
        let store = RepStore::load(dir.path(), &cfg.swarm_trust);
        let unknown = [42u8; 32];
        assert_eq!(store.rep(&unknown), 0.0, "unknown handle is rep 0");
        assert_eq!(store.seed_status(&unknown), SeedStatus::Unknown);
        assert_eq!(status_str(store.seed_status(&unknown)), "Unknown");
        assert!(store.seed_root_of(&unknown).is_none(), "unknown handle has ⊥ lineage");
        assert!(store.record(&unknown).is_none(), "unknown handle has no record");
    }

    #[test]
    fn status_str_maps_all_variants() {
        assert_eq!(status_str(SeedStatus::Seed), "Seed");
        assert_eq!(status_str(SeedStatus::Enrolled), "Enrolled");
        assert_eq!(status_str(SeedStatus::Unknown), "Unknown");
    }

    #[test]
    fn flag_value_finds_and_misses() {
        let args: Vec<String> = ["hard-reject", "MEM", "--origin", "PK"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(flag_value(&args, "--origin").as_deref(), Some("PK"));
        assert_eq!(flag_value(&args, "--missing"), None);
    }
}
