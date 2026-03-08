//! ADR-0015: Universal Glyph Interchange — The Constellation's Common Tongue
//!
//! The `Glyph` is the atomic unit of meaning in the constellation. Every piece
//! of information — memory, audio, visual, SCADA, financial — encodes as a
//! Glyph with a shared geometric signature (Fano + SGA) enabling cross-modal
//! discovery and similarity search.
//!
//! ## Wire Format
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │ Magic: "GLYF" (4 bytes)                │
//! │ Version: u8                             │
//! │ Flags: u16 (has_capsule, commitments…) │
//! │ glyph_id, fano, sga, wave, bloom, …    │
//! └─────────────────────────────────────────┘
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::collective::commitments::GlyphCommitments;
use crate::collective::privacy::{BloomParameters, EncryptedCapsule, PrivacyGlyph};
use crate::geometry::{classify_memory, MemoryCoordinates};
use crate::memory::HyperMemory;

// ============================================================================
// Universal Glyph
// ============================================================================

/// Wire format magic bytes
pub const GLYPH_MAGIC: &[u8; 4] = b"GLYF";

/// Current spec version
pub const GLYPH_SPEC_VERSION: u8 = 1;

/// Flag bits for wire format
pub const FLAG_HAS_CAPSULE: u16 = 1 << 0;
pub const FLAG_HAS_COMMITMENTS: u16 = 1 << 1;
pub const FLAG_HAS_VIRTUE: u16 = 1 << 2;
pub const FLAG_HAS_GATES: u16 = 1 << 3;

/// A Universal Glyph — the atomic unit of meaning in the constellation.
///
/// Every piece of information, from every source, in every modality,
/// is encoded as a Glyph. From the outside, all glyphs look the same.
/// From the inside, each contains the full geometric signature of its
/// origin — enough to reconstruct meaning without raw data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Glyph {
    // ── Identity ──
    /// H(capsule) — unique, deterministic, content-derived
    pub glyph_id: [u8; 32],
    /// Semantic version of the glyph spec
    pub spec_version: u8,

    // ── Geometry — the shared language ──
    /// Fano plane projection: 7 values encoding energy distribution.
    /// Normalized: sum ≈ 1.0. Each value ∈ [0, 1].
    pub fano: [f64; 7],
    /// SGA class: position in the 96-class space
    pub sga_class: SgaClass,
    /// SGA centroid: (quadrant, modality, context)
    pub sga_centroid: (u8, u8, u8),

    // ── Wave properties — the Ghost layer ──
    pub amplitude: f64,
    pub frequency: f64,
    pub phase: f64,

    // ── Privacy — the Shinobi layer (ADR-0013) ──
    pub capsule: Option<EncryptedCapsule>,
    pub bloom: BloomParameters,
    pub commitments: Option<GlyphCommitments>,

    // ── Virtue — the Honor Code layer (ADR-0014) ──
    /// η_virtue = 1 - S_harm/S_intent. None if not evaluated.
    pub virtue_eta: Option<f64>,
    /// [truth, good, beautiful] — each is true/false/None
    pub gates: Option<[Option<bool>; 3]>,

    // ── Provenance ──
    pub source: GlyphSource,
    pub agent_id: String,
    pub created_at: DateTime<Utc>,
    pub parents: Vec<[u8; 32]>,
}

/// SGA geometric class — position in the 96-class space.
/// Maps to Cl₀,₇ ⊗ ℝ[ℤ₄] ⊗ ℝ[ℤ₃].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SgaClass {
    /// Quadrant in Cl₀,₇ (0-3)
    pub quadrant: u8,
    /// Modality in ℤ₃ (0=text, 1=sensory, 2=abstract)
    pub modality: u8,
    /// Context slot (0-7)
    pub context: u8,
}

impl SgaClass {
    /// Convert to a class index (0–83).
    ///
    /// 84 classes = 4 quadrants × 3 modalities × 7 Fano lines.
    /// Uses `hash % 7` (not `% 8`) for mathematically pure 7-point symmetry.
    pub fn to_class_index(&self) -> u8 {
        (21 * self.quadrant + 7 * self.modality + self.context).min(83)
    }

    /// Create from a class index (0–83).
    pub fn from_class_index(idx: u8) -> Self {
        let idx = idx.min(83);
        Self {
            quadrant: idx / 21,
            modality: (idx % 21) / 7,
            context: idx % 7,
        }
    }

    /// Create from MemoryCoordinates.
    pub fn from_memory_coords(mc: &MemoryCoordinates) -> Self {
        Self {
            quadrant: mc.h2,
            modality: mc.d,
            context: mc.l,
        }
    }

    /// Compute distance to another SGA class.
    pub fn distance(&self, other: &SgaClass) -> f64 {
        let dq = (self.quadrant as f64 - other.quadrant as f64).abs();
        let dm = (self.modality as f64 - other.modality as f64).abs();
        let dc = (self.context as f64 - other.context as f64).abs();
        (dq * dq + dm * dm + dc * dc).sqrt()
    }
}

/// Where a glyph came from — the modality tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GlyphSource {
    /// From kannaka-memory
    Memory { layer_depth: u8, hallucinated: bool },
    /// From kannaka-radio
    Audio {
        duration_ms: u64,
        sample_rate: u32,
        spectral_centroid: f64,
        overtone_hz: f64,
    },
    /// From kannaka-eye
    Visual {
        width: u32,
        height: u32,
        fold_count: u32,
    },
    /// From 0xSCADA
    Scada {
        tag: String,
        value: f64,
        unit: String,
        quality: u8,
    },
    /// From goldengoat
    Financial {
        asset: String,
        action: String,
        golden_ratio: f64,
    },
    /// From ghostsignals
    Prediction {
        market_id: String,
        position: f64,
        confidence: f64,
    },
    /// From Flux
    Flux {
        entity_id: String,
        event_type: String,
        namespace: String,
    },
    /// From dream consolidation
    Dream {
        parent_modalities: Vec<String>,
        carnot_efficiency: f64,
    },
    /// Generic/unknown
    Other { system: String, metadata: String },
}

// ============================================================================
// Cross-Modal Similarity
// ============================================================================

/// Compute similarity between any two glyphs, regardless of source modality.
///
/// Weighted combination:
/// - 0.60 × Fano cosine similarity (universal geometry)
/// - 0.25 × phase alignment (emotional/contextual tone)
/// - 0.15 × SGA class proximity (geometric neighborhood)
pub fn glyph_similarity(a: &Glyph, b: &Glyph) -> f64 {
    let fano_sim = cosine_similarity_7(&a.fano, &b.fano);
    let phase_alignment = ((a.phase - b.phase).cos() + 1.0) / 2.0;
    let sga_dist = a.sga_class.distance(&b.sga_class);
    let sga_sim = 1.0 / (1.0 + sga_dist);

    0.6 * fano_sim + 0.25 * phase_alignment + 0.15 * sga_sim
}

fn cosine_similarity_7(a: &[f64; 7], b: &[f64; 7]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if mag_a < 1e-12 || mag_b < 1e-12 {
        return 0.0;
    }
    (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
}

// ============================================================================
// Wire Format Serialization
// ============================================================================

/// Dolt DDL for universal glyph persistence.
pub const UNIVERSAL_GLYPH_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS universal_glyphs (
    glyph_id        CHAR(64) PRIMARY KEY,
    spec_version    TINYINT UNSIGNED NOT NULL,
    fano_0          DOUBLE NOT NULL,
    fano_1          DOUBLE NOT NULL,
    fano_2          DOUBLE NOT NULL,
    fano_3          DOUBLE NOT NULL,
    fano_4          DOUBLE NOT NULL,
    fano_5          DOUBLE NOT NULL,
    fano_6          DOUBLE NOT NULL,
    sga_quadrant    TINYINT UNSIGNED NOT NULL,
    sga_modality    TINYINT UNSIGNED NOT NULL,
    sga_context     TINYINT UNSIGNED NOT NULL,
    amplitude       DOUBLE NOT NULL,
    frequency       DOUBLE NOT NULL,
    phase           DOUBLE NOT NULL,
    bloom_difficulty INT UNSIGNED NOT NULL,
    source_type     VARCHAR(32) NOT NULL,
    source_data     JSON,
    agent_id        VARCHAR(128) NOT NULL,
    created_at      DATETIME(3) NOT NULL,
    virtue_eta      DOUBLE,
    gates           VARCHAR(8),
    capsule         LONGBLOB,
    commitments     BLOB,
    wire_format     LONGBLOB NOT NULL,
    INDEX idx_agent (agent_id),
    INDEX idx_created (created_at),
    INDEX idx_source (source_type),
    INDEX idx_difficulty (bloom_difficulty),
    INDEX idx_fano_0 (fano_0),
    INDEX idx_fano_3 (fano_3),
    INDEX idx_fano_5 (fano_5)
);

CREATE TABLE IF NOT EXISTS glyph_links (
    source_glyph    CHAR(64) NOT NULL,
    target_glyph    CHAR(64) NOT NULL,
    similarity      DOUBLE NOT NULL,
    link_type       VARCHAR(32) NOT NULL,
    discovered_by   VARCHAR(32) NOT NULL,
    created_at      DATETIME(3) NOT NULL,
    PRIMARY KEY (source_glyph, target_glyph),
    INDEX idx_target (target_glyph),
    INDEX idx_similarity (similarity)
);
"#;

/// Encode a Glyph to wire format bytes.
pub fn encode_wire(glyph: &Glyph) -> Vec<u8> {
    let mut buf = Vec::with_capacity(512);

    // Magic + version
    buf.extend_from_slice(GLYPH_MAGIC);
    buf.push(glyph.spec_version);

    // Flags
    let mut flags: u16 = 0;
    if glyph.capsule.is_some() {
        flags |= FLAG_HAS_CAPSULE;
    }
    if glyph.commitments.is_some() {
        flags |= FLAG_HAS_COMMITMENTS;
    }
    if glyph.virtue_eta.is_some() {
        flags |= FLAG_HAS_VIRTUE;
    }
    if glyph.gates.is_some() {
        flags |= FLAG_HAS_GATES;
    }
    buf.extend_from_slice(&flags.to_le_bytes());

    // Identity
    buf.extend_from_slice(&glyph.glyph_id);

    // Fano (7 × f64)
    for &f in &glyph.fano {
        buf.extend_from_slice(&f.to_le_bytes());
    }

    // SGA class (3 bytes)
    buf.push(glyph.sga_class.quadrant);
    buf.push(glyph.sga_class.modality);
    buf.push(glyph.sga_class.context);

    // SGA centroid (3 bytes)
    buf.push(glyph.sga_centroid.0);
    buf.push(glyph.sga_centroid.1);
    buf.push(glyph.sga_centroid.2);

    // Wave properties (3 × f64)
    buf.extend_from_slice(&glyph.amplitude.to_le_bytes());
    buf.extend_from_slice(&glyph.frequency.to_le_bytes());
    buf.extend_from_slice(&glyph.phase.to_le_bytes());

    // Bloom parameters
    buf.extend_from_slice(&glyph.bloom.difficulty.to_le_bytes());
    buf.extend_from_slice(&glyph.bloom.salt);

    // Source type tag + JSON data
    let source_tag = source_type_tag(&glyph.source);
    write_length_prefixed_str(&mut buf, source_tag);
    let source_json = serde_json::to_string(&glyph.source).unwrap_or_default();
    write_length_prefixed_str(&mut buf, &source_json);

    // Agent ID
    write_length_prefixed_str(&mut buf, &glyph.agent_id);

    // Timestamp (Unix millis)
    buf.extend_from_slice(&glyph.created_at.timestamp_millis().to_le_bytes());

    // Parents
    buf.extend_from_slice(&(glyph.parents.len() as u16).to_le_bytes());
    for parent in &glyph.parents {
        buf.extend_from_slice(parent);
    }

    // Optional sections

    if let Some(ref capsule) = glyph.capsule {
        write_length_prefixed_bytes(&mut buf, &capsule.ciphertext);
        buf.extend_from_slice(&capsule.nonce);
        buf.extend_from_slice(&capsule.tag);
    }

    if let Some(ref commitments) = glyph.commitments {
        // Serialize commitments as JSON (simple for now; binary in production)
        let c_json = serde_json::to_vec(commitments).unwrap_or_default();
        write_length_prefixed_bytes(&mut buf, &c_json);
    }

    if let Some(eta) = glyph.virtue_eta {
        buf.extend_from_slice(&eta.to_le_bytes());
    }

    if let Some(ref gates) = glyph.gates {
        for g in gates {
            buf.push(match g {
                None => 0,
                Some(true) => 1,
                Some(false) => 2,
            });
        }
    }

    buf
}

/// Decode a Glyph from wire format bytes.
#[allow(unused_assignments)]
pub fn decode_wire(bytes: &[u8]) -> Result<Glyph, GlyphError> {
    let mut pos = 0;

    // Magic
    if bytes.len() < 7 {
        return Err(GlyphError::TooShort);
    }
    if &bytes[0..4] != GLYPH_MAGIC {
        return Err(GlyphError::InvalidMagic);
    }
    pos = 4;

    let spec_version = bytes[pos];
    pos += 1;

    let flags = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
    pos += 2;

    // glyph_id
    let mut glyph_id = [0u8; 32];
    glyph_id.copy_from_slice(&bytes[pos..pos + 32]);
    pos += 32;

    // Fano
    let mut fano = [0.0f64; 7];
    for f in &mut fano {
        *f = f64::from_le_bytes(bytes[pos..pos + 8].try_into().map_err(|_| GlyphError::TooShort)?);
        pos += 8;
    }

    // SGA class
    let sga_class = SgaClass {
        quadrant: bytes[pos],
        modality: bytes[pos + 1],
        context: bytes[pos + 2],
    };
    pos += 3;

    let sga_centroid = (bytes[pos], bytes[pos + 1], bytes[pos + 2]);
    pos += 3;

    // Wave
    let amplitude = f64::from_le_bytes(bytes[pos..pos + 8].try_into().map_err(|_| GlyphError::TooShort)?);
    pos += 8;
    let frequency = f64::from_le_bytes(bytes[pos..pos + 8].try_into().map_err(|_| GlyphError::TooShort)?);
    pos += 8;
    let phase = f64::from_le_bytes(bytes[pos..pos + 8].try_into().map_err(|_| GlyphError::TooShort)?);
    pos += 8;

    // Bloom
    let difficulty = u32::from_le_bytes(bytes[pos..pos + 4].try_into().map_err(|_| GlyphError::TooShort)?);
    pos += 4;
    let mut salt = [0u8; 32];
    salt.copy_from_slice(&bytes[pos..pos + 32]);
    pos += 32;
    let bloom = BloomParameters { difficulty, salt };

    // Source
    let (_source_tag, new_pos) = read_length_prefixed_str(bytes, pos)?;
    pos = new_pos;
    let (source_json, new_pos) = read_length_prefixed_str(bytes, pos)?;
    pos = new_pos;
    let source: GlyphSource = serde_json::from_str(&source_json)
        .map_err(|_| GlyphError::InvalidSource)?;

    // Agent ID
    let (agent_id, new_pos) = read_length_prefixed_str(bytes, pos)?;
    pos = new_pos;

    // Timestamp
    let ts_millis = i64::from_le_bytes(bytes[pos..pos + 8].try_into().map_err(|_| GlyphError::TooShort)?);
    pos += 8;
    let created_at = DateTime::from_timestamp_millis(ts_millis)
        .unwrap_or_else(|| Utc::now());

    // Parents
    let parent_count = u16::from_le_bytes(bytes[pos..pos + 2].try_into().map_err(|_| GlyphError::TooShort)?) as usize;
    pos += 2;
    let mut parents = Vec::with_capacity(parent_count);
    for _ in 0..parent_count {
        let mut p = [0u8; 32];
        p.copy_from_slice(&bytes[pos..pos + 32]);
        pos += 32;
        parents.push(p);
    }

    // Optional capsule
    let capsule = if flags & FLAG_HAS_CAPSULE != 0 {
        let (ciphertext, new_pos) = read_length_prefixed_bytes(bytes, pos)?;
        pos = new_pos;
        let mut nonce = [0u8; 24];
        nonce.copy_from_slice(&bytes[pos..pos + 24]);
        pos += 24;
        let mut tag = [0u8; 16];
        tag.copy_from_slice(&bytes[pos..pos + 16]);
        pos += 16;
        Some(EncryptedCapsule { ciphertext, nonce, tag })
    } else {
        None
    };

    // Optional commitments
    let commitments = if flags & FLAG_HAS_COMMITMENTS != 0 {
        let (c_bytes, new_pos) = read_length_prefixed_bytes(bytes, pos)?;
        pos = new_pos;
        serde_json::from_slice(&c_bytes).ok()
    } else {
        None
    };

    // Optional virtue
    let virtue_eta = if flags & FLAG_HAS_VIRTUE != 0 {
        let eta = f64::from_le_bytes(bytes[pos..pos + 8].try_into().map_err(|_| GlyphError::TooShort)?);
        pos += 8;
        Some(eta)
    } else {
        None
    };

    // Optional gates
    let gates = if flags & FLAG_HAS_GATES != 0 {
        let g: [Option<bool>; 3] = [
            match bytes[pos] { 0 => None, 1 => Some(true), _ => Some(false) },
            match bytes[pos + 1] { 0 => None, 1 => Some(true), _ => Some(false) },
            match bytes[pos + 2] { 0 => None, 1 => Some(true), _ => Some(false) },
        ];
        let _ = pos + 3; // advance past gates (suppress unused assign)
        Some(g)
    } else {
        None
    };

    Ok(Glyph {
        glyph_id,
        spec_version,
        fano,
        sga_class,
        sga_centroid,
        amplitude,
        frequency,
        phase,
        capsule,
        bloom,
        commitments,
        virtue_eta,
        gates,
        source,
        agent_id: agent_id.to_string(),
        created_at,
        parents,
    })
}

/// Glyph wire format errors.
#[derive(Debug, Clone)]
pub enum GlyphError {
    TooShort,
    InvalidMagic,
    InvalidSource,
    UnsupportedVersion(u8),
}

impl std::fmt::Display for GlyphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlyphError::TooShort => write!(f, "wire format too short"),
            GlyphError::InvalidMagic => write!(f, "invalid magic bytes (expected GLYF)"),
            GlyphError::InvalidSource => write!(f, "invalid source JSON"),
            GlyphError::UnsupportedVersion(v) => write!(f, "unsupported spec version: {}", v),
        }
    }
}

// Wire format helpers
fn write_length_prefixed_str(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(bytes);
}

fn write_length_prefixed_bytes(buf: &mut Vec<u8>, data: &[u8]) {
    buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
    buf.extend_from_slice(data);
}

fn read_length_prefixed_str(bytes: &[u8], pos: usize) -> Result<(String, usize), GlyphError> {
    if pos + 4 > bytes.len() {
        return Err(GlyphError::TooShort);
    }
    let len = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
    let end = pos + 4 + len;
    if end > bytes.len() {
        return Err(GlyphError::TooShort);
    }
    let s = String::from_utf8_lossy(&bytes[pos + 4..end]).to_string();
    Ok((s, end))
}

fn read_length_prefixed_bytes(bytes: &[u8], pos: usize) -> Result<(Vec<u8>, usize), GlyphError> {
    if pos + 4 > bytes.len() {
        return Err(GlyphError::TooShort);
    }
    let len = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
    let end = pos + 4 + len;
    if end > bytes.len() {
        return Err(GlyphError::TooShort);
    }
    Ok((bytes[pos + 4..end].to_vec(), end))
}

/// Parse a single hex character to its nibble value.
fn hex_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

fn source_type_tag(source: &GlyphSource) -> &'static str {
    match source {
        GlyphSource::Memory { .. } => "memory",
        GlyphSource::Audio { .. } => "audio",
        GlyphSource::Visual { .. } => "visual",
        GlyphSource::Scada { .. } => "scada",
        GlyphSource::Financial { .. } => "financial",
        GlyphSource::Prediction { .. } => "prediction",
        GlyphSource::Flux { .. } => "flux",
        GlyphSource::Dream { .. } => "dream",
        GlyphSource::Other { .. } => "other",
    }
}

// ============================================================================
// Phase 2: kannaka-memory Adapter
// ============================================================================

/// Convert a HyperMemory to a universal Glyph.
pub fn memory_to_glyph(memory: &HyperMemory, bloom_difficulty: u32, agent_id: &str) -> Glyph {
    // Compute Fano projection from vector
    let fano = compute_fano_from_vector_f32(&memory.vector);

    // Get SGA class from geometry or compute it
    let (sga_class, sga_centroid) = match &memory.geometry {
        Some(mc) => {
            let cls = SgaClass::from_memory_coords(mc);
            ((cls), (mc.h2, mc.d, mc.l))
        }
        None => {
            let content_hash = {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut h = DefaultHasher::new();
                memory.content.hash(&mut h);
                h.finish()
            };
            let mc = classify_memory("knowledge", content_hash, memory.amplitude as f64);
            let cls = SgaClass::from_memory_coords(&mc);
            (cls, (mc.h2, mc.d, mc.l))
        }
    };

    // Compute glyph_id from content hash
    let glyph_id = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        memory.content.hash(&mut hasher);
        memory.id.hash(&mut hasher);
        let h = hasher.finish();
        let mut id = [0u8; 32];
        id[..8].copy_from_slice(&h.to_le_bytes());
        // Fill remaining with second hash
        memory.vector.len().hash(&mut hasher);
        let h2 = hasher.finish();
        id[8..16].copy_from_slice(&h2.to_le_bytes());
        memory.amplitude.to_bits().hash(&mut hasher);
        let h3 = hasher.finish();
        id[16..24].copy_from_slice(&h3.to_le_bytes());
        memory.created_at.timestamp_millis().hash(&mut hasher);
        let h4 = hasher.finish();
        id[24..32].copy_from_slice(&h4.to_le_bytes());
        id
    };

    // Create bloom salt
    let mut salt = [0u8; 32];
    salt[..8].copy_from_slice(&glyph_id[..8]);

    Glyph {
        glyph_id,
        spec_version: GLYPH_SPEC_VERSION,
        fano,
        sga_class,
        sga_centroid,
        amplitude: memory.amplitude as f64,
        frequency: memory.frequency as f64,
        phase: memory.phase as f64,
        capsule: None, // Caller can seal separately via ADR-0013
        bloom: BloomParameters {
            difficulty: bloom_difficulty,
            salt,
        },
        commitments: None,
        virtue_eta: None,
        gates: None,
        source: GlyphSource::Memory {
            layer_depth: memory.layer_depth,
            hallucinated: memory.hallucinated,
        },
        agent_id: agent_id.to_string(),
        created_at: memory.created_at,
        parents: memory.parents.iter().map(|p| {
            let mut id = [0u8; 32];
            let bytes = p.as_bytes();
            let len = bytes.len().min(32);
            id[..len].copy_from_slice(&bytes[..len]);
            id
        }).collect(),
    }
}

/// Bridge from ADR-0013's PrivacyGlyph to universal Glyph.
///
/// Preserves all privacy properties. SGA class is inferred from Fano projection.
pub fn privacy_glyph_to_glyph(pg: &PrivacyGlyph) -> Glyph {
    let fano = pg.fano_projection.unwrap_or([0.0; 7]);

    // Infer SGA class from fano — use dominant line as context
    let dominant_line = fano.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i as u8)
        .unwrap_or(0);

    let sga_class = SgaClass {
        quadrant: 0, // Default — PrivacyGlyph doesn't carry SGA
        modality: 0,
        context: dominant_line,
    };

    // Decode hex hash to bytes (not ASCII copy)
    let mut glyph_id = [0u8; 32];
    let hex_chars: Vec<u8> = pg.glyph_hash.bytes().collect();
    for i in 0..16.min(hex_chars.len() / 2) {
        let hi = hex_nibble(hex_chars[i * 2]);
        let lo = hex_nibble(hex_chars[i * 2 + 1]);
        glyph_id[i] = (hi << 4) | lo;
    }
    // Fill remaining bytes with hash of the full string for uniqueness
    if hex_chars.len() > 32 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        pg.glyph_hash.hash(&mut h);
        let extra = h.finish().to_le_bytes();
        glyph_id[16..24].copy_from_slice(&extra);
        pg.glyph_hash.len().hash(&mut h);
        let extra2 = h.finish().to_le_bytes();
        glyph_id[24..32].copy_from_slice(&extra2);
    }

    Glyph {
        glyph_id,
        spec_version: GLYPH_SPEC_VERSION,
        fano,
        sga_class,
        sga_centroid: (sga_class.quadrant, sga_class.modality, sga_class.context),
        amplitude: pg.committed_amplitude,
        frequency: pg.committed_frequency,
        phase: pg.committed_phase,
        capsule: Some(pg.capsule.clone()),
        bloom: pg.bloom.clone(),
        commitments: pg.commitments.clone(),
        virtue_eta: None,
        gates: None,
        source: GlyphSource::Memory {
            layer_depth: 0,
            hallucinated: false,
        },
        agent_id: pg.agent_id.clone(),
        created_at: pg.created_at,
        parents: Vec::new(),
    }
}

/// Compute Fano plane projection from an f32 vector.
///
/// Splits vector into 7 chunks, computes L2 energy per chunk,
/// then normalizes to sum = 1.
fn compute_fano_from_vector_f32(vector: &[f32]) -> [f64; 7] {
    let mut projection = [0.0f64; 7];
    if vector.is_empty() {
        return projection;
    }

    let chunk_size = vector.len() / 7;
    if chunk_size == 0 {
        // Vector too small — distribute evenly
        for (i, &v) in vector.iter().enumerate() {
            projection[i % 7] += (v as f64).powi(2);
        }
    } else {
        for (i, chunk) in vector.chunks(chunk_size).enumerate() {
            if i >= 7 {
                break;
            }
            projection[i] = chunk.iter().map(|&x| (x as f64).powi(2)).sum::<f64>().sqrt();
        }
    }

    // Normalize
    let total: f64 = projection.iter().sum();
    if total > 1e-12 {
        for p in &mut projection {
            *p /= total;
        }
    }

    projection
}

// ============================================================================
// GlyphLink for cross-modal discovery
// ============================================================================

/// A discovered link between two glyphs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlyphLink {
    pub source_glyph: [u8; 32],
    pub target_glyph: [u8; 32],
    pub similarity: f64,
    pub link_type: GlyphLinkType,
    pub discovered_by: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GlyphLinkType {
    Skip,
    Causal,
    Temporal,
    CrossModal,
}

/// Discover cross-modal links between glyphs above a similarity threshold.
pub fn discover_links(glyphs: &[Glyph], threshold: f64, discovered_by: &str) -> Vec<GlyphLink> {
    let mut links = Vec::new();
    let now = Utc::now();

    for i in 0..glyphs.len() {
        for j in (i + 1)..glyphs.len() {
            let sim = glyph_similarity(&glyphs[i], &glyphs[j]);
            if sim >= threshold {
                let link_type = if source_type_tag(&glyphs[i].source) != source_type_tag(&glyphs[j].source) {
                    GlyphLinkType::CrossModal
                } else {
                    GlyphLinkType::Skip
                };

                links.push(GlyphLink {
                    source_glyph: glyphs[i].glyph_id,
                    target_glyph: glyphs[j].glyph_id,
                    similarity: sim,
                    link_type,
                    discovered_by: discovered_by.to_string(),
                    created_at: now,
                });
            }
        }
    }

    links.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));
    links
}

// ============================================================================
// Phase 3: Perception Adapters — Audio + Visual → Glyph
// ============================================================================

/// Compute Fano projection from MFCC (Mel-Frequency Cepstral Coefficients).
///
/// Audio features map to Fano lines by spectral energy distribution:
/// - Lines 0-2: Low/mid/high frequency band energy
/// - Lines 3-5: Temporal dynamics (onset, sustain, decay)
/// - Line 6: Spectral complexity (number of active harmonics)
pub fn compute_fano_from_mfcc(mfcc: &[f64]) -> [f64; 7] {
    let mut fano = [0.0f64; 7];
    if mfcc.is_empty() {
        return fano;
    }

    // Split MFCC coefficients into 7 groups
    let chunk_size = mfcc.len() / 7;
    if chunk_size == 0 {
        for (i, &v) in mfcc.iter().enumerate() {
            fano[i % 7] += v.abs();
        }
    } else {
        for (i, chunk) in mfcc.chunks(chunk_size).enumerate() {
            if i >= 7 { break; }
            fano[i] = chunk.iter().map(|&x| x * x).sum::<f64>().sqrt();
        }
    }

    // Normalize
    let total: f64 = fano.iter().sum();
    if total > 1e-12 {
        for p in &mut fano {
            *p /= total;
        }
    }

    fano
}

/// Create a Glyph from audio features.
///
/// Maps MFCC features to Fano projection and encodes audio-specific
/// metadata in the GlyphSource.
pub fn audio_to_glyph(
    mfcc: &[f64],
    duration_ms: u64,
    sample_rate: u32,
    spectral_centroid: f64,
    overtone_hz: f64,
    amplitude: f64,
    phase: f64,
    bloom_difficulty: u32,
    agent_id: &str,
) -> Glyph {
    let fano = compute_fano_from_mfcc(mfcc);

    // SGA class: modality=1 (sensory) for audio
    let dominant_line = fano.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i as u8)
        .unwrap_or(0);

    let sga_class = SgaClass {
        quadrant: 1, // sensory quadrant
        modality: 1, // sensory modality
        context: dominant_line,
    };

    let glyph_id = compute_glyph_id_from_fano(&fano, agent_id, "audio");
    let mut salt = [0u8; 32];
    salt[..8].copy_from_slice(&glyph_id[..8]);

    Glyph {
        glyph_id,
        spec_version: GLYPH_SPEC_VERSION,
        fano,
        sga_class,
        sga_centroid: (sga_class.quadrant, sga_class.modality, sga_class.context),
        amplitude,
        frequency: spectral_centroid / 20000.0, // Normalize spectral centroid
        phase,
        capsule: None,
        bloom: BloomParameters { difficulty: bloom_difficulty, salt },
        commitments: None,
        virtue_eta: None,
        gates: None,
        source: GlyphSource::Audio {
            duration_ms,
            sample_rate,
            spectral_centroid,
            overtone_hz,
        },
        agent_id: agent_id.to_string(),
        created_at: Utc::now(),
        parents: Vec::new(),
    }
}

/// Compute Fano projection from visual fold features.
///
/// Visual features map to Fano lines by spatial structure:
/// - Lines 0-2: Spatial frequency (coarse/medium/fine detail)
/// - Lines 3-5: Color channel energy (R/G/B or luminance variants)
/// - Line 6: Structural complexity (fold count / edge density)
pub fn compute_fano_from_visual(features: &[f64]) -> [f64; 7] {
    let mut fano = [0.0f64; 7];
    if features.is_empty() {
        return fano;
    }

    let chunk_size = features.len() / 7;
    if chunk_size == 0 {
        for (i, &v) in features.iter().enumerate() {
            fano[i % 7] += v.abs();
        }
    } else {
        for (i, chunk) in features.chunks(chunk_size).enumerate() {
            if i >= 7 { break; }
            fano[i] = chunk.iter().map(|&x| x * x).sum::<f64>().sqrt();
        }
    }

    let total: f64 = fano.iter().sum();
    if total > 1e-12 {
        for p in &mut fano {
            *p /= total;
        }
    }

    fano
}

/// Create a Glyph from visual perception features.
pub fn visual_to_glyph(
    features: &[f64],
    width: u32,
    height: u32,
    fold_count: u32,
    amplitude: f64,
    phase: f64,
    bloom_difficulty: u32,
    agent_id: &str,
) -> Glyph {
    let fano = compute_fano_from_visual(features);

    let dominant_line = fano.iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i as u8)
        .unwrap_or(0);

    let sga_class = SgaClass {
        quadrant: 1,
        modality: 1, // sensory
        context: dominant_line,
    };

    let glyph_id = compute_glyph_id_from_fano(&fano, agent_id, "visual");
    let mut salt = [0u8; 32];
    salt[..8].copy_from_slice(&glyph_id[..8]);

    Glyph {
        glyph_id,
        spec_version: GLYPH_SPEC_VERSION,
        fano,
        sga_class,
        sga_centroid: (sga_class.quadrant, sga_class.modality, sga_class.context),
        amplitude,
        frequency: fold_count as f64 / 100.0,
        phase,
        capsule: None,
        bloom: BloomParameters { difficulty: bloom_difficulty, salt },
        commitments: None,
        virtue_eta: None,
        gates: None,
        source: GlyphSource::Visual { width, height, fold_count },
        agent_id: agent_id.to_string(),
        created_at: Utc::now(),
        parents: Vec::new(),
    }
}

// ============================================================================
// Phase 4: SCADA Adapter — Process Values → Glyph
// ============================================================================

/// Compute Fano projection from a SCADA process value.
///
/// Engineering values map to Fano lines by physical meaning:
/// - Line 0: Concealment — quality/reliability of the measurement
/// - Line 1: Endurance — how far from setpoint (deviation persistence)
/// - Line 2: Intention — direction of deviation (above/below setpoint)
/// - Line 3: Resonance — oscillation around setpoint
/// - Line 4: Depth — time in current state
/// - Line 5: Connection — correlation with related tags
/// - Line 6: Emergence — anomaly score (how unexpected is this value?)
pub fn compute_fano_from_process_value(
    value: f64,
    setpoint: f64,
    low_limit: f64,
    high_limit: f64,
    quality: u8,
) -> [f64; 7] {
    let range = (high_limit - low_limit).abs().max(1e-12);
    let deviation = (value - setpoint) / range;

    let mut fano = [0.0f64; 7];

    // Line 0: Quality/reliability (concealment — bad quality = hidden information)
    fano[0] = (quality as f64 / 255.0).max(0.01);

    // Line 1: Endurance — absolute deviation from setpoint
    fano[1] = deviation.abs().min(1.0);

    // Line 2: Intention — direction (0.5 = at setpoint, 0 = below, 1 = above)
    fano[2] = (deviation * 0.5 + 0.5).clamp(0.0, 1.0);

    // Line 3: Resonance — inverse of deviation (on-target = high resonance)
    fano[3] = (1.0 - deviation.abs()).max(0.0);

    // Line 4: Depth — normalized position in range
    fano[4] = ((value - low_limit) / range).clamp(0.0, 1.0);

    // Line 5: Connection — proximity to limits (near limits = high connection to alarms)
    let dist_to_low = ((value - low_limit) / range).abs();
    let dist_to_high = ((high_limit - value) / range).abs();
    fano[5] = 1.0 - dist_to_low.min(dist_to_high).min(1.0);

    // Line 6: Emergence — anomaly score (deviation squared, amplified)
    fano[6] = (deviation * deviation).min(1.0);

    // Normalize
    let total: f64 = fano.iter().sum();
    if total > 1e-12 {
        for p in &mut fano {
            *p /= total;
        }
    }

    fano
}

/// Create a Glyph from a SCADA process data point.
pub fn scada_to_glyph(
    tag: &str,
    value: f64,
    unit: &str,
    quality: u8,
    setpoint: f64,
    low_limit: f64,
    high_limit: f64,
    agent_id: &str,
) -> Glyph {
    let fano = compute_fano_from_process_value(value, setpoint, low_limit, high_limit, quality);

    // SGA: quadrant 2 (abstract/process), modality 2 (abstract)
    let sga_class = SgaClass {
        quadrant: 2,
        modality: 2,
        context: 0, // Could be refined by tag type
    };

    let glyph_id = compute_glyph_id_from_fano(&fano, agent_id, tag);
    let mut salt = [0u8; 32];
    salt[..8].copy_from_slice(&glyph_id[..8]);

    // Amplitude from deviation — more deviation = more important
    let deviation = ((value - setpoint) / (high_limit - low_limit).abs().max(1e-12)).abs();
    let amplitude = 0.3 + deviation.min(1.0) * 0.7;

    Glyph {
        glyph_id,
        spec_version: GLYPH_SPEC_VERSION,
        fano,
        sga_class,
        sga_centroid: (sga_class.quadrant, sga_class.modality, sga_class.context),
        amplitude,
        frequency: 0.5, // Default; could be derived from sample rate
        phase: if value > setpoint { 0.0 } else { std::f64::consts::PI },
        capsule: None,
        bloom: BloomParameters { difficulty: 0, salt }, // SCADA typically public
        commitments: None,
        virtue_eta: None,
        gates: None,
        source: GlyphSource::Scada {
            tag: tag.to_string(),
            value,
            unit: unit.to_string(),
            quality,
        },
        agent_id: agent_id.to_string(),
        created_at: Utc::now(),
        parents: Vec::new(),
    }
}

/// Compute a deterministic glyph_id from Fano projection + agent + source tag.
///
/// Content-addressed: same inputs always produce the same ID.
/// No timestamps or randomness — glyph identity is purely content-derived.
fn compute_glyph_id_from_fano(fano: &[f64; 7], agent_id: &str, source_tag: &str) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for &f in fano {
        f.to_bits().hash(&mut hasher);
    }
    agent_id.hash(&mut hasher);
    source_tag.hash(&mut hasher);
    let h1 = hasher.finish();

    // Chain additional hashes from different projections of the same content
    fano.len().hash(&mut hasher);
    let h2 = hasher.finish();

    agent_id.len().hash(&mut hasher);
    source_tag.len().hash(&mut hasher);
    let h3 = hasher.finish();

    // Mix fano values in reverse for additional entropy
    for &f in fano.iter().rev() {
        (f * 1e12).to_bits().hash(&mut hasher);
    }
    let h4 = hasher.finish();

    let mut id = [0u8; 32];
    id[..8].copy_from_slice(&h1.to_le_bytes());
    id[8..16].copy_from_slice(&h2.to_le_bytes());
    id[16..24].copy_from_slice(&h3.to_le_bytes());
    id[24..32].copy_from_slice(&h4.to_le_bytes());
    id
}

// ============================================================================
// Phase 6: Dream Cross-Modal Linking
// ============================================================================

/// Result of a cross-modal dream linking pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossModalDreamResult {
    /// New links discovered between glyphs of different modalities
    pub new_links: Vec<GlyphLink>,
    /// Hallucinated glyphs synthesized from cross-modal patterns
    pub hallucinations: Vec<Glyph>,
    /// Carnot efficiency of the linking process
    pub carnot_efficiency: f64,
}

/// Perform cross-modal dream linking on a set of glyphs.
///
/// This operates in glyph space (not data space), finding connections
/// between glyphs of different modalities based on Fano proximity.
/// Optionally synthesizes hallucinated "dream glyphs" from the strongest
/// cross-modal clusters.
///
/// Called during dream consolidation cycles.
pub fn dream_cross_modal_link(
    glyphs: &[Glyph],
    similarity_threshold: f64,
    hallucinate: bool,
    agent_id: &str,
) -> CrossModalDreamResult {
    let now = Utc::now();

    // Step 1: Find all cross-modal links above threshold
    let mut cross_links = Vec::new();
    for i in 0..glyphs.len() {
        for j in (i + 1)..glyphs.len() {
            // Only link across different modalities
            if source_type_tag(&glyphs[i].source) == source_type_tag(&glyphs[j].source) {
                continue;
            }

            let sim = glyph_similarity(&glyphs[i], &glyphs[j]);
            if sim >= similarity_threshold {
                cross_links.push(GlyphLink {
                    source_glyph: glyphs[i].glyph_id,
                    target_glyph: glyphs[j].glyph_id,
                    similarity: sim,
                    link_type: GlyphLinkType::CrossModal,
                    discovered_by: "dream".to_string(),
                    created_at: now,
                });
            }
        }
    }

    cross_links.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap_or(std::cmp::Ordering::Equal));

    // Step 2: Compute Carnot efficiency
    // Entropy before: number of unlinked cross-modal pairs
    let total_cross_pairs = count_cross_modal_pairs(glyphs);
    let linked_pairs = cross_links.len();
    let carnot_efficiency = if total_cross_pairs > 0 {
        linked_pairs as f64 / total_cross_pairs as f64
    } else {
        1.0
    };

    // Step 3: Hallucinate dream glyphs from strongest clusters
    let hallucinations = if hallucinate && !cross_links.is_empty() {
        synthesize_dream_glyphs(&cross_links, glyphs, agent_id)
    } else {
        Vec::new()
    };

    CrossModalDreamResult {
        new_links: cross_links,
        hallucinations,
        carnot_efficiency,
    }
}

/// Count the total number of cross-modal pairs in a glyph set.
fn count_cross_modal_pairs(glyphs: &[Glyph]) -> usize {
    let mut count = 0;
    for i in 0..glyphs.len() {
        for j in (i + 1)..glyphs.len() {
            if source_type_tag(&glyphs[i].source) != source_type_tag(&glyphs[j].source) {
                count += 1;
            }
        }
    }
    count
}

/// Synthesize dream glyphs from the strongest cross-modal links.
///
/// Each hallucination merges the Fano projections of linked glyphs,
/// creating a new glyph that lives "between" modalities.
fn synthesize_dream_glyphs(
    links: &[GlyphLink],
    glyphs: &[Glyph],
    agent_id: &str,
) -> Vec<Glyph> {
    let now = Utc::now();
    let mut hallucinations = Vec::new();

    // Take up to 3 strongest links
    for link in links.iter().take(3) {
        let source = glyphs.iter().find(|g| g.glyph_id == link.source_glyph);
        let target = glyphs.iter().find(|g| g.glyph_id == link.target_glyph);

        if let (Some(s), Some(t)) = (source, target) {
            // Merge Fano projections (weighted average by amplitude)
            let total_amp = s.amplitude + t.amplitude;
            let ws = if total_amp > 1e-12 { s.amplitude / total_amp } else { 0.5 };
            let wt = 1.0 - ws;

            let mut fano = [0.0f64; 7];
            for i in 0..7 {
                fano[i] = s.fano[i] * ws + t.fano[i] * wt;
            }
            // Re-normalize
            let total: f64 = fano.iter().sum();
            if total > 1e-12 {
                for p in &mut fano {
                    *p /= total;
                }
            }

            // Average wave properties
            let amplitude = (s.amplitude + t.amplitude) / 2.0;
            let frequency = (s.frequency + t.frequency) / 2.0;
            let phase = (s.phase + t.phase) / 2.0;

            // SGA: use the average
            let sga_class = SgaClass {
                quadrant: ((s.sga_class.quadrant as u16 + t.sga_class.quadrant as u16) / 2) as u8,
                modality: 2, // Abstract — dream synthesis
                context: ((s.sga_class.context as u16 + t.sga_class.context as u16) / 2) as u8,
            };

            let parent_modalities = vec![
                source_type_tag(&s.source).to_string(),
                source_type_tag(&t.source).to_string(),
            ];

            let glyph_id = compute_glyph_id_from_fano(&fano, agent_id, "dream-hallucination");
            let mut salt = [0u8; 32];
            salt[..8].copy_from_slice(&glyph_id[..8]);

            hallucinations.push(Glyph {
                glyph_id,
                spec_version: GLYPH_SPEC_VERSION,
                fano,
                sga_class,
                sga_centroid: (sga_class.quadrant, sga_class.modality, sga_class.context),
                amplitude,
                frequency,
                phase,
                capsule: None,
                bloom: BloomParameters { difficulty: 0, salt },
                commitments: None,
                virtue_eta: None,
                gates: None,
                source: GlyphSource::Dream {
                    parent_modalities,
                    carnot_efficiency: link.similarity,
                },
                agent_id: agent_id.to_string(),
                created_at: now,
                parents: vec![s.glyph_id, t.glyph_id],
            });
        }
    }

    hallucinations
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collective::privacy::seal_with_commitments;

    fn test_memory(content: &str) -> HyperMemory {
        HyperMemory::new(vec![0.1f32; 100], content.to_string())
    }

    fn make_test_glyph(agent: &str, source: GlyphSource) -> Glyph {
        Glyph {
            glyph_id: [0u8; 32],
            spec_version: GLYPH_SPEC_VERSION,
            fano: [0.14, 0.14, 0.14, 0.14, 0.15, 0.15, 0.14],
            sga_class: SgaClass { quadrant: 0, modality: 0, context: 0 },
            sga_centroid: (0, 0, 0),
            amplitude: 0.8,
            frequency: 0.5,
            phase: 0.0,
            capsule: None,
            bloom: BloomParameters { difficulty: 0, salt: [0; 32] },
            commitments: None,
            virtue_eta: None,
            gates: None,
            source,
            agent_id: agent.to_string(),
            created_at: Utc::now(),
            parents: Vec::new(),
        }
    }

    #[test]
    fn test_wire_format_roundtrip() {
        let glyph = make_test_glyph("alice", GlyphSource::Memory {
            layer_depth: 2,
            hallucinated: false,
        });

        let wire = encode_wire(&glyph);
        assert!(wire.len() > 100);
        assert_eq!(&wire[0..4], b"GLYF");

        let decoded = decode_wire(&wire).expect("decode failed");
        assert_eq!(decoded.glyph_id, glyph.glyph_id);
        assert_eq!(decoded.spec_version, GLYPH_SPEC_VERSION);
        assert_eq!(decoded.sga_class, glyph.sga_class);
        assert!((decoded.amplitude - glyph.amplitude).abs() < 1e-10);
        assert!((decoded.frequency - glyph.frequency).abs() < 1e-10);
        assert!((decoded.phase - glyph.phase).abs() < 1e-10);
        assert_eq!(decoded.bloom.difficulty, glyph.bloom.difficulty);
        assert_eq!(decoded.agent_id, "alice");

        for i in 0..7 {
            assert!((decoded.fano[i] - glyph.fano[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_wire_format_with_virtue() {
        let mut glyph = make_test_glyph("bob", GlyphSource::Other {
            system: "test".to_string(),
            metadata: "{}".to_string(),
        });
        glyph.virtue_eta = Some(0.85);
        glyph.gates = Some([Some(true), Some(true), None]);

        let wire = encode_wire(&glyph);
        let decoded = decode_wire(&wire).unwrap();

        assert!((decoded.virtue_eta.unwrap() - 0.85).abs() < 1e-10);
        let gates = decoded.gates.unwrap();
        assert_eq!(gates[0], Some(true));
        assert_eq!(gates[1], Some(true));
        assert_eq!(gates[2], None);
    }

    #[test]
    fn test_wire_format_with_parents() {
        let mut glyph = make_test_glyph("alice", GlyphSource::Dream {
            parent_modalities: vec!["memory".to_string(), "audio".to_string()],
            carnot_efficiency: 0.92,
        });
        glyph.parents = vec![[1u8; 32], [2u8; 32]];

        let wire = encode_wire(&glyph);
        let decoded = decode_wire(&wire).unwrap();
        assert_eq!(decoded.parents.len(), 2);
        assert_eq!(decoded.parents[0], [1u8; 32]);
    }

    #[test]
    fn test_invalid_magic_rejected() {
        let mut wire = encode_wire(&make_test_glyph("x", GlyphSource::Memory {
            layer_depth: 0, hallucinated: false,
        }));
        wire[0] = b'X';
        assert!(decode_wire(&wire).is_err());
    }

    #[test]
    fn test_too_short_rejected() {
        assert!(decode_wire(&[0, 1, 2]).is_err());
    }

    #[test]
    fn test_glyph_similarity_identical() {
        let g = make_test_glyph("a", GlyphSource::Memory { layer_depth: 0, hallucinated: false });
        let sim = glyph_similarity(&g, &g);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_glyph_similarity_symmetric() {
        let mut g1 = make_test_glyph("a", GlyphSource::Memory { layer_depth: 0, hallucinated: false });
        let mut g2 = make_test_glyph("b", GlyphSource::Audio {
            duration_ms: 1000, sample_rate: 44100, spectral_centroid: 440.0, overtone_hz: 880.0,
        });
        g1.fano = [0.2, 0.1, 0.1, 0.2, 0.1, 0.2, 0.1];
        g2.fano = [0.1, 0.2, 0.2, 0.1, 0.2, 0.1, 0.1];

        assert!((glyph_similarity(&g1, &g2) - glyph_similarity(&g2, &g1)).abs() < 1e-10);
    }

    #[test]
    fn test_glyph_similarity_cross_modal() {
        let mut g_mem = make_test_glyph("a", GlyphSource::Memory { layer_depth: 0, hallucinated: false });
        let mut g_audio = make_test_glyph("b", GlyphSource::Audio {
            duration_ms: 1000, sample_rate: 44100, spectral_centroid: 440.0, overtone_hz: 880.0,
        });

        // Same Fano → high similarity regardless of modality
        g_mem.fano = [0.14, 0.14, 0.15, 0.14, 0.14, 0.15, 0.14];
        g_audio.fano = [0.14, 0.14, 0.15, 0.14, 0.14, 0.15, 0.14];

        let sim = glyph_similarity(&g_mem, &g_audio);
        assert!(sim > 0.9, "cross-modal similarity should be high for matching Fano: {}", sim);
    }

    #[test]
    fn test_sga_class_roundtrip() {
        for idx in 0..84u8 {
            let cls = SgaClass::from_class_index(idx);
            assert_eq!(cls.to_class_index(), idx, "SGA class index roundtrip failed for {}", idx);
        }
    }

    #[test]
    fn test_sga_class_from_memory_coords() {
        let mc = MemoryCoordinates {
            h2: 2, d: 1, l: 5,
            class_index: 0, amplitude: 0.5, phase: 0.0,
        };
        let cls = SgaClass::from_memory_coords(&mc);
        assert_eq!(cls.quadrant, 2);
        assert_eq!(cls.modality, 1);
        assert_eq!(cls.context, 5);
    }

    #[test]
    fn test_memory_to_glyph() {
        let mem = test_memory("quantum computing research");
        let glyph = memory_to_glyph(&mem, 8, "kannaka-01");

        assert_eq!(glyph.spec_version, GLYPH_SPEC_VERSION);
        assert!((glyph.amplitude - mem.amplitude as f64).abs() < 1e-6);
        assert_eq!(glyph.bloom.difficulty, 8);
        assert_eq!(glyph.agent_id, "kannaka-01");

        // Fano should be normalized
        let fano_sum: f64 = glyph.fano.iter().sum();
        assert!((fano_sum - 1.0).abs() < 1e-6, "Fano not normalized: sum={}", fano_sum);

        match &glyph.source {
            GlyphSource::Memory { layer_depth, hallucinated } => {
                assert_eq!(*layer_depth, 0);
                assert!(!hallucinated);
            }
            _ => panic!("Expected Memory source"),
        }
    }

    #[test]
    fn test_privacy_glyph_to_universal_glyph() {
        let mem = test_memory("secret document");
        let result = seal_with_commitments(&mem, 32, "agent-1");
        let pg = result.glyph;

        let glyph = privacy_glyph_to_glyph(&pg);
        assert_eq!(glyph.bloom.difficulty, 32);
        assert_eq!(glyph.agent_id, "agent-1");
        assert!(glyph.capsule.is_some());
        assert!(glyph.commitments.is_some());
        assert!((glyph.amplitude - pg.committed_amplitude).abs() < 1e-10);
    }

    #[test]
    fn test_discover_links_cross_modal() {
        let mut g_mem = make_test_glyph("a", GlyphSource::Memory { layer_depth: 0, hallucinated: false });
        let mut g_audio = make_test_glyph("b", GlyphSource::Audio {
            duration_ms: 1000, sample_rate: 44100, spectral_centroid: 440.0, overtone_hz: 880.0,
        });

        // Make them geometrically similar
        g_mem.fano = [0.14, 0.14, 0.15, 0.14, 0.14, 0.15, 0.14];
        g_mem.glyph_id = [1u8; 32];
        g_audio.fano = [0.14, 0.14, 0.15, 0.14, 0.14, 0.15, 0.14];
        g_audio.glyph_id = [2u8; 32];

        let links = discover_links(&[g_mem, g_audio], 0.5, "dream");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].link_type, GlyphLinkType::CrossModal);
    }

    #[test]
    fn test_discover_links_threshold() {
        let mut g1 = make_test_glyph("a", GlyphSource::Memory { layer_depth: 0, hallucinated: false });
        let mut g2 = make_test_glyph("b", GlyphSource::Memory { layer_depth: 1, hallucinated: false });

        // Make them very different
        g1.fano = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        g1.phase = 0.0;
        g1.glyph_id = [1u8; 32];
        g2.fano = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        g2.phase = std::f64::consts::PI;
        g2.glyph_id = [2u8; 32];

        let links = discover_links(&[g1, g2], 0.8, "search");
        assert_eq!(links.len(), 0, "Dissimilar glyphs should not link");
    }

    #[test]
    fn test_fano_from_vector_normalized() {
        let vec = vec![0.1f32; 700];
        let fano = compute_fano_from_vector_f32(&vec);
        let sum: f64 = fano.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "Fano should be normalized, got sum={}", sum);
    }

    #[test]
    fn test_fano_from_empty_vector() {
        let fano = compute_fano_from_vector_f32(&[]);
        assert_eq!(fano, [0.0; 7]);
    }

    // ── Phase 3: Perception Adapters ──

    #[test]
    fn test_fano_from_mfcc_normalized() {
        let mfcc = vec![0.5; 70]; // 70 MFCC coefficients
        let fano = compute_fano_from_mfcc(&mfcc);
        let sum: f64 = fano.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "MFCC Fano should be normalized, got {}", sum);
    }

    #[test]
    fn test_fano_from_mfcc_empty() {
        let fano = compute_fano_from_mfcc(&[]);
        assert_eq!(fano, [0.0; 7]);
    }

    #[test]
    fn test_audio_to_glyph() {
        let mfcc = vec![0.3; 42];
        let glyph = audio_to_glyph(&mfcc, 5000, 44100, 440.0, 880.0, 0.8, 0.0, 4, "radio-01");
        assert_eq!(glyph.spec_version, GLYPH_SPEC_VERSION);
        assert_eq!(glyph.sga_class.modality, 1); // sensory
        match &glyph.source {
            GlyphSource::Audio { duration_ms, sample_rate, .. } => {
                assert_eq!(*duration_ms, 5000);
                assert_eq!(*sample_rate, 44100);
            }
            _ => panic!("Expected Audio source"),
        }
    }

    #[test]
    fn test_fano_from_visual_normalized() {
        let features = vec![0.2; 56];
        let fano = compute_fano_from_visual(&features);
        let sum: f64 = fano.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "Visual Fano should be normalized, got {}", sum);
    }

    #[test]
    fn test_visual_to_glyph() {
        let features = vec![0.1; 49];
        let glyph = visual_to_glyph(&features, 1920, 1080, 12, 0.7, 0.0, 0, "eye-01");
        assert_eq!(glyph.sga_class.modality, 1); // sensory
        match &glyph.source {
            GlyphSource::Visual { width, height, fold_count } => {
                assert_eq!(*width, 1920);
                assert_eq!(*height, 1080);
                assert_eq!(*fold_count, 12);
            }
            _ => panic!("Expected Visual source"),
        }
    }

    #[test]
    fn test_cross_modal_audio_text_similarity() {
        // Audio and text with similar Fano should have high similarity
        let mfcc = vec![0.14; 70];
        let audio_glyph = audio_to_glyph(&mfcc, 1000, 44100, 440.0, 880.0, 0.8, 0.0, 0, "radio");
        let text_glyph = make_test_glyph("mem", GlyphSource::Memory { layer_depth: 0, hallucinated: false });

        // Both should have uniform-ish Fano → decent similarity
        let sim = glyph_similarity(&audio_glyph, &text_glyph);
        assert!(sim > 0.3, "cross-modal similarity too low: {}", sim);
    }

    // ── Phase 4: SCADA Adapter ──

    #[test]
    fn test_fano_from_process_value_at_setpoint() {
        let fano = compute_fano_from_process_value(50.0, 50.0, 0.0, 100.0, 192);
        let sum: f64 = fano.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "SCADA Fano should be normalized");
        // At setpoint: deviation = 0, resonance should be high
        assert!(fano[3] > fano[1], "resonance should exceed deviation at setpoint");
    }

    #[test]
    fn test_fano_from_process_value_at_high_limit() {
        let fano = compute_fano_from_process_value(100.0, 50.0, 0.0, 100.0, 192);
        let sum: f64 = fano.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        // At high limit: high deviation, high connection to alarms
        assert!(fano[1] > 0.1, "deviation should be significant at limit");
    }

    #[test]
    fn test_scada_to_glyph() {
        let glyph = scada_to_glyph("TI-101", 75.0, "degC", 192, 50.0, 0.0, 100.0, "scada-01");
        assert_eq!(glyph.sga_class.quadrant, 2); // abstract/process
        assert_eq!(glyph.bloom.difficulty, 0); // SCADA public
        match &glyph.source {
            GlyphSource::Scada { tag, value, unit, quality } => {
                assert_eq!(tag, "TI-101");
                assert!((value - 75.0).abs() < 1e-10);
                assert_eq!(unit, "degC");
                assert_eq!(*quality, 192);
            }
            _ => panic!("Expected Scada source"),
        }
    }

    #[test]
    fn test_scada_amplitude_from_deviation() {
        let on_target = scada_to_glyph("TI-101", 50.0, "degC", 192, 50.0, 0.0, 100.0, "s");
        let off_target = scada_to_glyph("TI-101", 95.0, "degC", 192, 50.0, 0.0, 100.0, "s");
        assert!(off_target.amplitude > on_target.amplitude,
            "off-target should have higher amplitude: {} vs {}", off_target.amplitude, on_target.amplitude);
    }

    // ── Phase 6: Dream Cross-Modal Linking ──

    #[test]
    fn test_dream_cross_modal_no_glyphs() {
        let result = dream_cross_modal_link(&[], 0.5, false, "dream-agent");
        assert!(result.new_links.is_empty());
        assert!(result.hallucinations.is_empty());
        assert!((result.carnot_efficiency - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_dream_cross_modal_same_modality_no_links() {
        let mut g1 = make_test_glyph("a", GlyphSource::Memory { layer_depth: 0, hallucinated: false });
        let mut g2 = make_test_glyph("b", GlyphSource::Memory { layer_depth: 1, hallucinated: false });
        g1.glyph_id = [1u8; 32];
        g2.glyph_id = [2u8; 32];

        let result = dream_cross_modal_link(&[g1, g2], 0.0, false, "dream");
        assert!(result.new_links.is_empty(), "same modality should not create cross-modal links");
    }

    #[test]
    fn test_dream_cross_modal_finds_links() {
        let mut g_mem = make_test_glyph("a", GlyphSource::Memory { layer_depth: 0, hallucinated: false });
        let mut g_audio = make_test_glyph("b", GlyphSource::Audio {
            duration_ms: 1000, sample_rate: 44100, spectral_centroid: 440.0, overtone_hz: 880.0,
        });
        // Same Fano → high cross-modal similarity
        g_mem.fano = [0.14, 0.14, 0.15, 0.14, 0.14, 0.15, 0.14];
        g_mem.glyph_id = [1u8; 32];
        g_audio.fano = [0.14, 0.14, 0.15, 0.14, 0.14, 0.15, 0.14];
        g_audio.glyph_id = [2u8; 32];

        let result = dream_cross_modal_link(&[g_mem, g_audio], 0.5, false, "dream");
        assert_eq!(result.new_links.len(), 1);
        assert_eq!(result.new_links[0].link_type, GlyphLinkType::CrossModal);
    }

    #[test]
    fn test_dream_hallucination() {
        let mut g_mem = make_test_glyph("a", GlyphSource::Memory { layer_depth: 0, hallucinated: false });
        let mut g_audio = make_test_glyph("b", GlyphSource::Audio {
            duration_ms: 1000, sample_rate: 44100, spectral_centroid: 440.0, overtone_hz: 880.0,
        });
        g_mem.fano = [0.14, 0.14, 0.15, 0.14, 0.14, 0.15, 0.14];
        g_mem.glyph_id = [1u8; 32];
        g_audio.fano = [0.14, 0.14, 0.15, 0.14, 0.14, 0.15, 0.14];
        g_audio.glyph_id = [2u8; 32];

        let result = dream_cross_modal_link(&[g_mem, g_audio], 0.5, true, "dream");
        assert_eq!(result.hallucinations.len(), 1);

        let hall = &result.hallucinations[0];
        match &hall.source {
            GlyphSource::Dream { parent_modalities, .. } => {
                assert!(parent_modalities.contains(&"memory".to_string()));
                assert!(parent_modalities.contains(&"audio".to_string()));
            }
            _ => panic!("Expected Dream source"),
        }
        assert_eq!(hall.parents.len(), 2);
    }

    #[test]
    fn test_dream_hallucination_fano_is_blend() {
        let mut g_mem = make_test_glyph("a", GlyphSource::Memory { layer_depth: 0, hallucinated: false });
        let mut g_scada = make_test_glyph("b", GlyphSource::Scada {
            tag: "TI".to_string(), value: 50.0, unit: "C".to_string(), quality: 192,
        });
        g_mem.fano = [0.3, 0.1, 0.1, 0.1, 0.1, 0.2, 0.1];
        g_mem.glyph_id = [1u8; 32];
        g_mem.amplitude = 0.8;
        g_scada.fano = [0.1, 0.3, 0.1, 0.1, 0.2, 0.1, 0.1];
        g_scada.glyph_id = [2u8; 32];
        g_scada.amplitude = 0.8;

        let result = dream_cross_modal_link(&[g_mem, g_scada], 0.0, true, "dream");
        assert!(!result.hallucinations.is_empty());

        let hall = &result.hallucinations[0];
        // Fano should be normalized
        let sum: f64 = hall.fano.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "hallucinated Fano should be normalized: {}", sum);
    }
}
