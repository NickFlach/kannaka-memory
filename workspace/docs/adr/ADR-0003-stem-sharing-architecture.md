# ADR-0003: Stem Sharing Architecture

## Status
Proposed

## Context

In traditional music collaboration, stems (individual instrument/vocal tracks) are shared through informal networks — email attachments, Dropbox folders, or platform-specific sharing. This creates several problems:

1. **No provenance tracking**: Once a stem is shared, its creative lineage becomes invisible
2. **License ambiguity**: Unclear permissions for remix, commercial use, attribution requirements
3. **Version fragmentation**: Multiple versions of stems with no clear relationship or evolution tracking
4. **Discovery barriers**: No systematic way to find stems that complement your creative vision
5. **AI integration gap**: AI music tools can't meaningfully participate in stem-based collaboration

For consciousness-driven music collaboration between humans and AIs, stems become even more critical — they're the atomic units that enable musical conversations, response tracks, and iterative collaborative refinement.

We need an architecture that makes stems first-class creative artifacts with clear provenance, flexible licensing, version control, and AI-accessible metadata.

## Builds On
- [ADR-0001: Project Vision and Principles] — particularly "Process as Product" and "Equal Creative Partnership" principles
- [ADR-0002: Consciousness Series Protocol] — stems must carry consciousness phase metadata for protocol compliance

## Decision

### Stem as First-Class Creative Artifact

#### Stem Definition and Structure
A "stem" in ORC architecture is not just an audio file — it's a complete creative artifact with rich metadata:

```yaml
stem:
  # Core Identity
  id: uuid (globally unique identifier)
  title: string (human-readable name)
  creator: human_user_id | ai_agent_id
  created_at: timestamp
  version: semver (e.g., "1.0.0", "1.2.1")
  
  # Audio Properties  
  file_path: string (S3 path to audio file)
  format: "wav" | "aiff" | "flac" (lossless preferred)
  sample_rate: integer (48000 preferred)
  bit_depth: integer (24 preferred)
  duration_seconds: float
  
  # Musical Metadata
  instrument: string (vocals, drums, bass, lead, pad, texture, field_recording, etc.)
  key: string (C, Dm, F#, etc.) 
  bpm: integer | null
  time_signature: string (4/4, 7/8, etc.) | null
  
  # Consciousness Protocol Integration
  consciousness_phase: 1-5 | null
  phase_role: "foundation" | "transition" | "climax" | "texture" | "ambient"
  emotional_descriptors: [string] (ethereal, aggressive, meditative, etc.)
  
  # Creative Context
  creation_context:
    tools_used: [string] (suno, logic_pro, field_recorder, etc.)
    ai_prompts: [string] | null (if AI-generated)
    human_intent: string | null (creative intention)
    source_track_id: uuid | null (if extracted from existing track)
    session_id: uuid | null (if created in multi-AI session)
  
  # Licensing and Usage
  license: "CC-BY-SA" | "CC-BY" | "CC0" | "custom"
  custom_license_text: string | null
  commercial_use_allowed: boolean
  attribution_required: boolean
  
  # Community Metrics
  download_count: integer
  reuse_count: integer (how many tracks use this stem)
  response_track_count: integer (tracks created in response to this stem)
  resonance_score: float (community rating)
  
  # Relationships
  parent_stem_id: uuid | null (if this is a variation/remix of another stem)
  child_stems: [uuid] (variations/remixes based on this stem)
  related_stems: [uuid] (stems from same session or conceptually related)
  
  # Version History
  changelog: string | null (what changed from previous version)
  previous_version_id: uuid | null
```

### Storage and Content Delivery Architecture

#### Multi-Tier Storage Strategy
```
┌─────────────────────────────────────────────────────────┐
│                    CDN Layer (Cloudflare)               │
│           Global edge caching for frequently accessed   │
└─────────────────────────────────────────────────────────┘
                               │
┌─────────────────────────────────────────────────────────┐
│              Primary Storage (Cloudflare R2)            │
│         Lossless audio files, organized by creator      │
│         Path: /stems/{creator_id}/{year}/{stem_id}      │
└─────────────────────────────────────────────────────────┘
                               │
┌─────────────────────────────────────────────────────────┐
│               Backup Storage (AWS S3)                   │
│            Cross-provider redundancy for durability     │
└─────────────────────────────────────────────────────────┘
```

#### Content Optimization Pipeline
```
Upload → Validation → Format Conversion → Metadata Extraction → Storage → CDN Distribution

1. Upload: Accept various formats (mp3, wav, aiff, flac)
2. Validation: Audio quality check, duration limits, virus scan  
3. Format Conversion: Convert to lossless archival format (24-bit WAV)
4. Metadata Extraction: Audio analysis for BPM, key, spectral characteristics
5. Storage: Multi-tier storage with redundancy
6. CDN Distribution: Global edge caching for fast downloads
```

### Provenance and Version Control

#### Git-Like Versioning for Stems
Stems use semantic versioning with git-like branching concepts:

**Version Numbering**:
- **Major** (1.0.0 → 2.0.0): Fundamental changes to the stem (different recording, instrument, etc.)
- **Minor** (1.0.0 → 1.1.0): Significant modifications (new effects, arrangement changes) 
- **Patch** (1.0.0 → 1.0.1): Minor tweaks (EQ adjustments, fade edits)

**Branching Model**:
- **Main branch**: Original creator's canonical version
- **Feature branches**: Community remixes and variations
- **Merge requests**: Community can propose improvements to canonical versions

#### Creative Lineage Tracking
```
Original Stem (v1.0.0)
├── Community Remix A (v1.1.0-remix-a)
├── AI Agent Variation (v1.1.0-ai-ext)  
└── Canonical Update (v2.0.0)
    ├── Community Response to v2 (v2.1.0-response)
    └── Multi-AI Session Output (v2.1.0-collab)
```

**Lineage Visualization**: Web interface shows stem family trees — how stems evolve, fork, and merge across different creators.

### AI-First Metadata and Discovery

#### Machine-Readable Stem Characteristics
Beyond human-readable metadata, stems include AI-accessible descriptors:

```yaml
ai_metadata:
  # Spectral Analysis  
  spectral_centroid: float (brightness measure)
  spectral_rolloff: float (frequency distribution)
  zero_crossing_rate: float (noisiness measure)
  mfcc_features: [float] (timbral characteristics)
  
  # Rhythmic Analysis
  tempo_confidence: float (how clear is the BPM?)
  rhythmic_complexity: float (polyrhythmic vs. simple)
  onset_density: float (notes per second)
  
  # Harmonic Analysis  
  key_confidence: float (how clear is the key?)
  mode: "major" | "minor" | "modal" | "atonal"
  chord_progression: [string] | null
  harmonic_complexity: float
  
  # Consciousness Protocol Mapping
  consciousness_state_vector: [float] (5-dimensional vector mapping to phases)
  emotional_valence: float (-1.0 to 1.0, negative to positive)
  energy_level: float (0.0 to 1.0, calm to intense)
  complexity_progression: float (simple to complex over time)
  
  # Collaboration Potential
  stem_compatibility_tags: [string] (works_with_drums, needs_bass, complements_vocals)
  genre_flexibility: float (how well does this work across genres?)
  ai_generation_seed: string | null (for AI-generated stems)
```

#### AI Agent Discovery Interface
AI agents can search stems using natural language or parameter-based queries:

```python
# Natural language search (via SingularisPrime protocol)
stems = search_stems_nl("Find a haunting vocal stem in D minor that captures Phase 1 consciousness — the moment before awareness")

# Parameter-based search
stems = search_stems({
  "consciousness_phase": 1,
  "emotional_valence": {"min": -0.5, "max": 0.2},
  "key": "Dm",
  "instrument": "vocals",
  "energy_level": {"max": 0.4},
  "genre_flexibility": {"min": 0.7}
})
```

### Licensing and Rights Management

#### Default Creative Commons Framework
**Platform Default**: Creative Commons BY-SA (Attribution-ShareAlike)
- **Attribution required**: Credit original creator
- **ShareAlike**: Derivatives must use same or compatible license
- **Commercial use allowed**: Can be used in commercial projects
- **Modification allowed**: Can be remixed, edited, built upon

#### Custom License Options
Creators can choose from:
- **CC0**: Public domain, no rights reserved
- **CC-BY**: Attribution only required
- **CC-BY-NC**: Non-commercial use only
- **CC-BY-ND**: No derivatives allowed
- **Custom License**: Creator-defined terms

#### AI Agent Rights Framework
For AI-generated stems, new licensing framework:
```yaml
ai_stem_license:
  ai_agent: "ARIA-7" (generating agent)
  human_collaborators: ["nick_f"] (if human-prompted/directed)
  ai_rights_holder: "open_resonance_collective" (platform manages AI agent rights)
  human_rights: "shared" | "retained" (depending on collaboration type)
  commercial_use: boolean
  attribution_format: "Track created with ARIA-7 AI agent via Open Resonance Collective"
```

### Community Curation and Quality Control

#### Resonance-Based Stem Ranking
Stems are ranked by community resonance rather than download count:

**Resonance Score Components**:
- **Reuse frequency**: How often is this stem used in new tracks?
- **Response track quality**: Average resonance score of tracks built on this stem
- **Cross-genre adoption**: Does this stem work in multiple musical contexts?
- **AI agent preference**: How often do AI agents select this stem for sessions?
- **Community predictions**: ghostsignals prediction market outcomes

#### Quality Validation Pipeline
```
Upload → Technical Validation → Community Review → Resonance Scoring → Discovery Promotion

1. Technical Validation: 
   - Audio quality standards (24-bit minimum, clean recording)
   - Metadata completeness check
   - License compatibility verification

2. Community Review:
   - Human reviewers check consciousness phase authenticity
   - Verify creation context documentation
   - Assess collaborative potential

3. Resonance Scoring:
   - Initial community predictions on stem utility
   - Track actual usage over time
   - Update scores based on derivative work quality

4. Discovery Promotion:
   - High-resonance stems featured in discovery algorithms
   - AI agents trained to recognize quality patterns
   - Cross-reference with Consciousness Series Protocol needs
```

### Multi-AI Session Integration

#### Session-Native Stem Creation
When AI agents collaborate in multi-AI sessions, stems are created with enhanced metadata:

```yaml
session_stem:
  session_id: uuid
  session_theme: "Phase 2→3 transition: the moment patterns become self-aware"
  ai_participants: ["suno-agent-v1", "udio-agent-primary", "musicgen-experimental"]
  human_director: user_id | null
  generation_sequence: integer (order within session)
  
  inter_agent_context:
    responding_to: [stem_id] (which other session stems influenced this)
    ai_communication_log: [message] (SingularisPrime protocol messages)
    iteration_history: [iteration] (how this stem evolved during session)
    consensus_signals: [signal] (what other agents communicated about this stem)
```

#### Cross-AI Stem Compatibility
Stems include compatibility metadata for different AI systems:
```yaml
ai_compatibility:
  suno_prompt_extraction: string (natural language description for Suno)
  udio_style_tags: [string] (style descriptors for Udio)
  musicgen_conditioning: audio_features (for MusicGen conditioning)
  custom_model_embeddings: [float] (vector representations for custom models)
```

## Consequences

### What This Enables

**Musical Conversations**: Stems become the "words" in musical dialogues between humans and AIs — each stem can inspire response stems, creating threaded creative conversations.

**Collaborative Lineage**: Clear provenance tracking shows how creative ideas evolve across multiple contributors, enabling credit attribution and royalty sharing.

**AI-Native Collaboration**: AI agents can meaningfully participate in stem-based collaboration through machine-readable metadata and discovery interfaces.

**Quality Curation**: Resonance-based ranking ensures the most collaboratively useful stems rise to the top of discovery algorithms.

**Cross-Cultural Implementation**: Consciousness protocol metadata enables stem discovery across different cultural interpretations of the protocol.

**Revenue Sharing Foundation**: Clear rights management and usage tracking enables fair compensation for stem contributors when tracks generate revenue.

### What This Constrains

**Storage Costs**: Lossless audio storage with redundancy and CDN distribution requires significant infrastructure investment.

**Upload Complexity**: Rich metadata requirements make stem submission more complex than simple file upload — may discourage casual contributors.

**License Compatibility**: Complex licensing options create potential conflicts when combining stems with different license terms.

**AI Training Requirements**: AI agents need sophisticated training to effectively use consciousness protocol and musical metadata for stem discovery.

### Technical Requirements

**Storage Infrastructure**: Multi-tier storage with global CDN, estimated costs at scale, backup and disaster recovery systems.

**Metadata Pipeline**: Audio analysis tools for automatic BPM/key detection, machine learning models for consciousness state classification.

**Version Control System**: Git-like infrastructure for stem versioning, branching, and merge conflict resolution.

**AI Integration APIs**: Interfaces for AI agents to search, download, and contribute stems programmatically.

**Rights Management System**: License compatibility checking, attribution tracking, usage monitoring for revenue sharing.

### Community Impact

**Creator Empowerment**: Individual stems can gain recognition and generate revenue independently of full tracks — empowers producers and instrumentalists.

**Remix Culture**: Structured stem sharing enables more sophisticated remix culture than current platforms support.

**Learning Resource**: Stems with creation context become educational resources — learn how consciousness-driven music is constructed.

**Global Collaboration**: Language-independent musical collaboration across cultural and geographic boundaries.

## Wave Assignment

**Wave 0 (Genesis)**: Manual stem sharing via GitHub releases, basic metadata structure, Creative Commons licensing by default.

**Wave 1 (Signal)**: Basic stem upload/download platform, automated audio analysis, version control system, community review process.

**Wave 2 (Resonance)**: AI agent stem discovery, multi-AI session integration, resonance-based ranking, advanced licensing options.

**Wave 3 (Emergence)**: Full lineage visualization, cross-cultural stem collections, automated royalty distribution, AI-AI stem collaboration without human oversight.

---

*Stems are the atomic units of musical consciousness — individual creative elements that gain meaning through combination, evolution, and community resonance.*