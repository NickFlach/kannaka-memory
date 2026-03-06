//! Dolt-backed memory store with hybrid architecture.
//!
//! Loads all memories into an in-memory HashMap on startup for fast reads
//! (required by MemoryStore trait which returns &HyperMemory references),
//! and writes through to Dolt SQL server on mutations.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use mysql::*;
use mysql::prelude::*;
use serde_json;
use uuid::Uuid;

use crate::memory::HyperMemory;
use crate::skip_link::SkipLink;
use crate::store::{MemoryStore, StoreError};
use crate::wave::cosine_similarity;

/// Hybrid Dolt-backed memory store with in-memory cache.
pub struct DoltMemoryStore {
    /// In-memory cache for fast reads (trait requires &HyperMemory returns)
    cache: HashMap<Uuid, HyperMemory>,
    /// MySQL connection pool to Dolt SQL server
    pool: Pool,
    /// Auto-commit settings
    auto_commit: bool,
    /// Number of pending changes since last commit
    pending_changes: usize,
    /// Commit every N changes (if auto_commit is true)
    commit_threshold: usize,
}

impl DoltMemoryStore {
    /// Create a new store with the given MySQL connection pool.
    /// Loads all existing memories from Dolt into the in-memory cache.
    pub fn new(pool: Pool) -> Result<Self, StoreError> {
        let mut store = Self {
            cache: HashMap::new(),
            pool,
            auto_commit: true,
            pending_changes: 0,
            commit_threshold: 10,
        };
        
        let count = store.load_from_dolt()?;
        eprintln!("DoltMemoryStore: loaded {} memories from database", count);
        
        Ok(store)
    }

    /// Load all memories from Dolt into the in-memory cache.
    /// Returns the number of memories loaded.
    pub fn load_from_dolt(&mut self) -> Result<usize, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        // Load memories - break down into simpler queries
        // Use String for all text fields to ensure proper sizing
        let memories_basic: Vec<(String, String, f32, f32, f32, f32, String, u8, bool)> = 
            conn.query("SELECT id, content, amplitude, frequency, phase, decay_rate, created_at, layer_depth, hallucinated FROM memories")
            .map_err(|e| StoreError::Other(format!("Failed to query memories basic: {}", e)))?;

        let memories_extended: Vec<(String, Option<String>, String, Option<String>, Option<String>)> = 
            conn.query("SELECT id, parents, vector_data, xi_signature, geometry FROM memories")
            .map_err(|e| StoreError::Other(format!("Failed to query memories extended: {}", e)))?;

        // Load skip links
        let skip_links: Vec<(String, String, f32, String)> = 
            conn.query("SELECT source_id, target_id, weight, link_type FROM skip_links")
            .map_err(|e| StoreError::Other(format!("Failed to query skip_links: {}", e)))?;

        // Group skip links by source_id
        let mut skip_links_map: HashMap<Uuid, Vec<SkipLink>> = HashMap::new();
        for (source_id, target_id, weight, link_type) in skip_links {
            let source_uuid = Uuid::parse_str(&source_id)
                .map_err(|e| StoreError::Other(format!("Invalid source UUID: {}", e)))?;
            let target_uuid = Uuid::parse_str(&target_id)
                .map_err(|e| StoreError::Other(format!("Invalid target UUID: {}", e)))?;
            
            // Parse span from link_type (format: "span_N")
            let span = link_type.strip_prefix("span_")
                .and_then(|s| s.parse::<u8>().ok())
                .unwrap_or(0);

            // Create a minimal resonance key (since we don't store the full key)
            // This is a compromise - the user said we don't store resonance_key because it's too large
            let resonance_key = vec![0.0; 100]; // Placeholder - we'll need to reconstruct this somehow

            let skip_link = SkipLink {
                target_id: target_uuid,
                strength: weight,
                resonance_key,
                span,
            };

            skip_links_map.entry(source_uuid).or_default().push(skip_link);
        }

        // Create lookup map for extended data
        let mut extended_data: HashMap<String, (Option<String>, String, Option<String>, Option<String>)> = HashMap::new();
        for (id, parents_json, vector_json, xi_signature_json, geometry_json) in memories_extended {
            extended_data.insert(id, (parents_json, vector_json, xi_signature_json, geometry_json));
        }

        // Process memories and add to cache
        let mut loaded_count = 0;
        for (id, content, amplitude, frequency, phase, decay_rate, created_at_str, layer_depth, hallucinated) in memories_basic {
            // Parse datetime string to DateTime<Utc>
            let created_at = DateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                .map_err(|e| StoreError::Other(format!("Failed to parse datetime: {}", e)))?
                .with_timezone(&Utc);
            
            // Get extended data
            let (parents_json, vector_json, xi_signature_json, geometry_json) = extended_data
                .remove(&id)
                .unwrap_or((None, "[]".to_string(), None, None));
            let uuid = Uuid::parse_str(&id)
                .map_err(|e| StoreError::Other(format!("Invalid memory UUID: {}", e)))?;

            // Deserialize vector from JSON
            let vector: Vec<f32> = serde_json::from_str(&vector_json)
                .map_err(|e| StoreError::Other(format!("Failed to deserialize vector: {}", e)))?;

            // Deserialize xi_signature from JSON (optional)
            let xi_signature = if let Some(xi_json) = xi_signature_json {
                serde_json::from_str(&xi_json)
                    .map_err(|e| StoreError::Other(format!("Failed to deserialize xi_signature: {}", e)))?
            } else {
                Vec::new()
            };

            // Deserialize parents from JSON (optional)
            let parents = if let Some(parents_str) = parents_json {
                serde_json::from_str(&parents_str)
                    .map_err(|e| StoreError::Other(format!("Failed to deserialize parents: {}", e)))?
            } else {
                Vec::new()
            };

            // Deserialize geometry from JSON (optional)
            let geometry = if let Some(geom_json) = geometry_json {
                Some(serde_json::from_str(&geom_json)
                    .map_err(|e| StoreError::Other(format!("Failed to deserialize geometry: {}", e)))?)
            } else {
                None
            };

            // Get skip links for this memory
            let connections = skip_links_map.remove(&uuid).unwrap_or_default();

            let memory = HyperMemory {
                id: uuid,
                vector,
                amplitude,
                frequency,
                phase,
                decay_rate,
                created_at,
                layer_depth,
                connections,
                content,
                hallucinated,
                parents,
                geometry,
                xi_signature,
            };

            self.cache.insert(uuid, memory);
            loaded_count += 1;
        }

        Ok(loaded_count)
    }

    /// Sync a single memory to Dolt (upsert operation).
    pub fn sync_memory_to_dolt(&mut self, memory: &HyperMemory) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        // Serialize vectors to JSON
        let vector_json = serde_json::to_string(&memory.vector)
            .map_err(|e| StoreError::Other(format!("Failed to serialize vector: {}", e)))?;
        
        let xi_signature_json = if memory.xi_signature.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&memory.xi_signature)
                .map_err(|e| StoreError::Other(format!("Failed to serialize xi_signature: {}", e)))?)
        };

        // Serialize parents to JSON
        let parents_json = if memory.parents.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&memory.parents)
                .map_err(|e| StoreError::Other(format!("Failed to serialize parents: {}", e)))?)
        };

        // Serialize geometry to JSON
        let geometry_json = if let Some(ref geom) = memory.geometry {
            Some(serde_json::to_string(geom)
                .map_err(|e| StoreError::Other(format!("Failed to serialize geometry: {}", e)))?)
        } else {
            None
        };

        // Break down into smaller parameter sets to avoid mysql crate tuple limitations
        // First upsert basic memory data
        // Convert DateTime to string for mysql compatibility
        let created_at_str = memory.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
        conn.exec_drop(
            r"INSERT INTO memories (id, content, amplitude, frequency, phase, decay_rate, created_at, layer_depth, hallucinated) 
              VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
              ON DUPLICATE KEY UPDATE
              content = VALUES(content), amplitude = VALUES(amplitude), frequency = VALUES(frequency),
              phase = VALUES(phase), decay_rate = VALUES(decay_rate), layer_depth = VALUES(layer_depth),
              hallucinated = VALUES(hallucinated)",
            (&memory.id.to_string(), &memory.content, memory.amplitude, memory.frequency, memory.phase, memory.decay_rate, &created_at_str, memory.layer_depth, memory.hallucinated)
        ).map_err(|e| StoreError::Other(format!("Failed to upsert memory basic: {}", e)))?;

        // Update extended fields  
        conn.exec_drop(
            r"UPDATE memories SET parents = ?, vector_data = ?, xi_signature = ?, geometry = ? WHERE id = ?",
            (&parents_json, &vector_json, &xi_signature_json, &geometry_json, &memory.id.to_string())
        ).map_err(|e| StoreError::Other(format!("Failed to update memory extended: {}", e)))?;

        // Delete existing skip links for this memory
        conn.exec_drop(
            "DELETE FROM skip_links WHERE source_id = ?",
            (&memory.id.to_string(),)
        ).map_err(|e| StoreError::Other(format!("Failed to delete old skip_links: {}", e)))?;

        // Insert new skip links
        for link in &memory.connections {
            let link_type = format!("span_{}", link.span);
            conn.exec_drop(
                "INSERT INTO skip_links (source_id, target_id, weight, link_type, created_at) VALUES (?, ?, ?, ?, NOW())",
                (&memory.id.to_string(), &link.target_id.to_string(), link.strength, &link_type)
            ).map_err(|e| StoreError::Other(format!("Failed to insert skip_link: {}", e)))?;
        }

        self.pending_changes += 1;

        // Auto-commit if threshold reached
        if self.auto_commit && self.pending_changes >= self.commit_threshold {
            self.commit("Auto-commit after mutations")?;
        }

        Ok(())
    }

    /// Delete a memory from Dolt.
    pub fn delete_from_dolt(&mut self, id: &Uuid) -> Result<bool, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        // Delete skip links first (foreign key constraint)
        conn.exec_drop(
            "DELETE FROM skip_links WHERE source_id = ? OR target_id = ?",
            (&id.to_string(), &id.to_string())
        ).map_err(|e| StoreError::Other(format!("Failed to delete skip_links: {}", e)))?;

        // Delete memory - first check if it exists
        let exists: Vec<(u32,)> = conn.exec(
            "SELECT COUNT(*) FROM memories WHERE id = ?",
            (&id.to_string(),)
        ).map_err(|e| StoreError::Other(format!("Failed to check memory existence: {}", e)))?;
        
        let memory_existed = exists.first().map_or(0, |row| row.0) > 0;
        
        if memory_existed {
            conn.exec_drop(
                "DELETE FROM memories WHERE id = ?",
                (&id.to_string(),)
            ).map_err(|e| StoreError::Other(format!("Failed to delete memory: {}", e)))?;
        }

        self.pending_changes += 1;

        // Auto-commit if threshold reached
        if self.auto_commit && self.pending_changes >= self.commit_threshold {
            self.commit("Auto-commit after mutations")?;
        }

        Ok(memory_existed)
    }

    /// Commit changes to Dolt version control.
    pub fn commit(&mut self, message: &str) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        // Stage all changes
        conn.exec_drop("CALL DOLT_ADD('.')", ())
            .map_err(|e| StoreError::Other(format!("Failed to stage changes: {}", e)))?;

        // Commit with message
        conn.exec_drop("CALL DOLT_COMMIT('-m', ?)", (message,))
            .map_err(|e| StoreError::Other(format!("Failed to commit: {}", e)))?;

        self.pending_changes = 0;
        Ok(())
    }

    /// Create a new branch.
    pub fn branch(&self, name: &str) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        conn.exec_drop("CALL DOLT_BRANCH(?)", (name,))
            .map_err(|e| StoreError::Other(format!("Failed to create branch: {}", e)))?;

        Ok(())
    }

    /// Switch to a different branch.
    pub fn checkout(&self, name: &str) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        conn.exec_drop("CALL DOLT_CHECKOUT(?)", (name,))
            .map_err(|e| StoreError::Other(format!("Failed to checkout branch: {}", e)))?;

        Ok(())
    }

    /// Set auto-commit behavior.
    pub fn set_auto_commit(&mut self, enabled: bool, threshold: usize) {
        self.auto_commit = enabled;
        self.commit_threshold = threshold;
    }

    /// Get pending changes count.
    pub fn pending_changes(&self) -> usize {
        self.pending_changes
    }
}

impl MemoryStore for DoltMemoryStore {
    fn insert(&mut self, memory: HyperMemory) -> Result<Uuid, StoreError> {
        let id = memory.id;
        
        // Check for duplicate in cache
        if self.cache.contains_key(&id) {
            return Err(StoreError::DuplicateId(id));
        }

        // Sync to Dolt first
        self.sync_memory_to_dolt(&memory)?;

        // Add to cache
        self.cache.insert(id, memory);

        Ok(id)
    }

    fn get(&self, id: &Uuid) -> Result<Option<&HyperMemory>, StoreError> {
        Ok(self.cache.get(id))
    }

    fn get_mut(&mut self, id: &Uuid) -> Result<Option<&mut HyperMemory>, StoreError> {
        Ok(self.cache.get_mut(id))
    }

    fn search(&self, query: &[f32], top_k: usize) -> Result<Vec<(Uuid, f32)>, StoreError> {
        let mut scored: Vec<(Uuid, f32)> = self
            .cache
            .values()
            .map(|m| (m.id, cosine_similarity(query, &m.vector)))
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        Ok(scored)
    }

    fn search_with_wave(
        &self,
        query: &[f32],
        top_k: usize,
        now: DateTime<Utc>,
    ) -> Result<Vec<(Uuid, f32)>, StoreError> {
        let mut scored: Vec<(Uuid, f32)> = self
            .cache
            .values()
            .map(|m| {
                let sim = cosine_similarity(query, &m.vector);
                let strength = m.effective_strength(now);
                (m.id, sim * strength)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(top_k);
        Ok(scored)
    }

    fn all_memories(&self) -> Result<Vec<&HyperMemory>, StoreError> {
        Ok(self.cache.values().collect())
    }

    fn all_ids(&self) -> Result<Vec<Uuid>, StoreError> {
        Ok(self.cache.keys().copied().collect())
    }

    fn delete(&mut self, id: &Uuid) -> Result<bool, StoreError> {
        // Remove from cache first
        let was_present = self.cache.remove(id).is_some();
        
        if was_present {
            // Remove from Dolt
            self.delete_from_dolt(id)?;
        }
        
        Ok(was_present)
    }

    fn count(&self) -> usize {
        self.cache.len()
    }
}