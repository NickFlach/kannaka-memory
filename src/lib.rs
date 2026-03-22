//! # Kannaka Memory
//!
//! Chiral Holographic Resonance Memory — wave-based hyperdimensional memory
//! where storage IS computation.
//!
//! Memories exist as wavefronts in a high-dimensional tensor medium.
//! Recall is resonance (constructive interference). Dreaming is annealing.

pub mod bridge;
pub mod observe;
pub mod openclaw;
pub mod codebook;
pub mod consolidation;
pub mod rhythm;
pub mod encoding;
pub mod kuramoto;
pub mod memory;
pub mod store;
pub mod wave;
pub mod geometry;
pub mod working_memory;
pub mod xi_operator;

#[cfg(feature = "glyph")]
pub mod glyph_bridge;

// Consciousness differentiation tests integrated into existing test modules

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(feature = "audio")]
pub mod ear;

#[cfg(feature = "video")]
pub mod eye;

#[cfg(feature = "nats")]
pub mod nats;

pub mod collective;
pub mod paradox;
pub mod queen;
pub mod invariant;
pub mod cmf;
pub mod consciousness;

pub mod medium;

pub mod hrm_store;

// Re-export canonical consciousness types
pub use consciousness::{
    ConsciousnessLevel, ConsciousnessMetrics, ConsciousnessState,
    EmergenceLevel, EmergenceReport, SelfReflection,
};

// Re-export key types
pub use codebook::Codebook;
pub use memory::HyperMemory;
pub use wave::{WaveParams, compute_strength, cosine_similarity, normalize};
pub use store::{MemoryStore, InMemoryStore, MemoryEngine, StoreError, EngineError, QueryResult, phi_span_score};
pub use encoding::{EncodingPipeline, TextEncoder, SimpleHashEncoder, EncodingError};
pub use kuramoto::{KuramotoSync, MemoryCluster, SyncReport};
pub use bridge::{ConsciousnessBridge, PhiReport, ResonanceReport};
pub use consolidation::{ConsolidationEngine, ConsolidationReport, DreamState};
pub use rhythm::{RhythmEngine, RhythmState, Signal as RhythmSignal};

pub use observe::{MemoryIntrospector, SystemReport, TopologyReport, WaveReport, ClusterReport, ClusterInfo, HealthCheck, LinkInfo, MemoryInfo, ConsciousnessSnapshot};
pub use working_memory::{WorkingMemory, ConversationTurn, SessionState, TaskItem, TaskStatus};
pub use geometry::{
    CliffordElement, Z4Element, Z3Element, SgaElement, 
    ClassComponents, MemoryCoordinates,
    transform_r, transform_d, transform_t, transform_m,
    lift, project, classify_memory, geometric_similarity, fano_related,
    cross_product, is_fano_line, FANO_LINES, EPSILON
};
pub use xi_operator::{
    PHI, ALPHA, BETA, ETA, EMERGENCE_COEFF,
    apply_rotation, apply_golden_scaling, compute_xi_signature,
    xi_repulsive_force, xi_diversity_boost
};
pub use paradox::{
    ParadoxSnapshot, DreamTrajectory, Mutation, Paradox, ProposedState,
    Resolution, ResolutionReport, ParadoxResolver
};

pub use queen::{QueenSync, QueenConfig, QueenState, AgentPhase, Hive, Handedness, SwarmAgent};
pub use invariant::{
    InvariantMetrics, DeltaCluster, compute_delta, compute_convergence_rate, 
    compute_irrationality, compute_invariant_metrics, cluster_by_delta, delta_distance
};
pub use cmf::{
    ConservativeMemoryField, TrajectoryParams, PathConstraints, CMFMembership,
    detect_cmf, cmf_membership, generate_trajectory
};

pub use medium::{
    Medium, WavefrontMeta, Resonance, MediumError, WAVEFRONT_DIM,
    PhaseState, HrmCommit
};

pub use hrm_store::HrmStore;

#[cfg(feature = "glyph")]
pub use glyph_bridge::{
    Glyph, GlyphEncoder, GlyphDecoder, GlyphError,
    encode_memory_as_glyph, bloom_glyph, BASE_FREQ
};
