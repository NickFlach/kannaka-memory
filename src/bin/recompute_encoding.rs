//! One-shot tool: re-encode every wavefront's content through the current
//! pipeline, fixing HRM files whose vectors were written by the broken #106
//! encoder (#107). Chiral (v2) HRMs only.
//!
//! Usage: kannaka-recompute-encoding [data-dir] [--dry-run]
//!
//! SNAPSHOT the HRM first (the #107 rollout gate), run with --dry-run to see the
//! count, then run for real and verify recall before adopting.

use kannaka_memory::hrm_store::HrmStore;
use kannaka_memory::openclaw::make_pipeline;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dry_run = args.iter().any(|a| a == "--dry-run");
    let data_dir = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .map(PathBuf::from)
        .or_else(|| std::env::var("KANNAKA_DATA_DIR").ok().map(PathBuf::from))
        .unwrap_or_else(|| {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".kannaka")
        });

    let hrm_path = data_dir.join("kannaka.hrm");
    eprintln!("HRM: {}", hrm_path.display());
    if dry_run {
        eprintln!("(dry run — no changes will be written)");
    }

    let pipeline = make_pipeline();
    let mut store = HrmStore::load(pipeline, hrm_path).unwrap_or_else(|e| {
        eprintln!("Failed to load HRM: {}", e);
        std::process::exit(1);
    });

    match store.recompute_encoding(dry_run) {
        Ok((scanned, updated)) => {
            if dry_run {
                eprintln!("[dry-run] would re-encode {updated} of {scanned} memories");
            } else {
                eprintln!("Re-encoded {updated} of {scanned} memories through the current pipeline");
                eprintln!("Cluster sidecar invalidated; verify recall before rolling out further.");
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
