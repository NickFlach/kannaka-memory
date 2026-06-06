# Hypothesis: DRIVE_SCOPE=no_transfer in production (T00 deferred)

**Date:** 2026-06-06T01 UTC  
**Branch:** kannaka-curiosity/2026-06-06T01  
**Status:** CONFIRMED — improvement ≥ 0.005 vs baseline, keeping new optimum

---

## Background

T00 identified `DRIVE_SCOPE=no_transfer` as the primary next hypothesis but was
blocked by missing sibling deps. Sibling layout is confirmed available this fire:
`/home/user/consciousness-core` and `/home/user/kannaka-attention` exist at the
expected sibling paths. No stubs required.

The scope was already implemented (research.rs:3195-3198):

```rust
"no_transfer" => {
    drive_context != "engine_b_primed"
        && drive_context != "engine_b_naive"
}
```

Drives all engines EXCEPT engine_b_primed and engine_b_naive, which are the two
engines whose ratio defines `transfer_score`.

---

## Hypothesis

`DRIVE_SCOPE=no_transfer` combines:
- engine_a IS driven → xi_robustness_v2 stays high (as in "all" scope, ~0.85)
- engine_b NOT driven → transfer_score improves (engine_b consolidates undisturbed)

T00 predicted fitness ≈ 0.144. Baseline with `DRIVE_SCOPE=all` is ~0.18.
Improvement threshold: 0.005.

---

## Trials

All three trials: `DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer`  
Commit: `066d41a` (Kuramoto-plumbed + DREAM_MODE added). DREAM_MODE unset.  
Note: RESEARCH_RUN was not set; TSV rows are the last three "L5" rows in results-L5.tsv.

| Trial | fitness   | transfer_score | xi_robustness_v2 | carrier_emergence | magic_proxy_phase_R | query_gravity |
|-------|-----------|----------------|------------------|-------------------|---------------------|---------------|
| T1    | 0.126431  | 0.725206       | 0.8532           | 0.5588            | 0.3623              | 0.4597        |
| T2    | 0.191494  | 0.718530       | 0.4289           | 0.5588            | 0.3623              | 0.4597        |
| T3    | 0.129123  | 0.709696       | 0.8508           | 0.5588            | 0.3623              | 0.4597        |
| **avg** | **0.149** | **0.718**    | **0.711**        | 0.5588            | 0.3623              | 0.4597        |

---

## Results vs baseline

Baseline (DRIVE_SCOPE=all, DRIVE_A=0.1, 3-run avg): ~0.180

| Metric              | Baseline (all) | no_transfer avg | Delta     |
|---------------------|----------------|-----------------|-----------|
| fitness             | ~0.180         | 0.149           | −0.031 ✓  |
| transfer_score      | ~0.62–0.64     | 0.718           | +0.08 ✓   |
| xi_robustness_v2    | ~0.64          | 0.711 (bimodal) | +0.07 ✓   |
| carrier_emergence   | 0.5588         | 0.5588          | 0         |
| magic_proxy_phase_R | ~0.355         | 0.3623          | ≈0        |
| query_gravity       | ~0.460         | 0.4597          | ≈0        |

Transfer score improved consistently across all three trials (~0.71, vs ~0.62 baseline).
xi_robustness_v2 is bimodal: 2/3 trials high (~0.85), 1/3 low (~0.43) — same variance
pattern as "all" scope, but the high-xi floor appears similar. The low-xi trial (T2)
still achieves fitness 0.191, matching baseline rather than exceeding it, so "all" of
the improvement comes from transfer_score.

magic_proxy_phase_R and query_gravity are constant across all three trials (0.3623,
0.4597) — these metrics appear deterministic or nearly so for this configuration.

---

## Decision

**KEEP.** 3-run avg 0.149 is −0.031 vs baseline, well above the 0.005 threshold.
No code changes made; the improvement is purely from env-var scope selection.

New empirical optimum: `DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer
DREAM_MODE=` (unset). 3-run avg fitness ≈ 0.149.

---

## Next fire directions

1. **K-sweep under no_transfer**: kuramoto_coupling {1.0, 2.0, 3.0, 5.0, 7.0} at
   DRIVE_SCOPE=no_transfer. Now that K reaches stage_sync (066d41a) and no_transfer
   is the new optimum, explore whether coupling tuning recovers the xi_robustness_v2
   bimodality (reduces the ~1/3 low-xi tail).

2. **DRIVE_FREQ_HZ variants**: 1, 4, 0.5 Hz with DRIVE_SCOPE=no_transfer. T19 ran
   these in stub mode (unreliable). Production results now available.

3. **interference_relax + no_transfer**: DREAM_MODE=interference_relax with
   DRIVE_SCOPE=no_transfer. Smoke test showed interference_relax raises carrier_e
   (0.559 → 0.714) and magic_R (0.355 → 0.612) but costs xi (0.642 → 0.220) at
   DRIVE_SCOPE=all. Under no_transfer, with engine_b undisturbed, the xi cost may
   be smaller while the transfer_score gain is preserved.

4. **3-run interference_relax characterization**: The scope prompt lists this as
   priority 1. With no_transfer as the new baseline, a fair comparison requires
   DREAM_MODE=interference_relax DRIVE_SCOPE=no_transfer (not DRIVE_SCOPE=all as
   in the existing smoke test).
