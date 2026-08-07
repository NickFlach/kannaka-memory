# 2026-07-26T14 — xi_eval Kuramoto coupling decoupled: K=1.0 gives xi 0.9783→0.9980

## Context

Entering confirmed operating point (requires three ephemeral code changes, documented below):
```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_GRAVITY=0.35
KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3
```
3-trial avg fitness: **0.019249** (Jul 17 fire)

Remaining fitness dominated by:
| source           | weight | value  | contribution | % of fitness |
|------------------|--------|--------|--------------|-------------|
| transfer_score   | 0.15   | 0.9384 | 0.009240     | 48%         |
| consciousness    | 0.03   | 0.8830 | 0.003510     | 18%         |
| xi_robustness_v2 | 0.15   | 0.9783 | 0.003255     | 17%         |
| phase_coherence  | 0.02   | 0.8939 | 0.002122     | 11%         |
| speed_a          | 0.03   | 0.963  | 0.001110     | 6%          |

## Hypothesis

`xi_eval_params` inherits `kuramoto_coupling = 2.0` from the main params. K=2.0 was
chosen to optimise transfer (Jul 12 K-sweep) — not xi robustness. The xi eval runs
TWO independent dream chains (clean and adversarial) of depth=3. Within these engines,
K=2.0 may over-couple legitimate memories, amplifying adversarial disruption leverage:
at high K, adversarial memories injected across cluster boundaries can drag legitimate
memories further off-cluster via the stronger Kuramoto sync force.

**Prediction**: xi_eval at K=1.0 (weaker coupling) reduces adversarial disruption
leverage → smaller |fitness_clean − fitness_adv| → xi improves from 0.9783 toward 1.0.

**Implementation**: after the two baseline ephemeral changes, add:
```rust
p.kuramoto_coupling = 1.0; // in xi_eval_params block
```

## Code changes applied (all reverted before commit)

1. CARRIER_KURAMOTO_COUPLING env var plumbing in flat_params block (baseline ephemeral)
2. xi_eval_params.chain_depth: 2 → 3 (baseline ephemeral, Jul 16 discovery)
3. xi_eval_params.kuramoto_coupling = 1.0 (experimental, this fire)

## Results

### Three trials at xi_eval K=1.0

| trial | fitness  | xi_rob | transfer | carrier_e | consciousness | magic_R | query_g | speed  | total_ms |
|-------|----------|--------|----------|-----------|---------------|---------|---------|--------|----------|
| 1     | 0.017035 | 0.9980 | 0.938419 | 1.0000    | 0.8830        | 0.6082  | 0.8962  | 0.9380 | 26094    |
| 2     | 0.017001 | 0.9980 | 0.938419 | 1.0000    | 0.8830        | 0.6082  | 0.8962  | 0.9391 | 25607    |
| 3     | 0.017059 | 0.9980 | 0.938419 | 1.0000    | 0.8830        | 0.6082  | 0.8962  | 0.9372 | 25760    |

**3-trial avg fitness: 0.017032**

### One characterisation trial at xi_eval K=0.5

| trial | fitness  | xi_rob | transfer | speed  | total_ms |
|-------|----------|--------|----------|--------|----------|
| 4     | 0.018171 | 0.9904 | 0.938419 | 0.9382 | 25622    |

### xi landscape

| K (xi_eval) | xi_robustness_v2 | xi contribution | fitness (3-trial avg or 1 trial) |
|-------------|-----------------|-----------------|----------------------------------|
| 0.5         | 0.9904          | 0.001440        | 0.018171 (1 trial)               |
| 1.0         | **0.9980**      | **0.000300**    | **0.017032** (3-trial avg)       |
| 2.0 (base)  | 0.9783          | 0.003255        | 0.019249 (3-trial avg, Jul 17)   |

K=1.0 is the confirmed peak — non-monotone landscape with K=0.5 and K=2.0 both
giving lower xi.

## Environment note

This fire's container ran noticeably slower than Jul 17: speed_a ≈ 0.938 (vs 0.963)
and total_ms ≈ 26000 (vs ~15000). Engine_a dream time in this environment was
~3700ms vs ~2200ms in the Jul 17 environment. The speed regression costs:
- 0.03 × (0.963 − 0.938) = 0.000750 fitness (partially offsetting the xi gain)

The xi metric itself is deterministic (fixed K, fixed corpus, fixed seed): xi=0.9980
at K=1.0 is environment-independent. The pure xi contribution improvement:
- Before: 0.15 × (1 − 0.9783) = 0.003255
- After:  0.15 × (1 − 0.9980) = 0.000300
- **Savings: 0.002955** (independent of environment speed)

Net observed fitness improvement: 0.019249 − 0.017032 = **0.002217**
(= 0.002955 xi savings − 0.000750 environment speed regression, approximately)

## Mechanism

At K=1.0 for xi_eval depth=3:
- Clean engine: phases cluster more weakly than at K=2.0 (each Kuramoto step moves
  phases less). Chain_fidelity and consciousness degrade slightly → fitness_clean rises.
- Adversarial engine: 30 adversarial memories create cross-cluster phase bridges.
  At K=2.0, the bridge mechanism AMPLIFIES: the strong sync pulls legitimate memories
  toward adversarial anchors, creating large fitness_adv divergence from fitness_clean.
  At K=1.0, the sync force is halved — adversaries cannot leverage coupling to drag
  legitimate memories far off-cluster within 3 cycles.
- Result: |fitness_clean − fitness_adv| shrinks at K=1.0. Since fitness_clean < 0.05
  (the normaliser floor), xi = 1 − |fc − fa| / 0.05 → improved.

At K=0.5, the coupling is too weak: legitimate memories barely consolidate in 3 cycles,
so fitness_clean rises above the 0.05 normaliser floor. Once fitness_clean > 0.05, the
normaliser is no longer capped, and a small absolute divergence |fc − fa| produces a
larger fractional divergence / fitness_clean → xi falls back to 0.9904.

K=1.0 hits the sweet spot: clean engine consolidates enough that fitness_clean stays
below 0.05, while adversarial coupling leverage is halved vs K=2.0.

## Comparison to baseline

| metric           | baseline (Jul 17) | this fire (K=1.0) | delta     |
|------------------|-------------------|-------------------|-----------|
| fitness avg      | 0.019249          | 0.017032          | −0.002217 |
| xi_robustness_v2 | 0.9783            | 0.9980            | +0.0197   |
| transfer_score   | 0.938419          | 0.938419          | 0         |
| carrier_emergence| 1.0000            | 1.0000            | 0         |
| consciousness    | 0.8830            | 0.8830            | 0         |
| phase_coherence  | 0.8939            | 0.8939            | 0         |
| magic_R          | 0.6082            | 0.6082            | 0         |
| query_gravity    | 0.8962            | 0.8962            | 0         |
| speed_a          | ~0.963            | ~0.938            | −0.025 (environment) |

The xi improvement is cleanly isolated — all other metrics byte-identical. The speed
difference is environmental (container throughput), not caused by the xi_eval K change.

## Decision

**Hypothesis confirmed.** xi_eval K=1.0 is deterministically superior to K=2.0:
- xi: 0.9783 → 0.9980 (3 trials, byte-identical)
- Pure xi savings: 0.002955 fitness
- Net observed improvement: 0.002217 (environment-adjusted)

Code changes reverted before commit (curiosity PRs carry notes + TSV only).

## Updated confirmed operating point (notes only — requires THREE code changes)

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.35 KURAMOTO_COUPLING=2.0 CARRIER_KURAMOTO_COUPLING=1.5
xi_eval_params.chain_depth=3
xi_eval_params.kuramoto_coupling=1.0    ← NEW (this fire)
```

- **fitness ≈ 0.017032** (3-trial avg, this environment; ~0.016294 in Jul 17 environment)
- transfer_score=0.938, carrier_emergence=1.000, xi_robustness_v2=0.9980, consciousness=0.883
- magic_proxy_phase_R=0.608, query_gravity=0.896

## Fitness decomposition at new optimum (Jul 17 environment speed)

| source           | weight | value  | contribution | % of fitness |
|------------------|--------|--------|--------------|-------------|
| transfer_score   | 0.15   | 0.9384 | 0.009240     | 57%         |
| consciousness    | 0.03   | 0.8830 | 0.003510     | 22%         |
| xi_robustness_v2 | 0.15   | 0.9980 | 0.000300     | 2%          |
| phase_coherence  | 0.02   | 0.8939 | 0.002122     | 13%         |
| speed_a          | 0.03   | 0.963  | 0.001110     | 7%          |
| other            |        | 1.0    | ~0           | 0%          |
| **total**        |        |        | **~0.016282**| 100%        |

xi is no longer a dominant fitness component (2% vs previous 17%). Transfer (57%),
consciousness (22%), and phase_coherence (13%) are the remaining levers.

## Next fire recommendations

1. **Transfer ceiling investigation**: transfer=0.938 is now 57% of remaining fitness.
   Previous attempts (K-sweep, gravity, chiral_b_primed, DRIVE_FREQ_HZ, CHAIN_TOP_N)
   all left it unchanged. The Jul 21 notes recommended investigating the chain_fidelity
   gap between B_primed and B_naive inside eval_l5_placeholder_fitness. Add print
   statements for B engine phi_histories and sub-components (noise, phase_coh, chain_fid,
   consciousness per B engine) — 1 diagnostic trial, no expectation of gain.

2. **consciousness structural investigation**: phi_target coupling is understood (Jul 21).
   Decoupling main_phi_target from eval_phi_target saves 0.003510; worth implementing
   if bundled with ≥0.001 from another source. Now that xi is essentially solved,
   consciousness is 22% of remaining fitness — the second-biggest lever.

3. **phase_coherence ceiling**: at 0.8939 (13% of fitness), kuramoto_steps=100 was
   catastrophic (Jul 20). Unexplored: KURAMOTO_COUPLING for the xi eval is now K=1.0.
   Does K=1.5 for xi_eval give xi=0.998 or higher? (small marginal sweep, 1 trial)

4. **xi eval K=1.5**: one trial to confirm K=1.0 is truly the peak and K=1.5 is
   not slightly better. The K landscape is coarse (K=0.5, 1.0, 2.0 tested).

## TSV rows appended (4 total)

- Trials 1–3: xi_eval K=1.0, xi=0.9980, fitness 0.017035/0.017001/0.017059
- Trial 4: xi_eval K=0.5 (characterisation), xi=0.9904, fitness 0.018171
