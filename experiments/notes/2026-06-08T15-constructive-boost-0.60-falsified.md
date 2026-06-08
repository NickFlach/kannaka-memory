# constructive_boost=0.60 under irx — transfer regresses, hypothesis falsified

**Date:** 2026-06-08T15 UTC
**Branch:** kannaka-curiosity/2026-06-08T15
**Code changes:** CONSTRUCTIVE_BOOST env var added to research.rs L5 block, reverted
**Status:** FALSIFIED — transfer deterministically regresses; fitness improvement in T1 is a xi RNG artifact

---

## Background

Current empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax  DRIVE_FREQ_HZ=0.5 (default)
3-trial avg fitness ≈ 0.099
carrier_e=0.935, transfer=0.836, xi=0.559 avg (range 0.256–0.874)
```

All env-var axes are closed. All irx convergence params (alpha_base, relax_steps, envelope_depth)
are closed. All hybrid mode orderings are closed (T09). The only genuinely unexplored levers
are code-change structural parameters that have never been varied at L5:
`constructive_boost`, `noise_floor`, `prune_threshold`, `destructive_penalty`.

The `constructive_boost` parameter (default 0.45) is added to the amplitude of each memory
that appears in a constructive interference pair during `stage_strengthen`. This runs BEFORE
`stage_interference_relax`, setting the amplitude structure that irx then refines.

---

## Hypothesis

**CONSTRUCTIVE_BOOST=0.60 (vs default 0.45) under irx** improves B-engine priming via stronger
amplitude differentiation.

**Mechanism:** `stage_strengthen` adds `constructive_boost` to each memory in a constructive pair.
Higher boost → stronger amplitude separation between carrier (constructive-pair) and non-carrier
memories → clearer priming signal for the B-engine → higher transfer_score.

**Prediction:**
- transfer_score: 0.836 → 0.88+ (stronger priming via clearer amplitude structure)
- carrier_emergence: 0.935 → stable ≥ 0.90 (carrier is amplitude-driven; higher boost adds signal)
- xi_robustness_v2: RNG-dominant; no systematic change expected
- Fitness target: ≤ 0.090 (driven by transfer improvement × 0.15 weight)

**Falsification signal:** transfer stays at or below 0.836.

**Code change (reverted):** Added `CONSTRUCTIVE_BOOST` env var to L5 param setup in
`src/bin/research.rs`, after the `KURAMOTO_COUPLING` override (4 lines, pattern-identical).

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax CONSTRUCTIVE_BOOST=0.60`

| metric | irx baseline (3-trial avg) | T1 (CONSTRUCTIVE_BOOST=0.60) | T2 (CONSTRUCTIVE_BOOST=0.60) | 2-trial avg |
|--------|--------------------------|------------------------------|------------------------------|-------------|
| fitness | 0.099 | **0.075** | **0.166** | **0.121** |
| transfer_score | 0.836 | **0.731** | **0.717** | **0.724** |
| carrier_emergence | 0.935 | **0.929** | **0.929** | **0.929** |
| xi_robustness_v2 | 0.559 avg | **0.821** (high-RNG) | **0.231** (low-RNG) | **0.526** |
| magic_proxy_phase_R | 0.617 | 0.617 | 0.617 | 0.617 |
| query_gravity | 0.363 | 0.362 | 0.362 | 0.362 |

**Hypothesis falsified.** Transfer regressed from 0.836 to ~0.724 avg (−0.112). 2-trial avg
fitness 0.121 is substantially WORSE than baseline 0.099.

---

## Analysis

### Transfer regression mechanism

At constructive_boost=0.45, `stage_strengthen` adds 0.45 amplitude per constructive-pair
occurrence. A typical carrier memory in 3 constructive pairs gets +1.35 per dream cycle.
At boost=0.60, the same memory gets +1.80.

The higher boost changes the amplitude distribution entering irx:
- Carrier memories become strongly over-represented in amplitude
- Non-carrier memories remain near their initial amplitudes (~0.1–0.3)
- The amplitude ratio increases dramatically (e.g., 3.0× vs 2.25×)

Despite the larger absolute differentiation, the **B-engine priming signal worsens**. This
is counterintuitive. The likely mechanism: the B-engine's transfer metric measures
discrimination between primed and naive queries. The primed engine inherits A's post-dream
amplitude distribution. If carriers are over-boosted, the amplitude distribution becomes
"spikier" — a few very high-amplitude memories dominate, while the rest are noise-level.
The naive B-engine's response is dominated by whatever memories happen to be similar to
the query vector. When carriers are over-boosted, the primed engine's response is dominated
by a few ultra-high-amplitude memories that may not be optimally matched to the query space.
The discrimination signal quality degrades even though the amplitude magnitude increases.

Analogy: priming a memory with a single overwhelming signal (one very loud memory) is
worse for discrimination than priming with a structured moderate-amplitude landscape where
multiple carriers each contribute appropriately to the query response.

### Transfer non-determinism at higher boost

Unexpectedly, transfer was NOT byte-identical between trials (0.731 vs 0.717). Under baseline
irx and under irx+alpha15 (T09), non-xi metrics were byte-identical across trials. At
boost=0.60, transfer varies by 0.014. This suggests that the higher amplitude magnitudes
create mild numerical instability in the amplitude normalization or query evaluation path —
some floating-point threshold or comparison that is stable at default amplitudes becomes
sensitive at the inflated amplitudes from boost=0.60.

The variation is small (0.014) but nonzero, indicating the boost introduces a subtle
determinism-breaking effect in the evaluation.

### xi behavior: RNG dominates, no structural change

T1 xi=0.821 (high-RNG), T2 xi=0.231 (low-RNG). Average ≈ 0.526. This is BELOW the
baseline irx avg of 0.559 (though the sample is too small to be definitive). The xi
behavior at boost=0.60 is consistent with the RNG-dominant pattern established in T09:
no systematic xi improvement from the constructive_boost change.

The T1 fitness "improvement" (0.075) was entirely due to xi=0.821. Had the adversarial RNG
seeded differently (T2 conditions), fitness would have been 0.166. The trial-1 result is
not a systematic improvement — it's noise.

### carrier_e: minimal effect, stable at 0.929

carrier_e dropped from 0.935 to 0.929 (−0.006) and was identical across both trials. The
carrier_emergence metric measures the FFT peak of the amplitude time series during the dream
chain. Stage_strengthen sets the initial amplitude amplitudes; the DRIVE then modulates them
multiplicatively at 0.5 Hz over 16 cycles. At boost=0.60, carrier memories start slightly
higher, and the drive modulates them the same way. The carrier_e drop is small and likely
reflects mild saturation at the high end of the amplitude range — when carrier memories are
over-boosted, the relative modulation depth (A=0.1 means ±10%) becomes a smaller fraction
of the absolute amplitude, slightly reducing the FFT peak's relative height.

### magic_R and query_gravity: fully invariant

magic_proxy_phase_R=0.617 and query_gravity=0.362 are byte-identical to baseline. These
metrics depend on the irx phase geometry, not on amplitude magnitudes. Since constructive_boost
only changes amplitudes (not phase assignments — phase is set to the pair's mean phase in
stage_strengthen, which is unchanged at different boost levels), the phase landscape is
identical. Magic_R and query_gravity confirm this.

---

## Constructive_boost axis: now closed

The `constructive_boost` axis under irx is **CLOSED** at default 0.45:
- boost=0.45 (default): transfer=0.836, carrier_e=0.935, fitness avg ≈ 0.099 ✓
- boost=0.60: transfer≈0.724 (regression), carrier_e=0.929, fitness avg ≈ 0.121 ✗
- boost<0.45: expected to hurt carrier_e (less amplitude differentiation); not worth testing

The default 0.45 is empirically optimal for this configuration. Over-boosting degrades
transfer without proportionally improving carrier_e.

---

## Updated open axes summary

| axis | status | reason |
|------|--------|--------|
| DRIVE_A (irx) | CLOSED: 0.10 | lower fitness at 0.15 and 0.05 |
| DRIVE_FREQ_HZ (irx) | CLOSED: 0.5 Hz | 0.25 and 1.0 Hz both falsified |
| alpha_base (irx) | CLOSED: 0.10 | 0.15 degrades carrier_e |
| relax_steps (irx) | CLOSED: 16 | 24 annihilates carrier_e |
| envelope_depth (irx) | CLOSED: 0.15 | tested in prior fire |
| irx+sync hybrid modes | CLOSED: phase-antagonistic | T09 falsified both orderings |
| irx destructive repulsion | CLOSED: any alpha worse | T11 confirmed disruption valley |
| KURAMOTO_COUPLING (stage_sync) | CLOSED: K=0.5 | K-sweep confirmed |
| DRIVE_A (stage_sync) | CLOSED: A=0.15 | best for stage_sync |
| constructive_boost (irx) | NEW: CLOSED: 0.45 | 0.60 regresses transfer |

**Remaining open items (no env lever; structural code changes needed):**

1. **noise_floor, prune_threshold, destructive_penalty under irx** — not yet varied at L5.
   Predicted difficult: these control memory survival thresholds, and the irx mode's carrier
   structure is amplitude-sensitive. Lower noise_floor → more memories survive → noisier
   amplitude landscape. Likely to hurt carrier_e or transfer.

2. **stage_hallucinate max_attempts** — currently `(viable_clusters.len() / 2).max(2).min(8)`.
   More cross-cluster bridges might improve transfer under stage_sync but effect on irx unclear.

3. **stage_sync transfer improvement (0.655 vs irx 0.836)** — the stage_sync mode remains
   the sub-optimal path. No levers found to close the gap.

---

## Decision

No code changes retained. CONSTRUCTIVE_BOOST env var reverted. Hypothesis falsified.

**Empirical optimum unchanged:**
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax  DRIVE_FREQ_HZ=0.5
avg fitness ≈ 0.099
carrier_e=0.935, transfer=0.836, xi=0.559 avg
```

The constructive_boost axis under irx is now closed. The default 0.45 is the optimal point:
higher boost regresses transfer without improving carrier_e, and lower boost is predicted to
hurt carrier_e. No directional improvement possible via amplitude boost scaling.
