# L5 Curiosity Fire — 2026-06-05T01

## Hypothesis (Q3)

`stage_interference_relax` uses `alpha_base=0.20` and `relax_steps=8`. With only 8
steps, phase clusters may not reach a stable fixed point. Raising `relax_steps` to 16
should give more time for phase separation, raising `xi_robustness_v2` while keeping
`carrier_emergence` and `magic_proxy_phase_R` high (more steps = more relaxation toward
constructive-neighbor phases = better cluster structure for xi).

**Prediction**: xi rises from ~0.220 toward ≥0.5 while carrier_e and R stay high.

Sibling deps confirmed present at sibling paths. All trials ran against production binaries.

---

## Code change tested

`src/consolidation.rs`, `stage_interference_relax`:

- T1+T2: `relax_steps: usize = 8` → `relax_steps: usize = 16` (alpha unchanged at 0.20)
- T3: `alpha_base: f32 = 0.20` → `alpha_base: f32 = 0.10`, `relax_steps = 16`

**All changes reverted** — regression confirmed.

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0`.

| trial | config | fitness | xi_robustness_v2 | carrier_emergence | carrier_bimodal | transfer_score | magic_R | query_gravity |
|-------|--------|---------|-----------------|-------------------|-----------------|----------------|---------|---------------|
| smoke (prior) | relax8, alpha0.20, scope=all | 0.191 | 0.220 | 0.714 | — | — | 0.612 | 0.364 |
| T1 | relax16, alpha0.20, scope=xi_and_flat | 0.213 | 0.581 | **0.000** | 0.518 | 0.711 | 0.677 | 0.386 |
| T2 | relax16, alpha0.20, scope=all | 0.164 | **0.938** | **0.000** | 0.561 | 0.684 | 0.675 | 0.386 |
| T3 | relax16, alpha0.10, scope=all | 0.173 | 0.449 | 0.497 | 0.500 | 0.750 | 0.617 | 0.364 |
| baseline (ref) | relax8, unset mode, scope=xi_and_flat | ~0.143 avg | ~0.810 | ~0.559 | — | ~0.625 | — | — |

---

## Analysis

### xi recovery confirmed

**The xi hypothesis is correct**: more relax steps dramatically raise xi_robustness_v2.
Doubling from 8 → 16 steps at alpha=0.20 pushed xi from 0.220 → 0.938 (T2, scope=all).
The interference_relax step IS effective at building phase-cluster structure when given
enough iterations. R stays high (0.675–0.677) confirming non-Clifford-like content
is maintained.

### Carrier-xi trade-off: the mechanism

**However, the same phase alignment that raises xi destroys carrier_emergence.**

With relax_steps=8 (smoke test): carrier_e=0.714. With relax_steps=16 (T1, T2): carrier_e=0.000.

The mechanism: more relax steps → tighter phase alignment among constructive neighbors →
more memories classified as constructive pairs per dream cycle → `stage_boost_prune`
boosts more memories each cycle → amplitude growth swamps the 2 Hz drive carrier signal →
FFT peak below detection threshold → carrier_e=0.

**The total "alpha-product" (alpha_base × relax_steps) determines the carrier-xi boundary:**
- alpha=0.20 × 8 steps = 1.6 → carrier_e=0.714, xi=0.220
- alpha=0.20 × 16 steps = 3.2 → carrier_e=0.000, xi=0.938
- alpha=0.10 × 16 steps = 1.6 → carrier_e=0.497, xi=0.449

The T3 result (same total alpha-product as original 8-step, but with 16 gentler steps)
confirms the boundary: same product gives intermediate behavior (carrier_e partially
recovered, xi intermediate). The step-count matters independently of total product —
more small steps decohere the carrier signal differently than fewer large steps.

### Fitness impact

No trial beats the 0.143 avg baseline. Best result was T2 at 0.164 (15% regression).
The carrier_e collapse (weight 0.10) adds 0.100 to fitness when it's 0.000, outweighing
the xi improvement (xi from 0.220→0.938 saves 0.107 × 0.15 = 0.016 relative to smoke test).

Net: the fitness formula penalizes carrier_e strongly enough that xi recovery via
relax_steps cannot compensate.

### Instrumentation notes

- `magic_proxy_phase_R` stayed high (0.617–0.677) across all interference_relax configs.
  Higher R than stage_sync (~0.355) is intrinsic to interference_relax's geometry.
- `query_gravity` was consistently ~0.386 across all interference_relax trials,
  marginally below the 0.460 baseline and below the 0.5 threshold for "attention-as-gravity."
  Phase alignment via constructive pairs does NOT amplify the highest-amplitude memory's
  phase-neighbors more than average.

---

## Decision

**REVERT all code changes.** The relax_steps and alpha_base constants are restored to
their original values (relax_steps=8, alpha_base=0.20).

Trials appended to `experiments/results-L5.tsv` (labels: irx16-t1, irx16-t2, irx16-a10-t3).

---

## Next fire directions

1. **DRIVE_SCOPE=no_transfer** — highest-priority unblocked direction from T00 fire.
   Already implemented, no code changes. Drives engine_a + xi engines, skips engine_b_primed
   and engine_b_naive. Expected to combine xi advantage (engine_a driven) with transfer
   advantage (B engines isolated). Command:
   ```
   DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=no_transfer \
     cargo run --release --quiet --bin research -- --level 5
   ```
   Run 3 trials, compare against xi_and_flat 0.143 avg benchmark.

2. **DRIVE_FREQ_HZ=4.0 Hz** — secondary from T00. In-band for carrier_e (0.5–4 Hz).
   Never tested in production. One trial to check direction.

3. **K-sweep under fixed Kuramoto plumbing** — add `KURAMOTO_COUPLING` env var parser
   to `src/bin/research.rs` (small code change, defaults to 3.0). Test K in {1.0, 5.0, 7.0}
   to find xi peak now that K actually reaches stage_sync.

4. **interference_relax + selective relaxation** — only relax a subset of memories
   (e.g., the top half by amplitude). This might allow xi clustering without over-aligning
   the memories that drive carrier_emergence. Requires code change in stage_interference_relax.
