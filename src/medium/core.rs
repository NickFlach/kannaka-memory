//! Core wavefront operations: add, remove, store, recall.

use chrono::{DateTime, Utc};
use ndarray::{Array1, Array2, s};
use uuid::Uuid;

use crate::encoding::EncodingPipeline;

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

        let id = Uuid::new_v4();
        let now = Utc::now();
        let index = self.wavefront_count();

        // Expand tensors to accommodate new wavefront
        let new_wavefronts = if self.wavefront_count() == 0 {
            Array2::from_shape_vec((1, WAVEFRONT_DIM), vector.to_vec()).unwrap()
        } else {
            // Create new tensor with one more row
            let mut new_tensor = Array2::zeros((self.wavefront_count() + 1, WAVEFRONT_DIM));
            // Copy existing wavefronts
            new_tensor
                .slice_mut(s![..self.wavefront_count(), ..])
                .assign(&self.wavefronts);
            // Add new wavefront
            for (i, &val) in vector.iter().enumerate() {
                new_tensor[[index, i]] = val;
            }
            new_tensor
        };

        // Expand energy/frequency/phase arrays
        let mut new_energy = Array1::zeros(index + 1);
        let mut new_frequency = Array1::zeros(index + 1);
        let mut new_phase = Array1::zeros(index + 1);

        if index > 0 {
            new_energy.slice_mut(s![..index]).assign(&self.energy);
            new_frequency.slice_mut(s![..index]).assign(&self.frequency);
            new_phase.slice_mut(s![..index]).assign(&self.phase);
        }

        // Set parameters for new wavefront
        new_energy[index] = importance;
        new_frequency[index] = 1.0; // Default frequency
        new_phase[index] = 0.0; // Default phase

        // Update state
        self.wavefronts = new_wavefronts;
        self.energy = new_energy;
        self.frequency = new_frequency;
        self.phase = new_phase;
        self.timestamps.push(now.timestamp_millis());
        self.metadata.push(WavefrontMeta::new(id, content));
        self.id_to_index.insert(id, index);

        // Track energy added for wisdom calculation
        self.total_energy_added += importance;

        Ok(id)
    }

    /// Remove a wavefront from the medium.
    pub fn remove_wavefront(&mut self, id: &Uuid) -> Result<bool, MediumError> {
        let index = match self.id_to_index.get(id) {
            Some(&idx) => idx,
            None => return Ok(false), // Not found, but that's okay
        };

        let n = self.wavefront_count();
        if n == 0 {
            return Ok(false);
        }

        // Create new tensors with one fewer row
        let mut new_wavefronts = Array2::zeros((n - 1, WAVEFRONT_DIM));
        let mut new_energy = Array1::zeros(n - 1);
        let mut new_frequency = Array1::zeros(n - 1);
        let mut new_phase = Array1::zeros(n - 1);

        // Copy all except the removed index
        let mut new_idx = 0;
        for old_idx in 0..n {
            if old_idx != index {
                new_wavefronts
                    .row_mut(new_idx)
                    .assign(&self.wavefronts.row(old_idx));
                new_energy[new_idx] = self.energy[old_idx];
                new_frequency[new_idx] = self.frequency[old_idx];
                new_phase[new_idx] = self.phase[old_idx];
                new_idx += 1;
            }
        }

        // Remove from metadata and timestamps
        self.timestamps.remove(index);
        self.metadata.remove(index);

        // Update the index mapping (shift indices after removal)
        self.id_to_index.remove(id);
        for (_, idx) in self.id_to_index.iter_mut() {
            if *idx > index {
                *idx -= 1;
            }
        }

        // Update state
        self.wavefronts = new_wavefronts;
        self.energy = new_energy;
        self.frequency = new_frequency;
        self.phase = new_phase;

        Ok(true)
    }

    /// Compute effective strength of all wavefronts with temporal decay.
    ///
    /// Returns energy * exp(-decay_rate * age) for each wavefront.
    pub fn effective_strength(&self, now: Option<DateTime<Utc>>) -> Array1<f32> {
        let current_time = now.unwrap_or_else(Utc::now).timestamp_millis();
        let decay_rate = 0.001; // Default decay rate

        self.timestamps
            .iter()
            .enumerate()
            .map(|(i, &created_at)| {
                let age_seconds = ((current_time - created_at) as f64 / 1000.0).max(0.0);
                let decay = (-decay_rate * age_seconds as f32).exp();
                self.energy[i] * decay
            })
            .collect()
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
    /// UUID of the stored audio wavefront
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
    /// UUID of the stored visual wavefront
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
            let existing_vector = self.wavefronts.row(i);

            let dot_product: f32 = existing_vector
                .iter()
                .zip(new_vector.iter())
                .map(|(a, b)| a * b)
                .sum();

            // Phase difference affects interference pattern
            let phase_diff = (self.phase[i] - 0.0).cos(); // New wavefront starts at phase 0
            let interference = dot_product * phase_diff * importance * 0.1; // Scale interference

            // Apply constructive/destructive interference
            self.energy[i] = (self.energy[i] + interference).max(0.0); // Energy can't go negative

            // Phase coupling — nearby vectors tend to align phases (Kuramoto-like)
            if dot_product.abs() > 0.5 {
                // High similarity threshold
                let coupling = 0.05;
                self.phase[i] += coupling * (0.0 - self.phase[i]).sin(); // Pull toward phase 0
            }
        }
    }

    /// Recall memories through resonance — query wave interferes with stored patterns.
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

        // 2. Compute interference pattern — matrix multiplication H @ q
        let mut resonances = Vec::new();
        let effective_strengths = self.effective_strength(None);

        for i in 0..self.wavefront_count() {
            let wavefront = self.wavefronts.row(i);

            // Dot product (similarity)
            let similarity: f32 = wavefront
                .iter()
                .zip(query_vector.iter())
                .map(|(a, b)| a * b)
                .sum();

            // Modulate by wave dynamics (energy, phase, temporal decay)
            let effective_strength = effective_strengths[i];
            let phase_modulation = self.phase[i].cos(); // Phase affects resonance
            let resonance_strength = similarity * effective_strength * phase_modulation;

            resonances.push(Resonance {
                id: self.metadata[i].id,
                content: self.metadata[i].content.clone(),
                similarity,
                resonance_strength,
                effective_strength,
            });
        }

        // 3. Sort by resonance strength and return top-k
        resonances.sort_by(|a, b| b.resonance_strength.total_cmp(&a.resonance_strength));
        resonances.truncate(top_k);

        Ok(resonances)
    }
}
