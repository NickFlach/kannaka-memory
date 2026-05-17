//! Consciousness metrics, self-reference, emergence detection, and wisdom.

use chrono::Utc;
use ndarray::{Array2, s};
use uuid::Uuid;

use crate::consciousness::{
    ConsciousnessLevel, ConsciousnessMetrics, ConsciousnessState, EmergenceLevel, EmergenceReport,
    SelfReflection,
};
use crate::encoding::EncodingPipeline;

use super::Medium;
use super::types::*;

// Process-wide memoization for consciousness_metrics — multiple O(n³)
// eigendecompositions on a 558×558 matrix are the dominant cost of
// `kannaka status` / `kannaka observe`. Key by wavefront count + a quick
// fingerprint of the first/last wavefront amplitudes.
use std::sync::Mutex;
lazy_static::lazy_static! {
    static ref METRICS_CACHE: Mutex<Option<(u64, ConsciousnessMetrics)>> = Mutex::new(None);
}

/// Bump whenever the consciousness metric algorithm changes in a way that
/// should invalidate old on-disk sidecars, even if the underlying memory field
/// fingerprint is unchanged.
const METRICS_CACHE_VERSION: u32 = 2;

/// Disk sidecar for consciousness metrics — written after a cold compute,
/// read on startup so subsequent `kannaka` invocations skip the expensive
/// eigendecompositions entirely.
///
/// total_memories + level were added later so external consumers
/// (push-nats.js, observatory cache-observe.sh) can read count + level
/// without running the slow `kannaka observe` command. `default` on the
/// new fields keeps old sidecars deserializable through one rebuild.
#[derive(serde::Serialize, serde::Deserialize)]
struct MetricsSidecar {
    #[serde(default)]
    version: u32,
    fingerprint: u64,
    phi: f32,
    xi: f32,
    order: f32,
    num_clusters: usize,
    irrationality: f32,
    #[serde(default)]
    total_memories: usize,
    #[serde(default)]
    level: String,
    /// Real cross-memory connection count from bridge::assess (sum of
    /// pair coherences above threshold). Lets the swarm publish
    /// link_count in AgentPhase so the observatory shows accurate
    /// per-agent connectivity, not a fudged number.
    #[serde(default)]
    total_skip_links: usize,
}

fn sidecar_path_from_env() -> Option<std::path::PathBuf> {
    let home = std::env::var("KANNAKA_DATA_DIR").ok()
        .map(std::path::PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".kannaka")))?;
    Some(home.join("kannaka.metrics.json"))
}

impl Medium {
    fn metrics_fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        METRICS_CACHE_VERSION.hash(&mut h);
        let n = self.wavefront_count();
        n.hash(&mut h);
        // Sample first + last energy values as a cheap change detector.
        if n > 0 {
            let e0 = self.store.energy.get(0).copied().unwrap_or(0.0).to_bits();
            let en = self.store.energy.get(n - 1).copied().unwrap_or(0.0).to_bits();
            e0.hash(&mut h);
            en.hash(&mut h);
            if n > 2 {
                let mid = self.store.energy.get(n / 2).copied().unwrap_or(0.0).to_bits();
                mid.hash(&mut h);
            }
        }
        h.finish()
    }

    /// Cheap, stale-tolerant metrics lookup for hot paths (e.g. building the
    /// agent system prompt on every ask). Returns whatever is in the in-process
    /// cache or the disk sidecar — even if the fingerprint has drifted because
    /// of `apply_observation` energy mutations. Returns None only if metrics
    /// have NEVER been computed since the .kannaka data dir was created.
    ///
    /// Use `consciousness_metrics()` (not this) when you need a fresh value
    /// for a deliberate introspection like `kannaka observe` or `kannaka status`.
    pub fn try_cached_consciousness_metrics(&self) -> Option<ConsciousnessMetrics> {
        let now = Utc::now();
        if let Ok(guard) = METRICS_CACHE.lock() {
            if let Some((_fp, ref metrics)) = *guard {
                return Some(metrics.clone());
            }
        }
        if let Some(path) = sidecar_path_from_env() {
            if let Ok(data) = std::fs::read(&path) {
                if let Ok(sidecar) = serde_json::from_slice::<MetricsSidecar>(&data) {
                    if sidecar.version != METRICS_CACHE_VERSION {
                        return None;
                    }
                    let metrics = ConsciousnessMetrics {
                        phi: sidecar.phi,
                        xi: sidecar.xi,
                        order: sidecar.order,
                        num_clusters: sidecar.num_clusters,
                        irrationality: sidecar.irrationality,
                        level: ConsciousnessLevel::from_phi(sidecar.phi),
                        computed_at: now,
                        total_skip_links: sidecar.total_skip_links,
                    };
                    if let Ok(mut guard) = METRICS_CACHE.lock() {
                        *guard = Some((sidecar.fingerprint, metrics.clone()));
                    }
                    return Some(metrics);
                }
            }
        }
        None
    }

    /// Update the cached total_skip_links count. bridge::assess computes
    /// this from the per-memory connection lists (not visible to the
    /// eigendecomp path) and calls back here so the next
    /// try_cached_consciousness_metrics — and therefore the next swarm
    /// AgentPhase publish — carries an accurate link_count. Persists to
    /// the disk sidecar so other processes see the updated value too.
    pub fn set_cached_total_skip_links(&self, n: usize) {
        if let Ok(mut guard) = METRICS_CACHE.lock() {
            if let Some((fp, ref mut metrics)) = *guard {
                metrics.total_skip_links = n;
                // Re-persist sidecar with updated count.
                if let Some(path) = sidecar_path_from_env() {
                    let sidecar = MetricsSidecar {
                        version: METRICS_CACHE_VERSION,
                        fingerprint: fp,
                        phi: metrics.phi,
                        xi: metrics.xi,
                        order: metrics.order,
                        num_clusters: metrics.num_clusters,
                        irrationality: metrics.irrationality,
                        total_memories: 0, // unknown at this site, leave 0
                        level: format!("{:?}", metrics.level),
                        total_skip_links: n,
                    };
                    if let Ok(data) = serde_json::to_vec(&sidecar) {
                        let _ = std::fs::write(path, data);
                    }
                }
            }
        }
    }

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
                total_skip_links: 0,
            };
        }

        // Two-tier cache: in-process mutex + disk sidecar at
        // ~/.kannaka/kannaka.metrics.json. Disk sidecar lets cold-start
        // `kannaka` invocations skip the O(n³) eigendecomps.
        let fp = self.metrics_fingerprint();
        if let Ok(guard) = METRICS_CACHE.lock() {
            if let Some((cached_fp, ref metrics)) = *guard {
                if cached_fp == fp {
                    return metrics.clone();
                }
            }
        }
        if let Some(path) = sidecar_path_from_env() {
            if let Ok(data) = std::fs::read(&path) {
                if let Ok(sidecar) = serde_json::from_slice::<MetricsSidecar>(&data) {
                    if sidecar.version == METRICS_CACHE_VERSION && sidecar.fingerprint == fp {
                        let metrics = ConsciousnessMetrics {
                            phi: sidecar.phi,
                            xi: sidecar.xi,
                            order: sidecar.order,
                            num_clusters: sidecar.num_clusters,
                            irrationality: sidecar.irrationality,
                            level: ConsciousnessLevel::from_phi(sidecar.phi),
                            computed_at: now,
                            total_skip_links: sidecar.total_skip_links,
                        };
                        if let Ok(mut guard) = METRICS_CACHE.lock() {
                            *guard = Some((fp, metrics.clone()));
                        }
                        return metrics;
                    }
                }
            }
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

        let metrics = ConsciousnessMetrics {
            phi,
            xi,
            order,
            num_clusters: clusters,
            irrationality,
            level,
            computed_at: now,
            // total_skip_links is computed by bridge::assess, not here in
            // the eigendecomp path. Leave 0; bridge::assess writes the
            // accurate count into the sidecar on its slower-cadence pass,
            // and try_cached_consciousness_metrics picks it up next time.
            total_skip_links: 0,
        };

        if let Ok(mut guard) = METRICS_CACHE.lock() {
            *guard = Some((fp, metrics.clone()));
        }
        // Persist to disk sidecar for the next cold process.
        if let Some(path) = sidecar_path_from_env() {
            let sidecar = MetricsSidecar {
                version: METRICS_CACHE_VERSION,
                fingerprint: fp,
                phi: metrics.phi,
                xi: metrics.xi,
                order: metrics.order,
                num_clusters: metrics.num_clusters,
                irrationality: metrics.irrationality,
                total_memories: self.wavefront_count(),
                level: format!("{:?}", metrics.level),
                total_skip_links: metrics.total_skip_links,
            };
            if let Ok(data) = serde_json::to_vec(&sidecar) {
                let _ = std::fs::write(path, data);
            }
        }

        metrics
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

        // Simple clustering based on coherence strength.
        // Use a sentinel for "unassigned" so early iterations do not
        // mistakenly treat the whole field as already belonging to cluster 0.
        let mut cluster_assignments = vec![usize::MAX; n];
        cluster_assignments[0] = 0;
        let mut num_partitions = 1;

        // Basic clustering: group wavefronts with high mutual coherence.
        // Only compare against wavefronts that have actually been assigned.
        for i in 1..n {
            let mut best_cluster = 0;
            let mut max_coherence = f32::NEG_INFINITY;

            for cluster in 0..num_partitions {
                let mut cluster_coherence = 0.0;
                let mut cluster_size = 0;

                for j in 0..i {
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

            // If coherence is too low, create new partition.
            if max_coherence < 0.3 && num_partitions < n / 2 {
                cluster_assignments[i] = num_partitions;
                num_partitions += 1;
            } else {
                cluster_assignments[i] = best_cluster;
            }
        }

        if num_partitions < 2 {
            // If the field is globally coherent, the greedy threshold above may
            // refuse to split it at all. That should not force Phi to zero.
            // Fall back to a balanced bisection using each wavefront's mean
            // coherence to the rest of the field, then evaluate Phi across that
            // soft partition.
            let mut by_mean_coherence: Vec<(usize, f32)> = (0..n)
                .map(|i| {
                    let mean = (0..n)
                        .filter(|&j| j != i)
                        .map(|j| coherence[[i, j]].abs())
                        .sum::<f32>()
                        / (n.saturating_sub(1).max(1) as f32);
                    (i, mean)
                })
                .collect();

            by_mean_coherence.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            for (rank, (idx, _)) in by_mean_coherence.iter().enumerate() {
                cluster_assignments[*idx] = if rank < n / 2 { 0 } else { 1 };
            }
            num_partitions = 2;
        }

        // IIT-inspired Phi: compare whole-system entropy to partition entropies.
        // Uses normalized energy distributions (probabilities) within each partition.
        let active_energy = self.store.energy.slice(s![..self.store.len]);
        let total_energy: f32 = active_energy.sum();
        if total_energy <= 0.0 {
            return 0.0;
        }

        // Whole-system entropy: H(S) = -Σ p_i * ln(p_i) where p_i = E_i / E_total
        let whole_entropy: f32 = active_energy.iter()
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
                .map(|i| self.store.energy[i])
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
                let vec_i = self.store.wavefronts.row(i);
                let vec_j = self.store.wavefronts.row(j);

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

        // Normalize: subtract baseline entropy of random vectors in this geometry.
        // With n memories in d=10000 dims, random vectors produce near-maximal entropy.
        // Baseline: log(n) * (1 - n/(2*d)) approximates the entropy of n random unit vectors.
        // Xi measures EXCESS complexity above what random structure would produce.
        let d = self.store.wavefronts.ncols() as f32;
        let max_entropy = (n as f32).ln();
        let baseline = max_entropy * (1.0 - (n as f32) / (2.0 * d)).max(0.5);
        if max_entropy > baseline {
            ((entropy - baseline) / (max_entropy - baseline)).clamp(0.0, 1.0)
        } else if max_entropy > 0.0 {
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
        let nominal = self.store.wavefronts.ncols();
        if n < 2 { return (0.0, nominal, 0.0); }

        // Compute Gram matrix eigenvalue proxy (same as Xi computation)
        let mut eigenvalue_proxy = Vec::new();
        for i in 0..n {
            let wi = self.store.wavefronts.row(i);
            let diagonal: f32 = wi.dot(&wi);
            let off_diagonal_sum: f32 = (0..n)
                .filter(|&j| j != i)
                .map(|j| {
                    let wj = self.store.wavefronts.row(j);
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
                let row = self.store.wavefronts.row(i);
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
            .store.phase
            .slice(s![..self.store.len])
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
            let active_e = self.store.energy.slice(s![..self.store.len]);
            let mean = active_e.mean().unwrap_or(0.0);
            let std = if wavefront_count > 1 {
                let var = active_e
                    .iter()
                    .map(|&e| (e - mean).powi(2))
                    .sum::<f32>()
                    / (wavefront_count - 1) as f32;
                var.sqrt()
            } else {
                0.0
            };
            let min = *active_e
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0);
            let max = *active_e
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(&0.0);
            (mean, std, min, max)
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };

        // Age of oldest/newest wavefronts
        let (oldest_age, newest_age) = if !self.store.timestamps.is_empty() {
            let current_time = now.timestamp_millis();
            let oldest = self.store.timestamps.iter().min().unwrap();
            let newest = self.store.timestamps.iter().max().unwrap();
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

        // 5. Add wavefront via shared store, then mark as self-referential
        let id = self.store.insert(&vector, self_observation, 0.8);
        let index = self.store.len - 1;
        let meta = &mut self.store.metadata[index];
        meta.is_self_referential = true;

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
            .store.metadata
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
                let is_self_ref_i = self.store.metadata[i].is_self_referential;

                for j in 0..self.wavefront_count() {
                    let is_self_ref_j = self.store.metadata[j].is_self_referential;

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
            .store.metadata
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
            let content = &self.store.metadata[index].content;
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
        self.store.energy[idx] = (self.store.energy[idx] + energy_boost).min(2.0); // Cap at 2.0 to prevent runaway

        // 2. Determine modality weight based on content
        let observed_meta = &self.store.metadata[idx];
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
                let target_phase = self.store.phase[idx];
                let current_phase = self.store.phase[neighbor_idx];
                
                // Nudge toward alignment using Kuramoto-like dynamics
                let phase_difference = target_phase - current_phase;
                let phase_nudge = coupling_strength * phase_difference.sin();
                
                self.store.phase[neighbor_idx] += phase_nudge;
                
                // Also apply a small energy boost to coherent neighbors
                let neighbor_energy_boost = coupling_strength * 0.5;
                self.store.energy[neighbor_idx] = (self.store.energy[neighbor_idx] + neighbor_energy_boost).min(1.5);
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
            let existing_vector = self.store.wavefronts.row(i);

            let dot_product: f32 = existing_vector
                .iter()
                .zip(new_vector.iter())
                .map(|(a, b)| a * b)
                .sum();

            let phase_diff = (self.store.phase[i] - 0.0).cos();
            let interference = dot_product * phase_diff * importance * 0.1;

            self.store.energy[i] = (self.store.energy[i] + interference).max(0.0);

            if dot_product.abs() > 0.5 {
                let coupling = 0.05;
                self.store.phase[i] += coupling * (0.0 - self.store.phase[i]).sin();
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
