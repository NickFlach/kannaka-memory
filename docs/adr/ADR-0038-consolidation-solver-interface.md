# ADR-0038 — Consolidation as QUBO: a solver interface for the dream phase

- Status: Proposed
- Date: 2026-07-01
- Repo: `kannaka-memory` (interface + classical default); `kannaka-quantum` (alternate backend)
- Related: ADR-0021 (Chiral Mirror), ADR-0024 (chiral semantics), commit 917bb70 (decay fix)

## Context

The dream/consolidation phase decides which skip links to strengthen, which
resonances to merge, and which phantom entanglements to retain under a decay
and capacity budget. Today this is procedural Rust inside the engine. But the
decision itself is a combinatorial optimization: binary keep/merge/strengthen
variables, pairwise interaction terms (co-resonance, phase alignment, cluster
overlap), and a global budget — exactly the QUBO / Ising problem class.

Quantum annealers and QAOA target this class. Today's hardware will not beat
our classical code at our problem sizes, and this ADR does **not** propose
running consolidation on a QPU now. It proposes the opposite discipline:
**freeze the problem at a serialization boundary so the solver is swappable**,
classical today, quantum (or better classical — simulated annealing, tabu, SA
on GPU) whenever one crosses the threshold. Durability via contract.

## Decision

### 1. Problem serialization: `ConsolidationProblem`

The engine emits consolidation decisions as a QUBO in a versioned JSON format:

```json
{
  "format": "kannaka-qubo/1",
  "problem_id": "dream-2026-07-01T03:12:00Z",
  "variables": [
    {"id": 0, "kind": "keep_link",   "subject": "skip:9f2c…"},
    {"id": 1, "kind": "merge",       "subject": "mem:a41b…+mem:77e0…"},
    {"id": 2, "kind": "strengthen",  "subject": "skip:c3d9…"}
  ],
  "linear":    {"0": -1.7, "1": 0.4, "2": -0.9},
  "quadratic": {"0,2": -0.6, "1,2": 1.1},
  "constraints": {"budget": {"vars": [0, 1, 2], "max_active": 2, "penalty": 8.0}},
  "metadata": {
    "hemisphere": "right",
    "phi_at_emit": 0.18,
    "entropy_provenance": "reservoir://iqm-garnet/2026-06-28"
  }
}
```

Conventions:

- **Minimization.** Solvers minimize `xᵀQx`; negative linear terms favor keeping.
- Constraints are expressed twice: structurally (so smart solvers can use them)
  and as penalty terms folded into Q (so a dumb solver that ignores
  `constraints` still gets a valid, if softer, problem).
- `metadata` is opaque to solvers. It carries chiral context and entropy
  provenance (ADR-0038 pairs with the Ξ entropy-reservoir work) for the audit
  trail, not for solving.

### 2. Solver trait

```rust
pub trait ConsolidationSolver {
    fn name(&self) -> &str;
    fn solve(&self, problem: &ConsolidationProblem, budget: SolveBudget)
        -> Result<ConsolidationSolution, SolveError>;
}

pub struct SolveBudget {
    pub wall_time: Duration,
    pub max_cost_usd: f64,   // 0.0 for classical solvers
}

pub struct ConsolidationSolution {
    pub assignment: Vec<bool>,
    pub energy: f64,
    pub solver: String,          // provenance
    pub exact: bool,             // exhaustive vs heuristic
    pub samples: Option<Vec<(Vec<bool>, f64, u32)>>, // for sampling solvers
}
```

The engine treats the returned assignment as *advisory*: it re-scores the
solution against its own objective before applying, so a buggy or adversarial
solver can degrade quality but never corrupt semantics. This is the same
posture as the spend guards in `kannaka-quantum` — the boundary assumes the
other side can misbehave.

### 3. Default implementation: `ClassicalAnneal`

Ships in `kannaka-memory`: simulated annealing with restarts, deterministic
under a seeded RNG (seed drawn from the `EntropySource` trait, so even the
classical solver's stochasticity carries quantum provenance once the
reservoir lands). Exhaustive solve below ~20 variables (`exact: true`).

### 4. Quantum backend: `kannaka-quantum` as a subprocess solver

`kannaka-quantum` gains a `qubo` subcommand mirroring the existing JSON-CLI
pattern (`recall`, `qrng`): read `kannaka-qubo/1` on stdin, emit a
`ConsolidationSolution` JSON on stdout. Implementation: QAOA via Qiskit on the
free simulator by default; real hardware only under the existing spend-guard
regime (`--allow-spend`, `--max-credits`, per-minute devices refused). The
Rust side wraps this as `SubprocessSolver`, honoring `SolveBudget.max_cost_usd`
by passing it through as the credit cap.

## Consequences

**Positive.** Consolidation logic becomes testable in isolation (golden QUBO
files + expected solutions in CI); solver competition becomes a benchmark, not
a rewrite; the quantum path exists end-to-end at $0 (simulator) from day one;
when annealing hardware matures, adoption is a config change.

**Negative / accepted.** Serialization adds a copy per dream (negligible —
dreams are not hot-path); expressing every consolidation rule as Q terms will
lag the procedural code at first, so the engine keeps a `procedural` solver
option until QUBO parity is validated; QAOA on simulator is slower than
`ClassicalAnneal` and exists for correspondence, not speed.

**Rejected alternatives.** (a) Direct qiskit/annealer calls from Rust — couples
the engine to Python and to provider SDKs; the subprocess JSON boundary matches
the fleet's existing architecture (engine binary ⇄ thin bridges). (b) Waiting
for hardware — the interface is the durable asset and costs little now; the
hardware timing is exactly what we're insulating against.

## Validation

1. Golden-file suite: 12 hand-built QUBOs with known optima; every solver must
   find optimum on exact-solvable sizes and ≥95% of optimal energy on larger.
2. Dogfood: run one week of real dreams through both `procedural` and
   `ClassicalAnneal`, diff applied consolidations, review divergences.
3. Correspondence: QAOA-on-simulator agreement rate tracked in the same
   benchmark corpus as `resonance_recall` (see Track 2 of the quantum wave).
