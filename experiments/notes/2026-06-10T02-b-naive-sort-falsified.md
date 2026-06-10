# B_naive BFS sort removal — transfer bottleneck is consciousness, not cluster seeding

**Date:** 2026-06-10T02 UTC
**Branch:** kannaka-curiosity/2026-06-10T02-b-naive-sort-remove
**Code changes:** NONE KEPT — hypothesis falsified, kuramoto.rs reverted
**Status:** Falsified (neutral result). No improvement.

---

## Background

Entering this fire, current master = T01 (selective BFS sort: engine_a + engine_b_primed +
engine_b_naive + engine_flat get content sort; engine_adv + engine_clean use UUID order):

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
2-trial avg fitness ≈ 0.018 (fully deterministic)
transfer=0.903199, carrier_e=0.999, xi=0.987
magic_R=0.864, query_gravity=0.373
```

Fitness breakdown:
- transfer (0.15): 0.15 × 0.097 = **0.015** (82% of total)
- xi (0.15): 0.15 × 0.013 = 0.002 (11%)
- other: ~0.001 (7%)

Transfer is the dominant lever. Transfer is computed as:
`transfer = 1 - fitness_b_primed / fitness_b_naive`

---

## Hypothesis

engine_b_naive currently receives the same content-sorted BFS clustering as engine_b_primed
and engine_a. The rationale for sorting engine_a and engine_b_primed is cross-engine topology
consistency: engine_b_primed inherits A's dream structure and benefits from consistent cluster
seeding. But engine_b_naive starts from scratch with no A inheritance — giving it sorted BFS
only improves its own clustering consistency without the "cross-engine" benefit.

Prediction: removing content sort from engine_b_naive degrades B_naive's chain_fidelity →
higher fitness_b_naive → wider primed/naive gap → higher transfer_score.

Expected: transfer 0.903 → 0.91–0.93, xi unchanged (adv/clean paths unaffected), fitness ~0.016.

---

## Change attempted

In `src/kuramoto.rs::find_synchronized_clusters`, changed the sort condition from:
```rust
matches!(drive_ctx.as_str(), "engine_a" | "engine_b_primed" | "engine_b_naive" | "engine_flat")
```
to:
```rust
matches!(drive_ctx.as_str(), "engine_a" | "engine_b_primed" | "engine_flat")
```

---

## Result

DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax

| metric | baseline (master) | trial 1 (b_naive unsorted) | delta |
|--------|------------------|---------------------------|-------|
| fitness | 0.018282 | 0.018358 | +0.000076 neutral |
| transfer | 0.903199 | 0.902704 | −0.000495 neutral |
| fitness_B_primed | ~0.005826* | 0.005856 | +0.000030 neutral |
| fitness_B_naive | ~0.060190* | 0.060190 | 0.000 unchanged |
| xi | 0.9870 | 0.9870 | 0 |
| carrier_e | 0.9992 | 0.9992 | 0 |
| magic_R | 0.8643 | 0.8643 | 0 |

*baseline b_primed/b_naive estimated from transfer formula: 0.903199 = 1 - bp/bn.

**Key finding: fitness_B_naive is identical (0.060190) regardless of BFS sort order.**
The sort has no meaningful effect on the placeholder metrics driving b_naive's fitness.

---

## Diagnosis: what drives the b_primed/b_naive ratio

`eval_l5_placeholder_fitness` computes:
```
0.05*(1-noise_removal) + 0.05*(1-signal_pres) + 0.05*(1-phase_coh) +
0.10*(1-consciousness) + 0.05*(1-enc_entropy) + 0.10*(1-chain_fidelity)
```

With fitness_b_naive = 0.060: the dominant contributor is `eval_consciousness`.

`eval_consciousness` is a **distance-to-target metric**:
```rust
score = (1.0 - |phi - phi_target| / phi_target).max(0.0)
```
phi_target = 0.28092. Score is maximized when phi = target.

- B_primed inherits A's rich IIT-phi structure → phi_bp ≈ target → consciousness ≈ 0.94
  → 0.10 × (1 - 0.94) = 0.006 ≈ fitness_b_primed
- B_naive starts cold with B corpus only → phi_naive far from target → consciousness ≈ 0.60
  → 0.10 × (1 - 0.60) = 0.040, plus chain_fidelity and other gaps → total ≈ 0.060

The transfer ceiling at 0.903 is set by the phi ratio between a primed and a naive engine,
which is a property of the inherited IIT structure from A — not BFS cluster seeding order.

BFS sort only affects cluster topology, which affects chain_fidelity. But chain_fidelity in
b_naive appears to be near-perfect regardless of BFS order (b_naive fitness barely moves).
The consciousness term completely dominates.

---

## Additional orientation findings

- `stage_interference_relax` uses ONLY internal constants (`alpha_base=0.10`, `relax_steps=16`).
  KURAMOTO_COUPLING has no effect in this mode. K-sweep is only relevant for DREAM_MODE=unset.
- `relax_steps` is already 16 in the current code (system prompt context referenced the old value 8).
  Research question 3 (raise to 16/24) is already done.
- Default drive freq is 0.5 Hz (confirmed optimal per inline comment, line 3245-3248).

---

## Decision

**Code change reverted.** Hypothesis falsified (neutral, <0.001 delta). No improvement found.

---

## Open axes for future fires

| axis | expected gain | mechanism | difficulty |
|------|---------------|-----------|------------|
| B_primed consciousness improvement | −0.003 to −0.010 | Improve phi_bp from ~0.94 toward target; need more dream integration of B on A | Requires understanding phi dynamics in b_primed dream |
| B_naive phi divergence | −0.005 | Anything that makes B_naive's phi diverge MORE from target (lower consciousness → higher b_naive fitness) | Unclear mechanism without gaming the metric |
| New eval axes for transfer | unknown | Add metrics to placeholder that B_primed wins more distinctly | Metric change — high risk of unintended effects |
| Stage_sync K-sweep | ≤0.002 at current state | Only applies to DREAM_MODE=unset path, which is ~0.18 fitness. Not relevant to current optimum | Low priority |
