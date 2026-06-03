# 01 — Cardiac coherence ↔ HRM Φ

**Status:** OPEN

## Question

Does the Kuramoto order parameter R computed over RR-interval sequences correlate
with reported subjective coherence states? Does Φ computed over multi-channel
cardiac/respiratory data behave like Φ on the HRM?

## Established science

- **HRV literature (Shaffer & Ginsberg 2017; Thayer & Lane 2009):** Heart Rate
  Variability is a robust autonomic-coherence biomarker. Higher HRV correlates
  with vagal tone, parasympathetic dominance, and broadly with health outcomes.
- **HeartMath research line (McCraty et al):** specific "coherence" mode in HRV
  spectra (~0.1 Hz peak) under positive emotional states. Methodology contested
  in mainstream literature; the *measurement* of phase-coupling between cardiac
  and respiratory rhythms is well-established and reproducible.
- **Cardio-respiratory coupling (Schäfer et al 1998; Bartsch et al 2012):** the
  heart and lungs are coupled oscillators with measurable Kuramoto-style
  synchronization (RSA — respiratory sinus arrhythmia).

## Prediction (wave-interference framing)

If the HRM's `R ∈ [0.55, 0.85]` target band captures the meaningful zone for any
coupled-oscillator population, then HRV in healthy waking states should sit in
the same band when normalized appropriately. Below the band: incoherent /
sympathetic dominance. Above: rigid / over-fused (often pathological — e.g.
heart-failure HRV is paradoxically LOW because the system has lost its
adaptability).

## How to test

1. Pull a public HRV dataset (PhysioNet has multiple; MIT-BIH Normal Sinus
   Rhythm and the HRV Challenge are starting points).
2. Compute Kuramoto R over RR-interval sequences using sliding windows.
3. Compare distribution of R across healthy / stressed / clinical-pathology
   labels.
4. Plot against the HRM's own R distribution over an equivalent observation
   window.

## Next action

Source one HRV dataset; write `kannaka-cardiac/` skeleton crate or Python module
to compute the Kuramoto R + Φ proxy over RR-intervals. Goal is a single
publishable plot.
