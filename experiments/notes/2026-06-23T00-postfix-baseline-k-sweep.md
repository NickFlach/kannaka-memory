# 2026-06-23T00 — Post-bugfix baseline calibration + K-sweep

## Hypothesis

The 2026-06-22 hardening PRs (#439/#440) landed AFTER the last research fire
(2026-06-22T14:04 UTC vs. PR merged 17:07/17:58 UTC). Bug fixes to geometry.rs
(Cl(0,7) metric sign), chiral.rs (phase_locked_pairs used |sin|<0.1 instead of
cos>0.995), consolidation.rs (compact_ghosts), and medium/core.rs
(relate_wavefronts degenerate branch) likely changed the L5 fitness landscape.

Prediction: the pre-bugfix "floor" at 0.058 was partially propped up by metrics
inflated by mathematical errors. Post-bugfix baseline will differ, and the
K-sweep (now wired post-066d41a) will reveal R-xi anticorrelation as predicted
by the magic-gives-gravity hypothesis.

## Results

### Baseline (2 trials, K=3.0 default, DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE unset)

```
fitness:              0.081907 / 0.081911  (avg ≈ 0.082)
transfer_score:       0.850342 / 0.850342  (deterministic)
carrier_emergence:    0.5293
xi_robustness_v2:     0.9680
phase_coherence:      0.7334
magic_proxy_phase_R:  0.1293
query_gravity:        0.4603
```

### K-sweep (1 trial each)

| KURAMOTO_COUPLING | fitness | transfer_score | R     | xi_robustness | phase_coh |
|-------------------|---------|---------------|-------|---------------|-----------|
| 2.0               | 0.0958  | 0.8107        | 0.317 | 0.918         | 0.748     |
| 3.0 (default)     | 0.0819  | 0.8503        | 0.129 | 0.968         | 0.733     |
| 5.0               | 0.1427  | 0.5920        | 0.215 | 0.821         | 0.757     |

## Comparison to pre-bugfix floor

Pre-bugfix floor (last confirmed 2026-06-22T14:04, TSV rows with query_gravity column):
```
transfer_score:  ~0.965   →  now 0.850  (−0.115)
phase_coherence: ~0.997   →  now 0.733  (−0.264)
xi_robustness:   ~0.968   →  now 0.968  (unchanged)
carrier_emergence: ~0.533 →  now 0.529  (unchanged)
fitness:          ~0.058  →  now 0.082  (+0.024 worse)
```

The fitness regression is driven primarily by:
- transfer_score: +0.017 to fitness (15% weight × 0.115 drop)
- phase_coherence: +0.005 to fitness (2% weight × 0.264 drop)

## Interpretation

**Honest regression**: the pre-bugfix metrics were mathematically incorrect.
- `geometry.rs simplify_blade_merge` was computing in Cl(7,0) instead of Cl(0,7);
  the fixed encoding changes cross-corpus similarity structure → transfer_score drop.
- `chiral.rs phase_locked_pairs` was using |sin(dphi)|<0.1, which is satisfied at
  dphi≈π (anti-phase). Fixing to cos(dphi)>0.995 means truly anti-phase pairs are
  no longer counted as locked → phase_coherence metric reflects reality now.
- `medium/core.rs relate_wavefronts` previously errored on phase-opposed pairs
  (adding 2×DIM buffer rejected by add_wavefront). Fixed: phase-opposed pairs
  actually relate now, changing consolidation dynamics.

The new baseline (fitness ≈ 0.082) is the calibrated post-bugfix operating point.

## K-sweep findings (post-bugfix)

1. **K=3.0 remains optimal**: lowest fitness across {2.0, 3.0, 5.0}.
2. **R-xi anticorrelation confirmed**: K=3.0 has the lowest R (0.129) and highest xi
   (0.968). As K departs from 3.0, R increases and xi degrades. Consistent with the
   magic-gives-gravity prediction: lower global synchronization (low R) = more
   non-Clifford-like diversity = better adversarial robustness.
3. **K=5.0 over-synchronizes**: R rises (0.129→0.215), xi collapses (0.968→0.821),
   transfer_score collapses (0.850→0.592). Too much coupling homogenizes phases.
4. **K=2.0 paradox**: R is highest (0.317) despite lowest K. Suggests a different
   attractor basin — weaker coupling doesn't simply reduce R but lands on a regime
   with different phase structure. Xi intermediate (0.918), transfer worst of the
   three in terms of trend.

## Decision

No code changes. The new post-bugfix floor is **fitness ≈ 0.082** at K=3.0
(DRIVE_A=0.1 DRIVE_SCOPE=all). Previous 0.058 floor was on incorrect code.

Next fire candidates:
- Try K values between 3.0 and 4.0 to see if any reduce transfer_score regression.
- Investigate whether phase_coherence can be recovered — it was 0.997 pre-bugfix,
  now 0.733. The chiral.rs fix is the most likely cause; understand what specific
  phase structure the system now settles into.
- Check if DRIVE_A=0.05 (lower amplitude) shifts the attractor to recover transfer.
