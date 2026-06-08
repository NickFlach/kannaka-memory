# irx_sync hybrid falsified — modes compete, not complement

**Date:** 2026-06-08T05 UTC
**Branch:** kannaka-curiosity/2026-06-08T05
**Code changes:** `DREAM_MODE=irx_sync` branch added to consolidation.rs dispatch, then reverted
**Status:** FALSIFIED — no code changes kept

---

## Background

Two dream modes have been characterized:
- **interference_relax (irx)**: `DRIVE_A=0.1 DREAM_MODE=interference_relax DRIVE_SCOPE=all` → 3-trial avg fitness **0.099**, carrier_e=0.935, transfer=0.836, xi=0.559
- **stage_sync (Kuramoto)**: `DRIVE_A=0.15 DRIVE_SCOPE=all` (code defaults, K=0.5) → 3-trial avg fitness **0.104**, carrier_e=0.853, transfer=0.655, xi=0.873

The modes trade off: irx wins on carrier_e and transfer, stage_sync wins on xi. The largest improvement opportunity is xi under irx: the xi gap × weight = (0.873 − 0.559) × 0.15 = 0.047 fitness potential if xi could be lifted to stage_sync levels while keeping carrier_e and transfer.

---

## Hypothesis: irx → Kuramoto sequential hybrid (`DREAM_MODE=irx_sync`)

**Prediction:** Running interference_relax first (to set up constructive-pair phase alignment and carrier amplitude structure), then running Kuramoto sync (to create category phase-separation), would combine irx's carrier/transfer strengths with Kuramoto's xi strength.

**Mechanism assumption:** stage_sync only modifies memory phases, not amplitudes. Since carrier_e measures an FFT peak of the amplitude time series, it should be unaffected by a phase-only operation running after irx.

**Code change (reverted):** Added `DREAM_MODE=irx_sync` branch in consolidation.rs stage 4.5 dispatch:
```rust
} else if dream_mode == "irx_sync" {
    let (n, r1) = self.stage_interference_relax(engine, &working_set, &pairs);
    let (_, r2) = self.stage_sync(engine, &working_set);
    (n, r1 + r2)
}
```

---

## Results

### Trial 1: K=0.5 (production default)

`DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=irx_sync KURAMOTO_COUPLING=0.5`

| metric | irx baseline | irx_sync K=0.5 | delta |
|--------|-------------|----------------|-------|
| fitness | 0.099 | **0.129** | **+0.030 regression** |
| transfer_score | 0.836 | **0.637** | **−0.199** |
| carrier_emergence | 0.935 | **0.744** | **−0.191** |
| xi_robustness_v2 | 0.559 | **0.798** | **+0.239** ← as predicted |
| magic_proxy_phase_R | 0.617 | **0.084** | −0.533 |
| query_gravity | 0.363 | 0.430 | +0.067 |

xi improved dramatically as predicted, but carrier_e dropped substantially (contrary to prediction) and transfer dropped sharply.

### Trial 2: K=0.1 (weak coupling, expected to preserve more irx structure)

`DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=irx_sync KURAMOTO_COUPLING=0.1`

| metric | irx baseline | irx_sync K=0.1 | delta |
|--------|-------------|----------------|-------|
| fitness | 0.099 | **0.198** | **+0.099 regression** |
| transfer_score | 0.836 | **0.436** | **−0.400** |
| carrier_emergence | 0.935 | **0.660** | **−0.275** |
| xi_robustness_v2 | 0.559 | **0.609** | **+0.050** (barely) |
| magic_proxy_phase_R | 0.617 | 0.142 | −0.475 |
| query_gravity | 0.363 | 0.468 | +0.105 |

Counterintuitively, weaker K causes MORE damage to carrier_e and transfer, and provides much less xi benefit.

---

## Mechanism revealed

**Why carrier_e drops despite phase-only operations:**

The assumption that "stage_sync only modifies phases → carrier_e unaffected" is incorrect because carrier_e is a cross-cycle metric, not a within-cycle snapshot. Here's the actual mechanism:

1. irx aligns phases by constructive pairs (similarity-weighted circular mean of neighbors)
2. Kuramoto sync reorganizes phases by within-category coherence (frequency-range defined categories)
3. The resulting phase landscape after irx_sync is Kuramoto-dominated (it runs last)
4. On the **next dream cycle**, stage_detect finds constructive/destructive pairs based on the Kuramoto phase landscape
5. The Kuramoto phases are organized differently from irx's constructive-pair phases
6. stage_strengthen then amplifies a different set of memories, disrupting the 0.5 Hz carrier amplitude pattern

The carrier_e emerges from repeated strengthening of the same memories across dream cycles. When the phase landscape changes (by Kuramoto overriding irx), the strengthen targets change, destroying the carrier amplitude time series.

**Why weaker K (0.1) causes MORE damage than stronger K (0.5):**

- K=0.5: Kuramoto creates strong, consistent category clusters. The resulting phase landscape is organized (just differently from irx). This organized landscape enables some carrier structure to re-emerge in subsequent cycles.
- K=0.1: Kuramoto creates partial, inconsistent phase nudges. The resulting phase landscape is disorganized — neither irx's pair-coherent landscape nor Kuramoto's category-coherent landscape. Disorganized phases → chaotic strengthen targets → maximum carrier disruption.

This is the "partial disorder is worse than ordered disruption" effect.

---

## Fitness decomposition

Using fitness ≈ Σ (1 − metric_i) × weight_i, the major contributors:

| config | xi contribution | carrier_e contribution | transfer contribution | approx total |
|--------|----------------|----------------------|----------------------|--------------|
| irx baseline | (1−0.559)×0.15 = **0.066** | (1−0.935)×0.10 = **0.007** | (1−0.836)×0.15 = **0.025** | ~0.098 |
| irx_sync K=0.5 | (1−0.798)×0.15 = **0.030** | (1−0.744)×0.10 = **0.026** | (1−0.637)×0.15 = **0.054** | ~0.110+ |
| irx_sync K=0.1 | (1−0.609)×0.15 = **0.059** | (1−0.660)×0.10 = **0.034** | (1−0.436)×0.15 = **0.085** | ~0.178+ |

The three major metrics account for most of the regression. Additionally, temporal_separation likely regresses (it's a major metric at weight 0.15) because Kuramoto's phase reorganization disrupts the temporal layer separation irx creates.

---

## Decision

Code change reverted. Hypothesis falsified.

**Key structural insight:** The irx and Kuramoto modes are **competing**, not complementary. Each mode creates a specific phase landscape optimized for different properties (irx: constructive-pair coherence → transfer+carrier; Kuramoto: category coherence → xi). Running them sequentially means the last stage overwrites the earlier stage's organization, defeating the purpose of the earlier stage.

The irx–Kuramoto trade-off is architectural. You cannot combine the best metrics from both modes by sequential application.

---

## Empirical optima unchanged

- `DRIVE_A=0.1 DREAM_MODE=interference_relax DRIVE_SCOPE=all` → avg fitness **0.099** (best known)
- `DRIVE_A=0.15 DRIVE_SCOPE=all` (code defaults, K=0.5) → avg fitness **0.104**

---

## Remaining open questions

1. **stage_hallucinate parameters**: completely unexplored. Controlling bridge generation between clusters could affect transfer under stage_sync.
2. **stage_boost_prune thresholds**: unexplored, could affect which memories survive to contribute to carrier and transfer.
3. **irx xi variance**: the 0.256–0.874 range under irx means the avg 0.559 may be improvable via mechanism changes (not phase-stage stacking). Seeding the adversarial test would clarify.
4. **The irx–Kuramoto trade-off itself**: Are there intermediate-architecture approaches that don't run the modes sequentially but blend their objectives? (e.g., a mixed objective function for stage_detect's pairing criteria)
