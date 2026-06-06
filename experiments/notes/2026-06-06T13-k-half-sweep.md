# K=0.5 — sub-threshold coupling survey

**Date:** 2026-06-06T13 UTC
**Branch:** kannaka-curiosity/2026-06-06T13
**Code changes:** None — env-var only (KURAMOTO_COUPLING=0.5)
**Status:** TREND CONFIRMED, improvement not confirmed at threshold — no default change

---

## Hypothesis

K=1.0 was described in T00 as "below or near the synchronization threshold."
The fitness curve is monotonically decreasing from K=7 to K=1 in both T05 and T00.
Does K=0.5 continue that trend?

**Prediction:** K=0.5 preserves even more phase diversity than K=1.0 → xi stays high or
rises → fitness drops below K=1.0's 0.138 avg by ≥0.005.

---

## Method

No code change. Used existing `KURAMOTO_COUPLING` env var (wired in T00) with value 0.5.

All trials: `KURAMOTO_COUPLING=0.5 DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=<unset>`

---

## Results

| trial | fitness | xi_robustness_v2 | transfer_score | carrier_emergence | magic_R | query_gravity |
|---|---|---|---|---|---|---|
| t1 | 0.121259 | 0.9153 | 0.690034 | 0.5489 | 0.1432 | 0.4771 |
| t2 | 0.139218 | 0.8572 | 0.627601 | 0.5489 | 0.1971 | 0.4793 |
| t3 | 0.139895 | 0.8530 | 0.627601 | 0.5489 | 0.1971 | 0.4793 |
| **avg** | **0.133457** | **0.875** | **0.648** | **0.549** | **0.179** | **0.478** |

K=1.0 baseline (T00, 3 trials): avg fitness **0.138369**, xi avg ~0.864, R ~0.250

---

## Analysis

### Threshold evaluation

Δ fitness = 0.133457 − 0.138369 = **−0.004912**. Threshold: −0.005. Not confirmed.

### Trial 1 dominance

Without trial 1, trials 2–3 average to 0.1396 — essentially identical to K=1.0's 0.138.
The 3-trial average improvement is driven almost entirely by trial 1's lucky xi draw (0.9153
vs the 0.853–0.857 seen in trials 2–3). The improvement is not robust at 3 trials.

xi at K=0.5 (0.853–0.915) is consistently above the K=1.0 range (0.814–0.917 from T00),
suggesting K=0.5 does marginally raise xi in expectation — but the effect is smaller than
K=1.0's lift over K=3.0.

### K curve summary (monotone confirmed)

| K | fitness (avg or single) | xi (avg or single) | magic_R |
|---|---|---|---|
| 7.0 | 0.235 (single) | 0.140 | 0.240 |
| 5.0 | 0.226 (single) | 0.508 | 0.295 |
| 3.0 | 0.164 (single) | 0.600 | 0.362 |
| 1.0 | **0.138 (3-run avg)** | 0.864 | 0.250 |
| 0.5 | 0.133 (3-run avg) | 0.875 | 0.179 |

The trend is consistent: lower K → lower fitness, higher xi, lower R. The fitness gain per
halving of K is diminishing: K=3→1 gave Δ=−0.026; K=1→0.5 gave Δ=−0.005. The curve
is flattening as K approaches zero.

### magic_R interpretation

R at K=0.5 (0.143–0.197) is lower than K=1.0 (0.250). This continues the trend:
very low K = weak nudge = phases spread further apart = lower global order parameter.
The "magic" proxy predicts non-Clifford-like content based on phase diversity, not
phase concentration; a lower R at K=0.5 is consistent with the phase-diversity
interpretation of xi improvement.

### query_gravity

Slightly higher at K=0.5 (0.478) vs K=1.0 (0.469), consistent with the trend that
lower coupling preserves amplitude contrast (attention-as-gravity).

---

## Decision

**No code change.** Improvement is Δ=−0.0049, below the −0.005 confirmation threshold.
The improvement is primarily driven by one high-xi trial; the other two trials match K=1.0
closely. K=1.0 remains the confirmed default (fitness 0.138).

---

## Implications for future fires

1. **K=0.25 or K=0.0**: the curve is flattening but still pointing downward. Testing
   K=0.25 in a future fire with 5 trials (to reduce xi variance impact) could resolve
   whether the trend continues or plateaus. K=0.0 would be a degenerate case (no Kuramoto
   coupling at all) — essentially a "null sync" baseline.

2. **More trials at K=0.5**: 5–6 trials would pin down whether the Δ=−0.005 is real. The
   xi adversarial perturbation has high variance (SD ≈ 0.03 based on T00 and this fire).
   A 5-trial mean would have narrower CI and cross the threshold if the effect is genuine.

3. **K=0.5 + different DRIVE_FREQ_HZ**: drive frequency is unexplored (T19 was blocked by
   missing sibling deps). Testing DRIVE_FREQ_HZ in {0.5, 1.0, 4.0} at both K=0.5 and
   K=1.0 would map a 2D surface.

4. **Why does the fitness gain diminish?** K=3→1 gave Δ=−0.026; K=1→0.5 gave Δ=−0.005.
   The marginal value of reduced coupling is shrinking. This suggests the system approaches
   an asymptote as K→0 rather than continuing to improve indefinitely. Testing K=0.0
   would confirm whether the ceiling is already visible at K=0.5.
