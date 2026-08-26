//! #836 / ADR-0049: `kannaka facets` — the facet backfill migration.
//!
//! Write-time decomposition (`KANNAKA_FACET_DECOMPOSE`) only ever touches NEW
//! memories. Every compound memory stored before the flag went live remains a
//! single smeared wavefront that short queries cannot reach (#836's
//! length/lexical findings). `facets backfill` migrates the existing corpus.
//!
//! Safety posture, in order:
//!   1. Dry run by default — `--apply` is required to mutate.
//!   2. `--apply` refuses to run without first writing a local pre-migration
//!      snapshot (gzip of the .hrm). Its filename does NOT end in
//!      `-<agent>.hrm.gz`, so the retention pruner never deletes it — it is a
//!      permanent restore point for a one-way operation.
//!   3. The HRM write lock is taken; if another writer (swarm join, a dream)
//!      holds it, we refuse rather than double-write. On non-Unix the lock is
//!      advisory-only and we say so.

use super::{data_dir, try_acquire_write_lock, KannakaConfig};
use std::io::Write;

pub(crate) fn handle_facets(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    args: &[String],
) {
    match args.first().map(String::as_str) {
        Some("backfill") => handle_backfill(sys, cfg, &args[1..]),
        _ => {
            eprintln!("Usage: kannaka facets backfill [--apply]");
            eprintln!();
            eprintln!("  Decompose every existing compound memory into atomic facets");
            eprintln!("  (ADR-0049, #836). Default is a DRY RUN that mutates nothing.");
            eprintln!("  --apply executes; a pre-migration snapshot is written first");
            eprintln!("  and the run refuses to proceed if that snapshot fails.");
            std::process::exit(2);
        }
    }
}

fn handle_backfill(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    args: &[String],
) {
    let apply = args.iter().any(|a| a == "--apply");
    for a in args {
        if a != "--apply" {
            eprintln!("[facets] ignoring unknown flag: {a}");
        }
    }

    // Single-writer guard. A backfill that races the swarm-join writer would
    // be a lost-update on a one-way file; refuse rather than proceed.
    let lock = try_acquire_write_lock();
    if lock.is_none() {
        eprintln!(
            "[facets] REFUSING: another process holds the HRM write lock \
             (a swarm join writer or a running dream). Stop it, then re-run."
        );
        std::process::exit(1);
    }
    #[cfg(not(unix))]
    if apply {
        eprintln!(
            "[facets] NOTE: the write lock is advisory-only on this platform — \
             confirm no kannaka writer daemon is running before trusting this run."
        );
    }

    if apply {
        if let Err(e) = pre_migration_snapshot(&cfg.agent.id, sys) {
            eprintln!("[facets] REFUSING --apply: pre-migration snapshot failed: {e}");
            eprintln!("[facets] a one-way corpus rewrite does not run without a restore point.");
            std::process::exit(1);
        }
    }

    match sys.engine.backfill_all_facets(apply) {
        None => {
            eprintln!("[facets] unsupported backend — not an HRM store, nothing to migrate.");
            std::process::exit(1);
        }
        Some(stats) => {
            let mode = if apply { "APPLIED" } else { "DRY RUN" };
            println!("[facets] {mode}: scanned {} canonical rows", stats.scanned);
            println!("  already decomposed:        {}", stats.already_decomposed);
            println!("  facet rows (never split):  {}", stats.facet_rows);
            println!("  atomic (nothing to mint):  {}", stats.atomic);
            if apply {
                println!("  parents decomposed:        {}", stats.parents_decomposed);
                println!("  facets minted:             {}", stats.facets_minted);
            } else {
                println!("  parents that WOULD split:  {}", stats.parents_decomposed);
                println!("  facets that WOULD mint:    {}", stats.facets_minted);
            }
            if stats.errors > 0 {
                println!("  errors (logged above):     {}", stats.errors);
            }
            if apply {
                match sys.engine.store.flush() {
                    Ok(_) => println!("[facets] flushed to disk."),
                    Err(e) => eprintln!(
                        "[facets] WARNING: flush after backfill failed: {e} — the \
                         in-memory migration is NOT persisted; re-run flush before \
                         trusting the on-disk state"
                    ),
                }
            } else {
                println!(
                    "[facets] no changes made. Re-run with --apply to execute \
                     (a pre-migration snapshot is taken first)."
                );
            }
        }
    }
    drop(lock);
}

/// Local, NATS-free snapshot: flush, gzip the .hrm, write it under snapshots/
/// with a name the retention pruner will never match (it prunes files ending
/// `-<agent>.hrm.gz`; this one ends `-pre-facet-backfill.hrm.gz`).
fn pre_migration_snapshot(
    agent_id: &str,
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
) -> Result<(), String> {
    sys.engine.store.flush().map_err(|e| format!("flush: {e}"))?;
    let hrm = data_dir().join("kannaka.hrm");
    let bytes = std::fs::read(&hrm).map_err(|e| format!("read {}: {e}", hrm.display()))?;
    let dir = data_dir().join("snapshots");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir {}: {e}", dir.display()))?;
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let path = dir.join(format!("{ts}-{agent_id}-pre-facet-backfill.hrm.gz"));
    let mut gz = flate2::write::GzEncoder::new(
        Vec::with_capacity(bytes.len() / 4),
        flate2::Compression::default(),
    );
    gz.write_all(&bytes).map_err(|e| format!("gzip: {e}"))?;
    let out = gz.finish().map_err(|e| format!("gzip finish: {e}"))?;
    std::fs::write(&path, &out).map_err(|e| format!("write {}: {e}", path.display()))?;
    println!(
        "[facets] pre-migration snapshot: {} ({} KB, exempt from retention pruning)",
        path.display(),
        out.len() / 1024
    );
    Ok(())
}
