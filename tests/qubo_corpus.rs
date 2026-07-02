//! T3.2 golden-corpus validation (ADR-0038 §Validation).
//!
//! The committed `tests/qubo/*.json` are the golden artifacts; `manifest.json`
//! documents each one's known optimum. This suite proves the artifacts are
//! well-formed and the documented optima are real:
//!   - every file parses, validates, and JSON round-trips exactly;
//!   - the documented assignment achieves the documented energy;
//!   - for exact-solvable files (<20 vars) brute force confirms it's the *global*
//!     optimum — so T3.3's solver has a trustworthy target.
//!
//! Regenerate the corpus with `cargo run --example gen_qubo_corpus`.

use std::path::PathBuf;

use kannaka_memory::qubo::ConsolidationProblem;
use serde::Deserialize;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("qubo")
}

#[derive(Debug, Deserialize)]
pub struct ManifestEntry {
    pub file: String,
    pub num_vars: usize,
    pub exact_solvable: bool,
    pub optimum_energy: f64,
    pub optimum_assignment: Vec<bool>,
    #[allow(dead_code)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub files: Vec<ManifestEntry>,
}

pub fn load_manifest() -> Manifest {
    let s = std::fs::read_to_string(dir().join("manifest.json"))
        .expect("tests/qubo/manifest.json — regenerate with `cargo run --example gen_qubo_corpus`");
    serde_json::from_str(&s).expect("parse manifest.json")
}

pub fn load_problem(file: &str) -> ConsolidationProblem {
    let s = std::fs::read_to_string(dir().join(file)).unwrap_or_else(|_| panic!("read {file}"));
    serde_json::from_str(&s).unwrap_or_else(|e| panic!("parse {file}: {e}"))
}

/// Exhaustive optimum — test-only ground truth for small problems.
pub fn brute_force(p: &ConsolidationProblem) -> (Vec<bool>, f64) {
    let n = p.num_vars();
    assert!(n < 24, "brute_force is for exact-solvable sizes only");
    let mut best = vec![false; n];
    let mut best_e = f64::INFINITY;
    for mask in 0u64..(1u64 << n) {
        let x: Vec<bool> = (0..n).map(|i| (mask >> i) & 1 == 1).collect();
        let e = p.energy(&x);
        if e < best_e {
            best_e = e;
            best = x;
        }
    }
    (best, best_e)
}

#[test]
fn corpus_is_twelve_with_expected_split() {
    let m = load_manifest();
    assert_eq!(m.files.len(), 12, "ADR-0038 calls for 12 golden QUBOs");
    let exact = m.files.iter().filter(|e| e.exact_solvable).count();
    let large = m.files.iter().filter(|e| !e.exact_solvable).count();
    assert_eq!(exact, 10, "10 exact-solvable (<20 var) files");
    assert_eq!(large, 2, "2 large (>=20 var) files for the >=95% test");
    for e in &m.files {
        assert_eq!(e.exact_solvable, e.num_vars < 20, "{}: exact flag vs size", e.file);
        assert_eq!(e.optimum_assignment.len(), e.num_vars, "{}: assignment width", e.file);
    }
}

#[test]
fn golden_files_round_trip_and_validate() {
    for e in load_manifest().files {
        let p = load_problem(&e.file);
        p.validate().unwrap_or_else(|err| panic!("{} invalid: {err}", e.file));
        assert_eq!(p.num_vars(), e.num_vars, "{} var count", e.file);
        let js = serde_json::to_string(&p).unwrap();
        let back: ConsolidationProblem = serde_json::from_str(&js).unwrap();
        assert_eq!(p, back, "{} did not round-trip", e.file);
    }
}

#[test]
fn documented_optima_are_correct() {
    for e in load_manifest().files {
        let p = load_problem(&e.file);
        // The documented assignment achieves the documented energy.
        let e_doc = p.energy(&e.optimum_assignment);
        assert!(
            (e_doc - e.optimum_energy).abs() < 1e-9,
            "{}: energy(documented assignment)={e_doc} != manifest optimum {}",
            e.file,
            e.optimum_energy
        );
        // For exact-solvable files, brute force must agree it is the global minimum.
        if e.exact_solvable {
            let (_, best) = brute_force(&p);
            assert!(
                (best - e.optimum_energy).abs() < 1e-9,
                "{}: brute-force optimum {best} != manifest {}",
                e.file,
                e.optimum_energy
            );
        }
    }
}
