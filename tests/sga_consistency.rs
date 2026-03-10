//! SGA reference consistency tests.
//!
//! Validates that the Rust SGA classifier (GlyphEncoder) produces consistent,
//! deterministic results by comparing against known reference vectors stored
//! in `tests/sga_reference_vectors.json`.
//!
//! Run with: cargo test --features glyph --test sga_consistency

#[cfg(feature = "glyph")]
mod sga_tests {
    use kannaka_memory::glyph_bridge::GlyphEncoder;
    use serde::Deserialize;
    use std::collections::HashSet;
    use std::path::PathBuf;

    /// Mirrors the "expected" block in the reference vectors JSON.
    #[derive(Debug, Deserialize)]
    struct Expected {
        dominant_class: u8,
        centroid: Centroid,
        fano_signature: [f64; 7],
        classes_used: usize,
    }

    #[derive(Debug, Deserialize)]
    struct Centroid {
        h2: u8,
        d: u8,
        l: u8,
    }

    /// One entry in the reference vectors file.
    #[derive(Debug, Deserialize)]
    struct ReferenceVector {
        id: String,
        input_type: String,
        #[serde(default)]
        #[allow(dead_code)]
        description: String,
        /// Present when input_type == "text"
        input: Option<String>,
        /// Present when input_type is "bytes" or "file_pattern"
        input_bytes: Option<Vec<u8>>,
        expected: Expected,
    }

    /// Load the reference vectors from the JSON file next to this test.
    fn load_vectors() -> Vec<ReferenceVector> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push("sga_reference_vectors.json");
        let data = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
        serde_json::from_str(&data)
            .unwrap_or_else(|e| panic!("Failed to parse reference vectors: {}", e))
    }

    /// Convert input to the same f64 representation the classify binary uses:
    /// each byte mapped to byte_value / 255.0
    fn input_to_f64(vec: &ReferenceVector) -> Vec<f64> {
        let raw_bytes: Vec<u8> = match vec.input_type.as_str() {
            "text" => vec
                .input
                .as_ref()
                .expect("text vector must have `input` field")
                .as_bytes()
                .to_vec(),
            "bytes" | "file_pattern" => vec
                .input_bytes
                .as_ref()
                .expect("byte vector must have `input_bytes` field")
                .clone(),
            other => panic!("Unknown input_type: {}", other),
        };
        raw_bytes.iter().map(|&b| b as f64 / 255.0).collect()
    }

    /// Compute dominant class the same way the binary does:
    /// mode of the fold_sequence.
    fn compute_dominant_class(fold_sequence: &[u8]) -> u8 {
        fold_sequence
            .iter()
            .copied()
            .max_by_key(|&c| fold_sequence.iter().filter(|&&x| x == c).count())
            .unwrap_or(0)
    }

    #[test]
    fn all_reference_vectors_match() {
        let vectors = load_vectors();
        assert!(
            !vectors.is_empty(),
            "Reference vectors file is empty or missing"
        );

        let encoder = GlyphEncoder::default();
        let mut failures: Vec<String> = Vec::new();

        for vec in &vectors {
            let data = input_to_f64(vec);
            let glyph = match encoder.encode(&data) {
                Ok(g) => g,
                Err(e) => {
                    failures.push(format!("{}: encode failed: {}", vec.id, e));
                    continue;
                }
            };

            let dominant = compute_dominant_class(&glyph.fold_sequence);
            let centroid = glyph.sga_centroid;
            let fano = glyph.fano_signature;
            let classes_used: usize = {
                let mut seen = HashSet::new();
                for &c in &glyph.fold_sequence {
                    seen.insert(c);
                }
                seen.len()
            };

            // Check dominant class
            if dominant != vec.expected.dominant_class {
                failures.push(format!(
                    "{}: dominant_class mismatch: got {} expected {}",
                    vec.id, dominant, vec.expected.dominant_class
                ));
            }

            // Check centroid
            let exp_c = &vec.expected.centroid;
            if centroid != (exp_c.h2, exp_c.d, exp_c.l) {
                failures.push(format!(
                    "{}: centroid mismatch: got {:?} expected ({},{},{})",
                    vec.id, centroid, exp_c.h2, exp_c.d, exp_c.l
                ));
            }

            // Check fano_signature (allow small float tolerance)
            for (i, (&got, &exp)) in fano.iter().zip(vec.expected.fano_signature.iter()).enumerate()
            {
                if (got - exp).abs() > 1e-12 {
                    failures.push(format!(
                        "{}: fano_signature[{}] mismatch: got {:.15e} expected {:.15e}",
                        vec.id, i, got, exp
                    ));
                }
            }

            // Check classes_used
            if classes_used != vec.expected.classes_used {
                failures.push(format!(
                    "{}: classes_used mismatch: got {} expected {}",
                    vec.id, classes_used, vec.expected.classes_used
                ));
            }
        }

        if !failures.is_empty() {
            panic!(
                "{} reference vector check(s) failed:\n  {}",
                failures.len(),
                failures.join("\n  ")
            );
        }

        eprintln!(
            "All {} reference vectors matched successfully.",
            vectors.len()
        );
    }

    #[test]
    fn deterministic_across_runs() {
        // Encode the same input twice and ensure identical output.
        let encoder = GlyphEncoder::default();
        let data: Vec<f64> = b"determinism check"
            .iter()
            .map(|&b| b as f64 / 255.0)
            .collect();

        let g1 = encoder.encode(&data).expect("encode 1");
        let g2 = encoder.encode(&data).expect("encode 2");

        assert_eq!(g1.fold_sequence, g2.fold_sequence, "fold_sequence differs");
        assert_eq!(g1.sga_centroid, g2.sga_centroid, "centroid differs");
        assert_eq!(g1.fano_signature, g2.fano_signature, "fano_signature differs");
        assert_eq!(
            g1.fold_amplitudes, g2.fold_amplitudes,
            "fold_amplitudes differs"
        );
        assert_eq!(g1.fold_phases, g2.fold_phases, "fold_phases differs");
    }
}
