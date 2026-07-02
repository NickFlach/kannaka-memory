# Golden QUBO corpus (ADR-0038 · T3.2)

12 hand-designed `kannaka-qubo/1` consolidation problems with **known optima**,
used to validate the format (T3.2) and every `ConsolidationSolver` (T3.3+).

- `NN-*.json` — the problems, in `kannaka-qubo/1` format.
- `manifest.json` — for each file: `num_vars`, `exact_solvable`, `optimum_energy`,
  an `optimum_assignment`, and a description. This is the documented optima.

Regenerate after an intentional change:

```
cargo run --example gen_qubo_corpus
```

## Conventions

- Solvers **minimize** `ConsolidationProblem::energy(x) = Σ linearᵢ·xᵢ + Σ quadraticᵢⱼ·xᵢ·xⱼ`.
- Budget constraints are carried **twice** (ADR-0038): structurally in
  `constraints`, and penalty-folded into `linear`/`quadratic` as
  `P·(Σx − K)²` (the additive `P·K²` constant is dropped — it can't change the
  argmin). `energy()` reads only the folded terms, so it already includes the
  soft penalty; the documented optima are over that objective.
- `exact_solvable = num_vars < 20` (brute-forceable in tests). The two large
  files (24, 20 vars) have optima known by construction (independent vars; and an
  all-negative ferromagnet whose optimum is all-true) for the ≥95%-of-optimal
  solver check.

## What's exercised

Trivial keep/drop, pairwise repulsion/attraction, `max_active` budgets (1-of-3,
2-of-4), mixed-sign objectives, a frustrated triangle, keep_link↔strengthen
coupling, a single var, and two ≥20-var problems.
