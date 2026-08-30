//! build.rs — embed the resolved `consciousness-core` version at compile
//! time so the binary can self-report its constellation-physics version
//! without a runtime crate-metadata lookup.
//!
//! Reads `Cargo.lock` (always present after `cargo build`) and finds the
//! `consciousness-core` package entry. The version is emitted as the
//! `KANNAKA_CONSCIOUSNESS_CORE_VERSION` env var captured into the binary
//! via `env!()`. Falls back to `"unknown"` if the lockfile is missing
//! or doesn't list the dep — neither should happen in a real build but
//! we never want `kannaka --version` to panic.

use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
    let lockfile = PathBuf::from(&manifest_dir).join("Cargo.lock");

    // Tell cargo to re-run only when the lockfile changes.
    println!("cargo:rerun-if-changed=Cargo.lock");

    let version = match fs::read_to_string(&lockfile) {
        Ok(contents) => parse_dep_version(&contents, "consciousness-core").unwrap_or_else(|| "unknown".to_string()),
        Err(_) => "unknown".to_string(),
    };

    println!("cargo:rustc-env=KANNAKA_CONSCIOUSNESS_CORE_VERSION={version}");
}

/// Minimal Cargo.lock parser — scans for `[[package]]` blocks and returns
/// the `version` of the named package. Avoids pulling in a TOML crate
/// from build-deps; the lockfile format is line-oriented and stable.
fn parse_dep_version(lock: &str, name: &str) -> Option<String> {
    let mut in_block = false;
    let mut current_name: Option<String> = None;
    for line in lock.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            in_block = true;
            current_name = None;
            continue;
        }
        if !in_block {
            continue;
        }
        if let Some(rest) = line.strip_prefix("name = \"") {
            if let Some(end) = rest.find('"') {
                current_name = Some(rest[..end].to_string());
            }
        } else if let Some(rest) = line.strip_prefix("version = \"") {
            if let Some(end) = rest.find('"') {
                if current_name.as_deref() == Some(name) {
                    return Some(rest[..end].to_string());
                }
            }
        } else if line.is_empty() {
            in_block = false;
            current_name = None;
        }
    }
    None
}
