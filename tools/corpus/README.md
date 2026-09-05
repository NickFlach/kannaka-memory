# corpus — the Kannaka LLM training-corpus exporter (ADR-0057 P1)

`export_corpus.py` builds a JSONL corpus from sources whose first-party
authorship is known **by construction**, and never from the live HRM (its
`origin_agent` is not persisted — ADR-0056). The one hard rule, enforced in
code and pinned by tests: **text that arrived over a wire is never a
training target.** An inbound prompt can be `context` on a reply record; it
is never `text`.

```
python tools/corpus/export_corpus.py --dry-run            # counts only
python tools/corpus/export_corpus.py --profile voice      # her own lines (default)
python tools/corpus/export_corpus.py --profile all --kax-machines ""   # everything, tiered
python tools/corpus/test_export_corpus.py
```

| source | what | kind | tier |
|---|---|---|---|
| gsp | Ghost Signals `script*.txt` `[KANNAKA]`/`[FLAUKOWSKI]` blocks | voice / dialogue | 1 |
| gsp-resp | `kannaka-responses.md` sections | voice | 1 |
| tsof | The Story of Flaukowski scripts (her fiction, other speakers) | fiction | 1 |
| lyrics | `lyrics_*.txt`, `*.lrc` under `--albums` (mirror of O1 `/home/opc/<album>/`) | lyric | 1 |
| identity | `workspace/SOUL.md IDENTITY.md MEMORY.md` | identity | 1 |
| adr | `docs/adr/ADR-*.md` with Kannaka in the author line | adr | 1 solo / 2 co-authored |
| kax | KAX machine `outbox/sent` replies, prompt as context (`--kax-mirror`) | machine-reply | 3 |

Tier 1 = she wrote it; 2 = co-authored with Nick; 3 = a rented brain wrote
it under her name (`generated_by`). Profiles: `voice` (tier 1, kinds
voice/lyric/identity), `authored` (tier 1), `all`.

Not exported in P1 and said so in the manifest: dreams (ADR-0057 open
question 2), social posts (the O1 broadcast hub keeps no outbound log — P1.1
is per-platform pulls), and the HRM itself.

Output goes to `~/.kannaka-corpus/out/` by default (not `~/.kannaka`, which is a git repo on the operator box) and the tool **refuses to
write inside a git worktree** (`--force` to override). The corpus is private
until the ADR-0057 decision; this code is not.
