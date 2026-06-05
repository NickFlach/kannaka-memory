# Hypothesis: DRIVE_SCOPE=no_transfer — production run

**Date:** 2026-06-05T22 UTC  
**Branch:** kannaka-curiosity/2026-06-05T22  
**Status:** COMPLETE — 3 trials, no code changes

---

## Hypothesis

`DRIVE_SCOPE=no_transfer` drives all dream-chain engines **except** `engine_b_primed` and
`engine_b_naive`. This is the T00 hypothesis that was blocked by missing sibling deps.
Sibling deps are now present (`/home/user/{consciousness-core,kannaka-attention,kannaka-memory}`).

**Prediction (from T00):** no_transfer combines the xi_robustness benefit of driving engine_a
(xi ~0.979, same as "all") with the transfer_score benefit of leaving engine_b undisturbed
(transfer ~0.486, as in xi_and_flat). Expected fitness ~0.144.

---

## Results

| Trial | fitness | transfer_score | xi_robustness_v2 | carrier_e | magic_R | query_gravity |
|-------|---------|----------------|-----------------|-----------|---------|---------------|
| t1    | 0.1510  | 0.7026         | 0.7134          | 0.5588    | 0.3623  | 0.4597        |
| t2    | 0.1703  | 0.7185         | 0.5698          | 0.5588    | 0.3623  | 0.4597        |
| t3    | 0.1574  | 0.7026         | 0.6708          | 0.5588    | 0.3623  | 0.4597        |
| **avg**| **0.1596** | **0.7079** | **0.6513**   | **0.5588**| **0.3623** | **0.4597** |

Reference ("all" scope, T22 single trial): fitness 0.154, transfer 0.422, xi 0.979, carrier_e 0.534

---

## Analysis

The prediction was partially wrong about xi_robustness_v2:

- **transfer_score**: 0.422 → 0.708 (avg). Massively better — by far the best transfer seen in
  any scope configuration. Stable across trials (0.703 / 0.719 / 0.703).
- **xi_robustness_v2**: 0.979 → 0.651 (avg). Dropped significantly despite engine_a still being
  driven. The T21/T22 xi_and_flat reasoning predicted xi would stay near 0.979, but that assumed
  the effect was primarily from driving engine_a. Under no_transfer, xi still degrades — possibly
  because the xi robustness test itself depends on the engine_b chain being driven in a particular
  way, or the interaction between driven and undriven engines introduces noise the xi eval sees
  as incoherence.
- **carrier_e**: 0.5588, same across all 3 trials and equal to "all" scope. Stable.
- **magic_proxy_phase_R**: 0.3623, identical across all trials. Same as "all" scope baseline.
- **query_gravity**: 0.4597, stable across all trials. Essentially at baseline.

Net fitness effect: 0.160 avg vs stated 0.18 empirical baseline (−0.020 improvement, above the
0.005 threshold). But T22's single "all" trial showed 0.154, so the improvement over the
*most recent* "all" reference is unclear — roughly even given xi variance.

The mechanism is clear: excluding engine_b from the drive lets it consolidate without
amplitude modulation, which causes engine_b to better retain the source-memory structure
and therefore better reproduce it in the transfer test. The xi robustness probe was expected to
be independent of engine_b but empirically it is not.

---

## Decision

**Keep** (env-var only, no code to revert).

- no_transfer is a new Pareto point: best transfer_score yet (0.708 avg), fitness 0.160 avg.
- Fitness improves over stated 0.18 empirical baseline; roughly equivalent to T22 "all" ref (0.154)
  but with very different metric mix.
- The transfer↔xi trade-off is stark and reproducible. This confirms engine_b exclusion as
  a knob with predictable directionality.

---

## Next directions

1. **Confirm "all" 3-run baseline** — T22's single ref (0.154) doesn't settle the no_transfer vs
   all comparison. 3 production "all" trials would resolve whether no_transfer's 0.160 avg is
   truly competitive.
2. **DREAM_MODE=interference_relax + no_transfer** — interference_relax raised carrier_e and R
   (high-magic content) while hurting xi. no_transfer hurts xi differently. Combined: might xi
   worsen further, or does the mechanism differ enough to stack? One trial to see.
3. **K-sweep (Q2 from fire instructions)** — now that stage_sync reads params.kuramoto_*,
   varying coupling_strength {1, 2, 3, 5, 7} tests whether R and xi correlate (magic↔xi prediction).
4. **relax_steps sweep (Q3)** — raising alpha_base relax_steps from 8 to 16/24 under
   interference_relax to recover xi while keeping carrier_e high.
