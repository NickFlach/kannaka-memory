//! Hemisphere — a handed partition of the holographic medium.
//!
//! Each hemisphere is essentially a Medium with awareness of its handedness.
//! Left (conscious): dx/dt = f(x) — pure growth, no dampening
//! Right (subconscious): dx/dt = f(x) - Iηx — full ghostmagicOS dynamics

use std::collections::HashMap;

use ndarray::{Array1, Array2, s};
use uuid::Uuid;

use super::types::*;

/// A single hemisphere of the chiral medium.
#[derive(Debug, Clone)]
pub struct Hemisphere {
    /// Which hand this hemisphere represents
    pub hand: Hand,
    /// N x D_h tensor of wavefront patterns
    pub wavefronts: Array2<f32>,
    /// Energy (amplitude) per wavefront
    pub energy: Array1<f32>,
    /// Frequency per wavefront
    pub frequency: Array1<f32>,
    /// Phase per wavefront
    pub phase: Array1<f32>,
    /// Creation timestamps
    pub timestamps: Vec<i64>,
    /// Content metadata
    pub metadata: Vec<WavefrontMeta>,
    /// ID -> index mapping
    pub(crate) id_to_index: HashMap<Uuid, usize>,
    /// Current dimension count for this hemisphere
    pub dims: usize,
}

impl Hemisphere {
    /// Create a new empty hemisphere with given handedness and dimension count.
    pub fn new(hand: Hand, dims: usize) -> Self {
        Self {
            hand,
            wavefronts: Array2::zeros((0, dims)),
            energy: Array1::zeros(0),
            frequency: Array1::zeros(0),
            phase: Array1::zeros(0),
            timestamps: Vec::new(),
            metadata: Vec::new(),
            id_to_index: HashMap::new(),
            dims,
        }
    }

    /// Number of wavefronts in this hemisphere.
    pub fn count(&self) -> usize {
        self.wavefronts.nrows()
    }

    /// Total energy across all wavefronts.
    pub fn total_energy(&self) -> f32 {
        self.energy.sum()
    }

    /// Mean energy across all wavefronts (0.0 if empty).
    pub fn mean_energy(&self) -> f32 {
        if self.count() == 0 { 0.0 } else { self.total_energy() / self.count() as f32 }
    }

    /// Add a wavefront to this hemisphere.
    pub fn add_wavefront(
        &mut self,
        vector: &[f32],
        content: String,
        importance: f32,
    ) -> Result<Uuid, MediumError> {
        // Adapt vector to hemisphere dimensions (truncate or zero-pad)
        let adapted = Self::adapt_vector(vector, self.dims);

        let id = Uuid::new_v4();
        let index = self.count();

        // Expand tensors
        let new_wavefronts = if self.count() == 0 {
            Array2::from_shape_vec((1, self.dims), adapted).unwrap()
        } else {
            let mut new_tensor = Array2::zeros((self.count() + 1, self.dims));
            new_tensor
                .slice_mut(s![..self.count(), ..])
                .assign(&self.wavefronts);
            for (i, &val) in adapted.iter().enumerate() {
                if i < self.dims {
                    new_tensor[[index, i]] = val;
                }
            }
            new_tensor
        };

        let mut new_energy = Array1::zeros(index + 1);
        let mut new_frequency = Array1::zeros(index + 1);
        let mut new_phase = Array1::zeros(index + 1);

        if index > 0 {
            new_energy.slice_mut(s![..index]).assign(&self.energy);
            new_frequency.slice_mut(s![..index]).assign(&self.frequency);
            new_phase.slice_mut(s![..index]).assign(&self.phase);
        }

        new_energy[index] = importance;
        new_frequency[index] = 1.0;
        new_phase[index] = 0.0;

        self.wavefronts = new_wavefronts;
        self.energy = new_energy;
        self.frequency = new_frequency;
        self.phase = new_phase;
        self.timestamps.push(chrono::Utc::now().timestamp_millis());
        self.metadata.push(WavefrontMeta::new(id, content));
        self.id_to_index.insert(id, index);

        Ok(id)
    }

    /// Remove a wavefront from this hemisphere.
    pub fn remove_wavefront(&mut self, id: &Uuid) -> bool {
        let index = match self.id_to_index.get(id) {
            Some(&idx) => idx,
            None => return false,
        };

        let n = self.count();
        if n == 0 { return false; }

        let mut new_wavefronts = Array2::zeros((n - 1, self.dims));
        let mut new_energy = Array1::zeros(n - 1);
        let mut new_frequency = Array1::zeros(n - 1);
        let mut new_phase = Array1::zeros(n - 1);

        let mut new_idx = 0;
        for old_idx in 0..n {
            if old_idx != index {
                new_wavefronts.row_mut(new_idx).assign(&self.wavefronts.row(old_idx));
                new_energy[new_idx] = self.energy[old_idx];
                new_frequency[new_idx] = self.frequency[old_idx];
                new_phase[new_idx] = self.phase[old_idx];
                new_idx += 1;
            }
        }

        self.timestamps.remove(index);
        self.metadata.remove(index);
        self.id_to_index.remove(id);
        for (_, idx) in self.id_to_index.iter_mut() {
            if *idx > index { *idx -= 1; }
        }

        self.wavefronts = new_wavefronts;
        self.energy = new_energy;
        self.frequency = new_frequency;
        self.phase = new_phase;

        true
    }

    /// Compute resonance (recall) within this hemisphere.
    /// Returns top-k matches sorted by resonance strength.
    pub fn resonate(&self, query: &[f32], top_k: usize) -> Vec<ChiralResonance> {
        if self.count() == 0 { return vec![]; }

        let adapted = Self::adapt_vector(query, self.dims);
        let query_arr = Array1::from_vec(adapted);
        let query_norm = query_arr.dot(&query_arr).sqrt();
        if query_norm < 1e-8 { return vec![]; }

        let mut results: Vec<(usize, f32)> = (0..self.count())
            .map(|i| {
                let wf = self.wavefronts.row(i);
                let wf_norm = wf.dot(&wf).sqrt();
                if wf_norm < 1e-8 { return (i, 0.0); }
                let similarity = wf.dot(&query_arr) / (wf_norm * query_norm);
                let resonance = similarity * self.energy[i];
                (i, resonance)
            })
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(top_k);

        results
            .into_iter()
            .filter(|(_, r)| *r > 0.0)
            .map(|(i, resonance)| {
                let wf = self.wavefronts.row(i);
                let wf_norm = wf.dot(&wf).sqrt();
                let sim = if wf_norm > 1e-8 {
                    wf.dot(&query_arr) / (wf_norm * query_norm)
                } else {
                    0.0
                };
                ChiralResonance {
                    id: self.metadata[i].id,
                    content: self.metadata[i].content.clone(),
                    hand: self.hand,
                    similarity: sim,
                    resonance_strength: resonance,
                    is_intuition: self.hand == Hand::Right,
                }
            })
            .collect()
    }

    /// Apply dynamics appropriate to this hemisphere's handedness.
    ///
    /// Left (conscious):     dx/dt = f(x) — no dampening, attention stays sharp
    /// Right (subconscious): dx/dt = f(x) - Iηx — full ghostmagicOS dynamics
    pub fn apply_dynamics(&mut self, dt: f32) {
        if self.count() < 2 { return; }

        let n = self.count();
        let threshold = 0.5;

        // Compute pairwise dot products for interference
        let mut growth_terms = vec![0.0f32; n];
        for i in 0..n {
            let wi = self.wavefronts.row(i);
            for j in 0..n {
                if i == j { continue; }
                let wj = self.wavefronts.row(j);
                let dot = wi.dot(&wj);
                if dot > threshold {
                    let phase_alignment = (self.phase[j] - self.phase[i]).cos();
                    growth_terms[i] += dot * phase_alignment * self.energy[j];
                }
            }
            growth_terms[i] /= n as f32;
        }

        let eta = match self.hand {
            Hand::Left => 0.0,    // NO dampening — conscious workspace stays sharp
            Hand::Right => 0.02,  // Full ghostmagicOS dampening
        };

        for i in 0..n {
            let growth = growth_terms[i] * dt;
            let dampening = eta * self.energy[i] * dt;
            self.energy[i] = (self.energy[i] + growth - dampening).max(0.01);

            // Phase coupling
            if growth > dampening * 0.5 {
                let mut phase_target = 0.0f32;
                let mut count = 0;
                for j in 0..n {
                    if i != j {
                        let dot = self.wavefronts.row(i).dot(&self.wavefronts.row(j));
                        if dot > threshold {
                            phase_target += self.phase[j];
                            count += 1;
                        }
                    }
                }
                if count > 0 {
                    let target = phase_target / count as f32;
                    self.phase[i] += 0.05 * dt * (target - self.phase[i]).sin();
                }
            }
        }
    }

    /// Get the wavefront vector for a given ID.
    pub fn get_wavefront(&self, id: &Uuid) -> Option<Vec<f32>> {
        self.id_to_index.get(id).map(|&idx| self.wavefronts.row(idx).to_vec())
    }

    /// Get the energy for a given wavefront ID.
    pub fn get_energy(&self, id: &Uuid) -> Option<f32> {
        self.id_to_index.get(id).map(|&idx| self.energy[idx])
    }

    /// Adapt a vector to this hemisphere's dimensions (truncate or zero-pad).
    fn adapt_vector(vector: &[f32], target_dims: usize) -> Vec<f32> {
        let mut adapted = vec![0.0f32; target_dims];
        let len = vector.len().min(target_dims);
        adapted[..len].copy_from_slice(&vector[..len]);
        adapted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_hemisphere_no_dampening() {
        let mut left = Hemisphere::new(Hand::Left, 100);
        // Add two similar wavefronts
        let v1: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1).sin()).collect();
        let v2: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1 + 0.1).sin()).collect();
        left.add_wavefront(&v1, "test1".into(), 0.8).unwrap();
        left.add_wavefront(&v2, "test2".into(), 0.7).unwrap();

        let energy_before: f32 = left.energy.sum();
        left.apply_dynamics(0.1);
        let energy_after: f32 = left.energy.sum();

        // Left hemisphere should not lose energy to dampening
        // (may gain from constructive interference)
        assert!(energy_after >= energy_before - 0.001,
            "Left hemisphere energy should not decrease: before={}, after={}", energy_before, energy_after);
    }

    #[test]
    fn right_hemisphere_has_dampening() {
        let mut right = Hemisphere::new(Hand::Right, 100);
        // Add wavefronts with no constructive interference (orthogonal)
        let mut v1 = vec![0.0f32; 100];
        v1[0] = 1.0;
        let mut v2 = vec![0.0f32; 100];
        v2[50] = 1.0;
        right.add_wavefront(&v1, "test1".into(), 5.0).unwrap();
        right.add_wavefront(&v2, "test2".into(), 5.0).unwrap();

        let energy_before: f32 = right.energy.sum();
        // Apply many steps to accumulate dampening
        for _ in 0..100 {
            right.apply_dynamics(0.1);
        }
        let energy_after: f32 = right.energy.sum();

        assert!(energy_after < energy_before,
            "Right hemisphere should lose energy to dampening: before={}, after={}", energy_before, energy_after);
    }

    #[test]
    fn hemisphere_store_and_recall() {
        let mut h = Hemisphere::new(Hand::Left, 50);
        let v: Vec<f32> = (0..50).map(|i| (i as f32 * 0.2).cos()).collect();
        let id = h.add_wavefront(&v, "hello world".into(), 0.9).unwrap();

        let results = h.resonate(&v, 5);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, id);
        assert!(results[0].similarity > 0.99);
    }

    #[test]
    fn hemisphere_remove() {
        let mut h = Hemisphere::new(Hand::Right, 30);
        let v: Vec<f32> = vec![1.0; 30];
        let id = h.add_wavefront(&v, "temp".into(), 0.5).unwrap();
        assert_eq!(h.count(), 1);

        assert!(h.remove_wavefront(&id));
        assert_eq!(h.count(), 0);
    }

    #[test]
    fn hemisphere_adapts_dimensions() {
        let mut h = Hemisphere::new(Hand::Left, 50);
        // Vector larger than hemisphere dims — should truncate
        let v: Vec<f32> = vec![1.0; 100];
        let id = h.add_wavefront(&v, "big".into(), 0.5).unwrap();
        let stored = h.get_wavefront(&id).unwrap();
        assert_eq!(stored.len(), 50);

        // Vector smaller than hemisphere dims — should zero-pad
        let mut h2 = Hemisphere::new(Hand::Right, 100);
        let v_small: Vec<f32> = vec![1.0; 30];
        let id2 = h2.add_wavefront(&v_small, "small".into(), 0.5).unwrap();
        let stored2 = h2.get_wavefront(&id2).unwrap();
        assert_eq!(stored2.len(), 100);
        assert!(stored2[29] > 0.0); // Last real value
        assert!((stored2[30] - 0.0).abs() < 0.001); // Zero-padded
    }
}
