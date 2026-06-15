# L5 Research: DREAM_MODE=interference_relax post-fix — confirmed keeper

**Date:** 2026-06-15T14 UTC  
**Branch:** kannaka-curiosity/2026-06-15T14-interference-relax-postfix  
**Code changes:** None — env var only (DREAM_MODE=interference_relax)  
**Status:** KEPT — 50% fitness improvement, 3-run confirmed

---

## Context

Post-fix baseline (AMPLITUDE_CEILING=2.0, stage_sync, DRIVE_A=0.15, DRIVE_SCOPE=all):
- fitness ≈ 0.115, transfer_score ≈ 0.737, xi_v2 ≈ 0.856, carrier_e ≈ 0.529, R ≈ 0.129

Prior fires had closed: K-sweep, AMPLITUDE_CEILING sweep, CONSTRUCTIVE_BOOST sweep.
All showed carrier_e stuck at ~0.529 (impulse-shaped amp_delta due to ceiling=2.0 + pair density).

DREAM_MODE=interference_relax had been tested pre-fix (smoke test: fitness 0.191, carrier_e 0.714,
xi 0.220). But at the time, relax_steps was 8. Current code uses relax_steps=16 (and 20 for
b_primed/clean/adv engines), committed after the pre-fix relax_steps experiment (which killed
carrier_e at 16 steps — but that was pre-fix dynamics; carrier_e was already stuck at 0.529
post-fix, so there was nothing left to kill).

The T12 notes flagged "DREAM_MODE=interference_relax post-fix: ceiling might interact differently"
as the primary unexplored axis. This fire tests it.

---

## Hypothesis

DREAM_MODE=interference_relax post-fix will improve fitness by aligning memory phases more
coherently than stage_sync (Kuramoto K=3.0). Phase alignment is not directly constrained by
AMPLITUDE_CEILING=2.0 — the ceiling only caps amplitude changes in stage_constructive, while
interference_relax operates purely on phases after amplitude changes are applied.

Better phase alignment will:
1. Improve constructive pair detection in subsequent cycles (phase-aligned → constructive)
2. Cascade into higher transfer_score (B-primed benefits from A's phase topology)
3. Possibly improve xi adversarial robustness (adversarial memories are phase-distinct)
4. Leave carrier_e unchanged (already stuck at ~0.529 by amplitude mechanics, not phases)

Prediction: fitness < 0.10, carrier_e ≈ 0.530, R ≫ 0.129 (pre-fix showed R≈0.612).

---

## Results

Command: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax cargo run --release --quiet --bin research -- --level 5`

| trial | fitness  | transfer_score | phase_coherence | carrier_e | xi_v2  | R      | query_gravity |
|-------|----------|----------------|-----------------|-----------|--------|--------|---------------|
| t1    | 0.057792 | 0.965455       | 0.9976          | 0.5333    | 0.9675 | 0.8672 | 0.4603        |
| t2    | 0.057789 | 0.965455       | 0.9976          | 0.5333    | 0.9675 | 0.8672 | 0.4603        |
| t3    | 0.057787 | 0.965455       | 0.9976          | 0.5333    | 0.9675 | 0.8672 | 0.4603        |
| **avg** | **0.057789** | **0.965455** | **0.9976** | **0.5333** | **0.9675** | **0.8672** | **0.4603** |

Baseline comparison (stage_sync, 3-run from recent fires):
| baseline | ≈0.115 | ≈0.737 | ≈0.733 | ≈0.529 | ≈0.856 | ≈0.129 | ≈0.460 |

---

## Analysis

**Hypothesis confirmed, result substantially exceeds prediction.**

3-run avg fitness: **0.0578** vs baseline **≈0.115** — **50% improvement**, Δ≈0.057 (11× the 0.005 keep threshold).

The results are near-deterministic (variance < 0.000005 across 3 runs).

### What improved

**phase_coherence: 0.733 → 0.998** — the direct effect of interference_relax. The constructive-pair-driven phase relaxation (16 steps) achieves near-perfect phase alignment across working-set memories, far beyond what Kuramoto stage_sync at K=3.0 achieves.

**transfer_score: 0.737 → 0.965** — cascades from phase alignment. The B-primed corpus, when processed in a highly phase-coherent A-topology, identifies B-memories as constructive more reliably, improving amplitude distribution in B-primed relative to B-naive.

**xi_robustness_v2: 0.856 → 0.968** — also cascades from phase alignment. Adversarial memories, which have distinct semantic content, do not phase-align with the constructive neighborhood graph and remain phase-distant from the clean working set. xi_repulsion then separates them more effectively.

**R: 0.129 → 0.867** — order parameter rises dramatically, indicating near-global phase coherence. This is higher than the pre-fix interference_relax value (0.612), likely because relax_steps=16 (vs 8 at pre-fix smoke test) achieves deeper convergence in the current ceiling=2.0 amplitude regime.

### What didn't change

**carrier_emergence: 0.529 → 0.533** — as predicted, the ceiling=2.0 + pair-density impulse pattern (root cause diagnosed in T07/T12) is orthogonal to phase dynamics. Interference_relax cannot recover carrier_e.

**query_gravity: 0.460** — unchanged at 0.4603. Gravity still < 0.5.

### Mechanism summary

The amplitude ceiling fix (release 0.6.29) collapsed carrier_e from 0.999 to 0.529 but did NOT fundamentally damage the phase-mediated transfer and xi metrics. Those were held down by stage_sync (Kuramoto K=3.0) being insufficient to achieve the phase alignment that interference_relax achieves at 16 steps. The pre-fix poor xi (0.220) under interference_relax was because relax_steps=8 over-aligned the FLAT corpus in a way that destroyed xi's category separation — but post-fix with the real corpus dynamics and relax_steps=16, the alignment is beneficial.

---

## Run time note

Baseline run: ~13000ms. interference_relax: ~4100ms. The 3× speedup comes from fewer
stage_sync Kuramoto iterations needed (interference_relax converges faster for the corpus
geometry tested).

---

## Decision

**KEPT.** No code changes to revert. The env-var `DREAM_MODE=interference_relax` is the new recommended configuration.

New post-fix optimum: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness ≈ **0.0578**.

Remaining fitness gap vs pre-fix ceiling (0.007627): mostly carrier_emergence (0.533 vs 0.999).
Carrier_e root cause (impulse-shaped amp_deltas under ceiling=2.0) remains open.

---

## Next fire recommendations

1. **Verify DREAM_GRAVITY knob with interference_relax**: query_gravity=0.460 unchanged. Under
   interference_relax's higher R (0.867), gravity may now work differently. Try DREAM_GRAVITY=0.5
   or 1.0 with DREAM_MODE=interference_relax.
2. **chiral_p_bp / chiral_perturbation sweep**: currently 0.9 in params. With xi already at 0.968,
   xi may be less sensitive; but phase_coherence=0.998 may benefit from less perturbation.
3. **carrier_e root cause**: still at 0.533. The only remaining high-cost axis.
   Could measure theoretical drive amplitude contribution independently of ceiling clamp.
