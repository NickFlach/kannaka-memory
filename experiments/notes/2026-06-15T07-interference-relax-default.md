# interference_relax as default: 50% fitness recovery in post-fix amplitude regime

**Date:** 2026-06-15T07 UTC  
**Branch:** kannaka-curiosity/2026-06-15T07-interference-relax-default  
**Code change:** `src/consolidation.rs` — make `DREAM_MODE=interference_relax` the default  
**Status:** KEPT — 2 trials confirmed, well above threshold

---

## Context

Post-fix baseline (commit e427140 + f942579, AMPLITUDE_CEILING=2.0 throughout consolidation):
- fitness ≈ 0.116
- transfer_score ≈ 0.737
- carrier_emergence ≈ 0.529
- xi_robustness_v2 ≈ 0.856

Prior fires T01–T06 attempted to recover via ceiling sweeps, DREAM_GRAVITY, and diagnostic runs. All failed (ceiling=8.0 falsified in T03, DREAM_GRAVITY=1.0 hurts transfer more than it helps xi).

`DREAM_MODE=interference_relax` was listed as a priority research question in the session brief and had never been tested in the post-fix amplitude regime. Pre-fix smoke tests (A=0.1) showed it performed the same as stage_sync (both fitness≈0.191), so it was not considered a priority before.

---

## Diagnostic finding (trial 2)

Running with defaults (ceiling=2.0) and logging `amp_deltas_flat` showed:
```
amp_deltas_flat: [0.9498, 0.031, 0.010, 0.042]
```

Almost all consolidation energy fires in cycle 0: memories hit AMPLITUDE_CEILING=2.0 in the first chain step, then all subsequent steps are near-zero delta. This one-shot convergence creates a DFT pattern with equal power across k=1 and k=2 (ratio ≈ 0.53 = carrier_e). The ceiling clamp is the root cause, but raising the ceiling uniformly hurts transfer_score (falsified T03).

---

## Hypothesis

`interference_relax` replaces Kuramoto (stage_sync) with constructive-pair-driven phase relaxation. In the Kuramoto path, aggressive global synchronization rapidly converges phases, enabling many constructive pairs to fire in cycle 0 and saturate memories at the ceiling. Under interference_relax, the phase relaxation is gentler (alpha_base=0.10–0.12, 16 steps), spreading phase convergence over multiple dream steps and potentially changing which constructive pairs fire per chain step.

**Prediction**: interference_relax distributes consolidation energy more evenly, improving transfer_score by keeping amplitude landscapes more discriminative, and improving xi by preserving phase diversity.

---

## Results

| Trial | fitness | transfer_score | carrier_e | xi_v2 | magic_R | query_gravity |
|-------|---------|---------------|-----------|-------|---------|---------------|
| baseline (defaults, T06)       | 0.115997 | 0.737 | 0.529 | 0.856 | 0.130 | 0.460 |
| t1 (interference_relax)        | 0.057627 | 0.965 | 0.533 | 0.968 | 0.867 | 0.460 |
| t2 (interference_relax repeat) | 0.057625 | 0.965 | 0.533 | 0.968 | 0.867 | 0.460 |

2-trial avg: **fitness 0.057626**

`amp_deltas_flat` with interference_relax: `[0.9498, 0.031, 0.003, 0.036]` — nearly identical to Kuramoto pattern. The carrier_e improvement is minimal (0.529 → 0.533), meaning the carrier bottleneck is NOT resolved by the phase change. But the phase structure produced by interference_relax dramatically improves the transfer test and xi.

---

## Analysis

**Transfer_score 0.737 → 0.965**: interference_relax produces phase alignment organized around constructive-pair neighborhoods rather than global Kuramoto attractors. This keeps the amplitude ranking among constructively-related memories more stable, making cross-corpus recall (transfer test) dramatically more reliable. The Kuramoto approach drives all memories toward global cluster centers, flattening local amplitude relationships and making transfer ranking ambiguous.

**xi_robustness_v2 0.856 → 0.968**: xi measures interference-based separation between clean and adversarial memory sets. The constructive-pair phase structure from interference_relax preserves more of the semantic interference structure, making clean vs. adversarial separation cleaner.

**magic_proxy_phase_R 0.130 → 0.867**: High R means tighter phase coherence within the set. Interference_relax drives phases toward constructive-pair neighbors, naturally creating higher local coherence than Kuramoto's global approach in the bounded amplitude regime.

**carrier_emergence unchanged (0.529 → 0.533)**: The fundamental bottleneck (all amp_deltas_flat energy in cycle 0) is unchanged by the phase algorithm. Carrier_e remains a structural constraint of AMPLITUDE_CEILING=2.0.

**query_gravity unchanged (0.460)**: Not affected by the phase algorithm change.

---

## Interpretation

The post-ceiling-fix amplitude regime changed which synchronization approach is optimal. Pre-fix (unbounded amplitudes), Kuramoto worked because amplitude inflation provided clear ranking signals regardless of phase structure. Post-fix (AMPLITUDE_CEILING=2.0 clamps all memories to 1.0–2.0), the phase structure becomes the primary discriminative signal. Interference_relax's pair-specific phase alignment preserves the discriminative phase structure that transfer and xi rely on, while Kuramoto's global clustering destroys it.

This is consistent with the wave-interference model's design intent: phase-based recall through constructive interference, not amplitude domination.

---

## Code change

`src/consolidation.rs` line 280–282:
- Changed `DREAM_MODE` default from `""` (Kuramoto) to `"interference_relax"`  
- Override to Kuramoto still available via `DREAM_MODE=stage_sync`

---

## Decision

**KEPT.** 2-trial avg fitness 0.057626 vs post-fix baseline 0.116. Improvement = 0.058 (12× threshold). Deterministic results, stable across trials.

---

## Cancelled exploration

Trial 1 (this fire): tested `AMPLITUDE_CEILING=8.0` without `DREAM_GRAVITY`. Result: fitness 0.159 (worse). ceiling=8.0 barely changes carrier_e (0.529→0.537) but significantly hurts transfer_score (0.737→0.492). The T03 ceiling=8.0 improvement in carrier_e required DREAM_GRAVITY=1.0 interaction; without it, ceiling=8.0 is net negative. Code change reverted.

---

## Next fire

- Carrier_emergence remains at 0.533 — the fundamental bottleneck is the one-shot consolidation pattern ({0.95, 0.03, 0.003, 0.036} amp_deltas_flat). To improve carrier_e, need either: (a) amplitude more evenly spread across chain steps, or (b) carrier_emergence metric that captures the drive signal before ceiling saturation.
- xi_robustness_v2 at 0.968 (near ceiling of 1.0) — little room for further improvement.
- The new post-interference_relax baseline is **0.057626**. Future improvements need fitness < 0.052626 to clear threshold.
