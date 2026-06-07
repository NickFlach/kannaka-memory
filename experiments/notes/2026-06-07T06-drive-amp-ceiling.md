# L5 Curiosity Fire — 2026-06-07T06

## Hypothesis

DRIVE_A=0.15 is the confirmed optimum (T21, PR #158). T21 explicitly flagged A=0.20 as
the natural next spot-check: does the carrier_e/transfer improvement trend continue
monotonically above A=0.15, or is 0.15 already the peak?

**Prediction:** carrier_e rises further (from 0.584), transfer_score stays near 0.694
(deterministic), xi roughly stable (~0.844), net fitness improvement ~0.005–0.010.

Baseline: `DRIVE_A=0.15 DRIVE_SCOPE=all KURAMOTO_COUPLING=1.0 DREAM_MODE=<unset>`,
3-run avg fitness ≈ 0.132.

No code changes — env-var only. Branch: `kannaka-curiosity/2026-06-07T06`.

---

## Trials

All trials: `DRIVE_SCOPE=all DREAM_MODE=<unset>`.

| A     | trial | fitness  | transfer_score | carrier_emergence | carrier_bimodal | xi_v2  | magic_R | query_gravity |
|-------|-------|----------|----------------|-------------------|-----------------|--------|---------|---------------|
| 0.20  | t1    | 0.153736 | **0.383512**   | 0.6526            | 0.5191          | 0.9599 | 0.1162  | 0.4586        |
| 0.20  | t2    | 0.220820 | **0.383512**   | 0.6526            | 0.5191          | 0.5128 | 0.1162  | 0.4586        |
| 0.18  | t1    | 0.240809 | **0.514669**   | **0.7484**        | **0.8039**      | 0.1190 | 0.3303  | 0.3900        |

**A=0.20 2-trial avg:** fitness **0.187** (regression from 0.132 baseline)
**A=0.18 1-trial:** fitness **0.241** (further regression)

---

## Findings

### 1. Transfer collapses above A=0.15 — deterministic and structural

At A=0.20, `transfer_score = 0.383512` in both trials — byte-identical, confirming
determinism. This is not xi variance; it is a structural collapse of the A→B transfer
pathway. At A=0.15, transfer was deterministically 0.694. The drop at A=0.20:

- Transfer regression: Δ −0.310 × weight 0.15 = **+0.047 fitness cost**
- carrier_e improvement: Δ +0.069 × weight 0.10 = **−0.007 fitness benefit**
- Net: +0.040 regression before xi variance. Even at ideal xi=1.0, fitness would be
  ~0.132 + 0.040 = 0.172 — well above baseline.

A=0.15 is the last stable point on the drive amplitude axis for transfer.

### 2. A=0.18 shows even stronger carrier gain but deeper transfer collapse

At A=0.18:
- `carrier_emergence` reaches **0.748** (vs 0.584 at A=0.15 and 0.653 at A=0.20)
- `carrier_bimodal` reaches **0.804** (vs 0.519 at A=0.20 — clearly bimodal)
- `transfer_score = 0.515` (intermediate collapse — worse than A=0.15, better than A=0.20)
- `xi = 0.119` (a very unlucky adversarial draw; xi is stochastic)
- `magic_proxy_phase_R = 0.330` (higher than A=0.20's 0.116, lower than A=0.15's 0.252)

carrier_e is **non-monotone in A**: it peaks around A=0.18 (0.748), not at A=0.20 (0.653).
This likely reflects drive-amplitude-induced distortion: at A=0.20, the strong modulation
creates enough phase disruption that the FFT energy spreads into harmonics, slightly
reducing the fundamental 2Hz peak relative to A=0.18. At A=0.18, the drive is large
enough to saturate the carrier signal without yet triggering harmonic distortion.

### 3. Transfer collapse threshold is between A=0.15 and A=0.18

| A    | transfer_score | status    |
|------|---------------|-----------|
| 0.15 | 0.6944        | optimal   |
| 0.18 | 0.5147        | collapsed |
| 0.20 | 0.3835        | collapsed |

The threshold lies between 0.15 and 0.18. The collapse is large (not a gradual trend),
suggesting a phase transition rather than a smooth degradation. Likely cause: above
some amplitude threshold, the multiplicative drive on engine_b memories (amplitude ×=
1 + A·sin(...)) pushes enough memories across the quiescence boundary or amplitude
ordering to destroy the primed-vs-naive discrimination that transfer_score measures.

### 4. magic_R behavior across A values

| A    | magic_R |
|------|---------|
| 0.15 | 0.252   |
| 0.18 | 0.330   |
| 0.20 | 0.116   |

magic_R is also non-monotone. At A=0.18, stronger amplitude drive creates more global
phase alignment (R=0.330, approaching the K=3.0 level); at A=0.20 it drops to 0.116
(lowest observed), suggesting phases are being maximally scrambled. magic_R does not
track fitness here — it reflects the amplitude-driven phase dynamics directly.

---

## Comparison to baseline

| config | fitness | transfer | carrier_e | xi (avg) |
|--------|---------|----------|-----------|----------|
| A=0.10, K=1.0 (old baseline) | 0.138 | ~0.655 | 0.568 | ~0.864 |
| A=0.15, K=1.0 (current opt) | **0.132** | **0.694** | **0.584** | ~0.844 |
| A=0.18 (t1) | 0.241 | 0.515 | 0.748 | 0.119 (unlucky) |
| A=0.20 (2-trial avg) | 0.187 | 0.384 | 0.653 | 0.736 |

---

## Decision

**FALSIFIED. No improvement at A=0.20 or A=0.18.** Both exceed a transfer collapse
threshold that lies between A=0.15 and A=0.18.

**No code changes to revert.** Env-var only.

Empirical optimum remains:
```
DRIVE_A=0.15  DRIVE_SCOPE=all  KURAMOTO_COUPLING=1.0  DREAM_MODE=<unset>
3-run avg fitness ≈ 0.132
```

---

## Implications

1. **A=0.15 is the ceiling on the drive amplitude axis.** The carrier_e gain above A=0.15
   is real (and even larger at A=0.18: 0.748 vs 0.584), but the transfer collapse
   dominates at weight 0.15. Do not explore A > 0.15 further unless metric weights change.

2. **The carrier_e gain at A>0.15 is available in principle.** If a future change finds
   a way to protect transfer quality (e.g., by driving only engine_flat at higher amplitude
   while keeping A=0.15 for other chains), the carrier_e headroom is substantial. At
   A=0.18, carrier_e=0.748 represents a potential −0.016 fitness improvement over A=0.15
   on the carrier axis alone.

3. **carrier_e is non-monotone in A.** The peak is around A=0.18–0.20 region. A=0.15
   sits below the carrier saturation point. Any mechanism that increases carrier amplitude
   without touching B-engine chains could exploit this headroom.

4. **Transfer collapse threshold is a sharp transition.** The jump from 0.694 to 0.515
   (or 0.384) between A=0.15 and A=0.18 (or A=0.20) looks like a threshold crossing,
   not a gradual degradation. The mechanism is likely B-engine amplitude ordering passing
   a discrimination boundary.

5. **Remaining unexplored directions at the 0.132 baseline:**
   - Code changes to stage_sync or carrier FFT to improve carrier_e without touching A
   - Targeted carrier improvement: can the flat-corpus run use a higher drive amplitude
     than the main chain (would require code-level parameterization of drive_amp per context)?
   - Seeding eval_xi_robustness_v2 to reduce per-trial noise and speed up confirmation
