//! `kannaka-qubo/1` — the ConsolidationProblem serialization boundary (ADR-0038).
//!
//! A problem is a QUBO: binary variables (keep_link / merge / strengthen),
//! `linear` (diagonal) and `quadratic` (pairwise) coefficients, named
//! `constraints`, and opaque `metadata`. Solvers **minimize** `energy(x)`.
//!
//! Constraints are carried **twice** (ADR-0038 §Decision.1): structurally in
//! `constraints` (so a smart solver can enforce them exactly) AND penalty-folded
//! into `linear`/`quadratic` (so a constraint-blind solver still gets a valid,
//! if softer, problem). [`ProblemBuilder`] does the fold; [`ConsolidationProblem::energy`]
//! reads only the folded `linear`/`quadratic`, which is the single objective
//! every solver and every golden optimum is scored against.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Format tag for version 1 of the QUBO boundary.
pub const FORMAT: &str = "kannaka-qubo/1";

/// The consolidation decision a variable represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VarKind {
    /// Retain a skip link that decay would otherwise drop.
    KeepLink,
    /// Merge a redundant, phase-locked resonance group into one carrier.
    Merge,
    /// Strengthen (up-weight) a skip link.
    Strengthen,
}

/// One binary decision variable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    pub id: usize,
    pub kind: VarKind,
    /// Human/audit subject, e.g. `"skip:9f2c…"` or `"mem:a41b…+mem:77e0…"`.
    pub subject: String,
}

/// A cardinality (budget) constraint: at most `max_active` of `vars` may be 1.
/// Also folded into `linear`/`quadratic` with weight `penalty` (see module docs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    pub vars: Vec<usize>,
    pub max_active: usize,
    pub penalty: f64,
}

/// Opaque-to-solvers audit context (ADR-0038 §Decision.1). Carries chiral
/// context + entropy provenance; never read while solving.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hemisphere: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phi_at_emit: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entropy_provenance: Option<String>,
    /// Any further metadata keys, preserved across a round-trip.
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_json::Value>,
}

/// A consolidation QUBO in the `kannaka-qubo/1` format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsolidationProblem {
    pub format: String,
    pub problem_id: String,
    pub variables: Vec<Variable>,
    /// Diagonal coefficients keyed by variable id (JSON object keys are strings).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub linear: BTreeMap<String, f64>,
    /// Pairwise coefficients keyed `"i,j"` with `i < j`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub quadratic: BTreeMap<String, f64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub constraints: BTreeMap<String, Constraint>,
    #[serde(default)]
    pub metadata: Metadata,
}

fn parse_idx(k: &str) -> Option<usize> {
    k.trim().parse().ok()
}

fn parse_pair(k: &str) -> Option<(usize, usize)> {
    let (a, b) = k.split_once(',')?;
    Some((parse_idx(a)?, parse_idx(b)?))
}

impl ConsolidationProblem {
    /// Number of decision variables.
    pub fn num_vars(&self) -> usize {
        self.variables.len()
    }

    /// Objective energy of an assignment: `Σ linear[i]·x_i + Σ quadratic[i,j]·x_i·x_j`.
    ///
    /// This reads only the folded `linear`/`quadratic` (penalties already
    /// included by the builder), so it is the exact objective solvers minimize
    /// and golden optima are documented against. Out-of-range keys are ignored.
    pub fn energy(&self, x: &[bool]) -> f64 {
        let mut e = 0.0;
        for (k, &c) in &self.linear {
            if let Some(i) = parse_idx(k) {
                if x.get(i).copied().unwrap_or(false) {
                    e += c;
                }
            }
        }
        for (k, &c) in &self.quadratic {
            if let Some((i, j)) = parse_pair(k) {
                if x.get(i).copied().unwrap_or(false) && x.get(j).copied().unwrap_or(false) {
                    e += c;
                }
            }
        }
        e
    }

    /// Cheap structural sanity check: right format tag, contiguous `0..n` ids,
    /// and every `linear`/`quadratic`/`constraint` index in range.
    pub fn validate(&self) -> Result<(), String> {
        if self.format != FORMAT {
            return Err(format!("format `{}` != `{FORMAT}`", self.format));
        }
        let n = self.variables.len();
        for (want, v) in self.variables.iter().enumerate() {
            if v.id != want {
                return Err(format!("variable ids must be contiguous 0..n; got {} at slot {want}", v.id));
            }
        }
        for k in self.linear.keys() {
            match parse_idx(k) {
                Some(i) if i < n => {}
                _ => return Err(format!("linear key `{k}` out of range")),
            }
        }
        for k in self.quadratic.keys() {
            match parse_pair(k) {
                Some((i, j)) if i < j && j < n => {}
                _ => return Err(format!("quadratic key `{k}` invalid (need i<j, in range)")),
            }
        }
        for (name, c) in &self.constraints {
            for &i in &c.vars {
                if i >= n {
                    return Err(format!("constraint `{name}` references var {i} >= {n}"));
                }
            }
        }
        Ok(())
    }
}

/// Builds a [`ConsolidationProblem`], folding each constraint's penalty into the
/// QUBO on [`ProblemBuilder::build`].
#[derive(Debug, Default)]
pub struct ProblemBuilder {
    problem_id: String,
    variables: Vec<Variable>,
    linear: BTreeMap<usize, f64>,
    quadratic: BTreeMap<(usize, usize), f64>,
    constraints: BTreeMap<String, Constraint>,
    metadata: Metadata,
}

impl ProblemBuilder {
    pub fn new(problem_id: impl Into<String>) -> Self {
        Self { problem_id: problem_id.into(), ..Default::default() }
    }

    /// Add a variable, returning its id.
    pub fn add_variable(&mut self, kind: VarKind, subject: impl Into<String>) -> usize {
        let id = self.variables.len();
        self.variables.push(Variable { id, kind, subject: subject.into() });
        id
    }

    /// Set (overwrite) a variable's linear coefficient.
    pub fn set_linear(&mut self, id: usize, c: f64) -> &mut Self {
        self.linear.insert(id, c);
        self
    }

    /// Add to a variable's linear coefficient.
    pub fn add_linear(&mut self, id: usize, c: f64) -> &mut Self {
        *self.linear.entry(id).or_insert(0.0) += c;
        self
    }

    /// Add a quadratic coefficient. `i==j` folds into the linear term
    /// (`x_i² = x_i` for binary x); otherwise stored canonically with `i < j`.
    pub fn add_quadratic(&mut self, i: usize, j: usize, c: f64) -> &mut Self {
        if i == j {
            *self.linear.entry(i).or_insert(0.0) += c;
        } else {
            let key = if i < j { (i, j) } else { (j, i) };
            *self.quadratic.entry(key).or_insert(0.0) += c;
        }
        self
    }

    /// Add a budget constraint (at most `max_active` of `vars`). Recorded
    /// structurally and folded into the QUBO on `build`.
    pub fn add_budget(
        &mut self,
        name: impl Into<String>,
        mut vars: Vec<usize>,
        max_active: usize,
        penalty: f64,
    ) -> &mut Self {
        vars.sort_unstable();
        vars.dedup();
        self.constraints.insert(name.into(), Constraint { vars, max_active, penalty });
        self
    }

    pub fn set_metadata(&mut self, m: Metadata) -> &mut Self {
        self.metadata = m;
        self
    }

    /// Fold constraints into the QUBO and emit the problem.
    ///
    /// Fold: `P·(Σ_{i∈S} x_i − K)² = P·Σ x_i(1−2K) + 2P·Σ_{i<j} x_i x_j + P·K²`.
    /// The additive `P·K²` constant is dropped — it shifts every energy equally
    /// and so can't change the argmin; golden optima are documented on this
    /// constant-free objective.
    pub fn build(mut self) -> ConsolidationProblem {
        for c in self.constraints.values() {
            let k = c.max_active as f64;
            for &i in &c.vars {
                *self.linear.entry(i).or_insert(0.0) += c.penalty * (1.0 - 2.0 * k);
            }
            for a in 0..c.vars.len() {
                for b in (a + 1)..c.vars.len() {
                    *self.quadratic.entry((c.vars[a], c.vars[b])).or_insert(0.0) += 2.0 * c.penalty;
                }
            }
        }
        let linear = self
            .linear
            .into_iter()
            .filter(|(_, v)| *v != 0.0)
            .map(|(i, v)| (i.to_string(), v))
            .collect();
        let quadratic = self
            .quadratic
            .into_iter()
            .filter(|(_, v)| *v != 0.0)
            .map(|((i, j), v)| (format!("{i},{j}"), v))
            .collect();
        ConsolidationProblem {
            format: FORMAT.to_string(),
            problem_id: self.problem_id,
            variables: self.variables,
            linear,
            quadratic,
            constraints: self.constraints,
            metadata: self.metadata,
        }
    }
}

/// A merge decision the dream would consider: a subject label and a `cohesion`
/// score (≥ 0; larger = stronger case to merge). Emitted as a `merge` variable
/// with linear `-cohesion`, so keeping the merge (x=1) lowers energy.
#[derive(Debug, Clone)]
pub struct MergeCandidate {
    pub subject: String,
    pub cohesion: f64,
}

/// Emit a `kannaka-qubo/1` problem from a dream's merge plan (the T3.2 emitter).
///
/// One `merge` variable per candidate; an optional `budget` constraint capping
/// how many merges a single pass may keep (structural + penalty-folded). This is
/// deliberately advisory and side-effect-free — the engine re-scores before any
/// apply (T3.5), and the procedural consolidation path is untouched.
pub fn dream_merge_problem(
    problem_id: impl Into<String>,
    merges: &[MergeCandidate],
    budget: Option<(usize, f64)>,
    metadata: Metadata,
) -> ConsolidationProblem {
    let mut b = ProblemBuilder::new(problem_id);
    let mut ids = Vec::with_capacity(merges.len());
    for m in merges {
        let id = b.add_variable(VarKind::Merge, m.subject.clone());
        b.set_linear(id, -m.cohesion);
        ids.push(id);
    }
    b.set_metadata(metadata);
    if let (Some((max_active, penalty)), false) = (budget, ids.is_empty()) {
        b.add_budget("budget", ids, max_active, penalty);
    }
    b.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn energy_sums_linear_and_quadratic() {
        let mut b = ProblemBuilder::new("t");
        let a = b.add_variable(VarKind::Merge, "a");
        let c = b.add_variable(VarKind::Merge, "b");
        b.set_linear(a, -2.0);
        b.set_linear(c, -1.0);
        b.add_quadratic(a, c, 5.0);
        let p = b.build();
        assert_eq!(p.energy(&[false, false]), 0.0);
        assert_eq!(p.energy(&[true, false]), -2.0);
        assert_eq!(p.energy(&[false, true]), -1.0);
        assert_eq!(p.energy(&[true, true]), -2.0 - 1.0 + 5.0);
    }

    #[test]
    fn quadratic_diagonal_folds_into_linear() {
        let mut b = ProblemBuilder::new("t");
        let a = b.add_variable(VarKind::Merge, "a");
        b.add_quadratic(a, a, -3.0);
        let p = b.build();
        assert!(p.quadratic.is_empty());
        assert_eq!(p.energy(&[true]), -3.0);
    }

    #[test]
    fn budget_fold_makes_single_active_optimal() {
        // raw linear [-3,-2,-1], budget max_active=1, penalty=10.
        let mut b = ProblemBuilder::new("t");
        let v: Vec<usize> = (0..3).map(|_| b.add_variable(VarKind::Merge, "m")).collect();
        b.set_linear(v[0], -3.0);
        b.set_linear(v[1], -2.0);
        b.set_linear(v[2], -1.0);
        b.add_budget("budget", v.clone(), 1, 10.0);
        let p = b.build();
        // fold: linear += 10*(1-2) = -10 each; quadratic pairs += 20.
        assert_eq!(p.linear["0"], -13.0);
        assert_eq!(p.linear["1"], -12.0);
        assert_eq!(p.linear["2"], -11.0);
        assert_eq!(p.quadratic["0,1"], 20.0);
        // Single best var active is the global optimum.
        assert_eq!(p.energy(&[true, false, false]), -13.0);
        assert_eq!(p.energy(&[true, true, false]), -13.0 - 12.0 + 20.0);
        assert_eq!(p.energy(&[false, false, false]), 0.0);
        let (best, e) = brute_force(&p);
        assert_eq!(best, vec![true, false, false]);
        assert_eq!(e, -13.0);
        // Structural constraint is retained too.
        assert_eq!(p.constraints["budget"].max_active, 1);
    }

    #[test]
    fn json_round_trip_is_exact() {
        let mut b = ProblemBuilder::new("dream-2026-07-02T00:00:00Z");
        let a = b.add_variable(VarKind::KeepLink, "skip:9f2c");
        let c = b.add_variable(VarKind::Merge, "mem:a41b+mem:77e0");
        let d = b.add_variable(VarKind::Strengthen, "skip:c3d9");
        b.set_linear(a, -1.7);
        b.set_linear(c, 0.4);
        b.set_linear(d, -0.9);
        b.add_quadratic(a, d, -0.6);
        b.add_quadratic(c, d, 1.1);
        b.set_metadata(Metadata {
            hemisphere: Some("right".into()),
            phi_at_emit: Some(0.18),
            entropy_provenance: Some("reservoir://iqm-garnet/2026-06-28".into()),
            ..Default::default()
        });
        let p = b.build();
        let js = serde_json::to_string_pretty(&p).unwrap();
        let back: ConsolidationProblem = serde_json::from_str(&js).unwrap();
        assert_eq!(p, back);
        assert!(back.validate().is_ok());
        assert_eq!(back.format, FORMAT);
    }

    #[test]
    fn dream_merge_problem_emits_merge_vars_and_budget() {
        let merges = vec![
            MergeCandidate { subject: "mem:a+mem:b".into(), cohesion: 2.0 },
            MergeCandidate { subject: "mem:c+mem:d".into(), cohesion: 1.0 },
        ];
        let p = dream_merge_problem("dream-x", &merges, Some((1, 5.0)), Metadata::default());
        assert_eq!(p.num_vars(), 2);
        assert!(p.variables.iter().all(|v| v.kind == VarKind::Merge));
        assert!(p.constraints.contains_key("budget"));
        assert!(p.validate().is_ok());
    }

    /// Test-only exhaustive optimum (the shipped exact solver is T3.3).
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
}
