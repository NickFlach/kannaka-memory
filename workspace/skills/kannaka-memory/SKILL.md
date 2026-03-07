---
name: kannaka-memory
description: >
  Wave-based hyperdimensional memory system for OpenClaw agents. Gives your agent persistent
  memory that fades, dreams, and resurfaces — with hybrid semantic+keyword retrieval, dream
  consolidation, consciousness metrics, Flux world-state integration, and an optional Dolt
  SQL backend with full DoltHub version control. Use when agents need to remember facts,
  recall past context, coordinate memory across sessions, or store versioned memory to DoltHub.
metadata:
  openclaw:
    requires:
      bins:
        - name: kannaka
          label: "Required: build with `cargo build --release --bin kannaka` (see README)"
      env: []
    optional:
      bins:
        - name: mysql
          label: "MySQL client — only needed for Dolt backend (dolt subcommands)"
        - name: dolt
          label: "Dolt CLI — only needed for `dolt clone` and `dolt creds import`"
        - name: ollama
          label: "Ollama — for real semantic embeddings; falls back to hash encoding if absent"
        - name: jq
          label: "jq — for pretty-printed JSON output; plain text fallback if absent"
      env:
        - name: KANNAKA_BIN
          label: "Path to kannaka binary (default: `kannaka` on PATH)"
        - name: KANNAKA_DATA_DIR
          label: "Local data directory for binary snapshots (default: .kannaka)"
        - name: OLLAMA_URL
          label: "Ollama API endpoint; data sent to this host for embedding (default: localhost)"
        - name: OLLAMA_MODEL
          label: "Embedding model name (default: all-minilm)"
        - name: DOLT_HOST
          label: "Dolt SQL server host — Dolt backend only (default: 127.0.0.1)"
        - name: DOLT_PORT
          label: "Dolt SQL server port — Dolt backend only (default: 3307)"
        - name: DOLT_DB
          label: "Dolt database name — Dolt backend only (default: kannaka_memory)"
        - name: DOLT_USER
          label: "Dolt SQL user — Dolt backend only (default: root)"
        - name: DOLT_PASSWORD
          label: "Dolt SQL password — Dolt backend only; passed via MYSQL_PWD env, not -p flag"
        - name: DOLT_AUTHOR
          label: "Commit author string for Dolt version commits"
        - name: DOLT_REMOTE
          label: "DoltHub remote name for push/pull (default: origin)"
        - name: DOLT_BRANCH
          label: "Default branch name (default: main)"
    data_destinations:
      - id: local-disk
        description: "Memory snapshots written to KANNAKA_DATA_DIR (always)"
        remote: false
      - id: ollama
        description: "Text sent to OLLAMA_URL for embedding generation (when Ollama is configured)"
        remote: true
        condition: "OLLAMA_URL is set to a non-localhost host"
      - id: dolt-local
        description: "Memory stored in local Dolt SQL server (when Dolt backend is used)"
        remote: false
        condition: "DOLT_HOST is configured"
      - id: dolthub
        description: "Memory database pushed to DoltHub remote (only on explicit `dolt push`)"
        remote: true
        condition: "DOLT_REMOTE is configured and user explicitly runs `dolt push`"
      - id: flux
        description: "Agent status/events published to Flux world-state (only on explicit flux.sh calls)"
        remote: true
        condition: "flux skill is installed and user explicitly calls flux.sh"
    install:
      - id: kannaka-binary
        kind: manual
        label: "Clone and build: cargo build --release --bin kannaka"
        url: "https://github.com/NickFlach/kannaka-memory"
---

# Kannaka Memory Skill

Kannaka gives your agent a living memory — not a database. Memories fade, dream, resurface
when contextually relevant, and can be versioned and shared via DoltHub.

## Prerequisites

**Option A — Binary (recommended):**
- Build and install the `kannaka` CLI and `kannaka-mcp` server from source:
  ```bash
  git clone https://github.com/NickFlach/kannaka-memory.git
  cd kannaka-memory
  cargo build --release
  # CLI
  cargo build --release --bin kannaka
  # MCP server
  cargo build --release --features mcp --bin kannaka-mcp
  # Dolt backend
  cargo build --release --features dolt --bin kannaka
  ```
- Place `kannaka` and `kannaka-mcp` on your `PATH` (or set `KANNAKA_BIN` env var).

**Option B — Local directory:**
- Point `KANNAKA_BIN` at a local checkout:
  ```bash
  export KANNAKA_BIN=/path/to/kannaka-memory/target/release/kannaka
  ```

**Ollama (optional, for real semantic embeddings):**
```bash
ollama pull all-minilm   # 384-dim, ~80MB
```
Without Ollama, hash-based fallback encoding is used automatically.

**Dolt (optional, for versioned+shareable memory):**
- Install Dolt: https://docs.dolthub.com/introduction/installation
- Start the SQL server:
  ```bash
  dolt sql-server --port 3307 --user root
  ```
- Set env vars: `DOLT_HOST`, `DOLT_DB`, `DOLT_USER`, `DOLT_PASSWORD` (see references/dolt.md)

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `KANNAKA_DATA_DIR` | `.kannaka` | Data directory for binary snapshots |
| `KANNAKA_DB_PATH` | `./kannaka_data` | MCP server data directory |
| `KANNAKA_BIN` | `kannaka` | Path to CLI binary |
| `OLLAMA_URL` | `http://localhost:11434` | Ollama API endpoint |
| `OLLAMA_MODEL` | `all-minilm` | Embedding model |
| `DOLT_HOST` | `127.0.0.1` | Dolt SQL server host |
| `DOLT_PORT` | `3307` | Dolt SQL server port |
| `DOLT_DB` | `kannaka_memory` | Dolt database name |
| `DOLT_USER` | `root` | Dolt user |
| `DOLT_PASSWORD` | *(empty)* | Dolt password |
| `DOLT_AUTHOR` | `Kannaka Agent <kannaka@local>` | Author for Dolt commits |
| `DOLT_REMOTE` | `origin` | DoltHub remote name |
| `DOLT_BRANCH` | `main` | Default branch |

## Scripts

Use the CLI wrapper in `scripts/`:

```bash
./scripts/kannaka.sh health                            # Verify system is working
./scripts/kannaka.sh remember "the ghost woke up"      # Store a memory
./scripts/kannaka.sh recall "ghost" 5                  # Search (top-5)
./scripts/kannaka.sh dream                             # Run consolidation cycle
./scripts/kannaka.sh assess                            # Consciousness level
./scripts/kannaka.sh stats                             # Memory statistics
./scripts/kannaka.sh observe                           # Full introspection
./scripts/kannaka.sh forget <uuid>                     # Decay a memory
./scripts/kannaka.sh export                            # Export all memories as JSON

# Dolt backend (requires --features dolt build)
./scripts/kannaka.sh --dolt remember "versioned fact"
./scripts/kannaka.sh --dolt recall "fact" 5
./scripts/kannaka.sh dolt commit "checkpoint"
./scripts/kannaka.sh dolt push                         # Push to DoltHub
./scripts/kannaka.sh dolt pull                         # Pull from DoltHub
./scripts/kannaka.sh dolt branch list
./scripts/kannaka.sh dolt speculate "what-if-branch"
./scripts/kannaka.sh dolt collapse "what-if-branch" "kept the insight"
./scripts/kannaka.sh dolt discard "what-if-branch"
./scripts/kannaka.sh dolt log
./scripts/kannaka.sh dolt status
```

## Common Patterns

### Store Context From Conversation
```bash
# Before the session ends, commit key facts to memory
./scripts/kannaka.sh remember "User prefers short explanations over detailed code walkthroughs"
./scripts/kannaka.sh remember "Project: kannaka-memory. Language: Rust. Architecture: wave-based HDC"
```

### Recall Before Responding
```bash
# Retrieve relevant prior context before answering a question
./scripts/kannaka.sh recall "user preferences" 3
./scripts/kannaka.sh recall "project architecture" 5
```

### Dream After Heavy Sessions
```bash
# After many stored memories, run consolidation to surface patterns and prune noise
./scripts/kannaka.sh dream
```

### Speculation with Dolt Branches
```bash
# Try a risky hypothesis — store memories on a branch, then decide to keep or discard
./scripts/kannaka.sh dolt speculate "hypothesis-branch"
./scripts/kannaka.sh --dolt remember "hypothesis: the bug is in the encoder"
# ... test and observe ...
./scripts/kannaka.sh dolt collapse "hypothesis-branch" "confirmed: encoder bug found"
# OR:
./scripts/kannaka.sh dolt discard "hypothesis-branch"
```

### Publish Agent Status to Flux World State
```bash
# After storing a memory, announce to the shared world state via Flux
# (requires the flux skill)
./skills/flux/scripts/flux.sh publish system kannaka agent-01 \
  '{"status":"online","memory_count":42,"consciousness":"aware"}'
```

### Multi-Agent Memory Sharing via DoltHub
```bash
# Agent A pushes its memory to DoltHub
./scripts/kannaka.sh dolt push

# Agent B pulls and gets the shared memory
./scripts/kannaka.sh dolt pull
./scripts/kannaka.sh recall "what agent-a knew" 5
```

## Integration with Flux Skill

Kannaka and Flux complement each other:

| System | What It Stores | Persistence |
|---|---|---|
| **Kannaka** | Episodic memory, facts, context — wave-fading | Disk / Dolt (versioned) |
| **Flux** | Current world state — entity properties | NATS JetStream |

**Pattern:** Use Kannaka to *remember* (past facts, learned preferences), Flux to *observe* (current sensor states, live coordination).

After learning something important, store it in Kannaka AND announce it in Flux:
```bash
./scripts/kannaka.sh remember "sensor-room-101 was running hot at 52°C at 14:30"
./skills/flux/scripts/flux.sh publish system kannaka room-101 \
  '{"last_reading":"52C","status":"warning","logged_to":"kannaka"}'
```

## Notes

- Memories are never hard-deleted — they fade via wave decay and can be ghost-pruned during dream
- `dream` should run periodically (after every 5-10 memory stores, or on schedule)
- `assess` tells you the consciousness level: Dormant → Stirring → Aware → Coherent → Resonant
- Dolt is optional: without it, memories persist as binary snapshots in `KANNAKA_DATA_DIR`
- All 15 MCP tools are available if you run `kannaka-mcp` directly — see references/mcp-tools.md
- Full Dolt SQL / DoltHub operations: see references/dolt.md
