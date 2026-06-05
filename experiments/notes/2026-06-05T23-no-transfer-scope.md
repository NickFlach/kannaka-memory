# Hypothesis: DRIVE_SCOPE=no_transfer — confirmed improvement

**Date:** 2026-06-05T23 UTC
**Branch:** kannaka-curiosity/2026-06-05T23
**Status:** CONFIRMED — 3-run avg fitness 0.147, beats 0.18 baseline by 0.033

---

## Hypothesis

`DRIVE_SCOPE=no_transfer` drives all engines *except* engine_b_primed and
engine_b_naive during the dream chain. The prediction from T00: this should
combine two benefits:

1. Transfer_score improves when engine_b is undisturbed (T22 showed xi_and_flat
   — which also skips engine_b — raised transfer_score from 0.422 to ~0.486).
2. xi_robustness_v2 stays high because engine_a IS driven (unlike xi_and_flat
   which excludes engine_a, causing xi to drop).

Both weights are 0.15. If transfer_score rose by ~0.30 and xi held near 0.98,
the net fitness improvement would be ~0.045, putting expected fitness near 0.135.

**DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer DREAM_MODE=(unset)**

---

## Results

| Trial | fitness  | transfer_score | xi_robustness_v2 | carrier_emergence | magic_proxy_phase_R | query_gravity |
|-------|----------|---------------|-----------------|-------------------|---------------------|---------------|
| t1    | 0.199085 | 0.7252        | 0.3769          | 0.5588            | 0.3623              | 0.4597        |
| t2    | 0.107292 | 0.7252        | 0.9893          | 0.5588            | 0.3623              | 0.4597        |
| t3    | 0.133595 | 0.7185        | 0.8230          | 0.5588            | 0.3623              | 0.4597        |
| **avg** | **0.1469** | **0.7230** | **0.7297**    | **0.5588**        | **0.3623**          | **0.4597**    |

---

## Comparison to baseline

| Condition           | fitness (3-run avg) | transfer_score | xi_robustness_v2 |
|---------------------|--------------------:|---------------:|-----------------:|
| Baseline (all)      | ~0.18               | ~0.422         | ~0.979           |
| xi_and_flat (T22)   | ~0.159              | ~0.486         | ~0.885           |
| **no_transfer**     | **0.147**           | **0.723**      | **0.730**        |

`no_transfer` delivers the largest transfer_score jump yet observed (+0.30 vs
baseline, +0.24 vs xi_and_flat). xi_robustness_v2 shows the same high variance
seen in earlier conditions (t1: 0.377, t2: 0.989, t3: 0.823; avg 0.730).

transfer_score is nearly deterministic (0.725/0.725/0.719) — this is a structural
effect of not driving engine_b, not noise.

The fitness improvement (0.18 → 0.147 = −0.033) exceeds the 0.005 keep threshold.
No code changes were made; this is env-var only.

---

## Interpretation

Not driving engine_b during consolidation protects the "naïve → primed" transfer
ratio. The transfer_score formula is `fitness_b_primed / fitness_b_naive`: when
engine_b's dream is undisturbed, both branches stay coherent and the ratio climbs.
xi_and_flat and no_transfer both skip engine_b and both show improved transfer_score.
The key difference: no_transfer also drives engine_a, which partially sustains
xi_robustness_v2 (avg 0.730 vs xi_and_flat avg ~0.885 — still somewhat lower, but
the transfer_score gain more than compensates).

magic_proxy_phase_R (0.362) and query_gravity (0.460) are identical to all prior
no-interference_relax runs — the drive-scope change doesn't alter end-of-dream
phase structure.

---

## Decision

KEEP — no code changes to revert. The TSV rows are labeled
`L5.no_transfer.A0.1-t1/t2/t3`. No code reverted.

**New empirical optimum: DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer
DREAM_MODE=(unset), 3-run avg fitness ≈ 0.147.**

---

## Next fire directions

1. **3-run interference_relax at no_transfer scope**: does DREAM_MODE=interference_relax
   + DRIVE_SCOPE=no_transfer combine the high carrier_e and high magic_R of
   interference_relax with the high transfer_score of no_transfer?
   Prediction: fitness likely rises (xi drops under interference_relax), but worth
   one 3-run characterization to understand the full no_transfer × DREAM_MODE space.

2. **K-sweep now that plumbing is fixed** (Q2 from fire prompt): kuramoto_coupling
   in {1.0, 2.0, 3.0, 5.0, 7.0} at DRIVE_SCOPE=no_transfer. May further reduce
   xi variance and push fitness below 0.13.

3. **relax_steps=16** at DRIVE_SCOPE=no_transfer + interference_relax (Q3):
   raising relax_steps from 8 to 16 predicted to raise xi under interference_relax
   while keeping carrier_e high.
