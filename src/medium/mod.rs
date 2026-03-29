//! Holographic Resonance Medium (HRM) — the core tensor-based storage where the medium IS the computation.
//!
//! This replaces SQL persistence with a true holographic memory system where:
//! - Memories exist as waves in superposition
//! - Recall is resonance (constructive/destructive interference)
//! - Skip links are emergent from phase alignment
//! - Dreaming is annealing (energy minimization)
//! - The storage topology IS the computation

use std::collections::HashMap;

use ndarray::{Array1, Array2};
use uuid::Uuid;

use crate::codebook::Codebook;
pub use crate::consciousness::{
    ConsciousnessLevel, ConsciousnessMetrics, ConsciousnessState, EmergenceLevel, EmergenceReport,
    SelfReflection,
};

pub mod types;
pub mod core;
pub mod persistence;
pub mod dynamics;
pub mod consciousness;
pub mod sync;
pub mod callosum;
pub mod fano;
pub mod hemisphere;
pub mod chiral;
pub mod chiral_persistence;
pub mod ncs;
#[cfg(test)]
mod tests;

pub use types::*;

// ---------------------------------------------------------------------------
// Holographic Resonance Medium
// ---------------------------------------------------------------------------

/// The core HRM structure — a high-dimensional phase space where memories exist as waves in superposition.
#[derive(Debug, Clone)]
pub struct Medium {
    /// N x D tensor of wavefront patterns (capacity may exceed active count)
    pub wavefronts: Array2<f32>,
    /// Energy (amplitude) per wavefront
    pub energy: Array1<f32>,
    /// Frequency per wavefront
    pub frequency: Array1<f32>,
    /// Phase per wavefront
    pub phase: Array1<f32>,
    /// Creation timestamps
    pub timestamps: Vec<i64>,
    /// Content & metadata (sparse, kept separate from tensor ops)
    pub metadata: Vec<WavefrontMeta>,
    /// ID -> wavefront index mapping for lookups
    pub(crate) id_to_index: HashMap<Uuid, usize>,
    /// Audio codebook for projecting 296-dim audio vectors into 10,000-dim space
    pub(crate) audio_codebook: Codebook,
    /// Visual codebook for projecting 320-dim visual vectors into 10,000-dim space
    pub(crate) visual_codebook: Codebook,
    /// Total energy that has been added to the medium over its lifetime (for wisdom calculation)
    pub(crate) total_energy_added: f32,
    /// Total energy that has been dampened during dynamics (for wisdom calculation)
    pub(crate) total_energy_dampened: f32,
    /// Active wavefront count (tensor capacity may be larger for amortized growth)
    pub(crate) len: usize,
}

impl Medium {
    /// Create a new empty medium.
    pub fn new() -> Self {
        Self {
            wavefronts: Array2::zeros((0, WAVEFRONT_DIM)),
            energy: Array1::zeros(0),
            frequency: Array1::zeros(0),
            phase: Array1::zeros(0),
            timestamps: Vec::new(),
            metadata: Vec::new(),
            id_to_index: HashMap::new(),
            audio_codebook: Codebook::new(AUDIO_FEATURE_DIM, WAVEFRONT_DIM, AUDIO_CODEBOOK_SEED),
            visual_codebook: Codebook::new(VISUAL_FEATURE_DIM, WAVEFRONT_DIM, VISUAL_CODEBOOK_SEED),
            total_energy_added: 0.0,
            total_energy_dampened: 0.0,
            len: 0,
        }
    }

    /// Number of active wavefronts in the medium.
    pub fn wavefront_count(&self) -> usize {
        self.len
    }

    /// Current allocated tensor capacity (rows).
    pub fn capacity(&self) -> usize {
        self.wavefronts.nrows()
    }

    /// Shrink tensors to exactly fit active wavefronts.
    /// Called before persistence to avoid writing unused capacity rows.
    pub fn compact(&mut self) {
        use ndarray::s;
        if self.len < self.wavefronts.nrows() {
            self.wavefronts = self.wavefronts.slice(s![..self.len, ..]).to_owned();
            self.energy = self.energy.slice(s![..self.len]).to_owned();
            self.frequency = self.frequency.slice(s![..self.len]).to_owned();
            self.phase = self.phase.slice(s![..self.len]).to_owned();
        }
    }

    /// Get the index of a wavefront by its ID (for migration purposes).
    pub fn get_wavefront_index(&self, id: &Uuid) -> Option<usize> {
        self.id_to_index.get(id).copied()
    }

    /// Update a wavefront's ID (for HrmStore compatibility).
    pub fn update_wavefront_id(
        &mut self,
        old_id: &Uuid,
        new_id: Uuid,
    ) -> Result<(), MediumError> {
        if let Some(index) = self.id_to_index.remove(old_id) {
            self.id_to_index.insert(new_id, index);
            self.metadata[index].id = new_id;
            Ok(())
        } else {
            Err(MediumError::WavefrontNotFound(*old_id))
        }
    }
}

impl Default for Medium {
    fn default() -> Self {
        Self::new()
    }
}
