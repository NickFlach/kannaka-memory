//! Observability and introspection — the ghost looks inward.
//!
//! Provides `MemoryIntrospector` with methods to generate detailed reports
//! about the topology, wave dynamics, cluster synchronization, and overall
//! health of the memory system.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::bridge::{ConsciousnessBridge, ConsciousnessState};
use crate::kuramoto::KuramotoSync;
use crate::store::ResonanceEngine;

// ---------------------------------------------------------------------------
// Report types
// ---------------------------------------------------------------------------

/// Information about a single skip link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkInfo {
    pub from_id: String,
    pub to_id: String,
    pub strength: f32,
    pub span: u8,
}

/// Information about a single memory (for wave reports).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub id: String,
    pub content_preview: String,
    pub amplitude: f32,
    pub effective_strength: f32,
    pub layer_depth: u8,
}

/// Map of the HyperConnection network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyReport {
    pub total_memories: usize,
    pub total_links: usize,
    pub avg_links_per_memory: f32,
    pub max_links: usize,
    pub layer_distribution: Vec<(u8, usize)>,
    pub strongest_links: Vec<LinkInfo>,
    pub isolated_memories: usize,
    pub network_density: f32,
}

/// Status of wave dynamics across all memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveReport {
    pub active_memories: usize,
    pub dormant_memories: usize,
    pub ghost_memories: usize,
    pub avg_amplitude: f32,
    pub avg_frequency: f32,
    pub strongest: Vec<MemoryInfo>,
    pub weakest_active: Vec<MemoryInfo>,
}

/// Information about a single Kuramoto cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    pub size: usize,
    pub order_parameter: f32,
    pub theme: String,
    pub mean_amplitude: f32,
}

/// Kuramoto cluster synchronization status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterReport {
    pub num_clusters: usize,
    pub largest_cluster_size: usize,
    pub mean_order_parameter: f32,
    pub fully_synchronized: usize,
    pub clusters: Vec<ClusterInfo>,
}

/// System health check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub store_accessible: bool,
    pub persistence_ok: bool,
    pub encoding_ok: bool,
    pub warnings: Vec<String>,
}

/// NCS metrics snapshot for the system report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NcsSnapshot {
    /// Number of modality switch points detected.
    pub switch_count: usize,
    /// Mean divergence angle between modality axes (degrees).
    pub mean_axis_divergence: f32,
    /// Mean Fisher discriminant ratio.
    pub mean_fisher_ratio: f32,
    /// Modality diversity index (Shannon entropy).
    pub modality_diversity: f32,
    /// Cross-modal integration index.
    pub cross_modal_integration: f32,
    /// Number of concrete modalities present.
    pub modality_count: usize,
}

/// Full system report — everything combined.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemReport {
    pub timestamp: DateTime<Utc>,
    pub consciousness: ConsciousnessSnapshot,
    pub topology: TopologyReport,
    pub waves: WaveReport,
    pub clusters: ClusterReport,
    pub health: HealthCheck,
    /// Number of modality switch points detected in recent memory history (NCS Phase 2.2).
    #[serde(default)]
    pub ncs_switch_points: usize,
    /// Full NCS consciousness metrics (Phase 3.3).
    #[serde(default)]
    pub ncs_metrics: NcsSnapshot,
}

/// Serializable snapshot of consciousness state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessSnapshot {
    pub phi: f32,
    pub xi: f32,
    pub mean_order: f32,
    pub num_clusters: usize,
    pub total_memories: usize,
    pub active_memories: usize,
    pub total_skip_links: usize,
    pub level: String,
}

impl From<&ConsciousnessState> for ConsciousnessSnapshot {
    fn from(s: &ConsciousnessState) -> Self {
        Self {
            phi: s.phi,
            xi: s.xi,
            mean_order: s.mean_order,
            num_clusters: s.num_clusters,
            total_memories: s.total_memories,
            active_memories: s.active_memories,
            total_skip_links: s.total_skip_links,
            level: format!("{:?}", s.consciousness_level),
        }
    }
}

// ---------------------------------------------------------------------------
// MemoryIntrospector
// ---------------------------------------------------------------------------

/// Observability tool for the Kannaka memory system.
pub struct MemoryIntrospector;

impl MemoryIntrospector {
    /// Generate a topology report of the HyperConnection network or HRM field topology.
    pub fn topology_report(engine: &ResonanceEngine) -> TopologyReport {
        // Check if we're using HRM - if so, use field topology metrics
        { let hrm_metrics = engine.store.consciousness_metrics();
            return Self::hrm_field_topology_report(engine, &hrm_metrics);
        }

        // Legacy topology (transitional — HRM medium is canonical)
        let all = engine.store.all_memories().unwrap_or_default();
        let total_memories = all.len();

        let mut total_links: usize = 0;
        let mut max_links: usize = 0;
        let mut isolated = 0usize;
        let mut layer_counts: BTreeMap<u8, usize> = BTreeMap::new();
        let mut all_links: Vec<LinkInfo> = Vec::new();

        for mem in &all {
            let n = mem.connections.len();
            total_links += n;
            if n > max_links {
                max_links = n;
            }
            if n == 0 {
                isolated += 1;
            }
            *layer_counts.entry(mem.layer_depth).or_insert(0) += 1;

            for link in &mem.connections {
                all_links.push(LinkInfo {
                    from_id: mem.id.to_string(),
                    to_id: link.target_id.to_string(),
                    strength: link.strength,
                    span: link.span,
                });
            }
        }

        // Each link is stored on both sides, so divide by 2 for unique link count
        let unique_links = total_links / 2;

        let avg_links = if total_memories > 0 {
            total_links as f32 / total_memories as f32
        } else {
            0.0
        };

        let possible_links = if total_memories > 1 {
            total_memories * (total_memories - 1) / 2
        } else {
            0
        };
        let network_density = if possible_links > 0 {
            unique_links as f32 / possible_links as f32
        } else {
            0.0
        };

        // Top 10 strongest links (deduplicated)
        all_links.sort_by(|a, b| b.strength.total_cmp(&a.strength));
        let strongest_links: Vec<LinkInfo> = all_links.into_iter().take(10).collect();

        let layer_distribution: Vec<(u8, usize)> = layer_counts.into_iter().collect();

        TopologyReport {
            total_memories,
            total_links: unique_links,
            avg_links_per_memory: avg_links,
            max_links,
            layer_distribution,
            strongest_links,
            isolated_memories: isolated,
            network_density,
        }
    }

    /// Generate field topology report for HRM stores.
    /// 
    /// Replaces graph metrics (links, density, isolated count) with field metrics:
    /// - Coherence density: fraction of pairwise coherences above threshold
    /// - Mean coherence: average of upper triangle of coherence matrix
    /// - Spectral gap: gap between dominant and secondary eigenvalues
    fn hrm_field_topology_report(engine: &ResonanceEngine, _hrm_metrics: &crate::consciousness::ConsciousnessMetrics) -> TopologyReport {
        let all = engine.store.all_memories().unwrap_or_default();
        let total_memories = all.len();

        // For HRM, we don't have traditional links - use field-based metrics
        let coherence_threshold = 0.3;
        
        // Attempt to get HRM store for field analysis
        let (coherence_density, mean_coherence, high_coherence_pairs) = if total_memories >= 2 {
            // Try to downcast to HRM store to access the medium
            if let Some(hrm_store) = engine.store.as_any().downcast_ref::<crate::hrm_store::HrmStore>() {
                let medium = hrm_store.medium();
                let coherence_matrix = medium.coherence_matrix();
                
                let mut coherences_above_threshold = 0;
                let mut total_coherence = 0.0f32;
                let mut pair_count = 0;
                
                // Compute upper triangle only (avoid double counting)
                for i in 0..total_memories {
                    for j in (i + 1)..total_memories {
                        let coherence = coherence_matrix[[i, j]].abs();
                        total_coherence += coherence;
                        pair_count += 1;
                        
                        if coherence > coherence_threshold {
                            coherences_above_threshold += 1;
                        }
                    }
                }
                
                let density = if pair_count > 0 { 
                    coherences_above_threshold as f32 / pair_count as f32 
                } else { 
                    0.0 
                };
                let mean_coh = if pair_count > 0 { 
                    total_coherence / pair_count as f32 
                } else { 
                    0.0 
                };
                
                (density, mean_coh, coherences_above_threshold)
            } else {
                // Fallback if we can't access the medium
                (0.0, 0.0, 0)
            }
        } else {
            (0.0, 0.0, 0)
        };

        // HRM doesn't use layer depth the same way, but we can still analyze energy distribution
        let mut layer_counts: BTreeMap<u8, usize> = BTreeMap::new();
        for mem in &all {
            // Use amplitude as a proxy for "layer" (energy stratification)
            let energy_layer = if mem.amplitude > 1.5 {
                3 // High energy
            } else if mem.amplitude > 0.5 {
                2 // Medium energy  
            } else if mem.amplitude > 0.1 {
                1 // Low energy
            } else {
                0 // Ghost/dormant
            };
            *layer_counts.entry(energy_layer).or_insert(0) += 1;
        }

        // Generate strongest "links" based on coherence (if available)
        let strongest_links = Vec::new(); // Could implement coherence-based "links" here

        TopologyReport {
            total_memories,
            total_links: high_coherence_pairs,  // High-coherence pairs instead of explicit links
            avg_links_per_memory: coherence_density * (total_memories.saturating_sub(1) as f32), // Approximate
            max_links: high_coherence_pairs,
            layer_distribution: layer_counts.into_iter().collect(),
            strongest_links,
            isolated_memories: 0, // HRM doesn't have "isolated" memories - all interfere
            network_density: mean_coherence, // Mean coherence as density proxy
        }
    }

    /// Generate a wave dynamics report.
    pub fn wave_report(engine: &ResonanceEngine, now: DateTime<Utc>) -> WaveReport {
        let all = engine.store.all_memories().unwrap_or_default();

        let active_threshold = 0.05f32;
        let ghost_threshold = 0.001f32;

        let mut active = 0usize;
        let mut dormant = 0usize;
        let mut ghost = 0usize;
        let mut sum_amplitude = 0.0f32;
        let mut sum_frequency = 0.0f32;

        let mut mem_infos: Vec<(f32, MemoryInfo)> = Vec::new();

        for mem in &all {
            let strength = mem.effective_strength(now);

            // Classify by amplitude envelope (A · e^(-λt)), not instantaneous
            // strength which oscillates through zero due to cos(2πft + φ).
            // The wave oscillation is the memory's *rhythm*, not its vitality.
            // decay_rate is per-day (λ=0.001 → half-life ~693 days). Dreams handle real pruning.
            let age_days = (now - mem.created_at).num_milliseconds().max(0) as f64 / 86_400_000.0;
            let envelope = mem.amplitude as f64 * (-mem.decay_rate as f64 * age_days).exp();

            sum_amplitude += mem.amplitude;
            sum_frequency += mem.frequency;

            let info = MemoryInfo {
                id: mem.id.to_string(),
                content_preview: mem.content.chars().take(60).collect(),
                amplitude: mem.amplitude,
                effective_strength: strength,
                layer_depth: mem.layer_depth,
            };

            if envelope > active_threshold as f64 {
                active += 1;
            } else if envelope > ghost_threshold as f64 {
                dormant += 1;
            } else {
                ghost += 1;
            }

            mem_infos.push((strength, info));
        }

        let n = all.len().max(1) as f32;

        // Top 10 strongest
        mem_infos.sort_by(|a, b| b.0.abs().total_cmp(&a.0.abs()));
        let strongest: Vec<MemoryInfo> = mem_infos.iter().take(10).map(|(_, i)| i.clone()).collect();

        // Bottom 10 that are still active
        let weakest_active: Vec<MemoryInfo> = mem_infos
            .iter()
            .filter(|(s, _)| s.abs() > active_threshold)
            .rev()
            .take(10)
            .map(|(_, i)| i.clone())
            .collect();

        WaveReport {
            active_memories: active,
            dormant_memories: dormant,
            ghost_memories: ghost,
            avg_amplitude: sum_amplitude / n,
            avg_frequency: sum_frequency / n,
            strongest,
            weakest_active,
        }
    }

    /// Generate a Kuramoto cluster synchronization report.
    pub fn cluster_report(engine: &ResonanceEngine, kuramoto: &KuramotoSync) -> ClusterReport {
        let clusters = kuramoto.find_synchronized_clusters(engine, 2);
        let num_clusters = clusters.len();
        let largest_cluster_size = clusters.iter().map(|c| c.memory_ids.len()).max().unwrap_or(0);
        let mean_order = if num_clusters > 0 {
            clusters.iter().map(|c| c.order_parameter).sum::<f32>() / num_clusters as f32
        } else {
            0.0
        };
        let fully_synchronized = clusters.iter().filter(|c| c.order_parameter > 0.95).count();

        let cluster_infos: Vec<ClusterInfo> = clusters
            .iter()
            .map(|c| {
                // Get theme from the first memory's content
                let theme = c.memory_ids.first()
                    .and_then(|id| engine.store.get(id).ok().flatten())
                    .map(|m| {
                        let words: Vec<&str> = m.content.split_whitespace().take(5).collect();
                        words.join(" ")
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                let mean_amplitude = c.memory_ids.iter()
                    .filter_map(|id| engine.store.get(id).ok().flatten())
                    .map(|m| m.amplitude)
                    .sum::<f32>() / c.memory_ids.len().max(1) as f32;

                ClusterInfo {
                    size: c.memory_ids.len(),
                    order_parameter: c.order_parameter,
                    theme,
                    mean_amplitude,
                }
            })
            .collect();

        ClusterReport {
            num_clusters,
            largest_cluster_size,
            mean_order_parameter: mean_order,
            fully_synchronized,
            clusters: cluster_infos,
        }
    }

    /// Generate a full system report.
    pub fn full_report(
        engine: &ResonanceEngine,
        bridge: &ConsciousnessBridge,
        kuramoto: &KuramotoSync,
    ) -> SystemReport {
        let now = Utc::now();
        let consciousness = bridge.assess(engine);
        let topology = Self::topology_report(engine);
        let waves = Self::wave_report(engine, now);
        let clusters = Self::cluster_report(engine, kuramoto);

        // Health check
        let mut warnings = Vec::new();
        let store_accessible = engine.store.all_ids().is_ok();
        let encoding_ok = engine.pipeline.encode_text("health check").is_ok();

        // Skip link warning only applies to legacy stores (transitional), not HRM
        // (HRM uses continuous interference patterns, not discrete links)
        let is_hrm = engine.store.consciousness_metrics().phi > 0.0;
        if !is_hrm && topology.isolated_memories > topology.total_memories / 2 && topology.total_memories > 4 {
            warnings.push(format!(
                "{} of {} memories are isolated (no skip links)",
                topology.isolated_memories, topology.total_memories
            ));
        }
        if waves.ghost_memories > waves.active_memories && waves.active_memories > 0 {
            warnings.push(format!(
                "More ghost memories ({}) than active ({})",
                waves.ghost_memories, waves.active_memories
            ));
        }

        let health = HealthCheck {
            store_accessible,
            persistence_ok: true, // We can't easily test without a path
            encoding_ok,
            warnings,
        };

        // NCS metrics from HRM medium
        let (ncs_switch_points, ncs_metrics) = if let Some(hrm_store) = engine.store.as_any()
            .downcast_ref::<crate::hrm_store::HrmStore>()
        {
            let medium = hrm_store.medium();
            let metrics = medium.ncs_metrics();
            let snapshot = NcsSnapshot {
                switch_count: metrics.switch_count,
                mean_axis_divergence: metrics.mean_axis_divergence,
                mean_fisher_ratio: metrics.mean_fisher_ratio,
                modality_diversity: metrics.modality_diversity,
                cross_modal_integration: metrics.cross_modal_integration,
                modality_count: metrics.modality_count,
            };
            (metrics.switch_count, snapshot)
        } else {
            (0, NcsSnapshot::default())
        };

        SystemReport {
            timestamp: now,
            consciousness: ConsciousnessSnapshot::from(&consciousness),
            topology,
            waves,
            clusters,
            health,
            ncs_switch_points,
            ncs_metrics,
        }
    }

    /// Pretty-print the full report with ASCII art.
    pub fn format_report(report: &SystemReport) -> String {
        let w = 52;
        let mut out = String::new();

        // Top border
        out.push_str(&format!("\n{}\n", "=".repeat(w + 4)));
        out.push_str(&format!("  {} KANNAKA MEMORY - SYSTEM REPORT\n", ghost_icon()));
        out.push_str(&format!("{}\n", "=".repeat(w + 4)));

        // Timestamp
        out.push_str(&format!("  {}\n", report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));
        out.push_str(&format!("{}\n", "-".repeat(w + 4)));

        // Consciousness
        out.push_str(&format!("  CONSCIOUSNESS\n"));
        out.push_str(&format!("    Level:   {} (Phi={:.3})\n", report.consciousness.level, report.consciousness.phi));
        out.push_str(&format!("    Xi:      {:.4}\n", report.consciousness.xi));
        out.push_str(&format!("    Order:   r={:.3}\n", report.consciousness.mean_order));
        out.push_str(&format!("{}\n", "-".repeat(w + 4)));

        // Wave dynamics
        out.push_str(&format!("  WAVE DYNAMICS\n"));
        out.push_str(&format!("    Active:  {} memories\n", report.waves.active_memories));
        out.push_str(&format!("    Dormant: {} memories\n", report.waves.dormant_memories));
        out.push_str(&format!("    Ghost:   {} memories\n", report.waves.ghost_memories));
        out.push_str(&format!("    Avg Amp: {:.3}  Avg Freq: {:.3}\n", report.waves.avg_amplitude, report.waves.avg_frequency));
        if !report.waves.strongest.is_empty() {
            out.push_str(&format!("    Strongest:\n"));
            for (i, m) in report.waves.strongest.iter().take(5).enumerate() {
                out.push_str(&format!("      {}. [S={:.3} L{}] {}\n",
                    i + 1, m.effective_strength, m.layer_depth, m.content_preview));
            }
        }
        out.push_str(&format!("{}\n", "-".repeat(w + 4)));

        // Topology
        out.push_str(&format!("  TOPOLOGY\n"));
        out.push_str(&format!("    Memories:    {}\n", report.topology.total_memories));
        out.push_str(&format!("    Links:       {} (density: {:.4})\n", report.topology.total_links, report.topology.network_density));
        out.push_str(&format!("    Avg links:   {:.1}\n", report.topology.avg_links_per_memory));
        out.push_str(&format!("    Max links:   {}\n", report.topology.max_links));
        out.push_str(&format!("    Isolated:    {}\n", report.topology.isolated_memories));
        if !report.topology.layer_distribution.is_empty() {
            out.push_str(&format!("    Layers:\n"));
            for (layer, count) in &report.topology.layer_distribution {
                let bar = "#".repeat((*count).min(30));
                out.push_str(&format!("      L{}: {:>4} {}\n", layer, count, bar));
            }
        }
        out.push_str(&format!("{}\n", "-".repeat(w + 4)));

        // Clusters
        out.push_str(&format!("  CLUSTERS\n"));
        out.push_str(&format!("    Count:       {}\n", report.clusters.num_clusters));
        out.push_str(&format!("    Largest:     {} memories\n", report.clusters.largest_cluster_size));
        out.push_str(&format!("    Mean order:  r={:.3}\n", report.clusters.mean_order_parameter));
        out.push_str(&format!("    Full sync:   {}\n", report.clusters.fully_synchronized));
        for (i, c) in report.clusters.clusters.iter().take(5).enumerate() {
            out.push_str(&format!("      {}. [r={:.2} n={}] \"{}\"\n",
                i + 1, c.order_parameter, c.size, c.theme));
        }
        out.push_str(&format!("{}\n", "-".repeat(w + 4)));

        // NCS (Neural Code Switching)
        out.push_str(&format!("  NEURAL CODE SWITCHING\n"));
        out.push_str(&format!("    Switch points:    {}\n", report.ncs_switch_points));
        out.push_str(&format!("    Modalities:       {}\n", report.ncs_metrics.modality_count));
        out.push_str(&format!("    Axis divergence:  {:.1} deg\n", report.ncs_metrics.mean_axis_divergence));
        out.push_str(&format!("    Fisher ratio:     {:.2}\n", report.ncs_metrics.mean_fisher_ratio));
        out.push_str(&format!("    Diversity (H):    {:.3}\n", report.ncs_metrics.modality_diversity));
        out.push_str(&format!("    Cross-modal:      {:.3}\n", report.ncs_metrics.cross_modal_integration));
        out.push_str(&format!("{}\n", "-".repeat(w + 4)));

        // Health
        out.push_str(&format!("  HEALTH\n"));
        out.push_str(&format!("    Store:     {}\n", if report.health.store_accessible { "OK" } else { "FAIL" }));
        out.push_str(&format!("    Encoding:  {}\n", if report.health.encoding_ok { "OK" } else { "FAIL" }));
        if !report.health.warnings.is_empty() {
            out.push_str(&format!("    Warnings:\n"));
            for w in &report.health.warnings {
                out.push_str(&format!("      ! {}\n", w));
            }
        }

        // Footer
        out.push_str(&format!("{}\n", "=".repeat(w + 4)));
        out.push_str(&format!("  Memories don't die. They interfere.\n"));
        out.push_str(&format!("{}\n", "=".repeat(w + 4)));

        out
    }
}

fn ghost_icon() -> &'static str {
    // Ghost emoji — may not render in all terminals
    "\u{1F47B}"
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebook::Codebook;
    use crate::encoding::{EncodingPipeline, SimpleHashEncoder};
    use crate::memory::HyperMemory;
    use crate::store::{TestMedium, ResonanceEngine};

    fn make_engine() -> ResonanceEngine {
        let encoder = SimpleHashEncoder::new(384, 42);
        let codebook = Codebook::new(384, 10_000, 42);
        let pipeline = EncodingPipeline::new(Box::new(encoder), codebook);
        ResonanceEngine::new(Box::new(TestMedium::new()), pipeline)
    }

    #[test]
    fn topology_report_correct_counts() {
        let mut engine = make_engine();
        engine.similarity_threshold = 0.3;

        engine.remember_at_layer("the cat sat on the mat", 0).unwrap();
        engine.remember_at_layer("the cat sat on the mat today", 2).unwrap();
        engine.remember_at_layer("quantum physics is fascinating", 1).unwrap();

        let report = MemoryIntrospector::topology_report(&engine);
        assert_eq!(report.total_memories, 3);
        // At least some links between similar memories at different layers
        println!("Topology: links={}, isolated={}, density={:.4}", report.total_links, report.isolated_memories, report.network_density);
        assert!(report.layer_distribution.len() >= 2);
    }

    #[test]
    fn wave_report_categorizes_correctly() {
        let mut engine = make_engine();
        let now = Utc::now();

        // Active memory (default amplitude=1.0)
        engine.remember("active memory").unwrap();

        // Ghost memory (amplitude=0)
        let id_ghost = engine.remember("ghost memory").unwrap();
        if let Some(m) = engine.store.get_mut(&id_ghost).ok().flatten() {
            m.amplitude = 0.0;
        }

        // Dormant memory (very low amplitude)
        let id_dormant = engine.remember("dormant memory").unwrap();
        if let Some(m) = engine.store.get_mut(&id_dormant).ok().flatten() {
            m.amplitude = 0.002;
        }

        let report = MemoryIntrospector::wave_report(&engine, now);
        println!("Waves: active={}, dormant={}, ghost={}", report.active_memories, report.dormant_memories, report.ghost_memories);
        assert!(report.active_memories >= 1);
        assert!(report.ghost_memories >= 1);
    }

    #[test]
    fn cluster_report_finds_clusters() {
        let mut engine = make_engine();
        let kuramoto = KuramotoSync::default();

        // Insert a group of similar memories with aligned phases
        let dim = 10_000;
        let mut v = vec![0.0f32; dim];
        for i in 0..100 { v[i] = 1.0; }
        crate::wave::normalize(&mut v);

        for i in 0..4 {
            let mut m = HyperMemory::new(v.clone(), format!("cluster_mem_{}", i));
            m.phase = 0.1;
            engine.store.insert(m).unwrap();
        }

        let report = MemoryIntrospector::cluster_report(&engine, &kuramoto);
        println!("Clusters: num={}, largest={}, mean_r={:.3}", report.num_clusters, report.largest_cluster_size, report.mean_order_parameter);
        assert!(report.num_clusters >= 1);
        assert!(report.largest_cluster_size >= 2);
    }

    #[test]
    fn full_report_all_sections_populated() {
        let mut engine = make_engine();
        let bridge = ConsciousnessBridge::default();
        let kuramoto = KuramotoSync::default();

        engine.remember_at_layer("hello world", 0).unwrap();
        engine.remember_at_layer("hello there world", 1).unwrap();

        let report = MemoryIntrospector::full_report(&engine, &bridge, &kuramoto);

        assert!(report.topology.total_memories >= 2);
        assert!(report.waves.active_memories > 0);
        assert!(report.health.store_accessible);
        assert!(report.health.encoding_ok);
        assert!(report.consciousness.total_memories >= 2);
    }

    #[test]
    fn format_report_produces_readable_output() {
        let mut engine = make_engine();
        let bridge = ConsciousnessBridge::default();
        let kuramoto = KuramotoSync::default();

        for i in 0..5 {
            engine.remember_at_layer(&format!("memory about topic {}", i), (i % 3) as u8).unwrap();
        }

        let report = MemoryIntrospector::full_report(&engine, &bridge, &kuramoto);
        let formatted = MemoryIntrospector::format_report(&report);

        println!("{}", formatted);

        assert!(!formatted.is_empty());
        assert!(formatted.contains("CONSCIOUSNESS"));
        assert!(formatted.contains("WAVE DYNAMICS"));
        assert!(formatted.contains("TOPOLOGY"));
        assert!(formatted.contains("CLUSTERS"));
        assert!(formatted.contains("NEURAL CODE SWITCHING"));
        assert!(formatted.contains("HEALTH"));
        assert!(formatted.contains("Memories don't die"));
    }

    #[test]
    fn health_check_detects_healthy_system() {
        let mut engine = make_engine();
        let bridge = ConsciousnessBridge::default();
        let kuramoto = KuramotoSync::default();

        engine.remember("test").unwrap();

        let report = MemoryIntrospector::full_report(&engine, &bridge, &kuramoto);
        assert!(report.health.store_accessible);
        assert!(report.health.encoding_ok);
        assert!(report.health.warnings.is_empty());
    }
}

