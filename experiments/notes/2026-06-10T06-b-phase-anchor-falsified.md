# B-phase anchoring and DRIVE_A sweep — both falsified

**Date:** 2026-06-10T06 UTC
**Branch:** kannaka-curiosity/2026-06-10T06-b-phase-anchor
**Code changes:** NONE retained — both hypotheses reverted.
**Status:** FALSIFIED — two regressions, no improvement.

---

## Background

Current master after T01 (selective BFS sort):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
2-trial avg fitness ≈ 0.018 (fully deterministic)
transfer=0.903, carrier_e=0.999, xi=0.987
magic_R=0.864, query_gravity=0.373
```

Fitness breakdown:
- transfer (0.15): 0.15 × 0.097 = **0.015** (82% of total)
- xi (0.15): 0.15 × 0.013 = 0.002 (11%)
- carrier_e (0.10): ~0.0001 (<1%)
- other: ~0.001 (7%)

Transfer is the sole remaining lever. Anything that doesn't move transfer by ≥0.033 can't
cross the 0.005 fitness improvement threshold.

---

## Hypothesis 1: B-phase anchoring

**Rationale:** When corpus B memories are inserted into engine_b_primed, their phases are
initialized at fixed default values (l4_dense → 0.0, l4_sparse → π/2), regardless of where
engine_a's memories ended up after dreaming. After interference_relax runs on engine_a,
A's memories have moved toward their constructive pair neighbors. B memories, starting at
the *initial* phase positions, might be misaligned with A's *post-dream* phase landscape,
forcing interference_relax in engine_b_primed to reposition B memories rather than
fine-tune them.

**Change:** After `snapshot_engine_for_plasticity(&engine_a)`, compute the circular mean
of engine_a's post-dream phases per frequency band:
- `b_dense_phase_base` = circular mean of A phases where `freq > 1.0`
- `b_sparse_phase_base` = circular mean of A phases where `freq <= 0.5`

Initialize B memories using these bases instead of 0.0 / π*0.5.

**Prediction:** transfer 0.903 → 0.930+, xi/carrier_e unchanged (near ceiling).
Fitness ≈ 0.013.

**Result (Trial 1):**

| metric | baseline | trial | delta |
|--------|----------|-------|-------|
| fitness | 0.018 | **0.159** | +0.141 **CATASTROPHIC** |
| transfer | 0.903 | 0.776 | −0.127 |
| xi | 0.987 | 0.372 | −0.615 |
| carrier_e | 0.999 | 0.714 | −0.285 |

**Verdict: Falsified — catastrophic regression across ALL metrics.**

**Why:** The default B phase initialization (0.0 for dense, π/2 for sparse) is a **load-bearing
invariant** for the BFS sort consistency mechanism introduced in T01.

The T01 BFS sort works because: engine_a and engine_b_primed both apply a content-based
sort to cluster seeds in `find_synchronized_clusters`. This produces CONSISTENT cluster
topologies because both engines' memories have the SAME content strings sorted the same way.

But the interference_relax stage that precedes cluster formation converges each engine's
phases toward its own constructive pair attractor. The attractor depends on the STARTING
phases. When engine_a (built from corpus_a with default phases) dreams, its phases evolve
from the default positions. When engine_b_primed is seeded with B at the POST-DREAM phase
centroid instead of the DEFAULT positions, the two engines' phase landscapes diverge at
initialization, causing their interference_relax attractors to be incompatible, breaking
the BFS sort consistency guarantee.

The xi crash (0.987 → 0.372) is likely a secondary effect: the disrupted b_primed dream
produced different skip-links or hallucinated memories that interfere with the xi eval path
through some mechanism consistent with T00's unexplained xi/transfer coupling.

**Key constraint identified:** B's initial phase distribution {0.0, π/2} is not arbitrary —
it must match engine_b_naive's initial distribution AND be consistent with engine_a's
DEFAULT phase-space initialization for the BFS sort to maintain consistent topologies.
Any initialization other than the default breaks the system.

---

## Hypothesis 2: DRIVE_A=0.15 under interference_relax

**Rationale:** DRIVE_A=0.1 was selected as optimal for the DEFAULT dream mode, not for
interference_relax. The interference_relax mode has different sensitivity to amplitude
modulation. With A=0.15, the constructive pair detection in stage_detect might yield
stronger signal, improving the interference_relax phase alignment and ultimately transfer.

**Change:** Environment variable only: `DRIVE_A=0.15` (vs baseline 0.10). No code changes.

**Prediction:** transfer might improve slightly 0.903 → 0.920. Fitness ≈ 0.016.

**Result (Trial 2):**

| metric | baseline | trial | delta |
|--------|----------|-------|-------|
| fitness | 0.018 | **0.161** | +0.143 **CATASTROPHIC** |
| transfer | 0.903 | 0.757 | −0.146 |
| xi | 0.987 | 0.463 | −0.524 |
| carrier_e | 0.999 | 0.577 | −0.422 |

**Verdict: Falsified — catastrophic regression across ALL metrics.**

**Why:** The system has an abrupt stability cliff between A=0.10 and A=0.15.

At A=0.10: peak drive factor = 1.071, suppression minimum = 0.929 → stable.
At A=0.15: peak drive factor = 1.106, suppression minimum = 0.894 → catastrophic collapse.

A 3.5% stronger suppression at cycle nadir causes carrier_e to drop from 0.999 to 0.577
and xi from 0.987 to 0.463. This suggests:
1. The system operates near a stability boundary at A=0.10.
2. The 5% amplitude suppression minimum is near a phase-transition threshold.
3. The combination of interference_relax phase coherence + BFS sort is fragile to
   amplitude perturbations beyond the operating point.

This aligns with the prior result that A was swept and 0.1 found optimal. The cliff
is sharper in the interference_relax regime than in the default stage_sync regime,
probably because interference_relax is building more intricate phase coherence that
can be destabilized by stronger amplitude oscillations.

---

## Combined learning: system fragility map

Two "innocuous-looking" changes each caused the system to collapse from 0.018 to ~0.160:

| change | regression | mechanism |
|--------|-----------|-----------|
| B phase anchoring | 0.018 → 0.159 | Broke BFS sort topology consistency |
| DRIVE_A 0.10 → 0.15 | 0.018 → 0.161 | Passed amplitude stability cliff |

The current 0.018 optimum is more fragile than it appears. It rests on a precise combination
of: (1) content-sorted BFS with default phase initialization, (2) DRIVE_A exactly 0.10,
(3) interference_relax at alpha=0.10/16 steps. Each of these is near a boundary — changing
any one causes catastrophic collapse, not graceful degradation.

---

## Open axes (updated)

| axis | expected gain | blocking constraint |
|------|---------------|---------------------|
| Transfer 0.903 → 0.929 | −0.004 | Need to improve b_primed fidelity WITHOUT changing B's initial phase or drive amplitude |
| Transfer via more relax steps | unknown | relax_steps=24 not tested; may help fine-tune convergence WITHOUT disrupting phase initialization |
| Transfer via B-memory amplitude scaling | unknown | Could test lowering B memory amplitude so A memories dominate cluster seeding; risk of disrupting xi |
| Understand xi/transfer coupling | N/A | T00 identified coupling mechanism unknown; T01 did not resolve it |

**HARD CONSTRAINTS established this fire:**
- Do NOT change B's initial phase initialization away from {0.0 for dense, π/2 for sparse}.
- Do NOT increase DRIVE_A above 0.10 in the interference_relax regime.

---

## Decision

No code changes retained. Both hypotheses falsified with catastrophic regressions.
Current master at 0.018 is the optimum; two more parameter axes confirmed as constraints.
