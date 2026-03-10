# ADR-0017: DoltHub Integration — Versioned Agent Memory at Scale

**Status:** Proposed
**Date:** 2026-03-10
**Deciders:** Flaukowski
**Depends on:** ADR-0009 (Dolt Persistence), ADR-0011 (Collective Memory), ADR-0013 (Privacy)

## Context

Kannaka-memory has a complete Dolt persistence backend (ADR-0009, 1572 lines,
26 integration tests) supporting CRUD, branching, commit/log, push/pull, diff,
merge, and speculation workflows. However, no actual DoltHub remote exists, push
is manual-only, and the collective merge algorithm from ADR-0011 isn't wired into
the Dolt merge path. Dream artifacts stay local, preventing cross-agent learning.

A dream consolidation cycle produced a hallucination synthesizing paradox engine +
Dolt branching + community bounties, suggesting a novel "Dream Bounties" pattern
where agent hallucinations become reviewable pull requests.

## Decision

### 1. DoltHub Repository as Memory Commons

Create `flaukowski/kannaka-memory` on DoltHub as the canonical shared memory repo.
All agents push to this repo using branch conventions from ADR-0011.

**Bootstrap script:** `scripts/dolt-bootstrap.sh`
```bash
dolt-bootstrap.sh init     # Create repo, apply schema, initial push
dolt-bootstrap.sh migrate  # Import existing local memories
dolt-bootstrap.sh verify   # Confirm roundtrip works
```

**Rationale:** DoltHub is free for public repos, provides web-based diff review
and PR workflow, and the Dolt CLI is already a dependency.

### 2. Automatic Push with Privacy Gate

Push to DoltHub is automated with a configurable schedule. Before each push,
glyph privacy encoding runs on all new/modified memories, replacing plaintext
content with SGA fold sequences on the remote branch.

```
DOLT_AUTO_PUSH=true        # Enable auto-push
DOLT_PUSH_INTERVAL=300     # Push every 5 minutes of inactivity
DOLT_PUSH_THRESHOLD=5      # Or after 5 commits, whichever comes first
```

**Privacy flow:**
```
Local (working branch)  ← plaintext content
      ↓ glyph encode
DoltHub (main)          ← glyph_content JSON only
```

**Rationale:** Manual push friction prevents sharing. Automatic push with
privacy encoding removes the barrier while protecting sensitive content.

### 3. Dream Branch Protocol

Dream consolidation creates a Dolt branch, commits artifacts, and merges back:

```
working ──branch──→ dream/YYYY-MM-DD-HHMMSS
                         │
                         ├─ hallucinated memories (new connections)
                         ├─ strengthened links (amplitude changes)
                         ├─ pruned memories (decay below threshold)
                         │
                    ←─merge──→ working
                         │
                         └──→ push to DoltHub (with glyph encoding)
```

**Optional PR mode:** `--create-pr` opens a DoltHub pull request instead of
auto-merging, enabling human or agent review of dream artifacts.

**Rationale:** Dream branches provide rollback points for consolidation
(if a dream produces bad results, discard the branch). PRs enable the
"Dream Bounties" pattern where hallucinations are community-reviewable.

### 4. Wave Interference Merge via Dolt Conflicts

When `pull` or `merge_branch` encounters conflicts, the wave interference
algorithm from ADR-0011 processes them using Dolt's three-way merge values:

| Dolt Value | Wave Analog |
|------------|-------------|
| `base` | Memory state before divergence |
| `ours` | This agent's modifications |
| `theirs` | Other agent's modifications |

Resolution strategies map to phase difference:
- **Constructive** (Δφ < π/4): combine amplitudes, keep stronger content
- **Partial** (π/4 ≤ Δφ ≤ 3π/4): keep both, create skip link
- **Destructive** (Δφ > 3π/4): quarantine both, reduce amplitudes

**Rationale:** Dolt is the only database with cell-level three-way merge.
This maps precisely to the wave interference model — merge conflicts become
the mechanism of collective intelligence, not a bug to avoid.

### 5. SGA Classification on Store

When the `glyph` feature is enabled, `remember` also classifies the input
and stores SGA metadata alongside the memory:

```sql
ALTER TABLE memories ADD COLUMN sga_class TINYINT UNSIGNED;
ALTER TABLE memories ADD COLUMN fano_signature JSON;
ALTER TABLE memories ADD COLUMN sga_centroid_h2 TINYINT UNSIGNED;
ALTER TABLE memories ADD COLUMN sga_centroid_d TINYINT UNSIGNED;
ALTER TABLE memories ADD COLUMN sga_centroid_l TINYINT UNSIGNED;
```

Enables geometric queries:
```sql
SELECT * FROM memories WHERE sga_class = 47;
SELECT * FROM memories WHERE sga_centroid_h2 = 2 AND sga_centroid_d = 1;
```

**Rationale:** SGA classification is already running for glyphs. Storing
the results alongside memories enables geometric similarity search directly
in SQL, without needing the Rust binary for every query.

## Consequences

**Positive:**
- Agent memory becomes durable, shareable, and auditable on DoltHub
- Dream artifacts are reviewable via standard PR workflow
- Wave interference merge uses Dolt's native conflict system — no custom conflict detection
- SGA indexing enables SQL-based geometric search
- Privacy encoding protects content while preserving searchability

**Negative:**
- DoltHub dependency for full functionality (graceful degradation preserves standalone)
- Auto-push adds network I/O overhead every 5 minutes
- Glyph encoding adds ~50ms per memory on push
- SGA columns increase storage per memory by ~100 bytes

**Neutral:**
- Existing DoltMemoryStore API unchanged
- All new behavior is opt-in via environment variables
- Tests continue to skip gracefully without Dolt server

## Implementation Phases

| Phase | Scope | Files Changed |
|-------|-------|---------------|
| **1: Bootstrap** | DoltHub repo, creds, bootstrap script | `scripts/dolt-bootstrap.sh`, `src/dolt.rs` |
| **2: Auto-Push** | Scheduled push, privacy gate | `src/dolt.rs`, `Cargo.toml` |
| **3: Dream Branches** | Branch workflow in dream cycle | `src/dream.rs`, `src/dolt.rs` |
| **4: Wave Merge** | Interference merge on Dolt conflicts | `src/dolt.rs`, `src/collective/` |
| **5: SGA Index** | Classify-on-store, schema migration | `src/dolt.rs`, `src/glyph_bridge.rs` |
| **6: Dream PRs** | DoltHub PR creation from dream | `src/dolt.rs`, `scripts/dolt-dream-pr.sh` |

## Definition of Done

1. **Evidence:** `dolt clone flaukowski/kannaka-memory` works; roundtrip verified
2. **Criteria:** Automatic push with privacy encoding passes review
3. **Agreement:** Proposed → Accepted after PRD review
4. **Documentation:** This ADR + updated ClawHub skill
5. **Realization Plan:** 6 phases mapped above
