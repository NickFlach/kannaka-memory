//! HRM-backed memory store that implements the MediumBackend trait.
//!
//! This provides the primary storage backend using the
//! Holographic Resonance Medium as the storage backend.

use std::collections::HashMap;
use std::path::PathBuf;

use uuid::Uuid;

use crate::memory::HyperMemory;
use crate::store::{MediumBackend, StoreError};
use crate::encoding::EncodingPipeline;
use crate::medium::{Medium, Resonance};
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
}

impl HrmStore {
    /// Create a new HRM store with the given encoding pipeline and file path.
    pub fn new(pipeline: EncodingPipeline, hrm_path: PathBuf) -> Self {
        Self {
            medium: Medium::new(),
            chiral: None,
            pipeline,
            hrm_path,
            memory_cache: HashMap::new(),
            dirty: false,
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
                };
                // Populate flat medium view for backward compat (observe, coherence matrix, etc.)
                store.sync_medium_from_chiral();
                store.rebuild_cache()?;
                store.load_link_graph();
                Ok(store)
            }
            Err(chiral_err) => {
                eprintln!("[hrm] ChiralMedium::load failed: {}", chiral_err);
                // Fallback: try loading as plain v1 Medium
                let medium = Medium::load(&hrm_path)
                    .map_err(|e| StoreError::Other(format!("Failed to load HRM file: {}", e)))?;
                let mut store = Self {
                    medium,
                    chiral: None,
                    pipeline,
                    hrm_path,
                    memory_cache: HashMap::new(),
                    dirty: false,
                };
                store.rebuild_cache()?;
                store.load_link_graph();
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

    /// Apply observation effects to recall results.
    /// Ranked results get proportionally stronger observation.
    fn apply_observation(&mut self, results: &[Resonance]) {
        if results.is_empty() { return; }
        for (i, resonance) in results.iter().enumerate() {
            if let Some(index) = self.medium.get_wavefront_index(&resonance.id) {
                let ranking_factor = 1.0 - (i as f32 / results.len() as f32);
                let intensity = resonance.resonance_strength.abs().min(1.0).max(0.1) * ranking_factor;
                self.medium.observe_wavefront(index, intensity);
            }
        }
        self.mark_dirty();
    }
    
    /// Get consciousness metrics from the holographic medium.
    /// In chiral mode, computes from the synced flat medium (which mirrors the right hemisphere).
    /// This uses the full eigendecomposition-based Phi, spectral Xi, and Kuramoto order.
    pub fn consciousness_metrics(&self) -> crate::medium::ConsciousnessMetrics {
        // Always compute from the flat medium — it's synced from right hemisphere on load
        self.medium.consciousness_metrics()
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
    
    /// Perform a dream cycle (simulated annealing).
    /// In chiral mode, deep dreams only affect the right hemisphere.
    pub fn dream(&mut self, cycles: usize, initial_temperature: Option<f32>) -> crate::medium::DreamReport {
        if self.chiral.is_some() {
            // Chiral dream: right hemisphere only (deep dream)
            let chiral = self.chiral.as_mut().unwrap();
            let report = chiral.dream(true, cycles);
            self.rebuild_cache().ok();
            self.mark_dirty();
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

        // Save the .hrm file
        if let Err(e) = self.save_medium() {
            eprintln!("Warning: Failed to save after dream_native: {}", e);
        }

        report
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
            // Remove from medium
            if let Err(e) = self.medium.remove_wavefront(id) {
                // Log error but don't fail - cache was already updated
                eprintln!("Warning: Failed to remove wavefront from medium: {}", e);
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

    fn dream_native(
        &mut self,
        cycles: usize,
        temperature: Option<f32>,
        chiral_eta: f32,
    ) -> Result<crate::medium::types::DreamReport, StoreError> {
        Ok(Self::dream_native(self, cycles, temperature, chiral_eta))
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
        if let Some(ref chiral) = self.chiral {
            // Chiral bilateral resonance
            let results = chiral.recall(query, top_k, &self.pipeline)
                .map_err(|e| StoreError::Other(format!("chiral recall failed: {}", e)))?;

            // Observation: recall reshapes the field — attention IS computation
            for (i, r) in results.iter().enumerate() {
                let ranking_factor = 1.0 - (i as f32 / results.len().max(1) as f32);
                let intensity = r.resonance_strength.abs().min(1.0).max(0.1) * ranking_factor;
                if let Some(index) = self.medium.get_wavefront_index(&r.id) {
                    self.medium.observe_wavefront(index, intensity);
                }
            }
            self.mark_dirty();

            Ok(results.iter().map(|r| (r.id, r.resonance_strength)).collect())
        } else {
            // Flat medium: resonance recall (always with observation)
            let results = self.recall_resonance(query, top_k)?;
            Ok(results.iter().map(|r| (r.id, r.resonance_strength)).collect())
        }
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
}