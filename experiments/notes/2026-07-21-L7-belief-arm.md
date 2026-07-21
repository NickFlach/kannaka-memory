# L7 belief arm — first light (2026-07-21)

`research --level 7` now scores ADR-0037's falsification clause on a live
multi-agent belief substrate (see `src/belief_fitness.rs` + the L7 session in
`research.rs`; README "belief substrate" section). Fitness convention: lower =
better; 0.5 per axis = no evidence.

## Baseline findings (n_agents=4, epochs=6, deterministic — 2 identical runs)

| axis | score | reading |
|---|---|---|
| stability_recall | 0.3738 | prediction 1 LEANS AGAINST: more-stable cores answer contested cues *less* reliably in this regime |
| merge_consolidation | 0.0000 | prediction 2 FALSIFIED at this timescale: 13 core merges, zero field consolidation events (energy prune threshold 0.01 is unreachable in 6 epochs — cores churn as embedding artifacts over a stable field) |
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
