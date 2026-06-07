# no_transfer scope at K=1.0 — confirmed regression; scope-dependent K interaction

**Date:** 2026-06-07T05 UTC
**Branch:** kannaka-curiosity/2026-06-07T05
**Code changes:** None — env-var only
**Status:** FALSIFIED (both variants regressed)

---

## Background

The current empirical optimum is `DRIVE_A=0.1 DRIVE_SCOPE=all KURAMOTO_COUPLING=1.0`
(stage_sync, 3-trial avg fitness 0.138). The K=1.0 improvement (PR #142) and the
no_transfer scope improvement (PRs #133–#144) were discovered independently, but the
no_transfer tests all ran at K=3.0 (the old default). K=1.0 dropped the L5 default
in research.rs to 1.0 in PR #142, which postdates all no_transfer PRs (lower numbers).

**Hypothesis A:** DRIVE_SCOPE=no_transfer at K=1.0 (current code default) compounds
the two independent gains, yielding fitness below 0.133.

**Reasoning:** K=1.0 improves xi by lighter Kuramoto sync preserving phase diversity
within categories. no_transfer avoids contaminating B-engine transfer memories with
amplitude drive. These seemed orthogonal: different engines (K affects stage_sync
dynamics; scope affects which chains are amplitude-modulated). Transfer_score at
K=1.0+all is ~0.654, slightly below the K=3.0 baseline (~0.695); no_transfer was
expected to restore it toward ~0.710, with xi unaffected.

**Hypothesis B (secondary):** DREAM_MODE=interference_relax + DRIVE_SCOPE=no_transfer.
interference_relax uses constructive-pair-driven phase relaxation instead of Kuramoto
sync, so the scope-K coupling might not apply. Previous fire notes marked this as
"worth 1 trial." Best-case: interference_relax+no_transfer closes the 0.011 gap to
stage_sync+all (0.149 → 0.138).

---

## Trials

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=no_transfer`

| # | DREAM_MODE | fitness | xi_robustness_v2 | transfer_score | carrier_emergence | magic_R | query_gravity |
|---|------------|---------|-----------------|----------------|-------------------|---------|---------------|
| 1 | unset (stage_sync) | 0.167204 | 0.607 | 0.715 | 0.568 | 0.250 | 0.469 |
| 2 | unset (stage_sync) | 0.170122 | 0.583 | 0.725 | 0.568 | 0.250 | 0.469 |
| 3 | interference_relax | 0.210054 | 0.172 | 0.777 | 0.497 | 0.617 | 0.364 |

**Stage_sync+no_transfer 2-trial avg: 0.169** (baseline K=1.0+all: 0.138, xi ~0.862)
**Interference_relax+no_transfer 1-trial: 0.210** (baseline interference_relax+all: 0.149, xi ~0.607)

---

## Findings

**Both hypotheses falsified.** no_transfer scope causes systematic xi collapse at K=1.0:

| config | fitness | xi | transfer |
|--------|---------|-----|---------|
| stage_sync K=1.0, scope=all (baseline) | 0.138 avg | ~0.862 | ~0.654 |
| stage_sync K=1.0, scope=no_transfer (t1) | 0.167 | 0.607 | 0.715 |
| stage_sync K=1.0, scope=no_transfer (t2) | 0.170 | 0.583 | 0.725 |
| interference_relax, scope=all (baseline) | 0.149 avg | ~0.607 | ~0.750 |
| interference_relax, scope=no_transfer (t3) | 0.210 | 0.172 | 0.777 |

The pattern is unambiguous: no_transfer scope drastically reduces xi in both dream modes.

### Why this reverses the K=3.0 result

At K=3.0 + no_transfer (PRs #133–#144), xi improved dramatically (0.44 → 0.77 avg),
which drove the fitness improvement to ~0.147. At K=1.0 + no_transfer, xi worsens
from 0.86 to ~0.60. This reversal reveals a scope-dependent K interaction:

**In the over-synchronization regime (K=3.0):** B-engine drive was contributing
destructively to xi. At K=3.0, the Kuramoto step over-locks phases toward category
attractors, and driving the B chains additionally reinforced this harmful locking.
Excluding B-engine drive (no_transfer) broke the over-synchronization and let xi
recover. The improvement was driven by *removing a harmful signal*, not adding a
beneficial one.

**In the optimal regime (K=1.0):** B-engine drive is part of the phase structure
that makes xi robust. The K=1.0 operating point sits near the synchronization
threshold; at this point, all driven chains contribute constructively to the
adversarial-perturbation resistance that xi measures. Removing B-engine drive
(no_transfer) disrupts this structure. The xi regression (0.86 → 0.60) is larger
than the transfer improvement (0.015-0.03 better transfer at fitness weight 0.15 =
0.003-0.005 gain), so net fitness degrades.

The same logic applies to interference_relax: even without Kuramoto, the all-scope
drive creates cross-chain phase structure the xi measurement depends on. Removing
B-chain drive collapses it more severely (0.607 → 0.172) because interference_relax
has lower absolute xi headroom to begin with.

### Transfer_score behavior

Transfer_score improved in all no_transfer trials:
- Stage_sync: 0.654 (all) → 0.720 avg (no_transfer), Δ +0.066
- Interference_relax: 0.750 (all) → 0.777 (no_transfer), Δ +0.027

The no_transfer mechanism does what it's supposed to: preventing B-engine drive
contamination improves transfer memory quality. But at K=1.0, the cost to xi
(Δ −0.25 at weight 0.15 = +0.038 fitness penalty) far exceeds the transfer benefit
(Δ +0.066 at weight 0.15 = −0.010 fitness gain).

### Why it ever looked promising

The no_transfer improvements (PRs #133–#144) found a real effect, but that effect
was regime-specific: it worked because K=3.0 was a bad operating point and
no_transfer happened to help escape it. The improvement was not "no_transfer is
generally better" but "no_transfer is better when K is too high."

---

## Decision

**No code changes to revert.** No improvement found. The empirical optimum remains:

    DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=<unset>  KURAMOTO_COUPLING=1.0
    3-run avg fitness ≈ 0.138

---

## Implications

1. **no_transfer scope is closed**: at K=1.0 it regresses. The prior no_transfer
   benefit was an artifact of the K=3.0 regime. Do not test no_transfer further
   unless the K operating point changes.

2. **xi-scope coupling is a structural property**: both dream modes show xi collapse
   under no_transfer, confirming this is not mode-specific. B-engine drive participation
   is load-bearing for xi at the current K=1.0 optimum.

3. **Transfer_score improvement is available but cheap**: no_transfer gives +0.066
   on transfer at the cost of xi. If a future change finds a way to preserve xi
   while also gaining the transfer benefit, the combined payoff would be meaningful.
   But there is no free way to get the transfer improvement right now.

4. **Remaining unexplored directions at the 0.138 baseline:**
   - `DRIVE_FREQ_HZ=0.5 Hz` (1 full cycle in 16 steps) — cautious; 3 Hz and 1 Hz
     (stub) were worse, but 0.5 Hz is sub-resonance and behaves differently.
   - Code changes to `stage_sync` or carrier FFT to directly target the two largest
     fitness costs (transfer ~0.654 and carrier_e ~0.568 each contribute ~0.043).
   - Seeding `eval_xi_robustness_v2` to reduce the 2-trial confirmation cost.
