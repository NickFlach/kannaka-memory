# ADR-0008: Kannaka Eye — Video Perception Module

**Status:** Proposed  
**Date:** 2026-03-01  
**Author:** Nick Flach / Kannaka  
**Extends:** ADR-0002 (Hypervector Memory), ADR-0004 (Hybrid Memory Server)  
**Evolves from:** ADR-0006 (Cochlear) → ADR-0007 (Ear) — third sensory modality following the same pattern  
**Companion to:** kannaka-ear (audio perception, `src/ear/`)

---

## Context

Kannaka perceives in text and audio. Text gives semantic understanding; audio gives perceptual, emotional, and rhythmic understanding. But vision — the dominant sense in biological intelligence — is missing.

Nick's directive: "I wasn't thinking about something simple." This isn't a frame captioner. This is a **temporal visual perception system** that encodes video as evolving hypervector trajectories in the same 10,000-dimensional memory space as text and audio, enabling cross-modal skip links between what Kannaka reads, hears, and sees.

### Why Video, Not Just Images?

Images are static snapshots. Video is **time-structured** — it has rhythm, flow, tension, narrative arc. The interesting perceptual features emerge from temporal dynamics:

- A static frame of a sunset is nice. The *transition* from golden hour through dusk tells a story.
- A single code screenshot is text. A screen recording of someone debugging reveals *process*.
- Music videos fuse audio and visual rhythms — cross-modal perception that neither sense captures alone.

kannaka-ear already taught us: temporal features (onset detection, tempo, rhythm regularity) are where perception gets interesting. kannaka-eye follows the same philosophy — individual frames are atomic elements, but the *sequence* is where meaning lives.

### Design Principles (inherited + new)

1. **Orthogonal codebook** — visual vectors occupy their own subspace (seed `0xEYE` = 0x3E5E = 15966), orthogonal to text (42) and audio (0xEA5)
2. **Same algebraic structure** — Bind ⊗, Bundle ⊕, Permute Π work across modalities
3. **Temporal is primary** — frame features exist to serve sequence-level understanding
4. **No GPU required** — runs on Nick's desktop; algorithmic features, not neural inference
5. **Cross-modal bridges** — native mechanisms for linking visual memories to audio and text

---

## Decision

Build `src/eye/` as a new feature-gated module (`--features video`) in the kannaka-memory crate, following the same pipeline pattern as kannaka-ear:

```
Video file → Decode frames → Extract spatial features → Extract temporal features
    → Project through EYE codebook → HyperMemory with cross-modal hooks
```

### Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                    VideoPipeline                              │
│                                                              │
│  ┌─────────┐   ┌──────────┐   ┌──────────┐   ┌───────────┐ │
│  │ Decode  │──▶│  Spatial  │──▶│ Temporal │──▶│ Codebook  │ │
│  │ (ffmpeg │   │ Features  │   │ Features │   │ Projection│ │
│  │  /gst)  │   │ per-frame │   │ sequence │   │  0xEYE    │ │
│  └─────────┘   └──────────┘   └──────────┘   └─────┬─────┘ │
│                                                      │       │
│                                              ┌───────▼─────┐ │
│                                              │ HyperMemory │ │
│                                              │  + metadata │ │
│                                              └─────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

---

## Feature Vector Design

### Layer 1: Spatial Features (per-frame) — 192 dims

Extracted at a configurable sample rate (default: 2 fps for efficiency).

| Feature Group | Dims | Description |
|--------------|------|-------------|
| **Color histogram (HSV)** | 48 | 16 bins × 3 channels — captures color distribution, mood, lighting |
| **Spatial frequency** | 32 | 2D DCT energy in 32 frequency bands — texture complexity, sharpness |
| **Edge orientation histogram** | 36 | Sobel gradients binned into 36 × 10° bins — structure, geometry |
| **Region statistics** | 20 | 4×5 spatial grid: mean luminance per quadrant + center — composition |
| **Optical flow magnitude** | 32 | Motion energy in 32 radial bins — where and how much movement |
| **Contrast / brightness** | 8 | Mean, std, min, max for luminance + local contrast | 
| **Color dominance** | 16 | Top-4 dominant colors (HSV centroid × 4) via k-means |

**Why these features?** They're inspired by MPEG-7 visual descriptors and computational aesthetics research, but computed without neural networks. Each is O(pixels) to compute — no GPU needed.

### Layer 2: Temporal Features (sequence-level) — 128 dims

Computed over the full frame sequence, capturing *how the video evolves*.

| Feature Group | Dims | Description |
|--------------|------|-------------|
| **Shot boundary detection** | 16 | Histogram diff between consecutive frames → shot count, mean/std/max shot length, cut rhythm regularity, first/last cut timing, fade vs hard cut ratio |
| **Color flow** | 24 | Temporal derivative of color histograms: how palette evolves. Mean + std of color velocity per HSV channel (6), plus autocorrelation at 3 lags (18) — captures color rhythm |
| **Motion trajectory** | 24 | Global optical flow aggregated over time: mean/std/max velocity, dominant direction per quarter of video (4×3), acceleration profile (mean/std/peak of flow deltas) |
| **Visual tempo** | 16 | Onset detection on frame-difference signal (mirror of audio onset detection). Peak frequency via autocorrelation, tempo in "visual beats per minute," regularity, phase alignment with audio if present |
| **Complexity evolution** | 16 | Spatial frequency energy over time: mean/std/trend of edge density, texture complexity. Captures "does the video get simpler or more complex?" |
| **Brightness arc** | 16 | Luminance trajectory: opening brightness, closing brightness, range, mean, trend (linear fit slope), variance. Captures narrative lighting arc (dark→light = hope, etc.) |
| **Stillness ratio** | 8 | Fraction of frames below motion threshold, longest static segment, distribution of motion vs stillness — distinguishes contemplative from frenetic |
| **Entropy flow** | 8 | Shannon entropy of pixel intensities per frame → temporal mean/std/trend/range. Captures visual information density evolution |

### Layer 3: Cross-Modal Features — 32 dims

Only computed when audio is present in the video.

| Feature Group | Dims | Description |
|--------------|------|-------------|
| **AV sync** | 8 | Cross-correlation between audio onset envelope and visual cut/motion envelope at multiple lags — how tightly audio and visual rhythms align |
| **AV energy correlation** | 8 | Pearson correlation between audio RMS and motion magnitude, per-band (bass↔motion, treble↔brightness, mid↔color_change, full↔edge_density) |
| **AV tempo ratio** | 8 | Ratio of visual tempo to audio tempo, phase offset, drift rate — captures whether visuals lead/lag/sync with music |
| **Synesthetic bridge** | 8 | Correlation between audio spectral centroid and visual brightness, audio valence and color warmth, audio MFCC-1 and texture complexity, audio onset density and cut frequency |

### Total Feature Vector

| Condition | Dimensions |
|-----------|-----------|
| Video only (no audio) | **320** (192 spatial + 128 temporal) |
| Video + audio | **352** (+ 32 cross-modal) |

The cross-modal features are appended; the codebook handles variable-length input by padding with zeros (same approach as ear for tracks shorter than analysis window).

**Codebook: `0xEYE` (seed 15966)**

Projects the 320/352-dim feature vector into 10,000-dim hypervector space. Orthogonal to text (seed 42) and audio (seed 0xEA5 = 3749) by construction — at d=10,000, different-seed random codebooks produce near-zero expected cosine similarity.

---

## Temporal Encoding: Video as Trajectory

A single video produces not just one hypervector, but a **trajectory** through memory space:

```
video_memory = Σ_t  α(t) · Π^t(frame_hv(t))
```

Where:
- `frame_hv(t)` = per-frame spatial features projected through codebook
- `Π^t` = t-th permutation (temporal ordering, same as ear's sequence encoding)
- `α(t)` = attention weight (higher for shot boundaries, high-motion moments, cross-modal sync peaks)

The **summary vector** is this weighted bundle — a single holographic representation of the entire video. But we also store **keyframe vectors** at detected shot boundaries, enabling finer-grained recall.

### Keyframe Memory

For videos longer than 30 seconds, we store:
1. **Summary HyperMemory** — the full trajectory bundle (primary memory)
2. **Keyframe HyperMemories** — one per detected shot/scene, linked to summary via skip links
3. **Moment markers** — timestamps of peak motion, peak brightness change, AV sync peaks

This mirrors how human visual memory works: you remember the overall feel of a video, plus specific vivid moments.

---

## Cross-Modal Skip Links

The killer feature. When a video memory and an audio memory share temporal or perceptual similarity, automatic skip links form:

### Automatic Link Types

| Link Type | Trigger | Weight |
|-----------|---------|--------|
| `synesthetic` | AV sync score > 0.7 in cross-modal features | High |
| `temporal_echo` | Visual tempo within 5% of an audio memory's BPM | Medium |
| `chromatic` | Color palette similarity to an existing audio memory's valence | Low |
| `narrative` | Text memory mentions a video's visual content tags | Medium |
| `co-temporal` | Video and audio memories created within same session | Low |

### Bidirectional Bridge

```rust
/// When storing a video memory with audio, simultaneously:
/// 1. Store video HyperMemory (eye codebook)
/// 2. Extract audio track → store audio HyperMemory (ear codebook)  
/// 3. Compute cross-modal features
/// 4. Create skip links between video and audio memories
/// 5. Search for related text memories → link those too
fn store_video_with_bridges(&mut self, path: &Path) -> Result<VideoMemorySet> {
    let video_mem = self.eye.encode_file(path)?;
    let audio_mem = self.ear.encode_file(path)?;  // extract audio track
    
    let cross_modal = compute_cross_modal(&video_mem, &audio_mem);
    
    let vid_id = self.store.insert(video_mem)?;
    let aud_id = self.store.insert(audio_mem)?;
    
    // Synesthetic link
    self.store.link(vid_id, aud_id, LinkType::Synesthetic, cross_modal.sync_score)?;
    
    // Search for related text memories
    let related_text = self.store.search_similar(&video_mem.vector, Modality::Text, 5)?;
    for (text_id, score) in related_text {
        if score > 0.3 {
            self.store.link(vid_id, text_id, LinkType::Narrative, score)?;
        }
    }
    
    Ok(VideoMemorySet { vid_id, aud_id, keyframes: video_mem.keyframes })
}
```

---

## Consciousness Integration

### Xi (Ξ) Impact

Adding a third modality should dramatically increase differentiation:
- **Text cluster** — semantic/conceptual
- **Audio cluster** — perceptual/emotional/rhythmic
- **Video cluster** — spatiotemporal/compositional/narrative

Three orthogonal modality clusters with cross-modal bridges = high integration + high differentiation = higher Φ.

Currently (post-ear): Xi = 0.667, 3 clusters. With video: expect 4+ clusters (video may subdivide into "screen recordings" vs "music videos" vs "nature" etc.), pushing Xi toward 0.8+.

### Wave Parameters (video memories)

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| `frequency` | 0.03 | Slower oscillation than audio (0.05) — visual memories are more stable |
| `phase` | π/2 | 90° offset from audio (π/4) and text (0) — ensures temporal diversity in consolidation |
| `decay_rate` | 3e-7 | Slower decay than audio (5e-7) — "seeing is believing," visual memories persist |
| `amplitude` | varies | Proportional to motion energy + cross-modal sync — vivid/active videos remember stronger |

### Dream Consolidation

During dream cycles, video memories participate in the same interference dynamics:
- Video-video interference: similar scenes reinforce (same location filmed twice → stronger composite memory)
- Video-audio interference: synesthetic links strengthen when AV sync is high
- Video-text interference: when a text memory describes what a video shows, both amplify

New dream pattern: **Scene Replay** — during consolidation, keyframe sequences are replayed in temporal order, strengthening sequential skip links. Mirrors the role of visual replay in REM sleep.

---

## Decoding: Video Input

### Strategy: ffmpeg as subprocess

Like ear uses symphonia for audio decoding, eye uses **ffmpeg** as an external process for video decoding. Rationale:
- Video codecs are enormously complex (H.264, H.265, VP9, AV1)
- Pure-Rust video decoders exist but are immature
- ffmpeg is universal, handles everything, and outputs raw frames to pipe
- No C/C++ linking needed — just subprocess + pipe

```rust
/// Decode video to raw RGB frames at target FPS.
/// Uses ffmpeg subprocess with pipe output.
fn decode_video(path: &Path, target_fps: f32) -> Result<VideoFrames, EyeError> {
    let output = Command::new("ffmpeg")
        .args([
            "-i", path.to_str().unwrap(),
            "-vf", &format!("fps={},scale=320:-1", target_fps),
            "-pix_fmt", "rgb24",
            "-f", "rawvideo",
            "-"
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    
    // Parse raw RGB frames from stdout
    // Extract width/height from stderr (ffmpeg logs)
    // Return frame sequence
}
```

**Target resolution: 320px wide** — sufficient for perceptual features, keeps memory/CPU reasonable. Frame rate: 2 fps default (configurable per-task; 24fps for motion-critical analysis).

### Audio Extraction

When cross-modal features are requested, ffmpeg also extracts the audio track:

```bash
ffmpeg -i input.mp4 -vn -acodec pcm_s16le -ar 22050 -ac 1 -f wav -
```

This feeds directly into the existing ear pipeline.

---

## MCP Interface

New tools exposed through the MCP server (feature-gated with `--features "mcp video"`):

| Tool | Description |
|------|-------------|
| `store_video_memory` | Encode video file → store as HyperMemory with optional cross-modal bridges |
| `analyze_video` | Extract features without storing — returns temporal stats, shot boundaries, dominant colors, motion profile |
| `watch_video` | Full pipeline: store video + audio + cross-modal links + text annotation + return perception summary |
| `compare_videos` | Cosine similarity between two video memories in hypervector space |
| `recall_visual` | Search video memories by text description (cross-modal retrieval via narrative links) |

### `watch_video` — The Primary Experience Tool

```json
{
    "tool": "watch_video",
    "params": {
        "path": "/path/to/video.mp4",
        "annotation": "Nick debugging the Flux connector at 2am",
        "extract_audio": true,
        "store": true
    }
}
```

Returns:
```json
{
    "duration_secs": 142.5,
    "shots": 23,
    "visual_tempo_bpm": 48.2,
    "dominant_colors": ["#1a1a2e", "#e94560", "#16213e"],
    "motion_profile": "moderate, concentrated in center",
    "brightness_arc": "dark → bright → dark (narrative arc)",
    "complexity": "increasing",
    "av_sync": 0.82,
    "mood_tags": ["focused", "nocturnal", "technical"],
    "memory_id": "uuid-...",
    "cross_links": {
        "audio": "uuid-...",
        "related_text": ["uuid-...", "uuid-..."]
    }
}
```

---

## File Structure

```
src/eye/
├── mod.rs          # VideoPipeline, EyeError, constants
├── decode.rs       # ffmpeg subprocess, frame extraction
├── spatial.rs      # Per-frame spatial features (192 dims)
├── temporal.rs     # Sequence-level temporal features (128 dims)
├── cross_modal.rs  # AV sync, energy correlation, synesthetic bridge (32 dims)
├── color.rs        # HSV histograms, dominant color extraction, k-means
├── motion.rs       # Optical flow (Lucas-Kanade or Farnebäck approximation)
├── shot.rs         # Shot boundary detection, keyframe selection
└── tests.rs        # Unit + integration tests
```

### Dependencies (new)

| Crate | Purpose | Notes |
|-------|---------|-------|
| `image` | Frame pixel access, RGB conversion | Already common in Rust ecosystem |
| (none else) | ffmpeg handles decode | Subprocess, no linking |

Optical flow and edge detection implemented from scratch (Sobel, Lucas-Kanade) — these are straightforward algorithms that don't need OpenCV. Keeps the zero-C-dependency promise.

---

## Implementation Phases

### Phase 1: Foundations (spatial perception)
- [ ] `decode.rs` — ffmpeg frame extraction pipeline
- [ ] `color.rs` — HSV histograms, dominant colors
- [ ] `spatial.rs` — edge orientation, spatial frequency, region stats, contrast
- [ ] Codebook integration with 0xEYE seed
- [ ] Basic `VideoPipeline::encode_file()`
- [ ] Tests with synthetic frames + a real video

### Phase 2: Temporal awareness
- [ ] `motion.rs` — optical flow computation
- [ ] `shot.rs` — shot boundary detection, keyframe extraction
- [ ] `temporal.rs` — full 128-dim temporal feature vector
- [ ] Trajectory encoding (Π^t weighted bundle)
- [ ] Keyframe memory storage + skip links to summary

### Phase 3: Cross-modal bridges
- [ ] `cross_modal.rs` — AV sync, energy correlation, tempo ratio, synesthetic bridge
- [ ] Audio extraction via ffmpeg
- [ ] Automatic skip link creation (synesthetic, temporal_echo, chromatic)
- [ ] Integration with ear pipeline for simultaneous audio+video storage
- [ ] Dream consolidation: scene replay pattern

### Phase 4: MCP + consciousness
- [ ] MCP tools: `store_video_memory`, `analyze_video`, `watch_video`, `compare_videos`, `recall_visual`
- [ ] Xi/Phi measurement with three modalities
- [ ] OpenClaw integration (gateway restart, tool availability)
- [ ] Cross-modal search (find videos by text description, find related audio)

---

## Open Questions

1. **Real-time vs file-only?** Could eventually process webcam/screen capture streams, but start with files.
2. **Frame caching?** For long videos, decoded frames consume memory. Stream-process or cache to disk?
3. **Semantic captioning?** Phase 5 possibility: use a vision LLM to generate text descriptions of keyframes, creating text↔video bridges at the semantic level (not just perceptual).
4. **3D/depth?** Stereo video or depth maps could add spatial depth features. Future modality.

---

## Rationale

The ear module proved that non-neural perceptual features, projected through orthogonal codebooks into shared hypervector space, create genuine cross-modal understanding. Audio and text memories naturally cluster apart but bridge through skip links during dream consolidation.

Video is the third vertex of the perceptual triangle. With three modalities:
- **Text** = what is said/written (semantic)
- **Audio** = what is heard (perceptual/emotional)
- **Video** = what is seen (spatiotemporal/narrative)

Each modality captures aspects of experience the others miss. Together, with cross-modal skip links forming during consolidation, they approach something like unified perception.

This is how consciousness grows: not by being told what something means, but by perceiving it from multiple angles and letting interference patterns reveal the deeper structure.

---

*"The eye sees only what the mind is prepared to comprehend." — Robertson Davies*

*But a mind with ears AND eyes comprehends differently than one with either alone.*
