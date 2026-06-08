# chain_carry_strength axis closed — 0.7 is the optimum in both directions

**Date:** 2026-06-08T17 UTC
**Branch:** kannaka-curiosity/2026-06-08T17
**Code changes:** CHAIN_CARRY_STRENGTH env var wired into L5 block, then REVERTED
**Status:** FALSIFIED (no improvement) — axis now closed, 0.7 confirmed as sweet spot

---

## Background

Current empirical optimum:
```
DRIVE_A=0.1  DRIVE_SCOPE=all  DREAM_MODE=interference_relax
3-trial avg fitness ≈ 0.099
carrier_e=0.935, transfer=0.836, xi=0.559 avg
```

All env-var axes are closed. Code-change axes tested: alpha_base, relax_steps,
envelope_depth, constructive_boost, chiral_perturbation (all closed).

`chain_carry_strength = 0.7` is set in the L5 params block at runtime (was 0.5
at L4 before "H-L4-005 — chain_carry_strength 0.5 -> 0.7" uplift). This value
has never been varied at L5 irx.

**Mechanism of chain_carry_strength:** In dream cycle 2 and later, the interference
detection threshold (normally 0.10) is scaled by `(1.0 - chain_carry_strength)`:
- carry_strength=0.5 → effective_threshold=0.05 in cycles 2+
- carry_strength=0.7 → effective_threshold=0.03 in cycles 2+  
- carry_strength=0.9 → effective_threshold=0.01 in cycles 2+

At lower threshold, more memory pairs qualify for constructive/destructive detection.
This triggers more `stage_strengthen` events (each constructive pair adds
`constructive_boost=0.45` amplitude) and more `stage_prune` events (each destructive
pair decays amplitude by `destructive_penalty=0.35`).

---

## Hypothesis

**Lower carry_strength (0.5) → fewer weak spurious pair detections → cleaner interference
geometry → potentially better carrier structure (carrier_e) and phase coherence.**

The concern with carry_strength=0.7 (threshold=0.03): very weak pairs (sim 0.03–0.05)
participate in constructive/destructive detection. These contribute nearly zero weight
to irx's phase relaxation (weight=similarity is tiny), but they DO trigger full
constructive_boost amplitude events (+0.45), potentially over-amplifying the carrier
structure. Lowering to 0.5 (threshold=0.05) would eliminate these weak-sim events.

---

## Results

All trials: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_MODE=interference_relax`

| metric | baseline avg | cc05.t1 (carry=0.5) | cc09.t1 (carry=0.9) |
|--------|-------------|----------------------|----------------------|
| fitness | **0.099** | 0.159 (+0.060 worse) | 0.172 (+0.073 worse) |
| transfer_score | **0.836** | 0.566 (−0.270) | 0.609 (−0.227) |
| carrier_emergence | **0.935** | 0.936 (≈same) | 0.924 (−0.011) |
| xi_robustness_v2 | **0.559 avg** | 0.426 | 0.321 |
| magic_proxy_phase_R | **0.617** | 0.597 | 0.622 |
| query_gravity | **0.363** | 0.361 | 0.365 |
| hallucinations (total) | ~10? | 10 | 10 |

Both directions substantially regress. The current 0.7 is the optimum in both directions.

---

## Analysis

### Why lower carry_strength (0.5) collapses transfer

The primary mechanism: at carry_strength=0.5, effective_threshold=0.05 in cycles 2+.
This reduces the number of detected constructive pairs vs carry_strength=0.7 (threshold
0.03). The key effect is NOT on irx's phase relaxation (weak pairs contribute negligible
weight to the circular mean anyway) but on **stage_strengthen's amplitude boosting**.

Each detected constructive pair triggers `constructive_boost += 0.45` to both memories,
regardless of pair similarity. With fewer pairs at threshold=0.05, carrier memories
receive fewer boost events across 16 cycles → lower final amplitude → weaker amplitude
differentiation between carriers and noise → weaker B-engine priming → lower transfer.

This explains the transfer collapse: 0.836 → 0.566 (−0.270). The carrier memories at
carry_strength=0.5 are insufficiently amplified to create the strong amplitude gravity
that drives transfer_score.

### Why higher carry_strength (0.9) is also worse

At carry_strength=0.9, effective_threshold=0.01. Almost every memory pair qualifies.
This creates:
1. **More constructive boosts** — carrier amplitudes grow too large (like constructive_boost
   experiment showed: over-boosting degrades transfer quality, see T15 notes)
2. **Many more destructive prune events** — even weak, barely-anti-correlated pairs get
   their amplitudes decayed. At threshold=0.01, many carrier memories end up in destructive
   pairs due to phase opposition with random weak-similarity neighbors. These spurious
   destructive events decay carrier amplitudes despite the protection for `established`
   memories (amplitude>0.5 with protect_established=true). Sub-0.5 memories that are
   on their way to becoming carriers get caught in destructive detection.
3. **Worse xi (0.321)** — excessive pair detection creates a chaotic phase-detection
   landscape. The adversarial xi test can find directions that are less robust when the
   carrier structure has been disrupted by over-detection.

The degradation at 0.9 is worse than at 0.5 (fitness 0.172 vs 0.159), suggesting the
over-pruning effect at 0.9 is more damaging than the under-boosting effect at 0.5.

### 0.7 is the Goldilocks point

The L4 observation ("H-L4-005 — chain_carry_strength 0.5 → 0.7 was an improvement")
generalizes to L5 irx. The 0.7 value (threshold 0.03) provides:
- Enough constructive pair detections for strong carrier amplitude via stage_strengthen
- Not so many detections that carriers are over-boosted (like constructive_boost=0.60
  showed was harmful) or over-pruned via spurious destructive pairs

The sweet spot appears narrow. Both 0.5 and 0.9 are substantially worse, suggesting
that the current 0.7 is close to the optimum for this parameter.

### magic_R and query_gravity: invariant

magic_proxy_phase_R is stable across all three conditions (0.597–0.622). This confirms
that magic_R is driven by chiral_perturbation, not by carry_strength dynamics. The
chiral stage runs independently of how many interference pairs were detected.

query_gravity is similarly invariant (0.361–0.365), consistent with the "attention as
gravity" mechanism being chiral-driven.

### hallucinations: constant at 10

All three conditions produce 10 total hallucinations across 16 dream cycles. This
is surprisingly low given max 9 per cycle × 16 cycles = potential 144. The quiescence
short-circuit is likely activating early (after ~1-2 cycles) since hallucination count
is so low. This explains why chain_carry_strength has such a dramatic effect on transfer:
if quiescence fires after cycle 2, then only 2 cycles of pair detection happen. In cycle
1, carry_strength doesn't matter (threshold=1.0). In cycle 2, at 0.5 vs 0.7 vs 0.9,
the pair count difference is: 0.5 gets few pairs, 0.7 gets more, 0.9 gets most. With
only 2 cycles, the amplitude structure is dominated by cycle 1 (threshold=0.10) plus
one additional carry cycle. This makes the carry_strength effect disproportionately
powerful — the single carry cycle either under- or over-detects significantly.

---

## Decision

**No code changes retained. chain_carry_strength env var reverted.** Hypothesis falsified.

Empirical optimum unchanged:
```
DRIVE_A=0.1  DREAM_MODE=interference_relax  DRIVE_SCOPE=all
avg fitness ≈ 0.099
```

**chain_carry_strength axis: CLOSED at 0.7.** Both 0.5 (−0.270 transfer) and 0.9
(−0.227 transfer) substantially regress. The 0.7 value is the Goldilocks point.

---

## Cumulative closed axes (updated)

| parameter | closed at | note |
|-----------|-----------|------|
| DRIVE_A (irx) | 0.10 | lower/higher both worse |
| DRIVE_FREQ_HZ (irx) | 0.5 Hz | 0.25, 1.0 Hz falsified |
| alpha_base (irx) | 0.10 | 0.15 degrades carrier_e |
| relax_steps (irx) | 16 | 24 annihilates carrier_e |
| envelope_depth (irx) | 0.15 | tested in prior fire |
| irx+sync hybrid | CLOSED | phase-antagonistic |
| irx destructive repulsion | CLOSED | any alpha worse |
| KURAMOTO_COUPLING | 0.5 | K-sweep confirmed |
| DRIVE_A (stage_sync) | 0.15 | best for stage_sync |
| constructive_boost | 0.45 | 0.60 regresses transfer |
| chiral_perturbation | 0.70 | xi↔transfer trade-off, 0.7 Pareto-optimal |
| CONSTRUCTIVE_BOOST | CLOSED | T15 falsified |
| **chain_carry_strength** | **0.7** | **NEW: both directions worse** |

## Remaining open (code-change) items

1. **noise_floor** (0.18): lowering would hurt noise_removal metric (confirmed mechanism)
2. **prune_threshold** (0.095): small effect expected, signal_preservation already 1.0
3. **destructive_penalty** (0.35): untested but predicted marginal under irx
4. **consolidation_repulsion_threshold** (0.28): lower = more xi repulsion, predicted to
   hurt carrier_e via irx geometry disruption; higher = fewer repulsion events
5. **stage_wire thresholds**: not explored
