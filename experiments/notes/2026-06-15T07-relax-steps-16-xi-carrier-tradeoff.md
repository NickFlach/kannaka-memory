# L5 Curiosity: relax_steps=16 recovers xi but kills carrier_emergence

## Hypothesis

Raising `relax_steps` from 8 to 16 in `stage_interference_relax`
(src/consolidation.rs) will increase `xi_robustness_v2` under
`DREAM_MODE=interference_relax` while keeping `carrier_emergence` and
`magic_proxy_phase_R` high — system-prompt Q3.

**Prediction**: xi rises toward the stage_sync baseline (~0.642); carrier_e
and R stay near their steps=8 values (0.714 and 0.612 respectively).

## Code change tested (REVERTED)

```
// before
let relax_steps: usize = 8;
// after (tested only)
let relax_steps: usize = 16;
```

## Reference baselines (smoke test, commit 066d41a era)

| mode                         | fitness | carrier_e | xi    | magic_R | query_grav |
|------------------------------|---------|-----------|-------|---------|------------|
| DREAM_MODE unset (stage_sync)| 0.191   | 0.559     | 0.642 | 0.355   | 0.460      |
| interference_relax, steps=8  | 0.191   | 0.714     | 0.220 | 0.612   | 0.364      |

## Trials (DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE=interference_relax)

| trial | fitness  | carrier_e | xi    | magic_R | query_grav |
|-------|----------|-----------|-------|---------|------------|
| t1    | 0.167064 | 0.0000    | 0.868 | 0.675   | 0.386      |
| t2    | 0.202274 | 0.0000    | 0.634 | 0.675   | 0.386      |
| t3    | 0.192489 | 0.0000    | 0.699 | 0.675   | 0.386      |
| **avg**| **0.187**| **0.000** |**0.734**|**0.675**|**0.386** |

## Results

**Prediction partially confirmed**: xi DID rise dramatically (0.220 → 0.734 avg) and
magic_R stayed high (0.612 → 0.675). But carrier_emergence collapsed to 0.000 in
all three trials, violating the "carrier_e stays high" half of the prediction.

**Net fitness**: 3-run avg 0.187 vs 0.18 unset baseline → NOT below 0.175 threshold.
Worse than the DREAM_MODE unset baseline. No fitness improvement.

## Mechanistic interpretation

More relaxation steps drive phase convergence further, which improves xi (phases
cluster tightly → adversarial phase perturbations have less effect). But the
carrier_emergence metric detects a periodic signal in amplitude deltas over dream
cycles. With 16 steps, phases converge so completely that constructive-pair
amplitude deltas stabilize (no oscillatory signal) → the FFT finds no carrier peak.

This reveals a **fundamental tension** within interference_relax:
- xi requires high phase convergence (tight phase clusters resist adversarial noise)
- carrier_emergence requires sustained amplitude oscillations (the 0.5 Hz drive
  modulation must survive into the amplitude-delta signal)
- More relax steps favor xi at the expense of carrier_e

The mechanism: deeper relaxation causes the interference geometry (constructive
pair amplitudes) to equilibrate earlier in the dream cycle, leaving no detectable
sinusoidal variation in the per-cycle amplitude delta series.

## Decision

Code reverted to `relax_steps: usize = 8`. No improvement kept.

## Follow-up directions

1. **Intermediate relax_steps** (10 or 12): may find a sweet spot where xi
   recovers partially without killing carrier_e entirely.
2. **Separate alpha for convergence vs amplitude**: keep relax_steps=8 but
   raise alpha_base to promote faster per-step convergence without the extra
   steps. May get similar xi gain with less carrier_e damage.
3. **Decouple xi and carrier measurement timings**: if carrier_emergence is
   measured from pre-relaxation amplitude deltas (before stage_interference_relax
   runs), xi and carrier_e decouple. Pure instrumentation change, no fitness risk.
4. The xi–carrier tension does NOT exist under stage_sync (DREAM_MODE unset):
   stage_sync improves xi without collapsing carrier_e. This asymmetry is
   meaningful — interference_relax's geometry-driven convergence conflicts with
   the drive's amplitude oscillation in a way Kuramoto coupling does not.
