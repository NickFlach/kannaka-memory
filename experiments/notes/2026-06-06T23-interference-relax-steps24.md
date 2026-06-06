# Hypothesis: interference_relax relax_steps=24, alpha_base=0.067

**Date:** 2026-06-06T23 UTC
**Branch:** kannaka-curiosity/2026-06-06T23
**Code changes:** Reverted — no improvement (avg fitness 0.153 > 0.149 baseline)

---

## Background

T00 (2026-06-06T00) established that `stage_interference_relax` at alpha_base=0.10,
relax_steps=16 (total coupling budget = 1.6) achieved 3-trial avg fitness 0.149 vs
the prior 8-step/alpha=0.20 result of ~0.21. The T00 mechanism: finer phase updates
at the same total coupling budget produce smoother convergence, lifting xi_robustness_v2
from ~0.083 to avg ~0.607.

The total coupling budget intuition from T00: "~1.6 units preserves carrier dynamics,
~3.2 units kills them." T00 inferred carrier_e is governed by total coupling budget.

Current optimum:
- stage_sync K=1.0: 3-run avg fitness ≈ 0.138
- interference_relax steps=16, alpha=0.10: 3-run avg fitness ≈ 0.149

---

## Hypothesis

If the T00 improvement (steps 8→16, same budget) was driven by finer phase granularity,
then steps 16→24 at proportionally reduced alpha (0.10 → 0.067, budget still ~1.6)
should continue the trend: even finer updates → cleaner phase convergence → xi rises
toward ~0.70 → fitness approaches stage_sync K=1.0 territory (~0.138).

**Prediction:**
- xi_robustness_v2 rises from avg ~0.607 → ~0.70+
- carrier_emergence stays ~0.497 (same total coupling budget)
- fitness drops from avg 0.149 → ~0.142 or below

---

## Code change

`src/consolidation.rs` — `stage_interference_relax`:
```
- let alpha_base: f32 = 0.10;
- let relax_steps: usize = 16;
+ let alpha_base: f32 = 0.067;
+ let relax_steps: usize = 24;
```

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| trial | fitness | xi_robustness_v2 | carrier_emergence | transfer_score | magic_R | query_gravity |
|-------|---------|-----------------|-------------------|----------------|---------|---------------|
| t1 | 0.145718 | 0.4676 | 0.7186 | 0.763218 | 0.5989 | 0.3651 |
| t2 | 0.102228 | 0.7559 | 0.7186 | 0.764839 | 0.5989 | 0.3651 |
| t3 | 0.209957 | 0.0236 | 0.7186 | 0.778987 | 0.5989 | 0.3651 |
| **avg** | **0.1526** | **0.416** | **0.719** | **0.769** | **0.599** | **0.365** |

**Reference (steps=16, alpha=0.10, T00 3-trial avg):** fitness 0.149, xi avg 0.607,
carrier_e 0.497, transfer 0.750, magic_R 0.617.

---

## Findings

### 1. Hypothesis falsified — no fitness improvement

3-trial avg 0.153 > 0.149 baseline. Delta = +0.004 (worse). Does not meet the
≥0.005 improvement threshold. **No code change kept.**

The xi prediction was wrong: xi mean dropped from 0.607 to 0.416, and variance
*increased* (range 0.024–0.756 vs T00's 0.294–0.925). The key-bad trial (t3, xi=0.024)
produced catastrophically low xi. The finer-steps mechanism does not consistently lift xi.

### 2. Carrier emergence: total coupling budget theory refuted

The main surprise: carrier_e jumped from 0.497 (steps=16) to 0.719 (steps=24).

| steps | alpha | budget | carrier_e |
|-------|-------|--------|-----------|
| 8 | 0.20 | 1.60 | ~0.714 (T00 trial 1, ref) |
| 16 | 0.20 | 3.20 | 0.000 (T00 trial 2, killed) |
| 16 | 0.10 | 1.60 | 0.497 (T00 confirmed) |
| 24 | 0.067 | 1.61 | **0.719** (this fire) |

At the same total coupling budget (1.6), carrier_e ranges from 0.497 to 0.719.
The T00 "total coupling budget governs carrier_e" theory is wrong. The correct
variable appears to be **per-step alpha**, not total budget: carrier_e is preserved
when per-step phase change is small (alpha ≤ ~0.08), and degrades when per-step
alpha is large (0.10 at steps=16, even more so at 0.20 at steps=16).

Steps=8, alpha=0.20 also preserved carrier_e (0.714) — but this is because 8 steps
gives less total phase integration time even at large per-step alpha. The full pattern:

| per-step alpha | steps | carrier_e |
|---------------|-------|-----------|
| 0.20 | 8 | 0.714 |
| 0.10 | 16 | 0.497 |
| 0.067 | 24 | 0.719 |
| 0.20 | 16 | 0.000 |

Carrier emergence is nonlinearly sensitive to the phase relaxation profile —
not to any simple total-budget summary. This is a structural property of the
carrier FFT metric (peak_power/total_power) responding to phase arrangement details.

### 3. magic_R and query_gravity invariant to steps count

magic_R: 0.599 (this fire) vs 0.617 (T00). Small difference, likely not significant
given the few-trial sample. query_gravity: 0.365 both fires. These metrics are
structurally invariant to relax_steps within the interference_relax mode.

---

## Decision

**Code reverted.** Steps=16, alpha=0.10 (T00) remains the interference_relax optimum.

The empirical ordering remains:
1. stage_sync K=1.0: avg fitness **0.138** (best)
2. interference_relax steps=16, alpha=0.10: avg fitness **0.149**
3. This fire (steps=24, alpha=0.067): avg fitness **0.153** (worse)

---

## Implications for future fires

1. **Xi stochasticity is the binding constraint under interference_relax.** Variance
   is driven by the adversarial perturbation RNG in eval_xi_robustness_v2 (unseeded).
   Finer relaxation steps do not reduce xi variance — they may increase it. The
   only reliable way to improve interference_relax fitness is to either (a) increase
   the xi *mean* dramatically or (b) accept the current variance and use more trials.

2. **Carrier emergence has a sweet spot in per-step alpha, not total budget.**
   alpha ≈ 0.067 recovers carrier_e to 0.719 (matching the original 8-step value).
   This means there's a potential hybrid: if we could get xi up to ~0.75 at
   carrier_e=0.719, the fitness could drop substantially. But the current experiments
   show no way to raise xi while keeping alpha small.

3. **The carrier↔xi tradeoff at steps=16 is not fundamental** — steps=24 gets
   carrier_e=0.719 at the cost of lower mean xi. The tradeoff at steps=16 (carrier_e
   lower, xi somewhat higher mean) may be the better operating point for overall fitness
   given current metric weights (xi: 0.15, carrier_e: 0.10).

4. **Stage_sync K=1.0 remains the production optimum.** The interference_relax
   mode has higher magic_R (0.599–0.617 vs 0.250) and higher transfer_score
   (0.763–0.769 vs 0.682), but cannot match stage_sync on xi without a structural
   change to how constructive-pair relaxation interacts with the xi adversary.

5. **Open: destructive-pair repulsion in stage_interference_relax.** Currently the
   mode only pulls phases toward constructive neighbors. Adding a repulsive term from
   destructive pairs could actively improve phase separation and xi, without changing
   the total coupling budget. This is the most promising structural change not yet tested.
