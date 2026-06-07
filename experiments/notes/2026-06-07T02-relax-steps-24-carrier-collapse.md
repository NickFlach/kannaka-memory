# interference_relax relax_steps=24 — carrier_emergence collapse narrows coupling budget

**Date:** 2026-06-07T02 UTC
**Branch:** kannaka-curiosity/2026-06-07T02
**Code changes:** None kept (reverted)
**Status:** FALSIFIED — carrier_emergence collapses at relax_steps≥20; 16 is the ceiling

---

## Background

T00 (2026-06-06T00) established:
- `stage_interference_relax` with `relax_steps=16, alpha_base=0.10` (total coupling ≈ 1.6 units)
  → 3-trial avg fitness 0.149; carrier_e 0.497, xi avg ~0.607
- `relax_steps=16, alpha_base=0.20` (total 3.2 units) → carrier_e **collapsed to 0.000**
- T00's estimate: "safe at 1.6, dead at 3.2"

Current stage_sync K=1.0 optimum: fitness avg 0.138.

## Hypothesis

Raise `relax_steps` from 16 to 24 (keeping alpha_base=0.10, total coupling = 2.4 units).  
The quiet-wave envelope completes one full cycle regardless of step count, so finer
steps sample it more smoothly. More relaxation time should push xi higher
(better cluster convergence) without necessarily collapsing carrier_emergence, since
2.4 is midway between the "safe" 1.6 and "dead" 3.2 thresholds from T00.

**Prediction:** xi rises from avg ~0.607 toward ~0.72, carrier_e stays in 0.25–0.45
range, fitness drops from 0.149 toward 0.138 (matching stage_sync optimum).

## Code change tested

```
- let relax_steps: usize = 16;
+ let relax_steps: usize = 24;   // trial 1
+ let relax_steps: usize = 20;   // trial 2 (narrowing)
```

## Trials

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| steps | total coupling | fitness | xi | carrier_e | magic_R | query_gravity | transfer |
|-------|---------------|---------|-----|-----------|---------|---------------|---------|
| 16 (T00 avg) | 1.6 | **0.149** | 0.607 | **0.497** | 0.617 | 0.364 | 0.750 |
| 20 | 2.0 | 0.231 | 0.325 | **0.000** | 0.611 | 0.365 | 0.817 |
| 24 | 2.4 | 0.195 | 0.762 | **0.000** | 0.627 | 0.414 | 0.677 |

## Findings

**Hypothesis falsified.** Carrier_emergence collapsed at 2.0 units (relax_steps=20),
not at 2.4 as predicted. T00's "safe at 1.6, dead at 3.2" estimate was optimistic —
the true carrier collapse threshold is between 1.6 and 2.0 total coupling units.

The collapse is consistent across both probes (carrier_e = 0.000 at both 20 and
24 steps). This is not sampling variance — the carrier FFT needs a minimum phase
spread to produce a carrier peak, and even 2.0 units of total phase relaxation
erases that spread.

**Xi behavior is not monotone**: xi = 0.762 at 2.4 steps but only 0.325 at 2.0.
This is the known xi stochasticity (unseeded eval_xi_robustness_v2 adversarial
perturbation). The high value at 2.4 is likely a lucky draw; the low value at 2.0
is unlucky. Both have carrier_e = 0.000, so both give bad fitness regardless of xi.

**transfer_score at 2.0 steps**: 0.817 (notably higher than T00's 0.750). This is
surprising — more relaxation steps improve transfer slightly. But this gain is
completely swamped by the carrier_e collapse cost (carrier_e weight = 0.10;
losing 0.497 in carrier_e costs ~0.050 fitness, vs transfer gain of ~0.067×0.15 = 0.010).

## Narrowing of the coupling budget

Updated estimate: carrier_emergence requires total phase coupling < ~1.8 units.
The operating point relax_steps=16, alpha_base=0.10 is close to the upper boundary.
There is no room to raise relax_steps further without crossing into the collapse regime.

## Decision

**No code change kept.** `src/consolidation.rs` reverted to relax_steps=16.

The relax_steps axis is now closed — 16 is both the current value and the practical
maximum. Further xi improvement through interference_relax mechanics would require
either:
1. A different coupling geometry (not relax further, but reweight which pairs are used)
2. Reducing alpha_base and raising relax_steps proportionally (e.g., alpha=0.08,
   relax_steps=20, total=1.6) — same total coupling, finer steps. Unlikely to help
   since total coupling is what matters for carrier collapse, not step granularity.
3. Exploring the `DRIVE_SCOPE=no_transfer` + `DREAM_MODE=interference_relax`
   combination, which has never been tested and uses a different lever.

## Empirical optimum unchanged

    DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=<unset>  KURAMOTO_COUPLING=1.0
    3-run avg fitness ≈ 0.138
