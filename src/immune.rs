//! Memory immune system — ADR-0035 Capability 4 / Wave 2 (Tasks 2.1–2.2).
//!
//! Pure, transport-agnostic cognitive hygiene: classify each memory's health
//! against five flags (duplicate, contradictory, stale, low-confidence,
//! hallucinated) and recommend the *least destructive* lifecycle action. Holds
//! NO I/O — the CLI/HRM layer extracts [`MemorySignals`] from the store, calls
//! these, and (for 2.2) applies the recommended actions reversibly.
//!
//! Contradiction is a pair-level signal, so it is folded in at the batch level
//! ([`classify_batch`]) by reusing Wave 1's `sensemaking::detect_contradictions`.

use crate::sensemaking::{detect_contradictions, PeerRecall};

/// A health concern detected on a memory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealthFlag {
    /// Near-identical to a higher-amplitude sibling (redundant).
    Duplicate,
    /// Same subject as another memory but opposed stance (from contradiction pass).
    Contradictory,
    /// Old and not recently accessed.
    Stale,
    /// Amplitude below the confidence floor.
    LowConfidence,
    /// Generated during dreaming and never corroborated.
    Hallucinated,
}

/// Reversible lifecycle actions, ordered least → most destructive. The immune
/// system NEVER hard-deletes; `Expire` is the existing zero-amplitude ghost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Clean,
    MarkForReview,
    DownRank,
    Quarantine,
    Expire,
}

/// Per-memory signals the HRM already tracks.
#[derive(Clone, Debug)]
pub struct MemorySignals {
    pub id: String,
    pub amplitude: f32,
    pub age_days: f32,
    pub recent_access: bool,
    pub hallucinated: bool,
    /// Cosine similarity to the most-similar OTHER memory, in [0, 1].
    pub max_sibling_similarity: f32,
    /// True when that most-similar sibling has higher amplitude (so THIS is the
    /// redundant copy, not the canonical one).
    pub sibling_higher_amp: bool,
}

/// Tunable thresholds (autoresearch-sweepable later).
#[derive(Clone, Debug)]
pub struct Thresholds {
    pub dedup: f32,
    pub low_amp: f32,
    pub stale_days: f32,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            dedup: 0.95,
            low_amp: 0.10,
            stale_days: 90.0,
        }
    }
}

/// The verdict for one memory.
#[derive(Clone, Debug)]
pub struct HealthVerdict {
    pub id: String,
    pub flags: Vec<HealthFlag>,
    /// Severity in [0, 1] — weighted accumulation of flags.
    pub severity: f32,
    pub action: Action,
}

impl HealthVerdict {
    pub fn is_clean(&self) -> bool {
        self.flags.is_empty()
    }
}

/// Per-flag severity weight. Duplicates and hallucinations are the most actionable.
fn flag_weight(f: HealthFlag) -> f32 {
    match f {
        HealthFlag::Duplicate => 0.45,
        HealthFlag::Hallucinated => 0.40,
        HealthFlag::Contradictory => 0.35,
        HealthFlag::LowConfidence => 0.25,
        HealthFlag::Stale => 0.20,
    }
}

/// Map accumulated severity to the least-destructive action that fits.
fn action_for(severity: f32, flags: &[HealthFlag]) -> Action {
    if flags.is_empty() {
        return Action::Clean;
    }
    // A clear duplicate is quarantined (redundant, safe to hold aside) even at
    // modest severity; everything else escalates by severity.
    if flags.contains(&HealthFlag::Duplicate) && severity >= 0.45 {
        return Action::Quarantine;
    }
    match severity {
        s if s >= 0.70 => Action::Expire,
        s if s >= 0.45 => Action::Quarantine,
        s if s >= 0.25 => Action::DownRank,
        _ => Action::MarkForReview,
    }
}

/// Classify one memory from its signals (the duplicate/stale/low-conf/halluc
/// flags). Contradiction is added at the batch level — see [`classify_batch`].
pub fn classify_memory_health(s: &MemorySignals, t: &Thresholds) -> HealthVerdict {
    let mut flags = Vec::new();
    if s.max_sibling_similarity >= t.dedup && s.sibling_higher_amp {
        flags.push(HealthFlag::Duplicate);
    }
    if s.hallucinated {
        flags.push(HealthFlag::Hallucinated);
    }
    if s.age_days > t.stale_days && !s.recent_access {
        flags.push(HealthFlag::Stale);
    }
    if s.amplitude < t.low_amp {
        flags.push(HealthFlag::LowConfidence);
    }
    // Severity saturates at 1.0 so a memory with many flags can't exceed full scale.
    let severity = flags.iter().map(|f| flag_weight(*f)).sum::<f32>().min(1.0);
    let action = action_for(severity, &flags);
    HealthVerdict {
        id: s.id.clone(),
        flags,
        severity,
        action,
    }
}

/// One memory's signals plus the content/phase needed for the contradiction pass.
#[derive(Clone, Debug)]
pub struct MemoryForReview {
    pub signals: MemorySignals,
    pub content: String,
    pub phase: f32,
}

/// Classify a whole memory set, folding in cross-memory contradictions. Both
/// members of each opposed pair (content-similar, phase-opposed) get the
/// `Contradictory` flag and a re-scored verdict. Returns only the at-risk
/// memories (non-`Clean`), sorted by severity desc.
pub fn classify_batch(
    memories: &[MemoryForReview],
    t: &Thresholds,
    sim: impl Fn(&str, &str) -> f32,
) -> Vec<HealthVerdict> {
    let mut verdicts: Vec<HealthVerdict> = memories
        .iter()
        .map(|m| classify_memory_health(&m.signals, t))
        .collect();

    // Contradiction pass over the same set, reusing the Wave 1 detector.
    let recalls: Vec<PeerRecall> = memories
        .iter()
        .map(|m| PeerRecall {
            peer_id: m.signals.id.clone(),
            content: m.content.clone(),
            similarity: 1.0,
            amplitude: m.signals.amplitude,
            phase: m.phase,
            confidence: 1.0,
        })
        .collect();
    let contradictions =
        detect_contradictions(&recalls, &sim, t.dedup.min(0.5), std::f32::consts::FRAC_PI_2);

    if !contradictions.is_empty() {
        // Mark both members of any opposed pair. Match by content (the detector
        // carries content, not ids); flag every memory whose content appears.
        for c in &contradictions {
            for (i, m) in memories.iter().enumerate() {
                if (m.content == c.a || m.content == c.b)
                    && !verdicts[i].flags.contains(&HealthFlag::Contradictory)
                {
                    verdicts[i].flags.push(HealthFlag::Contradictory);
                    verdicts[i].severity = verdicts[i]
                        .flags
                        .iter()
                        .map(|f| flag_weight(*f))
                        .sum::<f32>()
                        .min(1.0);
                    verdicts[i].action = action_for(verdicts[i].severity, &verdicts[i].flags);
                }
            }
        }
    }

    let mut at_risk: Vec<HealthVerdict> = verdicts.into_iter().filter(|v| !v.is_clean()).collect();
    at_risk.sort_by(|a, b| b.severity.total_cmp(&a.severity));
    at_risk
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(id: &str, amp: f32, age: f32, recent: bool, hall: bool, sib: f32, sib_hi: bool) -> MemorySignals {
        MemorySignals {
            id: id.into(),
            amplitude: amp,
            age_days: age,
            recent_access: recent,
            hallucinated: hall,
            max_sibling_similarity: sib,
            sibling_higher_amp: sib_hi,
        }
    }

    #[test]
    fn healthy_memory_is_clean() {
        let v = classify_memory_health(
            &sig("a", 0.9, 5.0, true, false, 0.3, false),
            &Thresholds::default(),
        );
        assert!(v.is_clean());
        assert_eq!(v.action, Action::Clean);
    }

    #[test]
    fn duplicate_is_quarantined_not_deleted() {
        let v = classify_memory_health(
            &sig("dup", 0.8, 1.0, true, false, 0.97, true),
            &Thresholds::default(),
        );
        assert!(v.flags.contains(&HealthFlag::Duplicate));
        assert_eq!(v.action, Action::Quarantine);
    }

    #[test]
    fn canonical_copy_not_flagged_duplicate() {
        // High similarity but THIS one has the higher amplitude -> it's the keeper.
        let v = classify_memory_health(
            &sig("keep", 0.9, 1.0, true, false, 0.97, false),
            &Thresholds::default(),
        );
        assert!(!v.flags.contains(&HealthFlag::Duplicate));
    }

    #[test]
    fn stale_low_conf_hallucinated_expires() {
        let v = classify_memory_health(
            &sig("rot", 0.05, 200.0, false, true, 0.1, false),
            &Thresholds::default(),
        );
        assert!(v.flags.contains(&HealthFlag::Stale));
        assert!(v.flags.contains(&HealthFlag::LowConfidence));
        assert!(v.flags.contains(&HealthFlag::Hallucinated));
        assert_eq!(v.action, Action::Expire); // severity >= 0.70
    }

    #[test]
    fn batch_flags_both_sides_of_contradiction() {
        let mem = |id: &str, content: &str, phase: f32| MemoryForReview {
            signals: sig(id, 0.8, 1.0, true, false, 0.2, false),
            content: content.into(),
            phase,
        };
        let memories = vec![
            mem("a", "rates rise", 0.0),
            mem("b", "rates rise", std::f32::consts::PI), // opposed stance
            mem("c", "unrelated", 0.0),
        ];
        let sim = |x: &str, y: &str| if x == y { 1.0 } else { 0.0 };
        let at_risk = classify_batch(&memories, &Thresholds::default(), sim);
        let flagged: Vec<&str> = at_risk
            .iter()
            .filter(|v| v.flags.contains(&HealthFlag::Contradictory))
            .map(|v| v.id.as_str())
            .collect();
        assert!(flagged.contains(&"a") && flagged.contains(&"b"));
        assert!(!flagged.contains(&"c"));
    }
}
