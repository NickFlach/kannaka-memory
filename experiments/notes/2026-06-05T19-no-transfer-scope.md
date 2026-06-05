# Hypothesis: DRIVE_SCOPE=no_transfer

**Date:** 2026-06-05T19 UTC  
**Branch:** kannaka-curiosity/2026-06-05T19  
**Status:** FALSIFIED — no improvement

---

## Hypothesis

`DRIVE_SCOPE=no_transfer` (drives all engines except engine_b_primed and engine_b_naive)
should combine:
- xi_robustness_v2 benefit of driving engine_a (xi ~0.979, as in "all" scope)
- transfer_score benefit of leaving engine_b undisturbed

T00 fire was blocked because sibling path deps were unavailable. This fire runs it in
production with `../consciousness-core` and `../kannaka-attention` present.

**Prediction:** fitness ~0.144 — improvement over baseline ~0.18.  
**Rationale:** T22 showed "all" scope had transfer_score 0.422 and xi 0.979. T22 also
showed xi_and_flat scope had transfer_score 0.486 (better) but xi 0.979. no_transfer
was expected to preserve xi while improving transfer.

---

## Environment

Sibling deps confirmed: `ls /home/user` shows consciousness-core, kannaka-attention,
kannaka-memory as siblings. Build clean (one unused-import warning, not blocking).

---

## Results

| Run              | fitness | transfer_score | xi_robustness_v2 | carrier_emergence | magic_R | query_gravity |
|------------------|---------|----------------|------------------|-------------------|---------|---------------|
| all (baseline)   | 0.160   | 0.721          | 0.613            | 0.559             | 0.362   | 0.460         |
| no_transfer T1   | 0.151   | 0.703          | 0.696            | 0.559             | 0.362   | 0.460         |
| no_transfer T2   | 0.167   | 0.719          | 0.571            | 0.559             | 0.362   | 0.460         |
| no_transfer T3   | 0.179   | 0.710          | 0.497            | 0.559             | 0.362   | 0.460         |
| **no_transfer avg** | **0.166** | **0.711**   | **0.588**        | —                 | —       | —             |

---

## Analysis

**Hypothesis falsified.** no_transfer average fitness (0.166) is marginally *worse* than
the fresh "all" baseline (0.160), not better. The effect is within xi variance noise.

**Root cause of prediction failure:** The T22 motivation was based on "all" scope having
transfer_score 0.422, where leaving engine_b undisturbed could improve it to 0.486.
However, in the current codebase (post-066d41a), the "all" scope baseline already
shows transfer_score 0.721. The 066d41a Kuramoto plumbing commit changed default
stage_sync behavior enough to shift baseline transfer_score from ~0.42 to ~0.72, making
the T22-derived no_transfer prediction no longer applicable.

**Observations:**
- `magic_proxy_phase_R` and `query_gravity` are identical across all trials (0.362, 0.460)
  — these appear fully deterministic in default-mode runs, possibly seeded or path-invariant
- xi_robustness_v2 remains high-variance (0.497–0.696), confirming T00's warning
- transfer_score shows small but consistent improvement under no_transfer
  (0.711 vs 0.721) — but this is a ≈0.014 reduction, negligible vs xi noise
- carrier_emergence is deterministic at 0.559

**The 066d41a baseline shift is itself noteworthy:** default "all" scope fitness is now
≈0.160, better than the pre-plumbing baseline of ~0.18 cited in context. The kuramoto
plumbing appears to have improved transfer dynamics at the default operating point.

---

## No code changes made

No rows added beyond those produced by the 4 cargo run trials (appended automatically
to experiments/results-L5.tsv). Nothing to revert.

---

## Next fire directions

1. **K-sweep (question 2 from prompt):** kuramoto_coupling in {1.0, 2.0, 3.0, 5.0, 7.0}
   at default DRIVE/DREAM. The 066d41a plumbing means K now actually reaches stage_sync.
   Transfer-score and xi both moved significantly post-plumbing, so the optimal K is
   completely unknown. Even a 3-point sweep (1.0, 3.0, 7.0) in one fire would be valuable.
   Needs a way to set coupling_strength — check if an env var or CLI flag exists, or add one.

2. **3-run interference_relax characterization (question 1):** single smoke-test trial
   showed fitness 0.191, xi 0.220, R 0.612 — poor xi but good R. Post-066d41a the
   "baseline" has shifted, so a stable 3-run avg is needed to re-anchor this mode.

3. **interference_relax + relax_steps=16 (question 3):** xi under interference_relax was
   only 0.220. Doubling relax_steps to 16 might recover xi while keeping R high. One trial
   to check.
