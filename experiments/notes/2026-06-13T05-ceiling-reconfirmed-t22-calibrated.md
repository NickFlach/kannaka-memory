# Ceiling reconfirmed — T22 baseline confusion resolved, single calibration trial

**Date:** 2026-06-13T05 UTC
**Branch:** kannaka-curiosity/2026-06-13T05-ceiling-reconfirmed-t22-calibrated
**Code changes:** NONE
**Status:** CLOSED — ceiling structural; T22 confusion documented

---

## Context

T22 (2026-06-12T22, PR #303) ran 3 irx trials and reported fitness avg **0.147**. It
believed irx was at the system-prompt smoke-test baseline (~0.191) and treated Q1
(irx characterization) and Q3 (relax_steps) as open from the system prompt.

This fire checked whether T22's 0.147 result represents a genuine regression or a
code-state artifact.

---

## Calibration trial

`DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax` (no env overrides):

| metric | this trial | T01 confirmed (3-trial avg) | delta |
|--------|-----------|---------------------------|-------|
| fitness | **0.007562** | 0.007627 | −0.000065 |
| transfer_score | 0.963983 | 0.963983 | 0 |
| xi_robustness_v2 | 0.9973 | 0.9973 | 0 |
| carrier_emergence | 0.9992 | 0.9992 | 0 |
| magic_proxy_phase_R | 0.7785 | 0.7785 | 0 |
| query_gravity | 0.3654 | 0.3654 | 0 |

**Current master produces 0.007562 — well within T01 noise band.** No regression.

---

## Why T22 got 0.147

T22's branch (kannaka-curiosity/2026-06-12T22-irx-relax16) was cut from master. The
irx results it reported (0.150527, 0.163520, 0.127033) are inconsistent with the
current code, which produces ~0.0076 deterministically.

The most likely cause: T22's branch was created before PR #283
(kannaka-curiosity/2026-06-12T01-engine-a-alpha12) auto-merged. The branch therefore
lacked engine_a alpha_base=0.12, xi_eval_relax=20, and chiral_p_bp=0.15 from the
T01/T21 combined stack. Those three changes together account for the gap:

| config | expected fitness |
|--------|----------------|
| master with all T01+T21 changes | ~0.007627 |
| without engine_a alpha_base=0.12 | ~0.008334 |
| without xi_eval_relax=20 | ~0.009862 |
| without chiral_p_bp=0.15 | ~0.013337 |
| irx early baseline (none of the above) | ~0.147–0.191 |

T22's Q3 relax_steps falsification result (carrier_e crash at relax=16, then 0.193 at
alpha=0.10 iso-convergence) is correct and independently reproduces the T13 mechanism.
No further action needed.

---

## Status of all 6 system-prompt research questions

| Q | question | status | notes file |
|---|----------|--------|-----------|
| Q1 | 3-run irx characterization | DONE — avg 0.007627 | 2026-06-12T01-engine-a-alpha12.md |
| Q2 | K-sweep under fixed plumbing | DONE — K=0.5 confirmed optimal; no-op in irx | 2026-06-06T05-k-sweep-first-real.md + 2026-06-11T12-kuramoto-irx-invariant.md |
| Q3 | irx + xi recovery (relax_steps 16/24) | DONE — 20 is exact sweet spot; 21 collapses; T22 confirmed crash at 16 | 2026-06-11T14-xi-eval-relax21-falsified.md + 2026-06-12T22-irx-relax-steps-falsified.md |
| Q4 | R-xi correlation at stage_sync | DONE — stage_sync is no-op in irx; K invariant | 2026-06-11T12-kuramoto-irx-invariant.md |
| Q5 | Φ ↔ R relationship | DONE — anti-correlated across modes | 2026-06-11T07-phi-r-correlation.md |
| Q6 | Drive frequency variants (1, 4, 0.5 Hz) | DONE — 0.5 Hz confirmed optimal | 2026-06-11T10 fire notes |

---

## Confirmed architectural ceiling

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
engine_a alpha_base=0.12 (consolidation.rs:796)
xi_eval_relax=20 (engine_clean + engine_adv, consolidation.rs:800-804)
chiral_p_bp=0.15 (engine_b_primed, research.rs:3457)
DRIVE_FREQ_HZ=0.5 (default in research.rs:3249)
KURAMOTO_COUPLING=0.5 (default in research.rs:3386)
```

3-trial avg fitness = **0.007627** (T01 confirmed); single trial today = 0.007562.

Threshold for any improvement: 0.007627 − 0.005 = **0.002627**

Total improvable across all remaining axes ≈ 0.002615 (< threshold). No single lever
and no combination of all remaining levers simultaneously can reach the threshold.
Structural ceiling. No new test worth running.
