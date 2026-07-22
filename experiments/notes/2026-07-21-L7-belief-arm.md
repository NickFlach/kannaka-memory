# L7 belief arm — first light (2026-07-21)

`research --level 7` now scores ADR-0037's falsification clause on a live
multi-agent belief substrate (see `src/belief_fitness.rs` + the L7 session in
`research.rs`; README "belief substrate" section). Fitness convention: lower =
better; 0.5 per axis = no evidence.

## Baseline findings (n_agents=4, epochs=6, deterministic — 2 identical runs)

| axis | score | reading |
|---|---|---|
| stability_recall | 0.3738 | prediction 1 LEANS AGAINST: more-stable cores answer contested cues *less* reliably in this regime |
| merge_consolidation | ~~0.0000~~ **0.5000** | ~~prediction 2 FALSIFIED~~ **RETRACTED — see below: the observable is structurally blind; prediction 2 is UNTESTABLE in ChiralMedium as built** |
| shared_agreement | 0.7464 | prediction 3 LEANS SUPPORTED: pairs sharing more cores agree more under ambiguous recall (the Track-D thesis) |
| l7_fitness | 0.5893 | |

## The coupling contrast (L7_COUPLE=1, one run)

Track-D per-epoch coupling: final cores 38→18, merges 13→3,
stability_recall 0.37→**0.74** (prediction 1 FLIPS to supported),
shared_agreement 0.75→0.50 (pair variance collapses — all pairs converge, so
the correlation loses its gradient), fitness 0.589→0.527.

**Coupling appears to stabilize beliefs into recall-reliable structures.**

## Coupling-strength sweep (same session, `L7_COUPLE_STRENGTH` knob added)

| s | final cores | stability_recall | shared_agreement | l7_fitness |
|---|---|---|---|---|
| 0.05 | 36 | 0.37 | 0.26 | 0.759 |
| 0.10 | 31 | 0.42 | **0.89** | **0.523** |
| 0.20 | 18 | **0.74** | 0.50 | 0.527 |
| 0.40 | 10 | 0.42 | 0.50 | 0.657 |
| 0.80 | 25 | 0.32 | **0.11** | 0.707 |

Non-monotone with a clear optimal band (~0.1–0.2), and the two predictions
peak at DIFFERENT strengths: s=0.1 maximizes the swarm claim (shared ⇒
agreement, 0.89) while keeping pair variance alive; s=0.2 maximizes the
single-agent claim (stability ⇒ recall, 0.74) at the cost of collapsing the
pair gradient; s=0.8 over-couples and actively anti-predicts (0.11) — the
same weak-coupling-preserves-diversity lesson as the L5 Kuramoto K-sweep
(K=0.5 beat K=1.0). Track-D's default strength should live in the 0.1–0.2
band, and which end depends on whether the swarm optimizes collective
agreement or individual recall reliability.

## Prediction 2 RETRACTION + architecture finding (same day)

The initial "falsified at this timescale" reading did not survive a detector
self-test: junk stored at importance 0.001 — far below the deep-dream prune
threshold (0.005) — STILL produced zero absorb events across 6, 12, and 18
epoch sessions. Root cause is structural, not parametric:
**`Hemisphere` wave dynamics floor energy at 0.01**
(`hemisphere.rs` `(e + growth − dampening).max(0.01)`, plus a
`dream_energy_floor` inside the dream), while the deep dream prunes below
0.005 — so `wavefronts_dissolved` can NEVER be nonzero in the holistic
hemisphere. Lite dreams have no prune path at all.

Consequences:
- `belief_fitness::merge_consolidation_score` now returns the no-evidence 0.5
  when the session observed zero absorb events (blind observable ≠
  refutation). Baseline l7_fitness moves 0.589 → 0.464.
- The floor-vs-threshold contradiction itself is a substrate finding worth an
  issue: either the floor or the prune threshold is dead code as shipped.
- The REAL consolidation machinery is `HrmStore`'s ADR-0036 resonance-merge
  (`KANNAKA_CONSOLIDATE`, belief-safe caps). Testing prediction 2 properly
  means an L7 variant driving HrmStore-level sessions where merges CAN
  consolidate — the natural next increment for this arm.

Session-length note: uncoupled stability_recall degrades with epochs
(6→12→18: 0.37→0.49→0.14, non-monotone but collapsing) while merges scale
linearly (~2.3/epoch) — long uncoupled belief fields churn.

## Prediction 2 unblinded: HrmStore substrate (same day, follow-up commit)

The session now runs a dedicated prediction-2 sub-experiment on `HrmStore`
(ADR-0036 belief-safe resonance-merge, `KANNAKA_MERGE_UNDER_BELIEF=1`,
Apply mode each epoch, throwaway readonly store): near-duplicate triplets per
domain give the merge machinery real prey, and **the absorb observable now
fires** (`would_absorb > 0` — 1 absorb epoch in the first run). Remaining gap:
the session generates no CORE-level merges to align against — near-dups
collapse inside a single domain core rather than forming two cores that fuse.
merge_consolidation therefore reads an honest 0.5 (no merges observed, live
channel) instead of the earlier blind 0.5.

**Open experimental design:** induce genuine core merges — ingest two
initially-distinct sub-domains, then BRIDGE them mid-session (shared-vocab
items, card-04 style) so their cores approach and fuse while apply-mode
consolidation runs; prediction 2 then gets its first real test.

## Prediction 2 — FIRST REAL RESULT (bridging + canary, follow-up commit)

The P2 sub-session now: (a) drives a bare ChiralMedium (HrmStore's
MediumBackend::insert never routes to the chiral field, so a fresh
chiral-upgraded store can't ingest — cores stayed empty); (b) applies the
EXACT ADR-0036 grouping (`compute_merge_grouping` made pub) to the right
hemisphere each epoch, absorbing admitted non-carriers; (c) detects core
FUSIONS by collision (two parents → one child core; the earlier
died-into-sibling detector read every fusion as two survivals — chiral
fusion count corrected 13 → 44); (d) proves the channel live with a CANARY
exact-duplicate that must pass the gates.

Result: canary absorbed ✓ (channel live), bridging fused cores 10→~5 with
8 fusion events, and **merge_consolidation = 0.125** — 7 of 8 core fusions
had NO consolidation event within ±1 epoch.

**Horizon sweep caveat (`L7_P2_WINDOW` 1/2/3/5 → 0.10/0.30/0.50/0.60 at 10
epochs): the rise is a VACUITY ARTIFACT, not eventual consolidation.** With
one absorb event (the canary) and merges scattered across epochs, a wider
window mechanically sweeps more merges into range of that single event; at
window ≥ session length any absorb scores 1.0 trivially. Keep the window ≪
epochs, and note a proper "fusion ⇒ eventual consolidation" test needs
MULTIPLE independent absorb events (a richer canary schedule, or a regime
where consolidation genuinely fires more than once). First genuine evidence AGAINST
prediction 2 in this regime: belief-core fusion is mostly an embedding-
geometry event (bridge content pulls clusters together), not a
consolidation event. Either the prediction needs restating (fusion ⇒
*eventual* consolidation at longer horizons?) or core identity and memory
consolidation are more independent than ADR-0037 assumed.

## Prediction 2 — CLEAN falsification (canary semantics fixed, evening)

Two instrument corrections finalize the P2 verdict:
1. Canary absorptions are EXPERIMENTER ARTIFACTS — they prove the channel
   can fire but are not alignment evidence. The scorer gained
   `merge_consolidation_score_proven(..., channel_proven)`: merges + proven
   channel + zero ORGANIC absorbs = genuine 0.0 (vs no-evidence 0.5 when
   unproven). The earlier 0.125 was artifact-contaminated.
2. Canary attribution is by CONTENT, not id — the merge may keep either pair
   member as carrier and absorb the *original* (which is exactly what
   happened: `channel_proven` initially read false while the pair merged).

Final: channel_proven ✓, organic absorbs 0, fusions 8 →
**merge_consolidation = 0.0000. Prediction 2 is falsified in this regime:
belief-core fusion and memory consolidation are independent phenomena.**
ADR-0037's clause should be restated — core merges are embedding-geometry
events; consolidation is an energy/redundancy event; the substrate exhibits
both without coupling them.

## Coupling SCHEDULES: the trade-off dissolves under alternation (evening)

`L7_COUPLE_SCHEDULE` (comma list, cycled per epoch):

| schedule | stability_recall | shared_agreement | l7_fitness |
|---|---|---|---|
| 0.2,0.1 (strong→weak) | **0.5850** | **0.8536** | **0.4360** (day's best) |
| 0.1,0.2 (weak→strong) | 0.6213 | 0.5000 | 0.5452 |
| 0.1,0.1,0.2 | 0.5794 | 0.4155 | 0.5916 |
| 0.15 fixed (compromise) | 0.4814 | 0.5696 | 0.5768 |

**A swarm need not choose what it couples for — it can alternate, and ORDER
matters: consolidate first (strong epoch), then diversify (weak epoch).**
Strong-then-weak keeps both claims substantially satisfied; weak-then-strong
collapses the swarm claim; a fixed midpoint satisfies neither. This answers
open experiment 2 and is the arm's headline design insight so far — worth a
Track-D default: heartbeat coupling could alternate strength instead of
holding one value.

## MIN_COS sensitivity + clause restatement (late evening)

`L7_MIN_COS` sweep on the alternating (0.2,0.1) config: 0.75 → both
correlations degrade (loose matching = false-positive tracks/shares); 0.85
(default) → best (P1 0.585 / P3 0.854); 0.92 → P3 holds (0.865), P1 dips
(track fragmentation). **Verdicts stable across the sensible band; P2 = 0.0
at every setting (robust falsification). Default 0.85 validated.**

`DREAM_GRAVITY` × belief is NOT yet runnable: that knob lives in the L5
dream-chain harness, not in `ChiralMedium::dream` — a medium-level gravity
hook is prerequisite (open experiment, requires substrate code).

README falsifiability clause restated from "predicts" to **measured** with
the day's verdicts (graduation criterion met for the clause; remaining:
issue #583 disposition + stochastic-regime 10-run checks).

## Gravity × belief (medium-level hook built — night)

`ChiralMedium::dream` now ends with an associative phase-gravity pass when
`KANNAKA_DREAM_GRAVITY` > 0 (default 0.0 byte-identical; faithful port of the
L5 harness knob incl. the pre-dream-anchor lesson; 3 unit tests). Sweep on
the alternating-coupling config:

| gravity | stability_recall | shared_agreement | l7_fitness |
|---|---|---|---|
| 0 | 0.5850 | **0.8536** | **0.4672** |
| 0.25 | 0.6597 | 0.1522 | 0.6829 |
| 0.5 | 0.6546 | 0.1522 | 0.6849 |
| 1.0 | **0.6837** | 0.1679 | 0.6678 |

**Gravity is an individual-recall knob and a swarm-coherence poison.** It
lifts stability⇒recall (as L5 found for solo query_gravity) while driving
shared⇒agreement to ANTI-predictive (~0.15): each node sharpens around its
own attractor domain, so shared beliefs stop predicting shared answers. The
individual-vs-collective axis appears a third time (coupling strength,
coupling order, now gravity). Recommendation: keep `KANNAKA_DREAM_GRAVITY`
default-off for swarm deployments; consider it for solo nodes only.

## Suggested next experiments (single-knob, per protocol)

1. DONE — refined 0.125/0.15/0.175: stability_recall 0.64/0.48/0.44,
   shared_agreement 0.36/0.57/0.63. **No strength satisfies both predictions
   at once**; the band interior is rugged and the two claims trade off
   monotonically across it. s=0.1 (swarm-optimal) and s=0.2
   (individual-optimal) stand as the two regimes — a swarm must choose (or
   alternate) what it couples FOR.
2. Consolidation-capable regime hunt for prediction 2: longer sessions
   (`L7_EPOCHS`), decay-rate pressure, or junk at importance near the 0.01
   prune threshold — does merge_consolidation leave 0 in ANY regime, or is
   the falsification scale-bound?
3. `L7_MIN_COS` sweep (track/share match, 0.85): sensitivity of all three axes.
4. `DREAM_GRAVITY` × belief interplay (the L6 question, now measurable at L7).
5. Session is currently deterministic — after any knob that introduces
   stochasticity, return to 10-run averages.

Rows append to `experiments/results-L7.tsv` (union-merged, allowlisted in
auto-merge-curiosity.yml). Env knobs: L7_AGENTS/L7_EPOCHS/L7_ITEMS/L7_MIN_COS/
L7_MERGE_COS/L7_COUPLE.
