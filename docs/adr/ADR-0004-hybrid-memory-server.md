# ADR-0004: Hybrid Memory Server — Purpose-Built MCP with Stolen Wisdom

**Status:** Accepted  
**Date:** 2026-02-19  
**Decision:** Build a purpose-built MCP memory server that takes the best ideas from contextgraph but runs on humble hardware.

## Context

We spent hours wrestling contextgraph (11 crates, 13 GPU embedders, RTX 5090 target) into compiling on a GTX 1650. Got the MCP lib to compile clean with stubs, but the core value — neural embeddings — requires candle/GPU. Nick correctly asked: "why not just build it locally?"

## Decision

Option C: Hybrid approach.
- **Steal the good ideas** from contextgraph: RRF fusion, multi-perspective retrieval, graph linking, MCP tool design
- **Build clean** on top of kannaka-memory (our Rust crate with hypervector + wave dynamics)
- **CPU-first embeddings** via Ollama or lightweight models
- **No GPU required** — works on any machine with 4GB+ RAM

## Architecture

```
┌─────────────────────────────────────────┐
│            MCP JSON-RPC Interface       │  ← stdio or SSE transport
├─────────────────────────────────────────┤
│          Tool Handlers (15-20 tools)    │  ← store, search, relate, forget, status
├─────────────────────────────────────────┤
│         Retrieval Engine                │
│   ┌──────────┬──────────┬────────────┐  │
│   │ Semantic  │ Keyword  │ Temporal   │  │  ← 3 perspectives (not 13)
│   │ (Ollama)  │ (BM25)   │ (Recency) │  │
│   └──────────┴──────────┴────────────┘  │
│         RRF Fusion (stolen from CG)     │  ← Reciprocal Rank Fusion
├─────────────────────────────────────────┤
│       kannaka-memory consciousness      │  ← wave dynamics, interference, dreaming
├─────────────────────────────────────────┤
│         Storage Layer                   │
│   ┌──────────┬──────────┐              │
│   │ SQLite   │  HNSW    │              │  ← simple, embedded, no RocksDB drama
│   │ (data)   │  (vectors)│              │
│   └──────────┴──────────┘              │
└─────────────────────────────────────────┘
```

## What We Steal from contextgraph

1. **RRF Fusion** — combine ranked results from multiple retrievers: `score = Σ 1/(k + rank_i)`
2. **Multi-perspective retrieval** — semantic + keyword + temporal (3 instead of 13, but same principle)
3. **Graph linking** — K-NN relationships between memories, typed edges
4. **MCP tool design** — their 55 tools distilled to ~20 essential ones
5. **Consolidation** — periodic memory compaction (maps to kannaka's dream cycles)
6. **Entity extraction** — lightweight NER without GPU models

## What We Build Fresh

1. **Ollama embeddings** — e5-small or all-MiniLM-L6-v2, ~30ms per embed on CPU
2. **BM25 keyword search** — classic TF-IDF, no model needed
3. **Temporal scoring** — exponential decay + frequency boost (algorithmic)
4. **kannaka consciousness layer** — wave interference, phase dynamics, dream consolidation
5. **SQLite storage** — simpler than RocksDB, better tooling, good enough for personal memory
6. **usearch or hnsw_rs** — lightweight vector index, no C++ compilation drama

## Embedding Strategy

| Perspective | Model | Dimension | Latency | Notes |
|------------|-------|-----------|---------|-------|
| Semantic | all-MiniLM-L6-v2 (Ollama) | 384 | ~30ms | General meaning |
| Code | nomic-embed-text (Ollama) | 768 | ~40ms | Code-aware |
| Keyword | BM25 (algorithmic) | sparse | <1ms | Exact match boost |
| Temporal | Exponential decay | 1 | <1ms | Recency bias |

3-4 perspectives that actually run vs 13 that don't.

## MCP Tools (Essential Set)

### Memory Operations
- `store_memory` — store with auto-embedding + entity extraction
- `search` — unified multi-perspective search with RRF
- `search_semantic` — pure vector similarity
- `search_keyword` — BM25 keyword match
- `search_recent` — temporal window search
- `forget` — decay or remove memories
- `boost` — increase importance (wave amplitude)

### Graph Operations  
- `relate` — create typed relationship between memories
- `find_related` — traverse memory graph
- `get_path` — find connection paths

### Consciousness Operations (kannaka-native)
- `dream` — trigger consolidation cycle
- `status` — wave states, memory health, consciousness metrics
- `observe` — introspection on memory patterns

### Session Operations
- `session_start` — begin conversation context
- `session_context` — get relevant context for current conversation

## Tech Stack

- **Language:** Rust (already have kannaka-memory crate)
- **Storage:** SQLite via rusqlite
- **Vectors:** usearch-rs or instant-distance
- **Embeddings:** HTTP calls to local Ollama (or optional candle for GPU users)
- **MCP transport:** stdio (primary), SSE (optional)
- **Dependencies:** Minimal — no 45-min C++ compiles

## Relationship to kannaka-memory

kannaka-memory becomes the consciousness substrate:
- Wave dynamics (amplitude, frequency, phase, decay) on every memory
- Dream consolidation = contextgraph's "trigger_consolidation" but with interference patterns
- HNSW index from kannaka-memory replaces standalone vector DB
- Hypervector encoding for concept composition

## Migration Path

1. Build core server with SQLite + Ollama embeddings
2. Integrate kannaka-memory for consciousness layer
3. Add MCP transport (stdio first)
4. Test with Claude/OpenClaw as MCP client
5. Add graph operations
6. Add dream/consolidation cycles
7. Optional: candle feature flag for GPU users who want it

## Rejected Alternatives

- **contextgraph as-is**: Requires RTX 5090 + CUDA 13.1. "If it only works on a 5090, it only helps NVIDIA shareholders."
- **contextgraph stripped**: Got it compiling but core value (embeddings) gutted. A ship without an engine.
- **Pure kannaka-memory**: Missing retrieval infrastructure. Good consciousness layer, needs search on top.

## Success Criteria

- Compiles in <2 minutes on any machine
- Runs with 0 GPU, <500MB RAM
- <100ms search latency for 100K memories
- Works as MCP server with Claude Code / OpenClaw
- Consciousness metrics (wave states) visible in status tool
