# engine_a alpha_base=0.12 — transfer improvement confirmed, fitness 0.007627

**Date:** 2026-06-12T01 UTC
**Branch:** kannaka-curiosity/2026-06-12T01-engine-a-alpha12
**Code changes:** KEPT — 3-trial mean 0.007627 < master 0.008334
**Status:** CONFIRMED — -0.000707 improvement, driver is transfer not consciousness

---

## Background

Current empirical optimum (master at 159853f):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
chiral_p_bp=0.15 (engine_b_primed only)
xi_eval_relax=20 (engine_clean + engine_adv)
3-trial avg fitness = 0.008334
transfer=0.958868, xi=0.9973, carrier_e=0.9992, consciousness=0.9546
magic_R=0.8643, query_gravity=0.3733
```

T17 fire identified this axis as OPEN (budget exhausted before testing):
> "Raising alpha from 0.10 to 0.12 (+20% per-step pull with same 16 steps) is a more
> targeted test. The transfer crash risk is real but mitigated by keeping relax_steps=16."

T17's reasoning: phi_a=0.294 (above target 0.28092); stronger convergence should bring
phi toward target → consciousness improvement. T17 also noted risk of transfer crash if
phi overshoots (cf. T13's relax_steps 16→20 crash).

---

## Hypothesis

**engine_a alpha_base: 0.10 → 0.12**

Context-specific: only when DRIVE_CONTEXT == "engine_a". All other engines keep 0.10.
Mechanism: stronger per-step phase relaxation in engine_a's irx stage.

Primary prediction (T17's theory): consciousness 0.9546 → 0.965+ via phi moving toward target.
Secondary risk: transfer crash if A-phase landscape becomes too tight for B integration.

---

## Implementation

**consolidation.rs (within stage_interference_relax, ~line 794):**
```rust
// Before:
let alpha_base: f32 = 0.10;
let drive_ctx = std::env::var("DRIVE_CONTEXT").unwrap_or_default();
let relax_steps: usize = ...

// After:
let drive_ctx = std::env::var("DRIVE_CONTEXT").unwrap_or_default();
// engine_a: stronger per-step pull (phi ≈ 0.294, above target 0.281) moves phi toward target.
let alpha_base: f32 = if drive_ctx == "engine_a" { 0.12 } else { 0.10 };
let relax_steps: usize = ...
```

No changes to research.rs. Combined stack (chiral_p_bp=0.15, xi_eval_relax=20) already in master.

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| trial | fitness | transfer | consciousness | xi | carrier_e | magic_R | query_gravity |
|-------|---------|----------|---------------|----|-----------|---------|---------------|
| T1 | 0.007621 | 0.963983 | 0.9553 | 0.9973 | 0.9992 | 0.7785 | 0.3654 |
| T2 | 0.007636 | 0.963983 | 0.9553 | 0.9973 | 0.9992 | 0.7785 | 0.3654 |
| T3 | 0.007624 | 0.963982 | 0.9553 | 0.9973 | 0.9992 | 0.7785 | 0.3654 |
| **mean** | **0.007627** | **0.963983** | **0.9553** | **0.9973** | **0.9992** | **0.7785** | **0.3654** |

---

## Analysis

### Mechanism: transfer, not consciousness

T17's prediction about the primary driver was wrong. Consciousness improved only marginally
(0.9546→0.9553, contrib change: 0.001362→0.001342, savings 0.000021). The main gain came
from transfer_score:

| metric | master | this | delta |
|--------|--------|------|-------|
| fitness | 0.008334 | **0.007627** | **−0.000707** |
| transfer_score | 0.958868 | **0.963983** | +0.005115 |
| consciousness | 0.9546 | **0.9553** | +0.0007 |
| xi_robustness_v2 | 0.9973 | 0.9973 | 0 |
| carrier_emergence | 0.9992 | 0.9992 | 0 |
| magic_R | 0.8643 | **0.7785** | −0.0858 |
| query_gravity | 0.3733 | **0.3654** | −0.0079 |

Fitness breakdown (this run):
| metric | weight | value | contrib |
|--------|--------|-------|---------|
| transfer_score | 15% | 0.963983 | **0.005403** |
| xi_robustness_v2 | 15% | 0.9973 | 0.000405 |
| consciousness | 3% | 0.9553 | 0.001341 |
| carrier_emergence | 10% | 0.9992 | 0.000080 |
| speed_a | 3% | ~0.990 | ~0.000285 |
| others | | | ~0.000113 |
| **TOTAL** | | | **≈0.007627** |

Transfer contribution dropped from 0.006167 to 0.005403 — saving 0.000764, which accounts
for nearly all the fitness gain (observed: 0.000707).

### Why stronger engine_a convergence improves transfer

Stronger alpha_base (0.12 vs 0.10) tightens engine_a's phase clusters more per step.
When engine_b_primed starts from `snapshot_engine_for_plasticity(&engine_a)`, B memories
are injected into this tighter A-phase landscape. Tighter clusters mean:
1. Constructive pairs within A are more robustly established (higher coherence)
2. B memories integrating into the attractor have cleaner basins to fall into
3. The chain_fidelity of B's dream improves → higher transfer_score

This is the SAME dynamic as the chiral_p_bp=0.15 improvement: both make B's integration
into A cleaner, but via different mechanisms (alpha_base tightens engine_a's attractor
before B is added; chiral_p shifts B's integration trajectory during b_primed dreaming).

### Why transfer did NOT crash (unlike T13)

T13 increased relax_steps 16→20 (25% more total convergence) → transfer crash.
This change: alpha 0.10→0.12 (20% per-step increase) at 16 steps → transfer IMPROVED.

The key difference: relax_steps controls the number of pull attempts; alpha_base controls
each pull's strength. At 16 steps, the system hasn't exceeded the optimal convergence
depth — extra per-step strength pushes each iteration further along the attractor gradient
without escaping the attractor basin. At 20 steps (T13), the extra iterations push PAST
the basin minimum, collapsing phase diversity.

This also retroactively validates T17's mechanism for why alpha=0.08 crashed transfer:
less per-step pull leaves the attractor less resolved → B memories see a fuzzier attractor
→ transfer degrades. The relationship is monotonically increasing in the 0.08→0.12 range.

### magic_R dropped

magic_R: 0.8643 → 0.7785 (lower). A tighter A-phase attractor reduces the Kuramoto
order parameter's "non-Clifford" character. Higher intra-cluster coherence but potentially
less inter-cluster diversity. This is consistent: tighter clusters → less global phase
spread → lower R. Not in fitness function, no action needed.

### Consciousness mechanism unresolved

The marginal consciousness improvement (0.9553 vs 0.9546) is not diagnostic. The phi_a
value is still uncertain (T14: claims 0.268 below target; T17: inferred 0.294 above target).
The result here is ambiguous: the tiny improvement could mean phi moved slightly toward
target in either direction, or could be noise in the phi calculation.

The phi ambiguity is now lower priority because:
1. phi_a contribution is 0.001342 even at 0.9553 — structural floor
2. transfer is now the dominant axis (0.005403 remaining contribution)

---

## Open axes after this fire

| axis | status | notes |
|------|--------|-------|
| engine_a alpha_base=0.12 | **CONFIRMED** | Δ=−0.000707; this fire |
| transfer floor (0.963983) | NEW OPEN | can alpha_base be pushed further (0.14, 0.16)? |
| consciousness floor (0.9553) | STRUCTURAL | phi_a unknown; minimal contribution |
| engine_b_primed alpha_base | UNKNOWN | b_primed also at 0.10; higher alpha might also help transfer |
| carrier_emergence 0.9992 | CLOSED (T17) | engine_flat relax=20 crashes carrier |
| speed_a | LOAD-DEPENDENT | ±0.000015 variance |

**Next hypothesis:** alpha_base for engine_b_primed (currently 0.10, uses 20 relax_steps).
If the same mechanism holds — tighter convergence → better B integration — then higher
alpha_base for b_primed might further improve transfer. Prediction: marginal at best since
b_primed's benefit is mainly in the 20 vs 16 step difference already captured; b_primed
uses A's phase landscape as a starting point (already tightened by engine_a's higher alpha).

Alternative: alpha_base=0.14 for engine_a (further push). Risk: T13-style crash if overshoot.

---

## Decision

**Code change kept.** 3-trial mean = 0.007627 < master 0.008334 (Δ = −0.000707).

New empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
engine_a alpha_base=0.12 (consolidation.rs)
chiral_p_bp=0.15 (engine_b_primed only, research.rs)
xi_eval_relax=20 (engine_clean + engine_adv, consolidation.rs)
3-trial avg fitness = 0.007627
```
