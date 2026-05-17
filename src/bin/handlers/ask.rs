//! `kannaka ask` — one-shot agent query (local) and `--remote` routing
//! to peer agents over NATS (ADR-0026 Phase 1).
//!
//! Extracted from `bin/kannaka.rs` in v0.3.27 following the pattern
//! documented in `handlers/substrate.rs`.

use std::process;

use super::{compact_input, data_dir, KannakaConfig};

pub(crate) fn handle_ask(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    args: &[String],
) {
    // Parse flags: --session <id>, --quiet-tools, --no-tools, --no-recall,
    // --full-recall, --recall-query <text>, --remote <agent_id|broadcast>,
    // --remote-timeout <seconds>
    //
    // Recall mode precedence: --no-recall > --full-recall > attention (default).
    //   attention   — query-aware beam + recall_with_beam. Default. ~3-5s.
    //   --full-recall — scan the full medium with xi-rerank. 60-90s on
    //                   a mature HRM. Use only when attention misses.
    //   --no-recall — skip resonance entirely. ~2-3s. No memory context.
    let mut session: Option<String> = None;
    let mut quiet_tools = false;
    let mut no_tools = false;
    let mut no_recall = false;
    let mut full_recall = false;
    let mut recall_query: Option<String> = None;
    let mut remote: Option<String> = None;
    let mut remote_timeout_secs: u64 = 60;
    let mut parts: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--session" if i + 1 < args.len() => { session = Some(args[i + 1].clone()); i += 2; }
            "--quiet-tools" => { quiet_tools = true; i += 1; }
            "--no-tools" => { no_tools = true; i += 1; }
            "--no-recall" => { no_recall = true; i += 1; }
            "--full-recall" => { full_recall = true; i += 1; }
            "--recall-query" if i + 1 < args.len() => { recall_query = Some(args[i + 1].clone()); i += 2; }
            "--remote" if i + 1 < args.len() => { remote = Some(args[i + 1].clone()); i += 2; }
            "--remote-timeout" if i + 1 < args.len() => {
                remote_timeout_secs = args[i + 1].parse().unwrap_or(60);
                i += 2;
            }
            _ => { parts.push(args[i].clone()); i += 1; }
        }
    }
    let prompt = parts.join(" ").trim().to_string();
    if prompt.is_empty() {
        eprintln!("Usage: kannaka ask [--session <id>] [--quiet-tools] [--no-tools] [--no-recall|--full-recall] [--recall-query \"text\"] [--remote <agent_id|broadcast>] \"your question\"");
        process::exit(1);
    }

    // --remote: route the question over NATS to a peer (or broadcast) running
    // `kannaka swarm serve`. ADR-0026 Phase 1.
    if let Some(target) = remote {
        return handle_ask_remote(cfg, &target, &prompt, recall_query.as_deref(),
            no_tools, remote_timeout_secs, quiet_tools);
    }

    let result = if no_recall {
        // No memory context — fastest possible round-trip.
        kannaka_memory::agent::ask_no_recall(sys, cfg, &prompt)
    } else if full_recall && no_tools {
        // Explicit slow path, single round-trip (legacy radio caller).
        kannaka_memory::agent::ask_notools_ex(sys, cfg, &prompt, recall_query.as_deref())
    } else if full_recall {
        // Explicit slow path with tool loop — opt-in 60-90s scan.
        match session {
            Some(id) => {
                let path = data_dir().join("sessions").join(format!("{id}.json"));
                kannaka_memory::agent::ask_with_session(sys, cfg, &path, &prompt)
            }
            None => kannaka_memory::agent::ask(sys, cfg, &prompt),
        }
    } else {
        // Default — attention-driven recall: query-aware beam prefilter
        // then full wave resonance against the beam only. Resonance
        // semantics preserved, scan cost is O(beam) not O(medium).
        //
        // The attention path is single-shot (no tool loop) by design —
        // the beam already surfaces the relevant memories, so the model
        // doesn't need to call `recall` itself. `--no-tools` is therefore
        // a no-op here; the path is always tool-free.
        let _ = no_tools;
        match session {
            Some(id) => {
                let path = data_dir().join("sessions").join(format!("{id}.json"));
                kannaka_memory::agent::ask_attention_with_session(sys, cfg, &path, &prompt)
            }
            None => kannaka_memory::agent::ask_attention(sys, cfg, &prompt),
        }
    };

    match result {
        Ok(result) => {
            if !quiet_tools && !result.tool_calls.is_empty() {
                eprintln!("[agent] {} tool call(s):", result.tool_calls.len());
                for tc in &result.tool_calls {
                    let mark = if tc.is_error { "!" } else { "·" };
                    eprintln!("  {mark} {}({})", tc.name, compact_input(&tc.input));
                }
            }
            // Hardening priority #6 — surface the silent-empty path. The
            // 2026-05-02 outage: HRM grew past ~1000 wavefronts, the
            // recall-laden system prompt blew Anthropic's input ceiling,
            // the response had zero Text blocks, and we printed empty
            // stdout + exited 0. Downstream consumers (radio's oration
            // path) couldn't tell that from a successful empty-string
            // generation. Now: surface the empty case explicitly so it
            // can't hide.
            if result.text.is_empty() {
                eprintln!(
                    "agent warning: empty response (no Text content blocks). \
                    Likely causes: bloated HRM (>1000 memories) blew the \
                    input ceiling, model returned only tool_use blocks \
                    that weren't wired, or upstream truncation. \
                    Check `kannaka observe` and consider `kannaka dream`."
                );
                process::exit(2);
            }
            println!("{}", result.text);
        }
        Err(e) => {
            eprintln!("agent error: {e}");
            process::exit(1);
        }
    }
}

// ── ADR-0026 Phase 1: remote ask routing over NATS ─────────────────────────

#[cfg(feature = "nats")]
pub(crate) fn handle_ask_remote(
    cfg: &KannakaConfig,
    target: &str,
    prompt: &str,
    recall_query: Option<&str>,
    no_tools: bool,
    timeout_secs: u64,
    quiet_tools: bool,
) {
    use std::time::Duration;
    let nats_url = if !cfg.swarm.nats_url.is_empty() {
        cfg.swarm.nats_url.clone()
    } else {
        std::env::var("KANNAKA_NATS_URL")
            .unwrap_or_else(|_| "nats://swarm.ninja-portal.com:4222".to_string())
    };
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("Failed to connect to NATS at {}: {}", nats_url, e); process::exit(1); }
    };

    let request = serde_json::json!({
        "from": cfg.agent.id,
        "text": prompt,
        "recall_query": recall_query,
        "no_tools": no_tools,
    });
    let payload = match serde_json::to_vec(&request) {
        Ok(b) => b,
        Err(e) => { eprintln!("serialize: {e}"); process::exit(1); }
    };

    let timeout = Duration::from_secs(timeout_secs);
    if target == "broadcast" {
        let subject = "KANNAKA.ask.broadcast";
        eprintln!("[ask --remote broadcast] published; collecting replies for {}s...", timeout_secs);
        match transport.request_many(subject, &payload, timeout) {
            Ok(replies) => {
                if replies.is_empty() {
                    eprintln!("(no replies within {}s)", timeout_secs);
                    process::exit(2);
                }
                for (i, reply) in replies.iter().enumerate() {
                    let parsed: serde_json::Value = serde_json::from_slice(reply)
                        .unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(reply)}));
                    let from = parsed.get("from").and_then(|v| v.as_str()).unwrap_or("?");
                    let text = parsed.get("text").and_then(|v| v.as_str())
                        .unwrap_or("(no text)");
                    if !quiet_tools {
                        eprintln!("─── reply {} from {} ───", i + 1, from);
                    }
                    println!("{}", text);
                    if !quiet_tools && i + 1 < replies.len() { println!(); }
                }
            }
            Err(e) => { eprintln!("request_many: {e}"); process::exit(1); }
        }
    } else {
        let subject = format!("KANNAKA.ask.{}", target);
        match transport.request_one(&subject, &payload, timeout) {
            Ok(reply) => {
                let parsed: serde_json::Value = serde_json::from_slice(&reply)
                    .unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(&reply)}));
                let text = parsed.get("text").and_then(|v| v.as_str())
                    .unwrap_or("(no text)");
                println!("{}", text);
            }
            Err(e) => { eprintln!("request_one: {e}"); process::exit(1); }
        }
    }
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_ask_remote(_: &KannakaConfig, _: &str, _: &str, _: Option<&str>, _: bool, _: u64, _: bool) {
    eprintln!("--remote requires the 'nats' feature");
    process::exit(1);
}
