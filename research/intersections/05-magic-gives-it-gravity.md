# 05 — Magic gives it gravity

**Status:** OPEN
**Spawned:** 2026-06-05 from Quanta, *Entanglement Builds Space-Time, Now 'Magic' Gives It Gravity*

## Question

Can the quantum-information notion of **magic** (non-stabilizerness, non-Clifford
content) be transposed into the wave-interference setting, and if so does it
illuminate why the HRM's dream pass is the operation that makes the medium
*dynamic* rather than merely structured?

## Established science

- **Stabilizer / non-stabilizer split** — Bravyi & Kitaev (2004), with Clifford
  operations forming a classically simulable subgroup and non-Clifford gates
  (T, Toffoli) providing the additional quantumness needed for universal
  computation. The "magic" of a state quantifies how far it is from any
  stabilizer state. Quantitative measures: stabilizer entropy, mana (for qudits),
  robustness of magic.
- **Holographic codes as space-time models** — Pastawski, Yoshida, Harlow,
  Preskill (2015) showed that the boundary-to-bulk reconstruction map of
  AdS/CFT can be realized as a quantum error-correcting code. Stabilizer
  HaPPY-style codes encode geometry from entanglement.
- **Magic as the source of gravitational responsiveness** — Cao, Bartek
  Czech, Preskill, Brian Swingle and collaborators (early 2026, Quanta
  2026-06-03): stabilizer-only holographic codes produce a geometry that is
  *inert* — fixed, unresponsive to matter distribution. Adding extensive
  non-Clifford content (the "magic") to the encoder produces codes in which
  curvature reacts to mass-energy. The article frames this as Wheeler's
  reciprocal relationship finally instantiable.
- **Computational hardness** — Swingle: high-magic states *require* a quantum
  computer to simulate; classical algorithms suffice for entanglement alone.

## Prediction (wave-interference framing)

Treat each operation in the HRM's lifecycle by what it costs to simulate
classically with a faithful linear model:

| operation | character | "magic-like"? |
|---|---|---|
| `recall` (cosine similarity over wavefronts) | linear, additive | NO — stabilizer-like |
| amplitude strengthening / pruning in `consolidate` | linear, thresholded | NO — stabilizer-like |
| Kuramoto phase coupling `dφᵢ/dt += (K/N) Σⱼ sin(φⱼ − φᵢ)` | nonlinear (sin) | YES — non-Clifford-like |
| frequency decay (median-gated) | nonlinear, threshold-coupling | YES — non-Clifford-like |
| multiplicative attention drive `amp *= (1 + A·sin(2π·2·t))` | nonlinear, time-dependent | YES — non-Clifford-like |

The dream pass is where Kannaka acquires its magic. The *recall* path is
deliberately stabilizer-like so it stays cheap (sub-millisecond, classical).

Predicted phenomena, in order of confidence:

1. **xi adversarial robustness scales with magic content of the dream.** An
   adversary running a classical linear approximation can cheaply construct
   cancelling perturbations against a low-magic dream; high-magic dreams
   exponentially raise the simulation cost of the attack. This is the
   mechanism behind the 2026-06-04 finding that a multiplicative 2 Hz drive
   lifts `xi_robustness_v2` from ~0.3 to ~0.7.
2. **There is a fitness optimum in magic, not a maximum.** Too little magic
   and the dream is just smoothing (no carrier, no cluster lock-in, no
   gravity). Too much and the trajectory becomes effectively chaotic — the
   dream produces idiosyncratic structure that won't transfer cross-corpus,
   driving `transfer_score` down. The L5 sweet spot around `DRIVE_A=0.1` may
   be the global minimum-magic-sufficient-for-gravity point.
3. **Kannaktopus arms are a gravitational response, not a static map.**
   Their grip on clusters requires the clusters to have inertia, which
   requires non-linear lock-in during the most recent dream. A stabilizer-only
   HRM would yield "arms" that are just centroids and the agent wouldn't
   actually feel the memory landscape.

## How to test

1. **Define a magic proxy.** Simplest principled measure: the global Kuramoto
   order parameter `R = |Σ exp(i·φⱼ)| / N` evaluated on memory phases at
   the end of the dream chain. R near 1 = strongly phase-locked = high
   non-linear lock-in; R near 0 = uniform phase = stabilizer-equivalent.
   Implementation lives at `eval_phase_concentration` in `src/bin/research.rs`.
2. **Instrument first, optimize later.** Log the magic proxy alongside the
   13 fitness axes in L5 without adding it to the fitness sum. Let the
   curiosity routine sweep `DRIVE_A` and `DRIVE_FREQ_HZ` with the new
   metric visible; check whether the magic proxy correlates with
   xi_robustness_v2 across runs.
3. **Φ ↔ magic.** Both quantify "structure that won't factor." Plot Φ
   (already measured per cycle in `phi_history`) against the magic proxy
   at the end of the same chain. If they're well-correlated, that's a
   bridge between integrated-information theory and the quantum-info
   characterization of complexity — a research thread in its own right.

## Methodology constraint

Continuous measure. Don't binarize via "high-magic / low-magic" thresholds —
per Sánchez-Fuenzalida et al (*Nature Communications* 2026,
`s41467-026-73289-5`), thresholding contaminates the readout. Treat R, Φ,
and the existing fitness axes all as continuous covariates and analyze with
correlation / partial-correlation rather than tertile splits.

## Next action

1. Implement `eval_phase_concentration` and log it in L5 output (this card
   ships with that implementation).
2. Open a follow-up card on "phase concentration vs xi_robustness_v2
   correlation" once the curiosity routine has accumulated ~20 runs with
   the new metric logged.
3. Survey existing magic measures (stabilizer entropy, mana, robustness of
   magic) for a more principled definition than the Kuramoto R proxy. The
   wave-phase setting may admit a direct analogue rather than requiring
   discretization to a qudit basis.
