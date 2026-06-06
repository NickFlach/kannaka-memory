# K < 1.0 falsified — K=1.0 is a local optimum, not a monotone trend

**Date:** 2026-06-06T07 UTC
**Branch:** kannaka-curiosity/2026-06-06T07
**Code changes:** None — env-var only
**Status:** NULL RESULT (hypothesis falsified, K=1.0 confirmed as local minimum)

---

## Hypothesis

The K=1.0 confirmation fire (2026-06-06T00) found that reducing Kuramoto coupling
from the K=3.0 default to K=1.0 lifted xi_robustness_v2 from ~0.64 to ~0.86 and
cut avg fitness from ~0.18 to ~0.138. The notes explicitly flagged K < 1.0 as an
open question: "K=0.5 or K=0.25 might perform better or worse."

**Prediction:** K=0.5 continues the trend (xi↑, fitness↓). Mechanism: even less
coupling → even more phase diversity within categories → adversarial perturbations
harder to construct → xi up. Expected fitness < 0.138.

---

## Trials

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=<unset>`

| K | fitness | xi_robustness_v2 | transfer_score | carrier_emergence | magic_R | query_gravity |
|---|---------|-----------------|----------------|-------------------|---------|---------------|
| 0.5 | 0.168193 | 0.6203 | 0.690034 | 0.5489 | 0.143 | 0.477 |
| 0.75 | 0.220606 | 0.3940 | 0.560599 | 0.5581 | 0.247 | 0.458 |

**Reference (K=1.0 confirmation, 3-run avg):** fitness 0.138, xi ~0.86, R 0.250

---

## Findings

**Hypothesis falsified.** Both K=0.5 and K=0.75 are substantially worse than K=1.0:

| K | fitness | xi |
|---|---------|-----|
| 0.75 | 0.221 | 0.394 |
| 0.50 | 0.168 | 0.620 |
| **1.0** | **0.138** | **~0.86** |
| 3.0 | ~0.18 | ~0.64 |
| 5.0 | 0.226 | 0.508 |
| 7.0 | 0.177 | 0.527 |

K=1.0 is a local minimum in fitness across the full explored range (0.5–7.0). The
improvement from K=3.0→1.0 does NOT extend to K < 1.0.

### Why xi drops at K < 1.0

The K=1.0 confirmation notes predicted xi would rise with K↓ because lower coupling
preserves phase diversity. That mechanism works for K in [1.0, 7.0] but reverses
below K=1.0. A plausible explanation: at K=1.0, the Kuramoto coupling is near the
synchronization threshold for these cluster sizes. The coupling "nudges" phases toward
their category attractors just enough to create clean, separated clusters — the xi
adversary cannot easily find a perturbation that maps one cluster to another.

At K < 1.0, the coupling falls below the coherence threshold. The phase updates in
stage_sync become too weak to overcome noise from the constructive-interference geometry
(the pairs detected in stage_detect). The result is a less organized phase distribution
where clusters bleed into each other — a softer boundary that adversarial perturbations
can exploit more easily. Less sync ≠ more diversity in the xi-relevant sense; it means
less structure.

### xi stochasticity note

xi is unseeded (eval_xi_robustness_v2 uses RNG without a seed), so single-trial xi
values are noisy. However, both K=0.5 and K=0.75 produced fitness values well above
the K=1.0 avg (0.168 and 0.221 vs 0.138). The fitness gap is large enough that
variance cannot explain the reversal. The ordering is K=0.75 > K=0.5 > K=1.0 on
fitness (worse to better), which also doesn't match the expected monotone.

### magic_R at K < 1.0

R is lower at K < 1.0 (0.143–0.247 vs 0.250 at K=1.0). This is consistent with
less synchronization → lower order parameter → lower R. But lower R correlates with
worse xi here, not better. The R-as-proxy-for-non-Clifford-content framing may need
a regime qualifier: R is informative within a coupling regime, not across regimes
where the phase structure has different character.

---

## Decision

**No improvement.** No code changes to revert (env-var only). K=1.0 is confirmed
as a genuine local minimum, not an arbitrary step on a monotone trend. The empirical
optimum remains:

    DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=<unset>  KURAMOTO_COUPLING=1.0
    3-run avg fitness ≈ 0.138

---

## Implications for future fires

1. **K=1.0 is robust**: the local-optimum finding with sharp drop on both sides
   makes K=1.0 a stable default. No further K refinement is likely to find a
   ≥0.005 improvement in the stage_sync path.

2. **The phase-structure quality threshold**: there appears to be a critical coupling
   where stage_sync produces well-separated category clusters (K≈1.0) vs diffuse
   clusters (K<1.0) vs over-locked clusters (K>1.0). Mapping this transition
   precisely would require a finer sweep (K ∈ {0.8, 0.9, 1.0, 1.1, 1.2}).

3. **interference_relax at the new lower fitness**: the best confirmed fitness is
   0.138 under stage_sync (K=1.0). interference_relax achieves avg 0.149 (T00).
   The two modes are mutually exclusive (interference_relax replaces stage_sync,
   so K is irrelevant to it). If interference_relax has further headroom (e.g.,
   via alpha or relax_steps tuning under the no_transfer scope), it might catch up.

4. **Drive frequency variants (Q6)** remain entirely untested: DRIVE_FREQ_HZ ∈
   {0.5, 1.0, 4.0} vs the default 2.0. This is still a clean, unexplored axis
   with no historical data at this code state. Sibling deps are now available so
   blocking issues are gone.
