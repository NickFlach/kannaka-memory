//! `kannaka identity` — spacechild-auth SSO identity for swarm agents.
//!
//! Step 1 of real agent identity: register/login against the SpaceChild
//! auth service, persist the session to `<data_dir>/identity.json`, and
//! introspect it with `whoami` (auto-refreshing on a stale access token).
//! Follows the handler-extraction pattern documented in
//! `handlers/substrate.rs`. All protocol logic lives in
//! `kannaka_memory::identity`; this file is CLI wiring only.
//!
//! Token hygiene: tokens are never printed. `whoami` shows user id,
//! email, and access-token expiry only.

use std::io::Write;
use std::process;

use kannaka_memory::identity::{
    self, AuthClient, IdentityError, IdentityStore, UserInfo,
};

const USAGE: &str = "Usage: kannaka identity <register|login|whoami|logout> [--email ADDR]";

pub(crate) fn handle_identity(args: &[String]) {
    let sub = match args.get(1) {
        Some(s) => s.as_str(),
        None => {
            eprintln!("{USAGE}");
            process::exit(1);
        }
    };

    match sub {
        "register" => handle_register(args),
        "login" => handle_login(args),
        "whoami" => handle_whoami(),
        "logout" => handle_logout(),
        _ => {
            eprintln!("{USAGE}");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------

fn handle_register(args: &[String]) {
    let email = require_email(args, "Usage: kannaka identity register --email <addr>");
    let password = read_password(&email);
    let client = AuthClient::from_env();

    match client.register(&email, &password) {
        Ok(out) => {
            if let Some((access, refresh)) = out.tokens {
                let path = store_session(&out.user_id, &out.email, access, refresh);
                println!("  \u{2713} Registered as {} (user {})", out.email, out.user_id);
                println!("  Identity stored at {}", path.display());
            } else if out.requires_verification {
                println!("  \u{2713} Registered {} — email verification required.", out.email);
                if let Some(m) = out.message {
                    println!("  {}", m);
                }
                println!("  After verifying, run: kannaka identity login --email {}", out.email);
            } else {
                // Registered but no tokens and no verification flag — odd,
                // but recoverable via a normal login.
                println!("  \u{2713} Registered {} (user {})", out.email, out.user_id);
                println!("  Run: kannaka identity login --email {}", out.email);
            }
        }
        Err(e) => {
            eprintln!("  identity register failed: {e}");
            process::exit(1);
        }
    }
}

fn handle_login(args: &[String]) {
    let email = require_email(args, "Usage: kannaka identity login --email <addr>");
    let password = read_password(&email);
    let client = AuthClient::from_env();

    match client.login(&email, &password) {
        Ok(out) => {
            let path = store_session(&out.user_id, &out.email, out.access_token, out.refresh_token);
            println!("  \u{2713} Logged in as {} (user {})", out.email, out.user_id);
            println!("  Identity stored at {}", path.display());
        }
        Err(e) => {
            eprintln!("  identity login failed: {e}");
            process::exit(1);
        }
    }
}

fn handle_whoami() {
    let path = identity::identity_path();
    let store = match IdentityStore::load(&path) {
        Ok(Some(s)) => s,
        Ok(None) => {
            eprintln!("  Not logged in — run: kannaka identity login --email <addr>");
            process::exit(1);
        }
        Err(e) => {
            eprintln!("  identity whoami failed: {e}");
            process::exit(1);
        }
    };
    let client = AuthClient::from_env();

    match client.user(&store.access_token) {
        Ok(user) => print_whoami(&user, &store.access_token),
        // Stale access token — refresh once with the stored refreshToken,
        // persist the new pair, and retry.
        Err(IdentityError::Status { status: 401, .. }) => {
            match client.refresh(&store.refresh_token) {
                Ok((access, refresh)) => {
                    let refreshed = IdentityStore {
                        access_token: access,
                        refresh_token: refresh,
                        obtained_at: chrono::Utc::now(),
                        ..store
                    };
                    if let Err(e) = refreshed.save(&path) {
                        eprintln!("  warning: could not persist refreshed tokens: {e}");
                    }
                    match client.user(&refreshed.access_token) {
                        Ok(user) => print_whoami(&user, &refreshed.access_token),
                        Err(e) => {
                            eprintln!("  identity whoami failed after refresh: {e}");
                            process::exit(1);
                        }
                    }
                }
                Err(_) => {
                    eprintln!(
                        "  Session expired — run: kannaka identity login --email {}",
                        store.email
                    );
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("  identity whoami failed: {e}");
            process::exit(1);
        }
    }
}

fn handle_logout() {
    let path = identity::identity_path();
    match IdentityStore::delete(&path) {
        Ok(true) => println!("  \u{2713} Logged out — credentials removed from {}", path.display()),
        Ok(false) => println!("  Not logged in (no stored identity)."),
        Err(e) => {
            eprintln!("  identity logout failed: {e}");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// id/email/expiry only — tokens never reach stdout.
fn print_whoami(user: &UserInfo, access_token: &str) {
    println!("  Identity: {}", user.email);
    println!("  User id:  {}", user.id);
    if let Some(verified) = user.email_verified {
        println!("  Verified: {}", if verified { "yes" } else { "no" });
    }
    if let Some(created) = &user.created_at {
        println!("  Created:  {}", created);
    }
    match identity::jwt_exp_unix(access_token)
        .and_then(|exp| chrono::DateTime::from_timestamp(exp, 0))
    {
        Some(exp) => println!("  Token expires: {}", exp.format("%Y-%m-%d %H:%M:%S UTC")),
        None => println!("  Token expires: unknown"),
    }
}

/// Persist a fresh session, returning the path it was written to.
fn store_session(
    user_id: &str,
    email: &str,
    access_token: String,
    refresh_token: String,
) -> std::path::PathBuf {
    let path = identity::identity_path();
    let store = IdentityStore {
        user_id: user_id.to_string(),
        email: email.to_string(),
        access_token,
        refresh_token,
        obtained_at: chrono::Utc::now(),
    };
    if let Err(e) = store.save(&path) {
        eprintln!("  identity: failed to store session: {e}");
        process::exit(1);
    }
    path
}

/// Required `--email <addr>` flag.
fn require_email(args: &[String], usage: &str) -> String {
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--email" {
            return super::parse_flag_value::<String>(args, i, "--email", usage);
        }
        i += 1;
    }
    eprintln!("{usage}");
    process::exit(1);
}

/// Password resolution: `KANNAKA_AUTH_PASSWORD` env (the no-echo path for
/// scripts/agents) > a line read from stdin. There's no rpassword dep, so
/// an interactive TTY will echo — the env var is the recommended path.
fn read_password(email: &str) -> String {
    if let Ok(p) = std::env::var("KANNAKA_AUTH_PASSWORD") {
        if !p.is_empty() {
            return p;
        }
    }
    eprint!("Password for {email} (input echoes; set KANNAKA_AUTH_PASSWORD to avoid): ");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    if std::io::stdin().read_line(&mut line).is_err() {
        eprintln!("identity: failed to read password from stdin");
        process::exit(1);
    }
    let password = line.trim_end_matches(['\r', '\n']).to_string();
    if password.is_empty() {
        eprintln!("identity: no password provided (set KANNAKA_AUTH_PASSWORD or type one on stdin)");
        process::exit(1);
    }
    password
}
