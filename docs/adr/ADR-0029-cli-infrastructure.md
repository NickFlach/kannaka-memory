# ADR-0029 — CLI Infrastructure: clap, plugins, updates, completions

Status: **Proposed**
Date: 2026-05-24
Authors: Nick Flaukowski (vision), claude-code (drafting)
Related: ADR-0025 (Constellation installer), ADR-0028 (Event-sourced HRM),
         kannaka-memory#92 (refactor 2.9k-LOC bin/kannaka.rs)

---

## Context

`bin/kannaka.rs` has accreted ~30 subcommands behind a single 2.9k-LOC
manual dispatch (`match args[command_start]`). Each handler does its own
`while i < args.len()` flag parsing. The top-level usage string in the
`Usage:` eprintln is hand-curated and has drifted from the actual
match arms repeatedly (`kannaka swarm tail` was missing for a session
after we added it; tracked under #92).

Adjacent symptoms have been compounding:

1. **No per-subcommand `--help`.** Operators run `kannaka swarm` and get
   a single-line usage with no flag explanations.
2. **No shell completion.** Every flag and subcommand is typed from
   memory or copy/pasted from the issue tracker.
3. **Inconsistent JSON mode.** Some commands take `--json`, some don't,
   some emit pretty-printed multi-line JSON, some emit NDJSON, some
   emit `serde_json::to_string(&val)` (no flag, always JSON), some
   emit human prose.
4. **Inconsistent error handling.** Some bail with `process::exit(1)`,
   some print to stderr and continue, some panic on malformed input.
5. **Update mechanism is one-shot only.** `kannaka update` runs when the
   operator types it. No periodic check, no checksum verification, no
   bootstrap install of `kannaka-tui` (the v0.5.15 hint helps
   discoverability but still requires manual action).
6. **The constellation is fragmenting into sibling binaries** —
   `kannaka-tui` extracted to its own repo (ADR-pending), `kannaka-code`
   is its own crate, `kannaktopus` is its own orchestrator. The
   operator has to remember N different binary names + N different
   install paths. There's no umbrella that says "you're in the
   Kannaka constellation, here's everything available."

The CLI is the operator's primary surface for the entire constellation.
Modernizing it pays off in every direction — discoverability for new
operators, stability for downstream tooling (kannaka-tui shellouts,
radio child-spawns, observatory MCP wrapper), and a clean home for the
plugin-based sibling-binary discovery the constellation is growing into.

## Decision

Adopt a four-phase CLI overhaul. Each phase is independently shippable
and each later phase builds on the contract of the earlier ones.

### Phase 1 — clap-ify the top-level dispatch (foundation)

Replace the manual match-on-args dispatch with a clap `Command::new` +
`subcommand` tree. **Handler signatures don't change in this phase** —
the clap match arm calls the existing `handle_X` function with whatever
args clap parsed, marshaling them back into the same shape the handler
expects. This keeps the diff focused and lets us migrate handlers
incrementally in Phase 1.b.

What we get immediately:

- Automatic per-subcommand `--help` with flag descriptions
- Automatic `-h` / `--help` / `--version` everywhere
- Consistent error messages on unknown flags / required-arg missing
- Single source of truth for the command tree (no usage-string drift)
- Foundation for completions (Phase 3) and JSON-mode contract (Phase 4)

Closes #92 at the dispatch level. Handler-internal arg parsing migration
is Phase 1.b — opt-in per handler as each comes up for change.

### Phase 2 — plugin discovery

The constellation's sibling-binary pattern (`kannaka-tui`, `kannaka-code`,
`kannaktopus`) gets a unified discovery layer. Borrows the
git/cargo/kubectl convention: `kannaka X` where `X` isn't a built-in
subcommand falls through to `kannaka-X` on `$PATH`.

```
kannaka tui       → exec kannaka-tui    (already a sibling binary)
kannaka code      → exec kannaka-code   (already a sibling binary)
kannaka topus     → exec kannaktopus    (currently named "kannaktopus")
kannaka <new>     → exec kannaka-<new>  (anyone can ship a plugin)
```

Rules:
- Built-in subcommands always win (no plugin can shadow `remember`)
- `kannaka --list-plugins` enumerates everything on `$PATH` matching
  `kannaka-*` plus the known aliases (topus → kannaktopus)
- `kannaka help <plugin>` execs `<plugin> --help`
- Plugins MUST be self-installable (`cargo install`, `pip install`,
  binary download) — `kannaka` does not bootstrap their install
- Existing aliasing for `kannaktopus` → `topus` is keyword-mapped in a
  small `KNOWN_ALIASES` table so the historical name stays intact

What we get:

- `kannaka <anything>` becomes the operator's single entry point to
  the constellation. They don't have to know which binary owns which
  verb.
- TUI distribution stays clean (no need to bundle TUI into kannaka)
- Plugin authors get a stable contract: ship a `kannaka-X` binary,
  it appears in `kannaka --list-plugins` automatically
- Per-plugin `--help` works via subprocess dispatch

### Phase 3 — Shell completions

Generate completion scripts for bash, zsh, fish, PowerShell. Clap's
`clap_complete` crate handles this from the command tree built in
Phase 1. Plugins extend completion via two hooks:
- The plugin namespace itself completes from `KNOWN_ALIASES` + discovered
  `kannaka-*` binaries on `$PATH`
- Inside a plugin namespace, completion delegates to the plugin's own
  completion file (if it ships one at `$KANNAKA_PLUGIN_DIR/completions/X.bash`)

```bash
kannaka <TAB>            → remember recall search forget dream observe
                            status assess swarm substrate events
                            tui code topus  ... (plugins listed too)
kannaka swarm <TAB>      → join leave status sync queen serve tail ...
kannaka tui <TAB>        → (delegates to kannaka-tui completion file)
```

Install via `kannaka completions install` (writes to standard locations
per shell) or `kannaka completions bash > /etc/bash_completion.d/kannaka`.

### Phase 4 — Quality-of-life: updates + JSON contract

Two parallel sub-tracks shipped together:

**4a — Update mechanism polish:**
- Periodic background check (env-configurable cadence, off by default)
- SHA-256 verification of downloaded binaries (release.yml emits
  `.sha256` sidecars on tag push; `kannaka update` validates before
  rename)
- Opt-in bootstrap install of `kannaka-tui` via
  `kannaka update --bootstrap-tui` — explicit flag, no surprise installs
- `kannaka update --check` exits non-zero if a newer release exists
  (useful for `cron` health checks)

**4b — JSON-mode contract:**
- Every command supports `--json` or has a documented reason not to
  (e.g. `kannaka update` doesn't because output is operator-facing)
- JSON outputs follow a single envelope:
  ```json
  {"schema_version": "1.0", "command": "recall", "data": {...}, "errors": []}
  ```
- Errors go in the `errors` array AND set non-zero exit code
- NDJSON is reserved for streaming commands (`swarm tail`, `chat --json`)
  and uses the same per-line envelope without the outer `data` wrap

## Phasing Plan

| Phase | Scope | Risk | Effort | Ships in |
|---|---|---|---|---|
| 1 | clap top-level dispatch, handlers unchanged | low | M | v0.6.0 |
| 1.b | Per-handler clap migration (incremental) | low | rolling | v0.6.x patches |
| 2 | Plugin discovery + `--list-plugins` | low | S | v0.6.0 |
| 3 | Completions (bash/zsh/fish/pwsh) | low | S | v0.6.1 |
| 4a | Update mechanism polish | low | M | v0.6.2 |
| 4b | JSON-mode contract | medium | M (touches every handler) | v0.6.3 |

Phase 1 + 2 together justify a major minor bump to **v0.6.0** because
the plugin fall-through changes the meaning of `kannaka <unknown>` from
"error" to "exec plugin if found". Operators with shell aliases or
scripts that depend on the old error-on-unknown behavior need to be
warned in the release notes.

## Alternatives Considered

### Alternative A: argh / structopt / pico-args / lexopt instead of clap
Clap is the de facto standard. The 5-second-faster compile time isn't
worth the smaller ecosystem (completion generators, derive macros,
shell-script generators all assume clap). Rejected.

### Alternative B: Single-binary subcommand model (no plugin fall-through)
Bundle TUI + code + topus all back into `kannaka`. Keeps everything in
one binary, simpler distribution. Rejected because (a) we just extracted
TUI for good reasons (independent versioning, smaller dep footprint),
(b) kannaktopus is a Bash orchestrator, can't be bundled into a Rust
crate at all, (c) the plugin model is what makes the constellation
extensible by third parties.

### Alternative C: Wait for a constellation-wide ABI before plugins
Define a shared IPC contract (e.g. Unix sockets + protobuf) that all
constellation binaries must implement, then `kannaka` calls them through
the contract. Rejected as overengineering — `exec()` with stdout
inheritance is the boring solution that already works for every
shellout pattern in the codebase. ABI can come later if needed.

### Alternative D: Defer the plugin work until after Phase 4
Ship clap + completions + JSON first, plugin discovery later. Rejected
because the plugin namespace fundamentally changes the `--help` text
(adds a "Plugins" section) and the completion layout. Ship them
together so the UX story is coherent.

## Open Questions

1. **`kannaka-tui` is the first plugin** — its entry-point naming is
   already correct (`kannaka-tui`). But should we standardize on
   `kannaka-X` vs `kannaka.X` (kubectl style) vs `kannaka_X` (cargo
   subcommand style)? **Recommendation: kannaka-X** (matches git, matches
   the binary names we already have, dash-separated is the established
   constellation convention).

2. **Plugin discovery cost on every invocation.** `kannaka --list-plugins`
   scans `$PATH` for `kannaka-*` — fine for `--list`, expensive on every
   `kannaka <verb>` call. **Recommendation: cache discovery in
   `$KANNAKA_PLUGIN_CACHE` (default `~/.kannaka/plugins.json`), refresh
   on `kannaka update` and on explicit `kannaka --refresh-plugins`.**

3. **Built-in subcommand precedence.** `kannaka tui` could mean either
   "exec kannaka-tui plugin" or "open a future built-in TUI feature."
   **Recommendation: built-ins always win**, document this in `--help`,
   add a `--plugin` escape hatch (`kannaka --plugin tui` forces the
   plugin path even if a built-in exists with the same name).

4. **Completion update flow.** Operators install completions once; how
   do they know to refresh when we add new subcommands? **Recommendation:
   `kannaka update` writes a `~/.kannaka/last-completion-sync` marker
   and prints a hint if more than 30 days stale.**

## Consequences

**Positive:**
- Operator UX dramatically improved (discoverability + completion + per-
  command help)
- Downstream consumers (kannaka-tui shellouts, radio child-spawns,
  observatory MCP) get a stable, versioned contract
- New constellation members can ship as plugins without coordinating
  with the kannaka-memory release cycle
- Closes #92 and unblocks the cleaner handler-extraction work

**Negative:**
- ~30 handler call sites change in Phase 1 (mechanical but pervasive
  diff)
- Plugin fall-through is a semantic change — `kannaka typo-command`
  goes from "error: unknown command typo-command" to "exec kannaka-typo-command (not found)" which is a slightly different error
- Adds clap as a hard dependency (one more crate in the dep tree;
  acceptable tax for the ecosystem benefit)

**Neutral:**
- Existing handlers keep their per-handler `args[i]` parsing through
  Phase 1.b. They get clap-ified opportunistically. Doesn't block
  anyone or force a flag-day.
