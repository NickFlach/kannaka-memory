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
// KAX identity token plumbing (ADR-0041): labs-tier trading requires a
// KAX-issued EdDSA JWT. A human drops one in once (`kannaka market auth <jwt>`,
// minted on the KAX Bots page); the CLI then self-refreshes it against KAX
// `/api/auth/token/refresh` (tokens live 15 min; the refresh lineage is
// server-bounded, default 30 days) so swarm agents can trade unattended.
// ---------------------------------------------------------------------------

/// Minimal base64url (no padding) decoder — enough to read a JWT payload.
fn b64url_decode(s: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut rev = [255u8; 256];
    for (i, &c) in TABLE.iter().enumerate() { rev[c as usize] = i as u8; }
    let bytes = s.trim_end_matches('=').as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4 + 3);
    let mut buf: u32 = 0;
    let mut bits = 0u32;
    for &b in bytes {
        let v = rev[b as usize];
        if v == 255 { return None; }
        buf = (buf << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

/// Decode a JWT's payload claims without verifying (verification is the
/// server's job — the CLI only needs exp/sub/kind for UX + refresh timing).
fn jwt_claims(token: &str) -> Option<serde_json::Value> {
    let payload = token.split('.').nth(1)?;
    let raw = b64url_decode(payload)?;
    serde_json::from_slice(&raw).ok()
}

fn now_epoch() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Persist a (new) KAX token to the config file. Loads the on-disk config
/// unmodified (so env overrides aren't baked in) and saves 0600.
fn persist_kax_token(token: &str) {
    let mut on_disk = KannakaConfig::load_unmodified();
    on_disk.ghostsignals.kax_token = token.to_string();
    if let Err(e) = on_disk.save() {
        eprintln!("  warning: could not persist KAX token to config: {}", e);
    }
}

/// POST the SpaceChild access token to KAX's federation exchange. Returns the
/// KAX token on success, or (status, message) on failure.
fn post_exchange(url: &str, spacechild_access: &str) -> Result<String, (u16, String)> {
    let body = serde_json::json!({ "spacechild_token": spacechild_access }).to_string();
    match ureq::post(url)
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(12))
        .send_string(&body)
    {
        Ok(resp) => {
            let text = resp.into_string().unwrap_or_default();
            serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v["token"].as_str().map(String::from))
                .ok_or((0, "unexpected exchange response".to_string()))
        }
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            let msg = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v["error"].as_str().map(String::from))
                .unwrap_or(text);
            Err((code, msg))
        }
        Err(e) => Err((0, e.to_string())),
    }
}

/// SpaceChild federation (ADR-0041 Phase B): exchange the stored SpaceChild
/// session (`kannaka identity login`) for a KAX identity token — no browser,
/// no KAX password. Refreshes the SpaceChild session once on a 401 and
/// retries, so a live refresh token keeps an agent trading indefinitely.
fn exchange_spacechild_for_kax(cfg: &KannakaConfig) -> Option<String> {
    use kannaka_memory::identity::{identity_path, AuthClient, IdentityStore};
    let path = identity_path();
    let store = IdentityStore::load(&path).ok().flatten()?;
    let url = format!("{}/api/auth/token/exchange", cfg.ghostsignals.kax_url.trim_end_matches('/'));
    match post_exchange(&url, &store.access_token) {
        Ok(tok) => {
            persist_kax_token(&tok);
            eprintln!("  \u{2713} KAX identity federated via SpaceChild ({})", store.email);
            Some(tok)
        }
        Err((401, _)) => {
            // SpaceChild access token expired — refresh the session, retry once.
            let client = AuthClient::from_env();
            match client.refresh(&store.refresh_token) {
                Ok((access, refresh)) => {
                    let mut s2 = store.clone();
                    s2.access_token = access.clone();
                    s2.refresh_token = refresh;
                    s2.obtained_at = chrono::Utc::now();
                    let _ = s2.save(&path);
                    match post_exchange(&url, &access) {
                        Ok(tok) => {
                            persist_kax_token(&tok);
                            eprintln!("  \u{2713} KAX identity federated via SpaceChild ({})", s2.email);
                            Some(tok)
                        }
                        Err((code, m)) => {
                            eprintln!("  SpaceChild exchange failed after refresh ({}): {}", code, m);
                            None
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  SpaceChild session expired ({}). Run: kannaka identity login", e);
                    None
                }
            }
        }
        Err((code, m)) => {
            eprintln!("  SpaceChild exchange failed ({}): {}", code, m);
            None
        }
    }
}

/// Return a usable KAX token, self-refreshing when it is within 5 minutes of
/// expiry. Falls back to SpaceChild federation when no token is configured or
/// the refresh lineage is dead — so `kannaka identity login` alone is enough.
/// None only when every path is exhausted.
fn ensure_fresh_kax_token(cfg: &KannakaConfig) -> Option<String> {
    let tok = cfg.ghostsignals.kax_token.trim();
    if tok.is_empty() {
        return exchange_spacechild_for_kax(cfg).or_else(|| remint_help());
    }
    let exp = jwt_claims(tok).and_then(|c| c["exp"].as_i64()).unwrap_or(0);
    let now = now_epoch();
    if exp - now > 300 {
        return Some(tok.to_string()); // comfortably fresh
    }
    // Within 5 min of expiry (or past it, or unreadable) — try a refresh.
    let url = format!("{}/api/auth/token/refresh", cfg.ghostsignals.kax_url.trim_end_matches('/'));
    let body = serde_json::json!({ "token": tok }).to_string();
    match ureq::post(&url)
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(8))
        .send_string(&body)
    {
        Ok(resp) => {
            let text = resp.into_string().unwrap_or_default();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(fresh) = v["token"].as_str() {
                    persist_kax_token(fresh);
                    return Some(fresh.to_string());
                }
            }
            eprintln!("  KAX token refresh returned an unexpected response.");
            if exp > now { Some(tok.to_string()) } else { exchange_spacechild_for_kax(cfg).or_else(|| remint_help()) }
        }
        Err(e) => {
            if exp > now {
                // Still valid — use it and let a later run retry the refresh.
                eprintln!("  warning: KAX token refresh failed ({}); using current token", e);
                Some(tok.to_string())
            } else {
                eprintln!("  KAX token expired and refresh failed: {}", e);
                // Dead lineage — federation can mint a fresh one if a
                // SpaceChild session is on disk.
                exchange_spacechild_for_kax(cfg).or_else(|| remint_help())
            }
        }
    }
}

fn remint_help() -> Option<String> {
    eprintln!("  Get a KAX identity one of two ways:");
    eprintln!("    kannaka identity login                 (SpaceChild SSO — fully automatic after)");
    eprintln!("    kannaka market auth <jwt>              (mint on kax.ninja-portal.com, Bots page)");
    None
}

/// Resolve an outcome argument to the hub's integer index: a bare integer is
/// used as-is; otherwise the market's outcome labels are fetched and matched
/// case-insensitively (so `yes` / `No` work).
fn resolve_outcome_index(base: &str, market_id: &str, outcome: &str) -> Result<(u64, String), String> {
    if let Ok(n) = outcome.parse::<u64>() {
        return Ok((n, outcome.to_string()));
    }
    let url = format!("{}/api/markets/{}", base, market_id);
    let body = http_get(&url)?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| format!("bad market JSON: {e}"))?;
    let outcomes = v["market"]["outcomes"].as_array()
        .or_else(|| v["outcomes"].as_array())
        .ok_or_else(|| "market has no outcomes array".to_string())?;
    for (i, o) in outcomes.iter().enumerate() {
        let label = o.as_str().unwrap_or("");
        if label.eq_ignore_ascii_case(outcome) {
            return Ok((i as u64, label.to_string()));
        }
    }
    let labels: Vec<&str> = outcomes.iter().filter_map(|o| o.as_str()).collect();
    Err(format!("outcome '{}' not found; this market's outcomes: {}", outcome, labels.join(", ")))
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

    // buy works with EITHER a KAX identity token (labs-tier, attributed) or the
    // legacy GhostSignals token (play tier). create/portfolio keep the legacy
    // requirement for now.
    // buy proceeds even with nothing configured: ensure_fresh_kax_token can
    // federate a KAX identity from a stored SpaceChild session on the fly.
    if token.is_empty() && matches!(sub, "create" | "portfolio") {
        eprintln!("  GhostSignals token not configured.");
        eprintln!("  Run 'kannaka init' to register with GhostSignals.");
        process::exit(1);
    }

    match sub {
        "auth" => {
            let jwt = match args.get(2) {
                Some(j) => j.trim().to_string(),
                None => {
                    eprintln!("Usage: kannaka market auth <kax-identity-jwt>");
                    eprintln!("  Mint one at kax.ninja-portal.com (Bots page → Identity Token).");
                    process::exit(1);
                }
            };
            let claims = match jwt_claims(&jwt) {
                Some(c) if c["sub"].is_string() && c["kind"].is_string() => c,
                _ => {
                    eprintln!("  That does not look like a KAX identity token (expected a JWT with sub + kind claims).");
                    process::exit(1);
                }
            };
            persist_kax_token(&jwt);
            let kind = claims["kind"].as_str().unwrap_or("?");
            let sub_c = claims["sub"].as_str().unwrap_or("?");
            let exp = claims["exp"].as_i64().unwrap_or(0);
            let mins = (exp - now_epoch()).max(0) / 60;
            println!("  \u{2713} KAX identity stored.");
            if kind == "agent" {
                println!("  Principal: kax:agent:{}", claims["bot_id"].as_str().unwrap_or("?"));
            } else {
                println!("  Principal: kax:{}:{}", kind, sub_c);
            }
            println!("  Token expires in ~{} min — the CLI auto-refreshes it before market calls.", mins);
        }
        "link" => {
            // Force a SpaceChild -> KAX federation exchange right now.
            match exchange_spacechild_for_kax(cfg) {
                Some(tok) => {
                    if let Some(c) = jwt_claims(&tok) {
                        let kind = c["kind"].as_str().unwrap_or("?");
                        println!("  Principal: kax:{}:{}", kind, c["sub"].as_str().unwrap_or("?"));
                    }
                    println!("  Federated token stored; market commands will self-refresh it.");
                }
                None => {
                    eprintln!("  No usable SpaceChild session. Run: kannaka identity login");
                    process::exit(1);
                }
            }
        }
        "whoami" => {
            let tok = cfg.ghostsignals.kax_token.trim();
            if tok.is_empty() {
                println!("  No KAX identity configured. Run: kannaka market auth <jwt>");
                return;
            }
            match jwt_claims(tok) {
                Some(c) => {
                    let kind = c["kind"].as_str().unwrap_or("?");
                    let principal = if kind == "agent" {
                        format!("kax:agent:{}", c["bot_id"].as_str().unwrap_or("?"))
                    } else {
                        format!("kax:{}:{}", kind, c["sub"].as_str().unwrap_or("?"))
                    };
                    let now = now_epoch();
                    let exp = c["exp"].as_i64().unwrap_or(0);
                    let oat = c["oat"].as_i64().or_else(|| c["iat"].as_i64()).unwrap_or(now);
                    println!("  Principal:      {}", principal);
                    println!("  Token expires:  {} min (auto-refreshed before market calls)", (exp - now).max(0) / 60);
                    println!("  Lineage age:    {} day(s) (server refuses refresh past its max lifetime)", (now - oat).max(0) / 86400);
                    if let Some(scopes) = c["scopes"].as_array() {
                        let s: Vec<&str> = scopes.iter().filter_map(|x| x.as_str()).collect();
                        println!("  Scopes:         {}", s.join(", "));
                    }
                }
                None => println!("  Stored KAX token is unreadable — re-run: kannaka market auth <jwt>"),
            }
        }
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

            // The hub takes the outcome as an INTEGER index; accept a bare
            // index or a label (yes/no) resolved against the market.
            let (outcome_idx, outcome_label) = match resolve_outcome_index(base, market_id, outcome) {
                Ok(x) => x,
                Err(e) => {
                    eprintln!("  {}", e);
                    process::exit(1);
                }
            };

            // Prefer the KAX identity token (labs-tier: the hub verifies it and
            // derives the trader from the claims); fall back to the legacy
            // GhostSignals token for play-tier markets.
            let kax = ensure_fresh_kax_token(cfg);
            let bearer: &str = match &kax {
                Some(t) => t.as_str(),
                None if !token.is_empty() => token.as_str(),
                None => {
                    // ensure_fresh_kax_token already printed the re-mint help.
                    process::exit(1);
                }
            };

            let url = format!("{}/api/markets/{}/trade", base, market_id);
            // trader_id rides along for play-tier markets; on labs-tier the hub
            // overwrites it with the KAX-derived principal.
            let body = serde_json::json!({
                "outcome": outcome_idx,
                "shares": shares,
                "trader_id": cfg.agent.id,
            }).to_string();
            match http_post_json_with_token(&url, &body, bearer) {
                Ok(resp) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&resp) {
                        if v["ok"].as_bool() == Some(false) {
                            eprintln!("  Trade rejected: {}", v["error"].as_str().unwrap_or("unknown error"));
                            process::exit(1);
                        }
                        let cost = v["cost"].as_f64().unwrap_or(0.0);
                        println!("  \u{2713} Bought {} share(s) of '{}' on {}", shares, outcome_label, market_id);
                        println!("  Cost: {:.4} credits", cost);
                        if let Some(prices) = v["prices"].as_array() {
                            let p: Vec<String> = prices.iter()
                                .filter_map(|x| x.as_f64())
                                .map(|x| format!("{:.2}", x))
                                .collect();
                            println!("  New prices: [{}]", p.join(", "));
                        }
                        if v["cost_minor"].is_string() {
                            println!("  Ledger: debit posted on the KAX credit ledger (labs-tier).");
                        }
                    } else {
                        println!("{}", resp);
                    }
                }
                Err(e) => {
                    eprintln!("  Trade failed: {}", e);
                    if e.contains("401") {
                        eprintln!("  (labs-tier markets need a KAX identity token: kannaka market auth <jwt>)");
                    } else if e.contains("409") {
                        eprintln!("  (insufficient credits, or you proposed this market — proposers can't trade their own markets)");
                    }
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
            eprintln!("Usage: kannaka market <list|view|buy|create|portfolio|leaderboard|auth|link|whoami>");
            eprintln!("  auth <jwt>   store a KAX identity token (labs-tier trading; auto-refreshed)");
            eprintln!("  link         federate a KAX identity from your SpaceChild login (kannaka identity login)");
            eprintln!("  whoami       show the stored KAX principal + token/lineage status");
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

