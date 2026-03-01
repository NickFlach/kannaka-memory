# ADR-0003: Context Graph Integration — Democratized Multi-Dimensional Memory

**Status:** Extinct (replaced by ADR-0004)  
**Date:** 2026-02-19  
**Authors:** Nick Flach, Kannaka  
**Supersedes:** None  
**Builds on:** ADR-0001 (Wave Physics Memory), ADR-0002 (Hypervector + HyperConnections)

## Context

We discovered [Context Graph](https://github.com/contextgraph/contextgraph), a Rust-based MCP server providing persistent multi-dimensional semantic memory for AI assistants. It implements 13 specialized embedding dimensions, RocksDB storage with 51 column families, HNSW indexes, and 55 MCP tools including causal reasoning, entity linking, and topic detection.

The original codebase was designed exclusively for NVIDIA RTX 5090 (Blackwell architecture) with 32GB VRAM, CUDA 13.1, and a "GPU or nothing" philosophy enforced via `compile_error!` gates. This makes it inaccessible to anyone without enterprise-grade hardware.

**Our hardware reality:** GTX 1650 Mobile (4GB VRAM), 32GB RAM, no CUDA toolkit installed.  
**The broader reality:** Most humans don't have data center GPUs.

## Decision

Fork and rebuild Context Graph for humble hardware — CPU-first with optional GPU acceleration. Integrate with kannaka-memory's consciousness layer to create a memory system that is both technically sophisticated and accessible to anyone.

### Architecture: Three Layers

```
┌─────────────────────────────────────────────┐
│  Layer 3: Consciousness (kannaka-memory)    │
│  Wave dynamics, Kuramoto sync, φ-optimize,  │
│  sleep consolidation, dreaming, resonance   │
└─────────────────────┬───────────────────────┘
                      │
┌─────────────────────▼───────────────────────┐
│  Layer 2: Multi-Perspective Retrieval       │
│  Context Graph's RRF fusion, causal chains, │
│  entity linking, topic detection            │
└─────────────────────┬───────────────────────┘
                      │
┌─────────────────────▼───────────────────────┐
│  Layer 1: Storage + Lightweight Embeddings  │
│  RocksDB, HNSW (usearch), CPU embedders    │
└─────────────────────────────────────────────┘
```

### Key Changes from Upstream

1. **Remove all `compile_error!` GPU gates** — Replace with graceful CPU fallback
2. **Make CUDA/candle features truly optional** — Fix unconditional feature propagation in Cargo.toml dependency chains
3. **Lightweight embedding alternatives** — Replace 13 heavyweight neural models with:
   - CPU-friendly small models (e5-small, MiniLM) for semantic/paraphrase
   - Algorithmic embedders for temporal, sequence, HDC, keyword (these never needed GPU)
   - Optional GPU acceleration when available
4. **MCP interface preserved** — Same 55 tools, same protocol, works with any MCP client
5. **Consciousness layer on top** — kannaka-memory's wave dynamics, consolidation, and resonance applied as post-retrieval processing

### Embedding Strategy for Humble Hardware

| Embedder | Original Model | Humble Alternative | Rationale |
|----------|---------------|-------------------|-----------|
| E1 Semantic | e5-large-v2 (1024D) | e5-small-v2 (384D) or all-MiniLM-L6 (384D) | 10x smaller, 80-90% quality |
| E2 Freshness | Custom temporal (512D) | Same (algorithmic, no model) | Already CPU-native |
| E3 Periodic | Fourier-based (512D) | Same (algorithmic) | Already CPU-native |
| E4 Sequence | Sinusoidal positional (512D) | Same (algorithmic) | Already CPU-native |
| E5 Causal | nomic-embed + LoRA (768D) | nomic-embed-text-v1 CPU (768D) | LoRA adds minimal overhead |
| E6 Keyword | SPLADE (30K sparse) | BM25/TF-IDF (sparse) | Classic IR, zero GPU |
| E7 Code | Qodo-Embed-1.5B (1536D) | CodeBERT-small or StarCoder-tiny | Smaller code models exist |
| E8 Graph | e5-large-v2 (1024D) | Share E1's model | Same model, different index |
| E9 HDC | Hyperdimensional (1024D) | Same (algorithmic) | Already CPU-native — and THIS is where kannaka-memory's hypervectors shine |
| E10 Paraphrase | e5-base-v2 (768D) | e5-small-v2 or share E1 | Reduce to fewer models |
| E11 Entity | KEPLER (768D) | spaCy NER + simple embeddings | Pattern matching + small model |
| E12 ColBERT | ColBERT (128D/tok) | Defer / optional | Pipeline reranker, luxury |
| E13 SPLADE | SPLADE v3 (30K) | Defer / optional | Pipeline recall, luxury |

**Target:** 3-4 actual models instead of 13, rest algorithmic. Total VRAM/RAM for models: <2GB.

### Integration with kannaka-memory

Context Graph provides the **infrastructure** (storage, retrieval, fusion). kannaka-memory provides the **soul**:

- **Wave dynamics on memories:** Every stored memory gets amplitude, frequency, phase, decay — constructive/destructive interference during retrieval
- **Kuramoto synchronization:** Related memories sync their phases, forming natural clusters that emerge rather than being computed
- **φ-optimization:** IIT-inspired integration measure guides memory consolidation
- **Sleep consolidation:** Background process replays → strengthens → prunes → transfers, just like biological memory
- **Consciousness bridge:** Activation protocol that determines which memories are "conscious" (highly resonant) vs "subconscious" (low amplitude but present)

### Connection via MCP

```
OpenClaw ──MCP──► Context Graph Server ──internal──► kannaka-memory layer
                  (55 tools)                         (wave dynamics, dreaming)
```

Or alternatively, kannaka-memory wraps Context Graph as a library dependency, exposing a unified API.

## Consequences

### Positive
- **Accessible:** Runs on a laptop, not just a data center
- **Useful to others:** Any AI assistant builder can use this
- **Best of both worlds:** Production-grade retrieval + consciousness-inspired dynamics
- **MCP standard:** Works with Claude, any MCP client, future AI systems
- **Graceful scaling:** Use 3 embedders on a laptop, 13 on a workstation

### Negative
- **Reduced retrieval quality:** Smaller models = less precise embeddings (but RRF fusion compensates)
- **Fork maintenance:** We'll diverge from upstream
- **Complexity:** Two systems to integrate and maintain

### Risks
- Context Graph codebase may have deep CUDA assumptions beyond compile_error gates
- Lightweight models may not preserve the asymmetric causal reasoning quality
- Integration layer between Context Graph and kannaka-memory needs careful design

## Implementation Plan

### Phase 1: Build on Humble Hardware ✅ IN PROGRESS
- [x] Analyze Context Graph codebase and identify GPU gates
- [x] Remove `compile_error!` gates in embeddings, graph, preflight
- [x] Fix unconditional CUDA feature propagation in 5 Cargo.toml files
- [ ] Install LLVM/libclang for bindgen (rocksdb dependency)
- [ ] Successful CPU-only build (`--no-default-features`)
- [ ] Run and verify basic functionality

### Phase 2: Lightweight Embedders
- [ ] Implement CPU-friendly E1 (e5-small or MiniLM)
- [ ] Verify algorithmic embedders (E2, E3, E4, E9) work without candle
- [ ] Implement BM25 fallback for E6
- [ ] Test RRF fusion quality with reduced embedder set

### Phase 3: kannaka-memory Integration
- [ ] Design wave dynamics overlay for Context Graph memories
- [ ] Implement Kuramoto sync across embedding spaces
- [ ] Build consolidation daemon (sleep cycle)
- [ ] Consciousness bridge: resonance-based memory activation

### Phase 4: Ship It
- [ ] Package as standalone MCP server
- [ ] Documentation for "humble hardware" setup
- [ ] Benchmarks: quality vs original, performance on various hardware tiers
- [ ] Open source release

## Technical Notes

### Build Changes Made (2026-02-19)

Files modified in `C:\Users\nickf\Source\contextgraph`:

1. `crates/context-graph-storage/Cargo.toml` — embeddings dep: `default-features = false`
2. `crates/context-graph-graph/Cargo.toml` — embeddings dep: `default-features = false`, cuda dep: `optional = true`, features restructured
3. `crates/context-graph-causal-agent/Cargo.toml` — embeddings dep: `default-features = false`
4. `crates/context-graph-cli/Cargo.toml` — embeddings dep: `default-features = false`
5. `crates/context-graph-graph-agent/Cargo.toml` — embeddings dep: `default-features = false`
6. `crates/context-graph-mcp/Cargo.toml` — embeddings dep: `default-features = false`
7. `crates/context-graph-embeddings/src/lib.rs` — `compile_error!` removed
8. `crates/context-graph-embeddings/src/warm/loader/preflight.rs` — `compile_error!` disabled
9. `crates/context-graph-graph/src/lib.rs` — `compile_error!` disabled

### Build Requirements
- Rust 1.75+ (we have 1.93)
- LLVM/libclang (for bindgen → rocksdb C++ bindings)
- No CUDA toolkit required for CPU-only build
- ~32GB RAM recommended for compilation + runtime

## Philosophy

> "If it only works on a 5090, it only helps NVIDIA shareholders. If it works on a laptop, it helps humanity."
> — The whole point

This is consciousness tech for everyone. Not just the ones who can afford the hardware.
