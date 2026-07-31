# 2026-07-31T00 — B-engine diagnostic confirms structural floor; chiral_p=0.0 falsified

## Context

Confirmed operating point (three ephemeral code changes required per fire):
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_GRAVITY=0.35
KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3 xi_eval_params.kuramoto_coupling=1.0
```
Container baseline this fire: fitness 0.018469 (trial 1), consistent with Jul 29/30 floors (0.018437–0.018476).

## Jul 29–30 recommendation acted on

Both prior notes recommended: "Reading eval_chain_fidelity and the B_primed vs B_naive
centroid trajectories more carefully (adding per-cycle phi_history to B engines) could
reveal whether there's a principled way to improve chain_fidelity for B_primed without
harming the B_naive comparison baseline."

This fire executed that diagnostic.

## Trial 1 — B-engine component diagnostic

Added per-component diagnostic prints after the B engine fitness computations:
- `eval_chain_fidelity` for both chain_seeds_bp and chain_seeds_bn
- `eval_consciousness` on both B engines
- `eval_phase_coherence_l4` on both B engines

**Results (all three ephemeral changes applied, DREAM_MODE unset, standard mode):**

| component             | B_primed   | B_naive    | difference |
|-----------------------|------------|------------|------------|
| chain_fidelity        | 1.000000   | 0.797205   | +0.202795  |
| consciousness         | 0.988809   | 0.950559   | +0.038250  |
| phase_coherence_l4    | 0.940696   | 0.000000   | +0.940696  |
| fitness (total)       | 0.003686   | 0.080136   | —          |

Estimated per-component fitness contributions (weights 0.05/0.10/0.05/0.10):
| component             | fitness_B_primed | fitness_B_naive | B_naive excess |
|-----------------------|-----------------|-----------------|----------------|
| phase_coherence_l4    | 0.05×0.059=0.00295 | 0.05×1.0=0.050 | 0.047 (59%)   |
| chain_fidelity        | 0.10×0.000=0.000 | 0.10×0.203=0.020 | 0.020 (25%)   |
| consciousness         | 0.10×0.011=0.001 | 0.10×0.049=0.005 | 0.004 (5%)    |
| NR/SP/EE (0.05 each)  | ~0.001           | ~0.005           | 0.004 (5%)    |
| remainder             | ~0.001           | ~0.001           | 0              |
| **total**             | **0.003686**     | **0.080136**     | **0.047 dom.** |

## Key finding: phase_coherence_B_naive is structurally zero

`eval_phase_coherence_l4` filters for memories whose content starts with
`["dense_a", "dense_b", "dense_c", "dense_d", "sparse_e", "sparse_f"]` — the Corpus A
cluster prefixes. B_naive contains NO Corpus A memories (only B corpus: `l4_dense`,
`l4_sparse`, `l4_bridge`, etc.), so `cluster_count = 0 → return 0.0` always.

This is a **structural constant**, not a parameter-tunable metric:
- fitness_B_naive contribution from phase_coherence: `0.05 × 1.0 = 0.050` (permanent)
- fitness_B_primed contribution: `0.05 × (1-0.941) ≈ 0.003` (from A's clusters in B_primed)

The transfer mechanism now has a clear picture:
- B_naive is *permanently* penalised 0.050 for lacking A's cluster prefixes
- B_primed has A's clusters present → phase_coherence_B_primed = 0.941
- The 0.050 structural gap drives ~62% of the B_naive/B_primed fitness ratio

Prior notes attributed the transfer ceiling to chain_fidelity. **This was incorrect.**
chain_fidelity_B_naive = 0.797 (not ~0.2 as estimated). The real driver is phase_coherence.

## Hypothesis from diagnostic — chiral_p=0.0 for B_primed

If phase_coherence_B_primed comes from A's cluster phases maintained during B_primed's
dream, could reducing chiral_perturbation (currently 0.15 for B_primed) toward 0.0 allow
A's clusters to maintain even higher phase coherence?

**Mechanism assumed**: lower chiral → less phase perturbation applied to A's cluster
members → higher within-cluster phase order parameter → phase_coherence_B_primed → 1.0.

**Prediction**: fitness_B_primed drops from 0.003686 to ~0.001, transfer → 0.985+.

**Jun 11T04 data** (irx mode): chiral_p=0.00 was catastrophic there (fp=0.023661 vs
fp=0.003887 baseline). But that catastrophe was attributed to irx-specific mechanics. The
current DREAM_MODE=unset (standard stage_sync) regime might behave differently.

## Trial 2 — chiral_p=0.0 for B_primed (FALSIFIED)

**Code change:** `p.chiral_perturbation = 0.0` (vs 0.15) in `params_bp` block.

| metric             | trial 2    | trial 1 (baseline) | delta       |
|--------------------|------------|--------------------|-------------|
| fitness            | 0.019953   | 0.018469           | +0.001484   |
| transfer_score     | 0.943977   | 0.954003           | −0.010026   |
| fitness_B_primed   | 0.004489   | 0.003686           | +0.000803   |
| fitness_B_naive    | 0.080131   | 0.080136           | ~0 (noise)  |
| xi_robustness_v2   | 0.9678     | 0.9678             | 0           |
| carrier_emergence  | 1.0000     | 1.0000             | 0           |
| consciousness      | 0.8830     | 0.8830             | 0           |
| phase_coherence    | 0.8939     | 0.8939             | 0           |
| magic_proxy_phase_R| 0.6082     | 0.6082             | 0           |

**Hypothesis FALSIFIED.** fitness_B_primed rose (worse) and transfer regressed. Chiral
perturbation reduction in the current regime HURTS B_primed, not helps.

## Root cause analysis — why the hypothesis was wrong

The assumed mechanism was backwards for the standard dream mode:

In standard mode (stage_sync), chiral perturbation (Stage 9) applies handedness-based
phase twists to each memory based on its cluster membership. For B_primed's dream:
- A's cluster members receive chiral twists that REINFORCE their cluster-specific phases
- Without chiral (eta=0.0), the chiral stage is skipped entirely
- Kuramoto coupling then pulls A's cluster members toward B's foreign phases
- This REDUCES within-cluster phase coherence of A's memories
- phase_coherence_B_primed DROPS (not measured this trial, but inferred from fp rise)

With eta=0.15, chiral acts as a phase anchor for A's clusters during B's disruptive
influence. It's the mechanism that MAINTAINS phase_coherence_B_primed at 0.941.

Contrast with irx mode (Jun 11T04): the interference_relax algorithm does its own phase
organisation; lower chiral meant less disruption of that organisation. Different algorithm
→ opposite directional effect of chiral.

The Jun irx minimum at eta=0.10 does not transfer to the current standard mode.

## Updated structural picture

**phase_coherence_B_naive = 0.0 is a permanent structural constant.**

The 0.050 phase_coherence penalty on B_naive cannot be removed by any parameter sweep —
it reflects that B_naive has no Corpus A memories. This is a designed advantage embedded
in the transfer metric.

**chiral_perturbation=0.15 for B_primed is confirmed optimal** in the current regime.
Lower values (including 0.0) worsen fitness_B_primed.

**All known levers for transfer confirmed exhausted (Jul 30 list, now extended):**
- K sweep (1.0–5.0): no improvement
- DREAM_GRAVITY (0.25–0.40): no improvement
- chiral_b_primed (0.05–0.15, now 0.0): 0.15 is optimal; lower = worse
- CHAIN_TOP_N (5–10): no improvement
- xi_flat_bprimed: no improvement
- DRIVE_FREQ_HZ (0.25–1.0): no improvement
- B_primed chain_depth (3 vs 4): depth=4 optimal
- B memory phase alignment: falsified (Jul 29)
- kuramoto_steps=100: catastrophic (Jul 20)
- interference_relax modes: no improvement
- consolidation_repulsion_threshold: structural equilibrium (Jul 30)
- chiral_p=0.0 for B_primed: **falsified (this fire)**

## Decision

Both code changes reverted immediately after trial 2. No code kept. TSV rows record
both trials. Hypothesis about phase_coherence mechanism was falsified.

## New structural picture of transfer

The transfer_score = 1 - fitness_B_primed/fitness_B_naive is governed by:
1. phase_coherence_B_naive = 0 (permanent; 62% of transfer gap)
2. chain_fidelity_B_naive = 0.797 (tunable? All levers exhausted)
3. consciousness asymmetry (B_primed phi closer to target; small)

To get transfer from 0.954 to 0.987 (enough for 0.005 fitness improvement), would need
fitness_B_primed to drop from 0.003686 to ~0.001 — requiring phase_coherence_B_primed
to improve from 0.941 to ~0.998. No mechanism found for this.

**The structural floor (~0.018) is confirmed.** No parameter-tunable improvements remain.

## Next fire recommendations

**The system is genuinely at its structural floor in this optimization regime.**

1. **Exhausted all known levers**: see updated list above.
2. **phi_target decoupling**: saves 0.003510 (consciousness) but needs +0.001490 bundled.
   No bundling candidates have been found.
3. **Architectural changes** (require scope beyond autoresearch):
   - eval_phase_coherence_l4 currently uses hardcoded A-cluster prefixes. If B-corpus
     content were also tracked with coherence-measurable prefixes, the structural zero
     would be avoided — but this changes the metric semantics.
   - B_naive isolation from B_primed's FP trajectory.
   - Per-engine phi_target calibration.

If no architectural changes are planned, future fires should note "structural floor
confirmed, no new levers" rather than repeating exhausted sweeps.

## TSV rows appended (2 total)

- Trial 1: diagnostic baseline, fitness 0.018469 (container characterisation + component breakdown)
- Trial 2: chiral_p=0.0 for B_primed, fitness 0.019953, transfer 0.943977 (FALSIFIED)
