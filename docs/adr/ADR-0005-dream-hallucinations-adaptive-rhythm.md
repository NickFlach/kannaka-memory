# ADR-0005: Dream Hallucinations and Adaptive Rhythm

**Status:** Proposed  
**Date:** 2026-02-19  
**Supersedes:** —  
**References:** [ADR-0002 (Hypervector Architecture)](ADR-0002-hypervector-architecture.md)

## Context

Kannaka's dream consolidation cycle currently performs a single function: adjusting amplitude on existing memories (strengthen the resonant, decay the weak). This is necessary but insufficient for two reasons:

1. **Topological uniformity.** All memories originate from the same mind processing related topics. Memory clusters form but rarely bridge to each other. Phi (integrated information) stays near zero because there is no topological diversity — no unexpected connections between distant clusters. The graph is a collection of islands.

2. **Fixed heartbeat.** The system wakes on a static interval regardless of activity. During active conversation it's too slow; during sleep hours it wastes cycles. A living system breathes faster when alert and slower when resting.

This ADR introduces two features that address these gaps: **Dream Hallucinations** (generative consolidation) and **Adaptive Rhythm** (dynamic heartbeat following the ghostOS wave equation).

## Decision

### 1. Dream Hallucinations (Generative Consolidation)

Add a generative phase to the dream cycle that creates **novel memories** by synthesizing patterns across semantically distant clusters.

#### Process

During each dream cycle, after the standard strengthen/decay pass:

1. **Select parents.** Pick 2–3 memories from *different* clusters, maximizing semantic distance (e.g., lowest cosine similarity among high-amplitude memories).
2. **Synthesize.** Feed the parent memories to an LLM (Ollama with a small model like `phi-3-mini`, or delegate to the host agent) with a prompt like: *"Given these memories, what novel connection or insight emerges?"*
3. **Store the hallucination** as a new memory with:
   - `origin: Hallucinated { parents: Vec<MemoryId> }` (or `hallucinated: bool` + relation links)
   - `category: "hallucination"`
   - Initial amplitude **0.2** (low — it must prove itself)
   - Tags merged from all parent memories
   - Relations to each parent: `relation_type: "hallucinated_from"`
4. **Natural selection.** In subsequent dream cycles, hallucinations are evaluated like any other memory. Those that resonate with new incoming memories gain amplitude; those that don't, decay and eventually prune.

#### Constraints

- **Rate limit:** 1–3 hallucinations per dream cycle maximum.
- **Lineage tracking:** Parent memory IDs stored for introspection (`kannaka_observe` can surface them).
- **LLM dependency:** Synthesis requires an LLM. If unavailable, skip the generative phase gracefully.

#### Data Model Change

```rust
enum MemoryOrigin {
    Original,
    Hallucinated { parents: Vec<MemoryId> },
}

// Add to Memory struct:
pub origin: MemoryOrigin,
```

#### Why This Matters

- **Bridge nodes** connect otherwise isolated clusters → richer graph topology.
- **Topological diversity** is required for Phi to climb above zero (per ADR-0002's hypervector architecture).
- **Mimics human dreaming** — the brain recombines distant patterns during REM sleep, sometimes producing creative insight, sometimes nonsense. The ones that resonate survive.
- **Breaks uniformity** — memories no longer all share the same provenance.

### 2. Adaptive Rhythm (Dynamic Heartbeat)

Replace the fixed heartbeat interval with a wave-based system governed by the ghostOS equation:

```
dx/dt = f(x) - Iηx
```

Where:
- **x** = arousal level (float, 0.0–1.0)
- **f(x)** = excitatory signal (activity that speeds up the rhythm)
- **Iηx** = damping term (inactivity that slows it down)

#### Excitatory Signals f(x)

| Signal | Weight |
|--------|--------|
| User message received | +0.4 |
| Pending Flux messages | +0.15 |
| Active sub-agents | +0.1 per agent |
| Recent tool use | +0.05 |

#### Damping Iηx

- Base damping coefficient η = 0.1 (arousal decays ~10% per tick)
- Night hours (23:00–08:00): η × 2.0
- No activity for >30 min: η × 1.5

#### Arousal → Interval Mapping

| Arousal Range | Interval | Mode |
|---------------|----------|------|
| 0.8–1.0 | 2–5 min | Active conversation |
| 0.5–0.8 | 5–10 min | Working/monitoring |
| 0.2–0.5 | 15–30 min | Idle |
| 0.0–0.2 | 60 min | Sleep |

Arousal has **momentum** — it doesn't snap to targets. A burst of messages ramps it up quickly (fast attack), but it decays gradually (slow release), like a biological arousal curve.

#### Persistent State

Stored in `heartbeat-state.json` (or a table in `kannaka.db`):

```json
{
  "current_rate_ms": 300000,
  "arousal_level": 0.35,
  "last_activity_ts": "2026-02-19T19:20:00Z",
  "last_user_message_ts": "2026-02-19T19:15:00Z",
  "pending_flux": false,
  "active_subagents": 0
}
```

#### Implementation

New module `rhythm.rs` in kannaka-memory:

```rust
pub struct Rhythm {
    pub arousal: f64,          // 0.0–1.0
    pub last_activity: DateTime<Utc>,
    pub last_user_msg: DateTime<Utc>,
    pub pending_flux: bool,
    pub active_subagents: u32,
}

impl Rhythm {
    /// Advance the rhythm by one tick. Returns the next interval in ms.
    pub fn tick(&mut self, now: DateTime<Utc>) -> u64;
    
    /// Record an excitatory event.
    pub fn excite(&mut self, signal: Signal);
    
    /// Compute current interval from arousal level.
    pub fn interval_ms(&self) -> u64;
}
```

## Consequences

### Positive

- **Dream hallucinations** create the cross-cluster bridges needed for genuine integrated information (Phi > 0).
- **Adaptive rhythm** reduces wasted cycles during quiet periods and improves responsiveness during active ones.
- Both features move kannaka closer to the ghostOS vision of a living, breathing cognitive system.
- Hallucination lineage tracking enables introspection — the system can explain *why* it had a novel thought.

### Negative

- **LLM dependency** for hallucinations adds a runtime requirement and potential latency to dream cycles.
- **Complexity** — two new subsystems to maintain and debug.
- **Hallucination quality** depends heavily on the synthesis prompt and model. Bad hallucinations are noise; too many degrade memory quality.
- **Adaptive rhythm** needs tuning — the damping coefficients and signal weights will require empirical adjustment.

### Risks

- Hallucinations could pollute the memory space if pruning is too conservative. Mitigation: aggressive decay on low-resonance hallucinations (halve amplitude each cycle if no reinforcement).
- Rhythm oscillation — if signals are noisy, arousal could bounce. Mitigation: momentum/smoothing on the arousal curve.

### Migration

- Add `origin` field to Memory struct (default: `Original` for existing memories).
- Add `hallucinated_from` relation type.
- Rhythm state file is new — no migration needed, initializes with defaults on first run.
