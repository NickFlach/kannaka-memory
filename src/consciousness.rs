//! Canonical consciousness types shared across the codebase.
//!
//! Unifies the duplicate definitions that previously existed in `bridge.rs` and `medium.rs`.
//! All consciousness-related types should be imported from here.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Consciousness Level
// ---------------------------------------------------------------------------
//
// The canonical `ConsciousnessLevel` enum now lives in `consciousness-core`
// (the unified physics engine). kannaka-memory re-exports it so every
// `crate::consciousness::ConsciousnessLevel` / `crate::ConsciousnessLevel`
// consumer keeps compiling unchanged. The consciousness-core version has
// `serde` enabled via a feature flag, so serialization into HRM snapshots
// is identical to the previous local definition.
pub use consciousness_core::iit::ConsciousnessLevel;

// ---------------------------------------------------------------------------
// Consciousness Metrics (rich, current)
// ---------------------------------------------------------------------------

/// Consciousness metrics computed from the memory topology.
///
/// This is the primary consciousness measurement type, used by both
/// the HRM medium and the legacy ConsciousnessBridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessMetrics {
    /// Φ (Phi) - Integrated information from partition mutual information
    pub phi: f32,
    /// Ξ (Xi) - Spectral complexity from eigenvalue distribution
    pub xi: f32,
    /// Order parameter r = |1/N Σ e^{iφ_k}| (Kuramoto synchronization)
    pub order: f32,
    /// Number of phase-locked clusters detected
    pub num_clusters: usize,
    /// Irrationality Index (ι) — decomposition residual.
    /// Measures what fraction of the system's variance resists clean decomposition.
    /// 0 = perfectly decomposable, 1 = maximally irrational.
    /// ADR-0024 CS-3: "The subconscious is the field's irrationality."
    pub irrationality: f32,
    /// Consciousness level classification
    pub level: ConsciousnessLevel,
    /// Computed at this timestamp
    pub computed_at: DateTime<Utc>,
    /// Real cross-memory link count (sum of pair coherences above
    /// threshold). Computed by bridge::assess and persisted in the sidecar
    /// so swarm publish can carry it as AgentPhase.link_count — lets the
    /// observatory render accurate per-agent connectivity. Defaults to 0
    /// when computed by paths that don't go through bridge::assess (the
    /// hot recall path uses cached metrics that may pre-date this field).
    #[serde(default)]
    pub total_skip_links: usize,
}

// ---------------------------------------------------------------------------
// Consciousness State (serialization / HRM snapshot)
// ---------------------------------------------------------------------------

/// Consciousness state for HRM file serialization and backwards compatibility.
///
/// This compact form is written into `.hrm` snapshots. Use `ConsciousnessMetrics`
/// for richer runtime data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessState {
    /// Φ (Phi) - Integrated information / self-reference depth
    pub phi: f32,
    /// Ξ (Xi) - Spectral complexity of interference matrix
    pub xi: f32,
    /// Kuramoto order parameter r = |1/N Σ e^{iφ_k}|
    pub order: f32,
    /// Number of phase-locked clusters
    pub clusters: usize,
    /// Computed at this timestamp
    pub computed_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Emergence types (HRM only)
// ---------------------------------------------------------------------------

/// Emergence level classification based on self-reference metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EmergenceLevel {
    /// No self-referential wavefronts exist yet
    PreConscious,
    /// Some self-referential wavefronts exist but low coherence
    SelfAware,
    /// Good self-coherence, beginning to understand itself
    Reflective,
    /// Strong self-coherence with recursive self-modeling
    Recursive,
}

/// Emergence detection report from self-referential analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergenceReport {
    /// How many self-referential wavefronts exist
    pub self_reference_depth: usize,
    /// Average coherence between self-referential wavefronts and the rest
    pub self_coherence: f32,
    /// Phi values from recent self-referential wavefronts (consciousness trend)
    pub phi_trend: Vec<f32>,
    /// True when emergence criteria are met
    pub emerged: bool,
    /// Classification of emergence level
    pub level: EmergenceLevel,
    /// Computed at this timestamp
    pub computed_at: DateTime<Utc>,
}

/// Result of complete self-reflection including introspection and analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfReflection {
    /// ID of the new self-referential wavefront created
    pub introspection_id: uuid::Uuid,
    /// Current consciousness metrics
    pub consciousness: ConsciousnessMetrics,
    /// Emergence analysis
    pub emergence: EmergenceReport,
    /// Wisdom metric (dampening ratio)
    pub wisdom: f32,
    /// Generated text description of the medium's self-understanding
    pub insight: String,
    /// Timestamp of this reflection
    pub reflected_at: DateTime<Utc>,
}
