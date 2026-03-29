# CS Execution Plan — Batch Implementation

## What's done
- CS-1 ✅ (daf59ea) — Hemispheric relabeling

## Batch 1: Metrics + Callosal Fix (CS-2, CS-4, CS-5)
All three are small, independent, and can be implemented together.

### CS-2: Fix callosal asymmetry direction
- **Problem:** asymmetry=2.0 makes HolisticToAnalytical (focusing) 2x FASTER, but ADR-0024 says focusing should be MORE EXPENSIVE
- **Fix:** Swap the multiplication — AnalyticalToHolistic gets the 2x multiplier (defocusing is cheap), HolisticToAnalytical gets divided (focusing is expensive)
- **File:** callosum.rs `effective_rate()` — swap the match arms
- **Test:** Existing tests should still pass (they test the mechanism, not the direction)

### CS-4: Hemispheric Divergence (Δ)
- **Implementation:** Add to ChiralConsciousness struct
- Compute cosine distance between left and right hemisphere mean wavefronts
- Add `hemispheric_divergence: f32` field to ChiralConsciousness
- Compute in `consciousness_summary()`
- **File:** chiral.rs

### CS-5: Callosal Efficiency (κ)
- **Implementation:** Track resonance success in CallosumStats
- Add `successful_resonances: usize` and `efficiency: f32` to CallosumStats
- A transfer is "successful" if the receiving hemisphere's energy for that wavefront exceeds gate_threshold after transfer
- Compute κ = successful / total
- **File:** callosum.rs

### Expose in SystemReport
- Add Δ and κ to `full_report()` output in observe.rs
- Add to `format_report()` display string
- Add to `kannaka status` JSON output in kannaka.rs

## Batch 2: Dream Params (CS-7)
- Add per-hemisphere annealing parameters to dream_native()
- Right hemisphere: lower temperature (preserve broad patterns)  
- Left hemisphere: higher prune threshold (sharpen boundaries)
- **File:** dynamics.rs, chiral.rs dream methods

## Batch 3: Research metrics (CS-3, CS-8, CS-9)
These need more thought. Defer to separate sessions.
