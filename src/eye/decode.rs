//! Video decoding via ffmpeg subprocess.
//!
//! Spawns ffmpeg to decode video → raw RGB frames at target resolution + FPS.
//! No C/C++ linking required.

use std::path::Path;
use std::process::{Command, Stdio};

use super::EyeError;

/// A single decoded RGB frame.
#[derive(Debug, Clone)]
pub struct FrameInfo {
    /// Raw RGB pixels (row-major, 3 bytes per pixel).
    pub rgb: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Frame index in the decoded sequence.
    pub index: usize,
}

impl FrameInfo {
    /// Get pixel at (x, y) as (R, G, B).
    pub fn pixel(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let idx = ((y * self.width + x) * 3) as usize;
        (self.rgb[idx], self.rgb[idx + 1], self.rgb[idx + 2])
    }

    /// Get luminance at (x, y) using BT.601 formula.
    pub fn luminance(&self, x: u32, y: u32) -> f32 {
        let (r, g, b) = self.pixel(x, y);
        0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32
    }

    /// Total number of pixels.
    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }
}

/// Collection of decoded frames with metadata.
#[derive(Debug, Clone)]
pub struct VideoFrames {
    pub frames: Vec<FrameInfo>,
    pub width: u32,
    pub height: u32,
    pub duration_secs: f32,
    pub source_fps: f32,
    pub analysis_fps: f32,
}

/// Check if ffmpeg is available.
fn check_ffmpeg() -> Result<(), EyeError> {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| EyeError::FfmpegNotFound)?;
    Ok(())
}

/// Probe video for resolution and duration using ffprobe.
fn probe_video(path: &Path) -> Result<(u32, u32, f32, f32), EyeError> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(path)
        .output()
        .map_err(|e| EyeError::Decode(format!("ffprobe failed: {}", e)))?;

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| EyeError::Decode(format!("ffprobe JSON parse error: {}", e)))?;

    // Find video stream
    let streams = json["streams"].as_array()
        .ok_or_else(|| EyeError::Decode("no streams in ffprobe output".into()))?;

    let video_stream = streams.iter()
        .find(|s| s["codec_type"].as_str() == Some("video"))
        .ok_or_else(|| EyeError::Decode("no video stream found".into()))?;

    let width = video_stream["width"].as_u64().unwrap_or(320) as u32;
    let height = video_stream["height"].as_u64().unwrap_or(240) as u32;

    // Parse FPS from r_frame_rate (e.g., "30000/1001")
    let fps_str = video_stream["r_frame_rate"].as_str().unwrap_or("30/1");
    let fps = if let Some((num, den)) = fps_str.split_once('/') {
        let n: f32 = num.parse().unwrap_or(30.0);
        let d: f32 = den.parse().unwrap_or(1.0);
        if d > 0.0 { n / d } else { 30.0 }
    } else {
        fps_str.parse().unwrap_or(30.0)
    };

    // Duration from format
    let duration = json["format"]["duration"].as_str()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.0);

    Ok((width, height, fps, duration))
}

/// Decode a video file to raw RGB frames using ffmpeg.
///
/// Downscales to `target_width` (height scales proportionally) and samples
/// at `target_fps` for efficiency.
pub fn decode_video(path: &Path, target_fps: f32, target_width: u32) -> Result<VideoFrames, EyeError> {
    check_ffmpeg()?;

    let (src_width, src_height, src_fps, duration) = probe_video(path)?;

    // Calculate target height preserving aspect ratio
    let aspect = src_height as f32 / src_width as f32;
    let target_height = ((target_width as f32 * aspect) as u32 / 2) * 2; // ensure even

    let output = Command::new("ffmpeg")
        .args([
            "-i", &path.to_string_lossy(),
            "-vf", &format!("fps={},scale={}:{}", target_fps, target_width, target_height),
            "-pix_fmt", "rgb24",
            "-f", "rawvideo",
            "-v", "quiet",
            "-",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| EyeError::Decode(format!("ffmpeg failed: {}", e)))?;

    if output.stdout.is_empty() {
        return Err(EyeError::EmptyVideo);
    }

    let frame_bytes = (target_width * target_height * 3) as usize;
    let num_frames = output.stdout.len() / frame_bytes;

    let mut frames = Vec::with_capacity(num_frames);
    for i in 0..num_frames {
        let start = i * frame_bytes;
        let end = start + frame_bytes;
        frames.push(FrameInfo {
            rgb: output.stdout[start..end].to_vec(),
            width: target_width,
            height: target_height,
            index: i,
        });
    }

    Ok(VideoFrames {
        frames,
        width: target_width,
        height: target_height,
        duration_secs: duration,
        source_fps: src_fps,
        analysis_fps: target_fps,
    })
}
