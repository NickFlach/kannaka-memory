//! T2.1 — recall-scenario benchmark export (`kannaka-recall-bench/1`).
//!
//! Dumps real recall events as JSON for the Quantum-Wave correspondence
//! benchmark (issue #476): each scenario is one recall event — a ranked set of
//! candidate amplitudes plus the classical argmax — that a quantum backend can
//! reproduce via amplitude amplification and compare against.
//!
//! **Privacy:** labels are HASHED (SHA-256, non-reversible), so a generated
//! scenario corpus may leave the private repo without any memory content
//! leaking. The hash is stable, so a candidate that IS the query seed keeps the
//! same label as `query_label` — ground truth survives the scrub.

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Wire-format identifier embedded in every export.
pub const RECALL_BENCH_FORMAT: &str = "kannaka-recall-bench/1";

/// Max candidates per scenario. 16 = 4 qubits, keeping circuits shallow on
/// noisy hardware (#476).
pub const MAX_CANDIDATES: usize = 16;

/// Stable, non-reversible label for a memory's content: the first 64 bits of
/// SHA-256, hex-encoded. Collision-safe at corpus scale and content can't be
/// recovered, so a scenario file is safe to publish. Identical content →
/// identical label (preserves the query↔target linkage post-scrub).
pub fn hash_label(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    let mut s = String::with_capacity(16);
    for b in digest.iter().take(8) {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// One recall candidate: a hashed label and its resonance amplitude.
#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    /// Hashed content label — never raw content.
    pub label: String,
    /// Resonance amplitude for this candidate under the scenario's query.
    pub amplitude: f32,
}

/// One recall event, ready for the correspondence benchmark.
#[derive(Debug, Clone, Serialize)]
pub struct RecallScenario {
    /// Hashed label of the query seed memory (matches its own candidate label).
    pub query_label: String,
    /// Candidates in recall-rank order, capped at [`MAX_CANDIDATES`].
    pub candidates: Vec<Candidate>,
    /// Index into `candidates` of the highest-amplitude candidate — the answer
    /// classical (argmax) recall returns. `None` iff there are no candidates.
    pub classical_argmax: Option<usize>,
    /// Which store the recall ran against: `"right"` (chiral deep hemisphere,
    /// which recall resonates on) or `"flat"` (single medium).
    pub hemisphere: String,
    /// RFC3339 time this scenario was exported.
    pub timestamp: String,
}

/// The full export envelope.
#[derive(Debug, Clone, Serialize)]
pub struct RecallBench {
    /// Always [`RECALL_BENCH_FORMAT`].
    pub format: String,
    /// RFC3339 export time.
    pub generated_at: String,
    /// Number of scenarios actually emitted.
    pub n: usize,
    pub scenarios: Vec<RecallScenario>,
}

/// Assemble a scenario from a query label and ranked candidates.
///
/// Truncates to [`MAX_CANDIDATES`] and computes the classical argmax over the
/// KEPT candidates (so the argmax always indexes into the emitted list). Pure —
/// no I/O — so the argmax/cap/hash contract is unit-tested directly.
pub fn build_scenario(
    query_label: String,
    mut candidates: Vec<Candidate>,
    hemisphere: &str,
    timestamp: String,
) -> RecallScenario {
    candidates.truncate(MAX_CANDIDATES);
    let classical_argmax = candidates
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.amplitude
                .partial_cmp(&b.amplitude)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i);
    RecallScenario {
        query_label,
        candidates,
        classical_argmax,
        hemisphere: hemisphere.to_string(),
        timestamp,
    }
}

/// A deterministic shuffle of `0..len`, seeded so an export is reproducible on
/// the same corpus. Callers walk this order and keep the first `n` seeds whose
/// recall yields candidates (some may be empty on a tiny/degenerate field).
pub fn shuffled_indices(len: usize, seed: u64) -> Vec<usize> {
    use rand::seq::SliceRandom;
    use rand::SeedableRng;
    let mut idx: Vec<usize> = (0..len).collect();
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed);
    idx.shuffle(&mut rng);
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_label_is_stable_and_hides_content() {
        let a = hash_label("the quiet wave settles");
        let b = hash_label("the quiet wave settles");
        let c = hash_label("a different memory");
        assert_eq!(a, b, "same content → same label");
        assert_ne!(a, c, "different content → different label");
        assert_eq!(a.len(), 16, "64-bit hex label");
        assert!(a.chars().all(|ch| ch.is_ascii_hexdigit()));
        // The raw content must not be recoverable from / present in the label.
        assert!(!a.contains("quiet"));
    }

    #[test]
    fn build_scenario_caps_at_16_and_reindexes_argmax() {
        // 20 candidates; the global max is beyond the 16-cap and must NOT be the
        // argmax — the argmax is over the KEPT candidates only.
        let mut cands: Vec<Candidate> = (0..20)
            .map(|i| Candidate { label: format!("{i:016x}"), amplitude: i as f32 * 0.01 })
            .collect();
        // Make candidate #3 (kept) the strongest within the first 16.
        cands[3].amplitude = 0.99;
        let s = build_scenario("q".into(), cands, "flat", "t".into());
        assert_eq!(s.candidates.len(), MAX_CANDIDATES, "capped to 16 (4 qubits)");
        assert_eq!(s.classical_argmax, Some(3), "argmax indexes into the kept set");
    }

    #[test]
    fn build_scenario_empty_has_no_argmax() {
        let s = build_scenario("q".into(), Vec::new(), "right", "t".into());
        assert_eq!(s.classical_argmax, None);
        assert!(s.candidates.is_empty());
    }

    #[test]
    fn shuffled_indices_is_a_seeded_permutation() {
        let a = shuffled_indices(100, 42);
        let b = shuffled_indices(100, 42);
        let c = shuffled_indices(100, 7);
        assert_eq!(a, b, "same seed → same order (reproducible export)");
        assert_ne!(a, c, "different seed → different order");
        let mut sorted = a.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..100).collect::<Vec<_>>(), "a true permutation, no dupes/drops");
    }

    #[test]
    fn envelope_serializes_with_format_tag() {
        let s = build_scenario(
            hash_label("seed"),
            vec![Candidate { label: hash_label("seed"), amplitude: 1.0 }],
            "flat",
            "2026-07-01T00:00:00+00:00".into(),
        );
        let bench = RecallBench {
            format: RECALL_BENCH_FORMAT.to_string(),
            generated_at: "2026-07-01T00:00:00+00:00".into(),
            n: 1,
            scenarios: vec![s],
        };
        let v: serde_json::Value = serde_json::to_value(&bench).unwrap();
        assert_eq!(v["format"], "kannaka-recall-bench/1");
        assert_eq!(v["n"], 1);
        assert_eq!(v["scenarios"][0]["classical_argmax"], 0);
        // No raw content anywhere — only hashed labels.
        assert_eq!(v["scenarios"][0]["query_label"], serde_json::json!(hash_label("seed")));
    }
}
