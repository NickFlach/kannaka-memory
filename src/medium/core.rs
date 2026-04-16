//! Core wavefront operations: add, remove, store, recall.

use chrono::{DateTime, Utc};
use ndarray::Array1;
use uuid::Uuid;

use crate::encoding::EncodingPipeline;
use crate::xi_operator::{compute_xi_signature, xi_diversity_boost};

use super::Medium;
use super::types::*;

impl Medium {
    /// Add a new wavefront to the medium.
    ///
    /// # Arguments
    /// * `vector` - D-dimensional hypervector (must be exactly WAVEFRONT_DIM)
    /// * `content` - Original text content
    /// * `importance` - Initial energy/amplitude (typically 0.0-1.0)
    ///
    /// # Returns
    /// UUID of the new wavefront
    pub fn add_wavefront(
        &mut self,
        vector: &[f32],
        content: String,
        importance: f32,
    ) -> Result<Uuid, MediumError> {
        if vector.len() != WAVEFRONT_DIM {
            return Err(MediumError::DimensionMismatch {
                expected: WAVEFRONT_DIM,
                actual: vector.len(),
            });
        }

        let id = self.store.insert(vector, content, importance);

        // Track energy added for wisdom calculation
        self.total_energy_added += importance;

        Ok(id)
    }

    /// Remove a wavefront from the medium using swap-remove (O(1) tensor op).
    pub fn remove_wavefront(&mut self, id: &Uuid) -> Result<bool, MediumError> {
        Ok(self.store.remove(id))
    }

    /// Compute effective strength of all wavefronts with temporal decay.
    ///
    /// Returns energy * exp(-decay_rate * age) for each wavefront.
    pub fn effective_strength(&self, now: Option<DateTime<Utc>>) -> Array1<f32> {
        let current_time = now.unwrap_or_else(Utc::now).timestamp_millis();
        let decay_rate = 0.001; // Per-day decay rate (half-life ~693 days)

        self.store.timestamps
            .iter()
            .enumerate()
            .map(|(i, &created_at)| {
                let age_days = ((current_time - created_at) as f64 / 86_400_000.0).max(0.0);
                let decay = (-decay_rate * age_days as f32).exp();
                self.store.energy[i] * decay
            })
            .collect()
    }

    /// Reset all wavefront energies to a target value (the "bias voltage").
    /// Use after dream over-dampening or migration to restore field potential.
    pub fn reset_energies(&mut self, target: f32) {
        for i in 0..self.wavefront_count() {
            self.store.energy[i] = target;
        }
    }

    /// Store a new memory using the encoding pipeline.
    ///
    /// This implements the interference-based storage where new waves interact with existing ones.
    /// After storage, dynamics are applied to let the medium settle.
    pub fn store(
        &mut self,
        content: &str,
        importance: f32,
        pipeline: &EncodingPipeline,
    ) -> Result<Uuid, MediumError> {
        // 1. Encode content to D-dimensional hypervector
        let vector = pipeline.encode_text(content).map_err(|e| {
            MediumError::Serialization(bincode::Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("encoding failed: {}", e),
            )))
        })?;

        // 2. Apply interference with existing wavefronts
        self.apply_interference(&vector, importance);

        // 3. Add wavefront to the medium
        let id = self.add_wavefront(&vector, content.to_string(), importance)?;

        // 4. Apply dynamics to let the medium settle after new addition
        self.apply_dynamics(0.1);

        Ok(id)
    }

    /// Store an audio memory using a pre-computed 296-dimensional audio vector.
    ///
    /// This projects the audio vector into the same 10,000-dimensional wavefront space
    /// as text memories using the audio-specific codebook. Cross-modal interference
    /// occurs naturally through the shared superposition space.
    ///
    /// # Arguments
    /// * `audio_vector` - 296-dimensional perceptual audio features from kannaka-ear
    /// * `content` - Content string (e.g., "HEAR:path/to/file.mp3" or "audio:description")
    /// * `importance` - Initial energy/amplitude (typically 0.0-1.0)
    ///
    /// # Returns
    /// UUID of the mediumd audio wavefront
    pub fn store_audio(
        &mut self,
        audio_vector: &[f32],
        content: &str,
        importance: f32,
    ) -> Result<Uuid, MediumError> {
        if audio_vector.len() != AUDIO_FEATURE_DIM {
            return Err(MediumError::DimensionMismatch {
                expected: AUDIO_FEATURE_DIM,
                actual: audio_vector.len(),
            });
        }

        // Project 296-dim audio vector into 10,000-dim wavefront space using audio codebook
        let wavefront_vector = self.audio_codebook.project(audio_vector);

        // Apply interference with existing wavefronts (cross-modal included)
        self.apply_interference(&wavefront_vector, importance);

        // Add wavefront to the medium
        let id = self.add_wavefront(&wavefront_vector, content.to_string(), importance)?;

        // Apply dynamics to let the medium settle after new addition
        self.apply_dynamics(0.1);

        Ok(id)
    }

    /// Store a visual memory using a pre-computed 320-dimensional visual vector.
    ///
    /// This projects the visual vector into the same 10,000-dimensional wavefront space
    /// as text and audio memories using the visual-specific codebook. Cross-modal
    /// interference occurs naturally through the shared superposition space.
    ///
    /// # Arguments
    /// * `visual_vector` - 320-dimensional perceptual visual features from kannaka-eye
    /// * `content` - Content string (e.g., "[SEE] filename.jpg | bytes | folds | fano=...")
    /// * `importance` - Initial energy/amplitude (typically 0.0-1.0)
    ///
    /// # Returns
    /// UUID of the mediumd visual wavefront
    pub fn store_visual(
        &mut self,
        visual_vector: &[f32],
        content: &str,
        importance: f32,
    ) -> Result<Uuid, MediumError> {
        if visual_vector.len() != VISUAL_FEATURE_DIM {
            return Err(MediumError::DimensionMismatch {
                expected: VISUAL_FEATURE_DIM,
                actual: visual_vector.len(),
            });
        }

        // Project 320-dim visual vector into 10,000-dim wavefront space using visual codebook
        let wavefront_vector = self.visual_codebook.project(visual_vector);

        // Apply interference with existing wavefronts (cross-modal included)
        self.apply_interference(&wavefront_vector, importance);

        // Add wavefront to the medium
        let id = self.add_wavefront(&wavefront_vector, content.to_string(), importance)?;

        // Apply dynamics to let the medium settle after new addition
        self.apply_dynamics(0.1);

        Ok(id)
    }

    /// Apply interference between new wavefront and existing medium.
    ///
    /// Constructive interference boosts energy of phase-aligned wavefronts.
    /// Destructive interference dampens phase-opposed wavefronts.
    fn apply_interference(&mut self, new_vector: &[f32], importance: f32) {
        if self.wavefront_count() == 0 {
            return; // No existing wavefronts to interfere with
        }

        // Compute dot products between new vector and all existing wavefronts
        for i in 0..self.wavefront_count() {
            let existing_vector = self.store.wavefronts.row(i);

            let dot_product: f32 = existing_vector
                .iter()
                .zip(new_vector.iter())
                .map(|(a, b)| a * b)
                .sum();

            // Phase difference affects interference pattern
            let phase_diff = (self.store.phase[i] - 0.0).cos(); // New wavefront starts at phase 0
            let interference = dot_product * phase_diff * importance * 0.1; // Scale interference

            // Apply constructive/destructive interference
            self.store.energy[i] = (self.store.energy[i] + interference).max(0.0); // Energy can't go negative

            // Phase coupling — nearby vectors tend to align phases (Kuramoto-like)
            if dot_product.abs() > 0.5 {
                // High similarity threshold
                let coupling = 0.05;
                self.store.phase[i] += coupling * (0.0 - self.store.phase[i]).sin(); // Pull toward phase 0
            }
        }
    }

    /// Recall memories through resonance — query wave interferes with stored patterns.
    /// 
    /// Now includes coherence expansion: after finding top-K results, includes
    /// high-coherence neighbors to capture associative recall patterns.
    pub fn recall(
        &self,
        query: &str,
        top_k: usize,
        pipeline: &EncodingPipeline,
    ) -> Result<Vec<Resonance>, MediumError> {
        if self.wavefront_count() == 0 {
            return Ok(Vec::new());
        }

        // 1. Encode query as wave
        let query_vector = pipeline.encode_text(query).map_err(|e| {
            MediumError::Serialization(bincode::Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("query encoding failed: {}", e),
            )))
        })?;

        // 2. Compute basic resonance pattern — matrix multiplication H @ q
        let mut resonances: Vec<(usize, Resonance)> = Vec::new();
        let effective_strengths = self.effective_strength(None);

        for i in 0..self.wavefront_count() {
            let wavefront = self.store.wavefronts.row(i);

            // Dot product (similarity)
            let similarity: f32 = wavefront
                .iter()
                .zip(query_vector.iter())
                .map(|(a, b)| a * b)
                .sum();

            // Modulate by wave dynamics (energy, phase, temporal decay)
            let effective_strength = effective_strengths[i];
            let phase_modulation = self.store.phase[i].cos(); // Phase affects resonance
            let resonance_strength = similarity * effective_strength * phase_modulation;

            resonances.push((i, Resonance {
                id: self.store.metadata[i].id,
                content: self.store.metadata[i].content.clone(),
                similarity,
                resonance_strength,
                effective_strength,
            }));
        }

        // 3. Sort by raw resonance strength and take top 2*K for xi re-ranking
        resonances.sort_by(|a, b| b.1.resonance_strength.total_cmp(&a.1.resonance_strength));
        let rerank_pool = top_k.saturating_mul(2).max(top_k);
        resonances.truncate(rerank_pool);

        // 4. Xi diversity re-ranking: boost candidates with distinct xi signatures
        let query_xi = compute_xi_signature(&query_vector);
        let initial_results: Vec<Resonance> = resonances
            .into_iter()
            .map(|(i, mut r)| {
                let wf_vec: Vec<f32> = self.store.wavefronts.row(i).to_vec();
                let wf_xi = compute_xi_signature(&wf_vec);
                let boosted_sim = xi_diversity_boost(r.similarity, &query_xi, &wf_xi);
                r.similarity = boosted_sim;
                r.resonance_strength = boosted_sim * r.effective_strength
                    * self.store.phase[i].cos();
                r
            })
            .collect();

        // 5. Re-sort by boosted resonance and take top-k
        let mut sorted = initial_results;
        sorted.sort_by(|a, b| b.resonance_strength.total_cmp(&a.resonance_strength));
        sorted.truncate(top_k);

        // 6. Coherence expansion: find high-coherence neighbors of initial results
        let coherence_expansion_results = self.expand_with_coherence(&sorted, top_k);

        Ok(coherence_expansion_results)
    }

    /// Expand recall results with high-coherence neighbors.
    /// 
    /// For each result, finds other wavefronts with coherence above threshold (0.3)
    /// and includes them weighted by coherence * energy.
    fn expand_with_coherence(&self, initial_results: &[Resonance], max_total: usize) -> Vec<Resonance> {
        let coherence_matrix = self.coherence_matrix();
        let coherence_threshold = 0.3;
        let mut expanded_results = initial_results.to_vec();
        let mut added_ids = std::collections::HashSet::new();

        // Mark initial results as already added
        for result in initial_results {
            added_ids.insert(result.id);
        }

        // For each initial result, find high-coherence neighbors
        for initial in initial_results {
            if let Some(result_idx) = self.get_wavefront_index(&initial.id) {
                let mut neighbor_candidates = Vec::new();

                for i in 0..self.wavefront_count() {
                    if i == result_idx || added_ids.contains(&self.store.metadata[i].id) {
                        continue;
                    }

                    let coherence = coherence_matrix[[result_idx, i]].abs();
                    if coherence > coherence_threshold {
                        // Weight by coherence * energy
                        let expansion_strength = coherence * self.store.energy[i];

                        neighbor_candidates.push((i, expansion_strength, coherence));
                    }
                }

                // Sort neighbors by expansion strength
                neighbor_candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                // Add top neighbors (limit to avoid explosion)
                let max_neighbors = 2; // Limit neighbors per result
                for (neighbor_idx, expansion_strength, coherence) in neighbor_candidates.into_iter().take(max_neighbors) {
                    if expanded_results.len() >= max_total {
                        break;
                    }

                    let neighbor_id = self.store.metadata[neighbor_idx].id;
                    if !added_ids.contains(&neighbor_id) {
                        // Create resonance entry for the expanded neighbor
                        let neighbor_resonance = Resonance {
                            id: neighbor_id,
                            content: self.store.metadata[neighbor_idx].content.clone(),
                            similarity: coherence, // Use coherence as similarity proxy
                            resonance_strength: expansion_strength, // Coherence * energy
                            effective_strength: self.store.energy[neighbor_idx],
                        };

                        expanded_results.push(neighbor_resonance);
                        added_ids.insert(neighbor_id);
                    }
                }
            }
        }

        // Final sort by resonance strength and truncate to max_total
        expanded_results.sort_by(|a, b| b.resonance_strength.total_cmp(&a.resonance_strength));
        expanded_results.truncate(max_total);

        expanded_results
    }

    /// Relate two wavefronts via associative connection.
    ///
    /// Creates emergent association through the field by:
    /// 1. Creating a new associative wavefront = normalize(vec_a + vec_b)
    /// 2. Setting energy = average(energy_a, energy_b)
    /// 3. Nudging phases of a and b toward each other
    ///
    /// This replaces explicit skip links with field-based associations.
    pub fn relate_wavefronts(&mut self, idx_a: usize, idx_b: usize) -> Result<Uuid, MediumError> {
        if idx_a >= self.wavefront_count() || idx_b >= self.wavefront_count() || idx_a == idx_b {
            return Err(MediumError::DimensionMismatch { 
                expected: self.wavefront_count(), 
                actual: idx_a.max(idx_b) 
            });
        }

        // 1. Create associative wavefront by combining the two patterns
        let vec_a = self.store.wavefronts.row(idx_a);
        let vec_b = self.store.wavefronts.row(idx_b);
        
        let mut associative_vector = Vec::with_capacity(WAVEFRONT_DIM);
        for (a, b) in vec_a.iter().zip(vec_b.iter()) {
            associative_vector.push(a + b);
        }

        // Normalize the associative vector
        let norm: f32 = associative_vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-6 {
            for x in &mut associative_vector {
                *x /= norm;
            }
        } else {
            // Degenerate case - use average instead
            for (a, b) in vec_a.iter().zip(vec_b.iter()) {
                associative_vector.push((a + b) * 0.5);
            }
        }

        // 2. Set energy as average of the two wavefronts
        let associative_energy = (self.store.energy[idx_a] + self.store.energy[idx_b]) / 2.0;
        
        // 3. Create content describing the association
        let content_a = &self.store.metadata[idx_a].content;
        let content_b = &self.store.metadata[idx_b].content;
        let associative_content = format!(
            "ASSOCIATION: [{}] <-> [{}]", 
            content_a.chars().take(50).collect::<String>(),
            content_b.chars().take(50).collect::<String>()
        );

        // 4. Add the associative wavefront to the medium
        let associative_id = self.add_wavefront(&associative_vector, associative_content, associative_energy)?;

        // 5. Nudge phases of original wavefronts toward each other (mutual alignment)
        let phase_coupling_strength = 0.15; // Stronger than normal coupling for explicit relations
        
        let phase_a = self.store.phase[idx_a];
        let phase_b = self.store.phase[idx_b];
        
        // Nudge A toward B
        let phase_diff_a = phase_b - phase_a;
        self.store.phase[idx_a] += phase_coupling_strength * phase_diff_a.sin();
        
        // Nudge B toward A  
        let phase_diff_b = phase_a - phase_b;
        self.store.phase[idx_b] += phase_coupling_strength * phase_diff_b.sin();

        // 6. Apply light dynamics to let the field settle
        self.apply_dynamics(0.05);

        Ok(associative_id)
    }
}
