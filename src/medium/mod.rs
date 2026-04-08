//! Holographic Resonance Medium (HRM) — the core tensor-based storage where the medium IS the computation.
//!
//! This replaces SQL persistence with a true holographic memory system where:
//! - Memories exist as waves in superposition
//! - Recall is resonance (constructive/destructive interference)
//! - Skip links are emergent from phase alignment
//! - Dreaming is annealing (energy minimization)
//! - The storage topology IS the computation

use uuid::Uuid;

use crate::codebook::Codebook;
pub use crate::consciousness::{
    ConsciousnessLevel, ConsciousnessMetrics, ConsciousnessState, EmergenceLevel, EmergenceReport,
    SelfReflection,
};

pub mod types;
pub mod wavefront_store;
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
pub use wavefront_store::WavefrontStore;

// ---------------------------------------------------------------------------
// Holographic Resonance Medium
// ---------------------------------------------------------------------------

/// The core HRM structure — a high-dimensional phase space where memories exist as waves in superposition.
#[derive(Debug, Clone)]
pub struct Medium {
    /// Shared wavefront tensor storage (wavefronts, energy, frequency, phase, metadata, etc.)
    pub store: WavefrontStore,
    /// Audio codebook for projecting 296-dim audio vectors into 10,000-dim space
    pub(crate) audio_codebook: Codebook,
    /// Visual codebook for projecting 320-dim visual vectors into 10,000-dim space
    pub(crate) visual_codebook: Codebook,
    /// Total energy that has been added to the medium over its lifetime (for wisdom calculation)
    pub(crate) total_energy_added: f32,
    /// Total energy that has been dampened during dynamics (for wisdom calculation)
    pub(crate) total_energy_dampened: f32,
}

impl Medium {
    /// Create a new empty medium.
    pub fn new() -> Self {
        Self {
            store: WavefrontStore::new(WAVEFRONT_DIM),
            audio_codebook: Codebook::new(AUDIO_FEATURE_DIM, WAVEFRONT_DIM, AUDIO_CODEBOOK_SEED),
            visual_codebook: Codebook::new(VISUAL_FEATURE_DIM, WAVEFRONT_DIM, VISUAL_CODEBOOK_SEED),
            total_energy_added: 0.0,
            total_energy_dampened: 0.0,
        }
    }

    // -- Convenience delegates to self.store --

    /// Number of active wavefronts in the medium.
    #[inline]
    pub fn wavefront_count(&self) -> usize {
        self.store.count()
    }

    /// Current allocated tensor capacity (rows).
    #[inline]
    pub fn capacity(&self) -> usize {
        self.store.capacity()
    }

    /// Shrink tensors to exactly fit active wavefronts.
    /// Called before persistence to avoid writing unused capacity rows.
    #[inline]
    pub fn compact(&mut self) {
        self.store.compact();
    }

    /// Get the index of a wavefront by its ID (for migration purposes).
    pub fn get_wavefront_index(&self, id: &Uuid) -> Option<usize> {
        self.store.id_to_index.get(id).copied()
    }

    /// Update a wavefront's ID (for HrmStore compatibility).
    pub fn update_wavefront_id(
        &mut self,
        old_id: &Uuid,
        new_id: Uuid,
    ) -> Result<(), MediumError> {
        if let Some(index) = self.store.id_to_index.remove(old_id) {
            self.store.id_to_index.insert(new_id, index);
            self.store.metadata[index].id = new_id;
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
