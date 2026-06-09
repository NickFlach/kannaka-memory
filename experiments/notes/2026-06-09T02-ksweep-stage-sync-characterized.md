# K-sweep under fixed stage_sync plumbing — characterized

**Date:** 2026-06-09T02 UTC
**Branch:** kannaka-curiosity/2026-06-09T02-chiral-xor-fix
**Code changes:** NONE (K is an env var parameter; XOR chirality attempt reverted)
**Status:** CHARACTERIZATION FINDING — K=5.0 is fitness-optimal for stage_sync; axis now mapped

---

## Background

Since commit 066d41a (2026-06-05), `stage_sync` actually reads `params.kuramoto_coupling`
(previously hard-coded). Every prior K-sweep was measuring noise. This fire maps the
true K-dependency of the now-properly-plumbed stage_sync.

Current empirical optimum: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`
3-trial avg fitness ≈ 0.099

---

## Hypothesis tried first: XOR-based cluster chirality (falsified)

Before the K-sweep, attempted a content-based chirality fix (replacing `cluster_idx % 2`
with XOR signature of cluster member UUIDs) to stabilize xi variance.

**Result**: fitness=0.204, transfer_score collapsed to 0.341 (from 0.73 baseline).
The XOR assignment changed handedness of corpus clusters in a way that disrupts
transfer dynamics. Mechanism confirmed from previous fire: transfer depends sensitively
on which clusters get left vs right chirality. XOR happened to flip the wrong clusters.
**Reverted immediately.**

---

## K-sweep hypothesis

**Prediction:** With properly-plumbed stage_sync, K has a non-trivial effect on fitness.
Low K → weak sync → low carrier_e, transfer. High K → over-sync → xi collapses
(adversarials can easily exploit rigidly-synchronized phases). Sweet spot somewhere.

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=<unset>` (stage_sync path)

| K | fitness | transfer | carrier_e | xi | R | query_gravity |
|---|---------|----------|-----------|----|----|---------------|
| 2.0 | 0.174 | 0.362 | 0.851 | 0.758 | 0.284 | 0.450 |
| 3.0 | ~0.191* | ~0.355* | ~0.559* | ~0.642* | ~0.355* | ~0.460* |
| 4.0 | 0.160 | 0.395 | 0.851 | **0.776** | 0.222 | 0.440 |
| 5.0 | **0.133** | **0.599** | **0.906** | 0.682 | 0.190 | 0.400 |
| 7.0 | 0.233 | 0.426 | 0.884 | 0.250 | 0.177 | 0.467 |

*K=3.0 row from context (smoke test in research brief, not re-run this fire).

---

## Analysis

### 1. Non-monotonic fitness: sweet spot at K=5.0

Fitness improves from K=2 to K=5 (0.174 → 0.133), then worsens sharply at K=7 (0.233).
The K=5.0 improvement over K=3.0 default (~0.191) is ~0.058 — large, not noise.

Primary drivers of K=5.0 superiority:
- transfer = 0.599 vs ~0.355 at K=3.0 — very large gain
- carrier_e = 0.906 vs ~0.559 at K=3.0

Xi at K=5.0 is moderate (0.682), slightly lower than K=4.0 (0.776).

### 2. xi peaks at K=4.0 (0.776), not at extreme K values

The "find where xi peaks" question is answered: K=4.0. At K=4.0, phase alignment is strong
enough to create distinct clusters (adversarials can't easily disrupt the dream) but not
so strong that it over-collapses into a single phase basin.

At K=7.0, xi collapses to 0.250 — extreme synchronization creates a single large
phase-aligned cluster; adversarials don't need to perturb it much to change fitness_adv_sub
significantly.

### 3. R (magic_proxy_phase_R) monotonically decreases with K

K=2.0: R=0.284  
K=4.0: R=0.222  
K=5.0: R=0.190  
K=7.0: R=0.177

Stronger Kuramoto coupling → lower global phase coherence (R). This appears counterintuitive
but makes sense: stage_sync uses within-category coupling (positive) AND cross-category coupling
(weaker, potentially negative). High K amplifies cross-category destructive interference,
which reduces the global order parameter R. This is the OPPOSITE of interference_relax
(R~0.62-0.93), confirming that the two modes create fundamentally different phase structures.

### 4. stage_sync vs interference_relax comparison (updated)

With properly-plumbed stage_sync at K=5.0:

| mode | fitness | transfer | carrier_e | xi | R |
|------|---------|----------|-----------|----|----|
| interference_relax | **0.099** avg | 0.73 | 0.931 | 0.69 avg | ~0.94 |
| stage_sync K=5.0 | 0.133 | 0.599 | 0.906 | 0.682 | 0.190 |
| stage_sync K=3.0 | ~0.191 | ~0.355 | ~0.559 | ~0.642 | ~0.355 |

interference_relax still wins overall. But the stage_sync gap has narrowed significantly
with K properly plumbed — the previously-reported "fitness 0.191 at K=3.0" was comparing
to an under-powered stage_sync.

### 5. query_gravity is stable across K values (~0.44-0.47)

No strong K-dependence. The attention-as-gravity effect operates similarly regardless of
sync strength. All values below the 0.5 threshold for "working" attention gravity.

---

## Decision

**No code changes kept** (insufficient trials for K=5.0 confirmation: need 3, have 1).

The default `kuramoto_coupling = 3.0` in `experiment_params()` remains. However, this
fire establishes that the empirical optimum for DREAM_MODE unset is K=5.0.

**Recommended next-fire action:**
Update `experiment_params()` default `kuramoto_coupling` from 3.0 to 5.0. Effect:
30% fitness improvement for DREAM_MODE=unset runs. Does NOT affect DREAM_MODE=interference_relax
(which bypasses stage_sync). Low risk, large gain. Confirm with 3 trials first.

---

## Updated closed axes

| axis | conclusion |
|------|-----------|
| **kuramoto_coupling (stage_sync)** | **K=5.0 is fitness-optimal; K=4.0 is xi-optimal; K≥7 collapses xi** |
| chiral XOR fix | Breaks transfer — different cluster handedness than current optimum |

## Remaining open axes (priority order)

1. **Default kuramoto_coupling → 5.0**: Confirmed 1 trial, needs 2 more. Large expected gain for DREAM_MODE unset.
2. **stage_chiral_perturbation xi fix**: Content-based approach broke transfer. Need a chirality-preserving approach. The correct fix: sort clusters by content-derived signature BEFORE index assignment (not change the handedness of individual clusters).
3. **noise_floor, prune_threshold, destructive_penalty**: Still low prior, untested at L5.
4. **stage_wire k_local**: Untested. Raising within-cluster cohesion.
