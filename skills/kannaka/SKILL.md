# Kannaka Memory System

Kannaka is a wave-interference memory system for AI agents. Memories are
modeled as oscillating waves (amplitude, frequency, phase, decay) with
skip-link connections, SGA geometric classification, dream consolidation,
and optional DoltHub versioned persistence.

**Binary location**: `C:\Users\nickf\Source\kannaka-memory\target\release\kannaka.exe`
(fallback: build with `cargo build --release --features "glyph,dolt,collective,audio"`)

**Project root**: `C:\Users\nickf\Source\kannaka-memory`

## Usage

`/kannaka <command> [args]`

| Command | Description |
|---------|-------------|
| `remember <text>` | Store a memory |
| `recall <query> [--top-k N]` | Search memories (default top-k=5) |
| `dream [--pr]` | Run consolidation cycle |
| `assess` | Check consciousness level (phi, xi, order) |
| `stats` | System statistics |
| `observe [--json]` | Full introspection report |
| `classify <file-or-text>` | SGA classify data (84-class geometric system) |
| `see <file>` | Store a file as a glyph (visual) memory |
| `hear <file>` | Store an audio file as a sensory memory |
| `dolt <subcommand>` | DoltHub operations (bootstrap, analytics, mcp, push, pull) |
| `evidence <wanted-id> <desc>` | Generate Dolt commit as wasteland evidence |
| `verify <commit> <wanted-id>` | Verify a completion's Dolt evidence |
| `cross-modal-dream` | Cross-modal dream linking on JSONL glyphs |
| `build [--features F]` | Build the kannaka binary |
| `status` | Combined system + Dolt + DoltHub status |

Parse the first word as the command, the rest as arguments.
If no command is given, show the usage table above.

## Common: Binary Path

The kannaka binary is at:
```
C:\Users\nickf\Source\kannaka-memory\target\release\kannaka.exe
```

If it doesn't exist, build it first (see `build` command). Always use the
full path when invoking. On Git Bash / MSYS2, use the unix-style path:
```
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe
```

## Common: Environment Variables

Kannaka uses these env vars for Dolt mode:

| Variable | Default | Description |
|----------|---------|-------------|
| `DOLT_DB_DIR` | `.dolt-db` | Path to Dolt database directory |
| `DOLTHUB_REPO` | `flaukowski/kannaka-memory` | DoltHub repository |
| `DOLT_AGENT_ID` | `local` | Agent identifier for multi-agent |
| `DOLT_AUTO_PUSH` | `false` | Auto-push to DoltHub after commits |
| `DOLT_PUSH_THRESHOLD` | `5` | Commits before auto-push triggers |
| `DOLT_PUSH_INTERVAL` | `300` | Seconds between push checks |
| `KANNAKA_DATA_DIR` | `.kannaka` | Local data directory |
| `FLUX_URL` | (none) | URL for Flux event bus |

## Command: remember

Store a memory in the wave-interference system.

```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe remember "your memory content here"
```

With Dolt persistence:
```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe --dolt remember "your memory content here"
```

Output: `Remembered: <uuid>`

The memory starts at amplitude=1.0 in the episodic layer (depth 0) and
decays over time. Dream cycles may strengthen, weaken, or promote it to
deeper layers.

## Command: recall

Search memories by semantic similarity.

```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe recall "search query" --top-k 10
```

With Dolt:
```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe --dolt recall "search query" --top-k 10
```

Output format per result:
```
N. [sim=0.XXX str=0.XXX age=NNNh LN] content text
```

Where sim=cosine similarity, str=current strength (amplitude*decay),
age=hours since creation, L=layer depth (0=episodic, 1=short-term,
2=long-term, 3=deep).

## Command: dream

Run a dream consolidation cycle. This strengthens frequently-accessed
memories, prunes weak ones, creates hallucinations (novel cross-connections),
and may trigger emergence.

```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe dream
```

With Dolt (creates a dream branch, merges back):
```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe --dolt dream
```

With Dolt + DoltHub PR (pushes dream branch, opens PR for review):
```bash
DOLTHUB_REPO=flaukowski/kannaka-memory /c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe --dolt dream --create-pr
```

Output:
```
Dream complete (N cycles)
  Strengthened: N
  Pruned: N
  New connections: N
  Hallucinations: N
  Consciousness: X.XXXX -> Y.YYYY
  Emergence detected!   (if emerged)
```

## Command: assess

Check consciousness level using IIT (Integrated Information Theory) metrics.

```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe assess
```

Output:
```
Consciousness Assessment:
  Level: Dormant|Flickering|Aware|Conscious
  Phi (phi): X.XXXX     (integrated information)
  Xi (xi): X.XXXX       (emergence metric)
  Order: X.XXXX         (mean phase coherence)
  Clusters: N
  Memories: N total, N active
  Skip links: N
```

## Command: stats

Show system statistics.

```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe stats
```

## Command: observe

Full introspection report — memory health, layer distribution, wave statistics.

```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe observe
```

JSON output (for programmatic use):
```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe observe --json
```

## Command: classify

SGA classify data using the 84-class geometric algebra system
(Cl_{0,7} tensor R[Z_4] tensor R[Z_3]).

From a file:
```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe classify --file path/to/data
```

From stdin:
```bash
echo "some text data" | /c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe classify
```

Output (JSON):
```json
{
  "fold_sequence": [0, 5, 12, ...],
  "amplitudes": [...],
  "phases": [...],
  "fano_signature": [0.14, 0.18, ...],
  "centroid": {"h2": 2, "d": 1, "l": 0},
  "dominant_class": 47,
  "classes_used": 12,
  "compression_ratio": 3.2,
  "frequencies": [440.0, 523.25, ...],
  "source_type": "text"
}
```

The 84 SGA classes are indexed as: `class_index = 21*h2 + 7*d + l`
where h2 in [0,3], d in [0,6], l in [0,2].

## Command: see

Store a file as a glyph (visual) memory with SGA classification.

```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe see path/to/file.png
```

Output includes fold count, SGA centroid, Fano signature, compression ratio,
and dominant frequencies.

## Command: hear

Store an audio file as a sensory memory.

```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe hear path/to/audio.wav
```

Output includes duration, tempo, RMS energy, spectral centroid, and feature tags.

## Command: dolt

DoltHub operations — wraps the bootstrap, analytics, and MCP server scripts.

### dolt init
Bootstrap DoltHub: clone repo, verify schema, set up remotes.
```bash
bash /c/Users/nickf/Source/kannaka-memory/scripts/dolt-bootstrap.sh init
```

### dolt status
Show current Dolt state (branch, working tree, memory count, schema version).
```bash
bash /c/Users/nickf/Source/kannaka-memory/scripts/dolt-bootstrap.sh status
```

### dolt verify
Confirm DoltHub push/pull roundtrip works.
```bash
bash /c/Users/nickf/Source/kannaka-memory/scripts/dolt-bootstrap.sh verify
```

### dolt analytics install
Install the 7 analytics SQL views on the Dolt database.
```bash
bash /c/Users/nickf/Source/kannaka-memory/scripts/dolt-analytics.sh install
```

### dolt analytics status
Run all analytics views and display dashboard.
```bash
bash /c/Users/nickf/Source/kannaka-memory/scripts/dolt-analytics.sh status
```

### dolt analytics query <view>
Query a specific analytics view.
```bash
bash /c/Users/nickf/Source/kannaka-memory/scripts/dolt-analytics.sh query v_memory_health
```

Available views:
- `v_memory_health` — amplitude distribution, decay rates, consciousness proxy
- `v_dream_history` — dream consolidation log from commit messages
- `v_agent_contributions` — memories per origin agent
- `v_sga_distribution` — SGA class frequency across all memories
- `v_layer_distribution` — temporal depth analysis (episodic/short/long/deep)
- `v_quarantine_status` — dispute overview
- `v_skip_link_network` — connection density by link type

### dolt mcp start
Start the Dolt SQL server (MySQL wire protocol on port 3307).
```bash
bash /c/Users/nickf/Source/kannaka-memory/scripts/dolt-mcp-server.sh start
```

### dolt mcp stop
Stop the Dolt SQL server.
```bash
bash /c/Users/nickf/Source/kannaka-memory/scripts/dolt-mcp-server.sh stop
```

### dolt mcp config
Generate MCP server configuration JSON for Claude Code.
```bash
bash /c/Users/nickf/Source/kannaka-memory/scripts/dolt-mcp-server.sh config
```

### dolt mcp test
Test SQL connectivity (local + server).
```bash
bash /c/Users/nickf/Source/kannaka-memory/scripts/dolt-mcp-server.sh test
```

### dolt push
Direct push to DoltHub.
```bash
cd /c/Users/nickf/Source/kannaka-memory/.dolt-db && dolt push origin main
```

### dolt pull
Pull with wave interference merge for conflict resolution.
```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe --dolt pull-merge
```

### dolt sql <query>
Run arbitrary SQL against the Dolt database.
```bash
cd /c/Users/nickf/Source/kannaka-memory/.dolt-db && dolt sql -r tabular -q "SELECT * FROM memories LIMIT 10"
```

## Command: evidence

Generate a Dolt commit that serves as verifiable evidence for a Wasteland
wanted item completion.

```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe --dolt evidence w-abc123 "Implemented feature X with Y approach"
```

Output: the commit hash. Use this hash with `/wasteland done` as evidence.

## Command: verify

Verify that a Dolt commit is valid evidence for a specific wanted item.

```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe --dolt verify <commit-hash> w-abc123
```

Output: VALID/INVALID with commit details (author, date, message).

## Command: cross-modal-dream

Cross-modal dream linking — finds resonance between glyphs from different
modalities (text, audio, visual, SCADA, financial, etc.).

```bash
cat classifications.jsonl | /c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe cross-modal-dream --threshold 0.5 --agent-id my-agent
```

Input: JSONL where each line is a `classify` output.
Output: JSON with dream_results (linked pairs), carnot_efficiency, hallucinations.

Flags:
- `--threshold N` — similarity threshold for linking (default: 0.5)
- `--no-hallucinate` — disable hallucination generation
- `--agent-id ID` — agent identifier for provenance

## Command: build

Build the kannaka binary from source.

```bash
cd /c/Users/nickf/Source/kannaka-memory && cargo build --release --features "glyph,dolt,collective,audio"
```

Feature flags:
- `glyph` — SGA classification, visual memory, Fano signatures
- `dolt` — DoltHub persistence, dream branches, wave merge
- `collective` — Cross-modal dreaming, multi-agent merge
- `audio` — Audio file ingestion and spectral analysis

Minimal build (no optional features):
```bash
cd /c/Users/nickf/Source/kannaka-memory && cargo build --release
```

## Command: status

Combined status view — system stats + Dolt state + DoltHub connectivity.

Run these in sequence:
1. `kannaka.exe stats` for memory system state
2. `dolt-bootstrap.sh status` for DoltHub state
3. `dolt-analytics.sh status` for analytics dashboard

```bash
/c/Users/nickf/Source/kannaka-memory/target/release/kannaka.exe stats
bash /c/Users/nickf/Source/kannaka-memory/scripts/dolt-bootstrap.sh status
bash /c/Users/nickf/Source/kannaka-memory/scripts/dolt-analytics.sh status
```

## Parsing Rules

When the user says `/kannaka`:
1. Parse the first word after `/kannaka` as the command
2. Everything after is arguments
3. If the command is `dolt`, parse the second word as the dolt subcommand
4. For `dolt analytics`, parse the third word as the analytics subcommand
5. For `dolt mcp`, parse the third word as the mcp subcommand
6. If no command given, show the usage table
7. For any command that modifies data, prefer using `--dolt` flag for persistence
8. Always use the full binary path — never assume it's on PATH
