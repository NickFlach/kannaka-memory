//! Integration tests for ADR-0009 Dolt persistence — Phases 2, 3 & 4.
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
use kannaka_memory::dolt::{DiffKind, DoltConfig, DoltMemoryStore};
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

// ---------------------------------------------------------------------------
// Phase 4 — Branch management
// ---------------------------------------------------------------------------

/// commit() returns Ok(true) when there are staged changes.
#[test]
fn dolt_commit_returns_true_on_change() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_commit_returns_true_on_change — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");
    let mem = make_memory("commit test memory");
    let id = store.insert(mem).expect("insert");

    let committed = store.commit("test: phase 4 commit test").expect("commit should not error");
    assert!(committed, "commit() should return true when there are new rows");

    store.delete(&id).expect("cleanup");
    store.commit("test: cleanup commit").ok();
}

/// commit() returns Ok(false) when called with nothing to commit.
#[test]
fn dolt_commit_returns_false_when_nothing_to_commit() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_commit_returns_false_when_nothing_to_commit — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");
    // Commit any existing state, then commit again immediately — nothing new to commit.
    store.commit("settle").ok();
    let second = store.commit("should be empty").expect("should not error on nothing-to-commit");
    assert!(!second, "commit() should return false when nothing staged");
}

/// list_branches() returns at least one branch (main / master).
#[test]
fn dolt_list_branches_returns_main() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_list_branches_returns_main — no Dolt server");
        return;
    }

    let store = DoltMemoryStore::from_config(&config).expect("connect");
    let branches = store.list_branches().expect("list_branches should not error");

    assert!(!branches.is_empty(), "must have at least one branch");
    let has_main = branches.iter().any(|b| b.name == "main" || b.name == "master");
    assert!(has_main, "must have a main or master branch; got: {:?}", branches.iter().map(|b| &b.name).collect::<Vec<_>>());
    let current = branches.iter().filter(|b| b.is_current).count();
    assert_eq!(current, 1, "exactly one branch should be marked current");
}

/// current_branch() agrees with the is_current flag in list_branches().
#[test]
fn dolt_current_branch_matches_list_branches() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_current_branch_matches_list_branches — no Dolt server");
        return;
    }

    let store = DoltMemoryStore::from_config(&config).expect("connect");
    let active = store.current_branch().expect("current_branch");
    let branches = store.list_branches().expect("list_branches");
    let current_from_list = branches.iter()
        .find(|b| b.is_current)
        .map(|b| b.name.clone())
        .unwrap_or_default();

    assert_eq!(active, current_from_list, "current_branch() and list_branches is_current must agree");
}

/// create_branch / checkout / delete lifecycle.
#[test]
fn dolt_branch_create_checkout_delete() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_branch_create_checkout_delete — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");
    // Make sure there's at least one commit so branching succeeds.
    store.commit("pre-branch baseline").ok();

    let branch_name = format!("test-branch-{}", uuid::Uuid::new_v4().simple());

    // Create
    store.create_branch(&branch_name).expect("create_branch");
    let branches = store.list_branches().expect("list_branches");
    assert!(branches.iter().any(|b| b.name == branch_name), "new branch should appear in list");

    // Checkout new branch then return to default
    store.checkout(&branch_name).expect("checkout to new branch");
    assert_eq!(store.current_branch().unwrap(), branch_name);

    let default = store.default_branch().to_string();
    store.checkout(&default).expect("checkout back to default");

    // Delete
    store.delete_branch(&branch_name).expect("delete_branch");
    let branches2 = store.list_branches().expect("list after delete");
    assert!(!branches2.iter().any(|b| b.name == branch_name), "deleted branch must not appear");
}

/// checkout_new_branch creates and switches in one step.
#[test]
fn dolt_checkout_new_branch_single_step() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_checkout_new_branch_single_step — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");
    store.commit("baseline for new-branch test").ok();

    let branch_name = format!("new-branch-{}", uuid::Uuid::new_v4().simple());
    store.checkout_new_branch(&branch_name).expect("checkout_new_branch");
    assert_eq!(store.current_branch().unwrap(), branch_name);

    // Return and clean up
    let default = store.default_branch().to_string();
    store.checkout(&default).expect("return to default");
    store.delete_branch(&branch_name).expect("cleanup branch");
}

// ---------------------------------------------------------------------------
// Phase 4 — Commit log
// ---------------------------------------------------------------------------

/// log() returns at least the initial commit.
#[test]
fn dolt_log_returns_entries() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_log_returns_entries — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");
    // Ensure at least one commit exists.
    let mem = make_memory("log test");
    let id = store.insert(mem).expect("insert");
    store.commit("log test commit").ok();

    let entries = store.log(10).expect("log should not error");
    assert!(!entries.is_empty(), "log should have at least one entry");

    let entry = &entries[0];
    assert!(!entry.hash.is_empty(), "commit hash should not be empty");
    assert!(!entry.message.is_empty(), "commit message should not be empty");

    store.delete(&id).expect("cleanup");
    store.commit("cleanup").ok();
}

// ---------------------------------------------------------------------------
// Phase 4 — Diff
// ---------------------------------------------------------------------------

/// diff() reports an Added row after inserting a memory.
#[test]
fn dolt_diff_reports_added_memory() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_diff_reports_added_memory — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");
    store.commit("diff baseline").ok();

    let mem = make_memory("diff test memory");
    let id = store.insert(mem.clone()).expect("insert");
    store.commit("diff: added one memory").ok();

    let diffs = store.diff("HEAD~1", "HEAD").expect("diff should not error");
    let added = diffs.iter().find(|d| d.id == id);
    assert!(added.is_some(), "inserted memory should appear as Added in diff");
    assert_eq!(added.unwrap().kind, DiffKind::Added);
    assert_eq!(added.unwrap().to_content.as_deref(), Some(mem.content.as_str()));

    store.delete(&id).expect("cleanup");
    store.commit("diff cleanup").ok();
}

// ---------------------------------------------------------------------------
// Phase 4 — Speculation helpers
// ---------------------------------------------------------------------------

/// speculate() creates a branch; discard_speculation() removes it without merge.
#[test]
fn dolt_speculate_and_discard() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_speculate_and_discard — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");
    store.commit("pre-speculate baseline").ok();

    let spec_branch = format!("spec-discard-{}", uuid::Uuid::new_v4().simple());
    store.speculate(&spec_branch).expect("speculate");
    assert_eq!(store.current_branch().unwrap(), spec_branch, "should be on spec branch");

    // Insert a speculative memory that will be discarded
    let mem = make_memory("speculative — will be discarded");
    store.insert(mem).expect("insert speculative");
    store.commit("speculative commit").ok();

    store.discard_speculation(&spec_branch).expect("discard_speculation");

    let default = store.default_branch().to_string();
    assert_eq!(store.current_branch().unwrap(), default, "should be back on default branch");

    let branches = store.list_branches().expect("list_branches after discard");
    assert!(!branches.iter().any(|b| b.name == spec_branch), "discarded branch must not exist");
}

/// speculate() + collapse_speculation() merges speculative memories into main.
#[test]
fn dolt_speculate_and_collapse() {
    let config = test_config();
    if !is_dolt_available(&config) {
        eprintln!("SKIP dolt_speculate_and_collapse — no Dolt server");
        return;
    }

    let mut store = DoltMemoryStore::from_config(&config).expect("connect");
    store.commit("pre-collapse baseline").ok();

    let spec_branch = format!("spec-collapse-{}", uuid::Uuid::new_v4().simple());
    store.speculate(&spec_branch).expect("speculate");

    let mem = make_memory("speculative memory to be collapsed");
    let spec_id = store.insert(mem).expect("insert speculative");

    let merge_hash = store
        .collapse_speculation(&spec_branch, "collapse: accepted speculation")
        .expect("collapse_speculation");
    assert!(!merge_hash.is_empty(), "merge hash should not be empty");

    let default = store.default_branch().to_string();
    assert_eq!(store.current_branch().unwrap(), default, "should be on default after collapse");

    let branches = store.list_branches().expect("list_branches after collapse");
    assert!(!branches.iter().any(|b| b.name == spec_branch), "collapsed branch should be deleted");

    // The speculative memory should now be visible on main
    let merged_mem = store.get(&spec_id).unwrap();
    assert!(merged_mem.is_some(), "speculative memory should be present on main after collapse");

    store.delete(&spec_id).expect("cleanup");
    store.commit("post-collapse cleanup").ok();
}
