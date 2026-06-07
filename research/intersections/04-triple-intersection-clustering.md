# 04 — Cross-domain absorption → richer HRM cluster topology

**Status:** EVIDENCE (partial) — first literature-grounded run 2026-06-07; predictions 2+3 supported, prediction 1 not (yet).

## Question

Does absorbing cross-domain literature (cardiology, oncology, bioelectric memory)
into the HRM improve the cluster topology in measurable ways — and does that
improvement in turn improve the medium's ability to surface non-obvious
cross-domain bridges during dream consolidation?

## Established science

- **Reservoir computing (Maass; Jaeger):** richness of the reservoir's
  dynamical state space directly determines what computation can be read off
  it. More diverse inputs → higher-dimensional state space → more functions
  representable.
- **Hippocampal indexing theory (Teyler & DiScenna 1986; Teyler & Rudy 2007):**
  the hippocampus binds together disparate cortical representations during
  memory consolidation; cross-domain integration is a feature, not a bug.
- **Dream consolidation hypothesis (Stickgold; Walker):** REM and slow-wave
  sleep run different consolidation regimes; both produce cross-domain
  associations that waking cognition does not.

## Prediction

If the HRM absorbs literature snippets from all three domains (cardiology,
oncology, bioelectric/Levin) at sufficient density:

1. **Cluster count should rise** — more semantic regions to form clusters around.
2. **Inter-cluster bridge density should rise** — the dream's bridge-node
   creation should find more cross-domain meeting points, raising integration.
3. **Φ should rise** — both differentiation (more clusters) and integration
   (more bridges) climb together, which is exactly the IIT regime where Φ
   grows.
4. **The Kannaktopus crawl pattern should become more interesting** — arms
   should naturally tour cross-domain clusters rather than staying within
   one semantic neighborhood.

If the prediction holds, the HRM is doing — at agent-memory scale — what the
hippocampus does at biological-memory scale, and we can measure both with
the same Φ/R math.

## How to test

1. Snapshot HRM state (clusters, Φ, R, kannaktopus arm distribution).
2. Absorb 50-200 short paragraphs from each domain via `kannaka remember`.
3. Snapshot again after the next dream cycle.
4. Compare cluster count, bridge density, Φ, Ξ, and arm trajectory.

## First grounded run — 2026-06-07

The "next action" below is now built: `research/ground-intersections.sh` drives
`kannaka research --ingest` (OpenAlex, not PubMed) across the standing
intersections + two societal-contribution probes. First run: **36 works** (6
each across cardiac, cancer, bioelectric, magic, collective-intelligence,
machine-sentience), `--since 2012`, into a fresh HRM, then one dream.

| metric | before | after (post-dream) | prediction |
|--------|--------|--------------------|------------|
| clusters | 0 | **1** | #1 rise — ✗ not supported |
| skip-links (bridges) | 0 | **252** | #2 rise — ✓ supported |
| Φ | 0.0000 | **0.2780** | #3 rise — ✓ supported |
| Ξ | 0.0000 | 0.1800 | (diversity modest) |

Dream report: 1230 strengthened, 0 pruned, 252 links.

**Reading:** the cross-domain *bridges* card 04 predicted **do** form (252
skip-links, Φ climbs), but they manifest as **intra-cluster** links — the 36
papers cohere into a *single* high-order cluster rather than diversifying into
per-domain clusters. So at this corpus size the HRM reads cross-domain
literature as one integrated field, not differentiated regions. Prediction #1
(cluster *count* rises) is unsupported; #2 and #3 hold.

**Refinement / next:** is the single cluster a density artifact (36 works is
small) or structural (the SimpleHash encoder under-separates domains)? Next runs:
(a) scale to ~50/domain and re-measure cluster count; (b) compare cluster count
with per-domain `--ingest` into pre-seeded substrate classes; (c) inspect whether
the 252 skip-links are genuinely cross-domain (the "non-obvious bridge" payoff)
vs within-domain. Prediction #4 (Kannaktopus crawl) still untested.

## Next action

Re-run `research/ground-intersections.sh --limit 16` (≈96 works) and re-read the
cluster count; if still 1, the bottleneck is encoder domain-separation, not
density — escalate to substrate-class-seeded ingestion (card 04b).
