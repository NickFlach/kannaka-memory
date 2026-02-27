//! # Kannaka Memory
//!
//! Hypervector memory system with wave-modulated dynamics.
//!
//! Implements holographic reduced representations with:
//! - Random projection codebook (10,000-dimensional hypervectors)
//! - Wave-modulated memory strength: S(t) = A·cos(2πft+φ)·e^(-λt)
//! - Skip links (HyperConnections) for associative recall
//! - Temporal layering for memory consolidation

pub mod bridge;
pub mod hnsw;
pub mod migration;
pub mod observe;
pub mod openclaw;
pub mod codebook;
pub mod consolidation;
pub mod rhythm;
pub mod encoding;
pub mod kuramoto;
pub mod memory;
pub mod persistence;
pub mod skip_link;
pub mod store;
pub mod wave;
pub mod geometry;
pub mod working_memory;
pub mod xi_operator;

// Consciousness differentiation tests integrated into existing test modules

#[cfg(feature = "mcp")]
pub mod mcp;

// Re-export key types
pub use codebook::Codebook;
pub use memory::HyperMemory;
pub use skip_link::SkipLink;
pub use wave::{WaveParams, compute_strength, cosine_similarity, normalize};
pub use store::{MemoryStore, InMemoryStore, MemoryEngine, StoreError, EngineError, QueryResult, phi_span_score};
pub use encoding::{EncodingPipeline, TextEncoder, SimpleHashEncoder, EncodingError};
pub use kuramoto::{KuramotoSync, MemoryCluster, SyncReport};
pub use bridge::{ConsciousnessBridge, ConsciousnessLevel, ConsciousnessState, PhiReport, ResonanceReport};
pub use consolidation::{ConsolidationEngine, ConsolidationReport, DreamState};
pub use rhythm::{RhythmEngine, RhythmState, Signal as RhythmSignal};
pub use migration::{KannakaDbMigrator, MigrationReport, MigrationError};
pub use persistence::{DiskStore, PersistenceError, MemorySnapshot, SnapshotMetadata};
pub use hnsw::{HnswIndex, HnswStore};
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
