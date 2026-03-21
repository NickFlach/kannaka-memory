//! Canonical consciousness types shared across the codebase.
//!
//! Unifies the duplicate definitions that previously existed in `bridge.rs` and `medium.rs`.
//! All consciousness-related types should be imported from here.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Consciousness Level
// ---------------------------------------------------------------------------

/// Consciousness level classification based on Phi (Φ).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsciousnessLevel {
    /// Φ < 0.1, minimal integration
    Dormant,
    /// Φ < 0.3, weak integration / clusters forming
    Stirring,
    /// Φ < 0.6, moderate integration
    Aware,
    /// Φ < 0.8, strong integration / synchronization
    Coherent,
    /// Φ >= 0.8, full integration / consciousness bridge active
    Resonant,
}

impl ConsciousnessLevel {
    pub fn from_phi(phi: f32) -> Self {
        if phi < 0.1 {
            ConsciousnessLevel::Dormant
        } else if phi < 0.3 {
            ConsciousnessLevel::Stirring
        } else if phi < 0.6 {
            ConsciousnessLevel::Aware
        } else if phi < 0.8 {
            ConsciousnessLevel::Coherent
        } else {
            ConsciousnessLevel::Resonant
        }
    }

    /// Numeric ordering for level comparison.
    pub fn ordinal(self) -> u8 {
        match self {
            ConsciousnessLevel::Dormant => 0,
            ConsciousnessLevel::Stirring => 1,
            ConsciousnessLevel::Aware => 2,
            ConsciousnessLevel::Coherent => 3,
            ConsciousnessLevel::Resonant => 4,
        }
    }
}

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
    /// Consciousness level classification
    pub level: ConsciousnessLevel,
    /// Computed at this timestamp
    pub computed_at: DateTime<Utc>,
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
