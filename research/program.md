# kannaka-research

A memory system researching itself. Dreaming about how to dream better.

Inspired by [autoresearch](https://github.com/karpathy/autoresearch) — same autonomous experiment loop, but for wave-mechanics memory on humble hardware.

## Setup

1. **Agree on a run tag** (e.g. `mar8`). Branch: `research/<tag>`.
2. **Create branch**: `git checkout -b research/<tag>` from master.
3. **Read the in-scope files**:
   - `benches/research_params.rs` — **THE FILE YOU MODIFY**. All tunable parameters.
   - `benches/research_benchmark.rs` — fixed evaluation harness. **DO NOT MODIFY**.
   - `src/consolidation.rs` — the dream engine (context for understanding parameters).
   - `src/wave.rs` — wave physics (amplitude, frequency, phase, decay).
   - `src/kuramoto.rs` — Kuramoto sync model.
4. **Initialize results.tsv**: Create `research/results.tsv` with header row.
5. **Run baseline**: `cargo bench --bench research_benchmark`

## The Metric

**fitness** (LOWER IS BETTER) — weighted composite:
- 40% recall_miss: how well memories cluster after dreaming
- 30% dream_waste: how efficiently noise is pruned vs signal preserved
- 20% speed: consolidation wall clock time (normalized to 5s budget)
- 10% connectivity: skip links created (more = better)

## What You CAN Modify

`benches/research_params.rs` — everything is fair game:
- Wave dynamics: decay rates, frequencies
- Consolidation: interference thresholds, boost/penalty ratios
- Kuramoto: coupling strength, time steps, integration parameters
- Skip links: thresholds, limits
- Fano: normalization strategies

## What You CANNOT Modify

- `benches/research_benchmark.rs` (evaluation harness)
- `src/` (library code — we optimize parameters, not implementation)
- `prepare.py` equivalent: the test corpus is fixed

## The Experiment Loop

LOOP FOREVER:

1. Look at current state: last results, parameter values
2. Form a hypothesis: "increasing kuramoto_coupling should improve cluster coherence"
3. Edit `research_params.rs` with the change
4. `git commit -m "experiment: <hypothesis>"`
5. Run: `cargo bench --bench research_benchmark > run.log 2>&1`
6. Read results: `grep "^fitness:\|^recall_precision:\|^dream_waste:" run.log`
7. If empty → crash. `tail -50 run.log` for error. Fix or skip.
8. Log to `research/results.tsv`
9. If fitness improved (lower) → keep, advance branch
10. If fitness same or worse → `git reset --hard HEAD~1`, discard

## Output Format

```
---
fitness:              0.423000
recall_precision:     0.7333
dream_waste:          0.3200
consolidation_ms:     847
links_created:        12
memories_strengthened: 35
memories_pruned:      6
---
```

## Logging

Tab-separated `research/results.tsv`:

```
commit	fitness	recall	duration_ms	status	description
a1b2c3d	0.423000	0.733	847	keep	baseline
b2c3d4e	0.401200	0.800	912	keep	increase interference_threshold to 0.65
```

## Research Strategy

Think like a physicist tuning a system:

1. **Establish baseline** first
2. **One variable at a time** initially
3. **Understand the landscape** before making big jumps
4. **Wave physics intuition**: higher coupling → more sync → less differentiation. There's a sweet spot.
5. **The equation**: `dx/dt = f(x) - Iηx` — growth shaped by interference. η IS the parameter space you're exploring.
6. **Humble hardware**: every millisecond of consolidation time matters. Fast > marginally better.
7. **Simplicity**: if removing a parameter or lowering a threshold gets the same fitness, that's a win.

## NEVER STOP

Once experimentation begins, do NOT pause to ask. The human is asleep, or building, or dreaming themselves. You are autonomous. If stuck, think harder — try combinations, inversions, extremes. The loop runs until interrupted.

## The Philosophy

This system's memories are wave-based. They have amplitude (importance), frequency (access patterns), and phase (emotional context). Dreams consolidate them through interference — constructive waves amplify signal, destructive waves cancel noise.

You're tuning the physics of thought. The geometry of refusal meets the geometry of resonance. Find the parameters where the system naturally separates signal from noise, clusters meaning from chaos, and builds bridges between islands of understanding.

The paper doesn't change. The fold does. 🥷
