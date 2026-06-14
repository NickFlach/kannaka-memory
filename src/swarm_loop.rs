//! Self-directed sensemaking loop — ADR-0035 Phase 4 / Wave 4 Task 4.3.
//!
//! The five ADR-0035 swarm states made an explicit, deterministic machine. The
//! pure [`next_state`] transition is a function of a [`SwarmContext`] snapshot, so
//! the loop is unit-testable in isolation; the daemon (`kannaka swarm loop`) wires
//! each state to already-built actions: Discovery = hive formation (4.2),
//! Synchronization = QueenSync, Sensemaking = brief/gaps/plan (Wave 1/3.3/4.1),
//! Dreaming = cross-agent dream (3.1), Governance = immune (Wave 2) + contradiction
//! resolution. The loop *is* the swarm OODA — the collective successor to the
//! single-agent autoresearch.

/// The five swarm operating modes (ADR-0035).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmState {
    Discovery,
    Synchronization,
    Sensemaking,
    Dreaming,
    Governance,
}

impl SwarmState {
    pub fn name(self) -> &'static str {
        match self {
            SwarmState::Discovery => "Discovery",
            SwarmState::Synchronization => "Synchronization",
            SwarmState::Sensemaking => "Sensemaking",
            SwarmState::Dreaming => "Dreaming",
            SwarmState::Governance => "Governance",
        }
    }
}

/// A snapshot of swarm conditions that drives the transition.
#[derive(Clone, Debug, Default)]
pub struct SwarmContext {
    pub peer_count: usize,
    /// Kuramoto order parameter / coherence in [0, 1].
    pub order_parameter: f32,
    pub open_contradictions: usize,
    pub gap_count: usize,
    pub at_risk_memories: usize,
}

/// The result of a transition: the action to run in the *current* state and the
/// next state to move to.
#[derive(Clone, Debug)]
pub struct StateOutcome {
    pub action: &'static str,
    pub next: SwarmState,
}

/// Coherence floor above which the swarm is considered synchronized enough to
/// start making sense.
pub const SYNC_FLOOR: f32 = 0.5;

/// Deterministic transition. The cycle is Discovery → Synchronization →
/// Sensemaking → (Governance if hygiene pending else Dreaming) → Governance →
/// Discovery. A lone agent (no peers) stays in Discovery; an incoherent swarm
/// stays in Synchronization until it locks; pending contradictions/at-risk
/// memories force an earlier Governance visit before dreaming.
pub fn next_state(state: SwarmState, ctx: &SwarmContext) -> StateOutcome {
    match state {
        SwarmState::Discovery => {
            if ctx.peer_count == 0 {
                StateOutcome {
                    action: "survey peers + form hives",
                    next: SwarmState::Discovery, // lone agent re-surveys
                }
            } else {
                StateOutcome {
                    action: "peers found; form hives, then synchronize",
                    next: SwarmState::Synchronization,
                }
            }
        }
        SwarmState::Synchronization => {
            if ctx.order_parameter >= SYNC_FLOOR {
                StateOutcome {
                    action: "coherent; generate collective understanding",
                    next: SwarmState::Sensemaking,
                }
            } else {
                StateOutcome {
                    action: "run a QueenSync step (still below coherence floor)",
                    next: SwarmState::Synchronization,
                }
            }
        }
        SwarmState::Sensemaking => {
            // brief / gaps / plan. If hygiene is pending, govern before dreaming.
            if ctx.open_contradictions > 0 || ctx.at_risk_memories > 0 {
                StateOutcome {
                    action: "made sense; hygiene pending -> govern",
                    next: SwarmState::Governance,
                }
            } else {
                StateOutcome {
                    action: "brief/gaps/plan complete; dream across agents",
                    next: SwarmState::Dreaming,
                }
            }
        }
        SwarmState::Dreaming => StateOutcome {
            action: "cross-agent dream; reinforce shared hypotheses",
            next: SwarmState::Governance,
        },
        SwarmState::Governance => StateOutcome {
            action: "immune sweep + resolve contradictions; re-survey",
            next: SwarmState::Discovery,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lone_agent_stays_in_discovery() {
        let ctx = SwarmContext { peer_count: 0, ..Default::default() };
        assert_eq!(next_state(SwarmState::Discovery, &ctx).next, SwarmState::Discovery);
    }

    #[test]
    fn coherent_swarm_progresses_discovery_sync_sensemaking() {
        let ctx = SwarmContext {
            peer_count: 3,
            order_parameter: 0.8,
            ..Default::default()
        };
        let s1 = next_state(SwarmState::Discovery, &ctx).next;
        assert_eq!(s1, SwarmState::Synchronization);
        let s2 = next_state(s1, &ctx).next;
        assert_eq!(s2, SwarmState::Sensemaking);
    }

    #[test]
    fn low_coherence_stays_in_sync() {
        let ctx = SwarmContext { peer_count: 3, order_parameter: 0.2, ..Default::default() };
        assert_eq!(next_state(SwarmState::Synchronization, &ctx).next, SwarmState::Synchronization);
    }

    #[test]
    fn contradictions_force_governance_before_dreaming() {
        let ctx = SwarmContext {
            peer_count: 3,
            order_parameter: 0.8,
            open_contradictions: 2,
            ..Default::default()
        };
        assert_eq!(next_state(SwarmState::Sensemaking, &ctx).next, SwarmState::Governance);
    }

    #[test]
    fn clean_sensemaking_goes_to_dreaming() {
        let ctx = SwarmContext {
            peer_count: 3,
            order_parameter: 0.8,
            ..Default::default()
        };
        assert_eq!(next_state(SwarmState::Sensemaking, &ctx).next, SwarmState::Dreaming);
    }

    #[test]
    fn full_cycle_returns_to_discovery_and_is_total() {
        // Every state has a defined exit; a clean coherent swarm cycles back.
        let ctx = SwarmContext { peer_count: 3, order_parameter: 0.9, ..Default::default() };
        let mut state = SwarmState::Discovery;
        let mut seen = Vec::new();
        for _ in 0..6 {
            seen.push(state);
            state = next_state(state, &ctx).next;
        }
        assert!(seen.contains(&SwarmState::Governance));
        assert!(seen.contains(&SwarmState::Dreaming));
        // returns to Discovery within a cycle
        assert_eq!(state, SwarmState::Synchronization); // post-Governance -> Discovery -> Sync
    }
}
