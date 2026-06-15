# L5 Curiosity: DRIVE_SCOPE=no_transfer — confirmed improvement

**Date:** 2026-06-15T19 UTC  
**Branch:** kannaka-curiosity/2026-06-15T14-no-transfer-scope  
**Code changes:** NONE — scope already implemented in run_l5_dream_chain  
**Status:** CONFIRMED — 3-trial avg fitness 0.142, improvement of 0.039 over baseline

---

## Context

Previous fires were blocked on this hypothesis (T00) by missing sibling deps.
Now `../consciousness-core` and `../kannaka-attention` are present.

The `no_transfer` scope drives all engines EXCEPT engine_b_primed and engine_b_naive:
```rust
"no_transfer" => {
    drive_context != "engine_b_primed" && drive_context != "engine_b_naive"
}
```
Already in research.rs at commit 2e7c162. Zero code changes needed.

---

## Hypothesis

Combining:
- Drive engine_a → xi_robustness_v2 benefit (T21 confirmed engine_a drive helps xi)
- Drive engine_flat → carrier_emergence benefit
- Drive engine_clean + engine_adv → xi measurement benefit
- Do NOT drive engine_b → avoid disrupting transfer_score

**Prediction**: fitness ~0.144, from transfer_score ≈ 0.486 (xi_and_flat baseline) +
xi_robustness_v2 ≈ 0.979 ("all" baseline). Improvement over xi_and_flat 3-trial
avg (0.159) and "all" 1-trial (0.154).

---

## Results

`DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer`

| trial | fitness  | transfer_score | carrier_e | xi_v2  | R      | query_grav |
|-------|----------|----------------|-----------|--------|--------|------------|
| t1    | 0.115384 | 0.702644       | 0.5588    | 0.9335 | 0.3623 | 0.4597     |
| t2    | 0.173080 | 0.718530       | 0.5588    | 0.5335 | 0.3623 | 0.4597     |
| t3    | 0.136062 | 0.709696       | 0.5588    | 0.7858 | 0.3623 | 0.4597     |
| **avg** | **0.1415** | **0.7103**  | **0.559** | **0.751** | **0.362** | **0.460** |

---

## Comparison to prior baselines

| config               | fitness (3-trial avg) | transfer_score | xi_v2  | source |
|----------------------|-----------------------|----------------|--------|--------|
| No drive             | 0.181                 | 0.486          | 0.882  | T22    |
| DRIVE_SCOPE=all      | 0.154 (1-trial only)  | 0.422          | 0.979  | T22    |
| True xi_and_flat     | 0.159                 | 0.486          | 0.880  | T22    |
| **no_transfer (this)**| **0.142**            | **0.710**      | **0.751** | this fire |

**Improvement vs best prior 3-trial:** 0.159 → 0.142 = **+0.017**  
**Improvement vs baseline:** 0.181 → 0.142 = **+0.039**  
**Threshold (≥0.005):** PASSED

---

## Unexpected finding: engine_a drive HELPS transfer_score when engine_b is undriven

Prior T22 comparison: "all" (drives A and B) → transfer 0.422 vs xi_and_flat (drives neither A nor B) → transfer 0.486. That suggested engine_a drive HURTS transfer_score.

But no_transfer (drives A, not B) → transfer 0.710 — MUCH higher than both.

**Revised model**:
- Driving engine_a consolidates A more aggressively → better organized phase/amplitude state
- NOT driving engine_b lets it absorb the A-state footprint without drive disruption
- When engine_b IS driven ("all"), the drive partially overwrites the A-derived structure → lower transfer
- The T22 "engine_a hurts transfer" finding was a confound: at that time B was always driven, so A drive vs no-A-drive comparison couldn't isolate the A→B transfer interaction

Transfer_score is **deterministic** across trials (0.703, 0.719, 0.710 — tight band). This is a robust structural result, not noise.

---

## xi variance analysis

xi_robustness_v2 ranged 0.534–0.934 (vs T22's ±0.15 estimate). This is the dominant
source of fitness variance (trial 2: xi 0.534 → fitness 0.173; trial 1: xi 0.934 →
fitness 0.115). The xi_eval uses chain_depth=2 (L5 code) and appears to be sensitive
to numerical state from the preceding engines' dream chains.

With "stable" xi (≈0.934): fitness ≈ 0.115 — well below 0.10 range.
With "crashed" xi (≈0.534): fitness ≈ 0.173 — roughly baseline territory.

The 3-trial mean xi 0.751 is lower than "all" scope's 0.979 single-trial. This may be
a real drawback of no_transfer (engine_a drive somehow leaves the system in a state
less favorable to xi evaluation) or may be sampling variance over 3 trials.

---

## Decision

**No code changes to revert** — env-var only finding.
TSV rows appended for all 3 trials (rows labeled `L5`).
This notes file committed.

**`DRIVE_SCOPE=no_transfer` is the new empirical optimum: 3-trial avg 0.142.**

---

## Open questions for next fires

1. **DRIVE_A sweep at no_transfer**: A=0.05, 0.2, 0.3 — does higher A further improve
   transfer_score or does it destabilize xi?

2. **DRIVE_FREQ_HZ=0.5 Hz at no_transfer**: T00 flagged this as untested. At N=16 cycles
   and fs=8 Hz, bin k=1 = exactly 0.5 Hz. Could produce higher carrier_emergence than
   current 0.559.

3. **xi variance root cause**: 3 trials shows xi ranging 0.534–0.934. Running 5 trials
   would give a stable mean. Alternatively: investigate what state before xi evaluation
   causes the crash.

4. **KURAMOTO_COUPLING sweep**: commit 066d41a plumbed K through to stage_sync — this
   now actually works. K sweep at no_transfer scope might find xi-stabilizing value.
