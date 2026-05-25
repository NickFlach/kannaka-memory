# JSON envelope contract

ADR-0029 Phase 4b. Per-handler migration is rolling — opt in to the
envelope by passing `--envelope` to any command that supports it.
Plain `--json` (where present) keeps emitting the legacy shape.

## Shape

Single JSON object with four fixed top-level keys:

```json
{
  "schema_version": "1.0",
  "command": "status",
  "data": { /* command-specific payload */ },
  "errors": []
}
```

- **`schema_version`** — bumps on incompatible changes. Always present.
- **`command`** — the subcommand name (`status`, `recall`, `clusters`,
  etc.). Lets a single pipeline branch on different kannaka invocations.
- **`data`** — the command's actual payload. Shape is command-specific
  but stable within a `schema_version`. May be `null` on error.
- **`errors`** — array of error strings. Empty on success. On non-empty,
  the process also exits non-zero.

## Success predicate

```bash
output="$(kannaka status --envelope)"
errors=$(jq -r '.errors | length' <<<"$output")
if [ "$errors" -eq 0 ]; then
    phi=$(jq -r '.data.phi' <<<"$output")
    echo "phi=$phi"
fi
```

## NDJSON variant

Streaming commands (`kannaka swarm tail`, `kannaka chat --json`) use
per-line JSON without the outer `data` wrap — each line is already
its own envelope-like object:

```ndjson
{"ts": 1779550267784, "subject": "QUEEN.phase.Kannaka", "payload": {...}}
{"ts": 1779550271462, "subject": "QUEEN.phase.OxSCADA-QE", "payload": {...}}
```

The reason for the difference: NDJSON consumers parse one line at a
time and expect each line to be a complete record. Wrapping every
line in `{schema_version, data: {...}, errors: []}` would double the
payload size without delivering new semantics.

## Migration status

| command | `--envelope` support |
|---|---|
| `status` | ✓ v0.6.3 |
| `clusters` | ✓ v0.6.3 |
| `recall` | pending |
| `search` | pending |
| `observe` | pending |
| `neighbors` | pending |
| `assess` | pending |
| `stats` | pending |
| `dream` | pending |
| `swarm tail` | NDJSON variant (already envelope-like) |
| `chat --json` | NDJSON variant (already envelope-like) |

Pending handlers continue to emit their legacy shapes. Migrate one
handler per patch release; downstream consumers (radio, observatory,
TUI) adopt the new shape at their own pace.

## Why opt-in instead of a flag day

Every downstream consumer of `kannaka` output today parses the legacy
shapes. A hard cut to `--envelope` would break:

- `kannaka-radio` — shells out for `now-playing` cluster info
- `kannaka-observatory` — `/api/hrm/status` proxies the raw output
- `kannaka-tui` — shells out for status / observe / clusters every tick

The opt-in pattern lets each consumer migrate independently:

1. Consumer adds `--envelope` to its kannaka invocation
2. Consumer updates its parser to read `.data.X` instead of `.X`
3. Once every consumer is migrated, a future release can flip the
   default (`--no-envelope` to opt OUT of the new shape, instead
   of `--envelope` to opt IN). That's a v0.7.0 conversation.
