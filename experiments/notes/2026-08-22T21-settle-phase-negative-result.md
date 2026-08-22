# Settle phase: NEGATIVE RESULT — damped coupling is not relaxation

Hypothesis: `kannaka-crystal`'s dream ends with `engine.resonate(50)`, a settle pass "so the
reshaped field relaxes into a self-consistent state before anyone probes it", and kannaka-memory
has no equivalent — it assesses and flushes immediately after Phase 3's dt=0.3 callosal coupling
kick. Since cluster structure was observed melting and recrystallizing (O3 18→4→3→**21**→8→6;
O2 witness 25→6→**52**→42) with phi peaking exactly at the recrystallization, a cooling schedule
should let the field land on the crystallized state instead of being frozen mid-melt.

## Method

Paired run, identical 654-memory seed snapshot, `KANNAKA_CONSOLIDATE=dryrun` in both arms so no
merge/decay mutation confounds the comparison. Only difference: `KANNAKA_DREAM_SETTLE_STEPS`.
Settle = re-apply `callosal_kuramoto` with geometrically decaying dt (0.180 → 0.0050, cooling 0.6).

## Result — the hypothesis is REFUTED

| metric | seed | control (0) | settled (8) | delta |
|---|---|---|---|---|
| phi | 0.27944 | **0.41908** | 0.39612 | **−0.023** |
| mean_order | 0.51402 | **0.49556** | 0.47498 | **−0.021** |
| num_clusters | 20 | **19** | 14 | **−5** |
| largest_cluster | 138 | 275 | 187 | −88 |
| clusters.mean_order | 0.87278 | 0.86536 | 0.86959 | +0.004 |
| skip_links | 0 | 8232 | 8766 | +534 |

Worse on every headline metric. Clusters moved AWAY from recrystallization (19 → 14).

## Why the analogy failed

Crystal's `resonate(50)` is **free evolution of a wave field under its own dynamics, with no
forcing**. My settle re-applies the *coupling operator* at decreasing strength — that is not
relaxation, it is more forcing, just gentler. Eight additional nudges toward phase alignment
merged clusters together rather than letting structure re-form.

**kannaka-memory has no true free-evolution step to borrow into.** The honest analogue is
`apply_dynamics` without coupling, not damped `callosal_kuramoto`. That is a different prototype.

## Caveats (this is n=1)

- One run per arm. Enough to rule out a large positive effect; **not** enough to rule out a small one.
- **`num_clusters` is not deterministic on load**: both arms read the SAME file and reported 20 vs
  24 clusters *before* dreaming. That instability is comparable to the effect size being chased, so
  the cluster deltas above are weak evidence in either direction. Any retry needs the 10-run
  treatment.
- Settled arm ran ~3 min longer (~10 min vs ~7 min on 654 memories).

## Disposition

`KANNAKA_DREAM_SETTLE_STEPS` defaults to 0 and the code is inert unless set, so nothing needs
reverting. Kept as a documented negative result rather than merged.

## Unrelated trap found while building the harness

**`[hrm].path` in `config.toml` silently OVERRIDES `KANNAKA_DATA_DIR`.** Copying a production
`config.toml` into a scratch directory to test "on a copy" points the run straight back at the LIVE
substrate. This caused two unintended dreams against a live HRM before it was caught (no
corruption; health clean). Nodes with `path = ""` (e.g. O1) are unaffected.

Any isolated-substrate harness must neutralise that key AND assert the loaded memory count differs
from production before doing anything mutating.
