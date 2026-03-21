//! Migration from Dolt SQL database to Holographic Resonance Medium (.hrm) files.
//!
//! This binary reads memories from an existing Dolt database (kannaka_memory on port 3307)
//! and converts them to the new HRM format, preserving all existing data while enabling
//! the new tensor-based storage system.

use std::env;
use std::path::PathBuf;

use clap::Parser;

use kannaka_memory::dolt::{DoltConfig, DoltMemoryStore};
use kannaka_memory::store::MemoryStore;
#[cfg(feature = "hrm")]
use kannaka_memory::medium::Medium;
#[cfg(feature = "hrm")]
use kannaka_memory::encoding::{EncodingPipeline, SimpleHashEncoder};
#[cfg(feature = "hrm")]
use kannaka_memory::codebook::Codebook;

#[derive(Parser)]
#[command(author, version, about = "Migrate memories from Dolt to HRM format", long_about = None)]
struct Args {
    /// Output .hrm file path
    #[arg(short, long, default_value = "kannaka.hrm")]
    output: PathBuf,
    
    /// Dolt database host
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    
    /// Dolt database port
    #[arg(long, default_value = "3307")]
    port: u16,
    
    /// Dolt database name
    #[arg(long, default_value = "kannaka_memory")]
    database: String,
    
    /// Dolt database user
    #[arg(long, default_value = "root")]
    user: String,
    
    /// Dry run - show what would be migrated without writing
    #[arg(long)]
    dry_run: bool,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug)]
struct MigrationStats {
    total_memories: usize,
    migrated_memories: usize,
    existing_vectors: usize,
    re_encoded_memories: usize,
    dimension_mismatch_re_encoded: usize,
    skipped_memories: usize,
    errors: Vec<String>,
}

impl Default for MigrationStats {
    fn default() -> Self {
        Self {
            total_memories: 0,
            migrated_memories: 0,
            existing_vectors: 0,
            re_encoded_memories: 0,
            dimension_mismatch_re_encoded: 0,
            skipped_memories: 0,
            errors: Vec::new(),
        }
    }
}

#[cfg(not(feature = "hrm"))]
fn main() {
    eprintln!("Error: This binary requires the 'hrm' feature to be enabled.");
    eprintln!("Please run with: cargo run --features migration --bin hrm_migrate");
    std::process::exit(1);
}

#[cfg(feature = "hrm")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    if args.verbose {
        println!("🥷 Kannaka HRM Migration Tool");
        println!("Migrating from Dolt to Holographic Resonance Medium");
        println!("Source: {}:{}:{}", args.host, args.port, args.database);
        println!("Target: {}", args.output.display());
        if args.dry_run {
            println!("⚠️  DRY RUN - No files will be written");
        }
        println!();
    }
    
    // Configure Dolt connection
    let mut dolt_config = DoltConfig::default();
    dolt_config.host = args.host;
    dolt_config.port = args.port;
    dolt_config.database = args.database;
    dolt_config.user = args.user;
    dolt_config.password = env::var("DOLT_PASSWORD").unwrap_or_default();
    
    // Setup encoding pipeline for re-encoding content if vectors are missing
    let encoder = SimpleHashEncoder::new(384, 42); // Use default hash encoder for migration
    let codebook = Codebook::new(384, 10_000, 42); // Use standard codebook parameters
    let pipeline = EncodingPipeline::new(Box::new(encoder), codebook);
    
    // Create Dolt store and load memories
    if args.verbose {
        println!("📡 Connecting to Dolt database...");
    }
    
    let pool = mysql::Pool::new(mysql::Opts::from_url(&format!(
        "mysql://{}:{}@{}:{}/{}",
        dolt_config.user,
        dolt_config.password,
        dolt_config.host,
        dolt_config.port,
        dolt_config.database
    ))?)?;
    
    let mut dolt_store = DoltMemoryStore::new(pool)?;
    
    if args.verbose {
        println!("✅ Connected to Dolt successfully");
        println!("📦 Loading memories from Dolt...");
    }
    
    let loaded_count = dolt_store.load_from_dolt()?;
    
    if args.verbose {
        println!("✅ Loaded {} memories from Dolt", loaded_count);
        println!("🔄 Converting to HRM format...");
    }
    
    // Get all memories from Dolt
    let all_memories = dolt_store.all_memories()?;
    let mut stats = MigrationStats::default();
    stats.total_memories = all_memories.len();
    
    // Create new HRM medium
    let mut medium = Medium::new();
    
    // Process each memory
    for memory in all_memories {
        if args.verbose && stats.migrated_memories % 50 == 0 {
            println!("  Progress: {}/{} memories", stats.migrated_memories, stats.total_memories);
        }
        
        // Extract vector - use existing vector_data if dimensions match, otherwise re-encode from content
        let vector = if !memory.vector.is_empty() && memory.vector.len() == 10_000 {
            // Vector exists and has correct dimensions (already 10K from codebook)
            if args.verbose && stats.migrated_memories < 5 {
                println!("  Using existing 10K vector for: {}", memory.content.chars().take(60).collect::<String>());
            }
            stats.existing_vectors += 1;
            memory.vector.clone()
        } else {
            // Vector is missing, empty, or has wrong dimensions (e.g. 384-dim Ollama) - re-encode through codebook
            let is_dimension_mismatch = !memory.vector.is_empty();
            if is_dimension_mismatch && args.verbose && stats.migrated_memories < 5 {
                println!("  Re-encoding content (dimension mismatch: {} → 10000) for: {}", 
                         memory.vector.len(), 
                         memory.content.chars().take(60).collect::<String>());
            } else if args.verbose && stats.migrated_memories < 5 {
                println!("  Re-encoding content (no vector) for: {}", memory.content.chars().take(60).collect::<String>());
            }
            match pipeline.encode_text(&memory.content) {
                Ok(vec) => {
                    stats.re_encoded_memories += 1;
                    if is_dimension_mismatch {
                        stats.dimension_mismatch_re_encoded += 1;
                    }
                    vec
                }
                Err(e) => {
                    let error = format!("Failed to encode content for memory {}: {}", memory.id, e);
                    stats.errors.push(error.clone());
                    if args.verbose {
                        println!("  ❌ {}", error);
                    }
                    stats.skipped_memories += 1;
                    continue;
                }
            }
        };
        
        // Add wavefront to medium
        match medium.add_wavefront(&vector, memory.content.clone(), memory.amplitude) {
            Ok(id) => {
                // Update the wavefront's metadata with original memory data
                if let Some(index) = medium.get_wavefront_index(&id) {
                    // Map Dolt fields to HRM fields
                    medium.energy[index] = memory.amplitude; // amplitude → energy
                    medium.frequency[index] = memory.frequency; // frequency → frequency
                    medium.phase[index] = memory.phase; // phase → phase
                    medium.timestamps[index] = memory.created_at.timestamp_millis();
                    
                    // Update metadata with additional fields
                    medium.metadata[index].created_at = memory.created_at;
                    medium.metadata[index].hallucinated = memory.hallucinated;
                } else {
                    let error = format!("Failed to find index for wavefront {}", id);
                    stats.errors.push(error.clone());
                    if args.verbose {
                        println!("  ❌ {}", error);
                    }
                }
                
                // Note: Skip links are not preserved in the HRM format as they emerge from interference patterns
                
                stats.migrated_memories += 1;
            }
            Err(e) => {
                let error = format!("Failed to add wavefront for memory {}: {}", memory.id, e);
                stats.errors.push(error.clone());
                if args.verbose {
                    println!("  ❌ {}", error);
                }
                stats.skipped_memories += 1;
            }
        }
    }
    
    // Save the HRM file (unless dry run)
    if !args.dry_run {
        if args.verbose {
            println!("💾 Saving HRM file to {}...", args.output.display());
        }
        
        medium.save(&args.output)?;
        
        if args.verbose {
            println!("✅ HRM file saved successfully");
        }
    } else {
        if args.verbose {
            println!("🚫 Dry run complete - no file saved");
        }
    }
    
    // Print migration report
    println!();
    println!("📊 Migration Report:");
    println!("  Total memories in Dolt: {}", stats.total_memories);
    println!("  Successfully migrated:  {}", stats.migrated_memories);
    println!("  Existing vectors used:  {}", stats.existing_vectors);
    println!("  Content re-encoded:     {}", stats.re_encoded_memories);
    if stats.dimension_mismatch_re_encoded > 0 {
        println!("    (due to wrong dims):  {}", stats.dimension_mismatch_re_encoded);
    }
    println!("  Skipped due to errors:  {}", stats.skipped_memories);
    
    if !stats.errors.is_empty() {
        println!("  Errors encountered:");
        for error in &stats.errors {
            println!("    • {}", error);
        }
    }
    
    if !args.dry_run {
        println!();
        println!("🎉 Migration complete!");
        println!("   HRM file: {} ({} wavefronts)", args.output.display(), medium.wavefront_count());
        println!("   Use: kannaka --features hrm <command> to use the new HRM backend");
    } else {
        println!();
        println!("🔍 Dry run complete - use without --dry-run to perform actual migration");
    }
    
    if stats.skipped_memories > 0 {
        std::process::exit(1);
    }
    
    Ok(())
}

#[cfg(feature = "hrm")]
#[cfg(test)]
mod tests {
    use super::*;
    use kannaka_memory::memory::HyperMemory;
    use kannaka_memory::medium::WAVEFRONT_DIM;
    use tempfile::NamedTempFile;
    
    #[test]
    fn migration_preserves_memory_data() {
        // Create a test memory with known data
        let mut test_memory = HyperMemory::new(
            vec![0.5; WAVEFRONT_DIM], 
            "Test memory content".to_string()
        );
        test_memory.amplitude = 0.8;
        test_memory.frequency = 1.2;
        test_memory.phase = 0.5;
        
        // Create encoding pipeline
        let encoder = SimpleHashEncoder::new(384, 42);
        let codebook = Codebook::new(384, WAVEFRONT_DIM, 42);
        let pipeline = EncodingPipeline::new(Box::new(encoder), codebook);
        
        // Create medium and add memory
        let mut medium = Medium::new();
        let id = medium.add_wavefront(&test_memory.vector, test_memory.content.clone(), test_memory.amplitude).unwrap();
        let index = medium.get_wavefront_index(&id).unwrap();
        
        // Apply migration mapping
        medium.energy[index] = test_memory.amplitude;
        medium.frequency[index] = test_memory.frequency;
        medium.phase[index] = test_memory.phase;
        medium.timestamps[index] = test_memory.created_at.timestamp_millis();
        medium.metadata[index].created_at = test_memory.created_at;
        medium.metadata[index].hallucinated = test_memory.hallucinated;
        
        // Verify data preservation
        assert_eq!(medium.energy[index], 0.8);
        assert_eq!(medium.frequency[index], 1.2);
        assert_eq!(medium.phase[index], 0.5);
        assert_eq!(medium.metadata[index].content, "Test memory content");
        assert_eq!(medium.metadata[index].hallucinated, false);
        
        // Test roundtrip save/load
        let temp_file = NamedTempFile::new().unwrap();
        medium.save(temp_file.path()).unwrap();
        
        let loaded_medium = Medium::load(temp_file.path()).unwrap();
        assert_eq!(loaded_medium.wavefront_count(), 1);
        assert_eq!(loaded_medium.energy[0], 0.8);
        assert_eq!(loaded_medium.frequency[0], 1.2);
        assert_eq!(loaded_medium.phase[0], 0.5);
    }
}