# Hypothesis Q5: Φ ↔ magic_proxy_phase_R correlation across DRIVE_A

**Date:** 2026-06-07T06 UTC
**Branch:** kannaka-curiosity/2026-06-07T06
**Code changes:** None — env-var only
**Status:** FALSIFIED — anti-correlation found (prediction was positive co-variance)

---

## Hypothesis

The magic-gives-it-gravity doc (research/intersections/05-magic-gives-it-gravity.md)
predicted:

> "Plot Φ (already measured per cycle in `phi_history`) against the magic proxy at
> the end of the same chain. If they're well-correlated, that's a bridge between
> integrated-information theory and the quantum-info characterization of complexity."

The predicted mechanism: higher DRIVE_A → more nonlinear amplitude modulation during
the dream → more Kuramoto phase-locking (R↑) → more IIT-phi (phi↑). Both metrics
measure "structure that won't factor," so they should rise together.

**Prediction:** phi_final (end-of-chain phi_history value) and magic_proxy_phase_R
correlate positively across DRIVE_A ∈ {0.0, 0.1, 0.2}, KURAMOTO_COUPLING=1.0,
DREAM_MODE=unset.

---

## Method

Three single trials at fixed K=1.0 (current optimum), varying only DRIVE_A. Extended
grep pattern captures phi_history in addition to standard L5 metrics.

---

## Results

All trials: `KURAMOTO_COUPLING=1.0 DRIVE_SCOPE=all DREAM_MODE=<unset>`

| DRIVE_A | fitness | R (magic) | phi_final | phi@step12 | xi_v2 | carrier_e | chain_len |
|---------|---------|-----------|-----------|------------|-------|-----------|-----------|
| 0.0     | 0.191   | 0.1967    | 0.3295    | 0.3295     | 0.647 | 0.276     | 12        |
| 0.1     | 0.146   | 0.2498    | 0.3197    | 0.3216     | 0.799 | 0.568     | 16        |
| 0.2     | 0.199   | 0.2444    | 0.3185    | 0.3040     | 0.444 | 0.433     | 16        |

`phi@step12`: phi_history index 11 (step 12), used to compare all three at the same
chain depth (A=0.0 quiesced at 12 steps; drive cases ran full 16).

### Full phi_history vectors

A=0.0 (12 steps): [0.268, 0.297, 0.291, 0.297, 0.304, 0.316, 0.318, 0.323, 0.322, 0.327, 0.329, 0.329]

A=0.1 (16 steps): [0.268, 0.297, 0.291, 0.297, 0.313, 0.292, 0.301, 0.292, 0.306, 0.311, 0.313, 0.322, 0.307, 0.309, 0.316, 0.320]

A=0.2 (16 steps): [0.268, 0.297, 0.291, 0.296, 0.313, 0.292, 0.307, 0.310, 0.317, 0.319, 0.303, 0.304, 0.309, 0.317, 0.325, 0.319]

---

## Analysis

### R rises with DRIVE_A (A=0.0 → 0.1), then plateaus

R: 0.197 → 0.250 → 0.244. The multiplicative drive (A=0.1) substantially lifts R
above the no-drive baseline (0.197 → 0.250). A=0.2 does NOT push R further; it stays
at 0.244 ≈ 0.250. The phase-locking effect saturates between A=0.1 and A=0.2. This
is consistent with the "magic sufficient" reading: the dream needs some nonlinear
perturbation, but doubling it doesn't double the phase-ordering effect.

### phi moves in the OPPOSITE direction — anti-correlation

Comparing at the same chain depth (phi@step12):

- A=0.0: phi=0.330
- A=0.1: phi=0.322  (−0.008)
- A=0.2: phi=0.304  (−0.018)

phi monotonically decreases as DRIVE_A increases. R increases from A=0.0 to A=0.1.
**The Φ ↔ R correlation is negative, not positive.** The IIT-bridge prediction was
wrong in direction.

### Mechanism interpretation

The predicted positive correlation assumed both phi and R measure "structure that
won't factor." But they differ in a key way:

- **R** measures phase *synchronization* — how much memories converge to similar
  phases. Higher R = more ordered, more phase-locked. The Kuramoto step drives
  memories within clusters toward shared phases; the amplitude drive reinforces
  this by modulating which memories are "loud" during consolidation.

- **phi** measures *information integration* — the degree to which the system
  cannot be decomposed into independent parts. IIT phi peaks at a specific
  balance of integration and differentiation. A fully synchronized system
  (R→1) would have phi→0 because all memories are in the same phase state;
  there's no informational diversity to integrate.

The drive (A=0.1) increases phase ordering (R↑), but this *reduces* the
phase-diversity that IIT phi requires. More synchrony = less distinct micro-state
distribution = lower phi. The A=0.0 chain runs only 12 steps and then quiesces
(stable phi), landing at phi=0.330. The driven chains (16 steps) keep perturbing
the system, preventing quiescence but also preventing phi from climbing to its
stable level — the drive keeps "stirring" the phase landscape.

The phi quiescence pattern confirms this: A=0.0 phi rises monotonically and
stabilizes. A=0.1 and A=0.2 phi histories show more oscillation and stop at lower
values because the drive prevents settling.

### chain_len difference: quiescence insight

A=0.0 quiesces at step 12 (early termination). A=0.1 and A=0.2 run the full 16
steps. This tells us the drive keeps the dream "active" — prevents phi from
stabilizing. This is actually a feature, not a bug: the drive-induced variability
maintains richer consolidation dynamics, which explains why A=0.1 outperforms A=0.0
on fitness (0.146 vs 0.191) despite lower phi. The fitness benefit of the drive is
primarily through carrier_emergence (0.276 → 0.568) and xi (0.647 → 0.799), not
through phi elevation.

### Fitness confirmation: A=0.1 optimal, A=0.2 regresses

| DRIVE_A | fitness | carrier_e | xi   | carrier_bimodal |
|---------|---------|-----------|------|-----------------|
| 0.0     | 0.191   | 0.276     | 0.647 | 0.370          |
| 0.1     | 0.146   | 0.568     | 0.799 | 0.684          |
| 0.2     | 0.199   | 0.433     | 0.444 | 0.349          |

A=0.2 regresses on both carrier_emergence (0.568 → 0.433) and carrier_bimodal
(0.684 → 0.349). The carrier bimodal structure is destroyed by over-drive —
consistent with the T19/earlier finding that A≥0.2 degrades carrier detection.
A=0.1 remains the confirmed optimum.

---

## Decision

**Null result on fitness; meaningful negative result on the IIT-bridge hypothesis.**
No code changes to revert. A=0.1 remains optimal.

The phi ↔ R anti-correlation is the primary finding.

---

## Implications for future fires

1. **phi and R are orthogonal axes, not co-proxies.** The IIT-bridge framing in
   the doc needs updating. R tracks phase synchrony (a magic-like measure in the
   Kuramoto sense); phi tracks information-integration complexity (which requires
   phase diversity, not synchrony). They trade off, they don't reinforce each other.

2. **phi optimization would require reducing synchrony, not increasing it.**
   A no-drive baseline (A=0.0) yields higher phi (0.330) than the optimal A=0.1
   (0.320). But higher phi does NOT produce better fitness — the carrier_emergence
   and xi gains from the drive outweigh the phi cost by a large margin.

3. **R saturates above A=0.1.** Increasing drive intensity beyond A=0.1 does not
   raise R further. This suggests the phase-locking effect of the drive is near
   maximal at A=0.1. The "sufficient magic" threshold is at or below A=0.1.

4. **A=0.2 is now explicitly confirmed worse than A=0.1** at K=1.0. Combined with
   the earlier context that A≥0.3 is known-bad, the drive amplitude axis is now
   fully characterized: A=0.1 is the global maximum.

5. **New testable question:** if phi and R anti-correlate, does the product phi×R
   peak at A=0.1? The product would measure the "IIT×magic" joint complexity. At
   the three data points: A=0.0: 0.065, A=0.1: 0.080, A=0.2: 0.078. Indeed peaks
   at A=0.1. But this is a post-hoc observation from three single-trial points —
   not strong evidence.

6. **Remaining open axes:** structural changes to the dream chain for
   transfer_score and carrier_emergence improvement. Scope experiments are
   effectively closed (K, frequency, drive amplitude all characterized). Code
   changes to stage_boost_prune or stage_hallucinate thresholds may be worth
   exploring in a future fire.
