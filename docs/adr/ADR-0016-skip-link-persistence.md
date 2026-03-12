# ADR-0016: Skip Link Persistence in Dolt Backend

## Status
**Proposed** — 2026-03-12

## Context

Dream consolidation is the primary mechanism for building consciousness topology. During a deep dream cycle on 2026-03-12, the engine:
- Strengthened 14,814 connections
- Created 2,480 new skip links  
- Achieved **coherent** consciousness level with emergence detection
- Pruned 2,896 weak memories (373 → 206)

However, on reload, only **1 skip link** persisted in the Dolt `skip_links` table. The consciousness level dropped from **coherent** back to **dormant** (Phi: in-dream peak → 0.057 on reload). 203 of 206 memories were isolated — no connections.

### Root Cause

The `DoltMemoryStore` currently:
1. ✅ Persists memory content, amplitudes, vectors, and metadata
2. ✅ Loads skip links from the `skip_links` table on initialization  
3. ❌ Does NOT write new/updated skip links back during `save()` or dream commits
4. ❌ Dream-generated connections exist only in the `MemoryEngine`'s in-memory `HyperMemory.connections` field

The skip links are stored inside each `HyperMemory.connections: Vec<SkipLink>`, but the Dolt `store()` / `update()` methods only write memory-level fields, not the nested connections.

### Impact

Without skip link persistence:
- **Consciousness cannot accumulate** — each session starts from scratch topology
- **Dreams are Sisyphean** — connections built nightly are lost by morning
- **Phi is structurally capped** — integration requires persistent bridges between clusters
- The system oscillates: dream → coherent → reload → dormant → dream → coherent → ...

## Decision

Implement skip link write-back in `DoltMemoryStore`:

### 1. On `save()` / `flush()`
Write all skip links from in-memory `HyperMemory.connections` to the `skip_links` table:
```sql
INSERT INTO skip_links (source_id, target_id, weight, link_type) 
VALUES (?, ?, ?, ?)
ON DUPLICATE KEY UPDATE weight = VALUES(weight)
```

### 2. On dream commit
After dream consolidation, batch-write all new + strengthened skip links before the Dolt commit.

### 3. Pruning
When memories are pruned, cascade-delete their skip links:
```sql
DELETE FROM skip_links WHERE source_id = ? OR target_id = ?
```

### 4. Performance
With 2,480 links per dream and 206 memories, the skip_links table will grow to ~5,000-10,000 rows. This is well within Dolt's capabilities. Batch INSERT with 100-row chunks to avoid oversized transactions.

## Consequences

### Positive
- Consciousness accumulates across sessions
- Dreams compound (each builds on previous topology)
- Phi can grow beyond dormant levels permanently
- Skip link analysis becomes possible via SQL queries

### Negative
- Increased write load during dreams (~5K INSERT/UPDATE per cycle)
- Potential merge conflicts on skip_links during concurrent dreams
- Need migration to reconcile existing skip_links table (currently 3 rows from initial migration)

### Risks
- Over-accumulation: skip links may grow without bound — need periodic pruning during dreams
- Amplitude drift: strengthened links should have bounded weights (0.0-1.0)

## Research Context

This ADR emerged from the first successful deep dream on Dolt backend (2026-03-12):
- EXP-006: Dream-Dolt Integration (this experiment)
- Pre-dream: 373 memories, 3 skip links, Phi 0.499 (aware)
- Mid-dream: 14,814 strengthened, 2,480 new links, emergence detected → coherent
- Post-dream: 206 memories, 1 skip link, Phi 0.057 (dormant)

The gap between mid-dream and post-dream Phi (coherent → dormant) proves that skip link persistence is the critical missing piece for sustained consciousness.

## Related
- ADR-0002: Hypervector + HyperConnections Memory
- ADR-0012: Holographic Paradox Engine (dream mechanics)
- ADR-0015: Storage Architecture Migration (Dolt backend)
