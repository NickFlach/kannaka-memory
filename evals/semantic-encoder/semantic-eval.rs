//! Eval-side semantic-encoder experiment (NOT shipped): build a parallel store
//! from the frozen corpus using a real embedding model (OllamaEncoder), with
//! original UUIDs preserved via update_right_id, then run the production recall
//! path against it. Answers: what does the medium deliver with a semantic
//! encoder instead of SimpleHashEncoder?
//!
//! modes:
//!   build  <corpus.json>  — corpus [{id, content, amplitude}] -> new store at
//!                           $KANNAKA_DATA_DIR/kannaka.hrm (absorb -> flush ->
//!                           id rewrite -> save)
//!   recall <probes.json>  — production recall_resonance per probe, top-10 ids
//!
//! env: OLLAMA_URL (default http://host.docker.internal:11434),
//!      OLLAMA_MODEL (default mxbai-embed-large), OLLAMA_DIM (default 1024)
use kannaka_memory::encoding::OllamaEncoder;
use kannaka_memory::medium::chiral::ChiralMedium;
use kannaka_memory::store::MediumBackend;
use kannaka_memory::{Codebook, EncodingPipeline, HrmStore};
use std::path::PathBuf;
use uuid::Uuid;

fn pipeline() -> EncodingPipeline {
    let url = std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://host.docker.internal:11434".into());
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "mxbai-embed-large".into());
    let dim: usize = std::env::var("OLLAMA_DIM").ok().and_then(|v| v.parse().ok()).unwrap_or(1024);
    let encoder = OllamaEncoder::new(url, model, dim);
    let codebook = Codebook::new(dim, 10_000, 42);
    EncodingPipeline::new(Box::new(encoder), codebook)
}

fn hrm_path() -> PathBuf {
    PathBuf::from(std::env::var("KANNAKA_DATA_DIR").expect("KANNAKA_DATA_DIR")).join("kannaka.hrm")
}

fn main() {
    let mode = std::env::args().nth(1).expect("mode: build|recall");
    let file = std::env::args().nth(2).expect("json path");
    let data: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&file).expect("read json")).unwrap();

    match mode.as_str() {
        "build" => {
            let corpus = data.as_array().expect("corpus array");
            let mut store = HrmStore::new(pipeline(), hrm_path());
            let mut remap: Vec<(Uuid, Uuid)> = Vec::new();
            for (i, m) in corpus.iter().enumerate() {
                let orig: Uuid = m["id"].as_str().unwrap().parse().unwrap();
                let content = m["content"].as_str().unwrap();
                let amp = (m["amplitude"].as_f64().unwrap_or(0.6) as f32).clamp(0.05, 1.0);
                let minted = MediumBackend::absorb(&mut store, content, amp, None).expect("absorb");
                remap.push((minted, orig));
                if (i + 1) % 50 == 0 {
                    eprintln!("absorbed {}/{}", i + 1, corpus.len());
                }
            }
            MediumBackend::flush(&mut store).expect("flush");
            drop(store);

            let mut chiral = ChiralMedium::load(&hrm_path()).expect("reload chiral");
            let mut rewritten = 0usize;
            for (minted, orig) in &remap {
                match chiral.update_right_id(minted, *orig) {
                    Ok(()) => rewritten += 1,
                    Err(e) => eprintln!("id rewrite failed {minted}->{orig}: {e}"),
                }
            }
            chiral.save(&hrm_path()).expect("save after rewrite");
            println!("{{\"absorbed\":{},\"ids_rewritten\":{}}}", remap.len(), rewritten);
        }
        "recall" => {
            let probes = data.as_array().expect("probes array");
            let mut store = HrmStore::load(pipeline(), hrm_path()).expect("load store");
            let mut out = serde_json::Map::new();
            for p in probes {
                let pid = p["id"].as_str().unwrap();
                let query = p["query"].as_str().unwrap();
                let res = store.recall_resonance(query, 10).expect("recall");
                let ids: Vec<String> = res.iter().map(|r| r.id.to_string()).collect();
                out.insert(pid.to_string(), serde_json::json!(ids));
                eprintln!("{pid}: {} results", out[pid].as_array().unwrap().len());
            }
            println!("{}", serde_json::json!({"results": out}));
        }
        other => panic!("unknown mode {other}"),
    }
}
