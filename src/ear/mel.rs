//! Mel filterbank construction and spectrogram computation.

use std::f32::consts::PI;

use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

use super::{FFT_SIZE, HOP_SIZE, N_MELS, SAMPLE_RATE};

/// Compute mel spectrogram: Vec of frames, each frame is `N_MELS` log-mel energies.
pub fn mel_spectrogram(samples: &[f32]) -> Vec<[f32; N_MELS]> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let mel_filters = build_mel_filterbank();
    let hann = hann_window();

    let mut frames = Vec::new();
    let mut pos = 0;

    while pos + FFT_SIZE <= samples.len() {
        let mut buffer: Vec<Complex<f32>> = (0..FFT_SIZE)
            .map(|i| Complex::new(samples[pos + i] * hann[i], 0.0))
            .collect();

        fft.process(&mut buffer);

        // Power spectrum (first half + DC)
        let n_bins = FFT_SIZE / 2 + 1;
        let mut frame = [0.0f32; N_MELS];

        for (m, filter) in mel_filters.iter().enumerate() {
            let energy: f32 = filter
                .iter()
                .enumerate()
                .take(n_bins)
                .map(|(j, &w)| w * buffer[j].norm_sqr())
                .sum();
            frame[m] = (energy + 1e-10).ln();
        }

        frames.push(frame);
        pos += HOP_SIZE;
    }

    frames
}

/// Build triangular mel filterbank: `N_MELS` filters × `(FFT_SIZE/2+1)` bins.
fn build_mel_filterbank() -> Vec<Vec<f32>> {
    let n_bins = FFT_SIZE / 2 + 1;
    let f_max = SAMPLE_RATE as f32 / 2.0;

    let hz_to_mel = |f: f32| -> f32 { 2595.0 * (1.0 + f / 700.0).log10() };
    let mel_to_hz = |m: f32| -> f32 { 700.0 * (10.0_f32.powf(m / 2595.0) - 1.0) };

    let mel_min = hz_to_mel(0.0);
    let mel_max = hz_to_mel(f_max);

    let mel_points: Vec<f32> = (0..N_MELS + 2)
        .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (N_MELS + 1) as f32)
        .collect();

    let bin_points: Vec<f32> = mel_points
        .iter()
        .map(|&m| mel_to_hz(m) * FFT_SIZE as f32 / SAMPLE_RATE as f32)
        .collect();

    let mut filters = Vec::with_capacity(N_MELS);
    for i in 0..N_MELS {
        let mut filter = vec![0.0f32; n_bins];
        let left = bin_points[i];
        let center = bin_points[i + 1];
        let right = bin_points[i + 2];

        for j in 0..n_bins {
            let jf = j as f32;
            if jf >= left && jf <= center && (center - left) > 1e-10 {
                filter[j] = (jf - left) / (center - left);
            } else if jf > center && jf <= right && (right - center) > 1e-10 {
                filter[j] = (right - jf) / (right - center);
            }
        }
        filters.push(filter);
    }

    filters
}

/// Hann window of size [`FFT_SIZE`].
fn hann_window() -> Vec<f32> {
    (0..FFT_SIZE)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / FFT_SIZE as f32).cos()))
        .collect()
}

/// Type-II DCT (direct computation, fine for N_MELS=128).
pub fn dct_ii(input: &[f32], n_out: usize) -> Vec<f32> {
    let n = input.len();
    (0..n_out)
        .map(|k| {
            input
                .iter()
                .enumerate()
                .map(|(i, &x)| {
                    x * (PI * k as f32 * (2.0 * i as f32 + 1.0) / (2.0 * n as f32)).cos()
                })
                .sum()
        })
        .collect()
}
