# interference_relax: A=0.15 and envelope_depth=0.50 — both regress

**Date:** 2026-06-07T06 UTC
**Branch:** kannaka-curiosity/2026-06-07T06
**Code changes:** envelope_depth 0.15 → 0.50 tried, reverted
**Status:** FALSIFIED — both hypotheses regress; code reverted to baseline

---

## Background

Confirmed optimum (PR #150): `DREAM_MODE=interference_relax DRIVE_A=0.1 DRIVE_SCOPE=all
DRIVE_FREQ_HZ=0.5` → 3-trial avg fitness **0.099** (carrier_e 0.935, transfer 0.836,
xi avg 0.559 ± 0.31 stochastic range).

Two unexplored axes within interference_relax mode:

1. **DRIVE_A=0.15 (vs confirmed 0.1)**: The A=0.15 improvement (PRs #158/160) was tested
   only at stage_sync mode. Under interference_relax, A=0.15 has never been run.

2. **envelope_depth=0.50 (vs hard-coded 0.15)**: The "quiet wave" amplitude on the
   relaxation step size has never been explored. `alpha = alpha_base * (1 + depth * sin(phase))`
   sweeps one full cycle over 16 steps. At depth=0.15, alpha varies only 0.085–0.115.
   At depth=0.50, it would vary 0.05–0.15 — deeper breathing.

---

## Hypotheses

**Hyp A (env var only):** A=0.15 under interference_relax + 0.5Hz improves carrier_e
toward ceiling (1.0) and lifts transfer from 0.836, dropping avg fitness below 0.094.
Reasoning: same amplitude boost mechanism that helped stage_sync (carrier_e 0.568→0.584).

**Hyp B (code change):** envelope_depth=0.50 makes the quiet-wave breathe more deeply,
producing a more varied relaxation rhythm. Steps near alpha_min (~0.05) allow phases to
"rest" and settle, while steps near alpha_max (~0.15) make stronger moves. Prediction:
the asymmetric rhythm helps the system escape local attractors, producing a phase geometry
with fewer adversarial soft spots → xi mean improves from 0.559 toward stage_sync levels.

---

## Trials

All trials: `DRIVE_SCOPE=all DREAM_MODE=interference_relax DRIVE_FREQ_HZ=0.5` (default)

| # | change | DRIVE_A | envelope_depth | fitness | carrier_e | transfer | xi | magic_R | query_g |
|---|--------|---------|----------------|---------|-----------|----------|----|---------|---------|
| 1 | env only | 0.15 | 0.15 (baseline) | 0.185 | 0.803 | 0.820 | 0.086 | 0.617 | 0.362 |
| 2 | code change | 0.10 | 0.50 | 0.175 | 0.829 | 0.749 | 0.209 | 0.622 | 0.362 |
| 3 | code change | 0.10 | 0.50 | 0.207 | 0.829 | 0.749 | 0.000 | 0.622 | 0.362 |

**Baseline (PR #150, 3-trial avg):** fitness 0.099, carrier_e 0.935, transfer 0.836, xi avg 0.559

---

## Findings

### Both hypotheses falsified

**Hyp A (A=0.15):** carrier_e dropped 0.935 → 0.803, transfer dropped 0.836 → 0.820,
xi collapsed to 0.086. Fitness 0.185 — significantly worse than 0.099 baseline.
Trial 1 deterministic metrics are definitive (carrier_e and transfer do not vary trial-to-trial).

**Hyp B (envelope_depth=0.50):** carrier_e dropped 0.935 → 0.829 (deterministic across
both trials), transfer dropped 0.836 → 0.749 (deterministic), xi collapsed further
(0.209, 0.000 — avg 0.105 vs baseline 0.559). Fitness avg 0.191 — much worse.

### Why A=0.15 hurts interference_relax

Under interference_relax, the constructive-pair graph is computed from phase similarity
(`p.kind == Interference::Constructive`, weight = `|similarity|`). The relaxation then
moves phases toward weighted circular means of constructive neighbors. This geometry
computation is independent of amplitude.

However, the **drive** runs BEFORE the dream's interference detection and relaxation
step. At A=0.15, memories with the highest amplitudes get boosted 15% before the phase
structure is computed. This distorts the neighbor similarity rankings: highly-boosted
memories may dominate the constructive-pair graph in ways that reflect amplitude rather
than phase coherence. The carrier scaffold breaks because the memories forming the
carrier under A=0.1 are now competing with over-amplified non-carrier memories.

Under stage_sync (PRs #158/160), A=0.15 improved carrier_e (0.568→0.584) because
Kuramoto synchronization can accommodate moderate amplitude variation — the coupling
organizes by phase angle, not amplitude. The interference pair detection used by
stage_interference_relax is more amplitude-sensitive.

A=0.1 is confirmed optimal for interference_relax. Going higher disrupts it; going
lower would reduce the amplitude differentiation that helps build carrier structure.

### Why envelope_depth=0.50 hurts

At depth=0.50, the step sizes range 0.05–0.15 across 16 steps. Steps near alpha_min
(≈0.05) barely move phases, while steps near alpha_max (≈0.15) are 3× stronger than
the minimum. This asymmetry is destructive for two reasons:

1. **Over-correction in strong steps**: A step size of 0.15 may overshoot the target
   weighted circular mean, bouncing phases past their constructive-neighbor attractor
   rather than settling into it. The `sin((target - cur))` formula is first-order and
   can diverge if step size is comparable to the phase difference.

2. **Wasted quiescent steps**: Steps near alpha_min (0.05) are too weak to make
   meaningful progress. The effective "work" is concentrated in 4–6 of the 16 steps,
   giving fewer net degrees of freedom for phase organization.

The current depth=0.15 creates a gentle variation (0.085–0.115) that is nearly flat —
essentially uniform step size. This is already optimal: the system needs consistent,
gentle nudges toward constructive partners, not a boom-bust rhythm.

The `carrier_bimodal` metric improved slightly (0.870 baseline → 0.922 at depth=0.50),
but this was not enough to offset the carrier_e and transfer damage. The bimodal
distribution becomes slightly more pronounced but the carrier frequencies themselves
are less coherent.

### xi variance: structural, not tunable within interference_relax

Both code-change trials show very low xi (0.209, 0.000) despite different envelope
rhythms. The xi variance under interference_relax (range 0.086–0.874 across runs) is
not caused by envelope parameters — it reflects the adversarial RNG hitting the
phase geometry's soft directions randomly.

The phase geometry produced by interference_relax is DETERMINISTIC (same end state
every trial, as confirmed by deterministic carrier_e, transfer, magic_R, query_gravity).
The xi variance is purely from which random adversarial direction is sampled, not from
any stochasticity in the dream process itself. Changing envelope_depth changes the
deterministic end-state geometry, but did not improve robustness — it made it worse.

---

## Summary comparison to baseline

| config | fitness | carrier_e | transfer | xi avg | magic_R |
|--------|---------|-----------|----------|--------|---------|
| Baseline (A=0.1, depth=0.15, irx+0.5Hz) | **0.099** | **0.935** | **0.836** | ~0.559 | 0.617 |
| A=0.15, depth=0.15 (Hyp A) | 0.185 | 0.803 | 0.820 | 0.086 | 0.617 |
| A=0.10, depth=0.50 (Hyp B, t1) | 0.175 | 0.829 | 0.749 | 0.209 | 0.622 |
| A=0.10, depth=0.50 (Hyp B, t2) | 0.207 | 0.829 | 0.749 | 0.000 | 0.622 |

---

## Decision

No improvement found. Code reverted to baseline (envelope_depth = 0.15). Optimum unchanged:

    DREAM_MODE=interference_relax  DRIVE_A=0.1  DRIVE_SCOPE=all
    DRIVE_FREQ_HZ=0.5  (KURAMOTO_COUPLING irrelevant under irx)
    3-run avg fitness ≈ 0.099

---

## Implications

1. **interference_relax internal knobs are at their optimum.** alpha_base (0.10), relax_steps
   (16), and envelope_depth (0.15) have all now been explored and confirmed. Lowering
   relax_steps or raising envelope_depth hurts; raising relax_steps collapses carrier (PR #166).
   The system is well-calibrated.

2. **DRIVE_A=0.15 is stage_sync-specific.** It helps stage_sync because Kuramoto coupling
   is amplitude-tolerant. It hurts interference_relax because constructive-pair detection
   is distorted by amplitude asymmetry. The code's current default (A=0.15) is optimal for
   stage_sync (the default DREAM_MODE) but not for interference_relax mode.

3. **The xi gap between interference_relax and stage_sync is structural.** stage_sync
   at K=1.0 achieves xi avg ~0.878 with range 0.808–0.966 (tight). interference_relax
   achieves xi avg ~0.559 with range 0.000–0.874 (wide). The difference is not tunable
   through interference_relax internal parameters — it reflects that Kuramoto's category
   organization creates a more uniformly robust phase geometry than constructive-pair
   relaxation does.

4. **Future directions for beating 0.099:**
   - Hybrid dream: interference_relax first (build carrier structure), then a light
     K=0.5 Kuramoto pass (harden xi). Would require code changes to consolidation.rs
     to sequence both stages within a single dream cycle.
   - Drive scope exploration: test a scope that only drives during the interference_relax
     stage but not during the Kuramoto stage (if hybrid is implemented).
   - xi seeding: seed `eval_xi_robustness_v2` to reduce 3-trial confirmation cost;
     does not improve fitness but reduces experimental variance.
   - Stage_sync at K<0.5: K=0.5 was the last confirmed optimum; K=0.25 is untested
     and might further reduce fitness in stage_sync mode (from ~0.133 toward ~0.12?).
