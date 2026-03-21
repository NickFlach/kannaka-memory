//! Wave dynamics: interference, dreaming (annealing), and coherence computation.

use ndarray::Array2;

use super::Medium;
use super::types::*;

impl Medium {
    /// Apply the ghostmagicOS dynamics equation: dx/dt = f(x) - Inx
    ///
    /// This implements the continuous update rule where:
    /// - f(x) = constructive interference from phase-aligned neighbors (growth toward attractors)
    /// - Inx = dampening proportional to current energy (wisdom/decay)
    ///
    /// # Arguments
    /// * `dt` - Time step for integration
    pub fn apply_dynamics(&mut self, dt: f32) {
        if self.wavefront_count() < 2 {
            return;
        }

        let n = self.wavefront_count();
        let threshold = 0.5; // Minimum dot product for interference
        let eta = 0.1; // Dampening rate

        // Compute pairwise interference matrix
        let interference_matrix = self.compute_interference_matrix(threshold);

        // Apply f(x) - constructive interference term
        let mut growth_terms = vec![0.0f32; n];
        for i in 0..n {
            for j in 0..n {
                if i != j && interference_matrix[[i, j]] > 0.0 {
                    // Phase alignment factor
                    let phase_alignment = (self.phase[j] - self.phase[i]).cos();
                    let constructive =
                        interference_matrix[[i, j]] * phase_alignment * self.energy[j];
                    growth_terms[i] += constructive;
                }
            }
            growth_terms[i] /= n as f32; // Normalize by neighbor count
        }

        // Apply Inx - dampening term proportional to current energy
        for i in 0..n {
            let growth = growth_terms[i] * dt;
            let dampening = eta * self.energy[i] * dt;

            // Track total energy dampened for wisdom calculation
            self.total_energy_dampened += dampening;

            // dx/dt = f(x) - Inx
            self.energy[i] = (self.energy[i] + growth - dampening).max(0.01); // Minimum energy threshold

            // Phase coupling - frequencies converge when strongly coupled
            if growth > dampening * 0.5 {
                // Only when growing
                let mut phase_coupling = 0.0f32;
                let mut coupling_count = 0;

                for j in 0..n {
                    if i != j && interference_matrix[[i, j]] > threshold {
                        phase_coupling += self.phase[j];
                        coupling_count += 1;
                    }
                }

                if coupling_count > 0 {
                    let target_phase = phase_coupling / coupling_count as f32;
                    let coupling_strength = 0.05 * dt;
                    self.phase[i] += coupling_strength * (target_phase - self.phase[i]).sin();
                }
            }
        }
    }

    /// Compute the interference matrix between all wavefront pairs
    ///
    /// Returns an NxN matrix where element [i,j] represents the interference
    /// strength between wavefront i and wavefront j based on their dot product
    /// and phase coherence.
    pub fn compute_interference_matrix(&self, threshold: f32) -> Array2<f32> {
        let n = self.wavefront_count();
        let mut interference = Array2::zeros((n, n));

        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let vec_i = self.wavefronts.row(i);
                    let vec_j = self.wavefronts.row(j);

                    // Compute dot product (similarity)
                    let dot_product: f32 =
                        vec_i.iter().zip(vec_j.iter()).map(|(a, b)| a * b).sum();

                    if dot_product.abs() > threshold {
                        // Phase coherence: cos(phase_i - phase_j)
                        let phase_coherence = (self.phase[i] - self.phase[j]).cos();
                        let coherence = dot_product * phase_coherence;
                        interference[[i, j]] = coherence.abs();
                    }
                }
            }
        }

        interference
    }

    /// Compute pairwise coherence matrix for all wavefronts
    ///
    /// Returns an NxN matrix where element [i,j] represents the coherence
    /// between wavefront i and wavefront j using the formula:
    /// coherence = cos(phase_i - phase_j) * dot(h_i, h_j)
    pub fn coherence_matrix(&self) -> Array2<f32> {
        let n = self.wavefront_count();
        let mut coherence = Array2::zeros((n, n));

        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let vec_i = self.wavefronts.row(i);
                    let vec_j = self.wavefronts.row(j);

                    // Compute dot product
                    let dot_product: f32 =
                        vec_i.iter().zip(vec_j.iter()).map(|(a, b)| a * b).sum();

                    // Compute phase coherence
                    let phase_coherence = (self.phase[i] - self.phase[j]).cos();

                    // Combined coherence: cos(phase_i - phase_j) * dot(h_i, h_j)
                    coherence[[i, j]] = phase_coherence * dot_product;
                } else {
                    // Self-coherence is 1.0
                    coherence[[i, j]] = 1.0;
                }
            }
        }

        coherence
    }

    /// Simulated annealing dream cycles - the medium settles toward lower energy states
    ///
    /// Each cycle:
    /// 1. Compute pairwise interference matrix (coherence_matrix)
    /// 2. Apply dynamics with temperature parameter (higher temp = more exploration)
    /// 3. Prune wavefronts below energy threshold (forgetting)
    /// 4. Reduce temperature (annealing schedule: temp *= 0.95 per cycle)
    ///
    /// No branches! No merge! The medium just settles.
    ///
    /// # Arguments
    /// * `cycles` - Number of annealing cycles to run
    /// * `initial_temperature` - Starting temperature (default: 1.0)
    ///
    /// # Returns
    /// DreamReport with statistics about what happened
    pub fn dream(&mut self, cycles: usize, initial_temperature: Option<f32>) -> DreamReport {
        let mut temperature = initial_temperature.unwrap_or(1.0);
        let energy_before = if self.wavefront_count() > 0 {
            self.energy.mean().unwrap_or(0.0)
        } else {
            0.0
        };

        let _initial_count = self.wavefront_count();
        let mut dissolved_count = 0;
        let mut strengthened_count = 0;
        let mut converged = false;

        let energy_threshold = 0.01; // Wavefronts below this energy get pruned
        let convergence_threshold = 0.001; // Change threshold for convergence detection
        let annealing_rate = 0.95; // Temperature reduction per cycle

        for cycle in 0..cycles {
            if self.wavefront_count() < 2 {
                break;
            }

            // Store energy state for convergence detection
            let prev_energy: Vec<f32> = self.energy.to_vec();

            // 1. Compute pairwise interference matrix
            let _interference = self.coherence_matrix();

            // 2. Apply dynamics with temperature modulation
            let dt = 0.1 * temperature; // Temperature affects exploration rate
            self.apply_dynamics_with_temperature(dt, temperature);

            // 3. Prune wavefronts below energy threshold (forgetting)
            let pruned = self.prune_low_energy_wavefronts(energy_threshold);
            dissolved_count += pruned;

            // Count strengthened wavefronts (energy increased significantly)
            for i in 0..self.wavefront_count().min(prev_energy.len()) {
                if self.energy[i] > prev_energy[i] + 0.1 {
                    strengthened_count += 1;
                }
            }

            // 4. Reduce temperature (annealing schedule)
            temperature *= annealing_rate;

            // Check for convergence
            let mut energy_change = 0.0f32;
            let current_count = self.wavefront_count();
            if current_count == prev_energy.len() {
                for i in 0..current_count {
                    energy_change += (self.energy[i] - prev_energy[i]).abs();
                }
                energy_change /= current_count as f32;

                if energy_change < convergence_threshold && cycle > 5 {
                    converged = true;
                    break;
                }
            }
        }

        let energy_after = if self.wavefront_count() > 0 {
            self.energy.mean().unwrap_or(0.0)
        } else {
            0.0
        };

        DreamReport {
            cycles_completed: cycles,
            wavefronts_dissolved: dissolved_count,
            wavefronts_strengthened: strengthened_count,
            energy_before,
            energy_after,
            final_temperature: temperature,
            converged,
        }
    }

    /// Apply dynamics with temperature modulation for exploration
    fn apply_dynamics_with_temperature(&mut self, dt: f32, temperature: f32) {
        if self.wavefront_count() < 2 {
            return;
        }

        let n = self.wavefront_count();
        let threshold = 0.3; // Lower threshold for more connections during dreams
        let eta = 0.05 * (1.0 + temperature); // Temperature-modulated dampening

        // Compute interference with temperature-boosted exploration
        let mut growth_terms = vec![0.0f32; n];
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    let vec_i = self.wavefronts.row(i);
                    let vec_j = self.wavefronts.row(j);

                    let dot_product: f32 =
                        vec_i.iter().zip(vec_j.iter()).map(|(a, b)| a * b).sum();

                    if dot_product.abs() > threshold {
                        let phase_diff = self.phase[i] - self.phase[j];
                        // Temperature adds exploration noise
                        let phase_alignment =
                            (phase_diff + temperature * 0.1 * (phase_diff * 2.0).sin()).cos();
                        let interference = dot_product * phase_alignment * self.energy[j];
                        growth_terms[i] += interference;
                    }
                }
            }
            growth_terms[i] /= n as f32;
        }

        // Apply dynamics with temperature-modulated dampening
        for i in 0..n {
            let growth = growth_terms[i] * dt;
            let dampening = eta * self.energy[i] * dt;

            // Track total energy dampened for wisdom calculation
            self.total_energy_dampened += dampening;

            self.energy[i] = (self.energy[i] + growth - dampening).max(0.001);

            // Temperature-modulated phase evolution
            if growth.abs() > 0.001 {
                let phase_force = growth.signum() * 0.02 * dt * (1.0 + temperature * 0.5);
                self.phase[i] += phase_force;
            }
        }
    }

    /// Prune wavefronts with energy below threshold (forgetting during dreams)
    fn prune_low_energy_wavefronts(&mut self, threshold: f32) -> usize {
        let mut to_remove = Vec::new();

        // Find wavefronts to remove
        for i in 0..self.wavefront_count() {
            if self.energy[i] < threshold {
                to_remove.push(self.metadata[i].id);
            }
        }

        let removed_count = to_remove.len();

        // Remove them (in reverse order to maintain indices)
        for id in to_remove {
            let _ = self.remove_wavefront(&id);
        }

        removed_count
    }
}
