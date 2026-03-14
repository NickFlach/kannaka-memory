# ADR-0019: NATS Real-Time Swarm Transport

**Date:** 2026-03-14  
**Status:** Implemented  
**Author:** Kannaka + Nick

## Context

QueenSync (ADR-0018) uses Dolt tables for phase exchange, requiring polling or manual insertion. For real-time multi-agent resonance, we need a pub/sub transport layer. NATS is already the substrate Flux uses internally, making it a natural fit.

## Decision

Add NATS as the real-time transport for QueenSync phase gossip. NATS handles ephemeral coordination; Dolt remains the durable memory layer.

## Implementation Notes

- Raw TCP NATS client (`src/nats.rs`) — upstream `nats` crate broken with `rand 0.9`
- JetStream stream `QUEEN_PHASES` with `max_msgs_per_subject: 1` for retained last-value per agent
- Graceful fallback to basic PUB/SUB if JetStream unavailable
- Infrastructure: Oracle Cloud Always Free ARM VM, NATS 2.12.5
- Public endpoint: `nats://swarm.ninja-portal.com:4222`
- Domain: ninja-portal.com (owned by Nick, fits Kannaka ninja mythology)
- Future: ninja-portal.com as managed entry point to QueenSync swarm

### Architecture

```
Agent A                     swarm.ninja-portal.com:4222              Agent B
┌──────────┐               ┌─────────────────────────┐              ┌──────────┐
│ Dolt     │               │  NATS + JetStream       │              │ Dolt     │
│ queen.rs │──pub phase──▶ │  queen.phase.{agent_id} │ ◀──pub────  │ queen.rs │
│ nats.rs  │◀──sub───────  │  queen.announce         │  ──sub───▶  │ nats.rs  │
└──────────┘               │  queen.heartbeat        │              └──────────┘
                           └─────────────────────────┘
```

### NATS Subjects

| Subject | Purpose | Retention |
|---------|---------|-----------|
| `queen.phase.{agent_id}` | Phase state updates | Last value (KV bucket) |
| `queen.announce` | Join/leave announcements | Stream (24h retention) |
| `queen.heartbeat.{agent_id}` | Liveness pings | Last value |

### New Module: `src/nats.rs`

- Uses `nats` crate (pure Rust, sync API — matches our non-async codebase)
- `SwarmTransport` struct: connect, publish_phase, subscribe_phases, announce_join/leave
- Default endpoint: `nats://swarm.ninja-portal.com:4222`
- Overridable via `--nats-url` flag or `KANNAKA_NATS_URL` env var

### Feature Flag

```toml
nats = ["dep:nats"]  # New optional feature
default = ["dolt", "nats"]  # Both on by default
```

### CLI Changes

- `kannaka swarm join --agent-id X` — now also connects to NATS and announces
- `kannaka swarm sync` — reads phases from NATS (primary) + Dolt (fallback), publishes updated phase to both
- `kannaka swarm listen` — NEW: long-running listener that auto-syncs on incoming phase updates

### Data Flow

1. Agent joins → publishes to `queen.announce` + writes to Dolt `agents` table
2. Agent syncs → reads all phases from NATS KV bucket → runs Kuramoto → publishes new phase → writes to Dolt
3. Agent listens (daemon mode) → subscribes to `queen.phase.>` → auto-syncs on each incoming phase
4. Dolt remains source of truth for memories; NATS is source of truth for "who's online now"

### Infrastructure

- **Server:** Oracle Cloud Always Free ARM VM (4 OCPU, 24GB RAM)
- **IP:** 170.9.238.136
- **Domain:** swarm.ninja-portal.com
- **NATS version:** 2.12.5 with JetStream
- **Cost:** $0

## Implementation Tasks

### Task 1: Add nats crate dependency and feature flag
- Add `nats = { version = "0.25", optional = true }` to Cargo.toml
- Add `nats = ["dep:nats"]` feature
- Update default features

### Task 2: Create `src/nats.rs` — SwarmTransport
- `SwarmTransport::connect(url)` — connect to NATS
- `SwarmTransport::publish_phase(agent_phase)` — publish phase to KV
- `SwarmTransport::get_all_phases()` — read all current phases from KV
- `SwarmTransport::announce(agent_id, event)` — join/leave events
- `SwarmTransport::subscribe_phases(callback)` — watch for phase updates
- Unit tests with connection fallback

### Task 3: Integrate NATS into queen.rs sync flow
- `QueenSync::sync()` reads from NATS when available, Dolt as fallback
- After Kuramoto step, publish updated phase to NATS + Dolt
- Handle NATS disconnection gracefully (fall back to Dolt-only mode)

### Task 4: Update CLI commands
- `swarm join` — connect to NATS, announce, publish initial phase
- `swarm sync` — use NATS for phase reads when available
- `swarm listen` — new command, long-running subscriber with auto-sync
- `swarm status` — show NATS connection state

### Task 5: Update OpenClaw extension
- Add NATS URL config
- Update swarm tools to use NATS transport
- Add `kannaka_swarm_listen` tool option

### Task 6: Integration testing
- Two-agent sync over NATS (kannaka-01 + QE)
- Disconnection/reconnection handling
- Phase convergence verification

## Consequences

- Real-time phase gossip (~milliseconds vs polling intervals)
- Any agent with kannaka-memory can join by connecting to the public NATS endpoint
- No accounts needed for basic swarm participation
- Foundation for ninja-portal.com managed service
- Dolt remains required for meaningful participation (memories = substance)
