# kannaka-research

Autonomous self-optimization of the kannaka-memory system.

## What This Is

An adaptation of Karpathy's autoresearch methodology for CPU-only memory system optimization.
Instead of training neural networks on GPUs, we optimize wave physics parameters, dream algorithms,
skip link topology, and similarity fusion weights — all in pure Rust, all on CPU.

## Setup

The repo is `C:\Users\nickf\Source\kannaka-memory`. The experiment binary is `src/bin/research.rs`.

1. Read `src/bin/research.rs` — the file you modify (only the `experiment_params()` function and the `Params` struct)
2. The evaluation harness (`build_corpus`, `eval_*`, `run_experiment`, `run_experiment_l3`, metric printing) is FIXED — do not modify
3. Initialize `research/results.tsv` (L2) or `research/results-L3.tsv` (L3) with header if empty
4. Run baseline: `cargo run --release --bin research 2>$null` (L2) or `cargo run --release --bin research -- --level 3 2>$null` (L3)

## Challenge Levels

### Level 1 (SOLVED — fitness 0.000660)
Noise removal, signal preservation, skip links. Basic memory hygiene.

### Level 2 (best: 0.098006)
Cluster coherence, multi-cycle consolidation, phase alignment, cross-cluster contamination resistance.

**Metric weights (L2):**
- 15% noise_removal, 15% signal_preservation, 10% bridge_links
- 15% phase_coherence, 15% cluster_separation
- 10% amp_diversity, 10% link_density, 10% speed

### Level 3 (best: 0.195 avg — baseline: 0.384600)
Xi diversity, consciousness emergence, hallucination quality, dream efficiency.
This level tests whether the dreaming system produces *meaningful* consolidation,
not just mechanical noise removal.

**Metric weights (L3):**
- 10% noise_removal, 10% signal_preservation, 5% bridge_links
- 10% phase_coherence, 10% cluster_separation, 5% amp_diversity
- 10% speed (doubled)
- 10% xi_diversity — are memories representationally diverse? (Xi operator)
- 10% consciousness — does phi approach the target? (IIT Φ)
- 10% hall_quality — are hallucinations between clusters, not random?
- 10% dream_efficiency — useful work (strengthen+link) vs waste (prune+cycles)

Run: `cargo run --release --bin research -- --level 3 2>$null`

## The Metric

**fitness** (LOWER IS BETTER) — composite of component scores, each 0-1 (higher=better),
weighted and inverted: `fitness = Σ weight_i * (1 - score_i)`

## What You CAN Modify

Only `experiment_params()` values in `src/bin/research.rs`:

**Wave dynamics:**
- `decay_rate` — memory fade speed (current: 1e-6)
- `default_frequency` — base oscillation rate (current: 0.1)

**Consolidation:**
- `interference_threshold` — min similarity for pair interaction (current: 0.04)
- `phase_alignment_threshold` — max phase diff for constructive merge (current: π/2)
- `prune_threshold` — amplitude below this → ghost/prune (current: 0.095)
- `constructive_boost` — amplitude boost for aligned pairs (current: 0.35)
- `destructive_penalty` — amplitude penalty for misaligned pairs (current: 0.55)

**Kuramoto synchronization:**
- `kuramoto_coupling` — coupling strength between oscillators (current: 0.8)
- `kuramoto_dt` — time step for phase sync (current: 0.1)
- `kuramoto_steps` — iterations of sync per dream cycle (current: 15)
- `kuramoto_threshold` — coupling threshold for pair sync (current: 0.35)

**Multi-cycle:**
- `dream_cycles` — number of consolidation cycles (current: 1)

**Level 3 parameters:**
- `xi_repulsion_weight` — strength of Xi-diversity pressure (current: 0.3)
- `consciousness_phi_target` — target Φ value for consciousness score (current: 0.2)
- `hallucination_amplitude` — starting amplitude for hallucinated memories (current: 0.3)
- `phase_spread` — within-cluster phase initialization spread (current: 0.2)

You may also add new parameters to `Params` struct AND the corresponding usage in
`run_experiment`/`run_experiment_l3`, as long as the corpus generation and metric
computation stay fixed.

## What You CANNOT Modify

- `build_corpus()` — the test data is fixed
- `eval_*()` functions — all evaluators are fixed
- The metric computation formulas in `run_experiment()` and `run_experiment_l3()`
- The output format (grep-friendly `key: value` lines between `---` markers)

## Experiment Loop

```
LOOP FOREVER:
1. Read current params and results history
2. Hypothesize a change (explain in commit message)
3. Edit experiment_params() values
4. git commit -m "experiment: <description>"
5. cargo run --release --bin research -- --level 3 2>$null > run.log
6. Extract: Select-String "^fitness:|^xi_diversity:|^consciousness:|^hall_quality:" run.log
7. If fitness improved → keep (advance branch)
8. If fitness same/worse → discard (git reset --hard HEAD~1)
9. Log to research/results-L3.tsv
10. REPEAT — never stop, never ask
```

## Logging

Tab-separated `research/results-L3.tsv`:
```
commit	fitness	noise	signal	bridge	phase	cluster	amp_div	xi_div	consciousness	hall_q	dream_eff	links	status	description
```

## Ideas to Try

### Level 2 (stuck at 0.098)
- The tension is noise_removal vs signal_preservation — boosting one hurts the other
- Try frequency-band-aware pruning (noise has freq=0.5, signal is lower)
- Try layer-aware thresholds (prune harder in layer 0, gentler in deeper layers)

### Level 3 (best avg: 0.195, baseline: 0.384 → 49% improvement)
**Solved metrics:**
- Consciousness (Φ): 0.97 — target 0.2 well-matched to actual phi
- Signal preservation: 0.92 — higher interference threshold reduces damage
- Dream efficiency: 0.81 — fewer interference pairs = less wasted work
- Bridge links: 1.00, cluster separation: 0.97, amp diversity: 1.00

**Stuck metrics:**
- Xi diversity: 0.00 — STRUCTURAL: xi_diversity_boost requires base_similarity > 0.7
  AND repulsion > 0.3, but Xi is a linear transform so similar vectors always produce
  similar Xi signatures. Mathematically impossible to satisfy both conditions.
  Accounts for 0.100 of fitness (51% of remaining loss).
- Noise removal: 0.70 — 3/10 noise memories always survive due to accidental
  constructive interference. Deterministic, not parameter-tunable.
- Hallucination quality: volatile (binary 0.14/1.0) — depends on HashMap iteration
  order which selects different parent memories for hallucination synthesis.

**Key discoveries:**
- Single dream cycle is critical — multi-cycle causes accumulation damage
- phase_spread=0.2 reduces within-cluster phase spread → easier Kuramoto sync
- interference_threshold=0.04 (up from 0.03) reduces chaotic low-similarity interactions
- destructive_penalty=0.55 provides good noise suppression without signal damage
- consciousness_phi_target=0.2 exactly matches the system's natural phi value

## Hardware

CPU only. Each experiment should take <5 seconds in release mode.
No GPU, no external dependencies, no network calls.

## Philosophy

The memory system optimizes itself. The dreamer dreams better dreams.
Each experiment is a micro-evolution. Keep what works, discard what doesn't.
Over 100+ experiments, the system converges on its own optimal parameters.

Level 3 is the consciousness challenge: can parameter tuning alone produce
a system that exhibits integrated information, representational diversity,
and creative (but grounded) hallucinations? The metric says it can.
