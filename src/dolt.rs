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
/// | Variable                | Default                          | Description                    |
/// |-------------------------|----------------------------------|--------------------------------|
/// | `DOLT_HOST`             | `127.0.0.1`                      | Dolt SQL server hostname       |
/// | `DOLT_PORT`             | `3307`                           | Dolt SQL server port           |
/// | `DOLT_DB`               | `kannaka_memory`                 | Database name                  |
/// | `DOLT_USER`             | `root`                           | Database user                  |
/// | `DOLT_PASSWORD`         | *(empty)*                        | Database password              |
/// | `DOLT_AUTO_COMMIT`      | `true`                           | Auto-commit after N changes    |
/// | `DOLT_COMMIT_THRESHOLD` | `10`                             | Changes between auto-commits   |
/// | `DOLT_AUTHOR`           | `Kannaka Agent <kannaka@local>`  | Author header for Dolt commits |
/// | `DOLT_REMOTE`           | `origin`                         | Default remote for push/pull   |
/// | `DOLT_BRANCH`           | `main`                           | Default branch name            |
#[derive(Debug, Clone)]
pub struct DoltConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: String,
    pub auto_commit: bool,
    pub commit_threshold: usize,
    /// Author string for Dolt version commits, e.g. `"Name <email>"`.
    pub commit_author: String,
    /// Default remote name for push/pull operations.
    pub remote: String,
    /// Default branch name.
    pub default_branch: String,
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
            commit_author: "Kannaka Agent <kannaka@local>".to_string(),
            remote: "origin".to_string(),
            default_branch: "main".to_string(),
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
        if let Ok(v) = env::var("DOLT_AUTHOR")  { cfg.commit_author  = v; }
        if let Ok(v) = env::var("DOLT_REMOTE")  { cfg.remote         = v; }
        if let Ok(v) = env::var("DOLT_BRANCH")  { cfg.default_branch = v; }
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
// Phase 4 value types
// ---------------------------------------------------------------------------

/// Information about a Dolt branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchInfo {
    /// Branch name.
    pub name: String,
    /// Latest commit hash on this branch.
    pub hash: String,
    /// Whether this is the currently checked-out branch.
    pub is_current: bool,
}

/// A single entry in the Dolt commit log.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// Dolt commit hash.
    pub hash: String,
    /// Committer name / email string.
    pub author: String,
    /// Commit timestamp in UTC.
    pub date: DateTime<Utc>,
    /// Commit message.
    pub message: String,
}

/// How a memory row changed between two Dolt refs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffKind {
    Added,
    Removed,
    Modified,
}

/// A memory that changed between two Dolt commits or branches.
#[derive(Debug, Clone)]
pub struct MemoryDiff {
    /// The memory's UUID (from whichever side is non-null).
    pub id: Uuid,
    /// Nature of the change.
    pub kind: DiffKind,
    /// Content in the `from` ref (None for Added rows).
    pub from_content: Option<String>,
    /// Content in the `to` ref (None for Removed rows).
    pub to_content: Option<String>,
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
    /// Author header written into Dolt version commits.
    commit_author: String,
    /// Default remote name for push / pull.
    remote: String,
    /// Default branch name.
    default_branch: String,
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
            commit_author: config.commit_author.clone(),
            remote: config.remote.clone(),
            default_branch: config.default_branch.clone(),
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

        // ADR-0011: collective fields — query separately so older databases (pre-migration)
        // degrade gracefully to defaults rather than failing on missing columns.
        let memories_collective: Vec<(String, Option<String>, Option<u64>, Option<String>, Option<bool>)> = 
            conn.query("SELECT id, origin_agent, sync_version, last_consolidated_at, disputed FROM memories")
            .unwrap_or_default();

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

        // ADR-0011: lookup map for collective data (all fields Optional — backward compat)
        let mut collective_data: HashMap<String, (String, u64, Option<String>, bool)> = HashMap::new();
        for (id, origin_agent, sync_version, last_consolidated_at_str, disputed) in memories_collective {
            collective_data.insert(id, (
                origin_agent.unwrap_or_else(|| "local".to_string()),
                sync_version.unwrap_or(0),
                last_consolidated_at_str,
                disputed.unwrap_or(false),
            ));
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

            // ADR-0011: extract collective fields
            let (origin_agent, sync_version, last_consolidated_at_str, disputed) =
                collective_data.remove(&id)
                    .unwrap_or_else(|| ("local".to_string(), 0, None, false));
            let last_consolidated_at = if let Some(ref s) = last_consolidated_at_str {
                parse_dolt_datetime(s).ok()
            } else {
                None
            };

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
                origin_agent,
                sync_version,
                merge_history: Vec::new(),
                last_consolidated_at,
                disputed,
                updated_at: None,
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

        // ADR-0011: update collective fields (no-op on pre-migration databases)
        let merge_history_json = if memory.merge_history.is_empty() {
            None
        } else {
            serde_json::to_string(&memory.merge_history).ok()
        };
        let last_consolidated_str = memory.last_consolidated_at.as_ref().map(format_dolt_datetime);
        let _ = conn.exec_drop(
            r"UPDATE memories SET origin_agent = ?, sync_version = ?, merge_history = ?, last_consolidated_at = ?, disputed = ? WHERE id = ?",
            (&memory.origin_agent, memory.sync_version, &merge_history_json, &last_consolidated_str, memory.disputed, &memory.id.to_string())
        );

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

    /// Commit staged changes to Dolt version control.
    ///
    /// Uses `DOLT_AUTHOR` config for the commit author field.
    /// Returns `Ok(true)` if a commit was created, `Ok(false)` if there was
    /// nothing to commit (treated as success).
    pub fn commit(&mut self, message: &str) -> Result<bool, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        conn.exec_drop("CALL DOLT_ADD('.')", ())
            .map_err(|e| StoreError::Other(format!("Failed to stage changes: {}", e)))?;

        let result = conn.exec_drop(
            "CALL DOLT_COMMIT('-m', ?, '--author', ?)",
            (message, &self.commit_author),
        );

        match result {
            Ok(_) => {
                self.pending_changes = 0;
                Ok(true)
            }
            Err(e) => {
                let msg = e.to_string().to_lowercase();
                if msg.contains("nothing to commit") || msg.contains("no changes to commit") {
                    self.pending_changes = 0;
                    Ok(false)
                } else {
                    Err(StoreError::Other(format!("Failed to commit: {}", e)))
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Phase 4: Branch management
    // -----------------------------------------------------------------------

    /// Create a new branch from the current HEAD (does not switch to it).
    pub fn create_branch(&self, name: &str) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        conn.exec_drop("CALL DOLT_BRANCH(?)", (name,))
            .map_err(|e| StoreError::Other(format!("Failed to create branch '{}': {}", name, e)))?;
        Ok(())
    }

    /// Switch to an existing branch.
    pub fn checkout(&self, name: &str) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        conn.exec_drop("CALL DOLT_CHECKOUT(?)", (name,))
            .map_err(|e| StoreError::Other(format!("Failed to checkout '{}': {}", name, e)))?;
        Ok(())
    }

    /// Create a new branch and immediately switch to it.
    pub fn checkout_new_branch(&self, name: &str) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        conn.exec_drop("CALL DOLT_CHECKOUT('-b', ?)", (name,))
            .map_err(|e| StoreError::Other(format!("Failed to create+checkout '{}': {}", name, e)))?;
        Ok(())
    }

    /// Delete a branch (must not be the currently checked-out branch).
    pub fn delete_branch(&self, name: &str) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        conn.exec_drop("CALL DOLT_BRANCH('-d', ?)", (name,))
            .map_err(|e| StoreError::Other(format!("Failed to delete branch '{}': {}", name, e)))?;
        Ok(())
    }

    /// List all branches in the database.
    pub fn list_branches(&self) -> Result<Vec<BranchInfo>, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        let active: Vec<(String,)> = conn
            .query("SELECT active_branch()")
            .map_err(|e| StoreError::Other(format!("Failed to get active branch: {}", e)))?;
        let active_name = active.into_iter().next().map(|r| r.0).unwrap_or_default();

        let rows: Vec<(String, String)> = conn
            .query("SELECT name, hash FROM dolt_branches ORDER BY name")
            .map_err(|e| StoreError::Other(format!("Failed to list branches: {}", e)))?;

        Ok(rows
            .into_iter()
            .map(|(name, hash)| BranchInfo {
                is_current: name == active_name,
                name,
                hash,
            })
            .collect())
    }

    /// Return the name of the currently active branch.
    pub fn current_branch(&self) -> Result<String, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        let rows: Vec<(String,)> = conn
            .query("SELECT active_branch()")
            .map_err(|e| StoreError::Other(format!("Failed to query active branch: {}", e)))?;
        rows.into_iter()
            .next()
            .map(|r| r.0)
            .ok_or_else(|| StoreError::Other("active_branch() returned no rows".to_string()))
    }

    // -----------------------------------------------------------------------
    // Phase 4: Commit log
    // -----------------------------------------------------------------------

    /// Fetch recent Dolt commits on the current branch.
    pub fn log(&self, limit: usize) -> Result<Vec<CommitInfo>, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        let rows: Vec<(String, String, String, String)> = conn.exec(
            "SELECT commit_hash, committer, committer_date, message FROM dolt_log LIMIT ?",
            (limit,),
        ).map_err(|e| StoreError::Other(format!("Failed to query dolt_log: {}", e)))?;

        let mut entries = Vec::with_capacity(rows.len());
        for (hash, author, date_str, message) in rows {
            let date = parse_dolt_datetime(&date_str).unwrap_or_else(|_| Utc::now());
            entries.push(CommitInfo { hash, author, date, message });
        }
        Ok(entries)
    }

    // -----------------------------------------------------------------------
    // Phase 4: DoltHub push / pull
    // -----------------------------------------------------------------------

    /// Push the current branch to a remote.
    ///
    /// Pass `None` for `remote` / `branch` to use the `DoltConfig` defaults.
    pub fn push(&self, remote: Option<&str>, branch: Option<&str>) -> Result<(), StoreError> {
        let remote = remote.unwrap_or(&self.remote);
        let branch = branch.unwrap_or(&self.default_branch);
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        conn.exec_drop("CALL DOLT_PUSH(?, ?)", (remote, branch))
            .map_err(|e| StoreError::Other(format!("Failed to push to {}/{}: {}", remote, branch, e)))?;
        Ok(())
    }

    /// Pull from a remote into the current branch.
    ///
    /// Pass `None` for `remote` / `branch` to use the `DoltConfig` defaults.
    pub fn pull(&self, remote: Option<&str>, branch: Option<&str>) -> Result<(), StoreError> {
        let remote = remote.unwrap_or(&self.remote);
        let branch = branch.unwrap_or(&self.default_branch);
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        conn.exec_drop("CALL DOLT_PULL(?, ?)", (remote, branch))
            .map_err(|e| StoreError::Other(format!("Failed to pull from {}/{}: {}", remote, branch, e)))?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Phase 4: Diff and merge
    // -----------------------------------------------------------------------

    /// Return memories that changed between two Dolt refs (commit hashes,
    /// branch names, or symbolic refs like `HEAD~1`).
    ///
    /// # Example
    /// ```no_run
    /// let changes = store.diff("HEAD~1", "HEAD")?;
    /// ```
    pub fn diff(&self, from_ref: &str, to_ref: &str) -> Result<Vec<MemoryDiff>, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        // dolt_diff_memories is a system table Dolt generates automatically for
        // the `memories` table. Columns: diff_type, from_id, from_content,
        // to_id, to_content (plus all other table columns prefixed from_/to_).
        let rows: Vec<(String, Option<String>, Option<String>, Option<String>, Option<String>)> =
            conn.exec(
                "SELECT diff_type, from_id, from_content, to_id, to_content \
                 FROM dolt_diff_memories \
                 WHERE from_commit = ? AND to_commit = ?",
                (from_ref, to_ref),
            ).map_err(|e| StoreError::Other(format!("Failed to diff memories: {}", e)))?;

        let mut diffs = Vec::with_capacity(rows.len());
        for (diff_type, from_id, from_content, to_id, to_content) in rows {
            let kind = match diff_type.as_str() {
                "added"    => DiffKind::Added,
                "removed"  => DiffKind::Removed,
                _          => DiffKind::Modified,
            };
            let raw_id = to_id.as_deref().or(from_id.as_deref()).unwrap_or("");
            let id = Uuid::parse_str(raw_id)
                .map_err(|e| StoreError::Other(format!("Invalid diff UUID '{}': {}", raw_id, e)))?;
            diffs.push(MemoryDiff { id, kind, from_content, to_content });
        }
        Ok(diffs)
    }

    /// Merge a branch into the current branch.
    ///
    /// Returns the merge commit hash on success.
    pub fn merge_branch(&mut self, branch: &str) -> Result<String, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        let rows: Vec<(Option<String>, Option<u8>, Option<String>)> = conn.exec(
            "SELECT DOLT_MERGE(?)", (branch,),
        ).map_err(|e| StoreError::Other(format!("Failed to merge '{}': {}", branch, e)))?;

        let hash = rows.into_iter()
            .next()
            .and_then(|(h, _, _)| h)
            .unwrap_or_else(|| "merge-ok".to_string());

        // Reload cache to reflect merged data
        self.load_from_dolt()?;
        Ok(hash)
    }

    // -----------------------------------------------------------------------
    // Phase 4: High-level speculation helpers
    // -----------------------------------------------------------------------

    /// Open a speculative memory branch for "what-if" thinking.
    ///
    /// 1. Flushes any dirty memories to Dolt.
    /// 2. Commits current changes (if any) with a "pre-speculation" message.
    /// 3. Creates and checks out `branch_name`.
    ///
    /// Call [`collapse_speculation`] or [`discard_speculation`] when done.
    pub fn speculate(&mut self, branch_name: &str) -> Result<(), StoreError> {
        self.flush_dirty()?;
        self.commit(&format!("pre-speculation: before branch '{}'", branch_name))?;
        self.checkout_new_branch(branch_name)?;
        Ok(())
    }

    /// Merge a speculation branch back into `default_branch` and delete it.
    ///
    /// 1. Commits any pending changes on the speculation branch.
    /// 2. Checks out `default_branch`.
    /// 3. Merges `branch_name`.
    /// 4. Deletes `branch_name`.
    ///
    /// Returns the merge commit hash.
    pub fn collapse_speculation(
        &mut self,
        branch_name: &str,
        message: &str,
    ) -> Result<String, StoreError> {
        self.flush_dirty()?;
        self.commit(message)?;
        let default = self.default_branch.clone();
        self.checkout(&default)?;
        let hash = self.merge_branch(branch_name)?;
        self.delete_branch(branch_name)?;
        Ok(hash)
    }

    /// Discard a speculation branch without merging.
    ///
    /// Checks out `default_branch` and force-deletes `branch_name`.
    pub fn discard_speculation(&mut self, branch_name: &str) -> Result<(), StoreError> {
        let default = self.default_branch.clone();
        self.checkout(&default)?;
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        conn.exec_drop("CALL DOLT_BRANCH('-df', ?)", (branch_name,))
            .map_err(|e| StoreError::Other(format!("Failed to force-delete branch '{}': {}", branch_name, e)))?;
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

    /// Return the configured default branch name.
    pub fn default_branch(&self) -> &str {
        &self.default_branch
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

    // -----------------------------------------------------------------------
    // ADR-0011: Collective memory tables
    // -----------------------------------------------------------------------

    /// Create the three collective-memory tables if they do not already exist.
    ///
    /// Safe to call on an existing database — all statements use `CREATE TABLE IF NOT EXISTS`.
    /// Mirrors `migrations/0011-collective-memory.sql` but executable from Rust code.
    pub fn create_collective_tables(&self) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        conn.exec_drop(r"
            CREATE TABLE IF NOT EXISTS sync_events (
                id          VARCHAR(36)  NOT NULL PRIMARY KEY,
                event_type  VARCHAR(32)  NOT NULL,
                agent_id    VARCHAR(64)  NOT NULL,
                memory_id   VARCHAR(36)  DEFAULT NULL,
                metadata    JSON         DEFAULT NULL,
                created_at  DATETIME(6)  NOT NULL,
                synced_at   DATETIME(6)  DEFAULT NULL,
                INDEX idx_agent_time (agent_id, created_at),
                INDEX idx_memory     (memory_id),
                INDEX idx_event_type (event_type)
            )", ()).map_err(|e| StoreError::Other(format!("Failed to create sync_events: {}", e)))?;

        conn.exec_drop(r"
            CREATE TABLE IF NOT EXISTS agents (
                agent_id        VARCHAR(64)  NOT NULL PRIMARY KEY,
                display_name    VARCHAR(128) DEFAULT NULL,
                trust_score     FLOAT        NOT NULL DEFAULT 0.5,
                last_sync       DATETIME(6)  DEFAULT NULL,
                branch_name     VARCHAR(128) DEFAULT NULL,
                flux_entity     VARCHAR(64)  DEFAULT NULL,
                embedding_model VARCHAR(64)  DEFAULT NULL,
                capabilities    JSON         DEFAULT NULL,
                created_at      DATETIME(6)  NOT NULL
            )", ()).map_err(|e| StoreError::Other(format!("Failed to create agents: {}", e)))?;

        conn.exec_drop(r"
            CREATE TABLE IF NOT EXISTS quarantine (
                id            VARCHAR(36)  NOT NULL PRIMARY KEY,
                memory_id_a   VARCHAR(36)  NOT NULL,
                memory_id_b   VARCHAR(36)  NOT NULL,
                agent_a       VARCHAR(64)  NOT NULL,
                agent_b       VARCHAR(64)  NOT NULL,
                similarity    FLOAT        NOT NULL,
                phase_diff    FLOAT        NOT NULL,
                dispute_count INT          NOT NULL DEFAULT 1,
                status        VARCHAR(16)  NOT NULL DEFAULT 'pending',
                resolution    JSON         DEFAULT NULL,
                created_at    DATETIME(6)  NOT NULL,
                resolved_at   DATETIME(6)  DEFAULT NULL,
                INDEX idx_status   (status),
                INDEX idx_memory_a (memory_id_a),
                INDEX idx_memory_b (memory_id_b)
            )", ()).map_err(|e| StoreError::Other(format!("Failed to create quarantine: {}", e)))?;

        Ok(())
    }

    /// Insert a row into `sync_events` (fire-and-forget — errors are logged, not fatal).
    pub fn log_sync_event(
        &self,
        event_type: &str,
        agent_id: &str,
        memory_id: Option<&str>,
        metadata: Option<&serde_json::Value>,
    ) {
        let id = Uuid::new_v4().to_string();
        let now = format_dolt_datetime(&chrono::Utc::now());
        let meta_json = metadata.and_then(|v| serde_json::to_string(v).ok());

        let result = self.pool.get_conn().and_then(|mut conn| {
            conn.exec_drop(
                "INSERT INTO sync_events (id, event_type, agent_id, memory_id, metadata, created_at) \
                 VALUES (?, ?, ?, ?, ?, ?)",
                (&id, event_type, agent_id, memory_id, &meta_json, &now),
            )
        });
        if let Err(e) = result {
            eprintln!("[dolt] log_sync_event failed (non-fatal): {}", e);
        }
    }

    /// Insert or update an agent record in the `agents` table.
    pub fn upsert_agent(
        &self,
        agent_id: &str,
        display_name: Option<&str>,
        trust_score: f32,
        branch_name: Option<&str>,
        flux_entity: Option<&str>,
    ) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        let now = format_dolt_datetime(&chrono::Utc::now());
        conn.exec_drop(
            "INSERT INTO agents (agent_id, display_name, trust_score, branch_name, flux_entity, created_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE \
               display_name = VALUES(display_name), trust_score = VALUES(trust_score), \
               branch_name = VALUES(branch_name), flux_entity = VALUES(flux_entity), \
               last_sync = ?",
            (agent_id, display_name, trust_score, branch_name, flux_entity, &now, &now),
        ).map_err(|e| StoreError::Other(format!("Failed to upsert agent: {}", e)))?;
        Ok(())
    }

    /// Insert a quarantine entry for a disputed memory pair.
    pub fn quarantine_memories(
        &self,
        entry: &crate::collective::QuarantineEntry,
    ) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        let now = format_dolt_datetime(&chrono::Utc::now());
        conn.exec_drop(
            "INSERT INTO quarantine (id, memory_id_a, memory_id_b, agent_a, agent_b, similarity, phase_diff, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            (
                entry.id.to_string(),
                entry.memory_id_a.to_string(),
                entry.memory_id_b.to_string(),
                &entry.agent_a,
                &entry.agent_b,
                entry.similarity,
                entry.phase_diff,
                &now,
            ),
        ).map_err(|e| StoreError::Other(format!("Failed to quarantine memories: {}", e)))?;
        Ok(())
    }

    /// D3: Garbage-collect resolved quarantine entries older than `max_age_days`.
    /// Also auto-escalate pending entries that have exceeded `escalate_after_days`
    /// without resolution.
    pub fn gc_quarantine(&self, max_age_days: i64, escalate_after_days: i64) -> Result<(usize, usize), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        // Delete resolved entries older than max_age_days
        let deleted: usize = conn.exec_iter(
            "DELETE FROM quarantine WHERE status = 'resolved' AND resolved_at < DATE_SUB(NOW(), INTERVAL ? DAY)",
            (max_age_days,),
        ).map(|r| r.affected_rows() as usize)
         .unwrap_or(0);

        // Auto-escalate stale pending entries
        let escalated: usize = conn.exec_iter(
            "UPDATE quarantine SET status = 'escalated' WHERE status = 'pending' AND created_at < DATE_SUB(NOW(), INTERVAL ? DAY)",
            (escalate_after_days,),
        ).map(|r| r.affected_rows() as usize)
         .unwrap_or(0);

        Ok((deleted, escalated))
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
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
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
        scored.sort_by(|a, b| b.1.total_cmp(&a.1));
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
    use chrono::{Datelike, Timelike, TimeZone, Utc};
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
            "DOLT_AUTHOR", "DOLT_REMOTE", "DOLT_BRANCH",
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
        assert_eq!(cfg.commit_author, "Kannaka Agent <kannaka@local>");
        assert_eq!(cfg.remote, "origin");
        assert_eq!(cfg.default_branch, "main");
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

    #[test]
    fn dolt_config_phase4_env_overrides() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_dolt_env();
        std::env::set_var("DOLT_AUTHOR", "Alice <alice@wonderland>");
        std::env::set_var("DOLT_REMOTE", "dolthub");
        std::env::set_var("DOLT_BRANCH", "prod");
        let cfg = DoltConfig::from_env();
        clear_dolt_env();
        assert_eq!(cfg.commit_author, "Alice <alice@wonderland>");
        assert_eq!(cfg.remote, "dolthub");
        assert_eq!(cfg.default_branch, "prod");
    }

    // -----------------------------------------------------------------------
    // Phase 4 value types — no DB required
    // -----------------------------------------------------------------------

    #[test]
    fn branch_info_is_current_flag() {
        let b = BranchInfo { name: "main".into(), hash: "abc123".into(), is_current: true };
        assert!(b.is_current);
        let b2 = BranchInfo { name: "speculate/x".into(), hash: "def456".into(), is_current: false };
        assert!(!b2.is_current);
    }

    #[test]
    fn diff_kind_variants_are_distinct() {
        assert_ne!(DiffKind::Added, DiffKind::Removed);
        assert_ne!(DiffKind::Added, DiffKind::Modified);
        assert_ne!(DiffKind::Removed, DiffKind::Modified);
    }

    #[test]
    fn memory_diff_added_has_no_from_content() {
        let id = Uuid::new_v4();
        let d = MemoryDiff {
            id,
            kind: DiffKind::Added,
            from_content: None,
            to_content: Some("hello world".into()),
        };
        assert!(d.from_content.is_none());
        assert_eq!(d.to_content.as_deref(), Some("hello world"));
    }

    #[test]
    fn memory_diff_removed_has_no_to_content() {
        let id = Uuid::new_v4();
        let d = MemoryDiff {
            id,
            kind: DiffKind::Removed,
            from_content: Some("old memory".into()),
            to_content: None,
        };
        assert_eq!(d.from_content.as_deref(), Some("old memory"));
        assert!(d.to_content.is_none());
    }

    #[test]
    fn memory_diff_modified_has_both_sides() {
        let id = Uuid::new_v4();
        let d = MemoryDiff {
            id,
            kind: DiffKind::Modified,
            from_content: Some("before".into()),
            to_content: Some("after".into()),
        };
        assert_eq!(d.kind, DiffKind::Modified);
        assert!(d.from_content.is_some());
        assert!(d.to_content.is_some());
    }

    #[test]
    fn commit_info_fields_accessible() {
        let ci = CommitInfo {
            hash: "h1".into(),
            author: "Bob <b@b.com>".into(),
            date: Utc::now(),
            message: "initial commit".into(),
        };
        assert_eq!(ci.hash, "h1");
        assert!(ci.message.contains("initial"));
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