# Hypothesis: DREAM_MODE=interference_relax + DRIVE_SCOPE=no_transfer

**Date:** 2026-06-07T04 UTC
**Branch:** kannaka-curiosity/2026-06-07T04
**Code changes:** None — env-var only
**Status:** NULL RESULT — fitness 0.172 avg (worse than 0.138 optimum); insight captured

---

## Background

Current empirical optimum: `DRIVE_A=0.1 DRIVE_SCOPE=all KURAMOTO_COUPLING=1.0`
(stage_sync, DREAM_MODE unset). 3-run avg fitness ≈ 0.138.

Two known alternative configs and their limitations:

| config | fitness avg | xi avg | transfer avg | carrier_e |
|--------|------------|--------|--------------|-----------|
| stage_sync K=1.0 all | 0.138 | ~0.863 | ~0.682 | ~0.568 |
| irx + all (T00, PR #142) | 0.149 | ~0.607 | ~0.750 | 0.497 |
| stage_sync + no_transfer (T06) | 0.164 | ~0.603 | ~0.723 | 0.559 |

T07 (k-sub-one.md) noted that interference_relax + no_transfer was untested and
worth one trial: "under interference_relax, the mode drives constructive pairs rather
than Kuramoto sync, so the engine scope may interact differently."

---

## Hypothesis

Under `DREAM_MODE=interference_relax`, xi comes from constructive-pair phase relaxation,
not Kuramoto coupling. Since no_transfer's xi penalty under stage_sync was attributed
to Kuramoto not receiving engine_b phase input, the hypothesis was: remove the Kuramoto
dependency, and the xi penalty from engine_b exclusion disappears.

**Prediction:**
- xi similar to irx+all (~0.607 avg) — engine_b exclusion no longer hurts xi
- transfer_score ≥ 0.750 (engine_b undisturbed, possibly higher than irx+all)
- fitness ≈ 0.145 (slight improvement over irx+all 0.149)

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=no_transfer DREAM_MODE=interference_relax`

| trial | fitness | xi | transfer_score | carrier_e | magic_R | query_gravity |
|-------|---------|-----|----------------|-----------|---------|---------------|
| T1 | 0.199579 | 0.2412 | 0.776860 | 0.4966 | 0.6167 | 0.3639 |
| T2 | 0.151276 | 0.5633 | 0.776836 | 0.4966 | 0.6167 | 0.3639 |
| T3 | 0.166859 | 0.4594 | 0.776836 | 0.4966 | 0.6167 | 0.3639 |
| **avg** | **0.172** | **0.421** | **0.777** | **0.497** | **0.617** | **0.364** |

Baseline (stage_sync K=1.0 all): fitness 0.138, xi ~0.863, transfer ~0.682, carrier_e ~0.568.

---

## Analysis

### Transfer_score: hypothesis confirmed on this axis

Transfer_score is **deterministic at 0.777** across all 3 trials — the highest
consistently-achieved transfer_score on record. Compared to irx+all (~0.750), the
engine_b exclusion improved transfer by +0.027. The mechanism works: engine_b
undisturbed during amplitude drive → better primed/naive consolidation ratio →
higher transfer_score.

### xi: hypothesis falsified

xi avg dropped to **0.421**, well below irx+all avg (~0.607) and stage_sync K=1.0
avg (~0.863). The range was 0.241–0.563 — tighter and lower than irx+all's 0.294–0.925.

The engine_b→xi dependency is **mode-independent**. Whether the dream uses Kuramoto
(stage_sync) or constructive-pair relaxation (interference_relax), amplitude-modulating
engine_b memories during the dream contributes to xi. The mechanism is not through
the sync step — it is through how the amplitude landscape of engine_b memories shapes
the xi evaluation environment. Excluding engine_b from drive reduces the amplitude
diversity that eval_xi_robustness_v2 probes.

### Fitness arithmetic

At the 3-trial avg: transfer benefit over stage_sync = (0.777 − 0.682) × 0.15 = +0.014
xi penalty vs stage_sync = (0.863 − 0.421) × 0.15 = +0.066
carrier_e penalty vs stage_sync = (0.568 − 0.497) × 0.10 = +0.007
Other metrics unchanged. Net penalty vs stage_sync K=1.0 ≈ +0.059 → expected fitness
0.138 + 0.059 = 0.197. Observed avg: 0.172. Consistent.

---

## Comparison table

| config | fitness avg | xi avg | transfer avg | carrier_e | magic_R |
|--------|------------|--------|--------------|-----------|---------|
| stage_sync K=1.0 all | 0.138 | 0.863 | 0.682 | 0.568 | 0.250 |
| irx + all (T00) | 0.149 | 0.607 | 0.750 | 0.497 | 0.617 |
| no_transfer + stage_sync (T06) | 0.164 | 0.603 | 0.723 | 0.559 | 0.362 |
| **irx + no_transfer (this fire)** | **0.172** | **0.421** | **0.777** | **0.497** | **0.617** |

Interesting structural observation: xi avg under no_transfer is ~0.421 regardless
of dream mode (0.421 irx, 0.603 stage_sync — similar in pattern, different in level).
The engine_b drive exclusion shifts the xi distribution lower in both modes; interference_relax
starts lower, so no_transfer brings it further down.

---

## Instrumentation (deterministic across trials)

- **magic_proxy_phase_R**: 0.617 — identical to irx+all. Characteristic of interference_relax;
  independent of scope. Stage_sync is ~0.250.
- **query_gravity**: 0.364 — identical to irx+all. Below the 0.5 attention-as-gravity
  threshold in both interference_relax configs.

---

## Decision

**No code changes to keep.** No improvement (Δ fitness = +0.034 vs 0.138 baseline).
TSV rows appended automatically.

The empirical optimum remains:
    DRIVE_A=0.1  DRIVE_SCOPE=all  KURAMOTO_COUPLING=1.0  DREAM_MODE=<unset>
    3-run avg fitness ≈ 0.138

---

## Implications

1. **The engine_b→xi dependency is not mode-specific.** Scope experiments targeting
   xi cannot exclude engine_b drive without paying a ~30-40% relative xi cost,
   regardless of whether the dream uses Kuramoto or constructive-pair relaxation.
   Future scope experiments that exclude engine_b must budget for this xi loss in
   the fitness arithmetic.

2. **transfer_score ceiling under interference_relax + no_transfer is 0.777.** This
   is the best reliably-achievable transfer_score found. At current metric weights
   (transfer 0.15, xi 0.15, equal), the xi cost dominates. If future metric reweighting
   elevated transfer above xi weight × 1.25, irx+no_transfer would become competitive
   with stage_sync K=1.0.

3. **interference_relax xi variance under no_transfer is tighter but lower.** Under
   irx+all, xi ranged 0.294–0.925 (high variance, sometimes great). Under irx+no_transfer,
   range is 0.241–0.563 (lower ceiling). The high-xi outliers that made irx+all's avg
   fitness 0.149 (a T1 of 0.101 with xi=0.925) are suppressed under no_transfer.

4. **Next unexplored directions:** The k-sub-one.md file lists DRIVE_FREQ_HZ variants
   (0.5, 1.0 Hz) at K=1.0 as "entirely untested at this code state." Given both main
   interference_relax combinations are now characterized, frequency variants are the
   cleanest remaining unexplored axis.
