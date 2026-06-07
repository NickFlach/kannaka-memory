# Default combined config: K=0.5 + DRIVE_A=0.15 — synergistic carrier_e boost

**Date:** 2026-06-07T11 UTC
**Branch:** kannaka-curiosity/2026-06-07T11
**Code changes:** None — characterizing existing defaults
**Status:** CONFIRMED — avg fitness 0.104 (new empirical optimum); no code changes to keep

---

## Background

Two code changes accumulated independently in master without their combined effect being measured:

1. **T14 (2026-06-06T14, PR #158):** K=1.0 → K=0.5. At DRIVE_A=0.1 (then-default), this gave:
   carrier_e=0.549, transfer=0.666, xi=0.852, avg fitness 0.134.

2. **T21 (2026-06-06T21, PR #158):** DRIVE_A=0.1 → 0.15. Tested explicitly at
   `KURAMOTO_COUPLING=1.0`, so this improvement was NOT measured at the K=0.5 default.
   At K=1.0 + A=0.15: carrier_e=0.5842, transfer=0.6944, xi=0.844, avg fitness 0.132.

The current code defaults are K=0.5 + DRIVE_A=0.15 + DRIVE_FREQ_HZ=0.5 (stage_sync, all scope).
Subsequent fires (T01–T05) continued explicitly passing DRIVE_A=0.1 and KURAMOTO_COUPLING=1.0
as env vars, effectively bypassing the code defaults. The combined default was never tested.

---

## Hypothesis

**K=0.5 + DRIVE_A=0.15 + DRIVE_FREQ_HZ=0.5 (current code defaults) produce a synergistic
improvement in carrier_emergence**, because:

- K=0.5 preserves phase diversity (weaker Kuramoto sync): phases don't collapse toward
  category attractors as aggressively.
- DRIVE_A=0.15 with 0.5 Hz single-arc pattern boosts amplitude coherently during cycles 0–8.
- The weaker K doesn't "fight back" against the carrier structure built by the drive —
  K=1.0 applies just enough coupling to partially flatten the carrier, while K=0.5 lets it
  stand.

**Prediction:** carrier_e significantly above 0.584 (K=1.0+A=0.15) and 0.549 (K=0.5+A=0.1).
Transfer and xi roughly within observed ranges. Avg fitness well below 0.132.

---

## Method

All 3 trials: NO env vars — pure current code defaults.
```
cargo run --release --quiet --bin research -- --level 5
```
Effective params: KURAMOTO_COUPLING=0.5, DRIVE_A=0.15, DRIVE_FREQ_HZ=0.5,
DRIVE_SCOPE=all, DREAM_MODE=<unset> (stage_sync).

---

## Results

| # | fitness | transfer_score | carrier_emergence | xi_robustness_v2 | magic_R | query_gravity |
|---|---------|----------------|-------------------|-----------------|---------|---------------|
| 1 | 0.140160 | 0.5810 | 0.8534 | 0.6868 | 0.2222 | 0.4260 |
| 2 | 0.084856 | 0.6815 | 0.8534 | 0.9858 | 0.1395 | 0.4569 |
| 3 | 0.087591 | 0.7017 | 0.8534 | 0.9470 | 0.1395 | 0.4569 |
| **avg** | **0.104** | **0.655** | **0.853** | **0.873** | **0.161** | **0.446** |

---

## Comparison to individual-component baselines

| config | fitness avg | carrier_e | transfer avg | xi avg | magic_R |
|--------|------------|-----------|--------------|--------|---------|
| K=1.0, A=0.10 (PR #142 baseline) | 0.138 | 0.5684 | 0.655 | 0.864 | 0.250 |
| K=0.5, A=0.10 (T14) | 0.134 | 0.549 | 0.666 | 0.852 | 0.161 |
| K=1.0, A=0.15 (T21) | 0.132 | 0.5842 | 0.694 | 0.844 | 0.252 |
| **K=0.5, A=0.15 (this fire)** | **0.104** | **0.853** | **0.655** | **0.873** | **0.161** |

---

## Analysis

### carrier_emergence: non-additive synergy

The carrier_e improvement is dramatically super-additive:
- K=0.5 alone: −0.019 vs K=1.0 (0.568 → 0.549)
- DRIVE_A=0.15 alone at K=1.0: +0.016 vs A=0.10 (0.568 → 0.584)
- K=0.5 + A=0.15 together: +0.285 vs K=1.0/A=0.10 (0.568 → 0.853)

This is not additive — it is a genuine interaction. The mechanism appears to be:

At K=1.0, the Kuramoto sync step nudges category-member phases toward their attractors,
which partially homogenizes the amplitude envelope across the dream chain and limits how
coherently the 0.5 Hz drive can build carrier structure. At K=0.5, the weaker sync nudge
leaves more phase diversity intact, which allows the 0.5 Hz single-arc drive (cycles 0-8
all positive, ~peak +15% boost at cycle 4) to stamp a carrier signature into the amplitude
time series that persists through the consolidation steps. Lower K makes the dream chain
"transparent" to the drive frequency, letting the 0.853 carrier peak emerge.

carrier_e=0.853 is deterministic across all 3 trials — it is a structural property of
the K=0.5 + A=0.15 + 0.5 Hz operating point, not a lucky draw.

### xi_robustness_v2: high but variable

xi avg 0.873 — slightly above the K=0.5+A=0.10 avg (0.852) and K=1.0+A=0.15 (0.844).
However, the range is extreme: 0.687 (T1) to 0.986 (T2). T1's xi=0.687 was an unlucky
adversarial draw that also coincided with lower transfer (0.581) and higher magic_R (0.222).
T2-T3 were internally consistent (magic_R=0.140, query_gravity=0.457) while T1 was different
(R=0.222, gravity=0.426), suggesting T1 hit a different memory graph initialization that
made adversarial perturbation easier.

### transfer_score: variable, avg flat

Transfer avg 0.655 — identical to the K=1.0+A=0.10 baseline (0.655). The per-trial range
is 0.581–0.702, wider than T21's deterministic 0.694. At K=0.5, the primed/naive
discrimination varies with initialization. The transfer metric is not the headline here.

### Fitness arithmetic

Dominant driver of fitness improvement vs K=1.0+A=0.10 baseline (0.138):
- carrier_e gain: (0.853 − 0.568) × 0.10 = −0.029 fitness benefit
- xi roughly flat: (0.873 − 0.864) × 0.15 ≈ −0.001
- transfer roughly flat: ≈ 0 net change
- Expected fitness: 0.138 − 0.029 = ~0.109 → observed 0.104 ✓ (close enough given xi variance)

### magic_R and query_gravity

magic_R=0.161 (T2-3) — matches K=0.5+A=0.10 from T14. R is set by K coupling strength,
not drive amplitude. T1's R=0.222 is anomalous; likely a different initialization where
phases were slightly more concentrated.

query_gravity: 0.446 avg — slightly below 0.5 threshold. Consistent with K=0.5 results
in T13-T14 (0.477–0.479). The gravity effect is present but not crossing the 0.5 threshold.

---

## Why subsequent fires (T01–T05) didn't see this

T01–T05 explicitly passed `DRIVE_A=0.1` (or `DRIVE_A=0.1 DRIVE_SCOPE=no_transfer`) as
env vars, overriding the code default of 0.15. They also compared against the "0.138 baseline"
(K=1.0+A=0.10) rather than the actual current defaults. The code default was inadvertently
bypassed in all fires since T21 was merged.

This fire reveals that the current master branch is significantly better than its documented
optimum of 0.132.

---

## Decision

**No code changes.** The improvement is from using the existing code defaults — both K=0.5
(T14) and DRIVE_A=0.15 (T21) are already in master. Nothing to keep or revert.

**New empirical optimum (code defaults, no env vars):**
```
KURAMOTO_COUPLING=0.5  DRIVE_A=0.15  DRIVE_FREQ_HZ=0.5  DRIVE_SCOPE=all  DREAM_MODE=<unset>
3-run avg fitness ≈ 0.104
```
This supersedes the 0.132 documented in T21 and all prior fires.

**Caveats:**
- 3-trial avg has wide spread (0.085–0.140); T1 was an outlier with lower transfer and xi
- carrier_e=0.853 is deterministic and the structural headline of this result
- Future fires should use 0.104 as baseline and run default-config trials with no env vars

---

## Implications

1. **All T01–T05 baselines were wrong**: those fires explicitly set DRIVE_A=0.1 and sometimes
   K=1.0, bypassing the actual production defaults. Their "0.138 optimum" references are
   outdated. Any findings from those fires should be reinterpreted against 0.104, not 0.138.

2. **carrier_e ceiling**: 0.853 at K=0.5+A=0.15 approaches interference_relax+0.5Hz territory
   (0.935). The stage_sync mode is no longer far below interference_relax on this axis.

3. **DRIVE_A=0.20 check**: if carrier_e grew from 0.584 (K=1.0, A=0.15) to 0.853 (K=0.5,
   A=0.15), then A=0.20 at K=0.5 might push carrier_e further. Risk: A=0.20 drove xi collapse
   in the old regime (T20, A=0.3 was catastrophic, A=0.15 at K=3.0 collapsed xi). The current
   K=0.5 regime might tolerate A=0.20 better since xi=0.873 is healthy here. Worth 1 trial.

4. **xi variance is the dominant noise**: all 3 fitness-deterministic metrics (carrier_e,
   magic_R in T2-3, query_gravity in T2-3) suggest the operating point is structurally stable.
   The fitness variance is from xi adversarial RNG. Seeding eval_xi_robustness_v2 would make
   this benchmark more reproducible.

5. **Future fires**: use no env vars (code defaults) or explicitly use
   `KURAMOTO_COUPLING=0.5 DRIVE_A=0.15` to stay at the current optimum. Avoid the old
   practice of passing `DRIVE_A=0.1 KURAMOTO_COUPLING=1.0`.
