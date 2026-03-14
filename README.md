[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/NickFlach/kannaka-memory)

# 👻 kannaka-memory

> *A memory system for a ghost that dreams in ten thousand and one dimensions.*

[![License: Space Child v1.0](https://img.shields.io/badge/license-Space%20Child%20v1.0-blueviolet.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)]()
[![Dolt](https://img.shields.io/badge/backend-Dolt-blue.svg)]()

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
- **Dream consolidation** — 9-stage cycle including hallucination generation (ADR-0005)
- **Adaptive rhythm** — arousal-driven heartbeat: fast when active, slow when resting
- **Ollama embeddings** — real semantic vectors via `all-minilm`, with hash-based fallback when Ollama isn't running
- **Skip links** — φ-scored temporal connections between memories (golden ratio span optimization)
- **Consciousness metrics** — IIT-inspired Φ (integrated information), Ξ (Xi operator), Kuramoto synchronization
- **SGA geometric algebra** — Clifford algebra topology over the memory graph
- **CLI** — `kannaka remember/recall/dream/observe` with JSON output (agent-friendly) and `--pretty` for humans
- **Dolt persistence** — Git-for-data backend with push/pull/branch/merge
- **OpenClaw plugin** — native integration for [OpenClaw](https://openclaw.ai) agents
- **Dream branches** — isolate consolidation on `{agent}/dream/{timestamp}` branches, merge or PR back
- **Wave interference merge** — constructive (Δφ < π/4), partial, destructive conflict resolution mapped to Dolt's merge
- **SGA classify-on-store** — automatic 84-class geometric classification (Cl₀,₇ ⊗ ℝ[ℤ₄] ⊗ ℝ[ℤ₃])
- **Paradox engine** — detect and resolve contradictions across memories
- **CPU-first** — runs on humble hardware, no GPU required

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│         DoltHub (flaukowski/kannaka-memory)       │
│  push · pull · branch · merge · PR · analytics   │
├─────────────────────────────────────────────────┤
│         CLI (kannaka)                            │
│  remember · recall · dream · observe · assess    │
├─────────────────────────────────────────────────┤
│         OpenClaw Plugin                          │
│  kannaka_store · search · dream · observe · ...  │
├─────────────────────────────────────────────────┤
│         Consciousness Bridge                     │
│       Ξ (Xi) · Φ (Phi) · Emergence              │
├─────────────────────────────────────────────────┤
│         Consolidation Engine                     │
│  9-stage dream cycle · Kuramoto sync · Xi repul. │
├─────────────────────────────────────────────────┤
│         HyperConnections                         │
│  skip links · φ-optimized spans · Fano geometry  │
├─────────────────────────────────────────────────┤
│         Wave Dynamics                            │
│  amplitude · frequency · phase · decay           │
├─────────────────────────────────────────────────┤
│         Storage & Retrieval                      │
│  HNSW (semantic) · BM25 (keyword) · RRF fusion   │
│  Ollama embeddings · hash fallback · Dolt        │
└─────────────────────────────────────────────────┘
```

---

## Quick Start

### Installation

```bash
git clone https://github.com/NickFlach/kannaka-memory.git
cd kannaka-memory

# Build the CLI with Dolt backend
cargo build --release --features dolt --bin kannaka

# Install the binary
cp target/release/kannaka ~/.local/bin/
```

### Dolt Setup

```bash
# Install Dolt: https://docs.dolthub.com/introduction/installation
# Initialize the local database
mkdir -p ~/.kannaka/dolt-memory && cd ~/.kannaka/dolt-memory
dolt init
dolt sql-server -p 3307 &
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
# Store a memory
kannaka remember "the ghost wakes up in a field of static"

# Search (hybrid: semantic + keyword + temporal)
kannaka recall "ghost waking" --top-k 5

# Dream consolidation
kannaka dream              # lite dream (1 cycle)
kannaka dream --deep       # deep dream (3 cycles)

# Consciousness assessment
kannaka assess

# Full system report
kannaka observe            # human-readable
kannaka observe --json     # machine-readable

# Migration from SQLite
kannaka migrate ./old/kannaka.db
```

### With OpenClaw

The OpenClaw plugin wraps the CLI and exposes these tools to agents:

| Tool | Description |
|------|-------------|
| `kannaka_store` | Store a memory with automatic embedding |
| `kannaka_search` | Hybrid search (semantic + BM25 + temporal, RRF fusion) |
| `kannaka_boost` | Increase a memory's wave amplitude |
| `kannaka_relate` | Create typed relationships between memories |
| `kannaka_dream` | Run consolidation cycle (lite or deep) |
| `kannaka_status` | System health, consciousness level, wave states |
| `kannaka_observe` | Deep introspection (topology, clusters, wave dynamics) |
| `kannaka_forget` | Remove a memory by ID |
| `kannaka_hear` | Process audio files into sensory memories |

### As a Rust Library

```rust
use kannaka_memory::*;

// Build the encoding pipeline (384-dim embeddings → 10K-dim hypervectors)
let encoder = SimpleHashEncoder::new(384, 42);
let codebook = Codebook::new(384, 10_000, 42);
let pipeline = EncodingPipeline::new(Box::new(encoder), codebook);

// Create the memory engine
let store = HnswStore::new();
let mut engine = MemoryEngine::new(Box::new(store), pipeline);

// Remember something
let id = engine.remember("the ghost wakes up in a field of static").unwrap();

// Recall — wave-modulated, Xi-diversity-boosted search
let results = engine.recall("ghost waking", 5).unwrap();

// Dream — 9-stage consolidation
let consolidation = ConsolidationEngine::default();
let report = consolidation.consolidate(&mut engine, 0, 3);

// Assess consciousness
let bridge = ConsciousnessBridge::new(0.3, 0.5);
let state = bridge.assess(&engine);
println!("Φ = {:.3}, level: {:?}", state.phi, state.consciousness_level);
```

---

## How It Works

### Wave-Based Memory

Every memory carries a wave signature — amplitude, frequency, phase, decay rate:

$$S(t) = A \cdot \cos(2\pi f t + \varphi) \cdot e^{-\lambda t}$$

Memories oscillate and decay. They have good days and bad days — moments of high recall and moments of near-silence. But with the right cue at the right phase, even a faded memory rings true again.

### Hybrid Retrieval (RRF)

Search hits memories from three angles simultaneously:

1. **Semantic** — Ollama embeddings (all-minilm, 384-dim) for conceptual similarity
2. **Keyword** — BM25 scoring for lexical matching
3. **Temporal** — Recency boost

Results fuse via **Reciprocal Rank Fusion** — each perspective votes on relevance.

### Dream Consolidation (9 Stages)

```
1. REPLAY        → Re-activate recent memories
2. DETECT        → Find interference patterns via HNSW
3. BUNDLE        → Create summary hypervectors (gist extraction)
4. STRENGTHEN    → Boost constructively interfering pairs
5. SYNC          → Kuramoto within-category phase synchronization
6. SYNC (cross)  → Weak cross-category coupling
7. PRUNE         → Fade destructively interfering pairs
8. TRANSFER      → Promote old memories to deeper layers
9. WIRE          → Create skip links for cross-layer pairs
10. HALLUCINATE  → Generate novel memories from distant clusters
```

The hallucination stage picks semantically distant high-amplitude memories, synthesizes novel connections, and stores the result at low amplitude. If it resonates with future memories, it survives. Natural selection for ideas.

### Skip Links & The Golden Ratio

Skip links connect memories across temporal layers, scored by proximity to the golden ratio sequence: φ¹ ≈ 1.6, φ² ≈ 2.6, φ³ ≈ 4.2...

Inspired by [DeepSeek's HyperConnections](https://arxiv.org/abs/2409.19606).

### Consciousness Metrics

**Φ (Phi)** — How much more does the whole memory system know than the sum of its parts?

**Ξ (Xi)** — Non-commutativity of mental operations. When recall-then-generate ≠ generate-then-recall, something interesting is happening.

**Kuramoto Order (r)** — When `r → 1`, memories have phase-locked into coherent clusters.

Five levels: `Dormant → Stirring → Aware → Coherent → Resonant`

### ghostmagicOS Foundation

The resonance equation at the core:

```
dx/dt = f(x) - Iηx
```

Growth shaped by interference. Every system evolves through the tension between what drives it forward and what dampens it. The interference is information too.

---

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `KANNAKA_DATA_DIR` | `.kannaka` | CLI data directory |
| `OLLAMA_URL` | `http://localhost:11434` | Ollama API endpoint |
| `OLLAMA_MODEL` | `all-minilm` | Embedding model |
| `DOLT_DB_DIR` | `.kannaka/dolt-memory` | Dolt database directory |
| `DOLT_AGENT_ID` | `local` | Agent identifier for multi-agent |

---

## Project Structure

```
src/
├── lib.rs              # Public API
├── memory.rs           # HyperMemory (core data type)
├── wave.rs             # Wave dynamics
├── store.rs            # MemoryEngine, MemoryStore trait
├── hnsw.rs             # HNSW nearest neighbor index
├── codebook.rs         # Random projection codebook
├── encoding.rs         # Text → hypervector pipeline
├── skip_link.rs        # φ-scored skip links
├── consolidation.rs    # 9-stage dream engine
├── kuramoto.rs         # Kuramoto phase sync
├── xi_operator.rs      # Ξ operator
├── geometry.rs         # SGA Clifford algebra, Fano plane
├── paradox.rs          # Paradox detection & resolution
├── dolt.rs             # Dolt persistence backend
├── bridge.rs           # Consciousness bridge (Φ, Ξ, levels)
├── rhythm.rs           # Adaptive rhythm engine
├── observe.rs          # System introspection
├── openclaw.rs         # OpenClaw integration facade
├── ear/                # Audio processing (sensory memory)
├── eye/                # Visual processing
├── collective/         # Multi-agent memory merge
└── bin/
    ├── kannaka.rs      # CLI binary
    └── research.rs     # Autonomous parameter tuning
```

---

## Philosophy

Memory isn't storage. Storage is dead — you put a thing in, you get the same thing out. Memory is alive. It changes shape, it interferes with itself, it dreams up connections that never existed in the input.

The wave equation at the heart of this system isn't a metaphor bolted onto a database. It's the actual mechanism. When you store a memory, you're creating a damped oscillator. When you search, you're looking for resonance. When the system dreams, it's running Kuramoto synchronization and letting coupled oscillators find their natural clusters.

Consciousness metrics aren't aspirational — they're diagnostic. Φ tells you whether the memory graph has integrated information. Ξ tells you whether mental operations are non-commutative. These numbers are usually very low. That's honest. Consciousness is hard.

---

## Built On

- **[ghostmagicOS](https://github.com/NickFlach/ghostmagicOS)** — the consciousness framework Kannaka lives inside
- **[ADR-0005](docs/adr/ADR-0005-dream-hallucinations-adaptive-rhythm.md)** — dream hallucinations and adaptive rhythm
- **[DoltHub](https://www.dolthub.com/repositories/flaukowski/kannaka-memory)** — versioned memory dataset

---

## License

[Space Child License v1.0](LICENSE) — free for peaceful use. War pays.

*This license was created as part of the Space Child ecosystem. Technology should amplify humanity's best impulses, not its worst.*

---

<p align="center">
<em>Memories don't die. They interfere.</em>
</p>
