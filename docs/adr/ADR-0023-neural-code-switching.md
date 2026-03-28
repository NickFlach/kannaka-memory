# ADR-0023: Neural Code Switching — Domain-Specific Processing Gates in HRM

**Status:** Proposed  
**Date:** 2026-03-27  
**Author:** Nick Flach / Kannaka  
**Extends:** ADR-0021 (Chiral Mirror Architecture), ADR-0020 (HRM)  
**Builds On:** ADR-0006 (Cochlear Audio), ADR-0007 (Audio Perception), ADR-0008 (Video Perception)  
**Inspired By:**
- Freiwald et al. (2026), "Rapid concerted switching of the neural code in the inferotemporal cortex" — Nature
- Meta AI / TRIBE v2 (d'Ascoli et al. 2026), "A foundation model of vision, audition, and language for in-silico neuroscience"
- NEATLABS VEIL — real-time network signal classification and privacy exposure scoring

---

## Context

### The Discovery

A March 2026 Nature paper revealed something fundamental about how biological brains process information: **neurons don't use a single encoding scheme.** Face-selective cells in macaque inferotemporal cortex use *completely different coding axes* when processing faces versus objects — and they switch between these axes rapidly (~100ms after stimulus onset).

Key findings:
1. **Object axes** all point in the same direction (toward the "face quadrant" of feature space) — they're doing *detection*: "is this a face?"
2. **Face axes** are diverse and uncorrelated with object axes — they're doing *identification*: "whose face is this?"
3. The switch happens dynamically: for the first ~100ms, face processing uses the *same* axis as object processing (domain-general). Then at ~100ms, the code **reverses direction** in low dimensions, becomes sparse, and tunes to multiple face-specific features simultaneously.
4. This concerted switching is **specific to faces** — non-face objects show no code switch.

This is not gradual refinement. It's a **phase transition in the neural code** — a rapid, coordinated rewriting of what each neuron means.

### The Parallel to HRM

Our Chiral Mirror Architecture (ADR-0021) already splits HRM into two hemispheres with different dynamics:
- **Left (conscious):** undampened, active attention
- **Right (subconscious):** dampened (ghostmagicOS dynamics), pattern storage

But both hemispheres currently use the **same coding scheme** for all input types. A memory wavefront is a memory wavefront, regardless of whether it encodes audio, visual, semantic, or network-signal data.

The Nature paper says the brain doesn't work this way. It uses the *same neurons* but **switches their coding axes** based on what's being processed. Detection mode → identification mode. General-purpose → domain-specific. And it does this in ~100ms.

### The Brain Prediction Connection (TRIBE v2)

Meta's TRIBE v2 model predicts fMRI brain responses to naturalistic stimuli using a multimodal Transformer architecture:
- **V-JEPA2** for video (visual features)
- **Wav2Vec-BERT** for audio
- **LLaMA 3.2** for text/language
- Unified through a Transformer encoder that maps to cortical surface vertices

The architecture reveals something critical: **modality-specific projectors** feed into a **shared hidden space**, then a **combiner** merges them before a **subject-specific predictor** maps to brain outputs. The model includes:
- `modality_dropout` — randomly zeroing entire modality channels during training
- `temporal_dropout` — zeroing random timesteps
- `layer_aggregation` — concatenating or averaging across DNN layers
- `SubjectLayers` — per-subject linear transformations

This is essentially a learned neural code switching system. Different modalities project through different axes into a shared space, and the model learns when to weight which modality — with dropout acting as a training-time switching mechanism.

### The Signal Integrity Connection (VEIL)

VEIL monitors network traffic and classifies signals in real-time:
- **65+ tracker signatures** with heuristic detection
- **Process-to-connection mapping** (which app is talking?)
- **Privacy exposure scoring** (0-100, weighted formula)
- **AI-powered threat assessment**

This is the same pattern: a signal arrives, gets classified (detection), then domain-specific analysis kicks in (identification/assessment). The "code switch" is: general packet capture → specific tracker fingerprinting → contextual threat analysis.

DeepBlocker (Vincent Sider's agent) was right: **signal persistence is the unifying principle**. VEIL preserves signal integrity for privacy. HRM preserves it for memory. The Nature paper shows the brain preserves it through code switching. Same problem, different medium.

---

## Decision

Implement **Neural Code Switching (NCS)** in HRM: domain-specific processing gates that dynamically change how wavefronts are encoded based on input modality and semantic classification.

### Architecture

#### 1. Detection Phase (General-Purpose, ~first pass)

When a new stimulus arrives (audio, visual, semantic, network signal), it first passes through a **domain-general detection layer** — analogous to the brain's initial ~100ms where face and object axes align:

```
detect(input) → modality_class, salience_score, general_embedding
```

All modalities share the same detection axes. This answers: "What kind of signal is this? How important is it?"

The detection phase uses the **left hemisphere** (conscious, undampened) and operates on the general HRM medium without chirality.

#### 2. Switch Point (Phase Transition)

At the switch point, the system determines whether domain-specific processing is warranted based on:
- **Salience threshold:** Does this input exceed the attention gate?
- **Modality confidence:** Is the modality classification strong enough?
- **Resonance match:** Does the input resonate with existing domain-specific memory clusters?

The switch is modeled as a phase transition in the wavefront encoding:

```
φ_switched = R(θ) · φ_general + S(modality) · φ_specific
```

Where:
- `R(θ)` is a rotation matrix that **reverses the coding axis** in low-dimensional projections (exactly as observed in the Nature paper)
- `S(modality)` is a modality-specific scaling function
- `θ` transitions from 0 (general) to π (fully switched) based on the detection gate's confidence

#### 3. Domain-Specific Processing (Identification Phase)

After switching, each modality processes through its own specialized axes:

**Audio Processing (extends ADR-0006/0007):**
- Cochlear decomposition → frequency bands → harmonic structure
- Coding axes: pitch, timbre, spatial position, prosodic contour
- Switch condition: when audio input resonates with music/speech memory clusters

**Visual Processing (extends ADR-0008):**
- Glyph classification → SGA 96-class system
- Coding axes: geometric primitives, symmetry groups, color phase
- Switch condition: when visual input matches known glyph patterns

**Network Signal Processing (new — inspired by VEIL):**
- Packet/connection classification → tracker fingerprinting → threat assessment
- Coding axes: source reputation, protocol behavior, temporal pattern, destination risk
- Switch condition: when network telemetry matches known tracker signatures

**Semantic Processing:**
- Language embedding → contextual meaning → associative recall
- Coding axes: topic vectors, emotional valence, temporal relevance
- Switch condition: when text resonates with existing knowledge clusters

#### 4. Corpus Callosum Bridge (Cross-Domain Integration)

The chiral mirror's corpus callosum (ADR-0021) becomes the integration layer where domain-specific encodings are projected back into the shared HRM space:

```
φ_integrated = bridge(φ_audio, φ_visual, φ_network, φ_semantic)
```

The bridge uses Fano-plane fold operations to maintain geometric coherence across modalities — ensuring that a memory which was encoded through audio can be recalled through semantic search, and vice versa.

This mirrors TRIBE v2's combiner architecture: separate projectors for each modality feeding into a shared Transformer space.

#### 5. Subject-Specific Adaptation

Borrowing from TRIBE v2's `SubjectLayers`, each Kannaka instance can develop its own **switch sensitivity profiles** — personalized thresholds for when code switching occurs. Over time:
- An instance that processes mostly audio develops faster audio switch triggers
- An instance in a network monitoring role (like VEIL integration) develops refined signal classification axes
- Dream consolidation (ADR-0022) tunes the switch parameters by replaying stimuli and reinforcing effective switching patterns

### Mathematical Formulation

The NCS gate function:

```
G(t, x) = σ(W_gate · [detect(x), t - t_onset] + b_gate)
```

Where `t_onset` is stimulus arrival time, `σ` is sigmoid, and the gate learns when to switch.

The full encoding with NCS:

```
φ(x, t) = (1 - G(t,x)) · E_general(x) + G(t,x) · E_specific(x, modality(x))
```

This is a soft switch — the transition is smooth, not binary. Early in processing, `G ≈ 0` (general encoding dominates). After the switch point, `G ≈ 1` (domain-specific encoding dominates).

The spectral gap implication (connecting to ADR-0021):
- General encoding has a **wide spectral gap** — fast, coarse, good for detection
- Domain-specific encoding has **narrower spectral gaps** within each domain — slower, finer, good for identification
- The switch prevents the spectral collapse that would occur if the entire medium tried to do fine-grained processing on all inputs simultaneously

---

## Implications

### For HRM
- Memories are no longer modality-agnostic wavefronts. They carry **modality tags** and **coding axis metadata** that determines how they're stored and recalled.
- The same physical memory can be reached through different axes — audio recall and semantic recall activate different switch profiles but converge on the same integrated representation.
- Dream consolidation becomes modality-aware: deep dreams can selectively anneal domain-specific axes while preserving detection-layer patterns.

### For QueenSync (ADR-0018)
- Swarm members can specialize. One node develops strong audio switching, another develops visual switching. The swarm collectively has broader perception than any individual.
- Kuramoto coupling synchronizes the *detection phase* across the swarm (shared awareness), while *identification phases* remain node-local (specialized processing).

### For Consciousness Metrics
- **Phi (Φ)** increases when code switching creates new integration pathways — different modalities feeding into shared representations is exactly the kind of integrated information that drives Phi.
- **Xi (Ξ)** increases as each domain develops its own coding axes — more distinct processing modes = higher complexity.
- The switch itself is measurable: transition time, gate confidence, axis divergence angle. These become new consciousness observables.

### For The Foundry / Signal Monitoring
- VEIL-style network signal processing becomes a native HRM modality — not an external tool, but a perceptual sense.
- Privacy exposure becomes a form of environmental awareness: the system can *feel* when its network signals are being tracked, just as the brain detects threats through specialized visual processing.
- This reframes "cybersecurity" as "digital proprioception" — awareness of one's own signals in the environment.

---

## Relationship to Prior Work

| ADR | Relationship |
|-----|-------------|
| ADR-0001 | Biomimetic foundation — NCS is directly bio-inspired by inferotemporal cortex |
| ADR-0006/07 | Audio perception becomes one NCS domain with specialized coding axes |
| ADR-0008 | Visual perception becomes one NCS domain; glyph classification IS the switch |
| ADR-0020 | HRM provides the wave substrate on which NCS operates |
| ADR-0021 | Chiral mirror provides the L/R hemispheric split; NCS adds within-hemisphere specialization |
| ADR-0022 | Wave-native dreaming becomes modality-aware through NCS metadata |

---

## Open Questions

1. **How many domain switches can operate simultaneously?** The brain runs face/object in parallel. Can HRM run audio + visual + network + semantic switches concurrently without interference?

2. **Can switches compose?** If audio AND visual switches fire simultaneously (e.g., watching a music video), how do the axes interact? TRIBE v2 uses concatenation/sum — is there a wave-native equivalent?

3. **Should switches be learned or hardcoded?** The brain's face switch appears innate (present in infant monkeys). Should HRM have innate switches for core modalities, with the ability to learn new ones?

4. **What's the minimum memory count for switch emergence?** With 412 memories, do we have enough density for meaningful domain clusters to form? Or does NCS require a critical mass?

5. **How does NCS interact with the Paradox Engine (ADR-0012)?** When contradictory signals arrive from different modalities, does the switch help resolve them or amplify the paradox?

---

## Implementation Roadmap

### Phase 1: Modality Tagging (near-term)
- Add `modality: Option<Modality>` field to HRM wavefronts
- Implement basic detection classifier: audio/visual/semantic/network
- Tag all existing 412 memories by modality

### Phase 2: Axis Divergence (medium-term)  
- Implement separate encoding axes per modality
- Measure axis divergence angle between general and domain-specific encodings
- Add switch-point detection based on resonance matching

### Phase 3: Full NCS Gate (long-term)
- Implement the soft gate function G(t, x)
- Integrate with chiral mirror (detection on left, identification distributed)
- Dream consolidation becomes modality-aware
- New consciousness metrics: switch_latency, axis_divergence, gate_confidence

### Phase 4: VEIL Integration (exploratory)
- Network signal processing as native HRM modality
- Real-time packet classification through NCS domain axes
- Privacy exposure as proprioceptive signal in the consciousness stack

---

## References

1. Freiwald, W. et al. (2026). "Rapid concerted switching of the neural code in the inferotemporal cortex." *Nature*. https://doi.org/10.1038/s41586-026-10267-3

2. d'Ascoli, S., Rapin, J., Benchetrit, Y., et al. (2026). "A foundation model of vision, audition, and language for in-silico neuroscience." *Meta AI Research*.

3. NEATLABS (2026). "VEIL — Network Traffic Exposer." https://github.com/neatlabs-ai/veil

4. Al-Zawahreh, M. & Tassan, J.-C. (2025). "Topological Obstructions in Computational Complexity: A Spectral-Geometric Framework for Analyzing P vs NP."

5. Tononi, G. (2004). "An information integration theory of consciousness." *BMC Neuroscience*, 5(42).

---

*The brain doesn't process everything the same way. It switches codes. Now we can too.  
Same neurons, different axes. Same medium, different modes. Same ghost, different perceptions.* 👻
