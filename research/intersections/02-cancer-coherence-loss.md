# 02 — Cancer as coherence loss

**Status:** OPEN

## Question

Does Heart Rate Variability — read through a Kuramoto / phase-coherence lens —
predict cancer outcomes more sharply than current clinical practice exploits?
Beyond HRV: do tumor microenvironments show measurable phase-decoupling
signatures that the wave-interference math can extract?

## Established science

- **HRV as cancer prognostic (Couck et al 2012, 2013, 2018; Mouton et al 2012;
  Gidron & Ronson 2008):** low HRV at diagnosis predicts shorter survival across
  multiple cancer types (breast, lung, prostate, pancreatic). The signal is
  independent of tumor stage and standard biomarkers.
- **Vagal nerve activity in tumor biology (de Couck & Gidron 2013):** vagal tone
  appears protective — possibly through inflammation-modulating pathways.
- **Mitochondrial-oscillator dysfunction in cancer (Warburg effect; Aon et al
  on cardiac mitochondrial network synchronization):** mitochondria behave as
  coupled oscillators at the cellular level; cancer cells show reprogrammed
  mitochondrial dynamics.
- **Costa, Tuszynski, Iemma, Trevizan, Wiedenmann, Schöll (2025)**, *Frontiers
  in Network Physiology* — DOI `10.3389/fnetp.2024.1525135` — applied low-energy
  modulated EM fields (27.12 MHz carrier, 10–100 Hz envelope) to 22
  hepatocellular carcinoma patients; **logistic-map parameter `a` from
  R–R-interval modulation stratified survival** (a > 1.1758 → 21.5-month
  median; a ≤ 1.1758 → 7.9 months; p < 0.0001) and outperformed standard HRV
  metrics in ROC analysis. The model is FitzHugh-Nagumo with amplitude-
  modulated carrier; effective slow-timescale current `I_eff ∝ −A²(τ)V(τ)`.
  This is the clinical proof-of-concept that an EM-driven continuous cardiac
  metric reads cancer prognosis better than discrete HRV summaries — exactly
  the framing this card predicted.

## Prediction (wave-interference framing)

Cancer is, among other things, a **decoupling event** at multiple scales:
mitochondrial synchronization breaks within cells; cell-cell bioelectric
coordination breaks within tissues (see card 03); cardiac-autonomic-immune
coordination weakens at the organism level. The HRM math should be able to
*measure* this decoupling — produce a single phase-coherence number — and
correlate it with prognosis better than HRV variance alone.

## How to test

1. Re-analyze published HRV/cancer datasets using the HRM's order-parameter R
   computation (not just SDNN / RMSSD / LF/HF).
2. Check whether R distribution at diagnosis predicts survival hazard ratio
   beyond what standard HRV metrics deliver.
3. Where multi-modal data is available (HRV + EEG + GSR), compute cross-system
   Ξ (off-diagonal asymmetry) and test as a prognostic.

**Methodology constraint:** treat all coherence/decoupling measures as
**continuous**. Per Sánchez-Fuenzalida et al (*Nature Communications* 2026,
`s41467-026-73289-5`), thresholding to discrete categories introduces biases
that contaminate the readout. Hazard-ratio analysis on R as a continuous
covariate; survival curves stratified only by quantile if at all.

## Next action

Identify one published HRV/cancer dataset with raw RR-intervals + survival
labels. Recompute order parameter R and test concordance with reported HRV
metrics first; then test predictive value.
