# Hypothesis: DRIVE_SCOPE=no_transfer improves fitness over "all"

**Date:** 2026-06-05T10 UTC  
**Branch:** kannaka-curiosity/2026-06-05T10  
**Status:** NEGATIVE — no improvement

---

## Hypothesis

T00 was blocked by missing sibling deps. Now that sibling deps are present,
test `DRIVE_SCOPE=no_transfer` (implemented in research.rs lines 3195–3198).

"no_transfer" drives all engines EXCEPT engine_b_primed and engine_b_naive.
Prediction from T00: leaving engine_b undriven would protect transfer_score
(raise it toward ~0.486, as seen when xi_and_flat excluded engine_b in T22),
while engine_a remains driven (preserving xi_robustness_v2 ~0.979 seen with "all").
Expected fitness: ≤ 0.128 (>0.005 improvement over "all" historical avg ~0.113).

## Configuration

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer
DREAM_MODE=<unset>
```

No code changes.

---

## Results

| Trial | fitness  | transfer | xi_robustness_v2 | carrier_e | magic_R | query_grav |
|-------|----------|----------|------------------|-----------|---------|------------|
| 1     | 0.161401 | 0.709696 | 0.6451           | 0.5588    | 0.3623  | 0.4597     |
| 2     | 0.127856 | 0.709696 | 0.8685           | 0.5588    | 0.3623  | 0.4597     |
| 3     | 0.110317 | 0.702644 | 0.9947           | 0.5588    | 0.3623  | 0.4597     |

**3-run avg fitness: 0.133**

---

## Reference: DRIVE_SCOPE=all (historical L5.drive.A0.1 rows)

| Trial | fitness  | transfer | xi_robustness_v2 |
|-------|----------|----------|------------------|
| 1     | 0.115263 | 0.706831 | 0.9275           |
| 2     | 0.110411 | 0.706831 | 0.9609           |

Historical "all" 2-run avg: **0.113**

---

## Analysis

The core prediction failed: transfer_score under no_transfer is ~0.707, not ~0.486.
It is indistinguishable from "all" scope. This means driving vs. not driving
engine_b has no effect on the primed/naive fitness ratio (transfer_score).

The T22 xi_and_flat "ref-all" showed transfer 0.422, while current "all" runs show
0.707. Something changed in the code between T22 and the L5.drive.A0.1 runs that
significantly raised transfer_score for the "all" scope — likely the xi_and_flat
scope implementation in commit 141c0c0 (which re-structures the drive block in
research.rs and may have changed how engine_b chains run).

With transfer_score equal across scopes, no_transfer offers no benefit and actually
shows higher variance in xi_robustness_v2 (0.645–0.995 range vs. 0.928–0.961 for
"all"). This variance causes the no_transfer avg (0.133) to be worse than "all"
avg (0.113).

**Decision:** No improvement. No code changes to revert. No rows manually added
(binary appends automatically).

---

## Next fire directions

1. **3-run "all" reference**: Historical "all" only has 2 trials. A proper 3-run
   avg would confirm the 0.113 baseline under current code.

2. **K-sweep**: Now that Kuramoto plumbing is fixed (commit 066d41a), sweep
   `kuramoto_coupling` ∈ {1.0, 2.0, 3.0, 5.0, 7.0} with DRIVE_A=0.1 DRIVE_SCOPE=all.
   R and xi were not measured in K-sweeps before; now both log. The magic↔xi
   correlation hypothesis (question 4) is testable.

3. **interference_relax + relax_steps**: The smoke test showed interference_relax
   drops xi to 0.220 (vs 0.642 baseline). Raising relax_steps from 8 to 16 or 24
   may recover xi while keeping carrier_e 0.714 and magic_R 0.612 high.
   Requires a code change to consolidation.rs.
