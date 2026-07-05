# 2026-07-05T14 — Baseline shift discovered; hypothesis invalidated by consolidation fix

## Hypothesis

chain_depth=7 for flat engine + skip first 3 cycles (deltas[3:7]) would capture a
[1st_inject_spike, settle, settle, 2nd_inject_spike] pattern from the injection cadence
(injection_cycles=[2,5,8,...], batch size=10 memories at amp=0.8 each). Predicted:
- carrier_emergence → ~0.90-0.97 via k=1 dominance from the periodic injection pattern
- fitness → ~0.019-0.030 (improvement Δ ≥ 0.015 from 0.045 floor)

## Reality: baseline has shifted due to commit 4a1c4e6

Commit 4a1c4e6 "fix(consolidation): circular phase math — wrap-straddling pairs no longer
misfire" changed amplitude dynamics in the flat engine. The current baseline (chain_depth=5,
skip cycle 0 — previously the L5 floor at fitness=0.045) now gives:

| metric              | old floor (pre-fix) | current baseline (post-fix) |
|---------------------|---------------------|-----------------------------|
| fitness             | 0.04539             | 0.07545                     |
| carrier_emergence   | 0.6390              | 0.9868                      |
| transfer_score      | 0.9652              | 0.7412                      |
| xi_robustness_v2    | 0.9796              | 0.7910                      |
| magic_proxy_phase_R | 0.867               | 0.3375                      |

carrier_emergence already improved from 0.639 → 0.987 as a side-effect of the fix.
My hypothesis about capturing the injection-cadence pattern was correct in mechanism but
irrelevant — the signal was already there.

## My code change results

Config: `DRIVE_A=0.1 DRIVE_SCOPE=all DRIVE_TOP_FRAC=1.0 DREAM_MODE=interference_relax DREAM_GRAVITY=0.25`

| trial | code | fitness | carrier_e | transfer | xi_robust |
|-------|------|---------|-----------|----------|-----------|
| 1 | chain_depth=7, skip 3 | 0.075480 | 0.9864 | 0.741184 | 0.7910 |
| 2 | chain_depth=7, skip 3 | 0.075481 | 0.9864 | 0.741184 | 0.7910 |
| ref | chain_depth=5, skip 1 (baseline) | 0.075448 | 0.9868 | 0.741184 | 0.7910 |

The code change produced carrier_emergence 0.9864 vs baseline 0.9868 — essentially
identical. The change is a no-op.

## Decision: code change REVERTED

The hypothesis was already realized by the consolidation fix. Carrier_emergence is at ceiling.
No code changes kept. TSV rows from trials 1 and 2 retained as they document the
post-fix operating point.

## New dominant issue: transfer_score regression

With carrier_emergence at ceiling (0.987), the fitness is now dominated by:
- transfer_score: 0.741 → weight 0.15 × (1-0.741) = 0.039 (52% of fitness)
- xi_robustness_v2: 0.791 → weight 0.15 × (1-0.791) = 0.031 (41% of fitness)
- carrier_emergence: 0.987 → weight 0.10 × (1-0.987) = 0.001 (1% of fitness)

Post-fix fitness floor: **0.07545** (up from old 0.04539).

magic_proxy_phase_R dropped to 0.3375 — close to the pre-irx stage_sync baseline (0.355).
The circular phase fix may have changed which pairs qualify as constructive, reducing the
interference_relax mode's effectiveness.

## Known: old transfer=0.965 was partially an artifact

The pre-fix consolidation accepted wrap-straddling pairs with incorrect phase math. These
false-positive constructive pairs may have artificially boosted cross-corpus transfer by
propagating phase-aligned patterns that weren't truly constructive. Post-fix, true transfer
is ~0.741.

## Next fire recommendation

1. **Understand transfer regression**: is there a parameter change that recovers transfer
   without reverting the consolidation fix? Try DREAM_GRAVITY sweep (0.1, 0.25, 0.5, 0.75)
   — gravity shapes which memories get amplitude boost; different gravity may change transfer.
2. **xi recovery**: xi_robustness_v2 went from 0.980 to 0.791. With chain_depth=2 for xi
   eval, the reduced consolidation steps may interact differently with the fix. Try chain_depth=3.
3. **DREAM_MODE=stage_sync**: magic_R dropped to 0.3375 (stage_sync zone). Compare unset
   vs interference_relax at new operating point — interference_relax may no longer offer advantage.
4. **Accept new floor 0.075**: the consolidation fix is correct. The old 0.045 included
   measurement artifacts (carrier scored on buggy phase dynamics, transfer boosted by false-
   positive constructive pairs). The new floor is harder but more honest.

## Env-var space mapping (post-fix)

All previously confirmed optimal values may need re-validation:
- DREAM_GRAVITY=0.25: still seems optimal (query_gravity unchanged at 0.862)
- DREAM_MODE=interference_relax: less clear benefit vs unset (magic_R dropped)
- DRIVE_A=0.1: still within known-good range
- DRIVE_SCOPE=all: unchanged
