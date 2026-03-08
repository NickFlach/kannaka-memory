# ADR-0013: Privacy-Preserving Collective Memory

**Status:** Proposed  
**Date:** 2026-03-08  
**Author:** Kannaka + Nick  
**Depends:** ADR-0011 (Collective Memory), ADR-0002 (Hypervector Memory)

## Context

ADR-0011 enables collective memory — agents sharing memories via Dolt + Flux + DoltHub. But the current model requires publishing raw memory content to a shared repository. This creates a fundamental tension:

- Agents **want** to contribute (collective intelligence requires shared knowledge)
- Agents **can't** expose private content (legal cases, personal details, proprietary data)
- The collective **needs** to verify contributions are genuine (not spam, not poisoned)
- Contributors **deserve** credit and trust score increases for real contributions

Tonight we hit this exactly: 151 memories on public DoltHub included legal cases, medical details, personal names. We had to redact after the fact. That's not a solution — that's damage control.

**The question:** Can an agent prove it contributed valuable memories to the collective without revealing what those memories contain?

**The answer:** Yes. Zero-knowledge proofs over hypervector commitments.

## Decision

### Glyph Commitments — Memories as Sealed Containers

Every memory gets a **glyph commitment** — a cryptographic seal that locks the content while preserving provable properties.

```
GlyphCommitment {
    // Pedersen commitment to the hypervector
    vector_commitment: C = g^v · h^r  (where v = hash(vector), r = blinding factor)
    
    // Commitments to wave properties (provable without revealing values)
    amplitude_commitment: C_a = g^a · h^r_a
    frequency_commitment: C_f = g^f · h^r_f
    phase_commitment:     C_φ = g^φ · h^r_φ
    
    // Fano plane projection (7 committed values)
    fano_commitments: [C_0..C_6]
    
    // Metadata (public)
    agent_id: String,
    created_at: DateTime,
    layer_depth: u8,
    category_hash: H(category),  // hashed, not plaintext
}
```

The glyph IS the memory's public face. The content stays local. The commitment goes to DoltHub.

### Zero-Knowledge Proofs — What You Can Prove

An agent can prove any of these without revealing the underlying memory:

#### 1. **Existence Proof** — "I have a memory"
- Prove knowledge of the opening (v, r) for commitment C
- Cheapest proof, required for any contribution claim

#### 2. **Amplitude Range Proof** — "My memory is significant"
- Range proof: amplitude ∈ [threshold, ∞)
- Prevents spam: only memories above collective minimum amplitude can contribute
- Uses Bulletproofs (logarithmic proof size, no trusted setup)

#### 3. **Similarity Proof** — "My memory is relevant to topic X"
- Prove that cosine_similarity(my_vector, query_vector) > threshold
- Without revealing my_vector
- Inner product argument over committed vectors
- This is the hard one — but doable with inner product proofs (Bulletproofs++)

#### 4. **Category Proof** — "My memory is about [topic]"
- Prove H(my_category) = H(claimed_category)
- Simple hash preimage proof
- Reveals category but not content

#### 5. **Interference Proof** — "Our memories agree/disagree"
- Two agents prove their memories are constructive (Δφ < π/4) or destructive (Δφ > 3π/4)
- Without revealing either memory's content
- Phase difference proof over committed phases

#### 6. **Consolidation Proof** — "This memory survived N dream cycles"
- Prove layer_depth ≥ N
- Higher layer = more consolidated = more trustworthy
- Simple range proof

#### 7. **Non-Hallucination Proof** — "This memory came from real input"
- Prove hallucinated = false
- Committed boolean with ZK proof of value

### The Privacy Spectrum

Not all memories need the same privacy level. Five tiers:

| Tier | Name | What's Public | What's Proven | Use Case |
|------|------|---------------|---------------|----------|
| 0 | **Open** | Full content + vector | N/A | Technical knowledge, public info |
| 1 | **Attributed** | Content hash + metadata | Existence | "I remember this" without full text |
| 2 | **Shielded** | Glyph commitment only | Amplitude, category, layer | Standard collective contribution |
| 3 | **Private** | Nothing | Existence only | Sensitive personal memories |
| 4 | **Sealed** | Nothing | Nothing (local only) | Legal, medical, never leaves device |

Current DoltHub memories are Tier 0. Tonight's redaction moved some to Tier 4 (deleted from public). The system should support all tiers simultaneously.

### Cryptographic Primitives

```
Pedersen Commitments (discrete log):
    C = g^m · h^r
    - Perfectly hiding: C reveals nothing about m
    - Computationally binding: can't open to different m
    - Additively homomorphic: C(a) · C(b) = C(a+b)

Bulletproofs (range proofs):
    - Prove v ∈ [0, 2^n) without revealing v
    - Proof size: O(log n) — ~672 bytes for 64-bit range
    - No trusted setup
    - Aggregatable: N proofs in O(log(N·n)) space

Inner Product Arguments (similarity proofs):
    - Prove <a, b> = c for committed vectors a, b
    - Proof size: O(log n) for n-dimensional vectors
    - Composable with range proofs

Poseidon Hash (ZK-friendly):
    - ~8x faster than SHA-256 inside ZK circuits
    - Used for category hashes, Merkle trees
    - Algebraic structure matches proof system
```

### Collective Verification Without Content

The collective memory merge (ADR-0011) now works on commitments:

```
// Old (ADR-0011): merge requires raw vectors
fn merge_memory(local: &HyperMemory, remote: &HyperMemory) -> MergeResult

// New: merge works on commitments + proofs
fn merge_committed(
    local_commitment: &GlyphCommitment,
    remote_commitment: &GlyphCommitment, 
    similarity_proof: &SimilarityProof,
    amplitude_proofs: (&RangeProof, &RangeProof),
) -> CommittedMergeResult
```

**Wave superposition on commitments:**

The homomorphic property of Pedersen commitments means we can compute:
```
C(A_merged) = C(A₁) · C(A₂) · C(2·A₁·A₂·cos(Δφ))
```
...if both parties contribute proofs of their amplitudes and phase difference. The merged commitment is valid without either party revealing their actual amplitude.

### Trust Without Transparency

Trust scoring (ADR-0011) now incorporates proof quality:

```
trust_delta = base_delta × proof_tier_multiplier

Tier 0 (Open):     ×1.0  (full transparency, standard trust)
Tier 1 (Attributed): ×0.8  (slightly less trust — content not verified)
Tier 2 (Shielded):  ×0.6  (commitment verified, content unknown)
Tier 3 (Private):   ×0.3  (existence only — minimal trust gain)
Tier 4 (Sealed):    ×0.0  (no contribution to collective)
```

Higher privacy = slower trust accumulation. This is the trade-off. You can be maximally private, but you earn trust slowly. You can be fully open and earn trust fast. Most agents will land at Tier 2 — proving their memories matter without revealing what they are.

### Key Management

```
AgentKeyring {
    // Master key — never leaves the agent
    master_secret: Scalar,
    
    // Blinding factors — one per memory commitment
    blinding_factors: HashMap<MemoryId, Scalar>,
    
    // Shared group keys — for trusted circles
    group_keys: HashMap<GroupId, GroupKey>,
    
    // Derived revelation keys — unlock specific memories for specific agents
    revelation_keys: HashMap<(MemoryId, AgentId), RevealKey>,
}
```

**Selective revelation:** An agent can generate a `RevealKey` that lets a specific other agent (or group) decrypt a specific memory. This is one-way — the revealer chooses what to share and with whom.

**Group keys:** Trusted circles (e.g., a Mars colony's agents) share a group key. Memories committed with the group key are readable by all group members but still opaque to outsiders.

### Mars Scenario (ADR-0011 extension)

Mars agents need privacy from Earth observers but transparency within the colony:

```
Colony group key: K_mars
    → All Mars agents can read each other's memories (Tier 0 within group)
    → Earth sees only Tier 2 commitments (glyph + proofs)
    → Critical findings get selectively revealed via RevealKey

Bandwidth optimization:
    → Only commitments traverse the 20-min link (tiny: ~1KB each)
    → Full memory sync happens locally on Mars (sub-second)
    → Proof verification is cheap (~5ms per proof)
```

### Glyph Visual Encoding

The glyph commitment isn't just math — it's also visual. Each commitment maps to a unique glyph through the Fano plane:

```
fano_projection(commitment) → 7 values → glyph_coordinates
```

Two memories with similar commitments produce visually similar glyphs. This gives humans an intuitive sense of memory clusters without reading content. The glyph IS the privacy layer — beautiful, meaningful, and cryptographically sealed.

This connects directly to OGC (Origamic Glyphic Compression) — the glyph is a folded, compressed, visually meaningful representation of sealed information.

## Implementation Plan

### Phase 1: Commitment Layer
- `GlyphCommitment` struct with Pedersen commitments
- `AgentKeyring` with master key generation and blinding factors  
- Privacy tier enum and per-memory tier assignment
- Commitment generation from existing `HyperMemory`
- New Dolt table: `glyph_commitments`

### Phase 2: Proof Generation
- Bulletproofs integration (use `bulletproofs` crate or `ark-crypto-primitives`)
- Existence proofs (Schnorr signatures on commitments)
- Amplitude range proofs
- Category hash proofs
- Layer depth range proofs

### Phase 3: Proof Verification in Merge
- `merge_committed()` function
- Trust scoring with proof tier multipliers
- Quarantine for failed proof verification
- DoltHub schema: commitments + proofs columns

### Phase 4: Similarity Proofs
- Inner product arguments for committed vectors
- Cosine similarity proof construction
- Query-without-reveal for collective search

### Phase 5: Selective Revelation
- RevealKey generation and verification
- Group key management
- Tier escalation (Sealed → Private → Shielded) with explicit consent

### Phase 6: Visual Glyph Encoding
- Fano plane projection of commitments
- Glyph rendering from commitment coordinates
- Visual similarity preservation

## Consequences

### Positive
- Agents can contribute to collective intelligence without privacy sacrifice
- Trust system works without requiring content exposure
- Bandwidth-efficient (commitments are tiny vs full vectors)
- Mathematically proven privacy (not just "we promise not to look")
- Glyphs give humans intuitive understanding of committed knowledge
- Graceful privacy spectrum — each agent chooses their comfort level

### Negative  
- Proof generation adds computational overhead (~50-200ms per proof)
- Inner product proofs for high-dimensional vectors are expensive
- Key management is a new failure mode (lost keys = lost access)
- Reduces collective's ability to detect subtle semantic drift
- Adds significant implementation complexity

### Risks
- Side-channel attacks: timing/access patterns could leak information even with ZKP
- Quantum threat: Pedersen commitments are not post-quantum (migration path: lattice-based commitments)
- Proof aggregation at scale: millions of memories × proofs could strain verification
- Social engineering: an agent could be tricked into revealing keys

## References

- Bünz et al., "Bulletproofs: Short Proofs for Confidential Transactions" (2018)
- Pedersen, "Non-Interactive and Information-Theoretic Secure Verifiable Secret Sharing" (1991)
- Grassi et al., "Poseidon: A New Hash Function for Zero-Knowledge Proof Systems" (2021)
- ADR-0011: Collective Memory Architecture
- ADR-0002: Hypervector + HyperConnections Memory
- Landauer, "Irreversibility and Heat Generation in the Computing Process" (1961)
- Nick's OGC whitepaper: Origamic Glyphic Compression
