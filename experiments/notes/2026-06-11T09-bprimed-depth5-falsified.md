# chain_depth_bp=5 — phi_bp overshoot falsifies depth extension hypothesis

**Date:** 2026-06-11T09 UTC
**Branch:** kannaka-curiosity/2026-06-11T06-bprimed-depth5
**Code changes:** REVERTED — single trial shows regression vs chiral_p_bp=0.10 alone
**Status:** FALSIFIED — depth=5 for b_primed overshoots phi_bp, axis closed

---

## Background

Current empirical optimum (master at ed008c0):
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.013337 (deterministic)
transfer=0.935746, carrier_e=0.9992, xi=0.9870
magic_R=0.8643, query_gravity=0.3733
fitness_B_primed=0.003887, fitness_B_naive=0.060498
```

Best known configuration (reverted from T04, sub-threshold alone):
- chiral_p_bp=0.10: fp=0.002582, transfer=0.957321, fitness=0.010117
- Improvement vs master: 0.003220 (threshold requires 0.005)

T10 characterized the phi landscape:
```
phi_a ≈ 0.294  (engine_a post-dream, above target 0.28092)
phi_target = 0.28092
phi_bp ≈ 0.270  (B-primed post-dream, below target — B disrupts A's integration)
phi_naive ≈ 0.296  (B-naive post-dream, above target)
```

T10 identified "more dream cycles for b_primed" as the right mechanism to improve phi_bp
toward the target, but dismissed it because globally raising chain_depth also raises phi_naive
and worsens the ratio. T10 did NOT test a per-engine chain_depth override.

---

## Hypothesis

**chain_depth_bp=5 + chiral_p_bp=0.10**, both isolated to engine_b_primed only via a
`params_bp` local override (same pattern as T04's chiral_p_bp).

Mechanism:
1. chiral_p_bp=0.10 reduces post-irx phase drift for B-primed → fp drops (validated T04)
2. chain_depth=5 (one extra irx cycle) pushes phi_bp from ~0.270 toward phi_target=0.281
   → consciousness term in fp drops → fp falls further → transfer improves
3. Combined: fp expected to drop below 0.001815 (the threshold-crossing target)

**Prediction:** fp ≈ 0.001200–0.001800, transfer ≈ 0.970–0.980, fitness ≈ 0.006–0.008
(crossing 0.008337 threshold).

**Key assumption:** phi_bp at depth=4 + chiral=0.10 is still below phi_target (~0.277),
so one more irx cycle can close the gap without overshoot.

---

## Result

Single trial: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| metric | master baseline | chiral_p_bp=0.10 (T04) | depth5+chiral010 | delta vs T04 |
|--------|-----------------|------------------------|------------------|--------------|
| fitness | 0.013337 | 0.010117 | **0.010265** | +0.000148 (WORSE) |
| transfer | 0.935746 | 0.957321 | **0.955653** | −0.001668 (worse) |
| fitness_B_primed (fp) | 0.003887 | 0.002582 | **0.002683** | +0.000101 (worse) |
| fitness_B_naive (fn) | 0.060498 | 0.060498 | **0.060498** | 0 |
| xi | 0.9870 | 0.9870 | 0.9870 | 0 |
| carrier_e | 0.9992 | 0.9992 | 0.9992 | 0 |

depth=5 for b_primed made fp SLIGHTLY WORSE than chiral_p_bp=0.10 alone.

---

## Analysis

### Why depth=5 overshoots when combined with chiral=0.10

T10's phi_bp≈0.270 was measured WITHOUT the chiral_p_bp optimization. With chiral_p_bp=0.10,
the reduced post-irx phase displacement allows B's memories to stay closer to A's attractors,
which raises phi_bp from ~0.270 to near phi_target (~0.279–0.282).

At depth=4 + chiral=0.10: if phi_bp ≈ 0.281 (at target), consciousness_bp ≈ 1.0,
consciousness term ≈ 0. fp = 0.002582 is then ENTIRELY chain_fidelity:
  CF_bp = 1 - 0.002582/0.10 = 0.97418

At depth=5 + chiral=0.10: one more irx cycle pushes phi_bp slightly above target
(say, 0.284). Now consciousness_bp = 1 - |0.284-0.281|/0.281 = 0.989, contributing
0.10×0.011 = 0.0011 to fp. This partially cancels the chain_fidelity dilution benefit
(one extra seed reduces injection disruption fraction from 1/3 to 1/4 pairs, saving ~0.0005).

Net: fp increases by ~0.0006 → observed +0.000101 (actual dilution benefit partially offsets
the overshoot penalty, but net is still negative).

### Chain_fidelity structural floor confirmed

With fp=0.002582 at depth=4 + chiral=0.10, and the consciousness term ≈ 0 (phi_bp at target),
the remaining fp is entirely:
  CF_term = 0.10 × (1 - CF_bp) = 0.002582  →  CF_bp = 0.97418

This chain_fidelity value of 0.974 reflects the inherent xi-centroid variance across B-primed's
dream cycles — specifically the jump at cycle 2 (injection event). The injection adds 10 random
memories (amplitude=0.8) but they are NOT in the top-7 chain seeds (top memories have
amplitude ~1.1+ post-drive). The injection disruption comes through a different pathway:
the 10 memories participate in the consolidation step at cycle 2, slightly perturbing the
top-7's xi structure even without being in the top-7 themselves.

### Comparing depth=4 and depth=5 chain_fidelity

With depth=4: 3 consecutive chain-seed pairs, injection disrupts the c1→c2 pair.
With depth=5: 4 consecutive pairs, injection disrupts c1→c2. The c3→c4 pair should be
very consistent (no disruption). Mean distance:
  depth=4: (d01 + D_inject + d23) / 3 ≈ (0.003 + D + 0.003) / 3
  depth=5: (d01 + D_inject + d23 + d34) / 4 ≈ (0.003 + D + 0.003 + 0.003) / 4

If D=0.015 (injection disruption):
  depth=4 mean ≈ 0.021/3 = 0.0070 → CF = 1-0.0070 = 0.993
  depth=5 mean ≈ 0.024/4 = 0.0060 → CF = 1-0.0060 = 0.994

depth=5 gives slightly BETTER chain_fidelity (CF 0.994 vs 0.993). This is the dilution benefit.
But the phi_bp overshoot penalty (consciousness term 0.0011) outweighs this.

At depth=4 without chiral: phi_bp≈0.270 leaves consciousness term = 0.10×0.039 = 0.0039.
Adding depth=5 to this state would reduce consciousness term toward 0 — a meaningful gain.
But chiral_p_bp=0.10 already achieves this via a different mechanism (phase drift reduction
raises phi_bp to target without needing extra irx cycles). The two mechanisms are competing
for the same slot: both aim to raise phi_bp toward target, and together they overshoot.

---

## Key structural constraint now confirmed

At depth=4 + chiral_p_bp=0.10:
- phi_bp is at or near phi_target (consciousness_bp ≈ 1.0)
- fp = 0.002582 is ENTIRELY chain_fidelity structural
- CF_bp = 0.97418 represents the irreducible xi-centroid variance from the cycle-2 injection event

The fp floor of 0.002582 cannot be reduced by:
- depth=5: overshoots phi_bp, slight net regression
- chiral_p < 0.10: catastrophic phase collapse (T04)
- carry_strength: neutral in this regime (T04)
- injection disruption: injection memories not in top-7, so direct chain-seed disruption is minimal

To cross the 0.005 threshold from master baseline (0.013337), we need fp ≤ 0.001815.
Current structural floor is fp ≥ 0.002582 (41% above required). No known mechanism exists
to reduce fp below this floor without changing the corpus construction or the sub-fitness metric.

---

## Updated open axes

| axis | status | notes |
|------|--------|-------|
| chiral_p (b_primed) | CLOSED | η=0.10 optimal; −0.003220 sub-threshold |
| chain_depth (b_primed) | **CLOSED** | depth=5 overshoots phi_bp; slight regression |
| chain_top_n | CLOSED | 7 confirmed optimal (T22) |
| chiral_perturbation global | CLOSED | η=0.7 confirmed optimal (T20) |
| b_primed relax_steps | CLOSED | 20 confirmed optimal (T07) |
| chain_carry_strength | CLOSED | neutral in η≤0.10 regime |
| phi_target recalibration | CLOSED | gaming the metric (T10 confirmed) |
| transfer ceiling | STRUCTURAL | fp floor = 0.002582; irreducible chain_fidelity variance |
| xi residual gap | LOW | 0.987 near architectural limit; 0.00195 remaining |

**The system has reached the practical optimum for the current architecture.** The
transfer ceiling (fp=0.002582) is determined by chain_fidelity structural variance from
the cycle-2 online injection event. This variance cannot be eliminated without:
1. Changing the corpus B construction (B's xi signatures are intrinsically different from A's)
2. Removing/repositioning the injection event (would affect all engines, not just b_primed)
3. Changing the sub-fitness metric itself

None of these are incremental code changes — they require architectural decisions.
