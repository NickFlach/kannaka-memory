//! `kannaka-acp` — serve Kannaka's memory as an ACP agent over stdio.
//!
//! Speaks the Agent Client Protocol on stdin/stdout, so ACP clients can prompt
//! Kannaka and receive resonated memories. Two known consumers:
//!
//!   * `buzz-acp` — relays Buzz `@mentions` to an ACP agent over stdio.
//!   * Buzz desktop "bring your own harness" — discovers harnesses from JSON
//!     definitions in `<app_data>/custom_harnesses/` and spawns them over ACP.
//!
//! Read-only against the HRM by policy (single-writer); see `acp::HrmMemory`.
//!
//! Usage:
//!   kannaka-acp [--top-k N]
//!
//! Environment:
//!   KANNAKA_DATA_DIR   HRM data directory (default: ~/.kannaka)

use kannaka_memory::acp;

const USAGE: &str = "Usage: kannaka-acp [--top-k N]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut top_k = acp::DEFAULT_TOP_K;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--top-k" => {
                let raw = args.get(i + 1).unwrap_or_else(|| {
                    eprintln!("--top-k requires a value\n{USAGE}");
                    std::process::exit(2);
                });
                top_k = raw.parse::<usize>().unwrap_or_else(|_| {
                    eprintln!("--top-k must be a positive integer, got {raw:?}\n{USAGE}");
                    std::process::exit(2);
                });
                if top_k == 0 {
                    eprintln!("--top-k must be greater than zero\n{USAGE}");
                    std::process::exit(2);
                }
                i += 2;
            }
            // Tolerate a bare `acp` token. ACP clients spawn goose as
            // `goose acp`, and `buzz-acp`'s `--agent-args` defaults to "acp";
            // its `normalize_agent_args` only strips that default for runtimes
            // it recognizes, so an unrecognized command like this one receives
            // the token verbatim. Rejecting it would break the default
            // invocation for no benefit — we are always in ACP mode.
            arg if arg.eq_ignore_ascii_case("acp") => {
                i += 1;
            }
            "-h" | "--help" => {
                // stdout is the protocol stream, but --help is never used in a
                // protocol session, so printing there is correct for a CLI.
                println!("{USAGE}");
                return;
            }
            other => {
                eprintln!("unknown argument: {other}\n{USAGE}");
                std::process::exit(2);
            }
        }
    }

    if let Err(e) = acp::run(top_k) {
        eprintln!("[kannaka-acp] fatal: {e}");
        std::process::exit(1);
    }
}
