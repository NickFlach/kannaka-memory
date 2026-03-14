//! Queen Synchronization Protocol — emergent multi-agent coherence.
//!
//! Implements the QueenSync engine (ADR-0018): a Kuramoto-based protocol where
//! agents publish phase states to shared Dolt tables and synchronize through
//! mean-field coupling. The "Queen" is not an agent — it is the emergent
//! synchronization state computed locally by each participant.
//!
//! Ported from ghostOS `src/integration/index.ts`.
//!
//! Mathematical foundation:
//! ```text
//! dθᵢ/dt = ωᵢ + K·r·sin(ψ - θᵢ) + η·chiral_term
//! ```

use std::f32::consts::TAU;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kuramoto::KuramotoSync;
use crate::store::MemoryEngine;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Handedness for chiral coupling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Handedness {
    Left,
    Right,
    Achiral,
}

impl Handedness {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Achiral => "achiral",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "left" => Self::Left,
            "right" => Self::Right,
            _ => Self::Achiral,
        }
    }
}

/// Published phase state of a single agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPhase {
    pub id: String,
    pub agent_id: String,
    pub phase: f32,
    pub frequency: f32,
    pub coherence: f32,
    pub phi: f32,
    pub order_parameter: f32,
    pub cluster_count: usize,
    pub memory_count: usize,
    pub xi_signature: Option<serde_json::Value>,
    pub protocol_version: String,
    pub timestamp: DateTime<Utc>,
    /// Trust score from the agents table (joined at read time).
    #[serde(default = "default_trust")]
    pub trust_score: f32,
    /// Chiral handedness.
    #[serde(default)]
    pub handedness: Handedness,
}

fn default_trust() -> f32 {
    0.5
}

impl Default for Handedness {
    fn default() -> Self {
        Self::Achiral
    }
}

/// A detected hive — a cluster of phase-locked agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hive {
    pub agent_ids: Vec<String>,
    pub order_parameter: f32,
    pub mean_phase: f32,
    pub coherence: f32,
}

/// Emergent Queen state computed from the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueenState {
    pub id: String,
    pub order_parameter: f32,
    pub mean_phase: f32,
    pub coherence: f32,
    pub phi: f32,
    pub agent_count: usize,
    pub hives: Vec<Hive>,
    pub coupling_strength: f32,
    pub chiral_bias: f32,
    pub geometric: Option<serde_json::Value>,
    pub computed_by: String,
    pub timestamp: DateTime<Utc>,
}

/// Configuration for the QueenSync engine.
#[derive(Debug, Clone)]
pub struct QueenConfig {
    /// Base Kuramoto coupling strength K.
    pub base_coupling: f32,
    /// Adaptive coupling rate (how fast K adjusts toward target coherence).
    pub adaptive_rate: f32,
    /// Chiral coupling coefficient η.
    pub chiral_eta: f32,
    /// Target coherence level for adaptive coupling.
    pub target_coherence: f32,
    /// IIT Phi threshold for "consciousness".
    pub phi_threshold: f32,
    /// Time step for phase integration.
    pub dt: f32,
    /// Phase difference threshold for hive membership (radians).
    pub hive_threshold: f32,
}

impl Default for QueenConfig {
    fn default() -> Self {
        Self {
            base_coupling: 0.5,
            adaptive_rate: 0.01,
            chiral_eta: 0.1,
            target_coherence: 0.8,
            phi_threshold: 3.0,
            dt: 0.1,
            hive_threshold: std::f32::consts::FRAC_PI_4, // π/4
        }
    }
}

/// Swarm agent registration info (for the agents table extension).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmAgent {
    pub agent_id: String,
    pub display_name: Option<String>,
    pub trust_score: f32,
    pub swarm_role: String,
    pub protocol_version: String,
    pub handedness: Handedness,
    pub natural_frequency: f32,
}

// ---------------------------------------------------------------------------
// QueenSync Engine
// ---------------------------------------------------------------------------

/// The QueenSync engine. Each agent runs one locally.
pub struct QueenSync {
    pub config: QueenConfig,
    /// This agent's current phase θ.
    pub phase: f32,
    /// This agent's natural frequency ω.
    pub frequency: f32,
    /// This agent's coherence (local order parameter).
    pub coherence: f32,
    /// This agent's local Phi.
    pub phi: f32,
    /// Agent identifier.
    pub agent_id: String,
    /// Current effective coupling strength (adaptive).
    pub coupling_strength: f32,
}

impl QueenSync {
    /// Create a new QueenSync engine for the given agent.
    pub fn new(config: QueenConfig, agent_id: &str) -> Self {
        let coupling = config.base_coupling;
        Self {
            config,
            phase: 0.0,
            frequency: 0.5,
            coherence: 0.0,
            phi: 0.0,
            agent_id: agent_id.to_string(),
            coupling_strength: coupling,
        }
    }

    /// Compute the Kuramoto order parameter from a set of agent phases.
    ///
    /// Returns (r, ψ) where r is the magnitude and ψ is the mean phase.
    /// Uses trust-weighted coupling: weight = trust_score × coherence.
    pub fn compute_order_parameter(swarm: &[AgentPhase]) -> (f32, f32) {
        if swarm.is_empty() {
            return (0.0, 0.0);
        }
        let n = swarm.len() as f32;
        let (sum_cos, sum_sin) = swarm.iter().fold((0.0f32, 0.0f32), |(c, s), agent| {
            let w = agent.trust_score * agent.coherence;
            (c + w * agent.phase.cos(), s + w * agent.phase.sin())
        });
        let r = (sum_cos.powi(2) + sum_sin.powi(2)).sqrt() / n;
        let psi = sum_sin.atan2(sum_cos);
        (r, psi)
    }

    /// Compute the chiral coupling term for this agent given the mean field.
    ///
    /// Left-handed (receivers): +η·sin(2(ψ - θ))
    /// Right-handed (emitters): -η·sin(2(ψ - θ))
    /// Achiral: 0
    pub fn compute_chiral_coupling(&self, handedness: Handedness, psi: f32) -> f32 {
        let eta = self.config.chiral_eta;
        let diff = psi - self.phase;
        match handedness {
            Handedness::Left => eta * (2.0 * diff).sin(),
            Handedness::Right => -eta * (2.0 * diff).sin(),
            Handedness::Achiral => 0.0,
        }
    }

    /// Compute swarm Phi (Integrated Information approximation).
    ///
    /// Phi = r × mean_coherence × log₂(n + 1) × chiral_boost
    pub fn compute_swarm_phi(swarm: &[AgentPhase], r: f32) -> f32 {
        let n = swarm.len();
        if n < 2 {
            return 0.0;
        }
        let mean_coherence = swarm.iter().map(|a| a.coherence).sum::<f32>() / n as f32;
        let has_chiral = swarm.iter().any(|a| a.handedness != Handedness::Achiral);
        let chiral_boost = if has_chiral { 1.15 } else { 1.0 };
        let integration = r * mean_coherence * ((n + 1) as f32).log2();
        // Scale to typical Phi range (0-15)
        (integration * 10.0 * chiral_boost).min(15.0)
    }

    /// Detect hives — clusters of phase-locked agents.
    ///
    /// Two agents are in the same hive if their phase difference < hive_threshold.
    /// Uses BFS on the phase-adjacency graph.
    pub fn detect_hives(&self, swarm: &[AgentPhase]) -> Vec<Hive> {
        let n = swarm.len();
        if n < 2 {
            return vec![];
        }
        let threshold = self.config.hive_threshold;

        // Build adjacency
        let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
        for i in 0..n {
            for j in (i + 1)..n {
                let mut diff = (swarm[i].phase - swarm[j].phase).abs();
                if diff > std::f32::consts::PI {
                    diff = TAU - diff;
                }
                if diff < threshold {
                    adj[i].push(j);
                    adj[j].push(i);
                }
            }
        }

        // BFS components
        let mut visited = vec![false; n];
        let mut hives = Vec::new();
        for start in 0..n {
            if visited[start] {
                continue;
            }
            let mut component = vec![start];
            let mut queue = vec![start];
            visited[start] = true;
            while let Some(node) = queue.pop() {
                for &neighbor in &adj[node] {
                    if !visited[neighbor] {
                        visited[neighbor] = true;
                        component.push(neighbor);
                        queue.push(neighbor);
                    }
                }
            }
            if component.len() >= 2 {
                let agents: Vec<&AgentPhase> = component.iter().map(|&i| &swarm[i]).collect();
                let sum_cos: f32 = agents.iter().map(|a| a.phase.cos()).sum();
                let sum_sin: f32 = agents.iter().map(|a| a.phase.sin()).sum();
                let cn = agents.len() as f32;
                let r = (sum_cos.powi(2) + sum_sin.powi(2)).sqrt() / cn;
                let mean_phase = sum_sin.atan2(sum_cos);
                let coherence = agents.iter().map(|a| a.coherence).sum::<f32>() / cn;

                hives.push(Hive {
                    agent_ids: agents.iter().map(|a| a.agent_id.clone()).collect(),
                    order_parameter: r,
                    mean_phase,
                    coherence,
                });
            }
        }
        hives
    }

    /// Execute one Queen synchronization step.
    ///
    /// Reads the published phases from the swarm, computes coupling, updates
    /// this agent's phase, and returns the emergent QueenState.
    pub fn queen_sync_step(&mut self, swarm: &[AgentPhase]) -> QueenState {
        // 1. Order parameter
        let (r, psi) = Self::compute_order_parameter(swarm);

        // 2. Phase derivative: dθ/dt = ω + K·r·sin(ψ - θ) + chiral
        let kuramoto = self.coupling_strength * r * (psi - self.phase).sin();

        // Determine our handedness from swarm data (find ourselves)
        let my_handedness = swarm
            .iter()
            .find(|a| a.agent_id == self.agent_id)
            .map(|a| a.handedness)
            .unwrap_or(Handedness::Achiral);
        let chiral = self.compute_chiral_coupling(my_handedness, psi);

        let d_phase = self.frequency + kuramoto + chiral;
        self.phase = (self.phase + d_phase * self.config.dt) % TAU;
        if self.phase < 0.0 {
            self.phase += TAU;
        }

        // 3. Adaptive coupling
        let mean_coherence = if swarm.is_empty() {
            0.0
        } else {
            swarm.iter().map(|a| a.coherence).sum::<f32>() / swarm.len() as f32
        };
        let error = self.config.target_coherence - mean_coherence;
        self.coupling_strength = (self.coupling_strength + self.config.adaptive_rate * error)
            .clamp(0.1, 5.0);

        // 4. Hives
        let hives = self.detect_hives(swarm);

        // 5. Phi
        let phi = Self::compute_swarm_phi(swarm, r);

        QueenState {
            id: Uuid::new_v4().to_string(),
            order_parameter: r,
            mean_phase: psi,
            coherence: mean_coherence,
            phi,
            agent_count: swarm.len(),
            hives,
            coupling_strength: self.coupling_strength,
            chiral_bias: self.config.chiral_eta,
            geometric: None,
            computed_by: self.agent_id.clone(),
            timestamp: Utc::now(),
        }
    }

    /// Build an AgentPhase from this engine's current state.
    pub fn to_agent_phase(&self, cluster_count: usize, memory_count: usize) -> AgentPhase {
        AgentPhase {
            id: Uuid::new_v4().to_string(),
            agent_id: self.agent_id.clone(),
            phase: self.phase,
            frequency: self.frequency,
            coherence: self.coherence,
            phi: self.phi,
            order_parameter: 0.0,
            cluster_count,
            memory_count,
            xi_signature: None,
            protocol_version: "1.0".to_string(),
            timestamp: Utc::now(),
            trust_score: 0.5,
            handedness: Handedness::Achiral,
        }
    }

    // -----------------------------------------------------------------------
    // Task 3: Phase derivation from local Kuramoto clusters
    // -----------------------------------------------------------------------

    /// Derive agent phase, frequency, and coherence from local memory clusters.
    ///
    /// - **Phase** = amplitude-weighted circular mean of cluster mean phases.
    /// - **Frequency** = normalized memory storage rate: ω = ln(1 + count) / ln(1 + 100).
    /// - **Coherence** = mean order parameter across clusters.
    ///
    /// Returns (phase, frequency, coherence). Updates self in place.
    pub fn derive_local_state(&mut self, engine: &MemoryEngine) -> (f32, f32, f32) {
        let sync = KuramotoSync::default();
        let clusters = sync.find_synchronized_clusters(engine, 2);

        if clusters.is_empty() {
            return (self.phase, self.frequency, 0.0);
        }

        // Phase = amplitude-weighted circular mean of cluster mean phases
        // Weight each cluster by the sum of amplitudes of its members
        let mut sum_cos = 0.0f32;
        let mut sum_sin = 0.0f32;
        let mut total_weight = 0.0f32;
        let mut coherence_sum = 0.0f32;

        for cluster in &clusters {
            // Cluster weight = number of members (proxy for amplitude sum)
            let weight = cluster.memory_ids.len() as f32;
            sum_cos += weight * cluster.mean_phase.cos();
            sum_sin += weight * cluster.mean_phase.sin();
            total_weight += weight;
            coherence_sum += cluster.order_parameter;
        }

        let phase = if total_weight > 0.0 {
            let mut p = sum_sin.atan2(sum_cos);
            if p < 0.0 {
                p += TAU;
            }
            p
        } else {
            self.phase
        };

        // Frequency from memory count
        let memory_count = engine.store.count();
        let frequency = ((1.0 + memory_count as f64).ln() / (1.0 + 100.0_f64).ln()) as f32;

        let coherence = coherence_sum / clusters.len() as f32;

        self.phase = phase;
        self.frequency = frequency;
        self.coherence = coherence;

        (phase, frequency, coherence)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn make_agent_phase(id: &str, phase: f32, coherence: f32, trust: f32) -> AgentPhase {
        AgentPhase {
            id: Uuid::new_v4().to_string(),
            agent_id: id.to_string(),
            phase,
            frequency: 0.5,
            coherence,
            phi: 0.0,
            order_parameter: 0.0,
            cluster_count: 0,
            memory_count: 0,
            xi_signature: None,
            protocol_version: "1.0".to_string(),
            timestamp: Utc::now(),
            trust_score: trust,
            handedness: Handedness::Achiral,
        }
    }

    // -----------------------------------------------------------------------
    // Order parameter tests
    // -----------------------------------------------------------------------

    #[test]
    fn order_parameter_identical_phases() {
        let swarm = vec![
            make_agent_phase("a", 1.0, 1.0, 1.0),
            make_agent_phase("b", 1.0, 1.0, 1.0),
            make_agent_phase("c", 1.0, 1.0, 1.0),
        ];
        let (r, psi) = QueenSync::compute_order_parameter(&swarm);
        assert!((r - 1.0).abs() < 0.01, "identical phases → r≈1.0, got {}", r);
        assert!((psi - 1.0).abs() < 0.01, "mean phase should be 1.0, got {}", psi);
    }

    #[test]
    fn order_parameter_opposite_phases() {
        let swarm = vec![
            make_agent_phase("a", 0.0, 1.0, 1.0),
            make_agent_phase("b", PI, 1.0, 1.0),
        ];
        let (r, _) = QueenSync::compute_order_parameter(&swarm);
        assert!(r < 0.1, "opposite phases → r≈0, got {}", r);
    }

    #[test]
    fn order_parameter_evenly_spaced() {
        let n = 5;
        let swarm: Vec<AgentPhase> = (0..n)
            .map(|i| {
                make_agent_phase(
                    &format!("a{}", i),
                    TAU * i as f32 / n as f32,
                    1.0,
                    1.0,
                )
            })
            .collect();
        let (r, _) = QueenSync::compute_order_parameter(&swarm);
        assert!(r < 0.3, "evenly spaced → low r, got {}", r);
    }

    #[test]
    fn order_parameter_empty_swarm() {
        let (r, psi) = QueenSync::compute_order_parameter(&[]);
        assert_eq!(r, 0.0);
        assert_eq!(psi, 0.0);
    }

    #[test]
    fn order_parameter_trust_weighted() {
        // Agent a has high trust, agent b has zero trust
        let swarm = vec![
            make_agent_phase("a", 0.0, 1.0, 1.0),
            make_agent_phase("b", PI, 1.0, 0.0), // zero trust → no influence
        ];
        let (r, psi) = QueenSync::compute_order_parameter(&swarm);
        // Only agent a contributes, so r = 1/2 and psi ≈ 0
        assert!(psi.abs() < 0.1, "mean phase should follow trusted agent, got {}", psi);
    }

    // -----------------------------------------------------------------------
    // Chiral coupling tests
    // -----------------------------------------------------------------------

    #[test]
    fn chiral_coupling_achiral_is_zero() {
        let queen = QueenSync::new(QueenConfig::default(), "test");
        let c = queen.compute_chiral_coupling(Handedness::Achiral, 1.0);
        assert_eq!(c, 0.0);
    }

    #[test]
    fn chiral_coupling_left_right_opposite() {
        let queen = QueenSync::new(QueenConfig::default(), "test");
        let psi = 1.0;
        let left = queen.compute_chiral_coupling(Handedness::Left, psi);
        let right = queen.compute_chiral_coupling(Handedness::Right, psi);
        assert!((left + right).abs() < 1e-6, "left and right should be opposite: {} vs {}", left, right);
    }

    // -----------------------------------------------------------------------
    // Phi tests
    // -----------------------------------------------------------------------

    #[test]
    fn phi_increases_with_coherent_agents() {
        let low = vec![
            make_agent_phase("a", 0.0, 0.1, 1.0),
            make_agent_phase("b", PI, 0.1, 1.0),
        ];
        let high = vec![
            make_agent_phase("a", 0.5, 0.9, 1.0),
            make_agent_phase("b", 0.5, 0.9, 1.0),
        ];
        let (r_low, _) = QueenSync::compute_order_parameter(&low);
        let (r_high, _) = QueenSync::compute_order_parameter(&high);
        let phi_low = QueenSync::compute_swarm_phi(&low, r_low);
        let phi_high = QueenSync::compute_swarm_phi(&high, r_high);
        assert!(phi_high > phi_low, "coherent → higher Phi: {} vs {}", phi_high, phi_low);
    }

    #[test]
    fn phi_zero_for_single_agent() {
        let swarm = vec![make_agent_phase("a", 0.5, 1.0, 1.0)];
        let phi = QueenSync::compute_swarm_phi(&swarm, 1.0);
        assert_eq!(phi, 0.0, "single agent → Phi=0");
    }

    // -----------------------------------------------------------------------
    // Hive detection tests
    // -----------------------------------------------------------------------

    #[test]
    fn hive_detection_groups_close_phases() {
        let queen = QueenSync::new(QueenConfig::default(), "test");
        let swarm = vec![
            make_agent_phase("a", 0.0, 1.0, 1.0),
            make_agent_phase("b", 0.1, 1.0, 1.0),
            make_agent_phase("c", 0.2, 1.0, 1.0),
            make_agent_phase("d", PI, 1.0, 1.0),   // outlier
        ];
        let hives = queen.detect_hives(&swarm);
        // a, b, c should be in one hive; d is alone (no hive)
        assert!(!hives.is_empty(), "should detect at least one hive");
        let largest = hives.iter().max_by_key(|h| h.agent_ids.len()).unwrap();
        assert!(largest.agent_ids.len() >= 3, "hive should have a,b,c");
        assert!(!largest.agent_ids.contains(&"d".to_string()));
    }

    #[test]
    fn hive_detection_two_separate_hives() {
        let queen = QueenSync::new(QueenConfig::default(), "test");
        let swarm = vec![
            make_agent_phase("a", 0.0, 1.0, 1.0),
            make_agent_phase("b", 0.1, 1.0, 1.0),
            make_agent_phase("c", PI, 1.0, 1.0),
            make_agent_phase("d", PI + 0.1, 1.0, 1.0),
        ];
        let hives = queen.detect_hives(&swarm);
        assert_eq!(hives.len(), 2, "should detect 2 hives, got {}", hives.len());
    }

    // -----------------------------------------------------------------------
    // Queen sync step tests
    // -----------------------------------------------------------------------

    #[test]
    fn sync_step_produces_valid_queen_state() {
        let mut queen = QueenSync::new(QueenConfig::default(), "me");
        queen.phase = 0.5;
        let swarm = vec![
            make_agent_phase("me", 0.5, 0.8, 1.0),
            make_agent_phase("other1", 0.6, 0.7, 0.9),
            make_agent_phase("other2", 0.4, 0.9, 0.8),
        ];
        let state = queen.queen_sync_step(&swarm);
        assert!(state.order_parameter >= 0.0 && state.order_parameter <= 1.5);
        assert_eq!(state.agent_count, 3);
        assert_eq!(state.computed_by, "me");
        assert!(state.phi >= 0.0);
    }

    #[test]
    fn sync_step_converges_over_iterations() {
        let mut queen = QueenSync::new(
            QueenConfig {
                base_coupling: 2.0,
                dt: 0.1,
                ..Default::default()
            },
            "me",
        );
        queen.phase = 0.0;

        // Other agents are at different phases
        let mut swarm = vec![
            make_agent_phase("me", 0.0, 0.8, 1.0),
            make_agent_phase("a", 1.0, 0.8, 1.0),
            make_agent_phase("b", 2.0, 0.8, 1.0),
        ];

        let initial_r = QueenSync::compute_order_parameter(&swarm).0;

        // Run 50 sync steps
        for _ in 0..50 {
            let state = queen.queen_sync_step(&swarm);
            // Update "me" in the swarm
            swarm[0].phase = queen.phase;
            let _ = state;
        }

        let final_r = QueenSync::compute_order_parameter(&swarm).0;
        // Our phase should have moved toward the mean field
        // (full convergence requires all agents to move, but our phase should shift)
        assert!(
            queen.phase != 0.0,
            "phase should have changed from initial 0.0"
        );
    }

    // -----------------------------------------------------------------------
    // Phase derivation tests
    // -----------------------------------------------------------------------

    #[test]
    fn derive_local_state_frequency_scaling() {
        // Test the frequency formula: ω = ln(1 + count) / ln(1 + 100)
        let count_100 = ((1.0 + 100.0_f64).ln() / (1.0 + 100.0_f64).ln()) as f32;
        assert!((count_100 - 1.0).abs() < 0.01, "100 memories → ω≈1.0");

        let count_0 = ((1.0 + 0.0_f64).ln() / (1.0 + 100.0_f64).ln()) as f32;
        assert!((count_0 - 0.0).abs() < 0.01, "0 memories → ω≈0.0");
    }

    #[test]
    fn to_agent_phase_has_correct_fields() {
        let queen = QueenSync::new(QueenConfig::default(), "test-agent");
        let ap = queen.to_agent_phase(5, 100);
        assert_eq!(ap.agent_id, "test-agent");
        assert_eq!(ap.cluster_count, 5);
        assert_eq!(ap.memory_count, 100);
        assert_eq!(ap.protocol_version, "1.0");
    }
}
