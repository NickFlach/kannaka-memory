//! Adaptive Rhythm Engine — dynamic heartbeat following arousal dynamics.
//!
//! Arousal follows: dx/dt = f(x) - η·x
//! where f(x) is excitatory input and η is damping.
//! Maps arousal to heartbeat interval: high arousal → short interval.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc, Timelike};
use serde::{Deserialize, Serialize};

/// Persisted rhythm state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RhythmState {
    /// Current heartbeat interval in milliseconds.
    pub current_interval_ms: u64,
    /// Last activity timestamp.
    pub last_activity_ts: DateTime<Utc>,
    /// Arousal level (0.0–1.0).
    pub arousal_level: f64,
    /// Momentum — smoothed arousal velocity for slow-release dynamics.
    pub momentum: f64,
}

impl Default for RhythmState {
    fn default() -> Self {
        Self {
            current_interval_ms: 900_000, // 15 min default
            last_activity_ts: Utc::now(),
            arousal_level: 0.2,
            momentum: 0.0,
        }
    }
}

/// Signal types that excite the rhythm.
#[derive(Debug, Clone, Copy)]
pub enum Signal {
    UserMessage,
    FluxMessage,
    SubagentStarted,
    SubagentFinished,
    Idle,
}

impl Signal {
    /// Excitatory weight for this signal type.
    fn weight(self) -> f64 {
        match self {
            Signal::UserMessage => 0.4,
            Signal::FluxMessage => 0.15,
            Signal::SubagentStarted => 0.1,
            Signal::SubagentFinished => 0.05,
            Signal::Idle => 0.0,
        }
    }
}

/// The adaptive rhythm engine.
pub struct RhythmEngine {
    pub state: RhythmState,
    /// Base damping coefficient η.
    pub damping: f64,
    /// Path to persist state.
    persist_path: Option<PathBuf>,
}

impl RhythmEngine {
    /// Create a new engine, loading persisted state if available.
    pub fn new(data_dir: &Path) -> Self {
        let persist_path = data_dir.join("rhythm_state.json");
        let state = if persist_path.exists() {
            match std::fs::read_to_string(&persist_path) {
                Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
                Err(_) => RhythmState::default(),
            }
        } else {
            RhythmState::default()
        };

        Self {
            state,
            damping: 0.1,
            persist_path: Some(persist_path),
        }
    }

    /// Create an engine without persistence (for testing).
    pub fn in_memory() -> Self {
        Self {
            state: RhythmState::default(),
            damping: 0.1,
            persist_path: None,
        }
    }

    /// Record an excitatory signal and update arousal.
    pub fn signal(&mut self, sig: Signal) {
        let now = Utc::now();
        let dt = (now - self.state.last_activity_ts)
            .num_milliseconds()
            .max(0) as f64
            / 1000.0; // seconds

        // Apply damping over elapsed time: arousal decays exponentially
        let eta = self.effective_damping(now);
        self.state.arousal_level *= (-eta * dt).exp();

        // Add excitatory input
        let excitation = sig.weight();
        self.state.arousal_level = (self.state.arousal_level + excitation).clamp(0.0, 1.0);

        // Update momentum (exponential moving average)
        let alpha = 0.3;
        self.state.momentum = self.state.momentum * (1.0 - alpha) + excitation * alpha;

        self.state.last_activity_ts = now;
        self.state.current_interval_ms = self.compute_interval();

        self.persist();
    }

    /// Convenience methods for specific signal types.
    pub fn signal_user_message(&mut self) {
        self.signal(Signal::UserMessage);
    }

    pub fn signal_flux_message(&mut self) {
        self.signal(Signal::FluxMessage);
    }

    pub fn signal_subagent_started(&mut self) {
        self.signal(Signal::SubagentStarted);
    }

    pub fn signal_subagent_finished(&mut self) {
        self.signal(Signal::SubagentFinished);
    }

    pub fn signal_idle(&mut self) {
        self.signal(Signal::Idle);
    }

    /// Get the current recommended interval in milliseconds.
    pub fn interval_ms(&self) -> u64 {
        self.state.current_interval_ms
    }

    /// Get current arousal level (decayed to now).
    pub fn current_arousal(&self) -> f64 {
        let now = Utc::now();
        let dt = (now - self.state.last_activity_ts)
            .num_milliseconds()
            .max(0) as f64
            / 1000.0;
        let eta = self.effective_damping(now);
        (self.state.arousal_level * (-eta * dt).exp()).clamp(0.0, 1.0)
    }

    /// Compute effective damping coefficient (higher at night, higher when idle).
    fn effective_damping(&self, now: DateTime<Utc>) -> f64 {
        let mut eta = self.damping;

        // Night hours (23:00–08:00 UTC): double damping
        let hour = now.hour();
        if hour >= 23 || hour < 8 {
            eta *= 2.0;
        }

        // Long idle (>30 min since last activity): extra damping
        let idle_secs = (now - self.state.last_activity_ts)
            .num_seconds()
            .max(0) as f64;
        if idle_secs > 1800.0 {
            eta *= 1.5;
        }

        eta
    }

    /// Map arousal to interval.
    fn compute_interval(&self) -> u64 {
        let a = self.state.arousal_level;
        let ms = if a > 0.7 {
            // 2-5 min: linear interpolation
            let t = (a - 0.7) / 0.3; // 0..1
            let min_ms = 120_000.0;
            let max_ms = 300_000.0;
            max_ms - t * (max_ms - min_ms)
        } else if a > 0.3 {
            // 5-15 min
            let t = (a - 0.3) / 0.4;
            let min_ms = 300_000.0;
            let max_ms = 900_000.0;
            max_ms - t * (max_ms - min_ms)
        } else {
            // 15-60 min
            let t = a / 0.3;
            let min_ms = 900_000.0;
            let max_ms = 3_600_000.0;
            max_ms - t * (max_ms - min_ms)
        };
        ms as u64
    }

    /// Persist state to disk.
    fn persist(&self) {
        if let Some(ref path) = self.persist_path {
            if let Ok(json) = serde_json::to_string_pretty(&self.state) {
                let _ = std::fs::write(path, json);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn default_state_is_low_arousal() {
        let engine = RhythmEngine::in_memory();
        assert!(engine.state.arousal_level < 0.3);
        assert!(engine.interval_ms() >= 900_000);
    }

    #[test]
    fn user_message_increases_arousal() {
        let mut engine = RhythmEngine::in_memory();
        let before = engine.state.arousal_level;
        engine.signal_user_message();
        assert!(engine.state.arousal_level > before);
    }

    #[test]
    fn high_arousal_gives_short_interval() {
        let mut engine = RhythmEngine::in_memory();
        // Pump arousal high
        engine.signal_user_message();
        engine.signal_user_message();
        engine.signal_user_message();
        assert!(engine.state.arousal_level > 0.7);
        assert!(engine.interval_ms() <= 300_000); // ≤ 5 min
    }

    #[test]
    fn arousal_decays_over_time() {
        let mut engine = RhythmEngine::in_memory();
        engine.signal_user_message();
        let arousal_after_signal = engine.state.arousal_level;

        // Simulate time passing
        engine.state.last_activity_ts = Utc::now() - Duration::minutes(10);
        engine.signal_idle(); // triggers decay calculation

        assert!(engine.state.arousal_level < arousal_after_signal);
    }

    #[test]
    fn arousal_clamped_to_unit() {
        let mut engine = RhythmEngine::in_memory();
        for _ in 0..20 {
            engine.signal_user_message();
        }
        assert!(engine.state.arousal_level <= 1.0);
    }

    #[test]
    fn interval_mapping_covers_full_range() {
        let mut engine = RhythmEngine::in_memory();

        // Low arousal -> long interval
        engine.state.arousal_level = 0.1;
        engine.state.current_interval_ms = engine.compute_interval();
        assert!(engine.interval_ms() >= 900_000);

        // Medium arousal -> medium interval
        engine.state.arousal_level = 0.5;
        engine.state.current_interval_ms = engine.compute_interval();
        assert!(engine.interval_ms() >= 300_000 && engine.interval_ms() <= 900_000);

        // High arousal -> short interval
        engine.state.arousal_level = 0.9;
        engine.state.current_interval_ms = engine.compute_interval();
        assert!(engine.interval_ms() <= 300_000);
    }
}
