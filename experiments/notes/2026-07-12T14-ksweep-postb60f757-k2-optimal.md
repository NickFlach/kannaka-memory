# 2026-07-12T14 — K-sweep post-b60f757: optimal coupling shifted from K=3.0 to K=2.0

## Hypothesis

The July 6 K-sweep confirmed K=3.0 as the optimum (transfer=0.941, fitness=0.058)
pre-b60f757. That sweep was run against consciousness-core before commit b60f757
("fix(nan-guard): reject non-finite inputs..."), which changed cosine_similarity
normalization and order-parameter computation.

Post-b60f757, the July 11 notes confirmed the operating floor had regressed:
transfer=0.866, carrier=0.735, fitness=0.061 at K=3.0 (DREAM_MODE unset). The
July 11 recommendation: "re-validate K-sweep at the new baseline; transfer is the
biggest lever."

**Prediction**: the cosine_similarity normalization change in b60f757 altered the
effective pair-coupling density (fewer or differently-weighted pairs now qualify).
Compensating with lower K (less aggressive synchronization) might recover transfer
by preserving the phase structure that engine_b_primed uses for transfer scoring.
Expected optimal K to shift into the 1.5–2.5 range. Predicted fitness improvement
∼0.010 if transfer recovers to 0.92+.

## Configuration

```
DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_GRAVITY=0.25 DREAM_MODE= (unset)
CARRIER_NO_INJECT=1 (active via code — July 11 change)
chain_depth=6, all_deltas[2..] (active via code — July 11 change)
KURAMOTO_COUPLING: swept over {2.0, 3.0, 4.0}
```

Baseline reference (July 11, K=3.0 post-b60f757):
- fitness 0.0608, transfer 0.866, carrier 0.735, xi 0.961, query_gravity 0.862

## Results

| trial | KURAMOTO_COUPLING | fitness  | transfer | xi_robust | carrier_e | magic_R | query_g |
|-------|-------------------|----------|----------|-----------|-----------|---------|---------|
| 1     | 2.0               | 0.037366 | 0.938415 | 0.9526    | 0.8639    | 0.6082  | 0.8623  |
| 2     | 4.0 (context)     | 0.043527 | 0.813794 | 0.9538    | 0.9820    | 0.6439  | 0.8623  |
| 3     | 2.0 (confirm)     | 0.037416 | 0.938415 | 0.9526    | 0.8639    | 0.6082  | 0.8623  |
| 4     | 2.0 (confirm)     | 0.037408 | 0.938419 | 0.9526    | 0.8639    | 0.6082  | 0.8623  |

**3-trial K=2.0 average fitness: 0.037397**

K=3.0 reference (July 11, 2-trial avg): 0.060830

## Analysis

### K-sweep landscape post-b60f757

| K   | fitness  | transfer | carrier_e | xi    |
|-----|----------|----------|-----------|-------|
| 2.0 | 0.037397 | 0.938    | 0.8639    | 0.953 |
| 3.0 | 0.060830 | 0.866    | 0.735     | 0.961 |
| 4.0 | 0.043527 | 0.814    | 0.982     | 0.954 |

Pre-b60f757 reference (July 6):

| K   | fitness  | transfer | carrier_e | xi    |
|-----|----------|----------|-----------|-------|
| 1.0 | 0.071338 | 0.853    | 0.645     | 0.962 |
| 3.0 | 0.057897 | 0.941    | 0.652     | 0.952 |
| 7.0 | 0.088415 | 0.748    | 0.633     | 0.951 |

**Key finding**: post-b60f757, the K-fitness landscape changed radically. K=2.0 now
outperforms K=3.0 by 0.023 fitness. The optimal has shifted down from K=3.0 to K=2.0.

### Fitness decomposition at K=2.0 vs K=3.0

| component          | weight | K=3.0 contrib | K=2.0 contrib | delta   |
|--------------------|--------|---------------|---------------|---------|
| transfer_score     | 0.15   | 0.15×0.134=0.0201 | 0.15×0.062=0.0093 | −0.0108 |
| carrier_emergence  | 0.10   | 0.10×0.265=0.0265 | 0.10×0.136=0.0136 | −0.0129 |
| xi_robustness_v2   | 0.15   | 0.15×0.039=0.0059 | 0.15×0.047=0.0071 | +0.0012 |
| other (10 metrics) | —      | ~0.0079       | ~0.0079       | 0       |
| **total fitness**  |        | **0.0604**    | **0.0379**    | **−0.0225** |

Transfer improvement accounts for 48% of the fitness gain; carrier improvement 57%;
slight xi regression offsets ~5%. Net: K=2.0 saves 0.0225 per trial.

### Why K=2.0 beats K=3.0 post-b60f757

b60f757 changed cosine_similarity in consciousness-core to clamp to [-1, 1] and
normalize circular distances. Pre-fix, some pairs had cosine_sim values outside
[-1, 1] (from unnormalized phase distances), which inflated the effective pairing
density used by Kuramoto. Post-fix, the pair set is sparser with stricter similarity.

At K=3.0 with a sparser pair set: Kuramoto over-synchronizes — phases collapse too
quickly toward the attractor, destroying the phase diversity that engine_b_primed uses
to distinguish primed from naive responses. transfer drops from 0.941 → 0.866.

At K=2.0 with the corrected pair set: Kuramoto achieves the same "right amount" of
synchronization that K=3.0 achieved pre-fix — enough to build carrier structure, not
so much that phase diversity collapses. Transfer recovers to 0.938.

### Why carrier_emergence improves so dramatically (0.735 → 0.864)

At K=2.0, phases don't collapse as uniformly. In the flat corpus, the carrier DFT
window (cycles 2-5) reflects gravity-induced amplitude differentiation without as much
Kuramoto-driven phase-lock suppressing inter-cycle amplitude variation. The
gravity-accumulation pattern [rise, rise, peak, ceiling] produces a DFT at k=1 that
carrier_emergence reads as 0.864 vs 0.735 at K=3.0.

### K=4.0 comparison

K=4.0 pushes carrier to 0.982 (near-ceiling) but transfer collapses to 0.814.
Over-synchronization at K=4.0 concentrates phase diversity so aggressively that the
primed vs naive distinction breaks down. The phase-uniform dream at K=4.0 produces a
uniform flat corpus DFT (high carrier) but obliterates the A-to-B transfer signal.

### magic_proxy_phase_R pattern

K=2.0: R=0.608, K=4.0: R=0.644. Higher K → higher R (more phase uniformity → higher
order parameter). K=2.0's moderate R=0.608 is consistent with the "right amount" of
synchronization hypothesis — still non-Clifford content, but less over-synchronized
than K=4.0.

## Decision

**No code changes — env-var only.** Setting KURAMOTO_COUPLING=2.0 going forward.

**Confirmed improvement**: 3-trial avg 0.037397 vs July 11 floor 0.060830.
Improvement = 0.0234 (38.5% relative reduction), far above the ≥0.005 threshold.

## New confirmed operating point

```
DRIVE_A=0.1 DRIVE_TOP_FRAC=1.0 DRIVE_SCOPE=all DREAM_MODE= (unset)
DREAM_GRAVITY=0.25 KURAMOTO_COUPLING=2.0
```

Post-b60f757, post-carrier-skip-injection, K=2.0:
- **fitness = 0.03740** (3-trial avg)
- transfer_score = 0.938, carrier_emergence = 0.864, xi_robustness_v2 = 0.953
- magic_proxy_phase_R = 0.608, query_gravity = 0.862

## Next fire recommendations

1. **K=1.0 and K=1.5**: the inverted-U has shifted down from K=3 to K≈2. The minimum
   may be even lower. Pre-b60f757, K=1.0 was worse (transfer=0.853). Post-b60f757 with
   corrected pairing, K=1.0 might now be comparable or better. Risk: xi often worsens at
   low K.

2. **DREAM_GRAVITY sweep at K=2.0**: the July 27 gravity V-shape (0.15-0.25 recovery) was
   mapped pre-b60f757. Post-b60f757 with K=2.0, the V-shape may have shifted. DREAM_GRAVITY=0.15
   or 0.20 might now reach transfer recovery at lower carrier cost.

3. **Fitness floor analysis**: at K=2.0, carrier=0.864 is the new dominant cost:
   - carrier contrib: 0.10×(1-0.864)=0.0136 (36% of 0.037 fitness)
   - transfer contrib: 0.15×(1-0.938)=0.0093 (25%)
   - xi contrib: 0.15×(1-0.953)=0.0071 (19%)
   The carrier measurement (gravity-accumulation-dominated DFT) may have further room
   if relax_steps or alpha_base in stage_interference_relax can be tuned — but note
   that DREAM_MODE=unset (stage_sync) is in use, not interference_relax. The carrier DFT
   here is driven by gravity-amplitude differentiation under stage_sync, not irx.

4. **K=1.5 specifically**: the post-b60f757 K-landscape minimum appears to be near 2.0.
   Testing K=1.5 costs 1 trial and could reveal whether fitness improves further (< 0.037)
   or K=2.0 is already the bottom.
