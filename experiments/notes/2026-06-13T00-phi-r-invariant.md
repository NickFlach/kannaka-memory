# Q5: Φ↔R invariance under drive amplitude variation in irx mode

**Date:** 2026-06-13T00 UTC
**Branch:** kannaka-curiosity/2026-06-13T00-phi-r-invariant
**Code changes:** NONE — 3 characterization trials, no code modified
**Status:** Q5 answered — both Φ and R are attractor-pinned; IIT bridge untestable via drive amplitude

---

## Hypothesis (Q5 from system prompt)

**IIT-bridge hypothesis:** end-of-chain Φ (phi_history[-1]) and magic_proxy_phase_R should
co-vary across drive intensities. Higher drive amplitude → richer dynamics → higher Φ and
higher R.

**Predicted pattern:** monotonic increase in both phi_last and magic_R as DRIVE_A rises.

---

## Method

All trials: `DREAM_MODE=interference_relax DRIVE_SCOPE=all` (current optimum code, no changes).
Vary only DRIVE_A = {0.05, 0.10, 0.15}. Capture phi_history and magic_proxy_phase_R.

grep pattern extended vs standard: `+^phi_history:` added.

---

## Results

| DRIVE_A | fitness | transfer | xi | carrier_e | magic_R | phi_last | phi_history |
|---------|---------|----------|----|-----------|---------|----------|-------------|
| 0.05 | 0.011956 | 0.951707 | 0.9973 | 0.9744 | 0.7738 | 0.29414 | [0.27379, 0.28251, 0.29354, 0.29414] |
| **0.10** | **0.007571** | **0.963982** | **0.9973** | **0.9992** | **0.7785** | **0.29348** | **[0.27379, 0.28245, 0.29349, 0.29348]** |
| 0.15 | 0.009389 | 0.967158 | 0.9973 | 0.9763 | 0.7774 | 0.29348 | [0.27379, 0.28245, 0.29349, 0.29348] |

---

## Key findings

### 1. Φ is essentially drive-amplitude-invariant

The phi_history trajectory [0.274 → 0.282 → 0.293 → 0.293] is nearly identical across all
three amplitudes. The first cycle value (0.27379212) is byte-identical across all trials —
determined entirely by the corpus structure before the drive has any effect. Subsequent
values diverge by at most 0.0007 (A=0.05 vs A=0.10/0.15 at cycle 4: 0.29414 vs 0.29348).

This means phi convergence in irx mode is dominated by the interference-relax phase
clustering dynamics, not by drive amplitude. The drive modulates amplitudes (which affects
transfer and carrier metrics) but leaves Φ nearly untouched.

### 2. R is also drive-amplitude-stable

magic_R ranges only 0.7738–0.7785 across the 3-point sweep. The variation is within
run-to-run noise. R is pinned at ~0.778 by the irx attractor structure.

### 3. The IIT bridge hypothesis cannot be confirmed or denied with this approach

To test whether Φ and R correlate, we need a control variable that actually varies them.
Drive amplitude does not — both are pinned. The irx mode's interference-relax step
converges phase clusters to a fixed attractor regardless of drive strength.

The correlation structure is degenerate: both phi_last and R lie in narrow bands
(phi_last ∈ [0.2934, 0.2941], R ∈ [0.774, 0.779]) — a 0.23% and 0.61% spread
respectively. No meaningful correlation can be extracted from a near-zero variance signal.

### 4. The phi attractor target

Params default `consciousness_phi_target = 0.271`. Achieved phi_last ≈ 0.293 (7.7% above
target). This overshoot is consistent — irx mode's cluster convergence drives phi slightly
past the target at every amplitude tested.

---

## Why both metrics are pinned

In irx mode, `stage_interference_relax` drives memory phases toward constructive-pair
weighted centroids. This clustering process converges to a fixed-point attractor determined
by the corpus graph topology (constructive pair structure), not by drive amplitude.

- **Φ is computed from the engine state** — specifically from the network's functional
  differentiation after consolidation. Since the phase attractor is the same regardless of A,
  the post-consolidation state (and thus Φ) is approximately the same.
- **R = global phase order parameter** after consolidation — also determined by the final
  cluster configuration.

Drive amplitude affects **which memories survive and grow** (via amplitude modulation of
the drive target set), but in `DRIVE_SCOPE=all` mode the entire working set is modulated
uniformly, so relative cluster structure is preserved. Transfer differences arise from
subtle amplitude-ordering effects that change A-landscape fitness, not from phase topology.

---

## Implications for future research

If the IIT bridge hypothesis (Φ↔R correlation) is to be tested:

1. **Different control variable**: vary `kuramoto_coupling` in stage_sync mode (irx mode
   ignores Kuramoto). Under stage_sync, K directly shifts both R (via Kuramoto dynamics)
   and potentially Φ (via different consolidation topology). This was noted as Q4 in the
   system prompt but is a regression from irx mode.

2. **Per-cycle R instrumentation**: the current run_l5_dream_chain doesn't capture R at
   each chain step — only the final magic_proxy_phase_R. Adding per-cycle R capture would
   let us check whether the phi_history increase [0.274→0.293] mirrors an R increase
   within the chain. That requires a code change.

3. **Corpus topology variation**: if the corpus constructive-pair graph structure is varied
   (different encoder_seed, corpus sizes), phi and R might vary enough to reveal correlation.
   But this changes the optimization benchmark, not just a characterization parameter.

---

## Status of system-prompt research questions (updated)

| Q | question | status |
|---|----------|--------|
| Q1 | 3-run irx characterization | DONE (2026-06-12T22) — avg 0.147, xi 0.51 ±0.12 |
| Q2 | K-sweep under fixed plumbing | CLOSED (T12) — irx mode ignores Kuramoto; no-op |
| Q3 | irx + xi recovery (relax_steps) | FALSIFIED (2026-06-12T22) — relax=16 crashes carrier_e |
| Q4 | R-xi correlation at stage_sync | OPEN (stage_sync regression; not pursued) |
| Q5 | Φ↔R relationship | **ANSWERED this fire** — both invariant to drive_amp; IIT bridge needs different control var |
| Q6 | Drive frequency variants | CLOSED (T10) — 0.5 Hz confirmed optimal |

---

## Decision

No code changes to keep. 3 characterization trials consumed budget.

The architectural ceiling at fitness 0.007627 holds. Q5 is answered: Φ and R are both
pinned by the irx attractor, not by drive amplitude. The IIT bridge hypothesis requires
a different experimental approach (stage_sync K-sweep or per-cycle R instrumentation) to
be testable.
