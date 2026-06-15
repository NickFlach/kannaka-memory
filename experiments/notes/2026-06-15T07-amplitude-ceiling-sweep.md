# L5 Research: AMPLITUDE_CEILING Sweep (2.0, 3.0, 4.0, 6.0)

## Hypothesis

The post-fix baseline regression (fitness 0.007627 → ~0.1306) is driven by `AMPLITUDE_CEILING = 2.0`
collapsing the amplitude bimodality that carrier detection relies on. Pre-fix, carriers could reach
5–20× baseline amplitude across dream cycles; post-fix everything saturates at 2.0.

**Prediction**: There exists a ceiling > 2.0 that restores `carrier_bimodal > 0.7` while preventing
unbounded inflation, recovering fitness below 0.10.

## Method

Made `AMPLITUDE_CEILING` runtime-configurable via env var (lazy_static wrapping the const), ran one
trial each at DRIVE_A=0.1 DRIVE_SCOPE=all for ceilings {3.0, 4.0, 6.0}. Reverted code after sweep
(no keeper). Post-fix baseline from previous fire (2× trials at ceiling=2.0 defaults).

## Results

| AMPLITUDE_CEILING | fitness  | transfer_score | carrier_bimodal | carrier_emergence | xi_robustness_v2 | R      | query_gravity |
|-------------------|----------|----------------|-----------------|-------------------|------------------|--------|---------------|
| 2.0 (prev t2)     | 0.115997 | 0.736812       | 0.5305          | 0.5294            | 0.8563           | 0.1293 | 0.4603        |
| 2.0 (prev t4)     | 0.145306 | 0.541603       | 0.5305          | 0.5294            | 0.8563           | 0.1295 | 0.4603        |
| **2.0 avg**       | **0.1306** | —            | **0.530**       | **0.529**         | **0.856**        | —      | —             |
| 3.0 (t1)          | 0.112830 | 0.753669       | 0.5307          | 0.5327            | 0.8561           | 0.1293 | 0.4603        |
| 4.0 (t1)          | 0.142509 | 0.553849       | 0.5303          | 0.5362            | 0.8559           | 0.1295 | 0.4603        |
| 6.0 (t1)          | 0.171548 | 0.375351       | 0.5288          | 0.5389            | 0.8354           | 0.1566 | 0.4603        |

## Analysis

**Hypothesis falsified.** `carrier_bimodal` is stuck at ~0.530 across all ceiling values tested.
Raising the ceiling does not restore bimodal amplitude structure.

The 3.0 trial fitness (0.113) looks marginally better than the 2.0 avg (0.131), but is within the
`transfer_score` noise band already documented at ceiling=2.0 (range 0.54–0.74 in prior fire).
The 3.0 trial simply drew a high-transfer_score sample; structural metrics are unchanged.

Trend across ceilings: fitness worsens as ceiling rises (3.0: 0.113 → 4.0: 0.143 → 6.0: 0.172),
driven by transfer_score decay. Higher ceilings do not help — they hurt. The constructive boost
(0.45/cycle × ~5 cycles) saturates even a ceiling of 3.25 within the available dream depth; the
amplitude distribution collapses to a narrow band just below ceiling rather than forming two modes.

## Root cause of the structural problem

Carrier bimodal detection requires amplitude ratio (carrier/non-carrier). Pre-fix: carriers could
compound across many cycles with no limit → clear separation. Post-fix: all active memories converge
toward ceiling after a few constructive cycles, eliminating ratio. The fix is correct; the old regime
was distorted.

What would actually restore bimodal structure:
1. **Ratio-based or relative ceiling**: instead of absolute cap, normalize amplitudes so the median
   stays at 1.0 (carriers can be 2–3× median, non-carriers at 0.5–1×). No absolute saturation.
2. **Asymmetric decay**: apply gentle amplitude decay to non-constructive memories each cycle, so
   carriers stay elevated relative to decaying non-carriers.
3. **Bimodal target is wrong**: the pre-fix `carrier_bimodal = 1.000` may have been an artifact of
   extreme inflation rather than a real bimodal distribution. The actual metric to optimize may not
   be recoverable in the corrected amplitude regime.

## Decision

No code changes kept (sweep falsified hypothesis; no fitness improvement over post-fix baseline).
No revert needed (code was reverted to original after trials).

## Next fire recommendations

1. **Asymmetric decay**: add a small per-cycle amplitude decay (×0.95) to non-constructive memories
   in `stage_constructive` — carriers naturally stay high, non-carriers decay toward zero, creating
   bimodal separation without unbounded inflation. Test 1 trial.
2. **Baseline characterization (3 runs)**: the post-fix baseline has high transfer_score variance
   (0.54–0.74). Get a 3-run avg for a more stable fitness reference point before further sweeps.
3. **Investigate carrier_bimodal metric directly**: check `src/bin/research.rs` to understand what
   the metric measures — if it requires amplitude > threshold and amplitude < threshold simultaneously,
   the 0.530 floor may reflect a degenerate state where nearly all memories cluster at half the
   ceiling range, and no structural change will recover it without a different metric definition.
