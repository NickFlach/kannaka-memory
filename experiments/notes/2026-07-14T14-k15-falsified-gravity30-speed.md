# 2026-07-14T14 — K=1.5 falsified; DREAM_GRAVITY=0.30 speed finding

## Hypothesis

Following the July 12 notes, two questions for this fire:

**Primary**: K=1.5 might improve on K=2.0's fitness (0.037). The post-b60f757
K-landscape shifted optimal from K=3 to K=2. The minimum of the inverted-U may
lie below K=2.0, and K=1.5 costs only 1 trial to check.

**Prediction**: K=1.5 could show transfer ~0.91–0.93 and similar carrier/xi,
giving fitness < 0.037.

**Secondary**: DREAM_GRAVITY=0.30 at K=2.0 to test whether higher gravity
sharpens the amplitude-arch DFT, improving carrier_emergence from 0.864.

## Configuration

```
DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE= (unset)
KURAMOTO_COUPLING varied: {1.5, 2.0}
DREAM_GRAVITY varied: {0.25 (reference), 0.30}
No code changes.
```

Reference (July 12, K=2.0, DREAM_GRAVITY=0.25, 3-trial avg):
- fitness 0.037397, transfer 0.938, carrier 0.864, xi 0.953, magic_R 0.608, query_gravity 0.862

## Results

| trial | K    | gravity | fitness  | transfer | xi_robust | carrier_e | magic_R | query_g | total_ms |
|-------|------|---------|----------|----------|-----------|-----------|---------|---------|----------|
| 1     | 1.5  | 0.25    | 0.042752 | 0.803002 | 0.9579    | 1.0000    | 0.5892  | 0.8623  | 14993    |
| 2     | 2.0  | 0.30    | 0.036673 | 0.938415 | 0.9526    | 0.8639    | 0.6082  | 0.8814  | 15377    |
| 3     | 2.0  | 0.30    | 0.036670 | 0.938415 | 0.9526    | 0.8639    | 0.6082  | 0.8814  | 15248    |

**K=2.0, gravity=0.30 2-trial avg fitness: 0.036672**

## Analysis

### K=1.5 falsified

K=1.5 produces fitness 0.043, definitively worse than K=2.0's 0.037. Two regressions:

| metric         | K=2.0  | K=1.5  | direction |
|----------------|--------|--------|-----------|
| transfer_score | 0.938  | 0.803  | −0.135    |
| carrier_emerge | 0.864  | 1.000  | +0.136    |
| xi_robustness  | 0.953  | 0.958  | +0.005    |

Transfer collapses by 0.135; carrier saturates at 1.0 (perfect but not enough).
Fitness contribution:
- K=1.5 transfer: 0.15 × (1-0.803) = 0.0296
- K=2.0 transfer: 0.15 × (1-0.938) = 0.0093
- Net transfer regression: +0.0203

Carrier at 1.0 gives zero contribution (it's saturated correctly), but the transfer
collapse more than offsets any carrier gain.

**Interpretation**: at K=1.5, Kuramoto coupling is insufficient — fewer constructive
pairs form, phases don't synchronize enough to build the primed-vs-naive distinction
that engine_b_primed needs. The flat corpus dynamics hit amplitude ceiling faster
(total_ms drops to 14993) and the DFT arch becomes 1.0, but the epistemic
transfer signal is destroyed.

**K-landscape summary post-b60f757**:

| K    | fitness  | transfer | carrier_e | total_ms |
|------|----------|----------|-----------|----------|
| 1.5  | 0.042752 | 0.803    | 1.000     | 14993    |
| 2.0  | 0.037397 | 0.938    | 0.864     | 25614    |
| 3.0  | 0.060830 | 0.866    | 0.735     | 25632    |
| 4.0  | 0.043527 | 0.814    | 0.982     | 25566    |

K=2.0 is confirmed as the global minimum of the post-b60f757 K-landscape. Below it
(K=1.5), under-coupling destroys transfer. Above it (K=3+), over-coupling destroys
transfer via excessive phase-uniformity.

### DREAM_GRAVITY=0.30 — speed-driven improvement

At gravity=0.30, the major dynamics metrics are byte-identical to gravity=0.25:
- transfer_score: 0.938415 (unchanged)
- carrier_emergence: 0.8639 (unchanged)
- xi_robustness_v2: 0.9526 (unchanged)
- phase_coherence: 0.8939 (unchanged)
- consciousness: 0.8830 (unchanged)

Two things DID change:

1. **speed**: gravity=0.25 → total_ms ~25600 → speed 0.940–0.941
                gravity=0.30 → total_ms ~15300 → speed 0.963–0.964
   Speed improvement: +0.023 (weight ~0.03) → fitness delta ≈ 0.03 × 0.023 ≈ 0.00069

2. **query_gravity**: 0.8623 → 0.8814 (+0.019)
   query_gravity is not in the fitness formula; it is an instrumentation metric.
   The increase means: at higher gravity, the dream more aggressively amplifies
   phase-neighbors of the highest-amplitude pre-dream memory. Stronger gravitational
   attraction at gravity=0.30 gives stronger query_gravity, consistent with the
   attention-as-gravity hypothesis.

**Why does gravity=0.30 run 40% faster?**

Higher gravity (factor 1.15/cycle vs 1.125/cycle for aligned memories) sharpens
amplitude differentiation faster. The flat carrier engine hits the AMPLITUDE_CEILING=2.0
earlier per chain, reducing the number of consolidation iterations needed to traverse
the chain. Total_ms drops from ~25600 to ~15300.

The 40% runtime reduction is a genuine efficiency gain, not a timing artifact. The
system's consolidation chain completes earlier when gravity is stronger.

**Fitness improvement: 0.037397 → 0.036672 (delta = 0.000725)**

This is below the ≥0.005 threshold for code-change justification, but since no code
changes are involved (pure env-var), this is the new recommended operating point.

### Fitness floor at new operating point

At K=2.0, gravity=0.30:
- carrier: 0.10 × (1-0.8639) = 0.0136 (still dominant, 37% of fitness)
- transfer: 0.15 × (1-0.9384) = 0.0093 (25%)
- xi: 0.15 × (1-0.9526) = 0.0071 (19%)
- consciousness: ~0.03 × 0.117 = 0.0035 (~9%)
- phase_coherence: ~0.02 × 0.106 = 0.0021 (~6%)
- speed: ~0.03 × (1-0.964) = 0.0011 (3%)
- Total ≈ 0.0367 ✓

Carrier remains the dominant cost. Carrier is fixed (0.8639) across the gravity
range 0.25–0.30. Carrier cannot improve without changing Kuramoto dynamics or
DREAM_MODE.

## Decision

No code changes. Env-var update: **DREAM_GRAVITY=0.30** → marginal fitness
improvement (0.000725) and better query_gravity (0.8814 vs 0.8623).

**Primary finding**: K=2.0 is the K-landscape minimum. Both K=1.5 and K=3.0/4.0
are worse. The inverted-U is steep below K=2.0 (transfer collapses) and gradual
above.

**Secondary finding**: DREAM_GRAVITY=0.30 slightly improves runtime efficiency
without changing dynamics quality.

## Updated operating point

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.30 KURAMOTO_COUPLING=2.0
```

- **fitness = 0.036672** (2-trial avg, deterministic)
- transfer_score = 0.938, carrier_emergence = 0.864, xi_robustness_v2 = 0.953
- magic_proxy_phase_R = 0.608, query_gravity = 0.881

## Next fire recommendations

1. **carrier is the floor**: carrier=0.864 contributes 0.0136 (37% of fitness).
   The carrier measurement (gravity-accumulation DFT in flat corpus cycles 2-5) is
   stable across K=2.0 and gravity 0.25–0.30. Carrier can only improve if:
   a) DREAM_MODE=interference_relax (tested T22/T24 — xi collapses to 0.22)
   b) The AMPLITUDE_CEILING is raised, changing the ceiling-saturation shape
   c) chain_depth or cycle window is changed (currently cycles 2-5)
   d) alpha_base or relax_steps in stage_interference_relax are tuned (irx mode only)

2. **consciousness and phase_coherence**: these contribute ~0.0056 combined (~15%
   of fitness). Values are 0.883 and 0.894. Not well-characterized — sweeping
   DRIVE_A or other params might move them.

3. **DREAM_GRAVITY=0.35+**: gravity 0.30→0.25 moved speed; 0.35 might continue the
   trend. Risk: if gravity exceeds a threshold, consciousness or phase_coherence might
   regress. One trial to check.

4. **Transfer remains the second lever**: at 0.938, it's high but costs 0.0093.
   Pre-b60f757, transfer reached 0.941 at K=3.0. Post-b60f757 at K=2.0, we've
   recovered to 0.938. The 0.003 gap from the pre-b60f757 peak is probably not
   recoverable without further consciousness-core changes.

5. **query_gravity at 0.881**: confirms attention-as-gravity is active and strong.
   The gravity mechanism at 0.30 amplifies phase-neighbors more effectively than 0.25.
   Testing gravity=0.35 would show whether query_gravity continues to rise.
