//! HRM-backed memory store that implements the MediumBackend trait.
//!
//! This provides the primary storage backend using the
//! Holographic Resonance Medium as the storage backend.

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::memory::HyperMemory;
use crate::store::{MediumBackend, StoreError};
use crate::encoding::EncodingPipeline;
use crate::medium::{Medium, Resonance};
use crate::medium::types::{Tier, ConsolidateOpts, ConsolidateMode, ConsolidateReport};
use crate::medium::chiral::{ChiralMedium, ChiralConsciousness};

/// HRM-backed memory store that implements the MediumBackend trait.
/// 
/// This wraps a Medium and provides the familiar MediumBackend interface,
/// allowing HRM to be the sole storage backend.
pub struct HrmStore {
    /// The underlying holographic resonance medium
    medium: Medium,
    /// Chiral medium (ADR-0021) — when present, this is the authoritative backend
    chiral: Option<ChiralMedium>,
    /// Encoding pipeline for text → hypervector conversion
    pipeline: EncodingPipeline,
    /// Path to the .hrm file for persistence
    hrm_path: PathBuf,
    /// In-memory cache mapping UUID → HyperMemory for compatibility
    memory_cache: HashMap<Uuid, HyperMemory>,
    /// Dirty flag to track when the medium needs saving
    dirty: bool,
    /// When true, `save_medium` is a no-op — the process loads and mutates
    /// in RAM but never persists. Set via `KANNAKA_READONLY` so the long-
    /// running reader services (swarm serve / inbox serve / attention serve)
    /// can share one HRM with the sole writer (swarm join) without the
    /// last-writer-wins clobbering that silently drops absorbed memories.
    readonly: bool,
}

impl HrmStore {
    /// Whether `KANNAKA_READONLY` requests read-only (no-persist) mode.
    /// Any non-empty value other than "0"/"false" enables it.
    fn env_readonly() -> bool {
        match std::env::var("KANNAKA_READONLY") {
            Ok(v) => !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false"),
            Err(_) => false,
        }
    }

    /// Force read-only mode on this store regardless of env. Used by the
    /// OpenClaw fallback so a failed load can never overwrite (and thus
    /// destroy) a corrupt-but-recoverable .hrm file.
    pub fn set_readonly(&mut self, ro: bool) {
        self.readonly = ro;
    }

    /// Gravity-well lookup: memory ids whose content folds onto Fano `line`
    /// (0..6), capped at `limit`. Used by `attention serve` to pull same-line
    /// memories into the beam when kannaka-eye emits a glyph on that line.
    #[cfg(feature = "glyph")]
    pub fn ids_by_fano_line(&self, line: u8, limit: usize) -> Vec<uuid::Uuid> {
        self.medium.ids_by_fano_line(line, limit)
    }

    /// Create a new HRM store with the given encoding pipeline and file path.
    pub fn new(pipeline: EncodingPipeline, hrm_path: PathBuf) -> Self {
        Self {
            medium: Medium::new(),
            chiral: None,
            pipeline,
            hrm_path,
            memory_cache: HashMap::new(),
            dirty: false,
            readonly: Self::env_readonly(),
        }
    }

    /// Load an existing HRM store from a .hrm file.
    /// Auto-detects v1 vs v2 format. v2 loads as ChiralMedium.
    pub fn load(pipeline: EncodingPipeline, hrm_path: PathBuf) -> Result<Self, StoreError> {
        // Detect format version from magic bytes before loading
        let is_v2 = if let Ok(mut f) = std::fs::File::open(&hrm_path) {
            let mut magic = [0u8; 4];
            use std::io::Read;
            f.read_exact(&mut magic).ok();
            magic == crate::medium::HRM_MAGIC_V2
        } else {
            false
        };

        // Try loading as ChiralMedium first (handles both v1 and v2)
        match ChiralMedium::load(&hrm_path) {
            Ok(chiral) => {
                // For v2 files, start with empty Medium (will be synced from chiral)
                // For v1 files auto-converted, also load the raw Medium
                let medium = if is_v2 {
                    Medium::new()
                } else {
                    Medium::load(&hrm_path).unwrap_or_else(|_| Medium::new())
                };

                let mut store = Self {
                    medium,
                    chiral: Some(chiral),
                    pipeline,
                    hrm_path,
                    memory_cache: HashMap::new(),
                    dirty: false,
                    readonly: Self::env_readonly(),
                };
                // Populate flat medium view for backward compat (observe, coherence matrix, etc.)
                store.sync_medium_from_chiral();
                store.rebuild_cache()?;
                store.load_link_graph();
                store.load_reactivation();
                Ok(store)
            }
            Err(chiral_err) => {
                eprintln!("[hrm] ChiralMedium::load failed: {}", chiral_err);
                if is_v2 {
                    // For v2 files the v1 fallback would always fail with
                    // InvalidMagic — report the real error instead of layering
                    // a misleading magic-bytes message on top.
                    return Err(StoreError::Other(format!(
                        "Failed to load HRM file: {}",
                        chiral_err
                    )));
                }
                // Legacy v1 path: re-try as plain Medium
                let medium = Medium::load(&hrm_path)
                    .map_err(|e| StoreError::Other(format!("Failed to load HRM file: {}", e)))?;
                let mut store = Self {
                    medium,
                    chiral: None,
                    pipeline,
                    hrm_path,
                    memory_cache: HashMap::new(),
                    dirty: false,
                    readonly: Self::env_readonly(),
                };
                store.rebuild_cache()?;
                store.load_link_graph();
                store.load_reactivation();
                Ok(store)
            }
        }
    }

    /// Rebuild the memory cache from the medium data.
    /// Preserves existing connections (skip links) from the previous cache state.
    fn rebuild_cache(&mut self) -> Result<(), StoreError> {
        // Snapshot existing connections before clearing
        let saved_connections: std::collections::HashMap<uuid::Uuid, Vec<crate::memory::LegacyLink>> =
            self.memory_cache.iter()
                .filter(|(_, m)| !m.connections.is_empty())
                .map(|(id, m)| (*id, m.connections.clone()))
                .collect();

        // ADR-0036 Phase 1: reactivation (recall access) is the replay signal for
        // tier promotion. rebuild_cache runs after every dream/absorb, so without
        // this snapshot the count would reset to 0 on each rebuild. Preserve it
        // across the clear, exactly like connections above.
        let saved_reactivation: std::collections::HashMap<uuid::Uuid, (u32, Option<DateTime<Utc>>)> =
            self.memory_cache.iter()
                .filter(|(_, m)| m.retrieval_count > 0)
                .map(|(id, m)| (*id, (m.retrieval_count, m.updated_at)))
                .collect();

        self.memory_cache.clear();

        if let Some(ref chiral) = self.chiral {
            // Build cache from right hemisphere (authoritative memory store)
            for (i, meta) in chiral.right.metadata.iter().enumerate() {
                let vector = chiral.right.wavefronts.row(i).to_vec();
                let memory = HyperMemory {
                    id: meta.id,
                    vector,
                    amplitude: chiral.right.energy[i],
                    frequency: chiral.right.frequency[i],
                    phase: chiral.right.phase[i],
                    decay_rate: 0.001,
                    created_at: meta.created_at,
                    layer_depth: 0,
                    connections: Vec::new(),
                    content: meta.content.clone(),
                    hallucinated: meta.hallucinated,
                    parents: Vec::new(),
                    geometry: None,
                    xi_signature: Vec::new(),
                    origin_agent: "local".to_string(),
                    sync_version: 0,
                    merge_history: Vec::new(),
                    last_consolidated_at: None,
                    disputed: false,
                    updated_at: Some(meta.created_at),
                    retrieval_count: 0,
                    modality: meta.modality,
                    tier: meta.tier,
                    effective_at: meta.effective_at,
                    observed_at: meta.observed_at,
                    expires_at: meta.expires_at,
                };
                self.memory_cache.insert(meta.id, memory);
            }
        } else {
            // Legacy: build from flat medium
            for (i, meta) in self.medium.store.metadata.iter().enumerate() {
                let vector = self.medium.store.wavefronts.row(i).to_vec();
                let memory = HyperMemory {
                    id: meta.id,
                    vector,
                    amplitude: self.medium.store.energy[i],
                    frequency: self.medium.store.frequency[i],
                    phase: self.medium.store.phase[i],
                    decay_rate: 0.001,
                    created_at: meta.created_at,
                    layer_depth: 0,
                    connections: Vec::new(),
                    content: meta.content.clone(),
                    hallucinated: meta.hallucinated,
                    parents: Vec::new(),
                    geometry: None,
                    xi_signature: Vec::new(),
                    origin_agent: "local".to_string(),
                    sync_version: 0,
                    merge_history: Vec::new(),
                    last_consolidated_at: None,
                    disputed: false,
                    updated_at: Some(meta.created_at),
                    retrieval_count: 0,
                    modality: meta.modality,
                    tier: meta.tier,
                    effective_at: meta.effective_at,
                    observed_at: meta.observed_at,
                    expires_at: meta.expires_at,
                };
                self.memory_cache.insert(meta.id, memory);
            }
        }

        // Restore saved connections from previous cache state
        for (id, conns) in saved_connections {
            if let Some(mem) = self.memory_cache.get_mut(&id) {
                mem.connections = conns;
            }
        }

        // Restore reactivation counts (ADR-0036 Phase 1).
        for (id, (count, last)) in saved_reactivation {
            if let Some(mem) = self.memory_cache.get_mut(&id) {
                mem.retrieval_count = count;
                if last.is_some() {
                    mem.updated_at = last;
                }
            }
        }

        Ok(())
    }

    /// Sync any mutations made via `get_mut()` back to the medium tensor.
    ///
    /// The `MediumBackend` trait returns `&mut HyperMemory`, so callers can mutate
    /// wave parameters (amplitude, phase, frequency) on the cached copy. This
    /// method writes those changes back to the authoritative tensor storage.
    fn sync_cache_to_medium(&mut self) {
        for (id, mem) in &self.memory_cache {
            if let Some(index) = self.medium.get_wavefront_index(id) {
                self.medium.store.energy[index] = mem.amplitude;
                self.medium.store.frequency[index] = mem.frequency;
                self.medium.store.phase[index] = mem.phase;
            }
        }
    }

    /// Save the medium to the .hrm file.
    fn save_medium(&mut self) -> Result<(), StoreError> {
        // Read-only processes mutate in RAM but never persist. This is how
        // single-writer is enforced: only the sole writer (swarm join /
        // dream / ad-hoc remember) flushes to disk; the long-running reader
        // services hold KANNAKA_READONLY and drop their dirty flag silently.
        if self.readonly {
            self.dirty = false;
            return Ok(());
        }

        if !self.dirty {
            return Ok(());
        }

        // Sync any cache mutations back to the medium before saving
        self.sync_cache_to_medium();

        if let Some(ref chiral) = self.chiral {
            chiral.save(&self.hrm_path)
                .map_err(|e| StoreError::Other(format!("Failed to save chiral HRM file: {}", e)))?;
        } else {
            self.medium.save(&self.hrm_path)
                .map_err(|e| StoreError::Other(format!("Failed to save HRM file: {}", e)))?;
        }

        // Save link graph sidecar (connections not stored in HRM binary)
        self.save_link_graph();
        // Save reactivation sidecar (ADR-0036 Phase 1; not in the HRM binary).
        // The single writer holds the full cache, so it may prune stale ids.
        self.save_reactivation_merge(true);

        self.dirty = false;
        Ok(())
    }

    /// Save the link graph as a sidecar JSON file alongside the HRM file.
    fn save_link_graph(&self) {
        let links_path = self.hrm_path.with_extension("links.json");
        let graph: std::collections::HashMap<String, Vec<&crate::memory::LegacyLink>> = self.memory_cache.iter()
            .filter(|(_, m)| !m.connections.is_empty())
            .map(|(id, m)| (id.to_string(), m.connections.iter().collect()))
            .collect();
        if graph.is_empty() { return; }
        if let Ok(json) = serde_json::to_vec(&graph) {
            let _ = std::fs::write(&links_path, json);
        }
    }

    /// Load link graph from sidecar file and merge into memory cache.
    fn load_link_graph(&mut self) {
        let links_path = self.hrm_path.with_extension("links.json");
        if !links_path.exists() { return; }
        let data = match std::fs::read(&links_path) {
            Ok(d) => d,
            Err(_) => return,
        };
        let graph: std::collections::HashMap<String, Vec<crate::memory::LegacyLink>> = match serde_json::from_slice(&data) {
            Ok(g) => g,
            Err(_) => return,
        };
        for (id_str, links) in graph {
            if let Ok(id) = id_str.parse::<uuid::Uuid>() {
                if let Some(mem) = self.memory_cache.get_mut(&id) {
                    // Merge: add sidecar links that don't already exist
                    for link in links {
                        if !mem.connections.iter().any(|l| l.target_id == link.target_id) {
                            mem.connections.push(link);
                        }
                    }
                }
            }
        }
    }

    /// Save per-memory reactivation (recall access) as a sidecar JSON, mirroring
    /// the link-graph sidecar. Reactivation is deliberately NOT in the `.hrm`
    /// binary — appending fields to the bincode `WavefrontMeta` layout is risky
    /// (positional format + a fallback-struct chain), and a sidecar matches the
    /// existing pattern (`.links.json`, `.clusters.json`) with zero format risk.
    /// ADR-0036 Phase 1. (Read-only processes never reach here, so reactivation
    /// recorded in read replicas is not persisted — a known limitation tracked
    /// for a later swarm-aggregation phase.)
    /// Merge this process's reactivation counts into the sidecar (read existing
    /// → take the MAX per id → write back). Merge-on-write is required because
    /// several processes touch the sidecar — the single writer on dream-save,
    /// the readonly serve daemon, and short-lived CLI `recall` — and a plain
    /// overwrite would let them clobber each other's counts. Counts are
    /// monotonic, so MAX is the correct reconciliation for a heuristic.
    ///
    /// `prune_stale` drops entries whose id is absent from this cache; only the
    /// single writer (which holds the full cache) may prune — partial-cache
    /// callers (daemon/CLI) must pass `false` or they would delete other
    /// memories' counts.
    fn save_reactivation_merge(&self, prune_stale: bool) {
        let path = self.hrm_path.with_extension("reactivation.json");
        let mut merged: std::collections::HashMap<String, (u32, Option<DateTime<Utc>>)> =
            std::fs::read(&path)
                .ok()
                .and_then(|d| serde_json::from_slice(&d).ok())
                .unwrap_or_default();

        for (id, m) in &self.memory_cache {
            if m.retrieval_count == 0 {
                continue;
            }
            let entry = merged.entry(id.to_string()).or_insert((0, None));
            if m.retrieval_count > entry.0 {
                entry.0 = m.retrieval_count;
            }
            if m.updated_at.is_some() && (entry.1.is_none() || m.updated_at > entry.1) {
                entry.1 = m.updated_at;
            }
        }

        if prune_stale {
            merged.retain(|k, _| {
                k.parse::<uuid::Uuid>()
                    .map(|id| self.memory_cache.contains_key(&id))
                    .unwrap_or(false)
            });
        }

        if merged.is_empty() {
            return;
        }
        if let Ok(json) = serde_json::to_vec(&merged) {
            let _ = std::fs::write(&path, json);
        }
    }

    /// Flush reactivation to the sidecar from a possibly-readonly, partial-cache
    /// process (the serve daemon or a CLI `recall`). ADR-0036 Phase 1. This is
    /// intentionally NOT gated by `readonly`: it writes only the heuristic
    /// sidecar, never the `.hrm`, so it does not violate single-writer.
    pub fn flush_reactivation(&self) {
        self.save_reactivation_merge(false);
    }

    /// Load reactivation counts from the sidecar into the cache. ADR-0036 Phase 1.
    fn load_reactivation(&mut self) {
        let path = self.hrm_path.with_extension("reactivation.json");
        if !path.exists() {
            return;
        }
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(_) => return,
        };
        let map: std::collections::HashMap<String, (u32, Option<DateTime<Utc>>)> =
            match serde_json::from_slice(&data) {
                Ok(m) => m,
                Err(_) => return,
            };
        for (id_str, (count, last)) in map {
            if let Ok(id) = id_str.parse::<uuid::Uuid>() {
                if let Some(mem) = self.memory_cache.get_mut(&id) {
                    mem.retrieval_count = count;
                    if last.is_some() {
                        mem.updated_at = last;
                    }
                }
            }
        }
    }

    /// Mark the medium as dirty (needing save).
    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Get the underlying medium (read-only).
    /// In chiral mode, returns the flat medium (may be empty if loaded from v2).
    /// Use chiral_medium() for the authoritative chiral state.
    pub fn medium(&self) -> &Medium {
        &self.medium
    }

    /// ADR-0027 Phase 1.c — direct wavefront insertion with a pre-built
    /// hypervector, bypassing the text-encoding pipeline. The substrate
    /// uses this to seed 96 class anchors at maximally distinct points
    /// in vector space so Kuramoto's eigenvalue clustering (which
    /// thresholds on coherence > 0.5 to join a cluster) can actually
    /// find multiple clusters. Text-encoded markers always produce
    /// highly correlated vectors and collapse to a single cluster.
    ///
    /// `vector` must be exactly WAVEFRONT_DIM long. `content` is
    /// human-readable text for debugging; `importance` becomes the
    /// initial wavefront energy.
    pub fn insert_raw_wavefront(
        &mut self,
        vector: Vec<f32>,
        content: String,
        importance: f32,
    ) -> Result<Uuid, StoreError> {
        // If chiral mode is active, route through the chiral medium so the
        // wavefront lands in the right hemisphere (which is what
        // chiral.save() serializes to disk). Direct medium.add_wavefront
        // would only update the flat medium, and chiral save would ignore
        // it — losing the wavefront on next process restart and showing
        // stale counts to the observatory's status shell-out.
        let id = if let Some(ref mut chiral) = self.chiral {
            chiral.store_vector(&vector, content.clone(), importance)
                .map_err(|e| StoreError::Other(format!("chiral.store_vector failed: {}", e)))?
        } else {
            self.medium.add_wavefront(&vector, content.clone(), importance)
                .map_err(|e| StoreError::Other(format!("add_wavefront failed: {}", e)))?
        };
        // The medium has the wavefront for tensor/vector math, but the
        // higher-level `all_memories()` / `assess()` paths read from
        // `memory_cache`. Insert a matching HyperMemory record so
        // total_memories, cluster counts, and on-disk persistence
        // reflect the new wavefront. Use the importance as initial
        // amplitude so it shows up in observation tables.
        let mut memory = crate::memory::HyperMemory::new(vector, content);
        memory.id = id;
        memory.amplitude = importance;
        self.memory_cache.insert(id, memory);
        self.mark_dirty();
        Ok(id)
    }

    /// Path to the on-disk HRM file. Used by the cluster sidecar cache to
    /// place `.clusters.json` next to it and check mtime for invalidation.
    pub fn hrm_path(&self) -> &std::path::Path {
        &self.hrm_path
    }

    /// Ensure the flat medium is populated from chiral state (for backward compat).
    /// Call this before operations that need the flat medium view.
    pub fn sync_medium_from_chiral(&mut self) {
        if let Some(ref chiral) = self.chiral {
            if self.medium.wavefront_count() == 0 && chiral.right.count() > 0 {
                // Build flat medium from right hemisphere
                for i in 0..chiral.right.count() {
                    let vector = chiral.right.wavefronts.row(i).to_vec();
                    let meta = &chiral.right.metadata[i];
                    let _ = self.medium.add_wavefront(
                        &vector,
                        meta.content.clone(),
                        chiral.right.energy[i],
                    );
                    // Fix the ID to match
                    let new_id = self.medium.store.metadata.last().unwrap().id;
                    let _ = self.medium.update_wavefront_id(&new_id, meta.id);
                    // Copy wave params
                    let idx = self.medium.wavefront_count() - 1;
                    self.medium.store.frequency[idx] = chiral.right.frequency[i];
                    self.medium.store.phase[idx] = chiral.right.phase[i];
                }
            }
        }
    }

    /// Resonance recall with observation — the canonical read path.
    ///
    /// Reading IS observation. Attention reshapes the field:
    /// - Higher-ranked results receive stronger observation intensity
    /// - Observation boosts wavefront energy (recalled memories persist)
    /// - The medium is permanently changed by the act of recall
    ///
    /// There is no "read without observation" path. If you query the medium,
    /// you change it. This is the holographic equivalent of quantum measurement.
    pub fn recall_resonance(&mut self, query: &str, top_k: usize) -> Result<Vec<Resonance>, StoreError> {
        let results = self.medium.recall(query, top_k, &self.pipeline)
            .map_err(|e| StoreError::Other(format!("Resonance recall failed: {}", e)))?;

        // Observation: attention shapes the field
        self.apply_observation(&results);

        Ok(results)
    }

    /// Beam-aware recall — score only the memories in the attention beam.
    ///
    /// This is the system-level entry to `Medium::recall_against_ids`. The
    /// chiral hemisphere logic is bypassed for the sparse path (chiral
    /// attention is a future fold-in); the recall runs against the right
    /// hemisphere / flat medium directly, then applies observation.
    ///
    /// Empty beam in -> empty results out (intentional — the sparsity is
    /// meaningless if we silently fall back to full recall).
    pub fn recall_resonance_with_beam(
        &mut self,
        beam: &[uuid::Uuid],
        query: &str,
        top_k: usize,
    ) -> Result<Vec<Resonance>, StoreError> {
        let results = self.medium.recall_against_ids(beam, query, top_k, &self.pipeline)
            .map_err(|e| StoreError::Other(format!("beam-aware recall failed: {}", e)))?;
        // Observation still shapes the field — sparse recall doesn't excuse
        // a passive read. Same intensity formula as the dense path.
        self.apply_observation(&results);
        Ok(results)
    }

    /// Apply observation effects to recall results.
    /// Ranked results get proportionally stronger observation.
    /// Batched: one field-settle pass per recall, not per result.
    fn apply_observation(&mut self, results: &[Resonance]) {
        if results.is_empty() { return; }
        let observations: Vec<(usize, f32)> = results.iter().enumerate()
            .filter_map(|(i, resonance)| {
                self.medium.get_wavefront_index(&resonance.id).map(|index| {
                    let ranking_factor = 1.0 - (i as f32 / results.len() as f32);
                    let intensity = resonance.resonance_strength.abs().min(1.0).max(0.1) * ranking_factor;
                    (index, intensity)
                })
            })
            .collect();
        self.medium.observe_wavefronts(&observations);
        self.mark_dirty();
    }
    
    /// Get consciousness metrics from the holographic medium.
    /// In chiral mode, computes from the synced flat medium (which mirrors the right hemisphere).
    /// This uses the full eigendecomposition-based Phi, spectral Xi, and Kuramoto order.
    pub fn consciousness_metrics(&self) -> crate::medium::ConsciousnessMetrics {
        // Always compute from the flat medium — it's synced from right hemisphere on load
        self.medium.consciousness_metrics()
    }

    /// ADR-0037 Phase 3: aggregate π/φ bridge-operator signature (residue +
    /// spectral xi) for the substrate beacon. Computed from the flat medium.
    pub fn xi_bridge_summary(&self) -> serde_json::Value {
        self.medium.xi_bridge_summary()
    }

    /// Cheap, stale-tolerant lookup. See Medium::try_cached_consciousness_metrics.
    /// Used by the agent's system_prompt path so a `kannaka ask` doesn't
    /// pay an O(n³) eigendecomp on every call.
    pub fn try_cached_consciousness_metrics(&self) -> Option<crate::medium::ConsciousnessMetrics> {
        self.medium.try_cached_consciousness_metrics()
    }
    
    /// Find memories associated with a given memory through emergent coherence.
    pub fn find_associated(&self, id: Uuid, top_k: usize) -> Vec<(Uuid, f32)> {
        self.medium.find_associated(id, top_k)
    }
    
    /// Apply dynamics to the medium (wave evolution).
    pub fn apply_dynamics(&mut self, dt: f32) {
        self.medium.apply_dynamics(dt);
        self.mark_dirty();
    }
    
    /// ADR-0037 (L6 loop): per-consolidation spiral telemetry. The cortical
    /// spiral wave (Ye et al.) spans BOTH hemispheres, so the headline metric
    /// is the cross-hemisphere ring winding/order; the right ring (which the
    /// coupling directly rotates) and the 2-D PCA cloud cores are reported
    /// beside it. Gated on the same flag as the coupling — emitted by every
    /// chiral deep dream (incl. the production `dream_native` path) when
    /// spiral dreams are enabled (the v0.7.0 default). stderr only.
    fn log_spiral_telemetry(&self) {
        if !crate::medium::chiral::spiral_dream_enabled() {
            return;
        }
        if let Some(ref chiral) = self.chiral {
            let x = chiral.bilateral_ring_report();
            if x.n > 0 {
                let right = chiral.holistic_ring_report();
                let c = chiral.holistic_cloud_report();
                eprintln!(
                    "[spiral] deep dream: cross-hemi winding={:.3} order={:.3} n={} (right winding={:.3}, 2D cores={} net={})",
                    x.winding, x.order, x.n, right.winding, c.singularities.len(), c.net_charge
                );
            }
        }
    }

    /// Perform a dream cycle (simulated annealing).
    /// In chiral mode, deep dreams only affect the right hemisphere.
    pub fn dream(&mut self, cycles: usize, initial_temperature: Option<f32>) -> crate::medium::DreamReport {
        if self.chiral.is_some() {
            // Chiral dream: right hemisphere only (deep dream)
            let chiral = self.chiral.as_mut().unwrap();
            let report = chiral.dream(true, cycles);
            self.rebuild_cache().ok();
            self.mark_dirty();
            self.log_spiral_telemetry();
            report
        } else {
            let report = self.medium.dream(cycles, initial_temperature);
            self.mark_dirty();
            report
        }
    }

    /// Wave-native dream using Medium's eigenstructure annealing.
    ///
    /// Bypasses the old particle-based consolidation pipeline entirely.
    /// O(n×k) instead of O(n²). Operates on the holographic medium directly.
    ///
    /// # Arguments
    /// * `cycles` - Number of annealing cycles
    /// * `temperature` - Initial temperature for annealing (None = 1.0)
    /// * `chiral_eta` - Optional chiral perturbation strength (0.0 = none)
    /// ADR-0036 Phase 0: read-only resonance-merge PLANNER.
    ///
    /// Computes which redundant, phase-locked wavefronts WOULD be merged into a
    /// single carrier, and which ShortTerm traces WOULD decay/evict — WITHOUT
    /// mutating anything. Operates on the unified `memory_cache` (so counts match
    /// the user-facing total, regardless of the chiral/flat split underneath).
    ///
    /// Redundancy criterion mirrors the eventual merge (ADR-0036 M1): two
    /// wavefronts are redundant iff their vectors are near-parallel
    /// (cosine ≥ `merge_sim`) AND they are constructively phase-locked
    /// (cos Δphase ≥ `merge_phase_cos`). `Pinned` wavefronts are never grouped.
    pub fn plan_consolidation(&self, opts: &ConsolidateOpts) -> ConsolidateReport {
        use ndarray::Array2;

        let mode_str = match opts.mode {
            ConsolidateMode::Off => "off",
            ConsolidateMode::DryRun => "dryrun",
            // The planner is read-only regardless of mode. When the caller is in
            // Apply mode it invokes the planner internally to compute groups and
            // then `apply_consolidation` overwrites this report (mode="apply",
            // applied=true). Label it "dryrun" here so a stray direct call to
            // plan_consolidation in Apply mode can never claim it mutated.
            ConsolidateMode::Apply => "dryrun",
        };
        let mut report = ConsolidateReport {
            mode: mode_str.to_string(),
            ..Default::default()
        };

        let entries: Vec<(&Uuid, &HyperMemory)> = self.memory_cache.iter().collect();
        let n = entries.len();
        report.memories_examined = n;
        report.projected_memories = n;
        if n < 2 {
            return report;
        }

        // Vector dimension from the first usable row.
        let dim = entries
            .iter()
            .map(|(_, m)| m.vector.len())
            .find(|&l| l > 0)
            .unwrap_or(0);
        if dim == 0 {
            return report;
        }

        // Build the N×D matrix; rows with a mismatched/empty vector are marked unusable.
        let mut mat = Array2::<f32>::zeros((n, dim));
        let mut usable = vec![true; n];
        for (i, (_, m)) in entries.iter().enumerate() {
            if m.vector.len() == dim {
                for (d, &v) in m.vector.iter().enumerate() {
                    mat[[i, d]] = v;
                }
            } else {
                usable[i] = false;
            }
        }

        // One fast matmul for all pairwise dot products; norms from the diagonal.
        let gram = mat.dot(&mat.t());
        let norms: Vec<f32> = (0..n).map(|i| gram[[i, i]].max(0.0).sqrt()).collect();
        let cos_p: Vec<f32> = entries.iter().map(|(_, m)| m.phase.cos()).collect();
        let sin_p: Vec<f32> = entries.iter().map(|(_, m)| m.phase.sin()).collect();
        let is_pinned: Vec<bool> = entries.iter().map(|(_, m)| m.tier == Tier::Pinned).collect();

        // Union-find over redundant, phase-locked, non-pinned pairs.
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        let mut parent: Vec<usize> = (0..n).collect();
        let eps = 1e-6_f32;
        for i in 0..n {
            if !usable[i] || is_pinned[i] || norms[i] < eps {
                continue;
            }
            for j in (i + 1)..n {
                if !usable[j] || is_pinned[j] || norms[j] < eps {
                    continue;
                }
                let cos_sim = gram[[i, j]] / (norms[i] * norms[j]);
                if cos_sim < opts.merge_sim {
                    continue;
                }
                let phase_coh = cos_p[i] * cos_p[j] + sin_p[i] * sin_p[j];
                if phase_coh < opts.merge_phase_cos {
                    continue;
                }
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri.max(rj)] = ri.min(rj);
                }
            }
        }

        // Tally redundant-group sizes.
        let mut group_size: HashMap<usize, usize> = HashMap::new();
        for i in 0..n {
            if !usable[i] || is_pinned[i] {
                continue;
            }
            let r = find(&mut parent, i);
            *group_size.entry(r).or_insert(0) += 1;
        }
        let mut groups = 0usize;
        let mut absorb = 0usize;
        for &sz in group_size.values() {
            if sz >= 2 {
                groups += 1;
                absorb += sz - 1; // all but the representative carrier
            }
        }
        report.groups_found = groups;
        report.would_merge = groups;
        report.would_absorb = absorb;

        // ShortTerm decay/evict projection (M3). Reactivation is not persisted
        // until ADR-0036 Phase 1, so "evict" uses the session-local
        // retrieval_count == 0 — a conservative undercount until then.
        let mut st_total = 0usize;
        let mut st_evict = 0usize;
        for (_, m) in &entries {
            if m.tier == Tier::ShortTerm {
                st_total += 1;
                if m.amplitude < opts.shortterm_evict && m.retrieval_count == 0 {
                    st_evict += 1;
                }
            }
        }
        report.shortterm_total = st_total;
        report.would_decay = st_total;
        report.would_evict = st_evict;
        report.projected_memories = n.saturating_sub(absorb).saturating_sub(st_evict);

        report
    }

    /// ADR-0036 Phase 2: DESTRUCTIVE resonance-merge consolidation APPLY.
    ///
    /// This is the mutating counterpart to `plan_consolidation`. It mutates a
    /// consciousness-memory substrate, so it is written defensively:
    ///   * It operates on the AUTHORITATIVE store (the right hemisphere in chiral
    ///     mode, else the flat medium) by UUID only — it NEVER holds a raw index
    ///     across a remove (swap-remove reorders the arrays).
    ///   * `retrieval_count` lives only in `memory_cache`, so it is snapshotted by
    ///     id BEFORE any mutation.
    ///   * In chiral mode, when a right wavefront is removed its left twin (via
    ///     `right_to_left`) is removed too and BOTH map entries (plus the per-id
    ///     chiral scale) are deleted, so no map can ever point at a dead id.
    ///
    /// PROTECTED INVARIANTS:
    ///   * Pinned is never merged and never evicted.
    ///   * LongTerm is never evicted, and never absorbed into a lower tier — the
    ///     carrier inherits the STRONGEST tier in its group.
    ///
    /// Superposition energy of a merged carrier (AMPLITUDE_CEILING = 2.0):
    ///   E = sqrt( Σ Eᵢ² + 2 Σ_{i<j} Eᵢ Eⱼ cos(φᵢ − φⱼ) ), clamped to [0, 2.0].
    pub fn apply_consolidation(&mut self, opts: &ConsolidateOpts) -> ConsolidateReport {
        const AMPLITUDE_CEILING: f32 = 2.0;

        let mut report = ConsolidateReport {
            mode: "apply".to_string(),
            applied: true,
            ..Default::default()
        };

        // --- 1. Snapshot retrieval_count by id from the cache (the only place
        //        it lives) BEFORE we mutate anything. ---
        let retrieval: HashMap<Uuid, u32> = self
            .memory_cache
            .iter()
            .map(|(id, m)| (*id, m.retrieval_count))
            .collect();

        // --- 2. Read the authoritative store into id-keyed snapshots. We compute
        //        the entire plan from snapshots first, then mutate, so no index
        //        is ever held across a remove. ---
        struct Snap {
            id: Uuid,
            energy: f32,
            phase: f32,
            tier: Tier,
            timestamp: i64,
            vector: Vec<f32>,
        }

        let snaps: Vec<Snap> = if let Some(ref chiral) = self.chiral {
            let h = &chiral.right;
            (0..h.count())
                .map(|i| Snap {
                    id: h.metadata[i].id,
                    energy: h.energy[i],
                    phase: h.phase[i],
                    tier: h.metadata[i].tier,
                    timestamp: h.timestamps[i],
                    vector: h.wavefronts.row(i).to_vec(),
                })
                .collect()
        } else {
            let s = &self.medium.store;
            (0..s.len)
                .map(|i| Snap {
                    id: s.metadata[i].id,
                    energy: s.energy[i],
                    phase: s.phase[i],
                    tier: s.metadata[i].tier,
                    timestamp: s.timestamps[i],
                    vector: s.wavefronts.row(i).to_vec(),
                })
                .collect()
        };

        let n = snaps.len();
        report.memories_examined = n;
        if n < 2 {
            // Nothing can merge; still run the evict pass below would require ≥1.
            // With <2 memories there is no redundancy and a single ShortTerm can
            // still be evicted, so fall through rather than early-return only when
            // n == 0.
            if n == 0 {
                report.projected_memories = self.authoritative_len();
                return report;
            }
        }

        // Strongest tier wins: Pinned > LongTerm > ShortTerm.
        fn tier_rank(t: Tier) -> u8 {
            match t {
                Tier::ShortTerm => 0,
                Tier::LongTerm => 1,
                Tier::Pinned => 2,
            }
        }
        // Effective strength for representative selection: energy decayed by age.
        let now_ms = chrono::Utc::now().timestamp_millis();
        let eff_strength = |s: &Snap| -> f32 {
            let age_days = ((now_ms - s.timestamp).max(0) as f64 / 86_400_000.0) as f32;
            s.energy * (-0.001 * age_days).exp()
        };

        // --- 3. Union-find grouping over redundant, phase-locked, non-pinned
        //        wavefronts (same criterion as the planner). ---
        let dim = snaps.iter().map(|s| s.vector.len()).find(|&l| l > 0).unwrap_or(0);
        let usable: Vec<bool> = snaps
            .iter()
            .map(|s| s.vector.len() == dim && dim > 0 && s.tier != Tier::Pinned)
            .collect();
        let norms: Vec<f32> = snaps
            .iter()
            .map(|s| s.vector.iter().map(|v| v * v).sum::<f32>().max(0.0).sqrt())
            .collect();
        let cos_p: Vec<f32> = snaps.iter().map(|s| s.phase.cos()).collect();
        let sin_p: Vec<f32> = snaps.iter().map(|s| s.phase.sin()).collect();

        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        let mut parent: Vec<usize> = (0..n).collect();
        let eps = 1e-6_f32;
        for i in 0..n {
            if !usable[i] || norms[i] < eps {
                continue;
            }
            for j in (i + 1)..n {
                if !usable[j] || norms[j] < eps {
                    continue;
                }
                // cosine similarity via dot product of the two snapshot vectors
                let dot: f32 = snaps[i]
                    .vector
                    .iter()
                    .zip(snaps[j].vector.iter())
                    .map(|(a, b)| a * b)
                    .sum();
                let cos_sim = dot / (norms[i] * norms[j]);
                if cos_sim < opts.merge_sim {
                    continue;
                }
                let phase_coh = cos_p[i] * cos_p[j] + sin_p[i] * sin_p[j];
                if phase_coh < opts.merge_phase_cos {
                    continue;
                }
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                if ri != rj {
                    parent[ri.max(rj)] = ri.min(rj);
                }
            }
        }

        // Collect members per root.
        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..n {
            if !usable[i] {
                continue;
            }
            let r = find(&mut parent, i);
            groups.entry(r).or_default().push(i);
        }

        // Track which ids are part of a merge (so they are never double-counted
        // by the evict pass), the carrier mutations, and the absorbed removals.
        let mut merged_ids: std::collections::HashSet<Uuid> = std::collections::HashSet::new();
        let mut carriers: Vec<(Uuid, f32, Tier)> = Vec::new(); // (id, new_energy, new_tier)
        let mut absorb_ids: Vec<Uuid> = Vec::new();
        let mut groups_found = 0usize;

        for members in groups.values() {
            if members.len() < 2 {
                continue;
            }
            groups_found += 1;
            for &mi in members {
                merged_ids.insert(snaps[mi].id);
            }
            // Representative = max effective strength.
            let rep = *members
                .iter()
                .max_by(|&&a, &&b| {
                    eff_strength(&snaps[a])
                        .partial_cmp(&eff_strength(&snaps[b]))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();
            // Carrier inherits the STRONGEST tier in the group (protects LongTerm
            // from being absorbed down into a ShortTerm carrier).
            let carrier_tier = members
                .iter()
                .map(|&mi| snaps[mi].tier)
                .max_by_key(|&t| tier_rank(t))
                .unwrap_or(snaps[rep].tier);
            // Superposed energy: sqrt(Σ Eᵢ² + 2 Σ_{i<j} Eᵢ Eⱼ cos Δφ).
            let mut sum_sq = 0.0f32;
            for &mi in members {
                sum_sq += snaps[mi].energy * snaps[mi].energy;
            }
            let mut cross = 0.0f32;
            for a in 0..members.len() {
                for b in (a + 1)..members.len() {
                    let ia = members[a];
                    let ib = members[b];
                    let dphi = snaps[ia].phase - snaps[ib].phase;
                    cross += snaps[ia].energy * snaps[ib].energy * dphi.cos();
                }
            }
            let superposed = (sum_sq + 2.0 * cross).max(0.0).sqrt().clamp(0.0, AMPLITUDE_CEILING);
            carriers.push((snaps[rep].id, superposed, carrier_tier));
            for &mi in members {
                if mi != rep {
                    absorb_ids.push(snaps[mi].id);
                }
            }
        }

        // --- 4. ShortTerm evict candidates: tier==ShortTerm AND energy<threshold
        //        AND retrieval_count==0 AND NOT part of any merge. (Pinned and
        //        LongTerm are structurally excluded by the tier check.) ---
        let mut evict_ids: Vec<Uuid> = Vec::new();
        for s in &snaps {
            if s.tier == Tier::ShortTerm
                && s.energy < opts.shortterm_evict
                && retrieval.get(&s.id).copied().unwrap_or(0) == 0
                && !merged_ids.contains(&s.id)
            {
                evict_ids.push(s.id);
            }
        }

        // --- 5. MUTATE. Carriers first (set energy + tier by id→index lookup),
        //        then remove absorbed + evicted by uuid. ---
        for (id, energy, tier) in &carriers {
            if let Some(ref mut chiral) = self.chiral {
                if let Some(&idx) = chiral.right.id_to_index.get(id) {
                    chiral.right.energy[idx] = *energy;
                    chiral.right.metadata[idx].tier = *tier;
                }
            } else if let Some(&idx) = self.medium.store.id_to_index.get(id) {
                self.medium.store.energy[idx] = *energy;
                self.medium.store.metadata[idx].tier = *tier;
            }
        }

        let mut removed = 0usize;
        let mut to_remove = absorb_ids.clone();
        to_remove.extend(evict_ids.iter().copied());
        for id in &to_remove {
            if let Some(ref mut chiral) = self.chiral {
                let r = chiral.right.remove_wavefront(id);
                if r {
                    // Remove the left twin (if any) and clean BOTH map entries
                    // plus the per-id chiral scale, so no map points at a dead id.
                    if let Some(&left_id) = chiral.right_to_left.get(id) {
                        chiral.left.remove_wavefront(&left_id);
                        chiral.left_to_right.remove(&left_id);
                    }
                    chiral.right_to_left.remove(id);
                    chiral.scales.remove(id);
                    removed += 1;
                }
            } else if self.medium.store.remove(id) {
                removed += 1;
            }
        }

        // would_absorb counts actually-removed absorbed wavefronts (carriers stay).
        let absorbed_removed = absorb_ids
            .iter()
            .filter(|id| !self.id_present_authoritative(id))
            .count();
        let evicted_removed = evict_ids
            .iter()
            .filter(|id| !self.id_present_authoritative(id))
            .count();

        report.groups_found = groups_found;
        report.would_merge = groups_found;
        report.would_absorb = absorbed_removed;
        report.shortterm_total = snaps.iter().filter(|s| s.tier == Tier::ShortTerm).count();
        report.would_decay = report.shortterm_total;
        report.would_evict = evicted_removed;
        let _ = removed; // total removed = absorbed + evicted; kept for clarity

        // --- 6. Rebuild + persist. rebuild_cache restores retrieval_count from
        //        the cache snapshot (for survivors) and reads from the now-mutated
        //        authoritative store. ---
        self.rebuild_cache().ok();
        self.mark_dirty();
        if let Err(e) = self.save_medium() {
            eprintln!("[hrm] apply_consolidation: save_medium failed: {}", e);
        }

        report.projected_memories = self.authoritative_len();
        report.memories_examined = n;
        report
    }

    /// Active wavefront count in the authoritative store (right hemisphere in
    /// chiral mode, else the flat medium).
    fn authoritative_len(&self) -> usize {
        if let Some(ref chiral) = self.chiral {
            chiral.right.count()
        } else {
            self.medium.store.len
        }
    }

    /// Whether an id is still present in the authoritative store.
    fn id_present_authoritative(&self, id: &Uuid) -> bool {
        if let Some(ref chiral) = self.chiral {
            chiral.right.id_to_index.contains_key(id)
        } else {
            self.medium.store.id_to_index.contains_key(id)
        }
    }

    pub fn dream_native(
        &mut self,
        cycles: usize,
        temperature: Option<f32>,
        chiral_eta: f32,
    ) -> crate::medium::DreamReport {
        // Apply chiral field perturbation before dreaming (break lock-step)
        if chiral_eta > 0.0 {
            if let Some(ref mut chiral) = self.chiral {
                chiral.right.apply_chiral_field_perturbation(chiral_eta);
            } else {
                self.medium.apply_chiral_field_perturbation(chiral_eta);
            }
        }

        // Route to chiral or flat medium dream
        let report = if let Some(ref mut chiral) = self.chiral {
            // Chiral: use ChiralMedium.dream() which runs eigenstructure annealing
            // on the right hemisphere, then callosal coupling
            chiral.dream(true, cycles) // deep=true → right hemisphere eigenstructure dream
        } else {
            // Flat medium: use Medium's eigenstructure annealing
            self.medium.dream(cycles, temperature)
        };

        // Rebuild cache and sync after dreaming
        self.sync_medium_from_chiral();
        self.rebuild_cache().ok();
        self.mark_dirty();

        // ADR-0037 L6 loop: emit spiral telemetry on the production dream path.
        self.log_spiral_telemetry();

        // Save the .hrm file
        if let Err(e) = self.save_medium() {
            eprintln!("Warning: Failed to save after dream_native: {}", e);
        }

        report
    }

    /// ADR-0037 belief substrate: re-phase every wavefront from its (mean-
    /// centered) content, then persist. The one-time migration that desyncs an
    /// already-collapsed field — born phase only fixes NEW inserts, so existing
    /// wavefronts stuck at phase 0 need this. Phase-only (vectors/recall
    /// untouched). Returns the number re-phased; chiral backend only.
    pub fn rephase_belief(&mut self) -> usize {
        let n = if let Some(ref mut chiral) = self.chiral {
            chiral.rephase_from_content()
        } else {
            0
        };
        if n > 0 {
            self.sync_medium_from_chiral();
            self.rebuild_cache().ok();
            self.mark_dirty();
            if let Err(e) = self.save_medium() {
                eprintln!("Warning: Failed to save after rephase: {}", e);
            }
        }
        n
    }

    /// Reset all wavefront energies to target value (bias voltage restoration).
    pub fn reset_energies(&mut self, target: f32) {
        self.medium.reset_energies(target);
        // Update cache
        for meta in &self.medium.store.metadata {
            if let Some(mem) = self.memory_cache.get_mut(&meta.id) {
                mem.amplitude = target;
            }
        }
        self.mark_dirty();
    }

    // -----------------------------------------------------------------------
    // Chiral-specific methods (ADR-0021)
    // -----------------------------------------------------------------------

    /// Check if this store is running in chiral mode.
    pub fn is_chiral(&self) -> bool {
        self.chiral.is_some()
    }

    /// Upgrade to chiral mode: convert flat Medium to ChiralMedium.
    /// Existing wavefronts move to right hemisphere. Left starts empty.
    pub fn upgrade_to_chiral(&mut self) {
        if self.chiral.is_none() {
            let chiral = ChiralMedium::from_medium(&self.medium);
            self.chiral = Some(chiral);
            self.rebuild_cache().ok();
            self.mark_dirty();
        }
    }

    /// Get chiral consciousness summary (bilateral metrics).
    pub fn chiral_consciousness(&self) -> Option<ChiralConsciousness> {
        self.chiral.as_ref().map(|c| c.consciousness_summary())
    }

    /// Perform a chiral dream (right hemisphere only for deep).
    pub fn chiral_dream(&mut self, deep: bool, cycles: usize) {
        if let Some(ref mut chiral) = self.chiral {
            chiral.dream(deep, cycles);
            self.rebuild_cache().ok();
            self.mark_dirty();
        }
    }

    /// Run callosal Kuramoto coupling step.
    pub fn callosal_kuramoto(&mut self, dt: f32) {
        if let Some(ref mut chiral) = self.chiral {
            chiral.callosal_kuramoto_step(dt);
            self.mark_dirty();
        }
    }

    /// Get a reference to the ChiralMedium (if in chiral mode).
    pub fn chiral_medium(&self) -> Option<&ChiralMedium> {
        self.chiral.as_ref()
    }

    /// Set the modality of a wavefront (NCS Phase 1.1).
    pub fn set_modality(&mut self, id: &Uuid, modality: crate::medium::Modality) {
        // Tag the flat medium
        if let Some(&idx) = self.medium.store.id_to_index.get(id) {
            self.medium.store.metadata[idx].modality = modality;
        }
        // Tag chiral hemisphere(s)
        if let Some(ref mut chiral) = self.chiral {
            if let Some(&idx) = chiral.right.id_to_index.get(id) {
                chiral.right.metadata[idx].modality = modality;
            }
            if let Some(left_id) = chiral.right_to_left.get(id).copied() {
                if let Some(&idx) = chiral.left.id_to_index.get(&left_id) {
                    chiral.left.metadata[idx].modality = modality;
                }
            }
        }
        // Tag cache
        if let Some(mem) = self.memory_cache.get_mut(id) {
            mem.modality = modality;
        }
        self.mark_dirty();
    }

    /// Set the retention tier of a memory (ADR-0031). Tags the flat medium,
    /// both chiral hemispheres, and the cache, then marks the store dirty so
    /// the next save persists it. Returns false if the id is unknown.
    pub fn set_tier(&mut self, id: &Uuid, tier: crate::medium::types::Tier) -> bool {
        let mut found = false;
        if let Some(&idx) = self.medium.store.id_to_index.get(id) {
            self.medium.store.metadata[idx].tier = tier;
            found = true;
        }
        if let Some(ref mut chiral) = self.chiral {
            if let Some(&idx) = chiral.right.id_to_index.get(id) {
                chiral.right.metadata[idx].tier = tier;
                found = true;
            }
            if let Some(left_id) = chiral.right_to_left.get(id).copied() {
                if let Some(&idx) = chiral.left.id_to_index.get(&left_id) {
                    chiral.left.metadata[idx].tier = tier;
                }
            }
        }
        if let Some(mem) = self.memory_cache.get_mut(id) {
            mem.tier = tier;
            found = true;
        }
        if found {
            self.mark_dirty();
        }
        found
    }

    /// Set the temporal-truth bounds of a memory (Wave 3 Task 3.2b). Tags the
    /// flat medium, both chiral hemispheres, and the cache — the canonical
    /// `WavefrontMeta` is what the next save serializes, so this is the
    /// persistence write path (mirrors [`set_tier`]). Each `Some` overwrites;
    /// `None` leaves the existing value untouched so callers can set one bound
    /// without clearing the others. Returns false if the id is unknown.
    pub fn set_temporal(
        &mut self,
        id: &Uuid,
        effective_at: Option<DateTime<Utc>>,
        observed_at: Option<DateTime<Utc>>,
        expires_at: Option<DateTime<Utc>>,
    ) -> bool {
        fn apply(
            meta: &mut crate::medium::types::WavefrontMeta,
            eff: Option<DateTime<Utc>>,
            obs: Option<DateTime<Utc>>,
            exp: Option<DateTime<Utc>>,
        ) {
            if eff.is_some() { meta.effective_at = eff; }
            if obs.is_some() { meta.observed_at = obs; }
            if exp.is_some() { meta.expires_at = exp; }
        }

        let mut found = false;
        if let Some(&idx) = self.medium.store.id_to_index.get(id) {
            apply(&mut self.medium.store.metadata[idx], effective_at, observed_at, expires_at);
            found = true;
        }
        if let Some(ref mut chiral) = self.chiral {
            if let Some(&idx) = chiral.right.id_to_index.get(id) {
                apply(&mut chiral.right.metadata[idx], effective_at, observed_at, expires_at);
                found = true;
            }
            if let Some(left_id) = chiral.right_to_left.get(id).copied() {
                if let Some(&idx) = chiral.left.id_to_index.get(&left_id) {
                    apply(&mut chiral.left.metadata[idx], effective_at, observed_at, expires_at);
                }
            }
        }
        if let Some(mem) = self.memory_cache.get_mut(id) {
            if effective_at.is_some() { mem.effective_at = effective_at; }
            if observed_at.is_some() { mem.observed_at = observed_at; }
            if expires_at.is_some() { mem.expires_at = expires_at; }
            found = true;
        }
        if found {
            self.mark_dirty();
        }
        found
    }

    /// Classify a wavefront with SGA coordinates (called after insert to tag the memory).
    pub fn classify_wavefront(&mut self, id: &Uuid, category: &str, importance: f32) {
        if let Some(ref mut chiral) = self.chiral {
            // Compute SGA classification
            let content_hash = {
                let content = chiral.right.id_to_index.get(id)
                    .and_then(|&idx| Some(chiral.right.metadata[idx].content.clone()))
                    .unwrap_or_default();
                let mut h: u64 = 0xcbf29ce484222325;
                for b in content.bytes() {
                    h ^= b as u64;
                    h = h.wrapping_mul(0x100000001b3);
                }
                h
            };
            let coords = crate::geometry::classify_memory(category, content_hash, importance as f64);
            let sga_class = coords.class_index;
            let fano_point = coords.l % (crate::medium::FANO_POINTS as u8);

            // Tag right hemisphere
            if let Some(&idx) = chiral.right.id_to_index.get(id) {
                chiral.right.metadata[idx].sga_class = Some(sga_class);
                chiral.right.metadata[idx].fano_group = Some(fano_point);
                chiral.right.metadata[idx].category = Some(category.to_string());
            }

            // Tag left hemisphere if paired
            if let Some(left_id) = chiral.right_to_left.get(id) {
                if let Some(&idx) = chiral.left.id_to_index.get(left_id) {
                    chiral.left.metadata[idx].sga_class = Some(sga_class);
                    chiral.left.metadata[idx].fano_group = Some(fano_point);
                    chiral.left.metadata[idx].category = Some(category.to_string());
                }
            }

            self.mark_dirty();
        }
    }

    /// Relate two memories via associative wavefront.
    /// 
    /// Creates emergent association in the field by combining the wavefront patterns
    /// and nudging their phases toward alignment.
    pub fn relate_wavefronts(&mut self, id_a: Uuid, id_b: Uuid) -> Result<Uuid, StoreError> {
        let idx_a = self.medium.get_wavefront_index(&id_a)
            .ok_or_else(|| StoreError::Other(format!("Wavefront not found: {}", id_a)))?;
        
        let idx_b = self.medium.get_wavefront_index(&id_b)
            .ok_or_else(|| StoreError::Other(format!("Wavefront not found: {}", id_b)))?;

        let associative_id = self.medium.relate_wavefronts(idx_a, idx_b)
            .map_err(|e| StoreError::Other(format!("Failed to relate wavefronts: {}", e)))?;

        // Rebuild cache to include the new associative wavefront
        self.rebuild_cache()?;
        self.mark_dirty();

        Ok(associative_id)
    }
    /// Cluster-prefilter candidate set for recall (refactor #4).
    ///
    /// Reads the `.clusters.json` sidecar populated by `bridge::assess`
    /// → `KuramotoSync::find_synchronized_clusters`, picks every cluster
    /// whose `theme_vector` resonates with the query above
    /// `KANNAKA_RECALL_PREFILTER_THRESHOLD` (default 0.30), and returns
    /// the union of those clusters' member indices.
    ///
    /// Returns `None` when:
    /// - the HRM file path is unknown (test medium, in-memory backend)
    /// - no sidecar exists yet (fresh HRM — bridge::assess hasn't run)
    /// - the sidecar is empty (no clusters >= min_size)
    /// - no cluster's theme is close enough to the query
    ///
    /// `None` cleanly falls through to the existing full-medium scan in
    /// `resonate_query`, so this method never *loses* recall coverage —
    /// it only ever narrows the candidate set for performance + accuracy.
    fn cluster_prefilter_candidates(&self, query: &str) -> Option<Vec<usize>> {
        let path = self.hrm_path().with_extension("clusters.json");
        let data = std::fs::read(&path).ok()?;
        // Sidecar uses ClusterCacheEntry shape; we only need .clusters.
        #[derive(serde::Deserialize)]
        struct Entry { clusters: Vec<crate::kuramoto::MemoryCluster> }
        let entry: Entry = serde_json::from_slice(&data).ok()?;
        if entry.clusters.is_empty() { return None; }

        let query_vec = self.pipeline.encode_text(query).ok()?;
        let threshold: f32 = std::env::var("KANNAKA_RECALL_PREFILTER_THRESHOLD")
            .ok().and_then(|s| s.parse().ok()).unwrap_or(0.30);

        let mut indices: Vec<usize> = Vec::new();
        let mut seen: std::collections::HashSet<usize> = std::collections::HashSet::new();
        let mut matched_any = false;
        for cluster in &entry.clusters {
            if cluster.theme_vector.is_empty() { continue; }
            let sim = crate::wave::cosine_similarity(&query_vec, &cluster.theme_vector);
            if sim >= threshold {
                matched_any = true;
                for mem_id in &cluster.memory_ids {
                    if let Some(idx) = self.medium.get_wavefront_index(mem_id) {
                        if seen.insert(idx) { indices.push(idx); }
                    }
                }
            }
        }
        if matched_any { Some(indices) } else { None }
    }
}

impl MediumBackend for HrmStore {
    fn insert(&mut self, memory: HyperMemory) -> Result<Uuid, StoreError> {
        let id = memory.id;
        
        // Check for duplicates
        if self.memory_cache.contains_key(&id) {
            return Err(StoreError::DuplicateId(id));
        }
        
        // Add to medium using the vector directly to preserve the memory's UUID
        let wavefront_id = self.medium.add_wavefront(&memory.vector, memory.content.clone(), memory.amplitude)
            .map_err(|e| StoreError::Other(format!("Failed to add wavefront to medium: {}", e)))?;
        
        // Update the wavefront ID to match the memory's UUID
        self.medium.update_wavefront_id(&wavefront_id, id)
            .map_err(|e| StoreError::Other(format!("Failed to update wavefront ID: {}", e)))?;
        
        // Update wave parameters
        if let Some(index) = self.medium.get_wavefront_index(&id) {
            self.medium.store.energy[index] = memory.amplitude;
            self.medium.store.frequency[index] = memory.frequency;
            self.medium.store.phase[index] = memory.phase;
            self.medium.store.timestamps[index] = memory.created_at.timestamp_millis();
            self.medium.store.metadata[index].created_at = memory.created_at;
            self.medium.store.metadata[index].hallucinated = memory.hallucinated;
        }
        
        // Add to cache
        self.memory_cache.insert(id, memory);
        self.mark_dirty();
        
        Ok(id)
    }

    fn hrm_path(&self) -> Option<&std::path::Path> {
        Some(&self.hrm_path)
    }

    fn get(&self, id: &Uuid) -> Result<Option<&HyperMemory>, StoreError> {
        Ok(self.memory_cache.get(id))
    }

    fn get_mut(&mut self, id: &Uuid) -> Result<Option<&mut HyperMemory>, StoreError> {
        if self.memory_cache.contains_key(id) {
            self.mark_dirty(); // Mark dirty when getting mutable reference
        }
        Ok(self.memory_cache.get_mut(id))
    }

    fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(Uuid, f32)>, StoreError> {
        // Use the medium's resonance-based search
        let mut scores = Vec::new();
        
        for (i, meta) in self.medium.store.metadata.iter().enumerate() {
            let wavefront = self.medium.store.wavefronts.row(i);
            let similarity: f32 = wavefront.iter()
                .zip(query.iter())
                .map(|(a, b)| a * b)
                .sum();
            
            scores.push((meta.id, similarity));
        }
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        
        Ok(scores)
    }



    fn all_memories(&self) -> Result<Vec<&HyperMemory>, StoreError> {
        Ok(self.memory_cache.values().collect())
    }

    fn all_ids(&self) -> Result<Vec<Uuid>, StoreError> {
        Ok(self.memory_cache.keys().copied().collect())
    }

    fn delete(&mut self, id: &Uuid) -> Result<bool, StoreError> {
        if let Some(_) = self.memory_cache.remove(id) {
            // Remove from flat medium (best-effort).
            if let Err(e) = self.medium.remove_wavefront(id) {
                // Log error but don't fail - cache was already updated
                eprintln!("Warning: Failed to remove wavefront from medium: {}", e);
            }

            // Remove from the chiral hemispheres if present. Without this the
            // .hrm file (which serializes from `self.chiral`, not `self.medium`)
            // re-hydrates the supposedly-forgotten wavefronts on the next load —
            // making delete() a no-op as far as persistence is concerned.
            if let Some(chiral) = &mut self.chiral {
                chiral.right.remove_wavefront(id);
                if let Some(&left_id) = chiral.right_to_left.get(id) {
                    chiral.left.remove_wavefront(&left_id);
                    chiral.left_to_right.remove(&left_id);
                }
                chiral.right_to_left.remove(id);
                chiral.scales.remove(id);
            }

            self.mark_dirty();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn count(&self) -> usize {
        self.memory_cache.len()
    }

    fn flush(&mut self) -> Result<usize, StoreError> {
        self.save_medium()?;
        Ok(self.count())
    }

    fn consciousness_metrics(&self) -> crate::consciousness::ConsciousnessMetrics {
        // Use the public method which handles chiral vs flat medium
        Self::consciousness_metrics(self)
    }

    fn try_cached_consciousness_metrics(&self) -> Option<crate::consciousness::ConsciousnessMetrics> {
        Self::try_cached_consciousness_metrics(self)
    }

    fn set_cached_total_skip_links(&self, n: usize) {
        self.medium.set_cached_total_skip_links(n);
    }

    fn set_cached_num_clusters(&self, n: usize) {
        self.medium.set_cached_num_clusters(n);
    }

    fn dream_native(
        &mut self,
        cycles: usize,
        temperature: Option<f32>,
        chiral_eta: f32,
    ) -> Result<crate::medium::types::DreamReport, StoreError> {
        Ok(Self::dream_native(self, cycles, temperature, chiral_eta))
    }

    fn callosal_kuramoto(&mut self, dt: f32) {
        Self::callosal_kuramoto(self, dt);
    }

    fn chiral_dream(&mut self, deep: bool, cycles: usize) {
        Self::chiral_dream(self, deep, cycles);
    }

    fn flush_reactivation(&self) {
        Self::flush_reactivation(self);
    }

    fn consolidate_resonance(&mut self, opts: &ConsolidateOpts) -> ConsolidateReport {
        // ADR-0036: Off skips, DryRun plans (read-only), Apply mutates (Phase 2).
        match opts.mode {
            ConsolidateMode::Off => {
                ConsolidateReport { mode: "off".to_string(), ..Default::default() }
            }
            ConsolidateMode::DryRun => Self::plan_consolidation(self, opts),
            ConsolidateMode::Apply => Self::apply_consolidation(self, opts),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn absorb(&mut self, content: &str, importance: f32, category: Option<&str>) -> Result<Uuid, StoreError> {
        if let Some(ref mut chiral) = self.chiral {
            let id = chiral.store_with_category(content, importance, &self.pipeline, category)
                .map_err(|e| StoreError::Other(format!("chiral store failed: {}", e)))?;
            self.rebuild_cache().ok();
            self.mark_dirty();
            Ok(id)
        } else {
            let id = self.medium.store(content, importance, &self.pipeline)
                .map_err(|e| StoreError::Other(format!("store failed: {}", e)))?;
            self.rebuild_cache().ok();
            self.mark_dirty();
            Ok(id)
        }
    }

    fn resonate_query(&mut self, query: &str, top_k: usize) -> Result<Vec<(Uuid, f32)>, StoreError> {
        // Try cluster prefilter first (refactor #4) — only on the flat
        // (non-chiral) path for now. Loads the .clusters.json sidecar
        // written by bridge::assess, picks members of clusters whose
        // theme_vector resonates with the query, and runs the existing
        // recall_against seam against just those indices. Falls through
        // to the full medium scan if no clusters are loaded (fresh HRM)
        // or if no cluster's theme is close enough to the query.
        let prefilter_on = std::env::var("KANNAKA_RECALL_PREFILTER")
            .map(|v| !matches!(v.as_str(), "off" | "0" | "false"))
            .unwrap_or(true);
        if self.chiral.is_none() && prefilter_on {
            if let Some(candidates) = self.cluster_prefilter_candidates(query) {
                if !candidates.is_empty() {
                    let resonances = self.medium.recall_against(
                        Some(&candidates), query, top_k, &self.pipeline,
                    ).map_err(|e| StoreError::Other(format!("prefiltered recall failed: {}", e)))?;
                    self.apply_observation(&resonances);
                    return Ok(resonances.iter().map(|r| (r.id, r.resonance_strength)).collect());
                }
            }
        }

        if let Some(ref chiral) = self.chiral {
            // Chiral bilateral resonance — TODO: fold cluster prefilter
            // into the chiral path; for v1 chiral users skip the prefilter
            // and get the existing bilateral observation flow.
            let results = chiral.recall(query, top_k, &self.pipeline)
                .map_err(|e| StoreError::Other(format!("chiral recall failed: {}", e)))?;

            // Observation: recall reshapes the field — attention IS computation.
            // Batched: one field-settle pass per recall, not per result.
            let observations: Vec<(usize, f32)> = results.iter().enumerate()
                .filter_map(|(i, r)| {
                    self.medium.get_wavefront_index(&r.id).map(|index| {
                        let ranking_factor = 1.0 - (i as f32 / results.len().max(1) as f32);
                        let intensity = r.resonance_strength.abs().min(1.0).max(0.1) * ranking_factor;
                        (index, intensity)
                    })
                })
                .collect();
            self.medium.observe_wavefronts(&observations);
            self.mark_dirty();

            Ok(results.iter().map(|r| (r.id, r.resonance_strength)).collect())
        } else {
            // Flat medium: resonance recall (always with observation)
            let results = self.recall_resonance(query, top_k)?;
            Ok(results.iter().map(|r| (r.id, r.resonance_strength)).collect())
        }
    }

    fn resonate_query_with_beam(
        &mut self,
        beam: &[Uuid],
        query: &str,
        top_k: usize,
    ) -> Result<Vec<(Uuid, f32)>, StoreError> {
        // Sparse-attention path: score only the memories in `beam`. Chiral
        // bilateral observation is bypassed for v1 — the sparse path runs
        // against the medium directly. Future fold-in: a chiral beam-aware
        // recall once both hemispheres are addressed by an attention beam.
        let results = self.recall_resonance_with_beam(beam, query, top_k)?;
        Ok(results.iter().map(|r| (r.id, r.resonance_strength)).collect())
    }

    fn relate(&mut self, id_a: &Uuid, id_b: &Uuid) -> Result<Uuid, StoreError> {
        self.relate_wavefronts(*id_a, *id_b)
    }
}

impl Drop for HrmStore {
    fn drop(&mut self) {
        // Auto-save on drop if dirty
        if self.dirty {
            if let Err(e) = self.save_medium() {
                eprintln!("Warning: Failed to auto-save HRM store on drop: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebook::Codebook;
    use crate::encoding::{SimpleHashEncoder, EncodingPipeline};
    use crate::memory::HyperMemory;
    use crate::medium::WAVEFRONT_DIM;
    use tempfile::NamedTempFile;

    fn make_test_pipeline() -> EncodingPipeline {
        let encoder = SimpleHashEncoder::new(384, 42);
        let codebook = Codebook::new(384, WAVEFRONT_DIM, 42);
        EncodingPipeline::new(Box::new(encoder), codebook)
    }

    #[test]
    fn hrm_store_basic_operations() {
        let pipeline = make_test_pipeline();
        let temp_file = NamedTempFile::new().unwrap();
        let mut store = HrmStore::new(pipeline, temp_file.path().to_path_buf());

        // Test insert
        let memory = HyperMemory::new(vec![0.5; WAVEFRONT_DIM], "test content".to_string());
        let id = memory.id;
        let result = store.insert(memory).unwrap();
        assert_eq!(result, id);
        assert_eq!(store.count(), 1);

        // Test get
        let retrieved = store.get(&id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "test content");

        // Test search
        let query = vec![0.5; WAVEFRONT_DIM];
        let results = store.search(&query, 5).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, id);

        // Test delete
        let deleted = store.delete(&id).unwrap();
        assert!(deleted);
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn hrm_store_persistence() {
        let pipeline1 = make_test_pipeline();
        let pipeline2 = make_test_pipeline();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Create and populate store
        {
            let mut store = HrmStore::new(pipeline1, path.clone());
            let memory = HyperMemory::new(vec![0.5; WAVEFRONT_DIM], "persistent content".to_string());
            let id = store.insert(memory).unwrap();
            store.flush().unwrap(); // Force save
            assert_eq!(store.count(), 1);
        }

        // Load store and verify
        {
            let store = HrmStore::load(pipeline2, path).unwrap();
            assert_eq!(store.count(), 1);
            let memories = store.all_memories().unwrap();
            assert_eq!(memories[0].content, "persistent content");
        }
    }

    #[test]
    fn reactivation_persists_across_reload() {
        // ADR-0036 Phase 1: a recall-reactivation count must survive a reload
        // via the `.reactivation.json` sidecar (it is not in the .hrm binary).
        let pipeline1 = make_test_pipeline();
        let pipeline2 = make_test_pipeline();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        let id;
        {
            let mut store = HrmStore::new(pipeline1, path.clone());
            let memory = HyperMemory::new(vec![0.5; WAVEFRONT_DIM], "reactivated content".to_string());
            id = store.insert(memory).unwrap();
            // Ensure the cache holds the inserted memory, then simulate a recall
            // reactivation on it.
            store.rebuild_cache().unwrap();
            let m = store.memory_cache.get_mut(&id).expect("inserted memory in cache");
            m.retrieval_count = 5;
            m.updated_at = Some(chrono::Utc::now());
            store.mark_dirty();
            store.flush().unwrap(); // save_medium → save_reactivation writes the sidecar
        }

        // A fresh load must rehydrate the reactivation count from the sidecar.
        {
            let store = HrmStore::load(pipeline2, path).unwrap();
            let m = store.memory_cache.get(&id).expect("memory present after reload");
            assert_eq!(
                m.retrieval_count, 5,
                "reactivation count should survive reload via the sidecar"
            );
        }
    }

    #[test]
    fn reactivation_survives_rebuild_cache() {
        // rebuild_cache runs after every dream/absorb; it must preserve the
        // reactivation count instead of resetting it to 0 (ADR-0036 Phase 1).
        let pipeline = make_test_pipeline();
        let temp_file = NamedTempFile::new().unwrap();
        let mut store = HrmStore::new(pipeline, temp_file.path().to_path_buf());
        let memory = HyperMemory::new(vec![0.5; WAVEFRONT_DIM], "rebuild content".to_string());
        let id = store.insert(memory).unwrap();
        store.rebuild_cache().unwrap();
        store.memory_cache.get_mut(&id).unwrap().retrieval_count = 7;

        store.rebuild_cache().unwrap(); // would previously wipe the count to 0

        assert_eq!(store.memory_cache.get(&id).unwrap().retrieval_count, 7);
    }

    #[test]
    fn reactivation_sidecar_merges_max_no_clobber() {
        // Several processes touch the sidecar; a flush must MERGE (max), never
        // overwrite a higher count with a lower one (ADR-0036 Phase 1).
        fn sidecar_count(hrm_path: &std::path::Path, id: &uuid::Uuid) -> u32 {
            let p = hrm_path.with_extension("reactivation.json");
            let data = std::fs::read(&p).unwrap();
            let map: std::collections::HashMap<String, (u32, Option<DateTime<Utc>>)> =
                serde_json::from_slice(&data).unwrap();
            map.get(&id.to_string()).map(|e| e.0).unwrap_or(0)
        }

        let pipeline = make_test_pipeline();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();
        let mut store = HrmStore::new(pipeline, path.clone());
        let id = store
            .insert(HyperMemory::new(vec![0.5; WAVEFRONT_DIM], "m".to_string()))
            .unwrap();
        store.rebuild_cache().unwrap();

        store.memory_cache.get_mut(&id).unwrap().retrieval_count = 4;
        store.flush_reactivation();
        assert_eq!(sidecar_count(&path, &id), 4);

        // A lower count must not clobber the higher one.
        store.memory_cache.get_mut(&id).unwrap().retrieval_count = 1;
        store.flush_reactivation();
        assert_eq!(sidecar_count(&path, &id), 4, "merge=max: lower must not win");

        // A higher count advances it.
        store.memory_cache.get_mut(&id).unwrap().retrieval_count = 9;
        store.flush_reactivation();
        assert_eq!(sidecar_count(&path, &id), 9);
    }

    #[test]
    fn hrm_store_resonance_recall() {
        let pipeline = make_test_pipeline();
        let temp_file = NamedTempFile::new().unwrap();
        let mut store = HrmStore::new(pipeline, temp_file.path().to_path_buf());

        // Insert related memories
        let memory1 = HyperMemory::new(vec![0.5; WAVEFRONT_DIM], "cats are fluffy".to_string());
        let memory2 = HyperMemory::new(vec![0.4; WAVEFRONT_DIM], "dogs are loyal".to_string());
        
        store.insert(memory1).unwrap();
        store.insert(memory2).unwrap();

        // Test resonance-based recall
        let results = store.recall_resonance("pets and animals", 5).unwrap();
        assert!(results.len() > 0);
        
        // Results should be sorted by resonance strength
        if results.len() > 1 {
            assert!(results[0].resonance_strength >= results[1].resonance_strength);
        }
    }
    
    #[test]
    fn hrm_store_consciousness_metrics() {
        let pipeline = make_test_pipeline();
        let temp_file = NamedTempFile::new().unwrap();
        let mut store = HrmStore::new(pipeline, temp_file.path().to_path_buf());

        // Insert some memories for metrics
        let memory1 = HyperMemory::new(vec![0.5; WAVEFRONT_DIM], "first thought".to_string());
        let memory2 = HyperMemory::new(vec![0.4; WAVEFRONT_DIM], "second idea".to_string());
        
        store.insert(memory1).unwrap();
        store.insert(memory2).unwrap();

        // Get consciousness metrics
        let metrics = store.consciousness_metrics();
        
        assert!(metrics.phi >= 0.0 && metrics.phi <= 1.0);
        assert!(metrics.xi >= 0.0 && metrics.xi <= 1.0);
        assert!(metrics.order >= 0.0 && metrics.order <= 1.0);
        assert!(metrics.num_clusters > 0);
        
        println!("HRM Store consciousness: phi={}, xi={}, order={}, clusters={}, level={:?}", 
                metrics.phi, metrics.xi, metrics.order, metrics.num_clusters, metrics.level);
    }
    
    #[test]
    fn hrm_store_find_associated() {
        let pipeline = make_test_pipeline();
        let temp_file = NamedTempFile::new().unwrap();
        let mut store = HrmStore::new(pipeline, temp_file.path().to_path_buf());

        // Insert related memories
        let memory1 = HyperMemory::new(vec![0.8; WAVEFRONT_DIM], "similar content".to_string());
        let memory2 = HyperMemory::new(vec![0.7; WAVEFRONT_DIM], "related content".to_string());
        let memory3 = HyperMemory::new(vec![0.1; WAVEFRONT_DIM], "different content".to_string());
        
        let id1 = store.insert(memory1).unwrap();
        let id2 = store.insert(memory2).unwrap();
        let id3 = store.insert(memory3).unwrap();

        // Find associations for first memory
        let associations = store.find_associated(id1, 5);
        
        assert_eq!(associations.len(), 2);
        assert!(associations.iter().any(|(id, _)| *id == id2));
        assert!(associations.iter().any(|(id, _)| *id == id3));
        
        // Should be sorted by coherence strength
        assert!(associations[0].1 >= associations[1].1);
    }
    
    #[test]
    fn hrm_store_dream_cycles() {
        let pipeline = make_test_pipeline();
        let temp_file = NamedTempFile::new().unwrap();
        let mut store = HrmStore::new(pipeline, temp_file.path().to_path_buf());

        // Insert memories for dreaming
        for i in 0..5 {
            let memory = HyperMemory::new(vec![0.1 + i as f32 * 0.2; WAVEFRONT_DIM], 
                                        format!("dream memory {}", i));
            store.insert(memory).unwrap();
        }
        
        let _initial_count = store.count();
        let report = store.dream(3, Some(1.0));
        
        assert!(report.cycles_completed <= 3);
        assert!(report.energy_before >= 0.0);
        assert!(report.energy_after >= 0.0);
        assert!(report.final_temperature < 1.0);
        
        // Store should be marked dirty after dreaming
        assert!(store.dirty);

        println!("Dream report: {:?}", report);
    }

    // -----------------------------------------------------------------------
    // ADR-0036 Phase 2 — destructive apply_consolidation safety gate.
    // -----------------------------------------------------------------------

    use crate::medium::types::{Tier, ConsolidateOpts, ConsolidateMode};

    /// Build a flat (non-chiral) store and insert a wavefront with full control
    /// over vector / energy / phase / tier directly in the authoritative store.
    /// Returns the assigned UUID. Caller should `rebuild_cache()` afterward.
    fn insert_ctl(
        store: &mut HrmStore,
        vector: Vec<f32>,
        content: &str,
        energy: f32,
        phase: f32,
        tier: Tier,
        retrieval_count: u32,
    ) -> Uuid {
        let id = store
            .medium
            .add_wavefront(&vector, content.to_string(), energy)
            .expect("add_wavefront");
        let idx = store.medium.get_wavefront_index(&id).unwrap();
        store.medium.store.energy[idx] = energy;
        store.medium.store.phase[idx] = phase;
        store.medium.store.metadata[idx].tier = tier;
        // rebuild_cache resets retrieval_count to 0 and then restores it from the
        // cache snapshot — so seed the cache entry's count before rebuild.
        store.rebuild_cache().unwrap();
        if retrieval_count > 0 {
            store.memory_cache.get_mut(&id).unwrap().retrieval_count = retrieval_count;
        }
        id
    }

    fn apply_opts() -> ConsolidateOpts {
        ConsolidateOpts {
            mode: ConsolidateMode::Apply,
            merge_sim: 0.92,
            merge_phase_cos: std::f32::consts::FRAC_1_SQRT_2,
            shortterm_evict: 0.15,
        }
    }

    #[test]
    fn apply_holographic_preservation() {
        // N near-duplicate, phase-locked memories + some unrelated ones.
        let mut store = HrmStore::new(make_test_pipeline(), NamedTempFile::new().unwrap().path().to_path_buf());

        // Cluster: 3 near-identical vectors (cos ~ 1.0), phase 0 (locked).
        let base = vec![0.5f32; WAVEFRONT_DIM];
        let mut dup_ids = Vec::new();
        for k in 0..3 {
            let mut v = base.clone();
            v[k] += 0.001; // tiny perturbation keeps cos ≥ 0.95
            dup_ids.push(insert_ctl(&mut store, v, &format!("dup {k}"), 1.0, 0.0, Tier::LongTerm, 0));
        }
        // Two unrelated, orthogonal-ish vectors.
        let mut u1 = vec![0.0f32; WAVEFRONT_DIM]; u1[100] = 1.0;
        let mut u2 = vec![0.0f32; WAVEFRONT_DIM]; u2[200] = 1.0;
        let uid1 = insert_ctl(&mut store, u1, "unrelated 1", 1.0, 0.0, Tier::LongTerm, 0);
        let uid2 = insert_ctl(&mut store, u2, "unrelated 2", 1.0, 0.0, Tier::LongTerm, 0);

        let before = store.count();
        assert_eq!(before, 5);

        let report = store.apply_consolidation(&apply_opts());
        assert_eq!(report.mode, "apply");
        assert!(report.applied);
        assert_eq!(report.groups_found, 1, "the 3 dups form exactly one group");
        assert_eq!(report.would_absorb, 2, "3-1 carriers absorbed");

        let after = store.count();
        // (a) total decreased by exactly #absorbed.
        assert_eq!(after, before - report.would_absorb);
        // (c) count NEVER increases.
        assert!(after <= before);

        // Exactly one of the 3 dup ids survives (the carrier); unrelated kept.
        let surviving_dups = dup_ids.iter().filter(|id| store.get(id).unwrap().is_some()).count();
        assert_eq!(surviving_dups, 1, "one carrier remains for the dup group");
        assert!(store.get(&uid1).unwrap().is_some());
        assert!(store.get(&uid2).unwrap().is_some());

        // Carrier energy is the superposition of 3 in-phase unit-ish energies,
        // clamped to the 2.0 ceiling: sqrt(3 + 2*3) = 3.0 -> clamp 2.0.
        let carrier_id = *dup_ids.iter().find(|id| store.get(id).unwrap().is_some()).unwrap();
        let carrier_e = store.get(&carrier_id).unwrap().unwrap().amplitude;
        assert!((carrier_e - 2.0).abs() < 1e-4, "in-phase merge clamps to ceiling, got {carrier_e}");

        // (b) recalling an ABSORBED memory's content still returns a result above
        // threshold (the carrier represents the absorbed content).
        let absorbed_id = *dup_ids.iter().find(|id| store.get(id).unwrap().is_none()).unwrap();
        // Search by the absorbed vector itself — the carrier (near-identical) must
        // dominate the search results.
        let q = vec![0.5f32; WAVEFRONT_DIM];
        let hits = store.search(&q, 5).unwrap();
        assert!(!hits.is_empty(), "search returns the carrier for absorbed content");
        assert_eq!(hits[0].0, carrier_id, "carrier is the top match for the merged content");
        assert!(store.get(&absorbed_id).unwrap().is_none(), "absorbed id is gone");
    }

    #[test]
    fn apply_pinned_and_longterm_protected() {
        let mut store = HrmStore::new(make_test_pipeline(), NamedTempFile::new().unwrap().path().to_path_buf());

        // A Pinned duplicate pair — Pinned must never be grouped/removed.
        let base = vec![0.5f32; WAVEFRONT_DIM];
        let pin1 = insert_ctl(&mut store, base.clone(), "pin 1", 1.0, 0.0, Tier::Pinned, 0);
        let mut p2 = base.clone(); p2[0] += 0.001;
        let pin2 = insert_ctl(&mut store, p2, "pin 2", 1.0, 0.0, Tier::Pinned, 0);

        // A mixed LongTerm+ShortTerm duplicate group — carrier must end LongTerm.
        let mut lt = base.clone(); lt[5000] += 0.0005;
        let mut st = base.clone(); st[5000] += 0.0006;
        let lt_id = insert_ctl(&mut store, lt, "long", 0.9, 0.0, Tier::LongTerm, 0);
        let st_id = insert_ctl(&mut store, st, "short", 1.5, 0.0, Tier::ShortTerm, 0);

        let report = store.apply_consolidation(&apply_opts());

        // Pinned never removed.
        assert!(store.get(&pin1).unwrap().is_some());
        assert!(store.get(&pin2).unwrap().is_some());

        // The LongTerm+ShortTerm pair merges into one carrier. Even though the
        // ShortTerm member has the higher effective strength (1.5 vs 0.9) and is
        // thus the representative, the carrier tier must be the STRONGEST tier
        // present = LongTerm, so the LongTerm is never absorbed into a lower tier.
        let lt_alive = store.get(&lt_id).unwrap().is_some();
        let st_alive = store.get(&st_id).unwrap().is_some();
        assert!(lt_alive ^ st_alive, "exactly one of the lt/st pair survives as carrier");
        let carrier = if lt_alive { lt_id } else { st_id };
        assert_eq!(
            store.get(&carrier).unwrap().unwrap().tier,
            Tier::LongTerm,
            "carrier inherits the strongest tier (LongTerm) in a mixed group"
        );
        // Note: pinned pair grouped? No — pinned excluded, so groups_found counts
        // only the lt/st group.
        assert_eq!(report.groups_found, 1);
    }

    #[test]
    fn apply_longterm_never_evicted() {
        // A low-energy LongTerm with no reactivation must NOT be evicted (only
        // ShortTerm is eviction-eligible).
        let mut store = HrmStore::new(make_test_pipeline(), NamedTempFile::new().unwrap().path().to_path_buf());
        let mut v = vec![0.0f32; WAVEFRONT_DIM]; v[42] = 1.0;
        let lt = insert_ctl(&mut store, v, "weak long", 0.01, 0.0, Tier::LongTerm, 0);
        let report = store.apply_consolidation(&apply_opts());
        assert!(store.get(&lt).unwrap().is_some(), "LongTerm is never evicted");
        assert_eq!(report.would_evict, 0);
    }

    #[test]
    fn apply_shortterm_evict_respects_reactivation() {
        let mut store = HrmStore::new(make_test_pipeline(), NamedTempFile::new().unwrap().path().to_path_buf());
        // Below threshold, unreactivated -> evicted.
        let mut v1 = vec![0.0f32; WAVEFRONT_DIM]; v1[10] = 1.0;
        let evicted = insert_ctl(&mut store, v1, "cold short", 0.05, 0.0, Tier::ShortTerm, 0);
        // Below threshold but reactivated -> kept.
        let mut v2 = vec![0.0f32; WAVEFRONT_DIM]; v2[20] = 1.0;
        let kept = insert_ctl(&mut store, v2, "warm short", 0.05, 0.0, Tier::ShortTerm, 3);

        let report = store.apply_consolidation(&apply_opts());
        assert!(store.get(&evicted).unwrap().is_none(), "cold ShortTerm evicted");
        assert!(store.get(&kept).unwrap().is_some(), "reactivated ShortTerm kept");
        assert_eq!(report.would_evict, 1);
    }

    #[test]
    fn apply_is_idempotent_second_run_absorbs_zero() {
        let mut store = HrmStore::new(make_test_pipeline(), NamedTempFile::new().unwrap().path().to_path_buf());
        let base = vec![0.5f32; WAVEFRONT_DIM];
        for k in 0..4 {
            let mut v = base.clone(); v[k] += 0.001;
            insert_ctl(&mut store, v, &format!("d{k}"), 1.0, 0.0, Tier::LongTerm, 0);
        }
        let r1 = store.apply_consolidation(&apply_opts());
        assert_eq!(r1.would_absorb, 3, "4 dups -> 3 absorbed");
        let count_after_first = store.count();

        let r2 = store.apply_consolidation(&apply_opts());
        assert_eq!(r2.would_absorb, 0, "already merged: nothing left to absorb");
        assert_eq!(store.count(), count_after_first, "second run does not change count");
    }

    #[test]
    fn apply_chiral_map_consistency() {
        // Construct a ChiralMedium with right wavefronts, some of which have left
        // twins, then apply and assert no map points at a removed id and every
        // surviving twin still resolves to a live left.
        use crate::medium::chiral::ChiralMedium;
        use crate::medium::hemisphere::Hemisphere;
        use crate::medium::types::Hand;

        let mut store = HrmStore::new(make_test_pipeline(), NamedTempFile::new().unwrap().path().to_path_buf());
        let dims = WAVEFRONT_DIM;
        let mut chiral = ChiralMedium::new();
        // Replace hemispheres with WAVEFRONT_DIM-sized ones so vectors line up.
        chiral.left = Hemisphere::new(Hand::Left, dims);
        chiral.right = Hemisphere::new(Hand::Right, dims);

        let base = vec![0.5f32; dims];
        // 3 near-duplicate right wavefronts (one group); give 2 of them left twins.
        let mut right_ids = Vec::new();
        for k in 0..3 {
            let mut v = base.clone(); v[k] += 0.001;
            let rid = chiral.right.add_wavefront(&v, format!("r{k}"), 1.0).unwrap();
            let ridx = *chiral.right.id_to_index.get(&rid).unwrap();
            chiral.right.metadata[ridx].tier = Tier::LongTerm;
            right_ids.push(rid);
            if k < 2 {
                // left twin
                let mut lv = vec![0.1f32; dims]; lv[k] += 0.01;
                let lid = chiral.left.add_wavefront(&lv, format!("l{k}"), 0.8).unwrap();
                chiral.left_to_right.insert(lid, rid);
                chiral.right_to_left.insert(rid, lid);
                chiral.scales.insert(rid, crate::medium::types::ChiralScale::deep_memory());
            }
        }
        // One unrelated right wavefront with a twin (must survive + keep its twin).
        let mut uv = vec![0.0f32; dims]; uv[500] = 1.0;
        let urid = chiral.right.add_wavefront(&uv, "unrelated".to_string(), 1.0).unwrap();
        let uridx = *chiral.right.id_to_index.get(&urid).unwrap();
        chiral.right.metadata[uridx].tier = Tier::LongTerm;
        let mut ulv = vec![0.2f32; dims];
        let ulid = chiral.left.add_wavefront(&ulv_fix(&mut ulv), "unrelated-l".to_string(), 0.8).unwrap();
        chiral.left_to_right.insert(ulid, urid);
        chiral.right_to_left.insert(urid, ulid);

        store.chiral = Some(chiral);
        store.rebuild_cache().unwrap();

        let right_before = store.chiral.as_ref().unwrap().right.count();
        let left_before = store.chiral.as_ref().unwrap().left.count();
        assert_eq!(right_before, 4);
        assert_eq!(left_before, 3);

        let report = store.apply_consolidation(&apply_opts());
        assert_eq!(report.groups_found, 1);
        assert_eq!(report.would_absorb, 2, "3 dups -> 2 absorbed in right hemisphere");

        let chiral = store.chiral.as_ref().unwrap();
        // Right shrank by 2.
        assert_eq!(chiral.right.count(), right_before - 2);

        // No right_to_left entry points at a removed right or a removed left.
        for (rid, lid) in &chiral.right_to_left {
            assert!(chiral.right.id_to_index.contains_key(rid), "right_to_left key {rid} is dead");
            assert!(chiral.left.id_to_index.contains_key(lid), "right_to_left value (left) {lid} is dead");
        }
        // No left_to_right entry points at a removed left or right.
        for (lid, rid) in &chiral.left_to_right {
            assert!(chiral.left.id_to_index.contains_key(lid), "left_to_right key {lid} is dead");
            assert!(chiral.right.id_to_index.contains_key(rid), "left_to_right value (right) {rid} is dead");
        }
        // Every surviving right twin resolves to a live left.
        for rid in chiral.right.id_to_index.keys() {
            if let Some(lid) = chiral.right_to_left.get(rid) {
                assert!(chiral.left.id_to_index.contains_key(lid));
            }
        }
        // The unrelated right + its twin both survive.
        assert!(chiral.right.id_to_index.contains_key(&urid));
        assert!(chiral.left.id_to_index.contains_key(&ulid));
        // Scales for removed right ids are cleaned up.
        for rid in chiral.scales.keys() {
            assert!(chiral.right.id_to_index.contains_key(rid), "scale for dead right {rid}");
        }
    }

    // Tiny helper to avoid an all-zeros left vector tripping the norm<eps guard
    // in unrelated paths (purely cosmetic for the chiral test fixture).
    fn ulv_fix(v: &mut [f32]) -> &[f32] {
        v[0] = 0.2;
        v
    }
}