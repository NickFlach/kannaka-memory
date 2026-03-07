//! Dolt-backed memory store with hybrid architecture.
//!
//! Loads all memories into an in-memory HashMap on startup for fast reads
//! (required by MemoryStore trait which returns &HyperMemory references),
//! and writes through to Dolt SQL server on mutations.
//!
//! # Devil's Advocate audit findings fixed here (vs original Phase 1):
//! - **Datetime parsing**: `parse_from_str` with `%Y-%m-%d %H:%M:%S` requires timezone
//!   info in the format; fixed to use `NaiveDateTime::parse_from_str` + `.and_utc()`.
//! - **resonance_key placeholder**: `vec![0.0; 100]` was wrong dim and semantically
//!   incorrect — all other callers use `Vec::new()`; fixed accordingly.
//! - **Delete atomicity**: cache was cleared before Dolt, leaving an inconsistent state
//!   on Dolt failure; fixed to attempt Dolt first, then evict from cache.
//! - **get_mut mutations not persisted**: mutations via `get_mut` were never written back;
//!   fixed with a dirty-set (`dirty_set: HashSet<Uuid>`) + `flush_dirty()` / `update()`.

use std::collections::{HashMap, HashSet};
use std::env;

use chrono::{DateTime, NaiveDateTime, Utc};
use mysql::*;
use mysql::prelude::*;
use serde_json;
use uuid::Uuid;

use crate::memory::HyperMemory;
use crate::skip_link::SkipLink;
use crate::store::{MemoryStore, StoreError};
use crate::wave::cosine_similarity;

// ---------------------------------------------------------------------------
// DoltConfig — Phase 3: configuration from environment variables
// ---------------------------------------------------------------------------

/// Connection configuration for a Dolt SQL server.
///
/// Build from environment variables with [`DoltConfig::from_env`] or
/// [`DoltConfig::try_from_env`].
///
/// | Variable            | Default       | Description                    |
/// |---------------------|---------------|--------------------------------|
/// | `DOLT_HOST`         | `127.0.0.1`   | Dolt SQL server hostname       |
/// | `DOLT_PORT`         | `3307`        | Dolt SQL server port           |
/// | `DOLT_DB`           | `kannaka_memory` | Database name               |
/// | `DOLT_USER`         | `root`        | Database user                  |
/// | `DOLT_PASSWORD`     | *(empty)*     | Database password              |
/// | `DOLT_AUTO_COMMIT`  | `true`        | Auto-commit after N changes    |
/// | `DOLT_COMMIT_THRESHOLD` | `10`      | Changes between auto-commits   |
#[derive(Debug, Clone)]
pub struct DoltConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
    pub auto_commit: bool,
    pub commit_threshold: usize,
}

impl Default for DoltConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3307,
            database: "kannaka_memory".to_string(),
            user: "root".to_string(),
            password: String::new(),
            auto_commit: true,
            commit_threshold: 10,
        }
    }
}

impl DoltConfig {
    /// Build configuration from environment variables, falling back to defaults.
    pub fn from_env() -> Self {
        let mut cfg = Self::default();
        if let Ok(v) = env::var("DOLT_HOST") { cfg.host = v; }
        if let Ok(v) = env::var("DOLT_PORT") {
            if let Ok(p) = v.parse::<u16>() { cfg.port = p; }
        }
        if let Ok(v) = env::var("DOLT_DB") { cfg.database = v; }
        if let Ok(v) = env::var("DOLT_USER") { cfg.user = v; }
        if let Ok(v) = env::var("DOLT_PASSWORD") { cfg.password = v; }
        if let Ok(v) = env::var("DOLT_AUTO_COMMIT") {
            cfg.auto_commit = v.to_lowercase() != "false" && v != "0";
        }
        if let Ok(v) = env::var("DOLT_COMMIT_THRESHOLD") {
            if let Ok(t) = v.parse::<usize>() { cfg.commit_threshold = t; }
        }
        cfg
    }

    /// Returns `Some(config)` if the `DOLT_HOST` env var is set, otherwise `None`.
    /// Useful for graceful fallback: only enable Dolt when it is explicitly configured.
    pub fn try_from_env() -> Option<Self> {
        if env::var("DOLT_HOST").is_ok() || env::var("DOLT_PORT").is_ok() {
            Some(Self::from_env())
        } else {
            None
        }
    }

    /// Build a MySQL [`OptsBuilder`] from this config.
    pub fn to_opts(&self) -> Opts {
        OptsBuilder::new()
            .ip_or_hostname(Some(&self.host))
            .tcp_port(self.port)
            .db_name(Some(&self.database))
            .user(Some(&self.user))
            .pass(if self.password.is_empty() { None } else { Some(&self.password) })
            .into()
    }

}

// ---------------------------------------------------------------------------
// Serialization helpers
// ---------------------------------------------------------------------------

/// Parse a MySQL DATETIME string (with or without fractional seconds) to UTC.
/// Extracted as a public(crate) function so it can be unit-tested independently
/// of a live Dolt connection.
///
/// Accepts:
/// - `"2026-03-06 14:30:00"`
/// - `"2026-03-06 14:30:00.123"`
pub(crate) fn parse_dolt_datetime(s: &str) -> Result<DateTime<Utc>, StoreError> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f"))
        .map(|naive| naive.and_utc())
        .map_err(|e| StoreError::Other(format!("Failed to parse datetime '{}': {}", s, e)))
}

/// Format a UTC datetime as a MySQL-compatible DATETIME string.
pub(crate) fn format_dolt_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

// ---------------------------------------------------------------------------
// DoltMemoryStore
// ---------------------------------------------------------------------------

/// Hybrid Dolt-backed memory store with in-memory cache.
///
/// All reads are served from the `cache` (HashMap); all mutations are written
/// through to Dolt immediately (via `sync_memory_to_dolt`). Mutations that
/// arrive via the `MemoryStore::get_mut` path are tracked in `dirty_set` and
/// must be flushed explicitly with [`DoltMemoryStore::flush_dirty`].
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
    /// IDs mutated through get_mut that have not yet been synced to Dolt.
    dirty_set: HashSet<Uuid>,
}

impl DoltMemoryStore {
    /// Create a new store from the given MySQL connection pool, using default commit settings.
    /// Loads all existing memories from Dolt into the in-memory cache.
    pub fn new(pool: Pool) -> Result<Self, StoreError> {
        let cfg = DoltConfig::default();
        Self::from_pool(pool, &cfg)
    }

    /// Create a store from a [`DoltConfig`], building the pool internally.
    pub fn from_config(config: &DoltConfig) -> Result<Self, StoreError> {
        let opts = config.to_opts();
        let pool = Pool::new(opts)
            .map_err(|e| StoreError::Other(format!("Failed to create Dolt pool: {}", e)))?;
        Self::from_pool(pool, config)
    }

    /// Create a store from environment variables (via [`DoltConfig::from_env`]).
    pub fn from_env() -> Result<Self, StoreError> {
        let config = DoltConfig::from_env();
        Self::from_config(&config)
    }

    /// Build from an already-constructed pool with explicit commit settings.
    pub fn from_pool(pool: Pool, config: &DoltConfig) -> Result<Self, StoreError> {
        let mut store = Self {
            cache: HashMap::new(),
            pool,
            auto_commit: config.auto_commit,
            pending_changes: 0,
            commit_threshold: config.commit_threshold,
            dirty_set: HashSet::new(),
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

            // FIX (devil's advocate): resonance_key is Vec::new() throughout the
            // codebase (see consolidation.rs, store.rs). The placeholder vec![0.0; 100]
            // was wrong dimensionality and semantically incorrect. The full 10K-dim
            // resonance key is not stored in the skip_links table (would be 40KB per row);
            // empty vec is the correct representation for Dolt-loaded skip links.
            let resonance_key = Vec::new();

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
            let created_at = parse_dolt_datetime(&created_at_str)?;
            
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
        let created_at_str = format_dolt_datetime(&memory.created_at);
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

    /// Explicitly sync a single memory that is already in the cache to Dolt.
    /// Use this when you need to persist a mutation made via [`MemoryStore::get_mut`].
    pub fn update(&mut self, id: &Uuid) -> Result<(), StoreError> {
        let memory = self.cache.get(id)
            .ok_or_else(|| StoreError::NotFound(*id))?;
        let memory = memory.clone();
        self.sync_memory_to_dolt(&memory)?;
        self.dirty_set.remove(id);
        Ok(())
    }

    /// Flush all memories that were mutated via `get_mut` to Dolt.
    /// Returns the number of memories synced.
    pub fn flush_dirty(&mut self) -> Result<usize, StoreError> {
        let dirty: Vec<Uuid> = self.dirty_set.iter().copied().collect();
        let count = dirty.len();
        for id in dirty {
            let memory = self.cache.get(&id)
                .ok_or_else(|| StoreError::NotFound(id))?;
            let memory = memory.clone();
            self.sync_memory_to_dolt(&memory)?;
            self.dirty_set.remove(&id);
        }
        Ok(count)
    }

    /// Returns the set of IDs mutated through `get_mut` but not yet flushed to Dolt.
    pub fn dirty_ids(&self) -> &HashSet<Uuid> {
        &self.dirty_set
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
        // FIX (devil's advocate): track IDs mutated via get_mut so callers can
        // later flush them to Dolt via flush_dirty() or update().
        if self.cache.contains_key(id) {
            self.dirty_set.insert(*id);
        }
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
        // FIX (devil's advocate): attempt Dolt deletion first so the cache is
        // only evicted on success, keeping cache and DB consistent on failure.
        let was_present = self.cache.contains_key(id);

        if was_present {
            self.delete_from_dolt(id)?;
            self.cache.remove(id);
            self.dirty_set.remove(id);
        }

        Ok(was_present)
    }

    fn count(&self) -> usize {
        self.cache.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike, TimeZone};
    use std::sync::Mutex;

    /// Serialise all tests that read/write process-wide environment variables.
    /// Cargo runs tests in parallel by default; without this lock they race on
    /// the shared env state and produce flaky results.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Convenience: clear every DOLT_* env var and return when done.
    fn clear_dolt_env() {
        for key in &[
            "DOLT_HOST", "DOLT_PORT", "DOLT_DB",
            "DOLT_USER", "DOLT_PASSWORD",
            "DOLT_AUTO_COMMIT", "DOLT_COMMIT_THRESHOLD",
        ] {
            std::env::remove_var(key);
        }
    }

    // -----------------------------------------------------------------------
    // DoltConfig — unit tests (no DB required)
    // -----------------------------------------------------------------------

    #[test]
    fn dolt_config_default_values() {
        let cfg = DoltConfig::default();
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 3307);
        assert_eq!(cfg.database, "kannaka_memory");
        assert_eq!(cfg.user, "root");
        assert!(cfg.password.is_empty());
        assert!(cfg.auto_commit);
        assert_eq!(cfg.commit_threshold, 10);
    }

    #[test]
    fn dolt_config_from_env_overrides_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_dolt_env();

        std::env::set_var("DOLT_HOST", "db.example.com");
        std::env::set_var("DOLT_PORT", "3308");
        std::env::set_var("DOLT_DB", "my_memories");
        std::env::set_var("DOLT_USER", "kannaka");
        std::env::set_var("DOLT_PASSWORD", "s3cr3t");
        std::env::set_var("DOLT_AUTO_COMMIT", "false");
        std::env::set_var("DOLT_COMMIT_THRESHOLD", "50");

        let cfg = DoltConfig::from_env();
        clear_dolt_env();

        assert_eq!(cfg.host, "db.example.com");
        assert_eq!(cfg.port, 3308);
        assert_eq!(cfg.database, "my_memories");
        assert_eq!(cfg.user, "kannaka");
        assert_eq!(cfg.password, "s3cr3t");
        assert!(!cfg.auto_commit);
        assert_eq!(cfg.commit_threshold, 50);
    }

    #[test]
    fn dolt_config_from_env_invalid_port_keeps_default() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_dolt_env();
        std::env::set_var("DOLT_PORT", "not_a_number");
        let cfg = DoltConfig::from_env();
        clear_dolt_env();
        assert_eq!(cfg.port, 3307);
    }

    #[test]
    fn dolt_config_auto_commit_false_variants() {
        let _guard = ENV_LOCK.lock().unwrap();
        for value in &["false", "False", "FALSE", "0"] {
            clear_dolt_env();
            std::env::set_var("DOLT_AUTO_COMMIT", value);
            let cfg = DoltConfig::from_env();
            assert!(!cfg.auto_commit, "expected false for DOLT_AUTO_COMMIT={}", value);
        }
        clear_dolt_env();
    }

    #[test]
    fn dolt_config_try_from_env_returns_none_without_vars() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_dolt_env();
        assert!(DoltConfig::try_from_env().is_none());
    }

    #[test]
    fn dolt_config_try_from_env_returns_some_with_host() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_dolt_env();
        std::env::set_var("DOLT_HOST", "dolt.local");
        let result = DoltConfig::try_from_env();
        clear_dolt_env();
        assert!(result.is_some());
        assert_eq!(result.unwrap().host, "dolt.local");
    }

    // -----------------------------------------------------------------------
    // Datetime helpers — unit tests (no DB required)
    // -----------------------------------------------------------------------

    #[test]
    fn parse_dolt_datetime_standard_format() {
        let dt = parse_dolt_datetime("2026-03-06 14:30:00").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.month(), 3);
        assert_eq!(dt.day(), 6);
        assert_eq!(dt.hour(), 14);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn parse_dolt_datetime_with_fractional_seconds() {
        let dt = parse_dolt_datetime("2026-03-06 14:30:00.123").unwrap();
        assert_eq!(dt.year(), 2026);
        assert_eq!(dt.second(), 0);
    }

    #[test]
    fn parse_dolt_datetime_invalid_returns_error() {
        let result = parse_dolt_datetime("not-a-date");
        assert!(result.is_err());
        // Error message should mention the offending string
        assert!(result.unwrap_err().to_string().contains("not-a-date"));
    }

    #[test]
    fn format_parse_dolt_datetime_roundtrip() {
        let original = Utc.with_ymd_and_hms(2026, 1, 15, 9, 5, 3).unwrap();
        let formatted = format_dolt_datetime(&original);
        assert_eq!(formatted, "2026-01-15 09:05:03");
        let parsed = parse_dolt_datetime(&formatted).unwrap();
        // Round-trip: sub-second precision is lost (MySQL DATETIME has 1s resolution)
        assert_eq!(parsed.year(), original.year());
        assert_eq!(parsed.month(), original.month());
        assert_eq!(parsed.day(), original.day());
        assert_eq!(parsed.hour(), original.hour());
        assert_eq!(parsed.minute(), original.minute());
        assert_eq!(parsed.second(), original.second());
    }

    // -----------------------------------------------------------------------
    // Devil's Advocate regression: the OLD broken format would have failed
    // -----------------------------------------------------------------------

    #[test]
    fn old_parse_with_timezone_required_would_fail() {
        // Demonstrate that the original Phase 1 approach was wrong:
        // DateTime::parse_from_str with a no-timezone format panics / errors.
        // This test documents the bug by showing chrono DOES need %z for parse_from_str.
        let result = chrono::DateTime::parse_from_str("2026-03-06 14:30:00", "%Y-%m-%d %H:%M:%S");
        assert!(
            result.is_err(),
            "parse_from_str without timezone info MUST fail — the Phase 1 bug was real"
        );
    }
}