# Xi-eval depth sweep: depth=2 is uniquely optimal — coupling characterized

**Date:** 2026-06-10T00 UTC
**Branch:** kannaka-curiosity/2026-06-10T00-xi-depth-revert-post-bfs
**Code changes:** NONE — all reverted. Notes-only commit.
**Status:** No improvement found. Baseline 0.030 confirmed stable.

---

## Background

Entering this fire, current master = T18 (xi-depth=2) + T20 (adv deletion in xi eval) + T21 (BFS sort revert):

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.030 (deterministic)
transfer=0.903, xi=0.907, carrier_e=0.999
magic_R=0.877, query_gravity=0.374
```

Fitness breakdown:
- xi (0.15): 0.15 × 0.093 = **0.014** (47%)
- transfer (0.15): 0.15 × 0.097 = **0.015** (50%)
- other: 0.001 (3%)

The notes for T21 showed xi=0.985 before T20 adv deletion was incorporated and before
T18's depth=2 override. The gap between T21-branch (0.028) and current master (0.030)
motivated three hypotheses about the xi eval configuration.

---

## Trial 1: xi eval depth=4 (revert T18)

**Hypothesis:** With T21's BFS sort fixed, adversarials cluster last — depth=2 is no
longer needed to limit disruption time. Restoring depth=4 should recover xi to T21's
0.985 while transfer stays at 0.903.

**Change:** `p.chain_depth = 2` → `p.chain_depth = 4` in xi_eval_params

**Result (T1 only — clearly worse, no further trials):**

| metric | baseline (depth=2) | trial (depth=4) | delta |
|--------|-------------------|-----------------|-------|
| fitness | 0.030 | 0.044 | +0.014 **REGRESSION** |
| xi | 0.907 | 0.878 | −0.029 |
| transfer | 0.903 | 0.848 | −0.055 |
| carrier_e | 0.999 | 0.998 | −0.001 |

**Verdict:** Falsified. Depth=4 regresses BOTH xi and transfer vs depth=2.

**Key observation:** Transfer dropped from 0.903→0.848 even though transfer is computed
BEFORE eval_xi_robustness_v2 in the code. The coupling mechanism is unexplained but
empirically consistent across trials.

---

## Trial 2: Remove T20's adversarial deletion

**Hypothesis:** T20 added adversarial deletion from engine_adv before fitness_adv eval.
This was designed to fix xi=0.681 when BFS sort clustered adversarials first. T21 fixed
the BFS sort — adversarials now cluster last. Without BFS amplification, the deletion
may be adding measurement noise. Removing it should recover xi toward T21's 0.985
while leaving transfer at 0.903.

**Change:** Removed the adv_ids collection + delete loop after the engine_adv dream chain

**Result (T1 only — neutral, no further trials):**

| metric | baseline (with deletion) | trial (no deletion) | delta |
|--------|--------------------------|---------------------|-------|
| fitness | 0.030 | 0.031 | +0.001 neutral |
| xi | 0.907 | 0.963 | +0.056 |
| transfer | 0.903 | 0.848 | −0.055 |

**Verdict:** Neutral. Xi improves but transfer drops proportionally. Net fitness
unchanged (0.030→0.031). The xi-transfer trade-off is symmetric to ~0.001 fitness.

---

## Trial 3: xi eval depth=1

**Hypothesis:** The depth=4→2 transition improved BOTH xi (0.878→0.907) AND transfer
(0.848→0.903) in the T20+T21 combined context. Perhaps depth=1 continues this trend,
improving both metrics further.

**Change:** `p.chain_depth = 2` → `p.chain_depth = 1` in xi_eval_params

**Result (T1 only — worse than depth=2, no further trials):**

| metric | baseline (depth=2) | trial (depth=1) | delta |
|--------|-------------------|-----------------|-------|
| fitness | 0.030 | 0.032 | +0.002 neutral/worse |
| xi | 0.907 | 0.957 | +0.050 |
| transfer | 0.903 | 0.848 | −0.055 |
| fitness_B_primed | — | 0.009146 | — |
| fitness_B_naive | — | 0.060190 | — |

**Verdict:** Falsified. Depth=1 drops transfer back to 0.848 just like depth=4.
Only depth=2 achieves transfer=0.903.

---

## Synthesis: depth=2 is uniquely optimal

| xi eval depth | xi   | transfer | fitness |
|---------------|------|----------|---------|
| depth=4       | 0.878 | 0.848   | 0.044   |
| **depth=2**   | **0.907** | **0.903** | **0.030** |
| depth=1       | 0.957 | 0.848   | 0.032   |
| depth=2, no deletion | 0.963 | 0.848 | 0.031 |

**Depth=2 is a unique operating point where transfer=0.903.** At all other tested
configurations, transfer=0.848 (T21-level). The coupling mechanism is unexplained:
transfer is computed BEFORE eval_xi_robustness_v2 in the code, yet xi eval depth
consistently affects the transfer value.

Possible explanations (unverified):
1. Rust optimizer / LLVM reorders computation such that xi eval state affects
   earlier values through some indirect path
2. A global lazy-static or thread-local is initialized differently depending on
   xi eval code path, and this feeds back into transfer computation
3. The xi eval at depth=2 consumes a specific number of allocations/RNG calls
   that put the heap/state in a configuration that affects chain selection in a
   shared subfunction

Without resolving this coupling mechanism, further xi/transfer adjustments are
unpredictable. Experiments that "improve xi at the cost of transfer" or vice versa
will continue to find a conservation law.

---

## TSV contamination note

Three exploratory rows were appended to experiments/results-L5.tsv during this fire:
- `L5	0.043862` — depth=4 xi eval (trial 1, intentional experiment)
- `L5	0.031224` — no adv deletion (trial 2, intentional experiment)
- `L5	0.032103` — depth=1 xi eval (trial 3, intentional experiment)

All three used DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax.

---

## Open axes

| axis | expected gain | mechanism | blocking issue |
|------|---------------|-----------|----------------|
| Transfer via B-primed-specific change | −0.005 to −0.015 | Something that affects ONLY engine_b_primed's dream quality without going through eval_xi_robustness_v2 | Need to identify what limits fitness_B_primed | 
| Diagnose xi/transfer coupling | N/A | Understand why xi eval depth affects transfer | Add debug output showing transfer BEFORE and AFTER xi eval call |
| xi via source-level adversarial isolation | −0.010 | Limit adversarial link formation during dreaming via DRIVE_CONTEXT | Large change, uncertain |

**Priority:** Before touching xi eval again, add a debug print showing transfer_score
immediately after it is computed (line 3479) and confirm whether it changes with xi eval
depth. If the value is the same at print time but different in the TSV row, there is a
late-binding issue in the logging code.

---

## Decision

**No code changes retained.** All three hypotheses neutral/negative. Current master at
0.030 is the local optimum for xi/transfer configuration. The depth=2 sweet spot should
not be changed without understanding the coupling mechanism.
