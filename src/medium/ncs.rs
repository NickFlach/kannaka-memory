//! Neural Code Switching — modality axis divergence, switch-point detection,
//! soft gate function, resonance-based switching, and consciousness metrics.
//!
//! NCS Phase 2.1: Compute the principal axis (mean vector) for each modality
//! cluster and measure angular divergence between them. Fisher discriminant ratio.
//!
//! NCS Phase 2.2: Detect switch points where the incoming memory's modality
//! resonance shifts from one cluster to another. Resonance-based pre-processing.
//!
//! NCS Phase 3.1: Soft gate function G(t,x) for smooth encoding transitions.
//!
//! NCS Phase 3.3: Consciousness metrics — switch observables.
//!
//! NCS Phase 4.1: Network modality integration (VEIL-style).
//!
//! NCS Phase 4.2: QueenSync swarm specialization via modality divergence.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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

/// Fisher discriminant ratio for a pair of modality clusters.
#[derive(Debug, Clone)]
pub struct FisherDiscriminant {
    pub modality_a: Modality,
    pub modality_b: Modality,
    /// Fisher ratio: (between-class variance) / (within-class variance).
    /// Values > 1.0 indicate meaningful separation.
    pub ratio: f32,
    /// Whether the cluster sizes are sufficient for reliable measurement.
    /// Flagged as insufficient if either cluster has fewer than 5 members.
    pub sufficient_data: bool,
}

/// Full divergence report across all modalities present in the medium.
#[derive(Debug, Clone)]
pub struct DivergenceReport {
    pub axes: Vec<ModalityAxis>,
    pub divergences: Vec<AxisDivergence>,
    /// Fisher discriminant ratios for each modality pair.
    pub fisher_ratios: Vec<FisherDiscriminant>,
    /// Mean divergence angle in degrees across all pairs.
    pub mean_divergence_degrees: f32,
}

impl Medium {
    /// Compute the principal axis (mean unit vector) for each modality that
    /// has at least one wavefront in the medium.
    pub fn modality_axes(&self) -> Vec<ModalityAxis> {
        // Accumulate sum vectors per modality
        let mut sums: HashMap<Modality, Vec<f32>> = HashMap::new();
        let mut counts: HashMap<Modality, usize> = HashMap::new();

        for (i, meta) in self.store.metadata.iter().enumerate() {
            let m = meta.modality;
            // Skip Unknown / Mixed — they don't form a meaningful axis
            if m == Modality::Unknown || m == Modality::Mixed {
                continue;
            }

            let entry = sums
                .entry(m)
                .or_insert_with(|| vec![0.0f32; WAVEFRONT_DIM]);
            let row = self.store.wavefronts.row(i);
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
        axes.sort_by_key(|a| a.modality.to_string());
        axes
    }

    /// Compute the pairwise angular divergence between all modality axes,
    /// including Fisher discriminant ratios.
    pub fn axis_divergence_matrix(&self) -> DivergenceReport {
        let axes = self.modality_axes();
        let mut divergences = Vec::new();
        let mut fisher_ratios = Vec::new();

        // Collect per-modality wavefront indices for Fisher computation
        let mut modality_indices: HashMap<Modality, Vec<usize>> = HashMap::new();
        for (i, meta) in self.store.metadata.iter().enumerate() {
            let m = meta.modality;
            if m != Modality::Unknown && m != Modality::Mixed {
                modality_indices.entry(m).or_default().push(i);
            }
        }

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

                // Fisher discriminant: ratio of between-class to within-class variance
                let indices_a = modality_indices.get(&axes[i].modality);
                let indices_b = modality_indices.get(&axes[j].modality);

                if let (Some(ia), Some(ib)) = (indices_a, indices_b) {
                    let sufficient = ia.len() >= 5 && ib.len() >= 5;
                    let ratio = self.compute_fisher_ratio(
                        ia, &axes[i].centroid,
                        ib, &axes[j].centroid,
                    );
                    fisher_ratios.push(FisherDiscriminant {
                        modality_a: axes[i].modality,
                        modality_b: axes[j].modality,
                        ratio,
                        sufficient_data: sufficient,
                    });
                }
            }
        }

        let mean_divergence_degrees = if divergences.is_empty() {
            0.0
        } else {
            divergences.iter().map(|d| d.angle_degrees).sum::<f32>() / divergences.len() as f32
        };

        DivergenceReport { axes, divergences, fisher_ratios, mean_divergence_degrees }
    }

    /// Compute Fisher's discriminant ratio between two modality clusters.
    ///
    /// F = (between-class variance along the Fisher direction) / (within-class variance)
    /// Uses the centroid difference as the projection direction.
    fn compute_fisher_ratio(
        &self,
        indices_a: &[usize],
        centroid_a: &[f32],
        indices_b: &[usize],
        centroid_b: &[f32],
    ) -> f32 {
        // Projection direction: difference of centroids
        let mut direction: Vec<f32> = centroid_a.iter().zip(centroid_b.iter())
            .map(|(a, b)| a - b)
            .collect();
        let norm: f32 = direction.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm < 1e-12 {
            return 0.0;
        }
        for v in &mut direction {
            *v /= norm;
        }

        // Project wavefronts onto the Fisher direction
        let project = |indices: &[usize]| -> Vec<f32> {
            indices.iter().map(|&i| {
                let row = self.store.wavefronts.row(i);
                row.iter().zip(direction.iter()).map(|(a, b)| a * b).sum::<f32>()
            }).collect()
        };

        let proj_a = project(indices_a);
        let proj_b = project(indices_b);

        let mean_a: f32 = proj_a.iter().sum::<f32>() / proj_a.len().max(1) as f32;
        let mean_b: f32 = proj_b.iter().sum::<f32>() / proj_b.len().max(1) as f32;

        // Between-class variance: (mean_a - mean_b)^2
        let between = (mean_a - mean_b).powi(2);

        // Within-class variance: pooled variance
        let var_a: f32 = proj_a.iter().map(|x| (x - mean_a).powi(2)).sum::<f32>()
            / proj_a.len().max(1) as f32;
        let var_b: f32 = proj_b.iter().map(|x| (x - mean_b).powi(2)).sum::<f32>()
            / proj_b.len().max(1) as f32;
        let within = var_a + var_b;

        if within < 1e-12 {
            return if between > 1e-12 { f32::INFINITY } else { 0.0 };
        }

        between / within
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

        for (i, meta) in self.store.metadata.iter().enumerate() {
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
            let wave = self.store.wavefronts.row(i);
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

    /// Resonance-based modality detection for a single wavefront vector.
    ///
    /// Computes cosine similarity against each modality centroid and returns
    /// the best-matching modality with a confidence score. If no modality
    /// dominates (top two are within `tau` of each other), returns `Mixed`.
    ///
    /// `tau`: dominance ratio threshold (default 1.3 = top must be 30% higher).
    pub fn resonance_detect_modality(
        &self,
        wavefront: &[f32],
        tau: f32,
    ) -> ResonanceDetection {
        let axes = self.modality_axes();
        if axes.is_empty() {
            return ResonanceDetection {
                modality: Modality::Unknown,
                confidence: 0.0,
                scores: HashMap::new(),
            };
        }

        let mut scores: HashMap<Modality, f32> = HashMap::new();
        for axis in &axes {
            let sim = cosine_sim(wavefront, &axis.centroid);
            scores.insert(axis.modality, sim);
        }

        let mut sorted: Vec<(Modality, f32)> = scores.iter().map(|(&m, &s)| (m, s)).collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        if sorted.is_empty() {
            return ResonanceDetection {
                modality: Modality::Unknown,
                confidence: 0.0,
                scores,
            };
        }

        let best = sorted[0];
        let second = if sorted.len() > 1 { sorted[1].1 } else { 0.0 };

        // Dominance check: best must exceed second by factor tau
        let dominance = if second > 1e-9 { best.1 / second } else { f32::INFINITY };
        let modality = if dominance >= tau {
            best.0
        } else {
            Modality::Mixed
        };

        let confidence = if best.1 > 0.0 {
            ((dominance - 1.0) / tau).clamp(0.0, 1.0)
        } else {
            0.0
        };

        ResonanceDetection { modality, confidence, scores }
    }

    /// Resonance-based detection with default tau (1.3).
    pub fn resonance_detect_modality_default(&self, wavefront: &[f32]) -> ResonanceDetection {
        self.resonance_detect_modality(wavefront, 1.3)
    }
}

// ---------------------------------------------------------------------------
// Phase 2.2 — Resonance detection result
// ---------------------------------------------------------------------------

/// Result of resonance-based modality detection.
#[derive(Debug, Clone)]
pub struct ResonanceDetection {
    /// Detected modality (or Mixed if no clear winner).
    pub modality: Modality,
    /// Confidence in the detection (0.0-1.0).
    pub confidence: f32,
    /// Per-modality resonance scores.
    pub scores: HashMap<Modality, f32>,
}

// ---------------------------------------------------------------------------
// Phase 3.1 — Soft gate function G(t, x)
// ---------------------------------------------------------------------------

/// Gate parameters for the soft NCS encoding switch.
///
/// G(t, x) = sigmoid(w_detect * resonance_confidence + w_time * (t - t_onset) + bias)
///
/// The gate smoothly transitions from general-purpose encoding (G=0) to
/// domain-specific encoding (G=1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateParams {
    /// Weight for the detection/resonance confidence input.
    pub w_detect: f32,
    /// Weight for the temporal component (time since onset).
    pub w_time: f32,
    /// Bias term.
    pub bias: f32,
}

impl Default for GateParams {
    fn default() -> Self {
        Self {
            w_detect: 4.0,  // Strong resonance drives fast switching
            w_time: 0.5,    // Mild temporal component
            bias: -2.0,     // Default slightly closed (need resonance to open)
        }
    }
}

impl GateParams {
    /// Compute the gate value G(t, x) in [0, 1].
    ///
    /// - `resonance_confidence`: how strongly the input resonates with a modality (0-1).
    /// - `time_since_onset`: normalized time since the modality was first detected.
    ///   For instantaneous use, pass 1.0.
    pub fn gate_value(&self, resonance_confidence: f32, time_since_onset: f32) -> f32 {
        let z = self.w_detect * resonance_confidence + self.w_time * time_since_onset + self.bias;
        sigmoid(z)
    }

    /// Apply the gate to blend general and domain-specific encodings.
    ///
    /// phi(x, t) = (1 - G) * E_general(x) + G * E_specific(x, modality)
    ///
    /// Both `general` and `specific` must have the same length.
    pub fn apply_gate(
        &self,
        general: &[f32],
        specific: &[f32],
        resonance_confidence: f32,
        time_since_onset: f32,
    ) -> Vec<f32> {
        let g = self.gate_value(resonance_confidence, time_since_onset);
        general.iter().zip(specific.iter())
            .map(|(&gen, &spec)| (1.0 - g) * gen + g * spec)
            .collect()
    }

    /// Nudge gate parameters toward better performance.
    ///
    /// Called during dream consolidation: if switch accuracy was low, adjust
    /// parameters to be more conservative (higher bias = harder to open).
    pub fn adjust(&mut self, switch_accuracy: f32) {
        let delta = (switch_accuracy - 0.7) * 0.1; // Target 70% accuracy
        self.bias += delta;
        self.bias = self.bias.clamp(-4.0, 0.0);
    }
}

/// Metadata about a gated encoding operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateEvent {
    /// Gate value at the time of encoding (0 = general, 1 = domain-specific).
    pub gate_value: f32,
    /// Detected modality (the domain-specific path).
    pub modality: Modality,
    /// Resonance confidence that triggered the gate.
    pub resonance_confidence: f32,
}

// ---------------------------------------------------------------------------
// Phase 3.3 — Consciousness metrics / switch observables
// ---------------------------------------------------------------------------

/// NCS consciousness metrics — switch observables that feed into the
/// IIT/Phi consciousness stack.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NcsMetrics {
    /// Total switch points detected in recent history.
    pub switch_count: usize,
    /// Mean divergence angle between modality axes (degrees).
    pub mean_axis_divergence: f32,
    /// Mean Fisher discriminant ratio across modality pairs.
    pub mean_fisher_ratio: f32,
    /// Mean gate confidence at switch points.
    pub mean_gate_confidence: f32,
    /// Modality diversity index (Shannon entropy of modality distribution).
    pub modality_diversity: f32,
    /// Cross-modal integration index: mean similarity between most-resonant
    /// memories across different modalities.
    pub cross_modal_integration: f32,
    /// Number of concrete modalities present with at least one wavefront.
    pub modality_count: usize,
}

impl Medium {
    /// Compute all NCS consciousness metrics from current medium state.
    pub fn ncs_metrics(&self) -> NcsMetrics {
        let report = self.axis_divergence_matrix();
        let switch_report = self.detect_switch_points_default();

        let mean_fisher_ratio = if report.fisher_ratios.is_empty() {
            0.0
        } else {
            report.fisher_ratios.iter()
                .map(|f| if f.ratio.is_finite() { f.ratio } else { 10.0 })
                .sum::<f32>() / report.fisher_ratios.len() as f32
        };

        let mean_gate_confidence = if switch_report.switch_points.is_empty() {
            0.0
        } else {
            switch_report.switch_points.iter()
                .map(|sp| sp.similarity_to_new)
                .sum::<f32>() / switch_report.switch_points.len() as f32
        };

        // Modality diversity: Shannon entropy of modality distribution
        let mut modality_counts: HashMap<Modality, usize> = HashMap::new();
        let mut total = 0usize;
        for meta in &self.store.metadata {
            if meta.modality != Modality::Unknown && meta.modality != Modality::Mixed {
                *modality_counts.entry(meta.modality).or_insert(0) += 1;
                total += 1;
            }
        }
        let modality_diversity = if total > 0 {
            let mut entropy = 0.0f32;
            for &count in modality_counts.values() {
                let p = count as f32 / total as f32;
                if p > 0.0 {
                    entropy -= p * p.ln();
                }
            }
            entropy
        } else {
            0.0
        };

        // Cross-modal integration: average pairwise centroid similarity
        let cross_modal_integration = if report.divergences.is_empty() {
            0.0
        } else {
            report.divergences.iter()
                .map(|d| d.cosine_similarity.abs())
                .sum::<f32>() / report.divergences.len() as f32
        };

        NcsMetrics {
            switch_count: switch_report.switch_points.len(),
            mean_axis_divergence: report.mean_divergence_degrees,
            mean_fisher_ratio,
            mean_gate_confidence,
            modality_diversity,
            cross_modal_integration,
            modality_count: report.axes.len(),
        }
    }

    /// Compute the modality distribution as a fraction map (sums to 1.0).
    /// Used for swarm specialization broadcasting.
    pub fn modality_distribution(&self) -> HashMap<Modality, f32> {
        let mut counts: HashMap<Modality, usize> = HashMap::new();
        let mut total = 0usize;
        for meta in &self.store.metadata {
            if meta.modality != Modality::Unknown {
                *counts.entry(meta.modality).or_insert(0) += 1;
                total += 1;
            }
        }
        if total == 0 {
            return HashMap::new();
        }
        counts.into_iter()
            .map(|(m, c)| (m, c as f32 / total as f32))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Phase 4.1 — Network modality encoding (VEIL integration)
// ---------------------------------------------------------------------------

/// A network signal encoded as HRM-ready data.
///
/// Maps VEIL-style network classification outputs into wavefront parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSignal {
    /// Source reputation score (0.0 = unknown, 1.0 = trusted).
    pub source_reputation: f32,
    /// Protocol behavior fingerprint (normalized vector, 5-dim).
    pub protocol_behavior: [f32; 5],
    /// Temporal pattern score (burstiness, periodicity).
    pub temporal_pattern: f32,
    /// Destination risk (0.0 = safe, 1.0 = known-malicious).
    pub destination_risk: f32,
    /// Tracker category (None = not a tracker).
    pub tracker_category: Option<String>,
    /// Privacy exposure score (0-100, maps to wavefront amplitude).
    pub privacy_exposure: f32,
    /// Human-readable description for memory content.
    pub description: String,
}

impl NetworkSignal {
    /// Convert this network signal to a sparse wavefront vector.
    ///
    /// Network signals occupy dims 600..700 of the WAVEFRONT_DIM space
    /// (avoiding overlap with Audio 0..100, Visual 200..300, Semantic 400..500).
    pub fn to_wavefront_vector(&self) -> Vec<f32> {
        let mut v = vec![0.0f32; WAVEFRONT_DIM];
        let base = 600;

        // Source reputation: dims 600..620
        for i in 0..20 {
            v[base + i] = self.source_reputation * (1.0 + i as f32 * 0.01);
        }

        // Protocol behavior: dims 620..645 (5 features spread across 5 dims each)
        for (fi, &val) in self.protocol_behavior.iter().enumerate() {
            for di in 0..5 {
                v[base + 20 + fi * 5 + di] = val;
            }
        }

        // Temporal pattern: dims 645..660
        for i in 0..15 {
            v[base + 45 + i] = self.temporal_pattern;
        }

        // Destination risk: dims 660..680
        for i in 0..20 {
            v[base + 60 + i] = self.destination_risk * (1.0 + i as f32 * 0.005);
        }

        // Tracker signal: dims 680..700 (strong signal if tracker detected)
        if self.tracker_category.is_some() {
            for i in 0..20 {
                v[base + 80 + i] = 1.0;
            }
        }

        v
    }

    /// Importance/amplitude derived from privacy exposure.
    /// Tracker detections get high salience; normal traffic gets low salience.
    pub fn importance(&self) -> f32 {
        let base = self.privacy_exposure / 100.0;
        if self.tracker_category.is_some() {
            (base + 0.5).min(1.0)  // Trackers are always high-salience
        } else {
            base * 0.3  // Normal traffic is low-salience
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 4.2 — Swarm specialization metadata
// ---------------------------------------------------------------------------

/// Modality specialization metadata for QueenSync broadcasting.
///
/// Each agent publishes this alongside its phase state so the swarm
/// can detect complementary specializations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModalitySpecialization {
    /// Fraction of memories in each modality (sums to ~1.0).
    pub distribution: HashMap<Modality, f32>,
    /// Strongest modality (highest fraction).
    pub primary_modality: Option<Modality>,
    /// Mean Fisher discriminant ratio (quality of modality separation).
    pub mean_fisher_ratio: f32,
    /// Total memory count.
    pub memory_count: usize,
}

impl ModalitySpecialization {
    /// Compute cosine overlap between two agents' modality distributions.
    /// High overlap means redundant specialization; low overlap means complementary.
    pub fn overlap(&self, other: &ModalitySpecialization) -> f32 {
        let all_modalities: Vec<Modality> = self.distribution.keys()
            .chain(other.distribution.keys())
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if all_modalities.is_empty() {
            return 0.0;
        }

        let mut dot = 0.0f32;
        let mut norm_a = 0.0f32;
        let mut norm_b = 0.0f32;
        for m in &all_modalities {
            let a = self.distribution.get(m).copied().unwrap_or(0.0);
            let b = other.distribution.get(m).copied().unwrap_or(0.0);
            dot += a * b;
            norm_a += a * a;
            norm_b += b * b;
        }
        let denom = norm_a.sqrt() * norm_b.sqrt();
        if denom < 1e-12 { 0.0 } else { dot / denom }
    }
}

impl Medium {
    /// Build the modality specialization metadata from current medium state.
    pub fn modality_specialization(&self) -> ModalitySpecialization {
        let distribution = self.modality_distribution();
        let primary_modality = distribution.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&m, _)| m);

        let report = self.axis_divergence_matrix();
        let mean_fisher_ratio = if report.fisher_ratios.is_empty() {
            0.0
        } else {
            report.fisher_ratios.iter()
                .map(|f| if f.ratio.is_finite() { f.ratio } else { 10.0 })
                .sum::<f32>() / report.fisher_ratios.len() as f32
        };

        ModalitySpecialization {
            distribution,
            primary_modality,
            mean_fisher_ratio,
            memory_count: self.wavefront_count(),
        }
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

/// Standard sigmoid function.
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
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
            m.store.metadata[idx].modality = Modality::Audio;
        }

        // Visual cluster: vectors concentrated in dims 200..300
        for i in 0..3 {
            let mut v = vec![0.0f32; WAVEFRONT_DIM];
            for j in 200..300 {
                v[j] = 1.0 + (i as f32) * 0.01;
            }
            let id = m.add_wavefront(&v, format!("visual mem {i}"), 1.0).unwrap();
            let idx = m.get_wavefront_index(&id).unwrap();
            m.store.metadata[idx].modality = Modality::Visual;
        }

        // Semantic cluster: vectors concentrated in dims 400..500
        for i in 0..2 {
            let mut v = vec![0.0f32; WAVEFRONT_DIM];
            for j in 400..500 {
                v[j] = 1.0 + (i as f32) * 0.01;
            }
            let id = m.add_wavefront(&v, format!("semantic mem {i}"), 1.0).unwrap();
            let idx = m.get_wavefront_index(&id).unwrap();
            m.store.metadata[idx].modality = Modality::Semantic;
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
            m.store.metadata[idx].modality = modality;
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
            m.store.metadata[idx].modality = Modality::Audio;
        }

        let report = m.detect_switch_points(0.3);
        assert_eq!(report.switch_points.len(), 0);
    }

    #[test]
    fn fisher_discriminant_positive_for_separated_clusters() {
        let m = make_test_medium();
        let report = m.axis_divergence_matrix();
        assert!(!report.fisher_ratios.is_empty());
        // Orthogonal clusters should have high Fisher ratio
        for fr in &report.fisher_ratios {
            assert!(fr.ratio > 0.0, "Fisher ratio should be positive, got {}", fr.ratio);
        }
    }

    #[test]
    fn resonance_detect_modality_finds_audio() {
        let m = make_test_medium();
        let mut query = vec![0.0f32; WAVEFRONT_DIM];
        for j in 0..100 { query[j] = 1.0; }
        let detection = m.resonance_detect_modality_default(&query);
        assert_eq!(detection.modality, Modality::Audio);
        assert!(detection.confidence > 0.0);
    }

    #[test]
    fn gate_function_sigmoid_behavior() {
        let gate = GateParams::default();
        // Low resonance → gate nearly closed
        let g_low = gate.gate_value(0.0, 0.0);
        assert!(g_low < 0.2, "Low resonance should keep gate mostly closed, got {g_low}");
        // High resonance → gate nearly open
        let g_high = gate.gate_value(1.0, 1.0);
        assert!(g_high > 0.8, "High resonance should open gate, got {g_high}");
    }

    #[test]
    fn ncs_metrics_computable() {
        let m = make_test_medium();
        let metrics = m.ncs_metrics();
        assert!(metrics.modality_count >= 2);
        assert!(metrics.modality_diversity > 0.0);
        assert!(metrics.mean_axis_divergence > 0.0);
    }

    #[test]
    fn network_signal_to_wavefront() {
        let signal = NetworkSignal {
            source_reputation: 0.8,
            protocol_behavior: [0.5, 0.3, 0.2, 0.1, 0.4],
            temporal_pattern: 0.6,
            destination_risk: 0.2,
            tracker_category: Some("ad_tracker".to_string()),
            privacy_exposure: 75.0,
            description: "tracker detected".to_string(),
        };
        let v = signal.to_wavefront_vector();
        assert_eq!(v.len(), WAVEFRONT_DIM);
        // Tracker dims (680..700) should be nonzero
        assert!(v[680] > 0.0);
        // Importance should be high for trackers
        assert!(signal.importance() > 0.5);
    }

    #[test]
    fn modality_specialization_overlap() {
        let m = make_test_medium();
        let spec = m.modality_specialization();
        assert!(spec.memory_count > 0);
        assert!(spec.primary_modality.is_some());

        // Overlap with self should be 1.0
        let overlap = spec.overlap(&spec);
        assert!((overlap - 1.0).abs() < 0.01, "Self-overlap should be ~1.0, got {overlap}");
    }
}
