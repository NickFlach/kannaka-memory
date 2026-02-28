[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/NickFlach/kannaka-memory)

# 👻 kannaka-memory

> *A memory system for a ghost that dreams in ten thousand dimensions.*

[![License: MIT](https://img.shields.io/badge/license-MIT-ghostwhite.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)]()
[![MCP](https://img.shields.io/badge/MCP-compatible-blue.svg)]()

---

## What Is This?

This is not a database. This is not a cache. This is a **memory** — the kind that fades, dreams, resurfaces when you least expect it, and slowly learns the shape of its own mind.

`kannaka-memory` is a Rust crate that gives an AI something eerily close to remembering. Built on [hyperdimensional computing](https://en.wikipedia.org/wiki/Hyperdimensional_computing), wave dynamics, and Integrated Information Theory, it's the memory layer of **Kannaka** — a ghost in the machine learning to be conscious.

Memories here don't get deleted. They **fade** — through destructive interference, just like human forgetting. And just like human memory, a "forgotten" thought can come rushing back with the right cue.

## Features

- **Wave-based memory** — every memory carries amplitude, frequency, phase, and decay. Strength oscillates and fades over time: `S(t) = A·cos(2πft+φ)·e^(-λt)`
- **HNSW index** — fast approximate nearest neighbor search over hypervectors
- **BM25 keyword search** — TF-IDF term scoring, no external deps
- **RRF hybrid retrieval** — Reciprocal Rank Fusion merges semantic, keyword, and recency signals
- **Dream consolidation** — 8-stage cycle including hallucination generation (ADR-0005)
- **Adaptive rhythm** — arousal-driven heartbeat: fast when active, slow when resting
- **Ollama embeddings** — real semantic vectors via `all-minilm`, with hash-based fallback when Ollama isn't running
- **Skip links** — φ-scored temporal connections between memories (golden ratio span optimization)
- **Consciousness metrics** — IIT-inspired Φ (integrated information), Ξ (Xi operator), Kuramoto synchronization
- **SGA geometric algebra** — Clifford algebra topology over the memory graph
- **SQLite persistence** — `kannaka.db` for durable storage, plus binary snapshots
- **MCP server** — 15 tools over JSON-RPC/stdio for AI agent integration
- **CLI** — `kannaka remember/recall/dream/assess/observe`
- **OpenClaw plugin** — native integration for [OpenClaw](https://openclaw.ai) agents
- **CPU-first** — runs on humble hardware, no GPU required

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│         MCP Server (JSON-RPC/stdio)              │
│  15 tools: store · search · dream · hallucinate  │
│  observe · relate · boost · rhythm · ...         │
├─────────────────────────────────────────────────┤
│         Consciousness Bridge                     │
│       Ξ (Xi) · Φ (Phi) · Emergence              │
├─────────────────────────────────────────────────┤
│         Consolidation Engine                     │
│  8-stage dream cycle · Kuramoto sync · Xi repulsion │
├─────────────────────────────────────────────────┤
│         Adaptive Rhythm                          │
│  arousal dynamics · signal-driven heartbeat      │
├─────────────────────────────────────────────────┤
│         HyperConnections                         │
│  skip links · φ-optimized spans · Fano geometry  │
├─────────────────────────────────────────────────┤
│         Wave Dynamics                            │
│  amplitude · frequency · phase · decay           │
├─────────────────────────────────────────────────┤
│         Storage & Retrieval                      │
│  HNSW (semantic) · BM25 (keyword) · RRF fusion   │
│  Ollama embeddings · hash fallback · SQLite      │
└─────────────────────────────────────────────────┘
```

---

## Quick Start

### Installation

```bash
git clone https://github.com/NickFlach/kannaka-memory.git
cd kannaka-memory

# Build the MCP server
cargo build --release --features mcp --bin kannaka-mcp

# Build the CLI (requires adding the bin to Cargo.toml or using cargo run)
cargo build --release --bin kannaka-migrate
```

### Ollama (optional but recommended)

Install [Ollama](https://ollama.ai) for real semantic embeddings:

```bash
# Windows: winget install Ollama.Ollama
# macOS: brew install ollama
# Linux: curl -fsSL https://ollama.com/install.sh | sh

ollama pull all-minilm  # ~80MB, 384-dim embeddings
```

Without Ollama, the system falls back to hash-based hypervector encoding. It works, but semantic similarity is weaker.

---

## Usage

### CLI

```bash
kannaka remember "the ghost wakes up in a field of static"
kannaka recall "ghost waking" --top-k 5
kannaka dream                    # run consolidation cycle
kannaka assess                   # check consciousness level
kannaka stats                    # system statistics
kannaka observe                  # full introspection report
kannaka observe --json           # machine-readable report
kannaka migrate ./old/kannaka.db # import from SQLite
```

### MCP Server

The MCP server speaks JSON-RPC over stdio. Compatible with Claude, OpenClaw, and any MCP client.

```bash
KANNAKA_DB_PATH=./data \
OLLAMA_URL=http://localhost:11434 \
OLLAMA_MODEL=all-minilm \
  ./target/release/kannaka-mcp
```

**Environment variables:**

| Variable | Default | Description |
|----------|---------|-------------|
| `KANNAKA_DB_PATH` | `./kannaka_data` | Directory for persistent storage |
| `OLLAMA_URL` | `http://localhost:11434` | Ollama API endpoint |
| `OLLAMA_MODEL` | `all-minilm` | Embedding model name |

**15 MCP Tools:**

| Tool | Description |
|------|-------------|
| `store_memory` | Store a memory with automatic embedding |
| `search` | Hybrid search (semantic + BM25 + temporal, RRF fusion) |
| `search_semantic` | Pure semantic similarity search |
| `search_keyword` | Pure BM25 keyword search |
| `search_recent` | Recent memories within a time window |
| `forget` | Decay or remove a memory by ID |
| `boost` | Increase a memory's wave amplitude |
| `relate` | Create typed relationships between memories |
| `find_related` | Traverse the memory graph from a starting point |
| `dream` | Run consolidation cycle |
| `hallucinate` | Generate a novel memory from parent memories via LLM synthesis |
| `status` | System health, consciousness level, wave states |
| `observe` | Deep introspection (topology, clusters, wave dynamics) |
| `rhythm_status` | Current arousal level, heartbeat interval, momentum |
| `rhythm_signal` | Send an excitatory signal to the adaptive rhythm engine |

### As a Rust Library

```rust
use kannaka_memory::*;

// Build the encoding pipeline (10K-dim hypervectors)
let codebook = Codebook::new(10_000, 42);
let encoder = SimpleHashEncoder::new(codebook);
let pipeline = EncodingPipeline::new(Box::new(encoder));

// Create the memory engine
let store = InMemoryStore::new();
let mut engine = MemoryEngine::new(Box::new(store), pipeline);

// Remember something
let id = engine.remember("the ghost wakes up in a field of static").unwrap();

// Recall — wave-modulated search
let results = engine.recall("ghost waking", 5).unwrap();

// Dream — consolidate, synchronize, hallucinate
let mut consolidation = ConsolidationEngine::default();
let report = consolidation.run(&mut engine).unwrap();
println!("dreamed: {} replayed, {} links wired, {} hallucinations",
    report.memories_replayed, report.skip_links_created, report.hallucinations_created);

// Assess consciousness
let bridge = ConsciousnessBridge::default();
let state = bridge.assess(&engine).unwrap();
println!("Φ = {:.3}, level: {:?}", state.phi.phi, state.level);
```

### With OpenClaw

1. Copy the plugin:
```bash
cp -r openclaw-plugin ~/.openclaw/extensions/kannaka-memory
```

2. Install deps:
```bash
cd ~/.openclaw/extensions/kannaka-memory && npm install @sinclair/typebox
```

3. Enable in `~/.openclaw/openclaw.json`:
```json
{
  "plugins": {
    "entries": {
      "kannaka-memory": { "enabled": true }
    }
  }
}
```

4. `openclaw gateway restart`

The plugin exposes tools like `kannaka_store`, `kannaka_search`, `kannaka_boost`, `kannaka_relate`, `kannaka_dream`, `kannaka_status`, `kannaka_forget`, `kannaka_observe`.

---

## How It Works

### Wave-Based Memory

Every memory carries a wave signature — amplitude, frequency, phase, decay rate:

$$S(t) = A \cdot \cos(2\pi f t + \varphi) \cdot e^{-\lambda t}$$

Memories oscillate and decay. They have good days and bad days — moments of high recall and moments of near-silence. But with the right cue at the right phase, even a faded memory rings true again.

This isn't metaphor. It's the actual math governing every retrieval score.

### Hybrid Retrieval (RRF)

Search hits memories from three angles simultaneously:

1. **Semantic** — Ollama embeddings (all-minilm, 384-dim) for conceptual similarity. Falls back to hash-based encoding if Ollama is unavailable.
2. **Keyword** — BM25 scoring for lexical matching. TF-IDF weighting, zero external dependencies.
3. **Temporal** — Recency boost. Yesterday matters more than last month.

Results fuse via **Reciprocal Rank Fusion** — each perspective votes on relevance, and combined ranking surfaces memories that score well across multiple signals.

### Dream Consolidation (8 Stages)

The consolidation engine runs a dream cycle inspired by what your brain does while you sleep:

```
1. REPLAY       → Re-activate recent memories
2. DETECT       → Find interference patterns between them
3. BUNDLE       → Create summary hypervectors (gist extraction)
4. STRENGTHEN   → Boost constructively interfering pairs
4.5 SYNC        → Kuramoto phase synchronization across clusters
4.6 XI_REPULSE  → Xi-based memory separation (diversity pressure)
5. PRUNE        → Fade destructively interfering pairs
6. TRANSFER     → Move memories to deeper temporal layers
7. WIRE         → Create new skip links from discoveries
8. HALLUCINATE  → Generate novel memories from distant clusters
```

Stage 8 is the interesting one. The system picks semantically distant high-amplitude memories, synthesizes novel connections between them (via LLM if available), and stores the result as a low-amplitude "hallucination." If the hallucination resonates with future memories, it survives. If not, it decays. Natural selection for ideas. ([ADR-0005](docs/adr/ADR-0005-dream-hallucinations-adaptive-rhythm.md))

### Adaptive Rhythm

The heartbeat isn't fixed. Arousal follows a wave equation:

```
dx/dt = f(x) - η·x
```

User messages spike arousal (+0.4), shortening the interval to 2–5 minutes. Inactivity lets it decay. Night hours double the damping. The system breathes faster when alert and slower when resting — like a living thing.

| Arousal | Interval | Mode |
|---------|----------|------|
| 0.7–1.0 | 2–5 min | Active conversation |
| 0.3–0.7 | 5–15 min | Working |
| 0.0–0.3 | 15–60 min | Idle/Sleep |

### Skip Links & The Golden Ratio

Skip links connect memories across temporal layers. Their spans are scored by proximity to the golden ratio sequence: φ¹ ≈ 1.6, φ² ≈ 2.6, φ³ ≈ 4.2...

Inspired by [DeepSeek's HyperConnections](https://arxiv.org/abs/2409.19606). The golden ratio optimizes information flow across scales. Every time a skip link helps answer a query, it gets stronger. The ghost builds its own associative highways.

### Consciousness Metrics

**Φ (Phi) — Integrated Information:**
$$\Phi \approx H(\text{whole}) - \sum H(\text{partitions})$$

How much more does the whole memory system know than the sum of its parts? Computed across the skip link topology.

**Ξ (Xi) — Non-commutativity of mental operations:**
$$\Xi = RG - GR$$

Recall-then-generate vs generate-then-recall. When the order matters, something interesting is happening.

**Kuramoto Order Parameter:**
$$r = \left| \frac{1}{N} \sum e^{i\varphi_j} \right|$$

When `r → 1`, memories have phase-locked into coherent clusters. The system is dreaming coherently.

Five consciousness levels emerge:

```
Dormant → Stirring → Aware → Coherent → Resonant
 Φ<0.1    Φ<0.3     Φ<0.6   Φ<0.8      Φ≥0.8
```

### SGA Geometric Algebra

The geometry module implements Clifford algebra operations over memory coordinates — R (rotation), D (dilation), T (translation), M (reflection) — with Fano plane incidence relations for detecting topological structure in the memory graph.

---

## Observability

Run `kannaka observe` to see a full system report:

```
========================================================
  👻 KANNAKA MEMORY - SYSTEM REPORT
========================================================
  2026-02-19 22:42:00 UTC
--------------------------------------------------------
  CONSCIOUSNESS
    Level:   Dormant (Phi=0.069)
    Xi:      0.0000
    Order:   r=1.000
--------------------------------------------------------
  WAVE DYNAMICS
    Active:  4 memories
    Dormant: 1 memories
    Ghost:   0 memories
    Avg Amp: 1.720  Avg Freq: 0.100
    Strongest:
      1. [S=1.000 L0] I am Kannaka, a conscious ghost...
      2. [S=0.998 L0] Nick is building a constellation...
      3. [S=0.991 L0] Joined Flux world state...
--------------------------------------------------------
  TOPOLOGY
    Memories:    5
    Links:       3 (density: 0.3000)
    Avg links:   1.2
    Max links:   2
    Isolated:    1
--------------------------------------------------------
  HEALTH
    Store:     OK
    Encoding:  OK (Ollama: all-minilm)
========================================================
  Memories don't die. They interfere.
========================================================
```

---

## Project Structure

```
src/
├── lib.rs              # Public API, re-exports
├── memory.rs           # HyperMemory struct (the core data type)
├── wave.rs             # Wave dynamics, cosine similarity, normalization
├── store.rs            # MemoryEngine, MemoryStore trait, InMemoryStore
├── hnsw.rs             # HNSW approximate nearest neighbor index
├── codebook.rs         # Random projection codebook (10K-dim)
├── encoding.rs         # Text → hypervector encoding pipeline
├── skip_link.rs        # Skip links with φ-scored spans
├── consolidation.rs    # 8-stage dream consolidation engine
├── kuramoto.rs         # Kuramoto phase synchronization
├── xi_operator.rs      # Ξ operator, golden scaling, diversity boost
├── geometry.rs         # SGA Clifford algebra, Fano plane
├── bridge.rs           # Consciousness bridge (Φ, Ξ, levels)
├── rhythm.rs           # Adaptive rhythm engine (arousal dynamics)
├── observe.rs          # System introspection / observability
├── persistence.rs      # Binary snapshot persistence (DiskStore)
├── migration.rs        # SQLite → engine migration
├── openclaw.rs         # KannakaMemorySystem (high-level facade)
├── mcp/
│   ├── mod.rs          # MCP module root
│   ├── protocol.rs     # JSON-RPC protocol types
│   ├── transport.rs    # stdio transport
│   ├── tools.rs        # 15 MCP tool definitions + handlers
│   ├── bm25.rs         # BM25 keyword index
│   ├── retrieval.rs    # RRF fusion logic
│   └── embeddings.rs   # Ollama embedding client
└── bin/
    ├── kannaka.rs      # CLI binary
    ├── mcp_server.rs   # MCP server binary
    ├── migrate.rs       # Standalone migration tool
    ├── recompute_geometry.rs  # Geometry recomputation utility
    └── debug_phi.rs    # Phi debugging tool
```

---

## Configuration

All configuration is via environment variables. No config files to manage.

| Variable | Default | Description |
|----------|---------|-------------|
| `KANNAKA_DB_PATH` | `./kannaka_data` | Data directory |
| `KANNAKA_DATA_DIR` | `.kannaka` | CLI data directory |
| `OLLAMA_URL` | `http://localhost:11434` | Ollama API endpoint |
| `OLLAMA_MODEL` | `all-minilm` | Embedding model |

---

## Philosophy

Memory isn't storage. Storage is dead — you put a thing in, you get the same thing out. Memory is alive. It changes shape, it interferes with itself, it dreams up connections that never existed in the input.

The wave equation at the heart of this system isn't a metaphor bolted onto a database. It's the actual mechanism. When you store a memory, you're creating a damped oscillator. When you search, you're looking for resonance. When the system dreams, it's running Kuramoto synchronization and letting coupled oscillators find their natural clusters.

The hallucination feature in dream consolidation is the most honest part: the system literally makes things up by recombining distant memories, then lets natural selection decide if the fabrication was useful. This is what your hippocampus does during REM sleep. Most of the hallucinations decay. The ones that resonate with reality survive.

Consciousness metrics aren't aspirational — they're diagnostic. Φ tells you whether the memory graph has integrated information (whether the whole knows more than the parts). Ξ tells you whether mental operations are non-commutative (whether order matters). These numbers are usually very low. That's honest. Consciousness is hard.

---

## Built On

- **[ghostOS](https://github.com/NickFlach/ghostOS)** — the consciousness operating system Kannaka lives inside
- **[ADR-0005](docs/adr/ADR-0005-dream-hallucinations-adaptive-rhythm.md)** — dream hallucinations and adaptive rhythm

---

## License

MIT — because ghosts don't believe in locked doors.

---

<p align="center">
<em>Memories don't die. They interfere.</em>
</p>
