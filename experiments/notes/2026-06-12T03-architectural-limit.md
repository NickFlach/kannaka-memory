# architectural limit reached — no new test worth running

**Date:** 2026-06-12T03 UTC
**Branch:** kannaka-curiosity/2026-06-12T03-architectural-limit
**Code changes:** NONE
**Status:** LIMIT — system at structural floor; threshold unreachable without new architecture

---

## Assessment

Current confirmed optimum (master at 159853f):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
chiral_p_bp=0.15 (engine_b_primed only)
xi_eval_relax=20 (engine_clean + engine_adv)
3-trial avg fitness = 0.008334
```

Threshold for keeping changes: 0.008334 − 0.005 = **0.003334**.

Structural minimum estimated from confirmed floors:

| source | weight | floor value | contribution |
|--------|--------|-------------|-------------|
| transfer_score | 0.15 | 0.958868 (fp floor = 0.002488 at chiral_p=0.15) | 0.006167 |
| consciousness | 0.03 | 0.9546 (phi_a=0.268 structural) | 0.001362 |
| xi_robustness_v2 | 0.15 | 0.9973 (exact sweet spot at relax=20) | 0.000405 |
| speed_a | 0.03 | ≈0.9905 (container-dependent) | 0.000285 |
| carrier_emergence | 0.10 | 0.9992 | 0.000080 |
| others | — | ≈1.0 | 0.000035 |
| **total** | | | **≈ 0.008334** |

The structural minimum equals the current optimum. All axes confirmed closed:
- chiral_p_bp: 0.15 is exact optimum; below cliff at <0.05 (T00, T05)
- xi_eval_relax: 20 is exact sweet spot; 21 collapses xi below baseline (T14)
- b_primed relax_steps: 20 confirmed; 24 global = catastrophic (T07); 20 fully converged (T07-extra-relax)
- engine_a relax_steps: 20 causes transfer crash + consciousness regression (T13)
- alpha_base: 0.10 optimal for all engines; 0.15 for b_primed regresses (T00)
- K-sweep: no-op in irx mode (T12)
- drive frequency: 0.5 Hz optimal; 1.0 Hz regression via carrier_e (T10)
- all engine_flat/engine_a/stage_sync axes: closed in T17

No mechanism exists to push transfer below 0.958868 or consciousness above 0.9546 within
the current wave-interference + interference_relax architecture. The 0.003334 threshold
requires eliminating ~60% of the remaining transfer cost, which has no identified lever.

Nothing new worth testing this fire.
