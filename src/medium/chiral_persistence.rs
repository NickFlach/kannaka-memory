//! Chiral HRM v2 persistence — save/load ChiralMedium to/from .hrm files.
//!
//! Format v2 wraps two hemispheres + callosum + scales into one file.
//! Backward compatible: detects v1 magic and loads as right-hemisphere-only.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use ndarray::{Array1, Array2};

use super::callosum::CorpusCallosum;
use super::chiral::ChiralMedium;
use super::fano::FanoPlane;
use super::hemisphere::Hemisphere;
use super::types::*;
use super::Medium;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

/// Pre-NCS WavefrontMeta (before modality field was added).
/// Used for backward-compatible deserialization of old .hrm files.
#[derive(Deserialize)]
struct WavefrontMetaLegacy {
    pub id: Uuid,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub hallucinated: bool,
    pub is_self_referential: bool,
}

impl From<WavefrontMetaLegacy> for WavefrontMeta {
    fn from(legacy: WavefrontMetaLegacy) -> Self {
        WavefrontMeta {
            id: legacy.id,
            content: legacy.content,
            tags: legacy.tags,
            created_at: legacy.created_at,
            hallucinated: legacy.hallucinated,
            is_self_referential: legacy.is_self_referential,
            sga_class: None,
            fano_group: None,
            category: None,
            modality: Modality::Unknown,
        }
    }
}

/// Verify the trailing 32-byte blake3 checksum on a .hrm file.
///
/// Both v1 and v2 save paths append `blake3(file[0..size-32])` as the final
/// 32 bytes before the atomic rename. Verifying on load catches data drift
/// that would otherwise crash deep inside the parser with a confusing error.
pub(crate) fn verify_blake3_trailing<P: AsRef<Path>>(path: P) -> Result<(), MediumError> {
    let path = path.as_ref();
    let size = std::fs::metadata(path)?.len();
    if size < 32 + 8 {
        // Too small to contain a header + checksum; let parse fail with its
        // own error rather than mis-reporting as checksum mismatch.
        return Ok(());
    }
    let mut f = File::open(path)?;
    let body_len = size - 32;
    let mut hasher = blake3::Hasher::new();
    // Stream body into the hasher
    let mut remaining = body_len;
    let mut buf = [0u8; 65536];
    while remaining > 0 {
        let want = remaining.min(buf.len() as u64) as usize;
        let n = std::io::Read::read(&mut f, &mut buf[..want])?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
        remaining -= n as u64;
    }
    let expected = hasher.finalize();
    let mut actual = [0u8; 32];
    std::io::Read::read_exact(&mut f, &mut actual)?;
    if expected.as_bytes() != &actual {
        return Err(MediumError::ChecksumMismatch);
    }
    Ok(())
}

impl ChiralMedium {
    /// Save the chiral medium to a .hrm v2 file.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), MediumError> {
        let path_ref = path.as_ref();
        let tmp_path = path_ref.with_extension("hrm.tmp");
        let file = File::create(&tmp_path)?;
        let mut w = BufWriter::new(file);

        // Magic bytes (v2)
        w.write_all(&HRM_MAGIC_V2)?;
        // Version
        w.write_all(&HRM_VERSION_CHIRAL.to_le_bytes())?;
        // Timestamp
        let timestamp = chrono::Utc::now().timestamp_millis();
        w.write_all(&timestamp.to_le_bytes())?;

        // Write left hemisphere
        Self::write_hemisphere(&mut w, &self.left)?;
        // Write right hemisphere
        Self::write_hemisphere(&mut w, &self.right)?;

        // Write callosum state (bincode)
        let callosum_bytes = bincode::serialize(&self.callosum)
            .map_err(|e| MediumError::Serialization(e))?;
        w.write_all(&(callosum_bytes.len() as u32).to_le_bytes())?;
        w.write_all(&callosum_bytes)?;

        // Write chiral scales (bincode)
        let scales_vec: Vec<(uuid::Uuid, ChiralScale)> =
            self.scales.iter().map(|(&k, &v)| (k, v)).collect();
        let scales_bytes = bincode::serialize(&scales_vec)
            .map_err(|e| MediumError::Serialization(e))?;
        w.write_all(&(scales_bytes.len() as u32).to_le_bytes())?;
        w.write_all(&scales_bytes)?;

        // Write ID mappings (bincode)
        let lr_vec: Vec<(uuid::Uuid, uuid::Uuid)> =
            self.left_to_right.iter().map(|(&k, &v)| (k, v)).collect();
        let lr_bytes = bincode::serialize(&lr_vec)
            .map_err(|e| MediumError::Serialization(e))?;
        w.write_all(&(lr_bytes.len() as u32).to_le_bytes())?;
        w.write_all(&lr_bytes)?;

        // Flush and compute checksum
        w.flush()?;
        drop(w);

        let mut file = File::open(&tmp_path)?;
        let mut hasher = blake3::Hasher::new();
        std::io::copy(&mut file, &mut hasher)?;
        let checksum = hasher.finalize();
        drop(file);

        let mut file = std::fs::OpenOptions::new().append(true).open(&tmp_path)?;
        file.write_all(checksum.as_bytes())?;
        file.flush()?;
        file.sync_all()?;
        drop(file);

        std::fs::rename(&tmp_path, path_ref)?;
        Ok(())
    }

    /// Load a chiral medium from a .hrm file.
    /// Auto-detects v1 vs v2 format:
    /// - v1: loads as Medium, then wraps with from_medium()
    /// - v2: loads native chiral format
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, MediumError> {
        let path_ref = path.as_ref();

        // Verify trailing blake3 checksum before parsing. Catches the kind of
        // mid-stream drift that previously surfaced as `failed to fill whole
        // buffer` somewhere deep inside `read_hemisphere`.
        verify_blake3_trailing(path_ref)?;

        let file = File::open(path_ref)?;
        let mut reader = BufReader::new(file);

        // Read magic bytes
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;

        if magic == HRM_MAGIC {
            // v1 format — load as Medium and convert
            drop(reader);
            let medium = Medium::load(path_ref)?;
            Ok(ChiralMedium::from_medium(&medium))
        } else if magic == HRM_MAGIC_V2 {
            // v2 format — load native chiral
            Self::load_v2(reader)
        } else {
            Err(MediumError::InvalidMagic)
        }
    }

    /// Load v2 chiral format (reader already past magic bytes).
    fn load_v2(mut reader: BufReader<File>) -> Result<Self, MediumError> {
        // Version
        let mut version_bytes = [0u8; 4];
        reader.read_exact(&mut version_bytes)?;
        let version = u32::from_le_bytes(version_bytes);
        if version != HRM_VERSION_CHIRAL {
            return Err(MediumError::UnsupportedVersion(version));
        }

        // Timestamp (skip)
        let mut ts_bytes = [0u8; 8];
        reader.read_exact(&mut ts_bytes)?;

        // Read hemispheres
        let left = Self::read_hemisphere(&mut reader, Hand::Left)?;
        let right = Self::read_hemisphere(&mut reader, Hand::Right)?;

        // Read callosum
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes)?;
        let callosum_len = u32::from_le_bytes(len_bytes) as usize;
        let mut callosum_bytes = vec![0u8; callosum_len];
        reader.read_exact(&mut callosum_bytes)?;
        let callosum: CorpusCallosum = bincode::deserialize(&callosum_bytes)
            .map_err(|e| MediumError::Serialization(e))?;

        // Read scales
        reader.read_exact(&mut len_bytes)?;
        let scales_len = u32::from_le_bytes(len_bytes) as usize;
        let mut scales_bytes = vec![0u8; scales_len];
        reader.read_exact(&mut scales_bytes)?;
        let scales_vec: Vec<(uuid::Uuid, ChiralScale)> = bincode::deserialize(&scales_bytes)
            .map_err(|e| MediumError::Serialization(e))?;
        let scales: HashMap<uuid::Uuid, ChiralScale> = scales_vec.into_iter().collect();

        // Read ID mappings
        reader.read_exact(&mut len_bytes)?;
        let lr_len = u32::from_le_bytes(len_bytes) as usize;
        let mut lr_bytes = vec![0u8; lr_len];
        reader.read_exact(&mut lr_bytes)?;
        let lr_vec: Vec<(uuid::Uuid, uuid::Uuid)> = bincode::deserialize(&lr_bytes)
            .map_err(|e| MediumError::Serialization(e))?;
        let left_to_right: HashMap<uuid::Uuid, uuid::Uuid> = lr_vec.iter().cloned().collect();
        let right_to_left: HashMap<uuid::Uuid, uuid::Uuid> =
            lr_vec.into_iter().map(|(l, r)| (r, l)).collect();

        // Skip checksum verification for now (same pattern as v1)

        Ok(ChiralMedium {
            left,
            right,
            callosum,
            fano: FanoPlane::new(),
            scales,
            left_to_right,
            right_to_left,
        })
    }

    /// Write a hemisphere to the output stream.
    fn write_hemisphere<W: Write>(w: &mut W, h: &Hemisphere) -> Result<(), MediumError> {
        // Hand
        let hand_byte: u8 = match h.hand {
            Hand::Left => 0,
            Hand::Right => 1,
        };
        w.write_all(&[hand_byte])?;

        // Dimensions
        w.write_all(&(h.dims as u32).to_le_bytes())?;

        // Wavefront count
        let n = h.count() as u32;
        w.write_all(&n.to_le_bytes())?;

        // Wavefronts tensor (row-major) — only active rows
        let active = n as usize;
        for i in 0..active {
            for j in 0..h.dims {
                w.write_all(&h.wavefronts[[i, j]].to_le_bytes())?;
            }
        }

        // Energy, frequency, phase — only active entries
        for i in 0..active {
            w.write_all(&h.energy[i].to_le_bytes())?;
        }
        for i in 0..active {
            w.write_all(&h.frequency[i].to_le_bytes())?;
        }
        for i in 0..active {
            w.write_all(&h.phase[i].to_le_bytes())?;
        }

        // Timestamps — write exactly `active` entries.
        //
        // Earlier revisions wrote the full Vec, which crashed the loader if
        // `h.timestamps.len()` ever drifted from `h.count()`. Pad with 0 if
        // the Vec is short so the writer stays self-consistent under any
        // upstream desync.
        for i in 0..active {
            let ts = h.timestamps.get(i).copied().unwrap_or(0);
            w.write_all(&ts.to_le_bytes())?;
        }

        // Metadata (bincode)
        let meta_bytes = bincode::serialize(&h.metadata)
            .map_err(|e| MediumError::Serialization(e))?;
        w.write_all(&(meta_bytes.len() as u32).to_le_bytes())?;
        w.write_all(&meta_bytes)?;

        Ok(())
    }

    /// Read a hemisphere from the input stream.
    fn read_hemisphere<R: Read>(r: &mut R, _expected_hand: Hand) -> Result<Hemisphere, MediumError> {
        // Hand
        let mut hand_byte = [0u8; 1];
        r.read_exact(&mut hand_byte)?;
        let hand = match hand_byte[0] {
            0 => Hand::Left,
            1 => Hand::Right,
            _ => return Err(MediumError::InvalidMagic), // Reuse error for bad hand byte
        };

        // Dimensions
        let mut dims_bytes = [0u8; 4];
        r.read_exact(&mut dims_bytes)?;
        let dims = u32::from_le_bytes(dims_bytes) as usize;

        // Wavefront count
        let mut n_bytes = [0u8; 4];
        r.read_exact(&mut n_bytes)?;
        let n = u32::from_le_bytes(n_bytes) as usize;

        // Wavefronts tensor
        let mut wf_data = vec![0.0f32; n * dims];
        for val in &mut wf_data {
            let mut bytes = [0u8; 4];
            r.read_exact(&mut bytes)?;
            *val = f32::from_le_bytes(bytes);
        }
        let wavefronts = if n > 0 {
            Array2::from_shape_vec((n, dims), wf_data).unwrap()
        } else {
            Array2::zeros((0, dims))
        };

        // Energy, frequency, phase
        let mut energy_data = vec![0.0f32; n];
        let mut freq_data = vec![0.0f32; n];
        let mut phase_data = vec![0.0f32; n];
        for val in &mut energy_data {
            let mut bytes = [0u8; 4];
            r.read_exact(&mut bytes)?;
            *val = f32::from_le_bytes(bytes);
        }
        for val in &mut freq_data {
            let mut bytes = [0u8; 4];
            r.read_exact(&mut bytes)?;
            *val = f32::from_le_bytes(bytes);
        }
        for val in &mut phase_data {
            let mut bytes = [0u8; 4];
            r.read_exact(&mut bytes)?;
            *val = f32::from_le_bytes(bytes);
        }

        // Timestamps
        let mut timestamps = vec![0i64; n];
        for ts in &mut timestamps {
            let mut bytes = [0u8; 8];
            r.read_exact(&mut bytes)?;
            *ts = i64::from_le_bytes(bytes);
        }

        // Metadata — try current format first, fall back to pre-NCS format without modality
        let mut len_bytes = [0u8; 4];
        r.read_exact(&mut len_bytes)?;
        let meta_len = u32::from_le_bytes(len_bytes) as usize;
        // Sanity cap: 256 MiB. Anything larger means we walked off the rails
        // (the count/timestamps section above was misaligned) — report it
        // before we try to allocate gigabytes and crash on read_exact.
        const MAX_META_BYTES: usize = 256 * 1024 * 1024;
        if meta_len > MAX_META_BYTES {
            return Err(MediumError::CorruptHrm(format!(
                "implausible meta_len={} (max {}) — file layout desync at hemisphere {:?}",
                meta_len, MAX_META_BYTES, hand
            )));
        }
        let mut meta_bytes = vec![0u8; meta_len];
        r.read_exact(&mut meta_bytes)?;
        let metadata: Vec<WavefrontMeta> = match bincode::deserialize(&meta_bytes) {
            Ok(m) => m,
            Err(_) => {
                // Pre-NCS format: deserialize without modality field, then add default
                let legacy: Vec<WavefrontMetaLegacy> = bincode::deserialize(&meta_bytes)
                    .map_err(|e| MediumError::Serialization(e))?;
                legacy.into_iter().map(|l| l.into()).collect()
            }
        };

        // Build ID index
        let mut id_to_index = HashMap::new();
        for (i, meta) in metadata.iter().enumerate() {
            id_to_index.insert(meta.id, i);
        }

        let len = metadata.len();
        Ok(Hemisphere {
            hand,
            wavefronts,
            energy: Array1::from_vec(energy_data),
            frequency: Array1::from_vec(freq_data),
            phase: Array1::from_vec(phase_data),
            timestamps,
            metadata,
            id_to_index,
            dims,
            len,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codebook::Codebook;
    use crate::encoding::{EncodingPipeline, SimpleHashEncoder};
    use std::path::PathBuf;

    fn test_pipeline() -> EncodingPipeline {
        let encoder = Box::new(SimpleHashEncoder::new(384, 42));
        let codebook = Codebook::new(384, WAVEFRONT_DIM, 42);
        EncodingPipeline::new(encoder, codebook)
    }

    #[test]
    fn load_rejects_tampered_file() {
        // Save a clean ChiralMedium, then flip one byte deep in the wavefront
        // section. Load must reject it via checksum verification rather than
        // crash mid-parse in `read_hemisphere`.
        let mut cm = ChiralMedium::new();
        let pipeline = test_pipeline();
        cm.store("checksum guard 1", 0.9, &pipeline).unwrap();
        cm.store("checksum guard 2", 0.5, &pipeline).unwrap();

        let dir = std::env::temp_dir();
        let path = dir.join("test_chiral_checksum_tampered.hrm");
        cm.save(&path).unwrap();

        // Sanity: clean file loads fine
        ChiralMedium::load(&path).unwrap();

        // Flip a byte in the middle of the file (not in the trailing checksum)
        let mut buf = std::fs::read(&path).unwrap();
        let flip_at = buf.len() / 2;
        buf[flip_at] ^= 0xff;
        std::fs::write(&path, &buf).unwrap();

        match ChiralMedium::load(&path) {
            Err(MediumError::ChecksumMismatch) => {}
            other => panic!("expected ChecksumMismatch, got {:?}", other),
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_load_roundtrip() {
        let mut cm = ChiralMedium::new();
        let pipeline = test_pipeline();

        cm.store("roundtrip test 1", 0.9, &pipeline).unwrap();
        cm.store("roundtrip test 2", 0.7, &pipeline).unwrap();
        cm.store("roundtrip test 3", 0.5, &pipeline).unwrap();

        let dir = std::env::temp_dir();
        let path = dir.join("test_chiral_roundtrip.hrm");

        // Save
        cm.save(&path).unwrap();

        // Load
        let loaded = ChiralMedium::load(&path).unwrap();

        // Verify counts
        assert_eq!(loaded.left.count(), cm.left.count());
        assert_eq!(loaded.right.count(), cm.right.count());
        assert_eq!(loaded.scales.len(), cm.scales.len());
        assert_eq!(loaded.left_to_right.len(), cm.left_to_right.len());

        // Verify energies match
        for i in 0..loaded.right.count() {
            assert!((loaded.right.energy[i] - cm.right.energy[i]).abs() < 0.001,
                "Right energy mismatch at {}", i);
        }
        for i in 0..loaded.left.count() {
            assert!((loaded.left.energy[i] - cm.left.energy[i]).abs() < 0.001,
                "Left energy mismatch at {}", i);
        }

        // Verify recall still works
        let results = loaded.recall("roundtrip test", 5, &pipeline).unwrap();
        assert!(!results.is_empty(), "Should recall stored memories after reload");

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn v1_backward_compatibility() {
        // Create a v1 Medium and save it
        let mut medium = Medium::new();
        let pipeline = test_pipeline();
        medium.store("v1 memory", 0.8, &pipeline).unwrap();

        let dir = std::env::temp_dir();
        let path = dir.join("test_v1_compat.hrm");
        medium.save(&path).unwrap();

        // Load as ChiralMedium — should auto-detect v1 and convert
        let cm = ChiralMedium::load(&path).unwrap();

        assert_eq!(cm.right.count(), 1, "v1 memory should be in right hemisphere");
        assert_eq!(cm.left.count(), 0, "Left should be empty after v1 migration");
        assert!(cm.scales.contains_key(&cm.right.metadata[0].id));

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_empty_chiral() {
        let cm = ChiralMedium::new();
        let dir = std::env::temp_dir();
        let path = dir.join("test_empty_chiral.hrm");

        cm.save(&path).unwrap();
        let loaded = ChiralMedium::load(&path).unwrap();

        assert_eq!(loaded.left.count(), 0);
        assert_eq!(loaded.right.count(), 0);

        let _ = std::fs::remove_file(&path);
    }
}
