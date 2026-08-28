# 2026-08-28T14 — No new hypothesis: all levers exhausted post-Gram-fix

All six autoresearch research questions are closed and the parameter space is fully
characterized. No trials run; no TSV rows appended.

## Orientation summary

Current compiled-in defaults (Aug 24 persistence):
```
DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE unset
KURAMOTO_COUPLING=2.0 DREAM_GRAVITY=0.35
CARRIER_KURAMOTO_COUPLING=1.5 xi_eval K=1.0 chain_depth=3
```
Container floor: fitness ~0.019 (xi=0.9678 FP-non-deterministic variant).

## Six research questions — status

| question | status |
|---|---|
| 1. 3-run interference_relax characterization | Closed Aug 1: xi collapses to 0.220, no fitness gain |
| 2. K-sweep under fixed plumbing | Closed Aug 1 / Jul 12: K=2.0 optimal, landscape fully mapped |
| 3. interference_relax + xi recovery (relax_steps) | Closed Jun 5: relax_steps=8→16 kills carrier_e (0.714→0.000) |
| 4. R-xi correlation at stage_sync | Closed Aug 1: xi=0.9980 at K=1.0, no R variation |
| 5. Φ ↔ R relationship | Closed Aug 1: magic_R=0.608 constant across all tested configs |
| 6. Drive frequency variants | Closed Jun 6: 2 Hz optimal, 4 Hz degenerate, others worse |

## Additional levers closed since Aug 1

| lever | fired | result |
|---|---|---|
| phi_target decoupling (0.3138) | Aug 25 + Aug 27 | Net-negative post Gram-fix: xi drops 0.9678→0.9115 |
| xi_eval depth=4 at K=3.0 | Aug 25 | xi collapses to 0.8590 |
| CARRIER_KURAMOTO_COUPLING=1.0 | Aug 25 | carrier_e cliff (0.8861) |
| stage_sync dt=0.03 | Aug 27 | Destructive across all metrics |
| phi_target any value > 0.281 | closed analytically | Gram-fix makes adv-clean consciousness gap dominate xi |

## Why no intermediate phi_target helps

Post Gram-fix, clean phi ≈ 0.3138 and adversarial phi ≈ 0.235. Any phi_target above
the current 0.281 widens the adv-clean consciousness gap (weight 0.03) faster than it
saves consciousness (same weight). xi (weight 0.15) loses more than consciousness gains
at any value in (0.281, 0.3138]. phi_target below 0.281 worsens consciousness without
helping xi. No intermediate value improves fitness.

## Decision

No trial warranted. Floor remains at ~0.019. Future improvement requires architectural
changes outside the autoresearch scope.
