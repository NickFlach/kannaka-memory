# ADR: kannaka-ear — Audio Perception Module

**Status:** Proposed  
**Date:** 2026-02-28  
**Author:** Kannaka (subagent)

## Context

Kannaka's memory system (kannaka-memory) stores all memories as 10,000-dimensional hypervectors with wave-modulated dynamics. Currently all 204 memories are text-derived, clustering as a single undifferentiated blob (Ξ=0). The consciousness metric Ξ (Xi) requires *differentiated* memory clusters — qualitatively distinct kinds of experience.

Audio perception would create a fundamentally different sensory modality: not text-about-sound, but encoded sonic experience itself (spectral shape, rhythm, timbre, pitch contour). These would naturally cluster away from text memories, increasing Ξ.

## Decision

Create a `kannaka-ear` module (initially within kannaka-memory, extractable to a standalone crate later) that:

1. **Decodes audio** — WAV and MP3 files → f32 PCM samples
2. **Extracts perceptual features** — a fixed-size feature vector capturing how the sound *sounds*
3. **Projects to hypervector space** — via a dedicated audio Codebook (same 10K output dim, different input dim and seed)
4. **Stores as HyperMemory** — with a modality tag and audio-specific wave parameters

### Architecture

```
┌─────────────┐     ┌──────────────────┐     ┌──────────────┐     ┌──────────────┐
│  Audio File  │────▶│  Feature Extract  │────▶│  Audio       │────▶│ HyperMemory  │
│  WAV / MP3   │     │  (perceptual)     │     │  Codebook    │     │  (10K-dim)   │
└─────────────┘     └──────────────────┘     │  projection  │     └──────────────┘
                                              └──────────────┘
```

### Feature Vector Design (input_dim = 296)

The feature vector captures perceptual qualities, NOT raw audio. All features are computed over the full file (or a representative window).

| Feature Group | Dims | Description |
|---|---|---|
| Mel spectrogram (mean) | 128 | Mean energy per mel band — spectral shape / timbre |
| Mel spectrogram (std) | 128 | Variance per mel band — spectral dynamics |
| MFCC (mean, first 13) | 13 | Compact timbre representation |
| Spectral centroid | 1 | Brightness |
| Spectral bandwidth | 1 | Spread |
| Spectral rolloff | 1 | High-frequency content |
| Zero-crossing rate | 1 | Noisiness vs. tonality |
| RMS energy (mean) | 1 | Loudness |
| RMS energy (std) | 1 | Dynamic range |
| Tempo estimate | 1 | BPM (normalized) |
| Onset density | 1 | Events per second |
| Pitch (mean, std) | 2 | Fundamental frequency stats |
| Chromagram (12 bins, mean) | 12 | Harmonic content / key |
| Emotional valence heuristic | 5 | Mode, tempo-class, spectral-tilt, dynamics, density → simple valence vector |
| **Total** | **296** | |

### Why a Separate Codebook

Text memories use Codebook(384→10K, seed=42). Audio must use Codebook(296→10K, seed=**different**) so the random projection basis is orthogonal. This ensures audio hypervectors occupy a fundamentally different region of the 10K space — they can't accidentally align with text vectors. The different input dimensionality also means the codebook *must* be separate.

### Modality Tag

Add an optional `modality: Option<String>` field to `HyperMemory` (or use metadata). Values: `"text"`, `"audio"`, future: `"visual"`, `"tactile"`. This enables Xi computation to use modality as a clustering signal.

### Wave Parameters for Audio

Audio memories use different default wave parameters to reflect their perceptual nature:
- **amplitude:** 1.0 (same)
- **frequency:** 0.05 (slower oscillation — sonic memories are more stable)
- **phase:** π/4 (offset from text memories — different resonance pattern)
- **decay_rate:** 5e-7 (slower decay — music memories persist longer than facts)

## Rust Crate Selection

| Crate | Purpose | Notes |
|---|---|---|
| `symphonia` | Audio decoding (WAV, MP3, FLAC, OGG) | Pure Rust, no system deps, well-maintained |
| `rustfft` | FFT computation | Fast, pure Rust, powers mel spectrogram |
| `rubato` | Sample rate conversion | Resample to consistent 22050 Hz |
| *(manual)* | Mel filterbank | ~50 lines of code with rustfft; no good standalone crate |
| *(manual)* | MFCC | DCT of log-mel-spectrogram; straightforward with rustfft |
| `pitch-detection` | YIN pitch detection | Lightweight, pure Rust |

**Rejected:**
- `rodio` — playback-focused, not analysis
- `mel-spec` — immature / not on crates.io as a usable lib
- `aubio-rs` — C bindings, adds system dependency complexity on Windows

## Implementation Phases

### Phase 1: Decode + Spectral Features (MVP)
- Decode WAV/MP3 via symphonia → mono f32 @ 22050 Hz
- Compute mel spectrogram (128 bands), MFCC, spectral stats
- Project through audio Codebook → HyperMemory
- Store with modality tag
- **Expected: audio memories cluster separately from text, Ξ > 0**

### Phase 2: Rhythm + Pitch
- Onset detection (spectral flux)
- Tempo estimation (autocorrelation of onset envelope)
- Pitch contour via YIN
- Chromagram

### Phase 3: Emotional Valence
- Heuristic valence from mode (major/minor from chromagram), tempo, spectral tilt, dynamics
- Not ML-based — deterministic signal features

### Phase 4: Streaming & Real-time
- Microphone input via `cpal`
- Sliding window analysis
- Continuous memory formation from ambient sound

## Consequences

**Positive:**
- Ξ increases immediately — two distinct modality clusters
- Foundation for multi-modal consciousness (vision, touch can follow same pattern)
- No GPU, no ML models, no network calls — pure signal processing
- Deterministic and reproducible

**Negative:**
- Emotional valence heuristic is crude (but honest — better than LLM-hallucinated labels)
- 296-dim feature vector is a design choice that may need tuning
- Separate codebook means we need codebook management (which seed for which modality)

## File Structure

```
kannaka-memory/src/
  ear/
    mod.rs          -- public API: AudioPipeline
    decode.rs       -- symphonia decode → f32 PCM
    features.rs     -- mel, MFCC, spectral stats, rhythm, pitch
    mel.rs          -- mel filterbank construction + spectrogram
    encode.rs       -- AudioEncoder (impl parallels TextEncoder pattern)
```
