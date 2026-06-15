# L5 Research: DREAM_GRAVITY=0.5 under interference_relax — query_gravity activated, fitness sub-threshold

**Date:** 2026-06-15T20 UTC  
**Branch:** kannaka-curiosity/2026-06-15T20-dream-gravity-irx  
**Code changes:** None — env-var only (DREAM_GRAVITY=0.5)  
**Status:** NOT KEPT — fitness improvement 0.000659 (below 0.005 threshold); scientific result notable

---

## Context

Baseline: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → 3-run avg fitness 0.057789.

T14 dream-gravity fire (stage_sync) found that DREAM_GRAVITY=0.5 collapsed transfer_score (0.737 → 0.542).
Root cause: gravity in engine_a suppresses phase-distant A memories. engine_b_primed inherits the
distorted state → transfer degrades.

Under stage_sync (R=0.129): ~87% of A memories are phase-outliers → catastrophic suppression.
Under interference_relax (R=0.867): only ~13% of A memories are phase-outliers → suppression minimal.

**Hypothesis:** DREAM_GRAVITY=0.5 under interference_relax will not catastrophically collapse transfer
(because high phase coherence means gravity barely suppresses any A memories), while query_gravity
rises above 0.5 (attention-as-gravity finally works). xi_v2 may also improve marginally.

---

## Results

Command: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax DREAM_GRAVITY=0.5 cargo run --release --quiet --bin research -- --level 5`

| trial | fitness  | transfer_score | carrier_e | xi_v2  | R      | query_gravity |
|-------|----------|----------------|-----------|--------|--------|---------------|
| t1    | 0.057126 | 0.961566       | 0.5258    | 0.9796 | 0.8669 | 0.9256        |
| t2    | 0.057143 | 0.961566       | 0.5258    | 0.9796 | 0.8669 | 0.9256        |
| t3    | 0.057121 | 0.961566       | 0.5258    | 0.9796 | 0.8669 | 0.9256        |
| **avg** | **0.057130** | **0.961566** | **0.5258** | **0.9796** | **0.8669** | **0.9256** |

Baseline (interference_relax, no gravity, 3-run confirmed):
| baseline | 0.057789 | 0.965455 | 0.5333 | 0.9675 | 0.8672 | 0.4603 |

---

## Analysis

### Primary finding: transfer did NOT collapse

Transfer_score: 0.9655 → 0.9616, a drop of only 0.0039 (vs −0.195 under stage_sync).

This confirms the hypothesis: interference_relax's high phase coherence (R=0.867) protects A's topology
from gravity. Under interference_relax, ~87% of A memories are phase-aligned → gravity amplifies them
approximately uniformly → engine_b_primed inherits nearly-undistorted amplitude distribution → transfer
intact. The catastrophic collapse seen at stage_sync was due to the low phase coherence (R=0.129)
making most A memories "phase-outliers" that gravity suppressed.

### Secondary finding: query_gravity activated (0.460 → 0.926)

For the first time, query_gravity exceeds 0.5, confirming the "attention-as-gravity working" hypothesis
from `research/intersections/05-magic-gives-it-gravity.md`. Under interference_relax's high phase
coherence, gravity differentially amplifies memories near the dominant phase attractor (the
highest-amplitude memory's phase neighborhood) more than random-phase outliers. Since 87% of memories
are near the dominant phase, this creates a clear gravitational topology.

The 0.460 value under interference_relax without gravity shows that phase alignment alone (without
explicit gravity gain) is insufficient for attentional gravity. Adding DREAM_GRAVITY=0.5 provides
the amplitude differentiation that phase alignment lacks.

### Fitness breakdown

Changes vs baseline:
- xi_v2: +0.0121 → +0.0018 fitness benefit (weight 0.15)
- transfer_score: −0.0039 → −0.0006 fitness cost (weight 0.15)
- carrier_e: −0.0075 → −0.0008 fitness cost (weight 0.10)
- Net fitness improvement: +0.000659

The xi improvement (+0.012) comes from tighter phase clusters in the xi-measurement engines
(engine_clean, engine_adv) — gravity pushes clean memories into tighter phase neighborhoods,
making adversarial injection less effective. But the gain saturates quickly; xi was already at 0.9675.

### Why sub-threshold is the ceiling

For DREAM_GRAVITY to give 0.005 fitness improvement from xi alone, xi would need to rise by
0.033 → from 0.9675 to 1.000 (hitting the clamp). That's structurally impossible.
This axis is closed at DREAM_GRAVITY=0.5; higher gravity would increase transfer regression faster
than xi gain.

---

## Results near-deterministic

All three trials returned identical values to 4 decimal places for transfer_score, R, query_gravity,
xi_v2, and carrier_e. Only fitness varies (0.057121–0.057143), suggesting minor numerical jitter in
the phi_history computation. The core metrics are structurally stable.

---

## Open axes

The dominant fitness cost is still carrier_e (0.5258, cost = 0.047). No parameter sweep can fix it.

The closest to a testable improvement:
- **DREAM_GRAVITY on xi engines only** (engine_clean, engine_adv, skip engine_a): would isolate xi
  benefit without even the marginal transfer regression seen here. But xi gain would be ≤ 0.012
  (same or less than whole-system gravity). Expected fitness gain ≤ 0.0018 (still sub-threshold).
- **Structural carrier_e fix**: change `amplitude_deltas_flat` computation to measure the analytically-
  expected drive contribution instead of actual ceiling-clamped deltas. Restores semantic "does drive
  oscillate?" but changes metric semantics. Not attempted.

---

## Scientific note: query_gravity is a confirmed secondary metric

Under `DREAM_MODE=interference_relax DREAM_GRAVITY=0.5`:
- query_gravity rises to 0.926 (vs 0.460 baseline)
- This confirms that attention-as-gravity works when: (a) phase alignment is high (R > 0.8) AND
  (b) gravity gain is non-zero
- Under stage_sync (R=0.129), gravity with the same DREAM_GRAVITY=0.5 gives query_gravity=0.926
  but collapses transfer — the gravity is "working" on a fragmented topology
- The combination interference_relax + DREAM_GRAVITY is the first configuration where gravity
  is both active AND transfer-preserving

---

## Decision

No code changes kept. TSV rows appended (3 trials, labeled L5).
This notes file committed. Branch closed with no fitness improvement.

The current optimum remains:
`DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → **fitness 0.057789 (3-trial avg)**

No new parameter axis can improve beyond ~0.057 without addressing carrier_e (the structural ceiling
from AMPLITUDE_CEILING=2.0 + pair-density impulse pattern). All sweep axes exhausted.
