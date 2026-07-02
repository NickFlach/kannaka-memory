//! T3.3 — the `ConsolidationSolver` trait and the default `ClassicalAnneal`
//! (ADR-0038 §Decision.2-3).
//!
//! A solver takes a [`ConsolidationProblem`] and a [`SolveBudget`] and returns a
//! [`ConsolidationSolution`] the engine treats as **advisory** (it re-scores
//! before applying — T3.5). [`ClassicalAnneal`] is exhaustive below 20 variables
//! (`exact: true`) and otherwise simulated annealing with restarts, its RNG
//! seeded from the [`EntropySource`] trait (T1.3) so even the classical solver's
//! stochasticity carries entropy provenance, recorded in the solution.

use std::fmt;
use std::time::{Duration, Instant};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::entropy::{seed_from_bytes, EntropySource, PrngSource, Provenance};

use super::problem::ConsolidationProblem;

/// Below this many variables `ClassicalAnneal` solves exhaustively (`exact`).
pub const EXACT_THRESHOLD: usize = 20;

/// Wall-time / cost budget for a solve. `max_cost_usd` is 0.0 for classical
/// solvers; a subprocess/quantum solver (T3.4) passes it through as a credit cap.
#[derive(Debug, Clone, Copy)]
pub struct SolveBudget {
    pub wall_time: Duration,
    pub max_cost_usd: f64,
}

impl Default for SolveBudget {
    fn default() -> Self {
        Self { wall_time: Duration::from_secs(5), max_cost_usd: 0.0 }
    }
}

/// A solver's answer. `assignment` is advisory — the engine re-scores it against
/// its own objective before applying (ADR-0038 review decision 2).
#[derive(Debug, Clone)]
pub struct ConsolidationSolution {
    pub assignment: Vec<bool>,
    pub energy: f64,
    /// Provenance: which solver produced this.
    pub solver: String,
    /// Exhaustive (`true`) vs heuristic (`false`).
    pub exact: bool,
    /// For sampling solvers: `(assignment, energy, count)` per distinct sample.
    pub samples: Option<Vec<(Vec<bool>, f64, u32)>>,
    /// T3.3 (#479): provenance of the entropy that seeded the solve. `prng://…`
    /// for the software default; `reservoir://…` + a QPU chain once T1.5 lands.
    pub provenance: Provenance,
}

/// Why a solve failed.
#[derive(Debug, Clone)]
pub enum SolveError {
    /// The problem's `linear`/`quadratic` reference a variable id out of range,
    /// etc. (carries [`ConsolidationProblem::validate`]'s message).
    Invalid(String),
}

impl fmt::Display for SolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SolveError::Invalid(m) => write!(f, "invalid problem: {m}"),
        }
    }
}

impl std::error::Error for SolveError {}

/// The solver seam (ADR-0038 §Decision.2). `ClassicalAnneal` is native; a
/// quantum backend is wrapped as a subprocess solver (T3.4).
pub trait ConsolidationSolver {
    fn name(&self) -> &str;
    fn solve(
        &self,
        problem: &ConsolidationProblem,
        budget: SolveBudget,
    ) -> Result<ConsolidationSolution, SolveError>;
}

/// Simulated annealing with restarts; exhaustive below [`EXACT_THRESHOLD`].
///
/// The seed is drawn ONCE (at construction) from an [`EntropySource`] so
/// `solve` stays `&self` and reproducible: the same seed yields the same
/// assignment. The draw's [`Provenance`] is recorded into every solution.
#[derive(Debug, Clone)]
pub struct ClassicalAnneal {
    seed: u64,
    provenance: Provenance,
    restarts: u32,
    /// Sweeps (flip attempts = iters × n) per restart.
    iters_per_var: u32,
}

impl Default for ClassicalAnneal {
    /// Seeded from the software PRNG (`prng://`). Never fails.
    fn default() -> Self {
        Self::from_entropy(&mut PrngSource::new())
    }
}

impl ClassicalAnneal {
    /// Draw a 64-bit seed from `src`, recording its provenance. If the source
    /// errors (e.g. an empty reservoir), fall back to the software PRNG and
    /// record its (honest `prng://`) provenance rather than failing a dream.
    pub fn from_entropy(src: &mut dyn EntropySource) -> Self {
        let (seed, provenance) = match src.draw(64) {
            Ok(d) => (seed_from_bytes(&d.bytes), d.provenance),
            Err(_) => {
                let d = PrngSource::new().draw(64).expect("PrngSource never fails");
                (seed_from_bytes(&d.bytes), d.provenance)
            }
        };
        Self { seed, provenance, restarts: 32, iters_per_var: 400 }
    }

    /// Deterministic constructor for tests / reproduction: a fixed seed, with
    /// `prng://legacy` provenance (no entropy was drawn).
    pub fn with_seed(seed: u64) -> Self {
        Self { seed, provenance: Provenance::legacy(), restarts: 32, iters_per_var: 400 }
    }

    pub fn with_schedule(mut self, restarts: u32, iters_per_var: u32) -> Self {
        self.restarts = restarts.max(1);
        self.iters_per_var = iters_per_var.max(1);
        self
    }

    /// Exhaustive minimum over all 2ⁿ assignments (for n < [`EXACT_THRESHOLD`]).
    fn exhaustive(problem: &ConsolidationProblem) -> (Vec<bool>, f64) {
        let n = problem.num_vars();
        let mut best = vec![false; n];
        let mut best_e = f64::INFINITY;
        for mask in 0u64..(1u64 << n) {
            let x: Vec<bool> = (0..n).map(|i| (mask >> i) & 1 == 1).collect();
            let e = problem.energy(&x);
            if e < best_e {
                best_e = e;
                best = x;
            }
        }
        (best, best_e)
    }

    /// One annealing run from a random start; returns the best state seen.
    fn anneal_once(&self, problem: &ConsolidationProblem, rng: &mut ChaCha8Rng) -> (Vec<bool>, f64) {
        let n = problem.num_vars();
        let mut x: Vec<bool> = (0..n).map(|_| rng.gen_bool(0.5)).collect();
        let mut e = problem.energy(&x);
        let mut best = x.clone();
        let mut best_e = e;

        let (t0, t1) = (2.0_f64, 0.01_f64);
        let total = (self.iters_per_var as usize).saturating_mul(n).max(1);
        for it in 0..total {
            let frac = it as f64 / total as f64;
            let temp = t0 * (t1 / t0).powf(frac);
            let flip = rng.gen_range(0..n);
            x[flip] = !x[flip];
            let e2 = problem.energy(&x);
            let de = e2 - e;
            if de <= 0.0 || rng.gen::<f64>() < (-de / temp).exp() {
                e = e2;
                if e < best_e {
                    best_e = e;
                    best.copy_from_slice(&x);
                }
            } else {
                x[flip] = !x[flip]; // reject: revert the flip
            }
        }
        (best, best_e)
    }
}

impl ConsolidationSolver for ClassicalAnneal {
    fn name(&self) -> &str {
        "ClassicalAnneal"
    }

    fn solve(
        &self,
        problem: &ConsolidationProblem,
        budget: SolveBudget,
    ) -> Result<ConsolidationSolution, SolveError> {
        problem.validate().map_err(SolveError::Invalid)?;
        let n = problem.num_vars();

        // Empty problem: the empty assignment, energy 0, trivially exact.
        if n == 0 {
            return Ok(ConsolidationSolution {
                assignment: Vec::new(),
                energy: 0.0,
                solver: self.name().to_string(),
                exact: true,
                samples: None,
                provenance: self.provenance.clone(),
            });
        }

        if n < EXACT_THRESHOLD {
            let (assignment, energy) = Self::exhaustive(problem);
            return Ok(ConsolidationSolution {
                assignment,
                energy,
                solver: self.name().to_string(),
                exact: true,
                samples: None,
                provenance: self.provenance.clone(),
            });
        }

        // Heuristic: SA with restarts, deterministic in `self.seed`. Each restart
        // gets its own stream (seed ⊕ restart index) so restarts don't correlate.
        let start = Instant::now();
        let mut best: Vec<bool> = Vec::new();
        let mut best_e = f64::INFINITY;
        let mut samples: Vec<(Vec<bool>, f64, u32)> = Vec::new();
        for r in 0..self.restarts {
            let mut rng = ChaCha8Rng::seed_from_u64(self.seed ^ (r as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            let (x, e) = self.anneal_once(problem, &mut rng);
            samples.push((x.clone(), e, 1));
            if e < best_e {
                best_e = e;
                best = x;
            }
            // Respect the wall-time budget between restarts (best-effort).
            if start.elapsed() >= budget.wall_time {
                break;
            }
        }

        Ok(ConsolidationSolution {
            assignment: best,
            energy: best_e,
            solver: self.name().to_string(),
            exact: false,
            samples: Some(samples),
            provenance: self.provenance.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qubo::problem::{ProblemBuilder, VarKind};

    fn attraction_pair() -> ConsolidationProblem {
        // linear [-1,-1], quadratic(0,1)=-3 → optimum both-true, energy -5.
        let mut b = ProblemBuilder::new("t");
        let x = b.add_variable(VarKind::Merge, "a");
        let y = b.add_variable(VarKind::Merge, "b");
        b.set_linear(x, -1.0);
        b.set_linear(y, -1.0);
        b.add_quadratic(x, y, -3.0);
        b.build()
    }

    #[test]
    fn exact_below_threshold_finds_global_optimum() {
        let p = attraction_pair();
        let s = ClassicalAnneal::with_seed(1);
        let sol = s.solve(&p, SolveBudget::default()).unwrap();
        assert!(sol.exact);
        assert_eq!(sol.assignment, vec![true, true]);
        assert_eq!(sol.energy, -5.0);
        assert!(sol.samples.is_none());
    }

    #[test]
    fn empty_problem_is_exact_zero() {
        let p = ProblemBuilder::new("empty").build();
        let sol = ClassicalAnneal::default().solve(&p, SolveBudget::default()).unwrap();
        assert!(sol.exact);
        assert_eq!(sol.energy, 0.0);
        assert!(sol.assignment.is_empty());
    }

    #[test]
    fn anneal_solves_large_ferromagnet_to_optimum() {
        // 22 vars (> EXACT_THRESHOLD), all-negative linear + negative chain: the
        // optimum is all-true. SA must reach it (no frustration).
        let mut b = ProblemBuilder::new("ferro");
        let v: Vec<usize> = (0..22)
            .map(|i| {
                let id = b.add_variable(VarKind::Merge, format!("m{i}"));
                b.set_linear(id, -0.5);
                id
            })
            .collect();
        for i in 0..v.len() - 1 {
            b.add_quadratic(v[i], v[i + 1], -1.0);
        }
        let p = b.build();
        let opt = -0.5 * 22.0 + -1.0 * 21.0;
        let sol = ClassicalAnneal::with_seed(7).solve(&p, SolveBudget::default()).unwrap();
        assert!(!sol.exact, "22 > threshold ⇒ heuristic");
        assert!(sol.samples.is_some());
        assert!(
            (sol.energy - opt).abs() < 1e-9,
            "SA energy {} != optimum {opt}",
            sol.energy
        );
    }

    #[test]
    fn same_seed_is_deterministic() {
        let mut b = ProblemBuilder::new("ferro");
        for i in 0..24 {
            let id = b.add_variable(VarKind::Merge, format!("m{i}"));
            b.set_linear(id, -0.5);
            if i > 0 {
                b.add_quadratic(i - 1, i, -1.0);
            }
        }
        let p = b.build();
        let a = ClassicalAnneal::with_seed(42).solve(&p, SolveBudget::default()).unwrap();
        let c = ClassicalAnneal::with_seed(42).solve(&p, SolveBudget::default()).unwrap();
        assert_eq!(a.assignment, c.assignment);
        assert_eq!(a.energy, c.energy);
    }

    #[test]
    fn provenance_recorded_from_entropy_source() {
        let sol = ClassicalAnneal::from_entropy(&mut PrngSource::new())
            .solve(&attraction_pair(), SolveBudget::default())
            .unwrap();
        assert!(sol.provenance.is_prng(), "PRNG-seeded solve carries prng:// provenance");
        assert_eq!(sol.solver, "ClassicalAnneal");
    }
}
