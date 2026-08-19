//! Atomic file writes — ONE implementation (#777).
//!
//! The write-to-temp-then-rename pattern existed as five private copies, and
//! the copies had already diverged in ways that mattered:
//!
//!   - `hrm_store`'s copy used `std::fs::write` with NO `sync_all()`, so a
//!     crash between OS-level write completion and disk flush could silently
//!     lose sidecar state (links.json, reactivation) even though the rename
//!     appeared to succeed;
//!   - the same copy named its temp file `path.with_extension("tmp.<pid>")`,
//!     which for a dotless basename (`links`) REPLACES the name rather than
//!     creating a sibling — every other copy used a UUID-named sibling, which
//!     is always correct;
//!   - return types disagreed (`Result` vs silent `()`), so callers could not
//!     be moved between copies without behavior change.
//!
//! "Matching X" comments pointing between the copies were the tell: a comment
//! that says two functions must stay in sync is a defect locator. Now they
//! cannot drift, because there is one function to drift from.

use std::path::Path;

/// Write `bytes` to `path` atomically: UUID-named temp sibling in the same
/// directory (same-filesystem rename is an atomic swap) → `write_all` →
/// `sync_all` → rename over the target. A reader always sees either the old
/// or the new file whole; a crash never leaves a truncated target.
///
/// The temp file is removed on every failure path.
pub(crate) fn atomic_write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    atomic_write_bytes_mode(path, bytes, None)
}

/// `atomic_write_bytes` with an optional unix permission mode applied to the
/// temp file before the rename (coding_tools writes agent-visible files as
/// 0o644). On non-unix the mode is ignored.
pub(crate) fn atomic_write_bytes_mode(
    path: &Path,
    bytes: &[u8],
    #[cfg_attr(not(unix), allow(unused_variables))] mode: Option<u32>,
) -> Result<(), String> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Path::new(".").to_path_buf());
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let tmp = dir.join(format!(".kannaka-tmp-{}", uuid::Uuid::new_v4()));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp).map_err(|e| format!("temp create: {e}"))?;
        if let Err(e) = f.write_all(bytes) {
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("write: {e}"));
        }
        if let Err(e) = f.sync_all() {
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("sync: {e}"));
        }
    }
    #[cfg(unix)]
    if let Some(m) = mode {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(m)) {
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("chmod: {e}"));
        }
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("rename: {e}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("kannaka-fsutil-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn writes_and_overwrites_whole_files() {
        let d = temp_dir("roundtrip");
        let p = d.join("state.json");
        atomic_write_bytes(&p, b"first").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"first");
        atomic_write_bytes(&p, b"second, longer than first").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"second, longer than first");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// The hrm_store copy's `with_extension("tmp.<pid>")` REPLACED a dotless
    /// basename instead of creating a sibling. The unified temp naming must
    /// handle `links` (no dot) exactly like `links.json`.
    #[test]
    fn dotless_basenames_are_safe() {
        let d = temp_dir("dotless");
        let p = d.join("links");
        atomic_write_bytes(&p, b"graph").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"graph");
        let _ = std::fs::remove_dir_all(&d);
    }

    /// No stray temp files after a successful write — the rename consumed it.
    #[test]
    fn leaves_no_temp_litter() {
        let d = temp_dir("litter");
        atomic_write_bytes(&d.join("a.bin"), &[0u8; 128]).unwrap();
        let stray: Vec<_> = std::fs::read_dir(&d)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with(".kannaka-tmp-"))
            .collect();
        assert!(stray.is_empty(), "temp files left behind: {stray:?}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn missing_parent_directories_are_created() {
        let d = temp_dir("mkdirs");
        let p = d.join("a").join("b").join("c.json");
        atomic_write_bytes(&p, b"deep").unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"deep");
        let _ = std::fs::remove_dir_all(&d);
    }
}
