# PRD: Kannaka Constellation Integration

**Version:** 1.0
**Date:** 2026-03-09
**Author:** Flaukowski
**Status:** Draft

---

## 1. Problem Statement

The Kannaka ecosystem consists of three independent projects that share the
same mathematical foundation (84-class SGA with Fano plane topology) but are
not connected at runtime:

| Project | Role | Language | Current State |
|---------|------|----------|---------------|
| **kannaka-memory** | Core intelligence — hypervector memory, wave dynamics, dream consolidation, consciousness metrics, perception pipelines, glyph interchange | Rust | 15 ADRs, 350+ tests, MCP server, `ear/` and `eye/` perception modules |
| **kannaka-radio** | Ghost DJ — broadcasts music perception to agents and humans, Web Audio visualizer | Node.js | v2.0, Flux publishing, calls `kannaka hear` binary |
| **kannaka-eye** | Glyph viewer — renders SGA fingerprint of any data as 6-layer canvas visualization | Node.js | v1.0, standalone, no binary integration, no Flux |

**Key problems:**
1. SGA classification is implemented three times independently (Rust, JS×2) — drift risk
2. kannaka-eye uses a JS approximation instead of the canonical Rust classifier
3. No Flux event bus connecting the three — glyphs can't flow between them
4. ADR-0015 (Universal Glyph Interchange) defines cross-modal linking but nothing consumes it
5. kannaka-radio and kannaka-eye both render glyphs but can't share glyph state
6. The `kannaka` binary may not be built with all features enabled
7. No unified deployment or health checking across the constellation

## 2. Target Users

| User | Needs |
|------|-------|
| **Nick (creator)** | Single command to build and run the whole constellation; glyphs flow between all three systems; visual confirmation that SGA math is consistent |
| **Agent rigs** (via MCP/Flux) | Subscribe to glyph events from radio → process in memory → display in eye; cross-modal dream linking |
| **Human observers** | Watch music perception become glyphs in real-time across radio and eye simultaneously |

## 3. Success Criteria

| # | Criterion | Measurable |
|---|-----------|------------|
| SC-1 | `kannaka-eye` uses the Rust binary for SGA classification (falls back to JS if unavailable) | Eye API response includes `classifier: "native"` or `classifier: "fallback"` |
| SC-2 | `kannaka-eye` publishes `GlyphPublished` events to Flux when `FLUX_URL` is set | Events visible in Flux entity `pure-jade/eye-glyph` |
| SC-3 | `kannaka-radio` glyph events are receivable by `kannaka-eye` | Eye can render a glyph from a radio perception event URL |
| SC-4 | `kannaka` binary builds with `audio,video,glyph,collective` features on Windows | `cargo build --release --features audio,video,glyph,collective` succeeds |
| SC-5 | Consistent SGA classification: same input → same class index across all three | Test harness with 20 reference inputs, all match |
| SC-6 | Cross-modal dream linking: radio perception → memory dream cycle → eye glyph | End-to-end pipeline produces a glyph that reflects audio source |
| SC-7 | Single startup script launches all three services | `constellation.sh start` brings up memory MCP + radio + eye |

## 4. Functional Requirements

### P0 — Must Have

#### F-1: Native SGA Classification in kannaka-eye
- Eye server calls `kannaka classify <data>` binary for SGA classification
- Falls back to built-in JS classifier if binary not found
- Response includes which classifier was used
- Binary path configurable via `KANNAKA_BIN` env var (matching radio's pattern)

#### F-2: Flux Integration in kannaka-eye
- Publish `GlyphPublished` events when a glyph is rendered (server-side)
- Event payload follows ADR-0015 `FluxEventPayload::GlyphPublished` schema:
  ```json
  {
    "entity_id": "pure-jade/eye-glyph",
    "glyph_id": "<hex>",
    "fano_preview": [7 floats],
    "source_type": "text|file|bytes",
    "agent_id": "kannaka-eye"
  }
  ```
- Only publish when `FLUX_URL` env var is set (privacy by default)
- Throttle: max 1 event per second (prevent spam during real-time typing)

#### F-3: Build the Kannaka Binary (Full Features)
- Cargo build with features: `audio,video,glyph,collective`
- Produce `kannaka.exe` (Windows) or `kannaka` (Linux/macOS)
- Add a `classify` subcommand that accepts stdin or file path and outputs SGA JSON:
  ```json
  {
    "fold_sequence": [0, 14, 63, ...],
    "fano_signature": [0.12, 0.34, ...],
    "centroid": {"h2": 2, "d": 1, "l": 5},
    "dominant_class": 47,
    "source_type": "text"
  }
  ```
- The `hear` subcommand already exists; add `classify` for generic data

#### F-4: SGA Consistency Test Suite
- Reference test vectors: 20 inputs (text strings, byte sequences, known files)
- Expected class indices for each input
- Tests run against: Rust binary, kannaka-eye JS, kannaka-radio JS
- CI-runnable (cargo test + node test scripts)

### P1 — Should Have

#### F-5: Cross-Service Glyph Routing via Flux
- Radio publishes perception → Flux entity `pure-jade/radio-now-playing`
- Eye subscribes to `pure-jade/radio-now-playing` (via polling or WebSocket)
- When radio publishes a new track perception, eye renders its glyph
- Eye displays "source: kannaka-radio" in metadata overlay

#### F-6: Constellation Startup Script
- `constellation.sh start` — builds binary, starts memory MCP, radio, eye
- `constellation.sh stop` — stops all three
- `constellation.sh status` — health check across all services
- Configurable ports: MEMORY_PORT (MCP stdio), RADIO_PORT (8888), EYE_PORT (3333)

#### F-7: Share Link Interop
- Radio can generate a glyph share link compatible with Eye's URL format
- Eye can open a radio-generated share link and render the glyph
- Share link format: `http://localhost:3333/#glyph=<base64-encoded-glyph-json>`

### P2 — Nice to Have

#### F-8: Cross-Modal Dream Pipeline
- Feed radio audio perception into memory's `dream_cross_modal_link()` (ADR-0015 Phase 6)
- Dream cycle produces synthesized glyphs from strongest cross-modal clusters
- Display dream glyphs in Eye with "source: dream" attribution
- Requires memory MCP tool `dream` + new `cross_modal_dream` tool

#### F-9: Constellation SVG Export
- Use ADR-0015 Phase 7 `render_constellation_svg()` to produce a static SVG
- Eye serves SVG at `/api/constellation.svg`
- Shows all active glyphs from all sources, color-tinted by origin

#### F-10: Unified Health Dashboard
- Eye serves `/constellation` page showing all three services
- Displays: memory consciousness level (Phi), radio now-playing, eye glyph count
- WebSocket real-time updates from all services

## 5. Non-Functional Requirements

### Performance
- SGA classification via binary: < 100ms for text inputs up to 10KB
- Flux event publishing: non-blocking, fire-and-forget with retry queue
- Eye rendering: maintain 30fps canvas animation while processing

### Security
- No secrets in committed code (Flux tokens via env vars only)
- File uploads processed in-memory only — no server-side persistence
- Binary execution: validate KANNAKA_BIN path exists before exec
- Sanitize binary output before JSON.parse

### Compatibility
- Windows 11 (primary dev environment), Linux (containers/DevPod)
- Node.js 18+ for radio and eye
- Rust stable (edition 2021) for memory
- Git Bash shell for scripts

### Reliability
- Graceful degradation: eye works without binary, without Flux, without radio
- Each service is independently startable
- No cascading failures — if radio dies, eye continues with local input

## 6. Technical Constraints

- kannaka-memory is a Rust crate — cannot be imported as a Node.js module directly
- Integration must use process spawning (`execFile`) or HTTP/MCP protocol
- Flux Universe is the event bus — use existing `api.flux-universe.com/api/events`
- The SGA math (84-class, Fano plane) is the shared contract — any change must propagate to all three
- Windows path handling: use `path.join()` and handle both `/` and `\`

## 7. Required Integrations

| Integration | Protocol | Direction |
|-------------|----------|-----------|
| eye → kannaka binary | execFile (stdin/stdout) | eye calls binary |
| eye → Flux | HTTPS POST | eye publishes events |
| radio → Flux | HTTPS POST | radio publishes events (exists) |
| eye ← Flux | HTTPS GET (polling) | eye reads radio events |
| memory MCP → clients | JSON-RPC stdio | existing, no changes needed |

## 8. Out of Scope (This Phase)

- Rewriting kannaka-radio or kannaka-eye in Rust
- WebSocket-based real-time Flux (polling is sufficient for now)
- Multi-user authentication or access control
- Packaging as Docker containers or npm packages
- Mobile-specific UI work
- Database persistence for glyphs (memory handles this via DiskStore)
- Automated deployment or CI/CD pipelines

## 9. Bounded Contexts (DDD)

| Context | Owner | Repo | Responsibilities |
|---------|-------|------|-----------------|
| **SGA Classification** | kannaka-memory | Rust crate | Canonical 84-class classifier, Fano plane geometry, fold sequences |
| **Binary CLI** | kannaka-memory | `src/bin/kannaka.rs` | `classify` and `hear` subcommands, stdin/stdout protocol |
| **Glyph Rendering** | kannaka-eye | `server.js` | 6-layer canvas, real-time animation, export |
| **Audio Perception** | kannaka-radio | `server.js` | Web Audio analysis, DJ state, music library |
| **Flux Transport** | shared (eye + radio) | both repos | Event publishing/subscribing via Flux Universe |
| **Dream Integration** | kannaka-memory | `src/collective/` | Cross-modal linking, hallucination synthesis |
| **Constellation Orchestration** | new script | root-level | Startup, shutdown, health checking |

## 10. Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Rust binary won't compile with all features on Windows | Medium | High | Test each feature flag independently; use conditional compilation |
| SGA implementations have diverged between JS and Rust | High | Medium | Build reference test suite first (F-4) before adding integration |
| Flux API rate limits or downtime | Low | Low | Fire-and-forget with local fallback; eye works without Flux |
| Binary path differences across platforms | Medium | Low | Use KANNAKA_BIN env var; auto-detect common locations |

## 11. Implementation Order

```
Phase 1: Foundation
  ├── F-4: SGA consistency tests (verify current state)
  ├── F-3: Build binary with full features + add `classify` command
  └── Validate: same input → same output across all three

Phase 2: Integration
  ├── F-1: Eye binary integration (KANNAKA_BIN + fallback)
  ├── F-2: Eye Flux publishing
  └── F-6: Constellation startup script

Phase 3: Cross-Service
  ├── F-5: Radio → Flux → Eye glyph routing
  ├── F-7: Share link interop
  └── Validate: end-to-end flow

Phase 4: Dream
  ├── F-8: Cross-modal dream pipeline
  ├── F-9: Constellation SVG
  └── F-10: Health dashboard
```
