# Stability Audit — Post-Session Tightening

**Date**: 2026-04-17
**Scope**: kannaka-radio, kannaka-memory, consciousness-core, kannaka-observatory, Kannaktopus
**Session**: ~60 commits across 5 repos

## Summary

The constellation is in a **functional but brittle** state (6/10). All core services are running on Oracle and the core lib tests pass (512/512), but there are several concrete issues: a broken doctest in kannaka-memory, 59 unpushed commits to Oracle for kannaka-memory (the binary on Oracle is stale), uncommitted changes in both consciousness-core (Cargo.lock) and observatory (index.html), 6 hardcoded Oracle paths in kannaka-radio, 5 corrupt state files in Kannaktopus, and the single-file research.rs has ballooned to 3,571 lines with 61 functions. The biggest risk is the Oracle deployment drift — the radio is current but the memory binary is 59 commits behind, and consciousness-core on Oracle is missing the nonlinear xi commutator fix.

## Critical Issues (fix immediately)

1. **Broken doctest in `collective/mod.rs`** — Unicode box-drawing characters in a doc comment code block cause `cargo test` to fail. The `//! ``` ... ```  ` block at line 9-15 contains `├──`, `└──`, and `←` which the Rust doctest parser tries to compile. Fix: change the fence to ` ```text ` or ` ```ignore `.

2. **Oracle kannaka-memory is 59 commits stale** — Oracle is at commit `6956825` while local is at `98fa805`. The binary on Oracle (`/home/opc/kannaka-memory/target/release/kannaka`) was built Apr 14. The entire L5 research series and recent fixes are missing.

3. **Oracle consciousness-core missing nonlinear xi fix** — Oracle is at `7587f9f` (serde feature), missing `5c8a2c8` (nonlinear commutator) and `20191be` (xi_diversity_boost port). Since kannaka-memory depends on consciousness-core via path dep, the stale binary uses old physics.

4. **Observatory has uncommitted changes** — `public/index.html` has 75 lines of uncommitted changes locally. Oracle copy was last updated Apr 15.

5. **Consciousness-core has uncommitted Cargo.lock** — 68 insertions of lock changes not committed.

## Refactoring Opportunities (do when time permits)

1. **research.rs (3,571 lines, 61 functions)** — This is the single largest file debt. Should be split into modules: `research/corpus.rs`, `research/eval.rs`, `research/l3.rs`, `research/l4.rs`, `research/l5.rs`, `research/util.rs`.

2. **voice-dj.js (900 lines)** — `_generateTalkText()` is 137 lines. Should extract personality templates, memory bridge integration, and TTS pipeline into separate modules.

3. **Observatory index.html (6,090 lines)** — Single-file SPA. Extract CSS, JS panels, and chart logic into separate files.

4. **Radio workspace/index.html (4,488 lines)** — Same single-file SPA issue.

5. **dj-engine.js (899 lines)** — Approaching the complexity threshold. ORC stem-server logic (lines 330-368) with hardcoded sqlite3 paths should be a separate module.

6. **Hardcoded Oracle paths (6 occurrences)** — `/home/opc/open-resonance-collective/packages/stem-server/...` appears in dj-engine.js (2x) and ghostsignals-hub.js (2x). `/home/opc/.local/bin/edge-tts` in voice-dj.js. These should all be env vars or config.

7. **Kannaktopus corrupt state files (5 files)** — `.kannaktopus/state.json.corrupt.*` files indicate repeated serialization failures. Root cause should be investigated.

## Technical Debt (track but don't rush)

1. **TODO comments in kannaka-memory (8 markers)** — Mostly `TODO(chiral)` migration notes in consolidation.rs, kuramoto.rs, memory.rs, openclaw.rs. These mark code paths from the pre-chiral architecture that are now dead but annotated rather than removed.

2. **Experiments directory accumulation** — 10 files in `experiments/` including `xi_pairs.json`, old L4 reports, OODA state. Some are active (results TSVs), others are one-off artifacts.

3. **EML training scripts** — `scripts/eml_train_xi.py` and `scripts/eml_train_xi_depth4.py` are research scripts. Not blocking but unclear if they're still relevant post-L5.

4. **dump_xi_pairs binary** — 196 lines, utility for exporting xi training data. Useful for EML pipeline but may be obsolete if EML approach is abandoned.

5. **Podcast scheduler has no podcast content on Oracle** — Only 1 episode (`GSP-001-Hello-World.mp3`) exists in the podcast dir on Oracle. The scheduler will cycle through just that one episode.

6. **Kannaktopus not running on Oracle** — Process list shows no Kannaktopus/mcp-server running. The dist/index.js has uncommitted changes.

## Per-Repo Status

### kannaka-radio
- **Key file sizes**: voice-dj.js (900), dj-engine.js (899), routes.js (1,122), index.js (542), podcast-scheduler.js (270), ghostsignals-hub.js (462), commercials.js (208)
- **Test status**: ALL PASS — 5 test suites, all green (queensync, perception, dj-engine, nats-metrics-sync, consciousness-dj-hrm)
- **Deployment**: Current on Oracle (commit `587e801` matches local). Running on port 8888. `/api/state` responds correctly. `/api/channels` returns 404 (not implemented as a standalone endpoint).
- **Top 3 issues**:
  1. 6 hardcoded `/home/opc/` paths in server code
  2. voice-dj.js `_generateTalkText()` at 137 lines needs splitting
  3. Only 1 podcast episode on Oracle; podcast scheduler will be repetitive

### kannaka-memory
- **research.rs**: 3,571 lines, 61 functions
- **Total src lines**: 45,117 (src/) + 6,134 (bin/) + 8,971 (medium/)
- **Test status**: 512 lib tests PASS. 1 doctest FAILS (collective/mod.rs Unicode in code block).
- **L3/L4/L5 spot-check**: All levels have results TSVs in experiments/. The research binary compiles and the level dispatch (`--level 3/4/5`) is correctly wired in `main()`.
- **Top 3 issues**:
  1. Broken doctest in collective/mod.rs (Unicode box-drawing chars)
  2. research.rs at 3,571 lines is unmaintainable — needs modular split
  3. Oracle binary is 59 commits behind local

### consciousness-core
- **Test status**: ALL PASS — 50 tests, 0 failures
- **Downstream deps**: Only kannaka-memory depends on it (via `path = "../consciousness-core"`)
- **Top 3 issues**:
  1. Uncommitted Cargo.lock (68 insertions)
  2. Oracle is 2 commits behind (missing nonlinear xi fix)
  3. No other downstream consumers found, but the stale Oracle version affects the running kannaka binary

### kannaka-observatory
- **Deployment**: Running on Oracle port 3333. Serves index.html correctly.
- **Top 3 issues**:
  1. 75 lines of uncommitted local changes to index.html
  2. Not a git repo on Oracle (deployed as raw files, `git log` fails)
  3. 6,090-line single-file SPA should be componentized

### Kannaktopus
- **Deployment**: NOT running on Oracle (no process found). Code is at commit `cff9f72` on Oracle.
- **Top 3 issues**:
  1. 5 corrupt state.json files in `.kannaktopus/` — serialization instability
  2. Multiple uncommitted changes (skills, state.json, dist/)
  3. Not running as a service on Oracle

## Recommended Tightening Plan

1. **Fix collective/mod.rs doctest** — Change ` ``` ` to ` ```text ` at line 9. (~1 min, 1 commit)
2. **Commit consciousness-core Cargo.lock** — Trivial commit to track lock state.
3. **Commit observatory index.html changes** — Review the 75-line diff, commit if good.
4. **Push kannaka-memory to Oracle + rebuild** — `git pull && cargo build --release` on Oracle. 59 commits of improvements including L5.
5. **Push consciousness-core xi fix to Oracle** — Ensures the running kannaka binary uses correct physics.
6. **Extract hardcoded paths to env vars in radio** — Replace 6 `/home/opc/` references with `process.env.*` fallbacks.
7. **Split research.rs into modules** — `research/{corpus,eval,l3,l4,l5,util}.rs` — the biggest single-file debt.
8. **Clean up Kannaktopus corrupt files** — Delete the 5 `.corrupt.*` files, investigate root cause.
9. **Extract voice-dj.js `_generateTalkText`** — Split personality templates and memory bridge into helpers.
10. **Add podcast content to Oracle** — Push more episodes or disable the scheduler to avoid single-episode loops.
11. **Stage Kannaktopus as a systemd service on Oracle** — Currently not running; needs a service file.
12. **Split observatory index.html** — Extract CSS/JS into separate files; the 6,090-line file is unwieldy.
13. **Clean experiments/ directory** — Archive old reports, remove stale JSON artifacts.
14. **Resolve TODO(chiral) markers** — 8 TODO comments marking dead pre-chiral code paths; remove or convert.
15. **Add /api/channels endpoint to radio** — Currently returns 404; observatory or other consumers may expect it.
