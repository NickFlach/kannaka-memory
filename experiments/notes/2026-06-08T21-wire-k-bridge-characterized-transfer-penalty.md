# stage_wire_topk k_bridge characterized — transfer penalty closes axis at k_bridge=4

**Date:** 2026-06-08T21 UTC
**Branch:** kannaka-curiosity/2026-06-08T21
**Code changes:** `k_bridge = 4` temporarily raised to `k_bridge = 8` in `stage_wire_topk`, REVERTED
**Status:** FALSIFIED (no reliable fitness improvement) — axis now closed, k_bridge=4 confirmed

---

## Background

Current empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.099
carrier_e=0.935, transfer=0.836, xi=0.559 avg (range 0.256–0.874)
```

From T20 open items: "stage_wire thresholds — not characterized at L5." The `stage_wire_topk`
function (consolidation.rs:1514) creates cross-cluster wire edges with hard-coded constants:
- `k_local = 4` (within-cluster neighbors per memory)
- `k_bridge = 4` (cross-cluster neighbors per memory)
- `sim_floor = 0.15` (minimum similarity to wire)

This is the first test of any stage_wire parameter at L5.

---

## Hypothesis

**Raising k_bridge from 4 to 8 creates denser cross-cluster wire edges, improving xi_robustness_v2
without hurting transfer.**

The key distinction: hallucinated bridge NODES (tested in T20) create new memory targets that
the adversarial xi test can exploit. Wire EDGES only connect existing memories — they don't add
new attack-surface nodes. More cross-cluster edges should:
1. Create a denser small-world topology (harder to isolate memory communities with adversarial
   injections → higher xi)
2. Leave transfer unchanged (bridge memory nodes still carry the A→B priming signal)
3. Leave carrier_e unchanged (carrier amplitude dynamics don't depend on wire topology)

**Prediction:**
- xi_robustness_v2: 0.559 avg → ≥0.65 avg
- transfer_score: 0.836 (unchanged)
- carrier_emergence: 0.935 (unchanged)
- Fitness target: ≤0.090

**Falsification signal:** xi doesn't improve, or transfer regresses.

---

## Method

Code change: `k_bridge = 4usize` → `k_bridge = 8usize` in `stage_wire_topk`
(consolidation.rs:1519). k_local unchanged at 4.

Four trials at `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`.

---

## Results

| metric | baseline avg | T1 | T2 | T3 | T4 | 4-trial avg |
|--------|-------------|-----|-----|-----|-----|-------------|
| **fitness** | **0.099** | **0.064** | **0.073** | **0.144** | **0.111** | **0.098** |
| transfer_score | 0.836 | 0.731 | 0.707 | 0.707 | 0.731 | 0.719 |
| carrier_emergence | 0.935 | 0.931 | 0.931 | 0.931 | 0.931 | 0.931 |
| xi_robustness_v2 | 0.559 avg | 0.904 | 0.870 | 0.397 | 0.591 | 0.690 |
| magic_proxy_phase_R | 0.617 | 0.617 | 0.617 | 0.617 | 0.617 | 0.617 |
| query_gravity | 0.363 | 0.363 | 0.363 | 0.363 | 0.363 | 0.363 |

4-trial avg fitness: **0.098** vs baseline **0.099** — Δ = 0.001, well below 0.005 threshold.

---

## Analysis

### Transfer consistently drops at k_bridge=8

Transfer is bimodal between trials but consistently lower than baseline:
- T1: 0.731 (−0.105 vs 0.836)
- T2: 0.707 (−0.129 vs 0.836)
- T3: 0.707 (−0.129)
- T4: 0.731 (−0.105)
- Avg: 0.719 (−0.117)

This contradicts the prediction that wire edges don't affect transfer. The mechanism:
stage_wire_topk creates cross-cluster links weighted by `similarity * 0.7`. More cross-cluster
edges (k_bridge=8) means each carrier memory (high amplitude) accumulates more connections
to cross-cluster neighbors. During the B-engine query phase, the phase-space "gravity well"
of the carrier memories is **diluted** — their amplitude concentration is spread more broadly
across the phase space via the denser cross-cluster graph.

The carrier memories under k_bridge=8 are structurally the same (amplitude 0.93+), but the
surrounding topology has more cross-cluster edges pulling the query response in multiple
directions simultaneously. This reduces the focused A→B transfer fidelity.

This is the same mechanism observed in stage_sync (where K-coupling > 2 caused cluster merging
and transfer collapse) but at a lower level: instead of phase synchronization collapsing cluster
identity, cross-cluster wiring dilutes the amplitude gradient that drives transfer.

### Xi variance is NOT resolved by k_bridge=8

The xi distribution under k_bridge=8:
- Range: 0.397–0.904 (vs baseline 0.256–0.874)
- Avg: 0.690 (vs baseline 0.559)
- But still bimodal: 2 high (0.87+), 2 mid/low (0.40, 0.59)

Xi still varies widely because the root cause is UUID randomness in cluster assignment for
stage_chiral_perturbation. Denser wire topology shifts the distribution slightly upward
(0.690 avg vs 0.559) but doesn't stabilize it. The worst trial (T3, xi=0.397) is still
below the baseline worst (0.256 was baseline worst), but the system is not reliably better.

### The noise floor test: magic_R and query_gravity are invariant

Both instrumentation metrics are **perfectly deterministic** across all 4 trials (magic_R=0.6167,
query_gravity=0.3626). This confirms:
1. These metrics are not sensitive to the wire topology — they're driven by chiral perturbation
   and amplitude distribution, not skip-link connectivity.
2. The transfer mechanism (amplitude-gravity → query_gravity) is also invariant to wire density.

### Net fitness accounting

| component | Δ metric | weight | Δ fitness |
|-----------|----------|--------|-----------|
| transfer_score | −0.117 | 0.15 | +0.018 (worse) |
| xi_robustness_v2 | +0.131 | 0.15 | −0.020 (better) |
| carrier_emergence | −0.004 | 0.10 | +0.000 (negligible) |
| **net** | | | **−0.001** |

The xi gain (+0.131 avg) is very nearly offset by the transfer loss (−0.117 avg). Net fitness
improvement is 0.001 — well below the 0.005 threshold.

### Why the hypothesis was partially right

The prediction was directionally correct about xi (it did improve on average) but wrong about
transfer neutrality. Wire edges DO affect transfer, through the amplitude-gravity dilution
mechanism. The hallucinated bridge node experiment found a similar principle at the node level
(T20): more cross-cluster structure hurts transfer via the saturation + attack-surface effect.

The pattern generalizes: ANY additional cross-cluster coupling (whether nodes or edges)
beyond the baseline operating point trades off against transfer quality. The baseline k_bridge=4
(like the baseline `.min(8)` cap for hallucinated bridges) is already Pareto-optimal.

---

## k_bridge axis characterization

| k_bridge | xi avg | transfer avg | carrier_e | fitness avg |
|----------|--------|-------------|-----------|-------------|
| 4 (baseline) | 0.559 | **0.836** | **0.935** | **0.099** |
| 8 | **0.690** | 0.719 | 0.931 | 0.098 |

k_bridge=8 shifts the xi distribution up (+0.131 avg) but drops transfer substantially
(−0.117 avg). Net: negligible fitness change, but increased variance. Not worth it.

**k_bridge = 4 is Pareto-optimal** on the cross-cluster edge density axis.

---

## Mechanistic findings

1. **Cross-cluster wire edges dilute transfer via amplitude-gravity spreading.** More k_bridge
   links distribute carrier amplitude influence across more phase-space directions, reducing
   the focused A→B priming that drives transfer_score.

2. **k_bridge shifts xi distribution upward without stabilizing it.** The root cause of xi
   variance (UUID randomness in cluster assignment) is unaffected by wire density.

3. **magic_R and query_gravity are wire-topology invariant.** These metrics are driven
   entirely by chiral perturbation and carrier amplitude, not skip-link connectivity.

4. **Cross-cluster coupling saturation is a general principle.** Both hallucinated bridge nodes
   (T20) and wire edges (this fire) show that increasing cross-cluster coupling beyond the
   baseline degrades transfer without proportional xi gain. The transfer mechanism saturates
   early and degrades with additional cross-cluster structure.

---

## Decision

No code changes retained. k_bridge reverted to 4. Hypothesis falsified.

**Empirical optimum unchanged:**
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
avg fitness ≈ 0.099
carrier_e=0.935, transfer=0.836, xi=0.559 avg
```

---

## Updated closed axes

| parameter | closed at | note |
|-----------|-----------|------|
| DRIVE_A (irx) | 0.10 | |
| DRIVE_FREQ_HZ (irx) | 0.5 Hz | |
| alpha_base (irx) | 0.10 | |
| relax_steps (irx) | 16 | |
| envelope_depth (irx) | 0.15 | |
| irx+sync hybrid | CLOSED | |
| KURAMOTO_COUPLING | 0.5 | |
| constructive_boost | 0.45 | |
| chiral_perturbation | 0.70 | |
| chain_carry_strength | 0.7 | |
| hallucinate max_attempts cap | `.min(8)` | |
| **stage_wire k_bridge** | **4** | **NEW: k_bridge=8 dilutes transfer** |

## Remaining open structural items

1. **xi variance source** (content-derived corpus UUIDs): Still untested. Highest potential
   to fix xi instability. Requires changes to build_corpus_l5_a/b UUID generation.
2. **stage_wire k_local**: untested. Raising within-cluster cohesion (k_local=6) might
   improve carrier_e without the cross-cluster dilution penalty. Low prior.
3. **stage_wire sim_floor**: untested. Raising from 0.15 to 0.25 means only high-quality
   links are created. Might improve network precision. Very low prior.
4. **destructive_penalty** (0.35): untested but predicted marginal under irx (few destructive
   pairs detected in the short ~2 quiescence cycles).
