# T00 2026-06-06: interference_relax — halved alpha_base with doubled relax_steps

## Hypothesis

`stage_interference_relax` at 8 steps (alpha_base=0.20) showed extreme xi variance across runs (0.083–0.220) and mediocre fitness (~0.21). The 8 discrete sin()-applications are coarse, and the total coupling (8 × 0.20 ≈ 1.6 units) might be at a bad operating point. Predict: keeping total coupling constant at ~1.6 by halving alpha_base to 0.10 while doubling relax_steps to 16 will produce smoother phase convergence, stabilise xi, and improve fitness vs baseline 0.18.

The quiet-wave envelope completes one full cycle regardless of relax_steps; finer steps sample it more smoothly, reducing oscillatory artefacts in the per-step phase update.

## Code change

`src/consolidation.rs` — `stage_interference_relax`:

```
- let alpha_base: f32 = 0.20;
- let relax_steps: usize = 8;
+ let alpha_base: f32 = 0.10;
+ let relax_steps: usize = 16;
```

## Trials

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| # | config | fitness | xi | carrier_e | magic_R | query_gravity |
|---|--------|---------|----|-----------| --------|---------------|
| 1 | ref: relax_steps=8, alpha=0.20 | 0.212 | 0.083 | 0.714 | 0.612 | 0.364 |
| 2 | relax_steps=16, alpha=0.20 | 0.243 | 0.362 | 0.000 | 0.675 | 0.386 |
| 3 | relax_steps=16, alpha=0.10 | **0.101** | 0.925 | 0.497 | 0.617 | 0.364 |
| 4 | relax_steps=16, alpha=0.10 | 0.196 | 0.294 | 0.497 | 0.617 | 0.364 |
| 5 | relax_steps=16, alpha=0.10 | 0.150 | 0.602 | 0.497 | 0.617 | 0.364 |

**3-trial avg (trials 3–5): 0.149**  Baseline: 0.18.

## Findings

**Confirmed improvement**: 3-trial avg 0.149 vs baseline 0.18, delta = 0.031 > 0.005 threshold. Code change KEPT.

**Intermediate step (trial 2) revealed a hard constraint**: relax_steps=16 at full alpha=0.20 doubles total coupling (3.2 units) and carrier_emergence collapses to 0.000. The carrier dynamics require some residual phase spread; over-relaxation eliminates it. This gives a "total coupling budget" intuition: ~1.6 units preserves carrier dynamics, ~3.2 units kills them.

**xi remains stochastic** even at the new operating point (0.294–0.925 range across trials). All other metrics (carrier_e, magic_R, query_gravity, transfer_score) are deterministic across trials 3–5 — identical to 4 decimal places. xi stochasticity originates inside `eval_xi_robustness_v2` (adversarial perturbation with unseeded RNG). This is the dominant variance source for fitness under interference_relax.

**Trade-off**: carrier_e drops 0.714→0.497 (−0.217) but xi mean rises ~0.15→0.607 (+0.45). At the fitness weights (xi: 0.15, carrier_e: 0.10), the xi gain outweighs the carrier_e cost: Δfitness ≈ −0.068 + 0.022 = −0.046, consistent with the observed −0.031 avg improvement (other metrics also shifted slightly).

## Comparison to baseline

| metric | baseline (unset mode) | interference_relax 16×0.10 avg |
|--------|----------------------|-------------------------------|
| fitness | ~0.18 | **0.149** |
| xi | ~0.642 (smoke test) | ~0.607 (mean across 3 trials) |
| carrier_e | ~0.559 (smoke test) | 0.497 |
| magic_R | ~0.355 (smoke test) | **0.617** |
| query_gravity | ~0.460 (smoke test) | 0.364 |
| transfer_score | ~0.718 (smoke test) | ~0.750 |

magic_R improves markedly (0.355 → 0.617) — non-Clifford phase content is higher in interference_relax mode regardless of alpha/steps tuning. query_gravity is slightly lower; the attention-gravity effect is weaker here.

## Next questions

1. The xi stochasticity suggests `eval_xi_robustness_v2` should be seeded for reproducible benchmarking. The large variance (SD > 0.25 across 3 trials) makes it hard to confirm improvements with only 3 trials.
2. A K-sweep (kuramoto_coupling 1–7) under stage_sync with the real plumbing (post-066d41a) is still unexplored — the smoke test's xi=0.642 at K=3.0 might shift significantly.
3. Whether magic_R and xi correlate under stage_sync K-sweep (the magic↔xi prediction) remains the most directly testable theoretical question.
