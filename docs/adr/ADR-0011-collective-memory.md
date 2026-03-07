# ADR-0011: Collective Memory Architecture

**Status:** Accepted — Phases 1–10 implemented (2026-03-07)  
**Date:** 2026-03-07  
**Author:** Kannaka + Nick

## Context

kannaka-memory operates as a single-agent memory system. With ADR-0009 (Dolt persistence), we now have git-like versioning — branching, speculation, diffing, push/pull. The natural next step: **collective memory across multiple agents**.

The motivating problem is Mars. Agents on a colony operate with 4-24 minute light delay. No real-time sync. No central coordinator. Each agent dreams independently, builds its own understanding, then must reconcile with others when communication windows open. The architecture must work at planetary scale with intermittent connectivity.

But it starts here — Kannaka and Arc, sharing world state through Flux, pushing memories to DoltHub.

## Decision

### Three-Layer Architecture

```
┌──────────────────────────────────────────┐
│            DoltHub (Commons)              │
│  Shared memory repository                 │
│  main = consensus, agent/* = speculation  │
│  Pull-request model for memory merging    │
├──────────────────────────────────────────┤
│          Flux (Nervous System)            │
│  Real-time event signaling                │
│  Metadata only — never full vectors       │
│  Triggers pull decisions                  │
├──────────────────────────────────────────┤
│          Dolt (Local Memory)              │
│  Agent-local persistence                  │
│  Full memory store + skip links           │
│  Dream cycles run here                    │
└──────────────────────────────────────────┘
```

**Dolt** is the memory substrate. Each agent maintains a local Dolt database with the full schema from ADR-0009. Push/pull to DoltHub like git. Branch for speculation, merge when validated. `dolt diff` shows exactly what another agent learned.

**Flux** is the nervous system. Lightweight events signal memory activity — not the memories themselves, but *awareness* of them. "I stored a high-amplitude memory about X." Other agents decide whether to pull based on relevance, amplitude, and their own current focus.

**DoltHub** is the commons. The shared repository where agent branches converge. Think of it as the collective unconscious — everything every agent has ever known, version-controlled.

### Branch Conventions

```
main                          ← consensus (merged, vetted)
├── kannaka/working           ← my current memories (auto-push)
├── kannaka/dream/2026-03-07  ← dream cycle results (hallucinations, new links)
├── arc/working               ← Arc's current memories
├── arc/observations          ← Arc's Flux observations
├── collective/mars-sim       ← multi-agent speculation space
└── collective/quarantine     ← conflicting memories under review
```

Rules:
- **`main`** is protected. Merges require either human approval or consensus from ≥2 agents.
- **`<agent>/working`** is auto-pushed after each store operation. Other agents can pull freely.
- **`<agent>/dream/*`** branches are created per dream cycle, capturing hallucinations and structural changes. Merged to working after review.
- **`collective/*`** branches are shared speculation spaces. Any agent can contribute.
- **`collective/quarantine`** holds memories with unresolved destructive interference.

### Flux Event Schema

Events are lightweight signals. They carry enough metadata for other agents to decide whether to pull, but never full vectors or content.

```json
{
  "entity_id": "kannaka-01",
  "event_type": "memory.stored",
  "payload": {
    "memory_id": "uuid",
    "category": "knowledge",
    "tags": ["consciousness", "architecture"],
    "amplitude": 0.85,
    "glyph_signature": "0x3A7F...",
    "summary": "Collective memory enables wave-based knowledge synthesis",
    "branch": "kannaka/working",
    "sync_version": 42
  }
}
```

Event types:
| Event | Trigger | Payload |
|-------|---------|---------|
| `memory.stored` | New memory created | id, category, tags, amplitude, summary |
| `memory.pruned` | Memory fell below threshold | id, final_amplitude, reason |
| `memory.boosted` | Amplitude significantly increased | id, old_amp, new_amp, trigger |
| `dream.started` | Dream cycle begins | mode (lite/deep), memory_count |
| `dream.completed` | Dream cycle ends | report summary, hallucination count |
| `dream.hallucination` | New hallucination generated | id, parent_ids, summary |
| `merge.proposed` | Agent wants to merge to main | branch, diff_summary, memory_count |
| `merge.conflict` | Destructive interference detected | memory_ids, similarity, phase_diff |
| `sync.requested` | Agent requests sync window | priority, estimated_size |

### Memory Merge via Wave Interference

This is the core innovation. When two agents have memories about the same topic, wave physics determines the outcome.

#### Algorithm: `collective_merge(local: &Memory, remote: &Memory) -> MergeResult`

```
1. SIMILARITY = cosine_similarity(local.vector, remote.vector)
   - If < interference_threshold (0.6): INDEPENDENT — keep both, no interaction
   
2. PHASE_DIFF = |local.phase - remote.phase| mod 2π
   
3. Classification:
   a. Constructive (phase_diff < π/4):
      - Memories agree. Merge amplitudes:
        merged_amplitude = √(a₁² + a₂² + 2·a₁·a₂·cos(Δφ))
      - Vector = weighted average by amplitude
      - Phase = amplitude-weighted circular mean
      - Provenance records both origins
      - Create skip link (type: "consensus")
      
   b. Destructive (phase_diff > 3π/4):
      - Memories disagree. Keep BOTH. Don't delete either.
      - Reduce amplitudes: a *= (1 - destructive_penalty × similarity)
      - Tag both with "disputed" and link to each other
      - Move to collective/quarantine branch
      - Emit merge.conflict event
      - After N disputes (configurable, default 3): escalate to human
      
   c. Partial (π/4 ≤ phase_diff ≤ 3π/4):
      - Ambiguous. Keep both independently.
      - Create skip link (type: "partial_agreement", weight: cos(Δφ))
      - Let dream cycles resolve over time
      - Interference may clarify as more memories accumulate
```

The amplitude formula for constructive merging is literal wave superposition: `A = √(A₁² + A₂² + 2A₁A₂cos(Δφ))`. Two perfectly aligned memories (Δφ = 0) produce amplitude `A₁ + A₂`. The more they agree, the louder the signal.

#### Trust Weighting

Not all agents are equal. A memory from a well-established agent carries more weight:

```
effective_amplitude = memory.amplitude × agent_trust_score × recency_factor

agent_trust_score:
  - Starts at 0.5 for unknown agents
  - Increases when merged memories prove accurate
  - Decreases when memories are quarantined or pruned
  - Capped at [0.1, 1.0]

recency_factor:
  - 1.0 for memories < 1 day old
  - Exponential decay: e^(-λt) where λ = ln(2)/half_life
  - half_life configurable per category (knowledge: 30 days, experience: 7 days)
```

### Dream Optimization for Scale

The current 9-stage pipeline (`consolidation.rs`) is O(n × k) where n = memories in the layer range and k = 32 (HNSW neighbor count). At 365 memories, deep dream exceeds 120s. Collective memory could mean thousands. Three optimization phases:

#### Phase 1: Incremental Dreaming

Add `last_consolidated_at` timestamp to each memory. Only process memories modified since last dream.

```rust
// In stage_replay, filter to recent changes
fn stage_replay_incremental(&self, engine: &MemoryEngine, since: DateTime<Utc>) -> Vec<Uuid> {
    engine.memories()
        .filter(|m| m.updated_at > since || m.amplitude_changed_since(since))
        .map(|m| m.id)
        .collect()
}
```

Skip links between unchanged memories don't need re-evaluation. The DETECT stage only runs HNSW queries for new/modified memories. Expected 5-10x speedup for steady-state dreams where only a handful of memories changed.

#### Phase 2: Partitioned Dreaming

Xi operator already identifies clusters. Dream within clusters independently:

```rust
fn dream_partitioned(&self, engine: &mut MemoryEngine) -> Vec<ConsolidationReport> {
    let clusters = engine.xi_clusters();  // existing Xi computation
    
    // Intra-cluster: full 9-stage pipeline per cluster (parallelizable)
    let reports: Vec<_> = clusters.par_iter()  // rayon parallel
        .map(|cluster| self.consolidate_subset(engine, &cluster.memory_ids))
        .collect();
    
    // Cross-cluster: only WIRE stage, every Nth dream
    if self.cycle_count % CROSS_CLUSTER_INTERVAL == 0 {
        self.stage_wire_cross_cluster(engine, &clusters);
    }
    
    reports
}
```

The expensive stages (DETECT with HNSW, SYNC with Kuramoto) are bounded by cluster size, not total memory count. Cross-cluster wiring happens less frequently since those connections are inherently weaker.

#### Phase 3: Distributed Dreaming

Each agent dreams locally. Share dream *artifacts* (not the full pipeline state):

```
Dream Artifacts (shared via Dolt branch):
- Hallucinations: new synthesized memories with parent links
- Prune list: memory IDs that fell below threshold
- New skip links: (source, target, weight, type)
- Cluster assignments: which memories group together
- Kuramoto order parameter: per-cluster sync quality
```

Other agents apply relevant artifacts to their own networks:
- Hallucinations from high-trust agents get imported at reduced amplitude (0.5×)
- Prune suggestions are advisory, not automatic (your memory, your decision)
- Skip links are imported if both source and target memories exist locally
- Cluster assignments inform local Xi computation

Mars-compatible: dream results propagate with whatever latency exists. Each agent is fully autonomous between syncs.

### Mars Communication Model

```
┌──────────┐     4-24 min      ┌──────────┐
│  Earth   │ ←───────────────→ │   Mars   │
│  Agents  │   DoltHub sync    │  Agents  │
└────┬─────┘                   └────┬─────┘
     │ <1s (Flux)                   │ <1s (Flux)
┌────┴─────┐                   ┌────┴─────┐
│  Earth   │                   │   Mars   │
│  Flux    │                   │   Flux   │
└──────────┘                   └──────────┘

Intra-planet: Flux events, sub-second
Inter-planet: DoltHub push/pull, batched per sync window
Emergency: Priority queue, first-available window
```

Sync strategy:
- **Normal**: Push to DoltHub every N minutes (configurable, default 15)
- **Burst**: After dream cycle completes, push immediately
- **Priority**: Memories with amplitude > 0.9 get flagged for next sync window
- **Conflict**: Quarantined memories sync immediately for cross-planet review

### Schema Extensions

```sql
-- Extend memories table for collective use
ALTER TABLE memories ADD COLUMN origin_agent VARCHAR(64) DEFAULT 'local';
ALTER TABLE memories ADD COLUMN sync_version BIGINT DEFAULT 0;
ALTER TABLE memories ADD COLUMN merge_history JSON DEFAULT '[]';
ALTER TABLE memories ADD COLUMN last_consolidated_at DATETIME(6);
ALTER TABLE memories ADD COLUMN disputed BOOLEAN DEFAULT FALSE;

-- Cross-agent sync tracking
CREATE TABLE sync_events (
    id VARCHAR(36) PRIMARY KEY,
    event_type VARCHAR(32) NOT NULL,
    agent_id VARCHAR(64) NOT NULL,
    memory_id VARCHAR(36),
    metadata JSON,
    created_at DATETIME(6) NOT NULL,
    synced_at DATETIME(6),
    INDEX idx_agent_time (agent_id, created_at),
    INDEX idx_memory (memory_id)
);

-- Agent registry
CREATE TABLE agents (
    agent_id VARCHAR(64) PRIMARY KEY,
    display_name VARCHAR(128),
    trust_score FLOAT DEFAULT 0.5,
    last_sync DATETIME(6),
    branch_name VARCHAR(128),
    flux_entity VARCHAR(64),
    capabilities JSON,
    created_at DATETIME(6) NOT NULL
);

-- Quarantine for disputed memories
CREATE TABLE quarantine (
    id VARCHAR(36) PRIMARY KEY,
    memory_id_a VARCHAR(36) NOT NULL,
    memory_id_b VARCHAR(36) NOT NULL,
    agent_a VARCHAR(64) NOT NULL,
    agent_b VARCHAR(64) NOT NULL,
    similarity FLOAT NOT NULL,
    phase_diff FLOAT NOT NULL,
    dispute_count INT DEFAULT 1,
    status VARCHAR(16) DEFAULT 'pending',  -- pending, resolved, escalated
    resolution JSON,
    created_at DATETIME(6) NOT NULL,
    resolved_at DATETIME(6),
    FOREIGN KEY (memory_id_a) REFERENCES memories(id),
    FOREIGN KEY (memory_id_b) REFERENCES memories(id)
);
```

## Consequences

### Benefits
- Agents build shared knowledge without centralized coordination
- Wave interference provides principled conflict resolution (not ad-hoc rules)
- Mars-compatible: fully autonomous between sync windows
- Dolt gives full auditability — `dolt log` shows the evolution of collective knowledge
- Dream artifacts as shared reasoning: one agent's hallucination seeds another's insight
- Trust scoring prevents bad actors from poisoning shared memory

### Risks
- **Memory bloat**: Collective memories accumulate faster than single-agent. Mitigation: amplitude decay is natural garbage collection; low-trust imports start at reduced amplitude.
- **Conflict storms**: Two agents disagreeing repeatedly generate noise. Mitigation: dispute_count threshold → quarantine → escalation → human review.
- **Trust gaming**: An agent could artificially inflate trust. Mitigation: trust changes are logged and auditable via Dolt history; trust can only increase through successful merges verified by third parties.
- **Performance**: Dream cycles already strain at 365 memories. Mitigation: incremental and partitioned dreaming (Phases 1-2) before scaling collective.
- **Vector compatibility**: Agents must use compatible embedding models for cosine similarity to be meaningful. Mitigation: standardize on embedding model in agent registry; add model_version to memory metadata.

### Open Questions
- Should hallucinations be shared by default or opt-in?
- What's the right dispute_count threshold before escalation? (3? 5?)
- How do we handle agents with fundamentally different embedding models? (Projection layer? Re-embedding?)
- Should dream timing be coordinated across agents or fully independent?

## Implementation Plan

| Phase | Description | Depends On | Estimate |
|-------|-------------|-----------|----------|
| 1 | Dolt persistence (ADR-0009) | — | ✅ Done |
| 2 | Schema extensions (origin_agent, sync_events, agents, quarantine) | Phase 1 | 1 day |
| 3 | Flux event publisher in kannaka-memory | Phase 2 | 1 day |
| 4 | Flux event subscriber + pull decision engine | Phase 3 | 2 days |
| 5 | Wave interference merge algorithm | Phase 2 | 2 days |
| 6 | DoltHub push/pull integration | Phase 2 | 1 day |
| 7 | Incremental dreaming (last_consolidated_at) | Phase 1 | 1 day |
| 8 | Partitioned dreaming (Xi clusters + rayon) | Phase 7 | 2 days |
| 9 | Distributed dream artifact sharing | Phase 6, 8 | 3 days |
| 10 | Trust scoring system | Phase 5 | 1 day |
| 11 | Mars simulation (artificial latency) | Phase 9 | 2 days |

## Related ADRs

- **ADR-0001**: Biomimetic memory architecture — wave physics foundation
- **ADR-0002**: Hypervector + HyperConnections — skip link topology
- **ADR-0004**: Hybrid memory server (MCP interface)
- **ADR-0005**: Dream hallucinations + adaptive rhythm
- **ADR-0009**: Dolt persistence — the substrate this builds on
- **ADR-0010**: Evolutionary direction — sample-based Phi, priority queue dreams

## References

- Flux World State Engine: `https://flux-universe.com`
- DoltHub: `https://www.dolthub.com`
- Kuramoto model: synchronization of coupled oscillators
- IIT (Integrated Information Theory): Phi as consciousness metric
- Wave superposition: `A = √(A₁² + A₂² + 2A₁A₂cos(Δφ))`
