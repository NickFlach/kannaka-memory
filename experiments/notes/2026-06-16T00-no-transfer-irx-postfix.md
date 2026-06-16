# L5 Curiosity: DRIVE_SCOPE=no_transfer + interference_relax — axis closed

**Date:** 2026-06-16T00 UTC  
**Branch:** kannaka-curiosity/2026-06-16T00-no-transfer-irx-postfix  
**Code changes:** NONE — env-var only  
**Status:** NOT KEPT — sub-threshold, slightly worse than "all" scope baseline

---

## Context

Post-fix optimum: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness 0.057789  
(T14, T20, T20b all confirmed; 3-trial avg with near-zero variance).

This axis was explicitly flagged as "post-fix untested" by two consecutive fires (T14 and T20). The
pre-fix version of this combination showed xi collapse (xi→0.067), but that was before the
amplitude ceiling fix and before irx was the production mode. Post-fix dynamics with irx differ
substantially from the pre-fix regime.

---

## Hypothesis

Under irx, `DRIVE_SCOPE=no_transfer` (drives engine_a, engine_flat, engine_clean, engine_adv but NOT
engine_b_primed or engine_b_naive) might preserve transfer_score by not disturbing B's natural phase
distribution while maintaining the irx benefits for engine_a.

**Prediction:** Similar to "all" baseline (0.057789), possibly slightly better if driving B slightly
hurts its ability to absorb A's phase-aligned structure. The pre-fix xi collapse should NOT recur
because post-fix irx creates much stronger phase coherence (R=0.867) that protects xi from drive scope.

---

## Results

Command: `DRIVE_A=0.15 DRIVE_SCOPE=no_transfer DREAM_MODE=interference_relax cargo run --release --quiet --bin research -- --level 5`

| trial | fitness  | transfer_score | carrier_e | xi_v2  | R      | query_gravity |
|-------|----------|----------------|-----------|--------|--------|---------------|
| t1    | 0.058965 | 0.957577       | 0.5333    | 0.9675 | 0.8672 | 0.4603        |

Baseline (DRIVE_SCOPE=all, 3-trial avg):
| baseline | 0.057789 | 0.965455 | 0.5333 | 0.9675 | 0.8672 | 0.4603 |

---

## Analysis

**Fitness: 0.058965 vs 0.057789 — net WORSE by 0.001176.**

The pre-fix xi collapse (xi→0.067) did NOT recur. Post-fix irx achieves R=0.867, making the
working set nearly fully phase-aligned regardless of scope. The no_transfer scope had no adverse
effect on xi or carrier_e.

**Transfer regression: 0.965 → 0.958 (Δ−0.008) → 0.15 × 0.008 = 0.0012 fitness cost.**

Under "all" scope, engine_b_primed receives the drive (DRIVE_A=0.15). This slightly pre-amplifies
B's memories before consolidation, improving their amplitude standing relative to A's memories.
This marginally helps B integrate into A's phase topology → slightly better transfer than if B is
not driven. Under irx (phase-driven constructive detection), B's drive doesn't disrupt transfer the
way it did pre-irx under stage_sync (which was amplitude-order sensitive).

All other metrics (carrier_e, xi, R, query_gravity) byte-identical to "all" baseline. The scope
change only affects engine_b_primed and engine_b_naive, which don't contribute to these metrics.

---

## Decision

**No code changes. No improvements kept.**  
1 TSV row appended (labeled `L5`).  
Notes file committed. **Axis CLOSED.**

`DRIVE_SCOPE=all` remains the optimal scope with `DREAM_MODE=interference_relax`.

---

## Comprehensive carrier_e ceiling analysis (explored this fire, no trial needed)

This fire performed a full geometric analysis of carrier_e improvement paths. All are blocked:

**Root constraint:** chain_depth=4 + AMPLITUDE_CEILING=2.0 + dense constructive pairs (40+ per
memory under irx's high R=0.867 phase coherence) = immediate ceiling saturation in cycle 0 for
ALL memories. Amplitude_deltas signal is always impulse-shaped [~0.95, ~0, ~0, ~0] regardless of:
- Initial amplitude (pair density saturates any starting point in 1 cycle)
- Non-constructive decay (pairs dominate; non-constructive fraction <5% under irx)
- Drive frequency (constructive boost per cycle > drive negative effect)
- Phase alignment threshold changes (dense initial phase spread still creates many pairs)

**Specific paths analyzed and closed:**

| path | expected carrier_e | blocker |
|------|-------------------|---------|
| FLAT_INIT_AMP=0.2 | 0.53 (same) | 40+ pairs per memory, hits ceiling in cycle 0 regardless of initial amp |
| FLAT_DRIVE_FREQ_HZ=2.0 | 0.508 (WORSE) | constructive boost cancels negative drive; non-constructive <5% |
| AMPLITUDE_CEILING > 2.0 under irx | ~0.62 at ceiling=4.0, but transfer probably regresses | T03 showed transfer/carrier tradeoff; post-irx uncertain but expected sub-threshold net |
| chain_depth=16 for flat engine | injection periodicity at 2.67 Hz, not drive frequency | not a valid measurement of drive emergence |
| theoretical drive delta | carrier_e → 0.86 trivially | measures drive intent, not actual system emergence; scientifically invalid |
| post-constructive amplitude decay | small improvement; wrong mechanism | doesn't create oscillatory signal under always-positive 0.5 Hz drive |

**The carrier_e problem requires architectural change:** either (a) reduce pair density in the flat
corpus while preserving the flat-corpus emergence test semantics, or (b) raise AMPLITUDE_CEILING with
synchronized multi-parameter rebalancing (constructive_boost, chiral_p_bp, transfer protection) —
a ≥3-trial multi-fire investigation outside this fire's scope.

---

## Current optimum (confirmed stable)

`DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → **fitness 0.057789 (deterministic)**

All remaining axes exhausted at single-parameter level. Next improvement requires multi-parameter
co-optimization of AMPLITUDE_CEILING + constructive dynamics.
