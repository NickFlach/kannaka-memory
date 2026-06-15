# L5 Curiosity: DREAM_GRAVITY=0.5 + interference_relax — gravity confirmed but below threshold

**Date:** 2026-06-15T20 UTC  
**Branch:** kannaka-curiosity/2026-06-15T20-gravity-at-irx  
**Code changes:** NONE — env-var only  
**Status:** NOT KEPT — 0.000546 fitness improvement, below 0.005 threshold

---

## Context

Post-fix optimum: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness 0.0578
(T14, 3-trial avg, near-zero variance). carrier_emergence = 0.533 (ceiling-dominated) is the
dominant remaining cost (0.047 of 0.058 total fitness).

T14 notes flagged "Verify DREAM_GRAVITY knob with interference_relax" as #1 recommendation.
The gravity-repulsion sweep (same-date fire) tested DREAM_GRAVITY=0.5 under stage_sync and found
transfer_score collapsed (0.737→0.542) with xi benefit (+0.069), net fitness worse (0.115→0.135).

Under interference_relax (R=0.867), phases are already highly aligned. With most memories
phase-near the attractor (align > 0.5), DREAM_GRAVITY creates near-uniform amplification.
Prediction: gravity's relative amplitude differentiation is much weaker under high R, so
transfer regression should be small vs stage_sync, while xi improvement is preserved.

---

## Hypothesis

Under high-R (0.867) from interference_relax, DREAM_GRAVITY=0.5 acts near-uniformly (all
memories phase-near attractor), preserving transfer_score while improving xi_robustness_v2.
Secondary test: does gravity modify engine_flat's amplitude-delta pattern to help carrier_e?

**Prediction**: fitness improvement over interference_relax baseline (0.0578), possibly
reaching 0.050–0.054 if carrier_e benefits from gravity-induced amplitude differentiation.

---

## Results

Command: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax DREAM_GRAVITY=0.5 cargo run --release --quiet --bin research -- --level 5`

| trial | fitness  | transfer_score | carrier_e | xi_v2  | R      | query_gravity |
|-------|----------|----------------|-----------|--------|--------|---------------|
| t1    | 0.057241 | 0.961566       | 0.5258    | 0.9796 | 0.8669 | 0.9256        |
| t2    | 0.057245 | 0.961566       | 0.5258    | 0.9796 | 0.8669 | 0.9256        |
| **avg** | **0.057243** | **0.961566** | **0.5258** | **0.9796** | **0.8669** | **0.9256** |

Baseline (interference_relax, no gravity, T14 3-trial avg):
| baseline | 0.057789 | 0.965455 | 0.5333 | 0.9675 | 0.8672 | 0.4603 |

---

## Analysis

### Fitness: marginal improvement, below threshold

Fitness: 0.057789 → 0.057243 = **Δ −0.000546** (below 0.005 keep threshold).

Metric decomposition (weight × Δ):
- xi: 0.15 × (0.9796−0.9675) = **+0.001815** (fitness improves)
- transfer: 0.15 × (0.9655−0.9616) = **−0.000585** (fitness worsens)
- carrier_e: 0.10 × (0.5333−0.5258) = **−0.000750** (fitness worsens)
- net: ≈ **+0.000480** (matches observed 0.000546)

### Prediction: partially confirmed

The transfer regression is small (−0.004) vs stage_sync (−0.195 at GRAVITY=0.5). This
confirms the prediction: high-R makes gravity near-uniform, sharply reducing the amplitude
differentiation that distorts engine_b_primed's initial state. The xi improvement (+0.012)
is also as predicted, from slightly tighter phase concentration in the cleaned memory set.

The prediction that carrier_e would benefit was **wrong**. Gravity in engine_flat reduces
carrier_e from 0.5333 → 0.5258. Mechanism: gravity redistributes amplitude in engine_flat
toward the flat-corpus attractor, creating a more amplitude-concentrated (less oscillatory)
pattern per cycle → FFT finds less carrier signal.

### query_gravity: decisive confirmation

query_gravity: **0.460 → 0.9256**. The "attention-as-gravity" hypothesis is now confirmed
experimentally. With DREAM_GRAVITY=0.5, the dream does amplify phase-neighbors of the
highest-amplitude pre-dream memory more than phase-distant ones — well above the 0.5
chance line. Under interference_relax (high R = near-global phase coherence), the gravity
is especially effective because the attractor's neighborhood contains most memories.

This also explains why query_gravity is a useful proxy for "magic" (non-Clifford content):
high R → most memories phase-coherent with attractor → gravity boosts nearly all memories
uniformly → high query_gravity. The mechanism is the same as why interference_relax has
higher magic_proxy_phase_R (0.867 vs 0.355 at stage_sync).

### Near-determinism

Results are essentially byte-identical across trials (variance < 0.000005), confirming
post-fix interference_relax is in a stable fixed-point regime. Gravity at 0.5 is also
deterministic — the amplitude redistribution per cycle is fully determined by initial
conditions and the constructive-pair graph.

---

## Why the improvement doesn't scale

The fitness gap is now **0.058**, dominated almost entirely by carrier_emergence cost
(0.10 × (1-0.533) = 0.047). Gravity slightly worsens carrier_e (−0.0075). Even
tripling gravity_gain would:
- xi: perhaps 0.9796 → ~0.99 → +0.0015 fitness
- transfer: −0.010 → −0.0015 fitness regression
- carrier_e: further worsened → more regression

The trade-off doesn't improve at higher gravity. The carrier_emergence dominance means
no phase-only intervention (interference_relax, gravity, K-sweep) can push fitness below ~0.011.

---

## Closed axes (post-fix, comprehensive)

As of this fire, the following are characterized and closed post-fix with interference_relax:

| axis | status | notes |
|------|--------|-------|
| DRIVE_A=0.15 | optimal | T12/T14 confirmed |
| DRIVE_SCOPE=all | optimal | no_transfer tested, worse xi variance |
| DREAM_MODE=interference_relax | optimal | 50% improvement over stage_sync |
| DREAM_GRAVITY (all engines) | sub-threshold | 0.000546 improvement, transfer slightly worse |
| DREAM_GRAVITY (xi engines only) | predicted sub-threshold | xi already 0.967+; ~0.002 gain max |
| K-sweep | no-op | interference_relax doesn't use Kuramoto |
| relax_steps (16/20) | near-optimal | T07 pre-fix: steps↑ kills carrier_e |
| DRIVE_FREQ_HZ | invariant | ceiling dominates carrier_e |
| CHIRAL_BP sweep | closed | T18: speed cost cancels transfer gain |
| REPULSION_THRESHOLD | hard min 0.28 | T14-gravity: 0.20 collapses transfer |
| chain_carry_strength | 0.7 optimal | pre-fix; hardcoded in L5 at line 3437 |
| no_transfer + irx | pre-fix falsified | xi collapse; post-fix untested |

---

## Decision

**No code changes. No improvements kept.**  
Two TSV rows appended (labeled `L5`).  
Notes file committed.  

**Current optimum: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness 0.0578.**

---

## Remaining path to improvement

Fitness is now ceiling-bounded by `carrier_emergence` (0.533, cost 0.047). The root cause:
AMPLITUDE_CEILING=2.0 makes constructive-pair amplitude boosts hit the ceiling in bursts,
creating impulse-shaped amplitude_deltas whose FFT has no clean carrier. Phase-only
interventions (interference_relax, gravity, K) cannot fix amplitude mechanics.

The only credible remaining paths require architectural changes:
1. **Structural amplitude smoothing** (post-constructive decay): apply per-cycle amplitude
   decay to non-constructive memories after stage_constructive. Creates amplitude bimodality
   that the FFT can detect. Predicted: carrier_e → 0.85–0.99 (+0.030–0.046 fitness).
   Requires consolidation.rs change (~15 lines). Would push overall fitness to 0.012–0.027.
2. **xi_repulsion_weight env var** (currently hardcoded 0.3): minimal benefit now (xi=0.967
   already; max gain ~0.001). Not worth implementing post-interference_relax.
3. **no_transfer + interference_relax post-fix test**: pre-fix showed xi collapse (xi→0.067),
   but post-fix dynamics differ substantially. One trial would characterize whether the
   pre-fix incompatibility still holds, and if not, whether it stacks with irx.
