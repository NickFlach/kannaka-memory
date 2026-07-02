//! Regenerates the golden QUBO corpus in `tests/qubo/` (ADR-0038 validation).
//!
//!   cargo run --example gen_qubo_corpus
//!
//! Writes 12 hand-designed `kannaka-qubo/1` problems plus a `manifest.json`
//! documenting each one's known optimum (energy + an optimal assignment) and
//! whether it is exact-solvable (< 20 vars → brute-forceable). The committed
//! files are the golden artifacts; `tests/qubo_corpus.rs` validates them (round
//! trip + re-derived optima) and `tests/qubo_solver.rs` (T3.3) checks the solver
//! against them. This generator only exists so the corpus can be rebuilt after
//! an intentional change — CI never runs it.

use std::path::PathBuf;

use kannaka_memory::qubo::{ConsolidationProblem, Metadata, ProblemBuilder, VarKind};
use serde::Serialize;

struct Spec {
    file: &'static str,
    problem: ConsolidationProblem,
    description: &'static str,
    /// Known optimum for problems too large to brute-force (≥ 20 vars). Small
    /// problems leave this `None`; the generator brute-forces them.
    known_optimum: Option<Vec<bool>>,
}

#[derive(Serialize)]
struct ManifestEntry {
    file: String,
    num_vars: usize,
    exact_solvable: bool,
    optimum_energy: f64,
    optimum_assignment: Vec<bool>,
    description: String,
}

#[derive(Serialize)]
struct Manifest {
    format: String,
    note: String,
    files: Vec<ManifestEntry>,
}

fn meta(hemisphere: &str) -> Metadata {
    Metadata {
        hemisphere: Some(hemisphere.to_string()),
        phi_at_emit: Some(0.18),
        entropy_provenance: Some("prng://legacy".to_string()),
        ..Default::default()
    }
}

fn corpus() -> Vec<Spec> {
    let mut specs = Vec::new();

    // 01 — all keep: every keep_link wants to stay (negative linear), no coupling.
    {
        let mut b = ProblemBuilder::new("golden-01-all-keep");
        for (s, c) in [("skip:a", -2.0), ("skip:b", -1.5), ("skip:c", -3.0)] {
            let id = b.add_variable(VarKind::KeepLink, s);
            b.set_linear(id, c);
        }
        b.set_metadata(meta("right"));
        specs.push(Spec { file: "01-all-keep.json", problem: b.build(), description: "3 keep_link, all negative linear; optimum keeps all.", known_optimum: None });
    }

    // 02 — all drop: every merge costs (positive linear); optimum is empty.
    {
        let mut b = ProblemBuilder::new("golden-02-all-drop");
        for (s, c) in [("mem:a", 1.0), ("mem:b", 2.0), ("mem:c", 0.5)] {
            let id = b.add_variable(VarKind::Merge, s);
            b.set_linear(id, c);
        }
        b.set_metadata(meta("flat"));
        specs.push(Spec { file: "02-all-drop.json", problem: b.build(), description: "3 merge, all positive linear; optimum activates none (E=0).", known_optimum: None });
    }

    // 03 — repulsion pair: both attractive alone, but strong positive coupling.
    {
        let mut b = ProblemBuilder::new("golden-03-repulsion-pair");
        let x = b.add_variable(VarKind::Merge, "mem:a+mem:b");
        let y = b.add_variable(VarKind::Merge, "mem:c+mem:d");
        b.set_linear(x, -2.0);
        b.set_linear(y, -2.0);
        b.add_quadratic(x, y, 5.0);
        b.set_metadata(meta("right"));
        specs.push(Spec { file: "03-repulsion-pair.json", problem: b.build(), description: "2 merge, negative linear, strong positive coupling; optimum takes exactly one.", known_optimum: None });
    }

    // 04 — attraction pair: negative coupling rewards taking both.
    {
        let mut b = ProblemBuilder::new("golden-04-attraction-pair");
        let x = b.add_variable(VarKind::Strengthen, "skip:a");
        let y = b.add_variable(VarKind::Strengthen, "skip:b");
        b.set_linear(x, -1.0);
        b.set_linear(y, -1.0);
        b.add_quadratic(x, y, -3.0);
        b.set_metadata(meta("right"));
        specs.push(Spec { file: "04-attraction-pair.json", problem: b.build(), description: "2 strengthen, negative linear + negative coupling; optimum takes both.", known_optimum: None });
    }

    // 05 — budget at most 1 of 3 (penalty-folded).
    {
        let mut b = ProblemBuilder::new("golden-05-budget-at-most-1");
        let v: Vec<usize> = [("mem:a", -3.0), ("mem:b", -2.0), ("mem:c", -1.0)]
            .into_iter()
            .map(|(s, c)| {
                let id = b.add_variable(VarKind::Merge, s);
                b.set_linear(id, c);
                id
            })
            .collect();
        b.add_budget("budget", v, 1, 10.0);
        b.set_metadata(meta("flat"));
        specs.push(Spec { file: "05-budget-at-most-1.json", problem: b.build(), description: "3 merge, budget max_active=1 (folded); optimum keeps only the strongest.", known_optimum: None });
    }

    // 06 — budget at most 2 of 4 (penalty-folded).
    {
        let mut b = ProblemBuilder::new("golden-06-budget-at-most-2");
        let v: Vec<usize> = [("mem:a", -4.0), ("mem:b", -3.0), ("mem:c", -2.0), ("mem:d", -1.0)]
            .into_iter()
            .map(|(s, c)| {
                let id = b.add_variable(VarKind::Merge, s);
                b.set_linear(id, c);
                id
            })
            .collect();
        b.add_budget("budget", v, 2, 10.0);
        b.set_metadata(meta("right"));
        specs.push(Spec { file: "06-budget-at-most-2.json", problem: b.build(), description: "4 merge, budget max_active=2 (folded); optimum keeps the strongest two.", known_optimum: None });
    }

    // 07 — mixed signs + a couple of interactions.
    {
        let mut b = ProblemBuilder::new("golden-07-mixed");
        let ids: Vec<usize> = ["keep_link", "merge", "keep_link", "merge", "strengthen"]
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let kind = match *k {
                    "keep_link" => VarKind::KeepLink,
                    "merge" => VarKind::Merge,
                    _ => VarKind::Strengthen,
                };
                b.add_variable(kind, format!("v{i}"))
            })
            .collect();
        for (id, c) in ids.iter().zip([-2.0, 1.0, -1.0, 3.0, -0.5]) {
            b.set_linear(*id, c);
        }
        b.add_quadratic(ids[0], ids[2], 1.5);
        b.add_quadratic(ids[2], ids[4], -2.0);
        b.set_metadata(meta("right"));
        specs.push(Spec { file: "07-mixed.json", problem: b.build(), description: "5 mixed-kind vars, mixed-sign linear + two couplings; brute-force optimum.", known_optimum: None });
    }

    // 08 — frustrated triangle: each attractive, every pair repels.
    {
        let mut b = ProblemBuilder::new("golden-08-frustrated-triangle");
        let v: Vec<usize> = (0..3).map(|i| {
            let id = b.add_variable(VarKind::Merge, format!("mem:{i}"));
            b.set_linear(id, -1.0);
            id
        }).collect();
        b.add_quadratic(v[0], v[1], 1.5);
        b.add_quadratic(v[0], v[2], 1.5);
        b.add_quadratic(v[1], v[2], 1.5);
        b.set_metadata(meta("flat"));
        specs.push(Spec { file: "08-frustrated-triangle.json", problem: b.build(), description: "3 merge, negative linear, all pairs repel; optimum takes exactly one.", known_optimum: None });
    }

    // 09 — keep_link/strengthen coupling: strengthening only pays if the link is kept.
    {
        let mut b = ProblemBuilder::new("golden-09-link-strengthen");
        let keep_a = b.add_variable(VarKind::KeepLink, "skip:A");
        let str_a = b.add_variable(VarKind::Strengthen, "skip:A");
        let keep_b = b.add_variable(VarKind::KeepLink, "skip:B");
        let merge = b.add_variable(VarKind::Merge, "mem:x+mem:y");
        b.set_linear(keep_a, -1.0);
        b.set_linear(str_a, 0.5); // strengthening alone is a small cost…
        b.set_linear(keep_b, -0.5);
        b.set_linear(merge, -1.0);
        b.add_quadratic(keep_a, str_a, -2.0); // …but a big win once the link is kept.
        b.set_metadata(meta("right"));
        specs.push(Spec { file: "09-link-strengthen.json", problem: b.build(), description: "keep_link/strengthen coupling; optimum keeps+strengthens A, keeps B, merges.", known_optimum: None });
    }

    // 10 — single variable.
    {
        let mut b = ProblemBuilder::new("golden-10-single");
        let id = b.add_variable(VarKind::KeepLink, "skip:only");
        b.set_linear(id, -1.0);
        b.set_metadata(meta("flat"));
        specs.push(Spec { file: "10-single.json", problem: b.build(), description: "1 keep_link, negative linear; optimum keeps it.", known_optimum: None });
    }

    // 11 — large independent (24 vars, no coupling): per-variable optimum, known.
    {
        let mut b = ProblemBuilder::new("golden-11-large-independent");
        let mut opt = Vec::new();
        for i in 0..24usize {
            let c = if i % 3 == 0 { 0.7 } else { -(1.0 + (i % 4) as f64 * 0.3) };
            let id = b.add_variable(VarKind::Merge, format!("mem:{i}"));
            b.set_linear(id, c);
            opt.push(c < 0.0); // independent ⇒ take iff its linear is negative
        }
        b.set_metadata(meta("flat"));
        specs.push(Spec { file: "11-large-independent.json", problem: b.build(), description: "24 independent vars, mixed signs; optimum = each var iff its linear<0.", known_optimum: Some(opt) });
    }

    // 12 — large ferromagnet (20 vars, all-negative linear + chain coupling): all-true.
    {
        let mut b = ProblemBuilder::new("golden-12-large-ferromagnet");
        let v: Vec<usize> = (0..20).map(|i| {
            let id = b.add_variable(VarKind::Merge, format!("mem:{i}"));
            b.set_linear(id, -0.5);
            id
        }).collect();
        for i in 0..v.len() - 1 {
            b.add_quadratic(v[i], v[i + 1], -1.0);
        }
        b.set_metadata(meta("right"));
        specs.push(Spec { file: "12-large-ferromagnet.json", problem: b.build(), description: "20 vars, all-negative linear + negative chain coupling; optimum is all-true.", known_optimum: Some(vec![true; 20]) });
    }

    specs
}

fn brute_force(p: &ConsolidationProblem) -> (Vec<bool>, f64) {
    let n = p.num_vars();
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

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("qubo");
    std::fs::create_dir_all(&dir).expect("create tests/qubo");

    let mut entries = Vec::new();
    for spec in corpus() {
        let n = spec.problem.num_vars();
        spec.problem.validate().unwrap_or_else(|e| panic!("{} invalid: {e}", spec.file));
        let exact = n < 20;
        let (assignment, energy) = match &spec.known_optimum {
            Some(opt) => (opt.clone(), spec.problem.energy(opt)),
            None => brute_force(&spec.problem),
        };
        let json = serde_json::to_string_pretty(&spec.problem).unwrap();
        std::fs::write(dir.join(spec.file), json + "\n").unwrap();
        entries.push(ManifestEntry {
            file: spec.file.to_string(),
            num_vars: n,
            exact_solvable: exact,
            optimum_energy: energy,
            optimum_assignment: assignment,
            description: spec.description.to_string(),
        });
        println!("wrote {} ({n} vars, exact={exact}, opt={:.3})", spec.file, entries.last().unwrap().optimum_energy);
    }

    let manifest = Manifest {
        format: "kannaka-qubo-golden/1".to_string(),
        note: "Golden ADR-0038 QUBOs. Regenerate with `cargo run --example gen_qubo_corpus`. optimum_energy is over ConsolidationProblem::energy (penalties already folded); exact_solvable=true means <20 vars (brute-forceable in tests).".to_string(),
        files: entries,
    };
    std::fs::write(dir.join("manifest.json"), serde_json::to_string_pretty(&manifest).unwrap() + "\n").unwrap();
    println!("wrote manifest.json with {} entries", manifest.files.len());
}
