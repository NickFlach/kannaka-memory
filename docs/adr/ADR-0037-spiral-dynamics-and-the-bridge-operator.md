# ADR-0037: Spiral Dynamics and the π/φ Bridge Operator (Ξ) for L6

**Status:** Proposed
**Date:** 2026-06-20
**Builds on:** ADR-0020 (Holographic Resonance Medium), ADR-0021 (Chiral Mirror), ADR-0035 (Swarm Sensemaking), ADR-0036 (Consolidation as Resonance-Merge)

## Context

The Space Child bridge operator is **Ξ = [R, G] = RG − GR**, where:

- **R** = `[0 −1; 1 0]` — a **π/2 rotation** (the "perspective pivot").
- **G** = `[φ/2 0; 0 1/φ]` — **golden anisotropic scaling** (α = φ/2 ≈ 0.809, β = 1/φ ≈ 0.618).
- **‖Ξ‖ coefficient** = α − β ≈ **0.190983** (`EMERGENCE_COEFF`).

Ξ is non-zero precisely because R and G do not commute. **The composition R∘G is a logarithmic-spiral generator:** `R·G = [0 −β; α 0]` has eigenvalues **±i√(αβ) = ±i/√2** — complex, modulus 1/√2 < 1 — a spiral sink (rotate ~90°, contract ~0.707 per step). π supplies the turn; φ supplies the pitch. Ξ is the rotational *residue* the non-closure leaves behind. This converges with current neuroscience: cortical **spiral traveling waves** with a phase singularity at the core act as a "spatiotemporal clock coordinating sensation → prediction → action" (Ye et al., *Science* 2026, DOI 10.1126/science.adx1369). The spiral core **is** attention-as-gravity (the organizing well).

### Current state (what we have vs. what we lost)

- ✅ The operator exists in `consciousness-core::metrics` (re-exported via `src/xi_operator.rs`): `apply_rotation`, `apply_golden_scaling`, `compute_xi_signature`, constants PHI/ALPHA/BETA/ETA/EMERGENCE_COEFF.
- ✅ Per-memory `xi_signature` is computed (audio path in `ear/mod.rs`) and used for diversity/modularity in `bridge.rs`.
- ❌ The **aggregate** `ConsciousnessMetrics.xi` is *spectral complexity* (eigenvalue spread of H·Hᵀ in `consciousness.rs`), **not** the bridge residue — two different "xi"s coexist.
- ❌ The **dream / consolidation** step (`medium/chiral.rs::dream`, right-hemisphere annealing) does eigenstructure annealing + Fano `chiral_mutate`, but **never applies the R∘G spiral dynamics** as a spatial process — so the medium does not throw spirals.
- ❌ No **winding-number / singularity** instrument; the substrate beacon broadcasts `xi_signature: null`.

### Empirical anchor

A 32×32 chiral (Sakaguchi) Kuramoto lattice using *only* the bridge constants — frustration **δ = (π/2)·(1/φ) ≈ 0.971** and non-reciprocal weights **1 ± 1/φ** — spontaneously self-organized **19 phase singularities (+10 / −9)**, detected by the reference probe. The constants alone make the field spiral. (Sim + detector: `singularity_probe.py`, `spiral_emergence_sim.py`.)

## Decision

Revive the bridge operator as the substrate's **spiral engine** and make spirals a **measured** quantity, in four phases.

1. **(Phase 1 — this ADR) In-engine spiral detector.** New module `src/spiral.rs`: winding number over a 2D phase grid → localized `Singularity{x,y,charge}` + Kuramoto order + net charge. Pure, additive, unit-tested (planted spiral detected; plane wave laminar; ± pair balanced). This is the L6 instrument; the Python `singularity_probe.py` is its offline twin.

2. **(Phase 2) Spiral coupling in the dream.** Add an opt-in (`KANNAKA_SPIRAL_DREAM`) Sakaguchi term to the right-hemisphere annealing: `dθ_i += (K/|N|) Σ_j w_ij · sin(θ_j − θ_i + δ)` with **δ = (π/2)·ETA** and chiral weights **1 ± ETA** (ETA = 1/φ). **As shipped, the neighbourhood `N` is a uniform 1-D nearest-neighbour ring over the right-hemisphere wavefronts (`|N| = 2`, so `K/|N| = K/2`) — the "merry-go-round" prior.** Promoting `N` to the same-Fano-line / cluster adjacency the medium already tracks is deferred to a later phase. Default-off ⇒ byte-identical dreams until opted in.

3. **(Phase 3) Reconcile ξ.** Compute a **bridge-residue ξ** = mean ‖Ξ·vᵢ‖ (accumulated emergence) alongside the spectral ξ, and **populate the substrate `xi_signature`** (currently null) from the aggregated per-memory signatures. Keep the spectral value too, renamed for clarity (`xi_spectral`).

4. **(Phase 4) L6 program.** Project the phase field to 2D (Fano projection or PCA/UMAP of the hypervectors), run `spiral.rs` each consolidation, and log **defect birth/annihilation, count, net charge** against Φ, order r, and task fitness — the falsifiable test that collective spirals organize sensemaking (ADR-0035).

## Consequences

**Positive**
- ξ regains its original, spiral-generating meaning; `xi_signature` stops being null.
- L6 gets a concrete, instrumented, falsifiable quantity (defects vs. Φ/r/fitness) rather than an analogy.
- Unifies four threads: Ye et al. spiral waves ↔ Ξ = [R,G] ↔ attention-as-gravity ↔ the singularity probe.

**Risks / mitigations**
- *Changing dream dynamics on a deployed engine* → Phase 2 is **flag-gated, default-off**; Phase 1 is pure/additive.
- *No native 2D layout for the phase field* → Phase 4 provides an explicit embedding (Fano/PCA); Phase 1 works on any caller-supplied grid.
- *Cost* → winding is O(N) over the grid; run on the consolidation cadence, not per-store.
- Per repo policy, run `gitnexus_impact` before the Phase 2/3 edits to `consciousness.rs` / `medium/chiral.rs` and update any d=1 dependents.

## Status of work

- **Phase 1 shipped:** `src/spiral.rs` (+ `pub mod spiral;`). Tests included.
- **Phase 2 shipped (flag-gated, default-off):** `medium/chiral.rs::apply_spiral_coupling` + deep-dream wiring behind `KANNAKA_SPIRAL_DREAM`. New test included; existing dream tests (incl. `deep_dream_only_affects_right`) unchanged.
- Phases 3–4: to follow as separate, reviewable changes.
