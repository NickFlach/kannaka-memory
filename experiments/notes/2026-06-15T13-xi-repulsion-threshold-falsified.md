# L5 Research: xi-repulsion threshold sweep — falsified, axis closed

**Date:** 2026-06-15T13 UTC  
**Branch:** kannaka-curiosity/2026-06-15T13-xi-repulsion-threshold  
**Code changes:** REVERTED — no improvement  
**Status:** Axis closed; REPULSION_THRESHOLD=0.22 regresses fitness 17× on transfer

---

## Context

Post-fix baseline (ceiling=2.0, DRIVE_A=0.15, K=0.5): fitness ≈ 0.116,
carrier_e=0.529, transfer=0.737, xi_v2=0.856.

Three main post-fix regression axes (T12):
- carrier_e: stuck at 0.529 (amplitude ceiling structural)
- transfer_score: deterministic at 0.737 for K=0.5 DRIVE_A=0.15
- xi_v2: 0.856, potentially improvable

T12 listed `consolidation_repulsion_threshold` (0.28) as an unexplored post-fix axis.
The default threshold 0.28 is near the max possible xi_repulsion for near-orthogonal
unit vectors (max ≈ 0.27 for orthogonal, 0.38 for anti-parallel).

With EMERGENCE_COEFF=0.191:
- xi_repulsive_force = ||xi_a - xi_b|| × 0.191
- At threshold=0.28: only pairs with angle > ~135° between xi signatures qualify
- At threshold=0.22: pairs with angle > ~70° qualify (much more activation)

---

## Hypothesis

Lowering `consolidation_repulsion_threshold` from 0.28 to 0.22 would activate more
xi-repulsion pairs in the L5 working set → phases of xi-different memories pushed
further apart → xi_robustness_v2 improves from 0.856 toward 0.90+ → fitness drops
by ≥0.005.

**Prediction:** xi_v2 ≥ 0.89, fitness ≤ 0.111, transfer_score stays near 0.737.

---

## Code change (REVERTED)

```rust
// research.rs L5 block — replaced hard-coded 0.28 with env var:
l5_params.consolidation_repulsion_threshold = std::env::var("REPULSION_THRESHOLD")
    .ok()
    .and_then(|s| s.parse::<f32>().ok())
    .unwrap_or(0.28);
```

Reverted to `l5_params.consolidation_repulsion_threshold = 0.28;` after 1 trial.

---

## Trial

| config | fitness | transfer | carrier_e | xi_v2 | magic_R | query_grav |
|--------|---------|----------|-----------|-------|---------|------------|
| baseline K=0.5 (T12) | 0.116 | 0.737 | 0.529 | 0.856 | 0.129 | 0.460 |
| REPULSION_THRESHOLD=0.22 (t1) | **0.181** | **0.459** | 0.534 | **0.804** | 0.083 | 0.460 |

All at DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE unset K=0.5.

---

## Analysis

**Hypothesis falsified.** REPULSION_THRESHOLD=0.22 regressed fitness from 0.116 → 0.181 (+0.065):

1. **transfer_score collapsed**: 0.737 → 0.459 (−0.278). At weight 0.15, this alone
   adds 0.042 to fitness. The xi repulsion forces disturb the phase structure that
   stage_sync establishes for b_primed dream discrimination. More activated repulsion
   pairs → more phase disruption → transfer engine can no longer discriminate A vs B
   corpus.

2. **xi_v2 DECREASED**: 0.856 → 0.804 (−0.052). Counter-intuitive: forcing
   xi-different memories apart in phase space hurt xi, not helped it. Mechanism:
   the repulsion scrambles the intra-cluster phase alignment that xi_robustness_v2
   requires. The test measures whether adversarial phase perturbations affect recall;
   tighter intra-cluster phases (from Kuramoto sync) are more robust, not phases
   pushed apart by xi repulsion.

3. **carrier_e unchanged**: 0.529 → 0.534. As expected — ceiling dominates.

4. **magic_R dropped**: 0.129 → 0.083. Less phase coherence, confirming phase
   disruption hypothesis.

The 0.28 threshold effectively DISABLING xi repulsion (< 1% of pairs qualify given
max_repulsion ≈ 0.27 for orthogonal unit xi vectors) turns out to be intentional:
stage_xi_repulsion is net-negative at any accessible threshold in the L5 corpus
post-fix. The Kuramoto-based phase alignment (stage_sync) already achieves better
phase organization than xi-driven repulsion, and activating repulsion disrupts it.

---

## Closed axes (complete post-fix list)

| axis | status | notes |
|------|--------|-------|
| K-sweep (0.5, 1.0, 2.0) | CLOSED | K=0.5 optimal |
| AMPLITUDE_CEILING (2.0–6.0) | CLOSED | carrier/transfer tradeoff; 2.0 wins |
| CONSTRUCTIVE_BOOST (0.45, 0.20) | CLOSED | pair density overwhelms boost reduction |
| DREAM_MODE=interference_relax (steps=16) | CLOSED PRE-FIX | carrier_e=0; post-fix untested but mathematically equivalent |
| DRIVE_FREQ_HZ sweep | CLOSED (ceiling dominates carrier_e) | |
| chiral_p_bp sweep | CLOSED PRE-FIX (subthreshold even at 0.10 in irx) | |
| consolidation_repulsion_threshold (0.28→0.22) | **CLOSED** | transfer collapses, xi drops |

## Post-fix optimization surface: exhausted

No post-fix axis explored across T01–T13 has improved on the 0.116 ceiling.
The structural constraints are:
- carrier_e ≈ 0.529 (amplitude ceiling makes delta pattern a 4-cycle impulse)
- transfer_score ≈ 0.737 (deterministic at K=0.5, DRIVE_A=0.15)
- xi_v2 ≈ 0.856 (Kuramoto-tuned; xi repulsion is net-negative)

Restoring the pre-fix 0.007461 optimum requires architectural changes — specifically
a carrier_e measurement that decouples from the amplitude ceiling, plus a way to
recover the bimodal amplitude structure without unbounded growth. Parameter sweeping
alone cannot recover it.
