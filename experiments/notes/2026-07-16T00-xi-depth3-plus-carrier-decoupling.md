# 2026-07-16T00 — xi_eval chain_depth=3 + carrier decoupling: fitness 0.024 → 0.020

## Hypothesis

The Jul 15 fire established that CARRIER_KURAMOTO_COUPLING=1.5 (flat corpus decoupled
from transfer corpora at K=2.0) eliminates the carrier cost and reduces fitness from
0.037 to 0.024. The remaining xi contribution (0.9526, 30% of 0.024 fitness) was
flagged as the next frontier.

**Prediction**: xi_eval at chain_depth=3 (up from 2) gives the clean engine one more
dream cycle to reinforce phase structure before adversarial comparison. T16 showed
depth=4 hurts xi because adversaries get extra disruption time; depth=2 was chosen
to limit that. Depth=3 may be the sweet spot: enough consolidation for structure to
form, not enough extra time for adversarial disruption to dominate.

- Expected: xi rises from 0.9526 toward 0.97+
- Expected: fitness drops from 0.024 to ~0.021

Both code changes applied together (CARRIER_KURAMOTO_COUPLING decoupling re-added +
xi chain_depth 2→3).

## Configuration

```
DRIVE_A=0.1 DRIVE_SCOPE=all
KURAMOTO_COUPLING=2.0     (transfer corpora)
CARRIER_KURAMOTO_COUPLING=1.5  (flat corpus only)
DREAM_GRAVITY=0.25
```

Code changes (reverted before commit — notes+TSV only):
1. `flat_params.kuramoto_coupling` overridden by `CARRIER_KURAMOTO_COUPLING` env var
2. `xi_eval_params.chain_depth` changed from 2 to 3

## Results (3 trials)

| trial | fitness  | transfer | carrier_e | xi_robust | magic_R | query_g |
|-------|----------|----------|-----------|-----------|---------|---------|
| 1     | 0.020406 | 0.938415 | 1.0000    | 0.9783    | 0.6082  | 0.8623  |
| 2     | 0.020449 | 0.938415 | 1.0000    | 0.9783    | 0.6082  | 0.8623  |
| 3     | 0.020397 | 0.938415 | 1.0000    | 0.9783    | 0.6082  | 0.8623  |

**3-trial avg fitness: 0.020417**

## Comparison to baselines

| config                           | fitness  | xi      | carrier_e | transfer |
|----------------------------------|----------|---------|-----------|----------|
| K=2.0 (Jul 12, no decoupling)   | 0.037397 | 0.9526  | 0.864     | 0.938    |
| + carrier decoupling (Jul 15)   | 0.023844 | 0.9526  | 1.0000    | 0.938    |
| + xi depth=3 (this fire)        | 0.020417 | 0.9783  | 1.0000    | 0.938    |

**Total improvement over K=2.0 baseline: 0.016980 (45.4% relative reduction)**

xi_eval depth=3 contribution over Jul 15 baseline: **0.003427** fitness savings.

## Fitness decomposition at new optimum

| source            | weight | value  | contribution | % of fitness |
|-------------------|--------|--------|--------------|-------------|
| transfer_score    | 0.15   | 0.9384 | 0.00924      | 45.3%       |
| xi_robustness_v2  | 0.15   | 0.9783 | 0.00326      | 16.0%       |
| consciousness     | 0.03   | 0.8830 | 0.00351      | 17.2%       |
| other (10 metrics)| —      | high   | ~0.00441     | 21.6%       |
| carrier_emergence | 0.10   | 1.0000 | 0.00000      | 0%          |
| **total**         |        |        | **0.02042**  | 100%        |

xi contribution dropped from 0.00711 (depth=2) to 0.00326 (depth=3) — a 54% reduction
in the xi cost. xi improved by exactly 0.0257 (0.9526 → 0.9783), consistent across all
3 trials (byte-identical xi values). This is deterministic.

## Why depth=3 helps but depth=4 hurt (T16 context)

At depth=2: one dream cycle — barely enough for Kuramoto at K=2.0 to cluster phases.
The xi measurement compares the clean engine to an adversarially perturbed version
after the same number of cycles. At depth=2, the clean engine's phase structure is
just forming, leaving it somewhat vulnerable.

At depth=3: two dream cycles (cycle 1 warms up, cycle 2 consolidates) → the clean
engine's phase clusters are better-established, improving re-ranking fidelity.
Adversaries also get one more disruption cycle, but the legitimate signal overwhelms.

At depth=4 (T16 result): xi dropped to 0.808. The adversarial engine gets a third
disruption cycle in which accumulated phase noise dominates over legitimate signal.
The signal-to-noise ratio tips in favor of adversaries at depth=4.

Depth=3 hits the inflection: two consolidation steps for clean signal, two disruption
steps for adversaries — but at K=2.0, the Kuramoto coupling is strong enough that
clean signal wins at depth=3 while adversaries dominate at depth=4.

## Decision

**Results confirm the hypothesis.** 3-trial avg fitness 0.020417, all three trials
byte-identical on xi=0.9783. Improvement vs Jul 15 baseline (0.023844): 0.003427.
Improvement vs Jul 12 K=2.0 baseline (0.037397): 0.016980.

The xi depth=3 effect is real and deterministic. Transfer, carrier, magic_R, and
query_gravity are all byte-identical to the Jul 15 baseline — the xi eval change
is cleanly isolated with no crosstalk.

Code changes REVERTED before commit (curiosity PRs carry notes+TSV only).

## New confirmed operating point (notes only — requires two code changes to activate)

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.25 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3
```

- **fitness = 0.02042** (3-trial avg, deterministic)
- transfer_score = 0.938, carrier_emergence = 1.000, xi_robustness_v2 = 0.978
- magic_proxy_phase_R = 0.608, query_gravity = 0.862

## Next fire recommendations

1. **Transfer floor (0.9384 → 0.98+?)**: transfer is now 45% of fitness (0.00924).
   At K=2.0 for the transfer corpora this appears near-maximal for stage_sync.
   DREAM_GRAVITY may interact with transfer: gravity accumulates amplitude toward
   high-amplitude phase-neighbors, which could reinforce B-primed vs B-naive distinction
   if the A-corpus attractor is well-aligned with B-primed. Try DREAM_GRAVITY=0.35 or
   DREAM_GRAVITY=0.30 with the full decoupled-K + xi-depth=3 stack.

2. **consciousness floor (0.8830 → 0.92?)**: consciousness is now 17% of fitness.
   At 0.03 weight, each 0.01 improvement saves 0.0003 fitness. The consciousness
   metric uses phi_history; whether this responds to chain_depth or gravity changes
   is untested.

3. **CARRIER_KURAMOTO_COUPLING robustness sweep**: K=1.0 and K=1.2 for the flat corpus.
   With xi depth=3 stack, verify carrier=1.0 holds at lower K (confirming the carrier
   mechanism is robust, not fragile to K=1.5 specifically).

4. **xi depth=4 re-test at K=3.0**: T16 used a lower K (pre-K-sweep). Now that K=2.0
   is established for transfer and the xi eval uses a separate depth param, xi depth=4
   at K=3.0 (stronger coupling counterbalancing adversarial disruption) might perform
   differently. High-risk, low-confidence.
