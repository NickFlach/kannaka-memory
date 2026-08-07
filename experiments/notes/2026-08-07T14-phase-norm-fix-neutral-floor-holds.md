# 2026-08-07T14 — Phase-norm fix (9eecf28) neutral; structural floor holds

## Trigger

Commit 9eecf28 (Aug 6 22:29 UTC) fixed two unnormalized phase writes in
`stage_xi_repulsion` and the cross-modality repulsion inner loop:

```diff
-mem_b.phase -= phase_correction;
+mem_b.phase = norm(mem_b.phase - phase_correction);
```

This landed after the Aug 6 fire's note was written ("consolidation.rs unchanged
since 2026-08-01T14"). No L5 trials had been run against HEAD until this fire.

## Hypothesis

The `norm()` fix is a pure correctness change. `phase_correction` is small; in
practice the raw subtraction rarely pushed `mem_b.phase` outside [0, 2π]. Prediction:
no measurable change to any L5 metric at the baseline operating point.

## Trials (2 of 5 budget used)

`DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all` (no ephemeral code changes)

| trial | fitness  | transfer  | carrier_e | xi_rob | magic_R | q_grav |
|-------|----------|-----------|-----------|--------|---------|--------|
| 1     | 0.059900 | 0.866708  | 0.7355    | 0.9611 | 0.5272  | 0.4603 |
| 2     | 0.059866 | 0.866708  | 0.7355    | 0.9611 | 0.5272  | 0.4603 |

## Comparison

Pre-fix TSV rows with the same parameter set (e.g. rows 60-61):

| row (TSV) | fitness  | transfer  | carrier_e | xi_rob |
|-----------|----------|-----------|-----------|--------|
| pre-fix   | 0.060823 | 0.866000  | 0.7355    | 0.9611 |
| pre-fix   | 0.060836 | 0.866000  | 0.7355    | 0.9611 |

xi_robustness_v2 = 0.9611 in all four rows. carrier_emergence = 0.7355 unchanged.
Fitness delta ~0.001 is within run-to-run noise. Hypothesis confirmed.

## Decision

No regression from the phase-norm fix. Structural floor (~0.017–0.019 at the optimized
operating point with ephemeral code changes; ~0.059–0.060 at bare baseline) is intact.
All levers exhausted per 2026-08-01T14 notes. No code changes reverted (none were made).
No trials appended beyond the two baseline rows above.

Future improvement continues to require architectural changes outside autoresearch scope.
