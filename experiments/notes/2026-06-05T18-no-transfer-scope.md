# L5 Curiosity Fire — 2026-06-05T18

## Hypothesis

T00 attempted `DRIVE_SCOPE=no_transfer` but was blocked by missing sibling deps
(`consciousness-core`, `kannaka-attention`). Sibling deps are now present. No code
changes required — the scope arm is already implemented in `run_l5_dream_chain`.

**Prediction** (from T00): "no_transfer" drives engine_a, engine_clean, engine_adv,
and engine_flat, but NOT engine_b_primed or engine_b_naive. Expected to combine:
- High xi_robustness_v2 from driving engine_a (T21 showed engine_a drive boosts xi)
- High transfer_score from leaving engine_b unperturbed (T22 showed xi_and_flat
  improves transfer vs "all" by not driving engine_b)

T00 estimated fitness ≈ 0.144.

## Command

```
RESEARCH_RUN="hyp-no_transfer.A0.1-tN" DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 \
  DRIVE_SCOPE=no_transfer \
  cargo run --release --quiet --bin research -- --level 5
```

## Results

| run | fitness | transfer_score | xi_robustness_v2 | carrier_emergence | magic_R | query_gravity |
|-----|---------|---------------|-----------------|-------------------|---------|---------------|
| no_transfer t1 | 0.160037 | 0.702644 | 0.6551 | 0.5588 | 0.3623 | 0.4597 |
| no_transfer t2 | 0.121374 | 0.718530 | 0.8960 | 0.5588 | 0.3623 | 0.4597 |
| no_transfer t3 | 0.179109 | 0.702644 | 0.5270 | 0.5588 | 0.3623 | 0.4597 |
| **avg**        | **0.153507** | **0.707939** | **0.6927** | **0.5588** | **0.3623** | **0.4597** |

Reference — xi_and_flat post-066a (1 trial):

| run | fitness | transfer_score | xi_robustness_v2 | carrier_emergence | magic_R | query_gravity |
|-----|---------|---------------|-----------------|-------------------|---------|---------------|
| xi_and_flat ref | 0.148249 | 0.644837 | 0.7723 | 0.5588 | 0.4224 | 0.4459 |

## Analysis

### Transfer_score surge from 066a Kuramoto plumbing

The most striking finding: `transfer_score` jumped dramatically compared to T22
pre-066a values for both scopes.

| config | epoch | transfer_score |
|--------|-------|---------------|
| xi_and_flat (T22, pre-066a) | ~2026-06-04T22 | 0.486 |
| xi_and_flat (this fire, post-066a) | 2026-06-05T18 | 0.645 |
| no_transfer (this fire, post-066a) | 2026-06-05T18 | 0.703–0.719 |

Commit 066a plumbed Kuramoto params through `stage_sync` (previously hard-coded
constants silently ignored params). With K=3.0 actually reaching stage_sync,
within-category phase coherence strengthens during the dream, which helps
engine_a memories consolidate into tighter clusters. Those tighter clusters then
transfer structure more effectively to engine_b_primed — raising transfer_score.

This explains why *every* post-066a transfer_score is substantially higher than
pre-066a, regardless of DRIVE_SCOPE.

### no_transfer vs xi_and_flat

no_transfer's avg transfer_score (0.708) is higher than xi_and_flat's (0.645),
consistent with the prediction: driving engine_a (vs xi_and_flat which skips it)
adds a second consolidation pass that further tightens cluster structure, while
still protecting engine_b.

However, no_transfer's avg xi (0.693) is lower than xi_and_flat's (0.772).
The fitness difference is small: no_transfer avg 0.154 vs xi_and_flat 0.148
(1 trial, high xi-variance means this comparison is uncertain).

Net: no_transfer and xi_and_flat are **within variance** of each other. Both beat
the 0.18 baseline comfortably.

### xi variance is the dominant noise source

xi_robustness_v2 ranges 0.527–0.896 across 3 no_transfer trials. This ±0.18
variance inflates single-trial fitness uncertainty by up to 0.027. Reliably
distinguishing configurations requires 5+ trials.

### magic_proxy_phase_R and query_gravity

Both metrics are deterministic within a scope (no variance across trials):
- no_transfer: R=0.362, gravity=0.460
- xi_and_flat: R=0.422, gravity=0.446

xi_and_flat has higher magic_R (0.422 vs 0.362), consistent with driving
engine_flat and engine_xi during consolidation introducing more non-linear phase
structure. query_gravity is similar between scopes (~0.45–0.46), slightly above 0.5
threshold not yet reached under either scope.

## Comparison to baseline

| config | avg fitness | transfer_score | xi (avg) | notes |
|--------|------------|---------------|---------|-------|
| all (0.18 baseline) | ~0.18 | ~0.422 | ~0.642 | pre-066a smoke test |
| xi_and_flat pre-066a (T22) | ~0.159 | ~0.486 | ~0.850 | 3 trials |
| xi_and_flat post-066a (this) | 0.148 | 0.645 | 0.772 | 1 trial ref |
| no_transfer post-066a (this) | 0.154 | 0.708 | 0.693 | 3 trials |

## Decision

**No code changes made** — nothing to revert. TSV rows appended automatically.

no_transfer avg fitness 0.154 is an improvement vs 0.18 baseline (Δ=0.026,
well above the 0.005 threshold). No code change is needed to use it; it's already
in the codebase as `DRIVE_SCOPE=no_transfer`.

The post-066a Kuramoto plumbing is a major confound: transfer_score increased for
both scopes, making all pre-066a comparisons approximate. The true empirical optimum
post-066a should be established with 3-trial averages for both xi_and_flat and
no_transfer.

## Next directions

1. **3-trial xi_and_flat post-066a baseline** — this fire's ref is 1 trial; xi variance
   is ±0.15 per trial, so the 0.148 single-trial result needs confirmation. Run 3
   trials at DRIVE_SCOPE=xi_and_flat to establish a reliable post-066a reference.

2. **K-sweep (Q2 from system prompt)** — now that K actually reaches stage_sync,
   sweeping KURAMOTO_COUPLING ∈ {1.0, 5.0, 7.0} at DRIVE_SCOPE=no_transfer (or all)
   would test whether higher K further boosts transfer_score. Needs a small code
   change to read KURAMOTO_COUPLING env var in the L5 local overrides block.

3. **DRIVE_FREQ_HZ=4.0 Hz** (T23 "highest-value untested direction") — env-var only,
   no code change needed.
