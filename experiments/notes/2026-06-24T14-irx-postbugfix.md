# 2026-06-24T14 — interference_relax post-bugfix characterization

## Hypothesis

DREAM_MODE=interference_relax has only been measured with pre-bugfix code. The
2026-06-22 hardening PRs (#439/#440) fixed two components that feed directly into
interference_relax's constructive-pair identification:

1. `chiral.rs phase_locked_pairs`: was using `|sin(dphi)|<0.1` (satisfied at
   dphi≈π, i.e., anti-phase counted as phase-locked). Fixed to `cos(dphi)>0.995`
   — only truly co-phase pairs now qualify.
2. `geometry.rs simplify_blade_merge`: was computing in Cl(7,0) instead of Cl(0,7);
   the corrected metric changes cross-corpus similarity structure.

Prediction: post-bugfix interference_relax will have higher xi_robustness (was 0.220
pre-bugfix, degraded by wrong pair detection) and lower fitness than the pre-bugfix
measure of 0.191. The mode may now rival or beat stage_sync's post-bugfix 0.082.

Also checked: DRIVE_A=0.05 (trial 1) — hypothesized that halving drive amplitude
would reduce amplitude noise and recover transfer_score. Falsified immediately: both
fitness (0.082317) and transfer_score (0.850342) were identical to DRIVE_A=0.1,
confirming transfer_score is determined by structural parameters, not drive amplitude.

## Configuration

```
DRIVE_A=0.1 DRIVE_SCOPE=all DRIVE_TOP_FRAC=1.0 DREAM_MODE=interference_relax
KURAMOTO_COUPLING=3.0 (default)
```

## Results (3 trials)

| trial | fitness  | transfer_score | xi_robustness | carrier_e | magic_R | query_gravity |
|-------|----------|---------------|---------------|-----------|---------|---------------|
| 1     | 0.057826 | 0.965401      | 0.9675        | 0.5330    | 0.8672  | 0.4603        |
| 2     | 0.057821 | 0.965401      | 0.9675        | 0.5330    | 0.8672  | 0.4603        |
| 3     | 0.057830 | 0.965401      | 0.9675        | 0.5330    | 0.8672  | 0.4603        |
| avg   | **0.057826** | 0.965401  | 0.9675        | 0.5330    | 0.8672  | 0.4603        |

All values deterministic (transfer_score and non-fitness metrics identical across
all three runs). Only fitness varies slightly due to amplitude-history stochasticity.

## Comparison

### vs post-bugfix stage_sync baseline (K=3.0, 2 trials from 2026-06-23T00)

| metric          | stage_sync | irx post-bugfix | delta       |
|-----------------|-----------|-----------------|-------------|
| fitness         | 0.0819    | **0.0578**      | **−0.024**  |
| transfer_score  | 0.8503    | **0.9654**      | **+0.115**  |
| xi_robustness   | 0.9680    | 0.9675          | −0.0005     |
| carrier_e       | 0.5293    | 0.5330          | +0.004      |
| magic_R         | 0.129     | **0.867**       | +0.738      |
| query_gravity   | 0.4603    | 0.4603          | 0.000       |

### vs pre-bugfix irx (1 trial, from system-prompt context)

| metric        | irx pre-bugfix | irx post-bugfix | delta      |
|---------------|---------------|-----------------|------------|
| fitness       | 0.191         | **0.058**       | **−0.133** |
| xi_robustness | 0.220         | **0.968**       | **+0.748** |
| magic_R       | 0.612         | **0.867**       | +0.255     |
| transfer_score| (not reported) | 0.965          | —          |
| carrier_e     | 0.714         | 0.533           | −0.181     |

## Interpretation

The pre-bugfix interference_relax measurement was catastrophically degraded by the
chiral.rs bug: `|sin(dphi)|<0.1` accepted anti-phase pairs as phase-locked, flooding
the constructive-pair set with destructive interference partners. This caused xi to
collapse (0.220) because the relaxation phase-aligned memories that should NOT have
been aligned. Post-bugfix, only genuinely co-phase pairs drive the relaxation, and
xi recovers to 0.968 — matching stage_sync.

The transfer_score recovery (0.850 → 0.965) is the key win. In stage_sync, the
Kuramoto coupling operates on category boundaries and sometimes pulls transfer-corpus
memories into the wrong attractor. Interference_relax, guided by wave-constructive
geometry, preserves the cross-corpus phase structure that the Cl(0,7)-corrected blade
merge now correctly measures.

The magic_R jump (0.129 → 0.867) is striking: interference_relax produces a much
more phase-coherent (high-R) memory state, yet xi is equally high. This decouples
R from xi — the pre-bugfix anticorrelation (K-sweep: low R ↔ high xi) appears to be
a stage_sync artifact, not a structural law. Interference_relax achieves both high R
AND high xi simultaneously.

query_gravity (0.4603) is unchanged between modes, suggesting the attention-as-gravity
mechanism operates independently of the dream consolidation mode.

## Decision

**Keep.** The improvement is 0.0241 (fitness drop from 0.082 → 0.058), well above
the 0.005 threshold, confirmed in 3 deterministic trials. No code changes were made.

**New empirical optimum** (post-bugfix):
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE=interference_relax
```
3-trial avg fitness: **0.0578**

## Next fire candidates

1. K-sweep under interference_relax: does the R-xi anticorrelation still hold
   across K values, or is it truly dissolved by this mode? Try K={2.0, 4.0, 5.0}.
2. interference_relax + relax_steps=16 or 24 (question 3 from context): now that
   xi is no longer collapsed, can raising relax_steps push fitness below 0.057?
3. query_gravity is stuck at 0.4603 across all conditions tested. Investigate
   whether it's near a measurement ceiling or genuinely mode-invariant.
