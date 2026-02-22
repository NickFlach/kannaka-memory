use kannaka_memory::openclaw::KannakaMemorySystem;
use std::path::PathBuf;
fn main() {
    let dir = PathBuf::from(r"C:\Users\nickf\.openclaw\kannaka-data");
    let sys = KannakaMemorySystem::init(dir).unwrap();
    let stats = sys.stats();
    println!("phi={} classes={} memories={}", stats.phi, stats.geometric_classes, stats.total_memories);
    // Check amplitude distribution
    // We can't easily access internals, but stats gives us what we need
}
