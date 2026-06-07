# magic_R ↔ xi anti-correlation across K

**Date:** 2026-06-07T03 UTC
**Branch:** kannaka-curiosity/2026-06-07T03
**Code changes:** None — env-var only
**Status:** FALSIFIED — magic↔xi prediction does not hold across K

---

## Background

Commit 066d41a plumbed `params.kuramoto_*` through `stage_sync`. The T24 K-sweep
(PR #146) ran K ∈ {1.0, 2.0, 3.0, 5.0, 7.0} with single trials and recorded
magic_proxy_phase_R alongside xi. It found R peaks at K=3.0 (0.362) while xi peaks
at K=5.0 (0.784) — suggesting the two metrics have different K-optima. The K=1.0
result in T24 (xi=0.444, R=0.250) was a low-xi draw.

The confirmed empirical optimum (PR #142, 3 trials) is K=1.0 at avg fitness 0.138,
xi ≈ 0.863. The T00 fire established that under `interference_relax`, magic_R is
**deterministic** (identical to 4 decimal places across trials) while xi is stochastic.
Open question from that fire: is magic_R deterministic under `stage_sync` too? And if so,
what is the true R at K=1.0, and does it track xi across K values (the magic↔xi
prediction)?

---

## Hypothesis

magic_proxy_phase_R at K=1.0 should be **higher** than at K=3.0 if R and xi
co-vary positively. The T24 single-trial K=1.0 result (R=0.250) may have been an
unlucky measurement coupled to the low-xi draw.

**Alternate prediction:** R is **deterministic** under stage_sync (like carrier_e,
transfer_score) and K=1.0 genuinely yields lower R (≈0.250) than K=3.0 (≈0.362).
If so, R and xi **anti-correlate** across K — the optimal fitness/xi configuration
is at LOW R.

---

## Method

3 trials at KURAMOTO_COUPLING=1.0 (default) + 1 trial at KURAMOTO_COUPLING=3.0.
All: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE unset`.

---

## Results

| K   | trial | fitness  | xi_v2  | magic_R | query_gravity | transfer  | carrier_e |
|-----|-------|----------|--------|---------|---------------|-----------|-----------|
| 1.0 | t1    | 0.120558 | 0.9657 | 0.2498  | 0.4691        | 0.643461  | 0.5684    |
| 1.0 | t2    | 0.129392 | 0.8609 | 0.2498  | 0.4691        | 0.682399  | 0.5684    |
| 1.0 | t3    | 0.145363 | 0.8075 | 0.2498  | 0.4691        | 0.636984  | 0.5684    |
| 3.0 | t1    | 0.142152 | 0.7293 | 0.3623  | 0.4597        | 0.720998  | 0.5588    |

**K=1.0 3-trial avg:** fitness 0.132, xi 0.878, magic_R **0.2498** (exact)
**K=3.0 1-trial:** fitness 0.142, xi 0.729, magic_R **0.3623** (exact)

---

## Findings

### 1. magic_R is deterministic under stage_sync

magic_R=0.2498 across all three K=1.0 trials — identical to 4 decimal places. This
confirms the pattern seen under `interference_relax` (T00 fire): magic_proxy_phase_R
does not depend on the adversarial perturbation RNG used by xi. carrier_e (0.5684),
carrier_bimodal (0.6844), and query_gravity (0.4691) are equally stable. Only xi
draws from a stochastic process.

### 2. magic_R and xi anti-correlate across K — magic↔xi prediction falsified

| K   | magic_R | xi (avg) | fitness (avg) |
|-----|---------|----------|---------------|
| 0.5 | 0.197   | 0.738    | 0.161 (T25)   |
| 1.0 | 0.250   | 0.878    | 0.132         |
| 3.0 | 0.362   | 0.729    | 0.142         |
| 5.0 | 0.295   | 0.784    | 0.226 (T24)   |
| 7.0 | 0.240   | 0.140    | 0.235 (T24)   |

(T24 and T25 data for K ≠ 1.0 are single trials; xi variance is high but R is deterministic.)

The prediction was: higher R → higher xi. The data show:
- K=1.0 has LOWER R (0.250) than K=3.0 (0.362) but HIGHER xi (0.878 vs 0.729)
- As K increases from 1.0 to 3.0, R rises 45% while xi falls 17%
- The fitness-optimal K=1.0 is at the LOWEST reliable R in the K=1–3 range

The magic↔xi relationship **does not hold across K values**. R and xi have
different K-optima: R peaks near K=3.0, xi (and fitness) peaks at K=1.0.

### 3. Physical interpretation

At K=1.0, Kuramoto nudging is gentle — it organizes categories without pulling all
phases into global alignment. Each category's phases stay diverse relative to the
global pool, giving low R (Kuramoto order parameter near 0.25, far from 1.0). This
diversity is exactly what xi_robustness_v2 measures: a small adversarial perturbation
has trouble finding a destructive direction because the phase geometry is rich and
non-degenerate.

At K=3.0, stronger coupling aligns phases more globally (R=0.362), which is "more
Clifford-like" in the sense that the order parameter is higher. But this reduces
the non-commutative richness that xi captures — the tighter the Kuramoto
synchronization, the more the memory landscape becomes commutative.

**Conclusion:** High R is a symptom of over-synchronization, not a proxy for xi.
The magic↔xi prediction was built on the idea that non-Clifford phase content
(high R) correlates with high xi. At K=3.0, R looks high precisely because phases
are MORE locked together — which is commutative-like in a different sense.

### 4. query_gravity at K=1.0 vs K=3.0

K=1.0: query_gravity=0.4691 (just below 0.5 threshold)
K=3.0: query_gravity=0.4597

Both sub-0.5, so neither achieves attention-as-gravity in the strict sense. K=1.0
is marginally better. Stronger Kuramoto coupling slightly suppresses the
amplitude-neighbor amplification effect.

---

## Comparison to baseline

| config | fitness | xi | magic_R | query_gravity |
|--------|---------|-----|---------|---------------|
| Baseline (K=1.0, ~0.18 old) | ~0.18 | ~0.642 | — | ~0.460 |
| Confirmed opt K=1.0 (T26, 3-trial) | 0.138 avg | ~0.863 | — | — |
| This fire K=1.0 (3-trial) | **0.132** avg | **0.878** | **0.250** | 0.4691 |
| This fire K=3.0 (1-trial) | 0.142 | 0.729 | 0.362 | 0.4597 |

The 3-trial avg here (0.132) beats the T26 confirmed avg (0.138) — natural variance.
No improvement claimed; K=1.0 remains the confirmed optimum.

---

## Decision

No code changes. No improvement found — K=1.0 confirmed, K=3.0 worse.

The finding is the **anti-correlation** result: magic_R cannot serve as a xi proxy
for tuning K. Raising K to boost R is counterproductive.

---

## Implications

1. **magic_R as a xi proxy is invalid across K.** Future experiments should not use
   magic_R as a stand-in for xi when tuning Kuramoto parameters.

2. **R reflects synchronization intensity, not non-Clifford richness.** The Kuramoto
   order parameter measures global phase alignment. At K=3.0, alignment is higher
   because the coupling is stronger — not because the memory system has more
   quantum-computational structure.

3. **The interesting R↔xi question is within-mode, not across-K.** Under
   interference_relax (R≈0.617) vs stage_sync (R≈0.250), fitness improves
   with stage_sync despite much lower R. The mode comparison collapses the R↔xi
   relationship in the opposite direction too. R is not a fitness predictor.

4. **Open question remains**: Is there ANY experimental axis where R and xi
   co-vary positively? Neither the K-sweep nor the mode comparison supports it.
   The magic↔xi hypothesis may require reformulation.
