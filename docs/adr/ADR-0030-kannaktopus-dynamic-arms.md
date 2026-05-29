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

### Healing, growth, and locomotion (`reanchor`)

The core operation is `reanchor`, applied to the current clusters:

1. **Heal** — for each arm, if its grip resolves to a fresh, not-yet-occupied
   cluster, keep it; otherwise (homeless because its cluster fell below
   min-size, pruned, or a duplicate of another arm's cluster) **re-grip** it onto
   the exemplar of the best free cluster. Arms are never silently lost.
2. **Fill** — grow up to `min(MAX_ARMS, num_clusters)` distinct clusters, most
   salient first. Arm count therefore tracks the HRM ("grow from 1 until 8" =
   bounded by how many clusters exist), and recovers immediately rather than one
   arm per dream.
3. **Crawl** — when `num_clusters > arms` (an HRM with >8 clusters), move the arm
   on the *smallest* covered cluster onto the biggest uncovered one (no
   downgrade), so arms tour without ever abandoning a dominant cluster.

`step` runs `reanchor` and **persists** (once per dream, in the single-writer
window). `observe` runs `reanchor` on a clone and shows the healed result
**without persisting** — so the observatory always reflects a full, live octopus
even between dreams, regardless of how stale the on-disk arm state is.

> Rationale (2026-05-29): the original "drop dead arms, regrow one per tick"
> design eroded on a churny HRM — Oracle reached 14 clusters but only 4 arms
> (one homeless), aggregate ~0. Heal-and-fill makes Kannaktopus self-repairing.

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
