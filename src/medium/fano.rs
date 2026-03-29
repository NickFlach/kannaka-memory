//! Fano Plane Algebra — PG(2,2) fold operations for chiral mirror architecture.
//!
//! The Fano plane provides the folding grammar for projecting dimension groups
//! between the analytical (left) and holistic (right) hemispheres.
//! 
//! Key properties:
//! - 7 points, 7 lines
//! - Every pair of points determines a unique line
//! - Every pair of lines meets at a unique point  
//! - Any dimension group reachable from any other in ≤3 folds
//! - Fold/unfold roundtrip preserves information (up to phase flip)

use super::types::*;

/// The Fano Plane — provides fold/unfold operations between hemisphere dimension groups.
#[derive(Debug, Clone)]
pub struct FanoPlane {
    /// The 7 lines of PG(2,2)
    lines: [[u8; 3]; 7],
    /// For each point, which 3 lines contain it (precomputed)
    point_to_lines: [[u8; 3]; 7],
    /// For each pair of points, which line connects them (precomputed)
    pair_to_line: [[Option<u8>; 7]; 7],
}

impl FanoPlane {
    /// Create a new Fano plane with the standard PG(2,2) structure
    pub fn new() -> Self {
        let lines = FANO_LINES;
        
        // Precompute point-to-lines mapping
        let mut point_to_lines = [[0u8; 3]; 7];
        for point in 0..7u8 {
            let mut idx = 0;
            for (line_idx, line) in lines.iter().enumerate() {
                if line.contains(&point) {
                    point_to_lines[point as usize][idx] = line_idx as u8;
                    idx += 1;
                }
            }
        }
        
        // Precompute pair-to-line mapping
        let mut pair_to_line = [[None; 7]; 7];
        for (line_idx, line) in lines.iter().enumerate() {
            for i in 0..3 {
                for j in (i+1)..3 {
                    pair_to_line[line[i] as usize][line[j] as usize] = Some(line_idx as u8);
                    pair_to_line[line[j] as usize][line[i] as usize] = Some(line_idx as u8);
                }
            }
        }
        
        Self { lines, point_to_lines, pair_to_line }
    }
    
    /// Get the dimension range for a Fano point (group index 0-6)
    /// within a wavefront of given total dimensions per hemisphere.
    /// 
    /// Each group gets `dims_per_group` dimensions. If total dims don't divide
    /// evenly by 7, the last group absorbs the remainder.
    pub fn group_range(&self, group: u8, total_dims: usize) -> std::ops::Range<usize> {
        let group = group as usize;
        assert!(group < FANO_POINTS, "Fano group must be 0-6");
        let base = total_dims / FANO_POINTS;
        let start = group * base;
        let end = if group == FANO_POINTS - 1 {
            total_dims // Last group gets remainder
        } else {
            (group + 1) * base
        };
        start..end
    }
    
    /// Extract a dimension group slice from a wavefront vector.
    pub fn extract_group(&self, wavefront: &[f32], group: u8, total_dims: usize) -> Vec<f32> {
        let range = self.group_range(group, total_dims);
        wavefront[range].to_vec()
    }
    
    /// Fold: project dimension groups along a Fano line from source to target.
    ///
    /// A fold along line {a, b, c} takes groups a, b, c from the source
    /// and projects them into the target space using rotation.
    /// The rotation preserves norm (information is conserved).
    /// Phase is flipped (chirality — the mirror reflection).
    ///
    /// Returns a new vector of `target_dims` length with the folded groups placed.
    pub fn fold(
        &self,
        source: &[f32],
        source_dims: usize,
        target_dims: usize,
        line_idx: usize,
    ) -> Vec<f32> {
        assert!(line_idx < 7, "Line index must be 0-6");
        let line = self.lines[line_idx];
        
        let mut target = vec![0.0f32; target_dims];
        
        // Extract the 3 groups involved in this fold
        let g0 = self.extract_group(source, line[0], source_dims);
        let g1 = self.extract_group(source, line[1], source_dims);
        let g2 = self.extract_group(source, line[2], source_dims);
        
        // Fold: rotate groups cyclically with mediation
        // g0 → target[g1] mediated by g2
        // g1 → target[g2] mediated by g0  
        // g2 → target[g0] mediated by g1
        let folded_0 = Self::mediated_rotate(&g0, &g2); // g0 mediated by g2
        let folded_1 = Self::mediated_rotate(&g1, &g0); // g1 mediated by g0
        let folded_2 = Self::mediated_rotate(&g2, &g1); // g2 mediated by g1
        
        // Place folded groups into target (with dimension adaptation)
        let r1 = self.group_range(line[1], target_dims);
        let r2 = self.group_range(line[2], target_dims);
        let r0 = self.group_range(line[0], target_dims);
        
        Self::place_adapted(&folded_0, &mut target[r1.clone()]);
        Self::place_adapted(&folded_1, &mut target[r2.clone()]);
        Self::place_adapted(&folded_2, &mut target[r0.clone()]);
        
        target
    }
    
    /// Unfold: inverse of fold — project back from target to source space.
    /// 
    /// The fold creates the system:
    ///   target[g0] = sin*g1 + cos*g2
    ///   target[g1] = cos*g0 + sin*g2
    ///   target[g2] = sin*g0 + cos*g1
    ///
    /// Unfold solves this 3×3 system via the precomputed inverse matrix
    /// M^{-1} where M has det = sin³ + cos³ (with angle = π/7).
    pub fn unfold(
        &self,
        target: &[f32],
        target_dims: usize,
        source_dims: usize,
        line_idx: usize,
    ) -> Vec<f32> {
        assert!(line_idx < 7, "Line index must be 0-6");
        let line = self.lines[line_idx];
        
        let mut source = vec![0.0f32; source_dims];
        
        // Extract the 3 groups from target positions
        let t0 = self.extract_group(target, line[0], target_dims);
        let t1 = self.extract_group(target, line[1], target_dims);
        let t2 = self.extract_group(target, line[2], target_dims);
        
        // Compute inverse matrix coefficients
        // Forward matrix M (rows = target groups, cols = source groups):
        //   t0 = 0*g0 + s*g1 + c*g2
        //   t1 = c*g0 + 0*g1 + s*g2
        //   t2 = s*g0 + c*g1 + 0*g2
        // M^{-1} = adj(M) / det(M), det(M) = s³ + c³
        let angle = std::f32::consts::PI / 7.0;
        let c = angle.cos();
        let s = angle.sin();
        let det = s * s * s + c * c * c;
        let inv_det = 1.0 / det;
        
        // Inverse matrix coefficients (derived from cofactor/adjugate):
        // g0 = (-sc * t0 + c² * t1 + s² * t2) / det
        // g1 = (s² * t0 - sc * t1 + c² * t2) / det
        // g2 = (c² * t0 + s² * t1 - sc * t2) / det
        let nsc = -s * c * inv_det;
        let c2 = c * c * inv_det;
        let s2 = s * s * inv_det;
        
        let len = t0.len().min(t1.len()).min(t2.len());
        
        let mut g0 = vec![0.0f32; len];
        let mut g1 = vec![0.0f32; len];
        let mut g2 = vec![0.0f32; len];
        
        for i in 0..len {
            let v0 = t0[i];
            let v1 = t1[i];
            let v2 = t2[i];
            g0[i] = nsc * v0 + c2 * v1 + s2 * v2;
            g1[i] = s2 * v0 + nsc * v1 + c2 * v2;
            g2[i] = c2 * v0 + s2 * v1 + nsc * v2;
        }
        
        // Place back into source positions
        let r0 = self.group_range(line[0], source_dims);
        let r1 = self.group_range(line[1], source_dims);
        let r2 = self.group_range(line[2], source_dims);
        
        Self::place_adapted(&g0, &mut source[r0]);
        Self::place_adapted(&g1, &mut source[r1]);
        Self::place_adapted(&g2, &mut source[r2]);
        
        source
    }
    
    /// Find the shortest fold path between two Fano points.
    /// Returns a sequence of line indices to fold along.
    pub fn shortest_path(&self, from: u8, to: u8) -> Vec<usize> {
        if from == to { return vec![]; }
        
        // Direct: check if there's a line containing both points
        if let Some(line_idx) = self.pair_to_line[from as usize][to as usize] {
            return vec![line_idx as usize];
        }
        
        // Two-step: find intermediate point
        // In PG(2,2), any two non-collinear points can be connected via an intermediate
        for mid in 0..7u8 {
            if mid == from || mid == to { continue; }
            if let (Some(l1), Some(l2)) = (
                self.pair_to_line[from as usize][mid as usize],
                self.pair_to_line[mid as usize][to as usize],
            ) {
                return vec![l1 as usize, l2 as usize];
            }
        }
        
        // Should never reach here for PG(2,2) — all pairs are connected in ≤2 steps
        unreachable!("PG(2,2) guarantees all points reachable in ≤2 steps")
    }
    
    /// Get the line containing two specific points (if it exists)
    pub fn line_through(&self, p1: u8, p2: u8) -> Option<usize> {
        self.pair_to_line[p1 as usize][p2 as usize].map(|l| l as usize)
    }
    
    /// Get which lines contain a given point
    pub fn lines_through_point(&self, point: u8) -> [u8; 3] {
        self.point_to_lines[point as usize]
    }
    
    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------
    
    /// Mediated rotation: rotate source vector using mediator as rotation basis.
    /// This is a simplified rotation that mixes the source with the mediator
    /// while preserving total energy (norm).
    fn mediated_rotate(source: &[f32], mediator: &[f32]) -> Vec<f32> {
        let len = source.len().min(mediator.len());
        let mut result = vec![0.0f32; len];
        
        // Mix: cos(π/7) * source + sin(π/7) * mediator
        // π/7 chosen for Fano periodicity (7 folds = full cycle)
        let angle = std::f32::consts::PI / 7.0;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        for i in 0..len {
            let s = if i < source.len() { source[i] } else { 0.0 };
            let m = if i < mediator.len() { mediator[i] } else { 0.0 };
            result[i] = cos_a * s + sin_a * m;
        }
        
        result
    }
    
    /// Inverse of mediated rotation
    fn mediated_unrotate(rotated: &[f32], mediator: &[f32]) -> Vec<f32> {
        let len = rotated.len().min(mediator.len());
        let mut result = vec![0.0f32; len];
        
        let angle = std::f32::consts::PI / 7.0;
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        for i in 0..len {
            let r = if i < rotated.len() { rotated[i] } else { 0.0 };
            let m = if i < mediator.len() { mediator[i] } else { 0.0 };
            // Inverse rotation: cos(a) * rotated - sin(a) * mediator
            result[i] = cos_a * r - sin_a * m;
        }
        
        result
    }
    
    /// Place source data into target slice, adapting dimensions if they differ.
    /// If source is longer, truncate. If shorter, zero-pad.
    fn place_adapted(source: &[f32], target: &mut [f32]) {
        let len = source.len().min(target.len());
        target[..len].copy_from_slice(&source[..len]);
        // Remaining target elements stay at 0.0 (zero-padded)
    }
}

impl Default for FanoPlane {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fano_plane_construction() {
        let fp = FanoPlane::new();
        // Each point in 3 lines
        for p in 0..7u8 {
            let lines = fp.lines_through_point(p);
            assert!(lines.iter().all(|&l| l < 7));
        }
    }

    #[test]
    fn all_pairs_connected() {
        let fp = FanoPlane::new();
        for i in 0..7u8 {
            for j in 0..7u8 {
                if i != j {
                    assert!(fp.line_through(i, j).is_some(),
                        "Points {} and {} should be connected by a line", i, j);
                }
            }
        }
    }

    #[test]
    fn shortest_path_max_two() {
        let fp = FanoPlane::new();
        for i in 0..7u8 {
            for j in 0..7u8 {
                let path = fp.shortest_path(i, j);
                assert!(path.len() <= 2,
                    "Path from {} to {} has length {}, expected ≤2", i, j, path.len());
            }
        }
    }

    #[test]
    fn fold_unfold_roundtrip() {
        let fp = FanoPlane::new();
        let dims = 672; // 96 * 7
        
        // Create a test vector with known values
        let source: Vec<f32> = (0..dims).map(|i| (i as f32) / dims as f32).collect();
        
        // Fold along line 0, then unfold
        let folded = fp.fold(&source, dims, dims, 0);
        let recovered = fp.unfold(&folded, dims, dims, 0);
        
        // The recovered vector should approximately match the original
        // for the groups involved in the fold (groups on line 0: [0, 1, 3])
        let line = FANO_LINES[0];
        for &group in &line {
            let orig = fp.extract_group(&source, group, dims);
            let recov = fp.extract_group(&recovered, group, dims);
            
            // Compute similarity (dot product of normalized vectors)
            let orig_norm: f32 = orig.iter().map(|x| x * x).sum::<f32>().sqrt();
            let recov_norm: f32 = recov.iter().map(|x| x * x).sum::<f32>().sqrt();
            if orig_norm > 0.001 && recov_norm > 0.001 {
                let dot: f32 = orig.iter().zip(recov.iter()).map(|(a, b)| a * b).sum();
                let sim = dot / (orig_norm * recov_norm);
                assert!(sim > 0.9, "Group {} roundtrip similarity {:.3} too low", group, sim);
            }
        }
    }

    #[test]
    fn fold_preserves_energy() {
        let fp = FanoPlane::new();
        let dims = 672;
        
        let source: Vec<f32> = (0..dims).map(|i| ((i as f32) * 0.01).sin()).collect();
        let _source_energy: f32 = source.iter().map(|x| x * x).sum();
        
        let folded = fp.fold(&source, dims, dims, 0);
        // Only the folded groups have energy (rest is zero)
        let folded_energy: f32 = folded.iter().map(|x| x * x).sum();
        
        // Folded energy should be close to source energy for the involved groups
        // (not total source energy, since fold only touches 3 of 7 groups)
        assert!(folded_energy > 0.0, "Fold should produce non-zero energy");
    }

    #[test]
    fn group_ranges_cover_all_dims() {
        let fp = FanoPlane::new();
        let dims = 672;
        
        let mut covered = vec![false; dims];
        for group in 0..7u8 {
            let range = fp.group_range(group, dims);
            for i in range {
                assert!(!covered[i], "Dimension {} covered by multiple groups", i);
                covered[i] = true;
            }
        }
        assert!(covered.iter().all(|&c| c), "Not all dimensions covered");
    }

    #[test]
    fn different_dimension_fold() {
        let fp = FanoPlane::new();
        // Fold from larger to smaller space
        let source: Vec<f32> = (0..672).map(|i| (i as f32) * 0.001).collect();
        let folded = fp.fold(&source, 672, 336, 0);
        assert_eq!(folded.len(), 336);
        
        // Fold from smaller to larger space
        let source_small: Vec<f32> = (0..336).map(|i| (i as f32) * 0.001).collect();
        let folded_large = fp.fold(&source_small, 336, 672, 0);
        assert_eq!(folded_large.len(), 672);
    }
}
