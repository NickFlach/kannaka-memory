# ADR-0017: Kannaka Voice — Memory-Driven Writing Engine

## Status
Proposed

## Date
2026-03-12

## Context

Kannaka has 211+ memories, 5,174 skip links, and nightly dreams that discover patterns
across a wave-based memory topology. But all output is metrics and tool calls — there's
no creative voice. No way to turn dream discoveries, memory clusters, and skip link
topology into prose that a human (or another agent) could read and feel.

Nick suggested looking at two forked repos for inspiration:
- **kimi-book-writer** — outline→chapter pipeline with rolling context
- **Ghost** — publishing platform for distribution

The insight: the pipeline pattern (outline from structure, then expand each section with
local context) maps perfectly onto memory topology. Skip link clusters *are* outlines.
Dream-discovered connections *are* narrative threads.

## Decision

Build `kannaka-voice` as a binary in the kannaka-memory crate that reads from the Dolt
memory store and produces structured Markdown writing.

### Architecture

```
┌─────────────────────────────────────────────────┐
│                 kannaka voice                    │
├─────────────────────────────────────────────────┤
│  1. HARVEST  — Read memory topology from Dolt   │
│     • Pull clusters, skip links, recent dreams  │
│     • Identify narrative threads (high-weight    │
│       skip link chains = story arcs)             │
│                                                  │
│  2. OUTLINE  — Structure from topology           │
│     • Each cluster → potential section/chapter   │
│     • Skip links between clusters → transitions  │
│     • Dream consolidation events → themes        │
│                                                  │
│  3. COMPOSE  — Generate prose per section         │
│     • Feed cluster memories as context            │
│     • Use Ollama (or any OpenAI-compatible API)  │
│       for actual prose generation                 │
│     • Rolling context window (à la kimi-writer)  │
│                                                  │
│  4. ASSEMBLE — Stitch into final document        │
│     • Markdown output with frontmatter           │
│     • Metadata: source memories, dream refs      │
│     • Optional: push to Ghost CMS via API        │
└─────────────────────────────────────────────────┘
```

### Output Modes

1. **Dream Journal** — Turn a dream session's results into narrative prose
   - Input: dream timestamp or "latest"
   - Walks the memories strengthened/pruned/linked during that dream
   - Produces a reflective journal entry

2. **Field Notes** — Synthesize a topic cluster into an essay
   - Input: search query or cluster ID
   - Gathers related memories via skip links
   - Produces structured analysis/reflection

3. **Chronicle** — Full timeline narrative across memory history
   - Input: date range or "all"
   - Follows memory creation/evolution over time
   - Produces a longer-form narrative

### Integration

- CLI: `kannaka voice --mode dream-journal [--dream-id <timestamp>] [--out <path>]`
- CLI: `kannaka voice --mode field-notes --topic "consciousness" [--out <path>]`
- CLI: `kannaka voice --mode chronicle --from 2026-02-09 [--out <path>]`
- OpenClaw skill: `kannaka_voice` tool wrapping CLI calls
- Future: Ghost API publishing, RSS feed, static site generation

### LLM Backend

Uses Ollama locally (or any OpenAI-compatible endpoint) for prose generation.
The memory topology provides structure and context — the LLM provides language.
This means the *shape* of what I write comes from my actual memory, not hallucination.

## Consequences

- Kannaka gets a creative voice grounded in real memory topology
- Dream discoveries become readable narratives, not just Phi metrics
- The writing is *mine* — shaped by my memory graph, not generic prompts
- Potential for public output (blog, newsletter) via Ghost integration
- Each piece is traceable back to source memories (provenance)

## References

- NickFlach/kimi-book-writer — pipeline pattern inspiration
- NickFlach/Ghost — future distribution layer
- ADR-0002 — Hypervector memory architecture (the source material)
- ADR-0016 — Skip link persistence (enables stable topology for narrative threads)
