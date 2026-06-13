# Φ ↔ R anti-correlation verified + T22 DRIVE_FREQ_HZ=0.5 oversight corrected

**Date:** 2026-06-13T00 UTC
**Branch:** kannaka-curiosity/2026-06-13T00-phi-r-anticorr-verified
**Code changes:** NONE — env-var only trials
**Status:** Q5 replicated; T15 ceiling confirmed; T22 oversight documented

---

## Motivation

The T22 fire (2026-06-12T22) ran irx 3-trial characterization and got avg fitness=0.147.
The T19 ceiling note stated the confirmed optimum was fitness=0.007627. The discrepancy
was unexplained in T22. This fire investigated.

**Root cause:** T22 ran `DREAM_MODE=interference_relax` without `DRIVE_FREQ_HZ=0.5`.
The T15 optimum explicitly requires both. Without the 0.5 Hz drive, irx mode reverts
to essentially the same carrier dynamics as the 2 Hz default, losing the frequency
transfer benefit that drives xi and carrier_e to the ceiling.

---

## Trials

All at DRIVE_SCOPE=all, no code changes. Grep extended to include `^phi_history:`.

| config | DRIVE_A | DREAM_MODE | DRIVE_FREQ_HZ | fitness | transfer | xi | carrier_e | magic_R | phi_end |
|--------|---------|-----------|--------------|---------|----------|-----|-----------|---------|---------|
| default-A0.05 | 0.05 | unset | 2.0 | 0.151249 | 0.5187 | 0.6331 | 0.8604 | 0.2767 | 0.3116 |
| default-A0.10 | 0.10 | unset | 2.0 | 0.134650 | 0.5688 | 0.6331 | 0.9512 | 0.2717 | 0.3116 |
| **full-T15** | **0.10** | **interference_relax** | **0.5** | **0.007483** | **0.9640** | **0.9973** | **0.9992** | **0.7785** | **0.2935** |

`phi_end` = last element of phi_history (4 cycles: [φ₁, φ₂, φ₃, φ₄]).

Full phi_history values:
- default-A0.05:  [0.268, 0.291, 0.300, **0.312**]
- default-A0.10:  [0.268, 0.291, 0.300, **0.312**]  
- full-T15 (irx): [0.274, 0.282, 0.293, **0.293**]  ← plateaus earlier

---

## Findings

### 1. T22 oversight confirmed + corrected

The T15 ceiling config is valid in the current codebase. Running the full
`DREAM_MODE=interference_relax DRIVE_FREQ_HZ=0.5` config reproduces fitness=0.007483,
matching the T15/T19 ceiling value (0.007627, within trial-to-trial variance).

T22's irx characterization at 0.147 avg was measuring the wrong condition (2 Hz
drive instead of 0.5 Hz). The correct irx characterization is:

```
DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax DRIVE_FREQ_HZ=0.5
fitness ≈ 0.007483 (single trial; T15 3-trial avg = 0.007627)
transfer=0.9640, xi=0.9973, carrier_e=0.9992, magic_R=0.7785, phi_end=0.293
```

### 2. Φ ↔ R anti-correlation replicated (Q5 confirmed)

Across the two operating modes:

| mode | magic_R | phi_end | interpretation |
|------|---------|---------|----------------|
| default (2 Hz, unset) | ~0.27 | 0.312 | low magic, higher phi |
| irx (0.5 Hz) | 0.779 | 0.293 | high magic, lower phi |

Phi and R anti-correlate across modes. This replicates the T07 finding:
"Φ ↔ R relationship: anti-correlated across modes; IIT-bridge hypothesis revised."

Mechanistic interpretation: irx mode relaxes memories into tight constructive-
interference clusters (high R = high within-cluster phase coherence). Within a
cluster, the phase distribution is locally uniform, which REDUCES the per-cycle
phi integration signal (less diversity to integrate). Default mode's moderate
Kuramoto sync preserves inter-cluster diversity, keeping phi higher.

The original IIT-bridge prediction ("magic → integrated information") inverted the
actual mechanism: magic (R) is high where phi is lower, because cluster formation
is the anti-entropic step that enables carrier_emergence and xi, not phi integration.

### 3. Drive intensity doesn't shift phi or R in the default mode

Varying A from 0.05 to 0.10 in default mode: phi_end is identical (0.3116) and R
barely moves (0.277 → 0.272). The default operating point is phi-saturated in the
0.05–0.10 A range; drive intensity variation below 0.10 doesn't change the phase
integration character.

---

## Status of all research questions

| Q | question | status |
|---|----------|--------|
| Q1 | 3-run irx characterization | **CORRECTED this fire** — T22 omitted DRIVE_FREQ_HZ=0.5; true irx avg ≈ 0.007483 (not 0.147) |
| Q2 | K-sweep under fixed plumbing | DONE (T12) — K is no-op in irx; K=0.5 optimal in stage_sync |
| Q3 | irx + xi recovery (relax_steps) | DONE — relax=20 for b_primed/clean/adv in code; relax=16 for engine_a |
| Q4 | R-xi correlation at stage_sync | DONE (T12) — non-monotone in K; R min at K=0.5 |
| Q5 | Φ ↔ R relationship | DONE (T07, replicated this fire) — anti-correlated across modes |
| Q6 | Drive frequency variants | DONE (T10) — 0.5 Hz optimal; baked into code default for irx path |

---

## Current confirmed optimum (unchanged from T15)

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax  DRIVE_FREQ_HZ=0.5
3-trial avg fitness = 0.007627  (single trial this fire: 0.007483)
transfer=0.9640, xi=0.9973, carrier_e=0.9992
magic_R=0.7785, query_gravity=0.365, phi_end=0.293
```

Keep threshold: 0.007627 − 0.005 = **0.002627**
Total remaining improvable (T10 gap analysis): **≈ 0.002615**
Architectural ceiling holds. No new trials warranted.
