//! Eval-side ablation driver (NOT shipped): localize the zero-overlap semantic
//! anomaly. Two modes against a disposable store copy:
//!   flat   — score probes through HrmStore::recall_resonance_readonly (flat
//!            medium scorer, no observation write-back; mirrors right hemisphere)
//!   encode — print each query's pipeline-encoded vector so pure-encoder cosine
//!            can be computed offline against exported wavefront vectors
//! Pipeline mirrors production (SimpleHashEncoder 384/42 + Codebook 384/10_000/42).
use kannaka_memory::{Codebook, EncodingPipeline, HrmStore, SimpleHashEncoder};
use std::path::PathBuf;

fn pipeline() -> EncodingPipeline {
    let encoder = SimpleHashEncoder::new(384, 42);
    let codebook = Codebook::new(384, 10_000, 42);
    EncodingPipeline::new(Box::new(encoder), codebook)
}

fn main() {
    let mode = std::env::args().nth(1).expect("mode: flat|encode");
    let probes_path = std::env::args().nth(2).expect("probes.json path");
    let probes: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&probes_path).expect("read probes")).unwrap();
    let probes = probes.as_array().expect("probes array");

    match mode.as_str() {
        "flat" => {
            let data_dir = std::env::var("KANNAKA_DATA_DIR").expect("KANNAKA_DATA_DIR");
            let store = HrmStore::load(pipeline(), PathBuf::from(&data_dir).join("kannaka.hrm"))
                .expect("load store");
            let mut out = serde_json::Map::new();
            for p in probes {
                let id = p["id"].as_str().unwrap();
                let query = p["query"].as_str().unwrap();
                let res = store
                    .recall_resonance_readonly(query, 10)
                    .expect("flat recall");
                let ids: Vec<String> = res.iter().map(|r| r.id.to_string()).collect();
                out.insert(id.to_string(), serde_json::json!(ids));
            }
            println!("{}", serde_json::json!({"mode":"flat","results":out}));
        }
        "encode" => {
            let pipe = pipeline();
            let mut out = serde_json::Map::new();
            for p in probes {
                let id = p["id"].as_str().unwrap();
                let query = p["query"].as_str().unwrap();
                let v = pipe.encode_text(query).expect("encode");
                out.insert(id.to_string(), serde_json::json!(v));
            }
            println!("{}", serde_json::json!({"mode":"encode","vectors":out}));
        }
        other => panic!("unknown mode {other}"),
    }
}
