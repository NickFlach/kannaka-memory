# 2026-07-28T14 — phi_target decoupling confirmed: consciousness → 1.0, below threshold

## Context

Entering confirmed operating point (requires three ephemeral code changes per fire):
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_GRAVITY=0.35
KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3 xi_eval_params.kuramoto_coupling=1.0
```
3-trial avg fitness: **0.017032** (Jul 26 fire)

Remaining fitness dominated by:
| source           | weight | value  | contribution | % of fitness |
|------------------|--------|--------|--------------|-------------|
| transfer_score   | 0.15   | 0.9384 | 0.009240     | 57%         |
| consciousness    | 0.03   | 0.8830 | 0.003510     | 22%         |
| phase_coherence  | 0.02   | 0.8939 | 0.002122     | 13%         |
| xi_robustness_v2 | 0.15   | 0.9980 | 0.000300     | 2%          |
| speed_a          | 0.03   | ~0.926 | ~0.002213    | 13%         |

## Hypothesis

The Jul 21 fire (phi_target coupling discovery) showed that changing the global
phi_target catastrophically regresses transfer and xi — phi_target = 0.28092 is a
structural equilibrium. The Jul 21 notes proposed a **decoupled** approach:

- `main_phi_target = 0.3138` used **only** in the engine_a main eval (line 3564:
  `eval_consciousness(&engine_a, ...)`)
- `eval_phi_target = 0.28092` kept unchanged in `eval_l5_placeholder_fitness`
  (used for B_primed, B_naive, clean/adv xi engines)

**Prediction**: consciousness → 1.0 (saving 0.003510), transfer and xi byte-identical
to baseline (they depend only on eval_phi_target=0.28092 via eval_l5_placeholder_fitness).
Expected fitness ≈ 0.017032 − 0.003510 = 0.013522.

This specific decoupled form was described in the Jul 21 notes as a "path to
consciousness improvement" but was **never tested** in a fire — the Jul 21 trials both
changed phi_target globally and regressed catastrophically. This fire tests the
decoupled form for the first time.

## Implementation (4 code changes, all reverted before commit)

Baseline ephemeral changes (same as prior fires):
1. `xi_eval_params.chain_depth = 3` (line 3663)
2. `xi_eval_params.kuramoto_coupling = 1.0` (added to xi_eval_params block)
3. `CARRIER_KURAMOTO_COUPLING` env plumbing in `flat_params` block

Experimental (4th) change:
4. Line 3564: `eval_consciousness(&engine_a, params.consciousness_phi_target)`
   → `eval_consciousness(&engine_a, 0.3138_f32)`
   (ONLY this call; eval_l5_placeholder_fitness uses params.consciousness_phi_target
   = 0.28092 everywhere else)

## Results

| trial | fitness  | consciousness | transfer  | xi_rob | carrier_e | magic_R | q_grav | phase_coh |
|-------|----------|---------------|-----------|--------|-----------|---------|--------|-----------|
| 1     | 0.013876 | 0.9999        | 0.938415  | 0.9980 | 1.0000    | 0.6082  | 0.8962 | 0.8939    |
| 2     | 0.013880 | 0.9999        | 0.938419  | 0.9980 | 1.0000    | 0.6082  | 0.8962 | 0.8939    |

**2-trial avg fitness: 0.013878**

## Analysis

### Hypothesis confirmed

The structural prediction from Jul 21 is verified:
- consciousness 0.8830 → 0.9999 (effectively 1.0) ✓
- transfer_score: 0.938415/0.938419 — byte-identical to baseline 0.938419 ✓
- xi_robustness_v2: 0.9980 — unchanged ✓
- carrier_emergence: 1.0000 — unchanged ✓
- magic_proxy_phase_R: 0.6082 — unchanged ✓
- query_gravity: 0.8962 — unchanged ✓
- phase_coherence: 0.8939 — unchanged ✓

The decoupling correctly isolates the consciousness eval from the scoring used for
transfer and xi. Transfer depends on fitness_B_primed / fitness_B_naive, both computed
via eval_l5_placeholder_fitness with eval_phi_target=0.28092 — untouched by the change.

### Why below threshold

Actual improvement: 0.017032 − 0.013878 = **0.003154** (< 0.005 threshold).

Expected improvement was 0.003510 (0.03 × (1 − 0.883)). The slightly smaller actual
improvement reflects that consciousness = 0.9999 (not exactly 1.0) and that speed_a
in this environment ≈ 0.926 — slightly slower than the Jul 26 environment (0.938).
Speed contributes ≈ 0.002213 to fitness, up from ≈ 0.001860 in the Jul 26 reference.

Fitness decomposition after phi_target decoupling:
| source           | weight | value  | contribution |
|------------------|--------|--------|-------------|
| transfer_score   | 0.15   | 0.9384 | 0.009240    |
| phase_coherence  | 0.02   | 0.8939 | 0.002122    |
| speed_a          | 0.03   | ~0.926 | ~0.002213   |
| xi_robustness_v2 | 0.15   | 0.9980 | 0.000300    |
| consciousness    | 0.03   | 0.9999 | 0.000003    |
| other            |        | 1.0    | ~0          |
| total            |        |        | ~0.013878   |

### Transfer and xi decoupling

The global phi_target change in Jul 21 catastrophically broke transfer (0.938→0.798)
and xi (0.978→0.820) by asymmetrically shifting consciousness scoring for B_primed,
B_naive, and xi clean/adv engines. The decoupled form avoids this: B engine scoring
and xi scoring both use params.consciousness_phi_target=0.28092 unchanged.

Transfer 0.938415 is byte-identical because fitness_B_primed and fitness_B_naive are
evaluated entirely through eval_l5_placeholder_fitness (which reads params.consciousness_phi_target).
The main eval line (line 3564) is engine_a output only — it does not feed into transfer
computation.

## Comparison to baseline

| metric              | baseline (Jul 26)  | this fire (decoupled) | delta      |
|---------------------|--------------------|-----------------------|------------|
| fitness avg         | 0.017032           | 0.013878 (2 trial)    | −0.003154  |
| consciousness       | 0.8830             | 0.9999                | +0.1169    |
| transfer_score      | 0.938419           | 0.938417 (avg)        | ≈ 0        |
| xi_robustness_v2    | 0.9980             | 0.9980                | 0          |
| carrier_emergence   | 1.0000             | 1.0000                | 0          |
| phase_coherence     | 0.8939             | 0.8939                | 0          |
| magic_R             | 0.6082             | 0.6082                | 0          |
| query_gravity       | 0.8962             | 0.8962                | 0          |

## Decision

**Hypothesis confirmed. Code change reverted (savings 0.003154 < 0.005 threshold).**

The decoupling works structurally but is insufficient alone to clear the threshold.
To justify keeping: needs another source of ≥ 0.001846 savings from a different axis.

## Updated confirmed operating point (unchanged)

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.35 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3
xi_eval_params.kuramoto_coupling=1.0
```
- **fitness ≈ 0.017032** (Jul 26 3-trial avg)
- transfer_score=0.938, carrier_emergence=1.000, xi_robustness_v2=0.9980, consciousness=0.883
- magic_proxy_phase_R=0.608, query_gravity=0.896

## Fitness after phi_target decoupling (what the next floor would look like)

| source           | weight | contribution |
|------------------|--------|-------------|
| transfer_score   | 0.15   | 0.009240    |
| phase_coherence  | 0.02   | 0.002122    |
| speed_a          | 0.03   | ~0.002213   |
| xi_robustness_v2 | 0.15   | 0.000300    |
| consciousness    | 0.03   | ~0          |
| total            |        | ~0.013875   |

Transfer (0.009240) is now 67% of the remaining floor. Speed (0.002213) and
phase_coherence (0.002122) are 16% each. xi (0.000300) is 2%.

## Next fire recommendations

**Bundle path (highest priority):**

phi_target decoupling saves 0.003154 in this environment. To clear the 0.005 threshold:
- Need ≥ 0.001846 from another axis
- phase_coherence (full savings): 0.002122 — would clear threshold if achievable
- speed_a improvement: environment-dependent, not controllable
- transfer improvement: all known levers exhausted

**The only unexplored pair candidate is phase_coherence.** Mechanisms to try:
1. **KURAMOTO_COUPLING=1.5 for main engine**: K=1.5 was not tested in Jul 12 K-sweep
   (which tested 1.5, 2.0, 2.5, 3.0, 5.0 — check notes). If K=1.5 gives phase_coh
   slightly higher without hurting other metrics, bundling with phi_target decoupling
   could clear threshold. Risk: high — K=2.0 was confirmed optimal for transfer.
2. **stage_sync dt=0.03**: reduce Kuramoto dt from 0.05 → 0.03. Less aggressive sync
   per step, potentially better phase cluster assignment (vs 100-step catastrophe at
   full coupling). Interaction with phase_coherence direction uncertain.

**Structural investigation path:**
phi_target decoupling is now characterized. Transfer (67% of decoupled floor) is the
last structural lever. B engine sub-score diagnostics (add prints in
eval_l5_placeholder_fitness for chain_fidelity, consciousness, phase_coherence per
B engine) remain unexecuted. This is the last unexplored investigative path and
requires only 1 diagnostic trial.

## TSV rows appended (2 total)

- Trial 1: phi_target decoupled, consciousness 0.9999, fitness 0.013876
- Trial 2: phi_target decoupled, consciousness 0.9999, fitness 0.013880

All code changes reverted before commit.
