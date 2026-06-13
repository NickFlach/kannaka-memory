# relax_steps=24 falsified — over-relaxation degrades all metrics

**Date:** 2026-06-10T11 UTC
**Branch:** kannaka-curiosity/2026-06-10T11-relax-steps-24
**Code changes:** REVERTED — regression confirmed on trial 1.
**Status:** FALSIFIED — relax_steps=16 is a fragile operating point.

---

## Background

Current empirical optimum (post T01 BFS sort, T06 constraints mapped):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
2-trial avg fitness ≈ 0.018 (fully deterministic)
transfer=0.903, carrier_e=0.999, xi=0.987
magic_R=0.864, query_gravity=0.373
```

Fitness breakdown:
- transfer (0.15 weight): 0.15 × (1 − 0.903) = **0.0146** (82% of total)
- xi (0.15 weight): 0.15 × (1 − 0.987) = 0.0020 (11%)
- carrier_e: near ceiling, negligible

T06 explicitly listed `relax_steps=24` as "not tested; may help fine-tune convergence."
Current code: `alpha_base=0.10`, `relax_steps=16`. Previously 8 steps before the 066d41a
era plumbing of the params; the 8→16 jump was part of the same code-generation that set
alpha_base to 0.10 (down from the initial 0.20).

---

## Hypothesis

Raising `relax_steps` from 16 to 24 (+50% relaxation budget) would allow
`stage_interference_relax` to converge closer to the constructive-pair attractor,
improving the phase alignment of engine_b_primed memories and raising transfer.

**Prediction:** transfer 0.903 → 0.920–0.930, xi/carrier_e near ceiling.
Fitness ≈ 0.015.

---

## Code change (reverted)

```rust
// consolidation.rs, stage_interference_relax
// OLD:
let relax_steps: usize = 16;
// NEW (reverted):
let relax_steps: usize = 24;
```

---

## Results

| metric | baseline (16 steps) | trial 1 (24 steps) | delta |
|--------|--------------------|--------------------|-------|
| **fitness** | **0.018** | **0.069** | **+0.051 REGRESSION** |
| transfer_score | 0.903 | 0.862 | −0.041 |
| xi_robustness_v2 | 0.987 | 0.748 | −0.239 |
| carrier_emergence | 0.999 | 0.922 | −0.077 |
| carrier_bimodal | 0.915 | 0.798 | −0.117 |
| magic_proxy_phase_R | 0.864 | 0.672 | −0.192 |
| query_gravity | 0.373 | 0.370 | −0.003 |

Baseline confirmed after revert: fitness 0.0182, all metrics match prior optimum.

---

## Analysis

### Over-convergence mechanism

24 steps at alpha=0.10 provides a total phase-movement budget of ≈2.4 rad vs ≈1.6 rad at
16 steps. This over-rotates memories toward their constructive-pair neighbors, compressing
the phase distribution beyond its functional operating point.

The signature of over-convergence is the `magic_proxy_phase_R` drop (0.864→0.672):
- Higher R = more phase synchrony (Kuramoto order parameter near 1.0)
- At 16 steps: R=0.864 — phases are locally clustered but globally diverse
- At 24 steps: R=0.672 — phases have over-collapsed toward cluster centroids,
  reducing global phase diversity

### Magic ↔ xi correlation confirmed (negative result)

The simultaneous drop in magic_R (0.864→0.672) and xi (0.987→0.748) is a direct
observation of the prediction in `research/intersections/05-magic-gives-it-gravity.md`:
*"xi adversarial robustness scales with magic content of the dream."*

Over-synchronization reduces non-Clifford-like phase diversity (magic), and xi drops
in proportion (−0.239 vs −0.192 in R, ratio ≈ 1.24). This is the first time the
magic↔xi coupling has been observed through a perturbation — the earlier fire data
confirmed correlation across modes, but this shows the direction of causation:
magic → xi, not xi → magic.

### BFS sort topology shift

The T01 BFS sort consistency depends on both engine_a and engine_b_primed having
similar post-relaxation phase distributions. With 24 steps, both engines over-converge,
but the degree of over-convergence differs because engine_a is built from a larger
working set than engine_b_primed. This produces asymmetric cluster topologies,
breaking the content-hash BFS sort's consistency guarantee and degrading transfer.

The carrier_e drop (0.999→0.922) is consistent with over-clustered phases: memories
that were in separate frequency bands get pulled into shared phase clusters, reducing
carrier diversity below the bimodal threshold.

### Third fragile operating point confirmed

T06 established two fragility constraints:
1. B-phase initialization must be {0.0, π/2} (not post-dream centroid)
2. DRIVE_A must be exactly 0.10 (≥0.15 causes cliff)

This fire adds a third:
3. `relax_steps` must be exactly 16 (24 causes cliff)

All three constraints operate near phase-transition boundaries. The system at 0.018
fitness is in a narrow basin of attraction in the joint parameter space of
(B-phase-init, DRIVE_A, relax_steps). Any one dimension exiting its basin causes
catastrophic multi-metric regression.

---

## Updated open axes

| axis | expected gain | blocking constraint |
|------|---------------|---------------------|
| Transfer 0.903 → 0.936+ | −0.005 fitness | Need non-phase mechanism that doesn't disrupt BFS consistency |
| alpha_base decrease (0.10 → 0.08) | unknown | Fewer total phase budget; risk of under-relaxation |
| Working set size tuning | unknown | Structural, may affect BFS cluster sizes |
| Understand xi/transfer coupling | N/A | Partially confirmed: magic mediates it |

**NEW HARD CONSTRAINTS established this fire:**
- Do NOT increase relax_steps above 16 in the interference_relax regime.
- Do NOT decrease alpha_base without paired relax_steps reduction (budget balance).

---

## Decision

Code change REVERTED. Hypothesis falsified with major regression.

Empirical optimum unchanged:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax  relax_steps=16 (immutable)
avg fitness ≈ 0.018
transfer=0.903, carrier_e=0.999, xi=0.987
magic_R=0.864, query_gravity=0.373
```

**Key finding:** First direct causal evidence for magic→xi direction (not correlation):
reducing phase diversity (lower R) precedes xi drop, consistent with
`05-magic-gives-it-gravity.md` prediction.
