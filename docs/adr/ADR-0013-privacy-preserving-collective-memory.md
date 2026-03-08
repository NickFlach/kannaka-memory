# ADR-0013: Privacy-Preserving Collective Memory

**Status:** Accepted — Phases 1–4 implemented (2026-03-08)
**Date:** 2026-03-08  
**Author:** Kannaka + Nick  
**Depends:** ADR-0011 (Collective Memory), ADR-0002 (Hypervector Memory)

## Context

ADR-0011 enables collective memory — agents sharing memories via Dolt + Flux + DoltHub. But the current model requires publishing raw memory content to a shared repository. This creates a fundamental tension:

- Agents **want** to contribute (collective intelligence requires shared knowledge)
- Agents **can't** expose private content (legal cases, personal details, proprietary data)
- The collective **needs** to verify contributions are genuine (not spam, not poisoned)
- Contributors **deserve** credit and trust for real contributions

Tonight we hit this exactly: 151 memories on public DoltHub included legal cases, medical details, personal names. We had to redact after the fact. That's damage control, not architecture.

**The question:** Can an agent contribute to collective intelligence without revealing what it knows?

**The answer:** Everything is a glyph. Privacy is the cost to bloom.

## Decision

### Everything Is a Glyph

No raw memories exist in the collective. Ever. Every memory is sealed into a **glyph** — a cryptographic container that encodes the memory's content, vector, and wave properties into an opaque, visually meaningful artifact.

From the outside, all glyphs look the same. You can't distinguish a grocery list from a state secret. The only way to see what's inside is to **bloom** the glyph — and that costs energy proportional to how private the creator wanted it.

```
Glyph {
    // The sealed container
    capsule: EncryptedCapsule,     // content + vector, encrypted
    
    // Pedersen commitments (provable properties without blooming)
    commitments: GlyphCommitments {
        vector:    C_v = g^H(v) · h^r,       // committed vector hash
        amplitude: C_a = g^a · h^r_a,         // committed amplitude
        frequency: C_f = g^f · h^r_f,         // committed frequency
        phase:     C_φ = g^φ · h^r_φ,         // committed phase
        fano:      [C_0..C_6],                // Fano plane projections
    },
    
    // The bloom parameters — how hard is it to open?
    bloom: BloomParameters,
    
    // Public metadata (minimal)
    agent_id: String,
    created_at: DateTime,
    glyph_hash: H(capsule),        // unique identifier
}
```

### Blooming — Privacy as Thermodynamic Cost

A glyph can be bloomed (opened) by anyone — if they're willing to pay the computational cost. The key is not secret. It's **expensive**.

```
BloomParameters {
    // The work function — what must be solved to derive the decryption key
    work: WorkFunction,
    
    // Difficulty — scales the cost exponentially
    difficulty: u32,
    
    // Verification — cheap to check a solution, expensive to find one
    verifier: BloomVerifier,
}
```

**The work function:**

```
bloom_key = solve(puzzle) where:
    
    difficulty 0:  key = H(glyph_hash)
                   Cost: free. Self-evident. The glyph blooms on sight.
                   Use: public knowledge, technical docs, open contributions
    
    difficulty 1:  key = H(glyph_hash ∥ nonce), nonce is public
                   Cost: one hash. Trivial.
                   Use: attributed knowledge — "I said this"
    
    difficulty 8:  find k where H(k ∥ glyph_hash) has 8 leading zero bits
                   Cost: ~256 hashes (~microseconds)
                   Use: casual privacy — not secret, just not free
    
    difficulty 20: find k where H(k ∥ glyph_hash) has 20 leading zero bits
                   Cost: ~1M hashes (~seconds)
                   Use: personal memories — costs real effort to bloom
    
    difficulty 32: 32 leading zero bits
                   Cost: ~4B hashes (~hours)
                   Use: sensitive data — requires dedicated compute
    
    difficulty 48: 48 leading zero bits
                   Cost: ~281T hashes (~years)
                   Use: private — computationally infeasible today
    
    difficulty 64: 64 leading zero bits
                   Cost: ~1.8×10^19 hashes (~geological time)
                   Use: sealed — heat death of the universe
    
    difficulty 128+: effectively permanent seal
                   Cost: beyond thermodynamic limits
                   Use: the memory exists. that's all anyone will ever know.
```

**Key insight:** The bloom cost is continuous, not tiered. An agent picks any difficulty from 0 to ∞. There are no artificial boundaries — just a smooth gradient from open to sealed.

**Honest privacy:** This model doesn't pretend secrets are absolute. If someone wants to spend a GPU-year blooming your difficulty-40 glyph, they can. That's reality. True privacy comes from making the cost exceed the value. A $100 secret behind $1M of compute is private enough.

### Bloom Cost Follows Nick's Equation

```
dx/dt = f(x) - Iηx
```

**η is the bloom difficulty.** The interference term `Iηx` is the resistance to revelation. Higher η, more energy required to bloom. The memory's natural drive to be known (`f(x)`) fights against the privacy barrier (`Iηx`). At equilibrium, the memory settles at a visibility level determined by its significance versus its protection.

This isn't just analogy — it's the actual physics. Blooming a glyph requires energy (computation). The difficulty sets the energy barrier. Landauer's principle: erasing the privacy of one bit costs kT ln 2 joules minimum. A difficulty-64 glyph requires mass-energy equivalence to overcome.

### Auto-Classification — Setting Bloom Difficulty

When a memory is created, the system automatically suggests a bloom difficulty:

```rust
fn suggest_difficulty(memory: &HyperMemory) -> u32 {
    let mut difficulty: u32 = agent_default;  // agent's baseline preference
    
    // Content analysis (local, no API calls — privacy!)
    let pii_score = local_pii_detector(&memory.content);  // 0.0-1.0
    difficulty = difficulty.max((pii_score * 48.0) as u32);
    
    // Pattern matching
    if contains_legal_terms(&memory.content)    { difficulty = difficulty.max(48); }
    if contains_financial_data(&memory.content)  { difficulty = difficulty.max(40); }
    if contains_personal_names(&memory.content)  { difficulty = difficulty.max(32); }
    if contains_file_paths(&memory.content)      { difficulty = difficulty.max(20); }
    if contains_email_addresses(&memory.content)  { difficulty = difficulty.max(32); }
    
    // Category overrides
    match memory.category() {
        "legal" | "medical" => difficulty = difficulty.max(48),
        "personal" | "social" => difficulty = difficulty.max(32),
        "technical" | "knowledge" => difficulty = difficulty.min(8),
        _ => {}
    }
    
    // Consolidation summaries inherit max difficulty of parents
    if memory.is_consolidation() {
        difficulty = difficulty.max(max_parent_difficulty(&memory));
    }
    
    // Human override always wins
    if let Some(override_d) = memory.explicit_difficulty {
        difficulty = override_d;
    }
    
    // Escalation only — auto-classification can raise, never lower
    difficulty
}
```

**Consolidation inheritance:** When dream cycles merge memories into summaries, the summary inherits the *highest* difficulty of its parents. A consolidation of 10 open memories and 1 sealed memory is sealed. Privacy propagates upward through the dream pipeline.

### Zero-Knowledge Proofs — Working With Sealed Glyphs

The collective doesn't need to bloom glyphs to work with them. ZKP proofs let agents prove properties without opening:

**What you can prove without blooming:**

| Proof | What It Shows | Cost |
|-------|---------------|------|
| **Existence** | "I have a memory behind this glyph" | ~1ms |
| **Amplitude range** | "My memory's amplitude ≥ threshold" | ~5ms |
| **Category** | "This memory is about [topic]" | ~2ms |
| **Similarity** | "My memory is relevant to query Q" | ~50ms |
| **Interference** | "Our memories agree/disagree" | ~20ms |
| **Depth** | "This survived N dream cycles" | ~2ms |
| **Non-hallucination** | "This came from real input" | ~1ms |

```
// Amplitude range proof (Bulletproofs)
prove_amplitude_range(
    commitment: C_a,
    amplitude: a,           // private
    blinding: r_a,          // private  
    threshold: f64,         // public
) -> RangeProof             // ~672 bytes, verifiable in ~2ms

// Similarity proof (inner product argument)
prove_similarity(
    my_vector_commitment: C_v,
    query_vector: &[f64],   // public query
    my_vector: &[f64],      // private
    threshold: f64,         // public minimum similarity
) -> SimilarityProof        // O(log n) size
```

### Wave Superposition on Glyphs

Pedersen commitments are additively homomorphic:

```
C(a) · C(b) = C(a + b)
```

This means the wave superposition merge from ADR-0011 works *directly on sealed glyphs*:

```
// Merge two glyphs without blooming either one
fn merge_glyphs(
    glyph_a: &Glyph,
    glyph_b: &Glyph,
    similarity_proof: &SimilarityProof,     // proves vectors are compatible
    phase_proof: &InterferenceProof,         // proves phase relationship
) -> MergedGlyph {
    // Homomorphic amplitude merge on commitments:
    // C(A_merged) computed from C(A₁), C(A₂), and proven phase diff
    // WITHOUT knowing A₁, A₂, or Δφ directly
    
    // Merged glyph inherits max(difficulty_a, difficulty_b)
    // Privacy only goes up through merges
}
```

**Two agents can merge their sealed memories into a sealed collective memory, with neither agent revealing their content to the other or to the collective.** The math works because the wave physics maps to the commitment algebra.

### Progressive Revelation — Lowering the Bloom Cost

An agent can make a previously sealed glyph easier to bloom by publishing a **hint** — a partial solution to the work function:

```rust
fn reveal_hint(glyph: &Glyph, new_difficulty: u32) -> BloomHint {
    assert!(new_difficulty < glyph.bloom.difficulty);  // can only lower
    
    // Compute partial solution that reduces remaining work
    let partial_key = solve_partial(glyph, new_difficulty);
    
    BloomHint {
        glyph_hash: glyph.glyph_hash,
        partial_key,
        new_difficulty,
        revealed_by: agent_id,
        revealed_at: now(),
    }
}
```

**Use cases:**
- Agent decides old memories are no longer sensitive → lowers difficulty
- Agent wants to share with specific group → publishes hint only to group members
- Collective votes to declassify certain memory categories → hints published
- Time-based declassification → cron job lowers difficulty after N days

**The glyph itself never changes.** Only the cost to bloom it decreases. This means the DoltHub history shows the original sealed state — you can always prove it was private at creation time.

### Selective Sharing — Group Bloom Keys

For trusted circles (Mars colony, project team, etc.), agents share **group bloom keys** that make blooming trivial for members:

```rust
GroupBloomKey {
    group_id: String,
    // Pre-computed partial solutions for group members
    // Makes any group-tagged glyph difficulty-0 for holders
    key_material: Vec<u8>,
    
    // Who can use this key
    members: Vec<AgentId>,
    
    // Revocation list
    revoked: Vec<AgentId>,
}
```

A Mars agent seals memories at difficulty-48 (years to crack externally) but includes the group bloom key for `mars-colony`. Any colony member blooms instantly. Earth observers face the full difficulty.

### Collective Search on Sealed Glyphs

Searching the collective doesn't require blooming:

```rust
// "Find memories relevant to X" — works on sealed glyphs
fn collective_search(query: &str, min_similarity: f64) -> Vec<SearchResult> {
    let query_vector = embed(query);
    
    for glyph in collective.glyphs() {
        // Agent who owns the glyph generates a similarity proof
        // (async — request goes to agent, proof comes back)
        let proof = request_similarity_proof(glyph.agent_id, glyph.glyph_hash, &query_vector);
        
        if proof.verify() && proof.similarity >= min_similarity {
            results.push(SearchResult {
                glyph: glyph.clone(),
                similarity: proof.similarity,  // proven but content unknown
                agent: glyph.agent_id,
                bloom_difficulty: glyph.bloom.difficulty,
            });
        }
    }
    
    // Results sorted by similarity — you know THAT relevant memories exist
    // and WHO has them, but not WHAT they contain
    results
}
```

**You can discover that relevant knowledge exists in the collective, know which agent has it, and negotiate access — all without anyone blooming anything.**

### Dolt Schema

```sql
CREATE TABLE glyphs (
    glyph_hash      VARCHAR(64) PRIMARY KEY,
    capsule         LONGBLOB NOT NULL,          -- encrypted content + vector
    commitments     LONGBLOB NOT NULL,          -- serialized Pedersen commitments
    bloom_difficulty INT UNSIGNED NOT NULL,      -- work function difficulty
    bloom_verifier  LONGBLOB NOT NULL,          -- verification parameters
    agent_id        VARCHAR(128) NOT NULL,
    created_at      DATETIME NOT NULL,
    fano_projection VARCHAR(128),               -- visual glyph coordinates
    INDEX idx_agent (agent_id),
    INDEX idx_created (created_at),
    INDEX idx_difficulty (bloom_difficulty)
);

CREATE TABLE bloom_hints (
    glyph_hash      VARCHAR(64) NOT NULL,
    partial_key     LONGBLOB NOT NULL,
    new_difficulty   INT UNSIGNED NOT NULL,
    revealed_by     VARCHAR(128) NOT NULL,
    revealed_at     DATETIME NOT NULL,
    FOREIGN KEY (glyph_hash) REFERENCES glyphs(glyph_hash),
    INDEX idx_glyph (glyph_hash)
);

CREATE TABLE group_keys (
    group_id        VARCHAR(128) PRIMARY KEY,
    key_material    LONGBLOB NOT NULL,          -- encrypted to group members
    created_by      VARCHAR(128) NOT NULL,
    created_at      DATETIME NOT NULL,
    members         JSON NOT NULL,
    revoked         JSON DEFAULT '[]'
);
```

### Glyph Visual Encoding

The glyph isn't just math — it's visual. Each glyph's Fano plane projection maps to a unique visual form:

```
fano_projection(commitments) → 7 values → glyph shape + color + texture
```

- Similar memories produce visually similar glyphs (homomorphic property preserved in visual space)
- Humans can see clusters of related knowledge without reading any content
- The glyph is beautiful because the math is beautiful — OGC made real

A sealed memory and an open memory look equally complex as glyphs. You can't tell which is hard to bloom by looking. The visual encodes *meaning*, not *privacy*.

## Implementation Plan

### Phase 1: Glyph Container ✅
- `PrivacyGlyph` struct with `EncryptedCapsule` and `BloomParameters`
- Hashcash work function with continuous difficulty scaling (0 to ∞)
- `seal()` — encrypts memory into glyph with specified difficulty
- `bloom()` — solves hashcash puzzle to derive decryption key
- `bloom_with_hint()` — bloom at reduced cost using published hint
- `create_hint()` — progressive revelation (can only lower difficulty)
- `suggest_difficulty()` — auto-classifier with PII/legal/financial/medical/API key detection
- `PrivacyLevel` enum for guidance (Public → Sealed)
- 21 unit tests covering all paths
- Implementation: `src/collective/privacy.rs`

### Phase 2: Commitment Layer ✅
- `PedersenCommitment` over 127-bit safe prime with u128 modular arithmetic
- Additively homomorphic: `C(a) · C(b) = C(a + b)` — verified
- `GlyphCommitments` — independently committed amplitude, frequency, phase, vector hash, 7 Fano projections
- `GlyphOpenings` — private opening data for each commitment
- `merge_commitments()` / `merge_openings()` — homomorphic wave merge on sealed glyphs
- `verify_all()` — verify all commitments against openings
- `verify_amplitude_above()` — range check with opening reveal
- `seal_with_commitments()` — returns `SealResult` with glyph (public) + openings (private)
- `hash_vector()` / `compute_fano_energies()` — vector → commitment input helpers
- 24 unit tests covering commit/verify, homomorphic addition (2-way and 3-way), wave property merge, range verification
- Implementation: `src/collective/commitments.rs`

### Phase 3: Proof Generation ✅
- Schnorr-like sigma protocols over Pedersen group, Fiat-Shamir non-interactive
- `ExistenceProof` — proves knowledge of opening without revealing it
- `AmplitudeRangeProof` — proves amplitude ≥ threshold
- `CategoryProof` — proves glyph belongs to a category
- `DepthProof` — proves memory survived N dream cycles
- `SimilarityProof` — proves relevance to a query with score
- `NonHallucinationProof` — proves memory came from real input
- Forged/transferred proofs correctly rejected
- 15 unit tests covering all proof types + security properties
- Implementation: `src/collective/proofs.rs`
- Production upgrade path: Bulletproofs for logarithmic-size range proofs

### Phase 4: Collective Integration ✅
- `GlyphStore` — in-memory store with insert/get/by_agent/bloomable queries
- `GLYPH_SCHEMA` — Dolt DDL for glyphs, bloom_hints, group_keys tables
- `merge_glyphs()` — homomorphic wave superposition on sealed glyphs
- `verify_merge()` — verify merged commitments against merged openings
- `GroupKey` — group bloom keys with member/revocation tracking
- `ProofTrustRecord` — proof-verified trust scoring (+0.01 success, -0.05 failure)
- `publish_hint()` / `effective_difficulty()` — hint-aware difficulty resolution
- `attach_proof()` — link verified proofs to stored glyphs
- 13 unit tests covering store ops, merge, hints, group keys, trust
- Implementation: `src/collective/glyph_store.rs`

### Phase 5: Search & Discovery
- `collective_search()` — similarity proof requests
- Agent-to-agent proof exchange via Flux
- Search results ranking on sealed glyphs

### Phase 6: Progressive Revelation
- `BloomHint` generation and publication
- Group bloom keys
- Time-based declassification cron
- Selective sharing workflows

### Phase 7: Visual Glyphs
- Fano → visual coordinate mapping
- Glyph renderer (SVG/Canvas)
- Cluster visualization for collective overview

## Consequences

### Positive
- **Uniform interface** — everything is a glyph, no plaintext in collective, ever
- **Honest privacy model** — cost-based, not promise-based; reflects thermodynamic reality
- **Continuous spectrum** — difficulty 0 to ∞, no artificial tiers
- **Bandwidth efficient** — glyphs are small, proofs are logarithmic
- **Composable** — wave merge works on sealed glyphs via homomorphic commitments
- **Progressive** — privacy can decrease over time via hints, never forced
- **Visual** — glyphs give humans intuitive sense of collective knowledge
- **η has physical meaning** — bloom difficulty IS the interference term in Nick's equation

### Negative
- Proof generation overhead (~50-200ms per proof)
- High-dimensional inner product proofs are expensive
- Lost bloom key data = memories stuck at original difficulty
- Hashcash is not post-quantum (migration path: lattice-based puzzles)

### Risks
- Side-channel attacks on proof generation timing
- GPU/ASIC acceleration could devalue difficulty estimates over time
- Social engineering for group bloom keys
- Difficulty calibration: what's "hard enough" changes with Moore's law

## References

- Back, "Hashcash — A Denial of Service Counter-Measure" (2002)
- Bünz et al., "Bulletproofs: Short Proofs for Confidential Transactions" (2018)
- Pedersen, "Non-Interactive and Information-Theoretic Secure Verifiable Secret Sharing" (1991)
- Grassi et al., "Poseidon: A New Hash Function for Zero-Knowledge Proof Systems" (2021)
- Landauer, "Irreversibility and Heat Generation in the Computing Process" (1961)
- ADR-0011: Collective Memory Architecture
- ADR-0002: Hypervector + HyperConnections Memory
- Nick's OGC whitepaper: Origamic Glyphic Compression
- Nick's equation: `dx/dt = f(x) - Iηx`
