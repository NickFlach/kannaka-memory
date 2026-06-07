# interference_relax + DRIVE_SCOPE=no_transfer — xi collapse, hypothesis falsified

**Date:** 2026-06-07T01 UTC
**Branch:** kannaka-curiosity/2026-06-07T01
**Code changes:** None — env-var only
**Status:** FALSIFIED — combination substantially worse than both components alone

---

## Hypothesis

Both `DREAM_MODE=interference_relax` (scope=all) and `DRIVE_SCOPE=no_transfer` (stage_sync)
independently improved fitness vs the 0.18 baseline:

- interference_relax scope=all (T00 fire): avg fitness 0.149 (vs 0.18 baseline)
- no_transfer K=1.0 scope (PRs #143-#147): avg fitness ~0.147 (vs 0.18 baseline)

These two changes operate on orthogonal pipeline stages:
- interference_relax: changes how the dream's phase consolidation works (constructive-pair
  relaxation replaces Kuramoto sync)
- no_transfer scope: changes which engine contexts receive the drive amplitude modulation
  (excludes engine_b_primed and engine_b_naive)

**Prediction:** Combination delivers both transfer_score improvement from interference_relax
(0.750 vs 0.682 baseline) and xi/carrier stability from no_transfer exclusion. Expected
combined fitness ~0.130–0.145, below the 0.138 stage_sync K=1.0 optimum.

---

## Trial

Config: `DRIVE_A=0.1 DRIVE_SCOPE=no_transfer DREAM_MODE=interference_relax`
(KURAMOTO_COUPLING irrelevant to interference_relax — Kuramoto sync is bypassed)

| trial | fitness | transfer_score | carrier_emergence | xi_robustness_v2 | magic_proxy_phase_R | query_gravity |
|-------|---------|----------------|-------------------|-----------------|---------------------|---------------|
| t1    | 0.225568 | 0.776860      | 0.4966            | 0.0672          | 0.6167              | 0.3639        |

**Baseline (stage_sync K=1.0 scope=all, 3-run avg):** fitness 0.138, xi ~0.86, transfer ~0.682,
carrier_e ~0.568, magic_R ~0.250

**Comparison to interference_relax scope=all (T00 avg):** fitness 0.149, xi ~0.607,
transfer ~0.750, carrier_e 0.497, magic_R ~0.617

---

## Finding: xi catastrophic collapse

**Prediction was wrong in the most important dimension.**

xi_robustness_v2 collapsed from ~0.607 (interference_relax scope=all) to **0.067** —
essentially the floor. The fitness penalty from this single metric accounts for the
regression: xi cost = (1 - 0.067) × 0.15 ≈ 0.140, more than the entire baseline fitness.

All other metrics are near their interference_relax scope=all values:
- transfer_score: 0.777 (even better than scope=all 0.750) ✓
- carrier_emergence: 0.497 (unchanged from scope=all) ✓
- magic_proxy_phase_R: 0.617 (unchanged) ✓
- query_gravity: 0.364 (unchanged) ✓

The xi collapse is the single cause of the fitness regression from 0.149 to 0.226.

### Why xi collapses under interference_relax + no_transfer

The `stage_interference_relax` function builds phase clusters through constructive-pair
alignment: memories that constructively interfere in the wave geometry nudge each other's
phases toward alignment over 16 relaxation steps. This phase clustering is what enables
high xi — once memories are clustered by category, the adversary can't find a small
perturbation that maps a clean-category memory to an adversarial-category one.

Under `scope=all`, every engine context (including engine_b_primed and engine_b_naive)
receives the drive amplitude modulation before each consolidation dream. The drive creates
a shared amplitude rhythm that the interference_relax step appears to use as an implicit
synchronization scaffold — the amplitude-sorted ordering and the drive phase together
create a consistent context for constructive-pair detection across all engines.

Under `scope=no_transfer`, engine_b_primed and engine_b_naive are excluded from the drive.
This breaks the amplitude-rhythm scaffold for exactly those engine contexts. The constructive-
pair detection in the B-engines runs on un-modulated amplitudes, creating a different pairing
geometry that then cross-contaminates via the dream chain into engine_a and the xi measurement
engines. The xi measurement (eval_xi_robustness_v2) runs on engine_clean and engine_adv, which
both see the degraded phase structure propagated through the chain.

Under `stage_sync` (Kuramoto), this contamination doesn't happen: stage_sync operates on
working_set phases directly with coupling_strength=1.0, independent of amplitude modulation.
The Kuramoto sync is self-contained per engine and isn't sensitive to whether the drive ran
before it. That's why no_transfer works well with stage_sync (K=1.0, scope=no_transfer:
avg ~0.147) but fails catastrophically with interference_relax.

### Decision boundary intuition

The interference_relax mechanism implicitly depends on amplitude-modulated synchronization
across ALL dreaming engines. The no_transfer scope violates this implicit assumption by
creating a two-tier amplitude landscape. stage_sync doesn't have this dependency.

---

## Result summary

| config | fitness | xi | transfer | carrier_e | magic_R | notes |
|--------|---------|-----|---------|-----------|---------|-------|
| stage_sync K=1.0 scope=all | 0.138 avg | ~0.860 | ~0.682 | ~0.568 | ~0.250 | current optimum |
| irx scope=all (T00) | 0.149 avg | ~0.607 | ~0.750 | 0.497 | ~0.617 | kept |
| no_transfer K=1.0 scope=no_transfer | ~0.147 avg | ~0.72 | higher | ~0.568 | — | kept |
| **irx + no_transfer (this fire)** | **0.226** | **0.067** | 0.777 | 0.497 | 0.617 | **failed** |

---

## Decision

**No code changes.** Single trial sufficient to falsify — xi collapse to 0.067 is not
recoverable by tuning; it is a structural incompatibility between interference_relax and
the no_transfer scope. The interference_relax code in consolidation.rs is unchanged (T00
state: alpha_base=0.10, relax_steps=16). Empirical optimum unchanged:

    DRIVE_A=0.1  DRIVE_SCOPE=all  KURAMOTO_COUPLING=1.0  DRIVE_FREQ_HZ=2.0  DREAM_MODE=<unset>
    3-run avg fitness ≈ 0.138

---

## Implications

1. **interference_relax requires full-scope drive**: the constructive-pair mechanism has
   an implicit dependency on consistent amplitude modulation across ALL dreaming engines.
   Selectively excluding any engine from the drive corrupts the pairing geometry. Future
   interference_relax tuning should always use `DREAM_MODE=interference_relax DRIVE_SCOPE=all`.

2. **Stage_sync's independence advantage**: stage_sync (Kuramoto) doesn't depend on
   amplitude modulation — it operates purely on phase angles. This makes it more robust
   to scope variations and explains why K=1.0 achieves higher xi (0.86) than
   interference_relax (0.607) even though interference_relax builds more "magical" R.

3. **The remaining fitness gap**: the 0.138 optimum is dominated by transfer_score cost
   (~0.048) and carrier_emergence cost (~0.043). Neither has responded to any
   parameter sweep. Structural changes to how the dream chain builds transfer-relevant
   associations (not phase-level tuning) would be needed to push below ~0.130.

4. **Closed directions as of this fire**: K-sweep, drive frequency, relax_steps/alpha,
   interference_relax + no_transfer. The parameter space is well-explored. Future
   improvements likely require architectural changes to the consolidation pipeline.
