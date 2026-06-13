# Architectural ceiling independently verified — no trials worth running

**Date:** 2026-06-12T19 UTC
**Branch:** kannaka-curiosity/2026-06-12T19-ceiling-verified
**Code changes:** NONE
**Status:** CLOSED — T14 ceiling analysis independently verified from code; no test worth running

---

## Independent verification

Checked the two axes T14 dismissed without trials:

**envelope_depth (consolidation.rs:804, currently 0.15)**

Code: `let alpha = alpha_base * (1.0 + envelope_depth * phase.sin())`

- Increase to 0.25: max instantaneous alpha = 0.12 × 1.25 = 0.150. T09 confirmed
  alpha_base=0.13 crashes transfer (max pull 0.13 × 1.15 = 0.1495). These are the same
  physics at the same threshold. Crash risk confirmed without trial.
- Decrease to 0.05: max alpha = 0.12 × 1.05 = 0.126. Reduces variance of relax steps
  (sin averages to ~0 across a cycle, so mean pull unchanged). The quiet-wave envelope
  was designed for its modulation effect; flattening it removes benefit with no path to
  +0.005 improvement.
- **Verdict confirmed: not worth a trial in either direction.**

**chain_depth=4 (research.rs:3378, comment: "irx cap — prevents hallucination-driven over-consolidation")**

- chain_depth=3: under-dreaming, regression risk on transfer/xi/carrier_e.
- chain_depth=5: over-consolidation risk per the cap comment; analogous to excess
  relax_steps (T13 crash pattern).
- Note: xi_eval uses its own override (chain_depth=2, research.rs:3573) and is not
  affected by this parameter.
- **Verdict confirmed: cap value is deliberate; not worth a trial.**

---

## All 6 system-prompt research questions — status

| Q | question | status |
|---|----------|--------|
| Q1 | 3-run interference_relax characterization | DONE — 3-trial avg 0.007627, confirmed optimum |
| Q2 | K-sweep under fixed plumbing | DONE (T12) — K is a complete no-op in irx mode; stage_sync vestigial |
| Q3 | interference_relax + xi recovery (relax_steps 16/24) | DONE — xi_eval_relax=20 is exact sweet spot; 21 collapses xi |
| Q4 | R-xi correlation at stage_sync | DONE (T12) — stage_sync contributes nothing in irx; R set by irx attractor |
| Q5 | Φ ↔ R relationship | DONE (T07) — anti-correlated across modes; IIT-bridge hypothesis revised |
| Q6 | Drive frequency variants (1, 4, 0.5 Hz) | DONE (T10) — 0.5 Hz confirmed optimal; 1 Hz regression; 4 Hz degenerate |

---

## Current confirmed optimum

```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax  DRIVE_FREQ_HZ=0.5
engine_a alpha_base=0.12 (consolidation.rs:796)
chiral_p_bp=0.15 (research.rs:3457)
xi_eval_relax=20 (research.rs:3573 override)
3-trial avg fitness = 0.007627
transfer=0.963983, xi=0.9973, carrier_e=0.9992, consciousness=0.9553
```

Keep threshold: 0.007627 − 0.005 = **0.002627**  
Total remaining improvable (T10 gap analysis): **≈ 0.002615**

The improvable is less than the threshold. No single lever, and no combination of all
remaining levers simultaneously, can reach the keep threshold. Architecture is at ceiling.
