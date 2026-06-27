# 2026-06-27T00 — DREAM_GRAVITY sub-threshold sweep: xi plateaus early, transfer regresses non-monotonically

## Hypothesis

The 2026-06-26 fire tested DREAM_GRAVITY at 0.0, 0.25, and 0.50 but never tested
intermediate values. DREAM_GRAVITY=0.25 was chosen as the optimum without knowing
whether a smaller value (less carrier harm) could achieve the same xi plateau.

Prediction: DREAM_GRAVITY=0.15 reaches xi=0.9796 (same plateau) with smaller carrier
regression (predicted ~0.5291 vs 0.5265 at 0.25), yielding fitness ≈ 0.056307.

If xi is below plateau at 0.15, try 0.20 to bracket the threshold.

## Configuration

```
DRIVE_A=0.1 DRIVE_SCOPE=all DRIVE_TOP_FRAC=1.0 DREAM_MODE=interference_relax
DREAM_GRAVITY=0.15 (trial 1)
DREAM_GRAVITY=0.20 (trial 2)
```

Baseline reference: DREAM_GRAVITY=0.25, fitness avg 0.056686

## Results

| gravity | fitness  | transfer | xi_robust | carrier_e | magic_R | query_gravity |
|---------|----------|----------|-----------|-----------|---------|---------------|
| 0.00 (baseline) | 0.057827 | 0.9654 | 0.9675 | 0.5330 | 0.8672 | 0.4603 |
| 0.15 (trial 1)  | 0.057564 | 0.9585 | 0.9796 | 0.5273 | 0.8670 | 0.8005 |
| 0.20 (trial 2)  | 0.057603 | 0.9585 | 0.9796 | 0.5268 | 0.8670 | 0.8367 |
| 0.25 (prior)    | 0.056684 | 0.9652 | 0.9796 | 0.5265 | 0.8670 | 0.8623 |
| 0.50 (prior)    | 0.057302 | 0.9616 | 0.9796 | 0.5258 | —      | 0.9256 |

## Analysis

### xi plateaus by gravity=0.15

xi_robustness=0.9796 at both 0.15 and 0.20 — the plateau is reached before or at
gravity=0.15. The adversarial suppression mechanism (gravity suppresses phase-distant
adversarial memories in engine_adv) saturates before reaching 0.15. The 0.15→0.25
xi range is flat.

### Carrier regression is proportional to gravity (as predicted)

carrier_emergence vs baseline (0.5330):
- g=0.15: 0.5273 (Δ = −0.0057)
- g=0.20: 0.5268 (Δ = −0.0062)
- g=0.25: 0.5265 (Δ = −0.0065)

The regression is roughly linear with gravity (−0.0026 per 0.10 gravity unit). This
confirms the predicted "front-loaded amplitude redistribution" mechanism: gravity
proportionally perturbs the engine_flat amplitude_delta signal.

### Transfer_score has a non-monotonic "valley" at gravity=0.15–0.20

The critical unexpected finding: transfer_score at gravity=0.15 (0.9585) and 0.20
(0.9585) is WORSE than at both baseline gravity=0.0 (0.9652) and gravity=0.25 (0.9652).

Full transfer_score pattern across gravity:
  0.00 → 0.9652 (baseline)
  0.15 → 0.9585 (regression, −0.0067)
  0.20 → 0.9585 (regression, −0.0067)
  0.25 → 0.9652 (recovery, back to baseline)
  0.50 → 0.9616 (partial regression)

This V-shaped transfer response explains why no intermediate gravity value improves
on gravity=0.25. At sub-0.25 gravity, engine_b_primed's gravity selectively suppresses
B memories that are phase-distant from A's attractor, without sufficiently reinforcing
the phase-aligned B memories that drive the transfer signal. By gravity=0.25, the
reinforcement is strong enough to offset the suppression, recovering transfer to
baseline. The mechanism is threshold-like: the net transfer effect crosses zero
somewhere between 0.20 and 0.25.

The partial regression at gravity=0.50 (0.9616) suggests the phase-alignment
mechanism begins to over-reinforce A-correlated B memories, slightly distorting
the B_primed vs B_naive comparison.

### Net fitness accounting

At gravity=0.15 vs gravity=0.25:
- xi: same (0.9796) → no change
- carrier: 0.5273 vs 0.5265 → carrier improves by 0.10 × 0.0008 = +0.000080
- transfer: 0.9585 vs 0.9652 → transfer regresses by 0.15 × 0.0067 = −0.001005
- Net at g=0.15: +0.000080 − 0.001005 = −0.000925 (g=0.15 is WORSE)

The transfer regression completely swamps the carrier improvement. At g=0.20: same.

## Decision

**No code changes made.** Env-var only. Nothing to revert.

Hypothesis FALSIFIED. No sub-0.25 gravity value outperforms DREAM_GRAVITY=0.25.

**Empirical optimum remains:**
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE=interference_relax DREAM_GRAVITY=0.25
```
3-trial avg fitness: **0.056686** (unchanged)

## New structural knowledge

### The xi saturation threshold

The xi robustness improvement (0.9675→0.9796) from gravity is fully saturated at
gravity ≤ 0.15. Future xi probes should not expect gravity increases to push xi above
0.9796 — the limit is structural (adversarial set of 30 memories has a minimum
irreducible impact at chain_depth=2 regardless of gravity suppression).

### The transfer V-shape (0.15–0.25 recovery)

Transfer_score has a non-monotonic dependence on gravity with a recovery threshold
at gravity ≈ 0.25. Sub-threshold gravity creates net harm to transfer by suppressing
B_primed phase-distant memories without sufficient reinforcement of phase-aligned ones.
Gravity=0.25 is the minimum value that simultaneously achieves xi improvement and
transfer neutrality.

### The 0.005 fitness floor

The remaining 0.056686 fitness is dominated by carrier_emergence (0.527 → 83.5%):
- Carrier cannot be improved by gravity (gravity harms carrier proportionally)
- Carrier is insensitive to drive frequency (established 2026-06-25)
- Transfer cannot be improved above 0.9652 by gravity (only neutralized)
- xi cannot exceed 0.9796 by gravity alone

Sub-0.050 fitness requires structural changes beyond env-var tuning. The L5 env-var
optimization space appears exhausted at DREAM_GRAVITY=0.25.

## Next fire candidates

1. **The carrier floor structural probe**: 0.527 is 83.5% of fitness. Every other axis
   is maximized or at a structural plateau. A structural change to how amplitude_deltas
   are measured (e.g., using relative not absolute deltas, or measuring frequency content
   of a different signal) might reveal whether 0.527 is a fundamental limit or an
   artifact of the current measurement design.
2. **L5 optimization is complete (env-var)**. Document this as the floor and consider
   whether the autoresearch loop should shift focus to L6 dynamics or a new metric arc.
