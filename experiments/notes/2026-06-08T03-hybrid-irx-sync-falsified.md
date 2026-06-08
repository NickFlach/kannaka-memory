# Hybrid dream modes falsified: irx_then_sync and sync_then_irx

**Date:** 2026-06-08T03 UTC
**Branch:** kannaka-curiosity/2026-06-08T03
**Code changes:** irx_then_sync and sync_then_irx modes added, then REVERTED — no code changes kept
**Status:** FALSIFIED — both hybrid orderings regress; code reverted to baseline

---

## Background

Empirical optima entering this fire:
- **interference_relax (irx)**: `DRIVE_A=0.1 DREAM_MODE=interference_relax DRIVE_SCOPE=all` → 3-trial avg fitness **0.099** (carrier_e 0.935, transfer 0.836, xi avg 0.559)
- **stage_sync**: `KURAMOTO_COUPLING=0.5 DRIVE_A=0.15 DRIVE_SCOPE=all` → 3-trial avg fitness **0.104** (carrier_e 0.853, transfer 0.655, xi avg 0.878)

Known mode trade-off: irx wins on carrier_e and transfer; stage_sync wins on xi. The last notes (T02) identified "hybrid dream" as the one remaining open direction: run both stages in a single dream cycle to get irx's carrier + transfer alongside Kuramoto's xi.

All irx-internal parameters (alpha_base, relax_steps, envelope_depth) and all stage_sync parameters (K, A, DRIVE_FREQ) have been confirmed at their optima.

---

## Hypothesis

A new `DREAM_MODE=irx_then_sync` that sequences both dream stages within one cycle:
1. `stage_interference_relax` first — builds carrier_e and transfer via constructive-pair phase geometry
2. `stage_sync` second — Kuramoto pass (K=0.5) for xi hardening via category clusters

**Prediction:** stage_sync only modifies `mem.phase`, not `mem.amplitude`, so the carrier_e signature (amplitude-time-series FFT peak at 2 Hz) should survive the second pass. The Kuramoto clustering would harden xi by creating distinct within-category phase regions. Expected outcome: carrier_e ≈ 0.935, transfer ≈ 0.836, xi ≈ 0.7+, fitness below 0.099.

Also tested reversed order: `DREAM_MODE=sync_then_irx` (Kuramoto first, then irx last). Prediction: irx runs last so downstream hallucinate/prune see irx's geometry; carrier_e recovers toward 0.935.

**Code change:** Added two branches to the DREAM_MODE dispatch in `consolidation.rs` stage 4.5 — `irx_then_sync` and `sync_then_irx`. Both branches call both stage functions in sequence. Change is fully reverted.

---

## Trials

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all` (irx optimum A, not 0.15 stage_sync optimum)

| # | DREAM_MODE | fitness | carrier_e | transfer | xi | magic_R | query_g |
|---|------------|---------|-----------|----------|----|---------|---------|
| 1 | irx_then_sync | 0.148 | 0.744 | 0.589 | 0.717 | 0.103 | 0.430 |
| 2 | sync_then_irx | 0.193 | 0.000 | 0.621 | 0.894 | 0.346 | 0.443 |

**Baseline references:**
- irx alone (A=0.1): fitness 0.099, carrier_e 0.935, transfer 0.836, xi avg 0.559
- stage_sync alone (K=0.5, A=0.15): fitness 0.104, carrier_e 0.853, transfer 0.655, xi avg 0.878

---

## Analysis

### Why the prediction was wrong: carrier_e is not amplitude-only

The prediction assumed carrier_e would be unaffected by the Kuramoto pass because stage_sync only writes to `mem.phase`. This was wrong.

carrier_e measures the 2 Hz peak in the amplitude time series across dream cycles. Amplitudes are modified by DOWNSTREAM stages: `stage_hallucinate` (creates cross-cluster bridges, needs amplitude headroom), `stage_prune` (weakens destructive pairs, reduces amplitudes), and `stage_wire` (creates skip links weighted by amplitude). All of these run AFTER stage 4.5 and use the post-4.5 phase geometry to identify constructive vs destructive relationships.

When Kuramoto reorganizes phases (irx_then_sync trial 1), it changes which memory pairs appear constructive/destructive to the downstream stages. The carrier structure that irx built in the amplitude-time series is not just in `mem.amplitude` values — it is encoded in the interference geometry of the whole memory population. Changing the phase geometry (via Kuramoto) rewrites the interference landscape, and `stage_prune` then damages memories that no longer appear constructive to each other.

Result: carrier_e dropped 0.935 → 0.744 in irx_then_sync despite no direct amplitude writes by stage_sync.

### Why sync_then_irx caused catastrophic collapse

When Kuramoto runs FIRST, it heavily reorganizes phases (tight within-category clusters). Then irx runs based on the `pairs` list — but the pairs were computed by `stage_detect` (stage 2) using the PRE-Kuramoto phase state. The irx phase targets (weighted circular means of constructive neighbors) are based on the original phase geometry. After Kuramoto has displaced all phases significantly, the irx phase moves are large and conflicting: irx is trying to converge toward the original constructive-pair means, but starting from a Kuramoto-displaced state.

The net phase displacement across 16 irx steps is large and incoherent — equivalent to running irx with relax_steps ≫ 16. This is the same mechanism that caused the relax_steps=24 catastrophe (T02 fire, carrier_e → 0.000). The threshold for catastrophic carrier destruction is crossed when the total phase displacement in stage 4.5 exceeds a critical level.

### Partial success: xi did improve

The irx_then_sync hybrid produced xi = 0.717 (vs irx baseline 0.559). The Kuramoto pass did create useful category cluster structure. If there were a way to get this xi improvement without the carrier_e damage, fitness could drop below 0.099.

The sync_then_irx mode produced xi = 0.894 — among the best ever observed. This confirms that K=0.5 Kuramoto creates excellent adversarial robustness when it runs on a fresh or Kuramoto-primed phase state.

### The fundamental incompatibility

Both stage functions read the same phase state and both make significant net phase changes. They were calibrated independently — irx at alpha=0.10, relax_steps=16; stage_sync at K=0.5, steps=50. When chained:

- irx_then_sync: the total phase change is irx-change + Kuramoto-change. The combined effect is larger than either alone, damaging the interference geometry on which downstream stages depend.
- sync_then_irx: the pairs used by irx were computed before Kuramoto, so irx is moving phases toward stale targets. Large misalignment → large effective alpha → same failure mode as relax_steps=24.

A true hybrid would require recomputing pairs AFTER the Kuramoto pass (before irx) — that is, restructuring the dream pipeline to insert a second `stage_detect` call between the Kuramoto and irx passes. This is a significantly larger change and requires its own justification.

---

## Summary comparison

| config | fitness | carrier_e | transfer | xi | magic_R |
|--------|---------|-----------|----------|----|---------|
| irx baseline (A=0.1) | **0.099** | **0.935** | **0.836** | 0.559 | 0.617 |
| stage_sync baseline (K=0.5, A=0.15) | 0.104 | 0.853 | 0.655 | **0.878** | 0.161 |
| irx_then_sync (trial 1) | 0.148 | 0.744 | 0.589 | 0.717 | 0.103 |
| sync_then_irx (trial 1) | 0.193 | 0.000 | 0.621 | **0.894** | 0.346 |

---

## Decision

No improvement found. Code reverted to baseline. Optimum unchanged:

    DREAM_MODE=interference_relax  DRIVE_A=0.1  DRIVE_SCOPE=all
    DRIVE_FREQ_HZ=0.5  (KURAMOTO_COUPLING irrelevant under irx)
    3-run avg fitness ≈ 0.099

---

## What is now closed

| direction | status |
|-----------|--------|
| irx_then_sync | CLOSED — carrier_e 0.935→0.744 regression |
| sync_then_irx | CLOSED — catastrophic carrier_e→0.000 |
| All irx internal params | CLOSED (prior fires) |
| K axis for stage_sync | CLOSED (prior fires) |
| A axis for both modes | CLOSED (prior fires) |
| DRIVE_FREQ_HZ | CLOSED (prior fires) |
| relax_steps | CLOSED (prior fires) |
| envelope_depth | CLOSED (prior fire T06) |

## What remains structurally open

1. **Double stage_detect hybrid**: run `stage_detect` a second time after Kuramoto, then feed fresh pairs to irx. This would allow irx to operate on a Kuramoto-primed phase state using accurate pair geometry. Larger code change — restructures the dream pipeline. Not attempted this fire.

2. **stage_boost_prune / stage_hallucinate parameter tuning**: thresholds in these stages affect amplitude dynamics and have never been varied. May affect carrier_e and transfer independently.

3. **xi seeding**: seeding `eval_xi_robustness_v2` RNG to reduce per-trial variance. Does not improve fitness but reduces confirmation cost from 3 trials to 1.
