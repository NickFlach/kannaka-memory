# Three-hypothesis sweep: relax_steps / phi_target / chain_carry_strength

**Date:** 2026-06-10T09 UTC
**Branch:** kannaka-curiosity/2026-06-10T02-relax-steps-24
**Code changes:** ALL REVERTED — notes-only commit
**Status:** No code improvement above threshold; chain_carry_strength=0.85 promising but sub-threshold (0.004 < 0.005)

---

## Starting state (T01 baseline)

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
fitness ≈ 0.018282 (deterministic)
transfer=0.903199, xi=0.987, carrier_e=0.999
magic_R=0.864, query_gravity=0.373
```

Diagnostics collected this fire:
```
fitness_B_primed:  0.005856
fitness_B_naive:   0.060498
phi_actual_a:      0.293672  (NEW — first time phi measured directly)
```

Fitness breakdown (T01 state):
- transfer (0.903): 0.15 × 0.097 = **0.01455** (80%)
- xi (0.987): 0.15 × 0.013 = **0.00195** (11%)
- consciousness (0.9546): 0.03 × 0.0454 = **0.00136** (7%)
- other: ~0.00034 (2%)

---

## Hypothesis 1: relax_steps 16→24

**Prediction:** More convergence per dream cycle → B-primed snaps tighter to A's clusters → lower fitness_b_primed → higher transfer. Context question 3 (from relax_steps=8) not yet tested at current fitness level.

**Trial (relax_steps=24):**
| metric | baseline | trial | delta |
|--------|----------|-------|-------|
| fitness | 0.018223 | 0.068528 | +0.050 **REGRESSION** |
| transfer | 0.903 | 0.862 | −0.041 |
| xi | 0.987 | 0.748 | −0.239 |
| carrier_e | 0.999 | 0.922 | −0.077 |
| fitness_b_primed | 0.005856 | 0.008464 | +0.003 (WORSE) |
| fitness_b_naive | 0.060498 | 0.061456 | +0.001 |

**Verdict: Falsified.** Over-relaxation homogenizes all phases, including in B-primed — A's organizational context is disrupted rather than reinforced. The extra 8 steps (24 total) give adversarials more disruption time in the xi eval, collapsing xi from 0.987→0.748.

**Key learning:** Unlike going from 8→16 steps (which helped), going 16→24 reveals that relaxation passes through an optimum. At 16 steps, B-primed's A-inherited structure is preserved while B's memories align to it. At 24 steps, B's memories start pulling A's memories out of their optimal post-dream configuration.

---

## Hypothesis 2: consciousness_phi_target recalibration

**Background:** consciousness_phi_target=0.28092 was set for L4+stage_sync. With L5+interference_relax, phi_actual_a=0.293672 (measured this fire). The consciousness score for engine_a = 0.9546 because phi drifted upward.

**Prediction:** If phi_b_primed ≈ phi_a ≈ 0.294, setting target=0.293672 makes consciousness_b_primed → 1.0. Consciousness contributes ~75% of fitness_b_primed → fitness_b_primed drops ~75% → transfer improves dramatically (estimated 0.903→0.977).

**Trial (CONSCIOUSNESS_PHI_TARGET=0.293672):**
| metric | baseline | trial | delta |
|--------|----------|-------|-------|
| fitness | 0.018223 | 0.029262 | +0.011 **REGRESSION** |
| transfer | 0.903 | 0.821 | −0.082 |
| fitness_b_primed | 0.005856 | 0.010019 | +0.004 (WORSE) |
| fitness_b_naive | 0.060498 | 0.055929 | −0.005 (better, as predicted) |

**Verdict: Falsified.** The prediction assumed phi_b_primed ≈ phi_a. This is wrong.

**Critical learning: phi_b_primed ≈ 0.280 (near OLD target), not 0.294 (phi_a).**

Analysis from trial results:
- fitness_b_primed INCREASED by 0.004 when target moved from 0.281 to 0.294. This means phi_b_primed was CLOSER to the old target 0.281 and further from the new target 0.294. The consciousness contribution to fitness_b_primed under old target was near 0 (not 0.0045 as assumed).
- fitness_b_naive DECREASED by 0.005 when target moved to 0.294. This means phi_b_naive > 0.281 (phi_b_naive ≈ 0.32, below target in old direction, above in new direction).

**Revised phi estimates:**
| engine | phi estimate | method |
|--------|-------------|--------|
| engine_a | **0.293672** | measured directly |
| engine_b_primed | **≈ 0.280** | inferred from fitness_b_primed response |
| engine_b_naive | **≈ 0.320** | inferred from fitness_b_naive response |

**Revised fitness_b_primed decomposition:**
| term | weight | value | contribution |
|------|--------|-------|-------------|
| chain_fidelity | 0.10 | ≈ 0.941 | 0.0059 |
| consciousness | 0.10 | ≈ 0.997 | 0.0003 |
| other | 0.30 | ≈ 1.000 | ≈ 0 |
| **TOTAL** | | | **0.005856** |

Chain_fidelity (≈0.941) is the binding constraint, NOT consciousness.

**Note for future phi work:** To improve transfer via consciousness tuning, the target should approach phi_b_primed ≈ 0.280. But this would require lowering the target BELOW the old 0.28092, slightly hurting consciousness_a. The net effect on transfer may be marginal since consciousness_b_primed was already near 1.0 under old target.

The only consciousness lever for transfer is through phi_b_naive ≈ 0.32 (contributes 0.0139 to fitness_b_naive). Moving target away from phi_b_naive INCREASES fitness_b_naive → improves transfer. But target=0.28092 is already somewhat below phi_b_naive (0.320 > 0.281), and lowering target further would HURT consciousness_a. Not a clean win.

---

## Hypothesis 3: chain_carry_strength 0.7→0.85

**Background:** chain_carry_strength determines how strongly the previous cycle's xi centroid biases the next cycle's pair selection. Current: 0.7 (L4-calibrated). No L5-specific sweep has been done.

**Prediction:** A's cycle-1 xi centroid is from a high-quality 4-cycle dream → it's a well-organized centroid. Higher chain_carry_strength should amplify this quality and create tighter chain_fidelity in B-primed. B-naive's cycle-1 centroid is from a cold start → amplifying it with stronger carry may help or hurt equally. Net effect: B-primed benefits more → lower fitness_b_primed/fitness_b_naive ratio → higher transfer.

**Trial (CHAIN_CARRY_STRENGTH=0.85):**
| metric | baseline | trial | delta |
|--------|----------|-------|-------|
| fitness | 0.018223 | 0.013883 | **−0.004340 (IMPROVEMENT)** |
| transfer | 0.903 | 0.931 | **+0.028** |
| xi | 0.987 | 0.988 | +0.001 |
| fitness_b_primed | 0.005856 | 0.004181 | **−0.001675** |
| fitness_b_naive | 0.060498 | 0.060557 | ≈ 0 |
| magic_R | 0.864 | 0.868 | +0.004 |

**Verdict: Sub-threshold improvement.** The improvement is 0.004340, just below the 0.005 threshold. Additionally, only 1 trial was run; the 3-trial confirmation required by the protocol was not possible within the 5-run budget.

**Mechanism confirmed:** fitness_b_primed dropped 0.001675 while fitness_b_naive was essentially unchanged (0.060557 vs 0.060498). This is exactly the predicted asymmetric benefit. B-primed's better starting point (A's organized xi centroid) compounds more effectively with higher chain_carry_strength, while B-naive's cold-start centroid doesn't improve similarly.

**Why just below threshold:** 
- chain_fidelity_b_primed improved: 0.941 → estimated ≈ 0.958 (based on fitness_b_primed drop)
- chain_fidelity_b_naive unchanged: ≈ 0.534

The carry compounding effect is real but moderate at 0.85. Trying 0.90 might cross the threshold, but could also over-constrain later cycles (if cycle 1's centroid is imperfect, carry strength 0.90 amplifies that imperfection more aggressively).

---

## TSV rows added this fire

4 rows appended to experiments/results-L5.tsv during this fire:
1. relax_steps=24 trial: fitness 0.068528 (regression)
2. Diagnostic baseline run: fitness 0.018223 (confirms T01 state with slightly different timing)
3. CONSCIOUSNESS_PHI_TARGET=0.293672 trial: fitness 0.029262 (regression)
4. CHAIN_CARRY_STRENGTH=0.85 trial: fitness 0.013883 (sub-threshold improvement)

---

## Open axes for next fire

| axis | priority | evidence | mechanism | predicted delta |
|------|----------|----------|-----------|----------------|
| **chain_carry_strength sweep** | HIGH | 0.85 gives 0.004 improvement; 0.90 may cross 0.005 | Stronger xi centroid carry benefits B-primed's organized starting state more than B-naive's cold start | ~0.005–0.007 |
| **chain_carry_strength sweep** (2) | HIGH | Check 0.80 as well to map the curve | Ensure 0.85 is on monotonic improvement slope and not near a peak | — |
| **phi_b_primed diagnosis** | MEDIUM | phi_b_primed ≈ 0.280, not phi_a=0.294 — indicates combined corpora reduce phi | If phi_b_primed can be raised (better consolidation in B-primed dream) → improved consciousness_b_primed | ~0.001 |
| **xi ceiling** | LOW | xi=0.987, contributes 0.002 to fitness | — | <0.001 alone |

**Priority action next fire: Chain_carry_strength sweep {0.80, 0.85, 0.90} with 3-trial confirmation for the winning value.**

---

## Decision

**No code changes retained.** CHAIN_CARRY_STRENGTH=0.85 is the most promising direction found this fire (0.004340 improvement, transfer 0.903→0.931) but falls below the 0.005 threshold and wasn't confirmed in 3 trials. Must be re-tested with full budget next fire.

Current master remains at T01 state: fitness ≈ 0.018282, transfer=0.903, xi=0.987.
