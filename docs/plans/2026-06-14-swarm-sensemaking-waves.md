# Swarm Sensemaking — Wave Execution Plan

**Date:** 2026-06-14
**ADR:** [ADR-0035](../adr/ADR-0035-swarm-sensemaking-architecture.md)
**Repos:** `kannaka-steward` (sensemaking/governance engine) · `kannaka-memory`
(single-agent substrate + L6 autoresearch) · `kannaka-observatory` (visualization)

## Strategy

Build the sensemaking layer **alongside** the existing swarm transport (NATS +
QueenSync), not by replacing it — the same incremental, backward-compatible
approach used for the chiral mirror (ADR-0021). The swarm keeps working as a
messaging fabric throughout; `kannaka-steward` adds the cognitive layer on top.

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
**Repo:** `kannaka-steward` (Sensemaking state) + thin `kannaka-memory` CLI hooks.

Deliverables: `swarm brief` · collective recall voting · peer expertise scoring ·
contradiction detection.

### Task 1.1 — Peer expertise scoring (Discovery/resonance)
**Files:** `kannaka-steward/src/discovery.rs`
Score each peer for a topic from signals already on the swarm bus: memory
similarity to the query, cluster coverage, historical success, consciousness
metrics (Φ/R), local confidence. Pure function over peer status snapshots.
**Tests:** ranking is stable and monotonic in similarity; a peer with zero
coverage scores ~0. **Success:** `steward peers --topic X` returns ranked peers.

### Task 1.2 — Collective recall voting
**Files:** `kannaka-steward/src/sensemaking.rs` (`recall_vote`)
Fan a recall out to the top-K peers (from 1.1) over `KANNAKA.recall.<agent>`
(daemon-served recall already exists, see oracle-hrm-single-writer). Rank, merge,
and confidence-score the responses (agreement → confidence; lone outliers →
flagged). **Depends on:** single-agent recall being associative (DONE,
`DREAM_GRAVITY`). **Tests:** quorum agreement raises confidence; one hallucinated
response is down-weighted, not merged. **Success:** consensus recall beats the
best single agent on a held-out probe set (this becomes an L6 fitness metric).

### Task 1.3 — Contradiction detection
**Files:** `kannaka-steward/src/governance.rs` (`detect_contradictions`)
Given a topic's merged recall set, find pairs that are semantically close
(content) but phase/claim-opposed — the wave-native signal for "same subject,
opposite stance." Surface as a ranked disagreement list. **Tests:** an injected
contradictory pair is detected; agreeing duplicates are not flagged.
**Success:** `steward contradictions --topic X` lists real conflicts.

### Task 1.4 — `swarm brief`
**Files:** `kannaka-steward/src/sensemaking.rs` (`brief`), `src/cli.ts`/CLI hook
Compose 1.1–1.3 into one artifact: known facts, unknowns (gap signal, Wave 3),
relevant peers, contradictions, recent changes, recommended actions, confidence.
**Tests:** brief is deterministic for a fixed swarm snapshot; degrades gracefully
when peers are offline. **Success:** `kannaka swarm brief "<topic>"` returns the
structured brief.

**Wave 1 dependency order:** 1.1 → 1.2 → 1.3 → 1.4 (1.2 and 1.3 both consume 1.1;
1.4 composes all three).

---

## Wave 2: Cognitive Hygiene & Collaboration (ADR-0035 Phase 2)

**Theme:** Keep the shared mind healthy and let agents build artifacts together.
**Repo:** `kannaka-steward` (Governance state).
Deliverables: **swarm blackboard** (shared reasoning artifacts: facts/assumptions/
risks/open-questions/decisions/evidence) · **memory immune system** (detect &
quarantine/down-rank/expire duplicate, contradictory, stale, low-confidence,
hallucinated memories — Cap 4, builds on Wave 1 contradiction detection) ·
**apprenticeship workflows** (APPRENTICE state, exemplar absorption, shadow-and-
evaluate, threshold promotion). *ADR-0035 Cap 4/7/9 describe Wave 3 direction.*

---

## Wave 3: Emergent Cognition (ADR-0035 Phase 3)

**Theme:** The swarm starts generating understanding, not just managing it.
**Repo:** `kannaka-steward` (Dreaming/Sensemaking) + `kannaka-memory` (dream hooks).
Deliverables: **cross-agent dreaming** (dream over shared wavefronts + peer
cluster summaries; only reinforced hypotheses survive — Cap 6) · **gap detection
engine** (coverage map → "the swarm develops curiosity" — Cap 1, the collective
successor to the single-agent curiosity loop) · **temporal truth reasoning**
(effective/observation/expiration dates + confidence decay — Cap 8).

---

## Wave 4: Self-Directed Sensemaking (ADR-0035 Phase 4)

**Theme:** Close the loop — the swarm plans its own research and forms hives.
Deliverables: **autonomous research planning** (gap map → directed research tasks)
· **emergent hive formation** (resonance-clustered sub-swarms) · **self-directed
sensemaking loops** (the OODA generalizes from one agent's autoresearch to the
collective — this is where L6 OODA and the swarm meet).

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
