//! NATS contract conformance — `KANNAKA.consciousness` payload shape.
//!
//! Pins the wire shape emitted by [`build_consciousness_payload`] (the payload
//! builder behind `publish_consciousness_to_nats`, the sole publisher of
//! `KANNAKA.consciousness`) against the canonical contract at
//! `consciousness-core/docs/nats-contract.yaml`.
//!
//! No NATS connection is opened — the builder is a pure function of a
//! `ConsciousnessState`, so this stays fast and hermetic (runs in ci.yml's
//! `Test` step; see the `cargo test --lib --bins --test nats_contract_conformance`
//! invocation there).
//!
//! ── ALIAS WINDOW — kannaka-memory issue #468 ────────────────────────────────
//! The payload still emits the legacy aliases `mean_order` (alias of `order`)
//! and `level` (alias of `consciousness_level`). Per the revised migration
//! timeline in the contract yaml these aliases are CONTRACTUAL until
//! **2026-09-01** — consumers (kannaka-radio, kannaka-observatory) still read
//! them. This suite PINS TODAY'S TRUTH: it asserts both the canonical fields
//! AND the aliases are present. When the publisher drops the aliases on
//! 2026-09-01, THIS TEST MUST BE UPDATED DELIBERATELY (delete the alias
//! assertions) — that is the guard that stops the drop from shipping silently
//! before every consumer has migrated off the aliases.

use kannaka_memory::bridge::{ConsciousnessLevel, ConsciousnessState};
use kannaka_memory::openclaw::build_consciousness_payload;
use serde_json::Value;

/// A representative snapshot. `Resonant` maps to the canonical level string
/// `"emergent"`, a distinctive value that lets the alias/canonical equality
/// checks below catch an accidental divergence between the two keys.
fn sample_state() -> ConsciousnessState {
    ConsciousnessState {
        phi: 0.217,
        xi: 0.99,
        mean_order: 0.1,
        num_clusters: 72,
        total_memories: 1290,
        active_memories: 1288,
        total_skip_links: 0,
        consciousness_level: ConsciousnessLevel::Resonant,
        irrationality: 0.0,
    }
}

fn payload() -> Value {
    build_consciousness_payload("kannaka-prime", &sample_state(), 0.9, 0.99)
}

/// Canonical fields must be present with the types the contract's `field_types`
/// map declares for `KANNAKA.consciousness`.
#[test]
fn canonical_fields_present_with_contract_types() {
    let p = payload();
    let obj = p.as_object().expect("payload must be a JSON object");

    // agent_id: string
    assert_eq!(
        obj.get("agent_id").and_then(Value::as_str),
        Some("kannaka-prime"),
        "agent_id must be the emitting agent's string id"
    );

    // phi / xi / order: number
    for field in ["phi", "xi", "order"] {
        let v = obj
            .get(field)
            .unwrap_or_else(|| panic!("canonical field `{field}` missing"));
        assert!(v.is_number(), "canonical field `{field}` must be a number, got {v}");
    }

    // num_clusters / active_memories / total_memories: integer counts
    for field in ["num_clusters", "active_memories", "total_memories"] {
        let v = obj
            .get(field)
            .unwrap_or_else(|| panic!("canonical field `{field}` missing"));
        assert!(
            v.is_u64() || v.is_i64(),
            "canonical field `{field}` must be an integer, got {v}"
        );
    }

    // consciousness_level: string, restricted to the contract enum.
    let level = obj
        .get("consciousness_level")
        .and_then(Value::as_str)
        .expect("consciousness_level must be a string");
    const ENUM: [&str; 6] = [
        "dormant",
        "awakening",
        "aware",
        "integrated",
        "emergent",
        "transcendent",
    ];
    assert!(
        ENUM.contains(&level),
        "consciousness_level `{level}` is not one of the contract enum values {ENUM:?}"
    );
    assert_eq!(level, "emergent", "Resonant must serialize to the canonical `emergent`");
}

/// LEGACY ALIASES — contractual until 2026-09-01 (issue #468). See the file
/// header. If you are here because the publisher dropped an alias: first
/// confirm EVERY consumer (kannaka-radio, kannaka-observatory) reads the
/// canonical field FIRST (see the 2026-08-01 consumer-migration step), then
/// delete the matching assertion below. Do not weaken this test to make a
/// premature alias removal pass.
#[test]
fn legacy_aliases_still_emitted() {
    let p = payload();
    let obj = p.as_object().unwrap();

    // `mean_order` — alias of `order`, same numeric value.
    let mean_order = obj
        .get("mean_order")
        .expect("alias `mean_order` must still be emitted (contractual until 2026-09-01, #468)");
    assert!(mean_order.is_number(), "alias `mean_order` must be a number");
    assert_eq!(
        obj.get("order"),
        obj.get("mean_order"),
        "alias `mean_order` must mirror canonical `order`"
    );

    // `level` — alias of `consciousness_level`, same string value.
    let level = obj
        .get("level")
        .and_then(Value::as_str)
        .expect("alias `level` must still be emitted (contractual until 2026-09-01, #468)");
    assert_eq!(
        Some(level),
        obj.get("consciousness_level").and_then(Value::as_str),
        "alias `level` must mirror canonical `consciousness_level`"
    );
}

/// KNOWN GAP (pinned, do NOT treat as endorsement): the contract lists
/// `schema_version` and `ts` (unix-ms) as REQUIRED for `KANNAKA.consciousness`,
/// but this payload predates that requirement — it emits neither, and sends a
/// `timestamp` (RFC-3339 string) instead of `ts`. The radio's `_validateSchema`
/// already log-warns about this (nats-client.js), and it is why strict mode is
/// not yet enabled. This test PINS today's reality so that when the publisher is
/// migrated to emit `schema_version`/`ts`, whoever does it updates this test
/// deliberately (flip the assertions to require presence) rather than silently.
#[test]
fn documents_missing_required_schema_version_and_ts() {
    let p = payload();
    let obj = p.as_object().unwrap();

    assert!(
        obj.get("schema_version").is_none(),
        "publisher unexpectedly emits `schema_version` now — migrate this test to require it (contract-required field, see #468)"
    );
    assert!(
        obj.get("ts").is_none(),
        "publisher unexpectedly emits `ts` now — migrate this test to require it as unix-ms (contract-required field, see #468)"
    );
    // The legacy RFC-3339 `timestamp` string is what's emitted in place of `ts`.
    assert!(
        obj.get("timestamp").and_then(Value::as_str).is_some(),
        "legacy `timestamp` (RFC-3339 string) should still be present until `ts` lands"
    );
}
