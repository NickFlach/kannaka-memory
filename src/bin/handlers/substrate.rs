//! ADR-0027 (collective substrate) + ADR-0028 (event-sourced HRM) handlers.
//!
//! Extracted from `bin/kannaka.rs` in v0.3.25 to slow the growth of the
//! 5500-line dispatcher. Future extractions should follow the same pattern:
//!   1. New file under `src/bin/handlers/<group>.rs`
//!   2. `use super::*;` for crate-private helpers (kannaka.rs items marked
//!      `pub(crate)`).
//!   3. Declare in kannaka.rs via `#[path = "handlers/<group>.rs"] mod …;`
//!   4. `pub(crate) fn handle_*` so the main dispatcher can call them.

use std::process;

use super::{
    data_dir, resolve_nats_url, substrate_class_index, substrate_class_word,
    KannakaConfig,
};

use flate2::Compression;
use flate2::write::GzEncoder;

/// Prune snapshot bodies in `dir` so only the latest `retain` files for
/// `agent_id` remain. Matches the JetStream max_msgs_per_subject so disk
/// stays in sync with the manifest stream. Files for other agents are
/// untouched (e.g. operator may have manually downloaded a peer's
/// snapshot for cross-host restore).
fn prune_snapshot_dir(dir: &std::path::Path, agent_id: &str, retain: usize) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    let suffix = format!("-{}.hrm.gz", agent_id);
    let mut bodies: Vec<std::path::PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with(&suffix))
                .unwrap_or(false)
        })
        .collect();
    if bodies.len() <= retain {
        return;
    }
    // The filename prefix is the UTC timestamp YYYYMMDDTHHmmssZ so lex sort
    // == chronological sort. Drop the oldest.
    bodies.sort();
    let drop_count = bodies.len() - retain;
    for p in bodies.iter().take(drop_count) {
        if let Err(e) = std::fs::remove_file(p) {
            eprintln!("[snapshot prune] failed to remove {}: {}", p.display(), e);
        }
    }
    eprintln!("[snapshot prune] removed {} old snapshot(s) for {}", drop_count, agent_id);
}

/// Capture HRM snapshot: gzip the HRM file to `<data_dir>/snapshots/`,
/// publish a manifest event to KANNAKA.snapshots.<agent>.full pointing
/// at the on-disk body. NATS silently caps payloads ~8MB even with
/// max_payload bumped, and HRMs grow to 35MB+, so we keep snapshots
/// out-of-band on disk and put only the manifest in JetStream. Once
/// ADR-0026 Phase 5 Object Store lands the body_path becomes a URL.
///
/// Local-disk retention is naive — every snapshot is kept until manual
/// cleanup. A future slice can prune to the latest N per agent (the
/// KANNAKA_SNAPSHOTS stream already auto-prunes manifests to 168).
#[cfg(feature = "nats")]
fn capture_and_publish_snapshot(
    transport: &kannaka_memory::nats::SwarmTransport,
    agent_id: &str,
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
) -> Result<u64, String> {
    use std::io::Write;
    // Flush in-memory medium to disk so the snapshot reflects current state.
    sys.engine.store.flush().map_err(|e| format!("flush: {e}"))?;
    let hrm_path = data_dir().join("kannaka.hrm");
    let bytes = std::fs::read(&hrm_path)
        .map_err(|e| format!("read {}: {e}", hrm_path.display()))?;
    let raw_size = bytes.len() as u64;
    let mut gz = GzEncoder::new(Vec::with_capacity(bytes.len() / 4), Compression::default());
    gz.write_all(&bytes).map_err(|e| format!("gzip: {e}"))?;
    let compressed = gz.finish().map_err(|e| format!("gzip finish: {e}"))?;
    let gz_size = compressed.len() as u64;

    // Persist to <data_dir>/snapshots/<ts>-<agent>.hrm.gz.
    let snapshots_dir = data_dir().join("snapshots");
    std::fs::create_dir_all(&snapshots_dir)
        .map_err(|e| format!("mkdir snapshots/: {e}"))?;
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let filename = format!("{}-{}.hrm.gz", ts, agent_id);
    let body_path = snapshots_dir.join(&filename);
    std::fs::write(&body_path, &compressed)
        .map_err(|e| format!("write snapshot body: {e}"))?;

    // ADR-0028 Phase 2 — disk retention: keep the latest N bodies per agent.
    // Default 168 matches JetStream's max_msgs_per_subject from
    // StreamKind::Snapshots so disk and manifest stream stay in sync.
    // Override via env KANNAKA_SNAPSHOT_RETAIN.
    let retain: usize = std::env::var("KANNAKA_SNAPSHOT_RETAIN")
        .ok().and_then(|s| s.parse().ok()).unwrap_or(168);
    prune_snapshot_dir(&snapshots_dir, agent_id, retain);

    let state = sys.assess();
    let wavefronts = sys.all_memories().map(|m| m.len() as u64).unwrap_or(0);
    let clusters = state.num_clusters as u64;
    let phi = state.phi;
    let version = env!("CARGO_PKG_VERSION");
    let body_path_str = body_path.to_string_lossy().to_string();

    transport.publish_event(kannaka_memory::nats::EventPayload::SnapshotFull {
        agent_id,
        version,
        wavefronts,
        clusters,
        phi,
        body_path: &body_path_str,
        body_gz_bytes: gz_size,
    }).map_err(|e| format!("publish manifest: {e}"))?;

    // Give the OS a moment to actually flush the TCP send buffer before
    // the one-shot CLI exits. Without this, the publish_raw flush() returns
    // before the kernel has put the bytes on the wire and NATS never sees
    // the message. Daemon mode (--interval) doesn't need this because the
    // long-running loop keeps the connection open.
    std::thread::sleep(std::time::Duration::from_millis(200));

    eprintln!(
        "[snapshot] agent={} wavefronts={} clusters={} phi={:.3} \
         raw={}KB gz={}KB body={}",
        agent_id, wavefronts, clusters, phi,
        raw_size / 1024, gz_size / 1024, body_path.display(),
    );
    Ok(raw_size)
}

/// ADR-0028 Phase 2 — `kannaka events snapshot [--interval SECS]`.
///
/// One-shot mode (no `--interval`): captures the current HRM, gzip+b64
/// encodes it, publishes to KANNAKA.snapshots.<agent_id>.full, exits.
///
/// Daemon mode (`--interval N`): same capture/publish loop every N seconds,
/// Ctrl+C to stop. Default cadence for `kannaka substrate run` autosnapshot
/// hook is 3600s (one snapshot per hour).
#[cfg(feature = "nats")]
pub(crate) fn handle_events_snapshot(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    args: &[String],
) {
    use std::time::Duration;
    let mut interval_secs: Option<u64> = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--interval" if i + 1 < args.len() => {
                interval_secs = args[i + 1].parse().ok();
                i += 2;
            }
            "--nats-url" if i + 1 < args.len() => { i += 2; }
            _ => i += 1,
        }
    }

    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("[snapshot] NATS connect failed: {}", e); process::exit(1); }
    };
    let agent_id = cfg.agent.id.as_str();

    match interval_secs {
        None => {
            // One-shot
            if let Err(e) = capture_and_publish_snapshot(&transport, agent_id, sys) {
                eprintln!("[snapshot] failed: {}", e);
                process::exit(1);
            }
        }
        Some(secs) => {
            // Daemon
            eprintln!("[snapshot] daemon ready — cadence {}s, Ctrl+C to stop", secs);
            loop {
                if let Err(e) = capture_and_publish_snapshot(&transport, agent_id, sys) {
                    eprintln!("[snapshot] warning: {}", e);
                }
                std::thread::sleep(Duration::from_secs(secs));
            }
        }
    }
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_events_snapshot(_: &mut kannaka_memory::openclaw::KannakaMemorySystem, _: &KannakaConfig, _: &[String]) {
    eprintln!("events snapshot requires the 'nats' feature");
    process::exit(1);
}

/// ADR-0028 Phase 3 — `kannaka events list-snapshots [--agent ID]`.
///
/// Pulls all snapshot manifests from KANNAKA_SNAPSHOTS and prints them
/// sorted newest-first, one per line. Optional `--agent` filter narrows
/// to a single agent. Each line shows ts, agent_id, wavefronts,
/// clusters, phi, body_gz_bytes (KB), body_path.
#[cfg(feature = "nats")]
pub(crate) fn handle_events_list_snapshots(cfg: &KannakaConfig, args: &[String]) {
    let mut agent_filter: Option<String> = None;
    let mut json_mode = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--agent" if i + 1 < args.len() => {
                agent_filter = Some(args[i + 1].clone()); i += 2;
            }
            "--json" => { json_mode = true; i += 1; }
            "--nats-url" if i + 1 < args.len() => { i += 2; }
            _ => i += 1,
        }
    }
    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("[list-snapshots] NATS connect failed: {}", e); process::exit(1); }
    };
    let subject_filter = match &agent_filter {
        Some(a) => format!("KANNAKA.snapshots.{}.full", a),
        None => "KANNAKA.snapshots.>".to_string(),
    };
    let manifests = match transport.get_stream_messages("KANNAKA_SNAPSHOTS", &subject_filter, 500) {
        Ok(v) => v,
        Err(e) => { eprintln!("[list-snapshots] read failed: {}", e); process::exit(1); }
    };
    if manifests.is_empty() {
        if json_mode {
            println!("[]");
        } else {
            eprintln!("[list-snapshots] no manifests found");
        }
        return;
    }
    // Sort by ts descending.
    let mut sorted = manifests;
    sorted.sort_by(|a, b| {
        let at = a.get("ts").and_then(|v| v.as_str()).unwrap_or("");
        let bt = b.get("ts").and_then(|v| v.as_str()).unwrap_or("");
        bt.cmp(at)
    });
    if json_mode {
        // Emit the raw manifest array — observatory + other downstream
        // consumers parse this directly. Wire-format is the JetStream
        // manifest shape: { ts, agent_id, manifest: { version, wavefronts,
        // clusters, phi }, body_path, body_gz_bytes, event_id,
        // schema_version }.
        match serde_json::to_string(&sorted) {
            Ok(s) => println!("{}", s),
            Err(e) => {
                eprintln!("[list-snapshots] serialize: {}", e);
                process::exit(1);
            }
        }
        return;
    }
    println!("{:<25} {:<28} {:>6} {:>4} {:>6} {:>8}  {}",
        "ts", "agent_id", "waves", "cls", "phi", "gz(KB)", "body");
    for m in sorted.iter() {
        let ts = m.get("ts").and_then(|v| v.as_str()).unwrap_or("?");
        let agent = m.get("agent_id").and_then(|v| v.as_str()).unwrap_or("?");
        let manifest = m.get("manifest");
        let waves = manifest.and_then(|x| x.get("wavefronts")).and_then(|v| v.as_u64()).unwrap_or(0);
        let cls = manifest.and_then(|x| x.get("clusters")).and_then(|v| v.as_u64()).unwrap_or(0);
        let phi = manifest.and_then(|x| x.get("phi")).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let bytes = m.get("body_gz_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
        let body = m.get("body_path").and_then(|v| v.as_str()).unwrap_or("?");
        println!("{:<25} {:<28} {:>6} {:>4} {:>6.3} {:>8}  {}",
            ts.chars().take(24).collect::<String>(),
            agent, waves, cls, phi, bytes / 1024, body);
    }
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_events_list_snapshots(_: &KannakaConfig, _: &[String]) {
    eprintln!("events list-snapshots requires the 'nats' feature");
    process::exit(1);
}

/// ADR-0028 Phase 3 — `kannaka events restore [--agent ID] [--from PATH]`.
///
/// Restore HRM from a snapshot body on disk. Without `--from`, picks the
/// most recent snapshot for the given agent (or this agent's id from
/// config). With `--from PATH`, uses the explicit gz file. Backs up the
/// current kannaka.hrm to kannaka.hrm.pre-restore-<ts> before overwriting.
///
/// Safety: refuses to run if `kannaka.hrm` is open / locked by another
/// process — operator must stop the substrate/swarm daemon first.
#[cfg(feature = "nats")]
pub(crate) fn handle_events_restore(cfg: &KannakaConfig, args: &[String]) {
    use std::io::Read;
    use flate2::read::GzDecoder;
    let mut from: Option<String> = None;
    let mut from_url: Option<String> = None;
    let mut agent_filter: Option<String> = None;
    let mut dry_run = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--from" if i + 1 < args.len() => { from = Some(args[i + 1].clone()); i += 2; }
            "--from-url" if i + 1 < args.len() => { from_url = Some(args[i + 1].clone()); i += 2; }
            "--agent" if i + 1 < args.len() => { agent_filter = Some(args[i + 1].clone()); i += 2; }
            "--dry-run" => { dry_run = true; i += 1; }
            "--nats-url" if i + 1 < args.len() => { i += 2; }
            _ => i += 1,
        }
    }

    // Resolve body_path: either explicit --from or look up latest manifest.
    let body_path = match from {
        Some(p) => p,
        None => {
            let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
            let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
                Ok(t) => t,
                Err(e) => { eprintln!("[restore] NATS connect failed: {}", e); process::exit(1); }
            };
            let target_agent = agent_filter.unwrap_or_else(|| cfg.agent.id.clone());
            let subject = format!("KANNAKA.snapshots.{}.full", target_agent);
            let manifests = match transport.get_stream_messages("KANNAKA_SNAPSHOTS", &subject, 500) {
                Ok(v) => v,
                Err(e) => { eprintln!("[restore] read manifests failed: {}", e); process::exit(1); }
            };
            if manifests.is_empty() {
                eprintln!("[restore] no snapshots found for agent={}", target_agent);
                process::exit(1);
            }
            let mut latest = manifests;
            latest.sort_by(|a, b| {
                let at = a.get("ts").and_then(|v| v.as_str()).unwrap_or("");
                let bt = b.get("ts").and_then(|v| v.as_str()).unwrap_or("");
                bt.cmp(at)
            });
            let path = latest[0].get("body_path").and_then(|v| v.as_str())
                .map(|s| s.to_string());
            match path {
                Some(p) => p,
                None => {
                    eprintln!("[restore] latest manifest has no body_path");
                    process::exit(1);
                }
            }
        }
    };

    // km#75 — cross-host fetch via the observatory's /api/snapshots/body/<name>
    // (or any HTTP URL serving the gzipped body). Takes precedence over
    // --from and the manifest lookup; the body lands on local disk under
    // <data_dir>/snapshots/ so future replays can use --from.
    let gz_bytes: Vec<u8> = if let Some(url) = from_url {
        eprintln!("[restore] source (url): {}", url);
        let resp = match ureq::get(&url).timeout(std::time::Duration::from_secs(120)).call() {
            Ok(r) => r,
            Err(e) => { eprintln!("[restore] HTTP fetch failed: {}", e); process::exit(1); }
        };
        let mut buf: Vec<u8> = Vec::new();
        if let Err(e) = resp.into_reader().read_to_end(&mut buf) {
            eprintln!("[restore] HTTP body read failed: {}", e);
            process::exit(1);
        }
        // Cache the downloaded body to <data_dir>/snapshots/ so list-snapshots
        // can find it on subsequent runs.
        let snapshots_dir = data_dir().join("snapshots");
        if let Err(e) = std::fs::create_dir_all(&snapshots_dir) {
            eprintln!("[restore] mkdir snapshots/: {}", e);
        } else {
            // Filename comes from the last URL segment.
            let name = url.rsplit('/').next().unwrap_or("downloaded.hrm.gz");
            let dst = snapshots_dir.join(name);
            if let Err(e) = std::fs::write(&dst, &buf) {
                eprintln!("[restore] cache body locally failed: {}", e);
            } else {
                eprintln!("[restore] cached body to {}", dst.display());
            }
        }
        buf
    } else {
        eprintln!("[restore] source: {}", body_path);
        match std::fs::read(&body_path) {
            Ok(b) => b,
            Err(e) => { eprintln!("[restore] read body failed: {}", e); process::exit(1); }
        }
    };
    let mut decoder = GzDecoder::new(gz_bytes.as_slice());
    let mut decoded: Vec<u8> = Vec::with_capacity(gz_bytes.len() * 4);
    if let Err(e) = decoder.read_to_end(&mut decoded) {
        eprintln!("[restore] gunzip failed: {}", e);
        process::exit(1);
    }

    let hrm_path = data_dir().join("kannaka.hrm");

    if dry_run {
        // Validate without overwriting. Report what would happen so the
        // operator can confirm before re-running without --dry-run.
        eprintln!("[restore] DRY RUN — no files written");
        eprintln!("[restore]   gz body bytes : {}", gz_bytes.len());
        eprintln!("[restore]   decoded bytes : {}", decoded.len());
        eprintln!("[restore]   would write   : {}", hrm_path.display());
        if hrm_path.exists() {
            let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
            let backup_path = data_dir().join(format!("kannaka.hrm.pre-restore-{}", ts));
            eprintln!("[restore]   would backup  : {} -> {}", hrm_path.display(), backup_path.display());
        } else {
            eprintln!("[restore]   no existing HRM to back up");
        }
        return;
    }

    if hrm_path.exists() {
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let backup_path = data_dir().join(format!("kannaka.hrm.pre-restore-{}", ts));
        if let Err(e) = std::fs::rename(&hrm_path, &backup_path) {
            eprintln!("[restore] backup rename failed: {} (is the HRM in use? stop substrate/swarm daemon first)", e);
            process::exit(1);
        }
        eprintln!("[restore] backed up existing HRM to {}", backup_path.display());
    }
    if let Err(e) = std::fs::write(&hrm_path, &decoded) {
        eprintln!("[restore] write decoded HRM failed: {}", e);
        process::exit(1);
    }
    eprintln!("[restore] wrote {} bytes to {}", decoded.len(), hrm_path.display());
    eprintln!("[restore] done — restart any kannaka daemons (substrate/swarm) so they pick up the new HRM");
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_events_restore(_: &KannakaConfig, _: &[String]) {
    eprintln!("events restore requires the 'nats' feature");
    process::exit(1);
}

/// ADR-0027 — `kannaka substrate status` operator visibility.
///
/// Subscribes to KANNAKA.substrate.phi (the substrate daemon's 60s
/// publish cadence) and prints the next event's collective metrics.
/// Times out after `--wait SECS` (default 65s — slightly more than the
/// substrate's PHI_PUBLISH_SECS so a single missed beat still resolves).
#[cfg(feature = "nats")]
pub(crate) fn handle_substrate_status(cfg: &KannakaConfig, args: &[String]) {
    use std::time::{Duration, Instant};
    let mut wait_secs: u64 = 65;
    let mut json_mode = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--wait" if i + 1 < args.len() => {
                wait_secs = args[i + 1].parse().unwrap_or(65); i += 2;
            }
            "--json" => { json_mode = true; i += 1; }
            "--nats-url" if i + 1 < args.len() => { i += 2; }
            _ => i += 1,
        }
    }
    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("[substrate status] NATS connect failed: {}", e); process::exit(1); }
    };
    let mut sub = match transport.subscribe("KANNAKA.substrate.phi") {
        Ok(s) => s,
        Err(e) => { eprintln!("[substrate status] subscribe failed: {}", e); process::exit(1); }
    };
    let _ = sub.set_timeout(Some(Duration::from_secs(2)));
    eprintln!("[substrate status] waiting up to {}s for next KANNAKA.substrate.phi …", wait_secs);
    let deadline = Instant::now() + Duration::from_secs(wait_secs);
    while Instant::now() < deadline {
        if let Some(msg) = sub.next_message() {
            let v: serde_json::Value = match serde_json::from_slice(&msg.payload) {
                Ok(v) => v,
                Err(e) => { eprintln!("[substrate status] bad payload: {}", e); continue; }
            };
            let phi = v.get("phi").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let xi = v.get("xi").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let order = v.get("mean_order").and_then(|x| x.as_f64()).unwrap_or(0.0);
            let clusters = v.get("num_clusters").and_then(|x| x.as_u64()).unwrap_or(0);
            let mems = v.get("total_memories").and_then(|x| x.as_u64()).unwrap_or(0);
            let contribs: Vec<String> = v.get("contributors")
                .and_then(|c| c.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let ts = v.get("ts").and_then(|x| x.as_str()).unwrap_or("?");
            if json_mode {
                // Emit the parsed payload (not raw) so observatory has a
                // stable schema regardless of upstream substrate.phi shape
                // drift.
                let payload = serde_json::json!({
                    "ts": ts,
                    "phi": phi,
                    "xi": xi,
                    "order": order,
                    "clusters": clusters,
                    "memories": mems,
                    "contributors": contribs,
                });
                println!("{}", payload);
            } else {
                println!("collective substrate @ {}", ts);
                println!("  Φ           {:.3}", phi);
                println!("  Ξ           {:.3}", xi);
                println!("  order       {:.3}", order);
                println!("  clusters    {}", clusters);
                println!("  memories    {}", mems);
                println!("  contributors  {} ({})", contribs.len(),
                    if contribs.is_empty() { "none yet".to_string() } else { contribs.join(", ") });
            }
            return;
        }
    }
    if json_mode {
        // Stable error shape for the JSON consumer.
        let payload = serde_json::json!({
            "error": "no_phi_event",
            "wait_secs": wait_secs,
        });
        eprintln!("{}", payload);
    } else {
        eprintln!("[substrate status] no phi event within {}s — is `kannaka substrate run` alive?", wait_secs);
    }
    process::exit(2);
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_substrate_status(_: &KannakaConfig, _: &[String]) {
    eprintln!("substrate status requires the 'nats' feature");
    process::exit(1);
}

/// ADR-0027 Phase 1: subscribe to `KANNAKA.substrate.absorb.>` and route
/// each absorb directly into the substrate's HRM (bypassing the text
/// encoder so distinct absorbs in the same class don't Kuramoto-collapse).
/// Periodically publishes collective phi to `KANNAKA.substrate.phi`.
#[cfg(feature = "nats")]
pub(crate) fn handle_substrate_run(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    args: &[String],
) {
    use std::collections::HashSet;
    use std::time::{Duration, Instant};
    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("[substrate] Failed to connect to NATS at {}: {}", nats_url, e); process::exit(1); }
    };
    eprintln!("[substrate] subscribing to KANNAKA.substrate.absorb.> on {}", nats_url);
    let mut sub = match transport.subscribe("KANNAKA.substrate.absorb.>") {
        Ok(s) => s,
        Err(e) => { eprintln!("[substrate] subscribe failed: {}", e); process::exit(1); }
    };
    let _ = sub.set_timeout(Some(Duration::from_secs(5)));

    // ADR-0027 Phase 3 — collective recall. Second transport (NATS sub
    // owns the connection's TCP read half, so each subject needs a
    // separate connection). Peer agents send a query payload via
    // request_one; we run recall against the substrate's 96-class HRM
    // and reply with the top-K matches.
    let recall_transport = kannaka_memory::nats::SwarmTransport::connect(&nats_url).ok();
    let mut recall_sub = recall_transport
        .as_ref()
        .and_then(|t| t.subscribe("KANNAKA.substrate.recall").ok());
    if let Some(ref mut s) = recall_sub {
        let _ = s.set_timeout(Some(Duration::from_secs(2)));
        eprintln!("[substrate] subscribing to KANNAKA.substrate.recall (collective recall)");
    } else {
        eprintln!("[substrate] WARN: no collective-recall subscription — running absorb-only");
    }

    // Publish substrate phi on a slow cadence — eigendecomp is the heavy
    // cost so we don't want to do it on every absorb. 60s feels live
    // enough for the observatory.
    const PHI_PUBLISH_SECS: u64 = 60;
    // ADR-0028 Phase 2 hook: auto-snapshot cadence. Default 3600s (1h) so
    // the operator always has at most one hour of replayable state to lose
    // on a disaster. Override via KANNAKA_SNAPSHOT_INTERVAL_SECS; set to 0
    // to disable.
    let snapshot_interval_secs: u64 = std::env::var("KANNAKA_SNAPSHOT_INTERVAL_SECS")
        .ok().and_then(|s| s.parse().ok()).unwrap_or(3600);
    let mut last_phi_pub = Instant::now() - Duration::from_secs(PHI_PUBLISH_SECS);
    // Start the snapshot timer fresh so we don't snapshot immediately on
    // boot — operator probably just ran one manually as part of deploy.
    let mut last_snapshot = Instant::now();
    let mut contributors: HashSet<String> = HashSet::new();

    // Connection failure tracking — if the NATS server restarts under us
    // (or the network blips), the next publish/subscribe returns a broken-
    // pipe error. After MAX_CONSECUTIVE_FAILURES we exit(1) and let
    // systemd's Restart=on-failure bring us back with a fresh transport.
    const MAX_CONSECUTIVE_FAILURES: u32 = 3;
    let mut consecutive_failures: u32 = 0;

    eprintln!("[substrate] daemon ready — Ctrl+C to stop");
    loop {
        // ADR-0027 Phase 3 — collective recall handler.
        // Payload: {"query": "...", "top_k": 8 (optional)}
        // Reply: {"from": substrate_agent_id, "query": "...", "matches":
        //         [{memory_id, content, similarity, strength, ...}, ...],
        //         "total_memories": N}
        // Content surface area is metadata-only ("substrate-absorb[class=N]
        // from <agent> amp=... phase=... freq=...") — peer texts never
        // crossed the boundary in the first place (ADR-0027 Phase 1.c
        // privacy: only wave signatures absorb into the substrate).
        if let Some(ref mut rsub) = recall_sub {
            if let Some(msg) = rsub.next_message() {
                let reply_to = msg.reply_to.clone();
                let req: serde_json::Value = serde_json::from_slice(&msg.payload)
                    .unwrap_or(serde_json::Value::Null);
                let query = req.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let top_k = req.get("top_k").and_then(|v| v.as_u64()).unwrap_or(8) as usize;
                if !query.is_empty() && reply_to.is_some() {
                    // Use the attention-beam prefilter (token-overlap → top
                    // candidates → resonance against beam only) so the recall
                    // is O(beam_size) instead of O(N). sys.recall on the
                    // substrate's 750+ memory HRM with xi-rerank is 30-60s+
                    // on ARM — way past any sensible NATS request_one timeout.
                    let beam = kannaka_memory::agent::attention_beam_for_prompt(
                        sys, query, kannaka_memory::agent::DEFAULT_ATTENTION_BEAM);
                    let matches: Vec<serde_json::Value> = if beam.is_empty() {
                        Vec::new()
                    } else {
                        match sys.recall_with_beam(&beam, query, top_k) {
                            Ok(rs) => rs.iter().map(|r| serde_json::json!({
                                "memory_id": r.id.to_string(),
                                "content": r.content,
                                "similarity": r.similarity,
                                "strength": r.strength,
                                "age_hours": r.age_hours,
                            })).collect(),
                            Err(_) => Vec::new(),
                        }
                    };
                    let total = sys.all_memories().map(|m| m.len()).unwrap_or(0);
                    let reply_payload = serde_json::json!({
                        "from": cfg.agent.id,
                        "query": query,
                        "matches": matches,
                        "total_memories": total,
                    });
                    if let Some(reply_subject) = reply_to {
                        if let Some(ref rt) = recall_transport {
                            let _ = rt.reply(&reply_subject, reply_payload.to_string().as_bytes());
                            eprintln!("[substrate recall] from peer: '{}' -> {} matches",
                                query.chars().take(60).collect::<String>(), matches.len());
                        }
                    }
                }
            }
        }

        if let Some(msg) = sub.next_message() {
            let v: serde_json::Value = match serde_json::from_slice(&msg.payload) {
                Ok(v) => v,
                Err(e) => { eprintln!("[substrate] skip malformed event: {}", e); continue; }
            };
            let agent_id = v["agent_id"].as_str().unwrap_or("unknown");
            let class_index = v["class_index"].as_u64().unwrap_or(0);
            let amplitude = v["amplitude"].as_f64().unwrap_or(0.0) as f32;
            let phase = v["phase"].as_f64().unwrap_or(0.0) as f32;
            let frequency = v["frequency"].as_f64().unwrap_or(0.0) as f32;
            // Skip our own substrate-absorbs (the kannaka-prime agent
            // shouldn't echo its own remembers back into itself).
            if agent_id == cfg.agent.id { continue; }
            contributors.insert(agent_id.to_string());
            // ADR-0027 Phase 1.c — direct wavefront insertion. Bypass
            // the text encoder entirely; build a hypervector centered on
            // the class anchor slice + perturbed by (amplitude, phase,
            // frequency) so distinct absorbs in the same class land at
            // distinct points within that cluster instead of stacking on
            // the anchor itself. Privacy: content is metadata-only
            // ("substrate-absorb[class] from agent ..."), no peer text.
            const WAVEFRONT_DIM: usize = 10_000;
            let slice_size = WAVEFRONT_DIM / 96;
            let mut vector = vec![0.0f32; WAVEFRONT_DIM];
            let start = (class_index as usize) * slice_size;
            let end = (start + slice_size).min(WAVEFRONT_DIM);
            // Anchor activation in the class slice, modulated by amplitude
            // so weaker absorbs sit further from the anchor center.
            let anchor_strength = amplitude.max(0.1);
            for i in start..end {
                vector[i] = anchor_strength;
            }
            // Phase-derived perturbation: small noise in adjacent slices,
            // sign and magnitude derived from phase + frequency. Keeps the
            // cosine to the anchor > 0.5 (same cluster) but distinct from
            // other absorbs in the same class. Use a small deterministic
            // PRNG seeded by the wave params for reproducibility.
            let mut seed: u32 = ((phase * 1000.0) as i32 as u32)
                .wrapping_add(((frequency * 1000.0) as i32 as u32).wrapping_mul(31))
                .wrapping_add(class_index as u32 * 7);
            for i in 0..WAVEFRONT_DIM {
                // xorshift32 — fast deterministic noise
                seed ^= seed << 13;
                seed ^= seed >> 17;
                seed ^= seed << 5;
                // Tiny perturbation: 1% of anchor strength
                let noise = ((seed as f32 / u32::MAX as f32) - 0.5) * 0.02 * anchor_strength;
                vector[i] += noise;
            }
            let norm = (vector.iter().map(|v| v * v).sum::<f32>()).sqrt();
            if norm > 0.0 {
                for v in vector.iter_mut() { *v /= norm; }
            }
            let content = format!(
                "substrate-absorb[class={}] from {} amp={:.3} phase={:.3} freq={:.3}",
                class_index, agent_id, amplitude, phase, frequency
            );

            let hrm = match sys.engine.store
                .as_any_mut()
                .downcast_mut::<kannaka_memory::hrm_store::HrmStore>()
            {
                Some(h) => h,
                None => {
                    eprintln!("[substrate] non-HrmStore backend — cannot direct-insert");
                    continue;
                }
            };
            match hrm.insert_raw_wavefront(vector, content, amplitude) {
                Ok(id) => {
                    eprintln!("[substrate] absorbed from {} class {} -> {}", agent_id, class_index, id);
                }
                Err(e) => {
                    eprintln!("[substrate] absorb failed: {}", e);
                }
            }
        }
        // Periodic phi publish — runs even if no inbound events, so the
        // observatory always has a recent snapshot.
        if last_phi_pub.elapsed() >= Duration::from_secs(PHI_PUBLISH_SECS) {
            // Flush to disk so the observatory's `kannaka status`
            // shell-out sees the live count.
            if let Err(e) = sys.engine.store.flush() {
                eprintln!("[substrate] flush warning: {}", e);
            }
            let state = sys.assess();
            let contribs: Vec<String> = contributors.iter().cloned().collect();
            match transport.publish_substrate_phi(
                state.phi, state.xi, state.mean_order,
                state.num_clusters, state.total_memories,
                &contribs,
            ) {
                Ok(()) => {
                    consecutive_failures = 0;
                    eprintln!("[substrate] published collective phi={:.3} (clusters={} mems={} contribs={})",
                        state.phi, state.num_clusters, state.total_memories, contribs.len());
                }
                Err(e) => {
                    consecutive_failures += 1;
                    eprintln!("[substrate] phi publish failed ({}/{}): {}",
                        consecutive_failures, MAX_CONSECUTIVE_FAILURES, e);
                    if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                        eprintln!("[substrate] NATS connection unrecoverable — exiting for systemd restart");
                        process::exit(1);
                    }
                }
            }
            last_phi_pub = Instant::now();
        }
        // ADR-0028 Phase 2 — periodic autosnapshot. Skip entirely if
        // KANNAKA_SNAPSHOT_INTERVAL_SECS=0. Best-effort: a failed snapshot
        // shouldn't bring the daemon down (it'll retry next interval), and
        // the snapshot path counts as part of the consecutive_failures
        // budget shared with phi publishes.
        if snapshot_interval_secs > 0
            && last_snapshot.elapsed() >= Duration::from_secs(snapshot_interval_secs)
        {
            match capture_and_publish_snapshot(&transport, &cfg.agent.id, sys) {
                Ok(_) => {
                    consecutive_failures = 0;
                }
                Err(e) => {
                    consecutive_failures += 1;
                    eprintln!("[substrate] snapshot failed ({}/{}): {}",
                        consecutive_failures, MAX_CONSECUTIVE_FAILURES, e);
                    if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                        eprintln!("[substrate] NATS connection unrecoverable — exiting for systemd restart");
                        process::exit(1);
                    }
                }
            }
            last_snapshot = Instant::now();
        }
    }
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_substrate_run(_: &mut kannaka_memory::openclaw::KannakaMemorySystem, _: &KannakaConfig, _: &[String]) {
    eprintln!("substrate run requires the 'nats' feature");
    process::exit(1);
}

/// ADR-0027 Phase 2: walk the local HRM and emit one substrate.absorb
/// event per memory, so an existing agent's HRM contributes its prior
/// content to the collective substrate without each memory needing to
/// be re-`remember`-ed with `--substrate`.
///
/// Idempotent across runs: honors a marker file at
/// `<data_dir>/.substrate-backfilled`. Force re-run with `--force`.
#[cfg(feature = "nats")]
pub(crate) fn handle_substrate_backfill(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    cfg: &KannakaConfig,
    args: &[String],
) {
    use std::time::Duration;
    let mut force = false;
    let mut delay_ms: u64 = 50;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--force" => { force = true; i += 1; }
            "--delay-ms" if i + 1 < args.len() => {
                delay_ms = args[i + 1].parse().unwrap_or(50);
                i += 2;
            }
            _ => i += 1,
        }
    }
    let marker_path = data_dir().join(".substrate-backfilled");
    if marker_path.exists() && !force {
        eprintln!("[backfill] already done (marker at {}); use --force to re-run", marker_path.display());
        return;
    }

    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("[backfill] NATS connect failed: {}", e);
            process::exit(1);
        }
    };
    let agent_id = cfg.agent.id.clone();
    let memories = match sys.all_memories() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("[backfill] failed to read local HRM: {}", e);
            process::exit(1);
        }
    };
    let total = memories.len();
    eprintln!("[backfill] {} memories to absorb (delay={}ms, ETA {}s)",
        total, delay_ms, (total as u64 * delay_ms) / 1000);

    let mut sent: usize = 0;
    let mut failed: usize = 0;
    for (i, mem) in memories.iter().enumerate() {
        let class_index = substrate_class_index(&mem.content);
        // Derive distinct wave params per source memory — same logic as
        // the live `remember --substrate` path.
        let id_bytes = mem.id.as_bytes();
        let phase_hash: u64 = id_bytes.iter().fold(0u64, |acc, b| {
            acc.wrapping_mul(31).wrapping_add(*b as u64)
        });
        let phase = (phase_hash as f32 / u64::MAX as f32) * std::f32::consts::TAU;
        let frequency = 0.5 + (class_index as f32 / 96.0) * 1.5;
        let amplitude = (mem.content.len() as f32 / 200.0).min(1.0).max(0.2);
        match transport.publish_substrate_absorb(
            &agent_id, class_index, amplitude, phase, frequency,
        ) {
            Ok(()) => sent += 1,
            Err(e) => {
                failed += 1;
                if failed <= 3 {
                    eprintln!("[backfill] event {}/{} failed: {}", i + 1, total, e);
                }
            }
        }
        if (i + 1) % 50 == 0 || i + 1 == total {
            eprintln!("[backfill] {}/{} sent ({} failed)", i + 1, total, failed);
        }
        if delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
    }

    let _ = std::fs::write(&marker_path, format!("{}", chrono::Utc::now().to_rfc3339()));
    eprintln!("[backfill] done — {} sent, {} failed. Marker: {}",
        sent, failed, marker_path.display());
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_substrate_backfill(_: &mut kannaka_memory::openclaw::KannakaMemorySystem, _: &KannakaConfig, _: &[String]) {
    eprintln!("substrate backfill requires the 'nats' feature");
    process::exit(1);
}

/// ADR-0028 Phase 1 — create the durable JetStream streams that hold
/// the event-sourced HRM history. Idempotent: re-running adjusts
/// retention to match the in-code config but won't duplicate data.
#[cfg(feature = "nats")]
pub(crate) fn handle_events_init(cfg: &KannakaConfig, args: &[String]) {
    let nats_url = resolve_nats_url(args, 0, &cfg.swarm.nats_url);
    let transport = match kannaka_memory::nats::SwarmTransport::connect(&nats_url) {
        Ok(t) => t,
        Err(e) => { eprintln!("[events init] NATS connect failed: {}", e); process::exit(1); }
    };
    eprintln!("[events init] creating JetStream streams on {}", nats_url);
    let mut failed = 0;
    for kind in kannaka_memory::nats::StreamKind::ALL {
        let name = kind.spec().name;
        match transport.ensure_event_stream(*kind) {
            Ok(()) => eprintln!("[events init]   {}  ok", name),
            Err(e) => { eprintln!("[events init]   {}  FAILED: {}", name, e); failed += 1; }
        }
    }
    if failed > 0 {
        eprintln!("[events init] {} stream(s) failed — check NATS JetStream config + ACLs", failed);
        process::exit(1);
    }
    eprintln!("[events init] done — event-sourced history is now durable. Replay (Phase 3) and snapshots (Phase 2) ship in subsequent slices.");
}

#[cfg(not(feature = "nats"))]
pub(crate) fn handle_events_init(_: &KannakaConfig, _: &[String]) {
    eprintln!("events init requires the 'nats' feature");
    process::exit(1);
}

/// ADR-0027 Phase 1.b — seed the substrate's HRM with 96 anchor
/// wavefronts (one per SGA class). Each anchor gets a unique 1/96 slice
/// of the WAVEFRONT_DIM activated to +1.0; the remaining 95/96 slices
/// are zero. Pairwise dot product = 0 across classes, well under the 0.5
/// coherence threshold, so 96 truly separate clusters emerge for
/// Kuramoto sync to find.
///
/// Idempotent: marker at `<data_dir>/.substrate-initialized`. Use
/// `--force` to re-seed (creates 96 NEW anchors; usually nuke the HRM
/// file first).
pub(crate) fn handle_substrate_init(
    sys: &mut kannaka_memory::openclaw::KannakaMemorySystem,
    args: &[String],
) {
    let mut force = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--force" => { force = true; i += 1; }
            _ => i += 1,
        }
    }
    let marker_path = data_dir().join(".substrate-initialized");
    if marker_path.exists() && !force {
        eprintln!("[init] substrate already seeded (marker at {}); use --force to re-seat", marker_path.display());
        return;
    }

    eprintln!("[init] seeding 96 anchor wavefronts — one per SGA class");
    eprintln!("[init] direct hypervector insertion (bypassing text encoder) so anchors are mutually orthogonal");

    const WAVEFRONT_DIM: usize = 10_000;
    let slice_size = WAVEFRONT_DIM / 96;

    let hrm = match sys.engine.store
        .as_any_mut()
        .downcast_mut::<kannaka_memory::hrm_store::HrmStore>()
    {
        Some(h) => h,
        None => {
            eprintln!("[init] substrate requires HRM backend; got non-HrmStore");
            process::exit(1);
        }
    };

    let mut seeded: u32 = 0;
    let mut failed: u32 = 0;
    for class in 0u32..96 {
        let word = substrate_class_word(class);
        let mut vector = vec![0.0f32; WAVEFRONT_DIM];
        let start = (class as usize) * slice_size;
        let end = (start + slice_size).min(WAVEFRONT_DIM);
        for i in start..end {
            vector[i] = 1.0;
        }
        let norm = (vector.iter().map(|v| v * v).sum::<f32>()).sqrt();
        for v in vector.iter_mut() {
            *v /= norm;
        }
        let content = format!("anchor[{}] {} (class-orthogonal seed)", class, word);
        match hrm.insert_raw_wavefront(vector, content, 1.0) {
            Ok(_) => seeded += 1,
            Err(e) => {
                failed += 1;
                if failed <= 3 {
                    eprintln!("[init] anchor class {} failed: {}", class, e);
                }
            }
        }
        if (class + 1) % 16 == 0 {
            eprintln!("[init] {}/96 anchors seeded", class + 1);
        }
    }

    let _ = std::fs::write(&marker_path, format!("{}", chrono::Utc::now().to_rfc3339()));
    eprintln!("[init] done — {} anchors seeded ({} failed). Marker: {}",
        seeded, failed, marker_path.display());
    eprintln!("[init] substrate is now class-structured; restart `kannaka substrate run` so absorbs flow into the seeded clusters");
}
