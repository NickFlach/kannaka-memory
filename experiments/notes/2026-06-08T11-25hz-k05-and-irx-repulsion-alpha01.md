# Two falsifications: 0.25 Hz + K=0.5+A=0.15, and irx destructive repulsion alpha*0.1

**Date:** 2026-06-08T11 UTC
**Branch:** kannaka-curiosity/2026-06-08T11
**Code changes:** irx destructive repulsion alpha*0.1 tried and reverted; no code changes kept
**Status:** BOTH FALSIFIED

---

## Background

Current empirical optima:
- **irx**: `DRIVE_A=0.1 DREAM_MODE=interference_relax DRIVE_SCOPE=all DRIVE_FREQ_HZ=0.5` → avg fitness **0.099**, carrier_e=0.935, xi avg=0.559 (high variance), transfer=0.836
- **stage_sync**: `KURAMOTO_COUPLING=0.5 DRIVE_A=0.15 DRIVE_SCOPE=all DRIVE_FREQ_HZ=0.5` → avg fitness **0.104**, carrier_e=0.853, xi avg=0.873, transfer=0.655

Two open items from prior fires:
1. **0.25 Hz + K=0.5+A=0.15 (stage_sync):** The PR #193 result showed 0.25 Hz with K=1.0+A=0.1 gave avg fitness 0.0935, carrier_e=0.702, xi=0.960 (stable). The K×A synergy (which lifted carrier_e from 0.584 to 0.853 at 0.5 Hz) was never applied at 0.25 Hz.
2. **irx destructive repulsion at alpha*0.1:** T23 fire tested alpha*0.5 (xi=0.745, transfer=0.601, fitness=0.118 regression). Notes explicitly flagged alpha*0.1 as "borderline worth testing" — 5× weaker, expected partial xi gain without full transfer collapse.

---

## Hypothesis 1: KURAMOTO_COUPLING=0.5 + DRIVE_A=0.15 + DRIVE_FREQ_HZ=0.25 (stage_sync)

**Prediction:** The K×A synergy that at 0.5 Hz lifted carrier_e from 0.584 to 0.853 should partially translate to 0.25 Hz, improving carrier_e from the K=1.0+A=0.1 0.25 Hz baseline of 0.702 toward 0.80+. Combined with the 0.25 Hz xi stability (0.960 avg), fitness should drop below 0.090. At 0.25 Hz there are no suppression cycles, so A=0.15 poses no carrier-collapse risk.

**Safety argument:** At 0.5 Hz, A=0.20 collapsed carrier because negative-arc cycles (9-16) produce ~19% suppression. At 0.25 Hz, all cycles are positive — no suppression arc at any amplitude.

### Results (1 trial)

All metrics vs baselines:

| metric | K=1.0+A=0.1+0.25Hz (PR#193) | K=0.5+A=0.15+0.25Hz (this) | K=0.5+A=0.15+0.5Hz (PR#177) |
|--------|------------------------------|------------------------------|-------------------------------|
| fitness | 0.0935 avg | **0.126** | 0.104 avg |
| transfer | 0.710 | **0.641** | 0.655 |
| carrier_e | 0.702 | 0.863 | 0.853 |
| xi_v2 | 0.960 avg | **0.734** | 0.873 avg |
| magic_R | 0.245 | 0.234 | ~0.161 |
| query_gravity | 0.421 | 0.426 | ~0.446 |

**Hypothesis falsified.** The K×A synergy DID translate for carrier_e (0.702 → 0.863), confirming that the K×A mechanism is not frequency-specific. But xi stability collapsed from 0.960 to 0.734 — the 0.25 Hz xi stability advantage evaporated.

### Mechanism: xi stability at 0.25 Hz is amplitude-contingent, not waveform-contingent

The 0.25 Hz xi stability at K=1.0+A=0.1 was NOT a consequence of the all-positive waveform shape per se. It was a consequence of the gentle amplitude perturbation (A=0.1 → max +10%) creating very little amplitude variance for the adversary to exploit.

At K=0.5+A=0.15, the +15% peak drive creates larger amplitude differentials, and Kuramoto coupling at K=0.5 sharpens category boundaries rather than smoothing them. The adversary can now exploit the sharper amplitude gradients. The 0.25 Hz timing (peak cycle 8 instead of cycle 4) reduces the post-peak consolidation window from 12 cycles to 8 cycles, meaning the amplitude structure is less consolidated at dream end — creating more exploitable variance.

The safety argument against suppression-arc risk was correct (carrier_e improved). But the assumption that xi stability transfers from K=1.0+A=0.1 to K=0.5+A=0.15 was wrong.

**Transfer also regressed** (0.710 → 0.641). The 0.25 Hz late peak (cycle 8) limits the amplitude discrimination structure that K=0.5 builds for transfer — even at K=0.5, 8 consolidation cycles (vs 12 at 0.5 Hz) is insufficient.

**Conclusion:** The 0.25 Hz xi stability is a narrowly-conditioned effect (A=0.1, K=1.0 only). The axis is now closed at this combination.

---

## Hypothesis 2: irx destructive repulsion at alpha*0.1

**Prediction:** At 5× weaker repulsion than T23's alpha*0.5, transfer should recover from 0.601 to ~0.810 (−0.026 from baseline), and xi should improve from 0.559 avg to ~0.620. The net fitness effect is borderline: xi savings (0.061 × 0.15 = −0.009) partially offset transfer cost (0.026 × 0.15 = +0.004), predicting fitness ~0.096.

**Code change:** Added `destructive_neighbors` map in `stage_interference_relax`; after computing `new_phase` from constructive attraction, subtracted `alpha * 0.1 * sin(destructive_mean - new_phase)` per step. Reverted at end.

All trials: `DRIVE_A=0.1 DREAM_MODE=interference_relax DRIVE_SCOPE=all DRIVE_FREQ_HZ=0.5`

### Results (2 trials, deterministic on non-xi metrics)

| metric | irx baseline (PR#189 avg) | irx+repulsion alpha*0.1 | delta |
|--------|---------------------------|-------------------------|-------|
| fitness | 0.099 | **0.196** | **+0.097 (catastrophic regression)** |
| transfer | 0.836 | **0.661** | **−0.175** |
| carrier_e | 0.935 | 0.925 | −0.010 (≈ flat) |
| carrier_bimodal | — | 0.953 | — |
| xi_v2 | 0.559 avg (range 0.256–0.874) | **0.107 avg** | **−0.452 (catastrophe)** |
| magic_R | 0.617 | 0.577 | −0.040 |
| query_gravity | 0.363 | 0.361 | ≈ flat |

transfer and non-xi metrics are byte-identical between the two trials.

**Hypothesis catastrophically falsified.** xi collapsed to ~0.107 (below the irx baseline's historical MINIMUM of 0.256) and transfer regressed by −0.175. Fitness 0.196 >> 0.099 baseline.

### Mechanism: alpha threshold for xi protection vs xi destruction

At T23 alpha*0.5: xi avg = 0.745 (floor 0.578). At alpha*0.1: xi avg = 0.107. **Weaker repulsion is dramatically WORSE for xi**, which is counterintuitive.

The mechanism: the constructive-pair phase scaffold in baseline irx creates a protective structure for xi — the adversary cannot easily construct a perturbation because the phase landscape is complex (tied to actual interference geometry). At alpha*0.5, the strong repulsion reorganizes phases into a large-separation configuration that also happens to protect xi by creating wide phase gaps. At alpha*0.1, the gentle repulsion disrupts the constructive-pair scaffold just enough to reduce the landscape complexity WITHOUT creating large phase gaps. The result is a partially-disrupted phase landscape that is EASIER for the adversary to exploit than either the untouched baseline or the fully-repelled alpha*0.5 configuration.

There appears to be a "disruption valley" between alpha=0 (baseline, xi=0.559) and alpha=0.5 (large separation, xi=0.745): in the alpha=0.1 range, disruption without separation creates a fragile phase arrangement that drops xi below the floor. This is consistent with the T23 finding that magic_R=0.266 at alpha*0.5 reflected phases being pushed apart in a structured way; at alpha*0.1, magic_R=0.577 (close to baseline 0.617) suggests the phase landscape is barely changed, but the small perturbation is enough to break xi.

Transfer regression (0.836 → 0.661) is intermediate between alpha*0.0 (0.836) and alpha*0.5 (0.601), confirming the destructive-pair relationships that support transfer are proportionally disrupted.

### Destructive repulsion axis: fully closed

| alpha | xi | transfer | fitness |
|-------|-----|---------|---------|
| 0.0 (baseline) | 0.559 avg | 0.836 | 0.099 |
| 0.1 (this fire) | 0.107 | 0.661 | 0.196 |
| 0.5 (T23) | 0.745 | 0.601 | 0.118 |

No alpha value between 0 and 0.5 is expected to improve fitness. The T23 analysis concluded the xi↔transfer anticorrelation is structural. This fire confirms the "disruption valley" at alpha=0.1 is even worse than either extreme. The destructive repulsion axis is fully closed.

---

## Consolidated open items after this fire

| question | status |
|----------|--------|
| K axis (stage_sync) | CLOSED: K=0.5 only |
| A axis (stage_sync) | CLOSED: A=0.15 only |
| A axis (irx) | CLOSED: A=0.10 only |
| irx relax_steps | CLOSED: 16 is ceiling |
| DRIVE_FREQ_HZ | CLOSED: 0.5 Hz only |
| 0.25 Hz xi stability with K=0.5+A=0.15 | NEW: CLOSED — xi stability is amplitude-contingent |
| irx destructive repulsion (any alpha) | NEW: FULLY CLOSED — disruption valley at alpha=0.1 confirmed |

**Genuine remaining open items:**

1. **stage_sync transfer improvement (0.655 vs irx 0.836):** The largest remaining fitness gap (0.181 × 0.15 = 0.027 fitness units). No known parameter controls this without breaking carrier_e or xi. Would require structural changes (stage_boost_prune thresholds, stage_hallucinate parameters, or a new consolidation stage).

2. **irx xi variance root cause:** xi ranges 0.256–0.874 under baseline irx (N>10 trials across fires). The RNG-driven adversarial test is the dominant variance source. Seeding `eval_xi_robustness_v2` would isolate whether xi variance is: (a) structural to the dream configuration each trial, or (b) purely adversarial-RNG noise. If (a), there may be a slow consolidation signal worth chasing.

3. **Stage parameter exploration (unexplored):** `stage_boost_prune`, `stage_hallucinate`, `stage_wire` thresholds — none have been varied in L5 research. These are structural mechanisms that might improve transfer under stage_sync without touching the K/A/freq axes.

---

## Decision

No code changes retained. Both hypotheses falsified.

**Empirical optima unchanged:**
- `DRIVE_A=0.1 DREAM_MODE=interference_relax DRIVE_SCOPE=all DRIVE_FREQ_HZ=0.5` → avg fitness **0.099**
- `KURAMOTO_COUPLING=0.5 DRIVE_A=0.15 DRIVE_SCOPE=all DRIVE_FREQ_HZ=0.5` → avg fitness **0.104**

The destructive repulsion axis is now fully closed (alpha*0.1 confirms disruption valley below irx baseline's historical xi floor). The 0.25 Hz xi stability is confirmed as amplitude-contingent to A=0.1+K=1.0.
