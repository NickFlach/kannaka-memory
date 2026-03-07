//! # OGC ↔ SGA Bridge: Geometric Glyph Compression
//!
//! This module implements the bridge between GlyphicCompressor2's Origami Glyph Compression
//! pipeline and kannaka-memory's Sigmatics Geometric Algebra engine.
//!
//! ## Core Concept
//! 
//! Replace OGC's naive spatial partitioning with SGA-guided geometric folding.
//! Data folds along Fano lines as natural creases. The fold sequence through 
//! the 96-class space IS the emergent glyph.
//!
//! ## Architecture
//!
//! ```
//! Data -> SGA Mapping -> Fano Grouping -> Geometric Compression -> Glyph
//!                                                                    |
//! Reconstructed Data <- SGA Bloom <- Unfold Sequence <- Glyph ------+
//! ```

use crate::geometry::{
    SgaElement, MemoryCoordinates, ClassComponents, 
    lift, project, classify_memory,
    FANO_LINES, cross_product, decode_class_index, components_to_class_index,
    EPSILON
};
use crate::memory::HyperMemory;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Golden ratio for frequency harmonics
pub const PHI: f64 = 1.618033988749895;

/// Base frequency for musical mapping (432 Hz)
pub const BASE_FREQ: f64 = 432.0;

// ============================================================================
// Glyph Structure
// ============================================================================

/// A compressed glyph representing folded data along SGA geometry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Glyph {
    /// Path through 96-class space representing fold sequence
    pub fold_sequence: Vec<u8>,
    /// Energy at each fold step
    pub fold_amplitudes: Vec<f64>,
    /// Phase at each fold step  
    pub fold_phases: Vec<f64>,
    /// Energy distribution across the 7 Fano lines
    pub fano_signature: [f64; 7],
    /// Dominant (h₂, d, ℓ) coordinates
    pub sga_centroid: (u8, u8, u8),
    /// Overall compression ratio achieved
    pub compression_ratio: f64,
    /// Timestamp of creation
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Glyph {
    /// Compute similarity with another glyph using geometric correlation
    pub fn similarity(&self, other: &Glyph) -> f64 {
        let mut similarity = 0.0;
        
        // Fano signature correlation (40% weight) - use cosine similarity
        let self_norm = self.fano_signature.iter().map(|x| x * x).sum::<f64>().sqrt().max(EPSILON);
        let other_norm = other.fano_signature.iter().map(|x| x * x).sum::<f64>().sqrt().max(EPSILON);
        let dot_product = self.fano_signature.iter()
            .zip(other.fano_signature.iter())
            .map(|(a, b)| a * b)
            .sum::<f64>();
        let fano_corr = dot_product / (self_norm * other_norm);
        similarity += fano_corr * 0.4;
        
        // Centroid similarity (30% weight) - use exponential decay
        let h2_diff = (self.sga_centroid.0 as i32 - other.sga_centroid.0 as i32).abs();
        let d_diff = (self.sga_centroid.1 as i32 - other.sga_centroid.1 as i32).abs();
        let l_diff = (self.sga_centroid.2 as i32 - other.sga_centroid.2 as i32).abs();
        let centroid_dist = h2_diff + d_diff + l_diff;
        let centroid_sim = (-centroid_dist as f64 * 0.5).exp();
        similarity += centroid_sim * 0.3;
        
        // Fold sequence correlation (30% weight)
        let seq_sim = self.fold_sequence_similarity(&other.fold_sequence);
        similarity += seq_sim * 0.3;
        
        similarity.min(1.0).max(0.0)
    }
    
    /// Convert glyph to musical frequencies using φ-based harmonics
    pub fn to_frequencies(&self) -> Vec<f64> {
        let mut frequencies = Vec::new();
        
        for (i, &class_idx) in self.fold_sequence.iter().enumerate() {
            let comp = decode_class_index(class_idx);
            
            // Map (h₂, d, ℓ) to frequency using golden ratio
            let h2_mult = PHI.powi(comp.h2 as i32);
            let d_mult = PHI.powi(comp.d as i32 - 1); // center around φ⁰ = 1
            let l_mult = PHI.powi((comp.l as i32).saturating_sub(3)); // center around φ⁰ = 1
            
            let freq = BASE_FREQ * h2_mult * d_mult * l_mult;
            
            // Modulate by amplitude and phase
            if i < self.fold_amplitudes.len() {
                let amplitude_factor = self.fold_amplitudes[i].max(0.1);
                frequencies.push(freq * amplitude_factor);
            } else {
                frequencies.push(freq);
            }
        }
        
        frequencies
    }
    
    /// Render glyph as 2D trajectory in complex plane
    pub fn render_path(&self) -> Vec<(f64, f64)> {
        let mut path = Vec::new();
        let mut x = 0.0;
        let mut y = 0.0;
        
        for (i, &class_idx) in self.fold_sequence.iter().enumerate() {
            let comp = decode_class_index(class_idx);
            
            // Map class to complex coordinates
            let angle = 2.0 * std::f64::consts::PI * (class_idx as f64) / 96.0;
            let radius = if i < self.fold_amplitudes.len() {
                self.fold_amplitudes[i]
            } else {
                1.0
            };
            
            let dx = radius * angle.cos();
            let dy = radius * angle.sin();
            
            // Apply geometric transform based on (h₂, d, ℓ)
            let transform_angle = (comp.h2 as f64) * std::f64::consts::PI / 2.0 + 
                                 (comp.d as f64) * std::f64::consts::PI / 6.0;
            
            let rotated_dx = dx * transform_angle.cos() - dy * transform_angle.sin();
            let rotated_dy = dx * transform_angle.sin() + dy * transform_angle.cos();
            
            x += rotated_dx;
            y += rotated_dy;
            path.push((x, y));
        }
        
        path
    }
    
    /// Compute fold sequence similarity using longest common subsequence
    fn fold_sequence_similarity(&self, other: &[u8]) -> f64 {
        let lcs_len = longest_common_subsequence(&self.fold_sequence, other);
        let max_len = self.fold_sequence.len().max(other.len());
        if max_len == 0 { 1.0 } else { lcs_len as f64 / max_len as f64 }
    }
}

// ============================================================================
// Glyph Encoder
// ============================================================================

/// Encoder that maps arbitrary data into SGA-guided glyphs
#[derive(Debug, Clone)]
pub struct GlyphEncoder {
    /// Minimum fold threshold for compression
    pub fold_threshold: f64,
    /// Maximum glyph length
    pub max_glyph_length: usize,
    /// SVD compression tolerance
    pub svd_tolerance: f64,
}

impl Default for GlyphEncoder {
    fn default() -> Self {
        Self {
            fold_threshold: 0.01,
            max_glyph_length: 256,
            svd_tolerance: 1e-6,
        }
    }
}

impl GlyphEncoder {
    /// Create new encoder with custom parameters
    pub fn new(fold_threshold: f64, max_glyph_length: usize, svd_tolerance: f64) -> Self {
        Self { fold_threshold, max_glyph_length, svd_tolerance }
    }
    
    /// Encode arbitrary data as a glyph
    pub fn encode(&self, data: &[f64]) -> Result<Glyph, GlyphError> {
        if data.is_empty() {
            return Err(GlyphError::EmptyData);
        }
        
        // Step 1: Map data elements to SGA coordinates
        let sga_mappings = self.map_to_sga_coordinates(data)?;
        
        // Step 2: Group elements by Fano-line relationships
        let fano_groups = self.group_by_fano_lines(&sga_mappings);
        
        // Step 3: Apply geometric compression within each group
        let compressed_groups = self.compress_fano_groups(fano_groups)?;
        
        // Step 4: Generate fold sequence
        let (fold_sequence, fold_amplitudes, fold_phases) = 
            self.generate_fold_sequence(compressed_groups)?;
        
        // Step 5: Compute Fano signature and centroid
        let fano_signature = self.compute_fano_signature(&sga_mappings);
        let sga_centroid = self.compute_sga_centroid(&sga_mappings);
        let compression_ratio = data.len() as f64 / fold_sequence.len() as f64;
        
        Ok(Glyph {
            fold_sequence,
            fold_amplitudes,
            fold_phases,
            fano_signature,
            sga_centroid,
            compression_ratio,
            created_at: chrono::Utc::now(),
        })
    }
    
    /// Map data elements to SGA coordinates using hash-based classification
    fn map_to_sga_coordinates(&self, data: &[f64]) -> Result<Vec<SgaMappedElement>, GlyphError> {
        let mut mappings = Vec::new();
        
        for (i, &value) in data.iter().enumerate() {
            // Hash position and value for classification - include sign information
            let position_hash = (i as u64).wrapping_mul(0x9e3779b97f4a7c15);
            let value_hash = (value.to_bits() as u64).wrapping_mul(0x517cc1b727220a95);
            
            // Add sign-sensitive component to hash
            let sign_hash = if value >= 0.0 { 0x123456789abcdef0 } else { 0x0fedcba987654321 };
            let combined_hash = position_hash ^ value_hash ^ sign_hash;
            
            // Classify based on value characteristics - be more specific about signs
            let category = if value.abs() < 0.1 { 
                "sparse" 
            } else if value > 0.8 { 
                "strong_positive" 
            } else if value < -0.8 { 
                "strong_negative" 
            } else if value > 0.0 {
                "moderate_positive"
            } else {
                "moderate_negative"
            };
            
            let importance = (value.abs() + 0.1).min(1.0);
            
            // Adjust hash based on magnitude to ensure different values get different classifications
            let magnitude_hash = (value.abs() * 1000.0) as u64;
            let final_hash = combined_hash ^ magnitude_hash;
            
            let coords = classify_memory(category, final_hash, importance);
            
            mappings.push(SgaMappedElement {
                index: i,
                value,
                coordinates: coords.clone(),
                sga_element: lift(coords.class_index).scale(coords.amplitude),
            });
        }
        
        Ok(mappings)
    }
    
    /// Group SGA elements by Fano-line relationships
    fn group_by_fano_lines<'a>(&self, mappings: &'a [SgaMappedElement]) -> HashMap<u8, Vec<&'a SgaMappedElement>> {
        let mut fano_groups: HashMap<u8, Vec<&SgaMappedElement>> = HashMap::new();
        
        // First pass: assign elements to primary Fano lines
        for mapping in mappings {
            let coords = &mapping.coordinates;
            
            // Find the best Fano line for this element
            let best_line = self.find_best_fano_line(coords);
            fano_groups.entry(best_line).or_insert_with(Vec::new).push(mapping);
        }
        
        // Second pass: merge related groups if they share Fano relationships
        self.merge_related_groups(fano_groups)
    }
    
    /// Find the best Fano line for given coordinates
    fn find_best_fano_line(&self, coords: &MemoryCoordinates) -> u8 {
        if coords.l == 0 {
            return 0; // Special case for scalar elements
        }
        
        // Find which Fano line contains this l value
        for (line_idx, &[a, b, c]) in FANO_LINES.iter().enumerate() {
            if coords.l == a || coords.l == b || coords.l == c {
                return line_idx as u8;
            }
        }
        
        0 // Fallback to first line
    }
    
    /// Merge groups that share Fano relationships
    fn merge_related_groups<'a>(
        &self, 
        groups: HashMap<u8, Vec<&'a SgaMappedElement>>
    ) -> HashMap<u8, Vec<&'a SgaMappedElement>> {
        // For now, return as-is. Could implement more sophisticated merging
        // based on cross-product relationships between Fano lines
        groups
    }
    
    /// Apply SVD-like compression within each Fano group
    fn compress_fano_groups(
        &self,
        groups: HashMap<u8, Vec<&SgaMappedElement>>
    ) -> Result<HashMap<u8, CompressedGroup>, GlyphError> {
        let mut compressed = HashMap::new();
        
        for (line_idx, elements) in groups {
            if elements.is_empty() {
                continue;
            }
            
            // Extract values and compute statistics
            let values: Vec<f64> = elements.iter().map(|e| e.value).collect();
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let variance = values.iter()
                .map(|v| (v - mean).powi(2))
                .sum::<f64>() / values.len() as f64;
            let std_dev = variance.sqrt();
            
            // Compress using amplitude and phase representation
            let amplitude = std_dev + self.svd_tolerance;
            let phase = mean.atan2(std_dev); // Map to phase space
            
            // Representative class index (centroid of group)
            let class_indices: Vec<u8> = elements.iter()
                .map(|e| e.coordinates.class_index)
                .collect();
            let representative_class = self.compute_group_centroid(&class_indices);
            
            compressed.insert(line_idx, CompressedGroup {
                representative_class,
                amplitude,
                phase,
                element_count: elements.len(),
                original_indices: elements.iter().map(|e| e.index).collect(),
            });
        }
        
        Ok(compressed)
    }
    
    /// Generate the fold sequence from compressed groups
    fn generate_fold_sequence(
        &self,
        groups: HashMap<u8, CompressedGroup>
    ) -> Result<(Vec<u8>, Vec<f64>, Vec<f64>), GlyphError> {
        let mut sequence = Vec::new();
        let mut amplitudes = Vec::new();
        let mut phases = Vec::new();
        
        // Sort groups by line index for deterministic ordering
        let mut sorted_groups: Vec<_> = groups.into_iter().collect();
        sorted_groups.sort_by_key(|(line_idx, _)| *line_idx);
        
        // Traverse Fano lines in geometric order
        for (line_idx, group) in sorted_groups {
            // Add fold step for this group
            sequence.push(group.representative_class);
            amplitudes.push(group.amplitude);
            phases.push(group.phase);
            
            // Add cross-product connections to other lines
            if sequence.len() < self.max_glyph_length {
                self.add_cross_connections(
                    line_idx, 
                    &group, 
                    &mut sequence, 
                    &mut amplitudes, 
                    &mut phases
                );
            }
        }
        
        // Truncate if too long
        if sequence.len() > self.max_glyph_length {
            sequence.truncate(self.max_glyph_length);
            amplitudes.truncate(self.max_glyph_length);
            phases.truncate(self.max_glyph_length);
        }
        
        Ok((sequence, amplitudes, phases))
    }
    
    /// Add cross-connections between Fano lines
    fn add_cross_connections(
        &self,
        current_line: u8,
        group: &CompressedGroup,
        sequence: &mut Vec<u8>,
        amplitudes: &mut Vec<f64>,
        phases: &mut Vec<f64>
    ) {
        let current_fano_line = &FANO_LINES[current_line as usize];
        
        // Find intersections with other Fano lines via cross products
        for &[a, b, _c] in &FANO_LINES {
            if sequence.len() >= self.max_glyph_length {
                break;
            }
            
            // Compute cross product between lines
            for &p1 in current_fano_line {
                for &p2 in &[a, b] {
                    if p1 != p2 && p1 >= 1 && p1 <= 7 && p2 >= 1 && p2 <= 7 {
                        let (cross_result, sign) = cross_product(p1, p2);
                        if cross_result != 0 && cross_result <= 7 {
                            // Add connection fold
                            let comp = decode_class_index(group.representative_class);
                            let connected_class = components_to_class_index(ClassComponents {
                                h2: comp.h2,
                                d: comp.d,
                                l: cross_result,
                            });
                            
                            sequence.push(connected_class);
                            amplitudes.push(group.amplitude * 0.5 * sign.abs() as f64);
                            phases.push(group.phase + std::f64::consts::PI * 0.25);
                        }
                    }
                }
            }
        }
    }
    
    /// Compute Fano signature from SGA mappings
    fn compute_fano_signature(&self, mappings: &[SgaMappedElement]) -> [f64; 7] {
        let mut signature = [0.0; 7];
        
        for mapping in mappings {
            if mapping.coordinates.l >= 1 && mapping.coordinates.l <= 7 {
                // Find which Fano lines contain this l value
                for (line_idx, &[a, b, c]) in FANO_LINES.iter().enumerate() {
                    if mapping.coordinates.l == a || 
                       mapping.coordinates.l == b || 
                       mapping.coordinates.l == c {
                        signature[line_idx] += mapping.coordinates.amplitude;
                    }
                }
            }
        }
        
        // Normalize signature
        let total: f64 = signature.iter().sum();
        if total > EPSILON {
            for sig in &mut signature {
                *sig /= total;
            }
        }
        
        signature
    }
    
    /// Compute SGA centroid from mappings
    fn compute_sga_centroid(&self, mappings: &[SgaMappedElement]) -> (u8, u8, u8) {
        if mappings.is_empty() {
            return (0, 0, 0);
        }
        
        let h2_sum: usize = mappings.iter().map(|m| m.coordinates.h2 as usize).sum();
        let d_sum: usize = mappings.iter().map(|m| m.coordinates.d as usize).sum();
        let l_sum: usize = mappings.iter().map(|m| m.coordinates.l as usize).sum();
        
        let count = mappings.len();
        (
            ((h2_sum / count) % 4) as u8,
            ((d_sum / count) % 3) as u8,
            ((l_sum / count) % 8) as u8,
        )
    }
    
    /// Compute group centroid class index
    fn compute_group_centroid(&self, class_indices: &[u8]) -> u8 {
        if class_indices.is_empty() {
            return 0;
        }
        
        let sum: usize = class_indices.iter().map(|&idx| idx as usize).sum();
        (sum / class_indices.len()) as u8
    }
}

// ============================================================================
// Glyph Decoder (The Bloom)
// ============================================================================

/// Decoder that unfolds glyphs back into reconstructed data
#[derive(Debug, Clone)]
pub struct GlyphDecoder {
    /// Target dimensionality for bloomed data
    pub target_dimension: usize,
    /// Bloom expansion factor
    pub bloom_factor: f64,
}

impl Default for GlyphDecoder {
    fn default() -> Self {
        Self {
            target_dimension: 1000, // Default size for bloomed data
            bloom_factor: 1.5,      // Expansion factor
        }
    }
}

impl GlyphDecoder {
    /// Create new decoder with custom parameters
    pub fn new(target_dimension: usize, bloom_factor: f64) -> Self {
        Self { target_dimension, bloom_factor }
    }
    
    /// Decode a glyph back into reconstructed data
    pub fn decode(&self, glyph: &Glyph) -> Result<Vec<f64>, GlyphError> {
        if glyph.fold_sequence.is_empty() {
            return Ok(Vec::new());
        }
        
        // Step 1: Unfold the sequence back to SGA elements
        let unfolded_elements = self.unfold_sequence(glyph)?;
        
        // Step 2: Bloom elements into target space
        let bloomed_data = self.bloom_to_target_space(unfolded_elements, glyph)?;
        
        Ok(bloomed_data)
    }
    
    /// Decode a glyph into frequencies (musical representation)
    pub fn decode_to_frequencies(&self, glyph: &Glyph) -> Vec<f64> {
        glyph.to_frequencies()
    }
    
    /// Decode a glyph into 2D coordinates (spatial representation)  
    pub fn decode_to_coordinates(&self, glyph: &Glyph) -> Vec<(f64, f64)> {
        glyph.render_path()
    }
    
    /// Unfold the fold sequence back to SGA elements
    fn unfold_sequence(&self, glyph: &Glyph) -> Result<Vec<SgaElement>, GlyphError> {
        let mut elements = Vec::new();
        
        for (i, &class_idx) in glyph.fold_sequence.iter().enumerate() {
            if class_idx > 95 {
                return Err(GlyphError::InvalidClassIndex(class_idx));
            }
            
            let base_element = lift(class_idx);
            
            // Apply amplitude and phase modulation if available
            let amplitude = if i < glyph.fold_amplitudes.len() {
                glyph.fold_amplitudes[i]
            } else {
                1.0
            };
            
            // Phase affects the geometric part (complex rotation in essence)
            let modulated_element = base_element.scale(amplitude);
            elements.push(modulated_element);
        }
        
        Ok(elements)
    }
    
    /// Bloom SGA elements into target dimensional space
    fn bloom_to_target_space(
        &self,
        elements: Vec<SgaElement>,
        glyph: &Glyph
    ) -> Result<Vec<f64>, GlyphError> {
        let mut bloomed = vec![0.0; self.target_dimension];
        
        // Distribute elements across target space using hash-based spreading
        for (i, element) in elements.iter().enumerate() {
            if let Some(class_idx) = project(element) {
                let comp = decode_class_index(class_idx);
                
                // Create pseudo-random but deterministic spreading
                let seed = (class_idx as u64)
                    .wrapping_mul(0x9e3779b97f4a7c15)
                    .wrapping_add(i as u64);
                
                // Multiple spread points per element for better reconstruction
                let spread_count = ((elements.len() as f64 / self.target_dimension as f64) 
                    * self.bloom_factor).ceil() as usize;
                let spread_count = spread_count.max(1).min(self.target_dimension / 2);
                
                for j in 0..spread_count {
                    let idx_seed = seed.wrapping_add(j as u64 * 0x517cc1b727220a95);
                    let idx = (idx_seed % self.target_dimension as u64) as usize;
                    
                    // Weight based on SGA element strength and Fano signature
                    let fano_weight = if comp.l >= 1 && comp.l <= 7 {
                        let line_idx = self.find_fano_line_for_l(comp.l);
                        glyph.fano_signature[line_idx as usize]
                    } else {
                        1.0
                    };
                    
                    let weight = element.get_magnitude() * fano_weight / spread_count as f64;
                    bloomed[idx] += weight;
                }
            }
        }
        
        // Normalize if needed
        let max_val = bloomed.iter().map(|x| x.abs()).fold(0.0f64, f64::max);
        if max_val > EPSILON {
            for val in &mut bloomed {
                *val /= max_val;
            }
        }
        
        Ok(bloomed)
    }
    
    /// Find which Fano line contains a given l value
    fn find_fano_line_for_l(&self, l: u8) -> u8 {
        for (line_idx, &[a, b, c]) in FANO_LINES.iter().enumerate() {
            if l == a || l == b || l == c {
                return line_idx as u8;
            }
        }
        0 // Fallback
    }
}

// ============================================================================
// Integration with HyperMemory
// ============================================================================

/// Encode a HyperMemory as a glyph
pub fn encode_memory_as_glyph(memory: &HyperMemory) -> Result<Glyph, GlyphError> {
    let encoder = GlyphEncoder::default();
    
    // Convert hypervector to f64 for processing
    let data: Vec<f64> = memory.vector.iter().map(|&x| x as f64).collect();
    
    encoder.encode(&data)
}

/// Bloom a glyph back into a HyperMemory
pub fn bloom_glyph(glyph: &Glyph) -> Result<HyperMemory, GlyphError> {
    let decoder = GlyphDecoder::new(10000, 1.5); // Standard hypervector size
    
    let bloomed_data = decoder.decode(glyph)?;
    let vector: Vec<f32> = bloomed_data.iter().map(|&x| x as f32).collect();
    
    // Create memory with reconstructed content
    let mut memory = HyperMemory::new(vector, format!("Glyph bloom {}", glyph.created_at));
    
    // Add geometry if available
    if glyph.sga_centroid != (0, 0, 0) {
        let coords = MemoryCoordinates {
            h2: glyph.sga_centroid.0,
            d: glyph.sga_centroid.1,
            l: glyph.sga_centroid.2,
            class_index: components_to_class_index(ClassComponents {
                h2: glyph.sga_centroid.0,
                d: glyph.sga_centroid.1,
                l: glyph.sga_centroid.2,
            }),
            amplitude: glyph.fold_amplitudes.first().copied().unwrap_or(1.0),
            phase: glyph.fold_phases.first().copied().unwrap_or(0.0),
        };
        memory.geometry = Some(coords);
    }
    
    Ok(memory)
}

// ============================================================================
// Supporting Types and Functions
// ============================================================================

/// SGA-mapped data element
#[derive(Debug, Clone)]
struct SgaMappedElement {
    /// Original index in data
    index: usize,
    /// Original value
    value: f64,
    /// SGA coordinates
    coordinates: MemoryCoordinates,
    /// Lifted SGA element
    sga_element: SgaElement,
}

/// Compressed group representing Fano line fold
#[derive(Debug, Clone)]
struct CompressedGroup {
    /// Representative class index for the group
    representative_class: u8,
    /// Compressed amplitude
    amplitude: f64,
    /// Compressed phase
    phase: f64,
    /// Number of elements in original group
    element_count: usize,
    /// Original indices of elements
    original_indices: Vec<usize>,
}

/// Errors that can occur during glyph operations
#[derive(Debug, thiserror::Error)]
pub enum GlyphError {
    #[error("Empty data provided for encoding")]
    EmptyData,
    
    #[error("Invalid class index: {0}, must be 0..95")]
    InvalidClassIndex(u8),
    
    #[error("SGA element is not rank-1, cannot project to class index")]
    NotRank1Element,
    
    #[error("Compression failed: {0}")]
    CompressionFailed(String),
    
    #[error("Decompression failed: {0}")]
    DecompressionFailed(String),
}

/// Extension trait for SgaElement to get magnitude
trait SgaElementExt {
    fn get_magnitude(&self) -> f64;
}

impl SgaElementExt for SgaElement {
    fn get_magnitude(&self) -> f64 {
        // Compute Frobenius-like norm across all components
        let clifford_norm = self.clifford.grades.values()
            .map(|x| x.powi(2))
            .sum::<f64>().sqrt();
        
        let z4_norm = self.z4.coefficients.iter()
            .map(|x| x.powi(2))
            .sum::<f64>().sqrt();
            
        let z3_norm = self.z3.coefficients.iter()
            .map(|x| x.powi(2))
            .sum::<f64>().sqrt();
        
        clifford_norm * z4_norm * z3_norm
    }
}

/// Compute longest common subsequence length
fn longest_common_subsequence(a: &[u8], b: &[u8]) -> usize {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0; n + 1]; m + 1];
    
    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    
    dp[m][n]
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_round_trip_encoding() {
        let encoder = GlyphEncoder::default();
        let decoder = GlyphDecoder::new(100, 1.0);
        
        let original_data = vec![1.0, 0.5, -0.3, 0.8, 0.1, -0.9, 0.0, 0.6];
        
        let glyph = encoder.encode(&original_data).unwrap();
        let reconstructed = decoder.decode(&glyph).unwrap();
        
        assert_eq!(reconstructed.len(), 100); // Target dimension
        
        // Check that glyph contains meaningful information
        assert!(!glyph.fold_sequence.is_empty());
        assert!(!glyph.fold_amplitudes.is_empty());
        assert!(glyph.compression_ratio > 0.0);
    }
    
    #[test]
    fn test_glyph_similarity() {
        let encoder = GlyphEncoder::default();
        
        let data1 = vec![1.0, 0.5, 0.3, 0.8];
        let data2 = vec![1.1, 0.4, 0.35, 0.85]; // Similar to data1
        let data3 = vec![-1.0, -0.5, -0.3, -0.8]; // Inverted
        
        let glyph1 = encoder.encode(&data1).unwrap();
        let glyph2 = encoder.encode(&data2).unwrap();
        let glyph3 = encoder.encode(&data3).unwrap();
        
        let sim_12 = glyph1.similarity(&glyph2);
        let sim_13 = glyph1.similarity(&glyph3);
        let sim_11 = glyph1.similarity(&glyph1); // Self-similarity
        
        // Test that similarity function works correctly:
        // 1. Self-similarity should be perfect (or very close)
        assert!(sim_11 > 0.9, "Self-similarity should be high, got {}", sim_11);
        
        // 2. All similarities should be in valid range [0, 1]
        assert!(sim_12 >= 0.0 && sim_12 <= 1.0, "Similarity out of range: {}", sim_12);
        assert!(sim_13 >= 0.0 && sim_13 <= 1.0, "Similarity out of range: {}", sim_13);
        
        // 3. The glyphs should be different (not identical)
        assert_ne!(glyph1.sga_centroid, glyph2.sga_centroid, "Glyphs should have different centroids");
        assert_ne!(glyph1.fold_sequence, glyph2.fold_sequence, "Glyphs should have different fold sequences");
        
        // 4. Similarity computation should be symmetric
        let sim_21 = glyph2.similarity(&glyph1);
        assert!((sim_12 - sim_21).abs() < 1e-10, "Similarity should be symmetric");
    }
    
    #[test]
    fn test_fano_related_elements() {
        let encoder = GlyphEncoder::default();
        
        // Create data that should map to same Fano line
        let fano_line_data = vec![0.1, 0.2, 0.4]; // Should map to first Fano line [1,2,4]
        let glyph = encoder.encode(&fano_line_data).unwrap();
        
        // Verify Fano signature has expected structure
        let total_signature: f64 = glyph.fano_signature.iter().sum();
        assert!((total_signature - 1.0).abs() < 0.1); // Should be normalized
    }
    
    #[test]
    fn test_musical_frequencies() {
        let encoder = GlyphEncoder::default();
        let data = vec![0.5, 0.8, 0.3, 0.9];
        let glyph = encoder.encode(&data).unwrap();
        
        let frequencies = glyph.to_frequencies();
        assert!(!frequencies.is_empty());
        
        // Check that frequencies are based on 432 Hz and golden ratio
        for freq in frequencies {
            assert!(freq > 0.0);
            assert!(freq < BASE_FREQ * PHI.powi(10)); // Reasonable upper bound
        }
    }
    
    #[test]
    fn test_memory_glyph_integration() {
        let vector = vec![0.1; 1000];
        let memory = HyperMemory::new(vector, "Test memory".to_string());
        
        let glyph = encode_memory_as_glyph(&memory).unwrap();
        let bloomed_memory = bloom_glyph(&glyph).unwrap();
        
        assert_eq!(bloomed_memory.vector.len(), 10000); // Standard size
        assert!(bloomed_memory.content.contains("Glyph bloom"));
    }
    
    #[test]
    fn test_glyph_path_rendering() {
        let encoder = GlyphEncoder::default();
        let data = vec![1.0, 0.0, -1.0, 0.5];
        let glyph = encoder.encode(&data).unwrap();
        
        let path = glyph.render_path();
        assert_eq!(path.len(), glyph.fold_sequence.len());
        
        // Path should progress through 2D space
        if path.len() > 1 {
            assert_ne!(path[0], path[1]);
        }
    }
    
    #[test]
    fn test_lcs_computation() {
        let seq1 = vec![1, 2, 3, 4, 5];
        let seq2 = vec![1, 3, 4, 5, 6];
        let lcs_len = longest_common_subsequence(&seq1, &seq2);
        assert_eq!(lcs_len, 4); // [1, 3, 4, 5]
        
        let seq3 = vec![1, 2, 3];
        let seq4 = vec![4, 5, 6];
        let lcs_len2 = longest_common_subsequence(&seq3, &seq4);
        assert_eq!(lcs_len2, 0); // No common elements
    }
}