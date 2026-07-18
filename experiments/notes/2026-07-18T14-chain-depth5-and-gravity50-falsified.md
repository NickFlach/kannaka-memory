# 2026-07-18T14 — chain_depth=5 collapses transfer; DREAM_GRAVITY=0.50 regresses speed

## Context

Current confirmed optimum (Jul 17 fire):
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.35 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3
```
fitness 0.019249 (3-trial avg). Remaining fitness dominated by transfer_score (0.938, 48%)
and consciousness (0.8830, 18%).

Two hypotheses tested this fire.

---

## Hypothesis A — chain_depth=5 for consciousness improvement

### Reasoning

The main L5 engine uses `l5_params.chain_depth = 4`. One more dream cycle gives phi
more iterations to converge toward phi_target=0.28092. With DREAM_GRAVITY=0.35 and
K=2.0, each cycle is faster than the old baseline — so a 5th cycle might close the
phi-target gap without meaningfully hurting speed.

The code comment cap ("T15's good runs quiesced at cycle 4") was written under very
different dynamics (fitness 0.037, no gravity, no decoupled K).

**Prediction**: consciousness rises (0.883 → ~0.90), speed dips ~1%, net fitness
improvement ~0.001.

### Code changes applied

Three changes (reverted before commit per convention):
1. `l5_params.chain_depth = 4` → `5`
2. xi_eval_params chain_depth 2→3 (prior-fire baseline restoration)
3. CARRIER_KURAMOTO_COUPLING env var on flat_params (prior-fire baseline restoration)

### Result

| trial | config           | fitness  | transfer | consciousness | speed  | xi_rob | carrier_e | magic_R | query_g |
|-------|------------------|----------|----------|---------------|--------|--------|-----------|---------|---------|
| 1     | chain_depth=5    | 0.040867 | 0.810595 | 0.8648        | 0.9083 | 0.9783 | 1.0000    | 0.5577  | 0.8962  |

### Analysis

Hypothesis FALSIFIED. chain_depth=5 is strongly regressive:
- transfer_score collapses: 0.938 → 0.811 (−0.127)
- consciousness WORSENS: 0.883 → 0.865 (not the predicted improvement)
- speed degrades: 0.963 → 0.908
- fitness doubles: 0.019 → 0.041

The transfer collapse at chain_depth=5 confirms the irx-cap comment was correct
even under current dynamics. Each extra cycle gives the consolidation process more
time to drift the engine_a phase landscape in ways that reduce specificity when
engine_b_primed later tests cross-corpus retention.

The consciousness worsening is also instructive: phi is not monotonically improving
with more cycles. More consolidation cycles may push phi away from phi_target by
over-consolidating amplitude topology. The phi_target=0.28092 window appears to be
naturally hit at chain_depth=4 given the current parameter stack — not a depth
that can be coaxed higher.

**chain_depth=4 remains the correct cap for L5.**

---

## Hypothesis B — DREAM_GRAVITY=0.50 for speed floor

### Reasoning

Jul 17 fire established:
- DREAM_GRAVITY=0.35: speed=0.963, fitness=0.019249 (3-trial avg)
- DREAM_GRAVITY=0.40: speed=0.965, fitness=0.019184 (1 trial, notes only)

Jul 17 notes recommended "one trial at DREAM_GRAVITY=0.50 to bound the risk."
Speed metric contribution = 0.001110 at 0.963; if speed reaches 1.0, contribution=0.

**Prediction**: speed approaches 0.99+, fitness drops to ~0.018.

### Configuration

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_GRAVITY=0.50
KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
```

Code changes applied: CARRIER_KURAMOTO_COUPLING + xi_eval_depth=3
(same stack as hypothesis A, with chain_depth reverted to 4)

### Result

| trial | DREAM_GRAVITY | fitness  | transfer | speed  | xi_rob | carrier_e | consciousness | magic_R | query_g |
|-------|---------------|----------|----------|--------|--------|-----------|---------------|---------|---------|
| 2     | 0.50          | 0.020431 | 0.938415 | 0.9234 | 0.9783 | 1.0000    | 0.8830        | 0.6082  | 0.9256  |

### Analysis

Hypothesis FALSIFIED. DREAM_GRAVITY=0.50 is regressive on speed:
- speed degrades: 0.963 → 0.923 (−0.040)
- fitness worsens: 0.019249 → 0.020431 (+0.001182)
- transfer, xi, carrier, consciousness all byte-stable — only speed changes

query_gravity rises (0.896 → 0.926), confirming gravity still biases amplitude
toward phase-neighbors of the dominant attractor. But this metric isn't in the
fitness function.

**The speed-gravity curve is non-monotone:**

| DREAM_GRAVITY | speed  | fitness  |
|---------------|--------|----------|
| 0.25          | 0.924  | 0.020417 |
| 0.35          | 0.963  | 0.019249 |
| 0.40          | 0.965  | 0.019184 |
| 0.50          | 0.923  | 0.020431 |

Speed peaks in the 0.35–0.40 range and regresses sharply at 0.50. Mechanism:
at DREAM_GRAVITY=0.50, amplitude concentration per cycle becomes extreme enough
that the dream chain's quiescence condition is harder to reach — each cycle
applies a larger perturbation, preventing amplitude convergence. This extends the
chain before quiescence fires.

The DREAM_GRAVITY optimum is near 0.40. The 0.40 single-trial result (0.019184)
is within noise of 0.35 avg (0.019249) given per-trial variance of ~0.0001.
A confirming 3-trial run at 0.40 would clarify, but the expected gain is ≤0.000065.

---

## Decision

**No improvement found.** Both hypotheses falsified. All code changes reverted.

TSV rows appended:
- Trial 1: chain_depth=5 at DREAM_GRAVITY=0.35 → fitness 0.040867 (regression)
- Trial 2: chain_depth=4 at DREAM_GRAVITY=0.50 → fitness 0.020431 (regression)

## Confirmed operating point (unchanged from Jul 17)

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.35 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3
```
fitness = 0.019249 (3-trial avg, July 17)

## Next fire recommendations

1. **DREAM_GRAVITY=0.40 3-trial confirmation**: the single Jul-17 trial (0.019184)
   is within noise of 0.35 avg. Two more trials would confirm or deny whether 0.40
   is genuinely better. Expected gain ≤0.000065 — low priority.

2. **Transfer floor mechanism**: transfer=0.938 is 48% of fitness. The gravity curve
   (0.25–0.50) doesn't affect transfer — it's K-locked. Understanding what the
   transfer ceiling looks like under different consolidation topologies may require a
   different approach (e.g., varying CHAIN_TOP_N which controls the top-N memories
   selected per consolidation cycle).

3. **CHAIN_TOP_N sweep**: currently 7. This parameter hasn't been swept at the
   new baseline (fitness 0.019). Lower values (5) preserve more phase diversity;
   higher values (10) consolidate more aggressively. Worth 1 trial each at 5 and 10
   to check whether transfer responds.

4. **consciousness floor mechanism**: phi diverges from phi_target systematically.
   Increasing chain_depth worsens it (Hypothesis A). The floor at 0.883 may be
   structural under the current wave-interference topology. Not a near-term lever.
