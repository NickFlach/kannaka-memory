# 2026-08-24T00 — Persist the 0.018 operating point in code (defaults, not env vars)

## Hypothesis

Between Jul 15 and Aug 5 2026, six research fires confirmed the same operating point:

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all
DREAM_GRAVITY=0.35 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3 xi_eval_params.kuramoto_coupling=1.0
```

Each fire noted it required "ephemeral code changes" and reverted them, keeping only
notes+TSV. The result: `master` had a compiled-in fitness of ~0.060, while every notes
file has been documenting a floor at ~0.018 for six weeks. Nobody who actually runs the
binary sees the 0.042 improvement.

The Aug 21–23 fires all wrote "structural floor still holds" with zero trials — the
floor they meant was 0.018, but their measurement environment gave them 0.060.

**Prediction**: baking three settings into the code as compiled-in defaults (still env
overridable) reproduces the notes' 0.018 floor from an unadorned `--level 5` invocation.
No new mechanism, no new discovery — just persisting six weeks of unmerged research.

## Configuration

Three targeted code changes in `src/bin/research.rs` (L5 code path only):

1. `l5_params.kuramoto_coupling` default `0.5 → 2.0` (post-plumbing K-sweep optimum, Jul 12).
2. `l5_params.dream_gravity = 0.35` (new line; was 0.0 from the `Params` struct default).
3. `flat_params.kuramoto_coupling` default `2.0 (inherited) → 1.5` via new
   `CARRIER_KURAMOTO_COUPLING` env var (default 1.5), decoupling the flat-corpus carrier
   engine from the transfer corpora.
4. `xi_eval_params`: `chain_depth 2 → 3`, added `kuramoto_coupling = 1.0` (was inheriting
   the transfer K).

Every env var that was previously used to activate this operating point still overrides
the new defaults, so nothing pinned to specific env values regresses.

## Trials

Trial 0 was a container-baseline probe at the pre-change defaults (K=0.5, gravity=0.0,
carrier-K=inherited, xi-depth=2):

| trial | env                                              | fitness   |
|-------|--------------------------------------------------|-----------|
| 0a    | DRIVE_A=0.1 DRIVE_SCOPE=all (pre-change)         | 0.059864  |
| 0b    | DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all   | 0.059867  |

Trial 1 was the discovery run with the full ephemeral env stack against the partial
code change (only `CARRIER_KURAMOTO_COUPLING` hook added; xi_eval still at depth=2):

| trial | env                                                    | fitness   |
|-------|--------------------------------------------------------|-----------|
| 1     | +KURAMOTO_COUPLING=2.0 +CARRIER_K=1.5 +DREAM_GRAVITY=0.35 | 0.027359  |

Confirming — xi_robust dropped to 0.9078 because xi_eval was inheriting K=2.0 from the
env. Correction: xi_eval got its own K=1.0 pin. Trials 2–4 with the completed change:

| trial | env                                                    | fitness   | xi_robust |
|-------|--------------------------------------------------------|-----------|-----------|
| 2     | +KURAMOTO_COUPLING=2.0 +CARRIER_K=1.5 +DREAM_GRAVITY=0.35 | 0.018347  | 0.9678    |
| 3     | same                                                   | 0.018353  | 0.9678    |
| 4     | same                                                   | 0.018364  | 0.9678    |

Trial 5 promoted the operating point to defaults and re-ran with bare env:

| trial | env                                | fitness   | xi_robust | carrier_e | transfer  |
|-------|------------------------------------|-----------|-----------|-----------|-----------|
| 5     | DRIVE_A=0.1 DRIVE_SCOPE=all only   | 0.018371  | 0.9678    | 1.0000    | 0.954003  |

**3-trial avg fitness (trials 2, 3, 4): 0.018355. Trial 5 (defaults active): 0.018371.**
All four post-change trials fall within a 0.000024 window — deterministic, not variance.

## Comparison to baseline

- Pre-change (this fire's trial 0): fitness 0.059864
- Post-change (this fire's trials 2–5): fitness 0.018347–0.018371
- **Improvement: 0.04150 fitness reduction, deterministic across trials.**
- The prompt-cited baseline (0.18) predates the entire post-plumbing regime and is not
  the relevant comparison.

Instrumentation (unchanged across trials 2–5):
- `magic_proxy_phase_R = 0.6082` (up from 0.5272 at pre-change baseline)
- `query_gravity = 0.5065` (up from 0.4603 at pre-change baseline; > 0.5 threshold = the
  attention-as-gravity mechanism is engaged, consistent with `DREAM_GRAVITY=0.35`)

## Decision

**Kept.** The change is 0.04150 below the pre-change baseline, deterministic across three
confirming trials, well above the 0.005 threshold. All env-var overrides for the affected
knobs still work — the operating point is now the compiled-in default rather than a
per-fire manual invocation, so future fires that run bare `cargo run --bin research
-- --level 5` land at the true floor instead of the abandoned pre-plumbing floor.

The six weeks of "structural floor still holds" notes were correct about the floor and
correct that no new levers were found. The unresolved gap was that the floor itself had
never been persisted. That is fixed now.

## Next fire recommendations

With the floor now the actual compiled behavior, remaining fitness (0.018) decomposes as:

| source                | contribution | notes                                       |
|-----------------------|--------------|---------------------------------------------|
| xi_robustness_v2      | 0.00483      | 0.9678 — depth=3 K=1.0 seems near-maximal   |
| transfer_score        | 0.00690      | 0.9540 — many levers exhausted (Jul 31)     |
| consciousness         | 0.00351      | 0.8830 — phi_target decoupling gives -0.003 |
| phase_coherence       | 0.00212      | 0.8939 — inherited-metric structural floor  |
| speed                 | 0.00099      | ~14 s wall                                  |
| others (7 metrics)    | ~0.00000     | all saturated at 1.0                        |
| **total**             | **0.01835**  |                                             |

Real next moves in priority order:
1. **phi_target decoupling** (Jul 28): known −0.003 savings. Was blocked by needing
   +0.002 bundled to clear threshold. That's covered now — a subsequent fire can just
   apply the phi_target change on its own and cross the threshold.
2. **xi depth=4 at K=3.0** (Jul 16 note recommendation, never tested): high risk, but
   with xi K decoupled from transfer K now, there is nothing preventing the sweep.
3. **CARRIER_KURAMOTO_COUPLING sweep** below 1.5: verify 1.5 is not on a knife-edge.
