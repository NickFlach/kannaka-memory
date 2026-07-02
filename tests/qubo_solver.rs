//! T3.3 solver acceptance against the golden corpus (ADR-0038 §Validation, #479).
//!
//!   - `ClassicalAnneal` finds the **optimum** on every exact-solvable golden
//!     file (< 20 vars ⇒ exhaustive, `exact: true`);
//!   - and ≥ 95% of optimal energy on the larger files (SA with restarts);
//!   - solutions carry entropy provenance; a fixed seed is reproducible.

use std::path::PathBuf;

use kannaka_memory::entropy::PrngSource;
use kannaka_memory::qubo::{
    ClassicalAnneal, ConsolidationProblem, ConsolidationSolver, SolveBudget,
};
use serde::Deserialize;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("qubo")
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    file: String,
    num_vars: usize,
    exact_solvable: bool,
    optimum_energy: f64,
    #[allow(dead_code)]
    optimum_assignment: Vec<bool>,
    #[allow(dead_code)]
    description: String,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    files: Vec<ManifestEntry>,
}

fn load_manifest() -> Manifest {
    let s = std::fs::read_to_string(dir().join("manifest.json"))
        .expect("tests/qubo/manifest.json — run `cargo run --example gen_qubo_corpus`");
    serde_json::from_str(&s).expect("parse manifest.json")
}

fn load_problem(file: &str) -> ConsolidationProblem {
    let s = std::fs::read_to_string(dir().join(file)).unwrap_or_else(|_| panic!("read {file}"));
    serde_json::from_str(&s).unwrap_or_else(|e| panic!("parse {file}: {e}"))
}

#[test]
fn classical_anneal_hits_optimum_on_exact_files() {
    let solver = ClassicalAnneal::with_seed(0xC0FFEE);
    for e in load_manifest().files.into_iter().filter(|e| e.exact_solvable) {
        let p = load_problem(&e.file);
        let sol = solver.solve(&p, SolveBudget::default()).unwrap();
        assert!(sol.exact, "{}: <20 vars must solve exactly", e.file);
        assert!(
            (sol.energy - e.optimum_energy).abs() < 1e-9,
            "{}: solver energy {} != optimum {}",
            e.file,
            sol.energy,
            e.optimum_energy
        );
        // The returned assignment actually achieves the reported energy.
        assert!((p.energy(&sol.assignment) - sol.energy).abs() < 1e-9, "{}: energy mismatch", e.file);
    }
}

#[test]
fn classical_anneal_within_95pct_on_large_files() {
    let solver = ClassicalAnneal::with_seed(0xBEEF);
    let large: Vec<_> = load_manifest().files.into_iter().filter(|e| !e.exact_solvable).collect();
    assert!(!large.is_empty(), "corpus should have large files");
    for e in large {
        let p = load_problem(&e.file);
        assert!(e.num_vars >= 20, "{}: large file should be >=20 vars", e.file);
        let sol = solver.solve(&p, SolveBudget::default()).unwrap();
        assert!(!sol.exact, "{}: >=20 vars is heuristic", e.file);
        // Can't do better than the true optimum (known by construction here).
        assert!(
            sol.energy >= e.optimum_energy - 1e-9,
            "{}: energy {} beats documented optimum {} — corpus bug",
            e.file,
            sol.energy,
            e.optimum_energy
        );
        // ≥95% of optimal energy. Optima here are negative, so achieving at least
        // 95% of the magnitude means solved/optimum >= 0.95.
        let quality = if e.optimum_energy == 0.0 {
            if sol.energy == 0.0 { 1.0 } else { 0.0 }
        } else {
            sol.energy / e.optimum_energy
        };
        assert!(
            quality >= 0.95,
            "{}: quality {:.4} < 0.95 (solved {}, optimum {})",
            e.file,
            quality,
            sol.energy,
            e.optimum_energy
        );
    }
}

#[test]
fn solutions_carry_provenance_and_are_reproducible() {
    let p = load_problem("12-large-ferromagnet.json");

    // Entropy-seeded solve records the source's provenance.
    let seeded = ClassicalAnneal::from_entropy(&mut PrngSource::new());
    let sol = seeded.solve(&p, SolveBudget::default()).unwrap();
    assert!(sol.provenance.is_prng(), "PRNG-seeded solve carries prng:// provenance");
    assert_eq!(sol.solver, "ClassicalAnneal");

    // A fixed seed is deterministic across constructions.
    let a = ClassicalAnneal::with_seed(99).solve(&p, SolveBudget::default()).unwrap();
    let b = ClassicalAnneal::with_seed(99).solve(&p, SolveBudget::default()).unwrap();
    assert_eq!(a.assignment, b.assignment);
    assert_eq!(a.energy, b.energy);
}
