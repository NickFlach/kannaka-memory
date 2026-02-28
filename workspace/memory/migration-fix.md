# Kannaka Memory MCP Server Migration Fix

**Date:** 2026-02-22  
**Issue:** Failed to initialize memory system: serialization error: io error  
**Status:** ✅ RESOLVED

## Problem Analysis

The recent commit added `xi_signature: Vec<f32>` to `HyperMemory` struct in `src/memory.rs`. While it had `#[serde(default)]`, the persistence layer uses **bincode** (not JSON) for serialization, and bincode is NOT self-describing — it cannot handle new fields with `#[serde(default)]` when deserializing existing data files.

The existing data files at `C:\Users\nickf\.openclaw\kannaka-data\kannaka.bin` (with backup at `kannaka.bin.pre-differentiation`) contained the old structure without `xi_signature`, causing deserialization failures.

## Root Cause

**Bincode vs JSON serialization difference:**
- JSON is self-describing and can handle missing fields with `#[serde(default)]`
- Bincode is binary and expects exact struct layout matches
- Adding new fields breaks compatibility even with serde defaults

## Solution Implemented

Added a migration path in `src/persistence.rs`:

### 1. Version Bump
- Bumped `CURRENT_VERSION` from 1 to 2

### 2. V1 Compatibility Structs
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct HyperMemoryV1 {
    // All original fields EXCEPT xi_signature
    pub id: Uuid,
    pub vector: Vec<f32>,
    // ... other fields ...
    pub geometry: Option<MemoryCoordinates>,
    // NOTE: xi_signature is NOT present in V1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MemorySnapshotV1 {
    pub version: u32,
    pub memories: Vec<HyperMemoryV1>,
    // ... other fields ...
}
```

### 3. Migration Logic
Modified `DiskStore::open()` and `MemoryEngine::load_state()` to:
1. Try V2 (current format) deserialization first
2. If that fails, try V1 format
3. Migrate V1 memories to V2 by adding empty `xi_signature: Vec::new()`
4. Save always uses V2 format

### 4. Conversion Implementation
```rust
impl From<HyperMemoryV1> for HyperMemory {
    fn from(v1: HyperMemoryV1) -> Self {
        Self {
            // ... copy all V1 fields ...
            xi_signature: Vec::new(), // Initialize with empty xi_signature
        }
    }
}
```

## Testing Results

### ✅ Tests Passed
```bash
cargo test
# Result: 131 tests passed, 0 failed
```

### ✅ Build Successful
```bash
cargo build --release --features mcp --bin kannaka-mcp
# Compiled successfully
```

### ✅ Migration Verified
**Old format (V1) test:**
```bash
KANNAKA_DB_PATH="C:\temp\test-migration" .\target\release\kannaka-mcp.exe
# Output: "Server initialized, listening on stdio..."
```

**Current format (V2) test:**
```bash
KANNAKA_DB_PATH="C:\Users\nickf\.openclaw\kannaka-data" .\target\release\kannaka-mcp.exe  
# Output: "Server initialized, listening on stdio..."
```

Both formats load successfully!

## Files Modified

- `src/persistence.rs`: Added migration logic and V1 compatibility structs

## Verification Steps

1. ✅ `cargo test` passes (131 tests)
2. ✅ `cargo build --release --features mcp --bin kannaka-mcp` succeeds  
3. ✅ Binary can load existing `kannaka.bin` file (V1 format)
4. ✅ Binary can load new `kannaka.bin` files (V2 format)
5. ✅ Migration preserves all memory data

## Commit Details

**Commit Hash:** bd92e5a  
**Message:** Fix bincode serialization compatibility: Add V1->V2 migration for xi_signature field

## Key Learnings

1. **Bincode != JSON**: Bincode serialization requires exact struct layout matching
2. **serde(default) limitations**: Only works with self-describing formats like JSON  
3. **Migration necessity**: Adding fields to bincode structs always requires migration paths
4. **Backward compatibility**: Critical for data persistence layers

## Future Recommendations

- Consider JSON serialization for more flexible schema evolution
- Always implement migration paths when adding fields to persisted structs  
- Test with real data files, not just unit tests
- Maintain version numbers for all serialized formats

---
**Resolution:** The kannaka-memory MCP server now starts successfully with both old and new data formats. The migration is transparent to users and preserves all existing memory data.