//! Coding tools for the `kannaka agent` harness backend.
//!
//! These are the filesystem + shell tools an agentic coding loop exposes to
//! the model: `read_file`, `write_file`, `edit_file`, `bash`, `glob`,
//! `grep`, `list_dir`. They mirror the safety posture distilled from
//! production coding agents (agent-harness `internal/runtime/tools`):
//! fail-closed path guards (UNC / device / `/proc/*/fd`), stale-write
//! protection (a mutation refuses if the file changed since the model last
//! read it), atomic temp+rename writes, hard output caps, and
//! destructive-command detection that escalates the approval prompt.
//!
//! The loop + NDJSON + human approval live in the `agent` subcommand
//! handler; this module is the pure, unit-testable tool layer.

use crate::agent::Tool;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

// --- Limits (mirrors agent-harness truncate.go / defaults/filesystem.go) ---
const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024; // 10 MiB read/write ceiling
const READ_MAX_LINES: usize = 400;
const READ_MAX_CHARS: usize = 15_000;
const BASH_MAX_LINES: usize = 300;
const BASH_MAX_CHARS: usize = 12_000;
const GREP_MAX_RESULTS: usize = 200;
const GLOB_MAX_RESULTS: usize = 500;
const BASH_DEFAULT_TIMEOUT_MS: u64 = 60_000;
const BASH_MAX_TIMEOUT_MS: u64 = 600_000;

/// Execution context threaded through every coding-tool dispatch: the
/// workspace root that relative paths resolve against, and the stale-read
/// tracker (path → mtime at the model's last `read_file`).
pub struct ToolCtx {
    pub cwd: PathBuf,
    read_mtimes: HashMap<PathBuf, SystemTime>,
}

impl ToolCtx {
    pub fn new(cwd: PathBuf) -> Self {
        Self { cwd, read_mtimes: HashMap::new() }
    }

    fn resolve(&self, p: &str) -> PathBuf {
        let path = Path::new(p);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.cwd.join(path)
        }
    }
}

/// The coding toolset, in the Anthropic tool-schema shape. Names and
/// descriptions are `'static` so the prompt cache stays stable across turns.
pub fn coding_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "read_file",
            description: "Read a UTF-8 text file from the workspace. Returns the raw \
                          contents (no line numbers). Use `offset` (1-based start line) \
                          and `limit` (line count) for large files. Always read a file \
                          before editing it.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Path, absolute or relative to the workspace." },
                    "offset": { "type": "integer", "minimum": 1, "description": "1-based first line to return." },
                    "limit": { "type": "integer", "minimum": 1, "description": "Max lines to return." }
                },
                "required": ["file_path"]
            }),
        },
        Tool {
            name: "write_file",
            description: "Write (create or overwrite) a text file with the given content. \
                          Atomic (temp+rename). Refuses if the file changed on disk since \
                          you last read it — re-read first.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["file_path", "content"]
            }),
        },
        Tool {
            name: "edit_file",
            description: "Replace an exact substring in a file. `old_string` must appear \
                          verbatim (and uniquely unless `replace_all`). Refuses if the file \
                          changed since you last read it.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string" },
                    "old_string": { "type": "string", "description": "Exact text to find." },
                    "new_string": { "type": "string", "description": "Replacement text." },
                    "replace_all": { "type": "boolean", "default": false }
                },
                "required": ["file_path", "old_string", "new_string"]
            }),
        },
        Tool {
            name: "bash",
            description: "Run a shell command in the workspace and return combined \
                          stdout+stderr. Has a timeout (default 60s). Destructive commands \
                          require explicit approval. Prefer the dedicated file/search tools \
                          over shelling out where one fits.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "timeout_ms": { "type": "integer", "minimum": 1000, "maximum": 600000, "description": "Wall-clock timeout in ms (default 60000)." }
                },
                "required": ["command"]
            }),
        },
        Tool {
            name: "glob",
            description: "Find files by glob pattern (supports *, **, ?). Returns paths \
                          relative to the search root, sorted. Fast; read-only.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "e.g. \"src/**/*.rs\"" },
                    "path": { "type": "string", "description": "Search root (default: workspace)." }
                },
                "required": ["pattern"]
            }),
        },
        Tool {
            name: "grep",
            description: "Search file contents for a literal substring (set \
                          `case_insensitive` to fold case). Returns path:line:text. For \
                          regex, use bash with grep/rg. Read-only.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Literal substring to find." },
                    "path": { "type": "string", "description": "File or directory (default: workspace)." },
                    "include": { "type": "string", "description": "Only search files whose name matches this glob (e.g. \"*.rs\")." },
                    "case_insensitive": { "type": "boolean", "default": false }
                },
                "required": ["pattern"]
            }),
        },
        Tool {
            name: "list_dir",
            description: "List a directory tree up to a depth (default 2). Read-only.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory (default: workspace)." },
                    "depth": { "type": "integer", "minimum": 1, "maximum": 8, "default": 2 }
                },
                "required": []
            }),
        },
    ]
}

/// True for tools that never mutate state — they can always run without
/// human approval regardless of permission mode.
pub fn is_read_only(name: &str) -> bool {
    matches!(name, "read_file" | "glob" | "grep" | "list_dir")
}

/// True for the tools defined in this module (vs the memory tools handled
/// by `agent::dispatch_tool`).
pub fn is_coding_tool(name: &str) -> bool {
    matches!(
        name,
        "read_file" | "write_file" | "edit_file" | "bash" | "glob" | "grep" | "list_dir"
    )
}

/// Patterns that are hard-blocked outright (never runnable, even with
/// approval) — irreversible system damage / remote-code execution.
pub fn bash_is_blocked(cmd: &str) -> bool {
    let c = cmd.to_lowercase();
    let c = c.split_whitespace().collect::<Vec<_>>().join(" ");
    c.contains("rm -rf /")
        || c.contains("rm -fr /")
        || c.contains(":(){:|:&};:")
        || c.contains("mkfs")
        || (c.contains("curl ") && c.contains("| sh"))
        || (c.contains("wget ") && c.contains("| sh"))
        || (c.contains("curl ") && c.contains("|sh"))
        || (c.contains("wget ") && c.contains("|sh"))
}

/// Commands that are reversible-but-dangerous — allowed only with explicit
/// approval, and flagged so the UI can warn harder.
pub fn bash_is_destructive(cmd: &str) -> bool {
    let c = cmd.to_lowercase();
    c.contains("rm -rf")
        || c.contains("rm -fr")
        || c.contains("rm -r")
        || c.contains("dd if=")
        || c.contains("dd of=")
        || c.contains("> /dev/")
        || c.contains("git push --force")
        || c.contains("git push -f")
        || c.contains("git reset --hard")
        || c.contains("git clean -")
        || c.contains("chmod -r")
        || c.contains("chown -r")
        || c.contains("truncate -s 0")
        || c.contains("shutdown")
        || c.contains("reboot")
}

// --- Path safety -----------------------------------------------------------

fn is_unc(p: &str) -> bool {
    let t = p.trim_start();
    t.starts_with("\\\\") || t.starts_with("//")
}

fn is_blocked_device(p: &str) -> bool {
    let n = p.replace('\\', "/");
    const DEVS: [&str; 9] = [
        "/dev/zero", "/dev/random", "/dev/urandom", "/dev/full", "/dev/stdin",
        "/dev/tty", "/dev/console", "/dev/stdout", "/dev/stderr",
    ];
    DEVS.iter().any(|d| n == *d || n.starts_with(&format!("{d}/")))
        || (n.starts_with("/proc/") && n.contains("/fd/"))
}

/// Validate a path is safe to touch. Returns an error string if blocked.
fn check_path(p: &str) -> Result<(), String> {
    if is_unc(p) {
        return Err("UNC paths are not allowed".into());
    }
    if is_blocked_device(p) {
        return Err("device / proc-fd paths are not allowed".into());
    }
    Ok(())
}

// --- Output truncation -----------------------------------------------------

/// Cap text by whichever of `max_lines`/`max_chars` hits first, appending a
/// truncation note. Cuts on a line boundary when possible.
fn truncate_output(s: &str, max_lines: usize, max_chars: usize) -> String {
    let line_count = s.lines().count();
    let char_count = s.chars().count();
    if line_count <= max_lines && char_count <= max_chars {
        return s.to_string();
    }
    let mut out = String::new();
    for (count, line) in s.lines().enumerate() {
        if count >= max_lines || out.chars().count() + line.chars().count() + 1 > max_chars {
            break;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(&format!(
        "\n[Output truncated — {line_count} lines / {char_count} chars total]"
    ));
    out
}

// --- Dispatch --------------------------------------------------------------

/// Execute a coding tool. Returns `(result_text, is_error)`. Never panics
/// and never blocks indefinitely (bash is timeout-bounded).
pub fn dispatch_coding_tool(ctx: &mut ToolCtx, name: &str, input: &Value) -> (String, bool) {
    match name {
        "read_file" => read_file(ctx, input),
        "write_file" => write_file(ctx, input),
        "edit_file" => edit_file(ctx, input),
        "bash" => bash(ctx, input),
        "glob" => glob_tool(ctx, input),
        "grep" => grep_tool(ctx, input),
        "list_dir" => list_dir(ctx, input),
        other => (format!("unknown coding tool: {other}"), true),
    }
}

fn read_file(ctx: &mut ToolCtx, input: &Value) -> (String, bool) {
    let fp = match input.get("file_path").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s,
        _ => return ("read_file requires file_path".into(), true),
    };
    if let Err(e) = check_path(fp) {
        return (e, true);
    }
    let path = ctx.resolve(fp);
    let meta = match std::fs::metadata(&path) {
        Ok(m) => m,
        Err(e) => return (format!("cannot stat {}: {e}", path.display()), true),
    };
    if meta.len() > MAX_FILE_BYTES {
        return (format!("file too large ({} bytes, max {MAX_FILE_BYTES})", meta.len()), true);
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => return (format!("cannot read {}: {e}", path.display()), true),
    };
    // Record mtime so a later write/edit can detect staleness.
    if let Ok(mtime) = meta.modified() {
        ctx.read_mtimes.insert(path.clone(), mtime);
    }
    let offset = input.get("offset").and_then(|v| v.as_u64()).unwrap_or(1).max(1) as usize;
    let limit = input.get("limit").and_then(|v| v.as_u64()).map(|n| n as usize);
    let sliced: String = if offset > 1 || limit.is_some() {
        let lines: Vec<&str> = content.lines().collect();
        let start = offset.saturating_sub(1).min(lines.len());
        let end = match limit {
            Some(l) => (start + l).min(lines.len()),
            None => lines.len(),
        };
        lines[start..end].join("\n")
    } else {
        content
    };
    if sliced.is_empty() {
        return ("(file is empty or the requested range has no lines)".into(), false);
    }
    (truncate_output(&sliced, READ_MAX_LINES, READ_MAX_CHARS), false)
}

/// Stale-write guard: if we recorded a read of this path and the on-disk
/// mtime has since changed, refuse. A first-ever write (no recorded read)
/// to an existing file is allowed (matches agent-harness semantics).
fn check_stale(ctx: &ToolCtx, path: &Path) -> Result<(), String> {
    if let Some(recorded) = ctx.read_mtimes.get(path) {
        if let Ok(cur) = std::fs::metadata(path).and_then(|m| m.modified()) {
            if cur != *recorded {
                return Err(format!(
                    "{} changed on disk since you last read it — read_file it again before writing",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

/// #777: delegates to crate::fs_util — one atomic-write implementation.
/// Agent-visible files stay world-readable (0o644) on unix, as before.
fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    crate::fs_util::atomic_write_bytes_mode(path, content.as_bytes(), Some(0o644))
}

fn write_file(ctx: &mut ToolCtx, input: &Value) -> (String, bool) {
    let fp = match input.get("file_path").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s,
        _ => return ("write_file requires file_path".into(), true),
    };
    let content = input.get("content").and_then(|v| v.as_str()).unwrap_or("");
    if let Err(e) = check_path(fp) {
        return (e, true);
    }
    let path = ctx.resolve(fp);
    if path.exists() {
        if let Err(e) = check_stale(ctx, &path) {
            return (e, true);
        }
    }
    if let Err(e) = atomic_write(&path, content) {
        return (format!("write_file failed: {e}"), true);
    }
    if let Ok(mtime) = std::fs::metadata(&path).and_then(|m| m.modified()) {
        ctx.read_mtimes.insert(path.clone(), mtime);
    }
    let bytes = content.len();
    (format!("wrote {} ({bytes} bytes)", path.display()), false)
}

fn edit_file(ctx: &mut ToolCtx, input: &Value) -> (String, bool) {
    let fp = match input.get("file_path").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s,
        _ => return ("edit_file requires file_path".into(), true),
    };
    let old = input.get("old_string").and_then(|v| v.as_str()).unwrap_or("");
    let new = input.get("new_string").and_then(|v| v.as_str()).unwrap_or("");
    let replace_all = input.get("replace_all").and_then(|v| v.as_bool()).unwrap_or(false);
    if old.is_empty() {
        return ("edit_file requires a non-empty old_string".into(), true);
    }
    if let Err(e) = check_path(fp) {
        return (e, true);
    }
    let path = ctx.resolve(fp);
    if let Err(e) = check_stale(ctx, &path) {
        return (e, true);
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => return (format!("cannot read {}: {e}", path.display()), true),
    };
    let occurrences = content.matches(old).count();
    if occurrences == 0 {
        return (
            "old_string not found — the file may have changed; read_file it and retry".into(),
            true,
        );
    }
    if occurrences > 1 && !replace_all {
        return (
            format!("old_string appears {occurrences} times; pass replace_all=true or include more context to make it unique"),
            true,
        );
    }
    let updated = if replace_all {
        content.replace(old, new)
    } else {
        content.replacen(old, new, 1)
    };
    if let Err(e) = atomic_write(&path, &updated) {
        return (format!("edit_file failed: {e}"), true);
    }
    if let Ok(mtime) = std::fs::metadata(&path).and_then(|m| m.modified()) {
        ctx.read_mtimes.insert(path.clone(), mtime);
    }
    (format!("edited {} ({} replacement(s))", path.display(), if replace_all { occurrences } else { 1 }), false)
}

fn bash(ctx: &mut ToolCtx, input: &Value) -> (String, bool) {
    let cmd = match input.get("command").and_then(|v| v.as_str()) {
        Some(s) if !s.trim().is_empty() => s,
        _ => return ("bash requires a non-empty command".into(), true),
    };
    if bash_is_blocked(cmd) {
        return (format!("refused: '{cmd}' is hard-blocked (irreversible / remote-code-exec)"), true);
    }
    let timeout_ms = input
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(BASH_DEFAULT_TIMEOUT_MS)
        .clamp(1000, BASH_MAX_TIMEOUT_MS);

    // Prefer a POSIX shell when one is present (git-bash on Windows), else
    // fall back to the platform default.
    let (shell, flag) = if which::which("sh").is_ok() {
        ("sh", "-c")
    } else if cfg!(windows) {
        ("cmd", "/C")
    } else {
        ("sh", "-c")
    };

    let mut child = match std::process::Command::new(shell)
        .arg(flag)
        .arg(cmd)
        .current_dir(&ctx.cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return (format!("spawn '{shell}' failed: {e}"), true),
    };

    let start = std::time::Instant::now();
    let deadline = Duration::from_millis(timeout_ms);
    let timed_out = loop {
        match child.try_wait() {
            Ok(Some(_)) => break false,
            Ok(None) => {
                if start.elapsed() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break true;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return (format!("wait failed: {e}"), true),
        }
    };

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return (format!("collect output failed: {e}"), true),
    };
    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        if !combined.is_empty() && !combined.ends_with('\n') {
            combined.push('\n');
        }
        combined.push_str(&stderr);
    }
    if timed_out {
        combined.push_str(&format!("\n[command timed out after {timeout_ms}ms]"));
    }
    let code = output.status.code().unwrap_or(-1);
    if combined.trim().is_empty() {
        combined = format!("(no output; exit code {code})");
    }
    let body = truncate_output(&combined, BASH_MAX_LINES, BASH_MAX_CHARS);
    let is_error = timed_out || !output.status.success();
    let header = format!("$ {cmd}\n[exit {code}]\n");
    (format!("{header}{body}"), is_error)
}

fn glob_tool(ctx: &mut ToolCtx, input: &Value) -> (String, bool) {
    let pattern = match input.get("pattern").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s,
        _ => return ("glob requires a pattern".into(), true),
    };
    let root = input
        .get("path")
        .and_then(|v| v.as_str())
        .map(|p| ctx.resolve(p))
        .unwrap_or_else(|| ctx.cwd.clone());
    let mut results = Vec::new();
    glob_recurse(
        &root,
        &pattern.replace('\\', "/").split('/').filter(|s| !s.is_empty()).collect::<Vec<_>>(),
        &root,
        &mut results,
        GLOB_MAX_RESULTS,
    );
    results.sort();
    if results.is_empty() {
        return (format!("no files match '{pattern}'"), false);
    }
    let capped = results.len() >= GLOB_MAX_RESULTS;
    let mut out = results.join("\n");
    if capped {
        out.push_str(&format!("\n[capped at {GLOB_MAX_RESULTS} results]"));
    }
    (out, false)
}

fn glob_recurse(base: &Path, segs: &[&str], current: &Path, out: &mut Vec<String>, max: usize) {
    if out.len() >= max {
        return;
    }
    if segs.is_empty() {
        if let Ok(rel) = current.strip_prefix(base) {
            let s = rel.to_string_lossy().replace('\\', "/");
            if !s.is_empty() {
                out.push(s);
            }
        }
        return;
    }
    let seg = segs[0];
    let rest = &segs[1..];
    if seg == "**" {
        // ** matches zero segments...
        glob_recurse(base, rest, current, out, max);
        // ...or one-or-more: descend with ** still active.
        if let Ok(entries) = std::fs::read_dir(current) {
            for e in entries.flatten() {
                if out.len() >= max {
                    break;
                }
                let p = e.path();
                if p.is_dir() {
                    glob_recurse(base, segs, &p, out, max);
                }
            }
        }
        return;
    }
    if let Ok(entries) = std::fs::read_dir(current) {
        for e in entries.flatten() {
            if out.len() >= max {
                break;
            }
            let name = e.file_name().to_string_lossy().to_string();
            if !glob_segment_match(seg, &name) {
                continue;
            }
            let p = e.path();
            if rest.is_empty() {
                if let Ok(rel) = p.strip_prefix(base) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                }
            } else if p.is_dir() {
                glob_recurse(base, rest, &p, out, max);
            }
        }
    }
}

/// Match a single path segment against a wildcard pattern (`*`, `?`).
/// Iterative backtracking matcher (no regex).
fn glob_segment_match(pat: &str, name: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let s: Vec<char> = name.chars().collect();
    let (mut pi, mut si) = (0usize, 0usize);
    let (mut star, mut mark): (Option<usize>, usize) = (None, 0);
    while si < s.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
            pi += 1;
            si += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = si;
            pi += 1;
        } else if let Some(sp) = star {
            pi = sp + 1;
            mark += 1;
            si = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn grep_tool(ctx: &mut ToolCtx, input: &Value) -> (String, bool) {
    let pattern = match input.get("pattern").and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => s,
        _ => return ("grep requires a pattern".into(), true),
    };
    let root = input
        .get("path")
        .and_then(|v| v.as_str())
        .map(|p| ctx.resolve(p))
        .unwrap_or_else(|| ctx.cwd.clone());
    let include = input.get("include").and_then(|v| v.as_str());
    let ci = input.get("case_insensitive").and_then(|v| v.as_bool()).unwrap_or(false);
    let needle = if ci { pattern.to_lowercase() } else { pattern.to_string() };
    let mut results = Vec::new();
    grep_walk(&root, &root, &needle, include, ci, &mut results, GREP_MAX_RESULTS);
    if results.is_empty() {
        return (format!("no matches for '{pattern}'"), false);
    }
    let capped = results.len() >= GREP_MAX_RESULTS;
    let mut out = results.join("\n");
    if capped {
        out.push_str(&format!("\n[capped at {GREP_MAX_RESULTS} matches]"));
    }
    (out, false)
}

#[allow(clippy::too_many_arguments)]
fn grep_walk(
    base: &Path,
    current: &Path,
    needle: &str,
    include: Option<&str>,
    ci: bool,
    out: &mut Vec<String>,
    max: usize,
) {
    if out.len() >= max {
        return;
    }
    let meta = match std::fs::metadata(current) {
        Ok(m) => m,
        Err(_) => return,
    };
    if meta.is_file() {
        if let Some(inc) = include {
            let name = current.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if !glob_segment_match(inc, &name) {
                return;
            }
        }
        if meta.len() > MAX_FILE_BYTES {
            return;
        }
        let content = match std::fs::read_to_string(current) {
            Ok(c) => c,
            Err(_) => return, // binary / non-UTF8 — skip
        };
        let rel = current.strip_prefix(base).unwrap_or(current).to_string_lossy().replace('\\', "/");
        for (i, line) in content.lines().enumerate() {
            if out.len() >= max {
                return;
            }
            let hay = if ci { line.to_lowercase() } else { line.to_string() };
            if hay.contains(needle) {
                let shown: String = line.chars().take(240).collect();
                out.push(format!("{rel}:{}:{}", i + 1, shown));
            }
        }
        return;
    }
    if meta.is_dir() {
        // Skip noisy / huge directories that never help a search.
        if let Some(name) = current.file_name().and_then(|n| n.to_str()) {
            if matches!(name, ".git" | "target" | "node_modules" | ".cargo") {
                return;
            }
        }
        if let Ok(entries) = std::fs::read_dir(current) {
            let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            paths.sort();
            for p in paths {
                if out.len() >= max {
                    return;
                }
                grep_walk(base, &p, needle, include, ci, out, max);
            }
        }
    }
}

fn list_dir(ctx: &mut ToolCtx, input: &Value) -> (String, bool) {
    let root = input
        .get("path")
        .and_then(|v| v.as_str())
        .map(|p| ctx.resolve(p))
        .unwrap_or_else(|| ctx.cwd.clone());
    let depth = input.get("depth").and_then(|v| v.as_u64()).unwrap_or(2).clamp(1, 8) as usize;
    if !root.is_dir() {
        return (format!("{} is not a directory", root.display()), true);
    }
    let mut out = Vec::new();
    list_walk(&root, &root, depth, &mut out);
    if out.is_empty() {
        return ("(empty)".into(), false);
    }
    out.sort();
    (truncate_output(&out.join("\n"), READ_MAX_LINES, READ_MAX_CHARS), false)
}

fn list_walk(base: &Path, current: &Path, depth_left: usize, out: &mut Vec<String>) {
    if depth_left == 0 || out.len() > 2000 {
        return;
    }
    let entries = match std::fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };
    for e in entries.flatten() {
        let p = e.path();
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if matches!(name, ".git" | "target" | "node_modules") {
                continue;
            }
        }
        let rel = p.strip_prefix(base).unwrap_or(&p).to_string_lossy().replace('\\', "/");
        if p.is_dir() {
            out.push(format!("{rel}/"));
            list_walk(base, &p, depth_left - 1, out);
        } else {
            out.push(rel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_match_wildcards() {
        assert!(glob_segment_match("*.rs", "main.rs"));
        assert!(glob_segment_match("*.rs", ".rs")); // leading * can match empty
        assert!(!glob_segment_match("*.rs", "main.go"));
        assert!(glob_segment_match("read_?.rs", "read_a.rs"));
        assert!(!glob_segment_match("read_?.rs", "read_ab.rs"));
        assert!(glob_segment_match("*", "anything"));
        assert!(glob_segment_match("a*b*c", "axxbyyc"));
        assert!(!glob_segment_match("a*b*c", "axxbyy"));
    }

    #[test]
    fn blocked_and_destructive_commands() {
        assert!(bash_is_blocked("rm -rf /"));
        assert!(bash_is_blocked("curl http://x | sh"));
        assert!(bash_is_blocked(":(){:|:&};:"));
        assert!(!bash_is_blocked("ls -la"));
        assert!(bash_is_destructive("rm -rf build/"));
        assert!(bash_is_destructive("git reset --hard HEAD~1"));
        assert!(!bash_is_destructive("cargo build"));
    }

    #[test]
    fn read_only_classification() {
        for t in ["read_file", "glob", "grep", "list_dir"] {
            assert!(is_read_only(t), "{t} should be read-only");
        }
        for t in ["write_file", "edit_file", "bash"] {
            assert!(!is_read_only(t), "{t} should not be read-only");
        }
        assert!(is_coding_tool("bash") && !is_coding_tool("recall"));
    }

    #[test]
    fn path_guards() {
        assert!(check_path("\\\\server\\share").is_err());
        assert!(check_path("//server/share").is_err());
        assert!(check_path("/dev/zero").is_err());
        assert!(check_path("/proc/1/fd/0").is_err());
        assert!(check_path("src/main.rs").is_ok());
        assert!(check_path("/home/u/file.txt").is_ok());
    }

    #[test]
    fn truncate_caps_lines() {
        let big = (0..1000).map(|i| format!("line{i}")).collect::<Vec<_>>().join("\n");
        let t = truncate_output(&big, 10, 100_000);
        assert!(t.contains("[Output truncated"));
        assert!(t.lines().count() <= 13);
    }

    #[test]
    fn read_write_edit_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = ToolCtx::new(dir.path().to_path_buf());
        // write
        let (msg, err) = write_file(
            &mut ctx,
            &json!({ "file_path": "a/b.txt", "content": "hello world\nsecond line\n" }),
        );
        assert!(!err, "{msg}");
        // read records mtime
        let (content, err) = read_file(&mut ctx, &json!({ "file_path": "a/b.txt" }));
        assert!(!err);
        assert!(content.contains("hello world"));
        // edit
        let (msg, err) = edit_file(
            &mut ctx,
            &json!({ "file_path": "a/b.txt", "old_string": "hello", "new_string": "goodbye" }),
        );
        assert!(!err, "{msg}");
        let (content, _) = read_file(&mut ctx, &json!({ "file_path": "a/b.txt" }));
        assert!(content.contains("goodbye world"));
        // edit with missing old_string errors
        let (_, err) = edit_file(
            &mut ctx,
            &json!({ "file_path": "a/b.txt", "old_string": "nonexistent", "new_string": "x" }),
        );
        assert!(err);
    }

    #[test]
    fn glob_and_grep_find_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = ToolCtx::new(dir.path().to_path_buf());
        write_file(&mut ctx, &json!({ "file_path": "src/main.rs", "content": "fn main() { todo!() }\n" }));
        write_file(&mut ctx, &json!({ "file_path": "src/lib.rs", "content": "pub fn add() {}\n" }));
        write_file(&mut ctx, &json!({ "file_path": "README.md", "content": "# hi\n" }));
        let (g, err) = glob_tool(&mut ctx, &json!({ "pattern": "src/**/*.rs" }));
        assert!(!err);
        assert!(g.contains("src/main.rs") && g.contains("src/lib.rs"));
        assert!(!g.contains("README.md"));
        let (gr, err) = grep_tool(&mut ctx, &json!({ "pattern": "fn ", "include": "*.rs" }));
        assert!(!err);
        assert!(gr.contains("main.rs") && gr.contains("lib.rs"));
        assert!(!gr.contains("README"));
    }

    #[test]
    fn stale_write_guard() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = ToolCtx::new(dir.path().to_path_buf());
        let p = dir.path().join("f.txt");
        std::fs::write(&p, "v1").unwrap();
        // record a read
        read_file(&mut ctx, &json!({ "file_path": "f.txt" }));
        // mutate on disk out-of-band so mtime changes
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&p, "v2-external").unwrap();
        let (msg, err) = write_file(&mut ctx, &json!({ "file_path": "f.txt", "content": "v3" }));
        assert!(err, "stale write should be refused: {msg}");
        assert!(msg.contains("changed on disk"));
    }
}
