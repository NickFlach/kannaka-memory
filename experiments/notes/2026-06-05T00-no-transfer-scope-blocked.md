# Hypothesis: DRIVE_SCOPE=no_transfer — blocked by environment

**Date:** 2026-06-05T00 UTC  
**Branch:** kannaka-curiosity/2026-06-05T00  
**Status:** BLOCKED — stub environment; results would not be comparable to production

---

## Hypothesis

`DRIVE_SCOPE=no_transfer` is already implemented in `run_l5_dream_chain`
(lines 3095–3098 of `src/bin/research.rs`):

```rust
"no_transfer" => {
    drive_context != "engine_b_primed"
        && drive_context != "engine_b_naive"
}
```

This drives all engines EXCEPT engine_b_primed and engine_b_naive. Compared to
"all" scope, engine_b is undisturbed during its dream chain. Compared to
"xi_and_flat" scope, engine_a IS driven (which T21/T22 showed helps xi_robustness_v2).

**Prediction**: "no_transfer" combines the xi_robustness advantage of driving engine_a
(xi_robustness_v2 ~0.979, as in "all") with the transfer_score advantage of leaving
engine_b unperturbed (transfer_score ~0.486, as in xi_and_flat). Expected fitness:
0.154 − 0.010 ≈ 0.144 (improvement over single-trial "all" ref 0.154 from T22).

**Rationale**: T22 showed that NOT driving engine_b improves transfer_score (0.422 →
0.486, deterministic). T21 showed that NOT driving engine_a hurts xi_robustness_v2.
"no_transfer" avoids the engine_a exclusion while still protecting engine_b.

---

## Environment block

Sibling crates `consciousness-core` and `kannaka-attention` are path dependencies
(`../consciousness-core`, `../kannaka-attention`) and are not checked out in this
remote execution environment. Compilation fails:

```
error: failed to read `/home/user/consciousness-core/Cargo.toml`
```

Creating stub crates (as done in T19) is not viable for this hypothesis because:

1. `eval_consciousness` (called inside `eval_l5_placeholder_fitness`) delegates to
   `ConsciousnessBridge`, which uses `consciousness_core::iit::ConsciousnessLevel`.
   A stub phi computation produces different phi values.

2. `transfer_score = fitness_b_primed / fitness_b_naive`, where each fitness
   includes the consciousness term (weight 0.10). Stub phi errors propagate directly
   into transfer_score.

3. T21 stub vs T22 production confirmed the scope effect on transfer_score is
   **direction-reversed** in stub mode: stub showed all > xi_and_flat in
   transfer_score (0.721 vs 0.645), while production showed the opposite (0.422 vs
   0.486). Any "no_transfer" stub result would be similarly unreliable.

---

## No trials run; no code changes made

No rows added to `experiments/results-L5.tsv`. Nothing to revert.

---

## Next fire directions

1. **Primary**: Run this same `DRIVE_SCOPE=no_transfer` hypothesis in a session where
   `../consciousness-core` and `../kannaka-attention` exist as siblings. No code
   changes required — the scope is already implemented. Command:
   ```
   DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer \
     cargo run --release --quiet --bin research -- --level 5 2>&1 \
     | grep -E '^fitness:|^transfer_score:|^carrier_emergence:|^carrier_bimodal:|^xi_robustness_v2:'
   ```
   Run 3 trials for no_transfer and 3 trials for "all" (1-trial "all" ref from T22
   is insufficient for reliable comparison given ±0.3 xi_robustness_v2 variance).

2. **Secondary**: DRIVE_FREQ_HZ=4.0 Hz (harmonic of default 2 Hz, within [0.5, 4.0]
   Hz carrier emergence band, never tested in production). Also env-var only, no code
   changes.

3. **Secondary**: DRIVE_FREQ_HZ=0.5 Hz (minimum boundary of carrier emergence band —
   whether it scores near-zero or full depends on FFT bin placement at n=16 cycles,
   fs=8 Hz, bin spacing 0.5 Hz → 0.5 Hz is exactly bin 1).
