# 2026-07-13T00 — K=1.5 falsified; DREAM_GRAVITY carrier-invariant under stage_sync K=2.0

## Hypotheses tested

**H1 (K=1.5)**: The K-landscape shifted from K=3→K=2 post-b60f757. Could the minimum
shift further to K<2.0? Test K=1.5 to probe downward.

**H2 (DREAM_GRAVITY sweep)**: Under stage_sync K=2.0, the carrier_emergence=0.864 is
the dominant fitness cost (36%). Does varying DREAM_GRAVITY from 0.25 affect carrier
by changing amplitude-differentiation strength? Test DREAM_GRAVITY=0.35 (upward) and
DREAM_GRAVITY=0.20 (downward, checking for V-shape repro under stage_sync).

## Configuration

```
DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE= (unset)
KURAMOTO_COUPLING= swept (1.5 in H1) or 2.0 (H2)
DREAM_GRAVITY= swept (0.20, 0.25 baseline, 0.35)
```

Baseline (July 12 confirmed, 3-trial avg):
- K=2.0, DREAM_GRAVITY=0.25: **fitness=0.037397**, transfer=0.938, carrier=0.864, xi=0.953, query_gravity=0.862

## Results

| trial | K   | DREAM_GRAVITY | fitness  | transfer | xi_robust | carrier_e | magic_R | query_g |
|-------|-----|---------------|----------|----------|-----------|-----------|---------|---------|
| 1     | 1.5 | 0.25          | 0.042772 | 0.803002 | 0.9579    | 1.0000    | 0.5892  | 0.8623  |
| 2     | 2.0 | 0.35          | 0.036675 | 0.938415 | 0.9526    | 0.8639    | 0.6082  | 0.8962  |
| 3     | 2.0 | 0.20          | 0.036675 | 0.938415 | 0.9526    | 0.8639    | 0.6082  | 0.8367  |

## Analysis

### H1: K=1.5 is worse than K=2.0

K=1.5 gives fitness=0.042772 — 14% worse than K=2.0 (0.037397). Transfer collapses
from 0.938 to 0.803 and carrier hits the ceiling (1.0). At K=1.5, Kuramoto
synchronization is insufficient: phases don't consolidate enough for engine_b_primed
to differentiate primed from naive responses. The under-synchronized phases produce
near-uniform amplitude in the flat corpus DFT (carrier=1.0 = maximum loss) and lose
the transfer signal.

Post-b60f757 K landscape (confirmed):

| K   | fitness  | transfer | carrier_e | xi    |
|-----|----------|----------|-----------|-------|
| 1.5 | 0.042772 | 0.803    | 1.000     | 0.958 |
| 2.0 | 0.037397 | 0.938    | 0.864     | 0.953 |
| 3.0 | 0.060830 | 0.866    | 0.735     | 0.961 |
| 4.0 | 0.043527 | 0.814    | 0.982     | 0.954 |

**K=2.0 is a clear minimum.** The landscape is V-shaped (approximately) with K=2.0 at
the bottom: K<2.0 under-synchronizes (carrier ceilings, transfer collapses), K>2.0
over-synchronizes (carrier drops, transfer degrades). This is consistent with the
b60f757 cosine_similarity fix reducing effective pair density — K=2.0 achieves
the same "right amount" of synchronization that K=3.0 achieved pre-fix.

### H2: carrier_emergence is gravity-invariant under stage_sync K=2.0

DREAM_GRAVITY=0.20 and 0.35 both produce carrier_emergence=0.8639 — identical to
DREAM_GRAVITY=0.25. transfer_score=0.938415 and xi_robustness=0.9526 are also
unchanged. The carrier DFT under stage_sync K=2.0 is locked at 0.864 regardless of
gravity strength in the 0.20-0.35 range.

**Mechanism**: The carrier metric measures the k=1 dominance of the flat-corpus
amplitude-delta pattern across dream cycles 2-5. Under stage_sync K=2.0 with
DREAM_GRAVITY≥0.20, the pattern is [rise, rise, rise, collapse]: cycles 2-4 show
monotonically increasing mean |delta| as gravity accumulates, then cycle 5 collapses
because gravity-aligned memories hit the AMPLITUDE_CEILING=2.0 and get capped back.

Different DREAM_GRAVITY values change the rate of rise (steeper with higher gravity)
but NOT the shape: the ceiling-cap collapse at cycle 5 is inevitable once memories
reach 2.0. The DFT k=1 power is determined by the [rise, collapse] shape, which is
ceiling-bounded, not gravity-bounded. Hence carrier_emergence is locked at 0.864.

**The June 27 V-shape does NOT reproduce under stage_sync.** Under interference_relax,
transfer dipped at DREAM_GRAVITY=0.15-0.20 and recovered at 0.25. Under stage_sync
at K=2.0, transfer is constant at 0.938 from DREAM_GRAVITY=0.20 to 0.35. The V-shape
was an interference_relax-specific phenomenon where the constructive-pair mechanism
had a threshold sensitivity to gravity; Kuramoto stage_sync doesn't share this
threshold.

### Apparent fitness variation at DREAM_GRAVITY=0.35 and 0.20 (0.036675 vs 0.037397)

Trials 2 and 3 show fitness=0.036675 vs the July 12 baseline of 0.037397. The
difference (0.000722) comes entirely from the `speed` metric: total_ms=15,272 and
14,963 today vs 25,594-25,904 in the July 12 baseline runs. The current container
is faster, producing a higher speed score. All core metrics (transfer, xi, carrier)
are identical. This is a container-timing artifact, not a parameter effect.

**The true fitness at K=2.0 is independent of DREAM_GRAVITY in the 0.20-0.35 range.**

### query_gravity responds monotonically to DREAM_GRAVITY (instrumentation, not fitness)

| DREAM_GRAVITY | query_gravity |
|---------------|---------------|
| 0.20          | 0.8367        |
| 0.25 (baseline) | 0.8623      |
| 0.35          | 0.8962        |

query_gravity rises with DREAM_GRAVITY — expected: higher gravity amplifies
phase-neighbors of the strongest memory more aggressively. This confirms the
attention-as-gravity mechanism is continuously tunable via DREAM_GRAVITY, but it
does not appear in the fitness formula.

## Decision

**No code changes, no parameter changes.** Operating point remains:

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.25 KURAMOTO_COUPLING=2.0
```

Confirmed fitness: **0.037397** (July 12, 3-trial avg). Neither K=1.5 nor
DREAM_GRAVITY variation yields a net improvement above the 0.005 threshold.

## What is now bounded

The fitness landscape around K=2.0 is now well-characterized:
- K: minimum at 2.0, bounded by {1.5, 3.0, 4.0} all being worse
- DREAM_GRAVITY: no effect on core metrics in [0.20, 0.35]; carrier is ceiling-locked

The dominant fitness cost is carrier_emergence at 0.864 (36% of total fitness).
Carrier is ceiling-limited by AMPLITUDE_CEILING=2.0 behavior, not by gravity tuning.

## Next fire recommendations

1. **Amplitude ceiling**: the AMPLITUDE_CEILING=2.0 cap creates the collapse at cycle 5
   that locks carrier_emergence. To improve carrier, the fix must address the ceiling:
   lower it (e.g., 1.5), raise it (3.0), or use relative normalization. At lower
   ceiling, the flat corpus memories hit ceiling earlier, possibly changing the DFT
   shape. At higher ceiling, the collapse may not happen — the pattern would be
   [rise, rise, rise, rise] → k=1 DFT diminished. Risk: transfer may be sensitive.
   
2. **DRIVE_A=0.15 at K=2.0**: the drive signal contributes to amplitude deltas. Stronger
   drive might lift the pre-ceiling cycles enough to shift the DFT shape. Prior testing
   showed A≥0.3 hurts, but A=0.15 hasn't been tested at K=2.0 post-b60f757.

3. **K=2.0 is the floor for Kuramoto.** No further K-sweeps needed unless the
   consciousness-core code changes again. The K-landscape shape is well-defined.

4. **Transfer ceiling check**: transfer=0.938 at K=2.0. Is there a path to 0.96+?
   The July 6 pre-b60f757 result was 0.941 at K=3.0. The b60f757 pair-coupling change
   may be recoverable via interference_threshold adjustment in consciousness-core.
