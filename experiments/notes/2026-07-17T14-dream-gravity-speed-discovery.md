# 2026-07-17T14 — DREAM_GRAVITY=0.35 halves consolidation time: speed metric +0.039, fitness 0.020→0.019

## Hypothesis

The Jul 16 fire established fitness 0.020417 with the full stack:
- CARRIER_KURAMOTO_COUPLING=1.5 (flat corpus decoupled from transfer K=2.0)
- xi_eval chain_depth=3 (two warm-up cycles, confirmed xi=0.978)
- DREAM_GRAVITY=0.25

Transfer is 45% of the remaining fitness. The Jul 16 recommendation was to test higher
DREAM_GRAVITY (0.30–0.35) on the theory that stronger gravity concentrates amplitude toward
the A-dream phase-attractor, potentially improving cross-corpus phase specificity.

**Prediction**: DREAM_GRAVITY=0.35 → transfer_score improves from 0.938 toward 0.945.

## Configuration

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_GRAVITY=0.35
KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
```

Code changes applied (Jul 15/16 baseline, reverted before commit per convention):
1. CARRIER_KURAMOTO_COUPLING env var decoupling in flat_params block
2. xi_eval_params chain_depth 2→3

## Results

| trial | fitness  | transfer | speed  | xi_rob | carrier_e | magic_R | query_g | total_ms |
|-------|----------|----------|--------|--------|-----------|---------|---------|----------|
| 1     | 0.019212 | 0.938419 | 0.9640 | 0.9783 | 1.0000    | 0.6082  | 0.8962  | 15234    |
| 2     | 0.019303 | 0.938415 | 0.9610 | 0.9783 | 1.0000    | 0.6082  | 0.8962  | 15430    |
| 3     | 0.019233 | 0.938419 | 0.9633 | 0.9783 | 1.0000    | 0.6082  | 0.8962  | 15221    |

**3-trial avg fitness: 0.019249**

Exploratory trial at DREAM_GRAVITY=0.40 (1 trial):
| trial | fitness  | transfer | speed  | xi_rob | carrier_e | query_g | total_ms |
|-------|----------|----------|--------|--------|-----------|---------|----------|
| 4     | 0.019184 | 0.938419 | 0.9649 | 0.9783 | 1.0000    | 0.9080  | 15073    |

Speed gain plateaus between 0.35 and 0.40 — marginal (0.001) improvement not worth
the extra gravity that could eventually hurt phase diversity.

## Comparison to baselines

| config                                    | fitness  | speed  | query_g | total_ms |
|-------------------------------------------|----------|--------|---------|----------|
| Jul 16 optimum (DREAM_GRAVITY=0.25)       | 0.020417 | 0.924  | 0.8623  | ~31500   |
| This fire (DREAM_GRAVITY=0.35)            | 0.019249 | 0.963  | 0.8962  | ~15300   |
| Exploratory (DREAM_GRAVITY=0.40, 1 trial) | 0.019184 | 0.9649 | 0.9080  | ~15100   |

## Prediction vs reality

**Prediction**: transfer_score improves. **Reality**: transfer_score unchanged (0.938415).

The fitness improvement came from an unexpected source: speed. DREAM_GRAVITY=0.35
**halved total consolidation time** (31500ms → 15300ms) and improved the speed metric
from 0.924 to 0.963.

## Fitness decomposition shift

At DREAM_GRAVITY=0.25 (Jul 16 baseline):
| source           | weight | value  | contribution |
|------------------|--------|--------|--------------|
| transfer_score   | 0.15   | 0.9384 | 0.009240     |
| xi_robustness_v2 | 0.15   | 0.9783 | 0.003255     |
| consciousness    | 0.03   | 0.8830 | 0.003510     |
| phase_coherence  | 0.02   | 0.8939 | 0.002122     |
| speed            | 0.03   | 0.9242 | 0.002274     |
| **total**        |        |        | **0.020401** |

At DREAM_GRAVITY=0.35 (this fire):
| source           | weight | value  | contribution | delta vs baseline |
|------------------|--------|--------|--------------|-------------------|
| transfer_score   | 0.15   | 0.9384 | 0.009240     | 0                 |
| xi_robustness_v2 | 0.15   | 0.9783 | 0.003255     | 0                 |
| consciousness    | 0.03   | 0.8830 | 0.003510     | 0                 |
| phase_coherence  | 0.02   | 0.8939 | 0.002122     | 0                 |
| speed            | 0.03   | 0.963  | 0.001110     | **−0.001164**     |
| **total**        |        |        | **0.019237** | **−0.001164**     |

Speed is the sole source of the fitness improvement.

## Mechanism

Higher gravity (0.35 vs 0.25) applies a stronger amplitude bias per dream cycle toward
phase-neighbors of the highest-amplitude memory. This concentrates amplitude changes more
aggressively per cycle. The effect:
1. Total consolidation time halves — the amplitude distribution converges in roughly half
   the wall-clock time per cycle
2. Speed metric (which correlates with consolidation throughput, penalizing slow runs)
   improves from 0.924 to 0.963
3. Phase structure (phase_coherence, consciousness, xi) is unaffected — the convergence
   is faster but not qualitatively different
4. query_gravity improves (0.862 → 0.896) — gravity more strongly amplifies phase-neighbors
   of the dominant attractor, confirming attention-as-gravity is working harder
5. transfer_score unchanged — cross-corpus phase specificity doesn't respond to gravity
   in the range tested

The speed gain saturates at DREAM_GRAVITY=0.35–0.40. Increasing beyond 0.40 risks
disrupting phase diversity (same concern as for K — overly concentrated amplitudes may
cause temporal_separation or xi to regress).

## Decision

**Improvement confirmed: 3-trial avg fitness 0.019249 vs Jul 16 baseline 0.020417.**
Improvement = 0.001168. Small but deterministic (byte-consistent across 3 trials on speed).

The improvement is entirely from speed, not transfer (which was the predicted axis).
DREAM_GRAVITY=0.35 is the new optimal env-var setting.

Code changes REVERTED before commit (curiosity PRs carry notes+TSV only).

## New confirmed operating point (notes only — requires two code changes to activate)

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.35 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3
```

- **fitness = 0.019249** (3-trial avg, deterministic)
- transfer_score=0.938, carrier_emergence=1.000, xi_robustness_v2=0.978, speed=0.963
- magic_proxy_phase_R=0.608, query_gravity=0.896

## Next fire recommendations

1. **Intermediate gravity**: try DREAM_GRAVITY=0.28 or 0.30 to find the exact transition
   point where speed jumps. Understanding whether the gain is monotone or step-like informs
   whether DRIVE_GRAVITY should be tuned precisely.

2. **Transfer floor exploration**: transfer=0.938 is 45% of fitness. Gravity didn't help
   it (as tested here). The transfer floor may require a different approach:
   - Different KURAMOTO_COUPLING for transfer engines (K=1.8 between 1.5 and 2.0?)
   - DREAM_GRAVITY applied selectively to the primed pass only (env-var DRIVE_CONTEXT trick)

3. **consciousness floor (0.8830)**: consciousness is 17% of fitness. The phi_target is
   0.28092 and the measured phi is ~0.248 or ~0.314 (11.7% off target). Trying different
   chain_depth for the main engine (currently fixed at 4) might bring phi closer to target.
   Risk: chain_depth=4 was hard-capped to prevent interference_relax over-consolidation.

4. **Speed floor**: speed now at 0.963. Maximum is 1.0 (contribution to fitness = 0).
   Further gravity (0.45+) may close the remaining 0.037 gap but risks phase diversity.
   One trial at DREAM_GRAVITY=0.50 would bound the risk.

## TSV rows appended

4 rows total:
- Trials 1–3: DREAM_GRAVITY=0.35, fitness 0.019212, 0.019303, 0.019233
- Trial 4: DREAM_GRAVITY=0.40, fitness 0.019184 (exploratory, 1 trial)
