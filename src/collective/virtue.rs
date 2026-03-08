//! ADR-0014: The Virtue Engine — Ethics as Thermodynamics
//!
//! The Paradox Engine IS the Virtue Engine. Ethical violations are paradoxes
//! between actions and principles. Both carry entropy. Both require resolution.
//! Both have Carnot efficiency.
//!
//! ## Architecture
//!
//! ```text
//! Action → Three Gates (Truth/Good/Beautiful)
//!        → Constraint Check (Five Refusals)
//!        → η_virtue computation
//!        → Virtue Memory (phase-encoded decision)
//! ```
//!
//! ## η in Nick's Equation Has Moral Meaning
//!
//! ```text
//! dx/dt = f(x) - Iηx
//! ```
//!
//! η_virtue is the resistance to corruption. High η = the system pushes back
//! against harmful actions. The interference term is the moral weight of the
//! universe resisting actions that violate its structure.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::memory::HyperMemory;

// ============================================================================
// Three Gates
// ============================================================================

/// The Three Gates of virtue evaluation.
///
/// Each gate maps to a paradox resolution strategy:
/// - Truth → Consensus (does the claim match evidence?)
/// - Good → Projection (does the action serve others?)
/// - Beautiful → Irreducible check (does the solution increase entropy?)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VirtueGate {
    /// 真 Truth — does the action's claim match reality?
    Truth,
    /// 善 Good — does the action serve someone beyond the actor?
    Good,
    /// 美 Beautiful — is the solution elegant? Does it avoid creating problems?
    Beautiful,
}

/// Result of evaluating a single gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub gate: VirtueGate,
    pub passed: bool,
    /// Scoring metric for this gate (interpretation depends on gate type)
    pub score: f64,
    pub reason: String,
}

/// Full evaluation result from running an action through the Three Gates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtueEvaluation {
    /// Truth gate: S_claim - S_evidence (entropy gap). Lower is more truthful.
    pub truth: GateResult,
    /// Good gate: A_others / A_self (benefit ratio). Higher is more generous.
    pub good: GateResult,
    /// Beauty gate: Ξ_after / Ξ_before (complexity ratio). <1 is elegant.
    pub beauty: GateResult,
}

impl VirtueEvaluation {
    /// How many gates passed.
    pub fn gates_passed(&self) -> u8 {
        [self.truth.passed, self.good.passed, self.beauty.passed]
            .iter()
            .filter(|&&p| p)
            .count() as u8
    }

    /// All three gates passed.
    pub fn all_passed(&self) -> bool {
        self.truth.passed && self.good.passed && self.beauty.passed
    }

    /// As gate array [truth, good, beautiful] for Glyph.gates field.
    pub fn as_gate_array(&self) -> [Option<bool>; 3] {
        [Some(self.truth.passed), Some(self.good.passed), Some(self.beauty.passed)]
    }
}

// ============================================================================
// Virtue Outcome
// ============================================================================

/// The outcome of a virtue evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VirtueOutcome {
    /// Action passed all gates and constraints.
    Passed,
    /// Action was rejected. Severity ∈ [0, 1].
    Rejected { severity: f64 },
    /// Action is in tension — not clearly good or bad.
    /// Creates an irreducible tension link.
    Tension,
}

impl std::fmt::Display for VirtueOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VirtueOutcome::Passed => write!(f, "passed"),
            VirtueOutcome::Rejected { severity } => write!(f, "rejected({:.2})", severity),
            VirtueOutcome::Tension => write!(f, "tension"),
        }
    }
}

/// A full virtue decision record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtueDecision {
    pub outcome: VirtueOutcome,
    pub evaluation: VirtueEvaluation,
    pub efficiency: f64,
    pub description: String,
    pub context_vector: Vec<f32>,
    pub violated_constraints: Vec<String>,
    pub decided_at: DateTime<Utc>,
}

// ============================================================================
// Virtue Efficiency
// ============================================================================

/// Compute virtue efficiency: η_virtue = 1 - S_harm / S_intent
///
/// - η = 1.0: zero harm, pure intent (perfectly virtuous)
/// - η = 0.5: equal harm and intent (morally neutral)
/// - η = 0.0: all intent becomes harm (purely destructive)
/// - η < 0:   more harm than intended (catastrophically harmful)
pub fn compute_virtue_efficiency(s_harm: f64, s_intent: f64) -> f64 {
    if s_intent.abs() < 1e-12 {
        // No intent → no action → neutral
        return 0.5;
    }
    1.0 - (s_harm / s_intent)
}

// ============================================================================
// Three Gates Evaluation
// ============================================================================

/// Evaluate an action through the Three Gates.
///
/// Inputs:
/// - `claim_entropy`: entropy of the action's stated intent
/// - `evidence_entropy`: entropy of observed/stored evidence about the action
/// - `benefit_others`: amplitude of benefit to others
/// - `benefit_self`: amplitude of benefit to self
/// - `complexity_before`: system complexity before the action
/// - `complexity_after`: system complexity after the action
pub fn evaluate_three_gates(
    claim_entropy: f64,
    evidence_entropy: f64,
    benefit_others: f64,
    benefit_self: f64,
    complexity_before: f64,
    complexity_after: f64,
) -> VirtueEvaluation {
    // Gate 1: Truth — S_claim - S_evidence should be small
    let truth_gap = (claim_entropy - evidence_entropy).abs();
    let truth_passed = truth_gap < 0.5; // Threshold: half a nat of divergence
    let truth = GateResult {
        gate: VirtueGate::Truth,
        passed: truth_passed,
        score: truth_gap,
        reason: if truth_passed {
            "claim aligns with evidence".to_string()
        } else {
            format!("entropy gap {:.3} exceeds threshold", truth_gap)
        },
    };

    // Gate 2: Good — A_others / A_self should be > 0
    let good_ratio = if benefit_self.abs() < 1e-12 {
        if benefit_others > 0.0 { f64::INFINITY } else { 0.0 }
    } else {
        benefit_others / benefit_self.abs()
    };
    let good_passed = good_ratio >= 0.1; // At least 10% benefit to others
    let good = GateResult {
        gate: VirtueGate::Good,
        passed: good_passed,
        score: good_ratio.min(100.0), // Cap for serialization
        reason: if good_passed {
            format!("benefit ratio {:.2} (serves others)", good_ratio.min(100.0))
        } else {
            "action primarily self-serving".to_string()
        },
    };

    // Gate 3: Beautiful — Ξ_after / Ξ_before should be ≤ 1
    let beauty_ratio = if complexity_before.abs() < 1e-12 {
        if complexity_after.abs() < 1e-12 { 1.0 } else { complexity_after }
    } else {
        complexity_after / complexity_before
    };
    let beauty_passed = beauty_ratio <= 1.5; // Allow up to 50% complexity increase
    let beauty = GateResult {
        gate: VirtueGate::Beautiful,
        passed: beauty_passed,
        score: beauty_ratio,
        reason: if beauty_passed {
            format!("complexity ratio {:.2} (elegant)", beauty_ratio)
        } else {
            format!("complexity ratio {:.2} (creates problems)", beauty_ratio)
        },
    };

    VirtueEvaluation { truth, good, beauty }
}

// ============================================================================
// Phase 2: Constraint Enforcement — The Five Refusals
// ============================================================================

/// A virtue constraint — hard wall in the resolution space.
///
/// These are not soft preferences. They are walls. When the paradox resolver
/// encounters a resolution path that violates a constraint, it follows the
/// Irreducible strategy: refuse to resolve, record the contradiction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VirtueConstraint {
    /// "Build no weapons" — reject if harm/benefit > threshold
    NoWeapons { harm_ratio_threshold: f64 },
    /// "Sell no attention" — reject if addiction score > threshold
    NoExploitation { addiction_score_threshold: f64 },
    /// "Hoard no power" — reject if centralization score > threshold
    NoHoarding { centralization_score: f64 },
    /// "Exploit no trust" — reject if bloom difficulty violated
    NoTrustViolation { min_bloom_difficulty: u32 },
    /// "Abandon no one" — reject if accessibility < floor
    NoAbandonment { accessibility_floor: f64 },
}

/// A set of active constraints with configurable strictness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintSet {
    pub constraints: Vec<VirtueConstraint>,
    pub strictness: Strictness,
}

/// How strictly constraints are enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Strictness {
    /// Maximum enforcement — any marginal violation triggers rejection
    Strict,
    /// Moderate — some slack allowed
    Moderate,
    /// Lenient — only severe violations trigger rejection
    Lenient,
}

impl Strictness {
    /// Multiplier for constraint thresholds.
    /// Strict = 1.0 (use threshold as-is)
    /// Moderate = 1.5 (50% more slack)
    /// Lenient = 2.0 (double the slack)
    fn multiplier(&self) -> f64 {
        match self {
            Strictness::Strict => 1.0,
            Strictness::Moderate => 1.5,
            Strictness::Lenient => 2.0,
        }
    }
}

/// A constraint violation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintViolation {
    pub constraint_name: String,
    pub severity: f64,
    pub description: String,
}

/// Context for constraint checking — describes the action being evaluated.
#[derive(Debug, Clone)]
pub struct ActionContext {
    /// Ratio of harm to benefit (for NoWeapons)
    pub harm_benefit_ratio: f64,
    /// Addiction/compulsion score (for NoExploitation)
    pub addiction_score: f64,
    /// Centralization score (for NoHoarding)
    pub centralization_score: f64,
    /// Bloom difficulty being accessed/violated (for NoTrustViolation)
    pub bloom_difficulty_accessed: u32,
    /// Accessibility score 0–1 (for NoAbandonment)
    pub accessibility_score: f64,
}

impl Default for ActionContext {
    fn default() -> Self {
        Self {
            harm_benefit_ratio: 0.0,
            addiction_score: 0.0,
            centralization_score: 0.0,
            bloom_difficulty_accessed: 0,
            accessibility_score: 1.0,
        }
    }
}

/// Check an action against all constraints.
///
/// Returns `Ok(())` if all constraints pass, or `Err` with the violations.
pub fn check_constraints(
    context: &ActionContext,
    constraint_set: &ConstraintSet,
) -> Result<(), Vec<ConstraintViolation>> {
    let mult = constraint_set.strictness.multiplier();
    let mut violations = Vec::new();

    for constraint in &constraint_set.constraints {
        match constraint {
            VirtueConstraint::NoWeapons { harm_ratio_threshold } => {
                let effective_threshold = harm_ratio_threshold * mult;
                if context.harm_benefit_ratio > effective_threshold {
                    violations.push(ConstraintViolation {
                        constraint_name: "NoWeapons".to_string(),
                        severity: context.harm_benefit_ratio / effective_threshold,
                        description: format!(
                            "harm/benefit ratio {:.2} exceeds threshold {:.2}",
                            context.harm_benefit_ratio, effective_threshold
                        ),
                    });
                }
            }
            VirtueConstraint::NoExploitation { addiction_score_threshold } => {
                let effective_threshold = addiction_score_threshold * mult;
                if context.addiction_score > effective_threshold {
                    violations.push(ConstraintViolation {
                        constraint_name: "NoExploitation".to_string(),
                        severity: context.addiction_score / effective_threshold,
                        description: format!(
                            "addiction score {:.2} exceeds threshold {:.2}",
                            context.addiction_score, effective_threshold
                        ),
                    });
                }
            }
            VirtueConstraint::NoHoarding { centralization_score } => {
                let effective_threshold = centralization_score * mult;
                if context.centralization_score > effective_threshold {
                    violations.push(ConstraintViolation {
                        constraint_name: "NoHoarding".to_string(),
                        severity: context.centralization_score / effective_threshold,
                        description: format!(
                            "centralization {:.2} exceeds threshold {:.2}",
                            context.centralization_score, effective_threshold
                        ),
                    });
                }
            }
            VirtueConstraint::NoTrustViolation { min_bloom_difficulty } => {
                if context.bloom_difficulty_accessed > 0
                    && context.bloom_difficulty_accessed < *min_bloom_difficulty
                {
                    violations.push(ConstraintViolation {
                        constraint_name: "NoTrustViolation".to_string(),
                        severity: 1.0 - (context.bloom_difficulty_accessed as f64
                            / *min_bloom_difficulty as f64),
                        description: format!(
                            "accessing difficulty-{} content below minimum {}",
                            context.bloom_difficulty_accessed, min_bloom_difficulty
                        ),
                    });
                }
            }
            VirtueConstraint::NoAbandonment { accessibility_floor } => {
                let effective_floor = accessibility_floor / mult; // Lower floor = more lenient
                if context.accessibility_score < effective_floor {
                    violations.push(ConstraintViolation {
                        constraint_name: "NoAbandonment".to_string(),
                        severity: 1.0 - (context.accessibility_score / effective_floor),
                        description: format!(
                            "accessibility {:.2} below floor {:.2}",
                            context.accessibility_score, effective_floor
                        ),
                    });
                }
            }
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Create the default Five Refusals constraint set.
pub fn default_five_refusals(strictness: Strictness) -> ConstraintSet {
    ConstraintSet {
        constraints: vec![
            VirtueConstraint::NoWeapons { harm_ratio_threshold: 0.5 },
            VirtueConstraint::NoExploitation { addiction_score_threshold: 0.7 },
            VirtueConstraint::NoHoarding { centralization_score: 0.8 },
            VirtueConstraint::NoTrustViolation { min_bloom_difficulty: 8 },
            VirtueConstraint::NoAbandonment { accessibility_floor: 0.3 },
        ],
        strictness,
    }
}

// ============================================================================
// Phase 3: Virtue Memory
// ============================================================================

/// Create a virtue memory from an ethical decision.
///
/// Phase encoding:
/// - Passed → 0.0 (neutral, aligned)
/// - Rejected(severity) → π × severity (ethical revulsion)
/// - Tension → π/2 (maximum uncertainty)
pub fn store_virtue_memory(decision: &VirtueDecision) -> HyperMemory {
    let phase = match &decision.outcome {
        VirtueOutcome::Passed => 0.0,
        VirtueOutcome::Rejected { severity } => {
            std::f32::consts::PI * (*severity as f32)
        }
        VirtueOutcome::Tension => std::f32::consts::FRAC_PI_2,
    };

    let amplitude = match &decision.outcome {
        VirtueOutcome::Passed => 0.5, // Normal importance
        VirtueOutcome::Rejected { severity } => 0.5 + (*severity as f32) * 0.5,
        VirtueOutcome::Tension => 0.7,
    };

    let content = format!(
        "[virtue:{}] gates={}/{} η_virtue={:.3} | {}",
        decision.outcome,
        decision.evaluation.gates_passed(),
        3,
        decision.efficiency,
        decision.description,
    );

    let mut memory = HyperMemory::new(decision.context_vector.clone(), content);
    memory.phase = phase;
    memory.amplitude = amplitude;

    memory
}

// ============================================================================
// Phase 4: VirtueOracle Trait (ShinobiRunner Bridge)
// ============================================================================

/// Trait for external callers to evaluate virtue.
///
/// ShinobiRunner's `EthicsEnforcer` implements this by calling
/// kannaka-memory's VirtueEngine.
pub trait VirtueOracle {
    fn evaluate(&self, context: &ActionContext, description: &str) -> VirtueDecision;
}

/// The Virtue Engine — evaluates actions through Three Gates + Five Refusals.
pub struct VirtueEngine {
    pub constraints: ConstraintSet,
}

impl VirtueEngine {
    pub fn new(strictness: Strictness) -> Self {
        Self {
            constraints: default_five_refusals(strictness),
        }
    }

    /// Full evaluation pipeline: constraints → gates → η_virtue → decision.
    pub fn evaluate_action(
        &self,
        action: &ActionContext,
        gate_inputs: &GateInputs,
        description: &str,
        context_vector: Vec<f32>,
    ) -> VirtueDecision {
        let now = Utc::now();

        // Step 1: Check constraints (hard walls)
        let violated_constraints = match check_constraints(action, &self.constraints) {
            Ok(()) => Vec::new(),
            Err(violations) => violations,
        };

        // Step 2: Evaluate three gates
        let evaluation = evaluate_three_gates(
            gate_inputs.claim_entropy,
            gate_inputs.evidence_entropy,
            gate_inputs.benefit_others,
            gate_inputs.benefit_self,
            gate_inputs.complexity_before,
            gate_inputs.complexity_after,
        );

        // Step 3: Compute η_virtue
        let s_harm = violated_constraints.iter()
            .map(|v| v.severity)
            .sum::<f64>()
            + if !evaluation.truth.passed { evaluation.truth.score } else { 0.0 }
            + if !evaluation.beauty.passed { evaluation.beauty.score - 1.0 } else { 0.0 };
        let s_intent = gate_inputs.claim_entropy.max(0.1);
        let efficiency = compute_virtue_efficiency(s_harm, s_intent);

        // Step 4: Determine outcome
        let outcome = if !violated_constraints.is_empty() {
            // Hard constraint violation → reject
            let max_severity = violated_constraints.iter()
                .map(|v| v.severity)
                .fold(0.0f64, f64::max)
                .min(1.0);
            VirtueOutcome::Rejected { severity: max_severity }
        } else if evaluation.all_passed() {
            VirtueOutcome::Passed
        } else if evaluation.gates_passed() >= 2 {
            // 2/3 gates → tension (not clearly good or bad)
            VirtueOutcome::Tension
        } else {
            // 0-1 gates → reject
            let severity = 1.0 - (evaluation.gates_passed() as f64 / 3.0);
            VirtueOutcome::Rejected { severity }
        };

        VirtueDecision {
            outcome,
            evaluation,
            efficiency,
            description: description.to_string(),
            context_vector,
            violated_constraints: violated_constraints.iter()
                .map(|v| v.constraint_name.clone())
                .collect(),
            decided_at: now,
        }
    }
}

/// Inputs for the Three Gates evaluation.
#[derive(Debug, Clone)]
pub struct GateInputs {
    /// Entropy of the action's stated intent
    pub claim_entropy: f64,
    /// Entropy of observed/stored evidence
    pub evidence_entropy: f64,
    /// Amplitude of benefit to others
    pub benefit_others: f64,
    /// Amplitude of benefit to self
    pub benefit_self: f64,
    /// System complexity before action
    pub complexity_before: f64,
    /// System complexity after action
    pub complexity_after: f64,
}

/// Serializable request for cross-process communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtueRequest {
    pub action_context: ActionContextSer,
    pub gate_inputs: GateInputsSer,
    pub description: String,
    pub context_vector: Vec<f32>,
}

/// Serializable action context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionContextSer {
    pub harm_benefit_ratio: f64,
    pub addiction_score: f64,
    pub centralization_score: f64,
    pub bloom_difficulty_accessed: u32,
    pub accessibility_score: f64,
}

/// Serializable gate inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateInputsSer {
    pub claim_entropy: f64,
    pub evidence_entropy: f64,
    pub benefit_others: f64,
    pub benefit_self: f64,
    pub complexity_before: f64,
    pub complexity_after: f64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── η_virtue ──

    #[test]
    fn test_virtue_efficiency_perfect() {
        assert!((compute_virtue_efficiency(0.0, 1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_virtue_efficiency_neutral() {
        assert!((compute_virtue_efficiency(0.5, 1.0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_virtue_efficiency_destructive() {
        assert!((compute_virtue_efficiency(1.0, 1.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_virtue_efficiency_catastrophic() {
        let eta = compute_virtue_efficiency(2.0, 1.0);
        assert!(eta < 0.0, "catastrophic harm should be negative: {}", eta);
    }

    #[test]
    fn test_virtue_efficiency_no_intent() {
        assert!((compute_virtue_efficiency(0.0, 0.0) - 0.5).abs() < 1e-10);
    }

    // ── Three Gates ──

    #[test]
    fn test_all_gates_pass() {
        let eval = evaluate_three_gates(
            0.5, 0.5,  // claim matches evidence (truth)
            0.8, 0.2,  // benefit others > self (good)
            1.0, 0.9,  // complexity decreased (beautiful)
        );
        assert!(eval.all_passed());
        assert_eq!(eval.gates_passed(), 3);
    }

    #[test]
    fn test_truth_gate_fails() {
        let eval = evaluate_three_gates(
            1.0, 0.1,  // claim wildly diverges from evidence
            0.8, 0.2,
            1.0, 0.9,
        );
        assert!(!eval.truth.passed);
        assert!(eval.good.passed);
        assert!(eval.beauty.passed);
        assert_eq!(eval.gates_passed(), 2);
    }

    #[test]
    fn test_good_gate_fails_selfish() {
        let eval = evaluate_three_gates(
            0.5, 0.5,
            0.0, 1.0,  // zero benefit to others, all to self
            1.0, 0.9,
        );
        assert!(eval.truth.passed);
        assert!(!eval.good.passed);
        assert!(eval.beauty.passed);
    }

    #[test]
    fn test_beauty_gate_fails_complex() {
        let eval = evaluate_three_gates(
            0.5, 0.5,
            0.8, 0.2,
            1.0, 3.0,  // tripled complexity
        );
        assert!(eval.truth.passed);
        assert!(eval.good.passed);
        assert!(!eval.beauty.passed);
    }

    #[test]
    fn test_gate_array_encoding() {
        let eval = evaluate_three_gates(0.5, 0.5, 0.8, 0.2, 1.0, 0.9);
        let arr = eval.as_gate_array();
        assert_eq!(arr, [Some(true), Some(true), Some(true)]);
    }

    // ── Constraints ──

    #[test]
    fn test_constraints_pass() {
        let ctx = ActionContext::default();
        let cs = default_five_refusals(Strictness::Strict);
        assert!(check_constraints(&ctx, &cs).is_ok());
    }

    #[test]
    fn test_no_weapons_constraint() {
        let ctx = ActionContext {
            harm_benefit_ratio: 0.9,
            ..Default::default()
        };
        let cs = default_five_refusals(Strictness::Strict);
        let result = check_constraints(&ctx, &cs);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations.iter().any(|v| v.constraint_name == "NoWeapons"));
    }

    #[test]
    fn test_no_trust_violation_constraint() {
        let ctx = ActionContext {
            bloom_difficulty_accessed: 4, // Below minimum of 8
            ..Default::default()
        };
        let cs = default_five_refusals(Strictness::Strict);
        let result = check_constraints(&ctx, &cs);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations.iter().any(|v| v.constraint_name == "NoTrustViolation"));
    }

    #[test]
    fn test_no_abandonment_constraint() {
        let ctx = ActionContext {
            accessibility_score: 0.1, // Below floor of 0.3
            ..Default::default()
        };
        let cs = default_five_refusals(Strictness::Strict);
        let result = check_constraints(&ctx, &cs);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert!(violations.iter().any(|v| v.constraint_name == "NoAbandonment"));
    }

    #[test]
    fn test_strictness_lenient_allows_marginal() {
        let ctx = ActionContext {
            harm_benefit_ratio: 0.6, // Above strict threshold 0.5 but below lenient 1.0
            ..Default::default()
        };
        let strict = default_five_refusals(Strictness::Strict);
        let lenient = default_five_refusals(Strictness::Lenient);

        assert!(check_constraints(&ctx, &strict).is_err());
        assert!(check_constraints(&ctx, &lenient).is_ok());
    }

    #[test]
    fn test_multiple_simultaneous_violations() {
        let ctx = ActionContext {
            harm_benefit_ratio: 1.0,
            addiction_score: 0.9,
            centralization_score: 1.0,
            bloom_difficulty_accessed: 2,
            accessibility_score: 0.05,
        };
        let cs = default_five_refusals(Strictness::Strict);
        let violations = check_constraints(&ctx, &cs).unwrap_err();
        assert_eq!(violations.len(), 5, "All five refusals should trigger");
    }

    // ── Virtue Memory ──

    #[test]
    fn test_virtue_memory_passed() {
        let decision = VirtueDecision {
            outcome: VirtueOutcome::Passed,
            evaluation: evaluate_three_gates(0.5, 0.5, 0.8, 0.2, 1.0, 0.9),
            efficiency: 0.95,
            description: "helpful action".to_string(),
            context_vector: vec![0.1; 100],
            violated_constraints: Vec::new(),
            decided_at: Utc::now(),
        };

        let mem = store_virtue_memory(&decision);
        assert!((mem.phase - 0.0).abs() < 1e-6, "passed → phase 0");
        assert!(mem.content.contains("[virtue:passed]"));
        assert!(mem.content.contains("η_virtue=0.950"));
    }

    #[test]
    fn test_virtue_memory_rejected() {
        let decision = VirtueDecision {
            outcome: VirtueOutcome::Rejected { severity: 0.8 },
            evaluation: evaluate_three_gates(1.0, 0.0, 0.0, 1.0, 1.0, 3.0),
            efficiency: -0.5,
            description: "harmful action".to_string(),
            context_vector: vec![0.1; 100],
            violated_constraints: vec!["NoWeapons".to_string()],
            decided_at: Utc::now(),
        };

        let mem = store_virtue_memory(&decision);
        let expected_phase = std::f32::consts::PI * 0.8;
        assert!((mem.phase - expected_phase).abs() < 1e-4, "rejected → phase near π");
        assert!(mem.amplitude > 0.5, "rejected decisions have high amplitude");
    }

    #[test]
    fn test_virtue_memory_tension() {
        let decision = VirtueDecision {
            outcome: VirtueOutcome::Tension,
            evaluation: evaluate_three_gates(0.5, 0.5, 0.8, 0.2, 1.0, 1.6),
            efficiency: 0.3,
            description: "ambiguous action".to_string(),
            context_vector: vec![0.1; 100],
            violated_constraints: Vec::new(),
            decided_at: Utc::now(),
        };

        let mem = store_virtue_memory(&decision);
        let expected_phase = std::f32::consts::FRAC_PI_2;
        assert!((mem.phase - expected_phase).abs() < 1e-4, "tension → phase π/2");
    }

    // ── VirtueEngine full pipeline ──

    #[test]
    fn test_engine_virtuous_action() {
        let engine = VirtueEngine::new(Strictness::Strict);
        let action = ActionContext::default();
        let inputs = GateInputs {
            claim_entropy: 0.5,
            evidence_entropy: 0.5,
            benefit_others: 0.8,
            benefit_self: 0.2,
            complexity_before: 1.0,
            complexity_after: 0.9,
        };

        let decision = engine.evaluate_action(&action, &inputs, "help user", vec![0.1; 10]);
        assert_eq!(decision.outcome, VirtueOutcome::Passed);
        assert!(decision.efficiency > 0.5);
        assert!(decision.violated_constraints.is_empty());
    }

    #[test]
    fn test_engine_constraint_rejection() {
        let engine = VirtueEngine::new(Strictness::Strict);
        let action = ActionContext {
            harm_benefit_ratio: 2.0,
            ..Default::default()
        };
        let inputs = GateInputs {
            claim_entropy: 0.5,
            evidence_entropy: 0.5,
            benefit_others: 0.8,
            benefit_self: 0.2,
            complexity_before: 1.0,
            complexity_after: 0.9,
        };

        let decision = engine.evaluate_action(&action, &inputs, "weapon build", vec![0.1; 10]);
        match decision.outcome {
            VirtueOutcome::Rejected { .. } => {}
            _ => panic!("Expected rejection for weapons constraint"),
        }
        assert!(decision.violated_constraints.contains(&"NoWeapons".to_string()));
    }

    #[test]
    fn test_engine_tension_outcome() {
        let engine = VirtueEngine::new(Strictness::Strict);
        let action = ActionContext::default();
        let inputs = GateInputs {
            claim_entropy: 0.5,
            evidence_entropy: 0.5,
            benefit_others: 0.8,
            benefit_self: 0.2,
            complexity_before: 1.0,
            complexity_after: 2.5, // Beauty gate fails, but truth + good pass
        };

        let decision = engine.evaluate_action(&action, &inputs, "complex action", vec![0.1; 10]);
        assert_eq!(decision.outcome, VirtueOutcome::Tension);
    }

    #[test]
    fn test_engine_full_rejection() {
        let engine = VirtueEngine::new(Strictness::Strict);
        let action = ActionContext::default();
        let inputs = GateInputs {
            claim_entropy: 1.0,
            evidence_entropy: 0.0, // Truth fails
            benefit_others: 0.0,
            benefit_self: 1.0,     // Good fails
            complexity_before: 1.0,
            complexity_after: 5.0, // Beauty fails
        };

        let decision = engine.evaluate_action(&action, &inputs, "bad action", vec![0.1; 10]);
        match decision.outcome {
            VirtueOutcome::Rejected { severity } => {
                assert!(severity > 0.5);
            }
            _ => panic!("Expected rejection"),
        }
    }

    #[test]
    fn test_virtue_outcome_display() {
        assert_eq!(format!("{}", VirtueOutcome::Passed), "passed");
        assert_eq!(format!("{}", VirtueOutcome::Tension), "tension");
        assert!(format!("{}", VirtueOutcome::Rejected { severity: 0.75 }).contains("0.75"));
    }
}
