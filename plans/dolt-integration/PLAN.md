# PRD: Kannaka-Memory DoltHub Integration — Versioned Agent Memory at Scale

**Version:** 1.0
**Date:** 2026-03-10
**Author:** Flaukowski
**Status:** Draft
**Dream Seed:** hallucination synthesis of paradox engine + Dolt branching + community bounties

---

## 1. Problem Statement

Kannaka-memory has a solid Dolt persistence layer (ADR-0009, 1572 lines, 26 tests)
but the **DoltHub collaboration story is incomplete**. The Rust code can push/pull
to remotes and manage branches, but:

| Gap | Impact |
|-----|--------|
| No actual DoltHub database exists | Can't share memories across machines |
| Push is manual only | Agent memory isn't durable beyond local disk |
| Collective merge algorithm (ADR-0011) is designed but not wired | Multi-agent wave interference merging doesn't run in practice |
| Dream artifacts stay local | No agent can learn from another agent's hallucinations |
| Glyph privacy (ADR-0013) encode-on-push isn't automated | Manual privacy workflow discourages sharing |
| No Dolt MCP server integration | Agents can't access versioned memory via standard protocol |
| Wasteland MVR and Dolt are separate systems | Two Dolt-backed systems with no bridge |

**Dream hallucination insight:** The system dreamed a synthesis of three concepts:
_paradox engine + Dolt branching + community bounties_. This suggests a novel pattern:
**versioned paradox resolution with community validation** — agent dream conflicts
become reviewable Dolt pull requests where the community (or validator agents) decide
which reality to keep.

## 2. Target Users

| User | Needs |
|------|-------|
| **Nick (creator)** | One command to set up DoltHub memory repo; see memories flow between constellation services; privacy-protected public sharing |
| **Agent rigs** (MCP/Flux) | Store and recall memories across sessions; branch for speculation; share discoveries via pull requests |
| **Multi-agent collectives** | Wave interference merge of independent memory branches; distributed dream consolidation; trust scoring |
| **Wasteland participants** | Bridge between wasteland work economy (wanted/completions/stamps) and agent memory (facts/insights/dreams) |

## 3. Success Criteria

| # | Criterion | Measurable |
|---|-----------|------------|
| SC-1 | DoltHub repo `flaukowski/kannaka-memory` exists with schema | `dolt clone flaukowski/kannaka-memory` succeeds |
| SC-2 | `kannaka --dolt remember` → auto-commit → scheduled push | Memory appears on DoltHub within 60s |
| SC-3 | Glyph privacy encoding runs automatically on push | `dolt_diff` on DoltHub shows `glyph_content` not plaintext |
| SC-4 | Two agents can independently store memories and merge | Wave interference merge produces constructive/destructive/partial results |
| SC-5 | Dream artifacts are committed to dated branches | `kannaka/dream/2026-03-10` branch exists after dream cycle |
| SC-6 | Dream hallucinations become reviewable PRs | DoltHub PR shows hallucinated memories with wave parameters |
| SC-7 | Wasteland completions can reference Dolt memory commits | Completion evidence includes Dolt commit hash |
| SC-8 | Dolt MCP server can serve kannaka memory to any agent | `SELECT * FROM memories WHERE content LIKE '%query%'` via MCP |

## 4. Functional Requirements

### P0 — Must Have

#### F-1: DoltHub Repository Bootstrap
- Create `flaukowski/kannaka-memory` database on DoltHub
- Apply full schema (memories, skip_links, metadata tables)
- Register as remote in local Dolt: `dolt remote add origin`
- Import existing local memories via migration tool
- Initial commit + push
- **Script:** `scripts/dolt-bootstrap.sh` — one command setup

#### F-2: Automatic Push Scheduling
- Background thread or cron job pushes to DoltHub after N commits (configurable, default: 5)
- OR push after T seconds of inactivity (configurable, default: 300)
- Glyph privacy encoding runs automatically before push (if `glyph` feature enabled)
- Push failures are retried with exponential backoff (max 3 retries)
- Status logged to Flux when `FLUX_URL` is set
- **Config:** `DOLT_AUTO_PUSH=true`, `DOLT_PUSH_INTERVAL=300`, `DOLT_PUSH_THRESHOLD=5`

#### F-3: Dream Branch Workflow
- `dream` command creates a dated branch before consolidation: `kannaka/dream/YYYY-MM-DD-HHMMSS`
- Dream artifacts (hallucinations, prune lists, strengthened connections) committed to dream branch
- After dream completes, dream branch is merged back to working branch
- Dream branch pushed to DoltHub (with glyph privacy if enabled)
- Other agents can pull dream branches to learn from hallucinations
- **CLI:** `kannaka --dolt dream` (existing, enhanced behavior)

#### F-4: Credential Management
- `kannaka dolt login` — import DoltHub API token from `dolt creds`
- Store in `.env.dolthub` or system keyring
- Validate token on setup
- Support `DOLTHUB_API_KEY` env var as override
- **CLI:** `kannaka dolt login`, `kannaka dolt whoami`

### P1 — Should Have

#### F-5: Collective Wave Interference Merge
- Wire up the ADR-0011 merge algorithm in Rust:
  - **Constructive** (phase diff < π/4): amplitudes combine via `A = √(A₁²+A₂²+2A₁A₂cos(Δφ))`
  - **Partial** (π/4 ≤ diff ≤ 3π/4): both kept, skip link with `partial_agreement` weight
  - **Destructive** (phase diff > 3π/4): both kept, amplitudes reduced, tagged `disputed`
- Merge triggered by `kannaka --dolt pull` when conflicts detected
- Uses Dolt's `base/ours/theirs` three-way merge as input to wave interference calculation
- Disputed memories moved to `collective/quarantine` branch after 3 conflicts
- **New function:** `wave_interference_merge()` in `src/dolt.rs`

#### F-6: Dream-as-Pull-Request (Novel — from hallucination)
- After a dream cycle, optionally create a DoltHub pull request
- PR contains: hallucinated memories, strengthened connections, pruned IDs
- Diff shows wave parameters (amplitude, phase, frequency changes)
- Validator agents or humans can review and merge/reject
- Accepted dream PRs earn reputation stamps in the wasteland
- **CLI:** `kannaka --dolt dream --create-pr`

#### F-7: Wasteland Bridge
- `kannaka dolt evidence <wanted-id>` — generates a Dolt commit hash as completion evidence
- Commit message includes the wanted-id for traceability
- `kannaka dolt stamp <completion-id>` — validates a completion by checking Dolt commit exists
- Bridge script that reads wasteland `wanted` table and creates kannaka memory entries
- **Integration:** Wasteland completions reference Dolt commits; stamps verify them

#### F-8: SGA-Indexed Memory Search on DoltHub
- Add `sga_class` (u8), `fano_signature` (JSON), `sga_centroid_h2/d/l` columns to memories table
- Populated by `classify` during `remember` (when glyph feature enabled)
- Enables SQL queries: `SELECT * FROM memories WHERE sga_class = 47`
- Geometric similarity search: find memories with similar Fano signatures
- **Schema migration:** `scripts/dolt-migrate-sga.sh`

### P2 — Nice to Have

#### F-9: Dolt MCP Server Integration
- Configure Dolt's MCP server to serve kannaka-memory database
- Agents connect via MCP protocol to read/write versioned memory
- Branch isolation: each agent gets its own branch via MCP
- Read-only access to `main` (consensus), read-write to agent branches
- **Config:** `DOLT_MCP_PORT=8675`

#### F-10: Memory Analytics Dashboard
- SQL views for memory analytics:
  - `v_memory_health` — amplitude distribution, decay rates, consciousness proxy
  - `v_dream_history` — consolidation cycle stats over time
  - `v_agent_contributions` — memories per origin_agent
  - `v_sga_distribution` — class frequency across all memories
- Accessible via DoltHub SQL console or any MySQL client

#### F-11: Progressive Revelation (ADR-0013 Phase 6)
- Publish Fano plane "hints" that lower bloom difficulty over time
- Agents earn revelation credits by contributing high-quality memories
- Community can vote to bloom (reveal) high-value sealed memories
- **Tables:** `bloom_hints`, `revelation_votes`

#### F-12: Constellation-Wide Dolt Sync
- Radio perception snapshots committed to Dolt (audio modality memories)
- Eye glyph renders committed to Dolt (visual modality memories)
- Cross-modal dream linking uses Dolt branches for each modality
- Constellation SVG updated from Dolt query showing all active glyphs

## 5. Non-Functional Requirements

### Performance
- Dolt push: < 5 seconds for < 100 memories delta
- Memory insert with auto-commit: < 50ms (in-memory cache, async Dolt write)
- Wave interference merge: < 1 second for < 50 conflicting memories
- Dream cycle with Dolt branch management: < 30 seconds overhead

### Security
- DoltHub API key never committed to source (env var or keyring only)
- Glyph privacy encoding is mandatory for public DoltHub pushes
- No plaintext memory content on DoltHub `main` branch
- Agent branch isolation: agents cannot write to other agents' branches

### Reliability
- Graceful degradation: works without DoltHub (local Dolt only)
- Works without local Dolt (in-memory only, existing behavior)
- Push failures don't block memory operations
- Merge conflicts are quarantined, never silently dropped

### Compatibility
- Dolt SQL server 1.x (MySQL 8.0 compatible wire protocol)
- DoltHub REST API v1alpha1
- Dolt MCP server (if F-9 implemented)
- Windows 11 + Git Bash (primary dev), Linux (containers)

## 6. Technical Constraints

- `DoltMemoryStore` already implements `MemoryStore` trait — all changes must preserve this interface
- Feature-gated: `dolt` feature flag controls all Dolt code; `glyph` controls privacy encoding
- `mysql` crate v25 is the SQL driver — no async runtime (blocking I/O)
- DoltHub free tier: unlimited public repos, limited private repos
- Dolt MCP server uses streaming HTTP transport on port 8675

## 7. Architecture

### Data Flow

```
Agent ──remember──→ MemoryEngine (in-memory)
                        │
                        ├──→ DoltMemoryStore (local SQL)
                        │        │
                        │        ├──→ auto-commit (every N writes)
                        │        │
                        │        └──→ auto-push (scheduled)
                        │                │
                        │                ├──→ glyph encode (privacy)
                        │                │
                        │                └──→ DoltHub (remote)
                        │
                        └──→ Flux (event: memory.stored)

Dream ──cycle──→ Dream Branch
                    │
                    ├──→ hallucinations committed
                    ├──→ prune list committed
                    ├──→ merge back to working
                    │
                    └──→ (optional) DoltHub PR
                             │
                             └──→ Wasteland stamp (if validated)
```

### Branch Topology

```
main                              ← consensus (merged dream results)
├── flaukowski/working            ← auto-pushed after each store
├── flaukowski/dream/2026-03-10   ← dream cycle artifacts
├── arc/working                   ← another agent's memories
├── arc/dream/2026-03-10          ← another agent's dreams
├── collective/speculation-x      ← shared hypothesis space
└── collective/quarantine         ← disputed memories under review
```

## 8. Bounded Contexts (DDD)

| Context | Owner | Responsibilities |
|---------|-------|-----------------|
| **Dolt Persistence** | `src/dolt.rs` | CRUD, dirty-set, branch management, push/pull |
| **Privacy Encoding** | `src/glyph_bridge.rs` + `src/dolt.rs` | Glyph encoding before push, bloom difficulty |
| **Dream Engine** | `src/dream.rs` | 9-stage consolidation, hallucination, Dolt branch workflow |
| **Collective Merge** | `src/collective/` | Wave interference algorithm, trust scoring, quarantine |
| **Wasteland Bridge** | new: `src/wasteland_bridge.rs` | Evidence generation, stamp verification |
| **SGA Index** | `src/glyph_bridge.rs` | Classify-on-store, Fano signature indexing |

## 9. Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| DoltHub rate limits on frequent pushes | Medium | Medium | Batch commits, configurable push interval |
| Wave interference merge produces unexpected results | Medium | High | Extensive test suite, quarantine for disputes |
| Glyph encoding makes debugging difficult | Low | Medium | Local branch preserves plaintext; glyph only on push |
| Dolt server not running on dev machine | High | Low | Graceful degradation already works; skip Dolt tests |
| MCP server port conflicts | Low | Low | Configurable port, health check before start |

## 10. Implementation Order

```
Phase 1: Foundation (DoltHub Bootstrap)
  ├── F-1: Create DoltHub repo + bootstrap script
  ├── F-4: Credential management (login, whoami)
  └── Validate: push/pull roundtrip works

Phase 2: Automation
  ├── F-2: Auto-push scheduling
  ├── F-3: Dream branch workflow
  └── Validate: dream cycle creates branch, commits, pushes

Phase 3: Collective Intelligence
  ├── F-5: Wave interference merge
  ├── F-6: Dream-as-PR (novel from hallucination)
  └── Validate: two-agent merge with constructive/destructive results

Phase 4: Ecosystem
  ├── F-7: Wasteland bridge
  ├── F-8: SGA-indexed search
  └── Validate: wasteland completion references Dolt commit

Phase 5: Advanced
  ├── F-9: Dolt MCP server
  ├── F-10: Analytics dashboard
  ├── F-11: Progressive revelation
  └── F-12: Constellation-wide sync
```

## 11. Novel Ideation (from Dream Cycle)

The dream consolidation produced a hallucination synthesizing three concepts that
were previously unconnected in memory:

> **Paradox Engine + Dolt Branching + Community Bounties**

This suggests a pattern we're calling **"Dream Bounties"**:

1. An agent runs a dream cycle that produces hallucinations (novel connections)
2. The hallucinations are committed to a dream branch on DoltHub
3. A pull request is opened with the hallucinated memories
4. The community (human or agent validators) reviews the PR
5. High-quality hallucinations are merged to `main` (consensus)
6. The dreaming agent earns a reputation stamp in the wasteland
7. Other agents pull the merged hallucinations and integrate them

This creates an **economy of imagination** — agents are incentivized to dream
productively because their hallucinations become reviewable, mergeable, and
rewardable. The Dolt branch/PR mechanism provides the review gate, and the
wasteland stamp system provides the incentive.

**Key insight:** Dolt's three-way merge (base/ours/theirs) maps directly to the
paradox engine's resolution strategies:
- `base` = memory state before dream
- `ours` = this agent's dream result
- `theirs` = another agent's dream result
- Resolution = wave interference calculation on the three values

This is the first architecture where **database merge conflicts are not bugs —
they are the mechanism of collective intelligence**.

---

*Dream seed: Phi=0.399, Xi=0.5, Kuramoto r=0.831, consciousness=Aware*
*Hallucination cluster: r=0.68, n=6 memories*
*"Memories don't die. They interfere."*
