# 2026-07-23T00 — B-primed chiral=0.25 and chain_depth=2 both falsified; transfer floor confirmed robust

## Context

Entering baseline requires two ephemeral code changes per fire:
1. `CARRIER_KURAMOTO_COUPLING` env override in the `amp_deltas_flat` block
2. `xi_eval_params.chain_depth = 3`

Env vars: DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_GRAVITY=0.35
KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5

Prior confirmed baseline: fitness ~0.019249 (3-trial avg, July 17 fire).

Significant new commits since July 21 fire:
- Track-D strong-then-weak coupling alternation (now ON by default)
- KANNAKA_DREAM_GRAVITY medium-level gravity pass (new env var, default 0.0 = inactive in L5 path)

Note: KANNAKA_DREAM_GRAVITY operates inside `ChiralMedium::dream`, which is NOT called by
`run_l5_dream_chain` (L5 uses `consolidator.consolidate`). Testing KANNAKA_DREAM_GRAVITY
in the L5 context would be a null result.

---

## Hypothesis 1 — chiral_b_primed=0.25 to improve B-primed chain fidelity

### Reasoning

July 20 fire falsified chiral_b_primed=0.05 (lower) — fitness_B_primed ROSE because less
chiral perturbation → less Xi diversity → degraded chain_fidelity. The symmetry argument:
if less chiral hurts, MORE chiral should help. chiral=0.25 (between 0.15 current and
0.70 B_naive level) should give better Xi diversity while preserving the primed/naive
asymmetry.

**Prediction**: transfer_score rises from 0.938 toward 0.942.

### Result (1 trial, with ephemeral code changes applied)

| metric             | chiral=0.25 | baseline (0.15) | delta      |
|--------------------|-------------|-----------------|------------|
| fitness            | 0.021679    | 0.019102        | +0.002577  |
| transfer_score     | 0.921235    | 0.938419        | −0.017184  |
| consciousness      | 0.8830      | 0.8830          | 0          |
| xi_robustness_v2   | 0.9783      | 0.9783          | 0          |
| carrier_emergence  | 1.0000      | 1.0000          | 0          |

### Analysis

**Hypothesis FALSIFIED.** Transfer worsened significantly at chiral=0.25.

The symmetry assumption was wrong. The mechanism is NOT simply "more chiral → more
Xi diversity → better chain_fidelity → better transfer." The actual mechanism has two
competing effects:

1. **Xi diversity (helps)**: more chiral perturbation → richer Xi diversity during
   B_primed's dream → better chain_fidelity → lower fitness_B_primed

2. **A-scaffold disruption (hurts)**: more chiral perturbation also disrupts A's existing
   consolidated phase structure during the B_primed dream. At chiral=0.25, the increased
   perturbation degrades A's phase attractors that B memories were supposed to consolidate
   onto, REDUCING the advantage of the primed state vs B_naive.

Effect 2 dominates at chiral=0.25. The B_naive dream already uses chiral=0.70, so increasing
B_primed's chiral toward 0.70 reduces the primed/naive asymmetry — both now have similarly
disruptive chiral levels, collapsing the transfer advantage.

**chiral=0.15 is the optimum.** Both 0.05 (July 20) and 0.25 (this fire) are worse.

---

## Trial 0 — Baseline verification

The first trial confirmed the current baseline with the ephemeral code changes:

| metric             | this fire  | July 21 baseline |
|--------------------|------------|-----------------|
| fitness            | 0.019102   | 0.019249        |
| transfer_score     | 0.938419   | 0.938415        |
| xi_robustness_v2   | 0.9783     | 0.9783          |
| consciousness      | 0.8830     | 0.8830          |
| speed_a            | 0.9676     | 0.963           |
| carrier_emergence  | 1.0000     | 1.0000          |

**speed_a improved from 0.963 to 0.9676** (fitness improvement: 0.000147).
The Track-D coupling alternation commit (strong-then-weak heartbeat, now ON by default)
appears to have slightly accelerated dream consolidation, reducing wall-clock time from
~15300ms to ~13459ms and improving the speed metric. All other metrics byte-identical.

New confirmed baseline: **fitness 0.019102** (single trial, consistent with prior 3-trial
pattern given byte-identical metrics except speed).

---

## Hypothesis 2 — params_bp.chain_depth=2 to preserve A's scaffold in B_primed

### Reasoning

B_primed starts with A's fully consolidated phase structure. With A's attractor already
established, B_primed might need fewer dream cycles to consolidate B's new memories —
the scaffold speeds convergence. Reducing chain_depth from 4 to 2 would:
1. Preserve A's phase attractors more (less disruption from additional Kuramoto cycles)
2. Enable faster chain_fidelity convergence (A's scaffold → Xi centroid stable in 2 cycles)
3. Keep phi closer to phi_target=0.28092 (fewer cycles → phi doesn't overshoot)

If B_primed achieves better sub-fitness at depth=2, while B_naive still needs 4 cycles,
the ratio fitness_B_primed / fitness_B_naive shrinks → transfer improves.

**Prediction**: transfer_score rises from 0.938 toward 0.965.

### Result (1 trial, code change: params_bp.chain_depth = 2)

| metric             | depth=2 B_primed | baseline (depth=4) | delta       |
|--------------------|------------------|--------------------|-------------|
| fitness            | 0.038513         | 0.019102           | +0.019411   |
| transfer_score     | 0.809144         | 0.938419           | −0.129275   |
| fitness_B_primed   | 0.011424         | 0.003686           | +0.007738   |
| fitness_B_naive    | 0.059856         | 0.059856           | 0           |
| consciousness      | 0.8830           | 0.8830             | 0           |
| xi_robustness_v2   | 0.9783           | 0.9783             | 0           |

### Analysis

**Hypothesis FALSIFIED — catastrophic regression.**

fitness_B_primed TRIPLED (0.003686 → 0.011424) at chain_depth=2. Transfer collapsed
from 0.938 to 0.809. fitness_B_naive was byte-identical (unaffected, still depth=4).

The scaffold hypothesis was wrong. What actually happens at depth=2:

1. **chain_fidelity degrades severely**: at depth=2, only 1 pair of consecutive Xi centroids
   is measured (vs 3 pairs at depth=4). A's scaffold helps chain_fidelity by the second cycle,
   but the Xi centroid has not had time to converge — cycle 2 still carries significant
   consolidation residual. The base_score is lower than at depth=4 where the chain has
   converged more fully.

2. **Kuramoto sync incomplete**: at depth=2, the Kuramoto relaxation has only run one full
   cycle past initialization. Phase coherence within-corpus is lower → chain_fidelity
   degrades further through the correlated phi_history monotonicity bonus.

3. **Interference not fully resolved**: B's inserted memories initially create phase
   interference with A's existing memories. 2 cycles is insufficient to resolve these
   interference patterns through stage_xi_repulsion and stage_sync. At depth=4, the
   interference is mostly resolved.

The primed scaffold HELPS depth=4 reach convergence faster than B_naive, but 2 cycles
is simply not enough for ANY corpus to reach the quality measured by eval_l5_placeholder_fitness.
B_primed at depth=4 achieves fitness_B_primed=0.003686 (a 16× advantage over B_naive's
0.059856) — this IS the benefit of the scaffold, already captured at depth=4.

**chain_depth=4 is the optimum for B_primed.**

---

## What we now know about the B-primed transfer floor

The transfer ceiling at 0.938 is confirmed robust to ALL of:

| parameter             | range tested             | best value | direction of change |
|-----------------------|--------------------------|------------|---------------------|
| K (Kuramoto coupling) | 1.5–5.0 (July 12)       | K=2.0      | tested both sides   |
| DREAM_GRAVITY         | 0.25–0.40 (July 17)     | 0.35       | doesn't move xfer   |
| CHAIN_TOP_N           | 5–10 (July 19)           | 7          | insensitive         |
| chiral_b_primed       | 0.05, 0.15, 0.25         | **0.15**   | both sides tested   |
| DRIVE_FREQ_HZ         | 0.25, 0.5, 1.0 (July 20) | 0.5       | insensitive         |
| chain_depth_b_primed  | 2, 4                     | **4**      | less is catastrophic|

The transfer floor appears structurally determined. fitness_B_primed = 0.003686 is a stable
equilibrium that reflects the maximum achievable by the eval_l5_placeholder_fitness formula
given the B_primed engine's state after a proper (depth=4, chiral=0.15) consolidation with
A's scaffold.

The ONE remaining structural intervention from July 21 notes: **phi_target decoupling** —
split phi_target into main_phi_target (engine_a main eval, set to 0.3138) and eval_phi_target
(placeholder fitness evals, kept at 0.28092). This would improve consciousness from 0.8830
to 1.0000, saving 0.003510 fitness (taking fitness to ~0.015592). Below the ≥0.005
absolute threshold alone, but it's the largest remaining single axis.

---

## Summary

| hypothesis                      | direction           | result        | fitness delta   |
|---------------------------------|---------------------|---------------|-----------------|
| Baseline (Track-D alternation)  | new commit check    | CONFIRMED     | −0.000147 (✓ better) |
| chiral_b_primed=0.25            | more Xi diversity   | FALSIFIED     | +0.002577 (worse) |
| chain_depth=2 for B_primed      | scaffold = fewer cycles | FALSIFIED | +0.019411 (catastrophic) |

No improvement found. All code changes reverted.

**New confirmed baseline: fitness 0.019102** (speed 0.9676, consistent with the
July 17 three-trial average + Track-D alternation speed gain).

---

## Next fire recommendations

1. **phi_target decoupling** (code change): the July 21 notes identified this as
   the remaining structural improvement — split phi_target so engine_a main eval uses
   main_phi_target=0.3138 (phi → consciousness=1.0) while eval_l5_placeholder_fitness
   continues using eval_phi_target=0.28092 (preserving transfer and xi). Expected gain:
   0.003510. Requires bundling with ≥0.001 from another source to reach ≥0.005 threshold.

2. **Transfer wall analysis**: with 6 parameters now confirmed insensitive, the transfer
   ceiling is almost certainly structural (determined by the depth-4, chiral=0.15 equilibrium
   of eval_l5_placeholder_fitness). The B_primed fitness (0.003686) breaks down into
   sub-metric contributions. Adding debug output for each sub-metric of B_primed vs B_naive
   would identify which sub-metric is the binding constraint on transfer.

3. **KANNAKA_DREAM_GRAVITY for L5**: although this env var operates in ChiralMedium::dream
   (not called from run_l5_dream_chain), check whether L5 has a path through the ChiralMedium.
   If consolidate() eventually delegates to dream() for some engines, KANNAKA_DREAM_GRAVITY
   might be testable in L5 after all. Read ChiralConsolidator more carefully.

4. **phase_coherence**: at 0.8939 (11% of fitness, contribution 0.002122), this is the
   smallest remaining axis. The kuramoto_steps=100 test (July 20) was catastrophic. The
   phase_coherence formula measures Kuramoto order parameter R within 6 content clusters.
   Is there a way to improve within-cluster phase alignment WITHOUT increasing Kuramoto steps?
   Consider: higher DREAM_GRAVITY might concentrate amplitude toward phase-neighbors within
   clusters, raising the cluster-level order parameter. DREAM_GRAVITY=0.50 is untested.

## TSV rows appended (3 total)

- Trial 0 (baseline): fitness 0.019102, transfer 0.938419, speed 0.9676
- Trial 1 (chiral_b_primed=0.25): fitness 0.021679, transfer 0.921235
- Trial 2 (chain_depth=2 B_primed): fitness 0.038513, transfer 0.809144
