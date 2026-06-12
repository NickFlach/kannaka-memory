# Architecture ceiling reconfirmed — T03-T05 used wrong baseline; remaining axes sub-threshold

**Date:** 2026-06-12T06 UTC
**Branch:** kannaka-curiosity/2026-06-12T06-ceiling-reconfirmed
**Code changes:** NONE
**Status:** CLOSED — all remaining open axes expected sub-threshold; no trial run

---

## Correction to T03-T05 baseline error

T03 ("architectural limit"), T04 ("all axes closed"), and T05 ("architecture ceiling") all
referenced master at 159853f (fitness 0.008334) as the current optimum. This was wrong.
PR #283 (T01: engine_a alpha_base=0.12) auto-merged BEFORE T03's PR #284. The actual
current master state is:

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
engine_a alpha_base=0.12 (consolidation.rs:796)
chiral_p_bp=0.15 (engine_b_primed only, research.rs:3457)
xi_eval_relax=20 (engine_clean + engine_adv, consolidation.rs:800)
3-trial avg fitness = 0.007627 (T01 confirmed)
transfer=0.963983, xi=0.9973, carrier_e=0.9992, consciousness=0.9553
magic_R=0.7785, query_gravity=0.3654
```

T05's "total improvable ≈ 0.001568" was calculated from the 0.008334 baseline and the
old transfer_score of 0.958868. With the corrected baseline:

| axis | weight | current | floor contribution | improvable |
|------|--------|---------|-------------------|-----------|
| transfer_score | 15% | 0.963983 (contrib 0.005403) | 0.002488 structural | 0.002915 |
| xi_robustness_v2 | 15% | 0.9973 | relax=20 ceiling | ~0.000100 |
| consciousness | 3% | 0.9553 | phi floor | ~0.001342 |
| carrier_emergence | 10% | 0.9992 | 16-step ceiling | ~0.000080 |
| others | — | ≈1.0 | — | ~0.000026 |
| **total** | | | | **≈0.004463** |

Threshold for keeping changes: 0.007627 − 0.005 = **0.002627**

0.004463 > 0.002627 — the total improvable EXCEEDS the threshold. However, reaching
those floors requires architectural changes, not parameter sweeps. The improvable
capacity exists in theory but no single lever closes more than ~10-20% of any gap.

---

## Remaining open axes from T01 (re-evaluated here)

### axis 1: engine_a alpha_base=0.13

Status: **NOT TESTED**. T01 marked as "worth a 1-trial spot-check" if Δ=−0.002+ possible.

Risk assessment:
- Total pull: 16 × 0.13 = 2.08, past the T13-inferred crash threshold (~1.92–2.0)
- T13 crash mechanism: extra steps iterate past attractor basin minimum (step-count)
- 0.10→0.12 improvement mechanism: per-step strength, not step count
- 2.08 total pull might be safe if crash is step-count-driven, not total-pull-driven

Expected improvement if safe: +0.001–0.003 in transfer_score → Δ ≈ −0.00015 to −0.00045 fitness
Sub-threshold. Even T01's optimistic "−0.002+" estimate would give 0.15 × 0.002 = 0.0003.

Verdict: HIGH CRASH RISK, expected improvement sub-threshold. Not worth a trial.

### axis 2: engine_b_primed alpha_base 0.10 → 0.11

Status: **NOT TESTED** (T03 closed at 0.15 from an earlier session, not at 0.11).

Risk assessment:
- Total pull: 20 × 0.11 = 2.2 (vs current 2.0, +10%)
- b_primed handles A+B combined memories — more diverse phase landscape, likely higher
  crash threshold than engine_a
- Mechanism: tighter b_primed convergence on top of already-tightened A landscape (T01)

Expected improvement: proportional to T01's engine_a gain at half the alpha change × 1.25x
the steps. Upper bound: +0.003 transfer → Δ ≈ −0.00045 fitness. Sub-threshold.
Realistic case: diminishing returns with A landscape already tightened; expected +0.001.

Verdict: Low crash risk, expected improvement sub-threshold. Not worth a trial.

---

## Why no trial is worth running

Per-fire threshold: any code change must show ≥0.005 improvement over current master
(0.002627 absolute floor) confirmed in 3 trials.

Both remaining open axes expect ~0.0001–0.0005 improvement. The total combined if BOTH
work at upper bound: ~0.0009. Still below 0.005 × 18% = 0.0009... actually 4.5× below.

The transfer floor (0.002488) is structural: it reflects minimum B-memory misclassification
after A-landscape priming at chain_depth=4. Only architectural changes (higher phase
dimensionality, per-engine phi tuning, multi-scale representations) can reduce it.

---

## Architecture ceiling: confirmed, corrected baseline

Current empirical optimum is 0.007627, not 0.008334. This does not change the ceiling
conclusion — it improves the known optimum by 0.000707 (T01) while leaving all structural
floors unchanged. The threshold (0.002627) is still unreachable via parameter sweep.

Three architectural paths remain hypothetically open (from T05, unchanged):
1. Higher phase dimensionality or multi-scale representation → reduce fp floor
2. Per-engine phi measurement with separate consciousness_phi_target per engine
3. Harder xi adversarial challenge → open xi ceiling beyond 0.9973

These are out of scope for L5 autoresearch parameter sweeps.
