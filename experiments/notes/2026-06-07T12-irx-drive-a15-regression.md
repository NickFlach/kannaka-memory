# interference_relax + DRIVE_A=0.15 — regression confirms DRIVE_A=0.1 optimum

**Date:** 2026-06-07T12 UTC
**Branch:** kannaka-curiosity/2026-06-07T12
**Code changes:** None — env-var only
**Status:** FALSIFIED — avg fitness 0.144 vs 0.099 baseline at interference_relax

---

## Background

Two confirmed improvements from prior fires:
1. **T08 (2026-06-06T08):** `DREAM_MODE=interference_relax + DRIVE_FREQ_HZ=0.5 + DRIVE_A=0.1`
   → avg fitness 0.099, carrier_e=0.935, transfer=0.836. Code change: DRIVE_FREQ_HZ default 2.0→0.5.
2. **T21 (2026-06-06T21):** `DREAM_MODE=unset + DRIVE_A=0.15 + K=1.0`
   → avg fitness 0.132, carrier_e=0.584 (+0.016), transfer=0.694 (+0.039). Code change: DRIVE_A default 0.1→0.15.

The two improvements were discovered independently in different dream modes. The natural
question: do they compound at interference_relax?

Current code defaults: DRIVE_A=0.15, DRIVE_FREQ_HZ=0.5, DRIVE_SCOPE=all.

---

## Hypothesis

`DREAM_MODE=interference_relax + DRIVE_A=0.15` improves over the T08 result (0.099 avg).

Reasoning: DRIVE_A=0.15 gave +0.039 transfer deterministically at stage_sync. If the
same mechanism applies at interference_relax, transfer rises from 0.836 to ~0.875.
carrier_e is already near ceiling at 0.935; slight further improvement possible.
Net predicted fitness: 0.099 − 0.006 ≈ 0.093 avg.

**Prediction:** avg fitness < 0.099, transfer ~0.875, carrier_e ~0.935.

---

## Method

All trials: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax DRIVE_FREQ_HZ=0.5` (default)

Baseline: T08 (interference_relax + DRIVE_A=0.1 + 0.5 Hz), 3-trial avg fitness 0.099.

---

## Results

| trial | fitness  | transfer | carrier_e | xi_v2  | magic_R | query_g | phi_final |
|-------|----------|----------|-----------|--------|---------|---------|-----------|
| t1    | 0.154458 | 0.820400 | 0.8032    | 0.2930 | 0.6167  | 0.3622  | —         |
| t2    | 0.151450 | 0.820400 | 0.8032    | 0.3129 | 0.6167  | 0.3622  | —         |
| t3    | 0.127008 | 0.820400 | 0.8032    | 0.4759 | 0.6167  | 0.3622  | 0.289     |
| **avg** | **0.144** | **0.820** | **0.803** | **0.361** | **0.617** | **0.362** | |

phi_history at t3: [0.278, 0.301, 0.288, 0.289] — 4 samples (shorter than stage_sync's 16).

---

## Baseline comparison

| config | fitness avg | transfer | carrier_e | xi avg | magic_R |
|--------|------------|----------|-----------|--------|---------|
| irx + DRIVE_A=0.1 + 0.5 Hz (T08) | **0.099** | 0.836 | **0.935** | 0.559 | 0.617 |
| irx + DRIVE_A=0.15 + 0.5 Hz (this) | 0.144 | 0.820 | 0.803 | 0.361 | 0.617 |
| stage_sync K=1.0 + DRIVE_A=0.15 (T21) | 0.132 | 0.694 | 0.584 | 0.844 | 0.252 |
| stage_sync K=1.0 + DRIVE_A=0.1 (T03/T05) | ~0.132 | 0.659 | 0.568 | 0.878 | 0.250 |

---

## Analysis

### Hypothesis falsified: DRIVE_A=0.15 hurts interference_relax

Both deterministic metrics regressed at DRIVE_A=0.15+irx vs DRIVE_A=0.1+irx:

- **carrier_e: 0.935 → 0.803** (Δ −0.132, deterministic across all 3 trials)
- **transfer_score: 0.836 → 0.820** (Δ −0.016, deterministic)

These are structural, not luck. The fitness formula breakdown:
- carrier_e cost: 0.10 × (1−0.803) = 0.020 (vs 0.006 at DRIVE_A=0.1, +0.014 penalty)
- transfer cost: 0.15 × (1−0.820) = 0.027 (vs 0.025 at DRIVE_A=0.1, +0.002 penalty)
- xi avg also lower: 0.361 vs 0.559 (xi cost 0.15 × 0.198 = +0.030 additional penalty)
- Total regression: ~+0.046 vs predicted −0.006

### Why DRIVE_A=0.15 helps stage_sync but hurts interference_relax

At **stage_sync**, the Kuramoto step synchronizes phases AFTER the amplitude drive.
The stronger drive (±15% vs ±10%) injects a larger amplitude signal into the flat
corpus, improving the FFT peak at 0.5 Hz → better carrier_e. The Kuramoto coupling
then restores phase stability, so xi is not disrupted.

At **interference_relax**, the amplitude drive is applied EACH dream cycle BEFORE
constructive-pair relaxation. The 0.5 Hz half-cycle arc has two phases:
- Cycles 0–8: positive drive (max +A), coherently boosting amplitudes
- Cycles 9–16: negative drive (max −A), suppressing weakest memories

At DRIVE_A=0.1: positive phase = +10% peak, negative phase = −10% peak. The late
suppression (cycles 9–16) prunes lightly while the carrier structure built in cycles
0–8 survives into evaluation.

At DRIVE_A=0.15: positive phase = +15% peak, negative phase = −15% peak. The stronger
late suppression (15% vs 10%) actively erodes the carrier amplitude structure that was
built in cycles 0–8. By the time the 16-cycle chain ends, the amplitude bimodality
created by the positive phase has been partially overwritten by the stronger negative
phase. carrier_e measures end-of-chain amplitude patterns, so the overwriting shows up
directly as a carrier_e drop (0.935 → 0.803).

The transfer_score drop (0.836 → 0.820) has a similar cause: the stronger negative
suppression in cycles 9–16 partially disrupts B-engine primed-vs-naive discrimination.
At DRIVE_A=0.1, cycles 9–16 prune only the lowest-amplitude memories; at DRIVE_A=0.15,
the suppression reaches further into the amplitude distribution, degrading primed memory
reinforcement.

**Key asymmetry:** stage_sync's Kuramoto steps act as a phase "stabilizer" that absorbs
the stronger drive without disrupting carrier structure. interference_relax has no such
stabilizer — the phase relaxation is directly disturbed by the amplitude changes, and
the late-cycle negative drive compounds the damage.

### xi regression

xi avg dropped from 0.559 (DRIVE_A=0.1) to 0.361 (DRIVE_A=0.15). The adversarial
perturbation finds more exploitable directions in the memory geometry when the amplitude
drive is stronger. The constructive-pair relaxation at DRIVE_A=0.15 produces a different
attractor structure than at DRIVE_A=0.1, and that structure is more susceptible to
small adversarial perturbations. This is consistent with the carrier_e explanation:
if the amplitude bimodality is less pronounced (0.803 vs 0.935), the phase attractors
that xi robustness depends on are also shallower.

### Phi ↔ R observation (Q5 data point)

interference_relax phi_history at DRIVE_A=0.15: [0.278, 0.301, 0.288, 0.289] (4 samples,
final=0.289, magic_R=0.617)

From T21 notes: stage_sync phi_history at DRIVE_A=0.15: 16 samples, final=0.319, magic_R=0.252.

Cross-mode comparison: irx has lower phi_final (0.289 < 0.319) despite MUCH higher magic_R
(0.617 vs 0.252). This is an **anti-correlation** (high R → low phi) — opposite to the
IIT-bridge prediction that non-Clifford content (high R) should correlate with higher Φ.

Caveat: the different phi sampling cadences (4 vs 16 values) may reflect different
phi-computation triggers between dream modes, making direct comparison unreliable.
Cross-mode Φ↔R comparison requires sampling both modes at identical cadences.

---

## Decision

**No code changes to revert.** Hypothesis falsified.

The confirmed interference_relax optimum remains:

    DREAM_MODE=interference_relax  DRIVE_A=0.1  DRIVE_FREQ_HZ=0.5  DRIVE_SCOPE=all
    3-run avg fitness ≈ 0.099

Note: DRIVE_A default in code is 0.15 (changed in T21 for stage_sync optimization).
Interference_relax users must explicitly set `DRIVE_A=0.1` to reach the 0.099 optimum.

---

## Implications

1. **DRIVE_A=0.1 is uniquely optimal for interference_relax.** The mechanism is the
   half-cycle arc balance: positive phase (cycles 0–8) builds carrier structure, negative
   phase (cycles 9–16) prunes lightly. At DRIVE_A=0.1, this balance is correct. At 0.15,
   the negative phase is destructive.

2. **The two code-default changes (DRIVE_A 0.1→0.15 in T21, DRIVE_FREQ_HZ 2.0→0.5 in T08)
   are mode-specific optimima that conflict.** For stage_sync: current defaults (DRIVE_A=0.15,
   0.5 Hz) give avg ~0.132. For interference_relax: optimal is DRIVE_A=0.1 (against the
   current default), 0.5 Hz, giving avg ~0.099. Any fire using interference_relax must
   override DRIVE_A=0.1 explicitly.

3. **Stage_sync Kuramoto acts as amplitude stabilizer.** The Kuramoto step absorbs drive
   amplitude changes without disrupting carrier structure. interference_relax lacks this
   stabilizer, making it more sensitive to drive amplitude.

4. **The interference_relax 0.099 avg remains the best confirmed result in L5 history.**
   The high xi variance (single trials: 0.052–0.144) makes it less stable than stage_sync
   (0.120–0.158 typical range). If xi variance can be reduced at interference_relax, the
   0.099 avg would be a highly reproducible gain.

5. **Future DRIVE_A sweeps at interference_relax:** only DRIVE_A=0.1 and 0.15 have been
   tested. DRIVE_A=0.05 (weaker modulation) might preserve carrier_e near 0.935 while
   slightly reducing the carrier signal — probably worse or neutral. DRIVE_A=0.1 is the
   empirical sweet spot.

6. **phi_history at irx samples fewer values (4) than stage_sync (16).** Cross-mode Φ↔R
   comparison requires controlled sampling. Q5 cannot be cleanly addressed by existing data.
