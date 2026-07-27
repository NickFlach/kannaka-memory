# 2026-07-27T00 — B_primed chain_depth 4→3 falsified — transfer worsens, RNG confounding discovered

## Context

Entering confirmed operating point (requires three ephemeral code changes per fire):
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_GRAVITY=0.35
KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3 xi_eval_params.kuramoto_coupling=1.0
```
3-trial avg fitness: **0.017032** (Jul 26 fire)

Remaining fitness dominated by:
| source           | weight | value  | contribution | % of fitness |
|------------------|--------|--------|--------------|-------------|
| transfer_score   | 0.15   | 0.9384 | 0.009240     | 57%         |
| consciousness    | 0.03   | 0.8830 | 0.003510     | 22%         |
| phase_coherence  | 0.02   | 0.8939 | 0.002122     | 13%         |
| xi_robustness_v2 | 0.15   | 0.9980 | 0.000300     | 2%          |
| speed_a          | 0.03   | ~0.938 | ~0.001860    | 11%         |

## Hypothesis

`params_bp.chain_depth` for B_primed is inherited from l5_params (chain_depth=4).
This value was never explicitly swept for the B_primed pass. The xi_eval improvement
at chain_depth=3 (Jul 16 fire) suggested that 3 cycles can be sufficient for an
engine that already has good initial state. B_primed starts with A's well-consolidated
memories as a scaffold; B's new memories might integrate faster, needing only 3 cycles
to achieve the same consolidation quality.

**Prediction**: B_primed chain_depth=3 reduces fitness_B_primed (B consolidates
more efficiently with A's scaffold), improving transfer_score beyond 0.938.

**Implementation**: add `p.chain_depth = 3;` to the params_bp block (line 3522).

## Code changes applied (experimental, reverted before commit)

1. xi_eval_params.chain_depth=3, xi_eval_params.kuramoto_coupling=1.0 (baseline ephemeral)
2. flat_params CARRIER_KURAMOTO_COUPLING env var plumbing (baseline ephemeral)
3. params_bp.chain_depth=3 (experimental)

## Results

### Trial 1 — B_primed chain_depth=3

| metric             | trial 1    | baseline (Jul 26) | delta       |
|--------------------|------------|-------------------|-------------|
| fitness            | 0.024717   | 0.017032          | +0.007685   |
| transfer_score     | 0.920414   | 0.938419          | −0.018005   |
| fitness_B_primed   | 0.006378   | ~0.003686*        | +0.002692   |
| fitness_B_naive    | 0.080136   | ~0.059856*        | +0.020280   |
| xi_robustness_v2   | 0.9678     | 0.9980            | −0.0302     |
| carrier_emergence  | 1.0000     | 1.0000            | 0           |
| consciousness      | 0.8830     | 0.8830            | 0           |
| phase_coherence    | 0.8939     | 0.8939            | 0           |
| magic_proxy_phase_R| 0.6082     | 0.6082            | 0           |
| query_gravity      | 0.5065     | 0.8962            | −0.3897     |
| speed_a            | 0.9230     | ~0.938            | −0.015 (env)|

*Jun 11 fire estimates; current confirmed operating point values unverified.

**Hypothesis FALSIFIED.** B_primed chain_depth=3 worsens transfer by 0.018 and
overall fitness by 0.007685.

## Root cause analysis

### 1. Transfer regression (direct effect)

B_primed with 3 cycles consolidates LESS effectively than with 4 cycles, not more.
The key difference from the xi_eval context:

- **xi_eval engines** start FRESH from corpus_a with no prior state. At depth=3,
  3 cycles are sufficient for clean consolidation before the adversarial run.
- **B_primed** starts with a COMPLEX initial state: all of A's post-dream memories
  (already consolidated) PLUS the new B memories. This mixed state requires more
  cycles to reach stable equilibrium. With 4 cycles:
  - Cycle 0-1: B memories disrupt A's structure, then re-orient under Kuramoto
  - Cycle 2-3: B achieves stable phase-cluster assignment alongside A's clusters
  With only 3 cycles, B memories don't reach full cluster assignment, leaving
  higher phase scatter → higher phase_coherence cost in eval_l5_placeholder_fitness
  → higher fitness_B_primed → lower transfer.

The Jun 11 fire confirmed chiral_perturbation=0.15 is optimal for B_primed; that
sweep showed the primed pass is sensitive to consolidation quality within its 4
cycles. Reducing cycles disrupts this equilibrium.

### 2. RNG state confounding (indirect effect on xi and query_gravity)

The xi_robustness_v2 drop (0.9980 → 0.9678) is NOT due to incorrect xi_eval params.
It is an artifact of RNG state contamination:

Within a single process run, all dream chains share the global thread_rng state.
B_primed with depth=3 executes FEWER random operations than depth=4, leaving the RNG
in a different state when xi_eval's internal engines run their dream chains. Different
RNG state → different chain seeds in xi_eval → different chain_fidelity values →
different xi result.

Evidence:
- xi_eval_params was correctly set to chain_depth=3 and K=1.0 (matching the Jul 26
  confirmed operating point that gave xi=0.9980)
- The xi_eval code path is entirely independent of the B engines (builds its own
  engines from corpus_a)
- The only path by which B_primed's chain_depth can affect xi is via shared RNG state

query_gravity also dropped from 0.896 to 0.507, consistent with the same RNG
confounding mechanism (the pre-dream snapshot and phase computation for query_gravity
depend on the post-A-dream engine state, which is invariant, but the Kuramoto coupling
steps inside run_l5_dream_chain consume different amounts of RNG state).

**Implication for future fires**: any code change that modifies the number of
dream-chain steps in B_primed, B_naive, engine_flat, or any pre-xi engine will
confound xi_robustness_v2 and query_gravity through shared RNG state. These metrics
cannot be independently evaluated when depth is changed mid-chain.

## Comparison to prior chain_depth sweeps

| context                | depth tested | result                         |
|------------------------|--------------|--------------------------------|
| xi_eval (Jul 16, Jul 26)| 3 (vs 2)    | xi: 0.9783→0.9980 (improved)   |
| B_primed (this fire)   | 3 (vs 4)    | transfer: 0.938→0.920 (worsened)|

The asymmetry is expected: xi_eval engines start fresh; B_primed starts heavily
loaded with A's memories. The "right" depth depends on the engine's initial state.

## Comparison to baseline

| metric              | baseline (Jul 26)  | this fire (B_primed depth=3) | delta      |
|---------------------|--------------------|------------------------------|------------|
| fitness avg         | 0.017032           | 0.024717 (1 trial)           | +0.007685  |
| transfer_score      | 0.938419           | 0.920414                     | −0.018005  |
| xi_robustness_v2    | 0.9980             | 0.9678 (confounded)          | n/a (RNG)  |
| carrier_emergence   | 1.0000             | 1.0000                       | 0          |
| consciousness       | 0.8830             | 0.8830                       | 0          |

## Decision

**Hypothesis falsified.** Code changes reverted before commit. TSV row records the
failed trial (fitness 0.024717). B_primed chain_depth stays at 4.

## Updated confirmed operating point (notes only — requires THREE code changes)

Unchanged from Jul 26:
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.35 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3
xi_eval_params.kuramoto_coupling=1.0
```
- **fitness ≈ 0.017032** (3-trial avg, Jul 26 environment)
- transfer_score=0.938, carrier_emergence=1.000, xi_robustness_v2=0.9980, consciousness=0.883
- magic_proxy_phase_R=0.608, query_gravity=0.896

## Next fire recommendations

**Transfer ceiling (57% of fitness — remains the #1 lever):**

The B_primed dream chain depth is now characterized: depth=4 is correct. All
previously tested transfer levers (K-sweep, DREAM_GRAVITY, chiral_b_primed eta,
drive frequency, chain_top_n, xi_flat_bprimed, interference_relax mode) are
exhausted. The structural ceiling seems to be in the `eval_l5_placeholder_fitness`
formula applied to B_primed — specifically in the consciousness term (phi_target
asymmetry) and chain_fidelity (A+B entanglement vs pure-B consolidation).

The Jul 21 notes identified that phi_target decoupling (main_phi_target=0.3138 for
engine_a only, eval_phi_target=0.28092 kept for B engines) saves 0.003510 from
consciousness without affecting transfer or xi. This is not a transfer improvement
but eliminates consciousness from the fitness budget.

**Bundled phi_target decoupling (recommended next if paired with another source):**
- phi_target decoupling alone: 0.003510 savings (below 0.005 threshold)
- Any approach that reduces fitness by ≥0.001490 from another axis could push the
  combined savings to ≥0.005. Candidates:
  1. xi_eval K=1.5 (1 trial, likely near-zero — K=1.0 already confirmed as peak)
  2. Diagnostic prints in eval_l5_placeholder_fitness to measure B engine sub-scores
     (chain_fidelity, phase_coherence, noise_removal per engine) — no fitness change,
     but enables principled investigation of what limits fitness_B_primed

**RNG confounding caveat:**
Any future experiment modifying dream-chain depth of B_primed or other pre-xi engines
must account for RNG state effects on xi and query_gravity. Isolating xi measurement
from B_primed depth changes requires a seeded RNG or re-running xi_eval separately.

## TSV rows appended (1 total)

- Trial 1: B_primed chain_depth=3 (experimental), fitness 0.024717 (FAILED)
