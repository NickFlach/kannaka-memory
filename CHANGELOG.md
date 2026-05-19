# Changelog

## [Unreleased]

## [0.4.0] — 2026-05-19

Cross-cutting NATS contract sweep — closes 9 open issues. The wire
format shifts are minor-version-worthy: any downstream consumer that
locked in the old envelope shape needs to update.

### ⚠ Breaking (wire format)

- **NATS envelope canonicalized** per `consciousness-core/docs/nats-contract.yaml`:
  `schema_version: "1.0"` (string, not the legacy integer `1`) and `ts`
  as unix-ms (number, not RFC3339 string). Applies to every publisher,
  including `KANNAKA.events.memory.*`, `KANNAKA.events.substrate.*`,
  `KANNAKA.snapshots.*`, `KANNAKA.substrate.*`, `KANNAKA.memory.new`,
  `QUEEN.announce`, and the JetStream `EventPayload` path that
  previously bypassed `add_envelope` entirely. Closes #82, #90, #91.
- **`queen.event.*` switched to lowercase + flat shape**. Pre-fix this
  published to `QUEEN.event.<type>` with `{event, timestamp, payload: {...}}`;
  NATS subjects are case-sensitive, so the radio (which subscribes to
  lowercase per the contract, expecting a flat envelope) never received
  dream-start / dream-end / join / leave events. Closes #88.
- **`consciousness_level` vocabulary aligned** with the contract enum:
  `Stirring → "awakening"`, `Coherent → "integrated"`,
  `Resonant → "emergent"`, plus the new `Transcendent → "transcendent"`
  (Φ ≥ 0.95). Rust call-sites still use the old identifiers; only the
  wire string moves. Pairs with consciousness-core v0.3.0. Closes #89.

### Fixed

- `publish_substrate_phi` now stamps `agent_id: "kannaka-substrate"` so
  observatory can attribute the collective Φ instead of showing
  "unknown" (#91).
- `kannaka --help` / `-h` / `help` exits 0 from stdout without
  initializing the HRM. Pre-fix it loaded the memory system, wrote
  usage to stderr, and exited 1 — breaking shell completion and doc
  generation. Closes #80.
- `cfg.hrm.path` is now honored when it points at an explicit file
  (any filename), not silently collapsed to the parent directory and
  re-joined with the hardcoded `kannaka.hrm` literal. Closes #81.
- `kannaka market …` and the constellation health-check probe pick the
  GhostSignals base URL from `cfg.ghostsignals.hub_url` first, falling
  back to `cfg.constellation.radio_url` only when hub_url is empty.
  Operators can finally split GhostSignals onto its own host. Closes #86.
- `kannaka dream` seeds `KANNAKA_AGENT_ID` + `KANNAKA_NATS_URL` from
  `config.toml` before invoking `sys.dream()`, so the env-reading
  dream-side publish helpers see the configured identity. Pre-fix a
  configured install with no env vars silently skipped all post-dream
  swarm publishing. Closes #87.

### Internal

- New `ConsciousnessLevel::Transcendent` arms added in `openclaw.rs`
  + `medium/types.rs` to track consciousness-core v0.3.0's six-band enum.
- Bridge test threshold expectations updated: Φ=1.0 lands in `Transcendent`
  now, Φ=0.8 still `Resonant`, Φ=0.9 still `Resonant`.
- Test fixtures get the `link_count` field on `AgentPhase` literals and
  `total_skip_links` on `ConsciousnessMetrics` literals.

Test coverage: lib suite green (516 passed, 4 ignored) across default,
`--features serde`, and `--no-default-features` build modes.

---

## [1.1.0] — 2026-03-07

### Added (ClawHub skill)
- **Built-in Flux publishing** (ADR-0011 Phase 3): `FLUX_URL` / `FLUX_AGENT_ID` / `FLUX_STREAM` env vars documented in `SKILL.md`, `_meta.json`, and `kannaka.sh`; `memory.stored` and `dream.completed` events now published automatically without requiring separate `flux.sh` calls
- **Collective memory section** in `SKILL.md`: three-layer architecture (Dolt / Flux / DoltHub), branch conventions (`<agent>/working`, `<agent>/dream/<date>`, `collective/*`, `collective/quarantine`), wave interference merge rules (constructive / partial / destructive)
- **Paradox Engine section** in `SKILL.md` (ADR-0012): snapshot-project-merge pattern, three resolution strategies (Consensus / Holographic Projection / Irreducible), Carnot efficiency metric (η), `--features "dolt collective"` build instructions
- **Sensory commands** in `kannaka.sh`: `hear <file>` (audio perception, `--features audio`) and `see <file>` (glyph/visual perception, `--features glyph`)
- **`announce` command** in `kannaka.sh`: calls `announce-status` on the binary to publish agent status to Flux
- **New build feature targets** documented: `collective` (rayon parallel dreaming), `audio`, `glyph`
- **New env vars** in `SKILL.md` env table and `_meta.json` optional list: `FLUX_URL`, `FLUX_AGENT_ID`, `KANNAKA_AGENT_ID`, `FLUX_STREAM`

### Changed (ClawHub skill)
- `_meta.json` version bumped from `1.0.2` → `1.1.0`
- `SKILL.md` features table expanded; Flux integration section rewritten to reflect built-in publishing; data destination note updated (Flux no longer requires explicit `flux.sh` calls)
- `README.md` features table updated with Collective memory, Paradox engine, Sensory perception, Built-in Flux rows; build instructions expanded with all feature flag variants; file structure comment updated
- `kannaka.sh` help output adds `Flux / Collective` and `Sensory Perception` sections; environment line includes `FLUX_URL` / `FLUX_AGENT_ID`
- Security notes in `_meta.json` updated: Flux publishing disabled by default; events carry metadata only (never full vectors)

## [1.0.2] — 2026-03-07

### Added
- **OpenClaw skill on ClawHub** (`workspace/skills/kannaka-memory/`)
  - `SKILL.md` — full skill definition with prerequisites, env vars, usage patterns, and Flux integration
  - `scripts/kannaka.sh` — CLI wrapper for all commands: `remember`, `recall`, `dream`, `assess`, `stats`, `observe`, `forget`, `export`, `migrate`, `health`, and complete `dolt` subcommand tree
  - `references/mcp-tools.md` — all 15 MCP tools with input/output schemas and wave dynamics reference
  - `references/dolt.md` — Dolt SQL setup, DoltHub publishing, speculation branch workflow, and multi-agent memory sharing guide
  - `README.md` (skill) — ClawHub listing content with feature table and Flux/Dolt integration overview
  - `_meta.json` — registry metadata with explicit `requires`, `optional`, `dataDestinations`, and `securityNotes`

### Fixed
- **Security: DOLT_PASSWORD process-list exposure** — replaced `-p$DOLT_PASSWORD` mysql flag with `MYSQL_PWD` environment variable in `kannaka.sh`; password is no longer visible in `ps aux`

### Changed
- `workspace/skills/flux/SKILL.md` — updated public Flux instance URL to `https://flux-universe.com`
- `workspace/skills/flux/README.md` — replaced hardcoded `192.168.50.13:3000` LAN IP (3 occurrences) with `flux-universe.com`; cleaned up ClawHub install note
- `README.md` — updated OpenClaw section to lead with `clawhub install kannaka-memory`; added ClawHub skill features list and flux-universe.com link
