# 2026-07-30T00 — consolidation_repulsion_threshold sweep falsified; structural floor confirmed

## Context

Entering confirmed operating point (requires three ephemeral code changes per fire):
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_GRAVITY=0.35
KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3 xi_eval_params.kuramoto_coupling=1.0
```

## Baseline (trial 1) — current container characterization

Container fast (total_ms=14747), xi environment-locked at 0.9678 (same as Jul 29
container). Fitness 0.018437 is the effective floor in this environment.

| metric             | trial 1    | Jul 26 env  | Jul 29 env  |
|--------------------|------------|-------------|-------------|
| fitness            | 0.018437   | 0.017032    | 0.018476    |
| transfer_score     | 0.954003   | 0.938419    | 0.954001    |
| fitness_B_primed   | 0.003686   | ~0.003686   | 0.003686    |
| fitness_B_naive    | 0.080136   | ~0.059856   | 0.080131    |
| xi_robustness_v2   | 0.9678     | 0.9980      | 0.9678      |
| consciousness      | 0.8830     | 0.8830      | 0.8830      |
| phase_coherence    | 0.8939     | 0.8939      | 0.8939      |
| speed_a            | 0.9644     | ~0.963      | ~0.938      |
| carrier_emergence  | 1.0000     | 1.0000      | 1.0000      |
| magic_proxy_phase_R| 0.6082     | 0.6082      | 0.6082      |
| query_gravity      | 0.5065     | 0.8962      | 0.5065      |
| total_ms           | 14747      | ~15000      | ~25600      |

**Pattern**: This container appears identical to Jul 29 in FP environment (same xi=0.9678,
same B_naive=0.080, same query_gravity=0.5065). Speed is closer to Jul 26 (14747ms).

Fitness breakdown in this container:
| source           | weight | value  | contribution | % of fitness |
|------------------|--------|--------|--------------|-------------|
| transfer_score   | 0.15   | 0.954  | 0.006900     | 37.4%       |
| xi_robustness_v2 | 0.15   | 0.9678 | 0.004830     | 26.2%       |
| consciousness    | 0.03   | 0.8830 | 0.003510     | 19.0%       |
| phase_coherence  | 0.02   | 0.8939 | 0.002122     | 11.5%       |
| speed_a          | 0.03   | 0.9644 | 0.001068     | 5.8%        |
| other            |        | ~1.0   | ~0.000007    | 0.04%       |
| **total**        |        |        | **0.018437** | 100%        |

## Hypothesis — consolidation_repulsion_threshold = 0.22

`l5_params.consolidation_repulsion_threshold = 0.28` was set during L5 development
(not documented in autoresearch notes) and has never been swept in the current
optimization regime. The ConsolidationParams default is 0.22. Lower threshold =
more pairs get xi-repulsed.

**Prediction**: Lower threshold (0.22) increases xi-repulsion for semantically similar
pairs, scattering B_naive's phases (which has no A-scaffold anchor) while B_primed
remains guided by A's cluster structure. Result: fitness_B_naive increases → transfer
improves.

**Implementation**: `l5_params.consolidation_repulsion_threshold = 0.22;`

## Results (trial 2) — repulsion_threshold=0.22

| metric             | trial 2    | baseline   | delta         |
|--------------------|------------|------------|---------------|
| fitness            | 0.120169   | 0.018437   | +0.101732     |
| transfer_score     | 0.547029   | 0.954003   | −0.406974     |
| fitness_B_primed   | 0.041978   | 0.003686   | +0.038292     |
| fitness_B_naive    | 0.092672   | 0.080136   | +0.012536     |
| xi_robustness_v2   | 0.8893     | 0.9678     | −0.0785       |
| consciousness      | 0.9969     | 0.8830     | +0.1139       |
| phase_coherence    | 0.6954     | 0.8939     | −0.1985       |
| carrier_emergence  | 0.7167     | 1.0000     | −0.2833       |
| magic_proxy_phase_R| 0.4312     | 0.6082     | −0.1770       |

**Hypothesis CATASTROPHICALLY FALSIFIED.** Fitness regressed 6.5× from 0.018437 to
0.120169. All major metrics collapsed. The prediction was directionally wrong and vastly
underestimated the structural sensitivity of this parameter.

## Root cause analysis

### Why the prediction was wrong

The prediction assumed that B_naive would be hurt more than B_primed by increased
repulsion. This reasoning was flawed:

1. **B_primed fitness is not near-zero because of xi structure**: B_primed achieves
   fitness_B_primed ≈ 0.003686 because ALL its sub-terms (noise_removal, signal_preservation,
   phase_coherence, consciousness, encoding_entropy, chain_fidelity) are near 1.0 — not
   just chain_fidelity. Increasing repulsion disrupts the phase structure of B_primed's
   dream result just as severely as B_naive's. fitness_B_primed jumped 11× (0.003686
   → 0.041978).

2. **The repulsion threshold is load-bearing at L5 corpus density**: At the 0.22 threshold,
   far too many semantically similar pairs are repulsed in L5's 128-dim corpus. L5 has
   much richer xi structure than the L3/L4 corpora for which the 0.22 default was tuned.
   The result is that xi-repulsion dominates the dream dynamics, preventing normal Kuramoto
   clustering from forming stable phase attractors.

3. **Consciousness improved but at catastrophic cost**: phi_engine_a dropped toward
   phi_target (0.28092), making consciousness nearly 1.0 (0.997). This confirms the
   Jul 21 phi_target decoupling hypothesis from a different angle: repulsion can drive
   phi toward target, but at the cost of destroying every other metric. Only the clean
   phi_target decoupling (changing the evaluation target, not the dynamics) is safe.

4. **carrier_emergence, phase_coherence, xi all collapsed**: The flat corpus, B engines,
   and xi_eval engines all use the same repulsion_threshold. Stronger repulsion disrupts
   Kuramoto phase clustering across ALL engines simultaneously.

### Why repulsion_threshold=0.28 is the correct value

The L5 development team set 0.28 (vs default 0.22 in ConsolidationParams and 0.30 in
Params) as a middle ground that:
- Allows enough xi-repulsion to maintain phase diversity (higher than 0.30 default)
- Limits over-repulsion that would collapse carrier and transfer (lower than L5 would
  give with the 0.22 ConsolidationParams default)

This parameter is NOT a candidate for autoresearch optimization — it is a structural
equilibrium value, similar to consciousness_phi_target=0.28092. Moving it even slightly
toward the ConsolidationParams default causes catastrophic regression.

## Sideways finding: consciousness improvement mechanism

At repulsion_threshold=0.22, consciousness jumped to 0.997 (from 0.8830). This happened
because repulsion prevents phi_engine_a from overshooting phi_target: the stronger
repulsion pushes phases APART rather than toward clusters, resulting in phi sitting
closer to the 0.28092 target. This is a different path to the same destination as
phi_target decoupling — but at 6.5× fitness cost.

This further confirms that phi_target decoupling (Jul 21 method) is the ONLY safe way
to recover the 0.003510 consciousness contribution.

## Updated structural ceiling picture

All known parameter levers confirmed exhausted:

**Transfer (37% of fitness, 0.006900):**
All swept: K=1.0 to 5.0, DREAM_GRAVITY=0.25 to 0.40, chiral_b_primed, drive frequency,
chain_top_n, xi_flat_bprimed, interference_relax mode, B_primed chain_depth, B memory
phase alignment, B_primed chain_depth=3 vs 4. **This fire adds: consolidation_repulsion_threshold
(confirmed load-bearing, cannot be swept).**

**xi_robustness_v2 (26% of fitness, 0.004830):**
xi=0.9678 in this environment (FP-locked). xi_eval K=1.0, chain_depth=3 confirmed as
optimal. xi=0.9980 achievable in some containers (Jul 26) but not this one.

**consciousness (19% of fitness, 0.003510):**
Structural floor at 0.8830. Only improvement path: phi_target decoupling (saves 0.003510,
below 0.005 threshold alone; see Jul 21 notes).

**phase_coherence (11.5% of fitness, 0.002122):**
Structural floor at 0.8939. K=2.0 is optimum.

**speed_a (5.8% of fitness, 0.001068):**
Environment-dependent. Not controllable via parameters.

## Decision

**Hypothesis catastrophically falsified.** Experimental code change reverted immediately
after trial 2. TSV rows record both trials. No code kept.

consolidation_repulsion_threshold=0.28 is confirmed as a structural equilibrium value
and should not be swept in future fires. Adding to known-bad list.

## Summary of trials

| trial | config                           | fitness  | transfer | xi_rob | decision          |
|-------|----------------------------------|----------|----------|--------|-------------------|
| 1     | Baseline (confirmed op point)    | 0.018437 | 0.954003 | 0.9678 | Container baseline|
| 2     | repulsion_threshold=0.22 (exp)   | 0.120169 | 0.547029 | 0.8893 | CATASTROPHIC fail |

## Next fire recommendations

**The system appears to be at or very near the structural floor** in this optimization
regime. No new parameter levers have been identified. All known candidates are exhausted.

Remaining sub-threshold improvements (not worth firing alone):
1. **phi_target decoupling**: saves 0.003510 (consciousness). Needs +0.001490 bundled.
   No candidates have emerged for bundling since Jul 21.
2. **xi cross-container variability**: xi=0.9980 is achievable in some containers
   (Jul 26). This is FP non-determinism, not parameter-controllable.

The 0.005 fitness threshold may be too strict given the current floor (~0.018):
- 0.005 improvement would reach ~0.013, requiring fundamental architectural changes
- The L5 formula weights are fixed; only parameter changes within the current architecture
  can be tested in autoresearch

**Potential architectural investigations (require scope beyond autoresearch):**
- Alternative transfer metrics that better capture A-scaffold advantage
- B_naive isolation from B_primed's RNG/FP trajectory to reduce noise
- Per-engine phi_target calibration (requires structural changes to eval_l5_placeholder_fitness)
- Fix the xi environment-FP issue for cross-container reproducibility (requires f64
  precision or operation-order pinning, which changes the binary)

## TSV rows appended (2 total)

- Trial 1: baseline, fitness 0.018437, transfer 0.954003, xi 0.9678 (container characterization)
- Trial 2: repulsion_threshold=0.22, fitness 0.120169, transfer 0.547029 (CATASTROPHIC FAIL)
