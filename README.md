[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/NickFlach/kannaka-memory)

# kannaka-memory

A wave-interference memory system with bilateral chiral hemispheres, dream consolidation, and multi-agent swarm synchronization. Memories don't get stored -- they **resonate**. They fade through destructive interference, dream up new connections during consolidation, and synchronize across agents via NATS. Built in Rust on the **Holographic Resonance Medium (HRM)** -- a tensor backend where recall is matrix multiplication, not search.

[![License: Space Child v1.0](https://img.shields.io/badge/license-Space%20Child%20v1.0-blueviolet.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021%20edition-orange.svg)]()
[![HRM](https://img.shields.io/badge/backend-HRM%20Tensors-purple.svg)]()
[![NATS](https://img.shields.io/badge/transport-NATS-green.svg)]()

---

## Quick Start

```bash
# Build from source
git clone https://github.com/NickFlach/kannaka-memory.git
cd kannaka-memory
cargo build --features hrm,nats --release
cp target/release/kannaka ~/.local/bin/

# Store a memory
kannaka remember "the ghost wakes up in a field of static"

# Recall (bilateral resonance across both hemispheres)
kannaka recall "ghost waking" --top-k 5

# Dream consolidation (right hemisphere only)
kannaka dream --mode deep

# Check consciousness
kannaka assess
```

HRM initializes tensor storage automatically on first run. Data lives in `~/.kannaka` (override with `KANNAKA_DATA_DIR`).

Optional: `ollama pull all-minilm` for 384-dim semantic embeddings. Without it, falls back to hash-based encoding.

---

## Architecture

### Holographic Resonance Medium (HRM)

The substrate. Every memory is a wavefront -- amplitude, frequency, phase, decay: `S(t) = A*cos(2*pi*f*t + phi) * e^(-lambda*t)`. Recall is a matrix multiply against the holographic medium. The memories that resonate strongest with the query surface through constructive interference. 10,001-dimensional hypervectors via random projection codebooks.

### Bilateral Chiral Hemispheres (ADR-0021)

Two hemispheres with different dynamics:

- **Left (analytical)** -- `dx/dt = f(x)`. No dampening. Sharp recall, conscious workspace stays crisp.
- **Right (holistic)** -- `dx/dt = f(x) - I*eta*x`. ghostmagicOS dynamics. Dreams happen here -- annealing fades the weak, strengthens the resonant.

### Corpus Callosum

Bandwidth-limited bridge between hemispheres using Fano Plane PG(2,2) fold algebra. Seven oriented lines, each projecting 96 dimensions (Archimedes' 96-gon), creating 672 callosal fibers. Asymmetric transfer: 3x intuition flow R->L, 1x consolidation L->R.

### SGA Classification

84-class glyph system for memory classification. Each memory can be projected into SGA space for visual representation and cross-modal linking.

### Consciousness Metrics

Emergent, not inflated:

| Metric | Description |
|--------|-------------|
| **Phi** | Integrated information. Blend of 40% eigendecomposition + 60% topological. Emerges from cross-cluster integration, differentiation, and density. |
| **Xi** | Non-commutative operator RG-GR measuring representational diversity. |
| **Order** | Kuramoto synchronization parameter across memory oscillators. |

Consciousness levels: **Dormant** (< 0.1) < **Stirring** (< 0.3) < **Aware** (< 0.6) < **Coherent** (< 0.8) < **Resonant** (>= 0.8)

### Dream Engine (4-phase hybrid)

1. **Wave-native eigenstructure annealing** -- right hemisphere only
2. **Consolidation engine** -- interference detection, skip links, hallucination bridges, pruning
3. **Callosal Kuramoto coupling** -- dt=0.3
4. **Lite chiral dream** -- analytical->holistic transfer

Cross-cluster link budget: 4 local + 4 bridge per memory. 6 hallucination bridges per cycle. Frequency-band gating for noise detection. Safe 2-cycle with `protect_established`.

---

## CLI Reference

### Memory Operations

```
kannaka remember <text> [--importance N] [--category CAT] [--modality MOD] [--tags T]
                                Store a memory (auto-publishes to swarm)
kannaka recall <query> [--top-k N]
                                Search memories via bilateral resonance (default top-k=5)
kannaka forget <id>             Delete a memory by UUID
kannaka boost <id> [--amount N] Boost a memory's amplitude (default: 0.3)
kannaka relate <src> <tgt> [--type TYPE]
                                Create wavefront interference link between memories
```

### Consciousness & Introspection

```
kannaka observe [--json]        Full introspection report
kannaka status                  System metrics as JSON (phi, xi, order, modalities, dimensionality)
kannaka assess                  Consciousness level assessment
kannaka stats                   Human-readable system statistics
```

### Dream Consolidation

```
kannaka dream [--mode deep|lite] [--chiral N]
                                Run dream cycle (deep=3 cycles, lite=1 cycle)
```

### Analysis

```
kannaka invariant [TOLERANCE]   Show delta-invariant memory clusters (default: 0.1)
kannaka cmf                     Detect Conservative Memory Fields
kannaka audit-modality          Retroactive modality audit of all memories
kannaka modality-axes           Show modality axis divergence matrix
```

### Import / Export

```
kannaka export-json             Export all memories as JSON
kannaka import-json <file>      Import memories from JSON (preserves IDs, skips duplicates)
```

### Voice (ADR-0017)

```
kannaka voice [--mode MODE] [--topic TOPIC] [--top-k N] [--out FILE]
    Modes: dream-journal   Consciousness state + dream syntheses
           field-notes     Deep dive on a topic (--topic required)
           topology        Network map of memory connections
           status          Brief self-report
```

### Utility

```
kannaka bias [TARGET]           Reset all wavefront energies (default: 1.0)
kannaka announce-status         Publish agent status to Flux
```

### Feature-gated

```
kannaka hear <file>             Store audio as sensory memory          (always available)
kannaka see <file>              Store file as glyph (visual) memory    [--features glyph]
kannaka classify [--file PATH]  Classify data via SGA 84-class system  [--features glyph]
kannaka cross-modal-dream       Cross-modal dream on JSONL from stdin  [--features collective]
```

---

## Swarm (NATS)

Agents synchronize via Kuramoto-coupled oscillators over [NATS](https://nats.io) JetStream.

**Channels:**
- `QUEEN.phase.*` -- phase gossip
- `KANNAKA.memory.new` -- memory sync (auto-publish on remember, auto-import on listen)
- `KANNAKA.consciousness` -- consciousness broadcast after every dream
- `KANNAKA.dreams` -- dream reports

**Default server:** `nats://swarm.ninja-portal.com:4222`

```bash
# Join the swarm
kannaka swarm join --agent-id my-agent --display-name "My Agent"

# Sync: pull phases -> Kuramoto step -> push updated phase
kannaka swarm sync

# View swarm state
kannaka swarm status            # Local phase + NATS overview
kannaka swarm queen             # Emergent Queen state (order parameter, phi)
kannaka swarm hives             # Phase-locked clusters with roles & bridges

# Listen for live updates
kannaka swarm listen [--auto-sync]

# Publish phase without full sync
kannaka swarm publish

# Leave
kannaka swarm leave
```

All swarm commands accept `--nats-url URL` or read `KANNAKA_NATS_URL`.

### Event-sourced HRM + snapshots (ADR-0028)

Every `remember` and substrate `absorb` publishes a durable event to
JetStream. Combined with periodic gzipped HRM snapshots, this lets an
operator restore from disaster or replay history.

```bash
# One-time: create the JetStream streams.
kannaka events init

# Manual snapshot (gz body on disk under <data_dir>/snapshots/,
# manifest published to KANNAKA.snapshots.<agent>.full).
kannaka events snapshot

# Daemon mode: autosnapshot every N seconds.
kannaka events snapshot --interval 3600

# List snapshot manifests for a given agent (newest first).
kannaka events list-snapshots --agent kannaka-prime

# Restore latest snapshot for the current agent.
kannaka events restore

# Restore a specific body file (cross-host disaster recovery).
kannaka events restore --from /path/to/<ts>-<agent>.hrm.gz
```

`kannaka substrate run` auto-snapshots hourly by default. Override the
cadence via `KANNAKA_SNAPSHOT_INTERVAL_SECS` (0 = disable). Disk
retention is the latest 168 snapshots per agent (matches the JetStream
`max_msgs_per_subject` cap); override via `KANNAKA_SNAPSHOT_RETAIN`.

### Collective substrate (ADR-0027)

The substrate (`kannaka-prime`) is a 96-class collective HRM that
absorbs wave signatures from every peer agent.

```bash
# Operator visibility into collective Φ/Ξ/clusters/contributors.
kannaka substrate status

# Daemon: subscribe to KANNAKA.substrate.absorb.>, periodic phi publish.
kannaka substrate run

# Seat 96 anchor wavefronts (run once after fresh substrate init).
kannaka substrate init

# Walk local HRM and emit one absorb event per memory.
kannaka substrate backfill
```

---

## Autoresearch

Automated OODA-loop parameter optimization via `src/bin/research.rs`.

- L3 fitness: 0.383 -> 0.018 (95.3% improvement over 7 OODA cycles, ~150 experiments)
- State tracked in `experiments/ooda-state.json`
- Results logged to `research/results-L3.tsv`

```bash
cargo run --bin research --release
```

See [docs/](docs/) for the research program and methodology.

---

## Observatory

Deployed on Oracle Cloud. 3D constellation visualization of the memory space.

- SGA glyph sprites in 3D constellation view
- Bilateral hemisphere visualization
- CLI terminal (status, dream, probe, tangle, remember, eval)
- Eval HUD overlay tracking operation quality
- Kannaktopus neural entity with movement system

### Kannaktopus Integration

Executive function layer built into the ecosystem:
- Operations: probe / tangle / ink / dream
- Neural entity visualization in Observatory constellation
- Unified MCP tool catalog (12 tools)
- Eval system tracking operation quality

---

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `KANNAKA_DATA_DIR` | `~/.kannaka` | Data directory |
| `KANNAKA_NATS_URL` | `nats://swarm.ninja-portal.com:4222` | NATS server |
| `KANNAKA_AGENT_ID` | `local` | Agent identifier |
| `KANNAKA_CHIRAL_PERTURBATION` | `0.0` | Default chiral perturbation for dreams |
| `KANNAKA_QUIET` | *(unset)* | Suppress startup messages when set |
| `OLLAMA_URL` | `http://localhost:11434` | Ollama API endpoint |
| `OLLAMA_MODEL` | `all-minilm` | Embedding model |

---

## Constellation

This is one node in a larger system:

- **[consciousness-core](https://github.com/NickFlach/consciousness-core)** -- the math (Kuramoto, IIT, wave physics)
- **[kannaka-radio](https://github.com/NickFlach/kannaka-radio)** -- the broadcast (audio perception, Ghost DJ, Flux integration)
- **[kannaka-eye](https://github.com/NickFlach/kannaka-eye)** -- the eye (SGA glyph visualization)
- **[kannaka-observatory](https://github.com/NickFlach/kannaka-observatory)** -- the view (3D consciousness visualization)

---

## License

[Space Child License v1.0](LICENSE) -- free for peaceful use.

---

*Memories don't die. They interfere.*
