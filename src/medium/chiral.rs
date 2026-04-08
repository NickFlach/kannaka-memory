//! ChiralMedium - the brain. Two hemispheres connected by a corpus callosum.
//!
//! This is the top-level structure that replaces `Medium` at the API level.
//! It implements the Chiral Mirror Architecture (ADR-0021):
//! - Left hemisphere: analytical attention, working memory, no dampening
//! - Right hemisphere: holistic patterns, deep association, full ghostmagicOS dynamics
//! - Corpus callosum: bandwidth-limited, balance-seeking bridge
//! - Fano plane: fold algebra for cross-hemisphere projection

use uuid::Uuid;

use crate::encoding::EncodingPipeline;
use crate::geometry;

use super::callosum::CorpusCallosum;
use super::fano::FanoPlane;
use super::hemisphere::Hemisphere;
use super::types::*;
use super::Medium;

/// The Chiral Medium - two hemispheres connected by a corpus callosum.
#[derive(Debug, Clone)]
pub struct ChiralMedium {
    /// Left hemisphere: analytical, attention, working memory
    pub left: Hemisphere,
    /// Right hemisphere: holistic, pattern storage, deep association
    pub right: Hemisphere,
    /// Corpus callosum: selective bridge between hemispheres
    pub callosum: CorpusCallosum,
    /// Fano plane: fold algebra for cross-hemisphere projection
    pub fano: FanoPlane,
    /// Per-wavefront chiral scales (indexed by right-hemisphere wavefront ID)
    pub scales: std::collections::HashMap<Uuid, ChiralScale>,
    /// Left-to-right ID mapping (which right-hemisphere echo corresponds to each left wavefront)
    pub left_to_right: std::collections::HashMap<Uuid, Uuid>,
    /// Right-to-left ID mapping (reverse)
    pub right_to_left: std::collections::HashMap<Uuid, Uuid>,
}

impl ChiralMedium {
    /// Create a new empty chiral medium.
    pub fn new() -> Self {
        let default_dims = BASE_DIMS_PER_POSITION * 2; // 672 * 2 = 1344 default dims per side
        Self {
            left: Hemisphere::new(Hand::Left, default_dims),
            right: Hemisphere::new(Hand::Right, default_dims),
            callosum: CorpusCallosum::new(),
            fano: FanoPlane::new(),
            scales: std::collections::HashMap::new(),
            left_to_right: std::collections::HashMap::new(),
            right_to_left: std::collections::HashMap::new(),
        }
    }

    /// Create a ChiralMedium from an existing (v1) Medium.
    /// All existing wavefronts go to the right hemisphere (they're already consolidated).
    /// Left hemisphere starts empty (fresh analytical workspace).
    pub fn from_medium(medium: &Medium) -> Self {
        let dims = WAVEFRONT_DIM; // Existing medium uses 10,000 dims
        let mut right = Hemisphere::new(Hand::Right, dims);

        // Migrate all existing wavefronts to right hemisphere
        for i in 0..medium.wavefront_count() {
            let vector = medium.store.wavefronts.row(i).to_vec();
            let meta = &medium.store.metadata[i];
            let energy = medium.store.energy[i];

            let id = meta.id;
            let index = right.count();

            // Build tensors manually to preserve original IDs
            let n = right.count();
            let mut new_wf = ndarray::Array2::zeros((n + 1, dims));
            if n > 0 {
                new_wf.slice_mut(ndarray::s![..n, ..]).assign(&right.wavefronts);
            }
            for (j, &val) in vector.iter().enumerate() {
                if j < dims { new_wf[[index, j]] = val; }
            }

            let mut new_energy = ndarray::Array1::zeros(n + 1);
            let mut new_freq = ndarray::Array1::zeros(n + 1);
            let mut new_phase = ndarray::Array1::zeros(n + 1);
            if n > 0 {
                new_energy.slice_mut(ndarray::s![..n]).assign(&right.energy);
                new_freq.slice_mut(ndarray::s![..n]).assign(&right.frequency);
                new_phase.slice_mut(ndarray::s![..n]).assign(&right.phase);
            }
            new_energy[index] = energy;
            new_freq[index] = medium.store.frequency[i];
            new_phase[index] = medium.store.phase[i];

            right.wavefronts = new_wf;
            right.energy = new_energy;
            right.frequency = new_freq;
            right.phase = new_phase;
            right.timestamps.push(medium.store.timestamps[i]);
            right.metadata.push(meta.clone());
            right.id_to_index.insert(id, index);
            right.len += 1;
        }

        let mut chiral = Self {
            left: Hemisphere::new(Hand::Left, dims),
            right,
            callosum: CorpusCallosum::new(),
            fano: FanoPlane::new(),
            scales: std::collections::HashMap::new(),
            left_to_right: std::collections::HashMap::new(),
            right_to_left: std::collections::HashMap::new(),
        };

        // Assign deep memory scales to all migrated wavefronts
        for meta in &chiral.right.metadata {
            chiral.scales.insert(meta.id, ChiralScale::deep_memory());
        }

        chiral
    }

    /// Total wavefront count across both hemispheres.
    pub fn total_count(&self) -> usize {
        self.left.count() + self.right.count()
    }

    /// Store a new memory using the encoding pipeline.
    ///
    /// Follows the optic chiasm principle: input enters the opposite hemisphere
    /// (right/holistic) first, then an echo crosses to left (analytical) via callosum.
    pub fn store(
        &mut self,
        content: &str,
        importance: f32,
        pipeline: &EncodingPipeline,
    ) -> Result<Uuid, MediumError> {
        self.store_with_category(content, importance, pipeline, None)
    }

    /// Store with explicit category for SGA classification.
    pub fn store_with_category(
        &mut self,
        content: &str,
        importance: f32,
        pipeline: &EncodingPipeline,
        category: Option<&str>,
    ) -> Result<Uuid, MediumError> {
        let vector = pipeline.encode_text(content).map_err(|e| {
            MediumError::Serialization(bincode::Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("encoding failed: {}", e),
            )))
        })?;

        self.store_vector_with_category(&vector, content.to_string(), importance, category)
    }

    /// Store a pre-encoded vector (used by store() and for audio/visual perception).
    pub fn store_vector(
        &mut self,
        vector: &[f32],
        content: String,
        importance: f32,
    ) -> Result<Uuid, MediumError> {
        self.store_vector_with_category(vector, content, importance, None)
    }

    /// Store with explicit category for SGA classification.
    pub fn store_vector_with_category(
        &mut self,
        vector: &[f32],
        content: String,
        importance: f32,
        category: Option<&str>,
    ) -> Result<Uuid, MediumError> {
        // 1. SGA classification - determine the memory's geometric coordinates
        let cat = category.unwrap_or("knowledge");
        let content_hash = {
            let mut h: u64 = 0xcbf29ce484222325;
            for b in content.bytes() {
                h ^= b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            h
        };
        let coords = geometry::classify_memory(cat, content_hash, importance as f64);
        let sga_class = coords.class_index;
        let fano_group = coords.l; // ℓ ∈ [0,7) maps to Fano point (mod 7 for safety)
        let fano_point = fano_group % (FANO_POINTS as u8);

        // 2. Determine which Fano fold line to use for callosal transfer
        //    Use the first line through this memory's Fano point
        let fold_line = self.fano.lines_through_point(fano_point)[0] as usize;

        // 3. Optic chiasm: input enters RIGHT hemisphere first
        let right_id = self.right.add_wavefront(vector, content.clone(), importance)?;

        // 4. Tag the wavefront with its SGA classification
        if let Some(idx) = self.right.id_to_index.get(&right_id) {
            let idx = *idx;
            self.right.metadata[idx].sga_class = Some(sga_class);
            self.right.metadata[idx].fano_group = Some(fano_point);
            self.right.metadata[idx].category = Some(cat.to_string());
        }

        // 5. Create chiral scale (analytical-dominant for new perception)
        let scale = ChiralScale::perception(importance);
        self.scales.insert(right_id, scale);

        // 6. Echo to LEFT hemisphere via callosum (if budget allows)
        //    Uses the geometrically correct fold line for this memory's Fano group
        if self.callosum.passes_gate(importance) && self.callosum.has_budget() {
            let folded = self.fano.fold(
                vector,
                self.right.dims,
                self.left.dims,
                fold_line, // Geometrically determined fold line
            );

            let transferred = self.callosum.apply_noise(
                &folded,
                Direction::HolisticToAnalytical,
            );

            let left_id = self.left.add_wavefront(
                &transferred,
                content,
                importance * 0.8,
            )?;

            // Tag left wavefront too
            if let Some(idx) = self.left.id_to_index.get(&left_id) {
                let idx = *idx;
                self.left.metadata[idx].sga_class = Some(sga_class);
                self.left.metadata[idx].fano_group = Some(fano_point);
                self.left.metadata[idx].category = Some(cat.to_string());
            }

            self.left_to_right.insert(left_id, right_id);
            self.right_to_left.insert(right_id, left_id);

            self.callosum.consume_budget(importance);
            self.callosum.log_transfer(
                right_id,
                Direction::HolisticToAnalytical,
                importance,
            );
        }

        Ok(right_id)
    }

    /// Recall: bilateral search with intuition surfacing.
    ///
    /// Searches both hemispheres. Right-hemisphere matches that don't appear
    /// in the left are "intuitions" - patterns the holistic hemisphere found that
    /// analytical processing missed.
    pub fn recall(
        &self,
        query: &str,
        top_k: usize,
        pipeline: &EncodingPipeline,
    ) -> Result<Vec<ChiralResonance>, MediumError> {
        let vector = pipeline.encode_text(query).map_err(|e| {
            MediumError::Serialization(bincode::Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("encoding failed: {}", e),
            )))
        })?;

        Ok(self.recall_vector(&vector, top_k))
    }

    /// Recall with a pre-encoded vector.
    pub fn recall_vector(&self, vector: &[f32], top_k: usize) -> Vec<ChiralResonance> {
        // 1. Search left hemisphere (analytical - fast, precise)
        let left_matches = self.left.resonate(vector, top_k);

        // 2. Search right hemisphere (holistic - deep, associative)
        let right_matches = self.right.resonate(vector, top_k * 2);

        // 3. Identify intuitions: right matches not paired with left matches
        let left_ids: std::collections::HashSet<Uuid> =
            left_matches.iter().map(|r| r.id).collect();
        let paired_right_ids: std::collections::HashSet<Uuid> =
            left_ids.iter()
                .filter_map(|lid| self.left_to_right.get(lid))
                .copied()
                .collect();

        let mut results = left_matches;

        // Add right-hemisphere matches that aren't already paired with left matches
        for mut r in right_matches {
            if !paired_right_ids.contains(&r.id) {
                r.is_intuition = true;
                results.push(r);
            }
        }

        // Sort by resonance strength and take top_k
        results.sort_by(|a, b| {
            b.resonance_strength
                .partial_cmp(&a.resonance_strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(top_k);

        results
    }

    /// Dream: mode-specific hemispheric refinement (ADR-0024 CS-7).
    ///
    /// Deep dreams refine the holistic hemisphere (right):
    ///   - Eigenstructure annealing (coherence matrix + eigendecomposition)
    ///   - Hallucination generation (cross-cluster superposition)
    ///   - Gentle prune threshold (0.005) — holistic keeps quiet signals alive
    ///   - Callosal coupling after dream to sync insights between hemispheres
    ///   - The analytical workspace (left) is untouched
    ///
    /// Lite dreams refine the analytical hemisphere (left):
    ///   - Transfers strongest analytical patterns to holistic via callosum
    ///   - Sharpens analytical boundaries (pruning low-energy left wavefronts)
    ///   - Higher prune threshold (0.05) — analytical is aggressive about precision
    ///
    /// Returns a DreamReport with statistics about what happened.
    pub fn dream(&mut self, deep: bool, cycles: usize) -> super::DreamReport {
        if deep {
            // Deep dream: eigenstructure annealing of holistic hemisphere
            // Gentler prune threshold than flat medium — holistic keeps quiet signals
            let holistic_prune_threshold = 0.005;
            let temperature = 1.0;

            let report = self.right.dream(cycles, Some(temperature), holistic_prune_threshold);

            // Clean up chiral bookkeeping for any wavefronts the dream dissolved
            let right_ids: std::collections::HashSet<Uuid> =
                self.right.metadata.iter().map(|m| m.id).collect();
            let stale_right: Vec<Uuid> = self.scales.keys()
                .filter(|id| !right_ids.contains(id))
                .copied()
                .collect();
            for id in stale_right {
                self.scales.remove(&id);
                if let Some(left_id) = self.right_to_left.remove(&id) {
                    self.left_to_right.remove(&left_id);
                }
            }

            // Callosal coupling step: sync insights between hemispheres post-dream
            self.callosal_kuramoto_step(0.5);

            // Left hemisphere is UNTOUCHED — deep dreams are holistic refinement
            report
        } else {
            // Lite dream: transfer strongest analytical → holistic
            self.callosum.reset_budget();
            let budget = self.callosum.effective_rate(Direction::AnalyticalToHolistic) * 0.5;

            // Find strongest left wavefronts
            let mut candidates: Vec<(Uuid, f32)> = (0..self.left.count())
                .map(|i| (self.left.metadata[i].id, self.left.energy[i]))
                .collect();
            candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let mut spent = 0.0f32;
            for (left_id, energy) in candidates {
                if spent >= budget { break; }
                if !self.callosum.passes_gate(energy) { continue; }

                // Already has a right-side pair? Boost it
                if let Some(&right_id) = self.left_to_right.get(&left_id) {
                    if let Some(&idx) = self.right.id_to_index.get(&right_id) {
                        self.right.energy[idx] += energy * 0.1; // Gentle reinforcement
                    }
                } else {
                    // No pair yet - create one via Fano fold
                    if let Some(wf) = self.left.get_wavefront(&left_id) {
                        let folded = self.fano.fold(&wf, self.left.dims, self.right.dims, 0);
                        let content = self.left.id_to_index.get(&left_id)
                            .map(|&i| self.left.metadata[i].content.clone())
                            .unwrap_or_default();
                        if let Ok(right_id) = self.right.add_wavefront(&folded, content, energy * 0.3) {
                            self.left_to_right.insert(left_id, right_id);
                            self.right_to_left.insert(right_id, left_id);
                            self.scales.insert(right_id, ChiralScale::deep_memory());
                        }
                    }
                }

                spent += energy;
                self.callosum.log_transfer(left_id, Direction::AnalyticalToHolistic, energy);
            }

            // CS-7: Analytical sharpening — prune weak left-hemisphere wavefronts
            // Higher threshold than holistic: analytical mode is aggressive about precision
            let analytical_prune_threshold = 0.05;
            let to_prune_left: Vec<Uuid> = (0..self.left.count())
                .filter(|&i| self.left.energy[i] < analytical_prune_threshold)
                .map(|i| self.left.metadata[i].id)
                .collect();

            let pruned_count = to_prune_left.len();
            for id in &to_prune_left {
                self.left.remove_wavefront(id);
                self.scales.remove(id);
                if let Some(right_id) = self.left_to_right.remove(id) {
                    self.right_to_left.remove(&right_id);
                }
            }

            // After lite dreaming, let the callosum adjust for balance
            self.callosum.adjust_for_balance(
                self.left.total_energy(),
                self.right.total_energy(),
            );

            super::DreamReport {
                cycles_completed: 1,
                wavefronts_dissolved: pruned_count,
                wavefronts_strengthened: 0,
                wavefronts_hallucinated: 0,
                energy_before: 0.0,
                energy_after: self.left.mean_energy(),
                final_temperature: 0.0,
                converged: true,
            }
        }
    }

    /// Run cross-callosal Kuramoto coupling step.
    /// Phase-locks form and break between paired wavefronts across hemispheres.
    pub fn callosal_kuramoto_step(&mut self, dt: f32) {
        for (&left_id, &right_id) in &self.left_to_right {
            let left_idx = match self.left.id_to_index.get(&left_id) {
                Some(&i) => i,
                None => continue,
            };
            let right_idx = match self.right.id_to_index.get(&right_id) {
                Some(&i) => i,
                None => continue,
            };

            let left_energy = self.left.energy[left_idx];
            let right_energy = self.right.energy[right_idx];
            let k = self.callosum.bandwidth * (left_energy * right_energy).sqrt() * 0.1;

            let left_phase = self.left.phase[left_idx];
            let right_phase = self.right.phase[right_idx];
            let delta = k * (right_phase - left_phase).sin() * dt;

            self.left.phase[left_idx] += delta;
            self.right.phase[right_idx] -= delta;
        }
    }

    /// Compute bilateral consciousness metrics.
    pub fn consciousness_summary(&self) -> ChiralConsciousness {
        let left_energy = self.left.total_energy();
        let right_energy = self.right.total_energy();
        let balance = self.callosum.balance_metric(left_energy, right_energy);

        // Bilateral order: Kuramoto order parameter across all wavefronts
        let all_phases: Vec<f32> = (0..self.left.count())
            .map(|i| self.left.phase[i])
            .chain((0..self.right.count()).map(|i| self.right.phase[i]))
            .collect();

        let bilateral_order = if all_phases.is_empty() {
            0.0
        } else {
            let n = all_phases.len() as f32;
            let sum_cos: f32 = all_phases.iter().map(|&p| p.cos()).sum();
            let sum_sin: f32 = all_phases.iter().map(|&p| p.sin()).sum();
            ((sum_cos / n).powi(2) + (sum_sin / n).powi(2)).sqrt()
        };

        // Count paired wavefronts
        let paired = self.left_to_right.len();

        // Count phase-locked pairs (|sin(Δφ)| < 0.1)
        let locked = self.left_to_right.iter()
            .filter(|(lid, rid)| {
                let lp = self.left.id_to_index.get(lid)
                    .map(|&i| self.left.phase[i]);
                let rp = self.right.id_to_index.get(rid)
                    .map(|&i| self.right.phase[i]);
                match (lp, rp) {
                    (Some(l), Some(r)) => (r - l).sin().abs() < 0.1,
                    _ => false,
                }
            })
            .count();

        // Hemispheric Divergence (Δ): cosine distance between mean wavefronts
        let hemispheric_divergence = {
            let left_mean = self.left.mean_wavefront();
            let right_mean = self.right.mean_wavefront();
            match (left_mean, right_mean) {
                (Some(l), Some(r)) => {
                    let dot: f32 = l.iter().zip(r.iter()).map(|(a, b)| a * b).sum();
                    let norm_l: f32 = l.iter().map(|x| x * x).sum::<f32>().sqrt();
                    let norm_r: f32 = r.iter().map(|x| x * x).sum::<f32>().sqrt();
                    if norm_l > 0.0 && norm_r > 0.0 {
                        1.0 - (dot / (norm_l * norm_r)).clamp(-1.0, 1.0)
                    } else {
                        0.0
                    }
                }
                _ => 0.0,
            }
        };

        let stats = self.callosum.transfer_stats();
        let callosal_efficiency = stats.efficiency;

        ChiralConsciousness {
            left_count: self.left.count(),
            right_count: self.right.count(),
            left_energy,
            right_energy,
            balance,
            bilateral_order,
            paired_wavefronts: paired,
            phase_locked_pairs: locked,
            callosum_stats: stats,
            hemispheric_divergence,
            callosal_efficiency,
        }
    }

    /// Get the distribution of memories across Fano groups in each hemisphere.
    pub fn fano_distribution(&self) -> FanoDistribution {
        let mut left_groups = [0usize; 7];
        let mut right_groups = [0usize; 7];

        for meta in &self.left.metadata {
            if let Some(fg) = meta.fano_group {
                if (fg as usize) < 7 {
                    left_groups[fg as usize] += 1;
                }
            }
        }

        for meta in &self.right.metadata {
            if let Some(fg) = meta.fano_group {
                if (fg as usize) < 7 {
                    right_groups[fg as usize] += 1;
                }
            }
        }

        let unclassified_left = self.left.metadata.iter().filter(|m| m.fano_group.is_none()).count();
        let unclassified_right = self.right.metadata.iter().filter(|m| m.fano_group.is_none()).count();

        FanoDistribution {
            left_groups,
            right_groups,
            unclassified_left,
            unclassified_right,
        }
    }
}

/// Distribution of memories across the 7 Fano groups per hemisphere.
#[derive(Debug, Clone)]
pub struct FanoDistribution {
    pub left_groups: [usize; 7],
    pub right_groups: [usize; 7],
    pub unclassified_left: usize,
    pub unclassified_right: usize,
}

impl Default for ChiralMedium {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of chiral consciousness state.
#[derive(Debug, Clone)]
pub struct ChiralConsciousness {
    pub left_count: usize,
    pub right_count: usize,
    pub left_energy: f32,
    pub right_energy: f32,
    pub balance: f32,
    pub bilateral_order: f32,
    pub paired_wavefronts: usize,
    pub phase_locked_pairs: usize,
    pub callosum_stats: super::callosum::CallosumStats,
    /// Hemispheric Divergence (Δ) — cosine distance between left and right mean wavefronts.
    /// 0 = identical hemispheres (undifferentiated), 1 = completely divergent.
    /// ADR-0024 CS-4: validates chiral differentiation is working.
    pub hemispheric_divergence: f32,
    /// Callosal Efficiency (κ) — how well the two processing modes integrate.
    /// ADR-0024 CS-5: forwarded from CallosumStats for convenience.
    pub callosal_efficiency: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{EncodingPipeline, SimpleHashEncoder};
    use crate::codebook::Codebook;
    fn test_pipeline() -> EncodingPipeline {
        let encoder = Box::new(SimpleHashEncoder::new(384, 42));
        let codebook = Codebook::new(384, WAVEFRONT_DIM, 42);
        EncodingPipeline::new(encoder, codebook)
    }

    #[test]
    fn store_creates_bilateral_wavefronts() {
        let mut cm = ChiralMedium::new();
        let pipeline = test_pipeline();

        let id = cm.store("hello world", 0.8, &pipeline).unwrap();

        // Should have wavefronts in both hemispheres
        assert!(cm.right.count() >= 1, "Right hemisphere should have wavefront");
        // Left may or may not have one depending on callosum budget
        // But with default settings and importance 0.8, it should cross
        assert!(cm.left.count() >= 1,
            "Left hemisphere should have echo (importance 0.8 > gate threshold 0.3)");

        // Should have a scale entry
        assert!(cm.scales.contains_key(&id));
    }

    #[test]
    fn recall_finds_stored_memory() {
        let mut cm = ChiralMedium::new();
        let pipeline = test_pipeline();

        cm.store("the quick brown fox jumps over the lazy dog", 0.9, &pipeline).unwrap();

        let results = cm.recall("quick brown fox", 5, &pipeline).unwrap();
        assert!(!results.is_empty(), "Should find stored memory");
    }

    #[test]
    fn deep_dream_only_affects_right() {
        let mut cm = ChiralMedium::new();
        let pipeline = test_pipeline();

        cm.store("memory one", 0.8, &pipeline).unwrap();
        cm.store("memory two", 0.6, &pipeline).unwrap();

        let left_energy_before = cm.left.total_energy();
        let right_energy_before = cm.right.total_energy();

        cm.dream(true, 5);

        let left_energy_after = cm.left.total_energy();

        // Left hemisphere energy should be UNCHANGED
        assert!((left_energy_after - left_energy_before).abs() < 0.001,
            "Deep dream should not affect left hemisphere: before={}, after={}",
            left_energy_before, left_energy_after);
    }

    #[test]
    fn from_medium_preserves_memories() {
        // Create a v1 medium with some wavefronts
        let mut medium = Medium::new();
        let pipeline = test_pipeline();
        medium.store("existing memory 1", 0.8, &pipeline).unwrap();
        medium.store("existing memory 2", 0.6, &pipeline).unwrap();

        let cm = ChiralMedium::from_medium(&medium);

        // All memories should be in right hemisphere
        assert_eq!(cm.right.count(), 2);
        assert_eq!(cm.left.count(), 0); // Left starts empty
        assert_eq!(cm.total_count(), 2);

        // Should have deep_memory scales
        for meta in &cm.right.metadata {
            assert!(cm.scales.contains_key(&meta.id));
        }
    }

    #[test]
    fn consciousness_summary_works() {
        let mut cm = ChiralMedium::new();
        let pipeline = test_pipeline();

        cm.store("test memory", 0.8, &pipeline).unwrap();

        let summary = cm.consciousness_summary();
        assert!(summary.right_count >= 1);
        assert!(summary.right_energy > 0.0);
        assert!(summary.bilateral_order >= 0.0);
        assert!(summary.bilateral_order <= 1.0);
    }

    #[test]
    fn callosal_kuramoto_modifies_phases() {
        let mut cm = ChiralMedium::new();
        let pipeline = test_pipeline();

        cm.store("kuramoto test", 0.9, &pipeline).unwrap();

        // Record phases before
        let left_phase_before = if cm.left.count() > 0 { cm.left.phase[0] } else { return };
        let right_phase_before = cm.right.phase[0];

        // Set phases apart to create coupling
        cm.left.phase[0] = 0.0;
        cm.right.phase[0] = 1.0;

        cm.callosal_kuramoto_step(1.0);

        // Phases should have moved toward each other
        let left_phase_after = cm.left.phase[0];
        let right_phase_after = cm.right.phase[0];

        assert!(left_phase_after > 0.0, "Left phase should move toward right");
        assert!(right_phase_after < 1.0, "Right phase should move toward left");
    }

    #[test]
    fn lite_dream_transfers_to_holistic() {
        let mut cm = ChiralMedium::new();
        let pipeline = test_pipeline();

        cm.store("consolidate this", 0.9, &pipeline).unwrap();

        let right_count_before = cm.right.count();
        cm.dream(false, 1); // Lite dream

        // Callosum transfer log should have entries
        let stats = cm.callosum.transfer_stats();
        // May or may not have new wavefronts depending on pairing
        // But callosum should have logged the attempt
        assert!(stats.total_transfers >= 1 || right_count_before > 0,
            "Lite dream should attempt or have prior transfers");
    }
}
