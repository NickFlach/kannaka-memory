//! T3.2 emit hook — write a `kannaka-qubo/1` problem to disk ALONGSIDE the
//! procedural consolidation plan, gated by a config flag (default off).
//!
//! This is deliberately advisory and side-effect-free with respect to the memory
//! store: the dream's procedural consolidation still runs and remains
//! authoritative. The engine's re-score-before-apply path is T3.5, not here — so
//! nothing consumes these files yet; they exist for the golden-file / solver
//! benchmark work and for eyeballing real dream QUBOs.

use std::path::PathBuf;

use super::problem::ConsolidationProblem;

/// Env flag: emit a QUBO per plan when set to `1|on|true`. **Default off** — the
/// procedural path is byte-identical when unset.
pub const EMIT_ENV: &str = "KANNAKA_QUBO_EMIT";
/// Env override for the output directory.
pub const DIR_ENV: &str = "KANNAKA_QUBO_EMIT_DIR";

/// Whether the emit hook is enabled.
pub fn enabled() -> bool {
    std::env::var(EMIT_ENV)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("on") || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// Directory QUBOs are written to: `KANNAKA_QUBO_EMIT_DIR`, else
/// `<KANNAKA_DATA_DIR|~/.kannaka>/qubo`.
pub fn emit_dir() -> PathBuf {
    if let Ok(d) = std::env::var(DIR_ENV) {
        return PathBuf::from(d);
    }
    let base = std::env::var("KANNAKA_DATA_DIR").map(PathBuf::from).unwrap_or_else(|_| {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".kannaka")
    });
    base.join("qubo")
}

/// Write `problem` as pretty JSON to `emit_dir()/<problem_id>.json`. Best-effort:
/// returns the path on success, logs and returns `None` on any I/O error (a
/// dream must never fail because a debug artifact couldn't be written).
pub fn write_problem(problem: &ConsolidationProblem) -> Option<PathBuf> {
    let dir = emit_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("[qubo emit] could not create {}: {e}", dir.display());
        return None;
    }
    // Timestamps in the id carry ':' — not a legal Windows filename char.
    let fname = format!("{}.json", problem.problem_id.replace(':', "-"));
    let path = dir.join(fname);
    let json = match serde_json::to_string_pretty(problem) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("[qubo emit] serialize failed: {e}");
            return None;
        }
    };
    match std::fs::write(&path, json) {
        Ok(()) => {
            eprintln!(
                "[qubo emit] wrote {} ({} vars) to {}",
                problem.problem_id,
                problem.num_vars(),
                path.display()
            );
            Some(path)
        }
        Err(e) => {
            eprintln!("[qubo emit] write {} failed: {e}", path.display());
            None
        }
    }
}
