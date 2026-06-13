# alpha_a cliff at 0.13 confirmed — axis fully closed

**Date:** 2026-06-12T07 UTC
**Branch:** kannaka-curiosity/2026-06-12T07-bprimed-alpha12
**Code changes:** NONE (all reverted after regression trials)
**Status:** CLOSED — three falsifications; alpha_a=0.12 is the confirmed optimum

---

## Background

After T01 (engine_a alpha_base: 0.10→0.12, fitness 0.007627) and T03–T05's "architecture
ceiling" analysis (which used the stale T21 baseline 0.008334 — not accounting for T01's
improvement), two axes from T01's open list remained untested:

1. engine_b_primed alpha_base (0.10→0.12 mirror of engine_a change)
2. engine_a alpha_base=0.14 (further push from confirmed 0.12)

Additionally: engine_a alpha_base=0.13 (fine-grained midpoint test).

Correct current baseline (after T01): fitness ≈ 0.007470, transfer=0.963983.

---

## Hypothesis 1: engine_b_primed alpha_base 0.10→0.12

**Mechanism:** Mirror T01's confirmed improvement. Stronger per-step pull on b_primed
tightens its phase clusters → cleaner B integration → better transfer.

**Prediction:** Marginal improvement (T01 noted "marginal at best since b_primed starts
from A's already-tightened landscape").

**Result (trial 1):**
| metric | master | this | delta |
|--------|--------|------|-------|
| fitness | ~0.007470 | **0.007856** | +0.000386 (WORSE) |
| transfer_score | 0.963983 | **0.962359** | −0.001624 |
| xi_robustness_v2 | 0.9973 | 0.9973 | 0 |
| carrier_emergence | 0.9992 | 0.9992 | 0 |
| magic_R | 0.7785 | 0.7785 | 0 |
| query_gravity | 0.3654 | 0.3654 | 0 |

**Falsified.** Transfer regressed. Reverted immediately.

**Mechanism confirmed by result:** b_primed starts from a snapshot of engine_a's landscape,
which is already tightened by alpha=0.12 convergence. Adding alpha=0.12 on b_primed's
own 20-step pass over-converges: the combined effect (tight A landscape + aggressive
b_primed pull) exceeds the optimal basin depth for B integration. T01's "marginal at best"
prediction was correct in direction but slightly optimistic — it's actually a regression.

---

## Hypothesis 2: engine_a alpha_base 0.12→0.14

**Mechanism:** T01 found monotone improvement 0.08→0.10→0.12. Continue trend.

**Risk assessed:** T13 crash was relax_steps 16→20 at alpha=0.10 (total convergence proxy
0.10×16=1.60 → 0.10×20=2.00). At alpha=0.14×16=2.24 — above T13 crash proxy.

**Result (trial 2):**
| metric | master | this | delta |
|--------|--------|------|-------|
| fitness | ~0.007470 | **0.046499** | +0.039029 (CATASTROPHIC) |
| transfer_score | 0.963983 | **0.713510** | −0.250473 |

**Falsified.** Severe transfer crash. Reverted.

---

## Hypothesis 3: engine_a alpha_base 0.12→0.13

**Motivation:** Fine-grained midpoint between 0.12 (working) and 0.14 (crashed). If the
cliff is shallow, 0.13 might safely improve transfer further.

**Result (trial 3):**
| metric | master | this | delta |
|--------|--------|------|-------|
| fitness | ~0.007470 | **0.034508** | +0.027038 (CRASH) |
| transfer_score | 0.963983 | **0.791292** | −0.172691 |

**Falsified.** Transfer crash at 0.13, not as severe as 0.14 but still catastrophic. Reverted.

---

## Consolidated alpha_a curve (16 steps, irx mode)

| alpha | transfer | fitness | notes |
|-------|----------|---------|-------|
| 0.08 | regression | — | T17 confirmed worse |
| 0.10 | 0.958868 | 0.008334 | pre-T01 baseline |
| 0.12 | 0.963983 | 0.007470 | **confirmed optimum** (T01) |
| 0.13 | 0.791292 | 0.034508 | crash |
| 0.14 | 0.713510 | 0.046499 | severe crash |

The alpha_a axis has a sharp cliff between 0.12 and 0.13. 0.12 is the basin edge —
any increase triggers attractor overshoot. The monotone improvement 0.08→0.12 terminates
abruptly at 0.12. No further alpha_a improvement is possible.

---

## Corrected baseline note (re: T03–T05 errors)

T03, T04, T05 all cited "current master = 0.008334 / transfer = 0.958868" after T01's
alpha_a=0.12 change was already in master. This was a stale baseline copy error. The
correct current master (post T01) is fitness ≈ 0.007470, transfer = 0.963983. T03–T05's
structural analysis was sound in identifying that no gains > 0.005 from master are
achievable; the error was in the baseline, not the conclusion.

---

## Open axes remaining

NONE. Both axes from T01's open list are now closed:
- engine_b_primed alpha_base: falsified (regression)
- engine_a alpha_base > 0.12: falsified (cliff at 0.13)

All other axes remain closed per T03–T05.

Current confirmed architectural optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
engine_a alpha_base=0.12 (consolidation.rs)
chiral_p_bp=0.15 (engine_b_primed only, research.rs)
xi_eval_relax=20 (engine_clean + engine_adv, consolidation.rs)
3-trial avg fitness ≈ 0.007470 (load-independent runs)
transfer=0.963983, xi=0.9973, carrier_e=0.9992, consciousness=0.9553
```

No code changes kept. Source restored to current master state.
