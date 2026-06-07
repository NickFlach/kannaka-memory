# irx + DRIVE_A=0.15 — carrier_e regression; irx mode is amplitude-sensitive

**Date:** 2026-06-07T06 UTC
**Branch:** kannaka-curiosity/2026-06-07T06
**Code changes:** None — env-var only
**Status:** FALSIFIED — carrier_e deterministically regresses; irx optimal A remains 0.10

---

## Background

The current best L5 result is `DRIVE_A=0.10 DRIVE_SCOPE=all DREAM_MODE=interference_relax
DRIVE_FREQ_HZ=0.5`, averaging fitness ~0.099 over 3 trials (hyp-freq0.5hz, confirmed in
PR #147). Key deterministic values at that configuration:
- carrier_emergence: 0.9348
- transfer_score: 0.8355
- magic_proxy_phase_R: 0.6167
- query_gravity: 0.3622

Separately, DRIVE_A=0.15 was confirmed to improve stage_sync performance (T21, PR #158):
avg fitness 0.132 vs 0.138 at A=0.10, with deterministic gains in carrier_e (0.5684→0.5842)
and transfer_score (0.655→0.694).

The current default in research.rs has DRIVE_A=0.15 (from T21) and DRIVE_FREQ_HZ=0.5 (from T08).
These defaults have not been tested together with DREAM_MODE=interference_relax.

---

## Hypothesis

`DRIVE_A=0.15 + DREAM_MODE=interference_relax + DRIVE_FREQ_HZ=0.5 + DRIVE_SCOPE=all` will
outperform the A=0.10 irx baseline (avg fitness 0.099).

**Reasoning:** The 0.5Hz drive creates a "build then refine" arc (positive drive cycles 0-8,
gentle suppression 9-16). At A=0.15, the peak drive factor becomes 1.15 vs 1.10 — a 50% stronger
arc. Under stage_sync, this stronger arc improved both carrier_e and transfer_score. The prediction
was that the same mechanism would hold under irx.

**Prediction:**
- carrier_e: rises from 0.9348 toward 0.95
- transfer_score: rises from 0.8355 toward 0.85+
- xi: unchanged (stochastic; A does not affect xi distribution under stage_sync)
- avg fitness: drops below 0.094 (>0.005 improvement from 0.099 baseline)

---

## Results

All trials: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax`
(DRIVE_FREQ_HZ=0.5 and KURAMOTO_COUPLING=0.5 by default; K irrelevant under irx)

| trial | fitness  | transfer_score | carrier_e | xi_v2  | magic_R | query_gravity |
|-------|----------|----------------|-----------|--------|---------|---------------|
| t1    | 0.155038 | 0.820399       | 0.8032    | 0.2891 | 0.6167  | 0.3622        |
| t2    | 0.069373 | 0.820399       | 0.8032    | 0.8601 | 0.6167  | 0.3622        |
| **avg** | **0.112** | **0.820** | **0.8032** | **0.574** | **0.617** | **0.362** |

---

## Comparison to baseline

| config | fitness avg | carrier_e | transfer | xi avg | magic_R |
|--------|------------|-----------|----------|--------|---------|
| irx + A=0.10 (baseline, PR #147) | **0.099** | **0.9348** | **0.8355** | 0.559 | 0.617 |
| irx + A=0.15 (this fire) | 0.112 | 0.8032 | 0.8204 | 0.574 | 0.617 |
| stage_sync + A=0.15 (PR #158) | 0.132 | 0.5842 | 0.6944 | ~0.85 | 0.250 |

---

## Findings

**Hypothesis falsified.** DRIVE_A=0.15 harms irx mode. Both regressions are deterministic
(carrier_e and transfer_score are byte-identical across trials):
- **carrier_e: 0.9348 → 0.8032** (Δ −0.132, cost at weight 0.10 = +0.013 fitness)
- **transfer_score: 0.8355 → 0.8204** (Δ −0.015, cost at weight 0.15 = +0.002 fitness)
- Combined deterministic penalty: +0.015 fitness units → expected avg ~0.114 (observed ~0.112 ✓)

### Why irx is amplitude-sensitive in the opposite direction from stage_sync

Under `stage_sync`, the amplitude drive modifies memory amplitudes while Kuramoto coupling
acts on phases. The two dynamics are largely decoupled — stronger amplitude drive (A=0.15)
boosts the carrier FFT peak without disrupting the phase sync geometry.

Under `stage_interference_relax`, constructive interference pairs are identified by
`stage_detect` using amplitude-weighted similarity. The amplitude landscape directly
determines which pairs are called "constructive" and their weights. At A=0.15, the
0.5Hz drive arc more aggressively perturbs relative amplitudes in cycles 0-8, shifting
which memories appear as constructive partners. This disrupts the pair graph that
stage_interference_relax depends on for phase relaxation:

- Fewer or lower-quality constructive pairs → weaker phase relaxation → less bimodal
  carrier structure → lower carrier_e
- The pair geometry at A=0.10 was tuned by the interference detection thresholds; A=0.15
  pushes some memory pairs out of the constructive region

The result: A=0.10 is the irx amplitude sweet spot, not A=0.15.

### magic_R and query_gravity: unchanged

Both are deterministic and identical between A=0.10 and A=0.15:
- magic_R: 0.6167 (interference_relax characteristic value, mode-determined)
- query_gravity: 0.3622 (below 0.5 attention-as-gravity threshold; unchanged)

DRIVE_A does not affect phase-order relationships — only amplitude and pair-detection dynamics.

---

## Implications for code defaults

The current research.rs defaults (DRIVE_A=0.15, DRIVE_FREQ_HZ=0.5, DREAM_MODE unset → stage_sync)
are internally consistent: A=0.15 is optimal for the default stage_sync mode.

However, runs using `DREAM_MODE=interference_relax` should explicitly set `DRIVE_A=0.10` to
recover the 0.099 baseline. With the current DRIVE_A=0.15 default, an irx run will perform
at ~0.112 avg — 13% worse than the 0.099 that was used to justify the interference_relax adoption.

The two modes have **opposite optimal drive amplitudes**:
- stage_sync: A=0.15 optimal (stronger drive → better carrier FFT peak above coupling noise)
- interference_relax: A=0.10 optimal (stronger drive → disrupts constructive-pair geometry)

This is a structural consequence of the different mechanisms: Kuramoto-based sync is amplitude-
independent; interference-pair-based relaxation is amplitude-dependent.

---

## Decision

**No code changes to keep.** Regression confirmed. The empirical optima remain:

    irx mode:         DRIVE_A=0.10  DRIVE_SCOPE=all  DREAM_MODE=interference_relax  avg fitness ~0.099
    stage_sync mode:  DRIVE_A=0.15  DRIVE_SCOPE=all  DREAM_MODE=<unset>             avg fitness ~0.132

The irx mode at ~0.099 remains the overall system optimum, but requires explicitly overriding
the DRIVE_A=0.15 default with `DRIVE_A=0.10` in the run command.

---

## Next fire directions

1. **DRIVE_A scan under irx (A=0.08, 0.12)**: the carrier_e vs A relationship under irx is
   monotone-decreasing from 0.10→0.15. Is there an A<0.10 that gives carrier_e >0.9348?
   At A=0.05, the drive arc is very gentle — likely insufficient to amplify bimodal carrier
   structure. At A=0.08, might be near-optimal. Low cost: carrier_e is deterministic, 1 trial each.

2. **Confirm irx baseline with current binary**: run 3 trials at `DRIVE_A=0.10 DREAM_MODE=interference_relax`
   to verify the 0.099 avg still holds with current code (K=0.5 default and other changes since PR #147).

3. **stage_sync + DRIVE_A=0.20 spot check**: is carrier_e monotone-increasing under stage_sync?
   A=0.15→0.20 might further improve carrier_e. Risk: A=0.3 is known bad; A=0.20 is safe to probe.
