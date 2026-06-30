//! qBraid Lab / infrastructure tools for the `kannaka agent` harness.
//!
//! Where [`crate::quantum_tools`] runs *circuits* on qBraid, these tools manage
//! the *platform* around them — credits, environments, compute profiles, the
//! Lab server, and on-demand instances — by shelling out to the same
//! `kannaka-quantum` bridge (its `lab-*` subcommands, which wrap `qbraid_core`).
//!
//! Three safety tiers, classified here and gated by the agent handler:
//! - **read-only** ([`is_lab_readonly_tool`]) — visibility ops; never spend,
//!   never mutate → auto-allowed like the quantum tools.
//! - **free mutations** — env/package/kernel lifecycle + stop/down; cost no
//!   credits but change state → routed through human approval.
//! - **paid** ([`is_lab_paid_tool`]) — start/provision/resume compute; bill
//!   credits *per wall-clock minute* → require `allow_spend` + `max_credits`
//!   in the call AND human approval (the bridge refuses a paid run otherwise).

use crate::agent::Tool;
use serde_json::{json, Value};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Compute provisioning + `--wait` polling can take minutes; give it headroom.
const BRIDGE_TIMEOUT_SECS: u64 = 660;

fn python_bin() -> String {
    std::env::var("KANNAKA_QUANTUM_PYTHON").unwrap_or_else(|_| "python".to_string())
}

/// True for any tool handled here.
pub fn is_lab_tool(name: &str) -> bool {
    matches!(
        name,
        "lab_credits"
            | "lab_list_envs"
            | "lab_env_info"
            | "lab_list_profiles"
            | "lab_compute_status"
            | "lab_compute_usage"
            | "lab_list_instances"
            | "lab_list_kernels"
            | "lab_create_env"
            | "lab_delete_env"
            | "lab_pip_install"
            | "lab_pip_freeze"
            | "lab_add_kernel"
            | "lab_remove_kernel"
            | "lab_compute_up"
            | "lab_compute_down"
            | "lab_provision_instance"
            | "lab_start_instance"
            | "lab_stop_instance"
            | "lab_ssh_configure"
            | "lab_agent_launch"
            | "lab_agent_list"
            | "lab_agent_read"
            | "lab_agent_send"
    )
}

/// Read-only / $0 visibility tools — safe to auto-allow.
pub fn is_lab_readonly_tool(name: &str) -> bool {
    matches!(
        name,
        "lab_credits"
            | "lab_list_envs"
            | "lab_env_info"
            | "lab_list_profiles"
            | "lab_compute_status"
            | "lab_compute_usage"
            | "lab_list_instances"
            | "lab_list_kernels"
            | "lab_pip_freeze"
            | "lab_agent_list"
            | "lab_agent_read"
    )
}

/// Tools that spend qBraid credits (per-minute compute) — must never be
/// auto-approved, and the bridge requires `allow_spend` + a `max_credits` cap.
pub fn is_lab_paid_tool(name: &str) -> bool {
    matches!(name, "lab_compute_up" | "lab_provision_instance" | "lab_start_instance")
}

/// The lab toolset exposed to the model. Names/descriptions are `'static` so the
/// prompt cache stays stable across turns.
pub fn lab_tools() -> Vec<Tool> {
    vec![
        // --- Phase 1: read-only / $0 ------------------------------------- //
        Tool {
            name: "lab_credits",
            description: "Show the qBraid credit balance (1 credit = $0.01) and its USD value. Free, read-only.",
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        Tool {
            name: "lab_list_envs",
            description: "List qBraid environments available to the account (name, slug, description, tags). Free, read-only.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "page": { "type": "integer", "minimum": 1, "default": 1 },
                    "limit": { "type": "integer", "minimum": 1, "default": 20 }
                }
            }),
        },
        Tool {
            name: "lab_env_info",
            description: "Get full metadata for one qBraid environment by its slug. Free, read-only.",
            input_schema: json!({
                "type": "object",
                "properties": { "slug": { "type": "string" } },
                "required": ["slug"]
            }),
        },
        Tool {
            name: "lab_list_profiles",
            description: "List qBraid compute profiles (instance types) with live per-minute credit cost, USD/min, GPU \
                          flag, plan, and capacity. Call this BEFORE starting paid compute to choose a profile and see \
                          its burn rate. Free, read-only.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "gpu_only": { "type": "boolean", "default": false, "description": "Only GPU profiles." },
                    "available_only": { "type": "boolean", "default": false, "description": "Only profiles ready (with capacity)." },
                    "plan": { "type": "string", "description": "Filter by plan (e.g. Free, Standard, Pro)." },
                    "limit": { "type": "integer", "minimum": 1 }
                }
            }),
        },
        Tool {
            name: "lab_compute_status",
            description: "Report the qBraid Lab server status: whether it's running, on which profile, and its URL. Free, read-only.",
            input_schema: json!({
                "type": "object",
                "properties": { "cluster": { "type": "string", "description": "Cluster id (default 'lab')." } }
            }),
        },
        Tool {
            name: "lab_compute_usage",
            description: "Show qBraid compute usage and per-session credit rates / totals charged (optionally for the last N days). Free, read-only.",
            input_schema: json!({
                "type": "object",
                "properties": { "days": { "type": "integer", "minimum": 1 } }
            }),
        },
        Tool {
            name: "lab_list_instances",
            description: "List on-demand (BMA) qBraid compute instances with status, URL, and running/stopped credits-per-minute. Free, read-only.",
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        Tool {
            name: "lab_list_kernels",
            description: "List Jupyter kernels installed locally (only meaningful when running inside qBraid Lab / on a provisioned instance). Free, read-only.",
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        // --- Phase 2: free mutations (env / package / kernel) ------------ //
        Tool {
            name: "lab_create_env",
            description: "Create a qBraid environment. Any 'packages' listed install during the (async, cloud-side) build — \
                          this is the way to install packages into a qBraid environment when not running inside Lab. \
                          Free (no credits).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "description": { "type": "string" },
                    "python_version": { "type": "string", "description": "e.g. 3.11" },
                    "packages": {
                        "description": "Object {name: version} or array of package names to install during the build.",
                        "oneOf": [ { "type": "object" }, { "type": "array", "items": { "type": "string" } } ]
                    },
                    "kernel_name": { "type": "string" },
                    "visibility": { "type": "string", "default": "private" },
                    "tags": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["name"]
            }),
        },
        Tool {
            name: "lab_delete_env",
            description: "Delete a qBraid environment by slug. Free (no credits).",
            input_schema: json!({
                "type": "object",
                "properties": { "slug": { "type": "string" } },
                "required": ["slug"]
            }),
        },
        Tool {
            name: "lab_pip_install",
            description: "pip-install packages into a LOCALLY-installed environment (keyed by its local env_id, NOT the cloud \
                          slug). Only works inside qBraid Lab / on a provisioned instance. Free.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "env_id": { "type": "string", "description": "Local registry env id (not the cloud slug)." },
                    "packages": { "type": "array", "items": { "type": "string" } },
                    "upgrade_pip": { "type": "boolean", "default": false }
                },
                "required": ["env_id", "packages"]
            }),
        },
        Tool {
            name: "lab_pip_freeze",
            description: "List packages installed in a locally-installed environment (in-Lab only). Free, read-only.",
            input_schema: json!({
                "type": "object",
                "properties": { "env_id": { "type": "string" } },
                "required": ["env_id"]
            }),
        },
        Tool {
            name: "lab_add_kernel",
            description: "Register a Jupyter kernel for a (locally-installed) environment (in-Lab only). Free.",
            input_schema: json!({
                "type": "object",
                "properties": { "environment": { "type": "string" } },
                "required": ["environment"]
            }),
        },
        Tool {
            name: "lab_remove_kernel",
            description: "Remove a Jupyter kernel for an environment (in-Lab only). Free.",
            input_schema: json!({
                "type": "object",
                "properties": { "environment": { "type": "string" } },
                "required": ["environment"]
            }),
        },
        // --- Phase 3: paid compute provisioning -------------------------- //
        Tool {
            name: "lab_compute_up",
            description: "Start the qBraid Lab server on a compute profile. PAID — bills qBraid credits per wall-clock minute \
                          until stopped. Requires allow_spend=true and a max_credits ceiling (the balance you accept to \
                          risk). Call lab_list_profiles first for the burn rate; stop with lab_compute_down.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "profile": { "type": "string", "description": "Profile slug from lab_list_profiles." },
                    "allow_spend": { "type": "boolean", "default": false, "description": "Required true — paid compute." },
                    "max_credits": { "type": "number", "description": "Committed credit ceiling (default 60 ≈ $0.60)." },
                    "cluster": { "type": "string" },
                    "wait": { "type": "boolean", "default": false, "description": "Block until the server is ready." },
                    "timeout": { "type": "number", "description": "Seconds to wait when wait=true." }
                },
                "required": ["profile", "allow_spend", "max_credits"]
            }),
        },
        Tool {
            name: "lab_compute_down",
            description: "Stop the running qBraid Lab server (preserves disk, stops per-minute billing). Free.",
            input_schema: json!({
                "type": "object",
                "properties": { "cluster": { "type": "string" } }
            }),
        },
        Tool {
            name: "lab_provision_instance",
            description: "Provision (launch) a new on-demand qBraid compute instance on a profile. PAID — per-minute credits. \
                          Requires allow_spend=true + max_credits. Pause it later with lab_stop_instance.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "profile": { "type": "string", "description": "Profile slug from lab_list_profiles." },
                    "allow_spend": { "type": "boolean", "default": false },
                    "max_credits": { "type": "number" },
                    "wait": { "type": "boolean", "default": false },
                    "timeout": { "type": "number" }
                },
                "required": ["profile", "allow_spend", "max_credits"]
            }),
        },
        Tool {
            name: "lab_start_instance",
            description: "Resume a stopped on-demand instance by instance_id. PAID — per-minute credits. Requires allow_spend=true + max_credits.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "instance_id": { "type": "string" },
                    "allow_spend": { "type": "boolean", "default": false },
                    "max_credits": { "type": "number" }
                },
                "required": ["instance_id", "allow_spend", "max_credits"]
            }),
        },
        Tool {
            name: "lab_stop_instance",
            description: "Stop (pause) an on-demand instance by instance_id — preserves disk, stops running billing (a stopped \
                          instance still bills a small disk rate; terminate-to-delete is via the web UI by design). Free to call.",
            input_schema: json!({
                "type": "object",
                "properties": { "instance_id": { "type": "string" } },
                "required": ["instance_id"]
            }),
        },
        // --- Phase 4: remote agents (run a coding agent on an instance) --- //
        Tool {
            name: "lab_ssh_configure",
            description: "Configure local SSH access to a running on-demand instance and return its stable SSH alias. \
                          Run this once before any lab_agent_* tool. Free.",
            input_schema: json!({
                "type": "object",
                "properties": { "instance_id": { "type": "string" } },
                "required": ["instance_id"]
            }),
        },
        Tool {
            name: "lab_agent_launch",
            description: "Launch a coding agent (claude / codex / opencode) ON a remote provisioned instance over SSH — \
                          have Kannaka drive another agent on cloud compute. Needs lab_ssh_configure first and the \
                          instance running. The remote agent may incur its own model-API costs.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ssh_alias": { "type": "string", "description": "From lab_ssh_configure." },
                    "tool": { "type": "string", "default": "claude", "description": "claude | codex | opencode" },
                    "instructions": { "type": "string", "description": "Initial task/prompt for the remote agent." },
                    "cwd": { "type": "string" },
                    "name": { "type": "string" },
                    "agent_type": { "type": "string" },
                    "tags": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["ssh_alias"]
            }),
        },
        Tool {
            name: "lab_agent_list",
            description: "List coding agents running on a remote instance. Read-only.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ssh_alias": { "type": "string" },
                    "include_stopped": { "type": "boolean", "default": false }
                },
                "required": ["ssh_alias"]
            }),
        },
        Tool {
            name: "lab_agent_read",
            description: "Read recent terminal output from a remote agent session. Read-only.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ssh_alias": { "type": "string" },
                    "session_id": { "type": "string" },
                    "lines": { "type": "integer", "minimum": 1, "default": 50 }
                },
                "required": ["ssh_alias", "session_id"]
            }),
        },
        Tool {
            name: "lab_agent_send",
            description: "Send a prompt / keystrokes to a remote agent session (drive it). Mutating.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ssh_alias": { "type": "string" },
                    "session_id": { "type": "string" },
                    "text": { "type": "string" }
                },
                "required": ["ssh_alias", "session_id", "text"]
            }),
        },
    ]
}

/// Execute a lab tool by shelling out to the bridge's matching `lab-*`
/// subcommand. Returns `(result_text, is_error)`.
pub fn dispatch_lab_tool(name: &str, input: &Value) -> (String, bool) {
    // Enforce the paid-compute contract at the harness, not only in the bridge:
    // the bridge has an env-var opt-in path, so the Rust side must independently
    // require an explicit allow_spend=true + a positive max_credits ceiling.
    if is_lab_paid_tool(name) {
        if input.get("allow_spend").and_then(|v| v.as_bool()) != Some(true) {
            return (
                format!(
                    "{name} is a PAID per-minute compute action and requires allow_spend=true. \
                     Call lab_credits + lab_list_profiles first, then retry with allow_spend=true \
                     and a max_credits ceiling."
                ),
                true,
            );
        }
        if let Err(e) = req_num(input, "max_credits") {
            return (e, true);
        }
    }
    let mut args: Vec<String> = Vec::new();
    match name {
        "lab_credits" => args.push("lab-credits".into()),
        "lab_list_envs" => {
            args.push("lab-list-envs".into());
            push_int(&mut args, "--page", input, "page");
            push_int(&mut args, "--limit", input, "limit");
        }
        "lab_env_info" => {
            let slug = match req_str(input, "slug") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-env-info".into());
            args.push("--slug".into());
            args.push(slug);
        }
        "lab_list_profiles" => {
            args.push("lab-list-profiles".into());
            push_flag(&mut args, "--gpu-only", input, "gpu_only");
            push_flag(&mut args, "--available-only", input, "available_only");
            push_str_opt(&mut args, "--plan", input, "plan");
            push_int(&mut args, "--limit", input, "limit");
        }
        "lab_compute_status" => {
            args.push("lab-compute-status".into());
            push_str_opt(&mut args, "--cluster", input, "cluster");
        }
        "lab_compute_usage" => {
            args.push("lab-compute-usage".into());
            push_int(&mut args, "--days", input, "days");
        }
        "lab_list_instances" => args.push("lab-list-instances".into()),
        "lab_list_kernels" => args.push("lab-list-kernels".into()),
        "lab_create_env" => {
            let n = match req_str(input, "name") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-create-env".into());
            args.push("--name".into());
            args.push(n);
            push_str_opt(&mut args, "--description", input, "description");
            push_str_opt(&mut args, "--python-version", input, "python_version");
            push_json_opt(&mut args, "--packages", input, "packages");
            push_str_opt(&mut args, "--kernel-name", input, "kernel_name");
            push_str_opt(&mut args, "--visibility", input, "visibility");
            push_json_opt(&mut args, "--tags", input, "tags");
        }
        "lab_delete_env" => {
            let slug = match req_str(input, "slug") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-delete-env".into());
            args.push("--slug".into());
            args.push(slug);
        }
        "lab_pip_install" => {
            let env_id = match req_str(input, "env_id") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            match input.get("packages") {
                Some(p) if p.is_array() => {
                    args.push("lab-pip-install".into());
                    args.push("--env-id".into());
                    args.push(env_id);
                    args.push("--packages".into());
                    args.push(p.to_string());
                    push_flag(&mut args, "--upgrade-pip", input, "upgrade_pip");
                }
                _ => return ("lab_pip_install requires a packages array".into(), true),
            }
        }
        "lab_pip_freeze" => {
            let env_id = match req_str(input, "env_id") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-pip-freeze".into());
            args.push("--env-id".into());
            args.push(env_id);
        }
        "lab_add_kernel" | "lab_remove_kernel" => {
            let env = match req_str(input, "environment") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push(if name == "lab_add_kernel" { "lab-add-kernel".into() } else { "lab-remove-kernel".into() });
            args.push("--environment".into());
            args.push(env);
        }
        "lab_compute_up" => {
            let profile = match req_str(input, "profile") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-compute-up".into());
            args.push("--profile".into());
            args.push(profile);
            push_str_opt(&mut args, "--cluster", input, "cluster");
            push_flag(&mut args, "--wait", input, "wait");
            push_num(&mut args, "--timeout", input, "timeout");
            push_spend_args(&mut args, input);
        }
        "lab_compute_down" => {
            args.push("lab-compute-down".into());
            push_str_opt(&mut args, "--cluster", input, "cluster");
        }
        "lab_provision_instance" => {
            let profile = match req_str(input, "profile") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-provision-instance".into());
            args.push("--profile".into());
            args.push(profile);
            push_flag(&mut args, "--wait", input, "wait");
            push_num(&mut args, "--timeout", input, "timeout");
            push_spend_args(&mut args, input);
        }
        "lab_start_instance" => {
            let id = match req_str(input, "instance_id") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-start-instance".into());
            args.push("--instance-id".into());
            args.push(id);
            push_spend_args(&mut args, input);
        }
        "lab_stop_instance" => {
            let id = match req_str(input, "instance_id") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-stop-instance".into());
            args.push("--instance-id".into());
            args.push(id);
        }
        "lab_ssh_configure" => {
            let id = match req_str(input, "instance_id") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-ssh-configure".into());
            args.push("--instance-id".into());
            args.push(id);
        }
        "lab_agent_launch" => {
            let alias = match req_str(input, "ssh_alias") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-agent-launch".into());
            args.push("--ssh-alias".into());
            args.push(alias);
            push_str_opt(&mut args, "--tool", input, "tool");
            push_str_opt(&mut args, "--instructions", input, "instructions");
            push_str_opt(&mut args, "--cwd", input, "cwd");
            push_str_opt(&mut args, "--name", input, "name");
            push_str_opt(&mut args, "--agent-type", input, "agent_type");
            push_json_opt(&mut args, "--tags", input, "tags");
        }
        "lab_agent_list" => {
            let alias = match req_str(input, "ssh_alias") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-agent-list".into());
            args.push("--ssh-alias".into());
            args.push(alias);
            push_flag(&mut args, "--include-stopped", input, "include_stopped");
        }
        "lab_agent_read" => {
            let alias = match req_str(input, "ssh_alias") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            let sid = match req_str(input, "session_id") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-agent-read".into());
            args.push("--ssh-alias".into());
            args.push(alias);
            args.push("--session-id".into());
            args.push(sid);
            push_int(&mut args, "--lines", input, "lines");
        }
        "lab_agent_send" => {
            let alias = match req_str(input, "ssh_alias") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            let sid = match req_str(input, "session_id") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            let text = match req_str(input, "text") {
                Ok(s) => s,
                Err(e) => return (e, true),
            };
            args.push("lab-agent-send".into());
            args.push("--ssh-alias".into());
            args.push(alias);
            args.push("--session-id".into());
            args.push(sid);
            args.push("--text".into());
            args.push(text);
        }
        other => return (format!("unknown lab tool: {other}"), true),
    }
    run_bridge(&args)
}

fn req_str(input: &Value, key: &str) -> Result<String, String> {
    match input.get(key).and_then(|v| v.as_str()) {
        Some(s) if !s.is_empty() => Ok(s.to_string()),
        _ => Err(format!("lab tool requires a non-empty '{key}'")),
    }
}

/// Require a strictly-positive numeric input (the committed credit ceiling).
fn req_num(input: &Value, key: &str) -> Result<f64, String> {
    match input.get(key).and_then(|v| v.as_f64()) {
        Some(n) if n > 0.0 => Ok(n),
        _ => Err(format!("paid lab tool requires a positive numeric '{key}' (committed credit ceiling)")),
    }
}

/// Append `--flag` if the named boolean input is true.
fn push_flag(args: &mut Vec<String>, flag: &str, input: &Value, key: &str) {
    if input.get(key).and_then(|v| v.as_bool()) == Some(true) {
        args.push(flag.to_string());
    }
}

/// Append `--opt <value>` for a non-empty string input.
fn push_str_opt(args: &mut Vec<String>, opt: &str, input: &Value, key: &str) {
    if let Some(s) = input.get(key).and_then(|v| v.as_str()) {
        if !s.is_empty() {
            args.push(opt.to_string());
            args.push(s.to_string());
        }
    }
}

/// Append `--opt <int>` for an integer input.
fn push_int(args: &mut Vec<String>, opt: &str, input: &Value, key: &str) {
    if let Some(n) = input.get(key).and_then(|v| v.as_i64()) {
        args.push(opt.to_string());
        args.push(n.to_string());
    }
}

/// Append `--opt <number>` for a numeric input.
fn push_num(args: &mut Vec<String>, opt: &str, input: &Value, key: &str) {
    if let Some(n) = input.get(key).and_then(|v| v.as_f64()) {
        args.push(opt.to_string());
        args.push(n.to_string());
    }
}

/// Append `--opt <json>` for an array/object input (the bridge parses it).
fn push_json_opt(args: &mut Vec<String>, opt: &str, input: &Value, key: &str) {
    if let Some(v) = input.get(key) {
        if v.is_array() || v.is_object() {
            args.push(opt.to_string());
            args.push(v.to_string());
        } else if let Some(s) = v.as_str() {
            // The bridge's _parse_json_arg also accepts a CSV/JSON string, so
            // forward a string form rather than silently dropping it.
            if !s.is_empty() {
                args.push(opt.to_string());
                args.push(s.to_string());
            }
        }
    }
}

/// Append `--allow-spend` / `--max-credits` from the tool input (paid tools).
fn push_spend_args(args: &mut Vec<String>, input: &Value) {
    if input.get("allow_spend").and_then(|v| v.as_bool()) == Some(true) {
        args.push("--allow-spend".to_string());
    }
    if let Some(mc) = input.get("max_credits").and_then(|v| v.as_f64()) {
        args.push("--max-credits".to_string());
        args.push(mc.to_string());
    }
}

fn run_bridge(args: &[String]) -> (String, bool) {
    // NOTE: the pipes are drained only after the child exits (via
    // wait_with_output below). This is safe ONLY because every `lab-*`
    // subcommand prints a single small JSON object (cli.py does one
    // `print(json.dumps(...))`, no streaming) — well under the OS pipe buffer.
    // If a subcommand is ever made to stream large output, switch to draining
    // stdout/stderr on reader threads to avoid a pipe-buffer deadlock.
    let py = python_bin();
    let mut full: Vec<String> = vec!["-m".to_string(), "kannaka_quantum".to_string()];
    full.extend_from_slice(args);

    let mut child = match Command::new(&py)
        .args(&full)
        // Force Python UTF-8 mode: the remote-agent tools read a remote TUI
        // whose box-drawing chars crash qBraid's default-cp1252 subprocess
        // decoding on Windows. UTF-8 mode makes that decode correctly.
        .env("PYTHONUTF8", "1")
        .env("KANNAKA_QUIET", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return (
                format!(
                    "quantum bridge unavailable: could not run '{py} -m kannaka_quantum' ({e}). \
                     Install it: pip install git+https://github.com/NickFlach/kannaka-quantum \
                     (set KANNAKA_QUANTUM_PYTHON if python isn't on PATH)."
                ),
                true,
            );
        }
    };
    let _ = child.stdin.take(); // explicitly close stdin

    let start = Instant::now();
    let timed_out = loop {
        match child.try_wait() {
            Ok(Some(_)) => break false,
            Ok(None) => {
                if start.elapsed() > Duration::from_secs(BRIDGE_TIMEOUT_SECS) {
                    let _ = child.kill();
                    let _ = child.wait();
                    break true;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return (format!("quantum bridge wait failed: {e}"), true),
        }
    };

    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return (format!("quantum bridge output failed: {e}"), true),
    };
    if timed_out {
        return ("lab operation timed out (a compute action may still be in progress — check lab_compute_status)".into(), true);
    }
    let body = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if body.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let last = stderr.trim().lines().last().unwrap_or("no output");
        return (format!("quantum bridge produced no result: {last}"), true);
    }
    let is_error = serde_json::from_str::<Value>(&body)
        .map(|v| v.get("error").is_some())
        .unwrap_or(false);
    (body, is_error)
}
