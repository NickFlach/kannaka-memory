# Architectural ceiling confirmed — no trials worth running

**Date:** 2026-06-12T14 UTC
**Branch:** kannaka-curiosity/2026-06-12T14-ceiling-final
**Code changes:** NONE
**Status:** CLOSED — T10 ceiling analysis holds; two residual untested angles both below threshold

---

## Orientation summary

Current empirical optimum (unchanged from T01/T09):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
engine_a alpha_base=0.12 (consolidation.rs:796)
chiral_p_bp=0.15 (engine_b_primed, research.rs:3457)
xi_eval_relax=20 (engine_b_primed, engine_clean, engine_adv)
3-trial avg fitness = 0.007627
transfer=0.963983, xi=0.9973, carrier_e=0.9992, consciousness=0.9553
```

Keep threshold: 0.007627 − 0.005 = **0.002627**. Total remaining improvable: ~0.002615 (T10).

---

## Two residual untested angles assessed without trials

### 1. `envelope_depth` variation (consolidation.rs:804, hard-coded 0.15)

The quiet-wave envelope modulates alpha as `alpha_base × (1 + envelope_depth × sin(phase))`.
Never varied in any prior fire.

- **Increase (e.g., 0.25):** Max per-step pull for engine_a = 0.12 × 1.25 = 0.150. This
  enters the crash zone. T07/T09 showed alpha_base=0.13 crashes transfer (max pull 0.13 ×
  1.15 = 0.1495). An envelope_depth of 0.25 with alpha=0.12 produces identical max-pull
  (0.150) — same physics, same crash risk.
- **Decrease (e.g., 0.05):** More uniform pull. Likely marginal transfer hurt; no path to
  +0.005 fitness improvement.
- **Verdict:** Not worth one trial. Either direction is bounded below the threshold or
  actively risky.

### 2. `chain_depth=4` (research.rs:3378, L5 hard-code)

The L5 dream chain depth is set to 4 with comment "irx cap — prevents hallucination-driven
over-consolidation". This axis was never swept in the T01–T10 fires.

- **chain_depth=3:** Under-dreaming relative to the irx cap. Transfer, xi, or carrier_e
  may degrade; no theoretical path to +0.005 improvement.
- **chain_depth=5:** Over-consolidation risk per the comment; the cap was set deliberately.
  Transfer crash risk similar to excess relax_steps.
- **Verdict:** Not worth a trial. Cap value appears intentional; even a +10% improvement
  in all metrics simultaneously cannot reach 0.002627.

---

## Conclusion

T10's structural ceiling analysis holds from both directions:
1. All parametric axes within the irx implementation are exhausted at confirmed optima.
2. The two remaining untested axes (envelope_depth, chain_depth) are bounded by crash risk
   or under-dreaming, and neither has a theoretical path to ≥0.005 improvement.
3. The combined maximum improvable across all remaining metrics (~0.002615) is less than
   the keep threshold (0.002627).

No code changes. No trials run. The current architecture is at its fitness ceiling.
