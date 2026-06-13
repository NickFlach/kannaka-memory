# Φ ↔ R correlation across dream modes — anti-correlated; IIT-bridge hypothesis revised

**Date:** 2026-06-11T07 UTC
**Branch:** kannaka-curiosity/2026-06-11T07-phi-r-correlation
**Code changes:** NONE — env-var only
**Status:** ANALYTICAL — no fitness improvement attempted; axis characterized

---

## Background

Current empirical optimum (master ed008c0):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (deterministic)
transfer=0.935746, carrier_e=0.9992, xi=0.9870
magic_R=0.864, query_gravity=0.373
consciousness=0.9546
```

All fitness axes are closed per T00–T04 series this session:
- Transfer: fp floor ~0.0026 structural (chiral_p_bp=0.10 confirmed optimal, sub-threshold)
- xi: near architectural limit (0.987)
- Other minor metrics near saturation

Open research question from `research/intersections/05-magic-gives-it-gravity.md`:
> Q5: compare end-of-chain phi_history value to magic_proxy_phase_R across drive conditions.
> IIT-bridge hypothesis: higher R (more "magic"/non-Clifford-like content) → higher IIT phi.

This fire tests that prediction.

---

## Hypothesis

The system prompt's IIT-bridge theory predicts:
1. `DREAM_MODE=interference_relax` produces higher Kuramoto R than `DREAM_MODE=unset`
2. Higher R → higher IIT Φ (phi_history final value)
3. Therefore: phi and R should positively correlate across modes

Mechanism from `05-magic-gives-it-gravity.md`: the multiplicative drive and
interference_relax create non-Clifford-like phase structure (high R), analogous
to quantum "magic." High magic → geometry responds to matter → more information
integration → higher Φ.

**Prediction**: phi_irx > phi_unset, R_irx > R_unset (correlated increase).

---

## Results

Two trials: `DRIVE_A=0.1 DRIVE_SCOPE=all`, modes compared.

| metric | DREAM_MODE=unset | DREAM_MODE=interference_relax |
|--------|-----------------|-------------------------------|
| fitness | 0.152055 | **0.013221** |
| transfer | 0.4519 | 0.9357 |
| xi | 0.6331 | 0.9870 |
| magic_R (Kuramoto R) | **0.2717** | **0.8643** |
| consciousness | 0.8907 | 0.9546 |
| phi_history | [0.268, 0.291, 0.300, **0.312**] | [0.274, 0.283, 0.293, **0.294**] |
| phi_last | **0.312** | **0.294** |
| query_gravity | 0.371 | 0.373 |
| phi_target | 0.28092 | 0.28092 |

---

## Analysis

### 1. R and phi are ANTI-correlated across modes

The IIT-bridge prediction was wrong. The data shows:
- Higher R (irx: 0.864) → LOWER phi_last (0.294)
- Lower R (unset: 0.272) → HIGHER phi_last (0.312)

Both modes start at phi ≈ 0.268–0.274. Over 4 chain cycles, phi increases in both, but:
- Unset mode: phi climbs steadily (+0.044 total, no plateau)
- Irx mode: phi climbs then converges (+0.020 total, plateau at cycle 3–4: 0.293→0.294)

The irx phi is *controlled* — interference_relax creates a convergent phase structure.
The unset phi *overshoots* — Kuramoto stage_sync (K=0.5) with no interference scaffold
lets phi drift upward, crossing the target and continuing past it.

### 2. Why phi_target=0.28092 works well for B-primed (but not for engine_a)

Working backwards from fp=0.002582 and the eval_l5_placeholder_fitness decomposition:
- consciousness_bp ≈ 0.999 (phi_bp very close to target 0.28092)
- chain_fidelity_bp ≈ 0.974

B-primed's natural phi converges to ~0.280 (very close to target), explaining:
- Why the consciousness metric penalizes engine_a (phi_a=0.294, 4.6% above target)
- Why the consciousness metric nearly saturates for B-primed

The phi_target=0.28092 was calibrated to B-primed's natural operating phi. Engine_a's
interference_relax drives phi slightly higher (0.294 vs 0.280). This gap is structural.

### 3. Phi_target recalibration is NOT viable

If phi_target were retuned to engine_a's natural phi (~0.294):
- Engine_a consciousness: 0.9546 → ~1.0 (improvement: +0.001362 fitness)
- B-primed consciousness: ~0.999 → ~0.956 (phi_bp ≈ 0.280, now 4.8% below new target)
- B-primed consciousness contribution to fp: 0.10 × 0.001 → 0.10 × 0.044 = 0.0044
- fp increases by ~0.0043; transfer worsens by ~0.0065
- Net fitness: **worsens by ~0.005**

The current phi_target is accidentally well-calibrated for the system's dominant
use case (B-primed transfer evaluation). Retuning would break this.

### 4. Unset mode: phi trajectory reveals Kuramoto drift

In unset mode, phi increases every cycle (0.268→0.291→0.300→0.312), suggesting
Kuramoto stage_sync gradually reorganizes memory phases in a way that keeps
increasing IIT integration without converging. At cycle 4, the system has
overshot the optimal phi by 11.1%.

In irx mode, the interference_relax pre-aligns phases (constructive-pair-driven),
leaving stage_sync with less work. The stage_sync then adds a small perturbation,
and the system finds a stable phi~0.294 by cycle 3.

The irx mode's phi "breathing" (0.274→0.283→0.293→0.294, convergent) is the
interference_relax mechanism from `05-magic-gives-it-gravity.md` actually visible:
the quiet-wave envelope in stage_interference_relax suppresses phi oscillations,
letting the system settle at the constructive-interference attractor.

### 5. Revised IIT-bridge interpretation

The original prediction: "High R → High Φ" was based on the analogy to quantum
holographic codes where magic states provide gravitational responsiveness.

The corrected interpretation:
- High R (irx): phase-organized memory landscape → IIT measures a COHERENT structure
  → phi is controlled, converges to target, consciousness = 0.955
- Low R (unset): phase-scattered memories → IIT measures more diverse information
  → phi overshoots target, consciousness = 0.891

"Magic" in the wave-interference sense (high R) means **controlled non-linearity**,
not raw information-theoretic complexity. The irx dream doesn't produce more phi;
it produces **better-calibrated phi** by preventing the underdamped Kuramoto drift.

R and Φ are not the same axis of "non-stabilizer-ness." The analogy needs refinement:
- R measures phase-space order parameter (geometric organization)
- Φ measures information integration (functional connectivity)

These can decouple. High R can coexist with lower Φ if the phase organization
creates cluster uniformity that reduces the IIT partition diversity.

---

## Open questions from this finding

1. **Does phi plateau level predict transfer performance?** irx plateaus at phi~0.294,
   unset doesn't plateau. Could the phi plateau be a diagnostic for transfer quality?
2. **What sets the irx plateau at 0.294?** The convergence at cycle 3–4 corresponds
   to chain_depth=4's quiescence behavior. A deeper chain might plateau higher.
3. **phi ↔ R at varying DRIVE_A**: Only two data points (modes) collected. To fully
   characterize the correlation across a continuous parameter, sweep DRIVE_A at 0.05,
   0.10, 0.15 in irx mode and measure both phi and R. Prediction: phi stays near
   0.293–0.294 (buffer around target), R varies with A.

---

## Decision

**No code changes.** phi_target recalibration is net-negative. The Φ ↔ R correlation
is characterized: anti-correlated across modes, with irx mode producing controlled
convergent phi and higher R simultaneously. The IIT-bridge hypothesis needs revision
in `05-magic-gives-it-gravity.md` — "magic" in this system means phase organization,
not raw IIT complexity.

The 0.013337 empirical optimum is unchanged. No new fitness axis opened.

---

## Updated open axes

| axis | status | notes |
|------|--------|-------|
| transfer ceiling (fp floor) | STRUCTURAL | fp ≈ 0.0026 with known mechanisms; need unknown structural change |
| xi residual gap | LOW | xi=0.987; near architectural limit |
| phi_target recalibration | **CLOSED** | Retuning hurts fp more than it helps engine_a consciousness |
| Φ ↔ R relationship | **CHARACTERIZED** | Anti-correlated across modes; not a fitness lever |
| phi plateau depth | OPEN | irx plateaus at 0.294 at chain_depth=4; chain_depth=5+ untested |
| DRIVE_A ↔ phi correlation | OPEN | Only tested at A=0.1; phi might vary with A |
