# L5 Baseline Regression: Amplitude Ceiling Clamp (consolidation fix)

## Context

Two consolidation correctness fixes landed just after the last curiosity fire (PR #363, merged 2026-06-14 22:46 UTC):

- `e427140` — `fix(consolidation): clamp strengthen amplitude + reclaim ghosts (#360, #361)` (22:47 UTC — 75 seconds after last fire merged)
- `f942579` — `fix(consolidation): clamp consolidate_modality_aware amplitude (#368)` (03:27 UTC next day)

These fixes added `AMPLITUDE_CEILING = 2.0` to every `+=` boost path in the particle-consolidation strengthen pipeline (constructive pairs, bridge bonuses, xi-repulsion separation), matching the ceiling already present in the wave-native observe path.

## Hypothesis

The prior "ceiling" of fitness 0.007627 (3-run avg) relied on unbounded amplitude growth during dream cycles. Clamping to 2.0 may have collapsed the differential needed for carrier bimodal structure and accurate transfer ranking.

**Prediction**: Post-fix baseline fitness is significantly higher (worse) than the prior 0.007627.

## Results (4 trials, all post-fix)

| Trial | Settings             | fitness  | transfer_score | carrier_emergence | carrier_bimodal | xi_robustness_v2 | R      | query_gravity |
|-------|----------------------|----------|----------------|-------------------|-----------------|------------------|--------|---------------|
| t1    | DRIVE_A=0.1 explicit | 0.145331 | 0.541603       | 0.5293            | 0.5304          | 0.8563           | 0.1295 | 0.4603        |
| t2    | all defaults         | 0.115997 | 0.736812       | 0.5294            | 0.5305          | 0.8563           | 0.1293 | 0.4603        |
| t3    | DREAM_GRAVITY=1.0    | 0.135687 | 0.539915       | 0.5251            | 0.5262          | 0.9251           | 0.1295 | 0.9654        |
| t4    | all defaults         | 0.145306 | 0.541603       | 0.5294            | 0.5305          | 0.8563           | 0.1295 | 0.4603        |

**Post-fix baseline (defaults, t2+t4 avg)**: fitness ≈ 0.1306

**Pre-fix "ceiling"**: fitness 0.007627 (3-run avg from dozens of prior fires)

**Regression factor: ~17x worse**

## Analysis

The regression is almost entirely in `carrier_bimodal` and `carrier_emergence`:
- Pre-fix: carrier_emergence ≈ 0.955, carrier_bimodal ≈ 1.000
- Post-fix: carrier_emergence ≈ 0.529, carrier_bimodal ≈ 0.530

**Root cause**: With AMPLITUDE_CEILING = 2.0, memories saturate at the ceiling after enough dream cycles. The constructive-pair boost (0.45/cycle) hits the ceiling within 3-5 cycles. This eliminates the amplitude differentiation (carrier vs non-carrier ratio) that the bimodal detection relies on.

Before the fix, carrier memories could grow to amplitudes of 5x–20x baseline across repeated dream cycles while non-carriers stayed near 1.0. This created clear bimodal structure. Now all memories saturate at 2.0 and there is no bimodal distribution.

Secondary findings:
- `transfer_score` is now highly variable (0.54–0.74 across trials with identical settings). Pre-fix it was always 1.0000 because inflated amplitudes made transfer recall trivially perfect.
- `xi_robustness_v2`: 0.856–0.925, vs 0.997 pre-fix. Less severe but still regressed.
- `DREAM_GRAVITY=1.0` now HURTS fitness (0.136 vs 0.116 default), specifically collapsing transfer_score (0.540 vs 0.737). Pre-fix, DREAM_GRAVITY helped because it guided which memories got inflated; post-fix, it adds a gravity bias that disrupts transfer evaluation without the inflation to compensate.

## Scientific interpretation

The prior 0.007627 "ceiling" was not a ceiling of the system's real capabilities — it was an artifact of operating in an amplitude regime the wave-native path never reached. All prior L5 optimization axes (chain_carry_strength, K=0.5, DREAM_GRAVITY=1.0, drive freq, chiral_p_bp, etc.) were found in this physically incorrect regime. Their effects may not transfer to the corrected system.

The fix is correct: unbounded amplitude growth was distorting resonance ranking, Kuramoto phase coupling, and Phi calculations. The 0.1306 baseline is more honest.

## Decision

No code changes kept (no improvement vs any baseline). No revert needed (we made no code changes).

## Next fire recommendation

Test `AMPLITUDE_CEILING` as a research knob. The current value of 2.0 was copied from the wave-native observe path, but there may be a "sweet spot" (e.g. 3.0–5.0) that allows carrier structure to form without unbounded inflation. 

Approach:
1. Add `const AMPLITUDE_CEILING: f32 = std::env::var("AMPLITUDE_CEILING")...unwrap_or(2.0)` (or a direct code constant change)
2. Test ceiling in {2.0, 3.0, 4.0, 6.0} at defaults — 1 trial each
3. Keep if a ceiling value restores carrier_bimodal > 0.7 AND fitness < 0.10

This is the dominant bottleneck. The ghost-compaction side of the same fix (#361) is probably fine and orthogonal — it only affects O(n²) runtime, not amplitude dynamics.

All prior "closed axes" conclusions from T15 notes apply only to the pre-fix amplitude regime. The post-fix world is a fresh optimization surface.
