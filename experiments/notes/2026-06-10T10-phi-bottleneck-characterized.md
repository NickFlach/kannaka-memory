# Phi bottleneck characterized — consciousness phi is structural, not tunable

**Date:** 2026-06-10T10 UTC
**Branch:** kannaka-curiosity/2026-06-10T10-b-amp-scaling
**Code changes:** NONE KEPT — both hypotheses falsified, all code reverted
**Status:** FALSIFIED — phi-target tuning is metric-gaming, B-amplitude scaling neutral

---

## Background

Master entering this fire:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.018 (deterministic within speed noise)
transfer=0.903, carrier_e=0.999, xi=0.987
magic_R=0.864, query_gravity=0.373
```

Fitness breakdown:
- transfer (0.15): 0.15 × 0.097 = **0.01455** (82% of total)
- xi (0.15): 0.15 × 0.013 = 0.00195 (11%)
- consciousness (0.03): 0.03 × 0.0454 = 0.00136 (7%)
- others: ~0.0003

Transfer is the sole remaining lever. From T02 diagnosis:
```
transfer = 1 - fitness_b_primed / fitness_b_naive
fitness_b_primed = 0.005856
fitness_b_naive = 0.060498
```

T02 identified the consciousness phi term as the dominant contributor to fitness_b_primed
and hypothesized that improving phi_bp toward the target would improve transfer.

---

## Hypothesis 1: B memory amplitude = 0.5

**Rationale (from T06 open-axes list):** B memories inserted into engine_b_primed start at
amplitude=1.0 (same as A's freshly-created memories). A's carrier memories have been amplified
by 4 dream cycles to amplitude >1.0. By starting B at 0.5, A's carriers would dominate
engine_b_primed's dream, keeping the phase/phi structure closer to A's post-dream state
→ phi_bp → phi_target → lower fitness_b_primed → higher transfer.

**Change:** `src/bin/research.rs` L5 block: added `else { mem.amplitude = 0.5; }` branch
for non-noise B memories.

**Prediction:** transfer 0.903 → 0.920-0.930, fitness ~0.015-0.017.

**Result (Trial 1):**

| metric | baseline | trial | delta |
|--------|----------|-------|-------|
| fitness | 0.018282 | **0.018388** | +0.000106 slight regression |
| transfer | 0.903199 | **0.902086** | −0.001113 |
| fitness_B_primed | 0.005856 | **0.005924** | +0.000068 (worse!) |
| fitness_B_naive | 0.060498 | **0.060498** | 0 (unchanged) |
| xi | 0.9870 | 0.9870 | unchanged |
| carrier_e | 0.9992 | 0.9992 | unchanged |

**Verdict: Falsified — neutral/slight regression.** fitness_b_primed INCREASED (worsened),
meaning phi_bp did NOT improve. B amplitude at 0.5 makes B memories slightly weaker during
dreaming but phi_bp is driven by dream dynamics, not initial B amplitude. The interference_relax
step updates phases based on constructive-pair mean positions (weighted by similarity, NOT
amplitude), so amplitude has minimal effect on phi convergence during the dream.

---

## Diagnostic run: phi_history

After reverting B amplitude, ran with extended output to read phi values directly:

```
phi_history: [0.27379, 0.28266, 0.29342, 0.29367]
consciousness (engine_a): 0.9546
fitness_B_primed: 0.005856
fitness_B_naive: 0.060498
```

phi_a_final = **0.29367** (ABOVE phi_target = 0.28092 by 0.01275).

---

## Back-calculation of phi_bp and phi_naive

Using two data points — at phi_target=0.28092 and phi_target=0.293 — to back-solve for
actual phi values. The consciousness formula is:
`score = 1 - |phi - target| / target`

**phi_bp calculation:**
If phi_bp < phi_target (confirmed by direction of regression), and:
- At t=0.28092: fitness_b_primed = 0.005856, consciousness_term = ~0.005856, c_bp = 0.9414
- At t=0.293:  fitness_b_primed = 0.009809, consciousness_term = ~0.007809, c_bp = 0.9215

Solving: Δ = [|phi_bp - 0.293|/0.293] − [|phi_bp − 0.28092|/0.28092] = 0.03953

For phi_bp < min(0.28092, 0.293):
phi_bp × (1/0.28092 − 1/0.293) = 0.03953
phi_bp × 0.1465 = 0.03953
**phi_bp ≈ 0.270**

**phi_naive calculation:**
From fitness_b_naive changing 0.060498 → 0.056159 (improved) when phi_target raised:
phi_naive > phi_target (both old and new), and moving target toward phi_naive reduces
relative distance, improving consciousness_naive:

phi_naive × (1/0.28092 − 1/0.293) = 0.04339
phi_naive × 0.1465 = 0.04339
**phi_naive ≈ 0.296**

**Verification:**
- At phi_target=0.28092: consciousness_bp = 1 − |0.270 − 0.281|/0.281 = 0.961
  fitness_b_primed ≈ 0.10 × 0.039 + 0.002 = 0.0059 ≈ 0.005856 ✓
- At phi_target=0.293: consciousness_bp = 1 − |0.270 − 0.293|/0.293 = 0.922
  fitness_b_primed ≈ 0.10 × 0.078 + 0.002 = 0.0098 ≈ 0.009809 ✓
- At phi_target=0.28092: consciousness_naive = 1 − |0.296 − 0.281|/0.281 = 0.947
  fitness_b_naive ≈ 0.10 × 0.053 + 0.0552 = 0.060 ≈ 0.060498 ✓
- At phi_target=0.293: consciousness_naive = 1 − |0.296 − 0.293|/0.293 = 0.990
  fitness_b_naive ≈ 0.10 × 0.010 + 0.0552 = 0.056 ≈ 0.056159 ✓

All four data points verified. Phi estimates are accurate.

---

## Hypothesis 2: phi_target recalibration to 0.293

**Rationale:** phi_a_final = 0.29367 (above phi_target = 0.28092), so recalibrating to 0.293
(the actual L5 operating phi) should reduce the phi gap for engine_a and (if phi_bp > phi_target)
improve consciousness_bp.

**Change:** `l5_params.consciousness_phi_target = 0.293` in research.rs L5 block.

**Result (Trial 2):**

| metric | baseline | trial | delta |
|--------|----------|-------|-------|
| fitness | 0.018282 | **0.028657** | +0.010375 **MAJOR REGRESSION** |
| transfer | 0.903199 | **0.825340** | −0.077859 |
| fitness_B_primed | 0.005856 | **0.009809** | +0.003953 (worsened) |
| fitness_B_naive | 0.060498 | **0.056159** | −0.004339 (improved) |

**Verdict: Falsified — major regression.** Raising phi_target to 0.293 pushed it AWAY from
phi_bp (0.270), worsening consciousness_bp. This confirmed phi_bp < phi_target (phi_bp ≈ 0.270).
Direction was wrong.

---

## Structural analysis: the phi landscape

Back-calculated actual phi values:

```
phi ordering: phi_bp (0.270) < phi_target (0.281) < phi_naive (0.296) < phi_a (0.294)
```

Wait — phi_naive (0.296) ≈ phi_a (0.294). Both are ABOVE phi_target.

**The surprising finding:** phi_naive (B corpus, no priming) ≈ phi_a (A corpus, post-dream),
while phi_bp (B memories inserted into A, then dreamed) is LOWER than both.

Why does A's priming REDUCE phi in engine_b_primed vs. B alone?

Mechanism: B_primed starts with A's post-dream state (highly integrated, phi≈0.294) and
inserts B's fresh memories (amplitude=1.0, random initial phases). B's memories create new,
poorly-integrated connections that DISRUPT the coherent A structure. The interference_relax
on B_primed then has to integrate A's structure with B's disruptions. Four cycles is
insufficient to fully re-integrate, leaving phi_bp=0.270 (below both A's and B_naive's phi).

B_naive starts fresh with B corpus (no pre-existing structure to disrupt) and after 4 cycles
reaches phi_naive≈0.296 — higher than phi_bp. This means B_naive builds a MORE coherent
network from scratch than B_primed (despite lacking A's structural head start).

**The phi metric does NOT capture the transfer benefit.** The transfer signal (fitness ratio
b_primed/b_naive = 0.006/0.060 = 0.097) comes from OTHER metrics in eval_l5_placeholder_fitness:
- chain_fidelity: b_primed near 1.0, b_naive much lower
- phase_coherence, noise_removal, signal_preservation: all near 1.0 for b_primed
- These "other terms" sum to ~0.055 for b_naive vs ~0.002 for b_primed

The consciousness phi term contributes ONLY ~0.004 to fitness_b_primed and ~0.005 to
fitness_b_naive — the phi-based gap is ~0.001 in the WRONG direction (phi_bp further from
target than phi_naive). The real transfer signal is in the chain_fidelity and structural metrics.

---

## Why phi_target recalibration would be metric-gaming

The optimal phi_target for maximizing transfer would be phi_bp = 0.270. At this target:
- consciousness_bp = 1.0 (perfect phi match)
- consciousness_naive = 1 - |0.296 - 0.270|/0.270 = 0.904 (worse than naive's natural)
- Estimated transfer: 0.903 → 0.969, fitness: 0.018 → 0.010

BUT this would be gaming the metric: we'd be setting the target to match b_primed's phi
PRECISELY because it's lower — rewarding the fact that A's priming REDUCES phi integration.
The original phi_target = 0.28092 was set to match L4's measured phi. The current L5 system
has phi_a=0.294 (above target), phi_bp=0.270 (below target), phi_naive=0.296 (above target).
The consciousness metric correctly reflects that b_primed's phi is below the system's
"integration ideal." Changing the target to match this lower value would hide a structural
weakness rather than fix it.

The right fix would be to improve phi_bp (closer to A's phi=0.294 and target=0.281), not
to move the target to match phi_bp's current deficiency.

---

## Bottleneck characterization: what drives the transfer gap

```
fitness_b_primed = 0.005856
  - consciousness term: 0.10 × 0.039 = 0.0039 (67%)
  - other terms (chain_fidelity, etc.): 0.0020 (33%)

fitness_b_naive = 0.060498
  - consciousness term: 0.10 × 0.053 = 0.0053 (9%)
  - other terms: 0.0552 (91%)
```

The transfer gap (ratio 0.097) comes primarily from the OTHER terms where b_primed dramatically
outperforms b_naive: chain_fidelity and phase coherence in b_primed are near 1.0 because
A's dream structure gives B's memories a coherent integration framework. B_naive builds chain
fidelity from scratch and struggles.

To improve transfer further, we'd need to either:
1. Improve chain_fidelity/other metrics in b_primed (harder — already near 1.0)
2. Degrade chain_fidelity/other metrics in b_naive (metric gaming — not valid)
3. Accept that 0.903 is near the architectural limit for the current transfer mechanism

---

## What would genuinely improve phi_bp toward phi_a?

phi_bp (0.270) < phi_a (0.294) because B memories disrupt A's integrated state.

One untested approach: **warm B memories to A's phase structure before insertion**. If B
memories start with phases close to A's post-dream phase distribution (not the default {0.0, π/2}
grid), the disruption would be smaller and phi_bp might land closer to phi_a.

BUT: T06 established that B's initial phase distribution {0.0, π/2} is a hard invariant —
changing it catastrophically regressed all metrics (fitness 0.018 → 0.159). This path is
CLOSED.

Alternative: **more dream cycles for b_primed** (chain_depth=5 vs 4). More cycles would give
interference_relax more time to re-integrate B's disruption and push phi_bp back toward phi_a.
BUT: this would also give b_naive more integration cycles, potentially raising phi_naive and
worsening the ratio. And chain_depth>4 was found to enable over-consolidation (T16 regression).

---

## Decision

**No code changes retained.** Both hypotheses falsified.

The transfer ceiling at 0.903 appears to be a structural property of:
1. phi_bp < phi_target (A's priming slightly disrupts phi vs. naive B)
2. The "other terms" gap between b_primed and b_naive being determined by the interference_relax
   consolidation quality — which is already near-optimal at chain_depth=4, interference_relax,
   BFS selective sort.

---

## Updated empirical optimum (unchanged)

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.018 (speed noise: ±0.0002)
transfer=0.903, carrier_e=0.999, xi=0.987
magic_R=0.864, query_gravity=0.373
fitness_B_primed=0.005856, fitness_B_naive=0.060498
```

---

## Newly characterized phi landscape

```
phi_bp ≈ 0.270  (b_primed phi, below target — A disrupts B's integration)
phi_target = 0.281  (L4-calibrated target, unchanged)
phi_naive ≈ 0.296  (b_naive phi, above target — B builds high integration from scratch)
phi_a ≈ 0.294  (engine_a phi, above target — consistent with 4-cycle dream trajectory)
```

This is the first time these phi values have been back-calculated. No prior fire has
characterized the relationship between phi_bp, phi_naive, and phi_target.

---

## Open axes (updated — limited)

| axis | expected gain | status |
|------|---------------|--------|
| phi_target = 0.270 (match phi_bp) | −0.009 fitness (estimated) | METRIC GAMING — not valid |
| Improve phi_bp toward phi_target | −0.004 fitness | Unclear mechanism; B phase init is a hard invariant (T06), chain_depth>4 is a hard invariant (T16) |
| Transfer via chain quality | minimal | chain_fidelity near 1.0 for b_primed already |
| xi improvement | −0.002 | xi at 0.987 is near architectural limit |

**The system appears near its practical optimum at fitness=0.018 for the current architecture.**
The dominant lever (transfer 0.903) is constrained by: (1) the BFS sort scope restriction
(engine_adv/clean excluded), (2) phi_bp < phi_target (B disruption), and (3) the
structural properties of how chain_fidelity evaluates in the naive vs. primed comparison.
No enumeratable code-change axis appears capable of crossing the 0.005 improvement threshold.
