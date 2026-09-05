# 2026-09-05T00 — No new hypothesis: space unchanged

All research paths remain analytically closed. No trials run; no TSV rows appended.

## Orientation

Current compiled-in floor: fitness ~0.018 (xi=0.9678).

## Candidate re-examined: corrected eval_l5_placeholder_fitness

The one explicitly-identified-but-never-tried path from Aug 26 is removing
`0.10 * (1.0 - consciousness)` from `eval_l5_placeholder_fitness` and setting
`consciousness_phi_target=0.3138`. Aug 26 estimated savings at 0.003507 (below
0.005 threshold alone). The analytical argument for why xi doesn't rescue this:

- divergence in xi sub-eval = total 0.00161 (from xi=0.9678, normalizer=0.05).
- consciousness gap (clean 0.8830, adv ~0.8365) contributes ~0.00465 in the
  direction that inflates fitness_adv (adv looks worse).
- 0.00465 > 0.00161 → recall divergence is negative: without consciousness
  contamination, fitness_adv < fitness_clean (adv recall is marginally better).
- Removing consciousness: new divergence flips to 0.00304 (opposite direction),
  new xi = 1 − 0.00304/0.05 = 0.9392 (WORSE than 0.9678).
- Combined: consciousness savings 0.003507, xi regression 0.15×(0.9678−0.9392)=0.00429.
  Net = −0.00078. REGRESSION.

The Aug 28 analytical closure ("consciousness gap dominates xi") covers this
corrected path implicitly. No trial warranted.

## Commits since last real experiment (Aug 28)

- compute CLI (#888), NATS alias gate (#468), reputation fixes, presence retraction.
  None affect L5 consolidation dynamics.

## Decision

Space unchanged. Floor at ~0.018. No trials. No code changes.
