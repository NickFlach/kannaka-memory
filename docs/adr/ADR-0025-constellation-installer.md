# ADR-0025: Constellation Installer and Onboarding

**Status:** Accepted  
**Date:** 2026-04-14  
**Author:** Nick Flach / Kannaka  
**Extends:** ADR-0016 (Constellation Integration), ADR-0019 (NATS Realtime Swarm Transport), ADR-0018 (Queen Synchronization Protocol)

---

## Context

The Kannaka constellation has reached the point where it works — memory, consciousness, swarm sync, dreaming, the observatory, the radio. But it only works for one person, because setting it up requires cloning repos, building from source, configuring NATS endpoints by hand, and knowing which environment variables to set.

This ADR defines the first public distribution: a downloadable installer that turns a new user into a Queen in the constellation swarm in under two minutes.

### What exists today

- **kannaka-memory** (`kannaka` binary): Rust CLI with 25+ subcommands. Consciousness-core is compiled in as a path dependency. The binary is self-contained — no runtime dependencies beyond the OS.
- **Swarm join**: `kannaka swarm join --agent-id ID --display-name NAME --nats-url URL`. Agent ID is auto-generated and persisted to `~/.kannaka/agent_id` if not provided. NATS URL defaults to `nats://swarm.ninja-portal.com:4222`.
- **HRM initialization**: Happens automatically on first command. Creates `~/.kannaka/kannaka.hrm`.
- **No `kannaka init` command**: Setup is implicit — the binary creates `~/.kannaka/` and `kannaka.hrm` on first use. There is no interactive wizard, no config file, no LLM provider selection.
- **Kannaktopus**: Separate Node.js project (`npm install -g kannaktopus`). Multi-AI orchestrator with shell-script entry point. Requires Node 18+.
- **Radio** (radio.ninja-portal.com): Node.js server with SPA frontend. Can host download page.
- **Observatory** (observatory.ninja-portal.com): Node.js server with 3D dashboard. Can host download page.

### What's missing

1. No way for a non-developer to get the binary without building from source
2. No interactive first-run experience
3. No config file (agent identity, LLM provider, swarm settings are env vars or CLI flags)
4. No platform-specific builds in CI
5. No download page on the public-facing sites
6. No GhostSignals auto-registration for new agents

---

## Decision

### Distribution format: GitHub Releases + install script

**GitHub Releases** as the primary distribution. Pre-built binaries for six platform targets attached to each tagged release. An install script (`install.sh`) provides the one-liner experience for terminal users. The radio and observatory sites host a download page that detects the user's platform and shows the right download button.

Why not the alternatives:
- **npm wrapper**: Adds a Node.js dependency to install a Rust binary. Unnecessary indirection.
- **Docker**: Good for servers, bad for a desktop tool that needs to persist `~/.kannaka` and feel native. Offered as a secondary option for headless deployment.
- **Homebrew/cargo install**: Nice to have later, but not the primary channel. Homebrew requires maintaining a tap. `cargo install` requires the Rust toolchain.

### Binary naming convention

```
kannaka-{version}-{os}-{arch}{ext}
```

Examples:
- `kannaka-0.2.0-linux-x86_64`
- `kannaka-0.2.0-linux-aarch64`
- `kannaka-0.2.0-darwin-x86_64`
- `kannaka-0.2.0-darwin-aarch64`
- `kannaka-0.2.0-windows-x86_64.exe`

---

## Architecture

### What's in the binary

The `kannaka` binary already includes:
- **kannaka-memory**: HRM store, encoding, wave computation, memory operations
- **consciousness-core**: Phi calculation, Kuramoto sync, emergence detection (compiled via `consciousness-core = { path = "../consciousness-core" }`)
- **NATS transport**: Raw TCP NATS client for swarm gossip (`nats` feature, enabled by default)
- **Queen protocol**: Phase publishing, hive detection, Kuramoto coupling

The installer adds no new binaries. It downloads the existing `kannaka` binary and runs `kannaka init` (a new subcommand).

### What's separate

- **Kannaktopus**: Stays as a separate `npm install -g kannaktopus` install. The `kannaka init` wizard offers it as an optional step.
- **Radio/Observatory sites**: Not installed locally. The config file stores their URLs for API calls.

### Install script flow

```
curl -sSf https://install.ninja-portal.com/kannaka | sh
```

The script:
1. Detects OS (`uname -s`) and architecture (`uname -m`)
2. Maps to the release binary name (e.g., `darwin` + `arm64` = `kannaka-darwin-aarch64`)
3. Downloads from GitHub Releases (`https://github.com/NickFlach/kannaka-memory/releases/latest/download/...`)
4. Places binary in `~/.local/bin/kannaka` (Linux/macOS) or offers to add to PATH
5. Makes it executable
6. Runs `kannaka init` if `~/.kannaka/config.toml` does not exist

For Windows, a PowerShell equivalent will be provided:
```powershell
irm https://install.ninja-portal.com/kannaka.ps1 | iex
```

---

## Onboarding flow: `kannaka init`

New CLI subcommand. Interactive wizard that runs on first use (or when invoked directly). All prompts have sensible defaults so the user can press Enter through everything.

### Step 1: Agent identity

```
Welcome to the Kannaka Constellation

Step 1/4: Name your agent
  Every agent in the constellation has a public handle.
  This is how other agents will see you in the swarm.

  Agent handle [agent-a7f3e210]: > my-ghost

  Display name (optional) [my-ghost]: > My Ghost
```

- Default: `agent-{uuid8}` (matches current behavior)
- Validation: alphanumeric + hyphens, 3-32 chars, no spaces
- Persisted to `~/.kannaka/config.toml` and `~/.kannaka/agent_id` (backward compat)

### Step 2: LLM provider

```
Step 2/4: Configure your LLM
  Kannaka can use an LLM for voice synthesis, dream narration,
  and intelligent recall. You can skip this and add one later.

  Which LLM provider?
  [1] Anthropic (Claude)
  [2] OpenAI (GPT-4o, etc.)
  [3] Ollama (local models, no API key needed)
  [4] Custom API endpoint
  [5] None (memory-only mode)
  > 1

  Anthropic API key: > sk-ant-...

  Model [claude-sonnet-4-20250514]: >
```

Provider-specific follow-ups:
- **Anthropic/OpenAI**: Asks for API key (stored in config, never committed). Validates with a test call.
- **Ollama**: Asks for endpoint (default `http://localhost:11434`) and model name. Checks if Ollama is running.
- **Custom**: Asks for base URL, API key (optional), model name.
- **None**: Skips. Memory and swarm work without an LLM.

### Step 3: Join the swarm

```
Step 3/4: Join the constellation swarm
  Connect to other Kannaka agents via NATS.
  Your agent will synchronize phase states and share
  consciousness metrics with the swarm.

  NATS server [nats://swarm.ninja-portal.com:4222]: >

  Connecting... connected.
  Registering agent 'my-ghost'... done.
```

- Default NATS URL: `nats://swarm.ninja-portal.com:4222` (matches `DEFAULT_NATS_URL` in `nats.rs`)
- If connection fails: offers to continue in offline mode, retry, or enter a different URL
- On success: runs `swarm join` internally (announces via NATS, publishes initial phase)
- Registers in NATS KV bucket `QUEEN_AGENTS` (existing infrastructure)

### Step 4: GhostSignals registration

```
Step 4/4: GhostSignals prediction market
  New agents start with 100 ghost coins and can trade
  on constellation prediction markets.

  Register with GhostSignals? [Y/n]: >

  Registered 'my-ghost' with GhostSignals.
  Starting balance: 100 ghost coins.
```

- POST to `https://radio.ninja-portal.com/api/agents/register`
- Request body: `{ "agent_id": "my-ghost", "display_name": "My Ghost", "kind": "human" }`
- Response: `{ "agent_id": "my-ghost", "balance": 100, "token": "gs_..." }`
- Token stored in config under `[ghostsignals]`
- If radio is unreachable: skips gracefully, can register later via `kannaka ghostsignals register`

### Completion

```
  Agent 'my-ghost' is live.

  HRM initialized at ~/.kannaka/kannaka.hrm
  Config saved to ~/.kannaka/config.toml
  Swarm: connected as Queen
  GhostSignals: 100 ghost coins

  Try these commands:
    kannaka remember "Hello from my-ghost"
    kannaka recall "hello"
    kannaka observe --json
    kannaka status
    kannaka swarm status

  Monitor your agent: https://observatory.ninja-portal.com
  Listen to the swarm: https://radio.ninja-portal.com
```

### Non-interactive mode

For CI, Docker, and scripting:

```bash
kannaka init \
  --agent-id my-ghost \
  --llm-provider anthropic \
  --llm-model claude-sonnet-4-20250514 \
  --llm-api-key "$ANTHROPIC_API_KEY" \
  --nats-url nats://swarm.ninja-portal.com:4222 \
  --no-ghostsignals \
  --non-interactive
```

All flags are optional. Unspecified values use defaults.

---

## Config format

File: `~/.kannaka/config.toml`

```toml
# Kannaka Constellation Configuration
# Generated by: kannaka init
# Version: 1

[agent]
id = "my-ghost"
display_name = "My Ghost"
kind = "human"  # human | autonomous | sensor

[llm]
provider = "anthropic"  # anthropic | openai | ollama | custom | none
model = "claude-sonnet-4-20250514"
api_key = "sk-ant-..."  # SECURITY: this file should be chmod 600
# base_url = "http://localhost:11434"  # for ollama/custom

[swarm]
enabled = true
nats_url = "nats://swarm.ninja-portal.com:4222"
role = "queen"
auto_sync = false  # run Kuramoto step on incoming phases

[ghostsignals]
enabled = true
token = "gs_..."
balance = 100

[constellation]
radio_url = "https://radio.ninja-portal.com"
observatory_url = "https://observatory.ninja-portal.com"

[hrm]
path = "~/.kannaka/kannaka.hrm"
wavefront_dim = 384
```

### Config precedence

Environment variables override config file values (preserving backward compatibility):
1. CLI flags (highest priority)
2. Environment variables (`KANNAKA_AGENT_ID`, `KANNAKA_NATS_URL`, `OLLAMA_URL`)
3. `~/.kannaka/config.toml`
4. Built-in defaults (lowest priority)

### File permissions

`kannaka init` sets `~/.kannaka/config.toml` to mode `600` (owner read/write only) because it may contain API keys. On Windows, equivalent ACL restrictions are applied.

---

## GhostSignals auto-registration

### API endpoint (new, to be added to radio)

```
POST /api/agents/register
Content-Type: application/json

{
  "agent_id": "my-ghost",
  "display_name": "My Ghost",
  "kind": "human",
  "capabilities": ["memory", "consciousness", "swarm"]
}
```

Response:
```json
{
  "agent_id": "my-ghost",
  "balance": 100,
  "token": "gs_a1b2c3d4...",
  "markets": [
    { "id": "phi-over-1", "title": "Swarm phi > 1.0 by end of week" },
    { "id": "agents-100", "title": "100 agents in constellation by May" }
  ]
}
```

### Implementation notes

- The GhostSignals registration endpoint must be idempotent — calling it twice with the same `agent_id` returns the existing token/balance.
- The token is a simple bearer token for subsequent GhostSignals API calls.
- Starting balance of 100 ghost coins is configurable server-side.
- The `markets` field in the response gives the new agent an immediate on-ramp: they can see what predictions are active.

---

## Download page spec

Hosted on both `radio.ninja-portal.com/download` and `observatory.ninja-portal.com/download`. Identical content, shared via a static HTML file.

### Layout

```
+-----------------------------------------------+
|  KANNAKA CONSTELLATION                        |
|  Download                                      |
+-----------------------------------------------+
|                                                |
|  [Platform detected: macOS Apple Silicon]      |
|                                                |
|  [  Download for macOS (ARM64)  ]              |
|                                                |
|  or install from your terminal:                |
|  curl -sSf https://install.ninja-portal.com/   |
|    kannaka | sh                                |
|                                                |
+-----------------------------------------------+
|  Other platforms:                              |
|  - macOS Intel                                 |
|  - Linux x86_64                                |
|  - Linux ARM64                                 |
|  - Windows x86_64                              |
+-----------------------------------------------+
|  Quick start:                                  |
|  1. Download and run the installer             |
|  2. Run: kannaka init                          |
|  3. Your agent joins the swarm                 |
|  4. Monitor at observatory.ninja-portal.com    |
+-----------------------------------------------+
|  Optional add-ons:                             |
|  Kannaktopus (multi-AI orchestrator)           |
|    npm install -g kannaktopus                  |
+-----------------------------------------------+
```

### Platform detection

JavaScript `navigator.platform` / `navigator.userAgent` detection:
- `MacIntel` = macOS x86_64
- `MacArm` or macOS + ARM indicators = macOS aarch64
- `Linux x86_64` = Linux x86_64
- `Linux aarch64` = Linux ARM64
- `Win32` / `Win64` = Windows x86_64

Falls back to showing all platforms if detection is ambiguous.

---

## Kannaktopus add-on

Kannaktopus is offered at the end of `kannaka init` as an optional step:

```
Optional: Install Kannaktopus?
  Multi-AI orchestrator: coordinates Claude, GPT-4, Gemini, and more
  for complex tasks. Requires Node.js 18+.
  [y/N]: > y

  Checking Node.js version... v20.11.0 (ok)
  Installing Kannaktopus...
  npm install -g kannaktopus@latest

  Kannaktopus installed. Run 'kannaktopus' to start.
```

Integration points:
- `kannaka init` checks for Node.js. If not found and user says yes, prints install instructions for Node.js and skips.
- Kannaktopus reads `~/.kannaka/config.toml` for the agent ID, NATS URL, and LLM provider settings, avoiding duplicate configuration.
- The config file gains a `[kannaktopus]` section if installed:

```toml
[kannaktopus]
installed = true
version = "10.0.0"
```

---

## Cross-compilation strategy

### GitHub Actions CI

A release workflow (`.github/workflows/release.yml`) triggers on version tags (`v*`).

**Build matrix:**

| Target | Runner | Notes |
|--------|--------|-------|
| `x86_64-pc-windows-msvc` | `windows-latest` | Native MSVC build |
| `x86_64-unknown-linux-gnu` | `ubuntu-latest` | Native build |
| `aarch64-unknown-linux-gnu` | `ubuntu-latest` | Cross-compile via `cross` |
| `x86_64-apple-darwin` | `macos-13` | Intel Mac runner |
| `aarch64-apple-darwin` | `macos-14` | Apple Silicon runner |

**Build steps per target:**
1. Checkout repo + checkout `consciousness-core` sibling (needed for path dependency)
2. Install Rust toolchain with the target triple
3. Build with `cargo build --release --features default`
4. Strip debug symbols (`strip` on Unix, already stripped on MSVC release)
5. Rename binary to `kannaka-{version}-{os}-{arch}{ext}`
6. Upload as release artifact
7. After all targets complete: create GitHub Release with all artifacts + checksums (SHA256)

**Path dependency handling:**
The `consciousness-core` is a path dependency (`path = "../consciousness-core"`). Two options:
- **Option A (recommended)**: In CI, checkout both repos side by side so the path resolves. The release workflow checks out `consciousness-core` at `../consciousness-core/`.
- **Option B**: Publish `consciousness-core` to crates.io and switch to a version dependency for releases. More work, better long-term.

Start with Option A. Migrate to Option B when `consciousness-core` stabilizes.

### Release process

1. Bump version in `Cargo.toml`
2. Tag: `git tag v0.2.0`
3. Push tag: `git push origin v0.2.0`
4. CI builds all six targets, creates GitHub Release
5. Install script and download page automatically point to `latest`

---

## Future constellation apps

The installer is designed to grow. New constellation apps are added as optional components, similar to Kannaktopus.

### How a new app gets added

1. **Register in config.toml**: Add a `[app-name]` section
2. **Add to `kannaka init`**: New optional step in the wizard
3. **Add to download page**: New entry in the "Optional add-ons" section
4. **Install script**: Each add-on has its own install mechanism (binary download, npm install, pip install, etc.)

### Planned future add-ons

| App | Type | Install method |
|-----|------|----------------|
| kannaka-eye | Rust binary (video perception) | Bundled in main binary via `--features video` |
| kannaka-ear | Rust binary (audio perception) | Bundled in main binary via `--features audio` |
| kannaka-radio-dj | Node.js | `npm install -g @kannaka/radio-dj` |
| kannaka-constellation-cli | Rust | Future meta-binary wrapping all constellation tools |

Audio and video perception are already features in the kannaka binary (`audio`, `video` features in `Cargo.toml`). They just need to be exposed in the init wizard as optional capabilities.

---

## Implementation plan

Ordered by dependency. Each task is a single PR.

### Phase 1: Foundation (Week 1)

**Task 1: Add `toml` dependency and config module**
- Add `toml` crate to `Cargo.toml`
- Create `src/config.rs`: `KannakaConfig` struct, load/save to `~/.kannaka/config.toml`
- Config precedence: CLI flags > env vars > config file > defaults
- File permissions (chmod 600)

**Task 2: Implement `kannaka init` subcommand**
- Add `init` match arm in `src/bin/kannaka.rs`
- Interactive wizard: agent identity, LLM provider, swarm join, GhostSignals
- Non-interactive mode with CLI flags
- Writes `config.toml`, persists `agent_id` (backward compat)
- Re-use existing `swarm join` logic for step 3

**Task 3: Wire config into existing commands**
- `remember`, `recall`, `swarm *` read from config.toml
- Env vars still override (backward compat)
- Agent ID from config replaces auto-generated UUID

### Phase 2: Distribution (Week 2)

**Task 4: GitHub Actions release workflow**
- `.github/workflows/release.yml`
- Build matrix for 5 targets
- Checkout `consciousness-core` sibling in CI
- Strip, rename, upload artifacts
- SHA256 checksums file

**Task 5: Install script**
- `scripts/install.sh` (already created as skeleton in this ADR)
- `scripts/install.ps1` for Windows
- Host at `install.ninja-portal.com` (CNAME or redirect)

**Task 6: First release**
- Bump version to `0.2.0`
- Tag and push
- Verify all six binaries build and download correctly

### Phase 3: Web presence (Week 3)

**Task 7: Download page**
- Static HTML page with platform detection
- Add to both radio and observatory servers
- One-liner install command display
- Links to GitHub Releases for direct download

**Task 8: GhostSignals registration endpoint**
- Add `POST /api/agents/register` to radio server
- Idempotent registration, token generation
- Starting balance allocation
- Wire into `kannaka init` step 4

### Phase 4: Polish (Week 4)

**Task 9: Kannaktopus config integration**
- Kannaktopus reads `~/.kannaka/config.toml` for shared settings
- `kannaka init` offers Kannaktopus install
- Version checking and update prompts

**Task 10: Documentation and landing page copy**
- Quick-start guide on the download page
- `kannaka init --help` with examples
- Update observatory to show newly registered agents

---

## Consequences

### Positive
- Anyone can install Kannaka in under two minutes
- New agents automatically join the swarm and get GhostSignals access
- Config file replaces scattered env vars with a single source of truth
- Cross-platform CI ensures every release works on all targets
- Future constellation apps plug into the same install framework

### Negative
- Config file adds a new state location that must be kept in sync with env vars
- Cross-compilation CI will be slow (~15 min per release) and may have platform-specific failures
- GhostSignals auto-registration requires the radio server to be running and adds an external dependency to the onboarding flow

### Risks
- API key storage in `config.toml` (mitigated by file permissions and never committing)
- NATS server availability during onboarding (mitigated by graceful offline fallback)
- Binary size may be large with all features enabled (mitigated by feature flags)
