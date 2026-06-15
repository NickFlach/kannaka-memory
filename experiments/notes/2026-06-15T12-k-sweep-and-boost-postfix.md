# L5 Post-Fix: K-sweep and constructive_boost sweep — both falsified

**Date:** 2026-06-15T12 UTC  
**Branch:** kannaka-curiosity/2026-06-15T12-k-sweep-postfix  
**Code changes:** REVERTED — no improvement found  
**Status:** Both axes falsified; root cause of carrier_e regression diagnosed as pair-density saturation

---

## Context

Post-fix baseline (ceiling=2.0): fitness ≈ 0.116 (T06 confirmed).  
Three main regression axes vs pre-fix 0.007461:
- carrier_emergence: 0.529 (needs 0.999), 10% weight → 0.047 fitness cost
- transfer_score: 0.737 (needs 0.964), 15% weight → 0.034 fitness cost
- xi_robustness_v2: 0.856 (needs 0.997), 15% weight → 0.021 fitness cost

Prior fires (T01, T03, T06) confirmed the regression is from AMPLITUDE_CEILING=2.0 in consolidation.rs.

---

## Hypothesis 1: K-sweep post-fix (no code changes)

The pre-fix K-sweep (2026-06-06) found K=0.5 optimal, but it was done in the unconstrained amplitude regime. With ceiling=2.0, the optimal K might differ. Tested K ∈ {1.0, 2.0} with default DRIVE_A=0.15, DRIVE_SCOPE=all.

**Results:**

| K    | fitness  | transfer_score | carrier_e | xi_v2  | phase_coherence | R      |
|------|----------|----------------|-----------|--------|-----------------|--------|
| 0.5  | 0.114918 | 0.736812       | 0.5294    | 0.8563 | 0.7334          | 0.1293 |
| 1.0  | 0.113571 | 0.720297       | 0.5259    | 0.8862 | 0.7445          | 0.2744 |
| 2.0  | 0.155810 | 0.436262       | 0.5331    | 0.8873 | 0.7477          | 0.3171 |

**Analysis:**
- K=1.0: marginally better fitness (0.114 vs 0.115) but within trial variance. Trade-off: xi_v2 improves (+0.030) while transfer_score regresses (-0.017). Net wash. R rises from 0.129 to 0.274 — stronger synchronization.
- K=2.0: dramatically worse. transfer_score collapses to 0.436. Phase_coherence stays similar (0.748 vs 0.733). Over-synchronization hurts transfer.
- K=0.5 remains near-optimal post-fix. **Axis closed.**

Surprising finding: phase_coherence at K=0.5 from this run is 0.733, matching K=1.0 and K=2.0 runs. Earlier T06 showed 0.933 for the same settings. Variance in phase_coherence is higher than expected.

---

## Hypothesis 2: Reduce constructive_boost to < DRIVE_A × ceiling (code change)

**Reasoning:** For saturated memories (amplitude = 2.0), the drive at DRIVE_A=0.15 knocks amplitude to 2.0×0.85=1.70 (cycle 3, sin=-1). Consolidation boost of 0.45 then over-corrects back to 2.0, giving zero delta. For drive oscillations to be visible post-saturation, need boost < 0.15×2.0=0.30. Tested constructive_boost=0.20.

**Code change:** Added `CONSTRUCTIVE_BOOST` env knob in `run_experiment_l5_session()` L5 params block.  
**Result:** REVERTED.

| boost | fitness  | transfer_score | carrier_e | xi_v2  | amp_deltas_flat                        |
|-------|----------|----------------|-----------|--------|----------------------------------------|
| 0.45  | 0.114918 | 0.736812       | 0.5294    | 0.8563 | [0.950, 0.031, 0.010, 0.042]           |
| 0.20  | 0.141437 | 0.555486       | 0.5363    | 0.8563 | [0.942, 0.037, 0.005, 0.039]           |

**Analysis:**
- carrier_e barely moved (0.529 → 0.536), despite boost=0.20 being below the 0.30 threshold.
- amp_deltas_flat is STILL an impulse pattern: [large, ~0, ~0, ~0].
- Root cause of failure: the flat corpus has ~10-15 constructive pairs per memory. Each pair boosts amplitude by boost value. Total effective per-memory boost = N_pairs × boost = 10×0.20 = 2.0, which STILL hits the ceiling at cycle 0 from initial amplitude 1.0.
- To prevent cycle-0 saturation, would need boost ≈ 1.0/N_pairs ≈ 0.07. This is unreasonably low.
- The pre-fix carrier_e=0.999 required unconstrained growth to get the [A, A, ~0, ~0] delta pattern (≈1:1 ratio between cycles 0 and 1 at large amplitude, then decay). With ceiling=2.0, the initial impulse at cycle 0 is bounded at ~0.95 and cannot be matched by a large cycle-1 delta.
- transfer_score regressed significantly (0.737 → 0.555) because lower boost reduced b_primed's consolidation advantage over b_naive.

**Hypothesis falsified.** No improvement.

---

## Key diagnostic from K=0.5 baseline run

```
fitness_B_primed: 0.016489
fitness_B_naive:  0.062651
amp_deltas_flat:  [0.9498, 0.0306, 0.0096, 0.0424]
amplitude_deltas_a: [0.9495, 0.0310, 0.0122, 0.0488]
magic_proxy_phase_R: 0.1293
query_gravity: 0.4603
```

The amp_deltas pattern for both engines is identical in shape: impulse at cycle 0, near-zero thereafter. carrier_e ≈ 0.529 is mathematically determined by this impulse pattern (approximately: peak_power(k=1)/total = 0.53 for a unit impulse over 4 cycles).

---

## Why carrier_e cannot be recovered by parameter sweeping

The pre-fix carrier_e=0.999 required the flat corpus to exhibit delta pattern [A, A, ~0, ~0] — equal large deltas at cycles 0 and 1, then near-zero. This was possible because:
1. Without ceiling, cycle-0 growth (from 1.0) and cycle-1 growth (from 1.45+) could both be large
2. The adaptive controller (or natural saturation) created the 1:1 ratio
3. With ceiling=2.0, cycle-0 growth is bounded at ~0.95 (max possible) and cycle-1 is near-zero (already saturated)

No parameter in the external knob space (K, DRIVE_A, constructive_boost, chain_depth, DREAM_MODE) can restore this pattern without either:
- Raising the ceiling (tried in T03: transfer_score tradeoff kills net improvement)
- Changing how amp_deltas is computed (e.g., measure drive signal independently of ceiling clamp)
- Fundamentally different corpus behavior under ceiling=2.0

---

## Closed axes for next fire

- K-sweep post-fix: closed. K=0.5 ± 0.5 all similar or worse.
- CONSTRUCTIVE_BOOST sweep: closed. Pair density makes ceiling irrelevant to per-pair boost.
- AMPLITUDE_CEILING sweep: closed (T03). Transfer-carrier tradeoff prevents net improvement.
- DRIVE_A sweep: known-bad for A≥0.3; A=0.15 is default (already near-optimal per T06).

## Open questions (unexplored post-fix axes)

1. **DREAM_MODE=interference_relax** post-fix: pre-fix showed carrier_e 0.714 vs 0.559 at stage_sync. Phase-based mechanism (not amplitude) — ceiling might interact differently. Risk: pre-fix also showed xi collapse (0.220 vs 0.642).
2. **chiral_p_bp sweep** (research.rs line 3523, currently 0.15): changing phase perturbation in b_primed dream might affect fitness_b_primed/fitness_b_naive ratio and thus transfer_score. Low risk, no code constraint.
3. **xi_repulsion_weight** or **consolidation_repulsion_threshold** in l5_params: xi_v2 (0.856) and phase_coherence (0.733-0.933 variable) might respond to these.
4. **Fundamental: measure carrier signal independently of ceiling** — add `delta_signal_amplitude` as the drive contribution to each cycle (computed analytically from DRIVE_A and current amplitude) rather than the actual amplitude change. This would decouple carrier_e measurement from the ceiling clamp and restore the semantic "does the drive oscillate at the target frequency?" It requires a code change to run_l5_dream_chain but doesn't require modifying the ceiling fix.

## Decision

No code changes kept. No improvement in fitness.

The post-fix optimization surface requires either a fundamental change to carrier_e measurement semantics or an architectural approach beyond parameter sweeping.
