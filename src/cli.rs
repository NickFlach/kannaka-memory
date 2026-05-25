//! ADR-0029 Phase 1+2: clap-based CLI parser + plugin discovery.
//!
//! Phase 1 (clap-ify top-level dispatch):
//! - Build a clap `Command` tree that knows every built-in subcommand
//!   by name (`remember`, `recall`, `swarm`, `events`, etc.) and a brief
//!   `about` string.
//! - clap handles `--help`, `--version`, unknown-flag errors, and the
//!   automatic usage rendering on stderr — replacing the hand-curated
//!   `usage()` and per-arm `eprintln!("Usage: ...")` lines.
//! - Handler signatures **do not change** in this phase. The clap match
//!   captures the remaining args via `external_subcommand`-style
//!   `arg(Arg::new("args").raw(true))` and the legacy
//!   `match args[command_start]` in `kannaka.rs::main` runs against
//!   the raw args slice as before. Handler-internal `args[i]` parsing
//!   migrates in Phase 1.b as each command comes up for change.
//!
//! Phase 2 (plugin discovery):
//! - `kannaka <verb>` where `<verb>` isn't a built-in falls through to
//!   `which kannaka-<verb>` on `$PATH` and execs that binary with the
//!   remaining args. Built-ins always win — no plugin can shadow
//!   `remember`.
//! - `KNOWN_ALIASES` maps legacy short names to actual binaries
//!   (`topus` → `kannaktopus`, the only constellation binary that
//!   doesn't follow the `kannaka-*` naming convention).
//! - `kannaka --list-plugins` enumerates every `kannaka-*` binary on
//!   `$PATH` plus any aliased binary.

use clap::{Arg, ArgAction, Command};
use std::ffi::OsStr;
use std::path::PathBuf;

/// Verbs that map to a non-`kannaka-*` binary name on `$PATH`. The
/// kubectl-style fall-through (`kannaka <verb>` → `kannaka-<verb>`)
/// would miss these without an alias table. Keep the table small —
/// new constellation binaries should follow the `kannaka-*`
/// convention so they auto-discover.
pub const KNOWN_ALIASES: &[(&str, &str)] = &[
    ("topus", "kannaktopus"),
    ("kax", "agent-kax"),
];

/// Build the clap `Command` tree. Mirrors every top-level built-in
/// subcommand `bin/kannaka.rs::main` dispatches on. Each subcommand
/// declares only its `about` + a catch-all `args` positional so the
/// existing handler can keep parsing its own flags.
///
/// `allow_external_subcommands(true)` is what makes Phase 2 work —
/// any unknown subcommand surfaces as an `ExternalSubcommand` match
/// instead of an error.
pub fn build_cli() -> Command {
    Command::new("kannaka")
        .version(crate::config::VERSION)
        .about("Wave-Interference Memory System — a Holographic Resonance Medium that remembers")
        .long_about(
            "kannaka — the constellation's substrate. A wave-interference memory \n\
             system with bilateral chiral hemispheres, dream consolidation, and \n\
             multi-agent swarm synchronization. Built on the Holographic Resonance \n\
             Medium where recall is matrix multiplication, not search.\n\n\
             Sibling plugins on PATH are discoverable as subcommands:\n  \
             kannaka tui     → kannaka-tui   (terminal dashboard)\n  \
             kannaka code    → kannaka-code  (Rust agentic CLI)\n  \
             kannaka topus   → kannaktopus   (orchestration)\n\
             Any binary named kannaka-X on PATH becomes `kannaka X`.",
        )
        .subcommand_required(false)
        .arg_required_else_help(false)
        .allow_external_subcommands(true)
        .arg(
            Arg::new("list-plugins")
                .long("list-plugins")
                .help("List every kannaka-* plugin discoverable on PATH")
                .action(ArgAction::SetTrue)
                .global(false),
        )
        // ── Setup / lifecycle ───────────────────────────────────────────
        .subcommand(passthrough("init", "First-time installer / config wizard"))
        .subcommand(passthrough("update", "Self-update from GitHub releases (also updates kannaka-tui sibling if installed)"))
        // ── Memory primitives ───────────────────────────────────────────
        .subcommand(passthrough("remember", "Store a memory in the holographic medium"))
        .subcommand(passthrough("recall", "Resonance recall — bilateral search across both hemispheres"))
        .subcommand(passthrough("search", "Literal text search (substring + tokenized)"))
        .subcommand(passthrough("forget", "Remove a memory by UUID"))
        .subcommand(passthrough("prune-prefix", "Bulk-forget every memory whose content starts with a prefix"))
        .subcommand(passthrough("boost", "Increase a memory's amplitude"))
        .subcommand(passthrough("relate", "Find semantically related memories"))
        // ── Consolidation + introspection ───────────────────────────────
        .subcommand(passthrough("dream", "Trigger dream consolidation (--mode deep|lite)"))
        .subcommand(passthrough("observe", "Dump full medium state (waves, clusters, links)"))
        .subcommand(passthrough("status", "Quick consciousness metrics snapshot"))
        .subcommand(passthrough("assess", "Full consciousness level assessment (Phi/Xi/order)"))
        .subcommand(passthrough("stats", "Memory + cluster statistics"))
        .subcommand(passthrough("clusters", "List clusters (optionally drill into one with --with-members)"))
        .subcommand(passthrough("neighbors", "Find nearest-neighbor memories for a query"))
        .subcommand(passthrough("cmf", "Conservative Memory Fields report"))
        .subcommand(passthrough("invariant", "δ-invariant cluster detection"))
        .subcommand(passthrough("topology", "Topology + connectivity report"))
        .subcommand(passthrough("bias", "Adjust the medium's interpretation bias"))
        // ── Perception ─────────────────────────────────────────────────
        .subcommand(passthrough("hear", "Absorb audio (file/url/stream) into the right hemisphere"))
        .subcommand(passthrough("see", "Absorb visual input as a wavefront"))
        // ── Reasoning ──────────────────────────────────────────────────
        .subcommand(passthrough("ask", "One-shot LLM query with HRM recall as grounding"))
        .subcommand(passthrough("chat", "Long-running chat REPL (--json for NDJSON mode)"))
        .subcommand(passthrough("voice", "Memory-driven writing (--mode for style)"))
        // ── Swarm / NATS ───────────────────────────────────────────────
        .subcommand(
            Command::new("swarm")
                .about("Multi-agent swarm operations over NATS")
                .arg(Arg::new("args").trailing_var_arg(true).allow_hyphen_values(true).num_args(0..)),
        )
        .subcommand(
            Command::new("events")
                .about("Event-sourced HRM (ADR-0028): init, snapshot, list-snapshots, restore")
                .arg(Arg::new("args").trailing_var_arg(true).allow_hyphen_values(true).num_args(0..)),
        )
        .subcommand(
            Command::new("substrate")
                .about("ADR-0027 kannaka-prime collective substrate: init, run, backfill, status")
                .arg(Arg::new("args").trailing_var_arg(true).allow_hyphen_values(true).num_args(0..)),
        )
        .subcommand(
            Command::new("attention")
                .about("Sparse-attention beam over HRM: serve, stats")
                .arg(Arg::new("args").trailing_var_arg(true).allow_hyphen_values(true).num_args(0..)),
        )
        // ── Constellation services (HTTP) ──────────────────────────────
        .subcommand(passthrough("radio", "Query kannaka-radio (now-playing, schedule)"))
        .subcommand(passthrough("market", "GhostSignals prediction markets"))
        .subcommand(passthrough("constellation", "Status of all constellation apps"))
        // ── Ops / data movement ────────────────────────────────────────
        .subcommand(passthrough("orchestrate", "Multi-step orchestration (legacy alias for ask --plan)"))
        .subcommand(passthrough("config", "Inspect / modify ~/.kannaka/config.toml"))
        .subcommand(passthrough("export", "Export all memories as JSON"))
        .subcommand(passthrough("export-json", "Stream every wavefront as NDJSON to stdout (heavy)"))
        .subcommand(passthrough("import", "Import memories from a JSON file"))
        .subcommand(passthrough("import-json", "Import NDJSON wavefronts"))
        .subcommand(passthrough("migrate", "Run any pending HRM format migration"))
        .subcommand(passthrough("announce-status", "Publish a one-shot status to QUEEN.announce"))
        // ── Optional / feature-gated ───────────────────────────────────
        .subcommand(passthrough("classify", "[glyph feature] Classify input into an SGA glyph"))
        .subcommand(passthrough("cross-modal-dream", "[collective feature] Cross-modal dream consolidation"))
        // ── Specialized writers (kept in the dispatch even if niche) ───
        .subcommand(passthrough("dream-journal", "Render a dream-journal entry"))
        .subcommand(passthrough("field-notes", "Render field-notes view"))
        .subcommand(passthrough("financial", "Financial dossier writer"))
        .subcommand(passthrough("prediction", "Render a prediction-market dossier"))
        .subcommand(passthrough("modality-axes", "Report per-modality activation axes"))
        .subcommand(passthrough("audit-modality", "Audit modality classification consistency"))
        .subcommand(passthrough("scada", "0xSCADA bridge writer"))
        .subcommand(passthrough("audio", "Audio-feature inspector"))
}

/// One-liner: build a passthrough subcommand that owns its name +
/// description but lets the existing handler parse its own args.
fn passthrough(name: &'static str, about: &'static str) -> Command {
    Command::new(name).about(about).arg(
        Arg::new("args")
            .trailing_var_arg(true)
            .allow_hyphen_values(true)
            .num_args(0..),
    )
}

/// Outcome of CLI parsing. Either we matched a built-in subcommand
/// (caller proceeds with the legacy dispatch), we matched an external
/// subcommand (caller execs the plugin), or clap handled the input
/// itself (--help, --version, --list-plugins — caller returns).
pub enum Dispatch {
    /// Built-in subcommand — fall through to the legacy match in main()
    /// with the original `args` slice unchanged.
    Builtin,
    /// External subcommand — exec `binary` with `args`.
    Plugin { binary: PathBuf, args: Vec<String> },
    /// clap handled it (help, version, --list-plugins). main() should
    /// return immediately.
    Handled,
}

/// Drive the CLI for `argv` (typically `std::env::args().collect()`).
/// Returns a `Dispatch` describing what main() should do next.
pub fn parse(argv: &[String]) -> Dispatch {
    let matches = build_cli().get_matches_from(argv);

    if matches.get_flag("list-plugins") {
        print_plugins();
        return Dispatch::Handled;
    }

    let (name, sub_matches) = match matches.subcommand() {
        Some(pair) => pair,
        None => {
            // No subcommand was given. clap already printed help via
            // `arg_required_else_help(false)` + main()'s own logic.
            return Dispatch::Builtin;
        }
    };

    // Built-in subcommands: caller already has the args, just hand back.
    // We test by whether the matched name appears in our subcommand list.
    // Bind the rebuilt Command tree to a local so the borrow of subcommand
    // names outlives the HashSet collection.
    let app = build_cli();
    let builtins: std::collections::HashSet<&str> =
        app.get_subcommands().map(|sc| sc.get_name()).collect();
    if builtins.contains(name) {
        return Dispatch::Builtin;
    }

    // External subcommand → plugin path. clap's `allow_external_subcommands`
    // model returns external args as OsString (so OS-encoded paths survive),
    // not String — that's why a `get_many::<String>("")` would panic with a
    // TypeId-downcast mismatch at runtime. We lossy-convert because every
    // downstream constellation binary accepts UTF-8 args.
    let plugin_args: Vec<String> = sub_matches
        .get_many::<std::ffi::OsString>("")
        .map(|vals| vals.map(|s| s.to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    resolve_plugin(name, plugin_args)
}

/// Resolve `verb` to an absolute binary path via either KNOWN_ALIASES
/// or the `kannaka-<verb>` PATH search. Returns `Plugin` on success,
/// `Handled` after printing an error on miss.
fn resolve_plugin(verb: &str, args: Vec<String>) -> Dispatch {
    // 1. Check aliases first — `topus` should hit `kannaktopus` not
    //    the (nonexistent) `kannaka-topus`.
    let target_name = KNOWN_ALIASES
        .iter()
        .find(|(alias, _)| *alias == verb)
        .map(|(_, real)| (*real).to_string())
        .unwrap_or_else(|| format!("kannaka-{}", verb));

    match which::which(OsStr::new(&target_name)) {
        Ok(path) => Dispatch::Plugin { binary: path, args },
        Err(_) => {
            eprintln!("error: '{}' is not a kannaka subcommand and no plugin '{}' was found on PATH", verb, target_name);
            eprintln!();
            eprintln!("Try:");
            eprintln!("  kannaka --help          # built-in subcommands");
            eprintln!("  kannaka --list-plugins  # discovered plugins on PATH");
            std::process::exit(127); // POSIX: command not found
        }
    }
}

/// Print every discoverable plugin to stdout. Combines:
/// - Every `kannaka-*` binary found on $PATH (the natural plugin namespace)
/// - Every alias in `KNOWN_ALIASES` whose target exists on $PATH
fn print_plugins() {
    println!("Kannaka plugins discovered on PATH:\n");

    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut rows: Vec<(String, String)> = Vec::new();

    // 1. Aliases pointing at known binaries
    for (verb, target) in KNOWN_ALIASES {
        if let Ok(p) = which::which(OsStr::new(target)) {
            if seen.insert(target.to_string()) {
                rows.push((format!("kannaka {}", verb), p.display().to_string()));
            }
        }
    }

    // 2. Anything named `kannaka-*` on $PATH that looks like a real
    //    executable (excludes shell scripts, update-temp files like
    //    .new / .old / .bak, and other non-binary noise — the discovery
    //    loop is opinionated about what counts as a plugin to keep the
    //    output operator-useful).
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for ent in entries.flatten() {
                let raw = ent.file_name().to_string_lossy().to_string();

                // Skip update-temp + backup + script files.
                if raw.ends_with(".new") || raw.ends_with(".old")
                    || raw.ends_with(".bak") || raw.ends_with(".tmp")
                    || raw.ends_with(".sh") || raw.ends_with(".py")
                    || raw.ends_with(".cmd") || raw.ends_with(".bat")
                    || raw.ends_with(".ps1")
                { continue; }

                #[cfg(windows)]
                let (is_exe, stem) = match raw.strip_suffix(".exe") {
                    Some(s) => (true, s.to_string()),
                    None => (false, raw.clone()),
                };
                #[cfg(unix)]
                let (is_exe, stem) = (true, raw.clone());

                if !is_exe { continue; }
                let Some(suffix) = stem.strip_prefix("kannaka-") else { continue };
                if suffix.is_empty() { continue; }
                if seen.insert(stem.clone()) {
                    rows.push((format!("kannaka {}", suffix), ent.path().display().to_string()));
                }
            }
        }
    }

    if rows.is_empty() {
        println!("  (none — install a kannaka-* binary or one of the aliased targets)");
        println!();
        for (verb, target) in KNOWN_ALIASES {
            println!("  alias 'kannaka {}' would exec '{}' (not on PATH)", verb, target);
        }
        return;
    }

    rows.sort();
    for (verb, path) in rows {
        println!("  {:<28} {}", verb, path);
    }
}

/// Exec the plugin binary, inheriting stdio so the operator sees the
/// plugin's output directly. On Unix this is a true `execvp` — kannaka
/// is replaced by the plugin. On Windows we spawn + wait + propagate
/// exit code because true exec isn't a primitive.
pub fn exec_plugin(binary: PathBuf, args: Vec<String>) -> ! {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&binary).args(&args).exec();
        // exec only returns on failure.
        eprintln!("error: failed to exec {}: {}", binary.display(), err);
        std::process::exit(126);
    }

    #[cfg(windows)]
    {
        let status = std::process::Command::new(&binary)
            .args(&args)
            .status();
        match status {
            Ok(s) => std::process::exit(s.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("error: failed to spawn {}: {}", binary.display(), e);
                std::process::exit(126);
            }
        }
    }
}
