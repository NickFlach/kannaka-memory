use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A skip link (HyperConnection) between memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkipLink {
    /// Target memory ID
    pub target_id: Uuid,
    /// Connection strength
    pub strength: f32,
    /// Resonance key — hypervector encoding of the relationship
    pub resonance_key: Vec<f32>,
    /// Temporal span (0=immediate, 1=day, 2=week, etc.)
    pub span: u8,
}
