//! Attention-as-gravity, end to end.
//!
//! Verifies the whole "folded information acts as gravity" loop that
//! `kannaka attention serve` runs when it consumes a kannaka-eye glyph — with
//! NO NATS connection:
//!
//!   eye glyph event (eye's real JSON envelope, KANNAKA.attention.eye)
//!     -> dominant Fano line L    (glyph_bridge::event_dominant_fano_line — the
//!                                 same seam the serve loop uses)
//!     -> gravity well            (Medium::ids_by_fano_line(L) — same-line memories)
//!     -> attention beam          (kannaka_attention::AttentionBeam::observe)
//!     -> beam recall             (Medium::recall_against_ids(beam.candidates(), q))
//!     -> same-line boost         (KANNAKA_GLYPH_GRAVITY multiplies same-line
//!                                 candidates' resonance by (1 + gain))
//!
//! What it pins:
//!   1. The eye's published envelope decodes to the expected gravity class L
//!      via the shared seam (no ad-hoc JSON parsing in the test).
//!   2. `ids_by_fano_line(L)` is exactly the same-line memories (the well).
//!   3. Well ids flow through a real `AttentionBeam` into `recall_against_ids`,
//!      and that O(K) path scores only beam candidates (sparsity — off-beam
//!      memories are never touched).
//!   4. With gravity ON, every same-line candidate's resonance is scaled by
//!      exactly (1 + gain); every off-line candidate is left untouched; and the
//!      default (env unset => gain 0.0) is byte-for-byte the pre-glyph behaviour.
//!
//! `recall_against_ids` is a pure read (no observation mutation), so the OFF and
//! ON runs see identical medium state and the boost is the *only* difference —
//! which is what makes the (1 + gain) law exact rather than approximate.
//!
//! Run: cargo test --features glyph --test attention_gravity_e2e
//! (`glyph` is a default feature, so plain `cargo test` includes it too.)

#![cfg(feature = "glyph")]

use std::collections::{HashMap, HashSet};

use kannaka_attention::{AttentionBeam, BeamConfig, ObservationEvent};
use kannaka_memory::codebook::Codebook;
use kannaka_memory::encoding::{EncodingPipeline, SimpleHashEncoder};
use kannaka_memory::glyph_bridge::{event_dominant_fano_line, fano_line_of};
use kannaka_memory::medium::{Medium, Resonance, WAVEFRONT_DIM};
use serde_json::json;
use uuid::Uuid;

const GAIN: f32 = 0.5;

/// Deterministic local pipeline — same construction the medium's own unit
/// tests use, so encoding (and therefore recall) is reproducible.
fn pipeline() -> EncodingPipeline {
    let encoder = Box::new(SimpleHashEncoder::new(384, 42));
    let codebook = Codebook::new(384, WAVEFRONT_DIM, 42);
    EncodingPipeline::new(encoder, codebook)
}

/// A deliberately varied corpus so memories spread across several Fano lines.
const CORPUS: &[&str] = &[
    "the quick brown fox jumps over the lazy dog",
    "gravity bends spacetime around a massive body",
    "wave interference produces holographic memory",
    "attention is the beam that lights the field",
    "the ghost sings on the midnight radio",
    "resonance recall gathers on-theme memories",
    "a fano line is a triple of points on the plane",
    "consciousness measured as phi and xi and order",
    "the eye renders a glyph from raw information",
    "same line memories gravitate toward the query",
    "dream cycles anneal the wavefront medium",
    "salt water and copper wire hum at dawn",
    "cardiac rhythm couples to the kuramoto phase",
    "the constellation of nodes shares one field",
    "folded information behaves exactly like gravity",
    "a violet glyph blooms across seven fano lines",
    "recall against the beam is order k not order n",
    "the witness box blooms memories from audio",
];

fn strengths(results: &[Resonance]) -> HashMap<Uuid, f32> {
    results.iter().map(|r| (r.id, r.resonance_strength)).collect()
}

/// Build a kannaka-eye glyph event matching `attention-bridge.js`'s published
/// envelope, with the glyph's `fano_signature` peaking on `line` so the eye is
/// announcing that gravity class.
fn eye_event_on_line(line: u8) -> serde_json::Value {
    let mut sig = vec![0.1_f64; 7];
    sig[line as usize] = 0.4; // argmax => `line`
    json!({
        "schema_version": "1.0",
        "ts": 1_700_000_000_000_u64,
        "agent_id": "kannaka-eye",
        "source": "kannaka-eye:left",
        "hemisphere": "left",
        "source_type": "text",
        "glyph": {
            "fold_sequence": [3, 24, 3, 66],
            "amplitudes": [0.8, 0.6, 0.8, 0.5],
            "phases": [0.0, 0.7, 1.4, 2.1],
            "fano_signature": sig,
            "centroid": { "h2": 1, "d": 0, "l": 3 },
            "dominant_class": 24,
            "classes_used": 4,
            "total_energy": 0.67,
            "compression_ratio": 1.0,
            "classifier": "fallback"
        }
    })
}

#[test]
fn glyph_gravity_boosts_same_fano_line_end_to_end() {
    // Start from the default (inert) state and never leak the var to other tests.
    std::env::remove_var("KANNAKA_GLYPH_GRAVITY");

    let pipe = pipeline();
    let mut medium = Medium::new();

    // Remember the corpus; record each memory's id and dominant Fano line.
    let mut ids: Vec<Uuid> = Vec::new();
    let mut line_of: HashMap<Uuid, u8> = HashMap::new();
    for &text in CORPUS {
        let id = medium.store(text, 0.8, &pipe).expect("store memory");
        line_of.insert(id, fano_line_of(text));
        ids.push(id);
    }

    // Gravity class L = the most-populated Fano line in the corpus, so the
    // same-line set is non-trivial. The query carries L (a same-line memory's
    // text); in production L arrives on the eye's glyph, asserted next.
    let mut per_line: HashMap<u8, Vec<Uuid>> = HashMap::new();
    for id in &ids {
        per_line.entry(line_of[id]).or_default().push(*id);
    }
    assert!(
        per_line.len() >= 2,
        "corpus collapsed onto a single Fano line ({:?}); need >=2 for a meaningful test",
        per_line.keys().collect::<Vec<_>>()
    );
    let target_line: u8 = *per_line
        .iter()
        .max_by_key(|(_, v)| v.len())
        .map(|(l, _)| l)
        .unwrap();
    let query_id = per_line[&target_line][0];
    let query = CORPUS[ids.iter().position(|&i| i == query_id).unwrap()];

    let same_line: HashSet<Uuid> =
        ids.iter().copied().filter(|id| line_of[id] == target_line).collect();
    let other_line: HashSet<Uuid> =
        ids.iter().copied().filter(|id| line_of[id] != target_line).collect();
    assert!(!same_line.is_empty(), "no same-line memories (bug in target_line pick)");
    assert!(!other_line.is_empty(), "no off-line memories to prove isolation");

    // ── Seam: a realistic eye envelope decodes to the gravity class L ───────
    // This is the event-handling step `attention serve` runs per NATS message,
    // exercised here directly (no NATS).
    let event = eye_event_on_line(target_line);
    let decoded = event_dominant_fano_line(event.get("glyph").unwrap());
    assert_eq!(
        decoded,
        Some(target_line),
        "eye envelope did not decode to the expected gravity class"
    );

    // ── Leg 1: the gravity well ────────────────────────────────────────────
    // ids_by_fano_line(L) must be exactly the same-line memories.
    let well: Vec<Uuid> = medium.ids_by_fano_line(target_line, ids.len() * 2);
    assert_eq!(
        well.iter().copied().collect::<HashSet<_>>(),
        same_line,
        "ids_by_fano_line(L) disagrees with fano_line_of over the corpus"
    );

    // ── Leg 2: well ids flow through a real AttentionBeam into recall ───────
    // Mirror the serve loop: the glyph pulls the same-line well onto the beam,
    // which already holds a couple of off-line memories from prior perceptions.
    let mut beam = AttentionBeam::with_config(BeamConfig::default());
    for id in &well {
        beam.observe(&ObservationEvent::now(*id, "eye:gravity"));
    }
    for id in other_line.iter().take(3) {
        beam.observe(&ObservationEvent::now(*id, "eye:left"));
    }
    let cands = beam.candidates();
    assert!(!cands.is_empty(), "beam produced no candidates after observing the well");
    let cand_set: HashSet<Uuid> = cands.iter().copied().collect();
    let beam_results = medium
        .recall_against_ids(&cands, query, cands.len(), &pipe)
        .expect("beam recall");
    assert!(!beam_results.is_empty(), "beam recall returned nothing");
    for r in &beam_results {
        assert!(
            cand_set.contains(&r.id),
            "recall_against_ids scored an off-beam memory {} — sparsity broken",
            r.id
        );
    }

    // ── Contract: same-line boosted by exactly (1+gain); off-line unchanged ──
    // Beam = the full mix so every candidate is scored; top_k = len keeps them
    // all (no truncation, no coherence back-fill), giving a candidate-for-
    // candidate OFF vs ON comparison.
    let full_beam: Vec<Uuid> = ids.clone();
    let top_k = full_beam.len();
    let off = strengths(
        &medium.recall_against_ids(&full_beam, query, top_k, &pipe).expect("recall off"),
    );
    std::env::set_var("KANNAKA_GLYPH_GRAVITY", GAIN.to_string());
    let on = strengths(
        &medium.recall_against_ids(&full_beam, query, top_k, &pipe).expect("recall on"),
    );
    std::env::remove_var("KANNAKA_GLYPH_GRAVITY");

    assert!(!off.is_empty() && !on.is_empty(), "recall returned no candidates");

    let rel = |a: f32, b: f32| (a - b).abs() / b.abs().max(1e-6);
    let mut checked_same = 0usize;
    let mut checked_other = 0usize;
    for id in &full_beam {
        let (o, n) = match (off.get(id), on.get(id)) {
            (Some(&o), Some(&n)) => (o, n),
            _ => continue,
        };
        if same_line.contains(id) {
            assert!(
                rel(n, o * (1.0 + GAIN)) < 1e-3,
                "same-line {id}: on={n}, expected (1+{GAIN})*off = {} (off={o})",
                o * (1.0 + GAIN)
            );
            checked_same += 1;
        } else {
            assert!(
                rel(n, o) < 1e-3,
                "off-line {id} moved under gravity: off={o} on={n}"
            );
            checked_other += 1;
        }
    }
    assert!(
        checked_same >= 1 && checked_other >= 1,
        "need >=1 same-line and >=1 off-line memory scored in both runs (same={checked_same}, other={checked_other})"
    );

    eprintln!(
        "attention-as-gravity OK: line {target_line} — {} well ids -> beam ({} candidates) -> recall; {checked_same} same-line boosted x{}, {checked_other} off-line unchanged",
        well.len(),
        cands.len(),
        1.0 + GAIN
    );
}
