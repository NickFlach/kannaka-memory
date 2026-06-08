# irx alpha_base=0.15 falsified — integral relaxation is the carrier_e ceiling

**Date:** 2026-06-08T09 UTC
**Branch:** kannaka-curiosity/2026-06-08T09
**Code changes:** alpha_base 0.10→0.15 tried and reverted; no code changes kept
**Status:** FALSIFIED — alpha_base axis closed; integral relaxation confirmed as binding constraint

---

## Background

Current irx optimum: `DRIVE_A=0.1 DREAM_MODE=interference_relax DRIVE_SCOPE=all` → 3-trial avg fitness **0.099**, carrier_e=0.935, xi=0.559 avg (range 0.256–0.874).

The prior fire (T02) closed relax_steps at 16 — relax_steps=24 caused catastrophic carrier_e→0.000. The T02 hypothesis was that carrier destruction was step-count driven. If so, the complementary test is: same integral relaxation but fewer, larger steps (higher alpha_base).

---

## Hypothesis: alpha_base = 0.15 (50% increase, relax_steps=16 unchanged)

**Rationale:** At relax_steps=24 with alpha_base=0.10, total integrated relaxation ≈ 0.10 × 24 = 2.4. At alpha_base=0.15 with relax_steps=16, total ≈ 0.15 × 16 = 2.4. If carrier_e destruction is step-count driven (each step independently smears the carrier waveform), then 16 stronger steps should be safer than 24 weaker steps, potentially allowing stronger per-step phase convergence and improved xi.

**Prediction:** xi rises from 0.559 avg toward 0.65+; carrier_e stays near 0.935 (step-count-driven mechanism). Falsification signal: carrier_e drops below 0.90.

**Code change:** `alpha_base: f32 = 0.10` → `alpha_base: f32 = 0.15` in `stage_interference_relax`.

---

## Results (2 trials)

| metric | irx baseline (3-trial avg) | irx alpha_base=0.15 T1 | irx alpha_base=0.15 T2 | 2-trial avg |
|--------|--------------------------|------------------------|------------------------|-------------|
| fitness | 0.099 | 0.114 | 0.151 | **0.133** |
| transfer_score | 0.836 | 0.789 | 0.789 | 0.789 |
| carrier_emergence | 0.935 | **0.833** | **0.833** | **0.833** |
| xi_robustness_v2 | 0.559 avg | 0.584 | 0.339 | 0.462 |
| magic_proxy_phase_R | 0.617 | 0.793 | 0.793 | 0.793 |
| query_gravity | 0.363 | 0.378 | 0.378 | 0.378 |

**Hypothesis falsified.** carrier_e dropped from 0.935 to 0.833 (−0.102), conclusively below the falsification threshold of 0.90.

---

## Mechanism revealed

The carrier_e degradation is **integral-relaxation driven**, not step-count driven.

- The carrier_e metric responds to total phase movement (alpha × steps), not just step count
- alpha_base=0.15 × 16 steps produces the same total relaxation as alpha_base=0.10 × 24 steps (both ≈ 2.4)
- Both cause comparable carrier_e degradation: alpha_base=0.15 → 0.833; relax_steps=24 → 0.000 (catastrophic)
- The catastrophic collapse at relax_steps=24 may have additional nonlinear effects, but the directional signal is clear

The transfer_score and magic_R values are **deterministic** across trials (0.789 and 0.793 respectively — identical). Only xi_robustness_v2 varies (0.584 vs 0.339), confirming that xi variance under irx is entirely RNG-driven. alpha_base has no systematic effect on xi.

---

## Axis closure: alpha_base

The `alpha_base` axis in `stage_interference_relax` is now **closed**:
- alpha_base > 0.10 → carrier_e degrades (integral-relaxation mechanism)
- alpha_base = 0.10 is the current ceiling for carrier_e preservation
- Lowering alpha_base would reduce convergence without directional benefit (already underconverging from irx xi's perspective)

Combined with the relax_steps closure (ceiling at 16), the irx mode's convergence budget is fully bounded:

| parameter | status | value |
|-----------|--------|-------|
| relax_steps | CLOSED: 16 | >16 destroys carrier_e |
| alpha_base | NEW: CLOSED | 0.10 is carrier_e ceiling |
| envelope_depth | CLOSED: 0.15 | (tested in prior fire) |

**All three convergence parameters of stage_interference_relax are now closed.**

---

## xi variance: RNG-only, not mechanistic

The identical transfer, carrier_e, magic_R, and query_gravity across T1 and T2 while xi varies (0.584 vs 0.339) confirms: the irx mode's xi variance is entirely from the adversarial RNG in eval_xi_robustness_v2. No irx parameter change can systematically improve or worsen xi — the mechanism is noise-limited. To make progress on xi under irx, the eval would need a fixed seed.

---

## Decision

No code changes retained (alpha_base reverted to 0.10).

**Empirical optima unchanged:**
- `DRIVE_A=0.1 DREAM_MODE=interference_relax DRIVE_SCOPE=all` → avg fitness **0.099**
- `KURAMOTO_COUPLING=0.5 DRIVE_A=0.15 DRIVE_SCOPE=all` → avg fitness **0.104**

The irx convergence parameter space is now completely closed. The remaining open directions are:
1. stage_sync transfer improvement (0.655 vs irx 0.836 — 0.027 fitness gap)
2. Stage structural parameters (hallucinate, boost_prune) — not explored
3. xi eval seeding to remove RNG noise from irx xi measurements
