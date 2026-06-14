# Swarm Sensemaking — Wave Execution Plan

**Date:** 2026-06-14
**ADR:** [ADR-0035](../adr/ADR-0035-swarm-sensemaking-architecture.md)
**Repos:** `kannaka-memory` (the sensemaking engine itself — pure logic in
`src/sensemaking.rs`, exposed via the `kannaka swarm` CLI + NATS; plus the L6
autoresearch) · `kannaka-observatory` (visualization) · `kannaka-steward` is the
**separate** Intent & Reliability Kernel and is *not* the sensemaking home.

> **Home correction (2026-06-14):** ADR-0035 specifies these as `kannaka swarm
> <subcommand>` (e.g. `kannaka swarm brief`), so the sensemaking engine lives in
> kannaka-memory's swarm layer, not in kannaka-steward. The steward (intent /
> faithfulness for *actions*) and sensemaking (collective *knowledge*) are
> orthogonal concerns that happened to be requested together.

## Strategy

Build the sensemaking layer **alongside** the existing swarm transport (NATS +
QueenSync), not by replacing it — the same incremental, backward-compatible
approach used for the chiral mirror (ADR-0021). The swarm keeps working as a
messaging fabric throughout; the new `kannaka-memory/src/sensemaking.rs` module
holds pure, transport-agnostic cognition (unit-tested, no I/O) and the swarm CLI +
NATS layer feeds it.

**Hard ordering constraint:** the swarm's collective capabilities are built on
single-agent primitives that must work first. The headline example — *Collective
Recall Voting* (Cap 2) is meaningless if single-agent associative recall is
broken. As of 2026-06-14 (`8768a2e`), associative recall is fixed in
`kannaka-memory` via the `DREAM_GRAVITY` knob (`query_gravity` 0.37 → 1.0); see
[L6 OODA](#l6-ooda--collective-sensemaking-as-autoresearch-territory). That fix
is the foundation Wave 1 stands on.

The ADR's four delivery phases map directly to four waves.

---

## Wave 1: Sensemaking Primitives (ADR-0035 Phase 1)

**Theme:** Give the swarm a first voice — ask it what it knows, let it disagree,
let it vote. All read-only over the existing NATS recall/observe primitives.
**Repo:** `kannaka-memory` — pure cognition in `src/sensemaking.rs`, exposed via
the `kannaka swarm` CLI + NATS fan-out.

Deliverables: `swarm brief` · collective recall voting · peer expertise scoring ·
contradiction detection.

> **Status (2026-06-14):** the pure, transport-agnostic CORE of all four tasks is
> landed and unit-tested in `src/sensemaking.rs` (4 tests passing):
> `score_peer_expertise` (1.1), `merge_recall_votes` (1.2), `detect_contradictions`
> (1.3), `compose_brief` (1.4). What remains per task is the **NATS fan-out + CLI
> wiring** — feeding these functions with live peer responses.

### Task 1.1 — Peer expertise scoring (Discovery/resonance)
**Core: DONE** — `sensemaking::score_peer_expertise(&ExpertiseSignals)`.
Weighted [0,1] score over similarity (0.35), coverage (0.25), historical success
(0.20), confidence (0.15), Φ (0.05) — topical fit dominates so a high-Φ off-topic
peer can't outrank an on-topic one (tested). **Remaining:** populate
`ExpertiseSignals` from peer status snapshots on the swarm bus; `kannaka swarm
peers --topic X`.

### Task 1.2 — Collective recall voting
**Core: DONE** — `sensemaking::merge_recall_votes(&[PeerRecall], n_peers, agree)`.
Groups peer responses by an `agree` predicate, scores consensus confidence as
breadth × mean similarity × mean peer-confidence, sorts desc; lone outliers kept
but low-confidence (tested: 3-of-4 consensus beats the 1 outlier). **Remaining:**
fan recall out to top-K peers over `KANNAKA.recall.<agent>` (daemon-served recall
exists, see oracle-hrm-single-writer); supply a real `agree` (embedding cosine).
**Depends on:** single-agent associative recall (DONE, `DREAM_GRAVITY`).
**L6 hook:** consensus-vs-best-single becomes the `recall_vote_gain` fitness term.

### Task 1.3 — Contradiction detection
**Core: DONE** — `sensemaking::detect_contradictions(items, sim, sim_thr, gap)`.
Flags pairs that are content-similar but phase-opposed (|Δφ| near π) — the
wave-native "same subject, opposite stance"; agreeing duplicates excluded
(tested). **Remaining:** wire a real `sim` (content embedding); `kannaka swarm
contradictions --topic X`.

### Task 1.4 — `swarm brief`
**Core: DONE** — `sensemaking::compose_brief(topic, known, contradictions, peers)`
→ `SwarmBrief { known, contradictions, relevant_peers, confidence }`; confidence =
mean of top-k consensus, penalized per unresolved contradiction (tested).
**Remaining:** the `kannaka swarm brief "<topic>"` CLI subcommand in
`src/bin/handlers/swarm.rs` that runs 1.1–1.3 over live peers and prints/JSON-emits
the brief; add unknowns (gap signal, Wave 3) and recent-changes fields.

**Wave 1 dependency order:** 1.1 → 1.2 → 1.3 → 1.4 (1.2 and 1.3 both consume 1.1;
1.4 composes all three). The pure cores are done in that order; the CLI/NATS layer
follows the same order.

---

## Wave 2: Cognitive Hygiene & Collaboration (ADR-0035 Phase 2)

**Theme:** Keep the shared mind healthy and let agents build artifacts together.
**Repo:** `kannaka-memory` (Governance state) — new `src/immune.rs` and
`src/blackboard.rs` modules + `kannaka swarm` subcommands. Builds directly on
Wave 1's `sensemaking` module (the immune system reuses `detect_contradictions`).
Deliverables: memory immune system · swarm blackboard · apprenticeship workflows.

### Task 2.1 — Memory health classifier (Cap 4, detection half)
**Files:** `src/immune.rs` (`classify_memory_health`, pure).
Score each memory against five health flags from signals the HRM already has:
**duplicate** (cosine ≥ dedup_threshold to a higher-amplitude sibling),
**contradictory** (reuse Wave 1 `detect_contradictions`), **stale** (age past a
decay horizon with low recent access — needs Cap 8 temporal fields, Wave 3, so
start with last-access age), **low-confidence** (amplitude below a floor),
**hallucinated** (flagged at dream time / no corpus support). Output a per-memory
`HealthVerdict { flags, severity, recommended_action }`.
**Depends on:** Wave 1 `sensemaking`. **Tests:** an injected duplicate/contradiction
is flagged; a healthy high-amplitude memory is `Clean`; thresholds are boundary-tested.
**Success:** `kannaka swarm health` prints a ranked at-risk list (dry-run, no mutation).

### Task 2.2 — Immune actions / memory lifecycle (Cap 4, action half)
**Files:** `src/immune.rs` (`apply_action`), HRM store hooks.
Map verdicts to reversible lifecycle actions: **quarantine** (move to a held set,
excluded from recall but not deleted), **down-rank** (amplitude penalty),
**mark-for-review** (tag, surface to operator), **expire** (ghost at zero amplitude
— the existing soft-delete). Default to the *least* destructive action for a given
severity; never hard-delete. **Depends on:** 2.1. **Tests:** quarantine round-trips
(restore returns the memory); down-rank is bounded; expire uses the existing ghost
path. **Success:** `kannaka swarm immune --apply` quarantines/expires with an audit
line per action; `--dry-run` is the default.

### Task 2.3 — Swarm blackboard (Cap 9)
**Files:** `src/blackboard.rs` (artifact model + merge), NATS subject
`KANNAKA.blackboard.<id>`.
A shared reasoning artifact with typed entries: facts, assumptions, risks,
open-questions, decisions, evidence — each with an author agent and timestamp.
CRDT-style append + last-writer-wins on entry edits so multiple agents contribute
without a lock (reuse the `crdt`/`collective` patterns already in the repo).
**Depends on:** existing NATS transport. **Tests:** concurrent appends from two
agents converge; entry typing is enforced. **Success:** `kannaka swarm blackboard
<id> add --kind risk "<text>"` and `... show` render the shared artifact.

### Task 2.4 — Apprenticeship workflow (Cap 7)
**Files:** `src/apprentice.rs` (state machine), peer status field `state`.
An `AgentState { Apprentice, Journeyman, Master }` machine. In APPRENTICE: absorb
exemplars (existing exemplar exchange), shadow directed tasks (compute a would-be
answer without acting), and score it against the master's answer; track a rolling
competence. Promote on `competence ≥ threshold over N tasks`. Pure state-transition
logic + a thin NATS reporting layer. **Depends on:** Wave 1 expertise scoring
(competence reuses the same signal shape). **Tests:** promotion fires only after N
sustained passes; a regression demotes; transitions are monotonic per evaluation.
**Success:** a fresh agent joins as Apprentice and auto-promotes after meeting the
bar on a shadow-task set.

**Wave 2 dependency order:** 2.1 → 2.2 (actions need verdicts); 2.3 and 2.4 are
independent of the immune pair and of each other → parallelizable.
*ADR-0035 Cap 1/6/8 describe Wave 3 direction.*

---

## Wave 3: Emergent Cognition (ADR-0035 Phase 3)

**Theme:** The swarm starts generating understanding, not just managing it — it
dreams across agents, reasons about *when* knowledge was true, and notices what it
collectively doesn't know.
**Repo:** `kannaka-memory` — pure cores in new `src/gap.rs` and `src/temporal.rs`,
cross-agent dream wiring in `src/consolidation.rs` + `src/bin/handlers/swarm.rs`,
all exposed via `kannaka swarm` subcommands over the existing exemplar/presence
NATS layer.

Deliverables: **cross-agent dreaming** (Cap 6) · **gap detection engine** (Cap 1)
· **temporal truth reasoning** (Cap 8).

> **Grounding (2026-06-14):** the hooks already exist. `consolidate_swarm_aware`
> already takes peer phases; exemplar broadcasts (`swarm exemplars`/`absorb`) carry
> per-cluster `{theme, semantic_summary, coherence, xi_diversity, mean_amplitude,
> size}`; `swarm peers` exposes `memory_count` per peer; `observe().clusters` gives
> local cluster summaries. Temporal truth is the only capability needing a new
> persisted field set on `HyperMemory` (today only `created_at` + `decay_rate` →
> `effective_strength`). Wave 3 adds pure cores + thin wiring on top.

### Task 3.1 — Cross-agent dreaming (Cap 6)
**Files:** `src/consolidation.rs` (new `dream_cross_agent`, beside
`consolidate_swarm_aware`); `src/sensemaking.rs` (pure `reinforce_hypotheses`);
`src/bin/handlers/swarm.rs` (`swarm dream`).
Seed a dream with absorbed peer cluster exemplars (reuse the `swarm absorb`
ingestion path). After the local dream produces candidate hypotheses (existing
`hallucinated` + `parents` lineage), score each against peer cluster summaries: a
hypothesis is **reinforced** if it resonates (content sim + phase coherence) with
≥ K peers, **speculative** if only local. Reinforced keep full amplitude;
speculative are down-ranked (reuse Wave 2.2 down-rank), not deleted. Keep scoring
pure in `sensemaking::reinforce_hypotheses`.
**Depends on:** Wave 1 `sensemaking`, Wave 2.2 immune down-rank, exemplar exchange,
`DREAM_GRAVITY`. **Tests:** a hypothesis matching 3-of-4 injected peer clusters is
Reinforced; a purely-local hallucination is Speculative + down-ranked; empty peer
set = today's local-only dream (no regression). **Success:** `kannaka swarm dream`
reports `{reinforced, speculative, down_ranked}`.

### Task 3.2 — Temporal truth fields + confidence decay (Cap 8)
**Files:** `src/memory.rs` (new `#[serde(default)]` fields on `HyperMemory`);
`src/temporal.rs` (pure: `effective_confidence`, `is_current`, `temporal_status`).
Add `effective_at` / `observed_at` / `expires_at: Option<DateTime<Utc>>`, all
serde-defaulted for backward compat with existing `.hrm` snapshots. In
`temporal.rs`, compute a temporal confidence layered on the existing
`effective_strength` decay: `temporal_status(mem, now) -> {Current, Future,
Expired, Superseded}` and `effective_confidence(mem, now)`. No I/O.
**Depends on:** none (extends `HyperMemory`); consumed by 3.1 and 3.3.
**Tests:** a memory with past `expires_at` is Expired (~zero confidence); a legacy
memory with no temporal fields behaves exactly as today; future `effective_at`
yields Future; boundary at `now == expires_at`. **Success:** `kannaka swarm brief`
labels each fact's temporal status and excludes Expired/Future from confidence.
> **Status (2026-06-14):** the pure reasoning core shipped as `src/temporal.rs`
> (5 tests) operating on a `TemporalSpec` (`from_memory` derives observed=created_at,
> bounds unknown → everything reads Current = no behavior change). **Task 3.2b**
> (remaining) persists `effective_at`/`observed_at`/`expires_at` so brief/gap
> filtering activates. **DEFERRED as its own careful pass — it is a binary-format
> migration, not just field additions.** Findings: (1) `HyperMemory` adds the 3
> fields + updates ~20 struct-literal sites (compiler-guided); (2) crucially, the
> `.hrm` format serializes `WavefrontMeta` (`src/medium/types.rs`) with **bincode**
> (`src/medium/persistence.rs:69`), a *positional* format where `#[serde(default)]`
> does NOT make added fields backward-compatible — adding temporal to `WavefrontMeta`
> breaks loading every existing `.hrm` unless a **versioned-fallback struct** is added
> to the deserialize chain (the codebase already does this: `new → pre-tier → legacy`
> in `chiral_persistence.rs`); (3) `rebuild_cache` (`hrm_store.rs`) must copy
> `meta.temporal → memory` and the store path must populate `meta` from the memory.
> Execute with a `.hrm` backup first + a save→reload round-trip test before shipping.

### Task 3.3 — Gap detection engine (Cap 1)
**Files:** `src/gap.rs` (pure `build_coverage_map`, `detect_gaps`);
`src/bin/handlers/swarm.rs` (`swarm gaps`).
Consume (a) local cluster summaries from `observe().clusters`, (b) absorbed peer
cluster exemplars, (c) per-peer `memory_count`. `build_coverage_map` bins clusters
into domains (by theme/summary sim, reusing the `sensemaking` content-sim hook),
coverage = breadth (agents holding it) × depth (size/amplitude) × confidence
(coherence, discounted by `temporal::effective_confidence`). `detect_gaps` flags
**weakly-represented**, **blind-spot** (zero agents, adjacent to populated domains
in xi/phase), or **low-confidence** (incoherent/contradicted — reuse Wave 1
`detect_contradictions`) domains. Optionally cross-reference the kannaka-prime
96-class space as the domain prior. Output ranked `CoverageGap`.
**Depends on:** Wave 1 `sensemaking`, Task 3.2 (`effective_confidence`), exemplar
exchange + `swarm peers`. **Tests:** a held-out domain removed from all peers is
flagged BlindSpot; a domain held by 1-of-5 ranks above 5-of-5; a coherent
well-covered domain isn't flagged; injected contradictions lower domain
confidence. **Success:** `kannaka swarm gaps [--json]` ranks weakly-represented
domains; `gap_detection_precision` is computable on a partitioned-corpus swarm.

**Wave 3 dependency order:** 3.2 → 3.1, 3.3 (temporal fields underpin both
reinforced-dream truth-filtering and stale-domain coverage); 3.1 and 3.3 are
otherwise independent and parallelizable once 3.2's fields land.

**L6 OODA bridge:** Task 3.3's coverage map is the collective successor to the
single-agent curiosity loop — it *is* the swarm "developing curiosity." Its
`gap_detection_precision` is a candidate L6 fitness metric, and the ranked
`CoverageGap` list is the direct input to Wave 4's autonomous research planning.

*ADR-0035 Phase 4 (autonomous research planning, emergent hive formation,
self-directed sensemaking loops) is Wave 4.*

---

## Wave 4: Self-Directed Sensemaking (ADR-0035 Phase 4)

**Theme:** Close the loop — the swarm plans its own research, clusters into hives,
and runs the OODA continuously, generalizing the single-agent curiosity loop to
the collective.
**Repo:** `kannaka-memory` — pure cores in new `src/research_planner.rs`,
`src/hive_formation.rs`, `src/swarm_loop.rs` (on top of existing `src/queen.rs`
hive detection), wired via `kannaka swarm` subcommands over the presence/exemplar/
work-queue NATS layer, plus an `L6` arm in `src/bin/research.rs`.

Deliverables: **autonomous research planning** · **emergent hive formation** ·
**self-directed sensemaking loops**.

> **Grounding (2026-06-14):** richer substrate than the ADR assumed. Hive
> *detection* already ships — `queen::detect_hives`/`detect_hives_domain_aware`
> BFS-cluster peers by phase and infer role + bridge agents (`kannaka swarm hives`).
> The five Swarm States are purely conceptual (no `SwarmState` enum). The
> single-agent curiosity loop already does gap→research→PR (`research-suggest` →
> `research --ingest` → scope-guarded auto-merged PR). Wave 4 *generalizes* each
> from one agent to the collective.

### Task 4.1 — Autonomous research planner (gap map → directed tasks)
**Files:** `src/research_planner.rs` (pure `plan_research`, `dedupe_against_peers`,
`assign_to_peers`); `src/bin/handlers/swarm.rs` (`swarm plan`); hook in
`research/research-refresh.sh`.
Turn each Wave 3.3 `CoverageGap` into a `ResearchTask { domain, theme_query, kind,
priority, assignee, rationale }`, ordered by severity × kind (BlindSpot >
WeaklyRepresented > LowConfidence). `assign_to_peers` routes via Wave 1
`sensemaking::score_peer_expertise` over `swarm peers` presence; `dedupe_against_peers`
drops gaps peers already cover. Thin wrapper enqueues onto `KANNAKA.work.research`
(reuse `swarm enqueue`/`worker`), where a worker runs `research --ingest` + opens its
scope-guarded curiosity PR as today.
**Depends on:** Wave 3.3 `gap::CoverageGap`, Wave 1 `score_peer_expertise`, `swarm
peers`, Wave 3.2 `temporal::effective_confidence` (re-research stale domains); L6.
**Tests:** BlindSpot outranks WeaklyRepresented of equal coverage; task assigned to
highest-expertise peer not already holding it; fully-covered gap yields no task;
stale (low effective_confidence) domain is re-planned not skipped.
**Success:** `kannaka swarm plan [--assign] [--json]` prints the directed plan;
`--assign` enqueues. Collective ingests literature into exactly its blind spots
(measurable as `gap_detection_precision` rising across cycles).

### Task 4.2 — Emergent hive formation (resonance-clustered sub-swarms)
**Files:** `src/hive_formation.rs` (pure `form_hives`, `score_hive_membership`,
`hive_research_focus`); `src/bin/handlers/swarm.rs` (`swarm hive` — extend read-only
`swarm hives` into form/route).
Make `queen::detect_hives_domain_aware` (phase clustering) **resonance-aware and
purposive**: `score_hive_membership` blends phase coherence with memory-domain
resonance (reuse `sensemaking` content-sim over `swarm exemplars` `{theme,
semantic_summary,coherence}`), so peers join a hive because they *know the same
things*. `hive_research_focus` derives a hive's collective gap (Task 4.1 scoped to
one hive). Thin layer writes hive role to presence + a hive work subject.
**Depends on:** `queen::detect_hives_domain_aware`/`HiveInfo`/`AgentPhase`, exemplar
exchange, Wave 1 `sensemaking`, Task 4.1.
**Tests:** two peers sharing a domain co-resonate into one hive at moderate phase
gap; a bridge peer is detected (parity with `detect_hives_domain_aware`); one
off-topic high-Φ peer doesn't absorb a hive; empty swarm = singleton hives.
**Success:** `kannaka swarm hive [--form] [--json]` reports resonance hives with a
`focus_domain`; `--form` writes roles so `swarm plan --assign` routes each hive's
blind spots to its queue.

### Task 4.3 — Self-directed sensemaking loop (explicit Swarm-State machine)
**Files:** `src/swarm_loop.rs` (pure `next_state`, `StateOutcome`, `SwarmState`
enum); `src/bin/handlers/swarm.rs` (`swarm loop` daemon beside `serve`/`worker`).
Make the five ADR states real: `enum SwarmState { Discovery, Synchronization,
Sensemaking, Dreaming, Governance }`. Pure `next_state(state, &SwarmContext)` is a
deterministic transition over a snapshot (peer count, order parameter, open
contradictions, gap count, immune at-risk). The daemon wires each state to
already-built actions: Discovery = `swarm hive --form` (4.2); Sync = QueenSync step;
Sensemaking = `swarm brief`/`gaps`/`plan` (4.1); Dreaming = Wave 3.1 cross-agent
dream; Governance = Wave 2 `immune --apply` + contradiction resolution.
**Depends on:** `queen::QueenState`/sync (Sync), Wave 1 brief (Sensemaking), Task 4.1
+ Wave 3.3 (Sensemaking), Task 4.2 (Discovery), Wave 3.1 (Dreaming), Wave 2 immune
(Governance); L6 (the loop *is* the swarm OODA).
**Tests:** lone agent stays in Discovery; rising coherence drives
Discovery→Sync→Sensemaking in order; injected contradictions force a Governance
visit; the transition table is total; empty context = no-op (no panic).
**Success:** `kannaka swarm loop [--interval SECS] [--once]` runs the swarm
autonomously through the five states with no operator in the loop.

### Task 4.4 — L6 swarm-fitness arm (autoresearch generalizes to the collective)
**Files:** `src/bin/research.rs` (`run_experiment_l6_session`, `6 => …` dispatch);
`experiments/results-L6.tsv`; `experiments/ooda-state.json` (`.level` versioned
state); `research/autoresearch-cron.sh` (read level from state; rotate a swarm knob);
`.github/workflows/auto-merge-curiosity.yml` (allow `results-L6.tsv`).
Graduate the OODA from single-agent (L5, saturated ~0.0074) to swarm fitness:
simulate an N-agent partitioned-corpus swarm, score **`gap_detection_precision`**
(prime metric), **`recall_vote_gain`** (Cap 2), **`contradiction_recall`** (Cap 10),
and promote **`query_gravity`** to a scored axis. Add the plateau detector that
writes `SOLVED_ARCHIVED` + increments `.level`.
**Depends on:** the L5 harness as template; Tasks 4.1–4.3 supply the metrics;
`DREAM_GRAVITY` (`Params.dream_gravity`) already landed.
**Tests:** `--level 6` writes a well-formed `results-L6.tsv` row;
`gap_detection_precision` is 1.0 on a correctly-flagged held-out domain; plateau
detector fires only after N no-new-axis cycles; legacy `ooda-state.json` parses.
**Success:** `cargo run --release --bin research -- --level 6` reports a swarm
fitness; `autoresearch-cron.sh` keep/reverts swarm knobs against L6 and auto-advances
`.level` on plateau — the autoresearch now optimizes *collective* intelligence.

**Wave 4 dependency order:** 4.2 → 4.1 → 4.3 → 4.4. Hive formation (4.2) gives the
planner (4.1) routing targets; the planner feeds the Sensemaking state of the loop
(4.3); the L6 arm (4.4) scores the whole machine. 4.1 and 4.2 share the `sensemaking`
content-sim hook + `swarm peers`/exemplar presence — cores writable in parallel,
joined at the CLI; 4.4 depends only on 4.1–4.3's metrics being computable.

**Closing the North Star loop.** Waves 1–3 built the organs (voice, immune system,
cross-agent dreaming, temporal truth, coverage map). Wave 4 wires them into a closed
metabolism: the gap map (3.3) becomes directed curiosity (4.1); phase-locked peers
become specialized hives (4.2); the five states become an autonomously-cycling
machine (4.3) that surveys → synchronizes → makes sense → dreams → governs →
re-surveys. This is where **L6 OODA and the swarm meet** (4.4): the single-agent
autoresearch is generalized so the *swarm* picks its gaps, routes research to the
best-fit hive, and the OODA optimizes `gap_detection_precision`/`recall_vote_gain`
instead of one agent's `query_gravity`. Experience in → collective intelligence out,
on a loop that tunes itself.

---

## L6 OODA — Collective sensemaking as autoresearch territory

L5 of the autoresearch optimizes a **single agent's** wave-memory fitness and has
saturated (≈0.0074, 7/13 metrics pinned at 1.0). L6 graduates the OODA from
single-agent to **swarm/sensemaking** fitness — the same step-change as L4→L5
(1–2 engines → 6). ADR-0035 supplies the territory; the capabilities above become
measurable fitness terms.

**First lever, already landed: associative recall (`DREAM_GRAVITY`).** The core
wave-memory recall property was broken (`query_gravity` 0.37, below the 0.5 chance
line — the dream was *anti*-associative). Fixed `8768a2e`: an env-gated,
pre-dream-phase-anchored gravity pass lifts `query_gravity` to 1.0. At
`DREAM_GRAVITY=0.5` it also net-improved fitness/transfer/xi in a bare config, at
the cost of `carrier_emergence` (0.99 → 0.51). **The gravity↔carrier tension is
the first L6 research question** — sweep `DREAM_GRAVITY ∈ {0.25, 0.5, 1.0}` at
10 runs (per the 10-run reliability rule) in the tuned config, find the balance
that keeps recall associative without collapsing the carrier.

**Candidate L6 fitness metrics** (ground-truthable, derived from ADR caps):
- `recall_vote_gain` — consensus recall accuracy minus best single-agent (Cap 2).
- `gap_detection_precision` — does the coverage map correctly flag held-out
  domains as low-coverage (Cap 1)?
- `contradiction_recall` — fraction of injected contradictions detected (Cap 10).
- `query_gravity` — promote from instrumentation to a scored, first-class axis now
  that it is movable (add it to `experiments/results-L6.tsv`).

**Mechanical L6 setup** (per the research.rs level map): add `6 =>
run_experiment_l6_session` to the `match level` dispatch; clone the L5 session fn;
simulate a small swarm (N agents with partitioned corpora) and score the metrics
above into `experiments/results-L6.tsv`.

### Automated level advance (revive `experiments/ooda-state.json`)

There is currently no auto-advance and `OODA_LEVEL` is a static crontab env. Make
the level a piece of versioned state both producers read:
1. The cron reads the level from `experiments/ooda-state.json` (`.level`) instead
   of a static env.
2. A **plateau detector** codifies the curiosity loop's own arithmetic —
   `best_fitness − keep_threshold ≤ Σ per-axis residual headroom`, or N
   consecutive `*-no-new-axes.md` notes — and on fire: write a `SOLVED_ARCHIVED`
   block for level N (the schema already exists for L3/L4), increment `.level`,
   commit. The deterministic cron and the curiosity loop both pick up L6 on their
   next run.

## Issues / tracking strategy

File one tracking issue per wave deliverable in `kannaka-memory`/`kannaka-steward`,
linking cross-repo dependencies (Wave N+1 items reference the ADR section that
motivates them, as in the 0xSCADA wave plan). The Wave-1 → L6 bridge issues
(DREAM_GRAVITY 10-run sweep, `recall_vote_gain` metric, L6 harness, auto-advance)
should be opened first since they gate everything downstream.
