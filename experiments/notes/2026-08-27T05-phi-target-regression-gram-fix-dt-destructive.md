# 2026-08-27T05 — phi_target decoupling now net-negative; dt=0.03 destructive

## Context

Aug 25 fire left two open levers claiming a combined threshold-crossing path:
- phi_target decoupling (0.28092→0.3138): confirmed +0.003507 savings in Aug 25 fire
- stage_sync dt reduction (0.05→0.03): untested, predicted to raise phase_coherence

Combined prediction: 0.003507 (consciousness) + ~0.002122 (phase_coherence) ≥ 0.005 threshold.

This fire's container baseline: fitness 0.019237, xi 0.9678, transfer 0.954001, phase_coherence 0.8939,
consciousness 0.8830, carrier_emergence 1.0000.

## Trials

| # | changes                          | fitness  | xi_robust | transfer | carrier_e | phase_coh | consciousness |
|---|----------------------------------|----------|-----------|----------|-----------|-----------|---------------|
| 0 | bare baseline (no code changes)  | 0.019237 | 0.9678    | 0.954001 | 1.0000    | 0.8939    | 0.8830        |
| 1 | phi_target=0.3138 + dt=0.03      | 0.104895 | 0.7043    | 0.815852 | 0.7116    | 0.9012    | 0.9942        |
| 2 | phi_target=0.3138 only           | 0.040756 | 0.9115    | 0.843628 | 1.0000    | 0.8939    | 0.9999        |
| 3 | phi_target=0.3138 only (repeat)  | 0.040749 | 0.9115    | 0.843621 | 1.0000    | 0.8939    | 0.9999        |

All code changes reverted before commit.

## Analysis

### dt=0.03 (trial 1)

**Destructive.** Reducing Kuramoto dt from 0.05 to 0.03 (same 50 steps = 40% less total
phase evolution per dream cycle) collapses consolidation across all engines:
- transfer_score: 0.954 → 0.815 (–0.139)
- carrier_emergence: 1.000 → 0.712 (–0.288)
- xi_robustness_v2: 0.968 → 0.704 (–0.264)

The phase_coherence improvement (0.8939→0.9012, +0.0073) is dwarfed by everything else
collapsing. Closed. Do not retry dt < 0.05.

### phi_target=0.3138 (trials 2–3)

**Now net-negative.** Consciousness rises (0.8830→0.9999, saving 0.03 × 0.1169 = 0.003507),
but xi drops from 0.9678 to 0.9115 (loss: 0.15 × 0.0563 = 0.008445). Net: −0.004938.
Overall fitness: 0.019237 → 0.040749 (+0.021512 WORSE, heavily driven by transfer degradation).

**Mechanism:** xi_eval_params inherits consciousness_phi_target from l5_params. After commit
3faeb6c (Gram matrix fix, 2026-08-26), the adversarial sub-engine's phi is now computed
accurately after adversarial deletion (previously the stale rows from deleted adversaries
inflated phi toward a neutral value, masking the adversarial phi suppression). With the fixed
Gram, adv-engine phi is genuinely lower post-deletion, and phi_target=0.3138 amplifies the
clean-vs-adv sub-fitness gap because consciousness_adv is now penalized much more than before:

  - clean sub-fitness: consciousness = 1.0 (phi ≈ 0.3138 = target)
  - adv sub-fitness (post-fix): consciousness ≈ 0.72 (adversaries suppress phi below target)
  - Δ consciousness in sub-fitness: 0.28 vs old Δ ≈ 0.01

xi = 1 - |fitness_clean - fitness_adv| / max(fitness_clean, 0.05) therefore drops sharply.

**This explains the Aug 25 fire discrepancy.** Aug 25 phi_target=0.3138 trials (1 and 2) ran
BEFORE commit 3faeb6c was merged (that commit landed at 2026-08-26T00). With the buggy Gram,
adv phi was biased toward clean phi, masking the adversarial impact. The Aug 25 result
(xi=0.9678 with phi_target=0.3138) was a false positive contingent on the Gram bug.

**phi_target decoupling is now closed.** The Gram fix makes it structurally net-negative.

## Summary

| path                            | status after this fire                            |
|---------------------------------|---------------------------------------------------|
| phi_target=0.3138 decoupling    | **CLOSED** — net −0.004938 (Gram fix exposed)     |
| stage_sync dt=0.03              | **CLOSED** — massive collateral damage            |
| phi_target + phase_coh bundle   | **CLOSED** — both constituent levers now closed   |
| xi_eval depth=4 (all K)         | Closed (T16 + Aug 25)                            |
| CARRIER_K < 1.5                 | Closed (Aug 25)                                  |
| transfer levers                 | Exhausted (Aug 24)                               |

## Decision

**Nothing kept.** All code changes reverted. Floor remains at ~0.019 (this container).

The Aug 25 open-path claim (phi_target + phase_coh bundle) was based on a false positive from
the pre-Gram-fix environment. With the Gram fix in master, phi_target decoupling is net-negative
and the bundle path is closed. No threshold-crossing path remains identified.

TSV rows appended (4 rows: baseline probe + 3 trials):
- Trial 0: bare baseline, fitness 0.019237
- Trial 1: phi_target=0.3138 + dt=0.03, fitness 0.104895
- Trial 2: phi_target=0.3138, fitness 0.040756
- Trial 3: phi_target=0.3138 repeat, fitness 0.040749
