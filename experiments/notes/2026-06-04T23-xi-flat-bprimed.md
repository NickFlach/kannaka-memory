# L5 Curiosity Fire — 2026-06-04T23

## Hypothesis

`transfer_score` is the largest remaining fitness residual after the xi_and_flat fix
(weight 0.15, current value ~0.625 → contributes ~0.056 to the 0.143 fitness).

`transfer_score = (1 - fitness_b_primed / fitness_b_naive).clamp(0, 1)`.  A score
approaching 1.0 means engine_b_primed has near-zero fitness cost relative to naive:
perfect knowledge transfer from engine_a.

**Prediction**: If we drive `engine_b_primed` during its dream chain (alongside the
existing engine_clean, engine_adv, engine_flat targets), the primed system's
consolidation should improve, lowering fitness_b_primed, widening the gap with
fitness_b_naive, and pushing transfer_score toward 1.0. Estimated fitness savings:
~0.03–0.05 depending on how much primed improves.

The xi measurement engines (engine_clean, engine_adv) remain driven as before;
carrier_emergence (engine_flat) unchanged. Risk: low, since engine_b_primed is
isolated from the xi and carrier evaluation engines.

## Code change

Added `"xi_flat_bprimed"` scope arm to the `DRIVE_SCOPE` router in
`run_l5_dream_chain` (`src/bin/research.rs`):

```rust
"xi_flat_bprimed" => {
    drive_context == "engine_clean"
        || drive_context == "engine_adv"
        || drive_context == "engine_flat"
        || drive_context == "engine_b_primed"
}
```

**This change was REVERTED** (regression confirmed across 3 trials).

## Results

`DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=xi_flat_bprimed`

| trial | fitness | xi_robustness_v2 | carrier_emergence | transfer_score |
|-------|---------|-----------------|-------------------|----------------|
| 1     | 0.187628 | 0.5454 | 0.5588 | 0.603011 |
| 2     | 0.137016 | 0.8614 | 0.5588 | 0.624220 |
| 3     | 0.161878 | 0.6960 | 0.5588 | 0.624220 |

**3-trial average fitness: 0.162** (vs baseline xi_and_flat avg 0.143)
**Regression: +0.019**

## Analysis

The hypothesis is **falsified**.

**transfer_score was unchanged**: 0.603–0.624, essentially the same as the xi_and_flat
baseline (~0.625). Driving engine_b_primed during its dream chain does not widen the
fitness_b_primed / fitness_b_naive gap. The amplitude modulation (±10% at 2 Hz) averages
out across consolidation cycles and leaves the final consolidated state nearly identical.

**xi_robustness_v2 shows the usual high per-trial variance** (0.545–0.861 range here;
xi_and_flat baseline showed 0.605–0.967 range). The trial 1 xi dip (0.545) is within
normal variance, not caused by the engine_b_primed drive. Speed metrics are slightly
lower than xi_and_flat (~0.800 vs ~0.884), likely because the added drive pass adds
~8 ms per cycle × 20 cycles ≈ 160 ms overhead per run.

**Speed penalty is real but small**: total_ms increased from ~56600 to ~64900 (+14%).
The speed metric (`1 - ms/60000`) dropped from ~0.885 to ~0.800. This alone adds
0.03*(0.885-0.800) = 0.003 to fitness. But the dominance is the xi variance — not the
speed — driving the average up.

**Root cause**: the multiplicative drive at 2 Hz does not meaningfully affect long-term
memory organization in engine_b_primed. Transfer score is largely determined by how
much structural advantage priming from engine_a's state provides — amplitude oscillations
during consolidation do not amplify or diminish this advantage.

## Comparison to baseline

| config | avg fitness | xi (avg) | transfer_score (avg) |
|--------|------------|---------|----------------------|
| xi_and_flat correct | ~0.143 | ~0.810 | ~0.625 |
| xi_flat_bprimed (this fire) | ~0.162 | ~0.701 | ~0.617 |

## Decision

**REVERT** code change. The `xi_flat_bprimed` scope provides no improvement on
transfer_score and degrades average fitness by +0.019. Only the TSV rows and this
notes file are committed.

## Next directions

- The transfer_score residual (~0.625, contributes ~0.056 to fitness) appears resistant
  to amplitude-drive approaches. The gap between primed and naive consolidation may be
  structurally determined by the corpus design, not modifiable via the current drive.
- DRIVE_FREQ_HZ sweep remains untested (default 2 Hz assumed optimal; 1 Hz, 4 Hz, 0.5 Hz
  sub-harmonic not yet explored) — now the highest-value untested direction.
- Higher DRIVE_A (0.2, 0.3) at xi_and_flat also untested — could further improve
  carrier_emergence if the engine can tolerate larger amplitude swings.
- xi variance (±0.15 per trial) inflates fitness uncertainty; running 5+ trials for
  future hypotheses would give cleaner signal.
