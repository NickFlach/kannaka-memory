# irx relax_steps sweep + Q1 characterization

**Date:** 2026-06-12T22 UTC
**Branch:** kannaka-curiosity/2026-06-12T22-irx-relax16
**Code changes:** REVERTED — all variations worse than baseline; no code committed
**Status:** Q3 falsified; Q1 characterized (3-trial avg)

---

## Q1: 3-trial irx characterization (DONE)

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax` (no code changes)

| trial | fitness | transfer | xi | carrier_e | magic_R | query_gravity |
|-------|---------|----------|-----|-----------|---------|---------------|
| T1    | 0.150527 | 0.718315 | 0.4915 | 0.7140 | 0.6119 | 0.3641 |
| T2    | 0.163520 | 0.718315 | 0.4049 | 0.7140 | 0.6119 | 0.3641 |
| T3    | 0.127033 | 0.724649 | 0.6431 | 0.7140 | 0.6119 | 0.3641 |
| **mean** | **0.147027** | **0.720426** | **0.5132** | **0.7140** | **0.6119** | **0.3641** |

**vs system-prompt smoke test (fitness 0.191, xi 0.220):** master at 2e7c162 includes commit
141c0c0 (xi_and_flat scope), which was not in the smoke test state. This accounts for the
fitness improvement (0.191 → 0.147 avg) and the xi recovery (0.220 → 0.513 avg).

**xi is highly variable:** range 0.40–0.64 across 3 trials. carrier_e and magic_R are
deterministic. The xi variation is stochastic — likely driven by random seeding in the
xi_robustness evaluation (adversarial noise injection). This variance is important context
for any future xi-targeting intervention.

---

## Q3: relax_steps sweep (FALSIFIED)

Hypothesis (from system prompt): raising relax_steps from 8 to 16 would raise xi while
keeping carrier_e and magic_R high.

### Test A: alpha_base=0.20, relax_steps=16

`DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| metric | baseline (relax=8) | relax=16 | delta |
|--------|-------------------|----------|-------|
| fitness | 0.150527 | 0.244705 | +0.094 (MUCH WORSE) |
| xi | 0.4915 | 0.3507 | −0.141 (WORSE) |
| carrier_e | 0.7140 | 0.0000 | **−0.714 (CRASH)** |
| transfer | 0.718315 | 0.713969 | ~0 |

**carrier_e collapses to 0.** The flat-corpus carrier emergence test requires memories at
0.1 Hz to show 2 Hz structure after dreaming. With relax_steps=16 and alpha=0.20, total
convergence 0.80^16 ≈ 0.028 remaining — over-consolidation erases the frequency structure
needed for carrier emergence.

### Test B: alpha_base=0.10, relax_steps=16 (iso-convergence)

Same total convergence as A (0.90^16 ≈ 0.185 ≈ baseline 0.80^8 = 0.168), achieved more gradually.

| metric | baseline (α=0.20, relax=8) | α=0.10, relax=16 | delta |
|--------|---------------------------|-----------------|-------|
| fitness | 0.150527 | 0.193992 | +0.043 (WORSE) |
| xi | 0.4915 | 0.3040 | −0.187 (WORSE) |
| carrier_e | 0.7140 | 0.4966 | −0.217 (WORSE) |

Even at equivalent total convergence, spreading relaxation across 16 smaller steps with
alpha=0.10 is worse than 8 steps at alpha=0.20. More steps is not intrinsically better;
the original 8-step / α=0.20 combination appears to be a local optimum for this irx mode.

**Both variations reverted.**

---

## Mechanism analysis

The irx mode's carrier_emergence depends on maintaining frequency diversity in the memory
phase landscape after relaxation. The α=0.20 × 8 steps combination (0.8^8 ≈ 16.8%
remaining distance) strikes a balance between:
- Enough convergence to form coherent phase clusters (needed for xi and magic_R)
- Enough residual diversity to preserve carrier frequency structure (needed for carrier_e)

Increasing total convergence past this point (relax=16 with α=0.20) collapses the
frequency diversity. Distributing the same total pull across more finer steps (α=0.10,
relax=16) also fails, suggesting the convergence bottleneck is in the final-state phase
distribution, not in the convergence path.

---

## Status of system-prompt research questions

| Q | question | status |
|---|----------|--------|
| Q1 | 3-run irx characterization | **DONE this fire** — avg fitness 0.147, xi 0.51 ±0.12, carrier_e 0.714 |
| Q2 | K-sweep under fixed plumbing | OPEN — no trials yet in this container's master history |
| Q3 | irx + xi recovery (relax_steps 16/24) | **FALSIFIED this fire** — relax=16 crashes carrier_e |
| Q4 | R-xi correlation at stage_sync | OPEN |
| Q5 | Φ ↔ R relationship | OPEN |
| Q6 | Drive frequency variants | OPEN |

---

## Remaining open hypothesis worth testing next fire

The xi variance (±0.12 across trials) suggests the xi_robustness evaluation is
stochastic. Q2 (K-sweep) could reveal whether kuramoto_coupling affects the xi variance
as well as the mean — if higher K tightens the irx cluster distribution, xi might become
more reliably high. The operating point (magic_R=0.612 at default K) is stable, so K
variation is a clean parametric sweep.
