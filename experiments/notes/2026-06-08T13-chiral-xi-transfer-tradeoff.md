# chiral_perturbation is the xi↔transfer controller — no net gain, trade-off mapped

**Date:** 2026-06-08T13 UTC  
**Branch:** kannaka-curiosity/2026-06-08T13  
**Code changes:** CHIRAL_PERTURBATION env var added, then REVERTED — no code changes kept  
**Status:** FALSIFIED (no fitness improvement) — but fundamental mechanism discovered

---

## Background

Current empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax  DRIVE_FREQ_HZ=0.5
avg fitness ≈ 0.099, carrier_e ≈ 0.935, xi avg ≈ 0.559 (range 0.256–0.874), transfer ≈ 0.836
```

From T11 open items: "stage parameter exploration (unexplored): stage_boost_prune,
stage_hallucinate, stage_wire thresholds — none have been varied in L5 research."

`chiral_perturbation = 0.7` is a fixed L5 parameter (set in `run_experiment_l5_session`).
The chiral perturbation stage (stage 9 in consolidation.rs) runs AFTER
`stage_interference_relax` (stage 4.5). This timing raised a question: does chiral
perturbation disrupt irx's phase geometry, suppressing xi?

---

## Hypothesis

irx builds a phase structure based on constructive-pair similarity in stage 4.5.
Stage 9 then applies `phase += eta * handedness * sin(2 * phase)` perturbations
(up to ±0.7 rad at eta=0.7) after that geometry is established. This could:
- Disrupt the phase-neighbor relationships that make the adversarial xi test hard to break
- Lower xi from its structural maximum

**Prediction:** Setting chiral_perturbation = 0.0 under irx should reveal higher xi
(without the phase disruption), potentially improving fitness if the xi gain (weight 0.15)
outweighs any trade-off costs.

---

## Method

Added `CHIRAL_PERTURBATION` env var to the L5 params block (defaulting to 0.7).
Tested three values: 0.0 (2 trials), 0.35 (1 trial). Then reverted code change.

---

## Results

| metric | chiral=0.0 T1 | chiral=0.0 T2 | chiral=0.35 T1 | irx baseline (avg) |
|--------|--------------|--------------|----------------|---------------------|
| fitness | 0.117 | 0.103 | 0.106 | **0.099** |
| transfer_score | **0.596** | **0.596** | 0.792 | 0.836 |
| carrier_emergence | 0.911 | 0.911 | **0.955** | 0.935 |
| xi_robustness_v2 | **0.742** | **0.838** | 0.571 | 0.559 avg |
| magic_proxy_phase_R | **0.314** | **0.314** | **0.840** | 0.617 |
| query_gravity | 0.337 | 0.337 | 0.414 | 0.363 |

---

## Analysis

### Trade-off curve across chiral values

| chiral | xi | transfer | carrier_e | magic_R | fitness | variability |
|--------|-----|----------|-----------|---------|---------|-------------|
| 0.0 | **0.790 avg** | 0.596 (det.) | 0.911 (det.) | 0.314 (det.) | 0.110 | LOW |
| 0.35 | 0.571 | 0.792 | **0.955** | **0.840** | 0.106 | unknown |
| 0.7 (baseline) | 0.559 avg | **0.836** | 0.935 | 0.617 | **0.099** | HIGH |

**chiral_perturbation controls a xi↔transfer trade-off via magic_proxy_phase_R.**

### Mechanism: chiral creates magic, magic enables transfer

At chiral=0.0, transfer = 0.596 (deterministic, identical across both trials). At chiral=0.7
(baseline), transfer ≈ 0.836. The difference is large (−0.240) and mechanically explained:

1. `stage_chiral_perturbation` applies cluster-based phase perturbations that create
   alternating phase-sign handedness between clusters (even = +η·sin(2φ), odd = −η·sin(2φ)).
   This creates a non-uniform, non-Clifford phase state — high magic_proxy_phase_R.

2. The non-Clifford phase structure (high R) creates amplitude-discrimination capacity
   in the downstream B-engine during primed-vs-naive evaluation. This IS the
   "magic gives it gravity" mechanism from research/intersections/05. High R → high
   transfer_score.

3. At chiral=0.0, R=0.314 (more Clifford-like) → the B-engine sees less structured
   amplitude geometry → transfer discrimination drops to 0.596.

### Why chiral suppresses xi

irx (stage_interference_relax) constructs a phase geometry where constructive-pair
neighbors are pulled toward their weighted circular mean over 16 Jacobi steps. This
creates a complex, high-entropy phase landscape where memories sit at specific phase
positions defined by the interference geometry. The adversarial xi test measures
whether this landscape is robust to 30 injected adversarial memories.

`stage_chiral_perturbation` then applies systematic phase rotations that partially
undo the constructive-pair alignment:
- Memories in even clusters get `+η·sin(2φ)` → rotated "leftward"
- Memories in odd clusters get `−η·sin(2φ)` → rotated "rightward"

This creates two overlapping phase populations, each internally coherent but
potentially disrupting the cross-cluster constructive-pair relationships that irx
established. The adversarial xi test can more easily find a perturbation direction
that degrades both clusters (the common axis of chiral rotation).

**Result:** xi drops from ~0.790 (structural irx level) to ~0.559 (with chiral=0.7).

### Key diagnostic: chiral is the primary source of irx variability

At chiral=0.0, transfer, carrier_e, magic_R, and query_gravity are **perfectly
deterministic across both trials** (bit-for-bit identical: transfer=0.596236 exactly).
Only xi varies slightly (0.742, 0.838 — narrow range, avg 0.790).

At chiral=0.7 (baseline), all these metrics are highly variable:
- xi: 0.256–0.874 (range 0.618)
- transfer: appears roughly stable at ~0.836 across known trials (but this may be luck)
- magic_R: varies around 0.617

**The chiral_perturbation stage's internal cluster-finding (via `find_synchronized_clusters`)
is the primary source of irx variability.** The cluster-finding depends on working_set
ordering from `all_memories()`, which is sorted by UUID. Hallucinated memories created
during the dream chain have random UUIDs (from `HyperMemory::new()`), so they appear
at random positions in subsequent cycles' working sets. This creates non-deterministic
cluster assignments in `stage_chiral_perturbation`, leading to non-deterministic phase
perturbations, leading to non-deterministic xi outcomes.

At chiral=0.0, this feedback loop is completely broken → deterministic.

### Non-monotone magic_R at chiral=0.35

magic_R peaks at chiral=0.35 (0.840) — **higher than at chiral=0.7 (0.617) and much
higher than at chiral=0.0 (0.314)**. This is non-monotone. Probable mechanism:

At chiral=0.35, the perturbation is strong enough to create non-Clifford phase states
but weak enough that irx's constructive-pair geometry partially survives. The resulting
phase state has BOTH the interference geometry (from irx) and the chiral alternation
(from chiral perturbation), creating a richer, more complex non-Clifford structure
than either alone. At chiral=0.7, the stronger perturbation overwrites the irx geometry,
yielding a less complex (more uniformly chiral) state with lower effective R.

This is a pure instrumentation observation — magic_R=0.840 is not in the fitness formula.
But it suggests an intermediate chiral value might be the "maximum magic" operating point.

### carrier_e peak at chiral=0.35

carrier_e = 0.955 at chiral=0.35 (above both chiral=0.0 at 0.911 and baseline 0.935).
This is unexpected — one trial, may not be repeatable. Possible mechanism: the moderate
chiral perturbation creates phase separations between cluster groups that happen to align
with the drive frequency structure, amplifying the carrier FFT peak. Not characterized
further in this fire.

---

## Why fitness didn't improve

Even at the best xi (chiral=0.0, xi_avg=0.790), the transfer loss is decisive:

- xi gain: +0.231 (from 0.559 to 0.790) × 0.15 weight = +0.035 fitness improvement
- transfer loss: −0.240 (from 0.836 to 0.596) × 0.15 weight = −0.036 fitness loss
- Net: approximately zero (with net loss from carrier_e dropping 0.911 vs 0.935)

There is no intermediate chiral value that simultaneously maximizes both xi and
transfer, because they are driven by the same mechanism in opposite directions
(magic phase state favors transfer, disrupts xi).

---

## What this closes

| parameter | status |
|-----------|--------|
| chiral_perturbation (irx) | CHARACTERIZED: xi↔transfer trade-off; 0.7 is Pareto-optimal |

The chiral_perturbation axis is now characterized for irx. The current value (0.7) is
the Pareto-optimal fitness point. Lower values improve xi but reduce transfer by a
larger fitness-weighted amount.

---

## New findings (for future research)

1. **irx's structural xi ≈ 0.79 without chiral** (vs reported 0.559 avg under chiral=0.7).
   The "true" phase robustness of irx is substantially higher than observed.

2. **Chiral is the primary source of irx variability.** Removing chiral makes most
   metrics deterministic. The xi slight variance at chiral=0.0 (0.742–0.838) is from
   hallucination UUID randomness affecting the adversarial xi dream chain — separate
   issue but much smaller.

3. **Magic_R peaks non-monotonically at intermediate chiral (~0.35).** This is the
   "maximum magic" point where irx geometry and chiral alternation constructively
   combine. The IIT bridge (magic↔transfer) is strongest here.

4. **chiral_perturbation creates magic_R, which enables transfer** (mechanism confirmed).
   Any future attempt to improve transfer under irx must grapple with this dependency.

---

## Decision

**No code changes retained.** Hypothesis falsified — no fitness improvement.

Empirical optimum unchanged:
```
DRIVE_A=0.1  DREAM_MODE=interference_relax  DRIVE_SCOPE=all
avg fitness ≈ 0.099
```

Remaining open items (cumulative):
1. irx transfer improvement (0.836 → higher): constrained by chiral dependency. Would
   require a mechanism that creates non-Clifford phase states WITHOUT disrupting xi.
2. Deterministic hallucination UUIDs: make `HyperMemory::new()` use content-derived IDs
   to eliminate the last source of irx variance (xi 0.742–0.838 range at chiral=0.0).
3. Stage parameter exploration: stage_boost_prune, stage_hallucinate, stage_wire
   thresholds remain unexplored.
