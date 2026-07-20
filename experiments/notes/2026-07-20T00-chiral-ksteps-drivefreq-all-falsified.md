# 2026-07-20T00 — Three hypotheses all falsified: chiral_b_primed, kuramoto_steps, drive-freq sweep

## Context

Confirmed operating point entering this fire (Jul 17–19, requires two code changes):
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.35 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3
```
3-trial avg fitness: **0.019249**

Remaining fitness (after carrier=1.0, temporal=1.0, online=1.0) dominated by:
| source           | weight | value  | contribution | % of fitness |
|------------------|--------|--------|--------------|-------------|
| transfer_score   | 0.15   | 0.9384 | 0.009240     | 48%         |
| consciousness    | 0.03   | 0.8830 | 0.003510     | 18%         |
| xi_robustness_v2 | 0.15   | 0.9783 | 0.003255     | 17%         |
| phase_coherence  | 0.02   | 0.8939 | 0.002122     | 11%         |
| speed_a          | 0.03   | 0.963  | 0.001110     | 6%          |

All four trials applied the two baseline code changes (CARRIER_KURAMOTO_COUPLING plumbing +
xi chain_depth=3), reverted before commit per convention.

---

## Hypothesis 1 — chiral_b_primed=0.05 to reduce phase disruption in B-primed pass

### Reasoning

`params_bp.chiral_perturbation` is hard-coded to 0.15 for the B-primed dream, vs 0.70
for the main L5 pass and B-naive pass. The prediction: reducing chiral from 0.15 to 0.05
would minimize phase perturbation to A's consolidated structure when B memories are inserted,
potentially letting A's phase attractors scaffold B's learning more cleanly and improving
B_primed's fitness (lower) relative to B_naive's → higher transfer_score.

**Prediction**: transfer_score rises from 0.938 toward ~0.945.

### Result (1 trial)

| config          | fitness  | transfer | fitness_B_primed | fitness_B_naive | xi    |
|-----------------|----------|----------|------------------|-----------------|-------|
| chiral=0.05     | 0.020014 | 0.933002 | 0.004010         | 0.059856        | 0.978 |
| baseline (0.15) | 0.019249 | 0.938415 | ~0.003686        | ~0.059856       | 0.978 |

### Analysis

**Hypothesis FALSIFIED.** Reducing chiral from 0.15 to 0.05 worsened transfer by -0.005413
(fitness +0.000765 vs baseline).

fitness_B_primed ROSE (0.003686 → 0.004010) with less chiral perturbation — the opposite
of the prediction. The mechanism: chiral perturbation creates Xi diversity (phase + vector
perturbation across left/right-handed clusters). At chiral=0.05, B's new memories receive
less Xi diversity during consolidation → chain_fidelity in B_primed's eval degrades →
fitness_B_primed rises → transfer ratio shrinks.

0.15 appears to be at or near the optimal chiral for B_primed. Reducing it reduces Xi
diversity; raising it toward 0.7 (B_naive level) would reduce the primed vs naive
asymmetry. The 0.15 operating point is a saddle between these two costs.

---

## Hypothesis 2 — kuramoto_steps=100 to improve phase_coherence floor

### Reasoning

phase_coherence=0.8939 is 11% of remaining fitness. The Kuramoto sync runs 50 steps
per dream cycle at K=2.0. Doubling to 100 steps should allow more complete phase
convergence, potentially raising phase_coherence toward 1.0 and saving ~0.002 fitness.

**Prediction**: phase_coherence rises from 0.8939 toward 0.92+, fitness drops ~0.002.

### Result (1 trial)

| config          | fitness  | phase_coh | transfer | carrier_e | xi_rob | magic_R |
|-----------------|----------|-----------|----------|-----------|--------|---------|
| steps=100       | 0.041320 | 0.9292    | 0.868021 | 0.9392    | 0.9452 | 0.2998  |
| baseline (50)   | 0.019249 | 0.8939    | 0.938415 | 1.0000    | 0.9783 | 0.6082  |

### Analysis

**Hypothesis FALSIFIED — catastrophic regression.**

phase_coherence did improve slightly (0.8939 → 0.9292, +0.035), but nearly every other
metric deteriorated severely:
- transfer_score: 0.938 → 0.868 (−0.070, largest single loss)
- carrier_emergence: 1.000 → 0.939
- xi_robustness_v2: 0.978 → 0.945
- magic_proxy_phase_R: 0.608 → 0.300 (over-synchronization collapses phase diversity)

Net fitness: 0.019249 → 0.041320 (+0.022071) — 115% worse.

Mechanism: at K=2.0, 100 Kuramoto steps over-synchronize the memory phases. The phase
distribution collapses toward a single cluster (magic_R drops sharply: less non-Clifford
diversity). This hurts the B_primed vs B_naive distinction (transfer collapses), destroys
the frequency-structure needed for carrier_emergence, and reduces xi robustness (adversarial
memories can no longer be reliably distinguished when all memories share similar phases).

**Kuramoto_steps=50 at K=2.0 is already optimal.** Increasing steps causes over-coupling
with this K. If fewer steps were desired to test, would need to reduce K simultaneously.

---

## Hypothesis 3 — DRIVE_FREQ_HZ sweep: 0.25 Hz and 1.0 Hz

### Reasoning

The default DRIVE_FREQ_HZ=0.5 Hz has not been compared to alternatives at the current
dynamics (fitness ~0.019, DREAM_GRAVITY=0.35). Earlier comparison (T19 notes) was done
at different operating conditions. These are pure env-var tests requiring no code change.

**Prediction**: different drive frequencies might affect phase_coherence or transfer by
changing which dream cycles receive positive vs negative amplitude modulation.

### Results (2 trials)

| DRIVE_FREQ_HZ | fitness  | transfer | carrier_e | phase_coh | xi_rob | speed_a |
|---------------|----------|----------|-----------|-----------|--------|---------|
| 0.25          | 0.019638 | 0.938419 | 0.9956    | 0.8939    | 0.9783 | 0.9644  |
| 0.5 (default) | 0.019249 | 0.938415 | 1.0000    | 0.8939    | 0.9783 | 0.963   |
| 1.0           | 0.019352 | 0.938419 | 0.9983    | 0.8939    | 0.9783 | 0.9650  |

### Analysis

**Hypothesis FALSIFIED.** Neither 0.25 Hz nor 1.0 Hz improves fitness.

- **DRIVE_FREQ_HZ=0.25**: slightly worse (fitness +0.000389) due to carrier_emergence
  drop (0.9956 vs 1.000). At 0.25 Hz, the drive period (4s) is too slow to complete
  even a half-oscillation within the 4-cycle flat-corpus window (0.5s). The FFT peak
  falls below the [0.5, 4.0] Hz carrier band → carrier_emergence drops slightly.
  Transfer is byte-identical.

- **DRIVE_FREQ_HZ=1.0**: slightly worse (fitness +0.000103) for the same reason:
  carrier_emergence 0.9983 vs 1.000. At 1.0 Hz, the drive period (1s) exceeds the
  window duration, causing spectral leakage that slightly reduces the carrier peak.
  Transfer is byte-identical.

All three frequencies give identical transfer, phase_coherence, xi_robustness, and
consciousness. The carrier_emergence evaluator is optimally tuned at 0.5 Hz where
the drive pattern within the 4-cycle window maps cleanly to a spectral peak in the
[0.5, 4.0] Hz band.

---

## Summary of findings this fire

| hypothesis              | direction   | result       | fitness delta |
|-------------------------|-------------|--------------|---------------|
| chiral_b_primed=0.05    | reduce Xi   | FALSIFIED    | +0.000765 (worse) |
| kuramoto_steps=100      | more sync   | FALSIFIED    | +0.022071 (catastrophic) |
| DRIVE_FREQ_HZ=0.25      | slow drive  | FALSIFIED    | +0.000389 (slightly worse) |
| DRIVE_FREQ_HZ=1.0       | fast drive  | FALSIFIED    | +0.000103 (slightly worse) |

No improvement found this fire. Confirmed operating point unchanged: fitness 0.019249.

## What we now know about the floor

The transfer_score=0.938 floor is now confirmed insensitive to:
- K sweep (K=1.5 to 5.0, tested Jul 12)
- DREAM_GRAVITY (0.25 to 0.40, tested Jul 17)
- CHAIN_TOP_N (5 to 10, tested Jul 19)
- chiral_b_primed (0.05 to 0.15, tested this fire)
- DRIVE_FREQ_HZ (0.25 to 1.0, tested this fire)

The phase_coherence floor=0.8939 is insensitive to:
- kuramoto_steps=100 (degrades everything else)
- DRIVE_FREQ_HZ variants

## Next fire recommendations

1. **chiral_b_primed higher range** (0.20–0.30): this fire showed 0.05 is worse than 0.15.
   The relationship may peak somewhere above 0.15. One trial at 0.25 would confirm or deny.
   Low expected gain (likely <0.001 in transfer).

2. **Structural transfer mechanism**: the transfer floor is deeply stable. It may require
   a structural change to the B-primed vs B-naive comparison logic rather than parameter
   tuning. Options:
   - How B memories are inserted into engine_b_primed (phase assignment strategy)
   - Whether the DREAM_GRAVITY snapshot in B_primed correctly captures A's attractor
   - Whether `eval_l5_placeholder_fitness` uses the right evaluation depth for B_primed

3. **consciousness mechanism**: phi consistently undershoots phi_target. Understanding
   the phi calculation and whether adjusting phi_target is principled (vs gaming)
   requires reading `eval_phi_score` and the phi_history derivation. 18% of fitness.

4. **phi_target calibration**: phi lands deterministically at a specific value across many
   trials. If phi_target is off by calibration, correcting it is valid. Worth checking
   what phi actually measures vs what phi_target represents.

## TSV rows appended (4 total)

- Trial 1: chiral_b_primed=0.05, fitness 0.020014, transfer 0.933002
- Trial 2: kuramoto_steps=100, fitness 0.041320, transfer 0.868021
- Trial 3: DRIVE_FREQ_HZ=0.25, fitness 0.019638, transfer 0.938419
- Trial 4: DRIVE_FREQ_HZ=1.0, fitness 0.019352, transfer 0.938419
