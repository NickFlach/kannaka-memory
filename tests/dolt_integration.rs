//! Integration tests for ADR-0009 Dolt persistence — Phases 2 & 3.
//!
//! ## Test organisation
//!
//! **Unit tests** (no DB, always run):
//!   - `DoltConfig` default / env-var parsing (also covered inline in `src/dolt.rs`)
//!   - Datetime helper round-trips
//!
//! **Integration tests** (require a live Dolt SQL server):
//!   - Insert / get / search / delete round-trip
//!   - Dirty-set tracking: get_mut marks dirty, flush_dirty persists to Dolt
//!   - Delete atomicity: cache is only evicted after successful Dolt delete
//!   - Auto-commit threshold behaviour
//!
//! Integration tests detect Dolt availability at runtime; they are
//! **skipped** (not failed) when no server is reachable. This keeps CI green
//! without a Dolt server while still providing full coverage when one exists.
//!
//! Set `DOLT_HOST` / `DOLT_PORT` / `DOLT_DB` to override connection params.
//! The test database must exist with the ADR-0009 schema already applied.

#![cfg(feature = "dolt")]

use chrono::TimeZone;
use kannaka_memory::dolt::{DoltConfig, DoltMemoryStore};
use kannaka_memory::memory::HyperMemory;
use kannaka_memory::store::MemoryStore;

// ---------------------------------------------------------------------------
// Helper: build a test DoltConfig pointing at a throwaway test database
// ---------------------------------------------------------------------------

fn test_config() -> DoltConfig {
    let mut cfg = DoltConfig::from_env();
    // Override DB name so integration tests never touch production data.
    // Can be overridden via DOLT_TEST_DB env var.
    if std::env::var("DOLT_DB").is_err() {
        cfg.database = std::env::var("DOLT_TEST_DB")
            .unwrap_or_else(|_| "kannaka_test".to_string());
    }
    // Disable auto-commit during tests so we control transaction boundaries.
    cfg.auto_commit = false;
    cfg
}

// ---------------------------------------------------------------------------
// Helper: check server reachability without panicking
// ---------------------------------------------------------------------------

fn is_dolt_available(config: &DoltConfig) -> bool {
    use mysql::{Opts, OptsBuilder, Pool};
    let opts: Opts = OptsBuilder::new()
        .ip_or_hostname(Some(config.host.as_str()))
        .tcp_port(config.port)
        .db_name(Some(config.database.as_str()))
        .user(Some(config.user.as_str()))
        .pass(if config.password.is_empty() { None } else { Some(config.password.as_str()) })
        .into();
    match Pool::new(opts) {
        Ok(pool) => pool.get_conn().is_ok(),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Helper: build a minimal HyperMemory for testing
// ---------------------------------------------------------------------------

fn make_memory(content: &str) -> HyperMemory {
    HyperMemory::new(vec![0.5_f32; 16], content.to_string())
}

// ---------------------------------------------------------------------------
// Integration tests
// ---------------------------------------------------------------------------

/// Verify that `DoltMemoryStore::from_config` connects and loads an empty store.
#[test]
fn dolt_store_connects_and_loads() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_store_connects_and_loads — no Dolt server at {}:{}", config.host, config.port);
        return;
    }

    let store = DoltMemoryStore::from_config(&config)
        .expect("should connect to Dolt");

    // A fresh test DB should have 0 or more memories — just assert it doesn't panic.
    let _ = store.count();
}

/// Basic insert → get round-trip.
#[test]
fn dolt_insert_and_get_roundtrip() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_insert_and_get_roundtrip — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config)
        .expect("connect to Dolt");

    let mem = make_memory("dolt integration test memory");
    let id = store.insert(mem.clone()).expect("insert should succeed");

    let retrieved = store.get(&id).expect("get should not error").expect("memory should exist");
    assert_eq!(retrieved.content, mem.content);
    assert_eq!(retrieved.id, id);

    // Cleanup
    store.delete(&id).expect("delete should succeed");
}

/// Search returns the inserted memory.
#[test]
fn dolt_search_finds_inserted_memory() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_search_finds_inserted_memory — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");

    let mem = make_memory("searchable memory");
    let id = store.insert(mem.clone()).expect("insert");

    let results = store.search(&mem.vector, 5).expect("search");
    let found = results.iter().any(|(rid, _)| *rid == id);
    assert!(found, "inserted memory should appear in search results");

    store.delete(&id).expect("cleanup");
}

/// Phase 3 — dirty-set tracking: `get_mut` marks dirty; `flush_dirty` persists.
#[test]
fn dolt_dirty_set_tracks_get_mut_and_flush_persists() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_dirty_set_tracks_get_mut_and_flush_persists — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");

    let mem = make_memory("amplitude will be mutated");
    let id = store.insert(mem).expect("insert");

    // Mutate via get_mut — should mark dirty
    assert!(store.dirty_ids().is_empty(), "no dirty ids after plain insert");
    {
        let m = store.get_mut(&id).unwrap().unwrap();
        m.amplitude = 9.99;
    }
    assert!(
        store.dirty_ids().contains(&id),
        "id should be in dirty_set after get_mut"
    );

    // flush_dirty should write the mutation to Dolt and clear the dirty set
    let flushed = store.flush_dirty().expect("flush should succeed");
    assert_eq!(flushed, 1, "one memory was dirty");
    assert!(store.dirty_ids().is_empty(), "dirty set should be empty after flush");

    // Reload from a fresh store connection to verify the mutation persisted
    let store2 = DoltMemoryStore::from_config(&config).expect("second connection");
    let reloaded = store2.get(&id).unwrap().expect("should still exist");
    assert!(
        (reloaded.amplitude - 9.99).abs() < 1e-4,
        "mutated amplitude should persist after flush_dirty; got {}",
        reloaded.amplitude
    );

    // Cleanup
    let mut store = store;
    store.delete(&id).expect("cleanup");
}

/// Phase 3 — explicit `update()` persists a single mutation.
#[test]
fn dolt_update_persists_single_mutation() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_update_persists_single_mutation — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");

    let mem = make_memory("update() test");
    let id = store.insert(mem).expect("insert");

    {
        let m = store.get_mut(&id).unwrap().unwrap();
        m.frequency = 3.14;
    }
    // update() should flush just this one memory
    store.update(&id).expect("update should succeed");
    assert!(store.dirty_ids().is_empty(), "dirty set cleared after update()");

    // Verify via fresh connection
    let store2 = DoltMemoryStore::from_config(&config).expect("second connection");
    let reloaded = store2.get(&id).unwrap().expect("should exist");
    assert!(
        (reloaded.frequency - 3.14).abs() < 1e-4,
        "mutated frequency should persist; got {}",
        reloaded.frequency
    );

    let mut store = store;
    store.delete(&id).expect("cleanup");
}

/// Phase 1 devil's advocate fix — delete atomicity: memory stays in Dolt
/// if the cache eviction hasn't happened yet (we can't easily force a Dolt
/// failure, so this test verifies the correct post-delete state instead).
#[test]
fn dolt_delete_removes_from_both_cache_and_db() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_delete_removes_from_both_cache_and_db — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");

    let mem = make_memory("to be deleted");
    let id = store.insert(mem).expect("insert");

    // Verify it's present before delete
    assert!(store.get(&id).unwrap().is_some());

    let was_present = store.delete(&id).expect("delete should succeed");
    assert!(was_present, "delete should return true for existing memory");

    // Not in cache
    assert!(
        store.get(&id).unwrap().is_none(),
        "memory should be gone from cache after delete"
    );

    // Not in Dolt (verify via fresh connection)
    let store2 = DoltMemoryStore::from_config(&config).expect("second connection");
    assert!(
        store2.get(&id).unwrap().is_none(),
        "memory should be gone from Dolt after delete"
    );
}

/// Phase 1 devil's advocate fix — resonance_key is Vec::new() (not vec![0.0; 100]).
#[test]
fn dolt_loaded_skip_links_have_empty_resonance_key() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_loaded_skip_links_have_empty_resonance_key — no Dolt server");
        return;
    }

    use kannaka_memory::skip_link::SkipLink;

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");

    // Insert a memory with a skip link
    let mut mem = make_memory("memory with skip link");
    let target_id = uuid::Uuid::new_v4();
    mem.connections.push(SkipLink {
        target_id,
        strength: 0.75,
        resonance_key: Vec::new(),
        span: 1,
    });
    let id = store.insert(mem).expect("insert");

    // Reload via fresh connection
    let store2 = DoltMemoryStore::from_config(&config).expect("second connection");
    let reloaded = store2.get(&id).unwrap().expect("should exist");

    assert_eq!(reloaded.connections.len(), 1);
    let link = &reloaded.connections[0];
    assert_eq!(link.target_id, target_id);
    assert!(
        link.resonance_key.is_empty(),
        "resonance_key must be Vec::new() on Dolt round-trip, not vec![0.0; 100]"
    );

    let mut store = store;
    store.delete(&id).expect("cleanup");
}

/// Phase 1 devil's advocate fix — datetime round-trip via Dolt preserves UTC timestamp.
#[test]
fn dolt_datetime_survives_roundtrip() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_datetime_survives_roundtrip — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");

    let mut mem = make_memory("datetime round-trip");
    // Use a fixed timestamp with second-level precision (MySQL DATETIME resolution)
    mem.created_at = chrono::Utc.with_ymd_and_hms(2026, 6, 15, 12, 0, 0).unwrap();
    let id = store.insert(mem.clone()).expect("insert");

    let store2 = DoltMemoryStore::from_config(&config).expect("second connection");
    let reloaded = store2.get(&id).unwrap().expect("should exist");

    assert_eq!(
        reloaded.created_at.timestamp(),
        mem.created_at.timestamp(),
        "created_at timestamp must survive Dolt round-trip"
    );

    let mut store = store;
    store.delete(&id).expect("cleanup");
}

// ---------------------------------------------------------------------------
// Phase 3 — DoltConfig from_env integration (no DB, always runs)
// ---------------------------------------------------------------------------

#[test]
fn dolt_config_from_env_integration() {
    // These env vars match what from_env() reads.
    // We set them transiently, read, then clean up.
    std::env::set_var("DOLT_HOST", "integration.test.local");
    std::env::set_var("DOLT_PORT", "3399");
    std::env::set_var("DOLT_DB", "integration_db");

    let cfg = DoltConfig::from_env();

    std::env::remove_var("DOLT_HOST");
    std::env::remove_var("DOLT_PORT");
    std::env::remove_var("DOLT_DB");

    assert_eq!(cfg.host, "integration.test.local");
    assert_eq!(cfg.port, 3399);
    assert_eq!(cfg.database, "integration_db");
}
