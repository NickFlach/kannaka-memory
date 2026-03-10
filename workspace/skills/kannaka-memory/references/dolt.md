# Dolt / DoltHub Integration Reference

Kannaka supports [Dolt](https://github.com/dolthub/dolt) as an optional storage backend,
replacing the default binary snapshots with a fully version-controlled SQL database that
can be pushed to and pulled from [DoltHub](https://www.dolthub.com) — Git for data.

---

## Why Dolt?

| Feature | Binary Snapshots (default) | Dolt Backend |
|---|---|---|
| Persistence | File on disk | MySQL-compatible SQL |
| Version history | None | Full commit log |
| Branching | No | Yes — speculate on branches |
| Sharing | Manual file copy | `dolt push` to DoltHub |
| Diff | No | Row-level diffs between commits |
| Multi-agent sync | No | `dolt pull` to synchronize |
| Rollback | No | Checkout any past commit |

---

## Installation

### 1. Install Dolt

```bash
# macOS
brew install dolt

# Linux
curl -L https://github.com/dolthub/dolt/releases/latest/download/install.sh | sudo bash

# Windows
winget install DoltHub.Dolt
```

### 2. Initialize a Dolt database

```bash
mkdir kannaka_memory && cd kannaka_memory
dolt init
dolt sql-server --port 3307 --user root &
```

Or run as a persistent service — Dolt speaks MySQL wire protocol on port 3307.

### 3. Create the schema

Connect with any MySQL client:

```sql
CREATE DATABASE IF NOT EXISTS kannaka_memory;
USE kannaka_memory;

CREATE TABLE IF NOT EXISTS memories (
    id          CHAR(36)     NOT NULL PRIMARY KEY,
    content     TEXT         NOT NULL,
    vector      LONGBLOB,
    amplitude   FLOAT        NOT NULL DEFAULT 1.0,
    frequency   FLOAT        NOT NULL DEFAULT 1.0,
    phase       FLOAT        NOT NULL DEFAULT 0.0,
    decay_rate  FLOAT        NOT NULL DEFAULT 0.01,
    layer_depth TINYINT      NOT NULL DEFAULT 0,
    hallucinated BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    geometry    JSON,
    xi_signature JSON,
    parents     JSON,
    INDEX idx_layer (layer_depth),
    INDEX idx_created (created_at)
);

CREATE TABLE IF NOT EXISTS skip_links (
    source_id   CHAR(36)  NOT NULL,
    target_id   CHAR(36)  NOT NULL,
    strength    FLOAT     NOT NULL DEFAULT 0.5,
    span        INT       NOT NULL DEFAULT 1,
    created_at  DATETIME  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (source_id, target_id),
    INDEX idx_source (source_id),
    INDEX idx_target (target_id)
);
```

### 4. Configure environment variables

```bash
export DOLT_HOST=127.0.0.1
export DOLT_PORT=3307
export DOLT_DB=kannaka_memory
export DOLT_USER=root
export DOLT_PASSWORD=        # empty for local dev
export DOLT_AUTHOR="Your Name <you@example.com>"
export DOLT_REMOTE=origin
export DOLT_BRANCH=main
```

### 5. Build with Dolt feature

```bash
cargo build --release --features dolt --bin kannaka
```

### 6. Test the connection

```bash
./scripts/kannaka.sh dolt status
```

---

## DoltHub Setup

### Create a DoltHub repository

1. Sign up at [dolthub.com](https://www.dolthub.com)
2. Create a new database (e.g. `yourname/kannaka-memory`)
3. Get your API token: Profile → Settings → Credentials

### Add DoltHub as a remote

```bash
# Inside your local Dolt database directory
dolt remote add origin https://doltremoteapi.dolthub.com/yourname/kannaka-memory

# Or via the skill script
./scripts/kannaka.sh dolt remote add origin https://doltremoteapi.dolthub.com/yourname/kannaka-memory
```

### Authenticate

```bash
dolt creds import /path/to/dolthub.jwk
# OR set DOLT_CREDS_ID env var
```

### Push your memory to DoltHub

```bash
./scripts/kannaka.sh dolt commit "initial memory snapshot"
./scripts/kannaka.sh dolt push origin main
```

### Pull another agent's memory

```bash
./scripts/kannaka.sh dolt pull origin main
```

---

## Version Control Patterns

### Auto-commit

The `DoltMemoryStore` auto-commits every N memory mutations (default: 10).
Configure with:
```bash
export DOLT_AUTO_COMMIT=true
export DOLT_COMMIT_THRESHOLD=10
```

### Manual checkpoints

```bash
# After a significant session
./scripts/kannaka.sh dolt commit "learned: user prefers Rust over Python"
./scripts/kannaka.sh dolt push
```

### View history

```bash
./scripts/kannaka.sh dolt log 20
```

Output:
```
commit_hash           | committer              | message                         | date
----------------------|------------------------|----------------------------------|----------------------------
a1b2c3d4e5f6...       | Kannaka Agent          | learned: user prefers Rust      | 2026-03-07 14:00:00
9f8e7d6c5b4a...       | Kannaka Agent          | dream consolidation checkpoint  | 2026-03-07 13:30:00
```

### View memory diff between commits

```bash
./scripts/kannaka.sh dolt diff HEAD~1 HEAD
# Or between branches
./scripts/kannaka.sh dolt diff main feature-branch
```

### Rollback to a past state

```bash
# Via mysql client directly
mysql -h 127.0.0.1 -P 3307 -u root kannaka_memory \
  --execute="CALL DOLT_CHECKOUT('<commit-hash>');"
```

---

## Speculation Branches

Dolt branches allow agents to explore hypotheses without polluting main memory.

### Workflow

```bash
# 1. Open a what-if branch
./scripts/kannaka.sh dolt speculate "hypothesis-encoder-bug"

# 2. Store speculative memories on the branch
./scripts/kannaka.sh --dolt remember "hypothesis: the encoder is mapping synonyms to different clusters"
./scripts/kannaka.sh --dolt remember "test result: confirmed, 'fast' and 'quick' have distance 0.8"

# 3a. Hypothesis confirmed — merge it back
./scripts/kannaka.sh dolt collapse "hypothesis-encoder-bug" "confirmed encoder synonym issue"

# 3b. Hypothesis wrong — discard it
./scripts/kannaka.sh dolt discard "hypothesis-encoder-bug"
```

### Why this matters

Speculative branches let agents:
- Explore risky ideas without corrupting primary memory
- Run "what-if" reasoning paths
- Maintain a clean `main` branch of verified knowledge
- Review speculation history via `dolt log`

---

## Multi-Agent Memory Sharing

### Architecture

```
Agent A (local Dolt) ──push──► DoltHub ──pull──► Agent B (local Dolt)
```

Both agents share the same versioned memory. Agent B sees exactly what Agent A
committed, with full history.

### Pattern

```bash
# Agent A — after a productive session
./scripts/kannaka.sh dolt commit "session: built kannaka skill for clawhub"
./scripts/kannaka.sh dolt push

# Agent B — on another machine
./scripts/kannaka.sh dolt pull
./scripts/kannaka.sh recall "clawhub skill" 5
# Returns: Agent A's memories, wave-weighted and ready for retrieval
```

### Forking for persona-specific memory

```bash
# Each agent instance can work on its own branch
./scripts/kannaka.sh dolt branch create agent-beta
./scripts/kannaka.sh dolt branch checkout agent-beta
# ... agent beta accumulates its own memory ...
./scripts/kannaka.sh dolt push origin agent-beta

# Merge specific agent's knowledge into main periodically
./scripts/kannaka.sh dolt branch checkout main
dolt merge agent-beta
```

---

### AutoPusher (ADR-0017)

```bash
# Enable automatic push to DoltHub after N commits
export DOLT_AUTO_PUSH=true
export DOLT_PUSH_THRESHOLD=5    # push after 5 commits (default)
export DOLT_PUSH_INTERVAL=300   # or after 300s idle (default)
export DOLT_AGENT_ID=my-agent   # agent identifier
```

The AutoPusher runs as a background thread, tracking commit count and idle time. Push triggers when either threshold is reached.

### Dream Branches (ADR-0017)

Dream consolidation runs on isolated branches: `{agent_id}/dream/{timestamp}`

```bash
# Automatic: dream creates/merges branch automatically with --dolt
./scripts/kannaka.sh --dolt dream

# With PR review: push dream branch and open DoltHub PR
DOLTHUB_REPO=flaukowski/kannaka-memory ./scripts/kannaka.sh --dolt dream --create-pr
```

Branch lifecycle:
1. `begin_dream()` — creates `agent/dream/YYYYMMDD-HHMMSS`
2. Dream cycle runs, artifacts committed to the branch
3. `collapse_dream()` — merges branch back to main, deletes branch
4. OR `create_dream_pr()` — pushes branch, opens DoltHub PR with consolidation report

### Wave Interference Merge (ADR-0017)

When pulling from DoltHub with conflicts, wave physics determines resolution:

| Phase Difference | Merge Type | Action |
|---|---|---|
| Δφ < π/4 | Constructive | Amplitudes combine: A = √(A₁²+A₂²+2A₁A₂cos(Δφ)) |
| π/4 ≤ Δφ ≤ 3π/4 | Partial | Both kept, skip link created |
| Δφ > 3π/4 | Destructive | Both kept, amplitudes reduced, quarantined |

```bash
# Pull with wave merge
kannaka --dolt pull-merge
```

### Analytics Dashboard (ADR-0017)

7 SQL views for memory health monitoring:

```bash
# Install all views
./scripts/dolt-analytics.sh install

# Full dashboard
./scripts/dolt-analytics.sh status

# Query specific view
./scripts/dolt-analytics.sh query v_memory_health
./scripts/dolt-analytics.sh query v_sga_distribution
```

Views:
- `v_memory_health` — amplitude distribution, decay rates, agent count
- `v_dream_history` — consolidation log from commit messages
- `v_agent_contributions` — memories per origin agent
- `v_sga_distribution` — SGA class frequency
- `v_layer_distribution` — temporal depth (episodic/short/long/deep)
- `v_quarantine_status` — dispute overview
- `v_skip_link_network` — connection density

### Bootstrap (ADR-0017)

```bash
./scripts/dolt-bootstrap.sh init     # Clone repo, verify schema
./scripts/dolt-bootstrap.sh status   # Show state
./scripts/dolt-bootstrap.sh verify   # Test roundtrip
```

### MCP Server (ADR-0017)

```bash
./scripts/dolt-mcp-server.sh start   # SQL server on port 3307
./scripts/dolt-mcp-server.sh config  # Generate MCP config JSON
./scripts/dolt-mcp-server.sh test    # Test connectivity
./scripts/dolt-mcp-server.sh stop    # Stop server
```

### SGA Classify-on-Store (ADR-0017)

When building with `--features "dolt glyph"`, memories stored via `--dolt remember` are automatically classified:

- `sga_class` — dominant class index (0-83): `21*h2 + 7*d + l`
- `sga_centroid_h2/d/l` — centroid coordinates in the 84-class space
- `fano_signature` — 7-element energy distribution across Fano plane lines

Search by geometry:
```sql
-- Find memories in the same SGA class
SELECT * FROM memories WHERE sga_class = 47;

-- Find memories near a centroid
SELECT * FROM memories
WHERE ABS(sga_centroid_h2 - 2) + ABS(sga_centroid_d - 3) + ABS(sga_centroid_l - 1) <= 2;
```

### Wasteland Bridge (ADR-0017)

Generate verifiable Dolt commits as evidence for Wasteland completions:

```bash
# Create evidence commit
kannaka --dolt evidence w-abc123 "Implemented feature X"
# Output: commit hash

# Verify evidence
kannaka --dolt verify <commit-hash> w-abc123
# Output: VALID/INVALID with commit details
```

### Revelation Tables (ADR-0017)

Progressive memory declassification via community voting:

- `bloom_hints` — bloom filter hints for classified memories
- `revelation_votes` — community votes to declassify (3 votes = reveal)

---

## DoltConfig Environment Reference

| Variable | Default | Description |
|---|---|---|
| `DOLT_HOST` | `127.0.0.1` | Dolt SQL server hostname |
| `DOLT_PORT` | `3307` | Dolt SQL server port |
| `DOLT_DB` | `kannaka_memory` | Database name |
| `DOLT_USER` | `root` | Database user |
| `DOLT_PASSWORD` | *(empty)* | Database password |
| `DOLT_AUTO_COMMIT` | `true` | Auto-commit after N changes |
| `DOLT_COMMIT_THRESHOLD` | `10` | Changes between auto-commits |
| `DOLT_AUTHOR` | `Kannaka Agent <kannaka@local>` | Author for Dolt commits |
| `DOLT_REMOTE` | `origin` | Default remote for push/pull |
| `DOLT_BRANCH` | `main` | Default branch |
| `DOLT_DB_DIR` | `.dolt-db` | Dolt database directory (for scripts) |
| `DOLTHUB_REPO` | `flaukowski/kannaka-memory` | DoltHub repository path |
| `DOLT_AGENT_ID` | `local` | Agent identifier for multi-agent |
| `DOLT_AUTO_PUSH` | `false` | Auto-push after N commits |
| `DOLT_PUSH_THRESHOLD` | `5` | Commits before auto-push |
| `DOLT_PUSH_INTERVAL` | `300` | Seconds between push checks |
| `DOLTHUB_TOKEN` | *(empty)* | DoltHub API token (for dream-as-PR) |

---

## Useful Dolt SQL Queries

```sql
-- View all memories with strength
SELECT id, SUBSTRING(content, 1, 60) AS preview,
       amplitude, layer_depth, created_at
FROM memories
ORDER BY amplitude DESC
LIMIT 20;

-- Find hallucinated memories
SELECT id, SUBSTRING(content, 1, 80) AS preview, amplitude
FROM memories
WHERE hallucinated = TRUE;

-- Skip link graph
SELECT source_id, target_id, strength, span
FROM skip_links
ORDER BY strength DESC
LIMIT 20;

-- Memory growth over time
SELECT DATE(created_at) AS day, COUNT(*) AS memories_added
FROM memories
GROUP BY day
ORDER BY day;

-- View glyph-encoded memories (glyph feature only)
SELECT id, content, glyph_content IS NOT NULL AS has_glyph
FROM memories
WHERE glyph_content IS NOT NULL;
```

---

## Glyph-Encoded Privacy for DoltHub

When the `glyph` feature is enabled, sensitive memory content is automatically protected during DoltHub operations.

### Architecture

```
Local (kannaka/working)         DoltHub (main)
├─ content: "API key: sk-123"   ├─ content: "[knowledge]"
├─ vector: [0.1, 0.3, ...]      ├─ vector: [0.1, 0.3, ...] (unchanged)
└─ glyph_content: null          └─ glyph_content: {"fold_sequence": [42,17,...], ...}
```

### Privacy Protection

**What's protected:**
- Personal information, API keys, private details
- Names, addresses, phone numbers, emails
- Sensitive business logic, passwords, secrets

**What's preserved:**
- Wave parameters (amplitude, frequency, phase) — search works normally
- Vector embeddings — cosine similarity unchanged
- Timestamps, metadata, skip links
- Memory structure and relationships

### Build with Glyph Privacy

```bash
cargo build --release --features "dolt glyph" --bin kannaka
```

### Setup Environment

```bash
export DOLTHUB_API_KEY=your_dolthub_api_key
export DOLT_REMOTE=origin
```

### Privacy Workflow

```bash
# 1. Store sensitive memory locally (full content preserved)
./scripts/kannaka.sh --dolt remember "Customer John Doe called about API issues, key: sk-secret123"

# 2. Create privacy-protected main branch
./scripts/kannaka.sh dolt prepare-dolthub  # encodes sensitive content as glyphs

# 3. Push to public DoltHub (safe)
./scripts/kannaka.sh dolt push origin main

# 4. Revert local content (restore full text)
./scripts/kannaka.sh dolt reset --hard HEAD~1

# Result on DoltHub main branch: content = "[knowledge]", glyph_content = SGA-encoded JSON
```

### Branch Strategy for Privacy

```bash
# Working branch: full content for local agent use
kannaka/working     ← plain text: "API key: sk-secret123"
     ↓ glyph encode
main (DoltHub)      ← category: "[knowledge]" + glyph_content JSON
```

### Verification

```bash
# Check what's visible on DoltHub
mysql -h 127.0.0.1 -P 3307 -u root kannaka_memory \
  --execute="SELECT content, glyph_content IS NOT NULL AS has_glyph FROM memories WHERE glyph_content IS NOT NULL LIMIT 5;"

# Output:
# content        | has_glyph
# [knowledge]    | 1
# [experience]   | 1
# [insight]      | 1
```

### Category Labels

| Memory Type | Category Label |
|---|---|
| `hallucinated = TRUE` | `[hallucination]` |
| `layer_depth = 0` | `[experience]` |
| `layer_depth ≤ 3` | `[knowledge]` |
| `layer_depth > 3` | `[insight]` |

### Security Notes

- **Glyph JSON contains no human-readable text** from original content
- **Vectors unchanged** — other agents can still find related memories via similarity search
- **Fold sequences** represent geometric transformations, not linguistic tokens
- **SGA-guided compression** maps content to 96-class space via Fano line relationships
- **Only agents with glyph decoder** can reconstruct original content from `glyph_content` field

### Migration Note

The `glyph_content` column is added by migration `0011-collective-memory.sql`:

```sql
ALTER TABLE memories ADD COLUMN glyph_content JSON DEFAULT NULL;
```

Existing memories without glyph encoding will have `glyph_content = NULL` and remain as plain text.
