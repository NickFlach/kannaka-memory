//! Neural Code Switching — modality axis divergence and switch-point detection.
//!
//! NCS Phase 2.1: Compute the principal axis (mean vector) for each modality
//! cluster and measure angular divergence between them.
//!
//! NCS Phase 2.2: Detect switch points where the incoming memory's modality
//! resonance shifts from one cluster to another.

use std::collections::HashMap;

use super::Medium;
use super::types::{Modality, WAVEFRONT_DIM};

// ---------------------------------------------------------------------------
// Phase 2.1 — Modality axis divergence
// ---------------------------------------------------------------------------

/// Principal axis for a single modality cluster.
#[derive(Debug, Clone)]
pub struct ModalityAxis {
    /// Which modality this axis represents.
    pub modality: Modality,
    /// Mean (centroid) vector of all wavefronts tagged with this modality.
    /// Normalized to unit length.
    pub centroid: Vec<f32>,
    /// Number of wavefronts in the cluster.
    pub count: usize,
}

/// Divergence angle (in degrees) between two modality axes.
#[derive(Debug, Clone)]
pub struct AxisDivergence {
    pub modality_a: Modality,
    pub modality_b: Modality,
    /// Cosine similarity between the two centroids.
    pub cosine_similarity: f32,
    /// Angular divergence in degrees (0 = identical, 90 = orthogonal).
    pub angle_degrees: f32,
}

/// Full divergence report across all modalities present in the medium.
#[derive(Debug, Clone)]
pub struct DivergenceReport {
    pub axes: Vec<ModalityAxis>,
    pub divergences: Vec<AxisDivergence>,
}

impl Medium {
    /// Compute the principal axis (mean unit vector) for each modality that
    /// has at least one wavefront in the medium.
    pub fn modality_axes(&self) -> Vec<ModalityAxis> {
        // Accumulate sum vectors per modality
        let mut sums: HashMap<Modality, Vec<f32>> = HashMap::new();
        let mut counts: HashMap<Modality, usize> = HashMap::new();

        for (i, meta) in self.metadata.iter().enumerate() {
            let m = meta.modality;
            // Skip Unknown / Mixed — they don't form a meaningful axis
            if m == Modality::Unknown || m == Modality::Mixed {
                continue;
            }

            let entry = sums
                .entry(m)
                .or_insert_with(|| vec![0.0f32; WAVEFRONT_DIM]);
            let row = self.wavefronts.row(i);
            for (j, val) in row.iter().enumerate() {
                entry[j] += val;
            }
            *counts.entry(m).or_insert(0) += 1;
        }

        let mut axes = Vec::new();
        for (modality, mut sum_vec) in sums {
            let count = counts[&modality];
            // Normalize to unit length
            let norm: f32 = sum_vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 1e-12 {
                for v in &mut sum_vec {
                    *v /= norm;
                }
            }
            axes.push(ModalityAxis {
                modality,
                centroid: sum_vec,
                count,
            });
        }

        // Sort by variant name for deterministic output
        axes.sort_by(|a, b| format!("{}", a.modality).cmp(&format!("{}", b.modality)));
        axes
    }

    /// Compute the pairwise angular divergence between all modality axes.
    pub fn axis_divergence_matrix(&self) -> DivergenceReport {
        let axes = self.modality_axes();
        let mut divergences = Vec::new();

        for i in 0..axes.len() {
            for j in (i + 1)..axes.len() {
                let cos_sim = cosine_sim(&axes[i].centroid, &axes[j].centroid);
                let clamped = cos_sim.clamp(-1.0, 1.0);
                let angle_rad = clamped.acos();
                let angle_deg = angle_rad.to_degrees();

                divergences.push(AxisDivergence {
                    modality_a: axes[i].modality,
                    modality_b: axes[j].modality,
                    cosine_similarity: cos_sim,
                    angle_degrees: angle_deg,
                });
            }
        }

        DivergenceReport { axes, divergences }
    }
}

// ---------------------------------------------------------------------------
// Phase 2.2 — Resonance-based switch-point detection
// ---------------------------------------------------------------------------

/// A detected modality switch point in a sequence of memories.
#[derive(Debug, Clone)]
pub struct SwitchPoint {
    /// Index in the chronological sequence where the switch occurs.
    pub index: usize,
    /// Modality of the memory just before the switch.
    pub from_modality: Modality,
    /// Modality of the memory at the switch point.
    pub to_modality: Modality,
    /// Cosine similarity of the switching memory to the *from* modality centroid.
    pub similarity_to_old: f32,
    /// Cosine similarity of the switching memory to the *to* modality centroid.
    pub similarity_to_new: f32,
}

/// Summary of switch-point analysis over recent memory history.
#[derive(Debug, Clone)]
pub struct SwitchReport {
    /// Detected switch points.
    pub switch_points: Vec<SwitchPoint>,
    /// Total memories analyzed.
    pub memories_analyzed: usize,
    /// The threshold that was used.
    pub switch_threshold: f32,
}

impl Medium {
    /// Detect modality switch points in the chronological memory sequence.
    ///
    /// A switch point occurs when an incoming memory's wavefront vector has
    /// low cosine similarity to the current modality centroid AND high
    /// similarity to another modality's centroid.
    ///
    /// `switch_threshold` controls sensitivity (default 0.3): a switch is
    /// detected when similarity to the current modality drops below this
    /// value while similarity to another exceeds it.
    pub fn detect_switch_points(&self, switch_threshold: f32) -> SwitchReport {
        let axes = self.modality_axes();
        if axes.is_empty() {
            return SwitchReport {
                switch_points: Vec::new(),
                memories_analyzed: self.wavefront_count(),
                switch_threshold,
            };
        }

        // Build centroid lookup by modality
        let centroid_map: HashMap<Modality, &[f32]> = axes
            .iter()
            .map(|a| (a.modality, a.centroid.as_slice()))
            .collect();

        // Walk the chronological sequence (metadata order = insertion order)
        let mut switch_points = Vec::new();
        let mut current_modality: Option<Modality> = None;

        for (i, meta) in self.metadata.iter().enumerate() {
            let m = meta.modality;
            if m == Modality::Unknown || m == Modality::Mixed {
                continue;
            }

            // First concrete-modality memory sets the baseline
            if current_modality.is_none() {
                current_modality = Some(m);
                continue;
            }

            let cur_mod = current_modality.unwrap();
            if cur_mod == m {
                continue; // Same modality — no switch
            }

            // Compute similarity of this wavefront to the current and candidate centroids
            let wave = self.wavefronts.row(i);
            let wave_slice: Vec<f32> = wave.iter().copied().collect();

            let sim_to_current = centroid_map
                .get(&cur_mod)
                .map(|c| cosine_sim(&wave_slice, c))
                .unwrap_or(0.0);

            let sim_to_new = centroid_map
                .get(&m)
                .map(|c| cosine_sim(&wave_slice, c))
                .unwrap_or(0.0);

            // Switch condition: low similarity to current AND high to another
            if sim_to_current < switch_threshold && sim_to_new > switch_threshold {
                switch_points.push(SwitchPoint {
                    index: i,
                    from_modality: cur_mod,
                    to_modality: m,
                    similarity_to_old: sim_to_current,
                    similarity_to_new: sim_to_new,
                });
            }

            // Update current modality to the tagged modality regardless
            // (the tag is ground truth even if resonance didn't trigger a formal switch)
            current_modality = Some(m);
        }

        SwitchReport {
            switch_points,
            memories_analyzed: self.wavefront_count(),
            switch_threshold,
        }
    }

    /// Convenience: detect switch points using the default threshold (0.3).
    pub fn detect_switch_points_default(&self) -> SwitchReport {
        self.detect_switch_points(0.3)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Cosine similarity between two equal-length slices.
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a < 1e-12 || norm_b < 1e-12 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a small test medium with known modality-tagged wavefronts.
    fn make_test_medium() -> Medium {
        let mut m = Medium::new();

        // Audio cluster: vectors concentrated in dims 0..100
        for i in 0..3 {
            let mut v = vec![0.0f32; WAVEFRONT_DIM];
            for j in 0..100 {
                v[j] = 1.0 + (i as f32) * 0.01;
            }
            let id = m.add_wavefront(&v, format!("audio mem {i}"), 1.0).unwrap();
            let idx = m.get_wavefront_index(&id).unwrap();
            m.metadata[idx].modality = Modality::Audio;
        }

        // Visual cluster: vectors concentrated in dims 200..300
        for i in 0..3 {
            let mut v = vec![0.0f32; WAVEFRONT_DIM];
            for j in 200..300 {
                v[j] = 1.0 + (i as f32) * 0.01;
            }
            let id = m.add_wavefront(&v, format!("visual mem {i}"), 1.0).unwrap();
            let idx = m.get_wavefront_index(&id).unwrap();
            m.metadata[idx].modality = Modality::Visual;
        }

        // Semantic cluster: vectors concentrated in dims 400..500
        for i in 0..2 {
            let mut v = vec![0.0f32; WAVEFRONT_DIM];
            for j in 400..500 {
                v[j] = 1.0 + (i as f32) * 0.01;
            }
            let id = m.add_wavefront(&v, format!("semantic mem {i}"), 1.0).unwrap();
            let idx = m.get_wavefront_index(&id).unwrap();
            m.metadata[idx].modality = Modality::Semantic;
        }

        m
    }

    #[test]
    fn axes_have_correct_counts() {
        let m = make_test_medium();
        let axes = m.modality_axes();

        assert_eq!(axes.len(), 3); // Audio, Visual, Semantic

        let audio = axes.iter().find(|a| a.modality == Modality::Audio).unwrap();
        assert_eq!(audio.count, 3);

        let visual = axes.iter().find(|a| a.modality == Modality::Visual).unwrap();
        assert_eq!(visual.count, 3);

        let semantic = axes.iter().find(|a| a.modality == Modality::Semantic).unwrap();
        assert_eq!(semantic.count, 2);
    }

    #[test]
    fn divergence_angles_are_near_90_for_orthogonal_clusters() {
        let m = make_test_medium();
        let report = m.axis_divergence_matrix();

        // Audio (dims 0..100) and Visual (dims 200..300) should be nearly orthogonal
        let av = report.divergences.iter().find(|d| {
            (d.modality_a == Modality::Audio && d.modality_b == Modality::Visual)
                || (d.modality_a == Modality::Visual && d.modality_b == Modality::Audio)
        });

        if let Some(div) = av {
            assert!(
                div.angle_degrees > 80.0,
                "Audio-Visual angle should be near 90 deg, got {:.1}",
                div.angle_degrees
            );
        }
    }

    #[test]
    fn switch_points_detected_in_alternating_sequence() {
        let mut m = Medium::new();

        // Alternate Audio / Visual wavefronts with distinct subspaces
        let audio_vec = {
            let mut v = vec![0.0f32; WAVEFRONT_DIM];
            for j in 0..100 { v[j] = 1.0; }
            v
        };
        let visual_vec = {
            let mut v = vec![0.0f32; WAVEFRONT_DIM];
            for j in 200..300 { v[j] = 1.0; }
            v
        };

        // A, A, V, V, A, V — expect switches at index transitions A→V and V→A
        let patterns: Vec<(Vec<f32>, Modality)> = vec![
            (audio_vec.clone(), Modality::Audio),
            (audio_vec.clone(), Modality::Audio),
            (visual_vec.clone(), Modality::Visual),
            (visual_vec.clone(), Modality::Visual),
            (audio_vec.clone(), Modality::Audio),
            (visual_vec.clone(), Modality::Visual),
        ];

        for (vec, modality) in patterns {
            let id = m.add_wavefront(&vec, format!("{modality} test"), 1.0).unwrap();
            let idx = m.get_wavefront_index(&id).unwrap();
            m.metadata[idx].modality = modality;
        }

        let report = m.detect_switch_points(0.3);
        assert!(
            report.switch_points.len() >= 2,
            "Expected at least 2 switch points, got {}",
            report.switch_points.len()
        );
        // First switch should be Audio → Visual
        assert_eq!(report.switch_points[0].from_modality, Modality::Audio);
        assert_eq!(report.switch_points[0].to_modality, Modality::Visual);
    }

    #[test]
    fn no_switch_points_in_uniform_modality() {
        let mut m = Medium::new();
        for i in 0..5 {
            let mut v = vec![0.0f32; WAVEFRONT_DIM];
            for j in 0..100 { v[j] = 1.0 + (i as f32) * 0.001; }
            let id = m.add_wavefront(&v, format!("audio {i}"), 1.0).unwrap();
            let idx = m.get_wavefront_index(&id).unwrap();
            m.metadata[idx].modality = Modality::Audio;
        }

        let report = m.detect_switch_points(0.3);
        assert_eq!(report.switch_points.len(), 0);
    }
}
