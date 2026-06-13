# chiral_perturbation=0.5 and chain_carry=0.85 — both falsified

**Date:** 2026-06-10T19 UTC
**Branch:** kannaka-curiosity/2026-06-10T19-chiral-05
**Code changes:** NONE retained — both hypotheses falsified, all code reverted
**Status:** FALSIFIED — 0.7 confirmed near-optimal for chiral; carry=0.85 over-constrains after b_primed=20 steps

---

## Master entering this fire

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
relax_steps: engine_b_primed=20, others=16 (alpha_base=0.10)
chain_carry_strength=0.70, chiral_perturbation=0.70, chain_top_n=7
3-trial avg fitness ≈ 0.013337 (fully deterministic)
transfer=0.935746, xi=0.987, carrier_e=0.9992
magic_R=0.864, query_gravity=0.373
```

Fitness breakdown:
- transfer (0.15): 0.15 × 0.064 = **0.00964** (72%)
- xi (0.15): 0.15 × 0.013 = **0.00195** (15%)
- other: **0.00175** (13%)

---

## Hypothesis 1: chiral_perturbation = 0.5

**Rationale:** `chiral_perturbation=0.7` was an L4-calibrated override from 0.9. T12 identified
"chiral_perturbation sweep {0.5, 0.6, 0.7, 0.8}" as untested in the L5+irx+bprimed=20 regime.

**Prediction:** Less B-vector disruption of A's chirality structure → phi_bp moves closer to
phi_target (0.281) → fp (fitness_b_primed) drops → transfer improves. Xi is at 0.987 (near
ceiling), so a small xi cost is acceptable. Net: transfer +0.01–0.02, fitness ~0.012.

**Code change:** `src/bin/research.rs` L5 block: `l5_params.chiral_perturbation = 0.5;`

**Result (Trial 1):**

| metric | baseline (0.7) | trial (0.5) | delta |
|--------|----------------|-------------|-------|
| fitness | 0.013337 | **0.017059** | +0.003722 **REGRESSION** |
| transfer | 0.935746 | **0.918387** | −0.017359 |
| xi_robustness_v2 | 0.9870 | **0.9809** | −0.0061 |
| carrier_emergence | 0.9992 | **0.9941** | −0.0051 |
| carrier_bimodal | ~0.915 | **0.8942** | −0.021 |
| magic_proxy_phase_R | 0.864 | **0.8075** | −0.057 |
| query_gravity | 0.373 | 0.3675 | −0.006 |

**Verdict: Falsified — regression across every axis.**

The prediction was wrong about the direction of effect. Reducing chirality from 0.7 to 0.5
*hurts* transfer (−0.018), not helps it. The mechanism is now clear: chiral_perturbation
creates structural vector diversity that is load-bearing for stage_interference_relax's ability
to anchor B memories to A's phase attractor. Less chirality diversity → interference_relax has
less gradient to work with → poorer convergence → lower transfer.

**magic_R also dropped (0.864 → 0.808).** This is a direct observation of the magic→xi coupling
in `research/intersections/05-magic-gives-it-gravity.md`: reducing non-Clifford-like phase
diversity (magic) through lower chirality drives xi down in proportion. Both dropped together,
consistent with magic mediating xi rather than the reverse.

**L4 history confirms:** L4.16 reduced chiral 0.9 → 0.7 as an improvement. Now 0.5 is worse
than 0.7. The sweep confirms 0.7 is already near or at the optimal point; going lower collapses
structural diversity without recovering transfer.

---

## Hypothesis 2: chain_carry_strength = 0.85

**Rationale:** T12 (chain_carry_sweep, 2026-06-10T12) tested carry=0.85 against the 0.018
baseline (before T07's b_primed=20 steps) and found sub-threshold improvement of −0.004403.
T07 improved fitness from 0.018 → 0.013 through a different mechanism (extra relaxation steps
for b_primed). The question: do carry=0.85 and b_primed=20 stack?

**Prediction:** Both mechanisms are orthogonal — carry strength affects xi centroid propagation
across chain cycles; relax_steps affects within-cycle convergence. If orthogonal, the ~0.004
from carry and ~0.005 from relax_steps might combine to ~0.009 total improvement from the
0.018 baseline, putting current fitness at ~0.009.

**Code change:** `src/bin/research.rs` L5 block: `l5_params.chain_carry_strength = 0.85;`

**Result (Trial 1):**

| metric | baseline (carry=0.70) | trial (carry=0.85) | delta |
|--------|-----------------------|---------------------|-------|
| fitness | 0.013337 | **0.013722** | +0.000385 slight regression |
| transfer | 0.935746 | **0.932064** | −0.003682 |
| xi_robustness_v2 | 0.9870 | **0.9883** | +0.0013 |
| carrier_emergence | 0.9992 | 0.9992 | 0 |
| carrier_bimodal | ~0.915 | 0.9167 | ~0 |
| magic_proxy_phase_R | 0.864 | 0.8677 | +0.004 |
| query_gravity | 0.373 | 0.3749 | +0.002 |

**Verdict: Falsified — slight regression; mechanisms do NOT stack cleanly.**

The mechanisms are not cleanly orthogonal. Carry=0.85 amplifies the cycle-1 xi centroid
constraint, which was beneficial when b_primed ran 16 relaxation steps (T12 baseline). After
T07's 20-step upgrade, each cycle's relaxation is more thorough — the system has already
found a good per-cycle attractor. Adding carry=0.85 on top re-constrains cycles 2-4 toward
a cycle-1 centroid that is slightly misaligned with the 20-step optimal, causing mild
over-constraining.

This is the same "amplification of imperfections" mechanism identified in T12 for carry=0.90
vs. 0.85 — but shifted one step: now carry=0.85 acts like T12's carry=0.90 did against the
16-step baseline.

**Interesting secondary signal:** magic_R improved slightly (+0.004) and xi improved slightly
(+0.001) at carry=0.85. The carry amplification affects the overall phase structure more than
it affects the primed-vs-naive ratio. The phase coherence of the full dream is marginally
better at 0.85 carry, but the critical transfer metric degrades.

---

## Combined picture of explored axes (this fire)

| axis | status | optimal value | constraint |
|------|--------|---------------|------------|
| chiral_perturbation | **CLOSED** | 0.7 | 0.5 worse (−transfer), L4 showed 0.9 worse |
| chain_carry_strength | **CLOSED** | 0.7 (post-T07) | 0.85 over-constrains after 20 relax steps |
| relax_steps (b_primed) | CLOSED (T07) | 20 | T11 confirmed 24 global collapses |
| DRIVE_A | CLOSED | 0.10 | cliff at 0.15 |
| chain_top_n | **OPEN** | 7 untested | Untested in L5+irx+bprimed=20 regime |

---

## Remaining open axes

| axis | expected direction | mechanism |
|------|--------------------|-----------|
| chain_top_n sweep {5, 6, 8, 9} | unknown | Seed breadth vs. focused centroid tradeoff; L4 found top_n=5 collapsed xi, but L5+irx has different dynamics |
| relax_steps b_naive ≠ 16 | likely neutral/regression | fn_naive is stable; improving it hurts transfer ratio |
| Stage ordering in irx | invasive | Run stage_sync after stage_interference_relax; T15 notes flagged this as possible xi fix but risky |

**chain_top_n is the last clean axis.** All others are either closed, structural, or invasive.

---

## Phi landscape (unchanged from T10 characterization)

```
phi_bp ≈ 0.273 (post-T07 estimate)  phi_target = 0.281  phi_naive ≈ 0.296  phi_a ≈ 0.294
```

The carry=0.85 trial did not improve phi_bp — transfer regressed, confirming phi_bp is not
moving meaningfully with carry changes in the current regime. The transfer ceiling at ~0.936
reflects the structural phi gap between phi_bp (0.273) and phi_target (0.281).

---

## Decision

**No code changes retained.** Both hypotheses falsified.

Master state unchanged:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness = 0.013337 (deterministic)
transfer=0.935746, xi=0.987, carrier_e=0.9992
magic_R=0.864, query_gravity=0.373
```

**New constraints confirmed this fire:**
- chiral_perturbation: 0.7 is confirmed near-optimal (direction toward 0.5 confirmed bad)
- chain_carry_strength: 0.7 is now optimal (0.85 was optimal at 16 steps but over-constrains at 20 steps)
- Carry and relax_steps are not orthogonal — the T07 relax improvement subsumed and superseded T12's carry benefit
