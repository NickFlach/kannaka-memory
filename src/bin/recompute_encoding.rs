//! One-shot tool: re-encode every TEXT wavefront's content through the current
//! pipeline, fixing HRM files whose vectors were written by the broken #106
//! encoder (#107). Chiral (v2) HRMs only.
//!
//! Usage: kannaka-recompute-encoding [data-dir] [--dry-run]
//!
//! SCOPE / SAFETY:
//!   - TEXT HRMs only. Dream-hallucinated wavefronts and non-text (audio/visual)
//!     modalities are skipped automatically. But do NOT run this on a SUBSTRATE
//!     HRM (ADR-0027): its raw one-hot anchors are Modality::Unknown, which is
//!     indistinguishable from pre-ADR-0042 text memories, so they WOULD be
//!     re-encoded and their orthogonal Kuramoto seed destroyed.
//!   - Requires the real Ollama embedding model. If Ollama is unreachable, the
//!     pipeline would silently fall back to the hash encoder and rewrite the whole
//!     corpus with pseudo-embeddings — so a real run ABORTS unless Ollama answers.
//!
//! ROLLOUT: SNAPSHOT the HRM first, run --dry-run to see the count, then run for
//! real and verify recall (#83 repro) before adopting / rolling to the next box.

use kannaka_memory::hrm_store::HrmStore;
use kannaka_memory::openclaw::make_pipeline;
use std::path::PathBuf;
use std::time::Duration;

/// The local Ollama endpoint `make_pipeline`'s OllamaEncoder::default_local uses.
const OLLAMA_URL: &str = "http://localhost:11434";

fn ollama_reachable() -> bool {
    ureq::get(&format!("{OLLAMA_URL}/api/tags"))
        .timeout(Duration::from_secs(3))
        .call()
        .is_ok()
}

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
    eprintln!(
        "NOTE: TEXT HRMs only — hallucinations + non-text modalities are skipped; do NOT run on a \
         substrate HRM (raw anchors would be corrupted)."
    );
    if dry_run {
        eprintln!("(dry run — no changes will be written)");
    }

    // Refuse a real run when Ollama is down: otherwise make_pipeline's
    // CompositeEncoder silently uses the hash fallback and we would overwrite every
    // vector with pseudo-embeddings while reporting success. (A dry run only counts,
    // so the probe is a real-run gate.)
    if !dry_run && !ollama_reachable() {
        eprintln!(
            "ABORT: Ollama ({OLLAMA_URL}) is unreachable. Re-encoding needs the real embedding \
             model — refusing to hash-rewrite the corpus. Start Ollama and retry (or --dry-run)."
        );
        std::process::exit(1);
    }

    let pipeline = make_pipeline();
    let mut store = HrmStore::load(pipeline, hrm_path).unwrap_or_else(|e| {
        eprintln!("Failed to load HRM: {e}");
        std::process::exit(1);
    });

    match store.recompute_encoding(dry_run) {
        Ok((scanned, updated)) => {
            if dry_run {
                eprintln!("[dry-run] would re-encode {updated} of {scanned} memories (text only)");
            } else {
                eprintln!("Re-encoded {updated} of {scanned} memories through the current pipeline");
                eprintln!("Cluster sidecar invalidated; verify recall before rolling out further.");
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    }
}
