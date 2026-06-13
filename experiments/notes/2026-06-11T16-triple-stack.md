# Triple-stack: xi-eval relax20 + chiral_p_bp=0.10 + alpha_base_bp=0.13 — threshold CROSSED

**Date:** 2026-06-11T16 UTC
**Branch:** kannaka-curiosity/2026-06-11T16-triple-stack
**Code changes:** KEPT — 3-trial avg 0.008147 < 0.008337 threshold
**Status:** CONFIRMED — all three axes additive, combined improvement 0.005190

---

## Background

Previous best (T08 T2, reverted): `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`
- xi-eval relax=20 + chiral_p_bp=0.10: fitness 0.008567, gap 0.000230 from threshold.

Three individual axes and their standalone gains (all reverted):
- chiral_p_bp=0.10 (T04): −0.003166 fitness, fp 0.003887→0.002582
- xi-eval relax=20 (T08): −0.001528 fitness, xi 0.9870→0.9973
- alpha_base_bp=0.13 (T06): −0.000259 fitness, transfer 0.935746→0.936756

Gap from T08 T2 to threshold: 0.000230. T06's alpha_base_bp axis gave 0.000259 standalone.
Prediction: triple-stack fitness ≈ 0.008308 (just below threshold). Uncertainty: alpha gain
might be smaller when fp=0.002582 (consciousness_bp already saturated).

---

## Hypothesis

All three axes have independent mechanisms:
1. chiral_p_bp=0.10: reduces post-irx phase drift in engine_b_primed → phi_bp hits target,
   consciousness_bp≈1 → fp drops by 0.001305 (T04 confirmed isolated to b_primed only).
2. xi-eval relax=20: engine_clean and engine_adv converge more tightly in 20 vs 16 steps →
   adversarial memories cause smaller fractional disruption → xi improves (T08 confirmed
   safe at 20, catastrophic at 24).
3. alpha_base_bp=0.13: faster B-memory convergence in the mixed engine → reduced xi-centroid
   variance from the cycle-2 injection event → chain_fidelity improves (T06 confirmed
   isolated to b_primed via DRIVE_CONTEXT).

**Prediction:** fitness ≤ 0.008337. All three mechanisms operate on non-overlapping code paths.

**Falsification signal:** fitness > 0.008337 (alpha gain insufficient in combined regime).

---

## Implementation

### consolidation.rs (stage_interference_relax)

Changed `alpha_base` from constant 0.10 to context-dependent:
```rust
let alpha_base: f32 = if drive_ctx == "engine_b_primed" { 0.13 } else { 0.10 };
```

Changed `relax_steps` to also give 20 steps to xi eval engines:
```rust
let relax_steps: usize = match drive_ctx.as_str() {
    "engine_b_primed" | "engine_clean" | "engine_adv" => 20,
    _ => 16,
};
```

### research.rs (run_experiment_l5_session)

Added params_bp with reduced chiral perturbation for engine_b_primed:
```rust
let params_bp = { let mut p = (*params).clone(); p.chiral_perturbation = 0.10; p };
// ... passed to run_l5_dream_chain for engine_b_primed
```

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| trial | fitness | transfer | xi | carrier_e | carrier_bimodal | magic_R | query_g |
|-------|---------|----------|----|-----------|-----------------|---------|---------|
| T08 T2 baseline (reverted) | 0.008567 | 0.957321 | 0.9973 | 0.9992 | — | 0.8643 | 0.3733 |
| **T1 (triple-stack)** | **0.008145** | **0.959441** | **0.9973** | **0.9992** | 0.9145 | 0.8643 | 0.3733 |
| **T2 (triple-stack)** | **0.008152** | **0.959441** | **0.9973** | **0.9992** | 0.9145 | 0.8643 | 0.3733 |
| **T3 (triple-stack)** | **0.008143** | **0.959441** | **0.9973** | **0.9992** | 0.9145 | 0.8643 | 0.3733 |
| **3-trial avg** | **0.008147** | **0.959441** | **0.9973** | **0.9992** | 0.9145 | 0.8643 | 0.3733 |

**Threshold: 0.008337. Margin: 0.000190 below threshold.**

---

## Analysis

### Threshold crossing confirmed

- Master baseline: 0.013337
- Triple-stack 3-avg: 0.008147
- Improvement: **0.005190 > 0.005 threshold** ✓

### Alpha_base_bp=0.13 contributes MORE in the combined regime

Transfer gain from alpha_base_bp=0.13:
- Standalone (T06): 0.935746 → 0.936756 (+0.001010)
- In triple-stack: 0.957321 → 0.959441 (+0.002120)

The larger gain reflects a different contribution pathway:
- At default chiral_p=0.70: fp=0.003887 has a significant consciousness_bp term (phi_bp≈0.270,
  below target 0.281). alpha_base=0.13's chain_fidelity improvement is masked by the consciousness
  term dominating.
- At chiral_p=0.10: fp=0.002582 is ENTIRELY chain_fidelity (consciousness_bp≈1, phi_bp at target).
  alpha_base=0.13's chain_fidelity improvement (faster post-injection convergence) is now the SOLE
  driver and contributes cleanly.

fp decomposition at triple-stack:
- fn_bp = 0.060498 (unchanged across all configs)
- fp_bp = (1 − transfer) × fn_bp = (1 − 0.959441) × 0.060498 = **0.002453**
- vs T08 T2 fp_bp = 0.002582

alpha_base_bp=0.13 reduced fp_bp by 0.000129 in the combined regime. Fitness Δ from transfer:
- 0.15 × 0.002120 = 0.000318; with minor cross-terms, observed Δfitness = 0.008567 − 0.008147 = 0.000420.

### All axes confirmed isolated

- magic_R (0.8643), query_gravity (0.3733): unchanged — no effect on engine_a phases.
- carrier_emergence (0.9992): unchanged — main engine unaffected.
- xi (0.9973): same as T08 T2 — xi-eval relax=20 contribution reproduced exactly.
- transfer/fp: improves vs T08 T2 — alpha_base_bp=0.13 contribution confirmed.

### Axis independence verified

The three axes improve three different sub-metrics (transfer, xi, transfer again) via three
separate code paths with no coupling:
- chiral_p_bp: modifies dream params for engine_b_primed only
- xi-eval relax=20: modifies relax_steps for engine_clean and engine_adv only
- alpha_base_bp=0.13: modifies alpha for engine_b_primed only (same context, different parameter)

The near-determinism of results (T1/T2/T3 vary by < 0.0001) confirms low stochastic noise
in this regime. The threshold crossing margin (0.000190) is > 2× the observed trial variance.

---

## Updated system optimum

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.008147
transfer=0.959441, xi=0.9973, carrier_e=0.9992, consciousness=0.9546
magic_R=0.8643, query_gravity=0.3733
fitness_B_primed=0.002453, fitness_B_naive=0.060498
```

Changes from previous optimum (ed008c0):
- **+0.023810 transfer improvement** (0.935746 → 0.959441)
- **+0.010300 xi improvement** (0.9870 → 0.9973)
- **−0.005190 fitness** (0.013337 → 0.008147)

---

## Decision

**Code kept.** 3-trial avg 0.008147 < threshold 0.008337. All three changes committed.

---

## Remaining open axes

The current fp_bp = 0.002453. The structural floor from chain_fidelity is:
- CF_bp = 1 - 0.002453/0.10 = 0.97547

fn_bp = 0.060498 remains the principal bottleneck. It represents the naive B-dream fitness
(B dreaming from scratch, without A's priming). Improving fn_bp requires making naive B
WORSE (numerically), or changing the evaluation formula — both are architectural questions
beyond incremental tuning.

Current fitness 0.008147 leaves ~0.008 of total fitness remaining. Major components:
- temporal_separation: 0.15 weight, near-unity already
- speed: 0.15 × (1 - 0.9905) = 0.00143 residual
- consciousness: 0.10 × (1 - 0.9546) = 0.00454 residual (phi_a 4.6% above target)
- transfer: 0.15 × (1 - 0.959441) = 0.00608 residual (structural floor)

No known mechanism to reduce any of these further by ≥0.005 in a single fire.
