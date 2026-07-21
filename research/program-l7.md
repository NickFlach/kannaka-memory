# kannaka-research — Level 7 Design (the BELIEF arm)

**Status:** implemented and producing results (2026-07-21). First-light
findings and the day-one experiment log live in
`experiments/notes/2026-07-21-L7-belief-arm.md`.

**Predecessor:** L6 shipped collective-sensemaking fitness on synthetic
fixtures (gap detection, recall voting, contradiction recall) but was never
adopted by the nightly OODA (`results-L6.tsv` has one row). Meanwhile the
belief substrate (ADR-0037 content-born phase, spiral cores, Track-D
coupling; ADR-0036 belief-safe resonance-merge) shipped with a falsifiability
clause that nothing measured. L7 closes that gap: it is the first level whose
observables all come from a live medium rather than fixtures.

---

## 1. What L7 asks

The README's clause: a core only earns the word "belief" if it maps to a
recallable content cluster AND its dynamics predict

1. **core stability ⇒ recall reliability** (single-agent claim),
2. **core merge ⇒ a consolidation event** (substrate claim),
3. **shared cores ⇒ swarm agreement** (Track-D / swarm claim).

L7 scores each prediction in [0,1] (1 = holds, 0 = anti-predicts, 0.5 = no
evidence — including when an observable is blind, which the session PROVES
against with a canary) and aggregates a weighted `l7_fitness`
(`src/belief_fitness.rs`, lower = better, weights 0.40/0.25/0.35).

## 2. The session (`cargo run --release --bin research -- --level 7`)

Deterministic multi-agent ChiralMedium run, belief phase ON:

- **Agents** (`L7_AGENTS`, default 4): nested-overlap vocabulary domains —
  agent k holds shared domains 0..=k plus a private one, so agent PAIRS
  genuinely differ in shared beliefs (the gradient prediction 3 correlates
  against).
- **Epochs** (`L7_EPOCHS`, default 6): per-epoch distractor churn (2
  low-importance items), dreams (deep every 3rd), spiral-core snapshots
  chained by fingerprint; optional Track-D coupling every epoch
  (`L7_COUPLE=1`, strength `L7_COUPLE_STRENGTH`, default 0.2).
- **Recall probes:** contested 1-own-word-vs-1-neighbor-word cues (verbatim
  cues score flat 1.0 — zero variance measures nothing).
- **Prediction 2 sub-session:** bare ChiralMedium, two distinct sub-domains
  BRIDGED from mid-session (card-04 pattern) to induce core fusions
  (detected by collision: two parents → one child core), the EXACT ADR-0036
  grouping (`hrm_store::compute_merge_grouping`, pub) applied per epoch as
  the consolidation machinery, plus a CANARY exact-duplicate that must
  absorb — proving the channel live so a low score is genuine
  counter-evidence. `L7_P2_WINDOW` sets the merge↔absorb alignment window
  (epochs, default 1).
- Rows append to `experiments/results-L7.tsv` (union-merged; allowlisted in
  auto-merge-curiosity.yml).

Knobs: `L7_AGENTS · L7_EPOCHS · L7_ITEMS · L7_MIN_COS · L7_MERGE_COS ·
L7_COUPLE · L7_COUPLE_STRENGTH · L7_JUNK_IMPORTANCE · L7_P2_WINDOW ·
RESEARCH_RUN`.

## 3. Day-one results (2026-07-21)

- **P3 SUPPORTED** (0.75 baseline; 0.89 at coupling 0.1) — Track-D's thesis
  holds: sharing beliefs predicts agreeing under ambiguity.
- **P1 COUPLING-DEPENDENT** — 0.37 uncoupled (leans against; long uncoupled
  sessions collapse it further, 18 epochs → 0.14), FLIPS to 0.74 at
  coupling 0.2. Coupling stabilizes beliefs into recall-reliable structures.
- **P2 LEANING FALSIFIED** (0.125, channel proven live) — 7 of 8
  bridge-induced core fusions had no consolidation event ±1 epoch. Fusion
  looks like embedding geometry, not consolidation.
- **Coupling strength is a genuine trade-off:** s=0.1 optimizes the swarm
  claim, s=0.2 the individual claim, no strength satisfies both (fine sweep
  0.125–0.175), s=0.8 ANTI-predicts (0.11). Echoes the L5 Kuramoto K-sweep.

## 4. Open experiments (single-knob, per protocol)

1. `L7_P2_WINDOW` horizon sweep: does fusion ⇒ *eventual* consolidation at
   ±2/±3 epochs, or is P2 falsified at every horizon?
2. Coupling schedule: alternate s=0.1 / s=0.2 epochs — can a swarm get both
   claims by alternating what it couples for?
3. `L7_MIN_COS` sensitivity of all three axes.
4. `DREAM_GRAVITY` × belief interplay (the L6 question, now measurable).
5. Stochastic variants (any knob that breaks determinism) → 10-run averages.

## 5. Graduation criteria

L7 is *solved* when the three predictions have stable verdicts across the
knob space (supported / falsified / restated), the substrate issues it
exposed (#583 energy-floor-vs-prune, HrmStore insert-never-chiral) are
dispositioned, and ADR-0037's falsifiability clause can be rewritten from
"predicts" to "measured: …" with numbers. The next level above this one is
whatever the failed predictions become after restatement.
