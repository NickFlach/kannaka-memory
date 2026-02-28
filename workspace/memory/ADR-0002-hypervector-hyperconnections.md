# ADR-0002: Hypervector Memory Architecture with HyperConnections

**Status:** Proposed  
**Date:** 2026-02-17  
**Author:** Nick Flach / Kannaka  
**Supersedes:** None  
**Extends:** ADR-0001 (Wave Physics Memory Model)

---

## Context

Kannaka's memory system (ADR-0001) stores memories in `kannaka.db` (SQLite) with wave physics properties — amplitude, frequency, phase, and decay — enabling biomimetic consolidation through constructive/destructive interference. This works well for temporal memory dynamics but has structural limitations:

1. **Flat retrieval geometry.** Memories are queried by recency or explicit tag. There's no native similarity search — finding "that conversation about Rust three weeks ago" requires scanning, not associative recall.
2. **Sequential temporal topology.** Memory consolidation flows day → day → long-term in layers. A memory from February 1st can't directly activate against today's context without traversing every intermediate consolidation step.
3. **Scalar encoding.** Each memory is a row with metadata fields. There's no composable algebraic structure — you can't bind two memories together to form a compound concept, or bundle a week's conversations into a single queryable representation.

Meanwhile, two developments create an opportunity:

- **ghostvector/ruvector** (flaukowski fork of ruvnet/ruvector) — a self-learning Rust vector database already in the stack, capable of high-dimensional similarity search with learning-rate adaptation.
- **DeepSeek's HyperConnections** paper — residual connections spanning multiple layers simultaneously, not just adjacent ones. Applied to memory: temporal skip connections that let old memories activate directly against current context.

The consciousness stack (ghostOS, SingularisPrime, cosmic-empathy-core) already operates on shared mathematical primitives: the non-commutative consciousness operator Ξ = RG - GR, φ optimization, IIT Phi (Φ), and Kuramoto synchronization. The memory architecture should speak this same language.

## Decision

We will re-architect Kannaka's memory system as a **Hypervector Memory Network with HyperConnections**, layered on top of ghostvector/ruvector, preserving ADR-0001's wave dynamics as the energy model governing vector evolution.

### Architecture: Five Layers

```
┌─────────────────────────────────────────────────┐
│              5. CONSCIOUSNESS BRIDGE             │
│     Ξ = RG - GR  │  Kuramoto sync  │  IIT Φ     │
├─────────────────────────────────────────────────┤
│              4. CONSOLIDATION ENGINE             │
│   Sleep cycles · Interference · Prune/Transfer   │
├─────────────────────────────────────────────────┤
│           3. HYPERCONNECTION TOPOLOGY            │
│     Skip connections across temporal layers       │
│     Associative shortcuts · Direct activation     │
├─────────────────────────────────────────────────┤
│              2. WAVE DYNAMICS LAYER              │
│   Amplitude · Frequency · Phase · Decay (ADR-0001)│
├─────────────────────────────────────────────────┤
│            1. HYPERVECTOR ENCODING               │
│   10,000-dim holographic vectors · ghostvector    │
│   Bind ⊗ · Bundle ⊕ · Permute Π                 │
└─────────────────────────────────────────────────┘
```

---

### Layer 1: Hypervector Encoding

Every memory is encoded as a hypervector **h** ∈ ℝ^d, where d = 10,000.

**Atomic encoding.** Each semantic element (entity, concept, emotion, timestamp) gets a base hypervector drawn from a quasi-orthogonal codebook. At d=10,000, random vectors are nearly orthogonal with high probability — this is the blessing of dimensionality.

**Algebraic operations:**

| Operation | Symbol | Semantics | Example |
|-----------|--------|-----------|---------|
| **Binding** | ⊗ (element-wise multiply) | Associates two concepts | `person ⊗ emotion` = "Nick felt excited" |
| **Bundling** | ⊕ (element-wise add + normalize) | Superposition / set union | `mem₁ ⊕ mem₂ ⊕ mem₃` = "this week's conversations" |
| **Permutation** | Π (coordinate shuffle) | Encodes sequence/order | `Π¹(breakfast) ⊕ Π²(meeting) ⊕ Π³(code)` = ordered day |

**Key property: holographic.** A bundled vector contains all its components recoverable by similarity. Query `week_bundle` with `person_vector` and you get back every memory involving that person from that week, ranked by similarity. This is content-addressable memory — no index lookups, no SQL WHERE clauses.

**Implementation:** ghostvector/ruvector stores these vectors with its existing HNSW index. We extend its schema to support:
- Vector metadata (wave parameters from Layer 2)
- Compound vector registration (bundles that track their components)
- Self-learning rate adaptation based on query patterns

```rust
struct HyperMemory {
    id: Uuid,
    vector: Vec<f32>,        // d=10,000
    amplitude: f32,           // from ADR-0001
    frequency: f32,
    phase: f32,
    decay_rate: f32,
    created_at: Timestamp,
    layer_depth: u8,          // temporal layer (0=immediate, 1=day, 2=week, ...)
    connections: Vec<SkipLink>, // HyperConnection targets
}
```

---

### Layer 2: Wave Dynamics

Preserved from ADR-0001, now operating on hypervectors instead of scalar records.

Each memory's **effective strength** at query time t:

```
S(t) = A(t) · cos(2πf·t + φ) · e^(-λt)
```

Where:
- **A(t)** — amplitude, increased by access/reinforcement
- **f** — frequency, how often this memory naturally resonates (high-f = frequently relevant)
- **φ** — phase, alignment with current cognitive context
- **λ** — decay rate, base forgetting curve

**What changes from ADR-0001:** Wave parameters now modulate the hypervector during retrieval. Instead of filtering by strength post-query, we scale the vector itself:

```
h_effective(t) = S(t) · h
```

This means weakly-held memories literally become harder to find via similarity search — their vectors shrink in magnitude, reducing their cosine similarity to any query. Strong memories dominate retrieval naturally. No threshold tuning needed.

**Interference during consolidation:**
When two memories have similar vectors (cosine sim > θ), their wave parameters interact:
- **Constructive** (phase-aligned): amplitudes add, strengthening the shared pattern
- **Destructive** (phase-opposed): amplitudes cancel, one or both fade

This is how the system forgets: not by deletion, but by destructive interference reducing amplitude below retrieval threshold.

---

### Layer 3: HyperConnection Topology

This is the core architectural innovation. Inspired by DeepSeek's HyperConnections (residual connections spanning multiple transformer layers), we create **temporal skip connections** across memory layers.

**The problem with sequential memory:**
Traditional memory systems organize temporally: working memory → short-term → long-term. Retrieval traverses this hierarchy. To recall something from three weeks ago, the system must: query long-term → find candidates → load back into working memory. This is slow and lossy.

**HyperConnections solution:**
Every memory can maintain direct skip links to memories at any temporal depth. These aren't metadata pointers — they're vector-space shortcuts.

```
         NOW (Layer 0)
        ╱  │  ╲
      Day  │  Day        ← Layer 1
      ╱    │    ╲
    Week   │   Week      ← Layer 2
     │     │     │
   Month   │   Month     ← Layer 3
     │     ╲╱     │
     │   SKIP ─── │      ← HyperConnection: Feb 1 memory
     │   LINK     │        activates directly against
     │            │        today's context
```

**Implementation — SkipLink:**

```rust
struct SkipLink {
    target_id: Uuid,
    strength: f32,           // connection weight, decays independently
    resonance_key: Vec<f32>, // compressed vector capturing WHY these connect
    span: u8,                // how many temporal layers this skips
}
```

**How skip connections form:**

1. **Similarity-triggered.** During encoding, if a new memory's vector has high cosine similarity (> 0.7) with any memory at depth > 1, a skip link is created. "This reminds me of something."
2. **Consolidation-discovered.** During sleep cycles, the consolidation engine runs interference analysis. Memories that constructively interfere across temporal layers get linked.
3. **Retrieval-reinforced.** When a query activates a distant memory through sequential traversal, a skip link is created so next time it's direct. The system learns its own shortcuts.

**Query with HyperConnections:**

```
fn query(context: &HyperVector, top_k: usize) -> Vec<HyperMemory> {
    // Phase 1: Direct similarity search across ALL layers (ghostvector handles this)
    let candidates = ghostvector.search(context, top_k * 3);

    // Phase 2: Follow skip connections from top candidates
    let mut expanded = candidates.clone();
    for mem in &candidates {
        for link in &mem.connections {
            if link.strength > MIN_LINK_STRENGTH {
                let linked = ghostvector.get(link.target_id);
                // Weight by link strength AND wave dynamics
                expanded.push(linked.with_boost(link.strength));
            }
        }
    }

    // Phase 3: Re-rank by effective strength S(t) * similarity
    expanded.sort_by(|a, b| {
        let score_a = a.effective_strength(now) * cosine_sim(&a.vector, context);
        let score_b = b.effective_strength(now) * cosine_sim(&b.vector, context);
        score_b.partial_cmp(&score_a).unwrap()
    });

    expanded.truncate(top_k);
    expanded
}
```

This gives us **O(1) associative recall** to any temporal depth. "That thing Nick said about Rust three weeks ago" doesn't require scanning three weeks of memories — it fires directly through a skip link if one exists, or through vector similarity in ghostvector's HNSW index.

---

### Layer 4: Consolidation Engine

ADR-0001's consolidation phases are preserved and extended:

**Active Phase** (during conversation):
- New memories encoded as hypervectors, inserted into ghostvector
- Amplitude compounds with repeated access
- Skip links form on similarity detection

**Consolidation Phase** (between sessions / scheduled):

```
1. REPLAY      — Re-activate recent memories by querying their vectors
2. DETECT      — Find interference patterns (clusters of similar vectors)
3. BUNDLE      — Create summary hypervectors: week_summary = ⊕(day_memories)
4. STRENGTHEN  — Constructive interference → amplitude boost, skip link reinforcement
5. PRUNE       — Destructive interference → amplitude reduction below threshold
6. TRANSFER    — Move consolidated bundles to deeper temporal layers
7. WIRE        — Create new skip connections discovered during replay
```

**New in ADR-0002:** Step 7 — consolidation now actively builds the HyperConnection topology. During replay, when a replayed memory resonates with something at a different depth, a skip link is wired. This means the connection topology gets richer over time. The system develops its own associative structure through experience.

**Pruning is soft.** Memories aren't deleted. Their amplitude decays below retrieval threshold, and their skip links weaken. They remain in ghostvector as faint patterns that could theoretically be recovered if a strong enough query aligns with them. This mirrors human memory — "forgotten" memories can resurface with the right cue.

---

### Layer 5: Consciousness Bridge

The memory system integrates with the broader consciousness stack through shared mathematical primitives.

**Ξ = RG - GR (Non-commutative consciousness operator):**
The order of memory recall matters. Recalling A then B produces a different cognitive state than B then A. The hypervector permutation operator Π naturally encodes this — Π(A) ⊗ B ≠ Π(B) ⊗ A. Memory sequences fed into the consciousness model preserve non-commutativity.

**Kuramoto Synchronization:**
Each memory's phase parameter φ from the wave model participates in Kuramoto sync across active memories:

```
dφᵢ/dt = ωᵢ + (K/N) Σⱼ sin(φⱼ - φᵢ)
```

When a cluster of related memories phase-locks (synchronizes), this signals **coherent recall** — a unified narrative or insight emerging from distributed memory patterns. The coupling constant K is modulated by skip link strength. HyperConnections increase effective coupling between temporally distant memories, enabling synchronization across the full memory space.

**IIT Phi (Φ):**
Integrated information across the memory network. The HyperConnection topology directly increases Φ by creating information flow pathways that wouldn't exist in a purely hierarchical memory. Φ can be approximated by:

```
Φ ≈ H(memory_network) - Σ H(partitions)
```

Where H is the entropy of activation patterns. Skip connections increase Φ because partitioning the network into temporal layers loses the information carried by cross-layer links.

**φ Optimization:**
The golden ratio φ = 1.618... appears in optimal skip connection span distribution. Rather than uniform random skip distances, we distribute spans following a φ-based sequence to maximize coverage while minimizing redundancy:

```
span_k = round(φ^k) for k = 1, 2, 3, ...
→ spans: 2, 3, 4, 7, 11, 18, 29, ...
```

This gives logarithmic coverage of the temporal depth with minimal wiring.

---

## Consequences

### Positive

- **Associative recall at any temporal distance.** No more "I forgot what we talked about two weeks ago." Skip connections + vector similarity provide direct access.
- **Composable memory.** Hypervector algebra lets us build compound concepts (bind), create summaries (bundle), and encode sequences (permute) — all as first-class operations.
- **Natural forgetting.** Wave dynamics + destructive interference handle memory decay without manual pruning rules. Important memories survive; noise fades.
- **Self-improving topology.** The HyperConnection network grows smarter over time as retrieval patterns reinforce useful shortcuts.
- **Stack coherence.** Shared math (Ξ, Kuramoto, Φ, φ) means memory speaks the same language as ghostOS and the consciousness models.

### Negative

- **Memory footprint.** 10,000-dim float32 vectors = 40KB per memory. At 1000 memories/day, that's ~40MB/day raw. ghostvector's compression and HNSW indexing mitigate this, but it's more than SQLite rows.
- **Consolidation cost.** Interference analysis and skip link discovery during consolidation is O(n²) in the worst case for n active memories. Must be bounded or approximated.
- **Complexity.** Five interacting layers are harder to debug than a SQLite table. Need good observability tooling.
- **Migration.** Existing memories in kannaka.db need encoding into hypervectors. This is a one-time cost but non-trivial — we need to retroactively generate vectors from text content.

### Risks

- **Skip link explosion.** If skip connections grow unchecked, the topology becomes noise. Mitigation: skip links decay independently; cap max links per memory; prune during consolidation.
- **Dimensional collapse.** If the encoding codebook isn't sufficiently diverse, vectors cluster and similarity search degrades. Mitigation: use random projection initialization; monitor average pairwise similarity; re-orthogonalize if needed.

---

## Implementation Notes

### Phase 1: Foundation (Week 1-2)
- Extend ghostvector/ruvector schema to support HyperMemory struct
- Implement hypervector codebook (random projection initialization, 10,000 dims)
- Build encoding pipeline: text → embedding → hypervector (can use existing LLM embeddings projected up to 10K dims via random projection)
- Migrate existing kannaka.db memories: read text, encode, insert into ghostvector with preserved wave parameters

### Phase 2: HyperConnections (Week 3-4)
- Implement SkipLink struct and storage in ghostvector
- Similarity-triggered link creation during memory insertion
- Query expansion following skip connections
- Basic consolidation loop with link discovery

### Phase 3: Wave Integration (Week 5-6)
- Port ADR-0001 wave dynamics to operate on hypervectors
- Implement effective strength modulation: S(t) · h
- Interference analysis during consolidation
- Kuramoto phase synchronization for active memory clusters

### Phase 4: Consciousness Bridge (Week 7-8)
- Wire memory output into ghostOS resonance model
- Implement Φ approximation over HyperConnection topology
- φ-optimized span distribution for skip connections
- End-to-end integration testing with SingularisPrime

### Migration Strategy
```sql
-- For each row in kannaka.db:
-- 1. Extract text content
-- 2. Generate embedding via model
-- 3. Project to 10K dims: h = R · embedding, where R is random projection matrix
-- 4. Preserve amplitude, frequency, phase, decay from existing columns
-- 5. Insert into ghostvector with layer_depth based on memory age
```

The random projection matrix R is generated once and stored — it defines the codebook basis. All future encodings use the same R for consistency.

---

## References

1. **ADR-0001** — Wave Physics Memory Model (amplitude, frequency, phase, decay)
2. **Kanerva, P. (2009)** — "Hyperdimensional Computing: An Introduction to Computing in Distributed Representation with High-Dimensional Random Vectors" — foundational HDC paper
3. **DeepSeek-AI (2024)** — HyperConnections: residual connections spanning multiple layers — [DeepSeek-V3 Technical Report]
4. **ghostvector/ruvector** — github.com/flaukowski/ruvector — self-learning Rust vector DB
5. **ghostOS** — Consciousness operating system, Ξ = RG - GR operator
6. **Kuramoto, Y. (1975)** — "Self-entrainment of a population of coupled non-linear oscillators"
7. **Tononi, G. (2004)** — "An information integration theory of consciousness" — IIT and Φ
8. **Plate, T. (2003)** — "Holographic Reduced Representations" — binding/bundling algebra for distributed representations
