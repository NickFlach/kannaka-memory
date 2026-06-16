# L5 Curiosity: carrier_emergence root-cause diagnosis — DFT resolution mismatch

**Date:** 2026-06-16T00 UTC  
**Branch:** kannaka-curiosity/2026-06-16T00-carrier-e-diagnosis  
**Code changes:** NONE — hypothesis falsified; all changes reverted  
**Status:** NOT KEPT — diagnosis fire, no improvement. Root cause identified.

---

## Context

Current optimum: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness 0.0578.  
Dominant cost: carrier_emergence = 0.533 (contributes 0.10 × (1−0.533) = 0.0467 of 0.0578 total).  
Previous notes hypothesized "structural amplitude smoothing (post-constructive decay)" as the fix.

---

## Hypothesis (falsified)

Post-constructive amplitude decay for non-constructive memories in engine_flat would prevent
ceiling saturation, allowing drive-frequency modulation to appear as a spectral peak in
amplitude_deltas. Tested via `POST_CONSTRUCTIVE_DECAY` env var in stage_strengthen, restricted
to DRIVE_CONTEXT=engine_flat.

**Prediction:** carrier_emergence 0.533 → 0.7+ with decay_rate ∈ {0.03, 0.15}.

---

## Results

| trial | decay_rate | carrier_e | fitness   | notes |
|-------|------------|-----------|-----------|-------|
| t1    | 0.03       | 0.5336    | 0.057765  | no change |
| t2    | 0.15       | 0.5350    | 0.057612  | no change |

Baseline (interference_relax, no decay): carrier_e = 0.5333.

---

## Analysis: why the hypothesis was wrong

### Actual limiting factor: ALL flat-corpus memories form constructive pairs

In engine_flat, all memories have frequency = 0.1 Hz. After the first dream cycle, the
constructive interference phase alignment (avg_phase applied in stage_strengthen) pulls all
memory phases together. By cycle 2, most memory pairs are constructive. The non-constructive
set is empty or nearly empty — decay targeting non-constructive memories has zero leverage.

### Root cause: DFT resolution mismatch with chain_depth=4

Diagnostic run: `amp_deltas_flat = [0.9498, 0.0307, 0.0030, 0.0364]` at chain_depth=4.

With N=4 samples at Ts=0.125s, DFT minimum detectable frequency = 1/(N×Ts) = 1/(4×0.125) = **2.0 Hz**.
DFT bins: k=1 → 2.0 Hz, k=2 → 4.0 Hz. These are the ONLY frequencies in the band [0.5, 4.0] Hz.

The drive is at DRIVE_FREQ_HZ=0.5 Hz. With chain_depth=4, the drive covers only 0.25 periods of
one 0.5 Hz cycle. There is **no DFT bin at 0.5 Hz**. The metric cannot detect the drive frequency.

carrier_emergence = 0.533 reflects the power ratio at 2.0 Hz vs 4.0 Hz from the shape of
[0.9498, 0.0307, 0.0030, 0.0364]. This signal is dominated by the initial constructive cascade
(cycle 0 spike), not the drive frequency. Verified analytically:
- DFT k=1 power ≈ 0.896 (2.0 Hz)
- DFT k=2 power ≈ 0.785 (4.0 Hz)
- carrier_emergence = 0.896 / (0.896 + 0.785) ≈ 0.533 ✓

### Why does the cycle-0 spike dominate?

In the flat corpus: all 0.1 Hz memories form constructive pairs immediately → all get boosted by
+0.3 in cycle 0 → many hit AMPLITUDE_CEILING=2.0. Mean amplitude delta ≈ 0.95 in cycle 0.
After that, all memories are ceiling-saturated: drive pushes to 2.3 (above ceiling) but
constructive boost clips them back to 2.0. Subsequent deltas are nearly zero.

### Why does decay help nothing?

With non-constructive decay: there ARE no non-constructive memories after cycle 1. Zero leverage.
With global decay: constructive memories would also decay. But 3 independent issues remain:
1. The cycle-0 spike is still huge (before decay takes effect)
2. With chain_depth=4, the DFT still can't see 0.5 Hz (need 16 cycles)
3. Online injection at cycle 2 contaminates the flat corpus with 2.0 Hz memories

---

## Root-cause summary

carrier_emergence ≈ 0.533 is a measurement artifact of three compounding factors:

| Factor | Contribution | Fix needed |
|--------|-------------|-----------|
| chain_depth=4 → DFT min freq = 2.0 Hz | Primary | engine_flat needs chain_depth=16 |
| Ceiling saturation cascade (cycle 0) | Secondary | global amplitude decay in engine_flat |
| Online injection at cycle 2 (10 × 2.0 Hz mems) | Tertiary | skip injection for engine_flat |

Without fixing ALL THREE, carrier_e will stay near 0.533.

---

## What the next fire should do (3-part architectural fix)

This is the only credible remaining path to meaningful fitness improvement (0.0467 gain).

### Part 1: engine_flat chain_depth=16

In `src/bin/research.rs`, just before the engine_flat run (~line 3616):
```rust
// engine_flat needs 16 cycles so DFT resolves 0.5 Hz (16×0.125s = 2.0s = 1 full drive period).
// chain_depth=4 gives DFT min=2.0 Hz, which can't detect the 0.5 Hz drive.
let params_flat = { let mut p = (*params).clone(); p.chain_depth = 16; p };
```
Then change `run_l5_dream_chain(params, &mut engine_flat)` to `run_l5_dream_chain(&params_flat, ...)`.

### Part 2: suppress injection in engine_flat

In `src/bin/research.rs`, in `run_l5_dream_chain`, the injection check:
```rust
if injection_cycles.contains(&cycle_idx) {
    let ids = inject_online_memories(engine, dim, injection_counter, params.encoder_seed);
```
Should be wrapped with a context gate:
```rust
let drive_ctx = std::env::var("DRIVE_CONTEXT").unwrap_or_default();
if injection_cycles.contains(&cycle_idx) && drive_ctx != "engine_flat" {
```
This keeps engine_flat as a pure flat-corpus carrier-emergence test (no injected 2.0 Hz memories).

### Part 3: global amplitude decay for engine_flat

In `src/consolidation.rs`, `stage_strengthen`, after bridge-node strengthening:
```rust
if std::env::var("DRIVE_CONTEXT").unwrap_or_default() == "engine_flat" {
    let decay: f32 = std::env::var("FLAT_GLOBAL_DECAY")
        .ok().and_then(|s| s.parse().ok()).unwrap_or(0.0);
    if decay > 0.0 {
        let all_ids: Vec<Uuid> = engine.store.all_memories()
            .unwrap_or_default().iter().map(|m| m.id).collect();
        for id in all_ids {
            if let Ok(Some(mem)) = engine.store.get_mut(&id) {
                mem.amplitude = (mem.amplitude * (1.0 - decay)).max(0.0);
            }
        }
    }
}
```

Steady-state analysis: without drive, constructive steady state = boost / decay.
- At FLAT_GLOBAL_DECAY=0.20 and constructive_boost=0.3: steady_amp = 0.3/0.20 = **1.5** (below ceiling 2.0) ✓
- Drive modulates around 1.5 with amplitude ±0.15×1.5 = ±0.225
- Over 16 cycles, amplitude_deltas form a 0.5 Hz oscillation pattern detectable by DFT

Run test with: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax FLAT_GLOBAL_DECAY=0.20`

**Predicted outcomes** (after all 3 parts):
- carrier_emergence: 0.533 → 0.70–0.90 (DFT resolves 0.5 Hz drive in non-saturated signal)
- fitness: 0.0578 → 0.025–0.040 (carrier_e cost: 0.047 → 0.010–0.030)
- transfer_score, xi, online_retention: UNCHANGED (engine_flat isolation is complete)

Requires minimum 2 trials per the consolidation.rs change constraint.

---

## Decision

No code changes. No TSV rows appended. Notes file committed.

**Current optimum unchanged: `DRIVE_A=0.15 DRIVE_SCOPE=all DREAM_MODE=interference_relax` → fitness 0.0578.**
