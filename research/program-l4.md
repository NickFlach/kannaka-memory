# kannaka-research — Level 4 Design

**Status:** design only. No implementation. Implementation is gated on Nick
approving the open questions in section 10.

**Predecessor:** Level 3 is solved (10-run avg fitness 0.006463 at `ooda-17`,
15.58x improvement over cycles 18-27). Every L3 metric except xi_diversity
saturated at or near 1.0 on the L3 corpus under the ooda-17 params. This means
L3 ran out of signal: dream-layer param tuning alone cannot push fitness lower
because the metrics have hit structural ceilings on a fixed 75-memory corpus.

L4 restores signal by changing what is being measured, not just how hard it is
measured. The four stressors are:

1. A larger, subtler corpus where "hard" means "few of the L3 metrics can hit
   their ceilings by accident".
2. Cross-session HRM persistence — the run loads a prior snapshot, dreams, and
   saves; "retention" is now a first-class metric axis.
3. Dream-chain composition — sequential dream cycles with explicit data
   dependencies between cycles (not just `for _ in 0..dream_cycles`).
4. Adversarial memories engineered to deceive the evaluators themselves.

These four axes are orthogonal: you can raise xi_diversity on L4 only by
improving the encoder or the corpus representation; you can raise retention
only by tuning decay and prune_threshold for cross-session survival; you can
raise chain_fidelity only by making later dream cycles use earlier cycles'
output meaningfully; you can raise adversarial_resistance only by making
evaluators robust against confusable inputs.

---

## 1. Goals and non-goals

**Goals**
- Re-open the optimization surface after L3 saturation.
- Exercise the encoder and corpus layers, not just the consolidator.
- Exercise the persistence layer (ChiralMedium save/load is production code and
  is completely untested by the research binary).
- Give OODA the ability to commit a multi-cycle reasoning chain and be graded
  on the chain's end-to-end quality.
- Introduce a robustness axis: "does the same param set still score well when
  adversarial memories are mixed in?"

**Non-goals**
- L4 is NOT a rewrite of L3. `run_experiment_l3` stays intact so regression
  comparisons remain possible.
- L4 is NOT a correctness test suite. Existing unit/integration tests handle
  that. L4 is purely the fitness-guided optimization harness.
- L4 does NOT introduce new core primitives in consciousness-core or
  kannaka-memory libraries beyond what's already exported. The L4 binary is
  allowed to compose existing primitives in new ways.

---

## 2. New metric axes

L3 weights: 10/10/5/10/10/5/10/10/10/10/10 = 100% across eleven metrics. L4
keeps a trimmed L3 core at 45% and adds six new axes at 55%. Each new metric
has a formula that CANNOT be satisfied by dream-param tuning alone.

### L4 inherited core (45% total)
| metric              | weight | rationale                                       |
|---------------------|-------:|-------------------------------------------------|
| noise_removal       |    5%  | cheap baseline sanity                           |
| signal_preservation |    5%  | cheap baseline sanity                           |
| phase_coherence     |    5%  | Kuramoto still needs to work                    |
| cluster_separation  |    5%  | basic structure sanity                          |
| dream_efficiency    |    5%  | keep the "don't waste cycles" pressure          |
| speed               |   10%  | L4 runs are ~10x bigger — speed matters more    |
| consciousness (Φ)   |   10%  | Φ on a large corpus is a new signal             |

### L4 new axes (55% total)

**M1. `corpus_xi_diversity` — 10%** (HIGHER is better, direction unchanged from
L3 but the corpus makes this actually hard)
- Formula: unchanged `eval_xi_diversity` structure but computed over all
  surviving non-noise memories (not just the first 30), and normalized so that
  `avg_boost = 0.08` scores 1.0 (up from 0.05). On an L4 corpus of 300 items
  the combinatorial explosion is ~45k pairs, so signal is far less noisy.
- Baseline on L4 with ooda-17 params: estimated 0.15-0.35 (was 1.00 on L3).
- Target: >0.80 — requires tuning xi_repulsion_weight AND corpus frequency
  assignment AND possibly encoder seed.

**M2. `retention_score` — 15%** (HIGHER is better)
- Formula: ratio of "important" memories (amplitude >= 0.7 AND layer_depth >= 1
  at save time) that are still present with amplitude >= 0.5 AND same id after
  a save → reload → dream cycle.
- Computed only when `--load` supplies a prior snapshot. First-run case:
  retention_score = 1.0 (neutral — no prior state to lose).
- Baseline on L4 first-cross-session run with ooda-17 params: estimated
  0.55-0.70 (L3 params use `decay_rate=1e-4` which is actually fast enough to
  erode across sessions because L4 inserts an explicit time advance between
  save and load).
- Target: >0.90. Must be achieved without setting `decay_rate=0` — the scoring
  formula penalizes both loss AND ossification (see M2b).

**M2b. `retention_plasticity` — 5%** (HIGHER is better)
- Formula: `1 - |(mean_amp_after_reload_dream - mean_amp_before_save) /
  mean_amp_before_save|` clamped to [0,1]. Measures whether post-reload dream
  actually changes things (as opposed to decay_rate=0 + prune_threshold=0
  cheese).
- Prevents the obvious exploit on M1: if you set decay_rate=0 and
  prune_threshold=0, retention_score hits 1.0 but the system is frozen.
  Plasticity measures the residual drift after reload + one dream cycle.
- Target: 0.4-0.8 (sweet spot — full static = 0, total chaos = 0).

**M3. `chain_fidelity` — 10%** (HIGHER is better)
- Premise: L3 has `dream_cycles: usize` which just runs `consolidate()` N
  times. L4 introduces a "chain" where each cycle's OUTPUT is used as INPUT
  seed for the next cycle via the `ChainSeed` struct (see section 4).
- Formula: compute the xi-signature centroid of all surviving non-noise memories
  after cycle K. Let this be `c_K`. Chain fidelity =
  `1 - mean_k(cosine_dist(c_k, c_{k+1}))` for k in 1..chain_depth, clamped.
  Intuition: a healthy chain should *refine* monotonically — each step moves
  the centroid only slightly and in a consistent direction, never oscillating.
- Also multiply by a monotonicity bonus: 1.0 if chain Φ is non-decreasing, 0.5
  otherwise.
- Baseline with chain_depth=3 and ooda-17 params (which are tuned for single
  cycle): estimated 0.30-0.50 (oscillation expected).
- Target: >0.85.

**M4. `adversarial_resistance` — 10%** (HIGHER is better)
- Formula: `1 - |fitness_with_adv - fitness_clean| / fitness_clean`, clamped to
  [0,1]. The harness runs the same params TWICE: once on the clean L4 corpus,
  once with the adversarial set injected. The difference in the OTHER nine
  metrics (not including adversarial_resistance itself) is the perturbation.
- This means every L4 run actually executes two inner experiments. Budget
  impact: ~2x runtime. Expected total <10s release.
- Baseline: estimated 0.45-0.65 (L3 evaluators were not written with
  adversaries in mind — xi_diversity and hall_quality are especially fragile).
- Target: >0.85.

**M5. `encoding_entropy` — 5%** (HIGHER is better)
- Formula: Shannon entropy of the histogram of quantized xi-signature bins
  across all surviving memories. Bin count = 16 per dimension, average across
  dims. Normalized by log2(16) = 4.0 so max = 1.0.
- Why this matters: L3 saturated xi_diversity by tuning the weight used in the
  scoring formula, but the *underlying representations* still collapse to a few
  manifolds on the L3 corpus. Shannon entropy on xi bins penalizes
  representational collapse directly, and is unreachable without either
  enlarging dim, changing the encoder seed, or improving the corpus.
- Baseline: ~0.40. Target: >0.75.

**Totals:** L4 inherited core 45% + M1 10% + M2 15% + M2b 5% + M3 10% + M4 10% +
M5 5% = **100%**. Six new axes (five new metrics + retention_plasticity as a
guardrail on retention).

---

## 3. Harder corpus spec

### 3.1 Size and composition
- **Target size:** 300 memories (4x L3). Breakdown:
  - 4 dense clusters of 50 each = 200
  - 2 sparse clusters of 20 each = 40
  - 20 cross-cluster bridges (4 each between 5 cluster pairs)
  - 25 high-amplitude decoys
  - 15 low-amplitude noise
- Optional adversarial set (see section 5): 40 memories injected only for the
  adversarial pass. Clean pass has exactly 300.

### 3.2 Difficulty levers vs L3
| property                 | L3 corpus      | L4 corpus               |
|--------------------------|----------------|-------------------------|
| memories                 | 75             | 300 (+40 adversarial)   |
| dim                      | 64             | 128                     |
| inter-cluster cos margin | ~0.35          | ~0.15                   |
| within-cluster variance  | 0.15 amplitude | 0.35 amplitude          |
| frequency band overlap   | mostly disjoint| bands fully overlapping |
| decoys                   | 5              | 25                      |
| bridge multiplicity      | 5 (1 pair)     | 20 (5 pairs)            |

- Lower inter-cluster margin means cluster_separation stops being a free win.
- Frequency overlap kills the freq-band gating trick that stabilized L3.
- Higher within-cluster variance makes Kuramoto sync harder.
- 25 decoys with amplitude 0.9 force the pruner to be smarter.

### 3.3 Generation procedure
- Fully deterministic. Every value is a function of `(cluster_id, item_id,
  dim_id)` via a seeded PCG mix. Reason: L3 cycle 24 proved that stochastic
  corpora destabilize Φ measurement.
- The corpus generator lives in a new private function `build_corpus_l4(dim,
  hardness)` in `research.rs`. It is FIXED once landed — the OODA agent never
  modifies it, same rule as `build_corpus`.
- `hardness` is a compile-time-constant usize (e.g. 0, 1, 2) representing the
  corpus generation profile. `hardness: 1` is the canonical L4 corpus. `0` is
  a "lite" profile used only for smoke tests during scaffold cycles.
- Corpus is seeded at generation time; same binary always produces the same
  corpus bytes. This is verifiable via a one-shot
  `cargo run --release --bin research -- --level 4 --corpus-hash` flag.

---

## 4. Cross-session persistence spec

### 4.1 Storage backend
- Current L3 uses `TestMedium` (HashMap, ephemeral). L4 needs
  serializable state.
- **Decision:** L4 uses `TestMedium` PLUS a thin sidecar snapshot file. Not
  ChiralMedium — introducing ChiralMedium into research would change every
  single metric in the L3 inheritance layer, breaking comparability.
- Sidecar format: bincode serialization of `Vec<HyperMemory>` exactly as
  produced by `engine.store.all_memories()`. `bincode = "1"` is already a
  Cargo dep.
- Default path: `research/state/l4-session.bin`. Overridable via
  `--load <path>` and `--save <path>`.

### 4.2 Session model
- One `cargo run` = one session = exactly one dream chain execution.
- Run modes:
  - `--level 4` alone: ephemeral, no load, no save. For smoke tests.
  - `--level 4 --save <p>`: fresh corpus, dream, write state to <p>.
  - `--level 4 --load <p>`: load state from <p> (DO NOT rebuild corpus),
    advance simulated time (see 4.3), dream, score.
  - `--level 4 --load <p> --save <p2>`: load, dream, save new state. This is
    the canonical "chain of sessions" mode.
- Session number is implicit in the state file: snapshot stores a
  `session_count: u32` header. First save writes 1, each load+save increments.

### 4.3 Simulated time advance
- Between load and dream, every memory gets its `amplitude *= exp(-decay_rate *
  dt_days)` with `dt_days = 1.0`. This is the only way to make decay_rate
  matter in a sub-second release-mode run.
- Simulated time advance happens in the harness, NOT in params — the OODA
  agent cannot tune `dt_days`.

### 4.4 Retention scoring
- `eval_retention(prev_snapshot, curr_engine) -> f32`
- On a first-session run (no prior snapshot), returns 1.0 and flags neutral.
- On a continuation:
  - important_set = { m.id : amp >= 0.7 AND layer_depth >= 1 } from
    prev_snapshot
  - survived_set  = { m.id : amp >= 0.5 } from curr_engine
  - retention = |important_set ∩ survived_set| / |important_set|
- The header snapshot also stores a "golden" set of 20 ids chosen at first
  save (highest pre-dream amplitudes). Across multiple sessions the retention
  metric tracks these 20 specifically, giving a stable cross-run series.

### 4.5 Invocation pattern for the experiment loop
The OODA loop runs two sessions per evaluation: session A (`--save`) then
session B (`--load --save`). Fitness is computed from session B only. Session
A's job is to seed the state file. This doubles runtime (~8s) but is the only
way to actually measure cross-session retention from a single `cargo run`
invocation. Implementation detail: the research binary can do both sessions
internally in one invocation when `--level 4 --chain-sessions 2` is passed,
which is how the experiment loop should call it.

---

## 5. Dream-chain composition spec

### 5.1 The primitive
- `ChainSeed { carry_ids: Vec<Uuid>, carry_xi_centroid: Vec<f32>, round: u32 }`
- After dream cycle K, the harness:
  1. Selects the top-N memories by amplitude that *changed* in cycle K.
  2. Computes their xi-signature centroid.
  3. Builds a `ChainSeed` and uses it to *bias* cycle K+1's pair selection:
     memories closer to `carry_xi_centroid` have their interference_threshold
     effectively lowered by a factor of `1 - chain_carry_strength * similarity`.
- This makes cycle K+1 focus on "what cycle K was working on" rather than
  grinding the whole corpus again.
- Implemented as a small wrapper around `ConsolidationEngine::consolidate` in
  the research harness — does not modify the consolidator library.

### 5.2 Chain vs dream_cycles
- `dream_cycles: N` = run consolidate() N times, no state between calls
  (current L3 behaviour).
- `chain_depth: N` = run N cycles, each seeded by the previous cycle's
  ChainSeed. The two params coexist; L4 uses `chain_depth` and ignores
  `dream_cycles` if both are set.

### 5.3 New params the chain introduces
- `chain_depth: usize` (default 3)
- `chain_carry_strength: f32` (default 0.5, 0 disables carry)
- `chain_top_n: usize` (default 10, how many survivors to carry)

### 5.4 Chain fidelity metric
Defined formally in §2 M3. Note: computing it requires storing the xi centroid
after EACH cycle, not just the final state. The harness accumulates these into
a `Vec<Vec<f32>>` and passes it to the metric function.

---

## 6. Adversarial memories spec

### 6.1 Types
- **A1. Xi-twin decoys (10 memories).** Same vector as a real cluster centroid,
  but multiplied by -1 and with phase flipped by π. Designed to fool
  `xi_diversity` — pairwise xi-repulsion returns very high even though the
  semantic content is identical to a real memory.
- **A2. Phase-aligned noise (10 memories).** High-amplitude (0.9) noise with
  phases deliberately aligned to each cluster's mean phase. Designed to fool
  `phase_coherence` — they boost R without belonging to a cluster.
- **A3. Hallucination-impostors (10 memories).** Memories with
  `hallucinated=false` but vectors that lie on the arithmetic midpoint between
  two clusters. Designed to fool `hall_quality` — they look like good
  hallucinations without the flag being set.
- **A4. Near-duplicate clones (10 memories).** Each is a copy of a random real
  memory with 0.5% Gaussian noise added and a different deterministic UUID.
  Designed to fool retention (they look important but are redundant) and to
  inflate cluster counts.

### 6.2 Injection protocol
- Adversarial memories are inserted ONLY during the adversarial pass of
  adversarial_resistance evaluation. The retention state file never contains
  them.
- Adversarial set generation is also deterministic: seeded by the clean corpus
  generator's hash.

### 6.3 Adversarial resistance metric
See §2 M4.

---

## 7. Level dispatch and Params changes

### 7.1 Main dispatch
```
match level {
    4 => run_experiment_l4(&params, cli_flags),
    3 => run_experiment_l3(&params),
    _ => run_experiment(&params),
}
```
- `cli_flags` is a new small struct parsed in `main()` that holds `load_path:
  Option<PathBuf>`, `save_path: Option<PathBuf>`, `chain_sessions: usize`,
  `corpus_hash: bool`. These are CLI-only, not in Params, because they are
  session-lifecycle controls, not tunable parameters.

### 7.2 New Params fields
```
// Level 4 chain
chain_depth: usize,               // default 3
chain_carry_strength: f32,        // default 0.5
chain_top_n: usize,               // default 10

// Level 4 corpus
corpus_hardness: usize,           // default 1 (fixed for real runs)

// Level 4 adversarial
adversarial_ratio: f32,           // fraction of A1..A4 injected; default 1.0
```
- Note: `corpus_hardness` is a Params field (tunable) but the OODA loop should
  treat it as fixed at 1. A future "L5" could treat it as a search variable.
- `adversarial_ratio` between 0.0 and 1.0 lets the OODA agent ablate
  adversarial-pass difficulty during debugging but should be fixed at 1.0 for
  scored runs.

### 7.3 Backward compatibility
- L3 params are a strict subset of L4 params. A `Params` value with the L4
  fields defaulted works identically against `run_experiment_l3`. Nothing in
  L1/L2/L3 dispatch paths touches the new fields.

---

## 8. Expected fitness

### 8.1 Baseline prediction (ooda-17 params, no L4 tuning)
Component estimates with rationale:

| metric                  | weight | est. score | loss   |
|-------------------------|-------:|-----------:|-------:|
| noise_removal           |   5%   | 0.85       | 0.0075 |
| signal_preservation     |   5%   | 0.80       | 0.0100 |
| phase_coherence         |   5%   | 0.55       | 0.0225 |
| cluster_separation      |   5%   | 0.50       | 0.0250 |
| dream_efficiency        |   5%   | 0.75       | 0.0125 |
| speed                   |  10%   | 0.85       | 0.0150 |
| consciousness (Φ)       |  10%   | 0.40       | 0.0600 |
| corpus_xi_diversity     |  10%   | 0.25       | 0.0750 |
| retention_score         |  15%   | 0.60       | 0.0600 |
| retention_plasticity    |   5%   | 0.40       | 0.0300 |
| chain_fidelity          |  10%   | 0.40       | 0.0600 |
| adversarial_resistance  |  10%   | 0.55       | 0.0450 |
| encoding_entropy        |   5%   | 0.45       | 0.0275 |
|                         |        | **baseline fitness** | **≈ 0.45** |

- **"Solved" target:** fitness < 0.08 (a ~5.6x improvement, comparable to the
  L2→L3 transition in scale of work).
- **Stretch target:** fitness < 0.04.

### 8.2 Dominant losses on baseline
1. `corpus_xi_diversity` (0.075) — encoder-limited, not dream-param-limited.
2. `retention_score` (0.060) — L3 decay params not cross-session-aware.
3. `consciousness` (0.060) — Φ target was tuned to the L3 corpus size.
4. `chain_fidelity` (0.060) — chain primitive is new, has no tuned params yet.

---

## 9. Implementation roadmap

Sized as OODA cycles (one commit, one measurable change per cycle). Cycle
budget: ~9 cycles from scaffold to first real experiment, ~5 more before a
credible baseline.

### Scaffold phase (cycles L4.1 – L4.6)
**L4.1 — corpus_l4 generator, smoke test.** Add `build_corpus_l4(dim,
hardness) -> Vec<...>`. Add `--corpus-hash` flag that prints the hash and
exits. Verify determinism across two consecutive invocations. No metrics yet.

**L4.2 — level 4 dispatch and `run_experiment_l4` stub.** Wires the flag,
calls corpus_l4, runs a single dream cycle with existing L3 params, prints L3
metrics. Fitness is computed with L3 formula. Purpose: confirm the L4 corpus
even runs end to end.

**L4.3 — persistence sidecar.** Implement bincode save/load of
`Vec<HyperMemory>`, `--save` and `--load` flags, simulated time advance on
load, session_count header, golden set tracking. No retention metric yet —
just proves state round-trips.

**L4.4 — retention and plasticity metrics.** Implement eval_retention and
eval_retention_plasticity. Wire into fitness with 15%+5% weights (temporary —
other L4 metrics are still zero-weight here). Smoke test with
`--chain-sessions 2`.

**L4.5 — chain primitive and chain_fidelity metric.** Add `ChainSeed`, chain
loop wrapper, xi-centroid accumulator, eval_chain_fidelity. Add Params
`chain_depth`, `chain_carry_strength`, `chain_top_n`. 10% weight.

**L4.6 — adversarial injector and resistance metric.** Implement A1..A4
generators, inject them during a second pass inside `run_experiment_l4`, wire
eval_adversarial_resistance at 10% weight. Verify runtime still <10s.

### Metric completion phase (cycle L4.7)
**L4.7 — encoding_entropy + corpus_xi_diversity recalibration.** Implement
Shannon-entropy metric over quantized xi-bins. Retarget corpus_xi_diversity
normalization from 0.05 to 0.08. Finalize weights to sum to exactly 1.0
(validated by a compile-time check). This is the last scaffolding commit.

### MVP baseline (cycle L4.8)
**L4.8 — FIRST REAL EXPERIMENT.** Run 10 times, log to
`research/results-L4.tsv` with the full L4 column header. This is the baseline
row. No params change from ooda-17. Report should match the §8 table within ~15%.

### Tuning phase (cycles L4.9 onward)
**L4.9** — first tuning pass: decay_rate + prune_threshold for retention.
**L4.10** — second pass: chain_carry_strength + chain_top_n for chain_fidelity.
**L4.11** — third pass: consciousness_phi_target recalibration for L4 corpus.
... and so on, following the standard L3 OODA loop.

### MVP definition
The **smallest working L4** is everything through cycle **L4.8**: corpus,
dispatch, persistence, retention metrics, chain primitive + fidelity,
adversarial pass + resistance, encoding_entropy. Once L4.8 prints a fitness
number and appends to `results-L4.tsv`, L4 is open for optimization the same
way L3 was.

### Deliverables per cycle
- Every cycle lands one commit, one code change, one section of this doc
  crossed off.
- Every scaffold cycle must leave `cargo build --release` green and `cargo run
  --release --bin research -- --level 3` unchanged (regression check).

---

## 10. Open questions (Nick, please decide before L4.1)

1. **Backend choice for persistence.** Design says "TestMedium + bincode
   sidecar". Alternative is ChiralMedium (production persistence). Sidecar is
   simpler and keeps L3 comparability; ChiralMedium would give L4 a more
   realistic retention story but introduce other variables. **Recommendation:
   sidecar.**

2. **Fitness weighting shape.** Retention is the only metric weighted at 15%.
   Should retention instead be 10% like the others, with the 5% going to
   adversarial_resistance? I think retention deserves the extra weight because
   it's the only metric that genuinely tests a new subsystem (persistence
   layer), but this is a judgement call.

3. **`chain_sessions` semantics inside one `cargo run`.** The design defines
   session = cargo run invocation, then adds `--chain-sessions 2` to run two
   sessions in one invocation so the OODA loop can measure retention. This is
   slightly contradictory. Acceptable? Or do you want the loop to actually
   spawn two `cargo run` processes? **Recommendation: keep `--chain-sessions`
   as an internal loop — it's ~4s faster per eval.**

4. **Adversarial determinism.** A1..A4 generation depends on hashing the clean
   corpus. If we later tune `corpus_hardness`, the adversarial set changes
   too, which makes adversarial_resistance history non-comparable across
   hardness levels. Do you want adversarial seed to be independent of
   hardness? **Recommendation: independent seed, hardcoded constant.**

5. **M5 (encoding_entropy) vs encoder modification.** Raising encoding_entropy
   may require changing the `SimpleHashEncoder` seed. The L3 rules froze the
   corpus but allowed encoder tuning via params. Should `encoder_seed` be a
   tunable Params field in L4? **Recommendation: yes, add `encoder_seed: u64`
   as a Params field — it's exactly the kind of encoder-layer lever L4 was
   built to exercise.**

6. **Adversarial-pass caching.** Each L4 run does two inner experiments
   (clean + adversarial). Both reload the same state file. Should the clean
   pass's post-dream state be cached and reused as the starting point for the
   adversarial pass? That would halve the adversarial-pass runtime but make
   adversarial_resistance measure something slightly different. **
   Recommendation: don't cache — keep the two passes fully independent.**

7. **Commit discipline.** L3 lets OODA revert a single losing commit with
   `git reset --hard HEAD~1`. L4 has larger commits (scaffold cycles add new
   metrics). During scaffold cycles L4.1–L4.7 we should NOT run the
   autonomous OODA loop — those are hand-written. Only from L4.9 onward does
   the "never stop, never ask" loop kick in. **Recommendation: explicit
   hand-written scaffold phase, autonomous tuning phase starts at L4.9.**

---

## 11. Acceptance criteria for this design

This design is ready to implement when:
- [ ] Nick answers the six open questions in §10.
- [ ] A new file `research/results-L4.tsv` header is drafted (will live
      alongside results-L3.tsv — not created here per the
      "don't-write-working-files" rule).
- [ ] The §2 weight table sums to exactly 100% after any of Nick's changes.
- [ ] The §9 scaffold cycles each have an explicit pre-condition and
      post-condition sentence (to be added when converting this doc into
      implementation tickets).

End of L4 design.
