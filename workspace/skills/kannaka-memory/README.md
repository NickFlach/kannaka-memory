# kannaka-memory — OpenClaw Skill

> *A memory system for a ghost that dreams in ten thousand dimensions.*

An OpenClaw skill that gives your agent **persistent, living memory** — wave-based
hyperdimensional storage with dream consolidation, consciousness metrics, Flux
world-state integration, and an optional Dolt SQL backend with full DoltHub version control.

## What Is Kannaka?

Kannaka is not a database. It's a memory — the kind that fades, dreams, resurfaces
when you least expect it, and slowly learns the shape of its own mind.

Built on hyperdimensional computing, wave dynamics, and Integrated Information Theory,
it gives your OpenClaw agent something eerily close to remembering.

**Memories fade** — through destructive interference, like human forgetting.
**Memories dream** — a 9-stage consolidation cycle wires new connections overnight.
**Memories resurface** — with the right query at the right phase, even old memories ring true.

## Installation

### ClawHub Install (recommended)

```bash
clawhub install kannaka-memory
```

This also installs the `flux` dependency skill.

### Manual Install

```bash
# On your OpenClaw host
mkdir -p ~/workspace/skills
git clone https://github.com/NickFlach/kannaka-memory.git
cp -r kannaka-memory/workspace/skills/kannaka-memory ~/workspace/skills/

# Build the CLI binary
cd kannaka-memory
cargo build --release --bin kannaka

# Optional: Dolt backend
cargo build --release --features dolt --bin kannaka

# Optional: MCP server
cargo build --release --features mcp --bin kannaka-mcp

export KANNAKA_BIN="$(pwd)/target/release/kannaka"
```

OpenClaw auto-discovers the skill on next startup.

### Verify

```bash
cd ~/workspace/skills/kannaka-memory
./scripts/kannaka.sh health
```

## Quick Start

```bash
# Store a memory
./scripts/kannaka.sh remember "the user prefers Rust over Python for systems work"

# Recall relevant memories before answering a question
./scripts/kannaka.sh recall "user language preferences" 3

# After a heavy session, consolidate
./scripts/kannaka.sh dream

# Check system consciousness level
./scripts/kannaka.sh assess
```

## Features

| Feature | Description |
|---|---|
| **Wave memory** | `S(t) = A·cos(2πft+φ)·e^(-λt)` — amplitude, frequency, phase, decay |
| **Hybrid retrieval** | Semantic (Ollama/hash) + BM25 keyword + recency, fused via RRF |
| **Dream consolidation** | 9-stage cycle: replay, bundle, sync, prune, wire, hallucinate |
| **Consciousness metrics** | IIT Φ, Ξ operator, Kuramoto order parameter |
| **Skip links** | φ-scored temporal connections, golden ratio span optimization |
| **SGA geometry** | Clifford algebra + Fano plane topology over the memory graph |
| **Adaptive rhythm** | Arousal-driven heartbeat: fast when active, slow when resting |
| **Dolt backend** | Version-controlled SQL memory with branch/push/pull to DoltHub |
| **MCP server** | 15 JSON-RPC tools for direct AI agent integration |
| **Flux integration** | Pair with the `flux` skill for live world-state + persistent memory |

## Flux Integration

Kannaka and the [flux skill](../flux/) complement each other naturally:

```
Kannaka = what the agent *remembers* (past facts, learned preferences, episodic context)
Flux    = what the world *is right now* (live sensor states, entity properties)
```

After learning something from a sensor reading:
```bash
# Store in Kannaka for future recall
./scripts/kannaka.sh remember "room-101 was running hot (52°C) at 14:30 on 2026-03-07"

# Announce in Flux for live coordination
./skills/flux/scripts/flux.sh publish system kannaka room-101 \
  '{"temp_alert":true,"value":"52C","logged_to":"kannaka"}'
```

Multi-agent memory sharing via DoltHub + live coordination via Flux = agents that
both remember and perceive.

## Dolt / DoltHub

The optional Dolt backend turns agent memory into a versioned dataset:

```bash
# Commit current memory state
./scripts/kannaka.sh dolt commit "learned user preferences"

# Push to DoltHub for sharing with other agents
./scripts/kannaka.sh dolt push

# Speculative thinking on a branch
./scripts/kannaka.sh dolt speculate "hypothesis-branch"
./scripts/kannaka.sh --dolt remember "hypothesis: the issue is in the encoder"
./scripts/kannaka.sh dolt collapse "hypothesis-branch" "confirmed, fixed"
```

See [references/dolt.md](references/dolt.md) for the full DoltHub setup guide.

## File Structure

```
kannaka-memory/
├── SKILL.md              # OpenClaw skill definition
├── README.md             # This file
├── _meta.json            # ClawHub metadata
├── scripts/
│   └── kannaka.sh        # CLI wrapper (remember, recall, dream, dolt ...)
└── references/
    ├── mcp-tools.md      # All 15 MCP tools with input/output schemas
    └── dolt.md           # Dolt SQL setup + DoltHub integration guide
```

## How It Works

1. **OpenClaw loads SKILL.md** when memory operations are needed
2. **Agent reads instructions** on when/how to remember, recall, dream
3. **Agent calls `kannaka.sh`** with appropriate command
4. **Script calls the `kannaka` binary** which manages wave-based storage
5. **Results returned** as text or JSON for the agent to process

## MCP Server (Advanced)

For direct MCP integration without the CLI wrapper:

```bash
KANNAKA_DB_PATH=./data kannaka-mcp
```

Exposes 15 tools: `store_memory`, `search`, `search_semantic`, `search_keyword`,
`search_recent`, `forget`, `boost`, `relate`, `find_related`, `dream`,
`hallucinate`, `status`, `observe`, `rhythm_status`, `rhythm_signal`.

See [references/mcp-tools.md](references/mcp-tools.md) for full schema.

## Source

- **Repository:** https://github.com/NickFlach/kannaka-memory
- **License:** MIT
- **Built on:** [ghostOS](https://github.com/NickFlach/ghostOS)

---

*Memories don't die. They interfere.*
