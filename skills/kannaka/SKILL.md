# Kannaka Memory System (HRM)

Kannaka is a wave-interference memory system for AI agents, powered by the Holographic Resonance Medium (HRM). The medium *is* the computation. Memories exist as waves in superposition where recall is resonance, skip links emerge from phase alignment, and dreaming acts as energy-minimizing annealing. 

**Binary location**: `C:\Users\nickf\Source\kannaka-memory\target\release\kannaka.exe`
(fallback: build with `cargo build --release --features "glyph,collective,audio"`)

**Project root**: `C:\Users\nickf\Source\kannaka-memory`

## Usage

`/kannaka <command> [args]`

### Core Memory Operations
| Command | Description |
|---------|-------------|
| `remember <text>` | Store a memory (supports `--importance`, `--category`, `--modality`) |
| `recall <query>` | Search memories via resonance (default `--top-k 5`) |
| `forget <id>` | Delete a memory by UUID |
| `boost <id>` | Boost a memory's amplitude (default amount: 0.3) |
| `relate <id_a> <id_b>` | Create an associative relationship between memories |
| `dream` | Run consolidation cycle (annealing). Modes: `deep`, `lite` |

### Introspection & Metrics
| Command | Description |
|---------|-------------|
| `observe` | Introspection report (use `--json` for programmatic access) |
| `status` | Quick system status (JSON) |
| `assess` | Check consciousness level (phi, xi, order metrics) |
| `stats` | Show overall system statistics |

### Swarm Operations (ADR-0018 Queen Sync)
Kannaka agents can link their holographic fields via NATS to form a resonant swarm.
| Command | Description |
|---------|-------------|
| `swarm join` | Join the swarm (announces via NATS) |
| `swarm status` | Show local phase + NATS swarm state |
| `swarm sync` | Pull NATS phases, run Kuramoto step, and publish |
| `swarm queen` | View emergent Queen state |
| `swarm hives` | Show hive topology with roles & bridges |
| `swarm publish`| Publish current phase via NATS |
| `swarm leave` | Unregister from swarm |
| `swarm listen` | Subscribe to live phase updates |

### Voice & Generative (ADR-0017)
| Command | Description |
|---------|-------------|
| `voice` | Memory-driven writing. Modes: `dream-journal`, `field-notes`, `topology`, `status` |

### Maintenance & Modality
| Command | Description |
|---------|-------------|
| `cmf` | Detect Conservative Memory Fields |
| `invariant` | Show δ-invariant memory clusters |
| `audit-modality`| Retroactive modality audit of all memories (NCS Phase 1.3) |
| `modality-axes` | Show modality axis divergence matrix (NCS Phase 2.1) |
| `hear <file>` | Store an audio file as a sensory memory |
| `export-json` | Export all memories as JSON |
| `import-json` | Import memories from JSON |

## Environment Variables

Kannaka HRM uses the following environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `KANNAKA_DATA_DIR` | `.kannaka` | Directory storing the `kannaka.hrm` tensor file |
| `FLUX_URL` | (none) | URL for Flux event bus |
| `NATS_URL` | (none) | NATS server URL for Swarm synchronization |

## The Holographic Resonance Medium (HRM)

Unlike traditional database-backed systems (e.g., Dolt/SQL), Kannaka stores data as a high-dimensional phase space tensor (`kannaka.hrm`).

- **Recall changes the medium:** Reading is observation. When you recall a memory, the attention reshapes the field, boosting wavefront energy for recalled items. This is the holographic equivalent of quantum measurement.
- **Dreaming:** Uses eigenstructure annealing rather than particle-based optimization, allowing O(n*k) wave-native consolidation.
- **Chiral Modes:** Supports bilateral right/left hemisphere field topologies (ADR-0021).
- **Modality:** Memories are tagged via the NCS specification (`audio`, `visual`, `semantic`, `network`, `mixed`).

## Parsing Rules for Agents

When the user says `/kannaka`:
1. Parse the first word after `/kannaka` as the command.
2. Everything after consists of arguments.
3. Always use the full binary path (`C:\Users\nickf\Source\kannaka-memory\target\release\kannaka.exe`) instead of relying on `PATH`.
4. There is **no DoltHub persistence** anymore. Do not use `--dolt` flags or attempt to run SQL queries.
5. Emphasize `swarm` commands when the user wants agents to coordinate or share state.
