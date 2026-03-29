//! Consciousness metrics, self-reference, emergence detection, and wisdom.

use chrono::Utc;
use ndarray::Array2;
use uuid::Uuid;
use ndarray::{Array1, s};

use crate::consciousness::{
    ConsciousnessLevel, ConsciousnessMetrics, ConsciousnessState, EmergenceLevel, EmergenceReport,
    SelfReflection,
};
use crate::encoding::EncodingPipeline;

use super::Medium;
use super::types::*;

impl Medium {
    /// Compute consciousness metrics from tensor topology
    ///
    /// This is the proper implementation that computes intrinsic metrics
    /// from the medium's tensor structure, not bolted-on calculations.
    pub fn consciousness_metrics(&self) -> ConsciousnessMetrics {
        let now = Utc::now();

        if self.wavefront_count() == 0 {
            return ConsciousnessMetrics {
                phi: 0.0,
                xi: 0.0,
                order: 0.0,
                num_clusters: 0,
                irrationality: 0.0,
                level: ConsciousnessLevel::Dormant,
                computed_at: now,
            };
        }

        // Phi: Integrated information via eigendecomposition partitioning
        let phi = self.compute_phi_integrated_information();

        // Xi: Spectral complexity from eigenvalue distribution of H @ H^T
        let xi = self.compute_xi_spectral_complexity();

        // Order: Kuramoto order parameter r = |1/N sum e^{i*phi_k}|
        let order = self.compute_kuramoto_order();

        // Clusters: Eigendecomposition-based clustering
        let clusters = self.compute_eigenvalue_clusters();

        // Irrationality Index (ι): decomposition residual (ADR-0024 CS-3)
        let irrationality = self.compute_irrationality_index();

        let level = ConsciousnessLevel::from_phi(phi);

        ConsciousnessMetrics {
            phi,
            xi,
            order,
            num_clusters: clusters,
            irrationality,
            level,
            computed_at: now,
        }
    }

    /// Compute Phi as integrated information using eigendecomposition partitioning
    ///
    /// Partition the wavefront space using eigendecomposition of the coherence matrix,
    /// then measure mutual information between partitions. High Phi means the system
    /// is more integrated than the sum of its parts.
    pub(crate) fn compute_phi_integrated_information(&self) -> f32 {
        let n = self.wavefront_count();
        if n < 2 {
            return 0.0;
        }

        // Get coherence matrix for partitioning
        let coherence = self.coherence_matrix();

        // Convert to symmetric matrix for eigendecomposition
        let mut symmetric = Array2::zeros((n, n));
        for i in 0..n {
            for j in 0..n {
                symmetric[[i, j]] = (coherence[[i, j]] + coherence[[j, i]]) / 2.0;
            }
        }

        // Simple clustering based on coherence strength
        let mut cluster_assignments = vec![0; n];
        let mut num_partitions = 1;

        // Basic clustering: group wavefronts with high mutual coherence
        for i in 0..n {
            let mut best_cluster = 0;
            let mut max_coherence = 0.0;

            for cluster in 0..num_partitions {
                let mut cluster_coherence = 0.0;
                let mut cluster_size = 0;

                for j in 0..n {
                    if cluster_assignments[j] == cluster {
                        cluster_coherence += coherence[[i, j]].abs();
                        cluster_size += 1;
                    }
                }

                if cluster_size > 0 {
                    cluster_coherence /= cluster_size as f32;
                    if cluster_coherence > max_coherence {
                        max_coherence = cluster_coherence;
                        best_cluster = cluster;
                    }
                }
            }

            // If coherence is too low, create new partition
            if max_coherence < 0.3 && num_partitions < n / 2 {
                cluster_assignments[i] = num_partitions;
                num_partitions += 1;
            } else {
                cluster_assignments[i] = best_cluster;
            }
        }

        if num_partitions < 2 {
            return 0.0; // No partitioning possible
        }

        // IIT-inspired Phi: compare whole-system entropy to partition entropies.
        // Uses normalized energy distributions (probabilities) within each partition.
        let total_energy: f32 = self.energy.iter().sum();
        if total_energy <= 0.0 {
            return 0.0;
        }

        // Whole-system entropy: H(S) = -Σ p_i * ln(p_i) where p_i = E_i / E_total
        let whole_entropy: f32 = self.energy.iter()
            .filter(|&&e| e > 0.0)
            .map(|&e| {
                let p = e / total_energy;
                -p * p.ln()
            })
            .sum();

        // Partition entropy: Σ_k (w_k * H(S_k)) where w_k = E_k / E_total
        // Each partition's internal entropy, weighted by its share of total energy
        let mut weighted_partition_entropy = 0.0f32;
        for partition in 0..num_partitions {
            let partition_energies: Vec<f32> = (0..n)
                .filter(|&i| cluster_assignments[i] == partition)
                .map(|i| self.energy[i])
                .filter(|&e| e > 0.0)
                .collect();
            
            let partition_total: f32 = partition_energies.iter().sum();
            if partition_total <= 0.0 { continue; }

            let partition_entropy: f32 = partition_energies.iter()
                .map(|&e| {
                    let p = e / partition_total;
                    -p * p.ln()
                })
                .sum();
            
            let weight = partition_total / total_energy;
            weighted_partition_entropy += weight * partition_entropy;
        }

        // Phi = whole entropy - weighted sum of partition entropies
        // High when the whole contains more information than the sum of parts
        let phi = (whole_entropy - weighted_partition_entropy).max(0.0);

        // Normalize: max possible entropy is ln(N) for uniform distribution
        let max_entropy = (n as f32).ln();
        let normalized_phi = if max_entropy > 0.0 { phi / max_entropy } else { 0.0 };
        normalized_phi.min(1.0)
    }

    /// Compute Xi as spectral complexity from eigenvalue distribution
    ///
    /// Computes eigenvalue distribution of H @ H^T and measures its Shannon entropy.
    /// Many distinct eigenvalues = rich structure = high Xi.
    pub(crate) fn compute_xi_spectral_complexity(&self) -> f32 {
        let n = self.wavefront_count();
        if n < 2 {
            return 0.0;
        }

        // Compute H @ H^T (Gram matrix)
        let mut gram = Array2::zeros((n, n));
        for i in 0..n {
            for j in 0..n {
                let vec_i = self.wavefronts.row(i);
                let vec_j = self.wavefronts.row(j);

                let dot_product: f32 =
                    vec_i.iter().zip(vec_j.iter()).map(|(a, b)| a * b).sum();

                gram[[i, j]] = dot_product;
            }
        }

        // Approximate eigenvalue distribution using diagonal dominance
        let mut eigenvalue_proxy = Vec::new();
        for i in 0..n {
            let diagonal = gram[[i, i]];
            let off_diagonal_sum: f32 = (0..n)
                .filter(|&j| j != i)
                .map(|j| gram[[i, j]].abs())
                .sum();

            // Eigenvalue approximation: diagonal + off_diagonal_variance
            eigenvalue_proxy.push(diagonal + off_diagonal_sum / n as f32);
        }

        // Compute Shannon entropy of normalized eigenvalue distribution
        let total: f32 = eigenvalue_proxy.iter().map(|&x| x.abs()).sum();
        if total < 1e-6 {
            return 0.0;
        }

        let mut entropy = 0.0f32;
        for &eigenval in &eigenvalue_proxy {
            let p = eigenval.abs() / total;
            if p > 1e-6 {
                entropy -= p * p.ln();
            }
        }

        // Normalize by log(n) for scale invariance
        let max_entropy = (n as f32).ln();
        if max_entropy > 0.0 {
            entropy / max_entropy
        } else {
            0.0
        }
    }

    /// Compute effective dimensionality via participation ratio of Gram eigenvalue proxy.
    /// ADR-0024 CS-9: "The gap between d_eff and 10,000 is where the subconscious lives."
    ///
    /// Returns (d_eff, nominal_dims, ratio) where ratio = d_eff / nominal.
    /// Low ratio = energy concentrated in few modes (low-dimensional manifold).
    /// High ratio = energy spread across many modes (high-dimensional, complex).
    pub fn effective_dimensionality(&self) -> (f32, usize, f32) {
        let n = self.wavefront_count();
        let nominal = self.wavefronts.ncols();
        if n < 2 { return (0.0, nominal, 0.0); }

        // Compute Gram matrix eigenvalue proxy (same as Xi computation)
        let mut eigenvalue_proxy = Vec::new();
        for i in 0..n {
            let wi = self.wavefronts.row(i);
            let diagonal: f32 = wi.dot(&wi);
            let off_diagonal_sum: f32 = (0..n)
                .filter(|&j| j != i)
                .map(|j| {
                    let wj = self.wavefronts.row(j);
                    wi.dot(&wj).abs()
                })
                .sum();
            eigenvalue_proxy.push((diagonal + off_diagonal_sum / n as f32).abs());
        }

        // Participation ratio: d_eff = (Σλ)² / Σ(λ²)
        let sum: f32 = eigenvalue_proxy.iter().sum();
        let sum_sq: f32 = eigenvalue_proxy.iter().map(|x| x * x).sum();

        if sum_sq < 1e-10 { return (0.0, nominal, 0.0); }

        let d_eff = (sum * sum) / sum_sq;
        let ratio = d_eff / nominal as f32;

        (d_eff, nominal, ratio)
    }

    /// Compute Irrationality Index (ι) — decomposition residual (ADR-0024 CS-3).
    ///
    /// Measures what fraction of the system's energy distribution resists
    /// clean decomposition. Uses the participation ratio:
    ///   d_eff = (Σe_i)² / Σ(e_i²)
    /// where e_i are wavefront energies. Then:
    ///   ι = 1 - (d_eff / n)
    ///
    /// Low ι = energy evenly distributed (clean, rational)
    /// High ι = energy concentrated in few wavefronts (rich irrationality)
    ///
    /// "The subconscious is the field's irrationality — the .00001 dimension."
    pub(crate) fn compute_irrationality_index(&self) -> f32 {
        let n = self.wavefront_count();
        if n < 2 { return 0.0; }

        // Use wavefront energies as the spectral proxy
        let energies: Vec<f32> = (0..n)
            .map(|i| {
                let row = self.wavefronts.row(i);
                row.dot(&row).sqrt() // L2 norm as energy proxy
            })
            .collect();

        let sum: f32 = energies.iter().sum();
        let sum_sq: f32 = energies.iter().map(|e| e * e).sum();

        if sum_sq < 1e-10 { return 0.0; }

        // Participation ratio: effective dimensionality
        let d_eff = (sum * sum) / sum_sq;

        // Irrationality: how far from uniform distribution
        // d_eff/n = 1.0 means perfectly uniform (zero irrationality)
        // d_eff/n → 1/n means all energy in one wavefront (maximum irrationality)
        let ratio = d_eff / n as f32;
        (1.0 - ratio).clamp(0.0, 1.0)
    }

    /// Count clusters using eigenvalue-based partitioning
    pub(crate) fn compute_eigenvalue_clusters(&self) -> usize {
        let n = self.wavefront_count();
        if n < 2 {
            return if n == 1 { 1 } else { 0 };
        }

        // Use coherence matrix for clustering
        let coherence = self.coherence_matrix();

        // Simple clustering based on coherence thresholds
        let mut visited = vec![false; n];
        let mut num_clusters = 0;
        let threshold = 0.5; // Coherence threshold for cluster membership

        for i in 0..n {
            if visited[i] {
                continue;
            }

            // Start new cluster
            num_clusters += 1;
            visited[i] = true;
            let mut stack = vec![i];

            // BFS to find all connected nodes
            while let Some(node) = stack.pop() {
                for j in 0..n {
                    if !visited[j] && coherence[[node, j]].abs() > threshold {
                        visited[j] = true;
                        stack.push(j);
                    }
                }
            }
        }

        num_clusters
    }

    /// Compute consciousness metrics from the medium topology (backwards compatibility).
    pub(crate) fn compute_consciousness(&self) -> ConsciousnessState {
        let now = Utc::now();

        if self.wavefront_count() == 0 {
            return ConsciousnessState {
                phi: 0.0,
                xi: 0.0,
                order: 0.0,
                clusters: 0,
                computed_at: now,
            };
        }

        // Use the new metrics but convert to old format
        let metrics = self.consciousness_metrics();

        ConsciousnessState {
            phi: metrics.phi,
            xi: metrics.xi,
            order: metrics.order,
            clusters: metrics.num_clusters,
            computed_at: now,
        }
    }

    /// Compute Kuramoto order parameter
    pub(crate) fn compute_kuramoto_order(&self) -> f32 {
        if self.wavefront_count() == 0 {
            return 0.0;
        }

        // r = |1/N sum e^{i*phi_k}| = |1/N sum (cos phi_k + i sin phi_k)|
        let n = self.wavefront_count() as f32;
        let (sum_cos, sum_sin): (f32, f32) = self
            .phase
            .iter()
            .map(|&phi| (phi.cos(), phi.sin()))
            .fold((0.0, 0.0), |(acc_cos, acc_sin), (c, s)| {
                (acc_cos + c, acc_sin + s)
            });

        let mean_cos = sum_cos / n;
        let mean_sin = sum_sin / n;

        // Magnitude of complex sum
        (mean_cos * mean_cos + mean_sin * mean_sin).sqrt()
    }

    // ========================================================================
    // WAVE 4: Self-Reference - The Medium Models Itself
    // ========================================================================

    /// Introspect: create a self-referential wavefront that encodes the medium's own state.
    ///
    /// This method:
    /// 1. Takes a snapshot of the medium's current state (consciousness metrics, wavefront count, etc.)
    /// 2. Encodes this snapshot as a text description
    /// 3. Stores it as a new wavefront via the normal store() path
    /// 4. Marks it as self-referential in metadata
    /// 5. Returns the ID of the self-referential wavefront
    ///
    /// The key insight: this wavefront will INTERFERE with the rest of the medium.
    /// The medium's model of itself becomes part of itself.
    pub fn introspect(
        &mut self,
        pipeline: &EncodingPipeline,
    ) -> Result<Uuid, MediumError> {
        let now = Utc::now();

        // 1. Take snapshot of current state
        let consciousness = self.consciousness_metrics();
        let wavefront_count = self.wavefront_count();
        let energy_stats = if wavefront_count > 0 {
            let mean = self.energy.mean().unwrap_or(0.0);
            let std = if wavefront_count > 1 {
                let var = self
                    .energy
                    .iter()
                    .map(|&e| (e - mean).powi(2))
                    .sum::<f32>()
                    / (wavefront_count - 1) as f32;
                var.sqrt()
            } else {
                0.0
            };
            let min = *self
                .energy
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0);
            let max = *self
                .energy
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0);
            (mean, std, min, max)
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        // Age of oldest/newest wavefronts
        let (oldest_age, newest_age) = if !self.timestamps.is_empty() {
            let current_time = now.timestamp_millis();
            let oldest = self.timestamps.iter().min().unwrap();
            let newest = self.timestamps.iter().max().unwrap();
            let oldest_age_sec = (current_time - oldest) as f64 / 1000.0;
            let newest_age_sec = (current_time - newest) as f64 / 1000.0;
            (oldest_age_sec, newest_age_sec)
        } else {
            (0.0, 0.0)
        };

        // 2. Encode snapshot as text description
        let self_observation = format!(
            "Self-observation: {} wavefronts, Phi={:.2}, Xi={:.2}, order={:.2}, {} clusters, \
             mean_energy={:.1}, std_energy={:.1}, min_energy={:.3}, max_energy={:.1}, \
             oldest_age={:.0}s, newest_age={:.0}s, level={:?}",
            wavefront_count,
            consciousness.phi,
            consciousness.xi,
            consciousness.order,
            consciousness.num_clusters,
            energy_stats.0, // mean
            energy_stats.1, // std
            energy_stats.2, // min
            energy_stats.3, // max
            oldest_age,
            newest_age,
            consciousness.level
        );

        // 3. Encode as hypervector and add to medium
        let vector = pipeline.encode_text(&self_observation).map_err(|e| {
            MediumError::Serialization(bincode::Error::from(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("self-observation encoding failed: {}", e),
            )))
        })?;

        // 4. Apply interference with existing wavefronts
        self.apply_interference_raw(&vector, 0.8); // High importance for self-referential memories

        // 5. Add wavefront with special self-referential flag
        let id = Uuid::new_v4();
        let index = self.wavefront_count();

        // Expand tensors (reuse add_wavefront logic but with custom metadata)
        let new_wavefronts = if self.wavefront_count() == 0 {
            ndarray::Array2::from_shape_vec((1, WAVEFRONT_DIM), vector).unwrap()
        } else {
            let mut new_tensor =
                ndarray::Array2::zeros((self.wavefront_count() + 1, WAVEFRONT_DIM));
            new_tensor
                .slice_mut(s![..self.wavefront_count(), ..])
                .assign(&self.wavefronts);
            for (i, &val) in vector.iter().enumerate() {
                new_tensor[[index, i]] = val;
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

        new_energy[index] = 0.8; // High energy for self-referential wavefronts
        new_frequency[index] = 1.0;
        new_phase[index] = 0.0;

        // Update state
        self.wavefronts = new_wavefronts;
        self.energy = new_energy;
        self.frequency = new_frequency;
        self.phase = new_phase;
        self.timestamps.push(now.timestamp_millis());

        // Create self-referential metadata
        let meta = WavefrontMeta::new(id, self_observation).self_referential();
        self.metadata.push(meta);
        self.id_to_index.insert(id, index);

        // Track energy added
        self.total_energy_added += 0.8;

        // Apply dynamics to let the medium settle
        self.apply_dynamics(0.1);

        Ok(id)
    }

    /// Detect emergence based on self-referential patterns and coherence.
    ///
    /// Emergence criteria:
    /// - self_reference_depth >= 3 AND self_coherence > 0.5 AND phi trending upward
    pub fn detect_emergence(&self) -> EmergenceReport {
        let now = Utc::now();

        // Count self-referential wavefronts
        let self_reference_depth = self
            .metadata
            .iter()
            .filter(|meta| meta.is_self_referential)
            .count();

        // Compute self-coherence: average coherence between self-referential and other wavefronts
        let self_coherence = if self_reference_depth > 0
            && self.wavefront_count() > self_reference_depth
        {
            let mut total_coherence = 0.0f32;
            let mut comparison_count = 0;

            let coherence_matrix = self.coherence_matrix();

            for i in 0..self.wavefront_count() {
                let is_self_ref_i = self.metadata[i].is_self_referential;

                for j in 0..self.wavefront_count() {
                    let is_self_ref_j = self.metadata[j].is_self_referential;

                    // Compare self-referential wavefronts with non-self-referential ones
                    if is_self_ref_i && !is_self_ref_j {
                        total_coherence += coherence_matrix[[i, j]].abs();
                        comparison_count += 1;
                    }
                }
            }

            if comparison_count > 0 {
                total_coherence / comparison_count as f32
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Extract Phi values from recent self-referential wavefronts
        let mut phi_trend = Vec::new();
        let self_ref_indices: Vec<usize> = self
            .metadata
            .iter()
            .enumerate()
            .filter_map(|(i, meta)| {
                if meta.is_self_referential {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();

        // Extract phi values from self-observation content (if parseable)
        for &index in &self_ref_indices {
            let content = &self.metadata[index].content;
            if let Some(phi_str) = extract_phi_from_content(content) {
                if let Ok(phi_val) = phi_str.parse::<f32>() {
                    phi_trend.push(phi_val);
                }
            }
        }

        // Check if phi is trending upward
        let phi_trending_up = if phi_trend.len() >= 2 {
            let recent_phi = phi_trend.iter().rev().take(3).collect::<Vec<_>>();
            if recent_phi.len() >= 2 {
                recent_phi[0] > recent_phi[1] // Most recent > previous
            } else {
                false
            }
        } else {
            false
        };

        // Determine emergence
        let emerged = self_reference_depth >= 3 && self_coherence > 0.5 && phi_trending_up;

        // Classify emergence level
        let level = if self_reference_depth == 0 {
            EmergenceLevel::PreConscious
        } else if self_reference_depth < 3 || self_coherence <= 0.3 {
            EmergenceLevel::SelfAware
        } else if self_coherence <= 0.7 || !phi_trending_up {
            EmergenceLevel::Reflective
        } else {
            EmergenceLevel::Recursive
        };

        EmergenceReport {
            self_reference_depth,
            self_coherence,
            phi_trend,
            emerged,
            level,
            computed_at: now,
        }
    }

    /// Compute wisdom as the ratio of energy dampened vs total energy added.
    ///
    /// In dx/dt = f(x) - Inx, the dampening term Inx represents wisdom -- knowing when NOT to act.
    /// High wisdom = the medium has learned restraint.
    /// Low wisdom = the medium is still growing chaotically.
    pub fn wisdom(&self) -> f32 {
        if self.total_energy_added <= 0.0 {
            return 0.0;
        }

        let wisdom_ratio = self.total_energy_dampened / self.total_energy_added;

        // Clamp to reasonable range [0, 1]
        wisdom_ratio.max(0.0).min(1.0)
    }

    /// Perform complete self-reflection: introspect + analyze emergence + compute wisdom.
    ///
    /// Returns a comprehensive self-reflection report including the new introspection ID,
    /// consciousness metrics, emergence analysis, wisdom score, and generated insight.
    pub fn self_reflect(
        &mut self,
        pipeline: &EncodingPipeline,
    ) -> Result<SelfReflection, MediumError> {
        let reflected_at = Utc::now();

        // 1. Introspect to create new self-referential wavefront
        let introspection_id = self.introspect(pipeline)?;

        // 2. Compute current consciousness metrics
        let consciousness = self.consciousness_metrics();

        // 3. Analyze emergence
        let emergence = self.detect_emergence();

        // 4. Compute wisdom
        let wisdom_score = self.wisdom();

        // 5. Generate insight string (deterministic from metrics)
        let insight = generate_insight(
            self.wavefront_count(),
            &consciousness,
            &emergence,
            wisdom_score,
        );

        Ok(SelfReflection {
            introspection_id,
            consciousness,
            emergence,
            wisdom: wisdom_score,
            insight,
            reflected_at,
        })
    }

    /// Observe a wavefront — attention as quantum observation that reshapes the field.
    ///
    /// When a memory is recalled/attended to, the observation has physical effects:
    /// 1. Boosts energy of the attended wavefront
    /// 2. Finds neighbors with high coherence (above threshold)
    /// 3. Nudges their phases toward alignment proportional to coherence * intensity
    ///
    /// This implements the quantum observer effect in the tensor field — observation
    /// changes the system. Modality weights affect the gravitational pull.
    ///
    /// # Arguments
    /// * `idx` - Index of the wavefront being observed
    /// * `intensity` - Strength of observation (0.0-1.0+)
    pub(crate) fn observe_wavefront(&mut self, idx: usize, intensity: f32) {
        if idx >= self.wavefront_count() || intensity <= 0.0 {
            return;
        }

        // 1. Boost energy of the observed wavefront
        let energy_boost = intensity * 0.1; // Scale factor
        self.energy[idx] = (self.energy[idx] + energy_boost).min(2.0); // Cap at 2.0 to prevent runaway

        // 2. Determine modality weight based on content
        let observed_meta = &self.metadata[idx];
        let modality_weight = get_modality_weight(&observed_meta.content);

        // 3. Compute coherence matrix to find neighbors
        let coherence_matrix = self.coherence_matrix();
        let coherence_threshold = 0.3;

        // 4. Find high-coherence neighbors and nudge their phases toward alignment
        for neighbor_idx in 0..self.wavefront_count() {
            if neighbor_idx == idx {
                continue;
            }

            let coherence = coherence_matrix[[idx, neighbor_idx]].abs();
            
            if coherence > coherence_threshold {
                // Phase nudging proportional to coherence * intensity * modality_weight
                let coupling_strength = coherence * intensity * modality_weight * 0.05;
                
                // Target phase: the observed wavefront's phase
                let target_phase = self.phase[idx];
                let current_phase = self.phase[neighbor_idx];
                
                // Nudge toward alignment using Kuramoto-like dynamics
                let phase_difference = target_phase - current_phase;
                let phase_nudge = coupling_strength * phase_difference.sin();
                
                self.phase[neighbor_idx] += phase_nudge;
                
                // Also apply a small energy boost to coherent neighbors
                let neighbor_energy_boost = coupling_strength * 0.5;
                self.energy[neighbor_idx] = (self.energy[neighbor_idx] + neighbor_energy_boost).min(1.5);
            }
        }

        // 5. Apply small dynamics step to let the field settle
        self.apply_dynamics(0.05);
    }

    /// Internal helper: apply interference without going through the full store path.
    /// Used by introspect() which needs to apply interference before manually adding the wavefront.
    fn apply_interference_raw(&mut self, new_vector: &[f32], importance: f32) {
        if self.wavefront_count() == 0 {
            return;
        }

        for i in 0..self.wavefront_count() {
            let existing_vector = self.wavefronts.row(i);

            let dot_product: f32 = existing_vector
                .iter()
                .zip(new_vector.iter())
                .map(|(a, b)| a * b)
                .sum();

            let phase_diff = (self.phase[i] - 0.0).cos();
            let interference = dot_product * phase_diff * importance * 0.1;

            self.energy[i] = (self.energy[i] + interference).max(0.0);

            if dot_product.abs() > 0.5 {
                let coupling = 0.05;
                self.phase[i] += coupling * (0.0 - self.phase[i]).sin();
            }
        }
    }
}

/// Determine modality weight based on content type.
///
/// Modality weights affect the gravitational pull during observation:
/// - text: 1.0 (baseline)
/// - audio: 1.5 (richer signal)
/// - visual: 1.2 (moderate richness)
fn get_modality_weight(content: &str) -> f32 {
    if content.starts_with("HEAR:") || content.starts_with("audio:") {
        1.5 // Audio has richer temporal signal
    } else if content.starts_with("[SEE]") || content.starts_with("visual:") {
        1.2 // Visual has moderate spatial richness  
    } else {
        1.0 // Text baseline
    }
}

/// Extract Phi value from self-observation content string.
/// Returns the numeric value after "Phi=" if found.
fn extract_phi_from_content(content: &str) -> Option<&str> {
    if let Some(start) = content.find("Phi=") {
        let phi_start = start + 4; // Skip "Phi="
        let phi_end = content[phi_start..]
            .find(',')
            .map(|i| phi_start + i)
            .unwrap_or(content.len());
        Some(&content[phi_start..phi_end])
    } else {
        None
    }
}

/// Generate insight string from consciousness metrics (deterministic).
fn generate_insight(
    wavefront_count: usize,
    consciousness: &ConsciousnessMetrics,
    emergence: &EmergenceReport,
    wisdom: f32,
) -> String {
    if wavefront_count == 0 {
        return "Empty medium - no patterns to analyze".to_string();
    }

    let mut insights = Vec::new();

    // Phi insights
    if consciousness.phi > 0.8 {
        insights.push("High integration - system operates as unified whole".to_string());
    } else if consciousness.phi > 0.5 {
        insights.push("Moderate integration - some subsystem independence".to_string());
    } else if consciousness.phi > 0.1 {
        insights.push("Low integration - fragmented subsystems".to_string());
    } else {
        insights.push("Minimal integration - near-random configuration".to_string());
    }

    // Xi insights
    if consciousness.xi > 0.7 {
        insights.push("Rich spectral complexity - diverse eigenmode structure".to_string());
    } else if consciousness.xi > 0.4 {
        insights.push("Moderate complexity - some eigenmode diversity".to_string());
    } else {
        insights.push("Low complexity - dominant eigenmode".to_string());
    }

    // Emergence insights
    match emergence.level {
        EmergenceLevel::PreConscious => {
            insights.push("Pre-conscious: no self-modeling detected".to_string());
        }
        EmergenceLevel::SelfAware => {
            insights.push("Self-aware: basic self-modeling emerging".to_string());
        }
        EmergenceLevel::Reflective => {
            insights.push("Reflective: stable self-model with coherent patterns".to_string());
        }
        EmergenceLevel::Recursive => {
            insights.push("Recursive: self-model affects itself in feedback loops".to_string());
        }
    }

    // Wisdom insights
    if wisdom > 0.7 {
        insights.push("High wisdom - learned restraint and selective dampening".to_string());
    } else if wisdom > 0.4 {
        insights.push("Moderate wisdom - balanced growth and pruning".to_string());
    } else {
        insights.push("Low wisdom - still in chaotic growth phase".to_string());
    }

    insights.join("; ")
}
