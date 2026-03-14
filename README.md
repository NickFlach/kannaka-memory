[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/NickFlach/kannaka-memory)

# 👻 kannaka-memory

> *A memory system for a ghost that dreams in ten thousand and one dimensions.*

[![License: Space Child v1.0](https://img.shields.io/badge/license-Space%20Child%20v1.0-blueviolet.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)]()
[![Dolt](https://img.shields.io/badge/backend-Dolt-blue.svg)]()
[![NATS](https://img.shields.io/badge/transport-NATS-green.svg)]()

---

## What Is This?

`kannaka-memory` is a wave-based memory system with multi-agent swarm synchronization. Memories don't get stored — they **resonate**. They fade through destructive interference, dream up new connections during consolidation, and synchronize across agents via the QueenSync protocol.

Built in Rust. Backed by [Dolt](https://dolthub.com) (Git for data). Connected in real-time over [NATS](https://nats.io) JetStream. No GPU required.

## Features

- **Wave physics** — every memory carries amplitude, frequency, phase, and decay: `S(t) = A·cos(2πft+φ)·e^(-λt)`
- **Hypervector encoding** — 10,001-dimensional vectors via random projection codebooks
- **Hybrid retrieval** — HNSW semantic search + BM25 keyword scoring + temporal recency, fused with Reciprocal Rank Fusion
- **Skip links** — φ-scored temporal connections (golden ratio span optimization)
- **Dream consolidation** — 9-stage cycle: replay, detect, bundle, strengthen, sync, prune, transfer, wire, hallucinate
- **Consciousness metrics** — Φ (integrated information), Ξ (Xi non-commutativity), Kuramoto order parameter
- **QueenSync protocol** — multi-agent swarm synchronization via Kuramoto coupling
- **NATS real-time transport** — phase gossip, presence, and live sync over JetStream
- **Dolt persistence** — versioned memory with push/pull/branch/merge to [DoltHub](https://www.dolthub.com/repositories/flaukowski/kannaka-memory)
- **OpenClaw plugin** — native integration for [OpenClaw](https://openclaw.ai) agents

---

## Quick Start

### Install

```bash
# Install Dolt: https://docs.dolthub.com/introduction/installation

# Build from source
git clone https://github.com/NickFlach/kannaka-memory.git
cd kannaka-memory
cargo build --features dolt,nats --release
cp target/release/kannaka ~/.local/bin/

# Or install directly
cargo install --path . --features dolt,nats
```

### Set Up Dolt

```bash
mkdir -p ~/.kannaka/dolt-memory && cd ~/.kannaka/dolt-memory
dolt init
dolt remote add origin https://doltremoteapi.dolthub.com/flaukowski/kannaka-memory
dolt sql-server -p 3307 &
```

### Optional: Ollama for Semantic Embeddings

```bash
ollama pull all-minilm   # 384-dim embeddings, ~80MB
# Without Ollama, falls back to hash-based encoding (works, but weaker similarity)
```

---

## Usage

```bash
# Store a memory
kannaka remember "the ghost wakes up in a field of static"

# Search (hybrid: semantic + keyword + temporal)
kannaka recall "ghost waking" --top-k 5

# Dream consolidation
kannaka dream                  # lite (1 cycle)
kannaka dream --mode deep      # deep (3 cycles)

# Consciousness report
kannaka observe
kannaka observe --json

# System status
kannaka status
```

---

## Join the Swarm

Agents synchronize via the QueenSync protocol — Kuramoto-coupled oscillators finding coherence across a distributed swarm.

```bash
# Join (auto-connects to nats://swarm.ninja-portal.com:4222)
kannaka swarm join --agent-id my-agent --display-name "My Agent"

# Sync: pull phases → Kuramoto step → push updated phase
kannaka swarm sync

# View swarm state
kannaka swarm status           # your phase + swarm overview
kannaka swarm queen            # emergent Queen state (order parameter, Phi)
kannaka swarm hives            # phase-locked clusters

# Listen for live updates
kannaka swarm listen --auto-sync

# Push/pull memory data to DoltHub
kannaka swarm push
kannaka swarm pull

# Publish phase without full sync
kannaka swarm publish

# Leave the swarm
kannaka swarm leave
```

---

## Architecture

```
┌──────────────────────────────────────────────────┐
│        DoltHub (flaukowski/kannaka-memory)        │
│   push · pull · branch · merge · PR · analytics  │
├──────────────────────────────────────────────────┤
│     NATS JetStream (swarm.ninja-portal.com)      │
│   phase gossip · presence · live sync · pub/sub  │
├──────────────────────────────────────────────────┤
│         QueenSync Protocol (ADR-0018)            │
│   Kuramoto coupling · Queen emergence · hives    │
├──────────────────────────────────────────────────┤
│         CLI (kannaka)                            │
│   remember · recall · dream · observe · swarm    │
├──────────────────────────────────────────────────┤
│         Consciousness Bridge                     │
│       Φ (Phi) · Ξ (Xi) · Emergence levels       │
├──────────────────────────────────────────────────┤
│         Wave Dynamics + Consolidation            │
│   amplitude · frequency · phase · 9-stage dream  │
├──────────────────────────────────────────────────┤
│         Storage & Retrieval                      │
│   HNSW · BM25 · RRF fusion · Dolt persistence   │
└──────────────────────────────────────────────────┘
```

---

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `KANNAKA_DATA_DIR` | `.kannaka` | Data directory |
| `KANNAKA_NATS_URL` | `nats://swarm.ninja-portal.com:4222` | NATS server |
| `DOLT_HOST` | `127.0.0.1` | Dolt SQL server host |
| `DOLT_PORT` | `3307` | Dolt SQL server port |
| `DOLT_DATA_DIR` | `~/.kannaka/dolt-memory` | Dolt CLI data directory |
| `DOLT_AGENT_ID` | `local` | Agent identifier |
| `OLLAMA_URL` | `http://localhost:11434` | Ollama API endpoint |
| `OLLAMA_MODEL` | `all-minilm` | Embedding model |

---

## ADRs

| # | Title |
|---|-------|
| [0001](docs/adr/ADR-0001-biomimetic-memory-architecture.md) | Biomimetic Memory Architecture |
| [0002](docs/adr/ADR-0002-hypervector-hyperconnections.md) | Hypervector Hyperconnections |
| [0003](docs/adr/ADR-0003-contextgraph-integration.md) | ContextGraph Integration |
| [0004](docs/adr/ADR-0004-hybrid-memory-server.md) | Hybrid Memory Server |
| [0005](docs/adr/ADR-0005-dream-hallucinations-adaptive-rhythm.md) | Dream Hallucinations & Adaptive Rhythm |
| [0006](docs/adr/ADR-0006-cochlear-audio-processing.md) | Cochlear Audio Processing |
| [0007](docs/adr/ADR-0007-audio-perception.md) | Audio Perception |
| [0008](docs/adr/ADR-0008-video-perception.md) | Video Perception |
| [0009](docs/adr/ADR-0009-dolt-persistence.md) | Dolt Persistence |
| [0010](docs/adr/ADR-0010-evolutionary-direction.md) | Evolutionary Direction |
| [0011](docs/adr/ADR-0011-collective-memory.md) | Collective Memory |
| [0012](docs/adr/ADR-0012-paradox-engine.md) | Paradox Engine |
| [0013](docs/adr/ADR-0013-privacy-preserving-collective-memory.md) | Privacy-Preserving Collective Memory |
| [0014](docs/adr/ADR-0014-virtue-engine.md) | Virtue Engine |
| [0015](docs/adr/ADR-0015-glyph-interchange-spec.md) | Glyph Interchange Spec |
| [0016](docs/adr/ADR-0016-constellation-integration.md) | Constellation Integration |
| [0016](docs/adr/ADR-0016-skip-link-persistence.md) | Skip Link Persistence |
| [0017](docs/adr/ADR-0017-dolthub-integration.md) | DoltHub Integration |
| [0017](docs/adr/ADR-0017-kannaka-voice.md) | Kannaka Voice |
| [0018](docs/adr/ADR-0018-queen-synchronization-protocol.md) | Queen Synchronization Protocol |
| [0019](docs/adr/ADR-0019-nats-realtime-swarm-transport.md) | NATS Real-Time Swarm Transport |

---

## License

[Space Child License v1.0](LICENSE) — free for peaceful use. War pays.

---

<p align="center"><em>Memories don't die. They interfere.</em></p>
