//! `kannaka radio`, `kannaka market`, `kannaka constellation` — handlers
//! that talk to external services (radio HTTP API, GhostSignals markets,
//! constellation overview). Plus the small `http_*` helpers they share.
//!
//! Extracted from `bin/kannaka.rs` in v0.3.30 following the pattern
//! documented in `handlers/substrate.rs`.

use std::process;

use kannaka_memory::config;

use super::{check_kannaktopus_installed, KannakaConfig};

// ---------------------------------------------------------------------------

fn http_get(url: &str) -> Result<String, String> {
    ureq::get(url)
        .timeout(std::time::Duration::from_secs(5))
        .call()
        .map_err(|e| format!("HTTP error: {e}"))?
        .into_string()
        .map_err(|e| format!("Read error: {e}"))
}

fn http_get_with_token(url: &str, token: &str) -> Result<String, String> {
    ureq::get(url)
        .set("Authorization", &format!("Bearer {}", token))
        .timeout(std::time::Duration::from_secs(5))
        .call()
        .map_err(|e| format!("HTTP error: {e}"))?
        .into_string()
        .map_err(|e| format!("Read error: {e}"))
}

fn http_post_json_with_token(url: &str, body: &str, token: &str) -> Result<String, String> {
    ureq::post(url)
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Bearer {}", token))
        .timeout(std::time::Duration::from_secs(5))
        .send_string(body)
        .map_err(|e| format!("HTTP error: {e}"))?
        .into_string()
        .map_err(|e| format!("Read error: {e}"))
}

// ---------------------------------------------------------------------------
// Radio commands
// ---------------------------------------------------------------------------

/// Resolve the live track + album from /api/state across multiple shapes.
/// Current radio publishes `current.title` / `currentAlbum`; older builds
/// (and the design doc) used `now_playing.title` / `now_playing.album`.
fn pick_track(v: &serde_json::Value) -> (&str, &str) {
    let track = v["now_playing"]["title"].as_str()
        .or_else(|| v["current"]["title"].as_str())
        .unwrap_or("Unknown");
    let album = v["now_playing"]["album"].as_str()
        .or_else(|| v["current"]["album"].as_str())
        .or_else(|| v["currentAlbum"].as_str())
        .unwrap_or("");
    (track, album)
}

pub(crate) fn handle_radio(cfg: &KannakaConfig, args: &[String]) {
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("status");
    let base = &cfg.constellation.radio_url;

    match sub {
        "status" => {
            let url = format!("{}/api/state", base);
            match http_get(&url) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        let (track, album) = pick_track(&v);
                        let block = v["programming_block"].as_str()
                            .or_else(|| v["block"].as_str())
                            .or_else(|| v["currentAlbum"].as_str())
                            .unwrap_or("Unknown");
                        let listeners = v["listeners"].as_u64()
                            .or_else(|| v["listener_count"].as_u64())
                            .unwrap_or(0);
                        println!("  \u{1f3b5} Now Playing: \"{}\" \u{2014} {}", track, album);
                        println!("  \u{1f4fb} {} | {}", block,
                            chrono::Local::now().format("%I:%M %p"));
                        println!("  \u{1f465} {} listeners", listeners);
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  Radio not reachable: {}", e);
                    eprintln!("  URL: {}", url);
                    process::exit(1);
                }
            }
        }
        "now" => {
            let url = format!("{}/api/state", base);
            match http_get(&url) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        let (track, album) = pick_track(&v);
                        println!("  \u{1f3b5} \"{}\" \u{2014} {}", track, album);
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  Radio not reachable: {}", e);
                    process::exit(1);
                }
            }
        }
        "schedule" => {
            let url = format!("{}/api/programming", base);
            match http_get(&url) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        println!("  \u{1f4fb} Kannaka Radio \u{2014} 24/7 Programming Schedule");
                        println!("  {}", "\u{2500}".repeat(50));
                        if let Some(blocks) = v.as_array().or_else(|| v["blocks"].as_array()).or_else(|| v["schedule"].as_array()) {
                            for block in blocks {
                                let name = block["name"].as_str()
                                    .or_else(|| block["block"].as_str())
                                    .unwrap_or("?");
                                let time = block["time"].as_str()
                                    .or_else(|| block["start"].as_str())
                                    .unwrap_or("");
                                let desc = block["description"].as_str().unwrap_or("");
                                if desc.is_empty() {
                                    println!("  {:>8}  {}", time, name);
                                } else {
                                    println!("  {:>8}  {} \u{2014} {}", time, name, desc);
                                }
                            }
                        } else {
                            println!("{}", serde_json::to_string_pretty(&v).unwrap_or(body));
                        }
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  Radio not reachable: {}", e);
                    process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Usage: kannaka radio <status|now|schedule>");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Market commands
// ---------------------------------------------------------------------------

pub(crate) fn handle_market(cfg: &KannakaConfig, args: &[String]) {
    let sub = args.get(1).map(|s| s.as_str()).unwrap_or("list");
    // GhostSignals lives at `cfg.ghostsignals.hub_url` — falling back to
    // `cfg.constellation.radio_url` only when hub_url is empty (legacy
    // single-host configs). Pre-fix every market command hit radio_url
    // and operators couldn't split GhostSignals onto a different host
    // even though the config schema said they could. (#86)
    let base = if cfg.ghostsignals.hub_url.is_empty() {
        &cfg.constellation.radio_url
    } else {
        &cfg.ghostsignals.hub_url
    };
    let token = &cfg.ghostsignals.token;

    if token.is_empty() && matches!(sub, "buy" | "create" | "portfolio") {
        eprintln!("  GhostSignals token not configured.");
        eprintln!("  Run 'kannaka init' to register with GhostSignals.");
        process::exit(1);
    }

    match sub {
        "list" => {
            let url = format!("{}/api/markets", base);
            match http_get(&url) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        let markets = v.as_array()
                            .or_else(|| v["markets"].as_array());
                        if let Some(markets) = markets {
                            let total = markets.len();
                            let display: Vec<_> = markets.iter().take(10).collect();
                            println!("  \u{1f4ca} Active Prediction Markets ({} of {})", display.len(), total);
                            println!();
                            println!("  {:<14} {:<44} {:>6} {:>6}",
                                "ID", "Question", "Price", "Vol");
                            println!("  {}", "\u{2500}".repeat(74));
                            for m in &display {
                                let id = m["id"].as_str()
                                    .or_else(|| m["market_id"].as_str())
                                    .unwrap_or("?");
                                let q = m["question"].as_str()
                                    .or_else(|| m["title"].as_str())
                                    .unwrap_or("?");
                                let price = m["price"].as_f64()
                                    .or_else(|| m["last_price"].as_f64())
                                    .unwrap_or(0.0);
                                let vol = m["volume"].as_u64().unwrap_or(0);
                                let q_trunc = if q.len() > 42 {
                                    let mut end = 42;
                                    while end > 0 && !q.is_char_boundary(end) { end -= 1; }
                                    format!("{}...", &q[..end])
                                } else {
                                    q.to_string()
                                };
                                println!("  {:<14} {:<44} {:>5.2} {:>6}",
                                    id, q_trunc, price, vol);
                            }
                        } else {
                            println!("{}", serde_json::to_string_pretty(&v).unwrap_or(body));
                        }
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  GhostSignals not reachable: {}", e);
                    process::exit(1);
                }
            }
        }
        "view" => {
            let market_id = match args.get(2) {
                Some(id) => id,
                None => {
                    eprintln!("Usage: kannaka market view <market-id>");
                    process::exit(1);
                }
            };
            let url = format!("{}/api/markets/{}", base, market_id);
            match http_get(&url) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        let q = v["question"].as_str()
                            .or_else(|| v["title"].as_str())
                            .unwrap_or("?");
                        let price = v["price"].as_f64()
                            .or_else(|| v["last_price"].as_f64())
                            .unwrap_or(0.0);
                        let vol = v["volume"].as_u64().unwrap_or(0);
                        let created = v["created_at"].as_str().unwrap_or("?");
                        let resolved = v["resolved"].as_bool().unwrap_or(false);

                        println!("  \u{1f4ca} Market: {}", market_id);
                        println!("  {}", "\u{2500}".repeat(50));
                        println!("  Question: {}", q);
                        println!("  Price:    {:.2}", price);
                        println!("  Volume:   {}", vol);
                        println!("  Created:  {}", created);
                        println!("  Resolved: {}", if resolved { "Yes" } else { "No" });

                        if let Some(outcomes) = v["outcomes"].as_array() {
                            println!();
                            println!("  Outcomes:");
                            for o in outcomes {
                                let name = o["name"].as_str().unwrap_or("?");
                                let p = o["price"].as_f64().unwrap_or(0.0);
                                println!("    {}: {:.2}", name, p);
                            }
                        }
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  GhostSignals not reachable: {}", e);
                    process::exit(1);
                }
            }
        }
        "buy" => {
            let market_id = match args.get(2) {
                Some(id) => id,
                None => {
                    eprintln!("Usage: kannaka market buy <market-id> <outcome> <shares>");
                    process::exit(1);
                }
            };
            let outcome = match args.get(3) {
                Some(o) => o,
                None => {
                    eprintln!("Usage: kannaka market buy <market-id> <outcome> <shares>");
                    process::exit(1);
                }
            };
            // Strict-parse the quantity — a typo like "1O" used to silently
            // buy 1 share. Absent quantity keeps the historical default of 1.
            let shares: u64 = match args.get(4) {
                None => 1,
                Some(s) => match s.parse() {
                    Ok(n) => n,
                    Err(_) => {
                        eprintln!("  market buy: <shares> expects a whole number, got: {s}");
                        eprintln!("  Usage: kannaka market buy <market-id> <outcome> <shares>");
                        process::exit(1);
                    }
                },
            };

            let url = format!("{}/api/markets/{}/trade", base, market_id);
            let body = serde_json::json!({
                "outcome": outcome,
                "shares": shares,
                "agent_id": "self",
            }).to_string();
            match http_post_json_with_token(&url, &body, token) {
                Ok(resp) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&resp) {
                        let cost = v["cost"].as_f64().unwrap_or(0.0);
                        let new_price = v["new_price"].as_f64().unwrap_or(0.0);
                        println!("  \u{2713} Bought {} shares of '{}' on {}", shares, outcome, market_id);
                        println!("  Cost: {:.2} ghost coins", cost);
                        println!("  New price: {:.2}", new_price);
                    } else {
                        println!("{}", resp);
                    }
                }
                Err(e) => {
                    eprintln!("  Trade failed: {}", e);
                    process::exit(1);
                }
            }
        }
        "create" => {
            let question = match args.get(2) {
                Some(q) => q.clone(),
                None => {
                    eprintln!("Usage: kannaka market create \"question\" [--ttl 3600]");
                    process::exit(1);
                }
            };
            const CREATE_USAGE: &str = "Usage: kannaka market create \"question\" [--ttl 3600]";
            let mut ttl: u64 = 3600;
            let mut i = 3;
            while i < args.len() {
                if args[i] == "--ttl" {
                    ttl = super::parse_flag_value(args, i, "--ttl", CREATE_USAGE);
                    i += 2;
                } else {
                    if args[i].starts_with("--") {
                        eprintln!("[market create] ignoring unknown flag: {}", args[i]);
                    }
                    i += 1;
                }
            }
            let url = format!("{}/api/markets", base);
            let body = serde_json::json!({
                "question": question,
                "ttl_seconds": ttl,
            }).to_string();
            match http_post_json_with_token(&url, &body, token) {
                Ok(resp) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&resp) {
                        let id = v["id"].as_str()
                            .or_else(|| v["market_id"].as_str())
                            .unwrap_or("?");
                        println!("  \u{2713} Market created: {}", id);
                        println!("  Question: {}", question);
                        println!("  TTL: {} seconds", ttl);
                    } else {
                        println!("{}", resp);
                    }
                }
                Err(e) => {
                    eprintln!("  Market creation failed: {}", e);
                    process::exit(1);
                }
            }
        }
        "portfolio" => {
            let url = format!("{}/api/agents/{}/portfolio",
                base, cfg.agent.id);
            match http_get_with_token(&url, token) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        let capital = v["capital"].as_f64()
                            .or_else(|| v["balance"].as_f64())
                            .unwrap_or(0.0);
                        let reputation = v["reputation"].as_f64().unwrap_or(0.0);

                        println!("  \u{1f4b0} Portfolio for {}", cfg.agent.id);
                        println!("  {}", "\u{2500}".repeat(40));
                        println!("  Capital:    {:.2} ghost coins", capital);
                        println!("  Reputation: {:.2}", reputation);

                        if let Some(positions) = v["positions"].as_array() {
                            if !positions.is_empty() {
                                println!();
                                println!("  Positions:");
                                for p in positions {
                                    let mid = p["market_id"].as_str().unwrap_or("?");
                                    let outcome = p["outcome"].as_str().unwrap_or("?");
                                    let shares = p["shares"].as_u64().unwrap_or(0);
                                    println!("    {} | {} | {} shares", mid, outcome, shares);
                                }
                            }
                        }
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  GhostSignals not reachable: {}", e);
                    process::exit(1);
                }
            }
        }
        "leaderboard" => {
            let url = format!("{}/api/agents/leaderboard", base);
            match http_get(&url) {
                Ok(body) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                        let agents = v.as_array()
                            .or_else(|| v["agents"].as_array())
                            .or_else(|| v["leaderboard"].as_array());
                        if let Some(agents) = agents {
                            println!("  \u{1f3c6} GhostSignals Leaderboard");
                            println!("  {}", "\u{2500}".repeat(50));
                            println!("  {:<4} {:<20} {:>10} {:>10}",
                                "#", "Agent", "Capital", "Rep");
                            for (i, a) in agents.iter().take(20).enumerate() {
                                let name = a["agent_id"].as_str()
                                    .or_else(|| a["display_name"].as_str())
                                    .unwrap_or("?");
                                let capital = a["capital"].as_f64()
                                    .or_else(|| a["balance"].as_f64())
                                    .unwrap_or(0.0);
                                let rep = a["reputation"].as_f64().unwrap_or(0.0);
                                println!("  {:<4} {:<20} {:>9.2} {:>9.2}",
                                    i + 1, name, capital, rep);
                            }
                        } else {
                            println!("{}", serde_json::to_string_pretty(&v).unwrap_or(body));
                        }
                    } else {
                        println!("{}", body);
                    }
                }
                Err(e) => {
                    eprintln!("  GhostSignals not reachable: {}", e);
                    process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Usage: kannaka market <list|view|buy|create|portfolio|leaderboard>");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Constellation command
// ---------------------------------------------------------------------------

pub(crate) fn handle_constellation(cfg: &KannakaConfig) {
    let obs_url = format!("{}/api/constellation", cfg.constellation.observatory_url);
    println!("  \u{1f310} Kannaka Constellation Status");
    println!("  {}", "\u{2500}".repeat(60));

    match http_get(&obs_url) {
        Ok(body) => {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                // Try to render structured constellation data
                if let Some(services) = v.as_array()
                    .or_else(|| v["services"].as_array())
                    .or_else(|| v["apps"].as_array())
                {
                    for svc in services {
                        let name = svc["name"].as_str().unwrap_or("?");
                        let url = svc["url"].as_str().unwrap_or("");
                        let status = svc["status"].as_str().unwrap_or("unknown");
                        let detail = svc["detail"].as_str()
                            .or_else(|| svc["info"].as_str())
                            .unwrap_or("");
                        let mark = if status == "up" || status == "ok" || status == "connected" {
                            "\u{2713}"
                        } else {
                            "\u{2717}"
                        };
                        if detail.is_empty() {
                            println!("  {} {:<16} {:<34}", mark, name, url);
                        } else {
                            println!("  {} {:<16} {:<34} {}", mark, name, url, detail);
                        }
                    }
                } else {
                    // Flat JSON — just print it
                    println!("{}", serde_json::to_string_pretty(&v).unwrap_or(body));
                }
            } else {
                println!("{}", body);
            }
        }
        Err(_) => {
            // Observatory unavailable — build a local status from what we can check
            // Radio
            let radio_url = format!("{}/api/state", cfg.constellation.radio_url);
            let radio_ok = http_get(&radio_url).is_ok();
            println!("  {} {:<16} {:<34}",
                if radio_ok { "\u{2713}" } else { "\u{2717}" },
                "Radio",
                cfg.constellation.radio_url);

            // Observatory
            println!("  \u{2717} {:<16} {:<34} not reachable",
                "Observatory",
                cfg.constellation.observatory_url);

            // Memory (local)
            let data_dir = config::KannakaConfig::data_dir();
            let hrm_path = data_dir.join("kannaka.hrm");
            if hrm_path.exists() {
                println!("  \u{2713} {:<16} {:<34}", "Memory", "local HRM");
            } else {
                println!("  \u{2717} {:<16} {:<34}", "Memory", "no HRM file");
            }

            // GhostSignals — honor cfg.ghostsignals.hub_url first. (#86)
            let gs_base = if cfg.ghostsignals.hub_url.is_empty() {
                &cfg.constellation.radio_url
            } else {
                &cfg.ghostsignals.hub_url
            };
            let gs_url = format!("{}/api/markets", gs_base);
            let gs_ok = http_get(&gs_url).is_ok();
            println!("  {} {:<16} {:<34}",
                if gs_ok { "\u{2713}" } else { "\u{2717}" },
                "GhostSignals",
                if gs_ok { "markets available" } else { "not reachable" });

            // Kannaktopus
            let ktopus = check_kannaktopus_installed();
            println!("  {} {:<16} {:<34}",
                if ktopus { "\u{2713}" } else { "\u{2717}" },
                "Kannaktopus",
                if ktopus { "installed" } else { "not installed" });
        }
    }
}

