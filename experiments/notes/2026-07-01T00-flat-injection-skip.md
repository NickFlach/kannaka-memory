# 2026-07-01T00 — Carrier floor diagnosis: cycle-0 spike is the structural limit

## Hypothesis

carrier_emergence = 0.5265 (83.5% of fitness) is held near 0.5 by a DFT artifact in the
4-point spectral measurement. The engine_flat runs with chain_depth=4, giving n=4
amplitude_delta samples at 8 Hz (cycle_period_s=0.125). DFT bins: only 2 Hz (k=1) and
4 Hz (k=2). carrier = max(P2, P4) / (P2 + P4).

Prior hypothesis: injection at cycle 2 adds 10 memories at amplitude 0.8; at cycle 3
their consolidation inflates amp_delta and splits power 50/50 between k=1 and k=2.

**Fix tested**: skip injections for engine_flat (check DRIVE_CONTEXT=="engine_flat").

**Prediction**: without injection artifact at cycle 3, delta sequence becomes monotonic
(drive at 0.5 Hz provides gradual upward ramp over 4 cycles); k=1 dominates; carrier
→ ~0.67; fitness improvement ≈ 0.014.

## Configuration

```
DRIVE_A=0.1 DRIVE_SCOPE=all DRIVE_TOP_FRAC=1.0 DREAM_MODE=interference_relax DREAM_GRAVITY=0.25
```

Code change: `is_flat_engine` guard around injection block in `run_l5_dream_chain`.

## Result (1 trial)

| metric           | baseline (3-run avg) | trial 1 (injection skip) |
|------------------|----------------------|--------------------------|
| fitness          | 0.056686             | 0.056885 (+0.000199)     |
| carrier_emergence| 0.5265               | 0.5235 (−0.003)          |
| transfer_score   | 0.9652               | 0.9652 (unchanged)       |
| xi_robustness    | 0.9796               | 0.9796 (unchanged)       |
| magic_R          | 0.8670               | 0.8670 (unchanged)       |
| query_gravity    | 0.8623               | 0.8623 (unchanged)       |

amp_deltas_flat diagnostic output:
```
[4.1693864, 0.18309976, 0.002769197, 0.01027902]
```

## Analysis

### Hypothesis falsified — injection is not the cause

The diagnostic reveals the true structure: cycle-0 amplitude delta is 4.17, which is
23× larger than cycle-1 (0.183) and 1500× larger than cycles 2-3 (≈0.003-0.010).

The first dream cycle of engine_flat sees ALL memories at 0.1 Hz (uniform-frequency
flat corpus). Constructive_boost=0.45 and destructive_penalty=0.35 create massive
interference because virtually every memory pair is near-frequency. Result: huge
amplitude reorganization in cycle 0. By cycle 1, the system is near equilibrium.

DFT of [4.17, 0.18, 0.003, 0.010]:
- k=1 (2 Hz): |(4.17 - 0.003) + i(-0.18 + 0.010)|² ≈ 4.167² + 0.170² ≈ 17.39
- k=2 (4 Hz): |4.17 - 0.18 + 0.003 - 0.010|² ≈ 3.983² ≈ 15.86
- carrier = 17.39 / 33.25 ≈ 0.523

The spike at index 0 dominates both DFT bins nearly equally (spike → equal power at
all harmonics). The carrier_emergence ≈ 0.52-0.53 reflects that k=1 wins slightly over
k=2 because the DC trend lifts k=1 slightly. Injections at cycle 2 barely matter —
they add at most 10× ~0.15 change to cycle-3 delta which is already ~0.01, a small
perturbation on top of a 23× larger cycle-0 spike.

### Why skip-injection slightly hurt

Without injection, cycle 3 is 0.010 (even smaller than with injection). This slightly
reduces the already-tiny "ramp" shape in later cycles, hurting k=1 marginally. Net
carrier: 0.5235 vs 0.5265 — a small regression.

### Structural floor confirmed

The 0.527 carrier_emergence floor is determined by:
1. First-cycle reorganization spike: ~4.17 in mean |amplitude delta|
2. Rapid equilibration: subsequent deltas drop to ≤0.18 by cycle 1
3. 4-point DFT: single-spike sequence at index 0 → power splits ~52/48 between 2 Hz and 4 Hz

This is NOT addressable by env-var tuning or injection-skipping. The floor is
structural to the flat-corpus design: uniform 0.1 Hz frequencies create maximal
consolidation disruption on first contact.

## Paths to breaking the 0.527 floor (for future fires)

1. **Exclude cycle 0 from DFT**: use only amp_deltas[1:] (skip the initialization burst).
   With [0.18, 0.003, 0.010] (3 points), bins would be at 0, 8/3=2.67, 8*2/3=5.33 Hz.
   The 2.67 Hz bin falls in [0.5, 4.0] Hz band; 5.33 Hz does not.
   This is a measurement redesign (changes research.rs L5 code path).

2. **Drive at 2 Hz**: use DRIVE_FREQ_HZ=2.0 so the drive signal falls at DFT bin k=1
   (2 Hz). With 4 samples at 8 Hz, 2 Hz fits perfectly. But 2 Hz was tested
   (2026-06-25 notes) and gave SAME carrier as 0.5 Hz — because consolidation still
   dominates cycle 0 by 23×.

3. **Warmup cycles before measurement**: run the flat engine for 1 or 2 cycles without
   measuring, then run 4 measurement cycles. This lets the corpus equilibrate before
   the DFT window. Requires a "skip_carrier_cycles" param in engine_flat's call.

4. **Per-memory spectral tracking**: instead of DFT of mean delta, track individual
   memory amplitudes over time and measure their frequency content. Drive at 0.5 Hz
   would show up per-memory even if masked by consolidation in the aggregate.

## Decision

**Code reverted.** No improvement — trial 1 fitness regressed by +0.000199, carrier
dropped by 0.003, and the structural floor cause is now understood.

**No code changes kept.**

## New structural knowledge: the cycle-0 spike

The carrier_emergence ≈ 0.527 is structurally determined by:
- DFT[0] = 4.17 (first-cycle reorganization)
- DFT[1-3] ≈ 0.01-0.18 (near-equilibrium)
- 4-point DFT of single-spike sequence ≈ 50/50 power split

This is not improvable by injection timing, drive frequency, or DREAM_GRAVITY changes
(all tested previously). The 0.527 floor requires redesigning either the flat corpus
initialization strategy or the carrier measurement itself.

## Next fire recommendation

The amp_deltas_flat diagnostic should be added as a permanent logged output (it is
already printed via `println!("amp_deltas_flat: {:?}")` in the current research.rs
but not in the TSV). Future autoresearch fires targeting carrier must address the
warmup-cycle approach (option 3 above) as the most promising structural fix.

L5 env-var optimization remains at floor fitness: **0.056686** (unchanged).
