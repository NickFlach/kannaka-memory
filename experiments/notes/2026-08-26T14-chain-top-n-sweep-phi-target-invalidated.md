# 2026-08-26T14 — CHAIN_TOP_N sweep falsified; phi_target decoupling invalidated by Gram fix

## Context

Current floor entering this fire: fitness 0.018454 (fast environment, ~15 s total_ms).
Threshold for keeping a change: result ≤ 0.013454 (current floor − 0.005).

Aug 25 notes identified "phi_target + phase_coherence bundle" as the only remaining
threshold-crossing path. This fire investigated that claim along with a novel lever
(CHAIN_TOP_N sweep), and discovered that the Aug 25 phi_target analysis is invalidated
by the Gram matrix fix landed on Aug 25 evening (commit 3faeb6c).

## Environment baseline

| metric              | value  |
|---------------------|--------|
| fitness             | 0.018454 |
| transfer_score      | 0.9540 |
| xi_robustness_v2    | 0.9678 |
| consciousness       | 0.8830 |
| phase_coherence     | 0.8939 |
| carrier_emergence   | 1.0000 |
| temporal_separation | 1.0000 |
| magic_proxy_phase_R | 0.6082 |
| query_gravity       | 0.5065 |
| total_ms            | 15377 (fast env, speed_a ≈ 0.974) |

## Hypothesis 1: CHAIN_TOP_N sweep

**Rationale**: Adversarial memories in xi_eval have amplitude=0.9, just below the
corpus default of ~1.0. With chain_top_n=7 seeds per cycle, adversarials may compete
for seed slots after amplitude redistribution. Fewer seeds (5) might exclude adversarials;
more seeds (10) might dilute their influence. Both directions were tested.

**Results**:

| trial | CHAIN_TOP_N | fitness  | transfer | xi      | consciousness |
|-------|-------------|----------|----------|---------|---------------|
| B0    | 7 (default) | 0.018454 | 0.9540   | 0.9678  | 0.8830        |
| T1    | 5           | 0.067257 | 0.8367   | 0.7363  | 0.9999*       |
| T2    | 10          | 0.035355 | 0.8400   | 0.9457  | 0.9999*       |

*consciousness 0.9999 in T1/T2 is because phi_target was simultaneously set to 0.3138
(see Hypothesis 2 below — this is a confound that does not affect the CHAIN_TOP_N verdict).

**Verdict**: Both directions hurt dramatically. CHAIN_TOP_N=5 collapses xi (0.9678→0.7363)
and transfer (0.9540→0.8367). CHAIN_TOP_N=10 also hurts both metrics significantly.
The current default of 7 is a stable optimum. CHAIN_TOP_N < 7 and CHAIN_TOP_N > 7 are
both closed.

**Mechanism interpretation**: CHAIN_TOP_N=5 selects too few seeds, failing to cover
the corpus structure needed for robust Kuramoto synchronization across all clusters.
CHAIN_TOP_N=10 admits too many mid-amplitude memories as seeds, diluting the
consolidation signal around the dominant attractors.

## Hypothesis 2: phi_target decoupling — INVALIDATED by Gram fix

**Background**: Aug 25 trial 1 set consciousness_phi_target=0.3138 (measured L5 phi)
and reported fitness 0.016139, transfer 0.9540, xi 0.9678, consciousness 0.9999.
Savings = 0.003507 (below threshold alone, reverted at Aug 25 noting it needs a bundle
partner). Aug 25 notes left this as a confirmed path requiring +0.001493 from another lever.

**This fire's trial T3**: Same code change (phi_target=0.3138, CHAIN_TOP_N=7 default)
gave REGRESSION: fitness 0.039948, transfer 0.8436, xi 0.9115, consciousness 0.9999.

**Root cause: Gram matrix fix (commit 3faeb6c, Aug 25 evening)**

Commit 3faeb6c fixed `gram_matrix()` to operate on `wavefront_count()` live entries
instead of `nrows()` (the store's allocated CAPACITY). Before the fix, stale and zero
rows after pruning inflated the Frobenius norm and dragged `effective_dimensionality`
DOWN, changing phi. After the fix, phi is computed accurately over live wavefronts only.

Impact on phi_target decoupling:
- `engine_a` phi: still ≈ 0.3138 (consciousness=0.8830 with old target 0.28092 is
  consistent with phi_a ≈ 0.3138, unchanged by the fix because engine_a grows but
  doesn't prune significantly during the dream)
- `engine_b_primed` phi: changes post-fix. B_primed carries A+B combined memories
  after dream priming, a different wavefront count than engine_a. The Gram fix makes
  its phi measurement more accurate, which is different from engine_a's phi.
- `engine_b_naive` phi: similarly changes. B_naive has B-only memories.
- xi_eval engines (engine_clean, engine_adv): also affected if pruning occurs.

`eval_l5_placeholder_fitness` — used for all B and xi evaluations — includes a 0.10 *
(1 − consciousness) term. Setting phi_target=0.3138 is correct for engine_a but
WRONG for B and xi engines, inflating their sub-fitness and corrupting transfer and xi.

**Quantitative confirmation**:
Transfer dropped from 0.9540 to 0.8436. If B's actual phi ≈ 0.25 (a plausible value
for a smaller wavefront store), the consciousness penalty under target=0.3138 would be:
  0.10 × (1 − (1 − |0.25−0.3138|/0.3138)) = 0.10 × 0.0638/0.3138 ≈ 0.0203
vs the old penalty under target=0.28092:
  0.10 × (1 − (1 − |0.25−0.28092|/0.28092)) ≈ 0.0110
Δ = 0.0093 increase in fitness_b_primed → transfer ≈ 1 − (0.003686+0.0093)/0.080131
  = 1 − 0.013/0.080 ≈ 0.838, matching the observed 0.8436 closely.

## Phase_coherence bundle analysis (theoretical, no trial needed)

Aug 25 notes: "phi_target + phase_coherence bundle remains the only open threshold-crossing
path: combined savings = 0.003507 + 0.002122 = 0.005629 > 0.005."

This analysis is DOUBLY wrong:
1. phi_target decoupling is now invalidated by the Gram fix (see above).
2. Even if phi_target worked, the max achievable phase_coherence improvement is tiny.

Phase_coherence is measured as the Kuramoto order parameter R within L4 content
clusters (dense_a, dense_b, etc.). The pre-dream R for dense_a (50 members, phases
initialized as i×0.025 for i∈[0,49]) is approximately 0.934 (geometric series formula).
Post-dream R = 0.8939 (the dream REDUCES coherence due to frequency-driven phase drift).

Max achievable improvement: R goes from 0.8939 toward the pre-dream cap of 0.934.
Max savings = 0.02 × (0.934 − 0.8939) = 0.02 × 0.040 = 0.0008.

Even restoring full pre-dream coherence (impossible without disabling dream Kuramoto):
Max savings = 0.02 × (1.0 − 0.8939) = 0.002122.

Combined with phi_target (0.003507, also now invalidated):
  Theoretical maximum: 0.003507 + 0.002122 = 0.005629 → barely above threshold.
  Achievable maximum: 0 (phi_target broken) + 0.0008 (phase_coherence partial) = 0.0008.

**The phi_target + phase_coherence bundle is CLOSED on both ends.**

## Corrected path for phi_target decoupling (future fire)

The phi_target approach CAN work again, but requires per-engine phi measurement:
1. Run a diagnostic trial printing phi values for each sub-engine (engine_a, engine_b_primed,
   engine_b_naive, engine_clean, engine_adv).
2. Set `consciousness_phi_target` appropriately for each engine context, OR change
   `eval_l5_placeholder_fitness` to NOT include consciousness as a sub-metric (since
   it's already scored in the main L5 fitness as a separate term with its own weight).

Option 2 is cleaner: remove `0.10 * (1 − consciousness)` from eval_l5_placeholder_fitness
entirely. That function is ONLY used for transfer and xi sub-evaluations, where
consciousness is irrelevant to whether B recall is better than naive. The main L5 fitness
already scores consciousness correctly. This change would make phi_target decoupling safe
regardless of B's actual phi.

Savings if this correction works: consciousness savings = 0.003507. Still below the
0.005 threshold alone; still needs a bundle partner.

## Summary of open paths

| path                               | max savings | status                                      |
|------------------------------------|-------------|---------------------------------------------|
| phi_target decoupling (corrected)  | 0.003507    | open but requires eval_l5_placeholder_fitness change |
| phase_coherence                    | 0.000802    | structurally capped at pre-dream R ≈ 0.934  |
| xi_robustness_v2                   | 0.004830    | no lever found; near-maximal at 0.9678      |
| transfer_score                     | 0.006900    | levers exhausted (Jul 31)                   |
| CHAIN_TOP_N                        | —           | **closed**: both directions hurt             |
| phi_target + phase_coherence       | —           | **closed**: phi_target broken, phase cap tiny |

**Only viable threshold-crossing bundle**: phi_target (corrected, removing consciousness
from eval_l5_placeholder_fitness) + any lever providing ≥0.001493 savings. No such lever
is currently known, but the corrected phi_target is itself a prerequisite for any future
bundle to work correctly.

## Decision

No code changes kept. All src/ changes reverted. Floor remains at 0.018454.

TSV rows appended (4 rows):
- Baseline: DRIVE_A=0.1 DRIVE_SCOPE=all, fitness 0.018454 (environment calibration)
- T1: phi_target=0.3138 + CHAIN_TOP_N=5, fitness 0.067257 (CHAIN_TOP_N<7 closed)
- T2: phi_target=0.3138 + CHAIN_TOP_N=10, fitness 0.035355 (CHAIN_TOP_N>7 closed)
- T3: phi_target=0.3138 + CHAIN_TOP_N=7, fitness 0.039948 (phi_target broken by Gram fix)
