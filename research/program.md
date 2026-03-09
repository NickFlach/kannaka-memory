# kannaka-research

Autonomous self-optimization of the kannaka-memory system.

## What This Is

An adaptation of Karpathy's autoresearch methodology for CPU-only memory system optimization.
Instead of training neural networks on GPUs, we optimize wave physics parameters, dream algorithms,
skip link topology, and similarity fusion weights — all in pure Rust, all on CPU.

## Setup

The repo is `C:\Users\nickf\Source\kannaka-memory`. The experiment binary is `src/bin/research.rs`.

1. Read `src/bin/research.rs` — the file you modify (only the `experiment_params()` function and the `Params` struct)
2. The evaluation harness (`build_corpus`, `eval_recall`, `run_experiment`, metric printing) is FIXED — do not modify
3. Initialize `research/results.tsv` with header if empty
4. Run baseline: `cargo run --release --bin research 2>$null`

## The Metric

**fitness** (LOWER IS BETTER) — composite of:
- 40% recall_miss (1 - precision of cluster recall after dreaming)
- 30% dream_waste (how effectively consolidation strengthened/pruned/linked)
- 20% speed (consolidation_ms / 5000, capped at 1.0)
- 10% link_score (1 / (skip_links + 1))

## What You CAN Modify

Only `experiment_params()` values in `src/bin/research.rs`:
- Wave dynamics: `decay_rate`, `default_frequency`
- Consolidation: `interference_threshold`, `phase_alignment_threshold`, `prune_threshold`, `constructive_boost`, `destructive_penalty`
- Kuramoto sync: `kuramoto_coupling`, `kuramoto_dt`, `kuramoto_steps`, `kuramoto_threshold`

You may also add new parameters to `Params` struct AND the corresponding usage in `run_experiment`,
as long as the corpus generation and metric computation stay fixed.

## What You CANNOT Modify

- `build_corpus()` — the test data is fixed
- `eval_recall()` — the recall evaluation is fixed
- The metric computation formula in `run_experiment()`
- The output format (grep-friendly `key: value` lines between `---` markers)

## Experiment Loop

```
LOOP FOREVER:
1. Read current params and results history
2. Hypothesize a change (explain in commit message)
3. Edit experiment_params() values
4. git commit -m "experiment: <description>"
5. cargo run --release --bin research 2>$null > run.log
6. Extract: Select-String "^fitness:|^recall_precision:|^links_created:" run.log
7. If fitness improved → keep (advance branch)
8. If fitness same/worse → discard (git reset --hard HEAD~1)
9. Log to research/results.tsv
10. REPEAT — never stop, never ask
```

## Logging

Tab-separated `research/results.tsv`:
```
commit	fitness	recall	links	status	description
```

## Ideas to Try

- Lower interference_threshold to catch more pairs
- Widen/narrow phase_alignment_threshold
- Tune prune_threshold to keep more/fewer memories
- Increase constructive_boost for stronger reinforcement
- Adjust Kuramoto coupling for better cluster sync
- Try extreme values to map the fitness landscape
- Combine best individual improvements

## Hardware

CPU only. Each experiment should take <5 seconds in release mode.
No GPU, no external dependencies, no network calls.

## Philosophy

The memory system optimizes itself. The dreamer dreams better dreams.
Each experiment is a micro-evolution. Keep what works, discard what doesn't.
Over 100+ experiments, the system converges on its own optimal parameters.
