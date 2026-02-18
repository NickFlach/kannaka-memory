//! Migration from legacy kannaka.db (SQLite) to the hypervector memory system.
//!
//! Reads working_memory, events, entities, relationships, and lessons tables
//! from the old SQLite database and encodes each record as a HyperMemory with
//! table-appropriate layer depth and wave parameters.

use std::path::PathBuf;
use std::time::Instant;

use chrono::Utc;
use rusqlite::Connection;
use thiserror::Error;

use crate::encoding::EncodingPipeline;
use crate::memory::HyperMemory;
use crate::store::MemoryEngine;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("database error: {0}")]
    DatabaseError(String),
    #[error("encoding error: {0}")]
    EncodingError(String),
    #[error("table not found: {0}")]
    TableNotFound(String),
}

impl From<rusqlite::Error> for MigrationError {
    fn from(e: rusqlite::Error) -> Self {
        MigrationError::DatabaseError(e.to_string())
    }
}

impl From<crate::encoding::EncodingError> for MigrationError {
    fn from(e: crate::encoding::EncodingError) -> Self {
        MigrationError::EncodingError(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// MigrationReport
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct MigrationReport {
    pub working_memory_count: usize,
    pub events_count: usize,
    pub entities_count: usize,
    pub relationships_count: usize,
    pub lessons_count: usize,
    pub total_migrated: usize,
    pub skip_links_created: usize,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Table configuration
// ---------------------------------------------------------------------------

struct TableConfig {
    name: &'static str,
    layer_depth: u8,
    amplitude: f32,
    frequency: f32,
}

const TABLES: &[TableConfig] = &[
    TableConfig { name: "working_memory", layer_depth: 0, amplitude: 0.8, frequency: 0.5 },
    TableConfig { name: "events",         layer_depth: 1, amplitude: 1.0, frequency: 0.1 },
    TableConfig { name: "entities",       layer_depth: 2, amplitude: 1.0, frequency: 0.05 },
    TableConfig { name: "relationships",  layer_depth: 2, amplitude: 1.0, frequency: 0.05 },
    TableConfig { name: "lessons",        layer_depth: 3, amplitude: 1.5, frequency: 0.02 },
];

// ---------------------------------------------------------------------------
// KannakaDbMigrator
// ---------------------------------------------------------------------------

pub struct KannakaDbMigrator {
    db_path: PathBuf,
    pipeline: EncodingPipeline,
}

impl KannakaDbMigrator {
    pub fn new(db_path: impl Into<PathBuf>, pipeline: EncodingPipeline) -> Self {
        Self {
            db_path: db_path.into(),
            pipeline,
        }
    }

    /// Migrate all records from kannaka.db into HyperMemory objects.
    /// Returns the memories and a report. If the db doesn't exist, returns an empty report.
    pub fn migrate(&self) -> Result<(Vec<HyperMemory>, MigrationReport), MigrationError> {
        let start = Instant::now();

        if !self.db_path.exists() {
            return Ok((Vec::new(), MigrationReport {
                duration_ms: start.elapsed().as_millis() as u64,
                ..Default::default()
            }));
        }

        let conn = Connection::open(&self.db_path)?;
        let mut memories = Vec::new();
        let mut report = MigrationReport::default();

        for table in TABLES {
            match self.migrate_table(&conn, table) {
                Ok(mems) => {
                    let count = mems.len();
                    match table.name {
                        "working_memory" => report.working_memory_count = count,
                        "events" => report.events_count = count,
                        "entities" => report.entities_count = count,
                        "relationships" => report.relationships_count = count,
                        "lessons" => report.lessons_count = count,
                        _ => {}
                    }
                    memories.extend(mems);
                }
                Err(MigrationError::DatabaseError(ref e)) if e.contains("no such table") => {
                    report.errors.push(format!("table '{}' not found, skipped", table.name));
                }
                Err(e) => {
                    report.errors.push(format!("error reading '{}': {}", table.name, e));
                }
            }
        }

        report.total_migrated = memories.len();
        report.duration_ms = start.elapsed().as_millis() as u64;
        Ok((memories, report))
    }

    /// Migrate all records directly into a MemoryEngine, then run consolidation
    /// to wire skip links between related migrated memories.
    pub fn migrate_into(&self, engine: &mut MemoryEngine) -> Result<MigrationReport, MigrationError> {
        let (memories, mut report) = self.migrate()?;

        let mut link_count = 0usize;
        for mem in memories {
            let id = mem.id;
            if let Err(e) = engine.store.insert(mem) {
                report.errors.push(format!("insert error: {}", e));
                continue;
            }
            // Create skip links to existing memories
            match engine.create_skip_links(&id) {
                Ok(links) => link_count += links.len(),
                Err(e) => report.errors.push(format!("skip link error: {}", e)),
            }
        }

        report.skip_links_created = link_count;
        Ok(report)
    }

    /// Read all rows from a single table and encode them as HyperMemory objects.
    fn migrate_table(
        &self,
        conn: &Connection,
        config: &TableConfig,
    ) -> Result<Vec<HyperMemory>, MigrationError> {
        // Get column names for this table
        let columns = self.get_text_columns(conn, config.name)?;
        if columns.is_empty() {
            return Ok(Vec::new());
        }

        let select_cols = columns.join(", ");
        let sql = format!("SELECT {} FROM {}", select_cols, config.name);

        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query([])?;
        let mut memories = Vec::new();

        while let Some(row) = rows.next()? {
            // Combine all text columns into one string
            let mut parts = Vec::new();
            for (i, col) in columns.iter().enumerate() {
                if let Ok(val) = row.get::<_, String>(i) {
                    if !val.is_empty() {
                        parts.push(format!("{}: {}", col, val));
                    }
                }
            }

            let text = parts.join(" | ");
            if text.trim().is_empty() {
                continue;
            }

            match self.pipeline.encode_memory(&text, Utc::now()) {
                Ok(mut mem) => {
                    mem.layer_depth = config.layer_depth;
                    mem.amplitude = config.amplitude;
                    mem.frequency = config.frequency;
                    memories.push(mem);
                }
                Err(_e) => {
                    // Skip encoding errors for individual rows
                    continue;
                }
            }
        }

        Ok(memories)
    }

    /// Get all TEXT/VARCHAR column names for a table.
    fn get_text_columns(&self, conn: &Connection, table: &str) -> Result<Vec<String>, MigrationError> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", table))?;
        let mut rows = stmt.query([])?;
        let mut columns = Vec::new();

        // Check if we got any rows (table exists)
        let mut found_any = false;
        while let Some(row) = rows.next()? {
            found_any = true;
            let col_name: String = row.get(1)?;
            let col_type: String = row.get(2).unwrap_or_default();
            let type_upper = col_type.to_uppercase();
            // Include TEXT, VARCHAR, or untyped columns (SQLite is flexible)
            if type_upper.contains("TEXT") || type_upper.contains("VARCHAR")
                || type_upper.contains("CHAR") || type_upper.is_empty()
            {
                columns.push(col_name);
            }
        }

        if !found_any {
            return Err(MigrationError::DatabaseError(format!("no such table: {}", table)));
        }

        Ok(columns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebook::Codebook;
    use crate::encoding::SimpleHashEncoder;
    use crate::store::{InMemoryStore, MemoryEngine};
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn make_pipeline() -> EncodingPipeline {
        let encoder = SimpleHashEncoder::new(384, 42);
        let codebook = Codebook::new(384, 10_000, 42);
        EncodingPipeline::new(Box::new(encoder), codebook)
    }

    fn create_test_db() -> (NamedTempFile, PathBuf) {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("
            CREATE TABLE working_memory (id INTEGER PRIMARY KEY, content TEXT, context TEXT);
            CREATE TABLE events (id INTEGER PRIMARY KEY, description TEXT, timestamp TEXT);
            CREATE TABLE entities (id INTEGER PRIMARY KEY, name TEXT, type TEXT, description TEXT);
            CREATE TABLE relationships (id INTEGER PRIMARY KEY, source TEXT, target TEXT, relation TEXT);
            CREATE TABLE lessons (id INTEGER PRIMARY KEY, lesson TEXT, context TEXT);

            INSERT INTO working_memory (content, context) VALUES ('current task is building memory system', 'development');
            INSERT INTO working_memory (content, context) VALUES ('user prefers Rust over Python', 'preferences');

            INSERT INTO events (description, timestamp) VALUES ('started kannaka project', '2026-01-15');
            INSERT INTO events (description, timestamp) VALUES ('first successful memory recall', '2026-01-20');

            INSERT INTO entities (name, type, description) VALUES ('Nick', 'person', 'the creator');
            INSERT INTO entities (name, type, description) VALUES ('Kannaka', 'project', 'AI memory system');

            INSERT INTO relationships (source, target, relation) VALUES ('Nick', 'Kannaka', 'creator of');

            INSERT INTO lessons (lesson, context) VALUES ('hypervectors preserve similarity under binding', 'architecture');
            INSERT INTO lessons (lesson, context) VALUES ('wave modulation enables natural forgetting', 'design');
            INSERT INTO lessons (lesson, context) VALUES ('skip links accelerate associative recall', 'performance');
        ").unwrap();

        (tmp, path)
    }

    #[test]
    fn migrate_creates_memories_from_all_tables() {
        let (_tmp, path) = create_test_db();
        let migrator = KannakaDbMigrator::new(&path, make_pipeline());

        let (memories, report) = migrator.migrate().unwrap();

        assert_eq!(report.working_memory_count, 2);
        assert_eq!(report.events_count, 2);
        assert_eq!(report.entities_count, 2);
        assert_eq!(report.relationships_count, 1);
        assert_eq!(report.lessons_count, 3);
        assert_eq!(report.total_migrated, 10);
        assert_eq!(memories.len(), 10);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn layer_depth_mapping_correct() {
        let (_tmp, path) = create_test_db();
        let migrator = KannakaDbMigrator::new(&path, make_pipeline());

        let (memories, _) = migrator.migrate().unwrap();

        // working_memory -> layer 0, events -> 1, entities -> 2, relationships -> 2, lessons -> 3
        let wm: Vec<_> = memories.iter().filter(|m| m.layer_depth == 0).collect();
        let ev: Vec<_> = memories.iter().filter(|m| m.layer_depth == 1).collect();
        let ent_rel: Vec<_> = memories.iter().filter(|m| m.layer_depth == 2).collect();
        let les: Vec<_> = memories.iter().filter(|m| m.layer_depth == 3).collect();

        assert_eq!(wm.len(), 2);
        assert_eq!(ev.len(), 2);
        assert_eq!(ent_rel.len(), 3); // 2 entities + 1 relationship
        assert_eq!(les.len(), 3);
    }

    #[test]
    fn lessons_get_higher_amplitude_than_working_memory() {
        let (_tmp, path) = create_test_db();
        let migrator = KannakaDbMigrator::new(&path, make_pipeline());

        let (memories, _) = migrator.migrate().unwrap();

        let wm_amp: f32 = memories.iter()
            .filter(|m| m.layer_depth == 0)
            .map(|m| m.amplitude)
            .next().unwrap();
        let lesson_amp: f32 = memories.iter()
            .filter(|m| m.layer_depth == 3)
            .map(|m| m.amplitude)
            .next().unwrap();

        assert!(lesson_amp > wm_amp,
            "lessons amplitude {} should be > working_memory amplitude {}", lesson_amp, wm_amp);
    }

    #[test]
    fn empty_db_produces_empty_report() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();
        // Create empty db (no tables)
        let _conn = Connection::open(&path).unwrap();

        let migrator = KannakaDbMigrator::new(&path, make_pipeline());
        let (memories, report) = migrator.migrate().unwrap();

        assert_eq!(report.total_migrated, 0);
        assert_eq!(memories.len(), 0);
        // All tables should be noted as missing
        assert_eq!(report.errors.len(), 5);
    }

    #[test]
    fn missing_db_returns_empty_report() {
        let migrator = KannakaDbMigrator::new("/nonexistent/path/kannaka.db", make_pipeline());
        let (memories, report) = migrator.migrate().unwrap();

        assert_eq!(report.total_migrated, 0);
        assert_eq!(memories.len(), 0);
        assert!(report.errors.is_empty());
    }

    #[test]
    fn migrate_into_inserts_and_memories_are_recallable() {
        let (_tmp, path) = create_test_db();
        let migrator = KannakaDbMigrator::new(&path, make_pipeline());

        let store = InMemoryStore::new();
        let mut engine = MemoryEngine::new(Box::new(store), make_pipeline());
        // Lower threshold so skip links form more easily in tests
        engine.similarity_threshold = 0.1;

        let report = migrator.migrate_into(&mut engine).unwrap();

        assert_eq!(report.total_migrated, 10);
        assert_eq!(engine.store.count(), 10);

        // Recall should find related memories
        let results = engine.recall("kannaka memory system", 5).unwrap();
        assert!(!results.is_empty(), "should recall at least one memory");
    }

    #[test]
    fn missing_tables_dont_crash() {
        let tmp = NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        let conn = Connection::open(&path).unwrap();
        // Only create some tables
        conn.execute_batch("
            CREATE TABLE events (id INTEGER PRIMARY KEY, description TEXT);
            INSERT INTO events (description) VALUES ('test event');
        ").unwrap();

        let migrator = KannakaDbMigrator::new(&path, make_pipeline());
        let (memories, report) = migrator.migrate().unwrap();

        assert_eq!(report.events_count, 1);
        assert_eq!(report.total_migrated, 1);
        // 4 missing tables should be noted
        assert_eq!(report.errors.len(), 4);
        assert!(report.errors.iter().all(|e| e.contains("not found")));
    }
}
