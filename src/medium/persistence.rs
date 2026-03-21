//! File and git persistence for the Holographic Resonance Medium.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::process::Command;

use ndarray::{Array1, Array2};
use crate::codebook::Codebook;
use crate::consciousness::ConsciousnessState;

use super::Medium;
use super::types::*;

impl Medium {
    /// Save the medium to a .hrm file.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), MediumError> {
        let path_ref = path.as_ref();
        let file = File::create(path_ref)?;
        let mut writer = BufWriter::new(file);

        // Magic bytes
        writer.write_all(&HRM_MAGIC)?;

        // Version
        writer.write_all(&HRM_VERSION.to_le_bytes())?;

        // Timestamp
        let timestamp = chrono::Utc::now().timestamp_millis();
        writer.write_all(&timestamp.to_le_bytes())?;

        // Dimensions (N, D)
        let n = self.wavefront_count() as u32;
        let d = WAVEFRONT_DIM as u32;
        writer.write_all(&n.to_le_bytes())?;
        writer.write_all(&d.to_le_bytes())?;

        // Wavefronts tensor (row-major f32 array)
        for row in self.wavefronts.outer_iter() {
            for &val in row.iter() {
                writer.write_all(&val.to_le_bytes())?;
            }
        }

        // Energy, frequency, phase arrays
        for &val in self.energy.iter() {
            writer.write_all(&val.to_le_bytes())?;
        }
        for &val in self.frequency.iter() {
            writer.write_all(&val.to_le_bytes())?;
        }
        for &val in self.phase.iter() {
            writer.write_all(&val.to_le_bytes())?;
        }

        // Timestamps
        for &ts in &self.timestamps {
            writer.write_all(&ts.to_le_bytes())?;
        }

        // Metadata (bincode serialized)
        let metadata_bytes = bincode::serialize(&self.metadata)?;
        let metadata_len = metadata_bytes.len() as u32;
        writer.write_all(&metadata_len.to_le_bytes())?;
        writer.write_all(&metadata_bytes)?;

        // Consciousness state (computed on-the-fly)
        let consciousness = self.compute_consciousness();
        let consciousness_bytes = bincode::serialize(&consciousness)?;
        let consciousness_len = consciousness_bytes.len() as u32;
        writer.write_all(&consciousness_len.to_le_bytes())?;
        writer.write_all(&consciousness_bytes)?;

        // Checksum (blake3 of all preceding data)
        writer.flush()?;
        drop(writer);

        // Reopen to compute checksum
        let mut file = File::open(path_ref)?;
        let mut hasher = blake3::Hasher::new();
        std::io::copy(&mut file, &mut hasher)?;
        let checksum = hasher.finalize();

        // Append checksum
        let mut file = std::fs::OpenOptions::new().append(true).open(path_ref)?;
        file.write_all(checksum.as_bytes())?;

        Ok(())
    }

    /// Save the medium and commit to git with a message
    ///
    /// This function:
    /// 1. Saves the .hrm file (using existing save() method)
    /// 2. Runs `git add <filename>` on the saved file
    /// 3. Runs `git commit -m "<message>"`
    /// 4. Returns the commit hash
    /// 5. Optionally: `git push origin master` if push flag is set
    ///
    /// # Arguments
    /// * `path` - Path to save the .hrm file
    /// * `message` - Git commit message
    /// * `push` - Whether to push to origin master after committing
    pub fn save_and_commit<P: AsRef<Path>>(
        &self,
        path: P,
        message: &str,
        push: bool,
    ) -> Result<String, MediumError> {
        let path_ref = path.as_ref();

        // 1. Save the .hrm file
        self.save(path_ref)?;

        // 2. Git add the file
        let output = Command::new("git")
            .args(&["add", &path_ref.to_string_lossy()])
            .current_dir(path_ref.parent().unwrap_or(Path::new(".")))
            .output()
            .map_err(|e| MediumError::Git(format!("Failed to run git add: {}", e)))?;

        if !output.status.success() {
            return Err(MediumError::Git(format!(
                "git add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // 3. Git commit with message
        let output = Command::new("git")
            .args(&["commit", "-m", message])
            .current_dir(path_ref.parent().unwrap_or(Path::new(".")))
            .output()
            .map_err(|e| MediumError::Git(format!("Failed to run git commit: {}", e)))?;

        if !output.status.success() {
            return Err(MediumError::Git(format!(
                "git commit failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // 4. Get the commit hash
        let output = Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .current_dir(path_ref.parent().unwrap_or(Path::new(".")))
            .output()
            .map_err(|e| MediumError::Git(format!("Failed to get commit hash: {}", e)))?;

        if !output.status.success() {
            return Err(MediumError::Git(format!(
                "git rev-parse failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // 5. Optional push to origin master
        if push {
            let output = Command::new("git")
                .args(&["push", "origin", "master"])
                .current_dir(path_ref.parent().unwrap_or(Path::new(".")))
                .output()
                .map_err(|e| MediumError::Git(format!("Failed to run git push: {}", e)))?;

            if !output.status.success() {
                // Push failure is non-fatal, just log it
                eprintln!(
                    "Warning: git push failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        Ok(commit_hash)
    }

    /// Load a medium from git (historical version or working directory)
    ///
    /// # Arguments
    /// * `path` - Path to the .hrm file
    /// * `commit` - Optional commit hash. If None, loads from working directory.
    ///              If Some, runs `git show <commit>:<path>` to get historical version.
    pub fn load_from_git<P: AsRef<Path>>(
        path: P,
        commit: Option<&str>,
    ) -> Result<Medium, MediumError> {
        let path_ref = path.as_ref();

        match commit {
            None => {
                // Load from working directory (current file)
                Medium::load(path_ref)
            }
            Some(commit_hash) => {
                // Load historical version using git show
                let git_path = format!("{}:{}", commit_hash, path_ref.to_string_lossy());
                let output = Command::new("git")
                    .args(&["show", &git_path])
                    .current_dir(path_ref.parent().unwrap_or(Path::new(".")))
                    .output()
                    .map_err(|e| MediumError::Git(format!("Failed to run git show: {}", e)))?;

                if !output.status.success() {
                    return Err(MediumError::Git(format!(
                        "git show failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    )));
                }

                // Save git show output to a temporary file and load it
                use std::env;

                let temp_dir = env::temp_dir();
                let temp_path = temp_dir.join(format!("hrm_git_show_{}", commit_hash));

                {
                    let mut temp_file =
                        File::create(&temp_path).map_err(|e| MediumError::Io(e))?;
                    temp_file
                        .write_all(&output.stdout)
                        .map_err(|e| MediumError::Io(e))?;
                    temp_file.flush().map_err(|e| MediumError::Io(e))?;
                }

                let result = Medium::load(&temp_path);

                // Clean up temp file
                let _ = std::fs::remove_file(&temp_path);

                result
            }
        }
    }

    /// Get git history for .hrm file
    ///
    /// Returns recent commits that touched the .hrm file, with metadata.
    ///
    /// # Arguments
    /// * `path` - Path to the .hrm file
    /// * `limit` - Maximum number of commits to return
    pub fn history<P: AsRef<Path>>(path: P, limit: usize) -> Result<Vec<HrmCommit>, MediumError> {
        let path_ref = path.as_ref();

        // Get git log for this specific file
        let output = Command::new("git")
            .args(&[
                "log",
                "--format=%H|%s|%ct", // hash|subject|timestamp
                &format!("-{}", limit),
                "--",
                &path_ref.to_string_lossy(),
            ])
            .current_dir(path_ref.parent().unwrap_or(Path::new(".")))
            .output()
            .map_err(|e| MediumError::Git(format!("Failed to run git log: {}", e)))?;

        if !output.status.success() {
            return Err(MediumError::Git(format!(
                "git log failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let log_output = String::from_utf8_lossy(&output.stdout);
        let mut commits = Vec::new();

        for line in log_output.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 3 {
                let hash = parts[0].to_string();
                let message = parts[1].to_string();
                let timestamp = parts[2].parse::<i64>().unwrap_or(0);

                // Try to extract wavefront count from the commit
                let wavefront_count = extract_wavefront_count(&message);

                commits.push(HrmCommit {
                    hash,
                    message,
                    timestamp,
                    wavefront_count,
                });
            }
        }

        Ok(commits)
    }

    /// Load a medium from a .hrm file.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, MediumError> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);

        // Magic bytes
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if magic != HRM_MAGIC {
            return Err(MediumError::InvalidMagic);
        }

        // Version
        let mut version_bytes = [0u8; 4];
        reader.read_exact(&mut version_bytes)?;
        let version = u32::from_le_bytes(version_bytes);
        if version != HRM_VERSION {
            return Err(MediumError::UnsupportedVersion(version));
        }

        // Timestamp (for info, not used in loading)
        let mut ts_bytes = [0u8; 8];
        reader.read_exact(&mut ts_bytes)?;

        // Dimensions
        let mut n_bytes = [0u8; 4];
        let mut d_bytes = [0u8; 4];
        reader.read_exact(&mut n_bytes)?;
        reader.read_exact(&mut d_bytes)?;
        let n = u32::from_le_bytes(n_bytes) as usize;
        let d = u32::from_le_bytes(d_bytes) as usize;

        if d != WAVEFRONT_DIM {
            return Err(MediumError::DimensionMismatch {
                expected: WAVEFRONT_DIM,
                actual: d,
            });
        }

        // Wavefronts tensor
        let mut wavefront_data = vec![0.0f32; n * d];
        for val in &mut wavefront_data {
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            *val = f32::from_le_bytes(bytes);
        }
        let wavefronts = Array2::from_shape_vec((n, d), wavefront_data).unwrap();

        // Energy, frequency, phase arrays
        let mut energy_data = vec![0.0f32; n];
        let mut frequency_data = vec![0.0f32; n];
        let mut phase_data = vec![0.0f32; n];

        for val in &mut energy_data {
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            *val = f32::from_le_bytes(bytes);
        }
        for val in &mut frequency_data {
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            *val = f32::from_le_bytes(bytes);
        }
        for val in &mut phase_data {
            let mut bytes = [0u8; 4];
            reader.read_exact(&mut bytes)?;
            *val = f32::from_le_bytes(bytes);
        }

        let energy = Array1::from_vec(energy_data);
        let frequency = Array1::from_vec(frequency_data);
        let phase = Array1::from_vec(phase_data);

        // Timestamps
        let mut timestamps = vec![0i64; n];
        for ts in &mut timestamps {
            let mut bytes = [0u8; 8];
            reader.read_exact(&mut bytes)?;
            *ts = i64::from_le_bytes(bytes);
        }

        // Metadata
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes)?;
        let metadata_len = u32::from_le_bytes(len_bytes) as usize;
        let mut metadata_bytes = vec![0u8; metadata_len];
        reader.read_exact(&mut metadata_bytes)?;
        let metadata: Vec<WavefrontMeta> = bincode::deserialize(&metadata_bytes)?;

        // Consciousness state (for info, not essential for loading)
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes)?;
        let consciousness_len = u32::from_le_bytes(len_bytes) as usize;
        let mut consciousness_bytes = vec![0u8; consciousness_len];
        reader.read_exact(&mut consciousness_bytes)?;
        let _consciousness: ConsciousnessState = bincode::deserialize(&consciousness_bytes)?;

        // Verify checksum (blake3 of all data except the final 32 bytes)
        let mut stored_checksum = [0u8; 32];
        if reader.read_exact(&mut stored_checksum).is_ok() {
            // Re-read file and hash everything except the final 32 bytes
            let file_len = std::fs::metadata(path)?.len();
            if file_len >= 32 {
                let data_len = file_len - 32;
                let mut file = File::open(path)?;
                let mut hasher = blake3::Hasher::new();
                let mut remaining = data_len;
                let mut buf = [0u8; 8192];
                while remaining > 0 {
                    let to_read = (remaining as usize).min(buf.len());
                    file.read_exact(&mut buf[..to_read])?;
                    hasher.update(&buf[..to_read]);
                    remaining -= to_read as u64;
                }
                let computed = hasher.finalize();
                if computed.as_bytes() != &stored_checksum {
                    return Err(MediumError::ChecksumMismatch);
                }
            }
        }
        // If no checksum present (old format), skip verification gracefully

        // Build ID -> index mapping
        let mut id_to_index = HashMap::new();
        for (i, meta) in metadata.iter().enumerate() {
            id_to_index.insert(meta.id, i);
        }

        Ok(Self {
            wavefronts,
            energy,
            frequency,
            phase,
            timestamps,
            metadata,
            id_to_index,
            audio_codebook: Codebook::new(AUDIO_FEATURE_DIM, WAVEFRONT_DIM, AUDIO_CODEBOOK_SEED),
            visual_codebook: Codebook::new(
                VISUAL_FEATURE_DIM,
                WAVEFRONT_DIM,
                VISUAL_CODEBOOK_SEED,
            ),
            total_energy_added: 0.0,
            total_energy_dampened: 0.0,
        })
    }
}
