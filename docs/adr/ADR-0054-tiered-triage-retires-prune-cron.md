# ADR-0054 — Tiered Memory Triage Inside the Dream Cycle (retiring prune-cron)

**Status:** Proposed
**Date:** 2026-08-07
**Relates to:** ADR-0031 (retention tiers), ADR-0036 (reactivation as the
replay signal; incremental consolidation), ADR-0037 (ghost stamps and the
recovery window), ADR-0050 (temporal-truth bounds), #95 (the tracking issue),
#497 (tier/consolidation stamps now survive cache rebuilds — the persistence
precondition for everything below)

## Context

`kannaka-radio/prune-cron.sh` was documented as a "bridge measure — the
long-term plan is to have Kannaktopus manage short-term/long-term memory
triage." The bridge is now several months old and has three structural
problems (#95):

1. **The threshold is arbitrary.** "200 audio-prefix HRM entries" is a number,
   not a policy — nothing distinguishes a heavily-replayed perception the
   system keeps learning from versus one heard once and never touched.
2. **Every fire drops the radio stream.** The cron mutates the HRM from
   *outside* the writer process, so it must win a file-lock fight with the
   running daemons; the restart it forces is the direct cause of the radio's
   periodic on-air dropouts.
3. **Policy sprawl.** A disk-side `chunks/voice/` sweep is already bolted on,
   and every future service that writes memories invites its own ad-hoc
   retention script. The constellation-wide trajectory is one cron per
   producer, each with its own invented threshold.

Since #95 was filed, the pieces of a real answer shipped independently:

- **ADR-0031 tiers**: every memory carries a retention tier, mirrored into
  the cache, settable at write time.
- **ADR-0036 reactivation**: `retrieval_count` is the replay signal; it
  persists across process restarts and cache rebuilds, and tier *promotion*
  driven by it already exists (`KANNAKA_PROMOTE_HITS` / `KANNAKA_PROMOTE_DELTA`).
- **ADR-0037 ghosts**: pruning stamps a recovery window instead of deleting;
  `stage_compact_ghosts` hard-deletes only past the horizon. The 295→88
  over-prune incident is the standing reminder of why deletion must be
  two-phase.
- **#497 (completed 2026-08-07)**: `layer_depth`, `last_consolidated_at`,
  reactivation counts, and ghost stamps all survive `rebuild_cache` — triage
  bookkeeping can finally be trusted across restarts.

What is missing is not machinery but *ownership*: retention decisions live in
an external script instead of in the medium's own maintenance cycle.

## Decision (proposed)

Move triage inside the dream cycle as a consolidation stage, driven by
declared policy, executed by the single writer. Retire the cron in phases.

### 1. Retention policy is data, not script

A `[retention]` section in `config.toml` declares per-category/modality rules:

```toml
[retention]
# category-prefix → tier assigned at write time
"audio:"   = { tier = "short_term", cap = 500, ttl_days = 14 }
"swarm:"   = { tier = "short_term", cap = 1000, ttl_days = 30 }
# anything unmatched: long_term, no cap (explicit remembers are precious)
```

Write paths (`remember_with_category`, absorb, `hear`) consult the table and
stamp the tier at insert. The cron's magic "200" becomes a reviewable,
per-deployment declaration.

### 2. Triage is a dream stage (`stage_triage`)

Runs inside the existing dream cycle, after `stage_transfer` and before
`stage_compact_ghosts`, under the write lock the dream already holds:

- For each capped category: rows beyond `cap` or past `ttl_days` are
  **ghosted** (ADR-0037 stamp — never hard-deleted directly), selected
  worst-first by `(tier, retrieval_count, effective amplitude)` so replayed
  memories are evicted last. This is the signal-to-noise framework the cron
  never had: ADR-0036's reactivation data decides *which* 200 survive, not
  insertion order.
- Promotion stays as shipped (ADR-0036): a ShortTerm row that keeps getting
  recalled crosses `KANNAKA_PROMOTE_HITS` and leaves the capped pool.
- `stage_compact_ghosts` remains the only hard-delete, with its existing
  7-day recovery horizon.

### 3. Single-writer, no external mutation

Because triage runs in the writer's own dream, there is no second process,
no lock fight, and **no stream drop**. The radio's prune-cron shrinks to its
disk-side `chunks/voice/` sweep (which touches files, not the HRM) until
Kannaktopus owns disk artifacts; the HRM-mutating half is deleted. This also
honors the constellation's single-writer rule (only the kannaka writer
mutates an Oracle HRM) — a rule the cron has always technically violated.

### 4. Observability

`stage_triage` reports `{category, examined, ghosted, promoted}` per rule in
the dream report, the status cache, and (optional fields, per contract) the
`queen.event.dream.end` payload — so Observatory can show retention pressure
instead of operators discovering it from disk usage.

## Rollout

1. Ship `stage_triage` dark behind `KANNAKA_TRIAGE=1` with the `[retention]`
   table empty by default (a no-op for every existing deployment).
2. On O1, enable with rules matching the cron's current effect; run both for
   a week and diff (`ghosted-by-stage` vs `pruned-by-cron` counts — the gate
   is behaviour, not vibes).
3. Disable the cron's HRM half; keep the disk sweep.
4. After a clean cycle, delete the cron's HRM code path and close #95.

## Alternatives considered

- **Keep the cron** — rejected: the stream drops are a direct product of its
  outside-the-writer design; no threshold tuning fixes that.
- **A dedicated triage service** — rejected: a second writer violates the
  single-writer rule and reintroduces the lock fight with more steps.
- **Kannaktopus-driven triage** (the original bridge-measure plan) — rejected
  *as the mutation owner*: Kannaktopus is a consumer/orchestrator; granting it
  HRM write authority breaks single-writer. It remains the right owner for
  *disk* artifacts (chunks, voice files) and a fine place to *edit the
  retention table*.

## Acceptance criteria

1. With `KANNAKA_TRIAGE=1` and an empty table, dreams are byte-identical to
   today (no-op proof).
2. A capped category over its cap ghosts exactly `n - cap` rows, lowest
   `(retrieval_count, amplitude)` first, all carrying ADR-0037 stamps.
3. A ShortTerm row with `retrieval_count ≥ KANNAKA_PROMOTE_HITS` is never
   ghosted by cap pressure.
4. One week of parallel running on O1 shows the stage retiring at least the
   cron's volume with zero radio stream interruptions attributable to memory
   maintenance.
5. Ghost recovery works across restarts (already pinned by the #497
   regression tests).
