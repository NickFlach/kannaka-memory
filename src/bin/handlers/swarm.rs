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
pub(crate) fn handle_swarm_serve(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    args: &[String],
) {
    use std::time::Duration;
    // Parse: kannaka swarm serve [--threshold 0.4] [--nats-url ...] [--agent-id ...]
    let mut threshold: f32 = 0.4;
    let mut agent_id_override: Option<String> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--threshold" if i + 1 < args.len() => {
                threshold = args[i + 1].parse().unwrap_or(0.4); i += 2;
            }
            "--nats-url" if i + 1 < args.len() => { i += 2; }
            "--agent-id" if i + 1 < args.len() => {
                agent_id_override = Some(args[i + 1].clone()); i += 2;
            }
            _ => i += 1,
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
            // Continue with directed-only.
            return _serve_directed_only(sys, cfg, &transport, directed_sub);
        }
    };
    let mut bcast_sub = match bcast_transport.subscribe("KANNAKA.ask.broadcast") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[swarm serve] WARN: broadcast subscribe failed: {e}");
            return _serve_directed_only(sys, cfg, &transport, directed_sub);
        }
    };
    let _ = bcast_sub.set_timeout(Some(Duration::from_millis(250)));

    // Daemon-served recall (KANNAKA.recall.<agent_id>): the observatory, OBC
    // pulses, and the radio DJ can recall against this agent's warm in-memory
    // HRM instead of paying a 21 MB load + full xi-rerank per CLI call on the
    // 1-vCPU box. Uses the attention-beam prefilter + recall_with_beam
    // (O(beam), sub-second) — the same path the substrate responder uses. swarm
    // serve runs KANNAKA_READONLY, so the observation mutation never persists.
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
        // Round-robin: try directed first, then broadcast.
        if let Some(msg) = directed_sub.next_message() {
            _handle_serve_msg(sys, cfg, &transport, &msg, /*is_broadcast*/ false, threshold);
        }
        if let Some(msg) = bcast_sub.next_message() {
            _handle_serve_msg(sys, cfg, &bcast_transport, &msg, /*is_broadcast*/ true, threshold);
        }
        // Daemon-served recall — reply with the agent's own memories (full content).
        if let Some(ref mut rsub) = recall_sub {
            if let Some(msg) = rsub.next_message() {
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
                        let payload = serde_json::json!({ "from": cfg.agent.id, "query": query, "results": results });
                        let _ = rt.reply(&reply, payload.to_string().as_bytes());
                    }
                }
            }
        }
    }
}

#[cfg(feature = "nats")]
fn _serve_directed_only(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    transport: &kannaka_memory::nats::SwarmTransport,
    mut sub: kannaka_memory::nats::NatsSubscription,
) {
    while let Some(msg) = sub.next_message() {
        _handle_serve_msg(sys, cfg, transport, &msg, false, 0.0);
    }
}

#[cfg(feature = "nats")]
fn _handle_serve_msg(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    transport: &kannaka_memory::nats::SwarmTransport,
    msg: &kannaka_memory::nats::NatsMessage,
    is_broadcast: bool,
    threshold: f32,
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
            let err = serde_json::json!({ "from": cfg.agent.id, "error": format!("bad json: {e}") });
            let _ = transport.reply(&reply_to, err.to_string().as_bytes());
            return;
        }
    };
    let from = req.get("from").and_then(|v| v.as_str()).unwrap_or("?");
    let text = req.get("text").and_then(|v| v.as_str()).unwrap_or("");
    let recall_q = req.get("recall_query").and_then(|v| v.as_str());

    if text.is_empty() {
        let err = serde_json::json!({ "from": cfg.agent.id, "error": "empty text" });
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
    let reply = match result {
        Ok(r) => serde_json::json!({ "from": cfg.agent.id, "text": r.text }),
        Err(e) => serde_json::json!({ "from": cfg.agent.id, "error": format!("{e}") }),
    };

    // Heavy ask calls take 3–5 min. The original NATS connection has been
    // idle that whole time and the server typically closes it on PING
    // timeout. Open a fresh connection just to send the reply so the
    // long-running subscription on the parent transport doesn't have to
    // be reconnected on every request.
    let nats_url_for_reply = std::env::var("KANNAKA_NATS_URL")
        .unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());
    let reply_payload = reply.to_string();
    let reply_result = match kannaka_memory::nats::SwarmTransport::connect(&nats_url_for_reply) {
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
            "--top-k" if i + 1 < args.len() => { top_k = args[i + 1].parse().unwrap_or(20); i += 2; }
            "--agent-id" if i + 1 < args.len() => { agent_id_override = Some(args[i + 1].clone()); i += 2; }
            "--from" if i + 1 < args.len() => { from = Some(args[i + 1].clone()); i += 2; }
            "--nats-url" if i + 1 < args.len() => { i += 2; }
            _ => i += 1,
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
            eprintln!("Usage: kannaka swarm exemplars <publish|list> [--top-k N] [--from <agent>]");
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
    // Usage: kannaka swarm absorb [--from <agent>] [--top-k N] [--threshold X] [--dry-run]
    let mut from: Option<String> = None;
    let mut top_k: usize = 50;
    let mut threshold: f32 = 0.4;
    let mut dry_run = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--from" if i + 1 < args.len() => { from = Some(args[i + 1].clone()); i += 2; }
            "--top-k" if i + 1 < args.len() => { top_k = args[i + 1].parse().unwrap_or(50); i += 2; }
            "--threshold" if i + 1 < args.len() => { threshold = args[i + 1].parse().unwrap_or(0.4); i += 2; }
            "--dry-run" => { dry_run = true; i += 1; }
            "--nats-url" if i + 1 < args.len() => { i += 2; }
            _ => i += 1,
        }
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
    // Usage: kannaka swarm peers [--json]
    let mut as_json = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => { as_json = true; i += 1; }
            "--nats-url" if i + 1 < args.len() => { i += 2; }
            _ => i += 1,
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
    use std::time::Duration;
    // Usage: kannaka swarm autoabsorb [--threshold 0.4] [--per-source-daily-cap N] [--dry-run]
    let mut threshold: f32 = 0.4;
    let mut per_source_daily_cap: usize = 10;
    let mut dry_run = false;
    let mut max_phi_drop: f32 = 0.05;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--threshold" if i + 1 < args.len() => { threshold = args[i + 1].parse().unwrap_or(0.4); i += 2; }
            "--per-source-daily-cap" if i + 1 < args.len() => { per_source_daily_cap = args[i + 1].parse().unwrap_or(10); i += 2; }
            "--max-phi-drop" if i + 1 < args.len() => { max_phi_drop = args[i + 1].parse().unwrap_or(0.05); i += 2; }
            "--dry-run" => { dry_run = true; i += 1; }
            "--nats-url" if i + 1 < args.len() => { i += 2; }
            _ => i += 1,
        }
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
    let _ = Duration::from_secs(0); // keep import live in case timeout helpers come back

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
    // Usage: kannaka swarm enqueue <kind> "payload text" [--timeout 600]
    let kind = match args.get(2) {
        Some(s) => s.clone(),
        None => { eprintln!("Usage: kannaka swarm enqueue <kind> \"payload\" [--timeout SECONDS]"); process::exit(1); }
    };
    let mut timeout_secs: u64 = 600;
    let mut text_parts: Vec<String> = Vec::new();
    let mut i = 3;
    while i < args.len() {
        match args[i].as_str() {
            "--timeout" if i + 1 < args.len() => {
                timeout_secs = args[i + 1].parse().unwrap_or(600); i += 2;
            }
            "--nats-url" if i + 1 < args.len() => i += 2,
            _ => { text_parts.push(args[i].clone()); i += 1; }
        }
    }
    let text = text_parts.join(" ").trim().to_string();
    if text.is_empty() {
        eprintln!("Usage: kannaka swarm enqueue <kind> \"payload\""); process::exit(1);
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
    // Usage: kannaka swarm worker [--kinds ask,dream,...] [--queue-group GROUP]
    let mut kinds: Vec<String> = vec!["ask".to_string()];
    let mut queue_group: String = "kannaka_workers".to_string();
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--kinds" if i + 1 < args.len() => {
                kinds = args[i + 1].split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                i += 2;
            }
            "--queue-group" if i + 1 < args.len() => {
                queue_group = args[i + 1].clone(); i += 2;
            }
            "--nats-url" if i + 1 < args.len() => i += 2,
            _ => i += 1,
        }
    }
    if kinds.is_empty() { kinds.push("ask".to_string()); }

    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("[worker] nats: {e}"); process::exit(1); }
    };

    eprintln!("[worker] kinds: {:?}, queue group: {}", kinds, queue_group);

    // v1: support exactly one kind per worker process (round-robin across multiple
    // would need a thread per kind). If multiple kinds were requested, we'll
    // process them by rotating subscriptions every 5s.
    if kinds.len() == 1 {
        let kind = &kinds[0];
        let subject = format!("KANNAKA.work.{}", kind);
        let group = format!("{}_{}", queue_group, kind);
        let mut sub = match transport.subscribe_with_queue(&subject, Some(&group)) {
            Ok(s) => s,
            Err(e) => { eprintln!("[worker] subscribe: {e}"); process::exit(1); }
        };
        eprintln!("[worker] subscribed to {} (group {})", subject, group);
        while let Some(msg) = sub.next_message() {
            _process_work_msg(sys, cfg, &transport, &nats_url, kind, &msg);
        }
    } else {
        // Multi-kind workers run sequentially with short polls — useful for
        // light test setups, suboptimal for production. Real production
        // should run one process per kind.
        eprintln!("[worker] WARNING: multi-kind worker — running each kind for ~5s in rotation");
        loop {
            for kind in &kinds {
                let subject = format!("KANNAKA.work.{}", kind);
                let group = format!("{}_{}", queue_group, kind);
                let mut sub = match transport.subscribe_with_queue(&subject, Some(&group)) {
                    Ok(s) => s,
                    Err(e) => { eprintln!("[worker] subscribe {kind}: {e}"); std::thread::sleep(Duration::from_secs(2)); continue; }
                };
                let _ = sub.set_timeout(Some(Duration::from_secs(5)));
                if let Some(msg) = sub.next_message() {
                    _process_work_msg(sys, cfg, &transport, &nats_url, kind, &msg);
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

    let mut subjects: Vec<String> = Vec::new();
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--subject" if i + 1 < args.len() => { subjects.push(args[i + 1].clone()); i += 2; }
            "--nats-url" if i + 1 < args.len() => { i += 2; }
            _ => i += 1,
        }
    }
    if subjects.is_empty() {
        for s in ["QUEEN.>", "KANNAKA.>", "RADIO.>", "KAX.>", "EYE.>"] {
            subjects.push(s.to_string());
        }
    }

    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    eprintln!("[tail] connecting to {} — subjects: {:?}", nats_url, subjects);

    // One dedicated transport per subject. SwarmTransport::subscribe hard-codes
    // sid 95 and a single subscription per connection — the cleanest way to
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
            while let Some(msg) = sub.next_message() {
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
            // Connection closed — reconnect.
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

