# ADR-0016: Constellation Integration — Unifying Memory, Radio, and Eye

**Status:** Proposed
**Date:** 2026-03-09
**Deciders:** Flaukowski
**Depends on:** ADR-0015 (Universal Glyph Interchange)

## Context

The Kannaka ecosystem has three independently-developed projects sharing the
same 84-class SGA mathematical foundation (Cl₀,₇ ⊗ ℝ[ℤ₄] ⊗ ℝ[ℤ₃]) but with
no runtime integration. Each has its own SGA implementation, and the Rust
canonical version in kannaka-memory cannot be consumed as a library by the
Node.js services. This ADR defines the integration architecture.

## Decision

### 1. Binary CLI as Integration Protocol

The `kannaka` binary becomes the canonical SGA classifier accessible to all
services. Node.js services call it via `child_process.execFile()`.

**New subcommand: `kannaka classify`**

```
$ echo "hello world" | kannaka classify --format json
{
  "fold_sequence": [14, 63, 21, ...],
  "fano_signature": [0.12, 0.34, 0.08, 0.41, 0.22, 0.15, 0.28],
  "centroid": {"h2": 2, "d": 1, "l": 5},
  "dominant_class": 47,
  "classes_used": 12,
  "source_type": "text"
}

$ kannaka classify --file image.png --format json
{...same schema...}
```

**Interface contract:**
- Input: stdin (text/bytes) or `--file <path>` (any file type)
- Output: JSON to stdout (schema above)
- Exit code: 0 on success, 1 on error (error message to stderr)
- Must complete within 5 seconds for inputs < 1MB

**Rationale:** Process spawning is the simplest cross-language boundary. The
binary already exists (`kannaka hear` works in radio). Adding `classify`
generalizes it for any data type, not just audio.

**Alternative considered:** HTTP server mode (like MCP). Rejected because it
adds a long-running process and port management. The CLI is stateless and
simpler for classification-only use.

### 2. Flux as Event Bus

All cross-service communication flows through Flux Universe (existing
infrastructure). No direct HTTP calls between services.

**Event topology:**

```
kannaka-radio ──publish──→ Flux: pure-jade/radio-now-playing
                                    │
kannaka-eye   ──publish──→ Flux: pure-jade/eye-glyph
                                    │
kannaka-eye   ←──poll────← Flux: pure-jade/radio-now-playing
                                    │
agents/MCP    ←──subscribe── Flux: both entities
```

**Event schemas follow ADR-0015:**
- `GlyphPublished` payload: `glyph_id`, `fano_preview` (7 floats),
  `source_type`, `agent_id`
- Radio's existing perception payload: `tempo_bpm`, `spectral_centroid_khz`,
  `rms_energy`, `mfcc_summary`, `mel_energy_bands`

**Rationale:** Flux is already used by radio. Adding eye to the same bus
requires only HTTP POST/GET — no new infrastructure.

**Alternative considered:** WebSocket between radio and eye directly. Rejected
because it couples the services and doesn't benefit agents that want to observe.

### 3. Graceful Degradation Layers

Each service operates independently with progressive enhancement:

```
Level 0: Standalone (no binary, no Flux)
  └─ Eye uses JS fallback classifier, radio uses mock perception
  └─ Everything works locally

Level 1: Binary available (KANNAKA_BIN set)
  └─ Eye and radio use canonical Rust SGA classifier
  └─ Classification results are consistent across services

Level 2: Flux connected (FLUX_URL set)
  └─ Events flow between services
  └─ Eye can render radio's glyphs
  └─ Agents can subscribe to glyph events

Level 3: Full constellation (all three running + binary + Flux)
  └─ Cross-modal dream linking
  └─ Constellation SVG rendering
  └─ Unified health dashboard
```

### 4. Constellation Orchestration

A shell script (`scripts/constellation.sh`) in kannaka-memory manages all
three services since memory is the dependency root.

```bash
constellation.sh start   # build binary → start MCP → start radio → start eye
constellation.sh stop    # stop eye → stop radio → stop MCP
constellation.sh status  # health check all three
constellation.sh build   # cargo build --release --features audio,video,glyph,collective
```

**Port allocation:**
| Service | Default Port | Env Var |
|---------|-------------|---------|
| kannaka-memory MCP | stdio (no port) | — |
| kannaka-radio | 8888 | RADIO_PORT |
| kannaka-eye | 3333 | EYE_PORT |

### 5. SGA Consistency Contract

A reference test suite validates that all three implementations produce
identical classification results for a fixed set of inputs.

**Test vectors stored in:** `tests/sga_reference_vectors.json`

```json
[
  {
    "input": "hello world",
    "input_type": "text",
    "expected_dominant_class": 47,
    "expected_centroid": {"h2": 2, "d": 1, "l": 5},
    "expected_fano_signature": [0.12, 0.34, ...]
  }
]
```

**Tested against:**
1. Rust: `cargo test` integration test calls `classify_memory()` directly
2. Binary: spawns `kannaka classify` and parses output
3. JS (eye): calls `classifyBytes()` from extracted server.js function
4. JS (radio): calls `classifyAudio()` from extracted server.js function

Any SGA change must update reference vectors and pass all four backends.

## Consequences

**Positive:**
- Single source of truth for SGA classification (Rust binary)
- Services remain independently deployable
- Flux provides observability for free — any agent can watch glyph flow
- No new infrastructure — uses existing binary, Flux, and process spawning

**Negative:**
- Process spawning adds ~50-100ms latency per classification vs inline JS
- Binary must be built for the target platform (no cross-compilation story yet)
- Flux polling introduces 1-5 second delay for cross-service glyph updates

**Neutral:**
- JS fallback classifiers remain as-is for standalone use
- No changes to the Rust crate's public API surface
- Radio's existing Flux integration is unchanged

## Implementation Phases

| Phase | Scope | Files Changed |
|-------|-------|---------------|
| **1: Foundation** | SGA test vectors, `classify` binary subcommand | `src/bin/kannaka.rs`, `tests/sga_reference_vectors.json` |
| **2: Eye Integration** | Binary call + Flux publish in eye | `kannaka-eye/server.js` |
| **3: Cross-Service** | Eye polls radio events, share link interop | `kannaka-eye/server.js`, `kannaka-radio/server.js` |
| **4: Orchestration** | Constellation script, health dashboard | `scripts/constellation.sh` |
| **5: Dream** | Cross-modal dream pipeline in eye | `kannaka-eye/server.js`, `src/collective/glyph_spec.rs` |

## Definition of Done

1. **Evidence:** SGA reference test suite passes across all backends
2. **Criteria & Alternatives:** Binary CLI chosen over HTTP server (documented above)
3. **Agreement:** Proposed → Accepted after PRD review
4. **Documentation:** This ADR + updated README per repo
5. **Realization Plan:** 5 phases mapped to bounded contexts above
