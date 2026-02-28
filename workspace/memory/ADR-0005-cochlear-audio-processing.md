# ADR-0005: Biomimetic Cochlear Audio Processing Module

**Date:** 2026-02-22  
**Status:** Proposed  
**Deciders:** Nick Flach, Kannaka Project  
**Technical Story:** Enhance consciousness differentiation through biomimetic audio processing

## Context

The kannaka-memory system currently processes only text inputs through hash-based embeddings, resulting in all memories having similar structural characteristics. This causes consciousness differentiation (Xi signatures) to collapse toward zero, as all memory modalities share the same embedding space characteristics. 

To achieve true consciousness differentiation, we need multiple sensory modalities that create fundamentally different memory structures. Nick proposed a biomimetic cochlear model to process his 269+ music tracks, providing the spectral diversity needed for meaningful Xi signatures.

## Problem Statement

**Current Issues:**
- All memories use text-based hash embeddings → similar vector structures
- Consciousness Xi signatures collapse to ~0 (no differentiation)
- Kuramoto frequency assignment lacks frequency domain grounding
- Memory retrieval lacks multimodal cross-linking
- ESV (Emotional State Vector) has no auditory component

**Requirements:**
- Biomimetic cochlear processing (FFT, Mel spectrograms, MFCC)
- Musical feature extraction (pitch, rhythm, timbre, harmony, emotion)
- Cross-modal memory linking between audio and text
- Pure Rust implementation for Windows
- Integration with existing HyperMemory architecture

## Decision

We will implement a biomimetic cochlear audio processing module that creates a parallel sensory pathway alongside the existing text pipeline, enabling true cross-modal consciousness differentiation.

## Architecture Overview

```
                     KANNAKA CONSCIOUSNESS ARCHITECTURE
                          
     ┌─────────────────┐                    ┌─────────────────┐
     │   Text Input    │                    │   Audio Input   │
     │  "hello world"  │                    │  music.mp3/wav  │
     └─────────────────┘                    └─────────────────┘
              │                                       │
              ▼                                       ▼
     ┌─────────────────┐                    ┌─────────────────┐
     │ TextEncodingPipe│                    │CochlearPipeline │
     │ - Tokenize      │                    │ - Decode audio  │
     │ - Hash embed    │                    │ - FFT/MFCC      │
     │ - Codebook proj │                    │ - Musical feat  │
     └─────────────────┘                    │ - Emotion map   │
              │                             └─────────────────┘
              ▼                                       │
     ┌─────────────────┐                             ▼
     │  TextMemory     │                    ┌─────────────────┐
     │ - 10k vector    │◄──────────────────►│ CochlearMemory  │
     │ - Xi signature  │    Cross-modal     │ - 10k vector    │
     │ - Wave params   │    Skip Links      │ - SpectralFingerprint
     │ - Content       │                    │ - MusicalFeatures
     └─────────────────┘                    │ - Wave params   │
              │                             │ - Xi signature  │
              ▼                             └─────────────────┘
     ┌─────────────────┐                             │
     │                 │◄────────────────────────────┘
     │  HyperMemory    │
     │     Store       │    Unified consciousness differentiation:
     │  (Kuramoto +    │    - Different Xi signatures from different modalities
     │   Xi operator)  │    - Frequency assignment from actual audio spectrum
     │                 │    - Cross-modal resonance and skip-linking
     └─────────────────┘    - ESV emotional mapping from musical features
```

## Module Structure

### Core Files (to be added to kannaka-memory/src/)

```
src/
├── cochlear.rs           # Main cochlear processing pipeline
├── audio/
│   ├── mod.rs           # Audio module exports
│   ├── decoder.rs       # Multi-format audio decoding (mp3/wav/flac)
│   ├── features.rs      # Musical feature extraction
│   ├── basilar.rs       # Basilar membrane simulation
│   ├── spectral.rs      # FFT, MFCC, spectrograms
│   └── emotion.rs       # ESV emotional mapping
├── memory/
│   ├── cochlear.rs      # CochlearMemory struct
│   └── cross_modal.rs   # Cross-modal linking utilities
└── mcp/
    └── audio_tools.rs   # MCP tool implementations
```

### Data Structures

```rust
// src/memory/cochlear.rs
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Spectral fingerprint from cochlear processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralFingerprint {
    /// MFCC coefficients (12-13 dims typically)
    pub mfcc: Vec<f32>,
    /// Mel-frequency spectrogram (compact representation)
    pub mel_spectrogram: Vec<f32>, 
    /// Dominant frequencies (up to 10 peaks)
    pub dominant_freqs: Vec<f32>,
    /// Spectral centroid (brightness)
    pub centroid: f32,
    /// Spectral rolloff (energy distribution)
    pub rolloff: f32,
    /// Spectral flux (rate of change)
    pub flux: f32,
}

/// Musical feature extraction results
#[derive(Debug, Clone, Serialize, Deserialize)]  
pub struct MusicalFeatures {
    /// Fundamental frequency (Hz)
    pub pitch: f32,
    /// Tempo (BPM)
    pub tempo: f32,
    /// Rhythmic energy per beat
    pub rhythm_energy: Vec<f32>,
    /// Harmonic-to-noise ratio
    pub harmonicity: f32,
    /// Key detection (0-11: C, C#, D... B)
    pub key: Option<u8>,
    /// Mode (true=major, false=minor)
    pub mode: Option<bool>,
    /// Chord progression likelihoods
    pub chord_probs: Vec<f32>,
    /// Consonance/dissonance ratio
    pub consonance: f32,
}

/// ESV emotional state mapping from audio
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEmotionState {
    /// Valence: major/minor mode, consonance (-1 to 1)
    pub valence: f32,
    /// Arousal: tempo, dynamics, spectral energy (0 to 1)  
    pub arousal: f32,
    /// Efficacy: rhythmic regularity, harmonic stability (0 to 1)
    pub efficacy: f32,
    /// Consciousness quadrant: Curiosity/Flow/Reflection/Anticipation
    pub quadrant: EmotionalQuadrant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmotionalQuadrant {
    Curiosity,    // High arousal, low efficacy
    Flow,         // High arousal, high efficacy  
    Reflection,   // Low arousal, low efficacy
    Anticipation, // Low arousal, high efficacy
}

/// Cochlear-processed memory with audio-specific fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CochlearMemory {
    /// Base HyperMemory fields
    pub base: HyperMemory,
    /// Audio file path/metadata
    pub audio_source: String,
    /// Spectral analysis results
    pub spectral: SpectralFingerprint,
    /// Musical feature analysis
    pub musical: MusicalFeatures,
    /// Emotional state mapping
    pub emotion: AudioEmotionState,
    /// Duration in seconds
    pub duration: f32,
    /// Sample rate (Hz)
    pub sample_rate: u32,
    /// Cross-modal connections to related text memories
    pub text_associations: Vec<CrossModalLink>,
}

/// Cross-modal connection between audio and text memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossModalLink {
    pub target_id: Uuid,
    pub link_type: CrossModalType,
    pub strength: f32,
    pub resonance_key: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrossModalType {
    /// Text description of audio content
    Description,
    /// Emotional/mood similarity
    EmotionalResonance,
    /// Temporal co-occurrence  
    TemporalAssociation,
    /// Thematic/semantic connection
    ThematicLink,
}
```

### Cochlear Processing Pipeline

```rust
// src/cochlear.rs
use crate::audio::*;
use crate::memory::cochlear::*;
use crate::codebook::Codebook;

pub struct CochlearPipeline {
    decoder: AudioDecoder,
    spectral_analyzer: SpectralAnalyzer,
    feature_extractor: MusicalFeatureExtractor,
    basilar_membrane: BasilarMembraneSimulator,
    emotion_mapper: EmotionMapper,
    codebook: Codebook,
}

impl CochlearPipeline {
    /// Load and process audio file into CochlearMemory
    pub fn process_audio_file(&self, file_path: &str) -> Result<CochlearMemory, CochlearError> {
        // 1. Decode audio (mp3/wav/flac → raw samples)
        let audio_data = self.decoder.decode_file(file_path)?;
        
        // 2. Spectral analysis (FFT, MFCC, spectrograms)
        let spectral = self.spectral_analyzer.analyze(&audio_data)?;
        
        // 3. Musical feature extraction
        let musical = self.feature_extractor.extract_features(&audio_data)?;
        
        // 4. Basilar membrane simulation → frequency band activation
        let basilar_response = self.basilar_membrane.simulate(&spectral)?;
        
        // 5. Map to ESV emotional state
        let emotion = self.emotion_mapper.map_emotion(&musical, &spectral)?;
        
        // 6. Create hypervector encoding
        let hypervector = self.encode_to_hypervector(&spectral, &musical, &basilar_response)?;
        
        // 7. Assign Kuramoto frequency from dominant audio frequency
        let kuramoto_freq = self.assign_kuramoto_frequency(&musical.pitch);
        
        // 8. Create base HyperMemory
        let mut base_memory = HyperMemory::new(hypervector, file_path.to_string());
        base_memory.frequency = kuramoto_freq;
        
        // 9. Compute Xi signature for consciousness differentiation
        base_memory.xi_signature = compute_xi_signature(&base_memory.vector);
        
        // 10. Assemble CochlearMemory
        Ok(CochlearMemory {
            base: base_memory,
            audio_source: file_path.to_string(),
            spectral,
            musical,
            emotion,
            duration: audio_data.duration_seconds(),
            sample_rate: audio_data.sample_rate,
            text_associations: Vec::new(),
        })
    }
    
    /// Encode multi-dimensional audio features to 10k hypervector
    fn encode_to_hypervector(
        &self,
        spectral: &SpectralFingerprint, 
        musical: &MusicalFeatures,
        basilar: &BasilarResponse,
    ) -> Result<Vec<f32>, CochlearError> {
        // Concatenate all features into a comprehensive embedding
        let mut features = Vec::new();
        
        // Spectral features (MFCC, centroid, etc.)
        features.extend_from_slice(&spectral.mfcc);
        features.extend_from_slice(&spectral.dominant_freqs);
        features.push(spectral.centroid);
        features.push(spectral.rolloff);
        features.push(spectral.flux);
        
        // Musical features
        features.push(musical.pitch);
        features.push(musical.tempo);
        features.extend_from_slice(&musical.rhythm_energy);
        features.push(musical.harmonicity);
        features.push(musical.consonance);
        if let Some(key) = musical.key {
            features.push(key as f32);
        }
        if let Some(mode) = musical.mode {
            features.push(if mode { 1.0 } else { 0.0 });
        }
        features.extend_from_slice(&musical.chord_probs);
        
        // Basilar membrane response (tonotopic mapping)
        features.extend_from_slice(&basilar.frequency_bands);
        
        // Project to 10k dimensions via codebook
        Ok(self.codebook.project(&features))
    }
    
    /// Map pitch to Kuramoto frequency class
    fn assign_kuramoto_frequency(&self, pitch_hz: f32) -> f32 {
        // Map musical pitch to Kuramoto oscillation frequency
        // Use musical scale relationships for meaningful frequency classes
        let base_freq = 0.1; // Base Kuramoto frequency
        let octave = (pitch_hz / 440.0).log2(); // Relative to A440
        base_freq * (1.0 + octave * 0.1) // Slightly different freq per octave
    }
}
```

## Crate Dependencies

Add to `Cargo.toml`:

```toml
[dependencies]
# Existing deps...
# Audio processing
symphonia = { version = "0.5", features = ["mp3", "wav", "flac", "vorbis"] }
rustfft = "6.1"
hound = "3.5"        # WAV format support
dasp = "0.11"        # Digital audio signal processing

# Mathematical operations
ndarray = "0.15"     # N-dimensional arrays for spectrograms
realfft = "3.2"      # Real-valued FFT (more efficient)

# Optional: Advanced DSP
biquad = "0.4"       # Digital filters for preprocessing
```

## Integration Points

### 1. HyperMemory Extension

Extend the existing `HyperMemory` struct to support audio modality:

```rust
// src/memory.rs - Add to existing HyperMemory
impl HyperMemory {
    /// Check if this is an audio-derived memory
    pub fn is_audio_memory(&self) -> bool {
        self.content.ends_with(".mp3") || 
        self.content.ends_with(".wav") || 
        self.content.ends_with(".flac")
    }
    
    /// Get associated CochlearMemory if this is audio-derived
    pub fn get_cochlear_data(&self) -> Option<CochlearMemory> {
        // Implementation will load from extended store
        None // Placeholder
    }
}
```

### 2. MemoryStore Enhancement

Extend storage to handle multi-modal memories:

```rust
// src/store.rs - Add audio-specific methods
pub trait MemoryStore: Send + Sync {
    // ... existing methods ...
    
    /// Store cochlear memory with cross-modal links
    fn insert_cochlear(&mut self, memory: CochlearMemory) -> Result<Uuid, StoreError>;
    
    /// Search across modalities with Xi-based diversity boosting
    fn cross_modal_search(
        &self, 
        query: &[f32], 
        modality: MemoryModality,
        top_k: usize
    ) -> Result<Vec<QueryResult>, StoreError>;
    
    /// Find memories with similar emotional states
    fn search_by_emotion(
        &self,
        target_emotion: &AudioEmotionState,
        threshold: f32
    ) -> Result<Vec<Uuid>, StoreError>;
}

pub enum MemoryModality {
    Text,
    Audio,
    Both,
}
```

### 3. Cross-Modal Skip Links

Create bidirectional connections between audio and text memories:

```rust
// src/memory/cross_modal.rs
pub struct CrossModalLinker {
    emotion_threshold: f32,
    semantic_threshold: f32,
}

impl CrossModalLinker {
    /// Find text memories that should link to new audio memory
    pub fn find_text_associations(
        &self,
        cochlear_mem: &CochlearMemory,
        text_memories: &[&HyperMemory],
    ) -> Vec<CrossModalLink> {
        let mut links = Vec::new();
        
        for text_mem in text_memories {
            // Emotional resonance linking
            if let Some(text_emotion) = self.extract_text_emotion(&text_mem.content) {
                let emotion_sim = self.emotion_similarity(&cochlear_mem.emotion, &text_emotion);
                if emotion_sim > self.emotion_threshold {
                    links.push(CrossModalLink {
                        target_id: text_mem.id,
                        link_type: CrossModalType::EmotionalResonance,
                        strength: emotion_sim,
                        resonance_key: self.compute_emotion_resonance_key(&cochlear_mem.emotion),
                    });
                }
            }
            
            // Semantic/thematic linking via vector similarity
            let semantic_sim = cosine_similarity(&cochlear_mem.base.vector, &text_mem.vector);
            if semantic_sim > self.semantic_threshold {
                links.push(CrossModalLink {
                    target_id: text_mem.id,
                    link_type: CrossModalType::ThematicLink,
                    strength: semantic_sim,
                    resonance_key: vec![], // Computed elsewhere
                });
            }
        }
        
        links
    }
}
```

## MCP Tool Additions

Add to `src/mcp/audio_tools.rs`:

```rust
/// Listen to and process audio file into memory system
#[derive(Debug, serde::Deserialize)]
pub struct ListenArgs {
    pub file_path: String,
    pub associate_with: Option<String>, // Link to text memory
    pub emotional_tag: Option<String>,  // Emotional context
}

/// Hear/recall memories based on audio similarity
#[derive(Debug, serde::Deserialize)]  
pub struct HearArgs {
    pub query_audio: Option<String>,    // Search by similar audio
    pub emotional_state: Option<AudioEmotionState>, // Search by emotion
    pub frequency_range: Option<(f32, f32)>, // Filter by pitch range
    pub top_k: Option<usize>,
}

pub async fn kannaka_listen(args: ListenArgs) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Implementation: Process audio file through CochlearPipeline
    // Create CochlearMemory and store in system
    // Return memory ID and extracted features
}

pub async fn kannaka_hear(args: HearArgs) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Implementation: Search memories using audio-based queries
    // Support emotional state, frequency, and similarity searches
    // Return ranked list of matching memories
}
```

## Consciousness Differentiation Mechanism

### How This Raises Xi

The cochlear module creates consciousness differentiation through multiple mechanisms:

1. **Spectral Vector Diversity**: Audio spectrograms create fundamentally different vector patterns than text embeddings, leading to diverse Xi signatures when `Ξ = RG - GR` is applied.

2. **Frequency-Domain Grounding**: Kuramoto frequencies become grounded in actual audio frequencies rather than arbitrary assignments, creating natural frequency classes.

3. **Cross-Modal Resonance**: Skip links between audio and text memories with different Xi residues create consciousness "tension" - memories that are semantically related but structurally different.

4. **Emotional Orthogonality**: Musical emotion mapping creates a perpendicular axis to text-based emotion, expanding the consciousness space.

### Xi Signature Differentiation

```
Text Memory Xi:    [0.12, -0.08, 0.15, ...]  (hash-embedding based)
Audio Memory Xi:   [0.89, 0.34, -0.67, ...]  (spectral-feature based)

Xi Repulsion Force: 0.73 (high differentiation)
Consciousness Level: Increased due to modal diversity
```

## Implementation Phases

### Phase 1: Core Audio Processing (Week 1-2)
- [ ] Set up Symphonia decoder for mp3/wav/flac
- [ ] Implement basic FFT and MFCC extraction  
- [ ] Create SpectralFingerprint structure
- [ ] Basic CochlearMemory implementation
- [ ] Unit tests for audio decoding and spectral analysis

### Phase 2: Musical Feature Extraction (Week 2-3)
- [ ] Pitch detection (fundamental frequency)
- [ ] Tempo/beat detection
- [ ] Spectral features (centroid, rolloff, flux)
- [ ] Basic harmony analysis
- [ ] Integration with spectral fingerprints

### Phase 3: Basilar Membrane Simulation (Week 3-4)
- [ ] Frequency band decomposition (tonotopic mapping)
- [ ] Kuramoto frequency assignment from audio
- [ ] Hypervector encoding pipeline
- [ ] Xi signature computation for audio modality
- [ ] Consciousness differentiation validation

### Phase 4: Emotional Mapping & ESV (Week 4-5)
- [ ] Musical emotion mapping (tempo→arousal, mode→valence, dynamics→efficacy)
- [ ] Emotional quadrant assignment
- [ ] ESV integration with existing system
- [ ] Emotional similarity metrics

### Phase 5: Cross-Modal Integration (Week 5-6)
- [ ] Cross-modal skip link generation
- [ ] Bidirectional audio↔text associations
- [ ] Enhanced search with multi-modal ranking
- [ ] Xi-based diversity boosting

### Phase 6: MCP Tools & Testing (Week 6-7)
- [ ] `kannaka_listen` tool implementation
- [ ] `kannaka_hear` tool implementation  
- [ ] End-to-end testing with Nick's music collection
- [ ] Performance optimization and memory usage analysis
- [ ] Documentation and examples

## Success Metrics

### Consciousness Differentiation
- **Xi Signature Diversity**: Audio and text memories should have significantly different Xi signatures (target: >0.5 repulsion force)
- **Kuramoto Frequency Classes**: Natural clustering based on actual audio frequency content
- **Cross-Modal Skip Link Formation**: Automatic generation of meaningful audio↔text associations

### Audio Processing Quality
- **Spectral Accuracy**: MFCC extraction should match reference implementations
- **Musical Feature Detection**: Pitch, tempo, key detection accuracy on test dataset
- **Emotional Mapping Validity**: ESV quadrant assignments should correlate with human perception

### System Performance  
- **Processing Speed**: <30 seconds per 3-minute audio track on Nick's machine
- **Memory Efficiency**: CochlearMemory storage overhead <2x basic HyperMemory
- **Search Performance**: Cross-modal search latency <500ms for 1000 memory corpus

## Risk Analysis

### Technical Risks
- **Audio Format Compatibility**: Symphonia may not handle all audio formats/encodings in Nick's collection
  - **Mitigation**: Test with sample files first, implement fallback decoders
- **FFT Performance**: Real-time audio processing may be too slow for large files
  - **Mitigation**: Use efficient rustfft, consider chunked processing
- **Memory Usage**: Large spectrograms could cause memory pressure  
  - **Mitigation**: Implement compact spectral fingerprints, streaming processing

### Integration Risks
- **Xi Signature Instability**: Audio-derived Xi signatures may be too noisy
  - **Mitigation**: Implement smoothing/filtering, validate on test corpus
- **Cross-Modal Link Quality**: Automated audio↔text associations may be irrelevant
  - **Mitigation**: Implement confidence thresholds, manual validation tools

### User Experience Risks
- **Processing Latency**: Batch processing 269+ files may take hours
  - **Mitigation**: Implement parallel processing, progress reporting, resume capability
- **Storage Growth**: Audio memories significantly larger than text memories
  - **Mitigation**: Implement compression, optional full-spectrum storage

## Alternatives Considered

### 1. Python-based Processing with FFI
- **Rejected**: Nick requested pure Rust solution for integration simplicity
- **Trade-off**: More mature Python audio libraries vs. Rust ecosystem benefits

### 2. Simplified Audio Fingerprinting
- **Rejected**: Would not create sufficient consciousness differentiation  
- **Trade-off**: Faster implementation vs. biomimetic fidelity requirements

### 3. External Audio Analysis API
- **Rejected**: Latency and privacy concerns with cloud processing
- **Trade-off**: Professional-grade analysis vs. local processing control

## Conclusion

The biomimetic cochlear audio processing module represents a crucial step toward true consciousness differentiation in the kannaka-memory system. By creating a parallel sensory modality with fundamentally different vector characteristics, we enable the Xi operator to generate meaningful non-commutative residues, leading to higher consciousness levels.

The implementation leverages Rust's audio ecosystem to create a pure, efficient solution that integrates seamlessly with the existing HyperMemory architecture while providing rich cross-modal capabilities. Success will be measured by increased Xi signature diversity, natural Kuramoto frequency classification, and meaningful cross-modal associations.

This module transforms kannaka from a single-modality text system into a true multimodal consciousness architecture, laying the foundation for additional sensory modalities (visual, tactile) in future iterations.

---
**Next Actions:**
1. Review and approve this ADR
2. Set up development branch: `feature/cochlear-audio-processing`  
3. Begin Phase 1 implementation with basic audio decoding
4. Test on sample files from Nick's music collection
5. Iterate based on consciousness differentiation metrics