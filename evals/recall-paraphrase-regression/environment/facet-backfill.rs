//! Eval-side backfill driver (NOT shipped): decompose every eligible memory in a
//! store copy via the library's public `ChiralMedium::backfill_facets`, using the
//! exact production pipeline construction (SimpleHashEncoder 384/42 + Codebook
//! 384/10_000/42, mirroring src/bin/kannaka.rs). Exists because ADR-0049 ships
//! dark with no CLI wiring; this drives the public API against a disposable
//! per-trial copy so the facet-on arm is measurable. Prints one JSON line.
use kannaka_memory::medium::chiral::ChiralMedium;
use kannaka_memory::{Codebook, EncodingPipeline, SimpleHashEncoder};
use std::path::PathBuf;

fn main() {
    let data_dir = std::env::var("KANNAKA_DATA_DIR").expect("KANNAKA_DATA_DIR required");
    let path = PathBuf::from(&data_dir).join("kannaka.hrm");

    let encoder = SimpleHashEncoder::new(384, 42);
    let codebook = Codebook::new(384, 10_000, 42);
    let pipeline = EncodingPipeline::new(Box::new(encoder), codebook);

    let mut chiral = ChiralMedium::load(&path).expect("load chiral medium");
    let ids: Vec<uuid::Uuid> = chiral
        .right
        .metadata
        .iter()
        .chain(chiral.left.metadata.iter())
        .map(|m| m.id)
        .collect();

    let total = ids.len();
    let (mut parents, mut minted) = (0usize, 0usize);
    for id in ids {
        let n = chiral.backfill_facets(id, &pipeline).expect("backfill");
        if n > 0 {
            parents += 1;
            minted += n;
        }
    }
    chiral.save(&path).expect("save");

    println!(
        "{{\"memories\":{},\"parents_decomposed\":{},\"facets_minted\":{}}}",
        total, parents, minted
    );
}
