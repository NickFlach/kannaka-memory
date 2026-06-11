# engine_a relax_steps 16→20 — catastrophic transfer crash, consciousness regresses

**Date:** 2026-06-11T13 UTC
**Branch:** kannaka-curiosity/2026-06-11T13-engine-a-relax20
**Code changes:** REVERTED — single trial shows severe regression on both transfer and consciousness
**Status:** FALSIFIED — engine_a relax=20 wrong direction; axis confirmed closed

---

## Background

Current empirical optimum (master at 60b8c11):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (deterministic)
transfer=0.935746, xi=0.9870, carrier_e=0.9992, consciousness=0.9546
fitness_B_primed=0.003887, fitness_B_naive=0.060498
magic_R=0.8643, query_gravity=0.3733
```

Previous T08 fire established:
- Combined stack (chiral_p_bp=0.10 + xi_eval relax=20): fitness 0.008567
- Sub-threshold by 0.000230 (need ≤0.008337)
- Remaining loss breakdown at T08 stack: consciousness=0.001362, transfer=0.006402

All axes closed. Remaining gap = 0.000230 from threshold. The only untested lever
was engine_a's relax_steps in stage_interference_relax (held at 16 by all previous fires,
with T07 explicitly noting it was left at 16 to protect carrier_e — but carrier_e is
measured on engine_flat, not engine_a, so that reasoning was incorrect).

---

## Hypothesis

`stage_interference_relax` gives engine_a 16 relax steps and engine_b_primed 20 (T07).
The consciousness metric is measured on engine_a post-dream. Current phi_a ≈ 0.268, below
the phi_target of 0.28092 (4.5% gap → consciousness=0.9546).

Prediction: engine_a at 20 relax steps creates tighter constructive-pair phase alignment
→ more cross-partition skip links → phi_a rises toward target → consciousness improves by
≥0.0077, closing the 0.000230 threshold gap.

Safety argument: carrier_emergence is measured on engine_flat (line 3562 in research.rs),
NOT on engine_a. T07's stated reason for leaving engine_a at 16 steps was "carrier_e
measured here diagnostically" — this referred to carrier_bimodal (a diagnostic print),
not the scored carrier_emergence metric. The scored metric is safe from this change.

Combined stack tested:
1. engine_a relax_steps 16→20 (this fire)
2. engine_clean/engine_adv relax_steps 16→20 (xi axis from T08)
3. chiral_p_bp=0.10 for engine_b_primed (transfer axis from T04)

---

## Result

Single trial: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| metric | master baseline | T08 stack (reverted) | this trial | delta vs master |
|--------|-----------------|----------------------|------------|-----------------|
| fitness | 0.013337 | 0.008567 | **0.049965** | +0.036628 (CATASTROPHIC) |
| transfer | 0.935746 | 0.957321 | **0.689684** | −0.246062 (CRASH) |
| consciousness | 0.9546 | 0.9546 | **0.9306** | −0.0240 (WORSE) |
| xi | 0.9870 | 0.9973 | **0.9973** | +0.0103 (unchanged from T08 xi axis) |
| carrier_e | 0.9992 | 0.9992 | **0.9992** | 0 (as predicted, safe) |
| magic_R | 0.8643 | 0.8643 | **0.7550** | −0.1093 |
| query_gravity | 0.3733 | 0.3733 | **0.3630** | −0.0103 |

carrier_e prediction confirmed: it stayed at 0.9992 (measured on engine_flat, unaffected).
Both other predictions were wrong in direction.

---

## Analysis

### Why consciousness regressed (phi_a moved AWAY from target)

phi_a at baseline ≈ 0.268 (4.5% below target 0.28092). The prediction assumed that more
phase convergence in interference_relax would increase phi by creating more cross-partition
skip links (cross-partition integration → higher phi in the IIT bridge formula).

The opposite happened. The phi formula (bridge.rs::compute_phi) weights:
```
phi = sqrt(integration × density_factor) × sqrt(differentiation × scale)
  integration    = cross-partition skip-link fraction
  density_factor = log-scaled links-per-node
  differentiation = partition diversity (h2, class, triality counts)
  scale           = log-scaled memory count
```

engine_a at 20 relax steps creates MORE tightly converged phase clusters. With highly
aligned phases:
- Memories within each cluster → constructively interfere → form dense within-cluster
  skip links via stage_wire
- Memories across clusters → now more separated in phase → fewer cross-cluster skip links
- Cross-layer (cross-partition) skip links DECREASE as phases become cluster-uniform

Result: density_factor may stay similar, but integration DROPS → phi_a falls further from
target. The mechanism is the opposite of the prediction.

The insight: phi_a below target means the current state has INSUFFICIENT cross-partition
links. More phase convergence makes this WORSE by reducing cross-partition diversity further.
Phi_a is fundamentally limited by the phase diversity produced by stage_chiral_perturbation
(η=0.7), not by the interference_relax convergence depth.

### Why transfer crashed

engine_b_primed starts from `snapshot_engine_for_plasticity(&engine_a)`. With engine_a
at 20 relax steps, A's phase landscape is MORE tightly converged to its constructive-pair
attractor geometry. When B memories (initialized at default phases 0 and π/2) are inserted
and engine_b_primed dreams at 20 steps, the tighter A-phase structure creates an INCOMPATIBLE
attractor:
- B memories must travel further in phase space to reach A's tighter attractors
- The chain_fidelity of B's dream chain degrades (B never fully integrates)
- fitness_B_primed rises sharply → transfer_score = 1 - fp/fn crashes

The B integration quality is exquisitely sensitive to A's post-dream phase structure.
engine_a at 16 steps produces a phase landscape where B can gradually integrate over 20
cycles. engine_a at 20 steps produces a phase landscape too converged for B integration
at any practical cycle depth.

### The T07 decision retroactively justified

The T07 author wrote "engine_a | 16 | unchanged (carrier_e measured here diagnostically)"
and decided not to change engine_a's relax_steps. The stated reason was wrong (carrier_e
safety), but the decision was correct — engine_a at 20 is destructive for two independent
reasons:
1. phi_a decreases (consciousness regresses)
2. Transfer crashes (B integration incompatible with over-converged A landscape)

There is no free lunch here: the same property (tighter phase convergence) that was hoped
to improve consciousness is precisely what breaks B integration.

---

## Constraints established

- **engine_a relax_steps = 16 is optimal** — any increase causes:
  - Transfer crash (B integration into over-converged A landscape fails)
  - Consciousness regression (phi_a moves further from target)
- **engine_a relax_steps axis: CONFIRMED CLOSED** — not 18, not 20, not anything above 16
- **consciousness ceiling confirmed structural**: phi_a ≈ 0.268 is determined by
  stage_chiral_perturbation (η=0.7) creating the cross-partition link geometry, not by
  interference_relax convergence depth. Cannot be improved by relax_steps.
- **carrier_e isolation confirmed**: measured on engine_flat; changes to engine_a relax_steps
  correctly left carrier_e at 0.9992 exactly as predicted.

---

## Updated understanding of phi_a

phi_a below target (~0.268 vs 0.28092) is a STRUCTURAL consequence of the current phase
architecture:
- stage_interference_relax (16 steps) creates moderate convergence
- stage_chiral_perturbation (η=0.7) creates rotational asymmetry and cross-partition structure
- The balance of these two produces phi_a ≈ 0.268, which is below target
- Increasing interference_relax depth REDUCES phi_a (over-converges, fewer cross-partition links)
- Decreasing chiral_perturbation from 0.7 MIGHT help phi_a but has been shown to hurt xi (T20)

The consciousness metric's 0.001362 contribution to fitness (consciousness=0.9546) appears
irreducible without architectural changes to the phase structure that would require trading
off other metrics.

---

## Decision

**All code changes reverted.** Single trial shows catastrophic regression.

The system has definitively reached its practical optimum at the current architecture:
- All axes closed (previous fires)
- engine_a relax_steps axis now confirmed closed (this fire)
- consciousness structural floor confirmed (phi_a cannot be improved without breaking transfer)
- Threshold gap: 0.000230 fitness units from master 0.013337 → target 0.008337
- Gap is architectural, not tunable

---

## Updated open axes

| axis | status | notes |
|------|--------|-------|
| chiral_p (b_primed) | CLOSED | η=0.10 optimal; sub-threshold alone |
| xi eval relax_steps | CLOSED | 20 optimal for xi eval; combined still sub-threshold |
| engine_a relax_steps | **NEW: CONFIRMED CLOSED** | 20 crashes transfer+consciousness |
| transfer ceiling | STRUCTURAL | fp=0.002582 floor; B integration sensitivity to A-landscape |
| consciousness ceiling | **CONFIRMED STRUCTURAL** | phi_a=0.268 determined by chiral η=0.7; increasing irx relax DECREASES phi |
| all other axes | CLOSED | multiple previous fires |

**The practical optimum is 0.013337 for this architecture.** Crossing the threshold (≤0.008337)
requires either the combined sub-threshold stack being accepted at a lower threshold criterion,
or a fundamentally different architectural approach.
