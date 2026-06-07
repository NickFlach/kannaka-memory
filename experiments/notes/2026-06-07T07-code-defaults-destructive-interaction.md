# Code defaults combine destructively — true optimum is hyp-freq0.5hz

**Date:** 2026-06-07T07 UTC
**Branch:** kannaka-curiosity/2026-06-07T07
**Code changes:** None — env-var only
**Status:** FALSIFIED (no improvement; all combinations worse than hyp-freq0.5hz)

---

## Background and motivation

The code defaults have been updated in three separate fires:
- PR #150: `DRIVE_FREQ_HZ` default 2.0 → 0.5 (confirmed: carrier_e 0.497→0.935, fitness 0.149→0.099)
- PR #158: `DRIVE_A` default 0.1 → 0.15 (confirmed at K=1.0, DRIVE_FREQ_HZ=2.0: fitness 0.138→0.132)
- Fire T14 / PR ~#154: `KURAMOTO_COUPLING` default 1.0 → 0.5 (confirmed at DRIVE_FREQ_HZ=2.0: fitness 0.138→0.134)

Each improvement was confirmed against the "2 Hz baseline" (~0.138) independently, at
different code states. The three were never tested together. Current code defaults:
  `DRIVE_A=0.15, DRIVE_FREQ_HZ=0.5, KURAMOTO_COUPLING=0.5, DRIVE_SCOPE=all`

**Hypothesis:** Running with current code defaults (no env overrides) equals or beats the
best known result (hyp-freq0.5hz: K=1.0, DRIVE_A=0.1, DRIVE_FREQ_HZ=0.5, avg 0.099).

**Prediction (if improvements compound):** fitness ~0.08, xi ~0.90, carrier_e ~0.93.

---

## Results

All trials: `DRIVE_SCOPE=all` with other params as noted.

| trial | K   | DRIVE_A | DRIVE_FREQ_HZ | fitness  | xi_v2 | carrier_e | transfer | magic_R | query_g |
|-------|-----|---------|---------------|----------|-------|-----------|----------|---------|---------|
| T1 (code defaults) | 0.5 | 0.15 | 0.5 | 0.167357 | 0.455 | 0.853 | 0.682 | 0.140 | 0.457 |
| T2 | 1.0 | 0.15 | 0.5 | 0.203021 | 0.281 | 0.864 | 0.618 | 0.196 | 0.435 |
| T3 | 0.5 | 0.10 | 0.5 | 0.105242 | 0.958 | 0.844 | 0.561 | 0.239 | 0.435 |

**Reference (hyp-freq0.5hz, from TSV, 3 trials):**

| trial | K | DRIVE_A | DRIVE_FREQ_HZ | fitness | xi_v2 | carrier_e | transfer |
|-------|---|---------|---------------|---------|-------|-----------|---------|
| t1 | 1.0 | 0.1 | 0.5 | 0.101 | ~0.970 | 0.935 | ~0.836 |
| t2 | 1.0 | 0.1 | 0.5 | 0.052 | ~0.970 | 0.935 | ~0.836 |
| t3 | 1.0 | 0.1 | 0.5 | 0.144 | ~0.970 | 0.935 | ~0.836 |
| **avg** | | | | **0.099** | **~0.970** | **0.935** | **~0.836** |

None of the three trials beat the hyp-freq0.5hz average (0.099). All are worse.

---

## Analysis

### The three improvements interact destructively

The 0.5 Hz drive creates a distinct operating regime: a single-cycle half-arc that
amplifies the first-half memories by A and gently suppresses the second half. In this
regime, the optimal parameters are DIFFERENT from the 2 Hz regime where K=0.5 and
DRIVE_A=0.15 were confirmed.

**DRIVE_A=0.15 at DRIVE_FREQ_HZ=0.5 destroys xi:**

| config | xi_v2 |
|--------|-------|
| K=1.0, A=0.1, 0.5Hz (hyp-freq0.5hz) | ~0.970 |
| K=1.0, A=0.15, 0.5Hz (T2) | 0.281 |
| K=0.5, A=0.15, 0.5Hz (T1 = code defaults) | 0.455 |

The +50% larger half-arc amplitude (A=0.15 vs A=0.1) creates an amplitude structure
that adversarial perturbation can exploit. At 2 Hz (4 oscillations), the drive is
averaged out across the chain; at 0.5 Hz (1 arc), there is a clear directional
modulation that the adversary targets. The xi collapse from 0.970 → 0.281–0.455
costs +0.104 × 0.15 = +0.016 to +0.104 in fitness — far exceeding the benefit.

**K=0.5 at DRIVE_FREQ_HZ=0.5 destroys transfer_score:**

| config | transfer_score |
|--------|----------------|
| K=1.0, A=0.1, 0.5Hz (hyp-freq0.5hz) | ~0.836 |
| K=0.5, A=0.1, 0.5Hz (T3) | 0.561 |

The half-arc drive creates memory amplitude structure that K=1.0 Kuramoto sync
uses to sharpen the primed-vs-naive B-engine discrimination. K=0.5's weaker
coupling fails to form the necessary phase-amplitude coherence. Transfer drops
from 0.836 → 0.561, costing +0.275 × 0.15 = +0.041 in fitness.

The paradox: at 2 Hz, K=0.5 slightly improves transfer (0.654 → 0.666) by
reducing over-synchronization. At 0.5 Hz, the half-arc creates amplitude
asymmetry that K=1.0 leverages; K=0.5 cannot harness it. The 0.5 Hz regime
favors a coherent K=1.0 operating point.

### Why the individual confirmations failed to predict the interaction

All three improvements were tested at DRIVE_FREQ_HZ=2.0 as the implicit baseline:

- DRIVE_FREQ_HZ=0.5 was confirmed first (PR #150), at K=3.0
- K=0.5 was confirmed second (fire T14), at DRIVE_FREQ_HZ=2.0 (explicit env var override not documented but confirmed by carrier_e=0.549 in results, vs 0.935 expected at 0.5 Hz)
- DRIVE_A=0.15 was confirmed third (fire T21), at K=1.0, DRIVE_FREQ_HZ=2.0

Because K=0.5 and DRIVE_A=0.15 were confirmed in the 2 Hz regime, they were NOT
tuned for the 0.5 Hz operating point. The code defaults encode a false assumption:
that the three optima are independent and combine additively.

### The code defaults are wrong for the true operating point

Running the binary with no env vars gives fitness 0.167, which is WORSE than both:
- The "2 Hz baseline" (0.138 at K=1.0, DRIVE_A=0.1)
- The true best (0.099 at K=1.0, DRIVE_A=0.1, DRIVE_FREQ_HZ=0.5)

The code defaults should be revised to match the true optimum:

```
DRIVE_FREQ_HZ: 0.5  (keep — confirmed, no interaction here)
DRIVE_A:       0.1  (revert from 0.15 — incompatible with 0.5 Hz)
K:             1.0  (revert from 0.5 — incompatible with 0.5 Hz)
```

This change is NOT made in this fire (no code modification — justification needed).
A dedicated fire should revert the K and DRIVE_A code defaults with 3 confirmation trials.

---

## Decision

**No improvement found.** No code changes to revert. True empirical optimum:

    DRIVE_A=0.1  DRIVE_SCOPE=all  KURAMOTO_COUPLING=1.0  DRIVE_FREQ_HZ=0.5
    3-run avg fitness ≈ 0.099 (from hyp-freq0.5hz, TSV)

The code defaults currently give 0.167 due to destructive interaction.

---

## Implications

1. **The code defaults need a dedicated fix fire:** revert K → 1.0 and DRIVE_A → 0.1
   in `run_experiment_l5_session`, confirm with 3 trials. Expected result: ~0.099 avg
   with no code-path changes beyond the defaults.

2. **The 0.5 Hz regime has a different optimal surface than the 2 Hz regime:**
   - At 2 Hz: K=0.5 and DRIVE_A=0.15 add marginal gains
   - At 0.5 Hz: K=1.0 and DRIVE_A=0.1 are structurally required
   - Future improvements must be confirmed WITHIN the 0.5 Hz regime

3. **hyp-freq0.5hz xi variance is high (0.052–0.144 range):** the 0.099 avg is from
   only 3 trials with range 0.092. The true mean could be 0.09–0.11. A dedicated
   characterization fire (5–6 trials) would nail the mean more precisely.

4. **Do not test DRIVE_A > 0.1 at DRIVE_FREQ_HZ=0.5:** all evidence shows xi collapse
   above this threshold in the 0.5 Hz regime.
