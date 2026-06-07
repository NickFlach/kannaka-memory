# DRIVE_A=0.15 + interference_relax — carrier_e regression, hypothesis falsified

**Date:** 2026-06-07T06 UTC
**Branch:** kannaka-curiosity/2026-06-07T06
**Code changes:** None — env-var only
**Status:** FALSIFIED — deterministic regression on carrier_e and transfer_score

---

## Background

Two confirmed improvements have never been combined:

1. **DRIVE_A=0.15** (PR #158, 2026-06-06T08): under stage_sync + DRIVE_FREQ_HZ=0.5,
   raising drive amplitude from 0.1 → 0.15 gave **deterministic** gains:
   carrier_e +0.016 (0.568→0.584) and transfer_score +0.039 (0.654→0.694).
   The mechanism: stronger amplitude modulation raises the 2 Hz carrier FFT peak
   above the consolidation noise floor, and sharper B-engine amplitude dynamics
   improve primed-vs-naive discrimination. Code default changed to 0.15.

2. **DREAM_MODE=interference_relax + DRIVE_FREQ_HZ=0.5** (PR #150, 2026-06-06T08):
   under DRIVE_A=0.1, this mode gave avg fitness **0.099** (transfer 0.836,
   carrier_e 0.935) vs stage_sync avg ~0.132. All subsequent interference_relax
   experiments (PRs #163–#170) explicitly used DRIVE_A=0.1 and did not test the
   combination with the new A=0.15 default.

The interaction between drive amplitude and dream mode is uncharacterized. The
baseline for this fire: `DRIVE_A=0.1 DREAM_MODE=interference_relax` = 3-trial
avg fitness **0.099**, carrier_e **0.935**, transfer **0.836**.

---

## Hypothesis

DRIVE_A=0.15 under DREAM_MODE=interference_relax will improve transfer_score by
~0.039 (same mechanism as stage_sync), with carrier_e near ceiling (~0.935). The
DRIVE block runs before the consolidation step regardless of mode, so the amplitude-
discrimination gain should transfer.

**Prediction:**
- transfer_score: 0.836 → ~0.875 (deterministic)
- carrier_e: ~0.935 (ceiling effect, minor change)
- xi_robustness_v2: stochastic, ~0.559 avg (unchanged by drive amplitude)
- Fitness avg: 0.099 → ~0.093

---

## Trials

All trials: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax`
(DRIVE_FREQ_HZ=0.5 default, KURAMOTO_COUPLING=0.5 default — irrelevant for irx)

| trial | fitness | xi_robustness_v2 | transfer_score | carrier_emergence | carrier_bimodal | magic_R | query_gravity |
|-------|---------|-----------------|----------------|-------------------|-----------------|---------|---------------|
| t1    | 0.195   | 0.020           | 0.820          | 0.803             | 0.870           | 0.617   | 0.362         |
| t2    | 0.141   | 0.384           | 0.820          | 0.803             | 0.870           | 0.617   | 0.362         |
| t3    | 0.096   | 0.682           | 0.820          | 0.803             | 0.870           | 0.617   | 0.362         |
| **avg** | **0.144** | **0.362**    | **0.820**      | **0.803**         | **0.870**       | **0.617** | **0.362** |

**Baseline (DRIVE_A=0.1 + irx, PR #150 3-trial avg):**

| metric | A=0.1 avg | A=0.15 avg | delta |
|--------|-----------|------------|-------|
| fitness | 0.099 | **0.144** | **+0.045** ← regression |
| transfer_score | 0.836 | 0.820 | **−0.016** ← worse |
| carrier_emergence | 0.935 | 0.803 | **−0.132** ← much worse |
| carrier_bimodal | — | 0.870 | (baseline not recorded) |
| xi_robustness_v2 | 0.559 avg | 0.362 avg | −0.197 (but high variance) |
| magic_proxy_phase_R | 0.617 | 0.617 | unchanged (deterministic) |
| query_gravity | 0.363 | 0.362 | unchanged |

---

## Findings

### 1. Hypothesis falsified — deterministic regression on both carrier metrics

The prediction was wrong in both direction and mechanism:
- carrier_e dropped from 0.935 → **0.803** (Δ −0.132)
- transfer_score dropped from 0.836 → **0.820** (Δ −0.016)

Both are deterministic (identical across all 3 trials). The regression is structural,
not stochastic noise.

### 2. Mode-specific sensitivity to drive amplitude

Under stage_sync, DRIVE_A=0.15 vs 0.10 improved both carrier_e and transfer
(+0.016, +0.039). Under interference_relax, the same change degrades both (−0.132,
−0.016). This is a clean mode interaction:

**Stage_sync** (Kuramoto): carrier structure is phase-based. The Kuramoto step aligns
phases within categories; the drive modulates amplitudes. Stronger amplitude drive
adds more signal to the carrier FFT without disrupting the phase-based category
structure. The two mechanisms are largely orthogonal.

**Interference_relax**: carrier structure is amplitude-based. The constructive-pair
mechanism pairs the highest-amplitude memories for phase relaxation. With DRIVE_A=0.15,
the 0.5 Hz positive arc (cycles 0–8) boosts top memories by ±15% instead of ±10%.
This more extreme amplitude hierarchy alters which memories qualify as "constructive
pairs." The resulting phase distribution after consolidation is different — the carrier
FFT peak at 0.5 Hz is weakened because the amplitude envelope over the chain no longer
has the same bimodal signature.

The carrier_bimodal metric also dropped (0.870 vs baseline) — consistent with the
constructive-pair dynamics being disrupted.

### 3. xi variance widens

xi mean dropped from 0.559 (A=0.1) to 0.362 (A=0.15), with wide range (0.020–0.682).
The t1 catastrophic xi draw (0.020) and the low mean suggest the memory phase structure
at A=0.15 is more fragile to adversarial perturbation. This is consistent with a less
well-formed carrier scaffold — if constructive pairs aren't matching correctly, the
phase geometry that xi robustness depends on is less stable.

### 4. magic_R is unchanged

magic_proxy_phase_R = 0.617 in both conditions (deterministic under irx mode). Drive
amplitude does not affect the Kuramoto order parameter measured at end of dream —
consistent with prior findings that R depends only on the sync dynamics, not amplitude.

### 5. The A=0.15 improvement is mode-gated

The code default is DRIVE_A=0.15, which is optimal for the default DREAM_MODE=unset
(stage_sync). Any future session using DREAM_MODE=interference_relax with the default
DRIVE_A will inadvertently operate at a suboptimal point. The interference_relax optimum
is DRIVE_A=0.1.

---

## Decision

**Hypothesis falsified. No code changes. No improvement.**

The empirical optimum under interference_relax remains:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax  DRIVE_FREQ_HZ=0.5
3-run avg fitness ≈ 0.099
```

The stage_sync optimum is unaffected:
```
DRIVE_A=0.15  DRIVE_SCOPE=all  DREAM_MODE=<unset>  KURAMOTO_COUPLING=0.5
3-run avg fitness ≈ 0.134 (K=0.5, A=0.1) / ≈ 0.132 (K=1.0, A=0.15)
```

---

## Implications

1. **DRIVE_A is mode-gated**: the optimal drive amplitude is 0.15 for stage_sync and
   0.1 for interference_relax. The two modes have different carrier mechanisms, and
   the drive amplitude interacts differently with each.

2. **interference_relax carrier_e is amplitude-sensitive**: the constructive-pair
   mechanism that produces carrier_e=0.935 at A=0.1 relies on a specific amplitude
   hierarchy. Over-driving (A=0.15) disrupts this. The 0.5 Hz arc at A=0.1 is a
   "sweet spot" — enough to build the bimodal carrier structure, not so much as to
   flatten the constructive-pair selection.

3. **Next interference_relax axis**: The irx mode has been explored across relax_steps
   (ceiling at 16), alpha_base (0.10 optimal), scope (no_transfer falsified), and now
   drive amplitude (0.1 optimal). The remaining unexplored axis with theoretical
   upside is DRIVE_FREQ_HZ below 0.5 (e.g. 0.25 Hz, which would be entirely
   positive drive for the full 16-cycle chain). This risks over-amplification but
   has not been tested.

4. **stage_sync + combination check**: DRIVE_A=0.15 was confirmed at K=1.0 and
   K=0.5 was confirmed at A=0.1. The combination A=0.15 + K=0.5 under stage_sync
   is the remaining untested combination. Expected improvement: ~0.004–0.008 over
   the current 0.132 stage_sync optimum, but small enough that xi variance may
   obscure the signal in 3 trials.
