# 2026-08-01T14 — Structural floor confirmed; all autoresearch levers exhausted

All six research questions from the autoresearch loop have been addressed in prior fires:
all parameter levers are exhausted, the system is at its structural floor (fitness ~0.018
in current containers, ~0.017 in Jul 26 environment), and no new hypotheses remain that
could yield ≥0.005 fitness improvement within the current architectural constraints.

## Summary of exhausted levers (complete as of Jul 31 fire)

| lever | status |
|-------|--------|
| KURAMOTO_COUPLING (1.0–5.0) | K=2.0 optimal for transfer; K=1.0 for xi_eval |
| DREAM_GRAVITY (0.25–0.40) | no improvement; 0.35 optimal |
| chiral_b_primed (0.0–0.15) | 0.15 is optimal (confirmed Jul 31) |
| CHAIN_TOP_N (5–10) | no improvement; 7 optimal |
| xi_flat_bprimed | no improvement |
| DRIVE_FREQ_HZ (0.5–3.0; 4 Hz degenerate) | 2 Hz optimal (closed Jun 6) |
| B_primed chain_depth (3 vs 4) | depth=4 optimal (Jul 27) |
| B memory phase alignment | falsified (Jul 29) |
| kuramoto_steps=100 | catastrophic (Jul 20) |
| interference_relax mode | no improvement; relax_steps already at 16/20 in code |
| consolidation_repulsion_threshold | catastrophic at 0.22; 0.28 is structural equilibrium |
| chiral_p=0.0 for B_primed | falsified (Jul 31) |
| xi_eval K (0.5, 1.0, 1.5, 2.0) | K=1.0 optimal (Jul 26); K=1.5 falsified (Jul 29) |

## Six autoresearch questions — disposition

1. **3-run interference_relax characterization**: mode shows no fitness improvement vs
   unset; xi collapses (0.220 vs 0.642). Closed.
2. **K-sweep under fixed plumbing**: K=2.0 optimal for transfer; K=1.0 for xi_eval.
   Non-monotone landscape fully mapped. Closed.
3. **interference_relax + xi recovery (relax_steps)**: code already uses relax_steps=16
   (engine_a) / 20 (engine_b_primed). Question was overtaken by code updates. Closed.
4. **R-xi correlation at stage_sync**: K sweep is exhausted; xi=0.9980 at K=1.0 with no
   R variation. No new signal possible from additional K trials.
5. **Φ ↔ R relationship across drive intensities**: magic_proxy_phase_R=0.608 is constant
   across all tested configs (transfer, xi, chiral sweeps). Drive intensity A≥0.3 is
   known-bad; A<0.1 offers no lever given carrier_emergence already saturated at 1.0.
   No improvement path exists here at the structural floor.
6. **Drive frequency 4 Hz**: identified as degenerate in Jun 6 production fire. Closed.

## Remaining sub-threshold improvements (not tested further)

- **phi_target decoupling**: saves 0.003510 (consciousness), needs +0.001490 bundled.
  No bundling candidates have been found across any fire since Jul 21 identification.
- **xi cross-container variability**: xi=0.9980 achievable in some containers (0.9678
  in others); FP non-determinism, not parameter-controllable.

## Decision

No trials run. No code changes made. No TSV rows appended.

Autoresearch autofires on this architecture are complete. Future improvement requires
architectural changes outside the autoresearch scope.
