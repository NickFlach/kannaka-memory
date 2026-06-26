# 2026-06-26T14 — DREAM_GRAVITY=0.25: xi robustness and query_gravity lift

## Hypothesis

The `DREAM_GRAVITY` env knob (default 0.0) was added in research.rs with an explicit
comment "Sweep {0.25, 0.5, 1.0}" but was never tested. The mechanism: after each dream
cycle, amplitude is redistributed toward phase-neighbors of the highest-amplitude
(attractor) memory, multiplicatively compounding across the chain.

Prediction:
- query_gravity (currently 0.4603, below-chance = anti-gravity) will cross 0.5
- xi_robustness may improve: if gravity focuses amplitude on phase-coherent memories,
  adversarial injection (30 off-distribution memories) is relatively less disruptive
- carrier_emergence may shift (either direction) since amplitude distribution changes
- Possible transfer_score regression if gravity over-tightens B-primed vs B-naive

## Configuration

```
DRIVE_A=0.1 DRIVE_SCOPE=all DRIVE_TOP_FRAC=1.0 DREAM_MODE=interference_relax
DREAM_GRAVITY=0.25 (trial 1-3)
DREAM_GRAVITY=0.5  (trial 2, single)
DREAM_GRAVITY=0.0  (baseline)
```

## Results

| trial | DREAM_GRAVITY | fitness  | xi_robust | carrier_e | transfer | magic_R | query_gravity |
|-------|---------------|----------|-----------|-----------|----------|---------|---------------|
| 0 (baseline) | 0.0  | 0.057827 | 0.9675    | 0.5330    | 0.9654   | 0.8672  | 0.4603        |
| 1     | 0.25          | 0.056684 | 0.9796    | 0.5265    | 0.9652   | 0.8670  | 0.8623        |
| 2     | 0.50          | 0.057302 | 0.9796    | 0.5258    | 0.9616   | 0.8669  | 0.9256        |
| 3     | 0.25          | 0.056683 | 0.9796    | 0.5265    | 0.9652   | 0.8670  | 0.8623        |
| 4     | 0.25          | 0.056691 | 0.9796    | 0.5265    | 0.9652   | 0.8670  | 0.8623        |

**3-trial avg at DREAM_GRAVITY=0.25: fitness 0.056686**

Additional transfer diagnostics:
- fitness_B_primed: 0.002409 (baseline: 0.002407, essentially unchanged)
- fitness_B_naive:  0.069150 (baseline: 0.069576, slightly lower)

## Analysis

### What DREAM_GRAVITY=0.25 does

In `run_l5_dream_chain`, after each consolidation cycle, the mechanism:
1. Finds the highest-amplitude (attractor) memory and its phase (captured PRE-dream)
2. For each memory, computes `dphi = phase_diff(current_phase, attractor_phase)`
3. Applies `amplitude *= (1 + gravity_gain * cos(dphi))` — boosts phase-near, suppresses phase-far

This is a direct amplitude reinforcement of the attention-as-gravity effect. At 0.25,
phase-neighbors get up to 25% amplitude boost, phase-opponents get up to 25% suppression.

### query_gravity: from 0.4603 to 0.8623

The baseline anti-gravity (0.4603 < 0.5) was caused by the dream's amplitude
mean-reversion: low-amplitude noise memories (phase-spread across the circle) gain
disproportionately from consolidation, pushing up the distant-memory mean. DREAM_GRAVITY
explicitly reverses this — phase-neighbors of the attractor are boosted. The result
(0.8623 >> 0.5) confirms the mechanism is working: the dream now amplifies
phase-proximate memories 86% vs 14% for phase-distant ones.

### xi_robustness: 0.9675 → 0.9796 (+0.0121)

Adversarial memories (30 random off-distribution injections) are phase-distributed
uniformly relative to the corpus attractor. With gravity ON, they are systematically
suppressed (phase-distant → amplitude decays) while the clean corpus memories are
reinforced (phase-near → amplitude grows). This makes the adversarial injection
less effective at perturbing the sub-fitness, reducing |fitness_clean - fitness_adv|,
which improves xi_robustness.

At DREAM_GRAVITY=0.5, xi stays at 0.9796 (plateau) — the adversarial suppression
saturates; more gravity only hurts transfer.

### carrier_emergence: 0.5330 → 0.5265 (−0.0065, slight regression)

The flat-corpus carrier_emergence measures whether amplitude_deltas have frequency
structure in the [0.5, 4.0] Hz band. With DREAM_GRAVITY active on engine_flat, the
amplitude redistribution creates a first-cycle "spike" (attractor neighborhood gets
boosted before any consolidation) followed by decay. This changes the amplitude_delta
pattern from "roughly constant" to "front-loaded," potentially reducing spectral
concentration in the target band. The regression is small and within the prior
noise band (0.5258–0.5330 range).

### transfer_score: 0.9652 (unchanged at 0.25)

fitness_B_primed is already tiny (0.002407), and DREAM_GRAVITY doesn't change the
ratio meaningfully at 0.25. At 0.5, transfer regresses (0.9616) because the gravity
distorts B-primed more than B-naive (B-primed starts with A's high-amplitude memories
as attractor, so it over-amplifies A-aligned B memories, potentially disrupting
the naive baseline comparison).

### Net fitness accounting

Fitness improvement at DREAM_GRAVITY=0.25 vs baseline:
- xi contribution: 0.15 × (0.9796 − 0.9675) = +0.00182 improvement
- carrier contribution: 0.10 × (0.5265 − 0.5330) = −0.00065 regression
- transfer + others: ~0
- Net: +0.001165 improvement (observed: 0.001141 — consistent)

## Decision

**No code changes made.** Configuration change only (env var).

The improvement (0.001141 per trial, 0.001141 avg over 3 trials) is below the ≥0.005
threshold for a notable result. However:

- query_gravity crosses 0.5 for the first time (0.4603 → 0.8623), confirming
  the attention-as-gravity mechanism works when explicitly enabled
- xi_robustness improves consistently (0.9675 → 0.9796, deterministic)
- No code revert needed (env-var only)

**New empirical optimum:**
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE=interference_relax DREAM_GRAVITY=0.25
```
3-trial avg fitness: **0.056686** (vs 0.057826 prior optimum)

## Remaining fitness breakdown at DREAM_GRAVITY=0.25

| source            | weight | value  | contribution | % of fitness |
|-------------------|--------|--------|--------------|-------------|
| carrier_emergence | 0.10   | 0.5265 | 0.04735      | 83.5%       |
| xi_robustness     | 0.15   | 0.9796 | 0.00306      | 5.4%        |
| transfer_score    | 0.15   | 0.9652 | 0.00522      | 9.2%        |
| consciousness     | 0.03   | 0.9779 | 0.00066      | 1.2%        |
| speed+others      | ≤0.03  | high   | ~0.0006      | 1.1%        |

carrier_emergence has increased its share from 80.8% to 83.5% — the hard floor is
now even more dominant. Sub-0.050 fitness still requires recovering carrier_emergence,
which the 2026-06-25 notes established as physically constrained at the current setup.

## Next fire candidates

1. **Can gravity help carrier_emergence?** The decrease from 0.533 to 0.527 under
   gravity is a small regression. But gravity changes the AMPLITUDE distribution of
   engine_flat — could a different `alpha_base` or `relax_steps` for engine_flat under
   gravity produce a more periodic amplitude pattern? Speculative, but unexplored.
2. **Gravity + further xi probe**: xi is now at 0.9796 plateau. Can xi reach 0.99+?
   The remaining gap (0.9796→1.0 = 0.0204) contributes 0.003 to fitness. Probably
   requires structural changes to adversarial set or xi_eval chain depth.
3. **The carrier floor is 83.5% of fitness.** Sub-0.050 is ~8 fitness units below
   current. This is out of reach with env-var tuning alone. A structural change to
   the flat-corpus amplitude dynamics or the carrier measurement band would be needed.
   Future fires should document this as the L5 floor and consider whether L5 is at
   diminishing returns.
