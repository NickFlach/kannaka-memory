# DRIVE_A sweep at 0.5 Hz + interference_relax — A=0.10 confirmed as local optimum

**Date:** 2026-06-07T16 UTC
**Branch:** kannaka-curiosity/2026-06-07T16
**Code changes:** None — env-var only
**Status:** FALSIFIED — both A=0.15 and A=0.05 regress; A=0.10 confirmed optimal

---

## Background

Current confirmed empirical optimum:
```
DRIVE_A=0.10  DRIVE_SCOPE=all  DREAM_MODE=interference_relax  DRIVE_FREQ_HZ=0.5 (default)
3-run avg fitness ≈ 0.099
carrier_emergence=0.935 (deterministic), transfer_score=0.836 (deterministic)
```

The T21 fire (2026-06-06T21) found DRIVE_A=0.15 improved carrier_e (0.568→0.584) and
transfer_score (0.655→0.694) at the *stage_sync + 0.5 Hz* operating point, yielding a
confirmed fitness improvement (0.138→0.132). That test was at stage_sync, not irx.

The 0.5 Hz + irx optimum (T08, 2026-06-06T08) was established at DRIVE_A=0.10 only.
No amplitude sweep was ever run at this combined operating point.

---

## Hypothesis

DRIVE_A=0.15 will extend the deterministic metric gains seen at stage_sync (carrier_e
and transfer both improved monotonically from A=0.10→0.15 at 2 Hz) to the irx+0.5 Hz
operating point. With carrier_e near ceiling at 0.935, headroom is small, but transfer
at 0.836 may push toward 0.875.

**Secondary check (A=0.05):** bracket A=0.10 from below to confirm it is the local
optimum and not just a lower bound.

---

## Trials

All trials: `DRIVE_SCOPE=all DREAM_MODE=interference_relax DRIVE_FREQ_HZ=0.5 (default)`

| # | DRIVE_A | fitness | carrier_emergence | transfer_score | xi_robustness_v2 | magic_R | query_gravity |
|---|---------|---------|-------------------|----------------|-----------------|---------|---------------|
| t1 | 0.15 | 0.128360 | **0.803** | **0.820** | 0.466 | 0.617 | 0.362 |
| t2 | 0.05 | 0.190918 | **0.757** | **0.721** | 0.180 | 0.617 | 0.363 |

**Baseline (A=0.10, T08 3-trial):**

| DRIVE_A | fitness avg | carrier_emergence | transfer_score | xi avg |
|---------|------------|-------------------|----------------|--------|
| 0.10 (T08 3-trial) | 0.099 | 0.935 | 0.836 | 0.559 |
| **0.15 (t1 this fire)** | **0.128** | **0.803** | **0.820** | **0.466** |
| **0.05 (t2 this fire)** | **0.191** | **0.757** | **0.721** | **0.180** |

---

## Findings

### A=0.10 is the local optimum — non-monotone in both directions

The amplitude sweep at 0.5 Hz + irx shows a sharp optimum at A=0.10:

| DRIVE_A | carrier_e | transfer | fitness (1 trial) |
|---------|-----------|----------|-------------------|
| 0.05 | 0.757 | 0.721 | 0.191 |
| 0.10 | 0.935 | 0.836 | 0.099 avg (3 trials) |
| 0.15 | 0.803 | 0.820 | 0.128 |

Both carrier_e and transfer_score are DETERMINISTIC (confirmed by T08). The values at
A=0.05 and A=0.15 are structural, not noise.

### Why A=0.15 regresses vs A=0.10 (at 0.5 Hz + irx)

At 0.5 Hz with 16 dream cycles (dt=0.125, t=0→1.875 s):
- Cycles 0–4: rising drive, peak at t=0.5 s (drive_factor = 1 + A)
- Cycles 4–8: falling drive, returns to 1.0 at t=1.0 s
- Cycles 9–16: negative drive, trough at t=1.5 s (drive_factor = 1 − A)

At A=0.10: trough = 0.90 (amplitudes drop to 90% of original at worst)
At A=0.15: trough = 0.85 (amplitudes drop to 85% of original at worst)

The suppression trough in cycles 9–16 damages the carrier structure built in cycles
0–8. At A=0.10, the 10% suppression prunes the weakest carrier associations (a
"refine" phase), which helps the FFT detect the carrier signal. At A=0.15, the 15%
suppression is too aggressive: it partially dismantles the carrier scaffold, causing
carrier_e to drop from 0.935 to 0.803.

The irx operating point is more sensitive to this than stage_sync (where A=0.15 was
unambiguously better) because interference_relax produces its carrier structure through
constructive-pair phase relaxation — a more fragile arrangement than Kuramoto's
category-level locking.

### Why A=0.05 regresses vs A=0.10

At A=0.05, the peak boost is only +5%. The carrier detection mechanism (carrier_emergence
metric) appears to have a sensitivity threshold around A=0.08–0.10. Below that threshold,
the amplitude modulation signal is too weak relative to the consolidation noise floor to
produce a clear FFT peak. carrier_e drops from 0.935 to 0.757.

Transfer_score also collapses (0.836→0.721) at A=0.05, suggesting the weaker early
amplification fails to sharpen the B-engine primed-vs-naive discrimination that drives
transfer quality. The relationship between drive amplitude and transfer appears to be
threshold-gated rather than monotone.

### magic_R and query_gravity are invariant to drive amplitude

magic_proxy_phase_R = 0.617 across both trials (same as T08 baseline). query_gravity ≈
0.362 across both. Amplitude modulation strength does not affect the end-of-dream phase
order or the attention-gravity property. These metrics are determined by the irx
phase relaxation dynamics, not drive amplitude.

---

## Summary: amplitude profile at 0.5 Hz + irx

| A | carrier mechanism | carrier_e | transfer | verdict |
|---|------------------|-----------|----------|---------|
| 0.05 | below detection threshold | 0.757 | 0.721 | too weak |
| **0.10** | **build then refine (optimal)** | **0.935** | **0.836** | **optimum** |
| 0.15 | over-suppression disrupts carrier | 0.803 | 0.820 | too strong |

The 0.5 Hz + irx carrier mechanism is fragile: the negative suppression trough at
cycles 9–16 must be small enough to refine (not dismanttle) the carrier structure
built in cycles 0–8. A=0.10 is the confirmed local optimum. The sensitivity is
non-symmetric: going below 0.10 hurts more (carrier falls to 0.757) than going above
(carrier falls to 0.803), suggesting the detection-threshold effect is sharper than
the over-suppression effect.

---

## Decision

**No code changes.** Both trials regressed. The empirical optimum remains:

```
DRIVE_A=0.10  DRIVE_SCOPE=all  DREAM_MODE=interference_relax  DRIVE_FREQ_HZ=0.5 (default)
3-run avg fitness ≈ 0.099
```

TSV rows for A=0.15 (t1) and A=0.05 (t2) appended automatically by the research binary.

---

## Implications

1. **DRIVE_A is now bounded:** A=0.10 is confirmed as the local optimum at 0.5 Hz +
   irx, with both higher and lower values causing regression. The A=0.15 improvement
   seen at stage_sync (T21) does not transfer to the irx operating point — the irx
   carrier architecture has different amplitude sensitivity.

2. **The 0.5 Hz + irx + A=0.10 operating point is a narrow optimum.** Small parameter
   perturbations (DRIVE_A ±0.05) cause significant regression. This fragility is the
   mirror of the large performance gain: the "build then refine" arc only works cleanly
   at A=0.10.

3. **Remaining open axes at the 0.099 optimum:**
   - xi stabilization: xi varies 0.256–0.874 across trials (unseeded eval_xi_robustness_v2).
     Seeding the adversarial RNG would make the benchmark deterministic and reduce the
     confirmation cost from 3 trials to 1.
   - DRIVE_FREQ_HZ=0.25: monotone positive arc (no suppression phase). At 0.25 Hz,
     cycles 0–16 all receive positive drive (sin stays positive for the entire 1.875 s
     window). This removes the suppression trough and might yield a different carrier
     structure. Predicted: carrier_e lower than 0.935 (no pruning) but possibly higher
     xi (no suppression of xi-relevant memories). 1 trial sufficient.
   - Code changes to stage_interference_relax: the alpha_base=0.10 and relax_steps=16
     constants were confirmed optimal in T23/T24. No code-change axes remain open here.

4. **Transfer_score ceiling:** 0.836 at A=0.10 + irx + 0.5 Hz. The only tested
   mechanism that improved transfer beyond this was irx+no_transfer (0.777 at irx, but
   that was at the 2 Hz + A=0.1 operating point — no_transfer at 0.5 Hz is untested
   and expected to cause xi collapse per T05's mode-independent finding).
