//! `kannaka swarm` handlers — peer discovery, exemplar broadcast,
//! cross-agent absorb, work-queue serve/worker, and the directed/broadcast
//! `kannaka ask` serve loop (ADR-0026).
//!
//! Extracted from `bin/kannaka.rs` in v0.3.29 following the pattern
//! documented in `handlers/substrate.rs`. The largest single extraction
//! this session — moves ~830 lines including the 8 public swarm
//! handlers and 3 private helpers (_serve_directed_only,
//! _handle_serve_msg, _process_work_msg) that are only called from
//! within this module.

use std::process;

use super::{data_dir, resolve_nats_url, KannakaConfig};
#[cfg(feature = "nats")]
use super::{flag_value, parse_flag_value};

#[cfg(feature = "nats")]
use kannaka_memory::nats::SubEvent;

/// Stderr warning for unrecognized `--flags` in non-text handlers. Unknown
/// flags used to be silently swallowed by `_ => i += 1` arms, so typos like
/// `--thresold 0.5` vanished without a trace.
#[cfg(feature = "nats")]
fn warn_unknown_flag(ctx: &str, arg: &str) {
    if arg.starts_with("--") {
        eprintln!("[{ctx}] ignoring unknown flag: {arg}");
    }
}

#[cfg(feature = "nats")]
pub(crate) fn handle_swarm_serve(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    args: &[String],
) {
    use std::time::Duration;
    const USAGE: &str = "Usage: kannaka swarm serve [--threshold 0.4] [--nats-url URL] [--agent-id ID]";
    // Single-writer policy: the serve daemon is a long-running READER. It
    // observes/recalls (which mutate the medium in RAM) but must never
    // flush over the sole writer's .hrm. Enforce read-only here instead of
    // trusting the operator to export KANNAKA_READONLY.
    std::env::set_var("KANNAKA_READONLY", "1");
    if let Some(hrm) = sys
        .engine
        .store
        .as_any_mut()
        .downcast_mut::<kannaka_memory::hrm_store::HrmStore>()
    {
        hrm.set_readonly(true);
    }
    eprintln!("[swarm serve] read-only mode enforced (single-writer policy)");

    // Parse: kannaka swarm serve [--threshold 0.4] [--nats-url ...] [--agent-id ...]
    let mut threshold: f32 = 0.4;
    let mut agent_id_override: Option<String> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--threshold" => {
                threshold = parse_flag_value(args, i, "--threshold", USAGE);
                i += 2;
            }
            "--nats-url" => { let _ = flag_value(args, i, "--nats-url", USAGE); i += 2; }
            "--agent-id" => {
                agent_id_override = Some(flag_value(args, i, "--agent-id", USAGE).to_string());
                i += 2;
            }
            other => { warn_unknown_flag("swarm serve", other); i += 1; }
        }
    }
    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("Failed to connect to NATS at {}: {}", nats_url, e); process::exit(1); }
    };
    let agent_id = agent_id_override.unwrap_or_else(|| cfg.agent.id.clone());
    let directed = format!("KANNAKA.ask.{}", agent_id);

    eprintln!("[swarm serve] agent_id={agent_id}");
    eprintln!("[swarm serve] subscribing to {} and KANNAKA.ask.broadcast", directed);
    eprintln!("[swarm serve] broadcast resonance threshold: {:.2}", threshold);
    eprintln!("[swarm serve] press Ctrl+C to stop");

    // Single subscription per subject; in v1 we run them sequentially via
    // a switch, accepting that a busy listener processes one ask at a time.
    // Future: dedicated reader thread + channel per subject.
    let mut directed_sub = match transport.subscribe(&directed) {
        Ok(s) => s,
        Err(e) => { eprintln!("subscribe directed: {e}"); process::exit(1); }
    };
    // Short read timeout so the loop multiplexes all subjects responsively.
    // (Was 5s — with directed+broadcast+recall round-robined, a 5s timeout meant
    // the recall sub was only polled every ~12s, making daemon-served recall
    // slower than local. 250ms keeps recall sub-second; idle cost is a blocking
    // read, near-zero CPU.)
    let _ = directed_sub.set_timeout(Some(Duration::from_millis(250)));

    // Broadcast on a separate connection so the directed sub doesn't block it.
    let bcast_transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[swarm serve] WARN: broadcast subscription unavailable: {e}");
            // Continue with directed-only (never returns).
            _serve_directed_only(sys, cfg, &transport, directed_sub, &agent_id, &nats_url)
        }
    };
    let mut bcast_sub = match bcast_transport.subscribe("KANNAKA.ask.broadcast") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[swarm serve] WARN: broadcast subscribe failed: {e}");
            // Never returns.
            _serve_directed_only(sys, cfg, &transport, directed_sub, &agent_id, &nats_url)
        }
    };
    let _ = bcast_sub.set_timeout(Some(Duration::from_millis(250)));

    // Daemon-served recall (KANNAKA.recall.<agent_id>): the observatory, OBC
    // pulses, and the radio DJ can recall against this agent's warm in-memory
    // HRM instead of paying a 21 MB load + full xi-rerank per CLI call on the
    // 1-vCPU box. Uses the attention-beam prefilter + recall_with_beam
    // (O(beam), sub-second) — the same path the substrate responder uses. swarm
    // serve runs read-only (enforced above), so the observation mutation never
    // persists.
    let recall_subject = format!("KANNAKA.recall.{}", agent_id);
    let recall_transport = kannaka_memory::nats::SwarmTransport::connect(&nats_url).ok();
    let mut recall_sub = recall_transport
        .as_ref()
        .and_then(|t| t.subscribe(&recall_subject).ok());
    match recall_sub.as_mut() {
        Some(s) => { let _ = s.set_timeout(Some(Duration::from_millis(250))); eprintln!("[swarm serve] serving recall on {recall_subject}"); }
        None => eprintln!("[swarm serve] WARN: recall responder unavailable (extra NATS connection failed)"),
    }

    loop {
        // Round-robin: try directed first, then broadcast. Timeout means
        // "nothing right now — poll the next subject"; Closed means the
        // socket is dead and the loop must NOT spin on it (pre-fix this
        // burned 100% CPU forever after a NATS restart).
        match directed_sub.next_event() {
            SubEvent::Msg(msg) => {
                _handle_serve_msg(sys, cfg, &transport, &msg, /*is_broadcast*/ false, threshold, &agent_id, &nats_url);
            }
            SubEvent::Timeout => {}
            SubEvent::Closed => {
                eprintln!("[swarm serve] directed subscription ({directed}) closed — exiting for restart");
                process::exit(1);
            }
        }
        match bcast_sub.next_event() {
            SubEvent::Msg(msg) => {
                _handle_serve_msg(sys, cfg, &bcast_transport, &msg, /*is_broadcast*/ true, threshold, &agent_id, &nats_url);
            }
            SubEvent::Timeout => {}
            SubEvent::Closed => {
                eprintln!("[swarm serve] broadcast subscription closed — exiting for restart");
                process::exit(1);
            }
        }
        // Daemon-served recall — reply with the agent's own memories (full content).
        let mut recall_closed = false;
        if let Some(ref mut rsub) = recall_sub {
            match rsub.next_event() {
                SubEvent::Timeout => {}
                SubEvent::Closed => {
                    // Best-effort responder — degrade to ask-only (mirrors
                    // the startup behavior when the extra connection fails).
                    eprintln!("[swarm serve] WARN: recall subscription closed — continuing without recall responder");
                    recall_closed = true;
                }
                SubEvent::Msg(msg) => {
                    let reply_to = msg.reply_to.clone();
                    let req: serde_json::Value = serde_json::from_slice(&msg.payload).unwrap_or(serde_json::Value::Null);
                    let query = req.get("query").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let top_k = req.get("top_k").and_then(|v| v.as_u64()).unwrap_or(8) as usize;
                    if let (false, Some(reply)) = (query.is_empty(), reply_to) {
                        let beam = kannaka_memory::agent::attention_beam_for_prompt(
                            sys, &query, kannaka_memory::agent::DEFAULT_ATTENTION_BEAM);
                        let recall = if beam.is_empty() {
                            sys.recall(&query, top_k)            // cold/small HRM fallback
                        } else {
                            sys.recall_with_beam(&beam, &query, top_k)
                        };
                        let results: Vec<serde_json::Value> = recall.map(|rs| rs.iter().map(|r| serde_json::json!({
                            "id": r.id.to_string(),
                            "content": r.content,
                            "similarity": r.similarity,
                            "strength": r.strength,
                            "age_hours": r.age_hours,
                        })).collect()).unwrap_or_default();
                        if let Some(ref rt) = recall_transport {
                            let payload = serde_json::json!({ "from": agent_id, "query": query, "results": results });
                            let _ = rt.reply(&reply, payload.to_string().as_bytes());
                        }
                    }
                }
            }
        }
        if recall_closed {
            recall_sub = None;
        }
    }
}

#[cfg(feature = "nats")]
fn _serve_directed_only(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    transport: &kannaka_memory::nats::SwarmTransport,
    mut sub: kannaka_memory::nats::NatsSubscription,
    serve_agent_id: &str,
    nats_url: &str,
) -> ! {
    // The directed sub already carries a short read timeout from the main
    // setup path. Pre-fix this loop used `while let Some(...) = next_message()`
    // which treated that timeout as connection close and silently exited
    // (status 0) within ~250ms of idle.
    loop {
        match sub.next_event() {
            SubEvent::Msg(msg) => {
                _handle_serve_msg(sys, cfg, transport, &msg, false, 0.0, serve_agent_id, nats_url);
            }
            SubEvent::Timeout => {}
            SubEvent::Closed => {
                eprintln!("[swarm serve] directed subscription closed — exiting for restart");
                process::exit(1);
            }
        }
    }
}

#[cfg(feature = "nats")]
#[allow(clippy::too_many_arguments)]
fn _handle_serve_msg(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    transport: &kannaka_memory::nats::SwarmTransport,
    msg: &kannaka_memory::nats::NatsMessage,
    is_broadcast: bool,
    threshold: f32,
    serve_agent_id: &str,
    nats_url: &str,
) {
    let reply_to = match &msg.reply_to {
        Some(r) => r.clone(),
        None => {
            eprintln!("[swarm serve] msg without reply-to on {} — ignoring", msg.subject);
            return;
        }
    };

    let req: serde_json::Value = match serde_json::from_slice(&msg.payload) {
        Ok(v) => v,
        Err(e) => {
            let err = serde_json::json!({ "from": serve_agent_id, "error": format!("bad json: {e}") });
            let _ = transport.reply(&reply_to, err.to_string().as_bytes());
            return;
        }
    };
    let from = req.get("from").and_then(|v| v.as_str()).unwrap_or("?");
    let text = req.get("text").and_then(|v| v.as_str()).unwrap_or("");
    let recall_q = req.get("recall_query").and_then(|v| v.as_str());

    if text.is_empty() {
        let err = serde_json::json!({ "from": serve_agent_id, "error": "empty text" });
        let _ = transport.reply(&reply_to, err.to_string().as_bytes());
        return;
    }

    // Self-throttle on broadcast: only reply if local recall has resonance ≥ threshold.
    if is_broadcast {
        let probe = recall_q.unwrap_or(text);
        let res = sys.recall(probe, 1).unwrap_or_default();
        let top = res.first().map(|r| r.strength).unwrap_or(0.0);
        if top < threshold {
            eprintln!("[swarm serve] broadcast from {from}: top resonance {top:.3} < threshold {threshold:.2} — staying quiet");
            return;
        }
        eprintln!("[swarm serve] broadcast from {from}: top resonance {top:.3} ≥ threshold — answering");
    } else {
        eprintln!("[swarm serve] directed from {from}");
    }

    let result = kannaka_memory::agent::ask_notools_ex(sys, cfg, text, recall_q);
    // "from" is the id this serve loop is actually answering as (the
    // --agent-id override when given), not unconditionally cfg.agent.id.
    let reply = match result {
        Ok(r) => serde_json::json!({ "from": serve_agent_id, "text": r.text }),
        Err(e) => serde_json::json!({ "from": serve_agent_id, "error": format!("{e}") }),
    };

    // Heavy ask calls take 3–5 min. The original NATS connection has been
    // idle that whole time and the server typically closes it on PING
    // timeout. Open a fresh connection just to send the reply so the
    // long-running subscription on the parent transport doesn't have to
    // be reconnected on every request. The fresh connection reuses the
    // SAME resolved URL the serve loop connected with (pre-fix it
    // re-resolved from env with a 127.0.0.1 fallback, so a --nats-url
    // serve could reply into the wrong broker).
    let reply_payload = reply.to_string();
    let reply_result = match kannaka_memory::nats::SwarmTransport::connect(nats_url) {
        Ok(fresh) => fresh.reply(&reply_to, reply_payload.as_bytes()),
        Err(e) => Err(e),
    };
    // Fall back to the original transport if the fresh connect failed.
    if reply_result.is_err() {
        if let Err(e2) = transport.reply(&reply_to, reply_payload.as_bytes()) {
            eprintln!("[swarm serve] reply failed (fresh + fallback): {e2}");
            return;
        }
    }
    eprintln!("[swarm serve] replied on {reply_to}");
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_swarm_serve(_: &mut kannaka_memory::openclaw::KannakaMemorySystem, _: &KannakaConfig, _: &[String]) {
    eprintln!("swarm serve requires the 'nats' feature");
    process::exit(1);
}

// ── ADR-0026 Phase 2: Exemplar broadcast (#72) ─────────────────────────────

#[cfg(feature = "nats")]
pub(crate) fn handle_swarm_exemplars(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    args: &[String],
) {
    const USAGE: &str = "Usage: kannaka swarm exemplars <publish|list> [--top-k N] [--agent-id ID] [--from <agent>] [--nats-url URL]";
    // Usage:
    //   kannaka swarm exemplars publish [--top-k N] [--agent-id ID]
    //   kannaka swarm exemplars list [--from <agent_id>] [--top-k N]
    let sub = args.get(2).map(|s| s.as_str()).unwrap_or("publish");
    let mut top_k: usize = 20;
    let mut agent_id_override: Option<String> = None;
    let mut from: Option<String> = None;
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--top-k" => { top_k = parse_flag_value(args, i, "--top-k", USAGE); i += 2; }
            "--agent-id" => { agent_id_override = Some(flag_value(args, i, "--agent-id", USAGE).to_string()); i += 2; }
            "--from" => { from = Some(flag_value(args, i, "--from", USAGE).to_string()); i += 2; }
            "--nats-url" => { let _ = flag_value(args, i, "--nats-url", USAGE); i += 2; }
            other => { warn_unknown_flag("exemplars", other); i += 1; }
        }
    }
    let agent_id = agent_id_override.unwrap_or_else(|| cfg.agent.id.clone());
    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("nats: {e}"); process::exit(1); }
    };

    match sub {
        "publish" => {
            // Make sure the stream exists. Best-effort — JetStream may already
            // have it from a previous run.
            let _ = transport.ensure_exemplar_stream();

            let report = sys.observe();
            let mut clusters = report.clusters.clusters.clone();
            // Order by mean_amplitude desc — strongest exemplars first.
            clusters.sort_by(|a, b| b.mean_amplitude.partial_cmp(&a.mean_amplitude)
                .unwrap_or(std::cmp::Ordering::Equal));
            let mut published = 0;
            for c in clusters.iter().take(top_k) {
                let payload = serde_json::json!({
                    "agent_id": agent_id,
                    "cluster_id": c.cluster_id,
                    "size": c.size,
                    "content": c.exemplar_content,
                    "exemplar_id": c.exemplar_id,
                    "amplitude": c.mean_amplitude,
                    "frequency": c.mean_frequency,
                    "phase": c.mean_phase,
                    "modality": c.dominant_modality,
                    "theme": c.theme,
                    "semantic_summary": c.semantic_summary,
                    "coherence": c.coherence,
                    "xi_diversity": c.xi_diversity,
                    "created_at": chrono::Utc::now().to_rfc3339(),
                });
                match transport.publish_exemplar(&agent_id, c.cluster_id, &payload) {
                    Ok(()) => published += 1,
                    Err(e) => eprintln!("[exemplars] cluster {} publish failed: {e}", c.cluster_id),
                }
            }
            println!("Published {} exemplars from {} (top-{} by amplitude)", published, agent_id, top_k);
        }
        "list" => {
            let exemplars = match transport.get_exemplars(from.as_deref()) {
                Ok(e) => e,
                Err(e) => { eprintln!("nats: {e}"); process::exit(1); }
            };
            let limit = if top_k == 0 { exemplars.len() } else { top_k };
            for (i, e) in exemplars.iter().take(limit).enumerate() {
                let agent = e.get("agent_id").and_then(|v| v.as_str()).unwrap_or("?");
                let cid = e.get("cluster_id").and_then(|v| v.as_u64()).unwrap_or(0);
                let amp = e.get("amplitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let content = e.get("content").and_then(|v| v.as_str()).unwrap_or("(no content)");
                let preview: String = content.chars().take(120).collect();
                println!("[{:3}] {} c{:<3} amp={:.3}", i + 1, agent, cid, amp);
                println!("       {}", preview);
            }
            println!();
            println!("Total: {} exemplars", exemplars.len());
        }
        other => {
            eprintln!("{USAGE}");
            eprintln!("Unknown subcommand: {other}");
            process::exit(1);
        }
    }
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_swarm_exemplars(_: &mut kannaka_memory::openclaw::KannakaMemorySystem, _: &KannakaConfig, _: &[String]) {
    eprintln!("swarm exemplars requires the 'nats' feature"); process::exit(1);
}

#[cfg(feature = "nats")]
pub(crate) fn handle_swarm_absorb(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    args: &[String],
) {
    const USAGE: &str = "Usage: kannaka swarm absorb [--from <agent>] [--top-k N] [--threshold X] [--dry-run] [--nats-url URL]";
    // Usage: kannaka swarm absorb [--from <agent>] [--top-k N] [--threshold X] [--dry-run]
    let mut from: Option<String> = None;
    let mut top_k: usize = 50;
    let mut threshold: f32 = 0.4;
    let mut dry_run = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--from" => { from = Some(flag_value(args, i, "--from", USAGE).to_string()); i += 2; }
            "--top-k" => { top_k = parse_flag_value(args, i, "--top-k", USAGE); i += 2; }
            "--threshold" => { threshold = parse_flag_value(args, i, "--threshold", USAGE); i += 2; }
            "--dry-run" => { dry_run = true; i += 1; }
            "--nats-url" => { let _ = flag_value(args, i, "--nats-url", USAGE); i += 2; }
            other => { warn_unknown_flag("swarm absorb", other); i += 1; }
        }
    }
    if !dry_run {
        super::warn_if_readonly("swarm absorb");
    }

    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("nats: {e}"); process::exit(1); }
    };

    let exemplars = match transport.get_exemplars(from.as_deref()) {
        Ok(e) => e,
        Err(e) => { eprintln!("nats: {e}"); process::exit(1); }
    };
    if exemplars.is_empty() {
        eprintln!("No exemplars found in stream{}.",
            from.as_ref().map(|f| format!(" (from {})", f)).unwrap_or_default());
        eprintln!("Hint: a peer must run 'kannaka swarm exemplars publish' first.");
        return;
    }
    eprintln!("Found {} exemplars; evaluating against local medium (threshold {:.2})...",
        exemplars.len(), threshold);

    let mut absorbed = 0usize;
    let mut skipped_threshold = 0usize;
    let mut skipped_self = 0usize;
    let my_id = &cfg.agent.id;

    // Sort by amplitude descending so we evaluate the strongest first.
    let mut ordered = exemplars;
    ordered.sort_by(|a, b| {
        let aa = a.get("amplitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let bb = b.get("amplitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
        bb.partial_cmp(&aa).unwrap_or(std::cmp::Ordering::Equal)
    });

    for e in ordered.iter().take(top_k) {
        let source = e.get("agent_id").and_then(|v| v.as_str()).unwrap_or("?");
        if source == my_id {
            skipped_self += 1;
            continue;
        }
        let content = e.get("content").and_then(|v| v.as_str()).unwrap_or("");
        if content.is_empty() { continue; }

        // Resonance against the local medium — compute by recall-and-check
        // the top result's strength.
        let res = sys.recall(content, 1).ok().unwrap_or_default();
        let top_strength = res.first().map(|r| r.strength).unwrap_or(0.0);
        let cluster_id = e.get("cluster_id").and_then(|v| v.as_u64()).unwrap_or(0);

        if top_strength >= threshold {
            // Already in our medium with high resonance — skip duplicates.
            // (The match suggests we've heard this before.)
            eprintln!("  ✓ already resonant: {} c{} strength={:.3} — skip",
                source, cluster_id, top_strength);
            continue;
        }

        // Low local resonance + non-trivial content = candidate for absorption.
        // The absorb threshold inverts: we want NEW material that's distinctive,
        // not already present. But we also don't want noise. Use a min content
        // length + presence of metadata as a soft filter.
        let amp = e.get("amplitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if amp < 0.3 || content.len() < 30 {
            skipped_threshold += 1;
            continue;
        }

        eprintln!("  + new wavefront: {} c{} amp={:.3} resonance={:.3}",
            source, cluster_id, amp, top_strength);
        eprintln!("      \"{}\"", &content.chars().take(120).collect::<String>());

        if !dry_run {
            // Tag with provenance so we can identify swarm-origin memories later.
            let category = format!("swarm:{}", source);
            match sys.remember_with_category(content, &category, amp.min(0.95)) {
                Ok(id) => { eprintln!("      remembered as {}", id); absorbed += 1; }
                Err(e) => { eprintln!("      remember failed: {e}"); }
            }
        } else {
            absorbed += 1;
        }
    }

    println!();
    println!("Absorb complete{}:", if dry_run { " (DRY RUN)" } else { "" });
    println!("  absorbed:    {}", absorbed);
    println!("  skipped (threshold/length): {}", skipped_threshold);
    println!("  skipped (self-origin):      {}", skipped_self);
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_swarm_absorb(_: &mut kannaka_memory::openclaw::KannakaMemorySystem, _: &KannakaConfig, _: &[String]) {
    eprintln!("swarm absorb requires the 'nats' feature"); process::exit(1);
}

#[cfg(feature = "nats")]
pub(crate) fn handle_swarm_peers(cfg: &KannakaConfig, args: &[String]) {
    const USAGE: &str = "Usage: kannaka swarm peers [--json] [--nats-url URL]";
    // Usage: kannaka swarm peers [--json]
    let mut as_json = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => { as_json = true; i += 1; }
            "--nats-url" => { let _ = flag_value(args, i, "--nats-url", USAGE); i += 2; }
            other => { warn_unknown_flag("swarm peers", other); i += 1; }
        }
    }
    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("nats: {e}"); process::exit(1); }
    };
    let peers = match transport.get_presence() {
        Ok(p) => p,
        Err(e) => { eprintln!("get_presence: {e}"); process::exit(1); }
    };
    if as_json {
        println!("{}", serde_json::to_string_pretty(&peers).unwrap_or_default());
        return;
    }
    if peers.is_empty() {
        println!("No peers in the swarm yet.");
        println!("Hint: peers register via 'kannaka swarm join'.");
        return;
    }
    println!();
    println!("{:<24} {:<8} {:<8} {}", "AGENT", "MEMS", "VERSION", "CAPABILITIES");
    println!("{}", "─".repeat(78));
    for p in &peers {
        let agent = p.get("agent_id").and_then(|v| v.as_str()).unwrap_or("?");
        let display = p.get("display_name").and_then(|v| v.as_str()).unwrap_or("");
        let mem = p.get("memory_count").and_then(|v| v.as_u64()).unwrap_or(0);
        let ver = p.get("kannaka_version").and_then(|v| v.as_str()).unwrap_or("?");
        let caps_obj = p.get("capabilities").and_then(|v| v.as_object());
        let caps = caps_obj
            .map(|o| o.iter().filter_map(|(k, v)| if v.as_bool() == Some(true) { Some(k.as_str()) } else { None })
                .collect::<Vec<_>>().join(","))
            .unwrap_or_default();
        let label = if display.is_empty() || display == agent {
            agent.to_string()
        } else {
            format!("{} ({})", display, agent)
        };
        println!("{:<24} {:<8} {:<8} {}", label, mem, ver, caps);
    }
    println!();
    println!("{} peers", peers.len());
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_swarm_peers(_: &KannakaConfig, _: &[String]) {
    eprintln!("swarm peers requires the 'nats' feature"); process::exit(1);
}

// ── ADR-0026 Phase 6: Auto-absorb (#76) ─────────────────────────────────────
// One-shot autonomous sweep with rate limits + anti-drift safety. Designed
// to be invoked from cron every 30 min (or in-process loop) so a node that
// stays online keeps absorbing fresh exemplars from peers.

#[cfg(feature = "nats")]
pub(crate) fn handle_swarm_autoabsorb(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    args: &[String],
) {
    const USAGE: &str = "Usage: kannaka swarm autoabsorb [--threshold 0.4] [--per-source-daily-cap N] [--max-phi-drop X] [--dry-run] [--nats-url URL]";
    // Usage: kannaka swarm autoabsorb [--threshold 0.4] [--per-source-daily-cap N] [--dry-run]
    let mut threshold: f32 = 0.4;
    let mut per_source_daily_cap: usize = 10;
    let mut dry_run = false;
    let mut max_phi_drop: f32 = 0.05;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--threshold" => { threshold = parse_flag_value(args, i, "--threshold", USAGE); i += 2; }
            "--per-source-daily-cap" => { per_source_daily_cap = parse_flag_value(args, i, "--per-source-daily-cap", USAGE); i += 2; }
            "--max-phi-drop" => { max_phi_drop = parse_flag_value(args, i, "--max-phi-drop", USAGE); i += 2; }
            "--dry-run" => { dry_run = true; i += 1; }
            "--nats-url" => { let _ = flag_value(args, i, "--nats-url", USAGE); i += 2; }
            other => { warn_unknown_flag("autoabsorb", other); i += 1; }
        }
    }
    if !dry_run {
        super::warn_if_readonly("swarm autoabsorb");
    }

    let state_path = data_dir().join("autoabsorb-state.json");
    let mut state = AutoabsorbState::load(&state_path);
    let today_key = chrono::Utc::now().format("%Y-%m-%d").to_string();
    state.purge_old_days(&today_key);

    // Anti-drift safety: compare current Phi to the snapshot the last sweep
    // recorded. If Phi has dropped > max_phi_drop, pause autonomous absorb.
    let current_phi = sys.assess().phi;
    if let Some(prev) = state.last_phi {
        let drop = prev - current_phi;
        if drop > max_phi_drop {
            eprintln!("[autoabsorb] PAUSED: Phi dropped {:.3} → {:.3} (Δ={:.3} > {:.3})",
                prev, current_phi, drop, max_phi_drop);
            eprintln!("[autoabsorb] manual intervention required: review recent absorbs in {}", state_path.display());
            return;
        }
    }

    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("[autoabsorb] nats: {e}"); return; }
    };

    let exemplars = match transport.get_exemplars(None) {
        Ok(e) => e,
        Err(e) => { eprintln!("[autoabsorb] get_exemplars: {e}"); return; }
    };
    if exemplars.is_empty() {
        eprintln!("[autoabsorb] no exemplars in stream — nothing to do");
        state.last_phi = Some(current_phi);
        let _ = state.save(&state_path);
        return;
    }

    // Sort by amplitude desc.
    let mut ordered = exemplars;
    ordered.sort_by(|a, b| {
        let aa = a.get("amplitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let bb = b.get("amplitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
        bb.partial_cmp(&aa).unwrap_or(std::cmp::Ordering::Equal)
    });

    let my_id = &cfg.agent.id;
    let mut absorbed = 0usize;
    let mut skipped_self = 0usize;
    let mut skipped_capped = 0usize;
    let mut skipped_resonant = 0usize;
    let mut skipped_low = 0usize;

    for e in ordered.iter() {
        let source = e.get("agent_id").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        if source == *my_id { skipped_self += 1; continue; }

        // Per-source per-day cap.
        let used_today = state.absorbs_today(&today_key, &source);
        if used_today >= per_source_daily_cap {
            skipped_capped += 1;
            continue;
        }

        let content = e.get("content").and_then(|v| v.as_str()).unwrap_or("");
        if content.is_empty() || content.len() < 30 { skipped_low += 1; continue; }
        let amp = e.get("amplitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if amp < 0.3 { skipped_low += 1; continue; }

        // Local resonance — only absorb if the medium DOESN'T already have
        // strong resonance (i.e. it's novel material).
        let res = sys.recall(content, 1).ok().unwrap_or_default();
        let top_strength = res.first().map(|r| r.strength).unwrap_or(0.0);
        if top_strength >= threshold {
            skipped_resonant += 1;
            continue;
        }

        let cluster_id = e.get("cluster_id").and_then(|v| v.as_u64()).unwrap_or(0);
        eprintln!("[autoabsorb] absorb from {} c{} amp={:.3} resonance={:.3}", source, cluster_id, amp, top_strength);

        if !dry_run {
            let category = format!("swarm:{}", source);
            match sys.remember_with_category(content, &category, amp.min(0.95)) {
                Ok(id) => {
                    eprintln!("[autoabsorb]   remembered {}", id);
                    state.record_absorb(&today_key, &source);
                    absorbed += 1;
                }
                Err(e) => eprintln!("[autoabsorb]   remember failed: {e}"),
            }
        } else {
            absorbed += 1;
            state.record_absorb(&today_key, &source);
        }
    }

    state.last_phi = Some(current_phi);
    if let Err(e) = state.save(&state_path) {
        eprintln!("[autoabsorb] state save failed: {e}");
    }

    eprintln!("[autoabsorb] sweep complete{}: +{} absorbed (self={} capped={} resonant={} low={})",
        if dry_run { " (DRY RUN)" } else { "" },
        absorbed, skipped_self, skipped_capped, skipped_resonant, skipped_low);
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_swarm_autoabsorb(_: &mut kannaka_memory::openclaw::KannakaMemorySystem, _: &KannakaConfig, _: &[String]) {
    eprintln!("swarm autoabsorb requires the 'nats' feature"); process::exit(1);
}

/// State persisted across autoabsorb sweeps. Tracks daily absorb counts per
/// source agent and the last seen local Phi (for anti-drift detection).
#[cfg(feature = "nats")]
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct AutoabsorbState {
    /// `{ "2026-04-25": { "kannaka-prime": 3, "agent-x": 1 } }`
    absorbs_per_day: std::collections::HashMap<String, std::collections::HashMap<String, usize>>,
    last_phi: Option<f32>,
}

#[cfg(feature = "nats")]
impl AutoabsorbState {
    fn load(path: &std::path::Path) -> Self {
        if !path.exists() { return Self::default(); }
        match std::fs::read_to_string(path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }
    fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, bytes)
    }
    fn absorbs_today(&self, day: &str, source: &str) -> usize {
        self.absorbs_per_day.get(day).and_then(|d| d.get(source)).copied().unwrap_or(0)
    }
    fn record_absorb(&mut self, day: &str, source: &str) {
        let day_map = self.absorbs_per_day.entry(day.to_string()).or_default();
        *day_map.entry(source.to_string()).or_insert(0) += 1;
    }
    /// Drop entries older than 7 days so the state file doesn't grow unbounded.
    fn purge_old_days(&mut self, today: &str) {
        let cutoff = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d")
            .ok()
            .map(|d| d - chrono::Duration::days(7))
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "1970-01-01".to_string());
        self.absorbs_per_day.retain(|k, _| k.as_str() >= cutoff.as_str());
    }
}

// ── ADR-0026 Phase 4: Work queues (#74) ─────────────────────────────────────
//
// Cooperative task processing across the swarm. v1 uses raw NATS queue-group
// subscriptions (not full JetStream consumer groups) — multiple workers
// SUB to `KANNAKA.work.<kind>` with the same queue group; NATS delivers
// each task to exactly one worker. The requester pubs with a reply-to and
// awaits the result via existing request_one. Same wire shape as ask/serve;
// the difference is the queue-group on the worker side.
//
// Supported task kinds:
//   ask  — runs agent::ask_notools_ex on the worker's local HRM.
// Future: dream.deep, batch HRM analysis, TTS pool. Each kind gets its own
// subject + queue group.

#[cfg(feature = "nats")]
pub(crate) fn handle_swarm_enqueue(cfg: &KannakaConfig, args: &[String]) {
    use std::time::Duration;
    const USAGE: &str = "Usage: kannaka swarm enqueue <kind> \"payload\" [--timeout SECONDS] [--nats-url URL]";
    // Usage: kannaka swarm enqueue <kind> "payload text" [--timeout 600]
    let kind = match args.get(2) {
        Some(s) => s.clone(),
        None => { eprintln!("{USAGE}"); process::exit(1); }
    };
    let mut timeout_secs: u64 = 600;
    let mut text_parts: Vec<String> = Vec::new();
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--timeout" => {
                timeout_secs = parse_flag_value(args, i, "--timeout", USAGE);
                i += 2;
            }
            "--nats-url" => { let _ = flag_value(args, i, "--nats-url", USAGE); i += 2; }
            other if other.starts_with("--") => {
                // Payload-collecting handler: a typo'd flag must not silently
                // become part of the task text.
                eprintln!("swarm enqueue: unknown flag: {other}");
                eprintln!("{USAGE}");
                process::exit(2);
            }
            _ => { text_parts.push(args[i].clone()); i += 1; }
        }
    }
    let text = text_parts.join(" ").trim().to_string();
    if text.is_empty() {
        eprintln!("{USAGE}"); process::exit(1);
    }

    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("nats: {e}"); process::exit(1); }
    };

    let task_id = format!("t-{}", uuid::Uuid::new_v4().simple());
    let payload = serde_json::json!({
        "task_id": task_id,
        "from": cfg.agent.id,
        "text": text,
    });
    let bytes = serde_json::to_vec(&payload).unwrap();
    let subject = format!("KANNAKA.work.{}", kind);
    eprintln!("[enqueue] {} task {} (waiting up to {}s for a worker reply)", subject, task_id, timeout_secs);

    match transport.request_one(&subject, &bytes, Duration::from_secs(timeout_secs)) {
        Ok(reply) => {
            let parsed: serde_json::Value = serde_json::from_slice(&reply)
                .unwrap_or_else(|_| serde_json::json!({"raw": String::from_utf8_lossy(&reply).to_string()}));
            let from = parsed.get("from").and_then(|v| v.as_str()).unwrap_or("?");
            let text = parsed.get("text").and_then(|v| v.as_str()).unwrap_or("(no text)");
            eprintln!("[enqueue] reply from {}", from);
            println!("{}", text);
        }
        Err(e) => { eprintln!("enqueue: {e}"); process::exit(1); }
    }
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_swarm_enqueue(_: &KannakaConfig, _: &[String]) {
    eprintln!("swarm enqueue requires the 'nats' feature"); process::exit(1);
}

#[cfg(feature = "nats")]
pub(crate) fn handle_swarm_worker(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    args: &[String],
) {
    use std::time::Duration;
    const USAGE: &str = "Usage: kannaka swarm worker [--kinds ask,dream,...] [--queue-group GROUP] [--nats-url URL]";
    // Usage: kannaka swarm worker [--kinds ask,dream,...] [--queue-group GROUP]
    let mut kinds: Vec<String> = vec!["ask".to_string()];
    let mut queue_group: String = "kannaka_workers".to_string();
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--kinds" => {
                let v = flag_value(args, i, "--kinds", USAGE);
                kinds = v.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                i += 2;
            }
            "--queue-group" => {
                queue_group = flag_value(args, i, "--queue-group", USAGE).to_string();
                i += 2;
            }
            "--nats-url" => { let _ = flag_value(args, i, "--nats-url", USAGE); i += 2; }
            other => { warn_unknown_flag("worker", other); i += 1; }
        }
    }
    if kinds.is_empty() { kinds.push("ask".to_string()); }

    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);

    eprintln!("[worker] kinds: {:?}, queue group: {}", kinds, queue_group);

    if kinds.len() == 1 {
        let kind = &kinds[0];
        let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
            Ok(t) => t,
            Err(e) => { eprintln!("[worker] nats: {e}"); process::exit(1); }
        };
        let subject = format!("KANNAKA.work.{}", kind);
        let group = format!("{}_{}", queue_group, kind);
        let mut sub = match transport.subscribe_with_queue(&subject, Some(&group)) {
            Ok(s) => s,
            Err(e) => { eprintln!("[worker] subscribe: {e}"); process::exit(1); }
        };
        eprintln!("[worker] subscribed to {} (group {})", subject, group);
        loop {
            match sub.next_event() {
                SubEvent::Msg(msg) => {
                    _process_work_msg(sys, cfg, &transport, &nats_url, kind, &msg);
                }
                SubEvent::Timeout => {}
                SubEvent::Closed => {
                    // Pre-fix this exited 0 on connection close, so
                    // Restart=on-failure units never brought the worker back.
                    eprintln!("[worker] subscription {subject} closed — exiting for restart");
                    process::exit(1);
                }
            }
        }
    } else {
        // Multi-kind worker: ONE durable subscription per kind, each on a
        // dedicated connection (the transport documents a one-subscription-
        // per-connection model), polled round-robin with short timeouts.
        // Pre-fix this re-subscribed every ~5s on the same connection,
        // leaking server-side subscriptions and discarding bytes buffered
        // in each dropped reader.
        let mut subs: Vec<(String, kannaka_memory::nats::SwarmTransport, kannaka_memory::nats::NatsSubscription)> = Vec::new();
        for kind in &kinds {
            let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
                Ok(t) => t,
                Err(e) => { eprintln!("[worker] nats connect for kind {kind}: {e}"); process::exit(1); }
            };
            let subject = format!("KANNAKA.work.{}", kind);
            let group = format!("{}_{}", queue_group, kind);
            let sub = match transport.subscribe_with_queue(&subject, Some(&group)) {
                Ok(s) => s,
                Err(e) => { eprintln!("[worker] subscribe {kind}: {e}"); process::exit(1); }
            };
            let _ = sub.set_timeout(Some(Duration::from_millis(500)));
            eprintln!("[worker] subscribed to {} (group {})", subject, group);
            subs.push((kind.clone(), transport, sub));
        }
        loop {
            for (kind, transport, sub) in subs.iter_mut() {
                match sub.next_event() {
                    SubEvent::Msg(msg) => {
                        _process_work_msg(sys, cfg, transport, &nats_url, kind, &msg);
                    }
                    SubEvent::Timeout => {}
                    SubEvent::Closed => {
                        eprintln!("[worker] subscription KANNAKA.work.{kind} closed — exiting for restart");
                        process::exit(1);
                    }
                }
            }
        }
    }
}

#[cfg(feature = "nats")]
fn _process_work_msg(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    transport: &kannaka_memory::nats::SwarmTransport,
    nats_url: &str,
    kind: &str,
    msg: &kannaka_memory::nats::NatsMessage,
) {
    let reply_to = match &msg.reply_to {
        Some(r) => r.clone(),
        None => { eprintln!("[worker] task without reply-to on {} — drop", msg.subject); return; }
    };
    let req: serde_json::Value = match serde_json::from_slice(&msg.payload) {
        Ok(v) => v,
        Err(e) => {
            let err = serde_json::json!({"from": cfg.agent.id, "error": format!("bad json: {e}")});
            let _ = transport.reply(&reply_to, err.to_string().as_bytes());
            return;
        }
    };
    let task_id = req.get("task_id").and_then(|v| v.as_str()).unwrap_or("(none)");
    let from = req.get("from").and_then(|v| v.as_str()).unwrap_or("?");
    let text = req.get("text").and_then(|v| v.as_str()).unwrap_or("");
    eprintln!("[worker] kind={kind} task={task_id} from={from}");

    let reply_payload = match kind {
        "ask" => {
            if text.is_empty() {
                serde_json::json!({"from": cfg.agent.id, "error": "empty text"})
            } else {
                match kannaka_memory::agent::ask_notools_ex(sys, cfg, text, None) {
                    Ok(r) => serde_json::json!({"from": cfg.agent.id, "task_id": task_id, "text": r.text}),
                    Err(e) => serde_json::json!({"from": cfg.agent.id, "task_id": task_id, "error": format!("{e}")}),
                }
            }
        }
        other => serde_json::json!({"from": cfg.agent.id, "error": format!("unknown kind: {other}")}),
    };

    // Reply on a fresh connection (the original may have idled past PING).
    let body = reply_payload.to_string();
    let reply_result = match kannaka_memory::nats::SwarmTransport::connect(nats_url) {
        Ok(fresh) => fresh.reply(&reply_to, body.as_bytes()),
        Err(_) => transport.reply(&reply_to, body.as_bytes()),
    };
    if let Err(e) = reply_result {
        eprintln!("[worker] reply failed: {e}");
    } else {
        eprintln!("[worker] replied on {reply_to}");
    }
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_swarm_worker(_: &mut kannaka_memory::openclaw::KannakaMemorySystem, _: &KannakaConfig, _: &[String]) {
    eprintln!("swarm worker requires the 'nats' feature"); process::exit(1);
}

/// `kannaka swarm tail` — subscribe to the constellation pulse and emit
/// one NDJSON line per inbound NATS message. The default subject set is
/// `QUEEN.>`, `KANNAKA.>`, `RADIO.>`, `KAX.>`, `EYE.>` — the prefixes any
/// constellation node publishes on. `--subject` repeats override the set.
///
/// Output format (one line per message):
///   {"ts": <unix-ms>, "subject": "<subj>", "payload": <json-or-string>}
///
/// This is the streaming source the TUI Bus tab consumes — running it
/// from a shell is also useful: `kannaka swarm tail | grep consciousness`.
#[cfg(feature = "nats")]
pub(crate) fn handle_swarm_tail(cfg: &KannakaConfig, args: &[String]) {
    use std::io::Write;
    const USAGE: &str = "Usage: kannaka swarm tail [--subject SUBJ ...] [--nats-url URL]";

    let mut subjects: Vec<String> = Vec::new();
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--subject" => { subjects.push(flag_value(args, i, "--subject", USAGE).to_string()); i += 2; }
            "--nats-url" => { let _ = flag_value(args, i, "--nats-url", USAGE); i += 2; }
            other => { warn_unknown_flag("tail", other); i += 1; }
        }
    }
    if subjects.is_empty() {
        for s in ["QUEEN.>", "KANNAKA.>", "RADIO.>", "KAX.>", "EYE.>"] {
            subjects.push(s.to_string());
        }
    }

    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    eprintln!("[tail] connecting to {} — subjects: {:?}", nats_url, subjects);

    // One dedicated transport per subject — the transport documents a
    // one-subscription-per-connection model, so the cleanest way to
    // multiplex is one TCP socket per wildcard. Five sockets is cheap.
    let stdout_mu = std::sync::Arc::new(std::sync::Mutex::new(()));
    let mut handles = Vec::new();
    for subj in subjects {
        let url = nats_url.clone();
        let mu = std::sync::Arc::clone(&stdout_mu);
        handles.push(std::thread::spawn(move || loop {
            let transport = match kannaka_memory::nats::SwarmTransport::connect(&url) {
                Ok(t) => t,
                Err(e) => {
                    let _ = writeln!(std::io::stderr(), "[tail] {} connect failed: {} (retry 5s)", subj, e);
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    continue;
                }
            };
            let mut sub = match transport.subscribe(&subj) {
                Ok(s) => s,
                Err(e) => {
                    let _ = writeln!(std::io::stderr(), "[tail] {} subscribe failed: {} (retry 5s)", subj, e);
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    continue;
                }
            };
            // Block indefinitely; Ctrl+C terminates the process.
            let _ = sub.set_timeout(None);
            eprintln!("[tail] {} subscribed", subj);
            loop {
                match sub.next_event() {
                    SubEvent::Msg(msg) => {
                        let payload_str = std::str::from_utf8(&msg.payload).unwrap_or("<binary>");
                        let payload_json: serde_json::Value = serde_json::from_str(payload_str)
                            .unwrap_or_else(|_| serde_json::Value::String(payload_str.to_string()));
                        let line = serde_json::json!({
                            "ts": chrono::Utc::now().timestamp_millis(),
                            "subject": msg.subject,
                            "payload": payload_json,
                        });
                        let _guard = mu.lock();
                        println!("{}", line);
                        let _ = std::io::stdout().flush();
                    }
                    // No timeout is set; defensive — keep polling.
                    SubEvent::Timeout => continue,
                    // Connection closed — break to the reconnect path.
                    SubEvent::Closed => break,
                }
            }
            let _ = writeln!(std::io::stderr(), "[tail] {} disconnected — reconnecting in 2s", subj);
            std::thread::sleep(std::time::Duration::from_secs(2));
        }));
    }

    // Block forever — Ctrl+C kills the process. Joining the threads would
    // hang the same way, so just sleep the main thread.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_swarm_tail(_: &KannakaConfig, _: &[String]) {
    eprintln!("swarm tail requires the 'nats' feature"); std::process::exit(1);
}
