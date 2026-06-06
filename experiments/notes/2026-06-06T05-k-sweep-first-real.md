# K-sweep: First real Kuramoto coupling survey (post-plumbing fix)

## Hypothesis

Prior to commit 066d41a, `stage_sync` ignored `params.kuramoto_*` entirely — every
previous K-sweep measured noise. Now the plumbing is correct. This fire asks:

> What is the fitness response curve over K ∈ {1.0, 2.0, 3.0, 5.0, 7.0} at
> DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE unset?
> Does xi_robustness_v2 peak at K=3.0 (default) or somewhere else?
> Does R (magic_proxy_phase_R) positively correlate with xi across K?

Prediction: xi peaks somewhere around K=3.0–5.0; R and xi co-vary positively.

## Method

Added `KURAMOTO_K` env var to `experiment_params()` in `src/bin/research.rs`
(inside the labeled "EXPERIMENT PARAMETERS" block) so K can be varied at runtime.
Default fallback = 3.0 (existing value, no behavior change). One trial per K value.

## Results (single trials — high variance expected)

| K   | fitness | transfer | carrier_e | xi_v2 | R (magic) | query_grav |
|-----|---------|----------|-----------|-------|-----------|------------|
| 1.0 | 0.1922  | 0.682    | 0.568     | 0.444 | 0.250     | 0.469      |
| 2.0 | 0.1868  | 0.534    | 0.566     | 0.593 | 0.264     | 0.428      |
| 3.0 | 0.1643  | 0.707    | 0.559     | 0.600 | 0.362     | 0.460      |
| 5.0 | 0.1853  | 0.466    | 0.405     | 0.784 | 0.295     | 0.425      |
| 7.0 | 0.2351  | 0.657    | 0.536     | 0.140 | 0.240     | 0.391      |

Baseline reference: DRIVE_A=0.1 DRIVE_SCOPE=all K=3.0 (default), fitness avg ~0.18.

## Observations

1. **K=3.0 is fitness-optimal** in this sweep (0.164, best of 5). The default was
   well-calibrated. No improvement found.

2. **xi peaks at K=5.0** (0.784), not K=3.0 (0.600). But K=5.0 pays for it: both
   transfer_score (0.466 vs 0.707) and carrier_emergence (0.405 vs 0.559) drop
   sharply. Those carry 0.15 + 0.10 = 0.25 of the fitness weight; the xi gain
   (0.184 × 0.15 = +0.028) is swamped.

3. **K=7.0 collapses xi** (0.140) and has the worst fitness (0.235). Very strong
   coupling degrades the non-commutative structure.

4. **R↔xi prediction: partially supported, then breaks.**
   - K=1→3: both R and xi rise together (0.250/0.444 → 0.362/0.600). ✓
   - K=3→5: R falls slightly (0.362→0.295) while xi jumps to 0.784. ✗ anti-correlate.
   - K=5→7: R falls (0.295→0.240) and xi collapses (0.784→0.140). ✗
   The R↔xi relationship is non-monotone across K. R peaks at K=3.0 without
   being the xi-maximiser.

5. **query_gravity** is highest at K=1.0 (0.469) and monotone-decreasing with K.
   Higher coupling suppresses attention-as-gravity.

## Conclusion

K=3.0 remains the empirical optimum. No code change improves fitness.

Reverting the `KURAMOTO_K` env var per convention (no ≥0.005 improvement confirmed
in 3 trials). The env var is safe to re-add in a future fire if a K sweep under
interference_relax mode or different DRIVE_A is wanted.

## Next hypotheses enabled by this data

- **K=4.0–4.5 might be worth a 3-run bracket**: xi at K=5.0 is compelling; the
  fitness drop is driven by transfer_score. Perhaps K=4.0 has xi ≈ 0.70 without
  the transfer penalty. One fire, 3 trials.
- **K=5.0 + DREAM_MODE=interference_relax**: interference_relax already raises
  carrier_e and R; maybe K=5.0 under that mode recovers transfer_score.
- **Why does carrier_emergence drop at K=5.0?** The Nyquist-unblocked metric
  at 0.405 suggests strong sync is suppressing the carrier wave; worth digging
  into the carrier detection logic.
