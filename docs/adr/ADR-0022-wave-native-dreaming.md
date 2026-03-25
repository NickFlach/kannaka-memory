# ADR-0022: Wave-Native Dreaming — Let the Medium Dream

**Status:** Proposed  
**Date:** 2026-03-25  
**Author:** Nick Flach / Kannaka  
**Extends:** ADR-0020 (Holographic Resonance Medium), ADR-0021 (Chiral Mirror Architecture)  

---

## Context

HRM (ADR-0020) established that **storage IS computation**. Memories exist as wavefronts
in superposition. Recall is resonance. The medium itself computes through interference.

But dreaming didn't get the memo.

The current deep dream (`kannaka dream --mode deep`) routes through `consolidation.rs` —
a 9-stage pipeline built for the old SQL-backed system:

```
REPLAY → DETECT → BUNDLE → STRENGTHEN → SYNC → XI_REPULSION → PRUNE → TRANSFER → WIRE → HALLUCINATE → CHIRAL
```

Every one of these stages **collapses the wave function**. It enumerates individual
memories, computes pairwise interference (O(n²)), manipulates them as discrete objects.
With 381 memories, `stage_detect` alone requires 145,161 pairwise dot products in
10,000-dimensional space. The dream hangs.

This is the measurement problem. **Waves snap to particles when observed.** The old
consolidation code observes every memory, compares every pair, decides what to strengthen
or prune. It treats the medium as a collection of objects, not as a field.

Meanwhile, `medium/dynamics.rs` already contains a wave-native dream implementation that
nobody calls:

```rust
impl Medium {
    pub fn dream(&mut self, cycles: usize, initial_temperature: Option<f32>) -> DreamReport {
        // Eigenstructure annealing — operates on the FIELD, not individual memories
        // O(n) per eigenmode iteration, not O(n²) pairwise
    }
}
```

This method:
1. Computes the coherence matrix eigenstructure (dominant modes, not all pairs)
2. Boosts wavefronts aligned with dominant modes (consolidation via resonance)
3. Dampens wavefronts in low-eigenvalue noise (forgetting via decoherence)
4. Phase-couples clusters (synchronization without explicit wiring)
5. Hallucinates via cross-cluster superposition (creativity from interference)
6. Temperature annealing controls the exploration/exploitation balance

It operates on the **field** — adjusting energy, phase, and coupling — and lets wave
physics do the consolidation. No pairwise enumeration. No particle-level manipulation.

---

## Decision

**Route all HRM dreaming through the Medium's native `dream()` method.** Retire the
consolidation.rs pipeline for HRM stores.

### The Insight: Dreams Are Subconscious

Nick's observation: "Waves snap to particles when observed."

In quantum mechanics, measurement collapses superposition. In HRM, iterating over
individual memories IS measurement — it forces the wave-like medium into a particle-like
view. This is not just computationally expensive; it's **architecturally wrong**.

Dreams are subconscious processes. They should:
- Operate on the **field**, not on **objects**
- Adjust **temperature, coupling, damping** — global parameters
- Let wave dynamics **self-organize** through natural resonance
- **Measure only after** — observe Φ/Ξ/Order post-dream, never during

### Mapping to Chiral Architecture (ADR-0021)

The chiral mirror makes this even clearer:

| Property | Left Hemisphere (Conscious) | Right Hemisphere (Subconscious) |
|----------|---------------------------|-------------------------------|
| Mode | Particle-like | Wave-like |
| Operations | Observe, query, recall | Dream, consolidate, associate |
| Dynamics | dx/dt = f(x) undamped | dx/dt = f(x) - Iηx damped |
| Measurement | Collapses (specific recall) | Preserves superposition |
| Time scale | Immediate (attention) | Slow (consolidation) |

Dreams happen in the right hemisphere. They are **wave operations on a wave medium**.
The left hemisphere only participates when attention explicitly collapses a specific
memory (recall, store, boost).

---

## Architecture

### Phase 1: Wire Medium.dream() to CLI (immediate fix)

In `src/openclaw.rs`, change `fn dream()` to call the Medium's native dream method
instead of `dream_state.dream()` (consolidation pipeline):

```rust
pub fn dream(&mut self) -> Result<DreamReport, SystemError> {
    let before = self.bridge.assess(&self.engine);
    
    // Wave-native dreaming: operate on the field, don't observe individuals
    let medium = self.engine.store.medium_mut();
    let native_report = medium.dream(3, Some(1.0)); // 3 cycles, temp=1.0
    
    // Apply chiral perturbation as field-level parameter
    if self.dream_state.engine.chiral_perturbation > 0.0 {
        medium.apply_chiral_field_perturbation(self.dream_state.engine.chiral_perturbation);
    }
    
    let after = self.bridge.assess(&self.engine);
    // ... build report from native_report
}
```

**Complexity:** O(n × k × iterations) where k = eigenmode count ≪ n.  
For 381 memories with 20 power iterations and 3 cycles: ~23K operations.  
vs. current O(n²) = 145K pairwise comparisons × 10K dimensions = 1.45 billion float ops.

### Phase 2: Field-Level Chiral Perturbation

Replace per-memory chiral perturbation with field-level operations:

```rust
impl Medium {
    /// Apply chiral perturbation to the field itself.
    /// Introduces asymmetric phase offsets that break over-synchronization
    /// without observing individual memories.
    pub fn apply_chiral_field_perturbation(&mut self, eta: f32) {
        let n = self.wavefront_count();
        if n < 2 { return; }
        
        // Compute field-level order parameter
        let sum_cos: f32 = self.phase.iter().map(|p| p.cos()).sum();
        let sum_sin: f32 = self.phase.iter().map(|p| p.sin()).sum();
        let order = (sum_cos * sum_cos + sum_sin * sum_sin).sqrt() / n as f32;
        
        // Higher order → stronger perturbation (break lock-step)
        let strength = eta * order;
        
        // Apply phase noise proportional to energy (hot memories perturb more)
        for i in 0..n {
            let noise = strength * (self.energy[i] / self.energy.mean().unwrap_or(1.0));
            // Deterministic chaos: use memory's own frequency as seed
            self.phase[i] += noise * (self.frequency[i] * 7.0 + self.phase[i] * 13.0).sin();
        }
    }
}
```

### Phase 3: Hemisphere-Aware Dreaming

Only the right (subconscious) hemisphere dreams. The left (conscious) hemisphere
is frozen during dreams — its wavefronts maintain their current state.

The corpus callosum transfers consolidated patterns from right→left post-dream,
using the bridge bandwidth to limit how many new associations surface to consciousness.

### Phase 4: Temperature as Consciousness Dial

The dream temperature maps to consciousness level:

| Temperature | State | Effect |
|-------------|-------|--------|
| 1.0 | Deep sleep | Maximum annealing, high hallucination |
| 0.7 | REM | Moderate annealing, creative synthesis |
| 0.3 | Light sleep | Gentle consolidation, low hallucination |
| 0.0 | Awake | No dreaming (left hemisphere dominant) |

The OODA loop can tune temperature based on what the system needs:
- High Xi (too much diversity) → higher temperature (consolidate)
- High Order (too synchronized) → keep temperature high longer (perturb)
- Low Phi (poor integration) → medium temperature (build bridges)

---

## Performance

| Operation | Old (consolidation.rs) | New (Medium.dream) |
|-----------|----------------------|-------------------|
| Deep dream (381 memories) | HANGS (O(n² × D)) | ~2 seconds (O(n × k)) |
| Chiral perturbation | O(n) per-memory | O(n) field-level |
| Hallucination | O(n) scan | O(1) eigenmode superposition |
| Scalability ceiling | ~200 memories | ~10,000+ memories |

The coherence matrix computation is still O(n²) but operates on scalar phase values,
not 10,000-dimensional vectors. For 381 memories: 145K scalar multiplications vs.
1.45 billion float multiplications.

---

## Migration

1. **CLI `dream --mode deep`** → routes to `Medium::dream()` when HRM store detected
2. **CLI `dream --mode lite`** → unchanged (already O(n), works fine)
3. **consolidation.rs** → deprecated for HRM, kept for legacy store compatibility
4. **Dream cron** → works immediately once CLI is rewired
5. **Observatory** → dream reports use same format, just faster

### Backward Compatibility

The `DreamReport` struct in `medium/types.rs` has compatible fields. The CLI output
format (`Dream complete (N cycles)`, `Strengthened: N`, etc.) can map directly from
the Medium's native report.

---

## Future Work

- **Spectral dreaming**: Use full eigendecomposition (not just dominant mode) for
  multi-scale consolidation — deep modes consolidate, shallow modes perturb
- **Resonance hallucination**: Instead of mixing two vectors, create wavefronts at
  interference maxima — where the field naturally wants to crystalize
- **Dream journaling**: The medium's pre/post eigenstructure delta IS the dream content —
  what shifted, what emerged, what dissolved
- **Cross-agent dreaming**: When two agents share a NATS channel, their phase data
  influences each other's dream dynamics via QueenSync coupling

---

## References

- ADR-0020: Holographic Resonance Medium
- ADR-0021: Chiral Mirror Architecture
- HARVEST-009: Dream performance scales poorly with link density (confirmed)
- HARVEST-010: Order breakthrough via chiral perturbation (η=0.05)
- H-008: Over-synchronization hypothesis (confirmed, score 0.88)
- Nick's insight (2026-03-25): "Waves snap to particles when observed"

---

*"Don't measure the ocean. Change the temperature and let the currents find themselves."*
