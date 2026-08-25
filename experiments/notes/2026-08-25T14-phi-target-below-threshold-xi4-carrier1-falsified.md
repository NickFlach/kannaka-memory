# 2026-08-25T14 — phi_target below threshold; xi depth=4+K=3.0 and CARRIER_K=1.0 falsified

## Context

Operating point entering this fire (Aug 24 defaults, `DRIVE_A=0.1 DRIVE_SCOPE=all`):

| metric           | value  | fitness contribution |
|------------------|--------|---------------------|
| transfer_score   | 0.9540 | 0.006900            |
| xi_robustness_v2 | 0.9678 | 0.004830            |
| consciousness    | 0.8830 | 0.003510            |
| phase_coherence  | 0.8939 | ~0.002122           |
| speed_a          | 0.9666 | ~0.001002           |
| **total**        |        | **~0.018371**       |

Aug 24 notes listed three priority hypotheses:
1. phi_target decoupling: known −0.003507 savings, claimed by Aug 24 to "cross threshold alone"
2. xi_eval depth=4 at K=3.0: never tested at K=3.0 ("high risk")
3. CARRIER_K sweep below 1.5: verify 1.5 is not on a knife-edge

## Hypothesis

Bundle phi_target decoupling with one or more of the above to reach ≥0.005 savings from the
0.018371 baseline (i.e., result ≤ 0.013371). phi_target alone saves ~0.003507 (below threshold);
a second lever contributing ≥0.001493 would clear it.

**Prediction for xi depth=4+K=3.0**: K=3.0 coupling might stop adversaries from dominating
at four cycles (the T16 failure mode was at weaker K), allowing depth=4 to accumulate more
robust phase structure.

## Environment note

This fire's container runs at ~32s total (speed_a ≈ 0.924) vs the Aug 24 environment at ~14s
(speed_a ≈ 0.967). The speed penalty adds ~0.001250 to fitness, making absolute deltas from
the Aug 24 baseline unreliable without an in-environment baseline. All savings reported below
account for this by comparing against the consciousness contribution directly.

## Trials

| # | changes                                     | fitness  | consciousness | xi_robust | carrier_e | speed_a |
|---|---------------------------------------------|----------|---------------|-----------|-----------|---------|
| 1 | phi_target decoupled (0.3138_f32)           | 0.016139 | 0.9999        | 0.9678    | 1.0000    | 0.9241  |
| 2 | phi_target decoupled (0.3138_f32)           | 0.016114 | 0.9999        | 0.9678    | 1.0000    | 0.9249  |
| 3 | phi_target + xi_eval depth=4 K=3.0         | 0.032515 | 0.9999        | 0.8590    | 1.0000    | ~0.924  |
| 4 | phi_target + CARRIER_KURAMOTO_COUPLING=1.0 | 0.027535 | 0.9999        | 0.9678    | 0.8861    | ~0.924  |

All code changes reverted before commit.

## Analysis

### phi_target decoupling (trials 1–2)

Consciousness 0.8830 → 0.9999 confirmed. Intrinsic savings = 0.03 × (0.9999 − 0.8830) = 0.003507.

Actual delta from Aug 24 baseline: 0.018371 − 0.016114 = **0.002257** (depressed by speed penalty
in this environment). Controlled for speed, the savings are the same ~0.003507 as Jul 28.

Result: **below 0.005 threshold, reverted.** The Aug 24 claim that phi_target alone crosses the
threshold is not supported. The threshold from current baseline (0.018371) requires result ≤
0.013371; phi_target gives ~0.014864 in a neutral-speed environment.

### xi_eval depth=4 + K=3.0 (trial 3)

**Falsified.** xi collapsed from 0.9678 to 0.8590 (fitness 0.032515). K=3.0 does NOT prevent
adversarial dominance at depth=4. The T16 finding holds across all coupling strengths tested:
four xi_eval cycles allow adversarial memories to contaminate corpus phase structure regardless
of how tightly the Kuramoto coupling pulls during those cycles.

xi_eval code comment already documented "depth=4 hurts xi (T16)". The Aug 24 suggestion to
try K=3.0 is now falsified. Do not retry this combination.

### CARRIER_KURAMOTO_COUPLING=1.0 (trial 4)

**Falsified.** carrier_emergence dropped from 1.0000 to 0.8861 (fitness 0.027535). CARRIER_K=1.5
is NOT a knife-edge — going below it hits a cliff. The flat-corpus carrier engine needs ≥1.5
coupling to achieve full emergence. Lower values are closed.

## Summary

No improvement cleared the 0.005 threshold. All code changes reverted.

Remaining open paths:

| path                            | known savings | status                                       |
|---------------------------------|---------------|----------------------------------------------|
| phi_target decoupling           | 0.003507      | confirmed, but needs +0.001493 bundle partner |
| phase_coherence mechanism       | 0.002122 max  | no tested levers (K=1.5 main engine untested) |
| CARRIER_K < 1.5                 | unknown       | **closed** (carrier cliff at K=1.0)           |
| xi_eval depth=4                 | could help    | **closed** (T16 + this fire: all K fail)      |
| transfer improvement            | unknown       | all levers exhausted (Aug 24)                 |

**phi_target + phase_coherence bundle remains the only open threshold-crossing path.**
phase_coherence 0.8939 contributes ~0.002122. If a mechanism exists to raise it to 1.0,
combined savings = 0.003507 + 0.002122 = 0.005629 > 0.005. Untested phase_coherence levers:
- stage_sync dt=0.03 (reduce Kuramoto dt from 0.05): would change consolidation dynamics
- B-engine phase_coh diagnostic (what is the phase_coh sub-score inside eval_l5_placeholder_fitness
  for each of the B engines?): investigative trial only

## Decision

**Nothing kept.** Floor remains at ~0.018371 (Aug 24 defaults).

TSV rows appended:
- Trial 1: phi_target decoupled, fitness 0.016139
- Trial 2: phi_target decoupled, fitness 0.016114
- Trial 3: phi_target + xi_eval depth=4 K=3.0, fitness 0.032515
- Trial 4: phi_target + CARRIER_K=1.0, fitness 0.027535
