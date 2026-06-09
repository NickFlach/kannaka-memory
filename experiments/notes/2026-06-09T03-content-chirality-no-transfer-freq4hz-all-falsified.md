# Three hypotheses falsified: content-based chirality, no_transfer scope, 4 Hz drive

**Date:** 2026-06-09T03 UTC
**Branch:** kannaka-curiosity/2026-06-09T03-content-chirality
**Code changes:** REVERTED (content-based chirality in stage_chiral_perturbation; reverted after 1 trial)
**Status:** ALL FALSIFIED — optimum unchanged

---

## Orientation finding: repo context vs. local state

The injected system prompt describes 159 additional fire commits (from 2026-06-05 through
2026-06-08) not present in the local clone. Local master is at 2e7c162 ("consolidate
curiosity routine notes T23+T00"). The session auto-started on a branch that had those 159
commits, but `git checkout master` moved to the true local HEAD.

**Consequence**: Experiments were run against the 2e7c162 codebase, not the advanced state
the context describes. Key differences:
- DREAM_MODE=interference_relax exists in the code but has NOT been established as the optimum
  (that happened in fires after 2e7c162)
- Local empirical optimum: DRIVE_A=0.1 DRIVE_SCOPE=all DRIVE_FREQ_HZ=2.0 (default)
  avg fitness ≈ 0.112 (2 trials: 0.115, 0.110), xi ≈ 0.944, transfer ≈ 0.707

The 3 hypotheses tested below are against this local baseline.

---

## Hypothesis A: Content-based chirality in stage_chiral_perturbation

**Motivation**: The injected context describes adversarial UUID ordering as the root cause of
xi variance, with content-based chirality (XOR of cluster member UUIDs) as the correct fix.

**Change**: In `stage_chiral_perturbation`, replaced `cluster_idx % 2` handedness with:
```rust
let sig: u64 = cluster.memory_ids.iter()
    .map(|id| id.as_u128() as u64)
    .fold(0u64, |acc, x| acc ^ x);
cluster_handedness.insert(cluster_idx, if sig % 2 == 0 { 1.0 } else { -1.0 });
```

**Trial**: DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax (1 trial)

| metric | baseline (all, 2-trial avg) | trial 1 |
|--------|---------------------------|---------|
| fitness | 0.112 | 0.220 |
| transfer_score | 0.707 | **0.138** |
| carrier_emergence | 0.559 | 0.532 |
| xi_robustness_v2 | 0.944 | 0.770 |
| magic_proxy_phase_R | N/A | 0.708 |
| query_gravity | N/A | 0.424 |

**Decision**: REVERTED after 1 trial.

Transfer collapsed catastrophically (0.138 vs 0.707). xi improved slightly (0.770 vs 0.944)
but not enough to offset transfer loss. The alternating cluster chirality pattern (even=left,
odd=right) is load-bearing for transfer_score. Content-based XOR chirality disrupts the
systematic inter-cluster phase structure that enables knowledge transfer.

Note: this trial also used DREAM_MODE=interference_relax, which was not the local optimum
at 2e7c162. The combination of two non-baseline changes made it impossible to isolate the
chirality effect cleanly, but the transfer collapse was severe enough that the direction is
unambiguous.

---

## Hypothesis B: DRIVE_SCOPE=no_transfer

**Motivation**: Notes from 2026-06-05T00 (T00 fire) identified this as the primary next
hypothesis, blocked only by missing sibling deps. Sibling deps are now present.

`no_transfer` drives all engines EXCEPT engine_b_primed and engine_b_naive. Prediction
from T00: xi stays high (engine_a still driven), transfer_score improves (engine_b not
perturbed).

**No code changes**: scope was already implemented in src/bin/research.rs.
**Trials**: DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer (2 trials)

| metric | baseline (all, 2-trial avg) | no_transfer.t1 | no_transfer.t2 |
|--------|---------------------------|----------------|----------------|
| fitness | 0.112 | 0.199 | 0.135 |
| transfer_score | 0.707 | 0.703 | 0.710 |
| carrier_emergence | 0.559 | 0.559 | 0.559 |
| xi_robustness_v2 | 0.944 | **0.397** | **0.809** |
| magic_proxy_phase_R | N/A | 0.362 | 0.362 |
| query_gravity | N/A | 0.460 | 0.460 |

2-trial averages: fitness=0.167, transfer=0.706, xi=0.603

**Decision**: FALSIFIED. No code changes to revert (none made).

Transfer was unchanged from baseline (0.707 ≈ 0.706). The T00 prediction about transfer
improvement was wrong. More importantly, xi degraded significantly and became more variable
(0.397–0.809 avg ~0.603 vs baseline 0.928–0.961). Engine_b drive appears to STABILIZE xi
rather than hurt it — possibly because the amplitude landscape shaped by engine_b's dream
also influences the cluster formation in engine_clean and engine_adv during the xi test.

Magic_proxy_phase_R is deterministic at 0.362 under no_transfer (vs. baseline values not
characterized here). Query_gravity also deterministic at 0.460.

---

## Hypothesis C: DRIVE_FREQ_HZ=4.0 (harmonic resonance)

**Motivation**: 4 Hz is the first harmonic of the default 2 Hz drive. Research question 6
in the injected context (previously blocked by stub environment in T19). Hypothesis: a
higher-frequency drive might constructively interfere at harmonics of the carrier frequency.

**No code changes needed.** Prediction: carrier_emergence either improves (harmonic
amplification) or degrades (aliasing into non-carrier bands).

**Trial**: DRIVE_A=0.1 DRIVE_SCOPE=all DRIVE_FREQ_HZ=4.0 (1 trial)

| metric | baseline (2 Hz, 2-trial avg) | 4 Hz trial |
|--------|------------------------------|-----------|
| fitness | 0.112 | **0.259** |
| transfer_score | 0.707 | **0.347** |
| carrier_emergence | 0.559 | **0.315** |
| xi_robustness_v2 | 0.944 | **0.486** |

**Decision**: FALSIFIED. Destructive interference at 4 Hz, not harmonic resonance.

carrier_emergence collapsed from 0.559 to 0.315 — the 4 Hz drive pushes wave activity
outside the carrier emergence band detected by the FFT metric. Transfer and xi both
degraded. 4 Hz closed; by symmetry, DRIVE_FREQ_HZ=8 Hz and above are unlikely to help.

---

## Summary of open axes

| hypothesis | prediction | result | status |
|-----------|-----------|--------|--------|
| Content-based chirality | xi↑ (fix UUID variance) | transfer collapse | FALSIFIED |
| DRIVE_SCOPE=no_transfer | transfer↑, xi stable | transfer same, xi↓ | FALSIFIED |
| DRIVE_FREQ_HZ=4.0 | carrier resonance | all axes ↓ | FALSIFIED |

**Remaining untested (env-var only, no code changes):**
- DRIVE_FREQ_HZ=0.5 Hz (band edge, FFT bin 1 at n=16 cycles fs=8 Hz)
- DRIVE_FREQ_HZ=1.0 Hz (half default; T19 attempted in stubs, unreliable)
- DRIVE_FREQ_HZ=3.0 Hz (minor harmonic)
- DRIVE_SCOPE=xi_and_flat (properly implemented at 2e7c162; characterization at this
  code version incomplete since T21/T22 data predates L5.drive.A0.1 improvements)

**Code changes not yet explored:**
- Content-based chirality is too disruptive to try as a standalone change. The correct
  fix to xi variance at this code version may require making engine_b drive a prerequisite
  for xi stability (both together).
- stage_interference_relax with DRIVE_SCOPE=all (not tested cleanly — T01 only combined
  with content-based chirality, confounding the comparison)
- For future fires: retry DREAM_MODE=interference_relax DRIVE_SCOPE=all (baseline for this
  mode at 2e7c162 not established — T01 was confounded by code change)

**Empirical optimum confirmed unchanged:**
DRIVE_A=0.1 DRIVE_SCOPE=all DRIVE_FREQ_HZ=2.0 DREAM_MODE= (unset)
avg fitness ≈ 0.112 (2-trial avg: 0.115 + 0.110)
xi ≈ 0.944, transfer ≈ 0.707, carrier_e ≈ 0.559
