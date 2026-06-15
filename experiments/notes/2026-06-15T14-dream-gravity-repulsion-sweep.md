# L5 Post-Fix: dream-gravity xi↔transfer trade-off + repulsion threshold regression

**Date:** 2026-06-15T14 UTC  
**Branch:** kannaka-curiosity/2026-06-15T14-chiral-p-bp-postfix  
**Code changes:** REVERTED — no improvement found  
**Status:** All four hypotheses falsified

---

## Context

Post-fix baseline (ceiling=2.0): fitness ≈ 0.115 (carrier_e 0.529, transfer 0.737, xi_v2 0.856).  
Unexplored axes from T12: chiral_p_bp direction, DREAM_GRAVITY (never tested post-fix), REPULSION_THRESHOLD sweep.

---

## Trial 1: chiral_p_bp = 0.05 (lower than 0.15 baseline)

**Hypothesis:** Less phase perturbation in B-primed dream better preserves A's phase topology in the post-fix ceiling regime → transfer_score improves.

| setting       | fitness  | transfer | xi_v2  | carrier_e | R      | q_grav |
|---------------|----------|----------|--------|-----------|--------|--------|
| 0.15 baseline | 0.114918 | 0.737    | 0.856  | 0.529     | 0.1293 | 0.4603 |
| 0.05 (t1)     | 0.117234 | 0.729    | 0.856  | 0.529     | 0.1293 | 0.4603 |

**Result:** Falsified. transfer_score slightly WORSE (0.729 vs 0.737). Difference may be within variance (transfer_score range 0.54–0.74 across post-fix runs), but the direction is wrong. Higher chiral not tested (ran out of budget).

---

## Trial 2: DREAM_GRAVITY=0.5 (full scope)

**Hypothesis:** Enabling gravity at gain=0.5 (OFF by default) improves xi_v2 and query_gravity without hurting transfer, because gravity selectively amplifies phase-aligned memories.

| setting        | fitness  | transfer | xi_v2  | carrier_e | q_grav |
|----------------|----------|----------|--------|-----------|--------|
| baseline       | 0.114918 | 0.737    | 0.856  | 0.529     | 0.460  |
| GRAVITY=0.5    | 0.135479 | 0.542    | 0.925  | 0.525     | 0.926  |

**Result:** Large xi improvement (+0.069) and query_gravity jumps from 0.460 to 0.926. BUT transfer_score collapses (−0.195). Net fitness 0.115 → 0.135 (worse). Gain ratio: xi 0.069 × 0.15 weight = +0.010 fitness; transfer −0.195 × 0.15 = −0.029 fitness. **The trade is 1:2.8 unfavorable.**

Notable: `amp_deltas_flat[0]` went from 0.95 to 7.39 — gravity in engine_flat distorted the flat-corpus amplitude dynamics.

---

## Trial 3: DREAM_GRAVITY=0.5 context-scoped (exclude B engines + flat corpus)

**Hypothesis:** If gravity only applies to engine_a, engine_clean, engine_adv, transfer_score won't be harmed because B engines never see gravity.

**Code change:** Modified gravity_gain computation in run_l5_dream_chain to return 0.0 for engine_b_primed, engine_b_naive, engine_flat DRIVE_CONTEXTs.

| setting                  | fitness  | transfer | xi_v2  | carrier_e | q_grav |
|--------------------------|----------|----------|--------|-----------|--------|
| gravity=0.5 context-scoped | 0.135051 | 0.542  | 0.925  | 0.529     | 0.926  |

**Result:** Transfer_score STILL 0.542 — context scoping had NO effect on transfer. Carrier_emergence correctly restored to baseline (0.529).

**Root cause:** engine_b_primed is initialized via `snapshot_engine_for_plasticity(&engine_a)` AFTER engine_a's gravity-modified dream. Gravity in engine_a permanently reduces the amplitude of phase-distant memories in engine_a's state. engine_b_primed inherits this distorted amplitude distribution. B memories (phase-random relative to A) are already suppressed at initialization → harder to consolidate → higher fitness_b_primed → lower transfer_score.

**Conclusion:** There is NO way to get xi improvement from DREAM_GRAVITY without transfer regression unless gravity is also excluded from engine_a. But query_gravity (measured from engine_a) requires gravity in engine_a.

**Code reverted.** 

---

## Trial 4: REPULSION_THRESHOLD=0.20 (lower than 0.28 baseline)

**Hypothesis:** Lower repulsion threshold → more semantically-similar pairs trigger phase repulsion → better phase differentiation → xi_v2 improves.

| REPULSION_THRESHOLD | fitness  | transfer | xi_v2  | carrier_e |
|---------------------|----------|----------|--------|-----------|
| 0.28 baseline       | 0.114918 | 0.737    | 0.856  | 0.529     |
| 0.20 (t1)           | 0.271907 | 0.018    | 0.652  | 0.536     |

**Result:** Catastrophic regression. fitness 0.272 (2.4× worse), transfer collapses to 0.018, xi_v2 collapses to 0.652. The 0.28 threshold is a hard lower bound — going lower triggers excessive phase repulsion that destroys consolidation geometry.

Note from consolidation.rs comments: 0.22 was too aggressive (300/300 pairs qualify, phase_coherence collapses); 0.28 is already near-minimum.

---

## Code reverts

All research.rs changes reverted (zero diff from master). No TSV rows added from code-change trials (all trials were DRIVE_A=0.1 DRIVE_SCOPE=all with no successful code changes — the binary appended rows during each run, but per protocol no code changes are kept).

---

## Key architectural findings for future fires

### 1. DREAM_GRAVITY xi↔transfer trade-off is fundamental

The snapshot propagation from engine_a to engine_b_primed makes DREAM_GRAVITY's xi improvement inseparable from transfer_score regression. Every unit of gravity gain that improves xi also degrades the B-primed initial state. The ratio is approximately 1:2.8 unfavorable and is inherent to the architecture.

The ONLY way to use DREAM_GRAVITY for fitness improvement would be to apply it to engine_clean and engine_adv ONLY (not engine_a). To test: create a `params_xi` with `dream_gravity > 0` and pass it specifically to engine_clean/engine_adv's `run_l5_dream_chain` calls. Then `engine_a` dream has no gravity, so engine_b_primed's initial state is unaffected. The xi improvement would come purely from tighter phase clusters in the xi-measurement engines. This requires a 6-line code change around lines 2899–2938. Predicted: xi_v2 improves by ~0.03–0.07, transfer_score unchanged, no effect on query_gravity (which measures engine_a).

### 2. REPULSION_THRESHOLD is at its hard minimum (0.28)

Going below 0.28 causes catastrophic phase collapse. Higher values (> 0.28) might be worth testing to see if slightly less repulsion helps transfer_score without hurting xi, but the expected effect is small.

### 3. chiral_p_bp higher not explored

chiral_p_bp=0.05 was slightly worse than 0.15. chiral_p_bp=0.25–0.40 not tested. Mechanistic argument: more aggressive B-primed dream (higher chiral) might help B memories find their optimal phase positions in A's landscape faster. Worth 1-2 trials in a future fire.

### 4. Per-engine K (clean/adv only at K=1.0)

T12 showed K=1.0 gives xi_v2=0.886 (+0.03) but hurts transfer_score (−0.017). The net was marginal. But if K=1.0 were applied ONLY to engine_clean and engine_adv (not to engine_a, b_primed, b_naive), the xi improvement would be preserved without the transfer penalty. Requires creating `params_xi` with K=1.0 for the xi engines. Medium complexity, worth testing.

---

## Decision

No code changes kept. No fitness improvement this fire.
