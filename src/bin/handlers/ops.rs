//! `kannaka orchestrate`, `kannaka config`, `kannaka search`, `kannaka export`,
//! `kannaka import` — ops-utility handlers.
//!
//! Extracted from `bin/kannaka.rs` in v0.3.31 following the pattern
//! documented in `handlers/substrate.rs`.

use std::process;

use super::{check_kannaktopus_installed, KannakaConfig};

pub(crate) fn handle_orchestrate(args: &[String]) {
    if !check_kannaktopus_installed() {
        eprintln!("  Kannaktopus is not installed.");
        eprintln!("  Install it with: npm install -g kannaktopus");
        process::exit(1);
    }

    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("status");

    match sub {
        "run" => {
            let task = match args.get(2) {
                Some(t) => t,
                None => {
                    eprintln!("Usage: kannaka orchestrate run \"task description\"");
                    process::exit(1);
                }
            };
            let status = std::process::Command::new("kannaktopus")
                .args(["run", task])
                .status();
            match status {
                Ok(s) if !s.success() => {
                    process::exit(s.code().unwrap_or(1));
                }
                Err(e) => {
                    eprintln!("  Failed to run kannaktopus: {}", e);
                    process::exit(1);
                }
                _ => {}
            }
        }
        "status" => {
            let status = std::process::Command::new("kannaktopus")
                .arg("status")
                .status();
            match status {
                Ok(s) if !s.success() => {
                    process::exit(s.code().unwrap_or(1));
                }
                Err(e) => {
                    eprintln!("  Failed to run kannaktopus: {}", e);
                    process::exit(1);
                }
                _ => {}
            }
        }
        "agents" => {
            let status = std::process::Command::new("kannaktopus")
                .arg("agents")
                .status();
            match status {
                Ok(s) if !s.success() => {
                    process::exit(s.code().unwrap_or(1));
                }
                Err(e) => {
                    eprintln!("  Failed to run kannaktopus: {}", e);
                    process::exit(1);
                }
                _ => {}
            }
        }
        _ => {
            eprintln!("Usage: kannaka orchestrate <run|status|agents>");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Config commands
// ---------------------------------------------------------------------------

pub(crate) fn handle_config(cfg: &KannakaConfig, args: &[String]) {
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("show");

    match sub {
        "show" => {
            // Print config with redacted API keys
            let mut display = cfg.clone();
            if display.llm.api_key.len() > 8 {
                display.llm.api_key = format!("{}...", &display.llm.api_key[..8]);
            } else if !display.llm.api_key.is_empty() {
                display.llm.api_key = "***".to_string();
            }
            if display.ghostsignals.token.len() > 8 {
                display.ghostsignals.token = format!("{}...", &display.ghostsignals.token[..8]);
            } else if !display.ghostsignals.token.is_empty() {
                display.ghostsignals.token = "***".to_string();
            }
            match toml::to_string_pretty(&display) {
                Ok(text) => println!("{}", text),
                Err(e) => {
                    eprintln!("Error serializing config: {}", e);
                    process::exit(1);
                }
            }
        }
        "set" => {
            let key = match args.get(2) {
                Some(k) => k,
                None => {
                    eprintln!("Usage: kannaka config set <key> <value>");
                    eprintln!();
                    eprintln!("Keys: agent.id, agent.display_name, llm.provider, llm.model,");
                    eprintln!("      llm.api_key, llm.base_url, swarm.enabled, swarm.nats_url,");
                    eprintln!("      ghostsignals.enabled, ghostsignals.token,");
                    eprintln!("      constellation.radio_url, constellation.observatory_url,");
                    eprintln!("      updates.auto_check");
                    process::exit(1);
                }
            };
            let value = match args.get(3) {
                Some(v) => v,
                None => {
                    eprintln!("Usage: kannaka config set <key> <value>");
                    process::exit(1);
                }
            };

            let mut new_cfg = cfg.clone();
            match key.as_str() {
                "agent.id" => new_cfg.agent.id = value.clone(),
                "agent.display_name" => new_cfg.agent.display_name = value.clone(),
                "llm.provider" => new_cfg.llm.provider = value.clone(),
                "llm.model" => new_cfg.llm.model = value.clone(),
                "llm.api_key" => new_cfg.llm.api_key = value.clone(),
                "llm.base_url" => new_cfg.llm.base_url = value.clone(),
                "swarm.enabled" => new_cfg.swarm.enabled = value == "true",
                "swarm.nats_url" => new_cfg.swarm.nats_url = value.clone(),
                "ghostsignals.enabled" => new_cfg.ghostsignals.enabled = value == "true",
                "ghostsignals.token" => new_cfg.ghostsignals.token = value.clone(),
                "constellation.radio_url" => new_cfg.constellation.radio_url = value.clone(),
                "constellation.observatory_url" => new_cfg.constellation.observatory_url = value.clone(),
                "updates.auto_check" => new_cfg.updates.auto_check = value == "true",
                other => {
                    eprintln!("Unknown config key: {}", other);
                    process::exit(1);
                }
            }
            match new_cfg.save() {
                Ok(()) => println!("  \u{2713} Set {} = {}", key, value),
                Err(e) => {
                    eprintln!("Error saving config: {}", e);
                    process::exit(1);
                }
            }
        }
        "path" => {
            println!("{}", KannakaConfig::config_path().display());
        }
        _ => {
            eprintln!("Usage: kannaka config <show|set|path>");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Search command
// ---------------------------------------------------------------------------

pub(crate) fn handle_search(sys: &mut kannaka_memory::openclaw::KannakaMemorySystem, args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: kannaka search \"query\" [--limit N]");
        process::exit(1);
    }
    let mut limit = 20usize;
    let mut query_parts = Vec::new();
    let mut i = 1;
    while i < args.len() {
        if (args[i] == "--limit" || args[i] == "--top-k") && i + 1 < args.len() {
            limit = args[i + 1].parse().unwrap_or(20);
            i += 2;
        } else {
            query_parts.push(args[i].as_str());
            i += 1;
        }
    }
    let query = query_parts.join(" ");
    match sys.recall(&query, limit) {
        Ok(results) => {
            if results.is_empty() {
                println!("  No results for \"{}\"", query);
                return;
            }
            println!("  \u{1f50d} Search: \"{}\" ({} results)", query, results.len());
            println!("  {}", "\u{2500}".repeat(70));
            for (i, r) in results.iter().enumerate() {
                let preview = if r.content.len() > 70 {
                    let mut end = 70;
                    while end > 0 && !r.content.is_char_boundary(end) { end -= 1; }
                    format!("{}...", &r.content[..end])
                } else {
                    r.content.clone()
                };
                println!("  {:>3}. [{:.2}] {}", i + 1, r.similarity, preview);
                println!("       id={} age={:.0}h strength={:.2}",
                    r.id, r.age_hours, r.strength);
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Export command
// ---------------------------------------------------------------------------

pub(crate) fn handle_export(sys: &mut kannaka_memory::openclaw::KannakaMemorySystem, args: &[String]) {
    let mut output_path: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if (args[i] == "--output" || args[i] == "-o") && i + 1 < args.len() {
            output_path = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--format" && i + 1 < args.len() {
            // Only JSON is supported, but accept the flag
            i += 2;
        } else {
            i += 1;
        }
    }

    let all_mems = sys.engine.store.all_memories()
        .unwrap_or_else(|e| { eprintln!("Error: {}", e); process::exit(1); });

    let output: Vec<serde_json::Value> = all_mems.iter().map(|m| {
        serde_json::json!({
            "id": m.id.to_string(),
            "content": m.content,
            "amplitude": m.amplitude,
            "frequency": m.frequency,
            "phase": m.phase,
            "decay_rate": m.decay_rate,
            "created_at": m.created_at.to_rfc3339(),
            "layer_depth": m.layer_depth,
            "hallucinated": m.hallucinated,
            "parents": m.parents,
            "vector": m.vector,
            "xi_signature": m.xi_signature,
            "geometry": m.geometry,
            "connections": m.connections.iter().map(|c| {
                serde_json::json!({
                    "target_id": c.target_id.to_string(),
                    "strength": c.strength,
                    "span": c.span
                })
            }).collect::<Vec<_>>()
        })
    }).collect();

    let json_str = serde_json::to_string(&output).unwrap();

    if let Some(path) = output_path {
        match std::fs::write(&path, &json_str) {
            Ok(()) => {
                println!("  \u{2713} Exported {} memories to {}", all_mems.len(), path);
            }
            Err(e) => {
                eprintln!("Error writing {}: {}", path, e);
                process::exit(1);
            }
        }
    } else {
        println!("{}", json_str);
    }
}

// ---------------------------------------------------------------------------
// Import command
// ---------------------------------------------------------------------------

pub(crate) fn handle_import(sys: &mut kannaka_memory::openclaw::KannakaMemorySystem, args: &[String]) {
    let path = match args.get(1) {
        Some(p) => p,
        None => {
            eprintln!("Usage: kannaka import <file.json>");
            process::exit(1);
        }
    };

    let file_data = std::fs::read_to_string(path)
        .unwrap_or_else(|e| { eprintln!("Failed to read {}: {}", path, e); process::exit(1); });
    let memories: Vec<serde_json::Value> = serde_json::from_str(&file_data)
        .unwrap_or_else(|e| { eprintln!("Failed to parse JSON: {}", e); process::exit(1); });

    let existing_ids: std::collections::HashSet<uuid::Uuid> = sys.engine.store.all_memories()
        .unwrap_or_default().iter().map(|m| m.id).collect();

    let mut imported = 0u32;
    let mut skipped = 0u32;
    let mut errors = 0u32;

    for val in &memories {
        let id_str = val["id"].as_str().unwrap_or("");
        let id = match uuid::Uuid::parse_str(id_str) {
            Ok(id) => id,
            Err(_) => { errors += 1; continue; }
        };

        if existing_ids.contains(&id) {
            skipped += 1;
            continue;
        }

        let content = val["content"].as_str().unwrap_or("").to_string();
        if content.is_empty() { skipped += 1; continue; }

        let amplitude = val["amplitude"].as_f64().unwrap_or(0.5) as f32;

        match sys.engine.store.absorb(&content, amplitude, None) {
            Ok(_) => { imported += 1; }
            Err(e) => {
                if errors < 5 { eprintln!("  Error importing {}: {}", id_str, e); }
                errors += 1;
            }
        }
    }

    if imported > 0 {
        if let Err(e) = sys.save() {
            eprintln!("Failed to save: {}", e);
            process::exit(1);
        }
    }

    println!("  \u{2713} Import complete");
    println!("    Imported: {}", imported);
    println!("    Skipped:  {} (duplicates)", skipped);
    println!("    Errors:   {}", errors);
    println!("    Total:    {} in file", memories.len());
}
