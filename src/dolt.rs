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
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}};
use std::time::{Duration, Instant};

use chrono::{DateTime, NaiveDateTime, Utc};
use mysql::*;
use mysql::prelude::*;
use serde_json;
use uuid::Uuid;

use crate::memory::HyperMemory;
use crate::skip_link::SkipLink;
use crate::store::{MemoryStore, StoreError};
use crate::wave::cosine_similarity;

#[cfg(feature = "glyph")]
use crate::glyph_bridge::{GlyphEncoder, encode_memory_as_glyph};

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
/// | `DOLT_AUTO_PUSH`        | `false`                          | Enable auto-push to DoltHub    |
/// | `DOLT_PUSH_INTERVAL`    | `300`                            | Push after N seconds idle      |
/// | `DOLT_PUSH_THRESHOLD`   | `5`                              | Push after N commits           |
/// | `DOLT_AGENT_ID`         | `local`                          | Agent identifier for sync      |
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
    /// Enable automatic push to DoltHub after commits.
    pub auto_push: bool,
    /// Push after this many seconds of inactivity (default: 300).
    pub push_interval_secs: u64,
    /// Push after this many commits (default: 5).
    pub push_threshold: usize,
    /// Agent identifier for sync events and branch naming.
    pub agent_id: String,
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
            auto_push: false,
            push_interval_secs: 300,
            push_threshold: 5,
            agent_id: "local".to_string(),
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
        if let Ok(v) = env::var("DOLT_AUTO_PUSH") {
            cfg.auto_push = v.to_lowercase() != "false" && v != "0";
        }
        if let Ok(v) = env::var("DOLT_PUSH_INTERVAL") {
            if let Ok(s) = v.parse::<u64>() { cfg.push_interval_secs = s; }
        }
        if let Ok(v) = env::var("DOLT_PUSH_THRESHOLD") {
            if let Ok(t) = v.parse::<usize>() { cfg.push_threshold = t; }
        }
        if let Ok(v) = env::var("DOLT_AGENT_ID") { cfg.agent_id = v; }
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
    /// Agent identifier for sync events and branch naming.
    agent_id: String,
    /// Background auto-push thread (ADR-0017).
    auto_pusher: Option<AutoPusher>,
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
        // Start auto-pusher if configured
        let auto_pusher = if config.auto_push {
            Some(AutoPusher::start(
                pool.clone(),
                config.remote.clone(),
                config.default_branch.clone(),
                config.commit_author.clone(),
                config.push_threshold,
                config.push_interval_secs,
                #[cfg(feature = "glyph")]
                true,
            ))
        } else {
            None
        };

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
            agent_id: config.agent_id.clone(),
            auto_pusher,
        };

        let count = store.load_from_dolt()?;
        let _ = count;

        Ok(store)
    }

    /// Return the configured agent identifier.
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Stop the auto-push background thread (if running).
    pub fn stop_auto_push(&mut self) {
        if let Some(ref mut pusher) = self.auto_pusher {
            pusher.stop();
        }
        self.auto_pusher = None;
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

        let memories_extended: Vec<(String, Option<String>, String, Option<String>, Option<String>, Option<String>)> = 
            conn.query("SELECT id, parents, vector_data, xi_signature, geometry, glyph_content FROM memories")
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
        let mut extended_data: HashMap<String, (Option<String>, String, Option<String>, Option<String>, Option<String>)> = HashMap::new();
        for (id, parents_json, vector_json, xi_signature_json, geometry_json, glyph_content_json) in memories_extended {
            extended_data.insert(id, (parents_json, vector_json, xi_signature_json, geometry_json, glyph_content_json));
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
            let (parents_json, vector_json, xi_signature_json, geometry_json, _glyph_content_json) = extended_data
                .remove(&id)
                .unwrap_or((None, "[]".to_string(), None, None, None));
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

            // Deserialize geometry from JSON (optional, non-fatal on schema mismatch)
            let geometry = if let Some(geom_json) = geometry_json {
                serde_json::from_str(&geom_json).ok()
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
                retrieval_count: 0,
            };

            self.cache.insert(uuid, memory);
            loaded_count += 1;
        }

        Ok(loaded_count)
    }

    /// Sync a single memory to Dolt (upsert operation).
    pub fn sync_memory_to_dolt(&mut self, memory: &HyperMemory) -> Result<(), StoreError> {
        #[cfg(feature = "glyph")]
        {
            self.sync_memory_to_dolt_with_glyph(memory)
        }
        #[cfg(not(feature = "glyph"))]
        {
            self.sync_memory_to_dolt_internal(memory, None, None)
        }
    }

    /// Encode content as glyph for privacy protection on DoltHub
    #[cfg(feature = "glyph")]
    fn encode_content_as_glyph(content: &str) -> Option<String> {
        // Convert text to f64 array (UTF-8 bytes as f64s)
        let bytes = content.as_bytes();
        let data: Vec<f64> = bytes.iter().map(|&b| b as f64).collect();
        
        if data.is_empty() {
            return None;
        }
        
        // Encode through GlyphEncoder
        let encoder = GlyphEncoder::default();
        match encoder.encode(&data) {
            Ok(glyph) => {
                // Serialize the Glyph struct to JSON
                serde_json::to_string(&glyph).ok()
            }
            Err(_) => None
        }
    }

    /// Update memory with glyph content and SGA classification if glyph feature is enabled.
    ///
    /// ADR-0017 F-8: Classify-on-store. When a memory is synced to Dolt, the SGA
    /// classifier runs and stores the dominant class, centroid coordinates, and
    /// Fano signature alongside the memory for geometric SQL queries.
    #[cfg(feature = "glyph")]
    fn sync_memory_to_dolt_with_glyph(&mut self, memory: &HyperMemory) -> Result<(), StoreError> {
        let bytes = memory.content.as_bytes();
        let data: Vec<f64> = bytes.iter().map(|&b| b as f64).collect();

        let (glyph_content, sga_data) = if !data.is_empty() {
            let encoder = GlyphEncoder::default();
            match encoder.encode(&data) {
                Ok(glyph) => {
                    let glyph_json = serde_json::to_string(&glyph).ok();
                    let centroid = glyph.sga_centroid; // (h2, d, l)
                    let dominant = glyph.fold_sequence.iter().copied()
                        .max_by_key(|&c| glyph.fold_sequence.iter().filter(|&&x| x == c).count())
                        .unwrap_or(0);
                    let fano_json = serde_json::to_string(&glyph.fano_signature).ok();
                    (glyph_json, Some((dominant, centroid.0, centroid.1, centroid.2, fano_json)))
                }
                Err(_) => (None, None),
            }
        } else {
            (None, None)
        };

        self.sync_memory_to_dolt_internal(memory, glyph_content.as_deref(), sga_data.as_ref())
    }

    /// Internal method that accepts glyph_content and SGA classification data.
    ///
    /// `sga_data`: Optional tuple of (dominant_class, centroid_h2, centroid_d, centroid_l, fano_json).
    fn sync_memory_to_dolt_internal(
        &mut self,
        memory: &HyperMemory,
        glyph_content: Option<&str>,
        #[allow(unused_variables)]
        sga_data: Option<&(u8, u8, u8, u8, Option<String>)>,
    ) -> Result<(), StoreError> {
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

        // Update extended fields with glyph_content
        conn.exec_drop(
            r"UPDATE memories SET parents = ?, vector_data = ?, xi_signature = ?, geometry = ?, glyph_content = ? WHERE id = ?",
            (&parents_json, &vector_json, &xi_signature_json, &geometry_json, &glyph_content, &memory.id.to_string())
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

        // ADR-0017 F-8: SGA classification columns (classify-on-store)
        #[cfg(feature = "glyph")]
        if let Some((dominant, h2, d, l, ref fano_json)) = sga_data {
            let _ = conn.exec_drop(
                r"UPDATE memories SET sga_class = ?, sga_centroid_h2 = ?, sga_centroid_d = ?, sga_centroid_l = ?, fano_signature = ? WHERE id = ?",
                (dominant, h2, d, l, fano_json, &memory.id.to_string())
            );
        }

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
                self.notify_commit();
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

    // -----------------------------------------------------------------------
    // Transaction support (Issue #7 — dream write safety)
    // -----------------------------------------------------------------------

    /// Begin a SQL transaction. All subsequent writes will be part of this transaction
    /// until `transaction_commit()` or `transaction_rollback()` is called.
    pub fn transaction_begin(&self) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        conn.exec_drop("BEGIN", ())
            .map_err(|e| StoreError::Other(format!("Failed to BEGIN transaction: {}", e)))?;
        Ok(())
    }

    /// Commit the current transaction.
    pub fn transaction_commit(&self) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        conn.exec_drop("COMMIT", ())
            .map_err(|e| StoreError::Other(format!("Failed to COMMIT transaction: {}", e)))?;
        Ok(())
    }

    /// Rollback the current transaction.
    pub fn transaction_rollback(&self) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        conn.exec_drop("ROLLBACK", ())
            .map_err(|e| StoreError::Other(format!("Failed to ROLLBACK transaction: {}", e)))?;
        Ok(())
    }

    /// Flush dirty memories within a transaction. Begins a transaction, flushes all dirty
    /// memories, and commits. On error, rolls back.
    pub fn flush_dirty_transactional(&mut self) -> Result<usize, StoreError> {
        self.transaction_begin()?;
        match self.flush_dirty() {
            Ok(count) => {
                self.transaction_commit()?;
                Ok(count)
            }
            Err(e) => {
                let _ = self.transaction_rollback();
                Err(e)
            }
        }
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

        // Add glyph_content column for privacy-protected DoltHub push (ADR-0011)
        let _ = conn.exec_drop(r"
            ALTER TABLE memories ADD COLUMN glyph_content JSON DEFAULT NULL
        ", ()).map_err(|e| {
            // Non-fatal: column might already exist
            eprintln!("[dolt] Note: glyph_content column may already exist: {}", e);
        });

        // ADR-0018: Queen Synchronization Protocol tables
        self.create_queen_tables()?;

        Ok(())
    }

    // -----------------------------------------------------------------------
    // ADR-0018: Queen Synchronization Protocol tables
    // -----------------------------------------------------------------------

    /// Create the Queen Sync tables (`agent_phases`, `queen_state`) and extend
    /// the `agents` table with swarm columns.
    ///
    /// Safe to call repeatedly — uses `CREATE TABLE IF NOT EXISTS` and non-fatal
    /// `ALTER TABLE` for column additions.
    pub fn create_queen_tables(&self) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        // Agent phase state (published periodically by each agent)
        conn.exec_drop(r"
            CREATE TABLE IF NOT EXISTS agent_phases (
                id                VARCHAR(36)  NOT NULL PRIMARY KEY,
                agent_id          VARCHAR(64)  NOT NULL,
                phase             DOUBLE       NOT NULL,
                frequency         DOUBLE       NOT NULL,
                coherence         DOUBLE       NOT NULL,
                phi               DOUBLE       DEFAULT 0,
                order_parameter   DOUBLE       DEFAULT 0,
                cluster_count     INT          DEFAULT 0,
                memory_count      INT          DEFAULT 0,
                xi_signature      JSON,
                protocol_version  VARCHAR(8)   DEFAULT '1.0',
                timestamp         DATETIME(6)  NOT NULL,
                INDEX idx_agent (agent_id),
                INDEX idx_time  (timestamp)
            )", ()).map_err(|e| StoreError::Other(format!("Failed to create agent_phases: {}", e)))?;

        // Emergent Queen state (computed, not assigned)
        conn.exec_drop(r"
            CREATE TABLE IF NOT EXISTS queen_state (
                id                VARCHAR(36)  NOT NULL PRIMARY KEY,
                order_parameter   DOUBLE       NOT NULL,
                mean_phase        DOUBLE       NOT NULL,
                coherence         DOUBLE       NOT NULL,
                phi               DOUBLE       NOT NULL,
                agent_count       INT          NOT NULL,
                hive_topology     JSON,
                coupling_strength DOUBLE,
                chiral_bias       DOUBLE,
                geometric         JSON,
                computed_by       VARCHAR(64),
                timestamp         DATETIME(6)  NOT NULL,
                INDEX idx_time (timestamp)
            )", ()).map_err(|e| StoreError::Other(format!("Failed to create queen_state: {}", e)))?;

        // Extend agents table with swarm columns (non-fatal if they already exist)
        let alter_statements = [
            "ALTER TABLE agents ADD COLUMN swarm_role VARCHAR(16) DEFAULT 'member'",
            "ALTER TABLE agents ADD COLUMN protocol_version VARCHAR(8) DEFAULT '1.0'",
            "ALTER TABLE agents ADD COLUMN handedness VARCHAR(8) DEFAULT 'achiral'",
            "ALTER TABLE agents ADD COLUMN natural_frequency DOUBLE DEFAULT 0.5",
        ];
        for stmt in &alter_statements {
            let _ = conn.exec_drop(stmt, ());
            // Non-fatal: columns may already exist
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // ADR-0018: Queen Sync read/write
    // -----------------------------------------------------------------------

    /// Publish this agent's phase state to the `agent_phases` table.
    pub fn publish_phase(&self, phase: &crate::queen::AgentPhase) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        let timestamp = format_dolt_datetime(&phase.timestamp);
        let xi_json = phase.xi_signature.as_ref().and_then(|v| serde_json::to_string(v).ok());
        conn.exec_drop(
            r"INSERT INTO agent_phases (id, agent_id, phase, frequency, coherence, phi,
              order_parameter, cluster_count, memory_count, xi_signature, protocol_version, timestamp)
              VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
              ON DUPLICATE KEY UPDATE
              phase = VALUES(phase), frequency = VALUES(frequency), coherence = VALUES(coherence),
              phi = VALUES(phi), order_parameter = VALUES(order_parameter),
              cluster_count = VALUES(cluster_count), memory_count = VALUES(memory_count),
              xi_signature = VALUES(xi_signature), timestamp = VALUES(timestamp)",
            (
                &phase.id, &phase.agent_id, phase.phase as f64, phase.frequency as f64,
                phase.coherence as f64, phase.phi as f64, phase.order_parameter as f64,
                phase.cluster_count as u32, phase.memory_count as u32,
                &xi_json, &phase.protocol_version, &timestamp,
            ),
        ).map_err(|e| StoreError::Other(format!("Failed to publish phase: {}", e)))?;
        Ok(())
    }

    /// Read all swarm agent phases published within `since` duration.
    ///
    /// Returns the most recent phase entry per agent.
    pub fn read_swarm_phases(&self, since: std::time::Duration) -> Result<Vec<crate::queen::AgentPhase>, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        let since_secs = since.as_secs();
        let rows: Vec<(String, String, f64, f64, f64, f64, f64, u32, u32, Option<String>, String, mysql::Value)> = conn.exec(
            r"SELECT ap.id, ap.agent_id, ap.phase, ap.frequency, ap.coherence, ap.phi,
              ap.order_parameter, ap.cluster_count, ap.memory_count, ap.xi_signature,
              ap.protocol_version, ap.timestamp
              FROM agent_phases ap
              INNER JOIN (
                  SELECT agent_id, MAX(timestamp) as max_ts
                  FROM agent_phases
                  WHERE timestamp > DATE_SUB(NOW(6), INTERVAL ? SECOND)
                  GROUP BY agent_id
              ) latest ON ap.agent_id = latest.agent_id AND ap.timestamp = latest.max_ts",
            (since_secs,),
        ).map_err(|e| StoreError::Other(format!("Failed to read swarm phases: {}", e)))?;

        // Also read trust scores and handedness from agents table
        let agents: Vec<(String, f32, Option<String>)> = conn.query(
            "SELECT agent_id, trust_score, handedness FROM agents"
        ).unwrap_or_default();
        let agent_map: std::collections::HashMap<String, (f32, String)> = agents.into_iter()
            .map(|(id, trust, hand)| (id, (trust, hand.unwrap_or_else(|| "achiral".to_string()))))
            .collect();

        let mut phases = Vec::with_capacity(rows.len());
        for (id, agent_id, phase, freq, coh, phi, order, clusters, memories, xi_json, proto, ts_val) in rows {
            let ts_str = match &ts_val {
                mysql::Value::Bytes(b) => String::from_utf8_lossy(b).to_string(),
                mysql::Value::Date(y, m, d, h, mi, s, us) =>
                    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}", y, m, d, h, mi, s, us),
                _ => format!("{:?}", ts_val),
            };
            let timestamp = parse_dolt_datetime(&ts_str).unwrap_or_else(|_| Utc::now());
            let xi_sig = xi_json.and_then(|s| serde_json::from_str(&s).ok());
            let (trust, hand_str) = agent_map.get(&agent_id)
                .cloned()
                .unwrap_or((0.5, "achiral".to_string()));
            phases.push(crate::queen::AgentPhase {
                id,
                agent_id,
                phase: phase as f32,
                frequency: freq as f32,
                coherence: coh as f32,
                phi: phi as f32,
                order_parameter: order as f32,
                cluster_count: clusters as usize,
                memory_count: memories as usize,
                xi_signature: xi_sig,
                protocol_version: proto,
                timestamp,
                trust_score: trust,
                handedness: crate::queen::Handedness::from_str(&hand_str),
            });
        }
        Ok(phases)
    }

    /// Write the computed QueenState to the `queen_state` table.
    pub fn write_queen_state(&self, state: &crate::queen::QueenState) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        let timestamp = format_dolt_datetime(&state.timestamp);
        let hive_json = serde_json::to_string(&state.hives).ok();
        let geom_json = state.geometric.as_ref().and_then(|v| serde_json::to_string(v).ok());
        conn.exec_drop(
            r"INSERT INTO queen_state (id, order_parameter, mean_phase, coherence, phi,
              agent_count, hive_topology, coupling_strength, chiral_bias, geometric,
              computed_by, timestamp)
              VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                &state.id, state.order_parameter as f64, state.mean_phase as f64,
                state.coherence as f64, state.phi as f64, state.agent_count as u32,
                &hive_json, state.coupling_strength as f64, state.chiral_bias as f64,
                &geom_json, &state.computed_by, &timestamp,
            ),
        ).map_err(|e| StoreError::Other(format!("Failed to write queen state: {}", e)))?;
        Ok(())
    }

    /// Read the most recent QueenState from the `queen_state` table.
    pub fn read_queen_state(&self) -> Result<Option<crate::queen::QueenState>, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        let rows: Vec<(String, f64, f64, f64, f64, u32, Option<String>, Option<f64>, Option<f64>, Option<String>, Option<String>, mysql::Value)> = conn.query(
            r"SELECT id, order_parameter, mean_phase, coherence, phi, agent_count,
              hive_topology, coupling_strength, chiral_bias, geometric, computed_by, timestamp
              FROM queen_state ORDER BY timestamp DESC LIMIT 1"
        ).map_err(|e| StoreError::Other(format!("Failed to read queen state: {}", e)))?;

        let row = match rows.into_iter().next() {
            Some(r) => r,
            None => return Ok(None),
        };
        let (id, order, mean_ph, coh, phi, count, hive_json, coupling, chiral, geom_json, computed_by, ts_val) = row;
        let ts_str = match &ts_val {
            mysql::Value::Bytes(b) => String::from_utf8_lossy(b).to_string(),
            mysql::Value::Date(y, m, d, h, mi, s, us) =>
                format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}", y, m, d, h, mi, s, us),
            _ => format!("{:?}", ts_val),
        };
        let timestamp = parse_dolt_datetime(&ts_str).unwrap_or_else(|_| Utc::now());
        let hives: Vec<crate::queen::Hive> = hive_json
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let geometric = geom_json.and_then(|s| serde_json::from_str(&s).ok());

        Ok(Some(crate::queen::QueenState {
            id,
            order_parameter: order as f32,
            mean_phase: mean_ph as f32,
            coherence: coh as f32,
            phi: phi as f32,
            agent_count: count as usize,
            hives,
            coupling_strength: coupling.unwrap_or(0.5) as f32,
            chiral_bias: chiral.unwrap_or(0.1) as f32,
            geometric,
            computed_by: computed_by.unwrap_or_default(),
            timestamp,
        }))
    }

    /// Register an agent in the swarm (upserts into `agents` with queen sync columns).
    pub fn register_swarm_agent(&self, agent: &crate::queen::SwarmAgent) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        let now = format_dolt_datetime(&Utc::now());
        conn.exec_drop(
            r"INSERT INTO agents (agent_id, display_name, trust_score, swarm_role,
              protocol_version, handedness, natural_frequency, created_at)
              VALUES (?, ?, ?, ?, ?, ?, ?, ?)
              ON DUPLICATE KEY UPDATE
              display_name = VALUES(display_name), trust_score = VALUES(trust_score),
              swarm_role = VALUES(swarm_role), protocol_version = VALUES(protocol_version),
              handedness = VALUES(handedness), natural_frequency = VALUES(natural_frequency),
              last_sync = ?",
            (
                &agent.agent_id, &agent.display_name, agent.trust_score,
                &agent.swarm_role, &agent.protocol_version,
                agent.handedness.as_str(), agent.natural_frequency as f64,
                &now, &now,
            ),
        ).map_err(|e| StoreError::Other(format!("Failed to register swarm agent: {}", e)))?;
        Ok(())
    }

    /// Read all registered swarm agents.
    pub fn read_swarm_agents(&self) -> Result<Vec<crate::queen::SwarmAgent>, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        let rows: Vec<(String, Option<String>, f32, Option<String>, Option<String>, Option<String>, Option<f64>)> = conn.query(
            r"SELECT agent_id, display_name, trust_score, swarm_role, protocol_version,
              handedness, natural_frequency FROM agents"
        ).map_err(|e| StoreError::Other(format!("Failed to read swarm agents: {}", e)))?;

        Ok(rows.into_iter().map(|(id, name, trust, role, proto, hand, freq)| {
            crate::queen::SwarmAgent {
                agent_id: id,
                display_name: name,
                trust_score: trust,
                swarm_role: role.unwrap_or_else(|| "member".to_string()),
                protocol_version: proto.unwrap_or_else(|| "1.0".to_string()),
                handedness: crate::queen::Handedness::from_str(&hand.unwrap_or_else(|| "achiral".to_string())),
                natural_frequency: freq.unwrap_or(0.5) as f32,
            }
        }).collect())
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

    // -----------------------------------------------------------------------
    // ADR-0017 Phase 5: Progressive Revelation Tables (F-11)
    // -----------------------------------------------------------------------

    /// Create the revelation tables for bloom hint publishing and community voting.
    ///
    /// Safe to call on an existing database — uses `CREATE TABLE IF NOT EXISTS`.
    pub fn create_revelation_tables(&self) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        conn.exec_drop(r"
            CREATE TABLE IF NOT EXISTS bloom_hints (
                id          VARCHAR(36)  NOT NULL PRIMARY KEY,
                memory_id   VARCHAR(36)  NOT NULL,
                hint_type   VARCHAR(32)  NOT NULL DEFAULT 'fano',
                hint_data   JSON         NOT NULL,
                difficulty  INT          NOT NULL DEFAULT 32,
                published_by VARCHAR(64) NOT NULL,
                created_at  DATETIME(6)  NOT NULL,
                expires_at  DATETIME(6)  DEFAULT NULL,
                INDEX idx_memory (memory_id),
                INDEX idx_difficulty (difficulty),
                INDEX idx_publisher (published_by)
            )", ()).map_err(|e| StoreError::Other(format!("Failed to create bloom_hints: {}", e)))?;

        conn.exec_drop(r"
            CREATE TABLE IF NOT EXISTS revelation_votes (
                id          VARCHAR(36)  NOT NULL PRIMARY KEY,
                memory_id   VARCHAR(36)  NOT NULL,
                voter       VARCHAR(64)  NOT NULL,
                vote        VARCHAR(16)  NOT NULL DEFAULT 'reveal',
                reason      TEXT         DEFAULT NULL,
                created_at  DATETIME(6)  NOT NULL,
                INDEX idx_memory (memory_id),
                INDEX idx_voter (voter),
                UNIQUE INDEX idx_unique_vote (memory_id, voter)
            )", ()).map_err(|e| StoreError::Other(format!("Failed to create revelation_votes: {}", e)))?;

        Ok(())
    }

    /// Publish a bloom hint that lowers difficulty for a sealed memory.
    ///
    /// Bloom hints are Fano plane projections that make it easier for other agents
    /// to "bloom" (decrypt) sealed memories. Progressive revelation:
    /// high difficulty initially, hints lower it over time.
    pub fn publish_bloom_hint(
        &self,
        memory_id: &Uuid,
        hint_data: &serde_json::Value,
        difficulty: u32,
        published_by: &str,
    ) -> Result<String, StoreError> {
        let id = Uuid::new_v4().to_string();
        let now = format_dolt_datetime(&Utc::now());
        let hint_json = serde_json::to_string(hint_data)
            .map_err(|e| StoreError::Other(format!("Failed to serialize hint: {}", e)))?;

        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        conn.exec_drop(
            "INSERT INTO bloom_hints (id, memory_id, hint_data, difficulty, published_by, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
            (&id, &memory_id.to_string(), &hint_json, difficulty, published_by, &now),
        ).map_err(|e| StoreError::Other(format!("Failed to publish hint: {}", e)))?;

        Ok(id)
    }

    /// Cast a revelation vote for a sealed memory.
    ///
    /// Community members vote to bloom (reveal) high-value sealed memories.
    /// When enough votes accumulate, the revelation policy can trigger.
    pub fn cast_revelation_vote(
        &self,
        memory_id: &Uuid,
        voter: &str,
        vote: &str,
        reason: Option<&str>,
    ) -> Result<(), StoreError> {
        let id = Uuid::new_v4().to_string();
        let now = format_dolt_datetime(&Utc::now());

        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        conn.exec_drop(
            "INSERT INTO revelation_votes (id, memory_id, voter, vote, reason, created_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON DUPLICATE KEY UPDATE vote = VALUES(vote), reason = VALUES(reason)",
            (&id, &memory_id.to_string(), voter, vote, reason, &now),
        ).map_err(|e| StoreError::Other(format!("Failed to cast vote: {}", e)))?;

        Ok(())
    }

    /// Count revelation votes for a memory.
    pub fn count_revelation_votes(
        &self,
        memory_id: &Uuid,
    ) -> Result<(usize, usize), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        let reveal: Vec<(u32,)> = conn.exec(
            "SELECT COUNT(*) FROM revelation_votes WHERE memory_id = ? AND vote = 'reveal'",
            (&memory_id.to_string(),),
        ).map_err(|e| StoreError::Other(format!("Failed to count votes: {}", e)))?;

        let reject: Vec<(u32,)> = conn.exec(
            "SELECT COUNT(*) FROM revelation_votes WHERE memory_id = ? AND vote = 'reject'",
            (&memory_id.to_string(),),
        ).map_err(|e| StoreError::Other(format!("Failed to count votes: {}", e)))?;

        Ok((
            reveal.first().map_or(0, |r| r.0 as usize),
            reject.first().map_or(0, |r| r.0 as usize),
        ))
    }

    // -----------------------------------------------------------------------
    // ADR-0017 Phase 5: Constellation Sync (F-12)
    // -----------------------------------------------------------------------

    /// Commit a constellation SVG snapshot to Dolt metadata.
    ///
    /// Stores the SVG as a metadata entry, creating a versioned history
    /// of constellation visualizations.
    pub fn commit_constellation_svg(
        &mut self,
        svg_content: &str,
        agent_id: &str,
    ) -> Result<bool, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        let now = format_dolt_datetime(&Utc::now());
        conn.exec_drop(
            "REPLACE INTO metadata (key_name, value_text) VALUES (?, ?)",
            ("constellation_svg", svg_content),
        ).map_err(|e| StoreError::Other(format!("Failed to store SVG: {}", e)))?;

        conn.exec_drop(
            "REPLACE INTO metadata (key_name, value_text) VALUES (?, ?)",
            ("constellation_updated_at", &now),
        ).map_err(|e| StoreError::Other(format!("Failed to store timestamp: {}", e)))?;

        conn.exec_drop(
            "REPLACE INTO metadata (key_name, value_text) VALUES (?, ?)",
            ("constellation_agent", agent_id),
        ).map_err(|e| StoreError::Other(format!("Failed to store agent: {}", e)))?;

        self.commit(&format!("constellation: SVG snapshot by {}", agent_id))
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

    /// Prepare for DoltHub push by encoding sensitive content as glyphs
    #[cfg(feature = "glyph")]
    pub fn prepare_for_dolthub(&self) -> Result<(), StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        // Update memories where glyph_content exists to replace content with category placeholder
        conn.exec_drop(
            r"UPDATE memories 
              SET content = CASE 
                  WHEN hallucinated = 1 THEN '[hallucination]'
                  WHEN layer_depth = 0 THEN '[experience]'  
                  WHEN layer_depth <= 3 THEN '[knowledge]'
                  ELSE '[insight]'
              END
              WHERE glyph_content IS NOT NULL",
            ()
        ).map_err(|e| StoreError::Other(format!("Failed to prepare for DoltHub: {}", e)))?;

        Ok(())
    }

    /// Helper function to generate content category for privacy
    #[cfg(feature = "glyph")]
    fn content_category(memory: &HyperMemory) -> String {
        if memory.hallucinated {
            "[hallucination]".to_string()
        } else if memory.layer_depth == 0 {
            "[experience]".to_string()
        } else if memory.layer_depth <= 3 {
            "[knowledge]".to_string()
        } else {
            "[insight]".to_string()
        }
    }

    /// Helper function for DoltHub-safe content (returns category if glyph exists, original otherwise)
    #[cfg(feature = "glyph")]
    pub fn dolthub_content(memory: &HyperMemory, has_glyph: bool) -> String {
        if has_glyph {
            Self::content_category(memory)
        } else {
            memory.content.clone()
        }
    }

    // -----------------------------------------------------------------------
    // ADR-0017 Phase 2: Dream Branch Workflow
    // -----------------------------------------------------------------------

    /// Create a dream branch and switch to it.
    ///
    /// Branch name: `{agent_id}/dream/{timestamp}` (e.g. `flaukowski/dream/2026-03-10-143000`).
    /// Flushes dirty memories and commits pending changes before branching.
    ///
    /// Returns the branch name for later use in [`collapse_dream`].
    pub fn begin_dream(&mut self, agent_id: &str) -> Result<String, StoreError> {
        let timestamp = Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
        let branch_name = format!("{}/dream/{}", agent_id, timestamp);

        self.flush_dirty()?;
        self.commit(&format!("pre-dream: snapshot before dream cycle ({})", &timestamp))?;
        self.checkout_new_branch(&branch_name)?;

        eprintln!("[dolt] Dream branch created: {}", branch_name);
        Ok(branch_name)
    }

    /// Commit dream artifacts to the current dream branch.
    ///
    /// Call this after each stage or at the end of the dream cycle to record
    /// hallucinations, strengthened connections, and prune results.
    pub fn commit_dream_artifacts(
        &mut self,
        stage: &str,
        stats: &serde_json::Value,
    ) -> Result<bool, StoreError> {
        let message = format!("dream({}): {}", stage,
            serde_json::to_string(stats).unwrap_or_else(|_| "{}".to_string()));
        self.flush_dirty()?;
        self.commit(&message)
    }

    /// Complete the dream cycle: merge dream branch back to working branch,
    /// optionally push to DoltHub with privacy encoding.
    ///
    /// Returns the merge commit hash and the dream branch name (for PR creation).
    pub fn collapse_dream(
        &mut self,
        dream_branch: &str,
        report_json: &str,
    ) -> Result<String, StoreError> {
        // Commit any remaining artifacts
        self.flush_dirty()?;
        let commit_msg = format!("dream: consolidation complete\n\n{}", report_json);
        self.commit(&commit_msg)?;

        // Merge back to default branch
        let default = self.default_branch.clone();
        self.checkout(&default)?;
        let hash = self.merge_branch(dream_branch)?;

        eprintln!("[dolt] Dream branch {} merged → {} (commit: {})",
            dream_branch, default, &hash[..8.min(hash.len())]);

        Ok(hash)
    }

    /// Push a dream branch to DoltHub, applying glyph privacy encoding first.
    ///
    /// If `create_pr` is true, returns a PR-ready branch name. The actual PR
    /// creation happens via DoltHub API (not SQL).
    #[cfg(feature = "glyph")]
    pub fn push_dream_branch(
        &self,
        dream_branch: &str,
        remote: Option<&str>,
    ) -> Result<(), StoreError> {
        // Privacy gate: encode content as glyphs before pushing
        self.prepare_for_dolthub()?;
        self.push(remote, Some(dream_branch))?;
        eprintln!("[dolt] Dream branch pushed to DoltHub: {}", dream_branch);
        Ok(())
    }

    /// Discard a dream branch without merging (e.g. if dream produced bad results).
    pub fn discard_dream(&mut self, dream_branch: &str) -> Result<(), StoreError> {
        eprintln!("[dolt] Discarding dream branch: {}", dream_branch);
        self.discard_speculation(dream_branch)
    }

    /// Notify the auto-pusher (if any) that a commit has occurred.
    /// Call this after each successful commit to trigger threshold-based pushes.
    pub fn notify_commit(&self) {
        if let Some(ref pusher) = self.auto_pusher {
            pusher.notify_commit();
        }
    }

    // -----------------------------------------------------------------------
    // ADR-0017 Phase 4: Wasteland Bridge (F-7)
    // -----------------------------------------------------------------------

    /// Generate a Dolt commit as evidence for a Wasteland completion.
    ///
    /// Commits pending changes with a message referencing the wanted-id,
    /// and returns the commit hash for use as `completions.evidence`.
    ///
    /// # Usage
    /// ```no_run
    /// let hash = store.evidence_commit("w-abc123", "Implemented feature X")?;
    /// // Use hash as evidence in: /wasteland done w-abc123
    /// ```
    pub fn evidence_commit(
        &mut self,
        wanted_id: &str,
        description: &str,
    ) -> Result<String, StoreError> {
        self.flush_dirty()?;

        let message = format!("wasteland({}): {}", wanted_id, description);
        let committed = self.commit(&message)?;

        if !committed {
            // Nothing to commit — return latest commit hash instead
            let log = self.log(1)?;
            return log.into_iter()
                .next()
                .map(|c| c.hash)
                .ok_or_else(|| StoreError::Other("No commits found".to_string()));
        }

        // Get the hash of what we just committed
        let log = self.log(1)?;
        let hash = log.into_iter()
            .next()
            .map(|c| c.hash)
            .ok_or_else(|| StoreError::Other("Commit succeeded but no hash found".to_string()))?;

        eprintln!("[dolt] Evidence commit for {}: {}", wanted_id, &hash[..12.min(hash.len())]);
        Ok(hash)
    }

    /// Verify that a Dolt commit exists and references a wanted-id.
    ///
    /// Used by validators to confirm completion evidence is genuine.
    /// Returns the commit info if valid, or an error explaining why not.
    pub fn verify_evidence(
        &self,
        commit_hash: &str,
        wanted_id: &str,
    ) -> Result<CommitInfo, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        let rows: Vec<(String, String, String, String)> = conn.exec(
            "SELECT commit_hash, committer, committer_date, message FROM dolt_log WHERE commit_hash = ? LIMIT 1",
            (commit_hash,),
        ).map_err(|e| StoreError::Other(format!("Failed to query commit: {}", e)))?;

        let (hash, author, date_str, message) = rows.into_iter()
            .next()
            .ok_or_else(|| StoreError::Other(format!(
                "Commit {} not found", commit_hash
            )))?;

        // Verify the commit message references the wanted-id
        if !message.contains(wanted_id) {
            return Err(StoreError::Other(format!(
                "Commit {} does not reference wanted-id '{}'", commit_hash, wanted_id
            )));
        }

        let date = parse_dolt_datetime(&date_str).unwrap_or_else(|_| Utc::now());
        Ok(CommitInfo { hash, author, date, message })
    }

    // -----------------------------------------------------------------------
    // ADR-0017 Phase 4: SGA-Indexed Search (F-8)
    // -----------------------------------------------------------------------

    /// Search memories by SGA class index.
    ///
    /// Returns memories where `sga_class` matches the given class number (0-83).
    /// The class index encodes the geometric position: `21*h2 + 7*d + l`.
    pub fn search_by_sga_class(&self, class_index: u8) -> Result<Vec<&HyperMemory>, StoreError> {
        // Query from Dolt for the IDs matching the class
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        let ids: Vec<(String,)> = conn.exec(
            "SELECT id FROM memories WHERE sga_class = ?",
            (class_index,),
        ).map_err(|e| StoreError::Other(format!("Failed to search by SGA class: {}", e)))?;

        let mut results = Vec::new();
        for (id_str,) in ids {
            if let Ok(uuid) = Uuid::parse_str(&id_str) {
                if let Some(mem) = self.cache.get(&uuid) {
                    results.push(mem);
                }
            }
        }
        Ok(results)
    }

    /// Search memories by SGA centroid coordinates.
    ///
    /// Finds memories with matching geometric position in the Cl₀,₇ ⊗ ℝ[ℤ₄] ⊗ ℝ[ℤ₃] space.
    /// Pass `None` for any coordinate to match all values on that axis.
    pub fn search_by_centroid(
        &self,
        h2: Option<u8>,
        d: Option<u8>,
        l: Option<u8>,
    ) -> Result<Vec<&HyperMemory>, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        // Build dynamic WHERE clause
        let mut conditions = Vec::new();
        let mut params: Vec<mysql::Value> = Vec::new();

        if let Some(h2_val) = h2 {
            conditions.push("sga_centroid_h2 = ?");
            params.push(mysql::Value::from(h2_val));
        }
        if let Some(d_val) = d {
            conditions.push("sga_centroid_d = ?");
            params.push(mysql::Value::from(d_val));
        }
        if let Some(l_val) = l {
            conditions.push("sga_centroid_l = ?");
            params.push(mysql::Value::from(l_val));
        }

        if conditions.is_empty() {
            return self.all_memories();
        }

        let query = format!(
            "SELECT id FROM memories WHERE {}",
            conditions.join(" AND ")
        );

        let ids: Vec<(String,)> = conn.exec(&query, mysql::Params::Positional(params))
            .map_err(|e| StoreError::Other(format!("Failed to search by centroid: {}", e)))?;

        let mut results = Vec::new();
        for (id_str,) in ids {
            if let Ok(uuid) = Uuid::parse_str(&id_str) {
                if let Some(mem) = self.cache.get(&uuid) {
                    results.push(mem);
                }
            }
        }
        Ok(results)
    }

    /// Search memories by Fano line similarity.
    ///
    /// Finds memories whose Fano signature (7-element energy distribution across
    /// Fano plane lines) is within `threshold` cosine similarity of the query signature.
    pub fn search_by_fano(
        &self,
        query_fano: &[f64; 7],
        threshold: f64,
    ) -> Result<Vec<(&HyperMemory, f64)>, StoreError> {
        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        // Get all memories with fano signatures
        let rows: Vec<(String, String)> = conn.query(
            "SELECT id, fano_signature FROM memories WHERE fano_signature IS NOT NULL"
        ).map_err(|e| StoreError::Other(format!("Failed to query fano: {}", e)))?;

        let mut results = Vec::new();
        let query_norm: f64 = query_fano.iter().map(|x| x * x).sum::<f64>().sqrt();
        if query_norm < 1e-12 { return Ok(results); }

        for (id_str, fano_json) in rows {
            if let Ok(fano) = serde_json::from_str::<[f64; 7]>(&fano_json) {
                let fano_norm: f64 = fano.iter().map(|x| x * x).sum::<f64>().sqrt();
                if fano_norm < 1e-12 { continue; }

                let dot: f64 = query_fano.iter().zip(fano.iter()).map(|(a, b)| a * b).sum();
                let similarity = dot / (query_norm * fano_norm);

                if similarity >= threshold {
                    if let Ok(uuid) = Uuid::parse_str(&id_str) {
                        if let Some(mem) = self.cache.get(&uuid) {
                            results.push((mem, similarity));
                        }
                    }
                }
            }
        }

        results.sort_by(|a, b| b.1.total_cmp(&a.1));
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // ADR-0017 Phase 3: Wave Interference Merge via Dolt Conflicts
    // -----------------------------------------------------------------------

    /// Pull from remote and resolve any conflicts using the wave interference
    /// algorithm from ADR-0011.
    ///
    /// Dolt's three-way merge provides `base`, `ours`, and `theirs` for each
    /// conflicting cell. This maps directly to the wave interference model:
    ///
    /// | Dolt Value | Wave Analog |
    /// |------------|-------------|
    /// | `base`     | Memory state before divergence |
    /// | `ours`     | This agent's modifications |
    /// | `theirs`   | Other agent's modifications |
    ///
    /// Returns a report of merge outcomes.
    pub fn pull_with_wave_merge(
        &mut self,
        remote: Option<&str>,
        branch: Option<&str>,
    ) -> Result<WaveMergeReport, StoreError> {
        let remote_name = remote.unwrap_or(&self.remote);
        let branch_name = branch.unwrap_or(&self.default_branch);

        // Try a normal pull first
        let pull_result = self.pull(Some(remote_name), Some(branch_name));

        match pull_result {
            Ok(_) => {
                // No conflicts — reload cache and return clean report
                self.load_from_dolt()?;
                Ok(WaveMergeReport::clean())
            }
            Err(e) => {
                let msg = e.to_string().to_lowercase();
                if msg.contains("conflict") || msg.contains("merge") {
                    // Conflicts detected — resolve with wave interference
                    self.resolve_dolt_conflicts()
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Resolve Dolt merge conflicts using wave interference classification.
    ///
    /// Reads the `dolt_conflicts_memories` table, classifies each conflict pair
    /// using `classify_merge`, applies the appropriate resolution, and commits.
    fn resolve_dolt_conflicts(&mut self) -> Result<WaveMergeReport, StoreError> {
        use crate::collective::merge::{
            classify_merge, merge_guard, apply_constructive, apply_destructive,
            apply_partial, MergeKind, QuarantineEntry,
        };

        let mut conn = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;

        // Read "ours" side of conflicts
        let ours: Vec<(Option<String>, Option<f32>, Option<f32>, Option<f32>, Option<String>)> = conn.query(
            "SELECT our_id, our_amplitude, our_phase, our_frequency, our_content \
             FROM dolt_conflicts_memories"
        ).unwrap_or_default();

        // Read "theirs" side of conflicts
        let theirs: Vec<(Option<String>, Option<f32>, Option<f32>, Option<f32>, Option<String>)> = conn.query(
            "SELECT their_id, their_amplitude, their_phase, their_frequency, their_content \
             FROM dolt_conflicts_memories"
        ).unwrap_or_default();

        if ours.is_empty() {
            let _ = conn.exec_drop("CALL DOLT_CONFLICTS_RESOLVE('--ours', 'memories')", ());
            drop(conn);
            self.commit("merge: no conflicts to resolve")?;
            self.load_from_dolt()?;
            return Ok(WaveMergeReport::clean());
        }

        let mut report = WaveMergeReport {
            total_conflicts: ours.len(),
            constructive: 0,
            destructive: 0,
            partial: 0,
            independent: 0,
            quarantined: Vec::new(),
        };

        for (our_row, their_row) in ours.iter().zip(theirs.iter()) {
            let (our_id, our_amp, our_phase, our_freq, our_content) = our_row;
            let (their_id, their_amp, their_phase, their_freq, their_content) = their_row;

            let our_id_str = our_id.as_deref().or(their_id.as_deref()).unwrap_or("");
            let our_uuid = match Uuid::parse_str(our_id_str) {
                Ok(u) => u,
                Err(_) => continue,
            };

            let local_mem = self.cache.get(&our_uuid).cloned();
            let their_id_str = their_id.as_deref().unwrap_or("");
            let their_uuid = Uuid::parse_str(their_id_str).unwrap_or(our_uuid);

            if let Some(mut local) = local_mem {
                let mut remote_mem = local.clone();
                remote_mem.id = their_uuid;
                if let Some(a) = their_amp { remote_mem.amplitude = *a; }
                if let Some(p) = their_phase { remote_mem.phase = *p; }
                if let Some(f) = their_freq { remote_mem.frequency = *f; }
                if let Some(c) = their_content { remote_mem.content = c.clone(); }
                if remote_mem.origin_agent == local.origin_agent {
                    remote_mem.origin_agent = format!("{}_remote", local.origin_agent);
                }

                if merge_guard(&local, &remote_mem, None, None).is_some() {
                    report.independent += 1;
                    continue;
                }

                let result = classify_merge(&local, &remote_mem);

                match result.kind {
                    MergeKind::Constructive => {
                        apply_constructive(&mut local, &remote_mem, &result);
                        self.cache.insert(our_uuid, local.clone());
                        self.sync_memory_to_dolt(&local)?;
                        report.constructive += 1;
                    }
                    MergeKind::Destructive => {
                        apply_destructive(&mut local, &remote_mem, &result);
                        self.cache.insert(our_uuid, local.clone());
                        self.sync_memory_to_dolt(&local)?;

                        let entry = QuarantineEntry::new(&local, &remote_mem, &result);
                        self.quarantine_memories(&entry)?;
                        report.quarantined.push(entry.id);
                        report.destructive += 1;
                    }
                    MergeKind::Partial => {
                        apply_partial(&mut local, &remote_mem, &result);
                        self.cache.insert(our_uuid, local.clone());
                        self.sync_memory_to_dolt(&local)?;
                        report.partial += 1;
                    }
                    MergeKind::Independent => {
                        report.independent += 1;
                    }
                }
            } else {
                report.independent += 1;
            }
        }

        // Resolve all conflicts (we've already applied our resolution)
        let mut conn2 = self.pool.get_conn()
            .map_err(|e| StoreError::Other(format!("Failed to get connection: {}", e)))?;
        let _ = conn2.exec_drop("CALL DOLT_CONFLICTS_RESOLVE('--ours', 'memories')", ());
        drop(conn2);

        // Commit the resolution
        let msg = format!(
            "wave-merge: {} conflicts resolved ({}C/{}D/{}P/{}I, {} quarantined)",
            report.total_conflicts, report.constructive, report.destructive,
            report.partial, report.independent, report.quarantined.len()
        );
        self.commit(&msg)?;
        self.load_from_dolt()?;

        eprintln!("[dolt] {}", msg);
        Ok(report)
    }

    // -----------------------------------------------------------------------
    // ADR-0017 Phase 3: Dream-as-Pull-Request (F-6)
    // -----------------------------------------------------------------------

    /// Create a DoltHub pull request from a dream branch.
    ///
    /// Uses the DoltHub REST API to create a PR. Requires `DOLTHUB_API_KEY`
    /// env var to be set. Returns the PR URL on success.
    pub fn create_dream_pr(
        &self,
        dream_branch: &str,
        title: &str,
        description: &str,
        dolthub_repo: &str,
    ) -> Result<String, StoreError> {
        let api_key = env::var("DOLTHUB_API_KEY")
            .map_err(|_| StoreError::Other(
                "DOLTHUB_API_KEY not set — required for PR creation".to_string()
            ))?;

        // Parse owner/repo
        let parts: Vec<&str> = dolthub_repo.split('/').collect();
        if parts.len() != 2 {
            return Err(StoreError::Other(format!(
                "Invalid DoltHub repo format '{}', expected 'owner/repo'", dolthub_repo
            )));
        }
        let (owner, repo) = (parts[0], parts[1]);

        // Push the dream branch first
        self.push(None, Some(dream_branch))?;

        // Create PR via DoltHub API
        let pr_body = serde_json::json!({
            "title": title,
            "description": description,
            "fromBranchName": dream_branch,
            "toBranchName": self.default_branch,
        });

        let url = format!(
            "https://www.dolthub.com/api/v1alpha1/{}/{}/pulls",
            owner, repo
        );

        let response = ureq::post(&url)
            .set("Authorization", &format!("token {}", api_key))
            .set("Content-Type", "application/json")
            .send_string(&pr_body.to_string())
            .map_err(|e| StoreError::Other(format!("DoltHub PR creation failed: {}", e)))?;

        let status = response.status();
        let body = response.into_string()
            .unwrap_or_else(|_| "{}".to_string());

        if status >= 200 && status < 300 {
            // Try to extract PR URL from response
            let pr_url = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                json.get("html_url")
                    .or(json.get("url"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        format!("https://www.dolthub.com/repositories/{}/{}/pulls", owner, repo)
                    })
            } else {
                format!("https://www.dolthub.com/repositories/{}/{}/pulls", owner, repo)
            };

            eprintln!("[dolt] Dream PR created: {}", pr_url);
            Ok(pr_url)
        } else {
            Err(StoreError::Other(format!(
                "DoltHub PR creation failed (HTTP {}): {}", status, body
            )))
        }
    }
}

// ---------------------------------------------------------------------------
// ADR-0017 Phase 3: Wave Merge Report
// ---------------------------------------------------------------------------

/// Report of a wave interference merge operation.
#[derive(Debug, Clone)]
pub struct WaveMergeReport {
    /// Total number of Dolt conflicts encountered.
    pub total_conflicts: usize,
    /// Conflicts resolved as constructive (amplitudes superposed).
    pub constructive: usize,
    /// Conflicts resolved as destructive (memories dampened + quarantined).
    pub destructive: usize,
    /// Conflicts resolved as partial (both kept, linked).
    pub partial: usize,
    /// Conflicts classified as independent (different topics).
    pub independent: usize,
    /// IDs of quarantined memory pairs.
    pub quarantined: Vec<Uuid>,
}

impl WaveMergeReport {
    /// Create a clean report (no conflicts).
    pub fn clean() -> Self {
        Self {
            total_conflicts: 0,
            constructive: 0,
            destructive: 0,
            partial: 0,
            independent: 0,
            quarantined: Vec::new(),
        }
    }

    /// Whether the merge was clean (no conflicts).
    pub fn is_clean(&self) -> bool {
        self.total_conflicts == 0
    }
}

// ---------------------------------------------------------------------------
// ADR-0017 Phase 2: Auto-Push Scheduler
// ---------------------------------------------------------------------------

/// Shared state between the auto-pusher thread and the DoltMemoryStore.
struct AutoPushState {
    /// Number of commits since last push.
    commits_since_push: AtomicUsize,
    /// Signal to stop the background thread.
    stop: AtomicBool,
    /// Last activity timestamp (for idle-based push).
    last_activity: Mutex<Instant>,
}

/// Background thread that automatically pushes to DoltHub based on
/// commit count or idle time thresholds.
///
/// Created via [`AutoPusher::start`] and stopped via [`AutoPusher::stop`] or on drop.
///
/// # Configuration (via environment)
/// - `DOLT_AUTO_PUSH=true` — enable auto-push
/// - `DOLT_PUSH_INTERVAL=300` — push after 300 seconds of inactivity
/// - `DOLT_PUSH_THRESHOLD=5` — push after 5 commits
pub struct AutoPusher {
    state: Arc<AutoPushState>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl AutoPusher {
    /// Start the auto-push background thread.
    ///
    /// The thread wakes up every `check_interval` seconds to see if either
    /// threshold (commit count or idle time) has been met. If so, it pushes.
    pub fn start(
        pool: Pool,
        remote: String,
        branch: String,
        commit_author: String,
        push_threshold: usize,
        push_interval_secs: u64,
        #[cfg(feature = "glyph")]
        enable_privacy: bool,
    ) -> Self {
        let state = Arc::new(AutoPushState {
            commits_since_push: AtomicUsize::new(0),
            stop: AtomicBool::new(false),
            last_activity: Mutex::new(Instant::now()),
        });

        let thread_state = Arc::clone(&state);
        let check_interval = Duration::from_secs(10.min(push_interval_secs));

        let handle = std::thread::Builder::new()
            .name("dolt-auto-push".into())
            .spawn(move || {
                eprintln!("[dolt] Auto-push started (threshold: {} commits, interval: {}s)",
                    push_threshold, push_interval_secs);

                while !thread_state.stop.load(Ordering::Relaxed) {
                    std::thread::sleep(check_interval);

                    if thread_state.stop.load(Ordering::Relaxed) {
                        break;
                    }

                    let commits = thread_state.commits_since_push.load(Ordering::Relaxed);
                    let idle_secs = thread_state.last_activity.lock()
                        .map(|t| t.elapsed().as_secs())
                        .unwrap_or(0);

                    let should_push =
                        (commits >= push_threshold) ||
                        (commits > 0 && idle_secs >= push_interval_secs);

                    if should_push {
                        eprintln!("[dolt] Auto-push triggered (commits: {}, idle: {}s)",
                            commits, idle_secs);

                        // Privacy encoding before push (if glyph feature enabled)
                        #[cfg(feature = "glyph")]
                        if enable_privacy {
                            if let Ok(mut conn) = pool.get_conn() {
                                let _ = conn.exec_drop(
                                    r"UPDATE memories
                                      SET content = CASE
                                          WHEN hallucinated = 1 THEN '[hallucination]'
                                          WHEN layer_depth = 0 THEN '[experience]'
                                          WHEN layer_depth <= 3 THEN '[knowledge]'
                                          ELSE '[insight]'
                                      END
                                      WHERE glyph_content IS NOT NULL",
                                    ()
                                );
                            }
                        }

                        // Stage, commit any pending, then push
                        let push_result = pool.get_conn().and_then(|mut conn| {
                            // Stage any unstaged changes
                            let _ = conn.exec_drop("CALL DOLT_ADD('.')", ());
                            // Try to commit (may be nothing)
                            let _ = conn.exec_drop(
                                "CALL DOLT_COMMIT('-m', ?, '--author', ?, '--allow-empty')",
                                ("auto-push: scheduled sync", &commit_author),
                            );
                            conn.exec_drop("CALL DOLT_PUSH(?, ?)", (&remote, &branch))
                        });

                        match push_result {
                            Ok(_) => {
                                thread_state.commits_since_push.store(0, Ordering::Relaxed);
                                eprintln!("[dolt] Auto-push succeeded to {}/{}", remote, branch);
                            }
                            Err(e) => {
                                eprintln!("[dolt] Auto-push failed (will retry): {}", e);
                            }
                        }
                    }
                }

                eprintln!("[dolt] Auto-push thread stopped");
            })
            .expect("Failed to spawn auto-push thread");

        Self {
            state,
            handle: Some(handle),
        }
    }

    /// Notify the auto-pusher that a commit occurred.
    pub fn notify_commit(&self) {
        self.state.commits_since_push.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut t) = self.state.last_activity.lock() {
            *t = Instant::now();
        }
    }

    /// Get the number of commits since the last push.
    pub fn commits_since_push(&self) -> usize {
        self.state.commits_since_push.load(Ordering::Relaxed)
    }

    /// Stop the auto-push background thread.
    pub fn stop(&mut self) {
        self.state.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for AutoPusher {
    fn drop(&mut self) {
        self.stop();
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
            "DOLT_AUTO_PUSH", "DOLT_PUSH_INTERVAL",
            "DOLT_PUSH_THRESHOLD", "DOLT_AGENT_ID",
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
        assert!(!cfg.auto_push);
        assert_eq!(cfg.push_interval_secs, 300);
        assert_eq!(cfg.push_threshold, 5);
        assert_eq!(cfg.agent_id, "local");
    }

    #[test]
    fn dolt_config_auto_push_env_overrides() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_dolt_env();
        std::env::set_var("DOLT_AUTO_PUSH", "true");
        std::env::set_var("DOLT_PUSH_INTERVAL", "60");
        std::env::set_var("DOLT_PUSH_THRESHOLD", "3");
        std::env::set_var("DOLT_AGENT_ID", "arc");
        let cfg = DoltConfig::from_env();
        clear_dolt_env();
        assert!(cfg.auto_push);
        assert_eq!(cfg.push_interval_secs, 60);
        assert_eq!(cfg.push_threshold, 3);
        assert_eq!(cfg.agent_id, "arc");
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

    // -----------------------------------------------------------------------
    // Glyph encoding tests (feature-gated)
    // -----------------------------------------------------------------------

    #[cfg(feature = "glyph")]
    #[test]
    fn glyph_encoding_roundtrip() {
        let content = "This is sensitive personal information that should be private.";
        let glyph_json = DoltMemoryStore::encode_content_as_glyph(content);
        
        assert!(glyph_json.is_some(), "Glyph encoding should succeed");
        
        let glyph_str = glyph_json.unwrap();
        // Verify the JSON doesn't contain human-readable plain text
        assert!(!glyph_str.contains("sensitive personal information"));
        assert!(!glyph_str.contains("should be private"));
        
        // Verify it's valid JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&glyph_str);
        assert!(parsed.is_ok(), "Glyph should be valid JSON");
    }

    #[cfg(feature = "glyph")]
    #[test]
    fn glyph_privacy_check() {
        let test_cases = vec![
            "My personal diary entry about feeling sad today",
            "Secret API key: sk-123456789abcdef",
            "John Doe lives at 123 Main Street, Anytown USA",
            "Phone number: +1-555-123-4567",
        ];
        
        for content in test_cases {
            let glyph_json = DoltMemoryStore::encode_content_as_glyph(content);
            assert!(glyph_json.is_some(), "Should encode: {}", content);
            
            let glyph_str = glyph_json.unwrap();
            // Verify no sensitive text leaks through
            let words: Vec<&str> = content.split_whitespace().collect();
            for word in words {
                if word.len() > 3 { // Skip short words like "at", "the"
                    assert!(!glyph_str.contains(word), "Glyph leaked word '{}' from: {}", word, content);
                }
            }
        }
    }

    #[cfg(feature = "glyph")]
    #[test]
    fn content_category_classification() {
        let memory1 = HyperMemory::new(vec![0.1; 100], "Test memory".to_string());
        let mut memory2 = memory1.clone();
        memory2.hallucinated = true;
        let mut memory3 = memory1.clone();
        memory3.layer_depth = 5;
        
        assert_eq!(DoltMemoryStore::content_category(&memory1), "[experience]");
        assert_eq!(DoltMemoryStore::content_category(&memory2), "[hallucination]");
        assert_eq!(DoltMemoryStore::content_category(&memory3), "[insight]");
    }

    #[cfg(feature = "glyph")]
    #[test]
    fn dolthub_content_privacy() {
        let memory = HyperMemory::new(vec![0.1; 100], "Sensitive personal data".to_string());
        
        let safe_content = DoltMemoryStore::dolthub_content(&memory, true);
        assert_eq!(safe_content, "[experience]");
        assert!(!safe_content.contains("Sensitive"));
        assert!(!safe_content.contains("personal"));
        
        let unsafe_content = DoltMemoryStore::dolthub_content(&memory, false);
        assert_eq!(unsafe_content, "Sensitive personal data");
    }

    #[cfg(feature = "glyph")]
    #[test]
    fn glyph_empty_content_handling() {
        let result = DoltMemoryStore::encode_content_as_glyph("");
        assert!(result.is_none(), "Empty content should return None");

        let result = DoltMemoryStore::encode_content_as_glyph("   ");
        assert!(result.is_some(), "Whitespace should encode successfully");
    }

    // -----------------------------------------------------------------------
    // Phase 3: Wave merge report tests
    // -----------------------------------------------------------------------

    #[test]
    fn wave_merge_report_clean() {
        let report = WaveMergeReport::clean();
        assert!(report.is_clean());
        assert_eq!(report.total_conflicts, 0);
        assert_eq!(report.constructive, 0);
        assert_eq!(report.destructive, 0);
        assert_eq!(report.partial, 0);
        assert_eq!(report.independent, 0);
        assert!(report.quarantined.is_empty());
    }

    #[test]
    fn wave_merge_report_with_conflicts_not_clean() {
        let report = WaveMergeReport {
            total_conflicts: 3,
            constructive: 2,
            destructive: 1,
            partial: 0,
            independent: 0,
            quarantined: vec![Uuid::new_v4()],
        };
        assert!(!report.is_clean());
        assert_eq!(report.total_conflicts, 3);
        assert_eq!(report.quarantined.len(), 1);
    }
}