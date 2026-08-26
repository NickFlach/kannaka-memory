//! Corpus Callosum — the bandwidth-limited, balance-seeking bridge between hemispheres.
//!
//! The callosum is NOT a passive pipe. It is an active optimizer that:
//! - Selectively gates transfers based on salience and coherence
//! - Limits bandwidth to prevent hemispheric collapse
//! - Asymmetrically favors intuition (holistic→analytical) over consolidation (analytical→holistic)
//! - Actively seeks balance between hemispheres
//! - Routes sensory input through the optic chiasm (crossed wiring)

use serde::{Deserialize, Serialize};
use super::types::*;

/// Transfer record — tracks what crossed the callosum and when
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRecord {
    /// Which wavefront was transferred
    pub wavefront_id: uuid::Uuid,
    /// Direction of transfer
    pub direction_label: String,
    /// Energy of the wavefront at transfer time
    pub energy: f32,
    /// Timestamp of transfer
    pub timestamp: i64,
}

/// The Corpus Callosum — connects left and right hemispheres
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusCallosum {
    /// Maximum information flow per timestep (total energy units transferable)
    pub bandwidth: f32,

    /// Minimum energy for a wavefront to cross the callosum
    pub gate_threshold: f32,

    /// Asymmetry ratio: how much faster holistic→analytical is vs analytical→holistic
    /// > 1.0 means intuition flows faster than consolidation
    pub asymmetry: f32,

    /// Noise magnitude injected during sub→conscious transfer
    /// Models the "fuzziness" of intuition
    pub recall_noise: f32,

    /// Minimum phase coherence with target hemisphere for transfer
    pub coherence_gate: f32,

    /// Current bandwidth remaining this timestep
    remaining_budget: f32,

    /// Transfer history (recent, for debugging and learning)
    pub transfer_log: Vec<TransferRecord>,

    /// Maximum transfer log size
    max_log_size: usize,

    /// #824: energy-gate decisions made this process -- how many transfers were
    /// CONSIDERED, and how many cleared the gate. `efficiency` is passes/attempts.
    /// Not persisted (runtime-only, `#[serde(skip)]`) so the .hrm format is
    /// unchanged and old readers are unaffected; resets to 0 on load. The old
    /// efficiency counted only LOGGED transfers, which had all already cleared
    /// the gate at their call site, so it was 1.0 by construction.
    #[serde(skip)]
    gate_attempts: u64,
    #[serde(skip)]
    gate_passes: u64,
}

impl CorpusCallosum {
    /// Create a new callosum with default parameters
    pub fn new() -> Self {
        Self {
            bandwidth: 20.0,
            gate_threshold: 0.3,
            asymmetry: 3.0, // intuition 3x faster than consolidation
            recall_noise: 0.05,
            coherence_gate: 0.1,
            remaining_budget: 20.0,
            transfer_log: Vec::new(),
            max_log_size: 100,
            gate_attempts: 0,
            gate_passes: 0,
        }
    }

    /// Reset the bandwidth budget for a new timestep
    pub fn reset_budget(&mut self) {
        self.remaining_budget = self.bandwidth;
    }

    /// Check if there's budget remaining for a transfer
    pub fn has_budget(&self) -> bool {
        self.remaining_budget > 0.0
    }

    /// Get remaining budget
    pub fn remaining_budget(&self) -> f32 {
        self.remaining_budget
    }

    /// Consume budget for a transfer. Returns true if budget was sufficient.
    pub fn consume_budget(&mut self, amount: f32) -> bool {
        if self.remaining_budget >= amount {
            self.remaining_budget -= amount;
            true
        } else {
            false
        }
    }

    /// Compute the effective transfer rate for a given direction.
    /// Holistic→Analytical (intuition surfacing) is fast — multiplied by asymmetry.
    /// Analytical→Holistic (consolidation) is slow — divided by asymmetry.
    /// ADR-0024: Intuition flows freely; consolidation requires effort.
    pub fn effective_rate(&self, direction: Direction) -> f32 {
        match direction {
            Direction::HolisticToAnalytical => self.bandwidth * self.asymmetry,
            Direction::AnalyticalToHolistic => self.bandwidth / self.asymmetry,
        }
    }

    /// Check if a wavefront passes the gate (energy above threshold)
    pub fn passes_gate(&self, energy: f32) -> bool {
        energy >= self.gate_threshold
    }

    /// Record and evaluate an energy-gate decision for a transfer being
    /// CONSIDERED, feeding the efficiency metric. Unlike `passes_gate` (a pure
    /// predicate reused in several contexts), only real transfer decisions call
    /// this, so `gate_passes / gate_attempts` becomes a genuine cross-callosal
    /// success rate rather than the constant 1.0 it used to be (#824).
    pub fn try_gate(&mut self, energy: f32) -> bool {
        self.gate_attempts += 1;
        let passed = self.passes_gate(energy);
        if passed {
            self.gate_passes += 1;
        }
        passed
    }

    /// Apply transfer noise for sub→conscious direction (intuition fuzz).
    /// Returns the noisy version of the input vector.
    pub fn apply_noise(&self, vector: &[f32], direction: Direction) -> Vec<f32> {
        match direction {
            Direction::HolisticToAnalytical => {
                // Add noise proportional to recall_noise
                // Use a simple deterministic noise based on vector content
                vector
                    .iter()
                    .enumerate()
                    .map(|(i, &v)| {
                        let noise = self.recall_noise * ((i as f32 * 0.7 + v * 13.0).sin());
                        v + noise
                    })
                    .collect()
            }
            Direction::AnalyticalToHolistic => {
                // No noise on consolidation — clean transfer
                vector.to_vec()
            }
        }
    }

    /// Compute the balance metric between hemispheres.
    /// Returns a value where:
    ///   0.0 = perfectly balanced
    ///   positive = left-heavy (too much conscious, need more consolidation)
    ///   negative = right-heavy (too much subconscious, need more intuition surfacing)
    pub fn balance_metric(&self, left_total_energy: f32, right_total_energy: f32) -> f32 {
        let total = left_total_energy + right_total_energy;
        if total < 0.001 {
            return 0.0;
        }
        (left_total_energy - right_total_energy) / total
    }

    /// Adjust transfer rates based on current balance.
    /// Called periodically to keep hemispheres from pathological dominance.
    pub fn adjust_for_balance(&mut self, left_total_energy: f32, right_total_energy: f32) {
        let balance = self.balance_metric(left_total_energy, right_total_energy);

        // If left-heavy, increase consolidation rate (boost analytical→holistic)
        // If right-heavy, increase intuition rate (boost holistic→analytical)
        // Adjustment is gentle — capped at ±50% of base asymmetry
        let adjustment = 1.0 + balance.clamp(-0.5, 0.5);

        // asymmetry controls holistic→analytical rate relative to analytical→holistic
        // If balance > 0 (left-heavy), we want LESS asymmetry (more consolidation)
        // If balance < 0 (right-heavy), we want MORE asymmetry (more intuition)
        self.asymmetry = (3.0 / adjustment).clamp(1.0, 5.0);
    }

    /// Route sensory input through the optic chiasm.
    /// Returns the target hemisphere (opposite of where you'd expect).
    /// This crossing creates the energy flow that keeps the callosum active.
    pub fn optic_chiasm_route(&self, _input: &[f32]) -> Hand {
        // Sensory input always enters the OPPOSITE hemisphere
        // In practice, external input goes to Right (subconscious first),
        // then crosses to Left (conscious) via the callosum.
        // This creates mandatory callosal flow for every perception.
        Hand::Right
    }

    /// Record a transfer in the log
    pub fn log_transfer(&mut self, wavefront_id: uuid::Uuid, direction: Direction, energy: f32) {
        let direction_label = match direction {
            Direction::AnalyticalToHolistic => "L→R".to_string(),
            Direction::HolisticToAnalytical => "R→L".to_string(),
        };

        self.transfer_log.push(TransferRecord {
            wavefront_id,
            direction_label,
            energy,
            timestamp: chrono::Utc::now().timestamp_millis(),
        });

        // Trim log if too large
        if self.transfer_log.len() > self.max_log_size {
            self.transfer_log
                .drain(0..self.transfer_log.len() - self.max_log_size);
        }
    }

    /// Get summary statistics about recent transfers
    pub fn transfer_stats(&self) -> CallosumStats {
        let l_to_r = self
            .transfer_log
            .iter()
            .filter(|t| t.direction_label == "L→R")
            .count();
        let r_to_l = self
            .transfer_log
            .iter()
            .filter(|t| t.direction_label == "R→L")
            .count();
        let total_energy: f32 = self.transfer_log.iter().map(|t| t.energy).sum();

        let total = self.transfer_log.len();
        // Efficiency (κ): fraction of CONSIDERED transfers whose energy cleared
        // the gate (see `try_gate`). #824: the old form counted only LOGGED
        // transfers -- which had all already passed the gate at their call site
        // -- so `successful == total` and efficiency was 1.0 by construction, a
        // number that could never report a problem. It is now the real
        // gate-pass rate over decisions made this process (0.0 if none yet).
        let efficiency = if self.gate_attempts > 0 {
            self.gate_passes as f32 / self.gate_attempts as f32
        } else {
            0.0
        };

        CallosumStats {
            total_transfers: total,
            analytical_to_holistic: l_to_r,
            holistic_to_analytical: r_to_l,
            total_energy_transferred: total_energy,
            current_bandwidth: self.bandwidth,
            current_asymmetry: self.asymmetry,
            remaining_budget: self.remaining_budget,
            efficiency,
        }
    }
}

impl Default for CorpusCallosum {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary statistics for the callosum's operation
#[derive(Debug, Clone)]
pub struct CallosumStats {
    pub total_transfers: usize,
    pub analytical_to_holistic: usize,
    pub holistic_to_analytical: usize,
    pub total_energy_transferred: f32,
    pub current_bandwidth: f32,
    pub current_asymmetry: f32,
    pub remaining_budget: f32,
    /// Callosal Efficiency (κ) — ratio of successful resonances to total transfers.
    /// A transfer is "successful" if it persists in the receiving hemisphere.
    /// ADR-0024 CS-5: integration health check.
    pub efficiency: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_callosum_has_budget() {
        let cc = CorpusCallosum::new();
        assert!(cc.has_budget());
        assert!(cc.remaining_budget() > 0.0);
    }

    #[test]
    fn budget_consumption() {
        let mut cc = CorpusCallosum::new();
        let initial = cc.remaining_budget();
        assert!(cc.consume_budget(3.0));
        assert!((cc.remaining_budget() - (initial - 3.0)).abs() < 0.001);

        // Consume rest
        assert!(cc.consume_budget(cc.remaining_budget()));
        assert!(!cc.has_budget() || cc.remaining_budget() < 0.001);

        // Can't consume more
        assert!(!cc.consume_budget(1.0));
    }

    #[test]
    fn budget_reset() {
        let mut cc = CorpusCallosum::new();
        cc.consume_budget(5.0);
        cc.reset_budget();
        assert!((cc.remaining_budget() - cc.bandwidth).abs() < 0.001);
    }

    #[test]
    fn gate_threshold() {
        let cc = CorpusCallosum::new();
        assert!(!cc.passes_gate(0.1)); // Below threshold
        assert!(cc.passes_gate(0.5)); // Above threshold
        assert!(cc.passes_gate(cc.gate_threshold)); // Exactly at threshold
    }

    #[test]
    fn efficiency_reflects_gate_pass_rate_not_constant_one() {
        // #824 regression: efficiency must drop below 1.0 when considered
        // transfers do not clear the gate. The old metric scored only logged
        // (already-passed) transfers, so it was always exactly 1.0.
        let mut cc = CorpusCallosum::new(); // gate_threshold default 0.3
        assert!(cc.try_gate(0.5)); // pass
        assert!(cc.try_gate(0.9)); // pass
        assert!(!cc.try_gate(0.1)); // fail
        assert!(!cc.try_gate(0.2)); // fail
        let eff = cc.transfer_stats().efficiency;
        assert!(
            (eff - 0.5).abs() < 1e-6,
            "efficiency should be 2 passes / 4 attempts = 0.5, got {eff}"
        );
        assert!(
            eff < 1.0,
            "efficiency must be able to report below 1.0 (#824)"
        );
    }

    #[test]
    fn efficiency_is_zero_before_any_gate_decision() {
        // No transfers considered yet -> nothing to report, not a spurious 1.0.
        let cc = CorpusCallosum::new();
        assert_eq!(cc.transfer_stats().efficiency, 0.0);
    }

    #[test]
    fn asymmetric_rates() {
        let cc = CorpusCallosum::new();
        let intuition_rate = cc.effective_rate(Direction::HolisticToAnalytical);
        let consolidation_rate = cc.effective_rate(Direction::AnalyticalToHolistic);
        assert!(
            intuition_rate > consolidation_rate,
            "Intuition should flow faster than consolidation"
        );
    }

    #[test]
    fn noise_only_on_intuition() {
        let cc = CorpusCallosum::new();
        let vec = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Consolidation (analytical→holistic) should be clean
        let consolidated = cc.apply_noise(&vec, Direction::AnalyticalToHolistic);
        assert_eq!(vec, consolidated);

        // Intuition (holistic→analytical) should be noisy
        let intuited = cc.apply_noise(&vec, Direction::HolisticToAnalytical);
        assert_ne!(vec, intuited); // Should differ due to noise
    }

    #[test]
    fn balance_metric_symmetric() {
        let cc = CorpusCallosum::new();

        // Equal energy = balanced
        assert!((cc.balance_metric(10.0, 10.0) - 0.0).abs() < 0.001);

        // Left-heavy = positive
        assert!(cc.balance_metric(20.0, 10.0) > 0.0);

        // Right-heavy = negative
        assert!(cc.balance_metric(10.0, 20.0) < 0.0);
    }

    #[test]
    fn balance_adjustment_shifts_asymmetry() {
        let mut cc = CorpusCallosum::new();
        let initial_asymmetry = cc.asymmetry;

        // Left-heavy: should reduce asymmetry (more consolidation)
        cc.adjust_for_balance(30.0, 10.0);
        assert!(
            cc.asymmetry < initial_asymmetry,
            "Left-heavy should reduce asymmetry"
        );

        // Reset and test right-heavy
        cc.asymmetry = initial_asymmetry;
        cc.adjust_for_balance(10.0, 30.0);
        assert!(
            cc.asymmetry > initial_asymmetry,
            "Right-heavy should increase asymmetry"
        );
    }

    #[test]
    fn optic_chiasm_routes_to_opposite() {
        let cc = CorpusCallosum::new();
        let input = vec![1.0, 2.0, 3.0];
        let target = cc.optic_chiasm_route(&input);
        assert_eq!(
            target,
            Hand::Right,
            "Optic chiasm should route to opposite hemisphere"
        );
    }

    #[test]
    fn transfer_logging() {
        let mut cc = CorpusCallosum::new();
        let id = uuid::Uuid::new_v4();
        cc.log_transfer(id, Direction::HolisticToAnalytical, 0.8);

        let stats = cc.transfer_stats();
        assert_eq!(stats.total_transfers, 1);
        assert_eq!(stats.holistic_to_analytical, 1);
        assert_eq!(stats.analytical_to_holistic, 0);
    }
}
