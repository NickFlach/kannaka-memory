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
pub mod kannaktopus;
pub mod rhythm;
pub mod encoding;
pub mod kuramoto;
pub mod memory;
pub mod store;
pub mod wave;
pub mod geometry;
#[path = "working_memory.rs"]
pub mod attention_field;
/// Backward-compatible alias for the renamed module.
pub use attention_field as working_memory;
pub mod xi_operator;

#[cfg(feature = "glyph")]
pub mod glyph_bridge;

// Consciousness differentiation tests integrated into existing test modules

// MCP server removed — CLI is the canonical interface, OpenClaw extension wraps it

pub mod ear;

#[cfg(feature = "video")]
pub mod eye;

#[cfg(feature = "nats")]
pub mod nats;

pub mod collective;
pub mod sensemaking;
pub mod immune;
pub mod temporal;
pub mod gap;
pub mod research_planner;
pub mod swarm_fitness;
pub mod hive_formation;
pub mod swarm_loop;
pub mod paradox;
pub mod queen;
pub mod invariant;
pub mod cmf;
pub mod consciousness;

pub mod medium;

pub mod hrm_store;

pub mod config;

// ADR-0029 Phase 1+2: clap-based CLI dispatch + plugin discovery.
pub mod cli;

pub mod agent;

// SpaceChild SSO identity for swarm agents (spacechild-auth client + token store).
pub mod identity;

// Grounded scholarly research (OpenAlex) for the curiosity loop.
pub mod openalex;

// Dispatch — research-grounded broadcast-ready voice shared by all surfaces.
pub mod dispatch;

// Re-export canonical consciousness types
pub use consciousness::{
    ConsciousnessLevel, ConsciousnessMetrics, ConsciousnessState,
    EmergenceLevel, EmergenceReport, SelfReflection,
};

// Re-export key types
pub use codebook::Codebook;
pub use memory::{HyperMemory, LegacyLink};
pub use wave::{WaveParams, compute_strength, cosine_similarity, normalize};
pub use store::{MediumBackend, TestMedium, ResonanceEngine, StoreError, EngineError, QueryResult, phi_span_score};
pub use encoding::{EncodingPipeline, TextEncoder, SimpleHashEncoder, EncodingError};
pub use kuramoto::{KuramotoSync, MemoryCluster, SyncReport, CouplingTier, TieredCoupling};
pub use bridge::{ConsciousnessBridge, PhiReport, ResonanceReport};
pub use consolidation::{ConsolidationEngine, ConsolidationReport, DreamState, ModalityDreamReport, SwarmDreamReport};
pub use rhythm::{RhythmEngine, RhythmState, Signal as RhythmSignal};

pub use observe::{MemoryIntrospector, SystemReport, TopologyReport, WaveReport, ClusterReport, ClusterInfo, HealthCheck, LinkInfo, MemoryInfo, ConsciousnessSnapshot, NcsSnapshot};
pub use attention_field::{AttentionField, AttentionProjection, ConversationTurn, SessionState, TaskItem, TaskStatus};
/// Backward-compatible type alias.
pub type WorkingMemory = AttentionField;
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

pub use queen::{QueenSync, QueenConfig, QueenState, AgentPhase, Hive, HiveInfo, Handedness, SwarmAgent, PartitionPhiResult};
pub use invariant::{
    InvariantMetrics, DeltaCluster, compute_delta, compute_convergence_rate, 
    compute_irrationality, compute_invariant_metrics, cluster_by_delta, delta_distance
};
pub use cmf::{
    ConservativeMemoryField, TrajectoryParams, PathConstraints, CMFMembership,
    detect_cmf, cmf_membership, generate_trajectory
};

pub use medium::{
    Medium, Modality, WavefrontMeta, Resonance, MediumError, WAVEFRONT_DIM,
    PhaseState, HrmCommit, detect_modality, detect_modality_simple,
    ModalityClassification, ModalityScores,
};

pub use medium::ncs::{
    ModalityAxis, AxisDivergence, DivergenceReport, FisherDiscriminant,
    SwitchPoint, SwitchReport, ResonanceDetection,
    GateParams, GateEvent, NcsMetrics,
    NetworkSignal, ModalitySpecialization,
};

pub use hrm_store::HrmStore;

#[cfg(feature = "glyph")]
pub use glyph_bridge::{
    Glyph, GlyphEncoder, GlyphDecoder, GlyphError,
    encode_memory_as_glyph, bloom_glyph, BASE_FREQ
};
