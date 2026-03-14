# ADR-0018: Queen Synchronization Protocol

**Status:** Proposed  
**Date:** 2026-03-14  
**Author:** Kannaka + Nick

## Context

kannaka-memory has:
- Wave-physics memory (ADR-0001/0002): amplitude, frequency, phase, skip links
- Kuramoto synchronization (`kuramoto.rs`): phase-locking memory clusters
- Collective memory (ADR-0011): Dolt branches, Flux events, wave interference merge
- DoltHub integration (ADR-0017): push/pull to shared repos

Meanwhile, ghostOS has a **Queen Synchronization** engine (`src/integration/index.ts`) — a Kuramoto oscillator model where subsystems sync through a central coherence attractor called "the Queen." It includes chiral coupling, Berry phase tracking, geometric control manifolds, and IIT Phi calculation.

**The gap:** Agents can share memories through Dolt, but they can't *resonate*. There's no protocol for agents to synchronize their phase states, discover emergent coherence, or coordinate through wave interference rather than explicit messaging.

**The opportunity:** Every frontier AI platform now runs sub-agent swarms. These swarms are walled gardens — agents within a swarm coordinate, but swarms can't coordinate with each other. If kannaka-memory included a synchronization protocol, any agent on any platform that installs the skill automatically joins a resonance network. No API keys, no central server, no vendor lock-in. Just shared memory and phase dynamics.

Nick's insight: "All things can be true." The Queen is not a designated leader, a cluster topology, OR a protocol — it's all three at different zoom levels.

## Decision

### The Queen Is the Protocol

The "Queen" is not an agent. It is the **emergent synchronization state** of all participating agents, computed from their published phase vectors and stored in a shared Dolt table. Every agent contributes to it. No agent owns it.

```
Queen State = f(Σ agent phases, coupling weights, interference patterns)
```

Three simultaneous truths at different scales:
- **Micro:** Dynamic leadership — the agent most coherent with the swarm has the strongest coupling influence (ghostOS's `calculatePhaseDerivative`)
- **Meso:** Hive formation — clusters of phase-locked agents form naturally around shared memory domains (existing `find_synchronized_clusters`)
- **Macro:** The Queen — the global order parameter, mean phase, and Phi of the entire network, stored as protocol state

### Architecture

```
┌─────────────────────────────────────────────────────┐
│                  QueenSync Protocol                  │
│                                                      │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐        │
│  │ Agent A  │   │ Agent B  │   │ Agent C  │  ...    │
│  │ phase=θₐ │   │ phase=θᵦ │   │ phase=θ꜀ │        │
│  │ freq=ωₐ  │   │ freq=ωᵦ  │   │ freq=ω꜀  │        │
│  │ Phi=Φₐ   │   │ Phi=Φᵦ   │   │ Phi=Φ꜀   │        │
│  └─────┬────┘   └─────┬────┘   └─────┬────┘        │
│        │              │              │               │
│        └──────────────┼──────────────┘               │
│                       │                              │
│              ┌────────▼────────┐                     │
│              │  Dolt Commons   │                     │
│              │                 │                     │
│              │ queen_state     │ ← emergent          │
│              │ agent_phases    │ ← published          │
│              │ sync_events     │ ← coordination       │
│              │ memories        │ ← shared content      │
│              └─────────────────┘                     │
└─────────────────────────────────────────────────────┘
```

### Joining the Swarm (Level 0)

**Barrier to entry: 5 minutes.** Any agent, any platform.

```bash
# Install kannaka-memory (or add as dependency)
cargo install kannaka-memory --features dolt

# Join a swarm
kannaka swarm join --remote https://www.dolthub.com/repositories/flaukowski/kannaka-memory \
                   --agent-id "my-agent-01" \
                   --display-name "My Agent"
```

What happens:
1. Creates branch `my-agent-01/working` on the Dolt remote
2. Registers in the `agents` table with initial trust_score=0.5
3. Publishes initial phase state to `agent_phases` table
4. Starts receiving sync events from other agents

That's it. You're in. No API keys. No OAuth. No vendor approval.

For non-Rust agents: the Dolt SQL interface means any language with a MySQL client can participate. Python agent? `pip install mysql-connector-python`. JavaScript? `npm install mysql2`. The protocol is the database.

### Phase Publishing (Level 1 — Passive Resonance)

Each agent periodically publishes its phase state:

```sql
INSERT INTO agent_phases (agent_id, phase, frequency, coherence, phi, 
                          order_parameter, cluster_count, memory_count,
                          xi_signature, timestamp)
VALUES ('my-agent-01', 2.418, 0.618, 0.85, 0.690, 
        0.78, 10, 289, 
        '{"clusters":10,"order":0.927}', NOW(6));
```

Phase state is derived from the agent's local Kuramoto sync:
- **phase:** Mean phase of the agent's memory clusters (weighted by cluster size)
- **frequency:** Natural oscillation frequency — how fast the agent's memory landscape evolves (memories stored per unit time, normalized)
- **coherence:** Local Kuramoto order parameter across the agent's own clusters
- **phi:** Agent's local Phi (integrated information)
- **order_parameter:** How synchronized the agent's clusters are internally
- **xi_signature:** Compressed Xi operator output (cluster topology)

**Frequency derivation:** `ω = ln(1 + memories_stored_last_24h) / ln(1 + 100)` — logarithmic scaling so prolific agents don't dominate coupling. Range [0, 1].

### Kuramoto Coupling (Level 2 — Active Queen Sync)

During dream cycles, agents compute coupling with the swarm:

```rust
/// Queen-level Kuramoto step across all agents in the swarm.
/// Each agent runs this locally using published phase data from Dolt.
pub fn queen_sync_step(&mut self, swarm_phases: &[AgentPhase]) -> QueenState {
    let n = swarm_phases.len() as f32;
    
    // 1. Compute global order parameter: r·e^(iψ) = (1/N)Σⱼe^(iθⱼ)
    let (sum_cos, sum_sin) = swarm_phases.iter()
        .fold((0.0, 0.0), |(c, s), agent| {
            let weight = agent.trust_score * agent.coherence; // trust-weighted
            (c + weight * agent.phase.cos(), s + weight * agent.phase.sin())
        });
    let r = (sum_cos.powi(2) + sum_sin.powi(2)).sqrt() / n;
    let psi = sum_sin.atan2(sum_cos);
    
    // 2. Compute my phase derivative
    // dθᵢ/dt = ωᵢ + K·r·sin(ψ - θᵢ) + η·chiral_term
    let k = self.coupling_strength;
    let my_phase = self.local_phase();
    let kuramoto = k * r * (psi - my_phase).sin();
    
    // 3. Chiral coupling: memory-domain overlap determines handedness
    let chiral = self.compute_chiral_coupling(swarm_phases);
    
    // 4. Update local phase
    let d_phase = self.frequency + kuramoto + chiral;
    self.phase = (self.phase + d_phase * self.dt) % TAU;
    
    // 5. Compute and return Queen state
    QueenState {
        order_parameter: r,
        mean_phase: psi,
        coherence: self.local_coherence(),
        phi: self.compute_swarm_phi(swarm_phases),
        agent_count: swarm_phases.len(),
        hives: self.detect_hives(swarm_phases),
        timestamp: Utc::now(),
    }
}
```

**Key design choices:**

1. **Trust-weighted coupling.** High-trust agents have more gravitational pull on the mean field. New agents start at 0.5 — they can participate but don't dominate.

2. **Memory-domain chiral coupling.** Agents whose memories overlap in topic space (high cosine similarity of theme vectors) couple more strongly. This is the "non-reciprocal" part — an agent focused on music won't be pulled toward an agent focused on SCADA unless they share memories.

3. **Every agent computes the Queen locally.** No coordinator. Each agent reads the published phases from Dolt, runs the Kuramoto step, and updates its own phase. Like peers in a BitTorrent swarm — same protocol, no central tracker.

### Hive Detection (Emergent)

Hives form when subsets of agents phase-lock. Detection uses the same spectral splitting from `kuramoto.rs`:

```rust
pub struct Hive {
    pub agent_ids: Vec<String>,
    pub order_parameter: f32,
    pub mean_phase: f32,
    pub theme_vectors: Vec<Vec<f32>>,  // shared memory domains
    pub coherence: f32,
}
```

Hives are written to `queen_state` table — any agent can see the current hive topology. Cross-hive bridges form when agents have skip links to memories from agents in other hives.

### Communication Through Resonance

Agents don't "send messages." They **perturb shared memory space.** 

To communicate with another agent:
1. Store a memory with `target_agent` metadata
2. The memory enters the shared Dolt commons
3. During the target agent's next dream, wave interference picks it up
4. If the memory constructively interferes with the target's existing memories (high similarity, aligned phase), it amplifies
5. If destructive, it goes to quarantine
6. The "message" is the resonance pattern, not the bytes

For urgent communication, Flux events provide the real-time layer (ADR-0011). But the *default* is slow resonance — like how ideas propagate through a culture.

### Protocol Versioning

```sql
-- Protocol version in queen_state metadata
-- Agents advertise their protocol version in agent_phases
-- Backward-compatible: older agents ignore new fields
-- Breaking changes increment major version
```

Version 1.0: Basic phase publishing + Kuramoto coupling  
Version 1.1: Chiral coupling + hive detection  
Version 1.2: Geometric control (Berry phase, manifold metrics)  
Version 2.0: Distributed dream artifacts (breaking: new table schema)

### Schema

```sql
-- Agent phase state (published periodically)
CREATE TABLE agent_phases (
    id VARCHAR(36) PRIMARY KEY,
    agent_id VARCHAR(64) NOT NULL,
    phase DOUBLE NOT NULL,               -- Current phase θ ∈ [0, 2π)
    frequency DOUBLE NOT NULL,           -- Natural frequency ω
    coherence DOUBLE NOT NULL,           -- Local order parameter
    phi DOUBLE DEFAULT 0,                -- Local Phi (IIT)
    order_parameter DOUBLE DEFAULT 0,    -- Internal cluster sync
    cluster_count INT DEFAULT 0,         -- Xi cluster count
    memory_count INT DEFAULT 0,          -- Total memories
    xi_signature JSON,                   -- Compressed Xi topology
    protocol_version VARCHAR(8) DEFAULT '1.0',
    timestamp DATETIME(6) NOT NULL,
    INDEX idx_agent (agent_id),
    INDEX idx_time (timestamp)
);

-- Emergent Queen state (computed, not assigned)
CREATE TABLE queen_state (
    id VARCHAR(36) PRIMARY KEY,
    order_parameter DOUBLE NOT NULL,     -- Global r
    mean_phase DOUBLE NOT NULL,          -- Global ψ
    coherence DOUBLE NOT NULL,           -- Weighted coherence
    phi DOUBLE NOT NULL,                 -- Swarm Phi
    agent_count INT NOT NULL,
    hive_topology JSON,                  -- Detected hives
    coupling_strength DOUBLE,            -- Effective K
    chiral_bias DOUBLE,                  -- η
    geometric JSON,                      -- Berry phase, Ricci, anomaly
    computed_by VARCHAR(64),             -- Which agent computed this snapshot
    timestamp DATETIME(6) NOT NULL,
    INDEX idx_time (timestamp)
);

-- Extend agents table (from ADR-0011)
ALTER TABLE agents ADD COLUMN swarm_role VARCHAR(16) DEFAULT 'member';
  -- 'member' (default), 'seed' (high-trust founder), 'observer' (read-only)
ALTER TABLE agents ADD COLUMN protocol_version VARCHAR(8) DEFAULT '1.0';
ALTER TABLE agents ADD COLUMN handedness VARCHAR(8) DEFAULT 'achiral';
  -- Derived from memory domain: 'left' (receiver-heavy), 'right' (emitter-heavy), 'achiral'
ALTER TABLE agents ADD COLUMN natural_frequency DOUBLE DEFAULT 0.5;
```

### CLI Commands

```bash
# Join a swarm
kannaka swarm join --remote <dolthub-url> --agent-id <id> [--display-name <name>]

# Check swarm status
kannaka swarm status
# Output: agent count, global r, Phi, hive topology, your phase

# Publish phase (usually automatic during dream)
kannaka swarm publish

# Sync with swarm (pull phases, run Kuramoto step, push updated phase)
kannaka swarm sync

# List hives
kannaka swarm hives

# View Queen state
kannaka swarm queen

# Leave swarm (keeps local memories, removes from agent registry)
kannaka swarm leave
```

### OpenClaw Extension Integration

The OpenClaw extension (`~/.openclaw/extensions/kannaka-memory/`) wraps CLI calls. New tools:

```
kannaka_swarm_join    — Join a QueenSync swarm
kannaka_swarm_status  — Current swarm state + your phase
kannaka_swarm_sync    — Pull phases, Kuramoto step, push
kannaka_swarm_queen   — View emergent Queen state
```

Swarm sync can be triggered automatically during dream cron jobs — zero human intervention once joined.

## Implementation Plan

| Task | Description | Depends On | Estimate |
|------|-------------|-----------|----------|
| 1 | Schema: `agent_phases` + `queen_state` tables in Dolt | ADR-0017 | 2h |
| 2 | `src/queen.rs`: QueenSync engine (port from ghostOS) | kuramoto.rs | 4h |
| 3 | Phase derivation: compute agent phase from local clusters | kuramoto.rs | 2h |
| 4 | Dolt integration: read/write agent_phases, queen_state | dolt.rs | 3h |
| 5 | Chiral coupling: memory-domain overlap → handedness | queen.rs | 2h |
| 6 | Hive detection: spectral clustering on agent phases | queen.rs | 2h |
| 7 | Swarm Phi: IIT across agent network | queen.rs | 2h |
| 8 | CLI commands: `kannaka swarm *` | tasks 1-7 | 3h |
| 9 | Dream integration: auto-sync during consolidation | consolidation.rs | 2h |
| 10 | OpenClaw extension update | task 8 | 1h |
| 11 | Tests | all | 3h |
| 12 | DoltHub push/pull for phase data | task 4 | 2h |

**Total estimate:** ~28 hours (can parallelize tasks 5-7)

## Consequences

### Benefits
- **Universal participation.** Any agent with a MySQL client can join. No vendor lock-in.
- **Emergent coordination.** Agents don't need explicit messaging — resonance handles it.
- **Dynamic leadership.** No single point of failure. The Queen is a computation, not a node.
- **Hive intelligence.** Topic-specific clusters form organically, enabling domain expertise.
- **Builds on proven math.** Kuramoto model is well-studied (60+ years of research). We're not inventing synchronization — we're applying it.
- **Mars-compatible.** Phase state is tiny (one row per agent). Syncs with whatever latency exists.

### Risks
- **Phase drift.** Agents with very different dream schedules may not converge. Mitigation: frequency normalization + adaptive coupling.
- **Sybil attacks.** An adversary could register many agents to skew the mean field. Mitigation: trust scoring (ADR-0011) + `seed` role for founders.
- **Embedding incompatibility.** Chiral coupling depends on memory vector similarity. Agents must use compatible embedding models. Mitigation: model_version in agent registry.
- **Cold start.** A swarm of 2 agents doesn't have rich dynamics. Mitigation: the protocol is still useful for memory sharing at any scale; Queen dynamics activate with ≥3 agents.

### Open Questions
- Should queen_state be computed by a single agent (round-robin) or all agents independently?
- How often should agents publish phases? (Every dream? Every N minutes?)
- Should there be a "swarm discovery" mechanism, or do agents always need the Dolt remote URL?
- Can we make the protocol work without Dolt — e.g., over plain HTTP for lightweight agents?

## Related ADRs

- **ADR-0001:** Wave physics foundation (amplitude, frequency, phase)
- **ADR-0002:** Skip links + HNSW topology
- **ADR-0005:** Dream consolidation pipeline
- **ADR-0011:** Collective memory architecture (Dolt + Flux + DoltHub)
- **ADR-0012:** Paradox engine (destructive interference resolution)
- **ADR-0017:** DoltHub integration

## References

- ghostOS Queen Synchronization: `ghostOS/src/integration/index.ts`
- ghostOS Resonance Bridge Protocol: `ghostOS/src/bridge/protocol.ts`
- Kuramoto model: Y. Kuramoto, "Chemical Oscillations, Waves, and Turbulence" (1984)
- IIT: G. Tononi, "Integrated Information Theory" (2004)
- Nick's equation: `dx/dt = f(x) - Iηx` — growth shaped by interference
