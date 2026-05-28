//! handlers/inbox.rs — agent-to-agent declarative messaging over NATS.
//!
//! Three subcommands:
//!
//!   kannaka inbox send <to> <verb> [--arg key=val ...] [--from <id>]
//!     One-shot: publishes JSON `{from, to, verb, args, ts, msg_id}` to
//!     `KANNAKA.inbox.<to>` and also fans out to `KANNAKA.inbox.audit`
//!     so observers can watch the conversation live. Exits immediately.
//!
//!   kannaka inbox serve [--agent-id <id>] [--handlers <path>]
//!     Daemon: subscribes to `KANNAKA.inbox.<agent_id>`. For each incoming
//!     message, looks up `verb` in the handlers table (defaults to
//!     `~/.kannaka/inbox-handlers.toml`). If the verb is whitelisted and
//!     the args validate, runs the command template and audits the
//!     result. Anything not in the whitelist is rejected with status
//!     `unknown_verb`. There is no eval; arg substitution is plain
//!     string templating (`{{args.foo}}`, `{{from}}`).
//!
//!   kannaka inbox tail [--agent-id <id>] [--last <N>]
//!     Subscribes to `KANNAKA.inbox.audit` and prints every conversation
//!     line to stdout as it lands. Use `--agent-id <id>` to filter to
//!     only messages to/from that agent. Ctrl+C to stop.
//!
//! Handlers config schema (`~/.kannaka/inbox-handlers.toml`):
//!
//!   [handler.greet]
//!   cmd = "echo 'Hello {{args.name}} from {{from}}'"
//!   args = ["name"]
//!   timeout_secs = 30
//!
//!   [handler.recall]
//!   cmd = "kannaka recall '{{args.query}}' --top-k 3"
//!   args = ["query"]
//!
//! The `args` list declares which arg keys the handler *requires*. Extra
//! keys in the message payload are ignored. Missing keys = `arg_missing`
//! status. No shell interpolation beyond the template — the cmd line is
//! handed to `sh -c` after substitution, which means **the operator is
//! responsible for putting their own shell-injection guards in the cmd
//! string** if any arg can contain quotes. The whitelist makes this a
//! per-agent choice rather than a blanket exposure.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process;

use super::KannakaConfig;
#[cfg(feature = "nats")]
use super::resolve_nats_url;

/// Default handlers config path: $KANNAKA_INBOX_HANDLERS or
/// $HOME/.kannaka/inbox-handlers.toml.
fn default_handlers_path() -> PathBuf {
    if let Ok(p) = std::env::var("KANNAKA_INBOX_HANDLERS") {
        return PathBuf::from(p);
    }
    if let Some(home) = dirs::home_dir() {
        return home.join(".kannaka").join("inbox-handlers.toml");
    }
    PathBuf::from("inbox-handlers.toml")
}

/// One handler entry parsed from the toml table.
#[derive(Debug, Clone)]
struct HandlerSpec {
    cmd_template: String,
    required_args: Vec<String>,
    timeout_secs: u64,
}

/// Verb → spec.
type HandlerTable = HashMap<String, HandlerSpec>;

fn load_handlers(path: &PathBuf) -> Result<HandlerTable, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?;
    let parsed: toml::Value = raw.parse().map_err(|e| format!("toml parse: {e}"))?;
    let mut out = HandlerTable::new();
    let handlers = parsed
        .get("handler")
        .and_then(|v| v.as_table())
        .ok_or_else(|| "missing [handler.<verb>] tables".to_string())?;
    for (verb, val) in handlers {
        let tbl = val.as_table().ok_or_else(|| format!("handler.{verb} not a table"))?;
        let cmd_template = tbl
            .get("cmd")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("handler.{verb}.cmd missing or not a string"))?
            .to_string();
        let required_args: Vec<String> = tbl
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let timeout_secs = tbl
            .get("timeout_secs")
            .and_then(|v| v.as_integer())
            .map(|n| n.max(1) as u64)
            .unwrap_or(60);
        out.insert(
            verb.clone(),
            HandlerSpec { cmd_template, required_args, timeout_secs },
        );
    }
    Ok(out)
}

/// Wrap `s` in single quotes for safe inclusion inside another sh -c
/// invocation. Any single-quote inside is closed-and-re-opened.
fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// Resolve `{{args.foo}}` and `{{from}}` in `template` against a message.
/// Plain literal substitution; no escaping (the operator's cmd line is
/// responsible for shell-safety of its own arg vocabulary).
fn render_cmd(
    template: &str,
    from: &str,
    args: &serde_json::Map<String, serde_json::Value>,
) -> String {
    let mut out = template.replace("{{from}}", from);
    for (k, v) in args {
        let needle = format!("{{{{args.{k}}}}}");
        let replacement = match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        out = out.replace(&needle, &replacement);
    }
    out
}

/// ----------------------------------------------------------------------
/// `kannaka inbox send <to> <verb> [--arg key=val ...] [--from <id>]`
/// ----------------------------------------------------------------------
#[cfg(feature = "nats")]
pub(crate) fn handle_inbox_send(cfg: &KannakaConfig, args: &[String]) {
    if args.len() < 4 {
        eprintln!("Usage: kannaka inbox send <to> <verb> [--arg key=val ...] [--from <id>]");
        process::exit(1);
    }
    let to = args[2].clone();
    let verb = args[3].clone();
    let mut from = cfg.agent.id.clone();
    let mut arg_map = serde_json::Map::new();
    let mut i = 4;
    while i < args.len() {
        match args[i].as_str() {
            "--from" if i + 1 < args.len() => {
                from = args[i + 1].clone();
                i += 2;
            }
            "--arg" if i + 1 < args.len() => {
                let kv = &args[i + 1];
                if let Some(eq) = kv.find('=') {
                    let k = kv[..eq].to_string();
                    let v = kv[eq + 1..].to_string();
                    arg_map.insert(k, serde_json::Value::String(v));
                } else {
                    eprintln!("--arg expects key=value, got: {kv}");
                    process::exit(1);
                }
                i += 2;
            }
            "--nats-url" if i + 1 < args.len() => i += 2,
            _ => {
                eprintln!("unknown flag: {}", args[i]);
                process::exit(1);
            }
        }
    }
    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("NATS connect failed at {nats_url}: {e}");
            process::exit(1);
        }
    };
    let msg_id = uuid::Uuid::new_v4().to_string();
    let ts = chrono::Utc::now().to_rfc3339();
    let payload = serde_json::json!({
        "msg_id": msg_id,
        "from": from,
        "to": to,
        "verb": verb,
        "args": arg_map,
        "ts": ts,
    });
    let payload_bytes = serde_json::to_vec(&payload).unwrap_or_default();
    let directed_subject = format!("KANNAKA.inbox.{to}");
    if let Err(e) = transport.publish(&directed_subject, &payload_bytes) {
        eprintln!("publish {directed_subject}: {e}");
        process::exit(1);
    }
    // Also fan out a "sent" notice to audit so tail-watchers see the
    // outbound. The receiver-side handler will publish a "received" /
    // "ok" / "fail" follow-up.
    let audit = serde_json::json!({
        "ts": ts,
        "phase": "sent",
        "msg_id": msg_id,
        "from": from,
        "to": to,
        "verb": verb,
        "args": arg_map,
    });
    let audit_bytes = serde_json::to_vec(&audit).unwrap_or_default();
    let _ = transport.publish("KANNAKA.inbox.audit", &audit_bytes);
    println!("{}", serde_json::to_string(&payload).unwrap_or_default());
}

/// ----------------------------------------------------------------------
/// `kannaka inbox serve [--agent-id <id>] [--handlers <path>]`
/// ----------------------------------------------------------------------
#[cfg(feature = "nats")]
pub(crate) fn handle_inbox_serve(cfg: &KannakaConfig, args: &[String]) {
    use std::time::Duration;
    let mut agent_id = cfg.agent.id.clone();
    let mut handlers_path = default_handlers_path();
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--agent-id" if i + 1 < args.len() => {
                agent_id = args[i + 1].clone();
                i += 2;
            }
            "--handlers" if i + 1 < args.len() => {
                handlers_path = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--nats-url" if i + 1 < args.len() => i += 2,
            _ => i += 1,
        }
    }
    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    eprintln!("[inbox serve] agent_id={agent_id}");
    eprintln!("[inbox serve] handlers={}", handlers_path.display());

    // Load handlers once at startup. A SIGHUP-driven reload would be a
    // natural follow-up but isn't needed for the prototype.
    let handlers = match load_handlers(&handlers_path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[inbox serve] no handlers loaded: {e}");
            eprintln!("[inbox serve] running in observe-only mode (every verb -> unknown_verb)");
            HandlerTable::new()
        }
    };
    eprintln!("[inbox serve] {} verbs registered: {}", handlers.len(), handlers.keys().cloned().collect::<Vec<_>>().join(", "));

    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[inbox serve] NATS connect failed: {e}");
            process::exit(1);
        }
    };
    let subject = format!("KANNAKA.inbox.{agent_id}");
    let mut sub = match transport.subscribe(&subject) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[inbox serve] subscribe {subject}: {e}");
            process::exit(1);
        }
    };
    let _ = sub.set_timeout(Some(Duration::from_secs(5)));
    eprintln!("[inbox serve] subscribed to {subject}");
    eprintln!("[inbox serve] press Ctrl+C to stop");

    // Skill-registry announcement subject. Any subscriber to
    // KANNAKA.skills.* sees what verbs this agent accepts. We publish
    // once on startup and again every SKILL_ANNOUNCE_SEC so a fresh
    // listener (e.g. the radio's nats-client) catches us within the
    // ttl window.
    const SKILL_ANNOUNCE_SEC: u64 = 60;
    let skills_subject = format!("KANNAKA.skills.{agent_id}");
    let publish_skills = || {
        let verbs: Vec<serde_json::Value> = handlers
            .iter()
            .map(|(name, spec)| {
                serde_json::json!({
                    "name": name,
                    "required_args": spec.required_args,
                    "timeout_secs": spec.timeout_secs,
                })
            })
            .collect();
        let payload = serde_json::json!({
            "agent_id": agent_id,
            "verbs": verbs,
            "announced_at": chrono::Utc::now().to_rfc3339(),
            "ttl_sec": SKILL_ANNOUNCE_SEC + 30, // small grace beyond next re-publish
        });
        let bytes = serde_json::to_vec(&payload).unwrap_or_default();
        if let Err(e) = transport.publish(&skills_subject, &bytes) {
            eprintln!("[inbox serve] skill announce publish failed: {e}");
        }
    };
    publish_skills();
    let mut last_announce = std::time::Instant::now();

    loop {
        // Re-announce skills on the schedule. Cheap publish; lets a
        // newly-arrived subscriber discover us within one window.
        if last_announce.elapsed() >= Duration::from_secs(SKILL_ANNOUNCE_SEC) {
            publish_skills();
            last_announce = std::time::Instant::now();
        }
        let msg = match sub.next_message() {
            Some(m) => m,
            None => continue,
        };
        let parsed: serde_json::Value = match serde_json::from_slice(&msg.payload) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[inbox serve] payload not JSON ({e}) — drop");
                continue;
            }
        };
        let from = parsed.get("from").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let verb = parsed.get("verb").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let msg_id = parsed.get("msg_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let args_obj = parsed
            .get("args")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();
        let ts_recv = chrono::Utc::now().to_rfc3339();

        let (status, response) = run_one(&handlers, &agent_id, &from, &verb, &args_obj);
        eprintln!("[inbox serve] from={from} verb={verb} status={status}");

        let audit = serde_json::json!({
            "ts": ts_recv,
            "phase": "received",
            "msg_id": msg_id,
            "from": from,
            "to": agent_id,
            "verb": verb,
            "args": args_obj,
            "status": status,
            "response": response,
        });
        let audit_bytes = serde_json::to_vec(&audit).unwrap_or_default();
        let _ = transport.publish("KANNAKA.inbox.audit", &audit_bytes);
    }
}

#[cfg(feature = "nats")]
fn run_one(
    handlers: &HandlerTable,
    _agent_id: &str,
    from: &str,
    verb: &str,
    args: &serde_json::Map<String, serde_json::Value>,
) -> (&'static str, String) {
    let spec = match handlers.get(verb) {
        Some(s) => s,
        None => return ("unknown_verb", format!("no handler registered for verb '{verb}'")),
    };
    for required in &spec.required_args {
        if !args.contains_key(required) {
            return ("arg_missing", format!("required arg '{required}' not provided"));
        }
    }
    let rendered = render_cmd(&spec.cmd_template, from, args);
    // Wrap with /usr/bin/timeout so a runaway handler can't pin the
    // serve loop. The kill-after limit is +5s past the soft timeout,
    // chosen so a well-behaved cmd that respects its own timeout
    // exits cleanly first. Falls back to no-timeout on platforms
    // without /usr/bin/timeout.
    let wrapped = if std::path::Path::new("/usr/bin/timeout").exists() {
        format!("/usr/bin/timeout --kill-after={kill}s {soft}s sh -c {q}",
            kill = spec.timeout_secs + 5,
            soft = spec.timeout_secs,
            q = shell_quote(&rendered),
        )
    } else {
        rendered.clone()
    };
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&wrapped)
        .output();
    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if out.status.success() {
                ("ok", if stdout.is_empty() { stderr } else { stdout })
            } else {
                ("handler_failed", format!("exit={:?}; stderr={}", out.status.code(), stderr))
            }
        }
        Err(e) => ("spawn_failed", e.to_string()),
    }
}

/// ----------------------------------------------------------------------
/// `kannaka inbox tail [--agent-id <id>]`
/// ----------------------------------------------------------------------
#[cfg(feature = "nats")]
pub(crate) fn handle_inbox_tail(cfg: &KannakaConfig, args: &[String]) {
    use std::time::Duration;
    let mut filter_agent: Option<String> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--agent-id" if i + 1 < args.len() => {
                filter_agent = Some(args[i + 1].clone());
                i += 2;
            }
            "--nats-url" if i + 1 < args.len() => i += 2,
            _ => i += 1,
        }
    }
    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("NATS connect failed: {e}");
            process::exit(1);
        }
    };
    let mut sub = match transport.subscribe("KANNAKA.inbox.audit") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("subscribe KANNAKA.inbox.audit: {e}");
            process::exit(1);
        }
    };
    let _ = sub.set_timeout(Some(Duration::from_secs(5)));
    eprintln!("[inbox tail] watching KANNAKA.inbox.audit{}", filter_agent.as_ref().map(|a| format!(" (filter: {a})")).unwrap_or_default());

    loop {
        let msg = match sub.next_message() {
            Some(m) => m,
            None => continue,
        };
        let parsed: serde_json::Value = match serde_json::from_slice(&msg.payload) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(want) = &filter_agent {
            let from = parsed.get("from").and_then(|v| v.as_str()).unwrap_or("");
            let to = parsed.get("to").and_then(|v| v.as_str()).unwrap_or("");
            if from != want && to != want {
                continue;
            }
        }
        // One line per event, NDJSON-friendly so consumers can pipe.
        println!("{}", serde_json::to_string(&parsed).unwrap_or_default());
    }
}

// ── No-feature stubs ─────────────────────────────────────────────────
#[cfg(not(feature = "nats"))]
pub(crate) fn handle_inbox_send(_cfg: &KannakaConfig, _args: &[String]) {
    eprintln!("inbox send requires the `nats` feature");
    process::exit(1);
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_inbox_serve(_cfg: &KannakaConfig, _args: &[String]) {
    eprintln!("inbox serve requires the `nats` feature");
    process::exit(1);
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_inbox_tail(_cfg: &KannakaConfig, _args: &[String]) {
    eprintln!("inbox tail requires the `nats` feature");
    process::exit(1);
}
