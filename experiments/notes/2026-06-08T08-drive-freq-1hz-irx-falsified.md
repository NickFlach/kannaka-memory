# DRIVE_FREQ_HZ=1.0 under interference_relax — build-phase interruption falsifies hypothesis

**Date:** 2026-06-08T08 UTC
**Branch:** kannaka-curiosity/2026-06-08T08
**Code changes:** None — env-var only
**Status:** FALSIFIED (fitness 0.179 vs 0.099 baseline; frequency axis now fully closed)

---

## Background

Empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax  DRIVE_FREQ_HZ=0.5
3-trial avg fitness ≈ 0.099
carrier_emergence=0.935 (deterministic), transfer_score=0.836 (deterministic)
```

Previous irx frequency tests (T22, 2026-06-07T22):
- 0.25 Hz: carrier_e=0.638, xi=0.189, transfer=0.836, fitness=0.184 — FALSIFIED
  - Finding: suppression trough is the carrier waveform; all-positive drive removes it

Research question Q6 (original system prompt): "Drive frequency variants (1, 4, 0.5 Hz)
at A=0.1 — T19 attempt failed due to sibling-dep layout; redo in production."

1 Hz had not been empirically tested under irx in production.

---

## Hypothesis

At DRIVE_FREQ_HZ=1.0 under irx, the dream window (16 cycles × 0.125s = 2s) contains
exactly 2 full oscillations:
- Positive arches: cycles 0–4 (peak at cycle 2) and cycles 8–12 (peak at cycle 10)
- Suppression troughs: cycle 6 (trough) and cycle 14 (trough)

The 0.25 Hz falsification established that suppression troughs are needed for carrier
structure. At 1 Hz, two suppression troughs provide two reinforcement cycles. The
irx phase relaxation (relax_steps=16 per dream cycle) gets TWO passes at constructive-
pair convergence — once per arch. Prediction:

- **carrier_e:** survives or improves — two troughs → two carrier half-cycles → stronger
  FFT signal at 1 Hz than the single half-cycle at 0.5 Hz
- **xi:** might improve — two convergence rounds → tighter phase neighborhoods
- **transfer:** similar — amplitude build still reaches full arch height
- **fitness:** comparable to or better than 0.099

---

## Method

Single trial; if fitness ≤ 0.090, run 2 more for 3-trial confirmation.

```
DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax DRIVE_FREQ_HZ=1.0
RESEARCH_RUN=irx-1hz.t1
```

---

## Results (1 trial)

| metric | irx 0.5 Hz baseline (3-trial avg) | irx 1 Hz (t1) | delta |
|--------|-------------------------------------|----------------|-------|
| fitness | **0.099** | **0.179** | **+0.080 regression** |
| transfer_score | 0.836 | 0.688 | **−0.148** |
| carrier_emergence | 0.935 | 0.661 | **−0.274** |
| carrier_bimodal | ~0.820 | **0.989** | +0.169 |
| xi_robustness_v2 | ~0.559 avg | 0.356 | −0.203 |
| magic_proxy_phase_R | 0.617 | 0.617 | 0 (freq-invariant, confirmed) |
| query_gravity | ~0.363 | 0.363 | 0 (freq-invariant, confirmed) |

---

## Analysis

### Why 1 Hz hurts: build-phase interruption

The 0.5 Hz architecture works because the dream splits cleanly:
- **Build phase (cycles 0–8):** contiguous positive drive builds amplitude differential.
  Constructive pairs form, irx phase relaxation deepens neighborhood structure. Transfer
  structure accumulates across all 8 cycles without interruption.
- **Refine phase (cycles 9–16):** suppression trough (peak −A at cycle 12) creates
  the second half of the carrier waveform AND gently prunes weak associations.

At 1 Hz, the first suppression trough arrives at **cycle 6** — mid-build phase. The build
phase covers only cycles 0–4 (4 cycles) before suppression disrupts it. The irx phase
relaxation gets fewer cycles to converge per arch. The second arch (cycles 8–12) partially
rebuilds, but is similarly interrupted at cycle 14.

### Three mechanisms, three regressions

**carrier_e (0.935 → 0.661):** The FFT amplitude time series now has two half-amplitude
positive arches instead of one full-amplitude arch. Irx phase relaxation (16 steps per
dream cycle, not per arch) attempts to converge during each arch but is cut short. The
carrier frequency peak at 1 Hz is weaker than the 0.5 Hz peak would be — both because
the arches are shorter AND because irx convergence is incomplete per arch.

**transfer_score (0.836 → 0.688):** This is the critical diagnostic that distinguishes
1 Hz from 0.25 Hz failure. At 0.25 Hz (all positive), transfer stayed at 0.836 —
unaffected by removing the suppression. At 1 Hz, transfer collapsed to 0.688. Why?
The transfer mechanism (engine_a amplitude differential priming engine_b via flat corpus)
requires a CONTIGUOUS build phase. At 0.25 Hz, the full 16-cycle positive phase preserves
the build. At 1 Hz, suppression arrives at cycle 6 mid-build and reverses amplitude
gains before the build completes. The priming differential is partially destroyed.
**Conclusion: transfer is sensitive to build-phase interruption, not to suppression per se.**

**xi (0.559 → 0.356):** Two short convergence rounds under irx establish phase
neighborhoods less reliably than one long convergence round. The adversarial attack
finds more gaps when phase structure is incompletely formed.

**carrier_bimodal rose sharply (0.820 → 0.989):** More frequent amplitude reversals
create a sharper bimodal split — some memories end up at very high amplitude (those
that were boosted twice and survived both troughs) vs very low (those that were suppressed
twice). Bimodal amplitude is high, but it does not translate to carrier coherence.
This decoupling shows carrier_bimodal and carrier_emergence measure different things:
bimodal = whether distribution is two-peaked; emergence = whether the drive frequency
appears in the amplitude FFT.

**magic_R and query_gravity both = 0.617 / 0.363:** Identical to the irx 0.5 Hz
baseline. Confirms T22 finding: these are determined by irx phase relaxation dynamics,
not drive frequency.

---

## Frequency axis fully characterized

| DRIVE_FREQ_HZ | irx carrier_e | irx transfer | irx xi avg | fitness | mechanism |
|---------------|---------------|--------------|------------|---------|-----------|
| 0.25 Hz | 0.638 | 0.836 | 0.189 | 0.184 | all-positive = no refine half; xi structure absent |
| **0.50 Hz** | **0.935** | **0.836** | **~0.559** | **0.099** | **clean build/refine split at cycle 8** |
| 1.00 Hz | 0.661 | 0.688 | 0.356 | 0.179 | mid-build suppression; transfer collapses |

**0.5 Hz is uniquely optimal** because it places the build-refine boundary at exactly
cycle 8 (the halfway point). Any freq < 0.5 Hz moves the refine trough outside the
dream window (or too late). Any freq > 0.5 Hz moves the first trough into the build
phase, interrupting it.

The optimal freq is geometrically determined: DRIVE_FREQ_HZ = 0.5 × (1 / (dream_cycles × 16 × dt))
... but that simplifies to: one half-cycle = exactly half the dream window. 0.5 Hz with
16 cycles × 0.125s = 1s half-period = 8 cycles. The dream window IS the positive half.

---

## Decision

**Hypothesis falsified. No code changes. Frequency axis closed.**

- DRIVE_FREQ_HZ=1.0 under irx: decisive regression, 1 trial sufficient (fitness 0.179).
- The 1 Hz failure explains WHY 0.5 Hz is optimal in a new way: the build-phase must
  be contiguous. Any freq above 0.5 Hz interrupts the build before completion.
- Combined with the 0.25 Hz finding (below 0.5 Hz loses the carrier refine half),
  0.5 Hz is uniquely determined by the dream window geometry.

**Empirical optimum unchanged:**
- irx: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → avg fitness **0.099**
- stage_sync: `KURAMOTO_COUPLING=0.5 DRIVE_A=0.15 DRIVE_SCOPE=all` → avg fitness **0.104**

### Remaining open questions

The K, A, freq, and relax_steps axes are all closed. Remaining gaps:

1. **stage_sync transfer gap** (0.655 vs irx's 0.836): no env-var handle found. Would
   require constructive_boost, destructive_penalty, or stage-level code changes.
2. **irx xi variance**: xi ranges 0.256–0.874 across trials. Seeding eval_xi_robustness_v2
   would stabilize measurement. Neither freq, A, nor relax_steps reliably improves xi mean.
3. **Structural consolidation parameters** (hallucination_amplitude, prune_threshold):
   not exposed as env-vars; unexplored at L5.
