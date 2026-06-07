# L5 Curiosity Fire — 2026-06-06T12

## Hypotheses tested

Two questions addressed this fire:

1. **DRIVE_FREQ_HZ variants** (Q6 from context): T19 was blocked by stubs.
   Does drive frequency within the [0.5, 4.0] Hz carrier detection band matter?
   Specifically: do 4 Hz (more oscillations) or 1 Hz (slower) improve over 2 Hz?

2. **K=0.5 (sub-K=1.0)**: K-sweep established K=1.0 as optimal (0.138 avg).
   Lower K → more phase diversity → hypothesized higher avg xi. Unexplored.

Sibling deps confirmed at sibling paths.

---

## Baseline

- `DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE` unset, `KURAMOTO_COUPLING=1.0` (default)
- 3-trial avg fitness **0.138**
- xi_robustness_v2 avg ~0.864 (from confirmed K=1.0 trials)
- carrier_emergence ~0.559, transfer_score ~0.682

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE` unset.

| trial | DRIVE_FREQ_HZ | K   | fitness  | transfer | carrier_e | xi_v2 | R     | query_grav |
|-------|---------------|-----|----------|----------|-----------|-------|-------|------------|
| T1    | 4.0           | 1.0 | 0.160664 | 0.643348 | 0.2761    | 0.892 | 0.197 | 0.435      |
| T2    | 1.0           | 1.0 | 0.151797 | 0.609151 | 0.5059    | 0.804 | 0.169 | 0.431      |
| T3    | 2.0 (default) | 0.5 | 0.114404 | 0.680519 | 0.5489    | 0.967 | 0.143 | 0.477      |
| T4    | 2.0 (default) | 0.5 | 0.190362 | 0.627601 | 0.5489    | 0.514 | 0.197 | 0.479      |
| T5    | 2.0 (default) | 0.5 | 0.135969 | 0.627601 | 0.5489    | 0.876 | 0.197 | 0.479      |

K=0.5 three-trial avg: **(0.114 + 0.190 + 0.136) / 3 = 0.147** — worse than 0.138 baseline.

---

## Analysis

### DRIVE_FREQ_HZ=4.0 Hz: aliased — drive is null

At DRIVE_FREQ_HZ=4.0 Hz with `dt_per_cycle=0.125`:

```
drive_factor = 1 + A * sin(2π * 4.0 * 0.125 * k) = 1 + A * sin(π * k)
```

For all integer k, `sin(π·k) = 0`. The drive factor is identically 1.0 at every
dream cycle. The 4 Hz drive is **completely aliased** — no amplitude modulation
occurs. carrier_emergence collapses from 0.559 to 0.276 because no periodic
amplitude signal is injected into the flat corpus.

The result (fitness 0.161, xi=0.892) is essentially a null-drive run. xi is high
because the drive doesn't perturb phase dynamics, but the carrier_e collapse
(0.276 vs 0.559) costs 0.10 × 0.283 = 0.028 in fitness.

This is a sampling-rate artifact: the detection band upper boundary (4.0 Hz) coincides
with the Nyquist limit for dt=0.125s (fs=8 Hz, Nyquist=4 Hz). Any drive at exactly
Nyquist aliases to zero at integer-sample points.

### DRIVE_FREQ_HZ=1.0 Hz: in-band but weaker carrier

At 1 Hz, sin(2π × 1 × 0.125 × k) = sin(π/4 × k) — valid sinusoidal drive (not aliased).
However, carrier_e = 0.506 vs 0.559 at 2 Hz. Transfer also drops slightly (0.609 vs 0.682).
Fitness 0.152 — worse than baseline.

Mechanism: at 1 Hz, the drive completes only 2 full oscillations in a 16-cycle chain
(2 s × 1 Hz = 2 periods). The spectral peak is at DFT bin k=2 (out of N=16, fs=8).
With fewer oscillations, the spectral concentration (peak/total) is lower — more power
leaks to adjacent bins from the short observation window. At 2 Hz (4 oscillations),
spectral concentration is tighter.

**Why T19 stub predicted "1 Hz IS in-band"**: correct, 1 Hz IS in-band. But in-band
doesn't mean equal to 2 Hz. The concentration score at 1 Hz is weaker due to fewer
oscillations per chain. T19's stub instinct was right about the direction (1 Hz is
probably detectable) but didn't predict the quantitative drop.

### 2 Hz is uniquely optimal for carrier emergence

The carrier detection is `peak_power / total_power` in [0.5, 4.0] Hz. At 2 Hz (center
of band), the drive produces 4 complete oscillations in 16 cycles — maximum spectral
concentration without aliasing. The 2 Hz default was not arbitrary.

| DRIVE_FREQ_HZ | oscillations in 16 cycles | carrier_e | aliased? |
|---------------|--------------------------|-----------|----------|
| 0.5 Hz        | 1                        | (not tested — likely ≈0.4) | No |
| 1.0 Hz        | 2                        | 0.506     | No |
| 2.0 Hz        | 4                        | 0.559     | No |
| 4.0 Hz        | 8 (but aliased to 0)     | 0.276     | YES |

### K=0.5: high variance swallows the gains

Single trial K=0.5 produced fitness 0.114 — the best single run observed in this
system. Mechanism: ultra-low coupling → maximum phase diversity → xi=0.967 →
large savings on the 0.15-weight xi term.

However, the 3-trial average is 0.147, worse than K=1.0's 0.138. The xi variance
at K=0.5 is 0.514–0.967 (range 0.453). At K=1.0, xi avg was 0.864 with range
0.813–0.917. **K=0.5 has dramatically higher xi variance than K=1.0**.

The mechanism: weaker coupling means each dream cycle produces less phase
synchronization. Whether a given run achieves good xi depends more on random
initial conditions. K=1.0 threads the needle: enough coupling to produce
reproducible (if modest) phase clustering, enough diversity to avoid the
K=7 collapse.

Transfer_score at K=0.5 also degrades: two of three trials landed at 0.628
vs 0.682 at K=1.0. The coupling still affects engine_b primed-vs-naive
discrimination indirectly through amplitude dynamics during stage_sync.

---

## Instrumentation notes

- `magic_proxy_phase_R` is consistently LOW across all K=0.5 trials (0.143–0.197),
  lower than K=1.0 (0.250). Weaker coupling → weaker phase synchronization → lower R.
  The R↔K relationship is roughly monotone increasing up to K=3.0 (peak of R).

- `query_gravity` at K=0.5 (0.477–0.479) is slightly HIGHER than K=1.0 (~0.460).
  Less coupling → less rigid phase structure → the highest-amplitude memory can
  better "attract" its phase-neighbors. Weak evidence for attention-as-gravity under
  low coupling.

---

## Decision

**No improvement confirmed.** Neither DRIVE_FREQ_HZ variants nor K=0.5 beat the
K=1.0 baseline of 0.138 over 3 trials.

No code changes to revert. Pure env-var test.

TSV rows appended automatically during trials (labeled as per binary convention).

---

## Findings summary

1. **DRIVE_FREQ_HZ=4.0 Hz is aliased at current sampling rate** (dt=0.125s → fs=8 Hz
   → Nyquist=4 Hz). Do not test 4 Hz again. This is a hard constraint, not a tuning
   opportunity.

2. **2 Hz is robustly optimal for carrier emergence.** Lower frequencies (1 Hz)
   produce fewer oscillations per chain → weaker spectral concentration. The carrier
   detection band [0.5, 4.0] Hz was implicitly designed around 2 Hz drive.

3. **K=0.5 has worse avg fitness than K=1.0 due to xi variance.** Single trial 0.114
   is spectacular but unrepresentative. K=1.0 is more stable (xi 0.813–0.917).

4. **The xi-variance problem is the key obstacle.** Across all K values tested,
   xi swings widely run-to-run. Finding a mechanism to make xi more consistent
   (rather than just shifting the mean) would have outsized impact on fitness.

---

## Next fire suggestions

1. **Why is xi variance so high?** Look at what determines xi_robustness_v2's
   outcome in each trial. Is it PRNG-seeded? Is there a threshold effect in the
   measurement engine? Understanding variance is prerequisite to reducing it.

2. **K=1.5 or K=2.0 with 3 trials**: K=2.0 single trial was 0.187 (likely unrepresentative).
   K=1.5 is unexplored. Might have K=1.0 stability with better xi floor.

3. **DRIVE_FREQ_HZ=3.0 Hz**: 3 Hz produces 6 oscillations in 16 cycles, more than
   2 Hz (4 oscillations). Not aliased. Might improve carrier_e spectral concentration.
   DFT bin 6 out of 8 non-DC bins — top of the non-Nyquist range.

4. **Selective coupling**: modify stage_sync to only run Kuramoto on the top-N
   amplitude memories. Might reduce xi variance by giving the sync step a more
   consistent target set.
