//! Temporal truth reasoning — ADR-0035 Capability 8 / Wave 3 Task 3.2.
//!
//! Pure logic for reasoning about *when* a fact is true, layered on top of the
//! existing amplitude / `effective_strength` decay. No I/O — operates on a
//! [`TemporalSpec`] (the temporal bounds + amplitude) plus a caller-supplied
//! `now`, so it is deterministic and unit-testable.
//!
//! The reasoning ships decoupled from persistence: today [`TemporalSpec::from_memory`]
//! derives `observed_at` from `created_at` and leaves `effective_at`/`expires_at`
//! unknown, so every existing memory reads as `Current` (exactly today's
//! behavior). Persisting the three temporal fields on `HyperMemory` — and
//! populating the spec from them — is the focused follow-up (Task 3.2b): it
//! touches ~20 struct-literal sites and is done deliberately rather than rushed.

use crate::memory::HyperMemory;
use chrono::{DateTime, Utc};

/// A fact's truth status at a given instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalStatus {
    /// True now.
    Current,
    /// Becomes true later (`effective_at` is in the future).
    Future,
    /// Was true but has passed its `expires_at`.
    Expired,
}

/// The temporal bounds of a fact plus the amplitude its confidence rides on.
#[derive(Clone, Debug)]
pub struct TemporalSpec {
    pub amplitude: f32,
    /// When this agent observed/learned the fact.
    pub observed_at: DateTime<Utc>,
    /// When the fact became true (None = always / unknown).
    pub effective_at: Option<DateTime<Utc>>,
    /// When the fact is known to stop being true (None = no known expiry).
    pub expires_at: Option<DateTime<Utc>>,
}

impl TemporalSpec {
    /// Derive a spec from a memory (Task 3.2b: temporal fields now persisted on
    /// `HyperMemory`). `observed_at` falls back to `created_at` when unset, so a
    /// memory with no temporal bounds reads as `Current` exactly as before.
    pub fn from_memory(mem: &HyperMemory) -> Self {
        Self {
            amplitude: mem.amplitude,
            observed_at: mem.observed_at.unwrap_or(mem.created_at),
            effective_at: mem.effective_at,
            expires_at: mem.expires_at,
        }
    }
}

/// The temporal status at `now`. `Expired` takes precedence over `Future`
/// (a fact past its expiry is expired regardless of effective date).
pub fn temporal_status(spec: &TemporalSpec, now: DateTime<Utc>) -> TemporalStatus {
    if let Some(exp) = spec.expires_at {
        if now >= exp {
            return TemporalStatus::Expired;
        }
    }
    if let Some(eff) = spec.effective_at {
        if now < eff {
            return TemporalStatus::Future;
        }
    }
    TemporalStatus::Current
}

/// True iff the fact is true at `now`.
pub fn is_current(spec: &TemporalSpec, now: DateTime<Utc>) -> bool {
    temporal_status(spec, now) == TemporalStatus::Current
}

/// Confidence folding amplitude with temporal validity, in [0, 1]:
/// - `Future` / `Expired` → 0.0 (not true now).
/// - `Current` with a known expiry → amplitude linearly faded from the
///   observation instant toward expiry (fresher = more confident).
/// - `Current` with no expiry → amplitude (clamped).
pub fn effective_confidence(spec: &TemporalSpec, now: DateTime<Utc>) -> f32 {
    if temporal_status(spec, now) != TemporalStatus::Current {
        return 0.0;
    }
    let base = spec.amplitude.clamp(0.0, 1.0);
    match spec.expires_at {
        Some(exp) => {
            let span = (exp - spec.observed_at).num_seconds().max(1) as f32;
            let elapsed = (now - spec.observed_at).num_seconds().max(0) as f32;
            let validity = (1.0 - elapsed / span).clamp(0.0, 1.0);
            base * validity
        }
        None => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn spec(amp: f32, observed: DateTime<Utc>) -> TemporalSpec {
        TemporalSpec {
            amplitude: amp,
            observed_at: observed,
            effective_at: None,
            expires_at: None,
        }
    }

    #[test]
    fn no_bounds_is_current_with_amplitude_confidence() {
        let now = Utc::now();
        let s = spec(0.8, now - Duration::days(10));
        assert_eq!(temporal_status(&s, now), TemporalStatus::Current);
        assert!(is_current(&s, now));
        assert!((effective_confidence(&s, now) - 0.8).abs() < 1e-6);
    }

    #[test]
    fn expired_fact_has_zero_confidence() {
        let now = Utc::now();
        let mut s = spec(0.9, now - Duration::days(30));
        s.expires_at = Some(now - Duration::days(1));
        assert_eq!(temporal_status(&s, now), TemporalStatus::Expired);
        assert_eq!(effective_confidence(&s, now), 0.0);
    }

    #[test]
    fn future_fact_is_not_yet_true() {
        let now = Utc::now();
        let mut s = spec(0.9, now);
        s.effective_at = Some(now + Duration::days(5));
        assert_eq!(temporal_status(&s, now), TemporalStatus::Future);
        assert_eq!(effective_confidence(&s, now), 0.0);
    }

    #[test]
    fn confidence_fades_toward_expiry() {
        let start = Utc::now() - Duration::days(50);
        let mut s = spec(1.0, start);
        s.expires_at = Some(start + Duration::days(100));
        let now = start + Duration::days(50); // halfway
        let c = effective_confidence(&s, now);
        assert!((c - 0.5).abs() < 0.02, "expected ~0.5, got {c}");
    }

    #[test]
    fn boundary_at_expiry_is_expired() {
        let now = Utc::now();
        let mut s = spec(0.7, now - Duration::days(2));
        s.expires_at = Some(now);
        assert_eq!(temporal_status(&s, now), TemporalStatus::Expired);
    }
}
