// Example demonstrating the OGC ↔ SGA Bridge
use kannaka_memory::{GlyphEncoder, GlyphDecoder, encode_memory_as_glyph, HyperMemory};

fn main() {
    println!("🧬 OGC ↔ SGA Bridge Demo");
    println!("========================");
    
    // 1. Create some sample data
    let data = vec![1.0, 0.5, -0.3, 0.8, 0.1, -0.9, 0.0, 0.6, 0.4, -0.2];
    println!("Original data: {:?}", data);
    
    // 2. Encode data as a glyph using SGA-guided folding
    let encoder = GlyphEncoder::default();
    let glyph = encoder.encode(&data).expect("Failed to encode glyph");
    
    println!("\n🔮 Glyph Properties:");
    println!("  Fold sequence length: {}", glyph.fold_sequence.len());
    println!("  SGA centroid: {:?}", glyph.sga_centroid);
    println!("  Compression ratio: {:.2}x", glyph.compression_ratio);
    println!("  Fano signature: {:?}", glyph.fano_signature);
    
    // 3. Convert glyph to musical frequencies
    let frequencies = glyph.to_frequencies();
    println!("\n🎵 Musical frequencies (432 Hz × φⁿ):");
    for (i, freq) in frequencies.iter().take(5).enumerate() {
        println!("  Step {}: {:.2} Hz", i, freq);
    }
    
    // 4. Decode glyph back to data
    let decoder = GlyphDecoder::new(data.len(), 1.0);
    let reconstructed = decoder.decode(&glyph).expect("Failed to decode glyph");
    println!("\n🔄 Reconstructed data length: {}", reconstructed.len());
    
    // 5. Test with HyperMemory integration
    let vector = vec![0.1; 1000];
    let memory = HyperMemory::new(vector, "Test memory for glyph encoding".to_string());
    
    let memory_glyph = encode_memory_as_glyph(&memory).expect("Failed to encode memory");
    println!("\n🧠 Memory → Glyph:");
    println!("  Memory vector length: {}", memory.vector.len());
    println!("  Glyph fold sequence: {} steps", memory_glyph.fold_sequence.len());
    println!("  Compression: {:.1}x", memory_glyph.compression_ratio);
    
    // 6. Render glyph as 2D path
    let path = glyph.render_path();
    println!("\n🎨 2D Path trajectory ({} points):", path.len());
    for (i, (x, y)) in path.iter().take(3).enumerate() {
        println!("  Point {}: ({:.3}, {:.3})", i, x, y);
    }
    
    println!("\n✅ OGC ↔ SGA Bridge operational!");
    println!("   Data flows: Raw → SGA coords → Fano groups → Glyph → Bloom → Reconstruction");
}