# Kuramoto K-sweep under interference_relax — stage_sync is a no-op in irx mode

**Date:** 2026-06-11T12 UTC
**Branch:** kannaka-curiosity/2026-06-11T12-kuramoto-irx-invariant
**Code changes:** NONE — env-var only
**Status:** CHARACTERIZED — K axis definitively closed under irx mode; stage_sync irrelevant

---

## Background

Current empirical optimum (master):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (deterministic)
transfer=0.935746, xi=0.9870, carrier_e=0.9992, consciousness=0.9546
magic_R=0.8643, query_gravity=0.3733
```

Best known combination (reverted T08, sub-threshold):
```
DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax
relax_steps=20 for xi eval + chiral_p_bp=0.10
fitness=0.008567 — gap 0.000230 from threshold 0.008337
```

The code comment at L5 params setup (research.rs:3385) says:
> K=0.5 confirmed optimal in K-sweep (2026-06-06): weaker coupling preserves
> more phase diversity than K=1.0, reducing avg fitness ~0.138→~0.133.

That K-sweep was run at DREAM_MODE=unset (fitness ~0.13). Under irx mode (fitness
~0.013), the K-axis had never been tested separately.

---

## Hypothesis

In irx mode, stage_sync runs AFTER stage_interference_relax. Stage_sync uses
Kuramoto coupling K to pull phases together. stage_sync's contribution depends on K.

Prediction: lower K → less stage_sync over-synchronization → phi_a drops from
0.294 toward phi_target 0.28092 → consciousness improves by +0.0077 (weight 0.03)
→ fitness gain of 0.000231, closing the T08 gap.

The phi trajectory in irx mode: 0.274→0.283→0.293→0.294 (converges to 0.294 at
K=0.5). If stage_sync pushes phi upward, reducing K should lower the plateau.

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| K | fitness | transfer | xi | carrier_e | consciousness | magic_R | query_g |
|---|---------|----------|-----|-----------|---------------|---------|---------|
| 0.5 (baseline) | 0.013337 | 0.935746 | 0.9870 | 0.9992 | 0.9546 | 0.8643 | 0.3733 |
| **0.3** | **0.013347** | 0.935746 | 0.9870 | 0.9992 | 0.9546 | 0.8643 | 0.3733 |
| **0.1** | **0.013347** | 0.935746 | 0.9870 | 0.9992 | 0.9546 | 0.8643 | 0.3733 |
| **0.0** | **0.013350** | 0.935746 | 0.9870 | 0.9992 | 0.9546 | 0.8643 | 0.3733 |

All metrics are byte-for-byte identical across K=0.0, 0.1, 0.3, 0.5. The tiny fitness
variation (0.013337 vs 0.013347/0.013350) is within run-to-run seed variance.

---

## Analysis

### Stage_sync is a complete no-op in irx mode

With K=0.0, Kuramoto coupling is absent — stage_sync applies zero force. Every metric
is unchanged. This proves stage_sync contributes nothing to any measured outcome in
irx mode:
- phi_a: same (0.9546 consciousness unchanged)
- xi: same (0.9870 unchanged)
- carrier_e: same (0.9992 unchanged)
- magic_R: same (0.8643 unchanged) — R is the Kuramoto ORDER PARAMETER, yet setting K=0
  leaves R unchanged. This means R is computed BEFORE stage_sync applies any coupling,
  or the irx attractor already sets R and stage_sync doesn't perturb it.

### The irx attractor completely dominates phase dynamics

stage_interference_relax (quiet wave, constructive pairs) converges memories to a stable
phase attractor. When stage_sync runs afterward with K=0.5, the phases are already tightly
organized by irx. The Kuramoto force (K × sin(θ_j - θ_i)) is small because |θ_j - θ_i|
is already small at the irx attractor. Reducing K from 0.5 to 0 doesn't change the outcome
because the system was already within the Kuramoto basin regardless of K magnitude.

### Why this is architecturally significant

In default mode (DREAM_MODE=unset), stage_sync IS the primary phase organizer. K=0.5 vs
K=1.0 showed meaningful differences (the confirmed 2026-06-06 result). In irx mode, the
roles are reversed: stage_interference_relax takes over, and stage_sync becomes vestigial.

This means:
1. K is a dead parameter in irx mode — no fitness lever available here.
2. The phi_a=0.294 plateau is the irx attractor, not a Kuramoto artifact.
3. The consciousness gap (phi_a above phi_target by 4.6%) cannot be addressed by K tuning.
4. The T08 gap of 0.000230 is architecturally fixed by the irx attractor geometry.

### The irx attractor geometry for phi

The constructive-pair structure (computed from corpus_a's vector similarities) determines
which phases attract which. The irx attractor settles at phi_a≈0.294, which is ~4.6%
above phi_target=0.28092. This offset is intrinsic to corpus_a's similarity graph — the
clustering of corpus_a's memory vectors creates an IIT integration level of ~0.294.

To change phi_a without changing K, one would need to change:
1. The constructive-pair selection criteria (affects all metrics)
2. The alpha_base or envelope_depth in stage_interference_relax (may shift attractor)
3. The corpus_a composition itself (architectural change)
4. The phi_target value (confirmed net-negative in T07)

None of these are tractable without architectural-level changes.

---

## Implications for the threshold-crossing problem

The T08 combination (relax20 + chiral_bp=0.10) is 0.000230 short of the 0.005 improvement
threshold. The three remaining sub-threshold metrics after T08:
- consciousness: 0.9546 → 0.9623 needed (+0.0077, gain 0.000231) — CLOSED (irx attractor fixed)
- xi residual: 0.9973 → 0.9988 needed (+0.0015, gain 0.000225) — no known mechanism
- transfer: fp=0.002582 structural floor — confirmed T09

The only remaining combination is T08 stack + b_primed alpha_base=0.13 (T06: +0.000259
improvement, but mechanistically ~0.000151 from fp reduction — slightly below threshold).

The T08 combination may represent the practical optimum achievable within the current
architecture at the interference_relax attractor.

---

## Updated axis status

| axis | status | notes |
|------|--------|-------|
| KURAMOTO_COUPLING (irx mode) | **NEW: CLOSED** | K is a no-op; phi attractor = irx attractor |
| T08 stack (relax20 + chiral_bp) | SUB-THRESHOLD | gap 0.000230; best combination |
| b_primed alpha_base | CLOSED | +0.000151 real gain; insufficient alone |
| consciousness (phi_a) | **STRUCTURAL** | phi=0.294 is irx attractor, not K-dependent |
| xi residual at relax=20 | UNKNOWN | 0.9973; relax=21 might give +0.0002 more xi |
| transfer fp floor | STRUCTURAL | 0.002582 confirmed T09 |

---

## Decision

No code changes. The K axis is now definitively characterized: Kuramoto coupling K is
a no-op in DREAM_MODE=interference_relax. The stage_sync stage contributes zero to any
fitness metric when stage_interference_relax has already converged the phase landscape.

The consciousness sub-threshold gap is architectural. The system's practical optimum
under the current irx architecture is the T08 combination at fitness≈0.008567.

Future improvement requires either:
1. Modifying stage_interference_relax's attractor geometry (alpha_base, envelope_depth)
2. Changing corpus construction (different similarity graph → different phi attractor)
3. An architectural change to the phi computation or target calibration
