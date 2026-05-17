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

    // Publish substrate phi on a slow cadence — eigendecomp is the heavy
    // cost so we don't want to do it on every absorb. 60s feels live
    // enough for the observatory.
    const PHI_PUBLISH_SECS: u64 = 60;
    let mut last_phi_pub = Instant::now() - Duration::from_secs(PHI_PUBLISH_SECS);
    let mut contributors: HashSet<String> = HashSet::new();

    // Connection failure tracking — if the NATS server restarts under us
    // (or the network blips), the next publish/subscribe returns a broken-
    // pipe error. After MAX_CONSECUTIVE_FAILURES we exit(1) and let
    // systemd's Restart=on-failure bring us back with a fresh transport.
    const MAX_CONSECUTIVE_FAILURES: u32 = 3;
    let mut consecutive_failures: u32 = 0;

    eprintln!("[substrate] daemon ready — Ctrl+C to stop");
    loop {
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
