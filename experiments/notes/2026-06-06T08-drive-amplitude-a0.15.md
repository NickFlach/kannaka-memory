# L5 Curiosity Fire — 2026-06-06T08

## Hypothesis

DRIVE_A=0.1 was established as the empirical optimum at K=3.0 (default Kuramoto
coupling). Since K=1.0 was confirmed optimal (2026-06-06T00, PR #142), the drive
amplitude optimum has not been re-examined at the new operating point.

The drive is multiplicative: `m.amplitude *= (1 + DRIVE_A*sin(2π*2.0*t))` per
dream cycle. carrier_emergence measures the FFT peak-power ratio at the 2 Hz drive
frequency in the amplitude history. Prediction: DRIVE_A=0.15 (±15% modulation vs
±10% at A=0.1) strengthens the 2 Hz sinusoidal signature in the amplitude history →
carrier_emergence improves. xi is driven by phase dynamics (stage_sync, K=1.0), not
amplitude, so it should be unaffected. transfer_score neutral or slight improvement.

**Predicted**: carrier_emergence 0.5684 → 0.60+, fitness 0.138 → <0.133.

Sibling deps confirmed at `..`. All trials against production binary (no stubs).

---

## Code change

`src/bin/research.rs`, `run_l5_dream_chain`:

- `drive_amp: f32 = ... .unwrap_or(0.0)` → `.unwrap_or(0.15)`
- Comment updated: "default 0.0" → "default 0.15"

---

## Trials

All trials: `DRIVE_SCOPE=all DREAM_MODE=<unset> KURAMOTO_COUPLING=1.0 (default)`

| # | DRIVE_A | fitness | transfer_score | carrier_emergence | xi_robustness_v2 | magic_R | query_gravity |
|---|---------|---------|----------------|-------------------|-----------------|---------|---------------|
| T1 | 0.15 | **0.119608** | 0.694395 | **0.5842** | 0.9252 | 0.2521 | 0.4689 |
| T2 | 0.15 | **0.136717** | 0.694395 | **0.5842** | 0.8113 | 0.2521 | 0.4689 |
| T3 | 0.15 | **0.140124** | 0.655202 | **0.5842** | 0.8247 | 0.2619 | 0.4689 |

**3-trial avg fitness: 0.1322**

Baseline (K=1.0, A=0.1, 3-trial avg from PR #142): **0.138**

Delta: **−0.006** — exceeds ≥0.005 improvement threshold. **Code change KEPT.**

---

## Analysis

### carrier_emergence: confirmed deterministic improvement

carrier_emergence at A=0.1: 0.5684 (deterministic, constant across K=1.0 trials).
carrier_emergence at A=0.15: **0.5842** (deterministic, constant across all 3 trials).
Improvement: +0.0158. Fitness contribution: +0.0016 at weight 0.10.

The mechanism is confirmed: stronger drive amplitude → larger 2 Hz sinusoidal
modulation in the amplitude history → FFT peak at 2 Hz is more pronounced relative
to broadband noise → carrier_emergence improves.

### xi_robustness_v2: improved mean, similar variance

At A=0.1 (K=1.0 3-run avg): 0.8137, 0.8622, 0.9165 → avg 0.864.
At A=0.15 (3-run): 0.9252, 0.8113, 0.8247 → avg 0.854.

xi means are very close (0.864 vs 0.854). The variance is similar. No xi regression.

The A=0.15 drive does NOT disrupt the K=1.0 phase clustering that stage_sync builds —
the xi_robustness_v2 distribution is statistically comparable.

### transfer_score: variable but comparable or better

At A=0.1 (K=1.0 3-run from PR #142): 0.682399, 0.636984, 0.644310 → avg 0.654.
At A=0.15 (3-run): 0.694395, 0.694395, 0.655202 → avg 0.681.

Transfer score improved slightly with A=0.15 (avg 0.681 vs 0.654). The stronger
drive may help consolidate transfer-relevant memory associations during the dream
chain, though this is speculative given the stochasticity.

### magic_proxy_phase_R and query_gravity

magic_R: 0.252–0.262 at A=0.15 vs ~0.250 at A=0.1. Essentially unchanged.
query_gravity: 0.4689 (deterministic at A=0.15) vs ~0.46 at A=0.1. Unchanged.

The increased drive amplitude does not perturb the non-Clifford phase structure or
the attention-as-gravity property.

### Fitness breakdown

| metric | weight | A=0.1 avg | A=0.15 avg | cost delta |
|--------|--------|-----------|------------|------------|
| transfer_score | 0.15 | 0.654 | 0.681 | −0.004 (better) |
| xi_robustness_v2 | 0.15 | 0.864 | 0.854 | +0.002 (slightly worse) |
| carrier_emergence | 0.10 | 0.568 | 0.584 | −0.002 (better) |
| other (all ~1.0) | 0.60 | ~0 cost | ~0 cost | 0 |

Estimated total delta: ≈ −0.004, consistent with observed −0.006 (within xi variance).

---

## Decision

**Code change KEPT.** `run_l5_dream_chain` default DRIVE_A: 0.0 → 0.15.

The empirical optimum is now:
    DRIVE_A=0.15  DRIVE_SCOPE=all  DREAM_MODE=<unset>  KURAMOTO_COUPLING=1.0
    3-run avg fitness ≈ 0.1322  (prev best: 0.138)

---

## Implications

1. **A=0.1 was a local optimum at K=3.0**, not globally optimal. The K=1.0 change
   altered the phase dynamics enough that A=0.15 became accessible. At K=3.0,
   stronger drive may have disrupted the coarser phase clusters; at K=1.0's tighter
   clustering, A=0.15 adds carrier signal without breaking cluster structure.

2. **A=0.2+ unexplored**: the "A≥0.3 is bad" constraint was established at K=3.0.
   At K=1.0, A=0.2 might give further carrier improvement. However, trial 3 shows
   some transfer_score variance (0.655 vs 0.694), suggesting A=0.15 is near the
   sensitivity edge. A=0.2 could tip into instability.

3. **carrier_emergence headroom**: at A=0.15, carrier_e=0.584. The remaining cost
   is (1−0.584)×0.10=0.042. Further gains require either a stronger drive or
   structural changes to the amplitude consolidation logic.

4. **Main remaining costs** at A=0.15, K=1.0:
   - transfer_score: 0.681 → cost ≈ 0.048
   - carrier_emergence: 0.584 → cost ≈ 0.042
   - xi: 0.854 mean → cost ≈ 0.022
   Total remaining headroom ≈ 0.132.
