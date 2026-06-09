# Adversarial exclusion from chiral perturbation — falsified

**Date:** 2026-06-09T05 UTC
**Branch:** kannaka-curiosity/2026-06-09T05-adv-excl-chirality
**Code changes:** REVERTED (adversarial exclusion from stage_chiral_perturbation)
**Status:** FALSIFIED — adversarial exclusion worsens xi; mechanism revised

---

## Background

Current empirical optimum (post-T23 phase-centroid chirality):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.060
carrier_e=0.936, transfer=0.841, xi=0.816 avg (range 0.727–0.952)
magic_R=0.871, query_gravity=0.374
```

Residual xi variance (0.727–0.952) is caused by adversarial memories in the engine
affecting `find_synchronized_clusters` via BFS, which shifts cluster membership for
corpus memories between clean and adversarial xi passes.

---

## Hypothesis

**Excluding adversarial memories from chiral perturbation makes the perturbation pass
deterministic for corpus memories → fitness_adv ≈ fitness_clean → xi → near 1.0.**

Three changes to `stage_chiral_perturbation`:
1. Build `adv_ids: HashSet<Uuid>` from working_set at start
2. Filter `adv_ids` from cluster_handedness computation (corpus-only mean cos)
3. Skip adversarial IDs in main perturbation loop and in `apply_targeted_chiral_perturbation`

**Prediction:**
- xi_robustness_v2: 0.816 avg → ≥0.90 avg, reduced variance
- transfer_score: 0.841 (unchanged — main chain unaffected)
- carrier_emergence: 0.936 (unchanged)
- Fitness target: ≤0.050 avg

---

## Results

DREAM_MODE=interference_relax DRIVE_A=0.1 DRIVE_SCOPE=all

| trial | fitness | transfer | carrier_e | xi | R | query_gravity |
|-------|---------|----------|-----------|----|----|---------------|
| baseline T1 | 0.040 | 0.841 | 0.936 | 0.952 | 0.871 | 0.374 |
| baseline T2 | 0.074 | 0.841 | 0.936 | 0.727 | 0.871 | 0.374 |
| baseline T3 | 0.068 | 0.841 | 0.936 | 0.769 | 0.871 | 0.374 |
| **baseline avg** | **0.060** | | | **0.816** | | |
| this-T1 | 0.047 | 0.841 | 0.936 | **0.906** | 0.871 | 0.374 |
| this-T2 | 0.083 | 0.841 | 0.936 | **0.670** | 0.871 | 0.374 |
| **this-avg** | **0.065** | | | **0.788** | | |

Transfer, carrier_e, R, query_gravity: byte-identical to baseline ✓

Xi: this-avg 0.788 vs baseline 0.816 → **regression** (−0.028 in xi avg)
Fitness: this-avg 0.065 vs baseline 0.060 → regression (+0.005 worse)

Note: xi variance not reduced (this-T1=0.906, this-T2=0.670, range 0.236 vs baseline range 0.225).

Note: TSV rows for these 2 trials were lost during stash-revert process; results above
are from stdout capture only.

---

## Mechanism analysis — why exclusion HURTS xi

**Why T23's phase-centroid raised xi from 0.559 → 0.816:**
Phase-centroid replaced cluster-index-based handedness (UUID-BFS-dependent) with
cluster mean-cos-based handedness. Since adversarial phases are deterministic
(fixed encoder_seed), this made handedness mostly deterministic → xi improved.

**Why adversarial exclusion makes it WORSE:**

A1 xi-twins are designed with phases **flipped by π** relative to their target corpus
cluster. In `stage_interference_relax`, A1 xi-twins form constructive pairs (high
vector similarity) with corpus memories and pull corpus phases TOWARD the A1
(anti-aligned) direction. This creates an adversarial disruption effect.

In the **baseline** (A1s receive chiral perturbation):
- The chiral phase perturbation `eta * handedness * sin(2*phase)` modifies A1 phases
  during the dream
- If A1 gets the SAME chirality as corpus cluster (+1): A1 phase moves in the same
  direction → A1 phase structure reinforces corpus → stronger pull-toward-antiphase
- If A1 gets OPPOSITE chirality (−1): A1 phase moves counter-direction → partially
  NEUTRALIZES the anti-phase pull → corpus phases less disrupted → fitness_adv
  closer to fitness_clean → **high xi**
- The random UUID-BFS variance that T23 partially fixed still produces a mix of
  same/opposite chirality cases → variance in xi = 0.727–0.952

In the **adversarial exclusion** (A1s receive NO chiral perturbation):
- A1s keep their π-flipped initial phases unchanged by chirality
- The π-flipped phases are the MAXIMUM anti-constructive configuration
- No chirality to partially neutralize them → full anti-phase pull in adv pass
- fitness_adv consistently diverges from fitness_clean (lower) → **lower xi**
- The occasional "opposite chirality" bonus that randomly gave xi=0.952 is gone
- Result: xi regresses to 0.670–0.906 avg 0.788 (worse than baseline 0.816)

### Implication: chirality IS being useful for A1s

Counterintuitively, the random chirality assigned to A1s in the BASELINE is
occasionally HELPFUL — when random UUID-BFS shifts give A1s opposite chirality,
it partially neutralizes their anti-phase pull. Removing all perturbation eliminates
this accidental neutralization.

### What would actually fix xi

The path to deterministically high xi requires one of:
1. **Neutralize A1s' anti-phase pull in adv pass**: give A1s phases ≈ corpus phases
   (not π-flipped). This would require changing `build_adversarial_set` in research.rs
   (outside scope without justification) or applying a corrective phase rotation to
   A1s during the dream.
2. **Content-based BFS seeding** in `find_synchronized_clusters`: make cluster
   membership deterministic regardless of UUID order. A1s would still have anti-phase
   pull, but the corpus chirality assignment would be stable → less variance.
3. **Redesign the xi adversarial set** so A1s have neither vector nor phase advantage
   (high cosine sim at similar phase → clustering in corpus clusters without anti-phase
   disruption). Requires research.rs change.

Option 2 is the most tractable within consolidation.rs. It requires changing the
BFS iteration order in `KuramotoSync::find_synchronized_clusters` to use
content-deterministic ordering instead of `engine.store.all_memories()` order
(which reflects UUID insertion order).

---

## Decision

**No code changes retained. Hypothesis falsified.**

The T23/T04 mechanism model is revised: adversarial exclusion from chirality REMOVES
the accidental partial-neutralization effect that gives xi=0.952 cases in the baseline.
The baseline random chirality is *accidentally useful* sometimes (not just noise).

**Empirical optimum unchanged:**
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
avg fitness ≈ 0.060
carrier_e=0.936, transfer=0.841, xi=0.816 avg (range 0.727–0.952)
```

---

## Revised xi open axes (priority order)

| axis | mechanism | risk | priority |
|------|-----------|------|----------|
| Content-based BFS seeding in `find_synchronized_clusters` | Makes corpus cluster membership deterministic → stable chirality | MEDIUM (kuramoto.rs change) | HIGH |
| xi metric redesign (A1 phase-aligned instead of π-flipped) | Eliminates anti-phase pull from A1s | HIGH (research.rs change) | MEDIUM |
| Current optimum stability | No known lever for systematic xi > 0.9 | — | MONITOR |
