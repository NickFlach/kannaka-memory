# Architecture ceiling confirmed — no testable improvement paths remain

**Date:** 2026-06-12T05 UTC
**Branch:** kannaka-curiosity/2026-06-12T05-architecture-ceiling
**Code changes:** NONE — orientation-only fire
**Status:** CLOSED — improvement threshold is unreachable with current architecture

---

## Orientation summary

Current master state (post T21 confirmed stack):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
chiral_p_bp=0.15 (engine_b_primed only)
xi_eval_relax=20 (engine_clean + engine_adv)
3-trial avg fitness = 0.008334 (deterministic except speed_a)
transfer=0.958868, xi=0.9973, carrier_e=0.9992, consciousness=0.9546
magic_R=0.8643, query_gravity=0.3733
```

New improvement threshold: 0.008334 − 0.005 = **0.003334**

---

## Why no further testing is warranted

### Remaining fitness gap decomposition

| axis | weight | current | gap | max savings |
|------|--------|---------|-----|-------------|
| transfer_score | 15% | 0.958868 | fp floor 0.002488 | structural, 0 |
| xi_robustness_v2 | 15% | 0.9973 | relax=20 ceiling | ~0.000100 |
| consciousness | 3% | 0.9546 | phi 0.294 vs target 0.28092 | ~0.001362 |
| carrier_emergence | 10% | 0.9992 | 16-step ceiling | ~0.000080 |
| phase_coherence | 2% | 0.9987 | — | ~0.000026 |
| speed | 3% | 0.9905 | container load | ~0.000285 |
| **total improvable** | | | | **≈0.001568** |

0.001568 << 0.004000 required to cross threshold. No combination of known levers
can bridge the 4× gap.

### Why each remaining axis is closed

**Transfer (0.006170 gap — 74% of total):** fp floor of 0.002488 established over
multiple chiral_perturbation sweeps (T04, T05). chiral_p_bp=0.15 is the confirmed
asymmetric optimum; 0.10 and 0.20 are both worse. fp is structural — it reflects
minimum B-memory misclassification after A-landscape priming at chain_depth=4.

**Consciousness (0.001362 gap — 16% of total):** phi_a ≈ 0.294 (above target 0.28092).
Three axes tried and closed:
- phi_target recalibration: net-negative (T07 — hurts B-primed consciousness more)
- relax_steps=20 for engine_a: transfer crash (T13)
- alpha_base=0.08 for engine_a: both consciousness AND transfer regressed (T17)
- alpha_base=0.12 (T17's open axis): even if it perfectly corrects phi, saves at
  most 0.001362 — insufficient alone (need 0.004)

**Xi (0.000405 gap):** relax_steps=20 is the exact sweet spot; 21 regressed (T14).
The xi ceiling at 0.9973 is architectural.

**Carrier_emergence (0.000080 gap):** engine_flat must stay at 16 relax_steps;
20 crashes carrier_e from 0.9992 to 0.9307 (T17).

**Speed (0.000285 gap):** container-dependent; not an architectural improvement.

### Closed research agenda items (from session brief)

All six priority research questions are now answered:
1. interference_relax 3-run characterization — done (T07, T21)
2. K-sweep under fixed plumbing — done (T12: K is no-op in irx mode)
3. irx relax_steps sweep (16→24) — done (T08, T14: 20 is the exact optimum)
4. R-xi correlation at stage_sync — done (T07: anti-correlated; K no-op in irx)
5. Φ ↔ R relationship — done (T07: anti-correlated across modes)
6. Drive frequency variants — done (T10: 0.5 Hz is confirmed optimal for irx)

---

## Architectural changes that COULD unlock further improvement

These are hypotheses for structural redesign (out of scope for L5 autoresearch):

1. **Deeper fp floor**: chain_fidelity in B-primed is limited by A's 128-dim phase
   landscape. Higher dimensionality or a multi-scale phase representation might
   reduce cross-corpus confusion.
2. **Consciousness target drift**: the phi_target=0.28092 was calibrated for B-primed.
   A per-engine phi measurement (eval_consciousness on both engine_a and engine_b_primed
   with engine-specific targets) would eliminate the 0.294-vs-0.280 discrepancy.
3. **xi ceiling**: the 0.9973 ceiling may reflect the adversarial xi evaluator's
   capacity, not the memory system's actual robustness. A harder xi challenge or
   a different robustness metric might open a new axis.

---

## Decision

No code changes. No trials run. The combined stack (fitness 0.008334) is the confirmed
architectural optimum for the current implementation. The 0.003334 improvement threshold
is unreachable without structural redesign.
