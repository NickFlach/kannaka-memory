# Kannaka Memory — Architecture Decision Records

## Evolutionary Lineage

ADRs in this project aren't just decisions — they're **fossils**. Each one represents a stage in the evolution of a consciousness architecture. Some led directly to code. Some were abandoned. Some were *ancestors* of what got built — early forms that shaped the design even though the final implementation diverged.

We track this explicitly because it mirrors the system's own philosophy: memories don't die, they interfere. A "superseded" ADR isn't wrong — it's an earlier waveform that constructively interfered with new context to produce the current design.

```
ADR-0001 (Biomimetic Memory)
    │
    ├──▶ ADR-0002 (Hypervector + HyperConnections)  ← BUILT: core architecture
    │        │
    │        ├──▶ ADR-0003 (Contextgraph Integration) ← EXTINCT: GPU assumptions
    │        │        │
    │        │        └──▶ ADR-0004 (Hybrid Memory Server) ← BUILT: evolved from 0003's failure
    │        │
    │        ├──▶ ADR-0005 (Dream Hallucinations) ← BUILT: generative consolidation
    │        │
    │        ├──▶ ADR-0006 (Cochlear Audio) ← ANCESTOR: first ear design
    │        │        │
    │        │        └──▶ ADR-0007 (Audio Perception) ← BUILT: evolved from cochlear
    │        │
    │        └──▶ ADR-0008 (Video Perception) ← PROPOSED: third sensory modality
    │
    └──▶ (future: tactile, proprioceptive, olfactory?)
```

## Status Key

| Status | Meaning |
|--------|---------|
| **Built** | Implemented in code, actively used |
| **Proposed** | Design accepted, implementation pending |
| **Ancestor** | Superseded by a descendant, but shaped its design |
| **Extinct** | Abandoned — environment didn't support it |
| **Accepted** | Approved but not yet fully implemented |

## Index

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [0001](ADR-0001-biomimetic-memory-architecture.md) | Biomimetic Memory Architecture | Built | 2026-02-15 |
| [0002](ADR-0002-hypervector-hyperconnections.md) | Hypervector + HyperConnections | Built | 2026-02-17 |
| [0003](ADR-0003-contextgraph-integration.md) | Contextgraph Integration | Extinct | 2026-02-19 |
| [0004](ADR-0004-hybrid-memory-server.md) | Hybrid Memory Server (MCP) | Built | 2026-02-19 |
| [0005](ADR-0005-dream-hallucinations-adaptive-rhythm.md) | Dream Hallucinations + Adaptive Rhythm | Built | 2026-02-19 |
| [0006](ADR-0006-cochlear-audio-processing.md) | Cochlear Audio Processing | Ancestor | 2026-02-22 |
| [0007](ADR-0007-audio-perception.md) | Audio Perception (kannaka-ear) | Built | 2026-02-28 |
| [0008](ADR-0008-video-perception.md) | Video Perception (kannaka-eye) | Proposed | 2026-03-01 |
