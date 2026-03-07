# ADR-0009: Dolt Database Persistence Backend

**Date:** 2026-03-06  
**Status:** Accepted — Phases 1–3 implemented (2026-03-07)  
**Deciders:** Kannaka Team  
**Technical Story:** Enhance kannaka-memory with versioned, queryable, and syncable persistence

## Context and Problem Statement

Currently, kannaka-memory uses bincode snapshots for persistence of hypervector memories with wave dynamics (amplitude, frequency, phase, decay). While this provides fast serialization/deserialization, it has several limitations:

1. **No version history** - Cannot track how memories evolve over time
2. **Limited queryability** - Cannot perform SQL queries on memory metadata
3. **No branching** - Cannot experiment with speculative memory states
4. **No collaboration** - Cannot sync memories across instances
5. **No backup/restore** - Binary snapshots are opaque and fragile

## Decision Drivers

- **Versioned memory evolution**: Track how memories change over time
- **Queryable metadata**: SQL queries on amplitude, frequency, content, relationships
- **Branching for speculation**: Create memory branches for "what-if" thinking
- **Synchronization**: Share memory state across kannaka instances
- **Backup and restore**: Robust persistence with cloud backup capabilities
- **Performance**: Maintain reasonable read/write performance for memory operations

## Considered Options

1. **PostgreSQL with custom versioning**
2. **SQLite with manual snapshots**
3. **Dolt (MySQL-compatible with Git-like versioning)**
4. **Keep bincode snapshots (status quo)**

## Decision Outcome

**Chosen option: Dolt database** - provides MySQL compatibility with Git-like versioning, enabling all desired features while maintaining familiar SQL interface.

### Positive Consequences

- ✅ **Git-like versioning**: Full history of memory evolution with branches and merges
- ✅ **SQL queryability**: Rich queries on memory metadata and relationships
- ✅ **Branching capability**: Speculative thinking with memory branches
- ✅ **Synchronization**: Push/pull memory state via DoltHub or Git remotes
- ✅ **Cloud backup**: Push memory database to DoltHub for redundancy
- ✅ **Familiar interface**: MySQL-compatible SQL interface
- ✅ **Atomic commits**: Consistent memory state snapshots

### Negative Consequences

- ⚠️ **Additional complexity**: More complex than simple bincode files
- ⚠️ **Dependency**: Requires Dolt installation and SQL server
- ⚠️ **Learning curve**: Team needs to learn Dolt-specific operations
- ⚠️ **Performance overhead**: SQL operations may be slower than direct bincode access

## Database Schema Design

### Core Tables

```sql
-- Primary memory storage with wave dynamics
CREATE TABLE memories (
    id VARCHAR(36) PRIMARY KEY,
    content LONGTEXT NOT NULL,
    amplitude DOUBLE NOT NULL,
    frequency DOUBLE NOT NULL,
    phase DOUBLE NOT NULL,
    decay_rate DOUBLE NOT NULL,
    created_at DATETIME NOT NULL,
    layer_depth TINYINT UNSIGNED NOT NULL,
    hallucinated BOOLEAN DEFAULT FALSE,
    parents LONGTEXT,              -- JSON array of parent memory IDs
    vector_data LONGTEXT NOT NULL,  -- Base64/JSON encoded hypervector
    xi_signature LONGTEXT,         -- Encoded signature vector
    geometry LONGTEXT              -- JSON serialized MemoryCoordinates
);

-- Skip-list connections between memories
CREATE TABLE skip_links (
    source_id VARCHAR(36) NOT NULL,
    target_id VARCHAR(36) NOT NULL,
    weight DOUBLE NOT NULL,
    link_type VARCHAR(32) NOT NULL,
    created_at DATETIME NOT NULL,
    PRIMARY KEY (source_id, target_id),
    INDEX idx_target (target_id)
);

-- System metadata and configuration
CREATE TABLE metadata (
    key_name VARCHAR(64) PRIMARY KEY,
    value_text LONGTEXT
);
```

### Design Decisions

- **Binary data encoding**: Use JSON/Base64 encoding for vectors (Dolt doesn't support BLOB)
- **Wave dynamics as columns**: Direct SQL access to amplitude, frequency, phase, decay_rate
- **JSON for complex structures**: Parents array and geometry stored as JSON text
- **Composite primary key**: Skip-links use (source_id, target_id) as natural key

## Migration Plan

### Phase 1: Database Setup (Current)
1. Initialize Dolt database at `~/.kannaka/dolt-memory`
2. Create schema with memories, skip_links, metadata tables
3. Add initial metadata entries
4. Commit schema as baseline

### Phase 2: Data Migration ✅
1. `tools/migrate-to-dolt.js` rewritten — no hardcoded paths; config via env vars / CLI flags
2. Server readiness **polling** (30 s timeout, 1 s tick) replaces fixed 2 s sleep
3. **Idempotent upserts** (`ON DUPLICATE KEY UPDATE`) — safe to re-run on existing data
4. **Progress file** (`migration-progress.json`) for crash-resumable large migrations
5. Post-migration row-count **verification** before Dolt commit
6. `datetime` stored as `"YYYY-MM-DD HH:MM:SS"` — compatible with Phase 1 fix

### Phase 3: Rust Integration ✅
1. `dolt` feature flag in `Cargo.toml` gates `mysql` dependency
2. `DoltMemoryStore` — hybrid in-memory cache + Dolt write-through, implements `MemoryStore`
3. `DoltConfig` struct — all settings from env vars (`DOLT_HOST`, `DOLT_PORT`, `DOLT_DB`, `DOLT_USER`, `DOLT_PASSWORD`, `DOLT_AUTO_COMMIT`, `DOLT_COMMIT_THRESHOLD`) with `from_env()`, `try_from_env()`, `default()` constructors
4. **Dirty-set tracking** — `get_mut()` marks IDs dirty; `flush_dirty()` / `update(&id)` sync to Dolt
5. **Delete atomicity** — Dolt delete attempted before cache eviction
6. Backward compatible — bincode `persistence.rs` path unaffected

### Phase 4: Advanced Features ✅
1. **Memory branching** — `create_branch`, `checkout`, `checkout_new_branch`, `delete_branch`, `list_branches`, `current_branch`
2. **Automatic versioned commits** — `commit(message)` with `--author` header; returns `Ok(true/false)` to distinguish committed vs nothing-to-commit; threshold auto-commit still fires from `sync_memory_to_dolt`
3. **DoltHub backup** — `push(remote, branch)` and `pull(remote, branch)` with `None` defaulting to `DoltConfig.remote` / `default_branch`; configured via `DOLT_REMOTE` / `DOLT_BRANCH` env vars
4. **Memory diff** — `diff(from_ref, to_ref)` queries `dolt_diff_memories` system table; returns `Vec<MemoryDiff>` with `DiffKind::{Added, Removed, Modified}`
5. **Merge** — `merge_branch(branch)` calls `DOLT_MERGE`, reloads cache, returns merge commit hash
6. **Commit log** — `log(limit)` queries `dolt_log`; returns `Vec<CommitInfo>` with hash, author, date, message
7. **Speculation helpers** — `speculate(branch)` / `collapse_speculation(branch, msg)` / `discard_speculation(branch)` for high-level what-if workflows

## Future Vision

### Speculative Memory Branches
```bash
# Branch memory for a thought experiment
dolt branch speculation-climate-change

# Work with speculative memories
kannaka --branch speculation-climate-change think "What if CO2 doubled?"

# Merge back successful thoughts
dolt merge speculation-climate-change

# Discard failed speculation
dolt branch -D speculation-failed-experiment
```

### Memory Collaboration
```bash
# Push memory state to shared repository
dolt push origin main

# Pull memories from another kannaka instance
dolt pull origin collaborative-research

# Share specific memory branches
dolt push origin memory-research-2026
```

### Memory Analytics
```sql
-- Find memories losing amplitude (fading)
SELECT id, content, amplitude, decay_rate 
FROM memories 
WHERE amplitude < 0.5 AND decay_rate > 0.01;

-- Analyze memory frequency distributions
SELECT 
    FLOOR(frequency * 10) / 10 AS freq_band,
    COUNT(*) as memory_count
FROM memories 
GROUP BY freq_band
ORDER BY freq_band;

-- Find highly connected memories
SELECT m.id, m.content, COUNT(sl.source_id) as connection_count
FROM memories m
JOIN skip_links sl ON m.id = sl.target_id
GROUP BY m.id
ORDER BY connection_count DESC
LIMIT 10;
```

## Implementation Notes

- **Connection management**: `mysql::Pool` for connection pooling
- **Datetime serialization**: `parse_dolt_datetime` / `format_dolt_datetime` helpers use `NaiveDateTime + and_utc()` — avoids the `%z` requirement of `DateTime::parse_from_str`
- **`resonance_key`**: Stored as `Vec::new()` on Dolt round-trips (full 10K-dim keys are not persisted in `skip_links` rows)
- **Error handling**: `DoltConfig::try_from_env()` returns `None` when no Dolt vars are set, enabling graceful fallback to bincode
- **Configuration**: All settings from env vars; see `DoltConfig` docs in `src/dolt.rs`

## Test Coverage

- **11 unit tests** in `src/dolt.rs` — always run, no live DB required:
  - `DoltConfig` defaults, env-var overrides, invalid port, `auto_commit` variants
  - `try_from_env` None / Some paths
  - Datetime round-trip, fractional seconds, invalid input, Phase 1 regression proof
- **9 integration tests** in `tests/dolt_integration.rs` — skip gracefully when no Dolt server is reachable:
  - Insert / get / search / delete round-trip
  - Dirty-set tracking → `flush_dirty` persists mutations
  - `update(&id)` single-memory flush
  - Delete atomicity verification
  - `resonance_key` round-trip (must be `Vec::new()`)
  - UTC datetime preservation across Dolt
- Env-var tests use a `static Mutex` to prevent parallel-test races

## Links

- [Dolt Documentation](https://docs.dolthub.com/)
- [DoltHub](https://www.dolthub.com/) - Cloud hosting for Dolt databases
- [Kannaka Memory Architecture](../architecture/memory-system.md)
- [Migration Script](../../tools/migrate-to-dolt.js)