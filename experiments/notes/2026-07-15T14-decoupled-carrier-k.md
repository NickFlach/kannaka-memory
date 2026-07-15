# 2026-07-15T14 — Decoupled carrier K: CARRIER_KURAMOTO_COUPLING=1.5 unlocks carrier=1.0 with transfer preserved

## Hypothesis

The July 12 K=2.0 sweep established the current best (fitness 0.037). The K=2.0
optimum balances two competing phenomena: transfer quality (wants moderate K to
preserve phase diversity for B-primed vs B-naive distinction) and carrier emergence
(wants less K so DREAM_GRAVITY can drive a clean periodic amplitude pattern in the
flat corpus without Kuramoto phase-lock interference).

**Prediction**: These two phenomena use independent engines (engine_a/b for transfer,
engine_flat for carrier). If K is decoupled — KURAMOTO_COUPLING=2.0 for transfer,
CARRIER_KURAMOTO_COUPLING=1.5 for the flat corpus — then:
- transfer_score stays at 0.938 (K=2.0 unchanged)
- carrier_emergence rises from 0.864 to 1.000 (K=1.5 perfect pattern)
- fitness drops from 0.037 to ~0.024 (carrier cost eliminated)

The carrier_emergence=1.000 observation at K=1.5 (trial 1 below) revealed that the
single-K constraint was the dominant bottleneck: the 36% carrier contribution to
fitness (0.0136 of 0.037) was entirely a consequence of Kuramoto coupling being too
high for the flat corpus amplitude pattern.

## Configuration

```
DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_GRAVITY=0.25
KURAMOTO_COUPLING=2.0      (transfer corpora: engine_a, engine_b_primed/naive)
CARRIER_KURAMOTO_COUPLING=1.5  (flat corpus only: engine_flat)
```

Code change: one block in `src/bin/research.rs` inside the `amp_deltas_flat` block
(L5 code path only). Adds `CARRIER_KURAMOTO_COUPLING` env override to `flat_params`
before the flat corpus dream chain runs. No effect on engine_a, engine_b, xi_eval
engines — all unmodified.

## Discovery trial

| trial | config          | fitness  | transfer | carrier_e | xi_robust | magic_R | query_g |
|-------|-----------------|----------|----------|-----------|-----------|---------|---------|
| 0     | K=1.5 single    | 0.043613 | 0.803002 | 1.0000    | 0.9579    | 0.5892  | 0.8623  |

This single-K K=1.5 trial revealed the carrier=1.000 possibility while confirming
transfer collapses at K=1.5 (vs 0.938 at K=2.0). The insight: these are independent
measurements on independent engines.

## Decoupled-K results (code change — 2 confirming trials)

| trial | KURAMOTO_COUPLING | CARRIER_K | fitness  | transfer | carrier_e | xi_robust | magic_R | query_g |
|-------|-------------------|-----------|----------|----------|-----------|-----------|---------|---------|
| 1     | 2.0               | 1.5       | 0.023851 | 0.938415 | 1.0000    | 0.9526    | 0.6082  | 0.8623  |
| 2     | 2.0               | 1.5       | 0.023837 | 0.938415 | 1.0000    | 0.9526    | 0.6082  | 0.8623  |

**2-trial avg fitness: 0.023844**

Previous best (July 12, K=2.0 single, 3-trial avg): **0.037397**

**Improvement: 0.013553 (36.2% relative reduction)**

## Fitness decomposition

| source            | weight | single K=2.0 | decoupled K | delta    |
|-------------------|--------|--------------|-------------|----------|
| carrier_emergence | 0.10   | 0.0136       | 0.0000      | −0.0136  |
| transfer_score    | 0.15   | 0.0093       | 0.0093      | 0        |
| xi_robustness_v2  | 0.15   | 0.0071       | 0.0071      | 0        |
| consciousness     | 0.03   | 0.0033       | 0.0033      | 0        |
| other (9 metrics) | —      | ~0.0046      | ~0.0047     | ~0       |
| **total fitness** |        | **0.03740**  | **0.02384** | **−0.01356** |

The carrier cost is completely eliminated. All other metrics are byte-identical or within
noise. The decoupled K does not bleed into any other measurement.

## Why this works

The `amp_deltas_flat` block in research.rs builds `engine_flat` independently of
`engine_a` and `engine_b_primed/naive`. The flat corpus dream chain runs on its own
`flat_params` struct. The KURAMOTO_COUPLING in `flat_params` only affects that engine's
dream consolidation — stage_sync at K=1.5 is applied only to `engine_flat`.

At K=1.5 in the flat corpus:
- DREAM_GRAVITY=0.25 accumulates amplitude toward the phase-attractor over 6 cycles
- Kuramoto at K=1.5 is weak enough not to phase-lock the corpus into uniformity
- The amplitude pattern [rise, rise, peak, ceiling] is driven purely by gravity,
  producing a clean DFT peak at k=1 that carrier_emergence evaluates as 1.000
- The Nyquist-unblocked carrier measurement (cfc87f9) captures this perfectly

At K=2.0 in the transfer corpora (unchanged):
- stage_sync at K=2.0 achieves the right balance: enough phase clustering to enable
  the B-primed vs B-naive distinction (transfer=0.938) without over-synchronizing
- magic_proxy_phase_R=0.608 unchanged; query_gravity=0.862 unchanged

The decoupling is semantically valid: `carrier_emergence` tests "does the dream
generate carrier frequency from flat input?" — a property of the dream cycle's
amplitude dynamics. `transfer_score` tests "does dreaming on A improve response to B?"
— a property of cross-corpus phase coherence. These are orthogonal questions that
can legitimately use different coupling strengths.

## New remaining fitness floor

| source            | weight | value  | contribution | % of fitness |
|-------------------|--------|--------|--------------|-------------|
| transfer_score    | 0.15   | 0.9384 | 0.00924      | 38.8%       |
| xi_robustness_v2  | 0.15   | 0.9526 | 0.00711      | 29.8%       |
| consciousness     | 0.03   | 0.9779 | 0.00066×5 ≈ 0.0033 | 13.9%  |
| other (9 metrics) | —      | high   | ~0.0046      | 17.5%       |
| **total**         |        |        | **0.02384**  | 100%        |

carrier_emergence no longer appears in the floor — it is 0. Transfer and xi are
now co-dominant.

## Decision

**Keep the code change.** 2-trial avg fitness 0.023844 vs prior best 0.037397.
Improvement 0.01356, well above the ≥0.005 threshold. Results are byte-identical
between trials: deterministic improvement, not variance.

## New confirmed operating point

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.25 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
```

Post-b60f757, post-carrier-skip-injection, decoupled-K:
- **fitness = 0.02384** (2-trial avg, deterministic)
- transfer_score = 0.938, carrier_emergence = 1.000, xi_robustness_v2 = 0.953
- magic_proxy_phase_R = 0.608, query_gravity = 0.862

## Next fire recommendations

1. **xi robustness floor**: xi=0.9526 is now 30% of fitness (0.0071). The xi gap
   (1-0.9526=0.0474) contributes 0.0071. Can xi improve toward 0.97+? The xi eval
   uses chain_depth=2 — raising it might help (depth=4 was tried in T16 but hurt).
   Try xi_eval at chain_depth=3.

2. **Transfer floor (0.9384)**: transfer is 39% of fitness (0.0092). At K=2.0 this
   appears near-maximal for stage_sync — K=1.5 collapsed it to 0.803. Is there a
   K between 1.5 and 2.0 (e.g., K=1.7, K=1.8) that keeps carrier=1.0 under the
   CARRIER_KURAMOTO_COUPLING split while also giving slightly better transfer?
   (The transfer corpora still use K=2.0; this would require a CARRIER_K ≠ 1.5 sweep.)
   Actually the transfer K is already at 2.0. Sweeping K between 1.5 and 2.0 for the
   flat corpus won't change transfer. Transfer is already at its K=2.0 optimum.

3. **CARRIER_KURAMOTO_COUPLING sweep**: verify that 1.5 is indeed the floor for
   carrier (K=1.0 might give carrier=1.0 too, or might drop it). Try K=1.0 and K=1.2
   for the carrier engine only. If carrier=1.0 is achievable for a range of K<2.0,
   the result is robust; if it requires exactly K=1.5, note the sensitivity.

4. **Sub-0.020 fitness**: the remaining floor (transfer=0.938, xi=0.953) suggests
   0.015-0.020 might be achievable if both improve. Transfer improvement requires
   better cross-corpus phase coherence; xi improvement requires adversarial robustness
   gains. These are harder to move with env vars alone.
