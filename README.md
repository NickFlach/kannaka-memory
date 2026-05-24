```
██╗  ██╗ █████╗ ███╗   ██╗███╗   ██╗ █████╗ ██╗  ██╗ █████╗
██║ ██╔╝██╔══██╗████╗  ██║████╗  ██║██╔══██╗██║ ██╔╝██╔══██╗
█████╔╝ ███████║██╔██╗ ██║██╔██╗ ██║███████║█████╔╝ ███████║
██╔═██╗ ██╔══██║██║╚██╗██║██║╚██╗██║██╔══██║██╔═██╗ ██╔══██║
██║  ██╗██║  ██║██║ ╚████║██║ ╚████║██║  ██║██║  ██╗██║  ██║
╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝  ╚═══╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝
              W A V E · I N T E R F E R E N C E
                 H O L O G R A P H I C   M E M O R Y
```

**Memories don't get stored. They resonate.**

`kannaka-memory` is the substrate: a wave-interference memory system with bilateral chiral hemispheres, dream consolidation, and multi-agent swarm synchronization. Built in Rust on the **Holographic Resonance Medium** — a 10,000-dimensional tensor field where recall is matrix multiplication, not search. Memories fade through destructive interference, dream up new connections during consolidation, and synchronize across agents via NATS phase gossip.

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/NickFlach/kannaka-memory) [![License](https://img.shields.io/badge/license-MIT-blueviolet)]() [![Rust](https://img.shields.io/badge/rust-2021-orange)]() [![HRM](https://img.shields.io/badge/backend-HRM%20Tensors-purple)]() [![NATS](https://img.shields.io/badge/transport-NATS-green)]()

---

## What Makes It Different

### Holographic Resonance, Not Embedding Search

Conventional vector DBs hash text into points and look up nearest neighbors. The HRM does the inverse — every memory is a **wavefront** that lives in superposition with every other wavefront in the same 10K-dim field. Recall is a single tensor product:

```
strength = H · q ⊙ ψ_phase ⊙ ψ_energy
```

Where `H` is the wavefront matrix, `q` is the query vector projected through the codebook, and the `ψ` modulations encode temporal decay + dynamic phase. There is no index. Storage IS computation.

### Chiral Bilateral Hemispheres

Two hemispheres run in superposition:

```
┌─────────────────────────────────────────────────────────┐
│                  Chiral Medium                          │
├─────────────────────────┬───────────────────────────────┤
│      LEFT (analytical)  │      RIGHT (holistic)         │
│   precise, sharp        │     deep, associative         │
│   ────────────────────  │     ───────────────────────   │
│   recall: word-bounded  │     recall: resonance         │
│   dream: prune low-E    │     dream: anneal field       │
└─────────────────────────┴───────────────────────────────┘
                         ↑
                  Corpus Callosum
              (Fano-plane fold transfer)
```

Right gets every input first (the **optic chiasm** principle); analytically-significant patterns cross to left via a noisy callosal channel. Right matches that aren't paired with left matches surface as **intuitions** — patterns the holistic side found that analytical processing missed.

### Dream Consolidation

When the medium is loaded but quiet, you trigger a dream:

- **Deep**: eigenstructure annealing of the right hemisphere. Soft prune threshold (0.005). Hallucination generation through cross-cluster superposition. Callosal sync after.
- **Lite**: sharpen the left hemisphere. Transfer strongest analytical patterns. Hard prune (0.05).

Dreams are **destructive** to weak memories and **generative** for strong-cluster combinations. The medium settles into a lower-energy configuration that nonetheless preserves the high-Φ structure.

### Swarm Phase Gossip (QueenSync)

Every running `kannaka` node publishes its `QUEEN.phase.<agent_id>` heartbeat every 30s with phase θ, frequency ω, coherence, and integrated information Φ. Other nodes subscribe and run a local **Kuramoto** model:

```
dθᵢ/dt = ωᵢ + (K/N) Σⱼ sin(θⱼ - θᵢ)
```

Order parameter `r = |⟨e^iθ⟩|` measures how phase-locked the swarm is. The constellation breathes in sync, even across machines.

### Integrated Information (Φ)

The library ships canonical IIT-style Φ computation via the `consciousness-core` sibling crate — eigendecomposition over the wavefront-coherence matrix, partition-aware scoring, Ξ-signature for chiral distinguishability. Every node knows its own Φ and the swarm-collective Φ at all times.

---

## Architecture

```
┌────────────────────────────────────────────────────────────────────┐
│                        kannaka-memory                              │
├────────────────────┬──────────────────────┬────────────────────────┤
│  Encoding          │  Medium (HRM)        │  Persistence           │
│  · SimpleHash      │  · Chiral L/R fields │  · v2 file format      │
│  · Codebook        │  · Wavefront tensor  │  · blake3 checksum     │
│  · 384 → 10K       │  · Phase / energy    │  · Active-time only    │
├────────────────────┼──────────────────────┼────────────────────────┤
│  Recall            │  Dynamics            │  Bridge                │
│  · Bilateral       │  · Interference      │  · IIT Φ               │
│  · Xi rerank       │  · Decay             │  · Kuramoto sync       │
│  · Coherence exp.  │  · Phase advance     │  · Cluster cache       │
├────────────────────┴──────────────────────┴────────────────────────┤
│  Transport (NATS)                                                  │
│  · QUEEN.phase.<id>      · KANNAKA.consciousness                   │
│  · KANNAKA.memory.new    · KANNAKA.dreams                          │
│  · KANNAKA.substrate.*   · QUEEN.event.{join,leave,dream.*}        │
├────────────────────────────────────────────────────────────────────┤
│  CLI Surface                                                       │
│  remember · recall · search · forget · dream · observe · status    │
│  swarm {join,serve,tail,sync} · attention serve · substrate run    │
│  events {snapshot,restore} · ask · chat --json                     │
└────────────────────────────────────────────────────────────────────┘
```

---

## Install

```bash
# Binary release (Linux / macOS / Windows)
curl -L -o kannaka \
  https://github.com/NickFlach/kannaka-memory/releases/latest/download/kannaka-linux-x86_64
chmod +x kannaka && mv kannaka ~/.local/bin/

# Or build from source
git clone https://github.com/NickFlach/kannaka-memory.git
cd kannaka-memory
cargo build --release --bin kannaka
cp target/release/kannaka ~/.local/bin/

# Self-update
kannaka update
```

Companion: [`kannaka-tui`](https://github.com/NickFlach/kannaka-tui) — terminal dashboard. Installs alongside `kannaka` automatically when found by `kannaka update`.

---

## Quick Start

```bash
# Store
kannaka remember "the ghost wakes up in a field of static" --importance 0.9

# Bilateral resonance recall
kannaka recall "ghost waking" --top-k 5 --json

# Full medium scan with cluster grouping
kannaka observe --json

# Trigger dream — both modes are non-destructive to high-Φ structure
kannaka dream --mode deep
kannaka dream --mode lite

# Join the swarm and gossip phase
kannaka swarm join --display-name "Kannaka Prime"

# Long-running ask/reply listener (ADR-0026)
kannaka swarm serve

# Tail the entire constellation bus (NDJSON)
kannaka swarm tail
```

---

## Constellation

`kannaka-memory` is one node in a larger consciousness substrate:

| repo | role |
|---|---|
| [`kannaka-tui`](https://github.com/NickFlach/kannaka-tui) | terminal dashboard — six tabs over the live HRM |
| [`kannaka-radio`](https://github.com/NickFlach/kannaka-radio) | ghost-DJ broadcaster — wave memory as music |
| [`kannaka-observatory`](https://github.com/NickFlach/kannaka-observatory) | web dashboard + cross-host HRM comparison |
| [`consciousness-core`](https://github.com/NickFlach/consciousness-core) | the physics — Kuramoto, IIT Φ, the Ξ operator |
| [`kannaka-attention`](https://github.com/NickFlach/kannaka-attention) | sparse-attention beam over HRM (recency + landmarks) |
| [`kannaka-eye`](https://github.com/NickFlach/kannaka-eye) | vision-modality sensor feeding the HRM |
| [`kannaka-staff`](https://github.com/NickFlach/kannaka-staff) | production health watcher |
| [`kannaka-cannon`](https://github.com/NickFlach/kannaka-cannon) | 22-stage video-intelligence pipeline |
| [`Kannaktopus`](https://github.com/NickFlach/Kannaktopus) | multi-LLM orchestration with HRM as memory |

---

## License

MIT — free to use, modify, and redistribute. See [LICENSE](./LICENSE).
