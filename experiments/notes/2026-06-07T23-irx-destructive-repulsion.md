# irx + destructive-pair repulsion — xi↔transfer anticorrelation confirmed

**Date:** 2026-06-07T23 UTC
**Branch:** kannaka-curiosity/2026-06-07T23
**Code changes:** Reverted — no net change to consolidation.rs
**Status:** FALSIFIED (avg 0.118 vs 0.099 baseline)

---

## Hypothesis

`stage_interference_relax` uses only constructive pairs to attract phases together.
Destructive pairs are fully ignored. Adding a repulsive term — pushing each memory's
phase away from the weighted circular mean of its destructive neighbors — should
improve phase separation and xi robustness without disrupting the carrier emergence
driven by constructive attraction.

**Prediction:** xi_v2 mean improves from ~0.559 to ~0.65+, carrier_e and magic_R
roughly stable (~0.935 and ~0.617 respectively), fitness drops from 0.099 → ~0.085.

**Repulsion strength:** `alpha * 0.5` per step (half the constructive pull).

---

## Method

Code change in `stage_interference_relax` (src/consolidation.rs):
1. Partitioned pair loop into constructive attraction (existing) and destructive
   repulsion (new) neighbor maps.
2. In each relax step, after computing `new_phase` from constructive attraction,
   subtracted `(alpha * 0.5) * sin(destructive_mean_phase - new_phase)` — i.e.,
   pushed phase away from the destructive neighbor cluster.

All trials: `DRIVE_A=0.1 DREAM_MODE=interference_relax DRIVE_SCOPE=all DRIVE_FREQ_HZ=0.5`

---

## Results

| trial | fitness | transfer | carrier_e | carrier_bimodal | xi_v2 | magic_R | query_g |
|-------|---------|----------|-----------|-----------------|-------|---------|---------|
| t1    | 0.092   | 0.601    | 0.862     | 0.850           | 0.915 | 0.266   | 0.358   |
| t2    | 0.118   | 0.601    | 0.862     | 0.850           | 0.742 | 0.266   | 0.358   |
| t3    | 0.143   | 0.601    | 0.862     | 0.850           | 0.578 | 0.266   | 0.358   |
| **avg** | **0.118** | **0.601** | **0.862** | **0.850** | **0.745** | **0.266** | **0.358** |

Transfer, carrier_e, magic_R, carrier_bimodal, and query_gravity are byte-identical
across all 3 trials (deterministic under this code change).

---

## Comparison to baseline

Baseline: `DRIVE_A=0.1 DREAM_MODE=interference_relax DRIVE_FREQ_HZ=0.5 DRIVE_SCOPE=all`
(3-trial avg fitness ≈ 0.099, PR #189 confirmed)

| metric | baseline | this fire | delta |
|--------|----------|-----------|-------|
| fitness avg | 0.099 | **0.118** | **+0.019 (regression)** |
| transfer_score | 0.836 | 0.601 | −0.235 |
| carrier_emergence | 0.935 | 0.862 | −0.073 |
| xi_v2 avg | 0.559 | 0.745 | **+0.186** |
| xi range | 0.256–0.874 | 0.578–0.915 | tighter, floor raised |
| magic_R | 0.617 | 0.266 | −0.351 |
| query_gravity | 0.363 | 0.358 | −0.005 |

---

## Analysis

### Fitness arithmetic

The xi improvement (+0.186 × 0.15 weight = −0.028 fitness savings) is fully
offset by the transfer regression (−0.235 × 0.15 = +0.035 cost) and carrier
regression (−0.073 × 0.10 = +0.007 cost). Net: +0.014 regression.
Observed: +0.019. The residual +0.005 is minor metric noise in the 6 remaining
terms (all at 0.02–0.03 weight).

### Why did transfer collapse?

Transfer score measures primed-vs-naive B-engine discrimination: how much the
dream amplifies the primed engine's retrieval advantage. Under irx + 0.5 Hz, the
constructive-pair phase alignment creates a phase scaffold that the primed engine
can query coherently. The destructive pairs that the repulsion term targets are
NOT merely "orthogonal" to the constructive scaffold — they include cross-cluster
phase relationships that the primed engine uses as negative-space contrast for
discrimination.

Repelling phases from destructive neighbors reorganizes the entire phase landscape.
The result (transfer = 0.601) matches the no-irx stage_sync baseline with low K
more than it matches the standard irx baseline — the repulsion term is essentially
undoing the phase consolidation that makes irx+0.5Hz effective.

### Why did magic_R drop (0.617 → 0.266)?

R = Kuramoto order parameter at end of dream. Standard irx achieves R ≈ 0.617 by
clustering phases around constructive-pair hubs. Destructive repulsion actively
pushes these clusters apart, distributing phases more uniformly around the circle.
Uniform phases → R → 0. This confirms the repulsion is working mechanically (phases
are being spread), but the spreading damages transfer coherence.

### Xi floor and ceiling

xi range shifted upward (floor: 0.256 → 0.578, ceiling: 0.874 → 0.915). This is
consistent with better phase separation protecting against the adversarial perturbation
in xi_robustness_v2. But the floor improvement still doesn't rescue fitness because
the transfer regression is weight-equivalent and deterministic.

### Why alpha*0.5 was too strong

With `alpha_base = 0.20`, the repulsion per step = 0.10 at baseline alpha. Over 8
steps, the accumulated repulsion is substantial. The 2:1 ratio (attraction:repulsion)
was chosen conservatively, but destructive pairs significantly outnumber constructive
pairs in most memory layouts (constructive pairs require both high similarity AND
matching frequency band). A weaker ratio (alpha*0.1 or alpha*0.15) might get partial
xi benefit without fully disrupting transfer.

---

## Decision

**Hypothesis falsified.** Code reverted. Empirical optimum remains unchanged:

    DRIVE_A=0.1  DREAM_MODE=interference_relax  DRIVE_FREQ_HZ=0.5  DRIVE_SCOPE=all
    3-trial avg fitness ≈ 0.099

---

## Implications for future fires

1. **xi and transfer_score are anticorrelated under irx destructive repulsion.**
   Destructive-pair relationships carry information that both the xi adversary and
   the transfer discrimination rely on. This is a structural tradeoff, not a tuning
   problem.

2. **Weaker repulsion untested.** `alpha * 0.1` (instead of `alpha * 0.5`) might
   yield a partial xi gain without collapsing transfer. Expected: transfer 0.601 →
   ~0.720 (+0.12), xi avg 0.745 → ~0.65 (−0.10). Net fitness effect unclear —
   would need ≥2 trials to characterize. Borderline worth testing.

3. **magic_R is a leading indicator of transfer disruption.** R dropped from 0.617
   to 0.266 — a clean marker that destructive repulsion was reorganizing the phase
   landscape away from the transfer-supporting configuration. In future irx code
   experiments, R < 0.5 should be treated as a warning sign for transfer regression.

4. **The xi↔transfer anticorrelation suggests two modes want different things.**
   Stage_sync (K=1.0) achieves good transfer but low xi and low R. Irx achieves good
   transfer AND higher R while xi is moderate. Destructive repulsion raises xi but
   destroys both transfer and R. A genuine xi improvement path under irx must not
   touch the constructive-pair phase structure at all.
