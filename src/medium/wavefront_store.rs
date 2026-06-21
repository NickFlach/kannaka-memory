//! WavefrontStore — shared tensor storage backing both Medium and Hemisphere.
//!
//! Extracted from duplicated logic in core.rs and hemisphere.rs (#69).
//! Provides amortized-growth insertion, O(1) swap-remove, and compact().

use std::collections::HashMap;

use ndarray::{Array1, Array2, s};
use uuid::Uuid;

use super::types::WavefrontMeta;

/// Shared wavefront tensor storage with amortized growth and swap-remove.
#[derive(Debug, Clone)]
pub struct WavefrontStore {
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
    pub id_to_index: HashMap<Uuid, usize>,
    /// Active wavefront count (tensor capacity may be larger for amortized growth)
    pub len: usize,
    /// Dimensionality of each wavefront vector
    pub dims: usize,
}

impl WavefrontStore {
    /// Create a new empty store with the given wavefront dimension.
    pub fn new(dims: usize) -> Self {
        Self {
            wavefronts: Array2::zeros((0, dims)),
            energy: Array1::zeros(0),
            frequency: Array1::zeros(0),
            phase: Array1::zeros(0),
            timestamps: Vec::new(),
            metadata: Vec::new(),
            id_to_index: HashMap::new(),
            len: 0,
            dims,
        }
    }

    /// Number of active wavefronts.
    #[inline]
    pub fn count(&self) -> usize {
        self.len
    }

    /// Current allocated tensor capacity (rows).
    #[inline]
    pub fn capacity(&self) -> usize {
        self.wavefronts.nrows()
    }

    /// Insert a wavefront into the store with amortized growth.
    ///
    /// The `vector` must already be adapted to `self.dims` length.
    /// Returns the UUID assigned to the new wavefront.
    pub fn insert(
        &mut self,
        vector: &[f32],
        content: String,
        importance: f32,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let index = self.len;

        // Amortized growth: only reallocate when capacity is exhausted
        let cap = self.wavefronts.nrows();
        if index >= cap {
            let new_cap = if cap == 0 { 8 } else { cap * 2 };
            let mut new_wf = Array2::zeros((new_cap, self.dims));
            if cap > 0 {
                new_wf.slice_mut(s![..cap, ..]).assign(&self.wavefronts);
            }
            self.wavefronts = new_wf;

            let mut new_energy = Array1::zeros(new_cap);
            let mut new_frequency = Array1::zeros(new_cap);
            let mut new_phase = Array1::zeros(new_cap);
            if cap > 0 {
                new_energy.slice_mut(s![..cap]).assign(&self.energy);
                new_frequency.slice_mut(s![..cap]).assign(&self.frequency);
                new_phase.slice_mut(s![..cap]).assign(&self.phase);
            }
            self.energy = new_energy;
            self.frequency = new_frequency;
            self.phase = new_phase;
        }

        // Write into pre-allocated slot
        let copy_len = vector.len().min(self.dims);
        for i in 0..copy_len {
            self.wavefronts[[index, i]] = vector[i];
        }
        self.energy[index] = importance;
        self.frequency[index] = 1.0;
        // Born phase: content-smooth (belief substrate) or legacy phase-0.
        self.phase[index] = if crate::medium::chiral::belief_phase_enabled() {
            crate::medium::chiral::content_born_phase(vector)
        } else {
            0.0
        };

        self.timestamps.push(chrono::Utc::now().timestamp_millis());
        self.metadata.push(WavefrontMeta::new(id, content));
        self.id_to_index.insert(id, index);
        self.len += 1;

        id
    }

    /// Remove a wavefront by ID using swap-remove (O(1) tensor op).
    /// Returns true if the wavefront was found and removed.
    pub fn remove(&mut self, id: &Uuid) -> bool {
        let index = match self.id_to_index.get(id) {
            Some(&idx) => idx,
            None => return false,
        };

        if self.len == 0 {
            return false;
        }

        let last = self.len - 1;
        self.id_to_index.remove(id);

        if index != last {
            let last_row = self.wavefronts.row(last).to_owned();
            self.wavefronts.row_mut(index).assign(&last_row);
            self.energy[index] = self.energy[last];
            self.frequency[index] = self.frequency[last];
            self.phase[index] = self.phase[last];

            self.timestamps.swap(index, last);
            self.metadata.swap(index, last);

            let swapped_id = self.metadata[index].id;
            self.id_to_index.insert(swapped_id, index);
        }

        self.timestamps.pop();
        self.metadata.pop();
        self.len -= 1;

        true
    }

    /// Shrink tensors to exactly fit active wavefronts.
    /// Called before persistence to avoid writing unused capacity rows.
    pub fn compact(&mut self) {
        if self.len < self.wavefronts.nrows() {
            self.wavefronts = self.wavefronts.slice(s![..self.len, ..]).to_owned();
            self.energy = self.energy.slice(s![..self.len]).to_owned();
            self.frequency = self.frequency.slice(s![..self.len]).to_owned();
            self.phase = self.phase.slice(s![..self.len]).to_owned();
        }
    }
}
