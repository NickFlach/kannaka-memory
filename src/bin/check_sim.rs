use kannaka_memory::openclaw::KannakaMemorySystem;
use kannaka_memory::wave::cosine_similarity;
use std::path::PathBuf;

fn main() {
    let data_dir = PathBuf::from(std::env::var("KANNAKA_DATA_DIR").unwrap());
    let sys = KannakaMemorySystem::init(data_dir).unwrap();
    let all = sys.engine.store.all_memories().unwrap();
    
    let mut sample: Vec<_> = Vec::new();
    for m in all.iter() {
        if m.content.starts_with("HEAR:") {
            sample.push(*m);
            if sample.len() >= 5 { break; }
        }
    }
    let ac = sample.len();
    for m in all.iter() {
        if !m.content.starts_with("HEAR:") && !m.content.starts_with("__") && !m.content.starts_with("[hall") {
            sample.push(*m);
            if sample.len() >= 10 { break; }
        }
    }
    
    println!("Sample: {} ({} audio, {} text)", sample.len(), ac, sample.len() - ac);
    
    // Check pairwise similarities in the sample
    for i in 0..sample.len() {
        for j in (i+1)..sample.len() {
            let sim = cosine_similarity(&sample[i].vector, &sample[j].vector);
            if sim.abs() > 0.2 {
                let a = &sample[i].content[..40.min(sample[i].content.len())];
                let b = &sample[j].content[..40.min(sample[j].content.len())];
                println!("[{}]-[{}] sim={:.4}: {} | {}", i, j, sim, a, b);
            }
        }
    }
    
    // Check what Xi actually sees
    // The issue: element-wise multiply of 10K dim vectors → most elements near 0 → normalized result is noise
    // After bind (element-wise multiply) of many vectors, result is dominated by cancellation
    println!("\nVector norms:");
    for (i, m) in sample.iter().enumerate() {
        let norm: f32 = m.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        let max: f32 = m.vector.iter().cloned().fold(0.0f32, |a, b| a.max(b.abs()));
        println!("  [{}] norm={:.4} max={:.6}", i, norm, max);
    }
}
