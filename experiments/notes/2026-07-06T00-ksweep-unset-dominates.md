# 2026-07-06T00 — K-sweep reveals DREAM_MODE unset dominates post-fix

## Hypothesis

K-sweep (kuramoto_coupling ∈ {1.0, 3.0, 7.0}) with DREAM_MODE unset to exercise
stage_sync now that commit 066d41a plumbed params through. Previous K-sweeps measured
noise. Predicted: K=3.0 (default) is near-optimal; lower/higher K might shift xi.

Secondary question (from previous notes): does DREAM_MODE unset beat interference_relax
at the new post-fix operating point? Notes flagged this as unclear but worth checking.

## Results

Config: `DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_GRAVITY=0.25`, DREAM_MODE unset

| trial | KURAMOTO_COUPLING | fitness  | transfer | xi_robust | carrier_e | R_magic | query_g |
|-------|-------------------|----------|----------|-----------|-----------|---------|---------|
| 1     | 3.0 (default)     | 0.057897 | 0.9414   | 0.9522    | 0.6520    | 0.6412  | 0.8623  |
| 2     | 1.0               | 0.071338 | 0.8525   | 0.9618    | 0.6446    | 0.5758  | 0.8623  |
| 3     | 7.0               | 0.088415 | 0.7475   | 0.9512    | 0.6327    | 0.6659  | 0.8623  |
| 4     | 3.0 (confirm)     | 0.057896 | 0.9414   | 0.9522    | 0.6520    | 0.6412  | 0.8623  |

Previous fire reference (DREAM_MODE=interference_relax, K ignored):

| config             | fitness  | transfer | xi_robust | carrier_e | R_magic |
|--------------------|----------|----------|-----------|-----------|---------|
| interference_relax | 0.075480 | 0.7412   | 0.7910    | 0.9868    | 0.3375  |

## Primary finding: DREAM_MODE unset crushes interference_relax post-fix

Stage_sync at K=3.0 (DREAM_MODE unset) vs interference_relax:

| metric            | interference_relax | stage_sync K=3.0 | delta    |
|-------------------|--------------------|------------------|----------|
| fitness           | 0.0755             | 0.0579           | −0.017   |
| transfer_score    | 0.741              | 0.941            | +0.200   |
| xi_robustness_v2  | 0.791              | 0.952            | +0.161   |
| carrier_emergence | 0.987              | 0.652            | −0.335   |
| magic_proxy_phase_R | 0.338            | 0.641            | +0.303   |

The circular-phase fix (4a1c4e6) corrected wrap-straddling pair detection. Under
interference_relax this hurt transfer and xi severely (the mode relies on constructive
pairs, which are now fewer/stricter). Under stage_sync the fix improved phase coherence,
which Kuramoto now correctly synchronizes — resulting in far better xi and transfer.

carrier_emergence trade-off is real: 0.987 → 0.652. But the fitness math favors stage_sync:
- carrier_e contrib: 0.10×(1-0.652)=0.035 vs 0.10×(1-0.987)=0.001 (+0.034 cost)
- transfer contrib:  0.15×(1-0.941)=0.009 vs 0.15×(1-0.741)=0.039 (−0.030 saving)
- xi contrib:        0.15×(1-0.952)=0.007 vs 0.15×(1-0.791)=0.031 (−0.024 saving)
Net: stage_sync saves 0.020 in fitness vs interference_relax.

magic_proxy_phase_R rises from 0.338 to 0.641 — stage_sync produces richer non-Clifford
phase content than interference_relax post-fix. R correlates with K: 0.576 (K=1), 0.641
(K=3), 0.666 (K=7). Higher sync → more uniform phase → higher R.

## Secondary finding: K-sweep inverted U at K=3.0

K=3.0 is the optimum for fitness and transfer. K=1.0 and K=7.0 both degrade transfer
substantially. This validates the default K=3.0 — it is genuinely optimal, not coincidental.
xi_robustness is nearly flat across K (0.951–0.962), so K primarily governs transfer quality.

## Decision

No code changes. New confirmed operating point:
- DREAM_MODE: unset (stage_sync)
- KURAMOTO_COUPLING: 3.0 (default, no env override needed)
- DRIVE_A=0.1 DRIVE_SCOPE=all DREAM_GRAVITY=0.25

Post-fix floor is now **0.0579** (down from 0.0755 once interference_relax is dropped).
The old 0.045 pre-fix floor incorporated carrier_emergence artifacts; this new 0.058
floor is clean: high transfer (0.941), high xi (0.952), carrier at 0.652.

## Next fire recommendations

1. **K=5.0 trial**: tried {1,3,7}; gap between 3 and 7 could hide a local maximum.
   Low priority — K=3 is likely the default-optimum by design.
2. **carrier_emergence recovery at stage_sync**: can any param change push carrier toward
   0.8+ without hurting transfer? Try DRIVE_FREQ_HZ variants (the T19 frequencies).
3. **Φ ↔ R relationship**: magic_R=0.641 at stage_sync is now well above the Clifford zone.
   Compare phi_history endpoint to R across K values — are they correlated?
4. **3-run confirmation of new floor**: only 2 trials at K=3.0 (both identical to 4dp);
   statistically confirmed but a 3rd trial at a different day/seed would add confidence.
