//! `kannaka nostr` — ADR-0043 Phase 0 identity tooling.
//!
//! Mints per-role secp256k1 keys and emits the Phase-0 artifacts an operator
//! needs to stand up a Nostr identity for an organ: the `nsec`/`npub`, a signed
//! kind-0 profile event ready to publish, and the NIP-05 map fragment for
//! `ninja-portal.com/.well-known/nostr.json`. CLI wiring only — all crypto and
//! serialization live in `kannaka_memory::nostr`.
//!
//! Key custody: `keygen` prints the `nsec` ONCE to stdout for the operator to
//! place in a 0600 env file (the NATS-creds pattern); it is never written to
//! disk here and never logged. The reputation-bearing voice key is not managed
//! by this tool — these are disposable per-role keys (bridge, labs, dvm…).

use std::process;

use kannaka_memory::nostr::{npub_from_pubkey_hex, Event, Keypair};

const USAGE: &str = "Usage: kannaka nostr <keygen|profile|nip05|verify> [args]\n\
    \n\
    keygen  [--role <name>]                 mint a per-role key; prints nsec (once) + npub\n\
    profile --nsec <nsec> --name <n> [--about <a>] [--nip05 <id@domain>] [--picture <url>]\n\
    \x20                                       emit a signed kind-0 profile event (JSON)\n\
    nip05   --name <local> --pubkey <hex|npub>  emit the .well-known/nostr.json fragment\n\
    verify  [--file <path>]                  verify a NIP-01 event JSON (stdin if no --file)";

pub(crate) fn handle_nostr(args: &[String]) {
    let sub = match args.get(1) {
        Some(s) => s.as_str(),
        None => {
            eprintln!("{USAGE}");
            process::exit(1);
        }
    };
    match sub {
        "keygen" => keygen(args),
        "profile" => profile(args),
        "nip05" => nip05(args),
        "verify" => verify(args),
        "-h" | "--help" | "help" => println!("{USAGE}"),
        other => {
            eprintln!("kannaka nostr: unknown subcommand '{other}'\n{USAGE}");
            process::exit(1);
        }
    }
}

/// Value of `--flag <value>`, or None.
fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
}

fn die(msg: &str) -> ! {
    eprintln!("kannaka nostr: {msg}");
    process::exit(1);
}

fn keygen(args: &[String]) {
    let role = flag(args, "--role").unwrap_or("role");
    let kp = Keypair::generate();
    let nsec = kp
        .to_nsec()
        .unwrap_or_else(|e| die(&format!("nsec encode: {e}")));
    let npub = kp
        .to_npub()
        .unwrap_or_else(|e| die(&format!("npub encode: {e}")));
    // The nsec is a secret. Print it once with a custody reminder; the operator
    // moves it into /home/opc/.kannaka-<role>-nostr.env (0600). We never persist
    // it. npub + hex pubkey are public and safe to echo anywhere.
    println!("# kannaka nostr key — role: {role}");
    println!("# SECRET — store nsec in a 0600 env file (NATS-creds pattern); never commit it.");
    println!(
        "KANNAKA_{}_NOSTR_NSEC={nsec}",
        role.to_uppercase().replace('-', "_")
    );
    println!("npub={npub}");
    println!("pubkey_hex={}", kp.public_hex());
}

fn profile(args: &[String]) {
    let nsec = flag(args, "--nsec").unwrap_or_else(|| die("profile requires --nsec"));
    let name = flag(args, "--name").unwrap_or_else(|| die("profile requires --name"));
    let kp = Keypair::from_nsec(nsec).unwrap_or_else(|e| die(&format!("bad --nsec: {e}")));

    // kind-0 content is a JSON object of profile metadata (NIP-01).
    let mut meta = serde_json::Map::new();
    meta.insert("name".into(), name.into());
    if let Some(about) = flag(args, "--about") {
        meta.insert("about".into(), about.into());
    }
    if let Some(nip05) = flag(args, "--nip05") {
        meta.insert("nip05".into(), nip05.into());
    }
    if let Some(picture) = flag(args, "--picture") {
        meta.insert("picture".into(), picture.into());
    }
    let content = serde_json::to_string(&serde_json::Value::Object(meta))
        .unwrap_or_else(|e| die(&format!("profile content: {e}")));

    // created_at from the wall clock; chrono is already a dep. Deterministic
    // per invocation is fine — this is an operator command, not the cron.
    let created_at = chrono::Utc::now().timestamp();
    let event = kp.sign_event(0, Vec::new(), &content, created_at);
    // Self-verify before printing — never emit an event we can't verify.
    if let Err(e) = event.verify() {
        die(&format!("internal: signed profile failed self-verify: {e}"));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&event).unwrap_or_else(|e| die(&format!("emit: {e}")))
    );
}

fn nip05(args: &[String]) {
    let name = flag(args, "--name").unwrap_or_else(|| die("nip05 requires --name"));
    let pk = flag(args, "--pubkey").unwrap_or_else(|| die("nip05 requires --pubkey"));
    // Accept either hex or npub; NIP-05 maps names → 32-byte hex pubkeys.
    let hex = if let Some(stripped) = pk.strip_prefix("npub1") {
        // decode npub → hex via a round-trip through the library
        match bech32_npub_to_hex(&format!("npub1{stripped}")) {
            Ok(h) => h,
            Err(e) => die(&format!("bad --pubkey npub: {e}")),
        }
    } else {
        if pk.len() != 64 || !pk.bytes().all(|b| b.is_ascii_hexdigit()) {
            die("--pubkey must be 64 hex chars or an npub1…");
        }
        pk.to_string()
    };
    // Emit just the fragment so the operator merges it into the existing
    // .well-known/nostr.json rather than clobbering other names.
    let fragment = serde_json::json!({ "names": { name: hex } });
    println!(
        "{}",
        serde_json::to_string_pretty(&fragment).unwrap_or_else(|e| die(&format!("emit: {e}")))
    );
}

/// npub → 32-byte hex, reusing the library's encoder as an oracle: we decode by
/// re-encoding candidate? No — decode directly through Keypair isn't possible
/// (public only). Use the library's npub encoder round-trip via a small local
/// bech32 decode.
fn bech32_npub_to_hex(npub: &str) -> Result<String, String> {
    let (hrp, data) = bech32::decode(npub).map_err(|e| e.to_string())?;
    if hrp.as_str() != "npub" || data.len() != 32 {
        return Err("not a 32-byte npub".into());
    }
    let mut s = String::with_capacity(64);
    for b in data {
        s.push_str(&format!("{b:02x}"));
    }
    // sanity: it must re-encode to the same npub
    if npub_from_pubkey_hex(&s).map_err(|e| e.to_string())? != npub {
        return Err("npub round-trip mismatch".into());
    }
    Ok(s)
}

fn verify(args: &[String]) {
    let json = match flag(args, "--file") {
        Some(path) => {
            std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("read {path}: {e}")))
        }
        None => {
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .unwrap_or_else(|e| die(&format!("read stdin: {e}")));
            buf
        }
    };
    let event: Event = serde_json::from_str(&json).unwrap_or_else(|e| die(&format!("parse: {e}")));
    match event.verify() {
        Ok(()) => {
            let npub = event.author_npub().unwrap_or_else(|_| "?".into());
            println!("OK  id={} kind={} author={}", event.id, event.kind, npub);
        }
        Err(e) => {
            eprintln!("INVALID  {e}");
            process::exit(2);
        }
    }
}
