# ADR-0030 — Kannaktopus Dynamic Arms (Resident Octopus Memory)

Status: **Proposed**
Date: 2026-05-28
Authors: Nick Flaukowski (vision), claude-flow (drafting)
Related: ADR-0020 (Holographic Resonance Medium), ADR-0021 (Chiral Mirror),
         ADR-0022 (Wave-Native Dreaming), ADR-0027 (Collective substrate)

---

## Context

Kannaktopus is the octopus that lives in the HRM — already visualised in the
observatory (`octopusTargetCluster`, `octopus_tangle`, tentacles) and already
present in the dream as **Stage 10**, the "executive oscillator that focuses
attention on weak topology" (`consolidation.rs::stage_kannaktopus`, returning
`kannaktopus_targets: Vec<usize>` = cluster indices: weakest / strongest /
isolated).

But Kannaktopus only *exists* for the duration of a dream. Between dreams it has
no standing state, so anything that asks "how much does Kannaktopus remember?"
gets **zero** — which is exactly what the observatory shows. It has a body and a
behaviour but no persistent identity or memory.

The operator's framing: make Kannaktopus's memory **dynamic within the system**
rather than a separate (empty) store. Kannaktopus lives in every HRM, has **up
to 8 arms**, one per cluster. On an HRM with more than 8 clusters it **crawls**,
moving one arm at a time across the clusters. It grows from 1 arm up to 8.
Kannaktopus's memory is the **aggregate of its arms** — if each occupied cluster
holds ~10 memories, an 8-arm Kannaktopus knows ~80 memories, and its
characteristics are those of exactly that set.

## Decision

Model Kannaktopus as a **persistent view over the host HRM**, not a store.

### Arms grip exemplar memories (Option A)

The hard problem is that clustering is **recomputed every dream** — cluster
indices and membership are not stable, so "arm on cluster 7" silently points at
a different set of memories after each consolidation. Therefore an arm does
**not** anchor to a cluster index. Each arm **grips one exemplar memory by
UUID**; the arm's cluster is *resolved at read time* as whichever cluster
currently contains that memory. A memory's identity is stable, so the grip
survives re-clustering. The chosen exemplar is the **highest-amplitude** (most
salient) member of a cluster.

### Memory = aggregate of occupied clusters

Kannaktopus's memory is the **union of the memories in the clusters its arms
occupy**. Its characteristics are computed over exactly that subset:

- **coherence** — Kuramoto order parameter over the aggregate
  (`KuramotoSync::order_parameter`),
- mean amplitude / mean frequency,
- modality distribution,
- per-occupied-cluster order parameters.

This gives Kannaktopus its own small consciousness signature, distinct from and
nested inside its host HRM's global metrics.

### Growth and locomotion (one movement per tick)

A "tick" is one `step`, run once per dream in the single-writer maintenance
window (ADR-aligned: arm state is mutated only there). Each tick performs at
most one movement, so Kannaktopus visibly grows and crawls over time:

1. **Re-anchor** — drop arms whose gripped memory was pruned/merged away.
2. **Grow** — while `arms < min(8, num_clusters)`, add one arm gripping the
   exemplar of the largest unoccupied cluster.
3. **Crawl** — once at target, if there are more clusters than arms, move the
   longest-tenured arm onto the biggest *uncovered* cluster, vacating its old
   one. Over many ticks the 8 arms tour an HRM of >8 clusters while always
   covering 8 distinct clusters.

### State and ownership

Persistent state is a tiny sidecar, `kannaktopus-arms.json`, next to the `.hrm`
(per-pid tmp + atomic rename, matching the HRM writer discipline). It records
`{agent_id, arms:[{id, grip, cluster, since}], max_arms, updated}`. Only the
dream-window `step` writes it; `observe` is read-only.

### Surface

- `kannaka kannaktopus observe [--json] [--members]` — read-only; arms +
  aggregate count + scoped characteristics (+ capped memory list with
  `--members`). Safe for the observatory and for `KANNAKA_READONLY` processes.
- `kannaka kannaktopus step [--json]` — advance one tick, persist arm state.
- The dream cron calls `step` after consolidating (fresh clusters, sole writer).
- The observatory reads `observe --json` and renders one tentacle per arm to its
  cluster, replacing the stuck-at-zero memory count with the live aggregate.

## Consequences

**Positive**
- The observatory's Kannaktopus memory is non-zero and *alive* the moment it has
  ≥1 arm on a populated cluster.
- No new store, no duplicate persistence, no extra HRM write contention (arm
  state is a separate small sidecar).
- Identity is stable across re-clustering; the dream's existing salience
  signals (weak/strong/isolated) can later drive smarter arm placement.
- "Lives in every HRM": each node instantiates its own Kannaktopus over its own
  clusters (prime, witness, substrate, local), each with its own signature.

**Negative / open**
- Coherence is a coherence measure, not full integrated information (Φ); it is
  labelled as Kannaktopus coherence, not host Φ.
- Cluster adjacency for crawl currently uses "biggest uncovered cluster" rather
  than true link-adjacency; a future revision can crawl along inter-cluster
  links for smoother locomotion.
- Exemplar = max amplitude is a heuristic; theme-vector centroid nearest is an
  alternative if salience proves noisy.
- Growth is one arm per dream, so a cold start reaches 8 arms over up to 8
  dreams. Acceptable (and faithful to "grows from 1 until 8"); a `--seed` could
  fast-fill if desired.
