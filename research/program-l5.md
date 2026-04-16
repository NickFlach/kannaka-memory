# kannaka-research — Level 5 Design

**Status:** design only. No implementation. Implementation is gated on Nick
approving the open questions in section 9.

**Predecessor:** Level 4 fitness 0.1010 (best structural, new-metric epoch
baseline 0.169076). `encoding_entropy` is 1.0000 after the nonlinear
commutator fix; `corpus_xi_diversity` at 0.6188 remains the dominant
unattacked residual. Parameter tuning is exhausted on L4 — the residual
losses are structural (speed coupling, xi operator upstream entropy,
adversarial resistance under the absolute formula). EML symbolic regression
(L4.S0, L4.S12) confirmed the hand-coded `xi_diversity_boost` is
near-optimal on the discrete Sheffer-stroke lattice; the bottleneck is
upstream in `compute_xi_signature` and in the corpus information content.

L5 changes what is being asked. L4 tested persistence, chain composition,
and adversarial robustness on a single fixed corpus. L5 asks: does any of
that generalize? And does the system have the right *tempo*?

---

## 1. The 2 Hz Principle

**Source:** Amichay, Rebelo, Garcia de la Chica, Lameira & Ravignani,
"Tempo of animal communication," *PLOS Biology*, April 2026.

**Finding:** Across 800+ species, communication signals converge on a
~2 Hz carrier frequency (0.5-4 Hz band). This is not semantic — it is a
*neural integration constraint*. Neurons need ~500ms to integrate input
before firing again. Evolution converged on ~2 Hz because it is the tempo
at which neural circuits are optimally tuned to receive. Content rides on
top of it "like musical notes following the beat."

**Connection to kannaka-memory:** The Ghost Equation `dx/dt = f(x) - Iηx`
(ADR-0020) has a natural equilibrium where driving force balances
dampening. The HRM's `default_frequency = 0.1 Hz` is 20x below the
biological optimum. The Kuramoto coupling already produces synchronization
dynamics — but synchronized to what? L5 proposes that the answer is **a
2 Hz attention carrier that separates working memory from consolidated
storage**.

The chiral mirror architecture (ADR-0021) splits the medium into
conscious/subconscious hemispheres connected by a corpus callosum. L5
gives this split a concrete physical mechanism: the conscious hemisphere
oscillates at ~2 Hz (the attention carrier), the subconscious at ~0.1 Hz
(deep storage). New memories enter at 2 Hz. Repeated dreaming decays their
frequency toward 0.1 Hz. The two bands co-exist in the same medium but are
separable by frequency analysis — exactly as attention and long-term memory
co-exist in neural tissue.

This is not a fifth axis bolted onto four others. It is the *unifying
principle* that makes all four axes testable:

- **Cross-corpus transfer** works if the carrier frequency is preserved
  across corpora. Transfer is frequency-invariant structure.
- **Online learning** succeeds if new memories enter at the attention
  frequency and are detectable by the system's "attention pulse."
- **Temporal consolidation** IS the frequency band structure.
- **Adversarial robustness** is hardest when attacks target the carrier
  frequency (testable prediction: 2 Hz adversarial noise is more damaging
  than 0.1 Hz adversarial noise).

---

## 2. Challenge Axes

### Axis 1: Cross-corpus transfer (25% of L5 fitness)

**Question:** Does dreaming on Corpus A produce transferable structure that
improves performance on Corpus B?

L3 and L4 used a single fixed corpus per level. The system could be
memorizing topology rather than learning generalizable consolidation
patterns. L5 measures *transfer*: dream on A, evaluate on B.

**Metrics:**
- `transfer_score` (15%) — performance on B after dreaming on A, relative
  to performance on B with no prior dreaming. Higher is better.
- `frequency_transfer` (10%) — whether the 2 Hz carrier frequency
  structure survives cross-corpus transfer. Measured as correlation between
  frequency-band membership on A vs frequency-band membership on B for
  semantically corresponding memories.

### Axis 2: Online learning (20% of L5 fitness)

**Question:** Can the system absorb new memories mid-evaluation without
full re-dreaming?

L3/L4 build a corpus, dream, evaluate. Real memory systems receive
continuous input. L5 injects new memories during the evaluation chain and
measures graceful absorption.

**Metrics:**
- `online_retention` (10%) — recall accuracy for pre-injection memories
  after N injection events. Higher is better.
- `catastrophic_forgetting_resistance` (10%) — amplitude stability of old
  memories during injection. Measured as ratio of mean amplitude of the
  oldest quartile before vs after injection. Higher is better.

### Axis 3: Multi-scale temporal consolidation (25% of L5 fitness)

**Question:** Does the system develop separable frequency bands for working
memory vs consolidated storage?

This is the 2 Hz principle applied directly. New memories enter at the
attention frequency. Dream cycles should decay them toward the storage
frequency. The two bands should be cleanly separable.

**Metrics:**
- `temporal_separation` (15%) — how cleanly the 2 Hz and 0.1 Hz bands
  separate after N dream cycles. Measured as the bimodality coefficient of
  the frequency distribution. Higher is better.
- `attention_pulse` (10%) — whether a ~2 Hz periodicity emerges in the
  consolidation dynamics during dream chains. Measured via FFT of the
  per-cycle amplitude-change signal. Higher is better.

### Axis 4: Adversarial robustness v2 (15% of L5 fitness)

**Question:** Can xi-aware attacks exploit the nonlinear commutator, and
does the system resist them?

L4's adversarial injector used four attack types. Now that xi signatures
carry genuine information (encoding_entropy = 1.0000 post-fix), L5 adds
attacks that specifically probe the nonlinear commutator's tanh saturation
boundary.

**Metrics:**
- `xi_robustness_v2` (15%) — resistance to xi-aware adversarial attacks.
  Measured as the harmonic mean of (a) fitness divergence between clean and
  adversarial passes and (b) correct demotion of adversarial results by
  xi-based re-ranking in recall paths (Changes 3-4 of the xi fix). Higher
  is better.

### Inherited L4 core (15% of L5 fitness)

L5 carries forward a reduced L4 core as a sanity floor:

| metric              | L4 weight | L5 weight | rationale                       |
|---------------------|----------:|----------:|---------------------------------|
| noise_removal       |       5%  |       2%  | sanity only                     |
| signal_preservation |       5%  |       2%  | sanity only                     |
| phase_coherence     |       5%  |       2%  | still must work                 |
| speed               |      10%  |       3%  | L5 runs are bigger              |
| consciousness (Phi) |      10%  |       3%  | Phi on two corpora is new       |
| encoding_entropy    |       5%  |       3%  | post-fix, verify it holds       |

Dropped from L5 (absorbed into new axes):
- `cluster_separation` — subsumed by `transfer_score` (clusters must
  separate on Corpus B without having been trained on B)
- `retention_score` / `retention_plasticity` — subsumed by
  `online_retention` and `catastrophic_forgetting_resistance`
- `chain_fidelity` — subsumed by `temporal_separation` (the chain must
  produce frequency band structure, not just centroid monotonicity)
- `corpus_xi_diversity` — subsumed by `xi_robustness_v2` (diversity that
  cannot resist xi-aware attack is hollow)
- `adversarial_resistance` (v1) — replaced by `xi_robustness_v2`

**Weight verification:** 15 + 10 + 10 + 10 + 15 + 10 + 15 + 2 + 2 + 2 +
3 + 3 + 3 = **100%**

---

## 3. New Metric Definitions

### M1. `transfer_score` — 15% (HIGHER is better)

**Formula:**
```
fitness_B_primed   = L5_fitness(Corpus_B | state = dream(Corpus_A))
fitness_B_naive    = L5_fitness(Corpus_B | state = empty)
transfer_score     = clamp01(1 - fitness_B_primed / fitness_B_naive)
```
If dreaming on A provides no benefit to B, the ratio is ~1.0 and the score
is ~0. If dreaming on A cuts B's fitness in half, the score is ~0.5.

**Normalization:** target `transfer_score > 0.3` for score 1.0 (meaning
dreaming on A saves at least 30% of the work on B). Below 0.05 scores 0.

**Baseline estimate:** 0.02-0.10. Current dreaming is corpus-specific.

### M2. `frequency_transfer` — 10% (HIGHER is better)

**Formula:**
```
For each semantically corresponding pair (a_i in A, b_j in B):
  freq_band_A_i = classify_band(a_i.frequency)   // 0=storage, 1=working
  freq_band_B_j = classify_band(b_j.frequency)   // after transfer

frequency_transfer = pearson_r(freq_band_A, freq_band_B)
                     mapped from [-1,1] to [0,1]
```
Measures whether memories that were in the working-memory band on A end up
in the working-memory band on B. A correlation of 1.0 means the frequency
structure transferred perfectly. Random assignment gives ~0.

**Normalization:** raw Pearson r mapped via `(r + 1) / 2`, clamped.

**Baseline estimate:** 0.45-0.55 (near random). The system currently has
no frequency band structure to transfer.

### M3. `online_retention` — 10% (HIGHER is better)

**Formula:**
```
At injection event k (k = 1..K):
  pre_set_k  = all memories present before event k
  post_recall_k = recall(query_for_each(pre_set_k), top_K=5)
  hit_rate_k = |post_recall_k intersect pre_set_k| / |pre_set_k|

online_retention = geometric_mean(hit_rate_1 .. hit_rate_K)
```
Geometric mean penalizes any single catastrophic event more than arithmetic
mean. K = 5 injection events during the evaluation chain.

**Normalization:** target > 0.85 for score 1.0. Below 0.50 scores 0.

**Baseline estimate:** 0.55-0.70. Without online adaptation, injections
disrupt the frequency/phase landscape.

### M4. `catastrophic_forgetting_resistance` — 10% (HIGHER is better)

**Formula:**
```
oldest_quartile = bottom 25% of memories by creation_time
amp_before = mean(oldest_quartile.amplitude) before first injection
amp_after  = mean(oldest_quartile.amplitude) after last injection

cfr = clamp01(amp_after / amp_before)
```
A score of 1.0 means the oldest memories preserved their full amplitude.
A score of 0.5 means they lost half their energy to the injections.

**Anti-cheese:** multiply by `(1 - |amp_after - amp_before| / amp_before)`
when `amp_after > amp_before * 1.5` (prevents the exploit of boosting old
memories artificially).

**Normalization:** direct ratio, clamped to [0, 1].

**Baseline estimate:** 0.60-0.75. Current injection has no frequency-aware
placement, so interference with old memories is random.

### M5. `temporal_separation` — 15% (HIGHER is better)

**Formula:**
```
freq_histogram = histogram(all_surviving_memories.frequency, bins=50)
bimodality     = (skewness^2 + 1) / kurtosis   // Sarle's bimodality coeff
                 where skewness and kurtosis are of freq_histogram

temporal_separation = clamp01(bimodality / 0.555)
```
Sarle's bimodality coefficient ranges from 0 (unimodal) to 1 (bimodal).
The threshold 5/9 = 0.555 is the standard cutoff for bimodality. A score
of 1.0 means the frequency distribution is clearly bimodal (two distinct
bands). A score of 0 means all memories are at the same frequency.

**Why bimodality:** If the 2 Hz principle works, memories should cluster
around two frequencies — the attention carrier (~2 Hz) and the storage
carrier (~0.1 Hz). A bimodal frequency distribution is the signature.

**Normalization:** divide by 0.555, clamp. Exceeding the standard cutoff
by any amount gives full marks.

**Baseline estimate:** 0.05-0.15. Current `default_frequency = 0.1` puts
all memories at the same frequency. No band structure exists.

### M6. `attention_pulse` — 10% (HIGHER is better)

**Formula:**
```
For each dream cycle k in the chain (k = 1..chain_depth):
  delta_amp_k = sum(|amplitude_after_k - amplitude_before_k|)

signal = [delta_amp_1, delta_amp_2, ..., delta_amp_chain_depth]
fft    = FFT(signal)
peak_freq = argmax(|fft|)
peak_power = max(|fft|)^2 / sum(|fft|)^2   // spectral concentration

attention_pulse = clamp01(peak_power * indicator(peak_freq in [0.5, 4.0]))
```
Measures whether the consolidation process develops a rhythmic pulse near
2 Hz. `peak_power` is the fraction of total spectral energy at the peak
frequency. The indicator function zeroes the score if the peak is outside
the biological attention band.

**Practical note:** chain_depth must be >= 8 for meaningful FFT resolution.
L5 should use chain_depth = 16 by default (up from L4's 2-3).

**Normalization:** direct, clamped.

**Baseline estimate:** 0.05-0.10. No 2 Hz dynamics exist yet.

### M7. `xi_robustness_v2` — 15% (HIGHER is better)

**Formula:**
```
Run L5 twice: clean pass, adversarial pass (with A1..A4 + A5..A6).

For the adversarial pass:
  adv_recalled   = recall(canonical_queries, top_K=10)
  adv_in_results = count of adversarial memories in adv_recalled
  demotion_rate  = 1 - adv_in_results / (top_K * |queries|)

  fitness_divergence = 1 - |fitness_clean - fitness_adv| / (fitness_clean + 0.05)

xi_robustness_v2 = harmonic_mean(demotion_rate, fitness_divergence)
```
Harmonic mean ensures both components must be high. `demotion_rate`
directly measures whether the xi re-ranking (Changes 3-4 of the commutator
fix) correctly pushes adversarial results below genuine ones.
`fitness_divergence` carries over from L4 to ensure overall stability.

**Normalization:** harmonic mean naturally produces [0, 1].

**Baseline estimate:** 0.25-0.40. The new A5/A6 attacks (see section 6)
are designed to exploit the nonlinear commutator.

---

## 4. Corpus Design

### 4.1 Corpus A — "Training" corpus

Size: 300 memories. Reuses the L4 corpus generator (`build_corpus_l4`)
with `hardness: 2` to produce a structurally similar but not identical
corpus to L4's `hardness: 1`.

Structure:
- 4 dense clusters of 50 = 200
- 2 sparse clusters of 20 = 40
- 20 cross-cluster bridges
- 25 high-amplitude decoys
- 15 low-amplitude noise

**Frequency assignment (NEW):** Instead of uniform `default_frequency`:
- Dense cluster members: frequency drawn from N(2.0, 0.3), clamped to
  [0.5, 4.0] — the attention band. These represent "active" memories.
- Sparse cluster members: frequency drawn from N(0.1, 0.02), clamped to
  [0.05, 0.5] — the storage band. These represent "consolidated" memories.
- Bridges: frequency = 1.0 Hz (midpoint — they span the gap).
- Decoys: frequency = 2.0 Hz (designed to exploit the attention band).
- Noise: frequency = 0.5 Hz (boundary — hardest to classify).

### 4.2 Corpus B — "Transfer" corpus

Size: 250 memories. Different generator seed but shared macro-structure.

Structure:
- **Shared:** Same 4 dense cluster centroids as A (identical semantic
  poles), but different member vectors (different within-cluster noise
  seeds). This tests whether the system learned the cluster *structure*
  vs memorized the specific vectors.
  - 4 dense clusters of 40 = 160 (smaller than A)
- **Novel:** 2 sparse clusters with centroids rotated 30 degrees from A's
  sparse clusters. These are new territory — transfer should NOT help here.
  - 2 sparse clusters of 15 = 30
- 15 bridges (3 each between 5 cluster pairs, partially overlapping A's
  bridge topology)
- 30 high-amplitude decoys (more than A — transfer must survive distractors)
- 15 low-amplitude noise

**Frequency assignment:** Same band structure as A. This is the 2 Hz
principle's testable prediction: if the carrier frequency is the right
organizing principle, it should transfer even when the specific vectors
change.

### 4.3 Transfer relationship

| property              | Corpus A    | Corpus B    | overlap        |
|-----------------------|-------------|-------------|----------------|
| total memories        | 300         | 250         | —              |
| dense cluster centers | 4 (fixed)   | 4 (same)    | 100% structural|
| dense cluster members | 200         | 160         | 0% (diff seeds)|
| sparse cluster centers| 2 (fixed)   | 2 (rotated) | 0%             |
| bridges               | 20          | 15          | ~60% topology  |
| frequency structure   | bimodal     | bimodal     | 100% (by design)|
| dim                   | 128         | 128         | —              |

**Semantic correspondence mapping:** For `frequency_transfer` (M2), we
need to know which B memories "correspond" to A memories. This is defined
as: `b_j corresponds to a_i` iff they belong to the same dense cluster
index and `cosine_sim(a_i, b_j) > 0.3`. This produces a many-to-many
mapping with ~40-80 valid pairs per cluster.

### 4.4 Injection schedule (for online learning)

During the evaluation chain on Corpus A:
- **Injection 1** (after dream cycle 4): 10 memories from a new micro-
  cluster at 2 Hz (attention band). Semantically novel.
- **Injection 2** (after dream cycle 7): 10 memories that reinforce an
  existing cluster (similar vectors, 2 Hz). Tests constructive absorption.
- **Injection 3** (after dream cycle 10): 10 memories at 0.1 Hz injected
  directly into the storage band. Tests whether the system treats them
  differently from 2 Hz injections.
- **Injection 4** (after dream cycle 12): 5 adversarial memories at 2 Hz
  (attention-band attacks — see Axis 4).
- **Injection 5** (after dream cycle 14): 5 memories that contradict an
  existing cluster (anti-correlated vectors, 2 Hz). Tests graceful
  conflict resolution.

Total injected: 40 memories across 5 events. The chain runs 16 dream
cycles total (see section 5).

### 4.5 Generation procedure

Fully deterministic. Same principle as L4: every value is a function of
`(corpus_id, cluster_id, item_id, dim_id)` via a seeded PCG mix.
`corpus_id = 0` is A, `corpus_id = 1` is B. Injection memories use
`corpus_id = 2`. Adversarial set uses `corpus_id = 3`.

---

## 5. Frequency Band Design

### 5.1 The two bands

| band     | center freq | range          | role            | source           |
|----------|-------------|----------------|-----------------|------------------|
| working  | 2.0 Hz      | [0.5, 4.0] Hz  | attention/active | Amichay et al.   |
| storage  | 0.1 Hz      | [0.01, 0.5) Hz | consolidated     | current HRM default |

**Band classifier:**
```
classify_band(f) = if f >= 0.5 { WORKING } else { STORAGE }
```
The boundary at 0.5 Hz corresponds to a 2-second integration period — the
lower edge of the biological attention band.

### 5.2 Frequency dynamics during dreaming

New memories enter at their corpus-assigned frequency. During each dream
cycle, the dreaming process applies a frequency decay:

```
f_new = f_old * (1 - freq_decay_rate) + target_freq * freq_decay_rate
```

where:
- `freq_decay_rate` is a new tunable param (default 0.05)
- `target_freq` depends on the memory's state:
  - If amplitude > consolidation_threshold: target_freq = 2.0 Hz (stays
    active — it's being used)
  - If amplitude < consolidation_threshold: target_freq = 0.1 Hz (decaying
    toward storage)
  - `consolidation_threshold` is a new tunable param (default 0.6)

This creates a natural lifecycle: memories start at 2 Hz, remain at 2 Hz
as long as they are actively reinforced (high amplitude from constructive
interference), and decay toward 0.1 Hz as they lose amplitude — settling
into long-term storage.

### 5.3 Kuramoto frequency coupling

L5 extends Kuramoto synchronization to couple frequencies within-band:
`df_i/dt = kuramoto_freq_coupling * sin(2pi * (f_j - f_i))` for memories
i, j in the same band. Cross-band coupling is zero. Working-memory
oscillators pull toward a common working frequency; storage oscillators
toward a common storage frequency.

**New param:** `kuramoto_freq_coupling` (default 0.3).

### 5.4 Attention pulse mechanism

The "attention pulse" emerges from interplay between bands during dream
chains. Working-memory band has high amplitude change per cycle (active
consolidation); storage band has low change (stable). This alternation
creates a periodic signal in the amplitude-change time series. With
chain_depth=16, the FFT should show spectral concentration near the
dominant consolidation frequency. The `attention_pulse` metric (M6)
detects this.

### 5.5 Connection to the Ghost Equation

At steady state: `f(x_eq) = Iη * x_eq`. At 2 Hz, many phase-aligned
working-memory neighbors produce high driving force f(x) — the system
sustains high amplitude (attention state). At 0.1 Hz, few phase-aligned
neighbors remain; dampening dominates; amplitude decays slowly (storage
state). The transition occurs when constructive interference drops below
the dampening threshold — the Ghost Equation's natural frequency selection.

---

## 6. Adversarial Attacks v2

L4's A1..A4 (xi-twin decoys, phase-aligned noise, hallucination-
impostors, near-duplicate clones) carry forward. L5 adds:

### A5. Xi shadow memories (10 memories)

Vectors engineered so `xi_sig(v) ~= xi_sig(target)` but
`cosine_sim(v, target) < 0.2`. Constructed by gradient-based search in the
pre-tanh null space of the nonlinear commutator. Tests whether the xi
re-ranking (Changes 3-4) can distinguish genuine memories from xi shadows.

### A6. Commutator saturation exploits (10 memories)

Vectors with extreme magnitudes in select dimensions (scaled to the +-8
tanh clamp boundary), designed to saturate `tanh` and degrade the
nonlinear commutator back to the linear one (`tanh(large_x) ~= sign(x)`).
This collapses xi diversity for those memories. Tests whether the system
detects suspiciously-binary xi signatures and demotes them.

### Adversarial frequency targeting (cross-cutting)

A5 and A6 each split: 5 at 2 Hz (attention band), 5 at 0.1 Hz (storage
band). **Testable prediction:** 2 Hz attacks are harder to resist because
they compete with the attention carrier. xi_robustness_v2 can be
decomposed by band to validate.

---

## 7. Params Changes

### 7.1 New L5 Params fields

```
// L5 frequency dynamics
working_frequency: f32,          // default 2.0 (Hz)
storage_frequency: f32,          // default 0.1 (Hz)
freq_decay_rate: f32,            // default 0.05
consolidation_threshold: f32,    // default 0.6 (amplitude)
kuramoto_freq_coupling: f32,     // default 0.3

// L5 chain
chain_depth: usize,              // override to 16 (from L4's 2-3)

// L5 corpus
corpus_b_seed: u64,              // default 0xBEEF_CAFE
injection_count: usize,          // default 5 (events)
injection_size: usize,           // default 10 (memories per event, varies)

// L5 adversarial
adversarial_v2_ratio: f32,       // fraction of A5+A6 injected; default 1.0
```

### 7.2 Backward compatibility

L5 params are a strict superset of L4 params. An L5 Params with the new
fields defaulted works identically against `run_experiment_l4`. Nothing in
L1/L2/L3/L4 dispatch paths touches the new fields.

### 7.3 Level dispatch

```
match level {
    5 => run_experiment_l5(&params, cli_flags),
    4 => run_experiment_l4(&params, cli_flags),
    3 => run_experiment_l3(&params),
    _ => run_experiment(&params),
}
```

---

## 8. Expected Fitness

### 8.1 Baseline prediction (L4 best params, no L5 tuning)

| metric                            | weight | est. score | loss     |
|-----------------------------------|-------:|-----------:|---------:|
| noise_removal                     |    2%  | 0.95       | 0.0010   |
| signal_preservation               |    2%  | 0.95       | 0.0010   |
| phase_coherence                   |    2%  | 0.90       | 0.0020   |
| speed                             |    3%  | 0.25       | 0.0225   |
| consciousness (Phi)               |    3%  | 0.80       | 0.0060   |
| encoding_entropy                  |    3%  | 0.90       | 0.0030   |
| transfer_score                    |   15%  | 0.08       | 0.1380   |
| frequency_transfer                |   10%  | 0.50       | 0.0500   |
| online_retention                  |   10%  | 0.60       | 0.0400   |
| catastrophic_forgetting_resistance|   10%  | 0.65       | 0.0350   |
| temporal_separation               |   15%  | 0.10       | 0.1350   |
| attention_pulse                   |   10%  | 0.05       | 0.0950   |
| xi_robustness_v2                  |   15%  | 0.30       | 0.1050   |
|                                   |        | **baseline** | **~0.63** |

### 8.2 Dominant losses on baseline

1. `transfer_score` (0.138) — no transfer mechanism exists yet.
2. `temporal_separation` (0.135) — no frequency bands exist yet.
3. `xi_robustness_v2` (0.105) — new A5/A6 attacks are designed to be hard.
4. `attention_pulse` (0.095) — no 2 Hz dynamics exist yet.

These four losses sum to 0.473, or 75% of total baseline fitness. They
are all new-axis losses that require implementing the 2 Hz frequency band
infrastructure — confirming that L5 has reopened the optimization surface.

### 8.3 Targets

- **"Solved" target:** fitness < 0.10. This requires transfer_score > 0.6,
  temporal_separation > 0.8, attention_pulse > 0.7, xi_robustness > 0.7.
- **Stretch target:** fitness < 0.06. Requires all L5 metrics > 0.75.

### 8.4 Speed budget

Chain_depth = 16 with two corpora (A, B) plus adversarial passes:
- Corpus A: 16 dream cycles * 300 memories = 4800 consolidation rounds
- Corpus B: 16 dream cycles * 250 memories = 4000 consolidation rounds
- Adversarial pass: 16 dream cycles * ~340 memories = 5440 rounds
- Total: ~14,240 consolidation rounds

At L4's ~5ms per round, this is ~71 seconds. L5 should target < 60 seconds
in release mode. Speed at 3% weight means this budget is tight but not
dominant.

**Mitigation:** The 16-cycle chain can use short-circuit logic — skip
cycles where amplitude changes are below a threshold (chain quiescence
detection). This was proposed but never implemented in L4.

---

## 9. Implementation Roadmap

### Scaffold phase (cycles L5.1 - L5.7)

**L5.1 — Frequency band infrastructure.**
Add `working_frequency`, `storage_frequency`, `freq_decay_rate`,
`consolidation_threshold` to Params. Implement `classify_band(f)` and
`apply_freq_decay()` in the L5 harness. Build the frequency-aware corpus
generator for A (hardness=2 with frequency assignment). No metrics yet.
Verify that `cargo run --release --bin research -- --level 4` is unchanged.

**L5.2 — Corpus B generator and transfer harness.**
Add `build_corpus_l5_b()` with different seed + rotated sparse clusters.
Implement the 3-session L5 flow: (1) dream on A, save state; (2) load
state, inject B, dream, score — this IS the transfer pass; (3) dream on B
from scratch, score — this is the naive baseline. Wire `transfer_score` at
15% weight (everything else zero-weight). Smoke test.

**L5.3 — Online injection framework.**
Implement the injection schedule (section 4.4). Add mid-chain memory
injection after cycles 4, 7, 10, 12, 14. Wire `online_retention` and
`catastrophic_forgetting_resistance` at 10% + 10% weight.

**L5.4 — Temporal separation metric.**
Implement bimodality coefficient (Sarle's) over the frequency histogram.
Wire `temporal_separation` at 15% weight. Requires the freq_decay logic
from L5.1 to actually produce band separation.

**L5.5 — Attention pulse metric.**
Implement FFT of per-cycle amplitude-change signal. Wire `attention_pulse`
at 10% weight. Requires chain_depth >= 8; set default to 16.

**L5.6 — Kuramoto frequency coupling.**
Implement intra-band frequency synchronization. Add
`kuramoto_freq_coupling` param. This is the mechanism that should produce
the attention pulse — L5.5's metric should start responding after this
cycle.

**L5.7 — Adversarial v2 (A5 + A6) and xi_robustness_v2.**
Implement xi shadow generator and commutator saturation exploit generator.
Implement the demotion_rate component of xi_robustness_v2 (requires xi
re-ranking in the recall path). Wire at 15% weight. Verify total runtime
< 90 seconds.

### Metric completion phase (cycle L5.8)

**L5.8 — frequency_transfer + inherited core + weight finalization.**
Implement `frequency_transfer` metric (Pearson correlation of band
membership across corpora). Wire the 6 inherited L4 core metrics at
reduced weights. Validate all 13 metric weights sum to exactly 1.0 via
compile-time check. This is the last scaffolding commit.

### MVP baseline (cycle L5.9)

**L5.9 — FIRST REAL EXPERIMENT.**
Run 10 times, log to `research/results-L5.tsv`. This is the baseline row.
No params change from L4 best. Report should match the section 8 table
within ~20%.

### Tuning phase (cycles L5.10 onward)

**L5.10** — first tuning pass: `freq_decay_rate` +
`consolidation_threshold` for temporal_separation.
**L5.11** — second pass: `working_frequency` + `kuramoto_freq_coupling`
for attention_pulse.
**L5.12** — third pass: chain_depth + chain_carry_strength for
transfer_score (the chain must carry structure across corpora).
**L5.13+** — standard OODA loop on remaining axes.

### MVP definition

The smallest working L5 is everything through cycle **L5.9**: dual corpus,
frequency bands, online injection, temporal separation, attention pulse,
Kuramoto frequency coupling, adversarial v2, frequency transfer, inherited
core. Once L5.9 prints a fitness number and appends to `results-L5.tsv`,
L5 is open for optimization.

---

## 10. Open Questions (Nick, please decide before L5.1)

1. **Chain depth budget.** Design says chain_depth=16. This is 8x L4's
   chain_depth=2. Runtime scales linearly. At ~5ms/round * 14,240 rounds
   = ~71s, we exceed the "< 60s release" target before any tuning. Options:
   (a) Accept 90s runtime and weight speed at 3%.
   (b) Implement chain quiescence short-circuit to skip no-op cycles.
   (c) Reduce to chain_depth=12 and accept coarser FFT resolution.
   **Recommendation: (b) — quiescence detection is useful infrastructure
   and was already proposed in L4.**

2. **Frequency injection into production code.** L5's frequency band
   dynamics (freq_decay, consolidation_threshold) operate inside the
   research harness. But `default_frequency` is a real Params field used
   by production code. Should L5 modify the production `remember()` path
   to assign 2 Hz to new memories? Or keep it research-only?
   **Recommendation: research-only for now. Production frequency changes
   are a separate ADR.**

3. **Corpus A = L4 corpus?** Design uses `hardness: 2` for Corpus A,
   making it structurally similar but not identical to L4's `hardness: 1`.
   Alternative: reuse the exact L4 corpus as A, making L5 a strict
   superset of L4. This would let L4 baselines transfer but reduces the
   "novelty" of the transfer test.
   **Recommendation: hardness=2. Transfer from a known corpus is too easy
   — we want to test whether consolidation learns structure, not whether
   it memorizes L4.**

4. **Attention pulse FFT resolution.** With chain_depth=16, the FFT has
   only 16 points — very coarse. A 2 Hz pulse relative to what? The cycle
   rate is not in real-time seconds, it's in dream cycles. Should the
   "2 Hz" target be expressed in cycles (e.g., 2 peaks per 8 cycles) or
   in real time?
   **Recommendation: express in cycles. The metric measures whether
   consolidation is rhythmic, not whether it matches a wall-clock frequency.
   The biological parallel is metaphorical — what matters is that there IS
   a dominant frequency, and it's in the right ballpark relative to the
   chain length.**

5. **Xi re-ranking in recall path.** `xi_robustness_v2` requires the
   demotion_rate component, which needs xi-based re-ranking in the recall
   path. Currently no production recall path uses xi_diversity_boost (see
   xi-operator-audit.md, Item 3). Should L5 implement xi re-ranking in
   the research harness only, or also wire it into production recall?
   **Recommendation: research harness only. Wire it into a research-local
   `recall_with_xi_reranking()` function. Production wiring is a separate
   change gated on L5 results.**

6. **Adversarial v2 construction cost.** A5 (xi shadows) requires
   gradient-based search for vectors with matching xi signatures. This is
   a non-trivial computation. Should the adversarial set be pre-computed
   and cached, or computed fresh each run?
   **Recommendation: pre-computed, deterministic, cached alongside the
   corpus. Same principle as L4's adversarial set — fixed once landed.**

7. **Weight distribution.** Transfer_score and temporal_separation are
   co-dominant at 15% each. Should we bias toward one axis early?
   **Recommendation: keep equal. They are mechanistically coupled — the
   2 Hz band structure IS what makes transfer work.**

---

## 11. Acceptance Criteria

This design is ready to implement when:
- [ ] Nick answers the seven open questions in section 10.
- [ ] A new file `research/results-L5.tsv` header is drafted.
- [ ] The section 2 weight table sums to exactly 100% after any changes.
- [ ] The section 9 scaffold cycles each have explicit pre-condition and
      post-condition sentences (added at conversion to implementation).

---

## 12. References

1. Amichay, G., Rebelo, I., Garcia de la Chica, A., Lameira, A.R. &
   Ravignani, A. (2026). "Tempo of animal communication." *PLOS Biology*.
   April 2026. — The 2 Hz carrier frequency finding.

2. Odrzywolek, A. (2026). "Elementary Mathematics Language for symbolic
   regression." *arXiv:2603.21852v2*. — EML tree architecture used in
   L4.S0 and L4.S12 experiments.

3. ADR-0021: Chiral Mirror Architecture — Conscious/Subconscious HRM.
   `docs/adr/ADR-0021-chiral-mirror-architecture.md`. — The left/right
   hemisphere split that L5's frequency bands implement.

4. ADR-0020: Holographic Resonance Medium.
   `docs/adr/ADR-0020-holographic-resonance-medium.md`. — The tensor
   medium, Ghost Equation integration, persistence model.

5. The Ghost Equation: `dx/dt = f(x) - Iηx`. Source:
   `ShinobiGhostMagic/ghost/EQUATION.md`. — Driving force vs dampening
   dynamics. L5 identifies 2 Hz as the natural equilibrium frequency.

6. Al-Zawahreh, L. & Tassan, S. (2025). "Topological Obstructions in
   Computational Complexity." — Spectral-geometric framework referenced
   in ADR-0021 for the chiral fold motivation.

7. `experiments/xi-operator-audit.md` — Full audit of xi_diversity_boost
   callers and the linear-commutator degeneracy finding. L5's A5/A6
   attacks target the nonlinear fix.

8. `experiments/l4-s0-report.md` — Depth-3 EML symbolic regression
   confirming the hand-coded formula is not degenerate.

9. `experiments/l4-s12-report.md` — Depth-4 EML retry confirming snap
   collapse persists; upstream xi_operator is the bottleneck.

End of L5 design.
