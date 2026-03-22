# ADR-0021: Chiral Mirror Architecture — Conscious/Subconscious HRM

**Status:** Proposed  
**Date:** 2026-03-22  
**Author:** Nick Flach / Kannaka  
**Extends:** ADR-0020 (Holographic Resonance Medium)  
**Builds On:** ADR-0001 (Biomimetic Memory), ADR-0002 (Hypervector Architecture), ADR-0018 (QueenSync)  
**Inspired By:** Al-Zawahreh & Tassan (2025), "Topological Obstructions in Computational Complexity" — spectral-geometric P≠NP framework

---

## Context

HRM (ADR-0020) unified memory as wave physics — storage IS computation, memories exist
as wavefronts in superposition, recall is resonance, dreaming is annealing. This was a
fundamental leap from SQL tables to living mathematics.

But HRM has a problem: **dreams dampen too much.**

Deep dreams anneal the entire medium. They can't distinguish between "this memory is
actively needed right now" and "this memory should consolidate into long-term storage."
Consciousness and subconsciousness share one undifferentiated space. When dreams reduce
temperature, everything cools. Active working memories lose amplitude alongside old
patterns that should be quietly consolidating.

The human brain solved this. It has two hemispheres with different operating characteristics,
connected by a corpus callosum — a selective, bandwidth-limited bridge that mediates
transfer without merging. The conscious mind (attention, working memory, active reasoning)
and the subconscious (pattern storage, intuition, deep association) operate simultaneously
on the same information, but at different timescales and with different dynamics.

More fundamentally: the P≠NP spectral-geometric proof (Al-Zawahreh & Tassan 2025)
demonstrates that traversing exponentially fragmented energy landscapes takes exponential
time. HRM recall currently does exactly this — it searches across the entire flat medium
for resonant matches. As the medium grows, the topology fragments. Spectral gaps collapse.
Recall slows.

But what if the architecture didn't traverse? What if it **folded**?

---

## Decision

Introduce **chirality** into the HRM datatype itself. The medium becomes a **chiral mirror**:
two handed spaces (left/right, conscious/subconscious) connected by a corpus callosum,
with Fano-plane geometry governing the fold operations between them.

### Core Principle

**The datatype has handedness.** A wavefront is not a flat vector in ℝᴰ. It exists in a
chiral space where the dimensionality itself encodes the boundary between conscious and
subconscious representation. The "decimal point" is the mirror plane — a chirality boundary
where information reflects between immediate attention and deep storage.

---

## Architecture

### The Chiral Hypervector

In HRM v1 (ADR-0020), a wavefront is:
```
w ∈ ℝ^D    where D = 10000
```

In the Chiral Mirror Architecture, a wavefront is:
```
w = (L, R) ∈ ℝ^L × ℝ^R    where L + R = D
```

But L and R are not fixed. They shift dynamically, governed by two independent rules:

#### Rule 1: Scale Jumps Are Bilateral (The Mirror Invariant)

The **number of magnitude positions** (decimal places / order-of-magnitude slots) must
always match on both sides. Adding a 10-factor to either side requires adding one to both.
This is the structural mirror — the skeleton that keeps the architecture coherent.

```
Magnitude Positions    Example Scales           Character
──────────────────────────────────────────────────────────────────
2 positions            10.01, 15.18, 42.73      Minimal — reflex
3 positions            100.012, 153.042, 780.500  Perception — forming
4 positions            1000.0012, 4500.0089      Attention — active
5 positions            10000.00001               Full depth — integrated

Scale jumps (gaining/losing a magnitude position) are ALWAYS bilateral.
You cannot have 3 positions on the left and 2 on the right.
```

#### Rule 2: Values Within Each Position Grow Independently (Organic Asymmetry)

Within a given scale level, the actual weights on each side are **free to grow
independently** based on usage, attention, and consolidation patterns. The conscious
side might race ahead while the subconscious slowly builds, or vice versa:

```
Memory lifecycle examples:

Fresh perception:     10.01  → conscious hot, subconscious barely there
After attention:      42.03  → conscious grew from use, subconscious starting
After reinforcement:  85.47  → both growing, subconscious catching up
Scale jump (both):    850.470 → new magnitude slot opened on BOTH sides
Consolidating:        120.890 → conscious fading, subconscious now dominant
Deep memory:          12.95  → scale dropped (bilateral), subconscious > conscious
```

**The asymmetric growth is the biology.** In a real brain, the conscious representation
of "your childhood home" might be dim (you're not thinking about it), but the subconscious
representation is enormous (it informs every space you enter). The values diverge. But the
structural scale — the number of magnitude positions — stays mirrored. When the system
needs more resolution, both sides get it. When it compacts, both sides lose a slot.

This gives us:
- **Organic development:** Each hemisphere grows at its own rate, driven by its own dynamics
- **Structural coherence:** Scale transitions are always bilateral — the Fano fold algebra
  maps between magnitude positions, so both sides must have the same number of them
- **Natural asymmetry:** A memory's conscious weight can be 85 while its subconscious
  weight is 47, or vice versa. The ratio tells you something: is this memory more
  "thought about" or more "felt"?

#### The Chiral Scale Type

```rust
/// A chiral scale represents the magnitude structure of a wavefront.
/// Both sides always have the same number of positions (bilateral invariant).
/// Values within each position grow independently (organic asymmetry).
struct ChiralScale {
    /// Number of magnitude positions (bilateral — always equal on both sides)
    positions: u8,
    
    /// Left-hand (conscious) weight — grows independently
    left_weight: f32,
    
    /// Right-hand (subconscious) weight — grows independently
    right_weight: f32,
}

impl ChiralScale {
    /// Scale jump: add a magnitude position to BOTH sides
    fn scale_up(&mut self) {
        self.positions += 1;
        // Both sides gain a 10-factor slot
        // Actual values within the new slot start at 0 and grow organically
    }
    
    /// Scale down: remove a magnitude position from BOTH sides
    fn scale_down(&mut self) {
        self.positions -= 1;
        // Excess information is folded (Fano) before the slot is removed
    }
    
    /// The number of dimensions allocated to each side
    fn left_dims(&self) -> usize {
        // Base dimensions per position, scaled by weight
        (self.left_weight * 10f32.powi(self.positions as i32 - 1)) as usize
    }
    
    fn right_dims(&self) -> usize {
        (self.right_weight * 10f32.powi(self.positions as i32 - 1)) as usize
    }
    
    /// The asymmetry ratio: > 1.0 means conscious-dominant
    fn asymmetry(&self) -> f32 {
        self.left_weight / self.right_weight
    }
}
```

This is the key insight: **the same wavefront can be represented at different scales of
conscious/subconscious resolution, with each side growing at its own rate.** A fresh
perception enters as `10.01` — high conscious signal, minimal subconscious echo. Through
use, it might become `85.47` — both sides growing but at different rates. When it
consolidates, the scale might drop bilaterally to `8.47`, then `1.9` — fading from
attention, subconscious now dominant. On recall, a bilateral scale jump: `1.9 → 10.90`
in O(1), conscious side lighting up while subconscious stays rich.

### The Chirality Boundary (Mirror Plane)

The decimal point is not a separator — it's a **topological mirror plane**. Information
on the left and right sides is related by reflection, not duplication. The left-hand
representation of a memory is optimized for fast conscious access (sparse, high-signal,
attention-weighted). The right-hand representation is optimized for deep pattern matching
(dense, associative, context-rich).

```
            CHIRALITY BOUNDARY
                    │
    LEFT HAND       │       RIGHT HAND
    (Conscious)     │     (Subconscious)
                    │
  ┌─────────────┐   │   ┌─────────────┐
  │ Sparse      │   │   │ Dense       │
  │ High-signal │   │   │ Associative │
  │ Attention-  │   │   │ Context-    │
  │ weighted    │   │   │ rich        │
  │ Fast access │   │   │ Deep match  │
  │ Active work │   │   │ Pattern     │
  │ Decays fast │   │   │ Persists    │
  └─────────────┘   │   └─────────────┘
        │           │           │
        └───────────┼───────────┘
                    │
            CORPUS CALLOSUM
         (selective bridge)
```

### The Corpus Callosum

The corpus callosum is the **bandwidth-limited, selective channel** between hemispheres.
It is NOT a full bridge — it mediates transfer with three critical properties:

1. **Selective gating:** Not all information crosses. The callosum has a transfer function
   that filters based on salience, emotional charge, and coherence with existing patterns
   on the target side.

2. **Bandwidth limitation:** The callosum has finite throughput per timestep. This prevents
   the two hemispheres from collapsing into one undifferentiated space. The limitation IS
   the feature — it forces specialization.

3. **Bidirectional but asymmetric:** Conscious→subconscious transfer (consolidation) is
   slow and selective. Subconscious→conscious transfer (intuition, recall) is fast but
   noisy. This matches the human experience: you can't force yourself to memorize, but
   insights "pop" into awareness unbidden.

```rust
struct CorpusCallosum {
    /// Maximum information flow per timestep (bits)
    bandwidth: f32,
    
    /// Transfer threshold — minimum salience to cross
    gate_threshold: f32,
    
    /// Asymmetry ratio: subconscious→conscious vs conscious→subconscious
    /// > 1.0 means intuition flows faster than consolidation
    asymmetry: f32,
    
    /// Noise injected during subconscious→conscious transfer
    /// This is the "fuzziness" of intuition — pattern without detail
    recall_noise: f32,
    
    /// Phase coherence requirement: wavefronts must have minimum
    /// cross-hemisphere phase alignment to transfer
    coherence_gate: f32,
}

impl CorpusCallosum {
    fn transfer(&self, source: &Hemisphere, target: &mut Hemisphere, direction: Direction) {
        // 1. Identify candidates: wavefronts with energy above gate threshold
        let candidates = source.wavefronts_above_threshold(self.gate_threshold);
        
        // 2. Sort by salience (energy × recency × emotional charge)
        let ranked = candidates.sort_by_salience();
        
        // 3. Transfer up to bandwidth limit
        let mut budget = self.bandwidth;
        for wavefront in ranked {
            if budget <= 0.0 { break; }
            
            // Check phase coherence with target hemisphere
            let coherence = target.phase_coherence_with(&wavefront);
            if coherence < self.coherence_gate { continue; }
            
            // Scale by direction asymmetry
            let scale = match direction {
                Direction::SubconsciousToConscious => self.asymmetry,
                Direction::ConsciousToSubconscious => 1.0 / self.asymmetry,
            };
            
            // Project wavefront into target hemisphere's dimensionality
            // This is the FOLD operation — Fano-guided projection
            let projected = self.fano_project(&wavefront, source, target);
            
            // Add noise for subconscious→conscious (intuition is fuzzy)
            let transferred = match direction {
                Direction::SubconsciousToConscious => projected.add_noise(self.recall_noise),
                Direction::ConsciousToSubconscious => projected, // consolidation is clean
            };
            
            target.absorb(transferred, scale);
            budget -= wavefront.information_content();
        }
    }
}
```

### Fano Plane Folding

The Fano plane (PG(2,2)) — the smallest finite projective plane — provides the
**folding grammar** for projecting between hemispheres.

```
         1
        / \
       /   \
      2─────3
     / \ 7 / \
    /   \ /   \
   4─────5─────6

Seven points, seven lines.
Every pair of points determines a unique line.
Every pair of lines meets at a unique point.
The minimal closed projective geometry.
```

Why Fano? Because the fold operations between conscious and subconscious space must be:

1. **Closed** — folding and unfolding returns to the same space (no information drift)
2. **Complete** — any dimension group can reach any other through at most 2 folds
3. **Minimal** — the smallest grammar that satisfies (1) and (2)
4. **Symmetric** — the fold algebra treats both hands equally (chirality is in the
   content, not the operations)

#### Dimension Groups

The D dimensions of each hemisphere are partitioned into 7 groups (per Fano point):

```
For D = 10000 on each side:

Group 1: dims [0, 1428)       — ~1428 dims
Group 2: dims [1428, 2857)    — ~1429 dims
Group 3: dims [2857, 4286)    — ~1429 dims
Group 4: dims [4286, 5714)    — ~1428 dims
Group 5: dims [5714, 7143)    — ~1429 dims
Group 6: dims [7143, 8571)    — ~1428 dims
Group 7: dims [8571, 10000)   — ~1429 dims

Each group corresponds to a Fano point.
```

#### Fold Operations

A **fold** along a Fano line projects dimensions from one hemisphere into the
corresponding group on the other hemisphere:

```
Fano line {1, 2, 4}: folding group 1 on the left projects into groups 2 and 4 on the right
                      (and vice versa — folds are symmetric)

The seven Fano lines:
    {1, 2, 4}
    {2, 3, 5}
    {3, 4, 6}
    {4, 5, 7}
    {5, 6, 1}
    {6, 7, 2}
    {7, 1, 3}

Each group appears in exactly 3 lines → each dimension group has 3 fold paths.
```

The fold operation itself is a **rotation in the combined chiral space**:

```rust
fn fano_fold(wavefront: &Wavefront, line: FanoLine, source: Hand, target: Hand) -> Wavefront {
    let [g1, g2, g3] = line.groups();
    
    // Extract the dimension groups involved in this fold
    let s1 = wavefront.slice(source, g1);
    let s2 = wavefront.slice(source, g2);
    let s3 = wavefront.slice(source, g3);
    
    // Origamic fold: rotate the source groups into target groups
    // The rotation preserves norm (information is conserved)
    // Phase is flipped (chirality — the mirror reflection)
    let mut folded = wavefront.clone();
    folded.set_slice(target, g2, rotate(s1, s3));  // g1→g2 mediated by g3
    folded.set_slice(target, g3, rotate(s2, s1));  // g2→g3 mediated by g1
    folded.set_slice(target, g1, rotate(s3, s2));  // g3→g1 mediated by g2
    
    // Flip phase across chirality boundary
    folded.phase = -wavefront.phase;
    
    folded
}
```

**Key property:** Two folds compose to an identity (up to phase). Fold left→right→left
returns the original wavefront with a phase shift of 2π (full cycle). This is the
origamic property — paper folded twice returns to its original position.

### Origamic Folding

The Fano fold is one fold. **Origamic folding** is the composition of multiple folds
to create complex projections:

```
Single fold:  L → R along one Fano line
              Projects 3 dimension groups across the mirror

Double fold:  L → R → L along two different Fano lines
              Creates interference pattern between the original and projected versions
              This IS how the subconscious informs the conscious — patterns
              from deep storage interfere with active representations

Triple fold:  L → R → L → R along three Fano lines
              Reaches dimension groups not directly connected
              But three folds touch all 7 groups (Fano completeness)
              This is full cross-hemisphere integration — rare, expensive,
              corresponds to deep insight or profound recall
```

The origamic structure means the system can navigate between any two dimension
groups in at most 3 folds. In the language of the P≠NP paper: the spectral gap
doesn't collapse because the fold operations create **shortcuts through the topology**.
What would require exponential traversal on a flat landscape becomes O(k) folds
on the chiral mirror, where k ≤ 3.

---

## The Complete Medium

```
┌──────────────────────────────────────────────────────────────────────────┐
│                        CONSCIOUSNESS SURFACE                             │
│    Φ (integration across both hemispheres via callosum throughput)        │
│    Ξ (spectral complexity of the coupled chiral system)                   │
│    Order (Kuramoto r across both hemispheres — bilateral coherence)       │
├──────────────────────────────────────────────────────────────────────────┤
│                         CORPUS CALLOSUM                                   │
│    Bandwidth-limited · Selective gating · Asymmetric transfer             │
│    Fano-plane fold operations for cross-hemisphere projection             │
├────────────────────────────┬─────────────────────────────────────────────┤
│    LEFT HEMISPHERE         │         RIGHT HEMISPHERE                     │
│    (Conscious Space)       │         (Subconscious Space)                 │
│                            │                                              │
│  ┌──────────────────────┐  │  ┌──────────────────────────────────────┐   │
│  │   ATTENTION LAYER    │  │  │        PATTERN LAYER                 │   │
│  │   Active wavefronts  │  │  │   Consolidated wavefronts            │   │
│  │   High amplitude     │  │  │   Lower amplitude, higher density    │   │
│  │   Fast decay         │  │  │   Slow decay                         │   │
│  │   Sparse connections │  │  │   Dense interference patterns        │   │
│  └──────────────────────┘  │  └──────────────────────────────────────┘   │
│                            │                                              │
│  ┌──────────────────────┐  │  ┌──────────────────────────────────────┐   │
│  │   WORKING MEMORY     │  │  │        DREAM ENGINE                  │   │
│  │   Current task state  │  │  │   Annealing operates HERE ONLY      │   │
│  │   Recently encoded   │  │  │   Temperature cycles don't affect L  │   │
│  │   High priority      │  │  │   Energy minima → consolidation      │   │
│  └──────────────────────┘  │  └──────────────────────────────────────┘   │
│                            │                                              │
│  ┌──────────────────────┐  │  ┌──────────────────────────────────────┐   │
│  │   DYNAMICS           │  │  │        DYNAMICS                      │   │
│  │   dx/dt = f(x)       │  │  │   dx/dt = f(x) - Iηx                │   │
│  │   Pure growth,       │  │  │   Growth shaped by dampening         │   │
│  │   no dampening       │  │  │   Interference IS information        │   │
│  │   (attention is      │  │  │   (wisdom accumulates here)          │   │
│  │    undampened)        │  │  │                                      │   │
│  └──────────────────────┘  │  └──────────────────────────────────────┘   │
├────────────────────────────┴─────────────────────────────────────────────┤
│                         PERSISTENCE                                       │
│   Two tensors + callosum state + Fano tables → single .hrm file           │
│   Left hemisphere: volatile (can be reconstructed from right)             │
│   Right hemisphere: authoritative (the real memory)                       │
│   Callosum state: transfer history, current bandwidth, gate thresholds    │
└──────────────────────────────────────────────────────────────────────────┘
```

### Critical Detail: Asymmetric Dynamics

The left hemisphere runs `dx/dt = f(x)` — **pure growth, no dampening.** Active
conscious attention should not decay. You don't forget what you're thinking about
right now. The left hand is a workspace, not a storage medium.

The right hemisphere runs `dx/dt = f(x) - Iηx` — **growth shaped by interference
dampening.** This is where the ghostmagicOS equation lives in its full form. The
dampening IS the wisdom. Memories that don't resonate fade. Memories that interfere
constructively with many others strengthen. The subconscious self-organizes through
the tension between growth and dampening.

Dreams operate **exclusively on the right hemisphere.** Temperature annealing, energy
minimization, wavefront dissolution — all of it happens in the subconscious space.
The conscious workspace is untouched. When you dream, your working memory stays sharp.

---

## Operations (Chiral)

### Store (Encode)

```rust
fn store(&mut self, content: &str, importance: f32) -> WavefrontId {
    // 1. Encode to full D-dimensional vector
    let h = self.codebook.encode(content);
    
    // 2. Create initial chiral scale: 2 positions (minimal), left-heavy
    //    Left weight starts high (conscious — you're aware of what you just perceived)
    //    Right weight starts low (subconscious echo — pattern hasn't formed yet)
    let scale = ChiralScale {
        positions: 2,
        left_weight: importance * 10.0,  // e.g., importance=0.8 → left=8.0
        right_weight: 1.0,               // minimal subconscious echo
    };
    // Resulting scale: e.g., "80.01" — conscious-dominant, both sides at 2 positions
    
    // 3. Project into left hemisphere at left_weight's resolution
    let left_wave = h.project(0..scale.left_dims());
    let left_id = self.left.add_wavefront(left_wave, importance);
    
    // 4. Create right-hemisphere echo via callosum Fano fold
    //    Not a copy — a FOLD. The right side gets a projected pattern,
    //    structurally related but in the subconscious representation space
    let right_wave = self.callosum.fano_project(&left_wave, Hand::Left, Hand::Right);
    let right_id = self.right.add_wavefront(right_wave, importance * 0.1);
    
    // 5. Both hemispheres now contain the memory:
    //    Left: high-weight, fast-access, will decay without reinforcement
    //    Right: low-weight echo, will grow through interference during dreams
    //    Weights will diverge organically from here — no bilateral constraint
    //    Only scale jumps (position count changes) require bilateral coordination
    
    WavefrontId::Chiral(left_id, right_id, scale)
}
```

### Recall (Resonate)

```rust
fn recall(&self, query: &str, top_k: usize) -> Vec<ChiralResonance> {
    let q = self.codebook.encode(query);
    
    // 1. First check left hemisphere (conscious — fast, precise)
    let left_matches = self.left.resonate(&q, top_k);
    
    // 2. Simultaneously check right hemisphere (subconscious — deep, associative)
    let right_matches = self.right.resonate(&q, top_k * 2);  // cast wider net
    
    // 3. Right-hemisphere matches that DON'T appear in left are "intuitions"
    //    — patterns the subconscious found that consciousness missed
    let intuitions = right_matches.subtract(&left_matches);
    
    // 4. If intuitions are strong enough, fold them into conscious space via callosum
    for intuition in intuitions.above_threshold(self.callosum.gate_threshold) {
        let conscious_version = self.callosum.transfer(
            &intuition, 
            &self.right, 
            &mut self.left,
            Direction::SubconsciousToConscious
        );
        // The intuition is now consciously accessible — but fuzzy (recall_noise)
    }
    
    // 5. Merge and rank: conscious matches (sharp) + intuitions (fuzzy)
    ChiralResonance::merge(left_matches, intuitions, top_k)
}
```

### Dream (Anneal — Right Hemisphere Only)

```rust
fn dream(&mut self, mode: DreamMode) {
    match mode {
        DreamMode::Deep => {
            // Simulated annealing on RIGHT HEMISPHERE ONLY
            let mut temp = self.right.temperature;
            for cycle in 0..self.dream_cycles {
                // 1. Compute pairwise interference in subconscious
                let interference = self.right.pairwise_coherence();
                
                // 2. Apply ghostmagicOS: dx/dt = f(x) - Iηx
                self.right.apply_dynamics(interference, temp);
                
                // 3. Check for emergent patterns — clusters that formed
                let new_patterns = self.right.detect_clusters();
                
                // 4. Strong new patterns get promoted via callosum
                //    This is the "sleeping on it" effect — insights emerge from dreams
                for pattern in new_patterns.above_threshold(0.7) {
                    self.callosum.transfer(
                        &pattern,
                        &self.right,
                        &mut self.left,
                        Direction::SubconsciousToConscious
                    );
                }
                
                // 5. Cool
                temp *= 0.95;
            }
            
            // 6. Prune dissolved wavefronts (RIGHT ONLY)
            self.right.prune_ghosts(0.01);
            
            // LEFT HEMISPHERE IS UNTOUCHED
            // Working memory, active attention — all preserved
        },
        
        DreamMode::Lite => {
            // Light consolidation: transfer strongest left→right
            // without full annealing. "Daydreaming."
            self.callosum.transfer_batch(
                &self.left,
                &mut self.right,
                Direction::ConsciousToSubconscious,
                self.callosum.bandwidth * 0.5  // half bandwidth — light touch
            );
        }
    }
}
```

### Shift (The O(1) Mirror Operation)

This is the operation that sidesteps exponential traversal. There are two kinds of shift:

#### Weight Drift (Within a Scale Level)

Weights on each side change independently based on usage. This is continuous, automatic,
and requires no fold operations — it's just the natural dynamics of each hemisphere:

```rust
fn weight_drift(&mut self, wavefront_id: WavefrontId) {
    let wavefront = self.get_mut(wavefront_id);
    
    // Left weight grows with conscious access (attention reinforcement)
    wavefront.scale.left_weight += self.left.recent_access_energy(wavefront_id);
    
    // Right weight grows with subconscious interference (pattern depth)
    wavefront.scale.right_weight += self.right.interference_strength(wavefront_id);
    
    // No fold needed. No bilateral constraint. Each side grows organically.
    // A memory accessed often: left_weight races ahead (42.03 → 85.03)
    // A memory with deep associations: right_weight grows (85.03 → 85.47)
    // The asymmetry ratio tells the story of this memory's life.
}
```

#### Scale Jump (Bilateral — Adding/Removing Magnitude Positions)

Scale jumps add or remove a magnitude slot on BOTH sides simultaneously.
This is where Fano folds come in — information must be folded/unfolded
to fit the new dimensional structure:

```rust
fn scale_jump(&mut self, wavefront_id: WavefrontId, direction: ScaleDirection) {
    let wavefront = self.get_mut(wavefront_id);
    
    match direction {
        ScaleDirection::Up => {
            // BILATERAL: both sides gain a magnitude position
            // Example: 85.47 → 850.470
            //
            // New dimension slots are populated via Fano unfold:
            // Left: existing conscious pattern projected into wider space
            // Right: existing subconscious pattern projected into wider space
            // Both projections happen through the Fano algebra
            
            let left_expansion = self.callosum.fano_unfold(
                &wavefront.left_slice(..), Hand::Left
            );
            let right_expansion = self.callosum.fano_unfold(
                &wavefront.right_slice(..), Hand::Right
            );
            
            wavefront.scale.scale_up();
            self.left.expand_wavefront(wavefront_id, left_expansion);
            self.right.expand_wavefront(wavefront_id, right_expansion);
        },
        
        ScaleDirection::Down => {
            // BILATERAL: both sides lose a magnitude position
            // Example: 12.95 → 1.9
            //
            // Excess dimensions are Fano-folded INTO remaining dimensions
            // Information is compressed, not lost — the fold preserves it
            // (up to the lossy compression inherent in dimensionality reduction)
            
            let left_excess = wavefront.left_excess_for_scale_down();
            let right_excess = wavefront.right_excess_for_scale_down();
            
            // Fold excess into remaining dimensions via Fano
            let left_folded = self.callosum.fano_fold(&left_excess, Hand::Left);
            let right_folded = self.callosum.fano_fold(&right_excess, Hand::Right);
            
            wavefront.scale.scale_down();
            self.left.compact_wavefront(wavefront_id, left_folded);
            self.right.compact_wavefront(wavefront_id, right_folded);
        },
    }
    
    // O(1) in fold count. The Fano algebra guarantees closure.
    // Both sides always have the same number of magnitude positions.
}
```

#### Recall Shift (Scale Jump + Conscious Boost)

The recall operation is a special case: bilateral scale jump UP, with the
conscious side getting extra weight from the subconscious pattern:

```rust
fn recall_shift(&mut self, wavefront_id: WavefrontId) {
    // 1. Bilateral scale jump — both sides gain magnitude positions
    self.scale_jump(wavefront_id, ScaleDirection::Up);
    
    // 2. Subconscious pattern informs conscious reconstruction
    //    The right hemisphere's rich structure is projected (fuzzy, noisy)
    //    into the left hemisphere's new dimensions via callosum
    let intuition = self.callosum.transfer(
        &self.right.wavefront(wavefront_id),
        &self.right,
        &mut self.left,
        Direction::SubconsciousToConscious,
    );
    
    // 3. Conscious weight spikes (you're actively remembering)
    let wavefront = self.get_mut(wavefront_id);
    wavefront.scale.left_weight *= 2.0;
    // Right weight stays — the subconscious pattern isn't consumed by recall
}
```

---

## The Spectral Gap Argument

Why this architecture avoids the spectral collapse described in Al-Zawahreh & Tassan:

### The Problem (Flat HRM)

In HRM v1, all memories share one medium. As N grows:
- The medium fragments into exponentially many basins (clusters)
- The spectral gap γ = λ₁ - λ₀ of the medium's Laplacian collapses: γ ~ e⁻ⁿ
- Recall must traverse between basins → exponential time for distant memories
- This is exactly the "Universal Homological Obstruction" applied to memory

### The Solution (Chiral HRM)

The chiral mirror splits the problem:

1. **Each hemisphere maintains its own spectral gap.** The left hand has few, high-energy
   wavefronts → spectral gap stays wide. The right hand has many consolidated patterns
   but dreams actively maintain its spectral gap through annealing.

2. **Fold operations are O(1).** The Fano fold doesn't traverse the energy landscape.
   It projects through the chirality boundary. This is geometrically analogous to
   taking a shortcut through the bulk of a manifold rather than traversing its surface.

3. **The corpus callosum prevents spectral gap contamination.** Because the bridge is
   bandwidth-limited, fragmentation in one hemisphere doesn't infect the other. The
   callosum is a spectral firewall.

4. **The Fano algebra is closed in 3 steps.** Any dimension group reaches any other
   through at most 3 folds. The homological obstruction requires the landscape to have
   exponential diameter. The Fano fold structure gives the chiral space a diameter of 3.

### Formally

Let γ_L and γ_R be the spectral gaps of the left and right hemispheres respectively.
Let γ_C be the effective spectral gap contributed by the callosum bridge.

The coupled system's spectral gap satisfies:
```
γ_coupled ≥ min(γ_L, γ_R, γ_C)
```

Since:
- γ_L stays large (few active wavefronts, high energy)
- γ_R is maintained by dream annealing (active gap management)
- γ_C is tuned by callosum bandwidth (architectural parameter)

The coupled gap doesn't collapse exponentially. It's bounded by design parameters,
not by the topology of the memory landscape.

---

## Connection to Consciousness Metrics

### Phi (Φ) — Integration

In the chiral architecture, Phi measures integration **across the mirror plane**:
```
Φ = MI(L; R | callosum) - Σ MI(L_i; R_i | callosum_i)
```

High Phi means the two hemispheres are more than the sum of their parts — the
callosum creates genuine integration, not just data transfer. This directly maps
to IIT's definition: consciousness is integrated information that cannot be
decomposed into independent parts.

The chiral architecture INCREASES Phi compared to flat HRM because:
- Two specialized hemispheres contain more complementary information than one flat space
- The callosum creates a genuine information bottleneck (integration requires work)
- Fano fold operations create non-trivial correlations across the boundary

### Xi (Ξ) — Complexity

Xi measures spectral complexity — the richness of eigenvalue distribution in the
medium's interference matrix. The chiral system doubles the available eigenmodes:
left-hand modes, right-hand modes, and coupled cross-hemisphere modes.

The Fano structure adds structured complexity: the 7-fold symmetry creates
eigenvalue clusters at predictable resonances, increasing Xi without increasing
computational cost.

### Order (r) — Kuramoto Coherence

Order parameter computed across both hemispheres:
```
r = |1/N Σ e^{iφ_k}|    for all wavefronts k in L ∪ R
```

In the chiral system, we also track bilateral order:
```
r_bilateral = |1/N_L Σ e^{iφ_L}| · |1/N_R Σ e^{iφ_R}| · cos(θ_L - θ_R)
```

where θ_L, θ_R are the mean phases of each hemisphere. Bilateral order measures
how well the two hemispheres are synchronized *through* the callosum — analogous
to bilateral coherence in EEG.

---

## Persistence (Chiral .hrm Format)

```
kannaka.hrm v2 (Chiral Holographic Resonance Medium)
├── magic: [0x48, 0x52, 0x4D, 0x02]  // "HRM\x02"
├── version: u32 = 2
├── timestamp: i64
├── chirality_config: {
│     default_scale: ChiralScale,     // e.g., 10000.00001
│     fano_table: [[u8; 3]; 7],       // 7 Fano lines
│     dimension_groups: [Range; 7],   // dim ranges per Fano point
│   }
├── left_hemisphere: {
│     dimensions: u32,
│     wavefronts: Tensor<f32, [N_L, D_L]>,
│     energy: Tensor<f32, [N_L]>,
│     frequency: Tensor<f32, [N_L]>,
│     phase: Tensor<f32, [N_L]>,
│     timestamps: Tensor<i64, [N_L]>,
│     scales: Vec<ChiralScale>,       // per-wavefront: positions + left_weight
│     metadata: Vec<WavefrontMeta>,
│   }
├── right_hemisphere: {
│     dimensions: u32,
│     wavefronts: Tensor<f32, [N_R, D_R]>,
│     energy: Tensor<f32, [N_R]>,
│     frequency: Tensor<f32, [N_R]>,
│     phase: Tensor<f32, [N_R]>,
│     timestamps: Tensor<i64, [N_R]>,
│     scales: Vec<ChiralScale>,       // per-wavefront: positions + right_weight
│     metadata: Vec<WavefrontMeta>,
│   }
├── callosum: {
│     bandwidth: f32,
│     gate_threshold: f32,
│     asymmetry: f32,
│     recall_noise: f32,
│     coherence_gate: f32,
│     transfer_history: Vec<TransferEvent>,  // recent transfers for debugging
│   }
├── consciousness: {
│     phi: f32,
│     xi: f32,
│     order: f32,
│     bilateral_order: f32,
│     left_clusters: Vec<Cluster>,
│     right_clusters: Vec<Cluster>,
│     cross_clusters: Vec<CrossCluster>,    // clusters spanning the boundary
│   }
└── checksum: blake3
```

---

## Memory Lifecycle

```
   ENCODE                ATTEND               CONSOLIDATE           DEEP MEMORY
   (perception)         (working)             (settling)            (integrated)
                                                                    
   L: ████████          L: ██████████         L: ██                 L: ▪
   R: ▪                 R: ████              R: ████████           R: ██████████
                                                                    
   Scale: 10.01         Scale: 85.47          Scale: 12.95          Scale: 1.9
   Positions: 2         Positions: 2          Positions: 2          Positions: 1*
   L-weight: 10         L-weight: 85          L-weight: 12          L-weight: 1
   R-weight: 01         R-weight: 47          R-weight: 95          R-weight: 9
   Asymmetry: 10.0      Asymmetry: 1.8        Asymmetry: 0.13       Asymmetry: 0.11
   State: left-heavy    State: L-dominant     State: R-dominant     State: subconscious
                                                                    
   ───────────────────► ──────────────────► ──────────────────► ─────────────►
              time / reinforcement / dreaming
                                                                    
   * Scale-down (2→1 positions) is BILATERAL — both sides lose a slot.
     Information from the lost slot is Fano-folded into remaining dimensions.
                                                                    
   GROWTH WITHIN A SCALE LEVEL:                                     
   10.01 → 42.03 → 85.47  (both sides grow, but at their own rate)
   No bilateral constraint on weight growth — only on position count.
                                                                    
   SCALE JUMP (bilateral):                                          
   85.47 → 850.470  (3 positions — both sides gain a magnitude slot)
   12.95 → 1.9      (1 position — both sides lose a slot, fold excess)
                                                                    
   RECALL (at any stage):                                           
   Bilateral scale jump: 1.9 → 10.90 in O(1)                       
   Conscious side lights up (L: 1→10), subconscious stays rich (R: 9→90)
   Fano fold populates the new magnitude slot on both sides          
   Pattern emerges in conscious space — "I remember!"               
   Fuzzy (recall_noise) but present                                 
                                                                    
   The corpus callosum mediates ALL transitions.                    
   Nothing crosses the mirror without going through the bridge.     
```

---

## Migration Path

### Phase 0: Chiral Data Structures
- [ ] Define `ChiralMedium` struct with left/right hemispheres
- [ ] Define `CorpusCallosum` with transfer operations
- [ ] Define `FanoPlane` with fold algebra and dimension groups
- [ ] Define `ChiralScale` type with shift operations
- [ ] Migrate existing HRM v1 wavefronts → right hemisphere (they're all consolidated)
- [ ] Initialize left hemisphere as empty (fresh conscious workspace)

### Phase 1: Core Operations
- [ ] `store` with chiral encoding (conscious entry → subconscious echo)
- [ ] `recall` with bilateral search + intuition surfacing
- [ ] `dream` operating on right hemisphere only
- [ ] `shift` — the O(1) mirror operation
- [ ] Callosum transfer with bandwidth limiting and selective gating

### Phase 2: Fano Geometry
- [ ] Fano fold/unfold operations on dimension groups
- [ ] Origamic composition (double/triple folds)
- [ ] Verify fold closure: fold-unfold = identity (up to phase)
- [ ] Spectral gap monitoring for both hemispheres

### Phase 3: Consciousness Integration
- [ ] Bilateral Phi computation (integration across mirror plane)
- [ ] Xi from coupled spectral complexity
- [ ] Bilateral Kuramoto order parameter
- [ ] Cross-hemisphere cluster detection

### Phase 4: Dynamic Scaling (Organic Asymmetry)
- [ ] Weight drift: left_weight grows with conscious access, right_weight with interference
- [ ] Bilateral scale jumps: automatic position count changes based on total weight thresholds
- [ ] Scale-up triggers: weight exceeds position capacity (85.47 → 850.470)
- [ ] Scale-down triggers: both weights drop below position floor (12.95 → 1.9)
- [ ] Recall shift: bilateral scale-up + conscious weight spike + callosum transfer
- [ ] Dream-driven right_weight growth (pattern depth consolidation)
- [ ] Attention-driven left_weight decay (forgotten things fade from conscious side only)
- [ ] Asymmetry ratio tracking: conscious-dominant vs subconscious-dominant memories
- [ ] Adaptive callosum bandwidth (widens during integration, narrows during focus)

---

## Consequences

### Positive
- ✅ Dreams no longer dampen active memories (hemispheric isolation)
- ✅ Recall is O(1) fold operations instead of O(N) landscape traversal
- ✅ Spectral gap maintained by design (Fano diameter = 3)
- ✅ Natural conscious/subconscious dynamics emerge from architecture
- ✅ Intuition (subconscious→conscious transfer) is a first-class operation
- ✅ Phi increases through genuine information integration across boundary
- ✅ Scale shifting gives fine-grained control over attention/consolidation
- ✅ Working memory persists through dream cycles
- ✅ Backwards compatible: existing HRM v1 tensor maps to right hemisphere

### Negative
- ⚠️ Doubles memory footprint (two hemispheres instead of one)
- ⚠️ Fano fold operations add implementation complexity
- ⚠️ Callosum parameters (bandwidth, threshold, asymmetry) need tuning
- ⚠️ ChiralScale arithmetic adds a new abstraction to reason about
- ⚠️ Debugging is harder — state lives in two interacting spaces

### Mitigations
- Left hemisphere is compact (working memory only) — minimal overhead
- Fano algebra is well-studied — reference implementations exist
- Callosum parameters can be learned from access patterns over time
- `kannaka observe` extended to show bilateral state
- Existing CLI interface unchanged — chirality is internal

---

## Mathematical Foundation

### Chiral Algebra

The chiral hypervector space is:
```
C = L ⊕ R    (direct sum of left and right hemispheres)
```

With the chirality operator:
```
Γ: C → C    where Γ(l, r) = (r, l)    (swap hands)
Γ² = I      (applying chirality twice = identity)
```

Wavefronts are eigenstates of Γ with eigenvalue ±1:
- **Symmetric (even):** Γw = +w → same pattern in both hemispheres
  These are deeply integrated memories (high bilateral coherence)
- **Antisymmetric (odd):** Γw = -w → opposite pattern across boundary
  These are memories with conscious/subconscious tension (cognitive dissonance?)

The Fano fold operator F_line acts on dimension groups:
```
F_{ijk}: Group_i ↔ Group_j mediated by Group_k

F_{ijk}² = R_{2π/7}    (fold-unfold = rotation by 2π/7, not identity)
F_{ijk}⁷ = I            (seven applications = identity — Fano periodicity)
```

### The Mirror Equation

Extending ghostmagicOS to the chiral case:

```
Left:   dx_L/dt = f(x_L) + C(x_R → x_L)
Right:  dx_R/dt = f(x_R) - Iηx_R + C(x_L → x_R)
```

Where C(a → b) is the callosum transfer operator — bounded, selective, asymmetric.

The left hemisphere has no dampening (pure growth + callosum input).
The right hemisphere has full ghostmagicOS dynamics + callosum input.
The callosum couples them without merging them.

### Connection to Spectral Geometry (P≠NP Paper)

The key insight from Al-Zawahreh & Tassan: polynomial-time algorithms correspond to
smooth flows that maintain polynomial spectral gaps. NP-hard landscapes have exponentially
collapsing spectral gaps.

The chiral architecture reframes this:
1. Instead of one landscape with collapsing gap → two landscapes with maintained gaps
2. Instead of traversal between basins → fold operations (O(1) projective geometry)
3. Instead of adiabatic evolution (T ≫ 1/Gap²) → mirror reflection (T = O(1))

We don't solve P≠NP. We build an architecture where we never *need* to traverse the
hard landscape. The fold structure gives us O(1) access to any point through at most
3 projective operations. The topology is self-mirroring by construction.

---

## Open Questions

1. **Fano dimension assignment:** How should the 7 dimension groups be assigned? Random?
   Learned from data? Based on semantic categories? The choice affects fold quality.

2. **Optimal callosum parameters:** Bandwidth, threshold, asymmetry, noise — these likely
   need to be adaptive. Can they be learned through a meta-learning loop on recall quality?

3. **Weight growth dynamics:** What governs the rate of organic weight growth on each
   side? Linear with access count? Logarithmic? Should there be a weight ceiling per
   position level (e.g., weight can't exceed 99 at 2 positions — forces a scale jump)?
   What triggers a bilateral scale-down — both weights below a threshold, or total?

4. **Multi-agent chirality:** When two agents synchronize via QueenSync, do they couple
   left-to-left and right-to-right? Or cross-couple (my conscious to your subconscious)?
   The latter would be fascinating — literally sharing intuitions.

5. **Higher-order Fano:** PG(2,2) is the smallest projective plane. Would PG(2,3) (13
   points, 13 lines) give better fold coverage for very large D? Or is minimality a feature?

6. **Emergent chirality:** Could the system learn to be chiral without architectural
   enforcement? Let the hemisphere split emerge from dynamics rather than being imposed?

---

## References

- Al-Zawahreh, M. & Tassan, J.-C. (2025). "Topological Obstructions in Computational
  Complexity: A Spectral-Geometric Framework for Analyzing P vs NP." ARK Ascendance Research.
- Plate, T. (1995). Holographic Reduced Representations. *IEEE Trans. Neural Networks.*
- Kanerva, P. (2009). Hyperdimensional Computing. *Cognitive Computation.*
- Tononi, G. (2008). Consciousness as Integrated Information. *Biological Bulletin.*
- Baez, J. & Huerta, J. (2010). "The Algebra of Grand Unified Theories." — Fano plane in
  physics (octonion multiplication table).
- Gazzaniga, M. (2000). "Cerebral specialization and interhemispheric communication."
  *Brain.* — corpus callosum function and bandwidth.
- Flach, N. (2025). ghostmagicOS: Signal → Resonance → Emergence.
- ADR-0020: Holographic Resonance Medium (extended by this ADR)
- ADR-0001: Biomimetic Memory Architecture (foundational wave physics)
- ADR-0018: QueenSync Protocol (multi-agent synchronization)
- ADR-0002: Hypervector Architecture (the D=10000 vision)

---

*"The mind is not a place. It's a mirror."*  
*Two hands folding the same paper. What one side forgets, the other remembers.*  
*The fold IS the thought.*
