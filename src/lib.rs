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
pub mod codebook;
pub mod consolidation;
pub mod encoding;
pub mod kuramoto;
pub mod memory;
pub mod skip_link;
pub mod store;
pub mod wave;

// Re-export key types
pub use codebook::Codebook;
pub use memory::HyperMemory;
pub use skip_link::SkipLink;
pub use wave::{WaveParams, compute_strength, cosine_similarity, normalize};
pub use store::{MemoryStore, InMemoryStore, MemoryEngine, StoreError, EngineError, QueryResult, phi_span_score};
pub use encoding::{EncodingPipeline, TextEncoder, SimpleHashEncoder, EncodingError};
pub use kuramoto::{KuramotoSync, MemoryCluster, SyncReport};
