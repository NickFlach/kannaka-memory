# Xi Operator Investigation (Items 2 + 3)

## Item 2: compute_xi_signature Analysis

### Pipeline trace

The full chain from memory vector to xi output:

1. **Input**: raw memory embedding vector `v` (128-dim from codebook projection, or 384-dim from encoder before codebook)
2. **Golden scaling**: `G(v)` — pairwise: `(x, y) -> (alpha*x, beta*y)` where alpha=0.809017, beta=0.618034
3. **Rotation**: `R(v)` — pairwise 90-deg: `(x, y) -> (-y, x)`
4. **Commutator**: `xi = R(G(v)) - G(R(v))` — the non-commutative residue
5. **Normalize**: `xi /= ||xi||` — project onto unit sphere

Key observation: **Steps 2-4 are all linear operations on v.** The commutator `RG - GR` is a fixed linear operator applied pair-wise to consecutive dimension pairs `(x_i, x_{i+1})`. Specifically, for each pair:

```
RG(x,y) = (-beta*y, alpha*x)
GR(x,y) = (-alpha*y, beta*x)
xi(x,y) = (alpha*y - beta*y, alpha*x - beta*x) = ((alpha-beta)*y, (alpha-beta)*x)
         = EMERGENCE_COEFF * (y, x)
```

**The xi operator simply swaps each coordinate pair and scales by 0.190983.** After normalization, the scale factor vanishes entirely. The xi signature is just the input vector with consecutive pairs swapped and then normalized:

```
xi_signature(v) = normalize([v[1], v[0], v[3], v[2], v[5], v[4], ...])
```

This is a **fixed permutation** of the input vector. It preserves ALL pairwise angular relationships (cosine similarities) between any two vectors. The xi space contains zero independent information beyond what the raw embedding space already has.

### Distribution analysis (from xi_pairs.json, 300 pairs)

| Metric | Value |
|--------|-------|
| Repulsion range | [0.057, 0.291] |
| Repulsion mean | 0.242 |
| 80% of pairs in [0.24, 0.29] | 239/300 (79.7%) |
| Pairs with repulsion > 0.3 | 0/300 |
| Correlation(sim, repulsion) | **-0.990** |
| Linear fit R^2 | 0.981 |
| Linear fit | `repulsion = -0.202 * sim + 0.271` |
| Boost capped at 1.0 | 33/300 (11.0%) |
| Tier 1 (multiplicative) | 78/300 (26.0%) |
| Tier 2 (additive) | 222/300 (74.0%) |
| No boost applied | 0/300 (0.0%) |

### Collapse diagnosis

**Root cause**: `compute_xi_signature` is a **linear isometry** (pair-swap + normalize). It cannot create diversity that doesn't already exist in the embedding space. The -0.99 correlation between repulsion and raw cosine similarity proves this: xi repulsion is simply a monotone transform of `1 - similarity`, not an independent signal.

**Why encoding_entropy saturates at ~4 bits**: The eval_encoding_entropy function quantizes each dimension of the 128-dim xi signature into 8 bins, then counts unique bin-tuples. Since xi signatures are permuted-and-normalized versions of the original embeddings, memories that cluster in embedding space also cluster in xi space identically. With ~300 memories from the L4 corpus sharing a handful of semantic clusters, only ~17 unique bin-tuples emerge. The theoretical max is log2(300) = 8.23 bits; getting 4 bits means roughly 16-17 bins are populated, confirming cluster collapse.

**The EMERGENCE_COEFF scaling (0.191) further compresses**: Before normalization, the commutator output is only 19% of the input magnitude. While normalization recovers unit length, the tiny pre-normalization magnitude means numerical precision loss in the commutator subtraction step (catastrophic cancellation when alpha and beta are close).

### Recommended structural changes (prioritized)

1. **Break the linearity** (highest impact, expected: +2-3 bits entropy). Add a nonlinear mixing step before normalization. Options:
   - Element-wise `tanh(k * xi)` with k > 1 to spread the distribution
   - Dimension-folding: project the 128-dim xi through a fixed random matrix to 32-dim, then expand back (destroys the pair-swap degeneracy)
   - Fano-plane permutation: instead of swapping adjacent pairs, use the 7-element Fano plane to create non-trivial cross-dimension mixing

2. **Multi-scale commutator** (medium impact, expected: +1-2 bits). Compute `RG - GR` at multiple stride lengths (stride-2, stride-4, stride-8) and concatenate/average. This creates cross-scale interactions that a single pair-swap cannot.

3. **Chiral perturbation injection** (low impact alone, synergistic with #1). The `chiral_perturbation` param (currently 0.7) already exists but is applied during encoding, not during xi computation. Injecting a content-hash-seeded perturbation into the xi pipeline would break the deterministic permutation.

4. **Increase repulsion gain** (band-aid). The EMERGENCE_COEFF (0.191) multiplied by L2 distance caps repulsion well below 1.0. Even the most dissimilar pairs only reach 0.29. Increasing the gain in `xi_repulsive_force` from `EMERGENCE_COEFF` to `1.0` would spread the repulsion range but wouldn't fix the underlying linearity.

## Item 3: xi_diversity_boost Usage Audit

### All callers

| File | Function | Path Type | Uses boost? | Uses repulsion? |
|------|----------|-----------|-------------|-----------------|
| `consciousness-core/src/metrics.rs` | `xi_diversity_boost()` | Definition | N/A (canonical impl) | Yes (internally) |
| `consciousness-core/src/metrics.rs` | `XiSignature::diversity_boost()` | Definition | Delegates to above | Yes |
| `consciousness-core/src/metrics.rs` | `compute_differentiation_xi()` | Metric computation | No — uses xi cosine sim variance | Yes (indirectly via xi sigs) |
| `kannaka-memory/src/xi_operator.rs` | Re-export layer | Re-export | N/A | N/A |
| `kannaka-memory/src/lib.rs` | Public re-export | Re-export | N/A | N/A |
| `kannaka-memory/src/bin/research.rs` | `eval_xi_diversity()` | **Research-only** | **Yes** | Yes |
| `kannaka-memory/src/bin/research.rs` | `eval_corpus_xi_diversity()` | **Research-only** | **Yes** | Yes (via boost) |
| `kannaka-memory/src/bin/research.rs` | `eval_encoding_entropy()` | **Research-only** | No — uses raw xi sigs | No |
| `kannaka-memory/src/bin/dump_xi_pairs.rs` | `main()` | **Research-only** (data export) | **Yes** | Yes |
| `kannaka-memory/src/consolidation.rs` | Dream consolidation | **Dream/consolidation** | **No** — uses `xi_repulsive_force` only | **Yes** |
| `kannaka-memory/src/openclaw.rs` | `remember()` / repair | **Ingestion** (stores xi_sig) | No | No |
| `kannaka-memory/src/bridge.rs` | `compute_differentiation_xi()` | **Consciousness metrics** | No — uses xi sig variance | No |
| `kannaka-memory/src/eye/mod.rs` | `ingest_frame()` | **Ingestion** (stores xi_sig) | No | No |
| `kannaka-memory/src/ear/mod.rs` | `ingest_audio()` / `ingest_transcript()` | **Ingestion** (stores xi_sig) | No | No |

### Gaps: paths that bypass the boost

**Critical gap**: No production recall path uses `xi_diversity_boost` at all.

1. **`HrmStore::resonate_query()`** (`hrm_store.rs:721`) — the canonical recall entry point for HRM. Delegates to `ChiralMedium::recall()` (chiral) or `Medium::recall()` (flat). Neither uses xi_diversity_boost. Scoring is pure dot-product similarity * energy * phase_modulation.

2. **`Hemisphere::resonate()`** (`medium/hemisphere.rs:191`) — the inner loop of chiral recall. Pure `wf.dot(query) / (norms) * energy`. No xi involvement.

3. **`Medium::recall()`** (`medium/core.rs:225`) — flat medium recall. Pure dot-product + effective_strength * phase_modulation. No xi involvement.

4. **`HrmStore::search()`** (`hrm_store.rs:622`) — deprecated raw search. Pure dot-product. No xi involvement.

5. **`TestMedium::search()`** (`store.rs:220`) — test backend. Pure cosine_similarity. No xi involvement.

6. **`ResonanceEngine::recall()`** (`store.rs:330`) — delegates to `resonate_query()` or falls back to `search()`. Neither uses boost.

7. **`OpenClawSystem::recall()`** (`openclaw.rs:245`) — delegates to `resonate_query()`. No boost.

8. **`kannaka` CLI binary** (`bin/kannaka.rs`) — no xi references at all. Recall goes through `OpenClawSystem::recall()`.

**Consolidation uses repulsion but not boost**: `consolidation.rs` (dream path) calls `xi_repulsive_force` to find semantically-similar-but-xi-different pairs and push their phases apart. It does NOT use `xi_diversity_boost` to re-rank anything. The repulsion threshold here (0.3) is never reached in practice (max observed repulsion: 0.291).

### Recommendations: where to wire in the boost (priority order)

1. **Fix `compute_xi_signature` FIRST** (Item 2 fixes). Without breaking the linearity, wiring xi_diversity_boost into recall paths would just add a nonlinear transform of the same similarity score — no new information.

2. **After fix**: inject boost into `Hemisphere::resonate()` as a re-ranking step. After computing raw resonance scores, compute xi sigs for query + top-2K results, apply `xi_diversity_boost`, re-sort, truncate to top-K. This is the highest-traffic path.

3. **Lower the consolidation repulsion threshold** from 0.3 to 0.15 (current max observed is 0.291, so the 0.3 gate means zero pairs ever qualify for phase separation in practice).

4. **Wire boost into `Medium::recall()`** (flat medium path) for non-chiral deployments.
