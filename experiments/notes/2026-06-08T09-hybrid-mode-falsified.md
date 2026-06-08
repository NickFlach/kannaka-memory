# Hybrid sequential mode falsified — irx and stage_sync are phase-antagonistic

**Date:** 2026-06-08T09 UTC
**Branch:** kannaka-curiosity/2026-06-08T09
**Code changes:** hybrid mode added and REVERTED; no code changes kept
**Status:** FALSIFIED — modes are phase-antagonistic; sequential combination degrades all paths

---

## Background

Current empirical optima:
- **stage_sync** (K=0.5, A=0.15): avg fitness **0.104**, carrier_e=0.853, xi=0.873, transfer=0.655
- **interference_relax** (irx, A=0.1): avg fitness **0.099**, carrier_e=0.935, xi=0.559, transfer=0.836

The notes from the prior fire flagged "combining modes" as an open question with no mechanism: irx wins on transfer+carrier, stage_sync wins on xi. Since both modes only directly modify phases (amplitudes are set by stage_drive earlier), running them sequentially appeared safe for carrier amplitude structure.

---

## Hypothesis

**DREAM_MODE=hybrid_relax_sync**: run irx first (constructive-pair phase alignment → high transfer/carrier), then stage_sync (Kuramoto category clustering → high xi). Both modes only touch `.phase`, never `.amplitude`. Carrier amplitude structure from stage_drive is in amplitudes and should survive.

**Prediction:** carrier_e ≥ 0.85, xi ≥ 0.70, transfer ≥ 0.75, fitness < 0.09.

---

## Trial 1: DREAM_MODE=hybrid_relax_sync (irx → sync)

Config: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=hybrid_relax_sync`

| metric | irx baseline | stage_sync baseline | hybrid irx→sync | delta vs irx |
|--------|-------------|---------------------|-----------------|--------------|
| fitness | 0.099 | 0.104 | **0.157** | **+0.058 regression** |
| transfer_score | 0.836 | 0.655 | **0.272** | **−0.564 catastrophe** |
| carrier_emergence | 0.935 | 0.853 | **0.744** | −0.191 |
| xi_robustness_v2 | 0.559 | 0.873 | **0.971** | +0.412 |
| magic_proxy_phase_R | 0.617 | ~0.35 | **0.106** | −0.511 |
| query_gravity | 0.363 | ~0.46 | **0.430** | +0.067 |

**Hypothesis falsified.** Transfer collapsed from 0.836 to 0.272 (catastrophic). Carrier degraded. xi rose to 0.971 (highest ever seen) but this did not compensate in fitness. Stage_sync running second dominated the end-state phase configuration, undoing irx's constructive pair alignment.

---

## Trial 2: DREAM_MODE=hybrid_sync_relax (sync → irx, reverse order)

Config: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=hybrid_sync_relax`

| metric | irx baseline | stage_sync baseline | hybrid sync→irx | delta vs irx |
|--------|-------------|---------------------|-----------------|--------------|
| fitness | 0.099 | 0.104 | **0.222** | **+0.123 regression** |
| transfer_score | 0.836 | 0.655 | **0.621** | −0.215 |
| carrier_emergence | 0.935 | 0.853 | **0.000** | **−0.935 catastrophe** |
| xi_robustness_v2 | 0.559 | 0.873 | **0.706** | +0.147 |
| magic_proxy_phase_R | 0.617 | ~0.35 | **0.346** | −0.271 |
| query_gravity | 0.363 | ~0.46 | **0.443** | +0.080 |

**Reverse order also falsified and worse.** carrier_e hit 0.000 — completely annihilated. Transfer partially recovered to 0.621 (near stage_sync alone), suggesting irx running last partially restores constructive alignment. But carrier_e was destroyed.

---

## Mechanism: phase-antagonism through downstream stage coupling

The critical assumption that failed: **carrier_emergence is not purely amplitude-based.**

Direct path: stage_drive applies `amplitude *= (1 + A*sin(2π*f*t))` → creates amplitude variation at drive frequency. carrier_emergence measures FFT peak of the amplitude time series.

But the downstream stages (stage_boost_prune, stage_hallucinate, stage_transfer) use cosine similarity of memory vectors, which includes phase as a component. When the phase configuration is perturbed by competing sync mechanisms, the downstream stages produce different amplitude evolutions across the dream chain. The carrier frequency peak in that amplitude time series is disrupted by the distorted phase landscape.

Asymmetry between orderings:

| ordering | "last stage wins" metric | carrier fate |
|----------|--------------------------|--------------|
| irx → sync | xi=0.971 (sync dominates) | degraded (0.744) |
| sync → irx | transfer=0.621 (irx partially restores) | annihilated (0.000) |

Neither ordering preserves carrier_e. The combined phase perturbation from both sync mechanisms disrupts the orderly amplitude modulation sequence that stage_drive establishes.

---

## What this closes

The "combining modes" open question is now closed: **irx and stage_sync are phase-antagonistic and cannot be naively combined sequentially**. Any hybrid requires a fundamentally different architecture — not sequential application but a mechanism that partitions phase-space responsibility (e.g., irx only for a subset of memories, stage_sync for another).

The xi=0.971 in trial 1 is the highest xi ever observed, but it's unactionable because transfer=0.272 (fitness impact: +0.084 from transfer collapse alone at weight 0.15).

---

## Decision

No code changes retained. Both orderings falsified.

**Empirical optima unchanged:**
- `DRIVE_A=0.1 DREAM_MODE=interference_relax DRIVE_SCOPE=all` → avg fitness **0.099**
- `KURAMOTO_COUPLING=0.5 DRIVE_A=0.15 DRIVE_SCOPE=all` (stage_sync defaults) → avg fitness **0.104**

**New structural insight:** carrier_e is phase-sensitive through downstream stage coupling. Phase-antagonism between irx and stage_sync prevents naive sequential combination. Any future hybrid strategy must respect phase-space partitioning, not just amplitude independence.
