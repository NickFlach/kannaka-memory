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
```
