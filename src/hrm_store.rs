//! HRM-backed memory store that implements the MemoryStore trait.
//!
//! This provides a drop-in replacement for DoltMemoryStore using the new
//! Holographic Resonance Medium as the storage backend.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::memory::HyperMemory;
use crate::store::{MemoryStore, StoreError};
use crate::encoding::EncodingPipeline;
use crate::medium::{Medium, MediumError, Resonance, WavefrontMeta};

/// HRM-backed memory store that implements the MemoryStore trait.
/// 
/// This wraps a Medium and provides the familiar MemoryStore interface,
/// allowing HRM to be used as a drop-in replacement for other storage backends.
#[cfg(feature = "hrm")]
pub struct HrmStore {
    /// The underlying holographic resonance medium
    medium: Medium,
    /// Encoding pipeline for text → hypervector conversion
    pipeline: EncodingPipeline,
    /// Path to the .hrm file for persistence
    hrm_path: PathBuf,
    /// In-memory cache mapping UUID → HyperMemory for compatibility
    memory_cache: HashMap<Uuid, HyperMemory>,
    /// Dirty flag to track when the medium needs saving
    dirty: bool,
}

#[cfg(feature = "hrm")]
impl HrmStore {
    /// Create a new HRM store with the given encoding pipeline and file path.
    pub fn new(pipeline: EncodingPipeline, hrm_path: PathBuf) -> Self {
        Self {
            medium: Medium::new(),
            pipeline,
            hrm_path,
            memory_cache: HashMap::new(),
            dirty: false,
        }
    }

    /// Load an existing HRM store from a .hrm file.
    pub fn load(pipeline: EncodingPipeline, hrm_path: PathBuf) -> Result<Self, StoreError> {
        let medium = Medium::load(&hrm_path)
            .map_err(|e| StoreError::Other(format!("Failed to load HRM file: {}", e)))?;
        
        let mut store = Self {
            medium,
            pipeline,
            hrm_path,
            memory_cache: HashMap::new(),
            dirty: false,
        };
        
        // Rebuild the memory cache from the medium
        store.rebuild_cache()?;
        
        Ok(store)
    }

    /// Rebuild the memory cache from the medium data.
    fn rebuild_cache(&mut self) -> Result<(), StoreError> {
        self.memory_cache.clear();
        
        for (i, meta) in self.medium.metadata.iter().enumerate() {
            // Reconstruct HyperMemory from medium data
            let vector = self.medium.wavefronts.row(i).to_vec();
            
            let memory = HyperMemory {
                id: meta.id,
                vector,
                amplitude: self.medium.energy[i],
                frequency: self.medium.frequency[i],
                phase: self.medium.phase[i],
                decay_rate: 0.001, // Default decay rate (not stored in medium)
                created_at: meta.created_at,
                layer_depth: 0, // Not stored in medium, could be derived from energy
                connections: Vec::new(), // Skip links are emergent, not stored
                content: meta.content.clone(),
                hallucinated: meta.hallucinated,
                parents: Vec::new(), // Could be tracked in future medium versions
                geometry: None, // Not stored in medium
                xi_signature: Vec::new(), // Could be computed on-demand
                origin_agent: "local".to_string(),
                sync_version: 0,
                merge_history: Vec::new(),
                last_consolidated_at: None,
                disputed: false,
                updated_at: Some(meta.created_at),
                retrieval_count: 0,
            };
            
            self.memory_cache.insert(meta.id, memory);
        }
        
        Ok(())
    }

    /// Sync any mutations made via `get_mut()` back to the medium tensor.
    ///
    /// The `MemoryStore` trait returns `&mut HyperMemory`, so callers can mutate
    /// wave parameters (amplitude, phase, frequency) on the cached copy. This
    /// method writes those changes back to the authoritative tensor storage.
    fn sync_cache_to_medium(&mut self) {
        for (id, mem) in &self.memory_cache {
            if let Some(index) = self.medium.get_wavefront_index(id) {
                self.medium.energy[index] = mem.amplitude;
                self.medium.frequency[index] = mem.frequency;
                self.medium.phase[index] = mem.phase;
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

        self.medium.save(&self.hrm_path)
            .map_err(|e| StoreError::Other(format!("Failed to save HRM file: {}", e)))?;

        self.dirty = false;
        Ok(())
    }

    /// Mark the store as dirty (needing save).
    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Get the underlying medium (read-only).
    pub fn medium(&self) -> &Medium {
        &self.medium
    }

    /// Perform resonance-based recall using the medium directly.
    pub fn recall_resonance(&self, query: &str, top_k: usize) -> Result<Vec<Resonance>, StoreError> {
        self.medium.recall(query, top_k, &self.pipeline)
            .map_err(|e| StoreError::Other(format!("Resonance recall failed: {}", e)))
    }

    /// Perform resonance-based recall with observation effects.
    /// 
    /// This version calls observe_wavefront on each returned result, implementing
    /// the attention-as-observation mechanics where recall reshapes the field.
    pub fn recall_resonance_with_observation(&mut self, query: &str, top_k: usize) -> Result<Vec<Resonance>, StoreError> {
        let results = self.medium.recall(query, top_k, &self.pipeline)
            .map_err(|e| StoreError::Other(format!("Resonance recall failed: {}", e)))?;

        // Apply observation to each result
        for (i, resonance) in results.iter().enumerate() {
            if let Some(index) = self.medium.get_wavefront_index(&resonance.id) {
                // Observation intensity based on resonance strength and ranking
                let ranking_factor = 1.0 - (i as f32 / results.len() as f32); // Higher rank = more intensity
                let base_intensity = resonance.resonance_strength.abs().min(1.0).max(0.1);
                let intensity = base_intensity * ranking_factor;
                
                self.medium.observe_wavefront(index, intensity);
            }
        }

        self.mark_dirty();
        Ok(results)
    }

    /// Perform vector search with observation effects.
    pub fn search_with_observation(&mut self, query: &[f32], top_k: usize) -> Result<Vec<(Uuid, f32)>, StoreError> {
        // First get standard search results
        let results = self.search(query, top_k)?;

        // Apply observation to each result
        for (i, (id, similarity)) in results.iter().enumerate() {
            if let Some(index) = self.medium.get_wavefront_index(id) {
                // Observation intensity proportional to similarity and ranking
                let ranking_factor = 1.0 - (i as f32 / results.len() as f32);
                let intensity = (similarity.abs() * ranking_factor).max(0.1).min(1.0);
                
                self.medium.observe_wavefront(index, intensity);
            }
        }

        self.mark_dirty();
        Ok(results)
    }
    
    /// Get consciousness metrics from the holographic medium.
    #[cfg(feature = "hrm")]
    pub fn consciousness_metrics(&self) -> crate::medium::ConsciousnessMetrics {
        self.medium.consciousness_metrics()
    }
    
    /// Find memories associated with a given memory through emergent coherence.
    #[cfg(feature = "hrm")]
    pub fn find_associated(&self, id: Uuid, top_k: usize) -> Vec<(Uuid, f32)> {
        self.medium.find_associated(id, top_k)
    }
    
    /// Apply dynamics to the medium (wave evolution).
    #[cfg(feature = "hrm")]
    pub fn apply_dynamics(&mut self, dt: f32) {
        self.medium.apply_dynamics(dt);
        self.mark_dirty();
    }
    
    /// Perform a dream cycle (simulated annealing).
    #[cfg(feature = "hrm")]
    pub fn dream(&mut self, cycles: usize, initial_temperature: Option<f32>) -> crate::medium::DreamReport {
        let report = self.medium.dream(cycles, initial_temperature);
        self.mark_dirty();
        report
    }
}

#[cfg(feature = "hrm")]
impl MemoryStore for HrmStore {
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
            self.medium.energy[index] = memory.amplitude;
            self.medium.frequency[index] = memory.frequency;
            self.medium.phase[index] = memory.phase;
            self.medium.timestamps[index] = memory.created_at.timestamp_millis();
            self.medium.metadata[index].created_at = memory.created_at;
            self.medium.metadata[index].hallucinated = memory.hallucinated;
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
        
        for (i, meta) in self.medium.metadata.iter().enumerate() {
            let wavefront = self.medium.wavefronts.row(i);
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

    fn search_with_wave(
        &self,
        query: &[f32],
        top_k: usize,
        now: DateTime<Utc>,
    ) -> Result<Vec<(Uuid, f32)>, StoreError> {
        // Use the medium's wave-modulated search
        let effective_strengths = self.medium.effective_strength(Some(now));
        let mut scores = Vec::new();
        
        for (i, meta) in self.medium.metadata.iter().enumerate() {
            let wavefront = self.medium.wavefronts.row(i);
            let similarity: f32 = wavefront.iter()
                .zip(query.iter())
                .map(|(a, b)| a * b)
                .sum();
            
            let wave_score = similarity * effective_strengths[i];
            scores.push((meta.id, wave_score));
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

    fn hrm_consciousness_metrics(&self) -> Option<crate::consciousness::ConsciousnessMetrics> {
        Some(self.medium.consciousness_metrics())
    }
}

#[cfg(feature = "hrm")]
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

#[cfg(feature = "hrm")]
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
    fn hrm_store_wave_search() {
        let pipeline = make_test_pipeline();
        let temp_file = NamedTempFile::new().unwrap();
        let mut store = HrmStore::new(pipeline, temp_file.path().to_path_buf());

        // Insert memory with specific wave parameters
        let mut memory = HyperMemory::new(vec![0.5; WAVEFRONT_DIM], "wave test".to_string());
        memory.amplitude = 1.0;
        memory.frequency = 2.0;
        memory.phase = 0.5;
        
        let id = store.insert(memory).unwrap();

        // Test wave-modulated search
        let query = vec![0.5; WAVEFRONT_DIM];
        let now = Utc::now();
        let results = store.search_with_wave(&query, 5, now).unwrap();
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, id);
        assert!(results[0].1 > 0.0); // Should have positive wave score
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