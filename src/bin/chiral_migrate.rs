//! Migrate an existing HRM v1 file to Chiral HRM v2 format.
//!
//! Usage: chiral_migrate <path-to-v1.hrm> [output-path]
//! If output-path is omitted, overwrites the input file (backup first!).

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: chiral_migrate <path-to-v1.hrm> [output-path]");
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[1]);
    let output_path = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        input_path.clone()
    };

    eprintln!("Loading v1 HRM from: {}", input_path.display());

    // Load as ChiralMedium (auto-detects v1 and converts)
    let chiral = kannaka_memory::medium::chiral::ChiralMedium::load(&input_path)
        .expect("Failed to load HRM file");

    eprintln!("Loaded successfully:");
    eprintln!("  Right hemisphere (subconscious): {} wavefronts", chiral.right.count());
    eprintln!("  Left hemisphere (conscious):     {} wavefronts", chiral.left.count());
    eprintln!("  Chiral scales:                   {}", chiral.scales.len());

    let summary = chiral.consciousness_summary();
    eprintln!("  Right total energy:              {:.2}", summary.right_energy);
    eprintln!("  Bilateral order:                 {:.3}", summary.bilateral_order);

    // Save as v2
    eprintln!("\nSaving as chiral HRM v2 to: {}", output_path.display());
    chiral.save(&output_path).expect("Failed to save chiral HRM file");

    // Verify roundtrip
    let reloaded = kannaka_memory::medium::chiral::ChiralMedium::load(&output_path)
        .expect("Failed to reload saved file");
    
    assert_eq!(reloaded.right.count(), chiral.right.count(), "Right hemisphere count mismatch!");
    assert_eq!(reloaded.left.count(), chiral.left.count(), "Left hemisphere count mismatch!");
    assert_eq!(reloaded.scales.len(), chiral.scales.len(), "Scales count mismatch!");

    eprintln!("\nMigration complete! ✅");
    eprintln!("  {} memories migrated to right hemisphere (subconscious)", reloaded.right.count());
    eprintln!("  Left hemisphere initialized empty (fresh conscious workspace)");
    eprintln!("  HRM v2 format with chiral metadata written");
}
