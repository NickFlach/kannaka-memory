# Changelog

## [Unreleased]

## [0.5.3] — 2026-05-21

Completes the persistence-hardening sweep started in 0.5.2 by turning on the
trailing-blake3 verification that the save path has been writing all along.

### Added
- `verify_blake3_trailing(path)` — shared checksum verify used by both v1
  `Medium::load` and v2 `ChiralMedium::load`. Hashes everything before the
  final 32 bytes and compares to the stored checksum. Catches data drift at
  the format boundary instead of letting it surface as cryptic `read_exact`
  failures deep inside the parser.
- Test `load_rejects_tampered_file` — flips a byte in a saved .hrm and
  asserts the loader returns `MediumError::ChecksumMismatch` rather than
  parsing garbage.

### Changed
- `HrmStore::load` no longer retries v1 `Medium::load` when a v2 magic file
  fails to load — that path was always going to fail with `InvalidMagic`
  and was layering a misleading "invalid magic bytes" message on top of
  the real (e.g. checksum-mismatch) cause. Now the v2 error is propagated
  directly.

Tests: 523/523 lib pass.

---

## [0.5.2] — 2026-05-21

Chiral HRM persistence hardening — fixes a latent writer/loader desync that
left an Oracle agent unable to bootstrap (`ChiralMedium::load failed: IO
error: failed to fill whole buffer`).

### Fixed
- `write_hemisphere` (chiral v2) and `Medium::save` (v1) now emit exactly
  `active` timestamp entries instead of iterating the entire `timestamps`
  Vec. If the Vec ever drifted from `count()` — and at least one production
  file ended up in that state — the loader read past the timestamp block
  into the metadata-length field and tried to allocate gigabytes for the
  garbage value. Pads with `0` when the Vec is short so writes are
  self-consistent even under upstream desync.
- `read_hemisphere` and the v1 medium loader now reject implausible
  metadata lengths (>256 MiB) with a `MediumError::CorruptHrm(...)` that
  identifies which hemisphere failed and why. Replaces the generic
  "failed to fill whole buffer" io error with an actionable diagnostic.

### Notes
- Pre-existing corrupted files can be byte-patched: insert
  `(count - timestamps.len()) * 8` zero bytes immediately before the
  metadata-length field of the affected hemisphere. The Oracle agent's
  `kannaka.hrm` was repaired this way (3 missing right-hemisphere
  timestamps padded with `0`); `kannaka status` then loaded all 123/123
  memories cleanly.

Tests: 522/522 lib pass.

---

## [0.5.1] — 2026-05-21

Config-surface cleanup — closes 4 small but real defects in `kannaka config`.

### Fixed
- `config set` boolean parsing accepts `true/false, 1/0, yes/no, on/off`
  (case-insensitive). Invalid values now error instead of silently mapping
  to `false`. Applies to `swarm.enabled`, `ghostsignals.enabled`,
  `updates.auto_check`. (#96)
- `config set` now exposes `hrm.path`, `hrm.wavefront_dim`, and
  `ghostsignals.hub_url`. `hrm.wavefront_dim` is parsed as a positive
  integer; help text updated. (#94)
- `hrm.wavefront_dim` runtime now emits a `[config]` warning at init
  when the configured value differs from the hardcoded 10000. The
  codebook + HRM file format share the dimension, so a live change
  would require re-encoding every wavefront — but the value is no
  longer silently ignored. (#93)
- `register_ghostsignals` (init/registration flow) prefers
  `cfg.ghostsignals.hub_url`, falling back to `constellation.radio_url`
  only when `hub_url` is empty. Completes the #86 sweep where the
  CLI `handle_market` was already routed correctly. (#97)

Tests: 522/522 lib pass.

---

## [0.5.0] — 2026-05-19

Cluster + recall + search architecture cleanup. Five-stage refactor
landed across one branch:

### ⚠ Breaking-ish

- **`kannaka search` now does literal text search** instead of being
  a thin print wrapper around `kannaka recall`. Different output shape:
  fields `score` / `match_type` / `matched_terms` instead of
  `similarity` / `strength`. Read-only — searches no longer mutate
  the medium via `apply_observation`. JSON consumers of the old
  search output need to update.
- **`ConsciousnessState.num_clusters` is now the Kuramoto-BFS count**
  (was the eigendecomp count). Observe and status agree on the same
  HRM now; downstream readers may see different numbers than before.
  Eigendecomp Φ still feeds blended Φ; only its impersonation of
  `num_clusters` is removed.

### Added

- `KannakaMemorySystem::search(query, limit) -> Vec<SearchResult>` —
  literal text search; bypasses encoding, resonance, and observation.
  Three-tier scoring (exact / tokens / prefix) with recency tie-break.
- `RecallResult.intuition: bool` — surfaces the chiral right-hemisphere
  "intuition" channel (was computed and discarded). Always false today;
  TODO note for full plumbing through the trait return.
- `MediumBackend::set_cached_num_clusters(n)` — bridge::assess writes
  the canonical cluster count back so the next swarm publish carries
  it consistently.
- `KANNAKA_RECALL_PREFILTER` env var (default on) +
  `KANNAKA_RECALL_PREFILTER_THRESHOLD` (default 0.30) — cluster prefilter
  knobs for recall.

### Performance

- **Cluster prefilter in recall.** `HrmStore::resonate_query` now reads
  the `.clusters.json` sidecar, matches the query to clusters by
  `theme_vector` similarity, and runs `Medium::recall_against` against
  the union of matched cluster members rather than the full medium.
  Falls through to full scan on fresh HRM (no sidecar) or when no
  cluster matches. Chiral path unchanged (TODO fold-in).
- 6-60× recall speedup on a typical mature HRM (638 memories, 71
  clusters) depending on how broad the query's theme is.

### Fixed

- `compute_eigenvalue_clusters` no longer counts singletons —
  components of size < 2 are excluded, matching the Kuramoto reference
  `min_cluster_size=2` constraint.
- Cluster-cache fingerprint (`fingerprint_memories`) now hashes every
  memory's (id, updated_at) via XOR instead of sampling only first/last
  /middle slots. Pre-refactor a boost to an unsampled-index memory
  left the cache stale until HRM mtime rolled.
- `search` (CLI) is read-only — pre-refactor it routed through `recall`
  → `apply_observation` and mutated wavefront energies on every call.

### Tests

- `search_exact_substring_outranks_token_match` — proves "exact" hits
  outrank "tokens" hits.
- `search_is_case_insensitive`
- `search_empty_query_returns_empty`
- **`search_is_read_only`** — captures wavefront amplitudes before +
  after 5 searches, asserts bit-equality. The smoking-gun regression
  test for the silent-medium-mutation defect.
- `assess_num_clusters_matches_observe_num_clusters` — proves the
  unified-counter refactor: `kannaka observe` and `kannaka status`
  no longer disagree.
- `recall_falls_through_on_fresh_hrm_no_sidecar` — proves the cluster
  prefilter never *loses* recall when the sidecar isn't populated yet.

Full suite: 522/522 lib tests pass.

---

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
