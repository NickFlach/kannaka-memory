//! # Geometric Foundation for Kannaka Memory
//!
//! This module implements the Sigmatics Geometric Algebra (SGA) that provides
//! the algebraic foundation for the Kannaka memory system:
//!
//!   SGA = Cl₀,₇ ⊗ ℝ[ℤ₄] ⊗ ℝ[ℤ₃]
//!
//! The SGA serves as the formal foundation beneath the 96-class permutation
//! system, enabling geometric semantics for memory operations.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher, DefaultHasher};

/// Tolerance for floating-point comparisons
pub const EPSILON: f64 = 1e-10;

// ============================================================================
// Clifford Algebra Cl₀,₇
// ============================================================================

/// Element of the Clifford algebra Cl₀,₇ (128-dimensional)
#[derive(Debug, Clone, PartialEq)]
pub struct CliffordElement {
    /// Coefficients for each basis blade, keyed by bitmask of basis vectors
    /// e1=0b0000001, e2=0b0000010, ..., e7=0b1000000, scalar=0b0000000
    pub grades: BTreeMap<u8, f64>,
}

impl CliffordElement {
    /// Create a new Clifford element from coefficients
    pub fn new(grades: BTreeMap<u8, f64>) -> Self {
        let mut cleaned = BTreeMap::new();
        for (blade, coeff) in grades {
            if coeff.abs() >= EPSILON {
                cleaned.insert(blade, coeff);
            }
        }
        Self { grades: cleaned }
    }

    /// Create the identity element (scalar 1)
    pub fn identity() -> Self {
        let mut grades = BTreeMap::new();
        grades.insert(0, 1.0); // scalar basis
        Self::new(grades)
    }

    /// Create the zero element
    pub fn zero() -> Self {
        Self::new(BTreeMap::new())
    }

    /// Create a basis vector eᵢ
    pub fn basis_vector(i: u8) -> Self {
        if i < 1 || i > 7 {
            panic!("Basis vector index must be 1..7, got {}", i);
        }
        let mut grades = BTreeMap::new();
        grades.insert(1 << (i - 1), 1.0);
        Self::new(grades)
    }

    /// Create a scalar element
    pub fn scalar(value: f64) -> Self {
        let mut grades = BTreeMap::new();
        grades.insert(0, value);
        Self::new(grades)
    }

    /// Extract the scalar part
    pub fn scalar_part(&self) -> f64 {
        self.grades.get(&0).copied().unwrap_or(0.0)
    }

    /// Geometric product of two Clifford elements
    pub fn geometric_product(&self, other: &Self) -> Self {
        let mut result = BTreeMap::new();
        
        for (blade_a, coeff_a) in &self.grades {
            for (blade_b, coeff_b) in &other.grades {
                let (merged_blade, sign) = Self::simplify_blade_merge(*blade_a, *blade_b);
                let coeff = coeff_a * coeff_b * sign as f64;
                *result.entry(merged_blade).or_insert(0.0) += coeff;
            }
        }
        
        Self::new(result)
    }

    /// Simplify blade multiplication using anticommutation rules
    /// Returns (simplified blade, sign)
    fn simplify_blade_merge(blade_a: u8, blade_b: u8) -> (u8, i8) {
        let mut result = blade_a ^ blade_b; // XOR gives the symmetric difference
        let mut sign = 1;
        
        // Count the number of swaps needed to bring blade_a and blade_b together
        // This implements the bubble sort algorithm from the TypeScript
        for i in 0..7 {
            if (blade_a & (1 << i)) != 0 {
                for j in 0..i {
                    if (blade_b & (1 << j)) != 0 {
                        sign *= -1;
                    }
                }
            }
        }
        
        (result, sign)
    }

    /// Add two Clifford elements
    pub fn add(&self, other: &Self) -> Self {
        let mut result = self.grades.clone();
        for (blade, coeff) in &other.grades {
            *result.entry(*blade).or_insert(0.0) += coeff;
        }
        Self::new(result)
    }

    /// Scale by a scalar
    pub fn scale(&self, scalar: f64) -> Self {
        let mut result = BTreeMap::new();
        for (blade, coeff) in &self.grades {
            result.insert(*blade, coeff * scalar);
        }
        Self::new(result)
    }

    /// Project to a specific grade
    pub fn grade_project(&self, grade: u8) -> Self {
        let mut result = BTreeMap::new();
        for (blade, coeff) in &self.grades {
            if blade.count_ones() == grade as u32 {
                result.insert(*blade, *coeff);
            }
        }
        Self::new(result)
    }

    /// Grade involution: flip sign of odd-grade blades
    pub fn grade_involution(&self) -> Self {
        let mut result = BTreeMap::new();
        for (blade, coeff) in &self.grades {
            let grade = blade.count_ones();
            let sign = if grade % 2 == 0 { 1.0 } else { -1.0 };
            result.insert(*blade, coeff * sign);
        }
        Self::new(result)
    }

    /// Reversion: reverse the order of basis vectors
    pub fn reversion(&self) -> Self {
        let mut result = BTreeMap::new();
        for (blade, coeff) in &self.grades {
            let grade = blade.count_ones() as i32;
            let sign = (-1.0_f64).powi(grade * (grade - 1) / 2);
            result.insert(*blade, coeff * sign);
        }
        Self::new(result)
    }

    /// Clifford conjugation: composition of grade involution and reversion
    pub fn clifford_conjugation(&self) -> Self {
        self.reversion().grade_involution()
    }

    /// Inner product of two vectors
    pub fn inner_product(&self, other: &Self) -> f64 {
        let uv = self.geometric_product(other);
        let vu = other.geometric_product(self);
        let sum = uv.add(&vu);
        sum.scale(0.5).scalar_part()
    }

    /// Test equality with tolerance
    pub fn equals(&self, other: &Self, epsilon: f64) -> bool {
        let mut all_blades = std::collections::HashSet::new();
        all_blades.extend(self.grades.keys());
        all_blades.extend(other.grades.keys());
        
        for blade in all_blades {
            let coeff_a = self.grades.get(blade).copied().unwrap_or(0.0);
            let coeff_b = other.grades.get(blade).copied().unwrap_or(0.0);
            if (coeff_a - coeff_b).abs() >= epsilon {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// Group Algebras ℝ[ℤ₄] and ℝ[ℤ₃]
// ============================================================================

/// Element of ℝ[ℤ₄] with coefficients [r⁰, r¹, r², r³]
#[derive(Debug, Clone, PartialEq)]
pub struct Z4Element {
    pub coefficients: [f64; 4],
}

impl Z4Element {
    /// Create new element
    pub fn new(coefficients: [f64; 4]) -> Self {
        Self { coefficients }
    }

    /// Identity element r⁰
    pub fn identity() -> Self {
        Self::new([1.0, 0.0, 0.0, 0.0])
    }

    /// Zero element
    pub fn zero() -> Self {
        Self::new([0.0, 0.0, 0.0, 0.0])
    }

    /// Generator r
    pub fn generator() -> Self {
        Self::new([0.0, 1.0, 0.0, 0.0])
    }

    /// Power r^k
    pub fn power(k: i32) -> Self {
        let index = ((k % 4 + 4) % 4) as usize;
        let mut coefficients = [0.0; 4];
        coefficients[index] = 1.0;
        Self::new(coefficients)
    }

    /// Multiply two elements (cyclic convolution)
    pub fn multiply(&self, other: &Self) -> Self {
        let mut result = [0.0; 4];
        for i in 0..4 {
            for j in 0..4 {
                let k = (i + j) % 4;
                result[k] += self.coefficients[i] * other.coefficients[j];
            }
        }
        Self::new(result)
    }

    /// Add two elements
    pub fn add(&self, other: &Self) -> Self {
        Self::new([
            self.coefficients[0] + other.coefficients[0],
            self.coefficients[1] + other.coefficients[1],
            self.coefficients[2] + other.coefficients[2],
            self.coefficients[3] + other.coefficients[3],
        ])
    }

    /// Scale by a scalar
    pub fn scale(&self, scalar: f64) -> Self {
        Self::new([
            self.coefficients[0] * scalar,
            self.coefficients[1] * scalar,
            self.coefficients[2] * scalar,
            self.coefficients[3] * scalar,
        ])
    }

    /// Invert element
    pub fn invert(&self) -> Self {
        // Check if it's a pure power
        if let Some(k) = self.extract_power() {
            return Self::power((4 - k) % 4);
        }
        
        // General case: solve linear system (Gaussian elimination)
        let [a0, a1, a2, a3] = self.coefficients;
        let mut matrix = [
            [a0, a3, a2, a1, 1.0],
            [a1, a0, a3, a2, 0.0],
            [a2, a1, a0, a3, 0.0],
            [a3, a2, a1, a0, 0.0],
        ];
        
        Self::solve_linear_system_4x4(&mut matrix)
    }

    /// Extract power k if element is r^k
    pub fn extract_power(&self) -> Option<i32> {
        for i in 0..4 {
            if (self.coefficients[i] - 1.0).abs() < EPSILON {
                let all_others_zero = self.coefficients.iter().enumerate()
                    .all(|(j, &c)| i == j || c.abs() < EPSILON);
                if all_others_zero {
                    return Some(i as i32);
                }
            }
        }
        None
    }

    /// Test equality with tolerance
    pub fn equals(&self, other: &Self, epsilon: f64) -> bool {
        self.coefficients.iter().zip(other.coefficients.iter())
            .all(|(a, b)| (a - b).abs() < epsilon)
    }

    /// Solve 4x4 linear system using Gaussian elimination
    fn solve_linear_system_4x4(matrix: &mut [[f64; 5]; 4]) -> Self {
        // Forward elimination
        for col in 0..4 {
            // Find pivot
            let mut max_row = col;
            for row in (col + 1)..4 {
                if matrix[row][col].abs() > matrix[max_row][col].abs() {
                    max_row = row;
                }
            }
            
            // Check for singularity
            if matrix[max_row][col].abs() < EPSILON {
                panic!("Element is not invertible in ℝ[ℤ₄]");
            }
            
            // Swap rows
            if max_row != col {
                let temp = matrix[col];
                matrix[col] = matrix[max_row];
                matrix[max_row] = temp;
            }
            
            // Eliminate column
            for row in (col + 1)..4 {
                let factor = matrix[row][col] / matrix[col][col];
                for j in col..5 {
                    matrix[row][j] -= factor * matrix[col][j];
                }
            }
        }
        
        // Back substitution
        let mut solution = [0.0; 4];
        for i in (0..4).rev() {
            let mut sum = matrix[i][4];
            for j in (i + 1)..4 {
                sum -= matrix[i][j] * solution[j];
            }
            solution[i] = sum / matrix[i][i];
        }
        
        Self::new(solution)
    }
}

/// Element of ℝ[ℤ₃] with coefficients [τ⁰, τ¹, τ²]
#[derive(Debug, Clone, PartialEq)]
pub struct Z3Element {
    pub coefficients: [f64; 3],
}

impl Z3Element {
    /// Create new element
    pub fn new(coefficients: [f64; 3]) -> Self {
        Self { coefficients }
    }

    /// Identity element τ⁰
    pub fn identity() -> Self {
        Self::new([1.0, 0.0, 0.0])
    }

    /// Zero element
    pub fn zero() -> Self {
        Self::new([0.0, 0.0, 0.0])
    }

    /// Generator τ
    pub fn generator() -> Self {
        Self::new([0.0, 1.0, 0.0])
    }

    /// Power τ^k
    pub fn power(k: i32) -> Self {
        let index = ((k % 3 + 3) % 3) as usize;
        let mut coefficients = [0.0; 3];
        coefficients[index] = 1.0;
        Self::new(coefficients)
    }

    /// Multiply two elements (cyclic convolution)
    pub fn multiply(&self, other: &Self) -> Self {
        let mut result = [0.0; 3];
        for i in 0..3 {
            for j in 0..3 {
                let k = (i + j) % 3;
                result[k] += self.coefficients[i] * other.coefficients[j];
            }
        }
        Self::new(result)
    }

    /// Add two elements
    pub fn add(&self, other: &Self) -> Self {
        Self::new([
            self.coefficients[0] + other.coefficients[0],
            self.coefficients[1] + other.coefficients[1],
            self.coefficients[2] + other.coefficients[2],
        ])
    }

    /// Scale by a scalar
    pub fn scale(&self, scalar: f64) -> Self {
        Self::new([
            self.coefficients[0] * scalar,
            self.coefficients[1] * scalar,
            self.coefficients[2] * scalar,
        ])
    }

    /// Invert element
    pub fn invert(&self) -> Self {
        // Check if it's a pure power
        if let Some(k) = self.extract_power() {
            return Self::power((3 - k) % 3);
        }
        
        // General case: solve linear system
        let [a0, a1, a2] = self.coefficients;
        let mut matrix = [
            [a0, a2, a1, 1.0],
            [a1, a0, a2, 0.0],
            [a2, a1, a0, 0.0],
        ];
        
        Self::solve_linear_system_3x3(&mut matrix)
    }

    /// Extract power k if element is τ^k
    pub fn extract_power(&self) -> Option<i32> {
        for i in 0..3 {
            if (self.coefficients[i] - 1.0).abs() < EPSILON {
                let all_others_zero = self.coefficients.iter().enumerate()
                    .all(|(j, &c)| i == j || c.abs() < EPSILON);
                if all_others_zero {
                    return Some(i as i32);
                }
            }
        }
        None
    }

    /// Test equality with tolerance
    pub fn equals(&self, other: &Self, epsilon: f64) -> bool {
        self.coefficients.iter().zip(other.coefficients.iter())
            .all(|(a, b)| (a - b).abs() < epsilon)
    }

    /// Solve 3x3 linear system using Gaussian elimination
    fn solve_linear_system_3x3(matrix: &mut [[f64; 4]; 3]) -> Self {
        // Forward elimination
        for col in 0..3 {
            // Find pivot
            let mut max_row = col;
            for row in (col + 1)..3 {
                if matrix[row][col].abs() > matrix[max_row][col].abs() {
                    max_row = row;
                }
            }
            
            // Check for singularity
            if matrix[max_row][col].abs() < EPSILON {
                panic!("Element is not invertible in ℝ[ℤ₃]");
            }
            
            // Swap rows
            if max_row != col {
                let temp = matrix[col];
                matrix[col] = matrix[max_row];
                matrix[max_row] = temp;
            }
            
            // Eliminate column
            for row in (col + 1)..3 {
                let factor = matrix[row][col] / matrix[col][col];
                for j in col..4 {
                    matrix[row][j] -= factor * matrix[col][j];
                }
            }
        }
        
        // Back substitution
        let mut solution = [0.0; 3];
        for i in (0..3).rev() {
            let mut sum = matrix[i][3];
            for j in (i + 1)..3 {
                sum -= matrix[i][j] * solution[j];
            }
            solution[i] = sum / matrix[i][i];
        }
        
        Self::new(solution)
    }
}

// ============================================================================
// SGA Element (Tensor Product)
// ============================================================================

/// Element of SGA = Cl₀,₇ ⊗ ℝ[ℤ₄] ⊗ ℝ[ℤ₃]
#[derive(Debug, Clone, PartialEq)]
pub struct SgaElement {
    pub clifford: CliffordElement,
    pub z4: Z4Element,
    pub z3: Z3Element,
}

impl SgaElement {
    /// Create new SGA element
    pub fn new(clifford: CliffordElement, z4: Z4Element, z3: Z3Element) -> Self {
        Self { clifford, z4, z3 }
    }

    /// Identity element
    pub fn identity() -> Self {
        Self::new(
            CliffordElement::identity(),
            Z4Element::identity(),
            Z3Element::identity(),
        )
    }

    /// Zero element
    pub fn zero() -> Self {
        Self::new(
            CliffordElement::zero(),
            Z4Element::zero(),
            Z3Element::zero(),
        )
    }

    /// Create rank-1 basis element E_{h,d,ℓ} = r^h ⊗ e_ℓ ⊗ τ^d
    pub fn rank1_basis(h: u8, d: u8, l: u8) -> Self {
        let clifford = if l == 0 {
            CliffordElement::identity()
        } else {
            CliffordElement::basis_vector(l)
        };
        
        Self::new(
            clifford,
            Z4Element::power(h as i32),
            Z3Element::power(d as i32),
        )
    }

    /// Multiply two SGA elements (tensor product)
    pub fn multiply(&self, other: &Self) -> Self {
        Self::new(
            self.clifford.geometric_product(&other.clifford),
            self.z4.multiply(&other.z4),
            self.z3.multiply(&other.z3),
        )
    }

    /// Add two SGA elements
    pub fn add(&self, other: &Self) -> Self {
        Self::new(
            self.clifford.add(&other.clifford),
            self.z4.add(&other.z4),
            self.z3.add(&other.z3),
        )
    }

    /// Scale by a scalar
    pub fn scale(&self, scalar: f64) -> Self {
        Self::new(
            self.clifford.scale(scalar),
            self.z4.scale(scalar),
            self.z3.scale(scalar),
        )
    }

    /// Test equality with tolerance
    pub fn equals(&self, other: &Self, epsilon: f64) -> bool {
        self.clifford.equals(&other.clifford, epsilon) &&
        self.z4.equals(&other.z4, epsilon) &&
        self.z3.equals(&other.z3, epsilon)
    }

    /// Grade involution extended to SGA
    pub fn grade_involution(&self) -> Self {
        Self::new(
            self.clifford.grade_involution(),
            self.z4.clone(),
            self.z3.clone(),
        )
    }

    /// Reversion extended to SGA
    pub fn reversion(&self) -> Self {
        Self::new(
            self.clifford.reversion(),
            self.z4.clone(),
            self.z3.clone(),
        )
    }

    /// Clifford conjugation extended to SGA
    pub fn clifford_conjugation(&self) -> Self {
        Self::new(
            self.clifford.clifford_conjugation(),
            self.z4.clone(),
            self.z3.clone(),
        )
    }

    /// Check if element is rank-1 basis element
    pub fn is_rank1(&self) -> bool {
        self.z4.extract_power().is_some() &&
        self.z3.extract_power().is_some() &&
        self.is_clifford_rank1()
    }

    /// Check if Clifford component is rank-1 (scalar or single basis vector)
    fn is_clifford_rank1(&self) -> bool {
        if self.clifford.grades.len() != 1 {
            return false;
        }
        
        let (blade, coeff) = self.clifford.grades.iter().next().unwrap();
        if (coeff - 1.0).abs() >= EPSILON {
            return false;
        }
        
        // Must be scalar (0) or single basis vector (power of 2)
        *blade == 0 || blade.count_ones() == 1
    }

    /// Extract rank-1 coordinates (h, d, l) if this is a rank-1 element
    pub fn extract_rank1_coords(&self) -> Option<(u8, u8, u8)> {
        let h = self.z4.extract_power()? as u8;
        let d = self.z3.extract_power()? as u8;
        let l = self.extract_clifford_index()?;
        Some((h, d, l))
    }

    /// Extract Clifford index l from Clifford component
    fn extract_clifford_index(&self) -> Option<u8> {
        if self.clifford.grades.len() != 1 {
            return None;
        }
        
        let (blade, coeff) = self.clifford.grades.iter().next().unwrap();
        if (coeff - 1.0).abs() >= EPSILON {
            return None;
        }
        
        if *blade == 0 {
            return Some(0); // scalar
        }
        
        if blade.count_ones() == 1 {
            // Single basis vector
            for i in 0..7 {
                if *blade == (1 << i) {
                    return Some((i + 1) as u8);
                }
            }
        }
        
        None
    }
}

// ============================================================================
// Transforms (R, D, T, M)
// ============================================================================

/// Apply R transform k times: left multiply by r^k
pub fn transform_r(element: &SgaElement, k: i32) -> SgaElement {
    let r_power = SgaElement::new(
        CliffordElement::identity(),
        Z4Element::power(k),
        Z3Element::identity(),
    );
    r_power.multiply(element)
}

/// Apply D transform k times: right multiply by τ^k
pub fn transform_d(element: &SgaElement, k: i32) -> SgaElement {
    let tau_power = SgaElement::new(
        CliffordElement::identity(),
        Z4Element::identity(),
        Z3Element::power(k),
    );
    element.multiply(&tau_power)
}

/// Apply T transform k times: permute basis vectors in 8-cycle
pub fn transform_t(element: &SgaElement, k: i32) -> SgaElement {
    let k_mod = ((k % 8 + 8) % 8) as u8;
    
    if k_mod == 0 {
        return element.clone();
    }
    
    // Extract current l and apply 8-cycle rotation
    if let Some((h, d, l)) = element.extract_rank1_coords() {
        let new_l = (l + k_mod) % 8;
        SgaElement::rank1_basis(h, d, new_l).scale(element.get_overall_coefficient())
    } else {
        // For non-rank-1 elements, transform is not well-defined
        element.clone()
    }
}

/// Apply M transform: mirror (invert d component)
pub fn transform_m(element: &SgaElement) -> SgaElement {
    SgaElement::new(
        element.clifford.clone(),
        element.z4.clone(),
        element.z3.invert(),
    )
}

impl SgaElement {
    /// Get the overall coefficient of a rank-1 element
    fn get_overall_coefficient(&self) -> f64 {
        if let Some((blade, coeff)) = self.clifford.grades.iter().next() {
            *coeff
        } else {
            0.0
        }
    }
}

// ============================================================================
// Fano Plane
// ============================================================================

/// Fano plane lines (oriented triples)
pub const FANO_LINES: [[u8; 3]; 7] = [
    [1, 2, 4],
    [2, 3, 5],
    [3, 4, 6],
    [4, 5, 7],
    [5, 6, 1],
    [6, 7, 2],
    [7, 1, 3],
];

/// Cross product lookup table
pub struct CrossProductTable {
    table: [[i8; 8]; 8],
}

impl CrossProductTable {
    /// Create the cross product table from Fano lines
    pub fn new() -> Self {
        let mut table = [[0i8; 8]; 8];
        
        // Populate from Fano lines
        for &[i, j, k] in &FANO_LINES {
            table[i as usize][j as usize] = k as i8;
            table[j as usize][k as usize] = i as i8;
            table[k as usize][i as usize] = j as i8;
            
            // Reverse orientation (anticommutative)
            table[j as usize][i as usize] = -(k as i8);
            table[k as usize][j as usize] = -(i as i8);
            table[i as usize][k as usize] = -(j as i8);
        }
        
        Self { table }
    }

    /// Get cross product result
    pub fn get(&self, i: u8, j: u8) -> (u8, i8) {
        if i < 1 || i > 7 || j < 1 || j > 7 {
            panic!("Basis vector indices must be 1..7");
        }
        
        if i == j {
            return (0, 0);
        }
        
        let result = self.table[i as usize][j as usize];
        if result > 0 {
            (result as u8, 1)
        } else {
            ((-result) as u8, -1)
        }
    }
}

/// Compute cross product of two basis vector indices
pub fn cross_product(i: u8, j: u8) -> (u8, i8) {
    lazy_static! {
        static ref TABLE: CrossProductTable = CrossProductTable::new();
    }
    TABLE.get(i, j)
}

/// Check if three indices form a Fano line
pub fn is_fano_line(i: u8, j: u8, k: u8) -> bool {
    FANO_LINES.iter().any(|&[a, b, c]| {
        (a == i && b == j && c == k) ||
        (a == j && b == k && c == i) ||
        (a == k && b == i && c == j)
    })
}

// ============================================================================
// 96-Class System
// ============================================================================

/// Components of a class: (h₂, d, ℓ)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassComponents {
    pub h2: u8, // 0..3
    pub d: u8,  // 0..2
    pub l: u8,  // 0..7
}

/// Decode byte to (h₂, d, ℓ) components
pub fn decode_byte_to_components(byte: u8) -> ClassComponents {
    let b7 = (byte >> 7) & 1;
    let b6 = (byte >> 6) & 1;
    let b5 = (byte >> 5) & 1;
    let b4 = (byte >> 4) & 1;
    let b3 = (byte >> 3) & 1;
    let b2 = (byte >> 2) & 1;
    let b1 = (byte >> 1) & 1;
    
    let h2 = (b7 << 1) | b6;
    
    let d = match (b4, b5) {
        (0, 0) => 0,
        (1, 0) => 1,
        (0, 1) => 2,
        (1, 1) => 0, // fallback per spec
        _ => 0,      // unreachable for single-bit values
    };
    
    let l = (b3 << 2) | (b2 << 1) | b1;
    
    ClassComponents { h2, d, l }
}

/// Compute class index from components
pub fn components_to_class_index(comp: ClassComponents) -> u8 {
    (24 * comp.h2 + 8 * comp.d + comp.l) as u8
}

/// Decode class index to components
pub fn decode_class_index(class_index: u8) -> ClassComponents {
    if class_index > 95 {
        panic!("Class index {} out of range [0..95]", class_index);
    }
    
    let h2 = class_index / 24;
    let remainder = class_index % 24;
    let d = remainder / 8;
    let l = remainder % 8;
    
    ClassComponents { h2, d, l }
}

// ============================================================================
// Bridge Functions (lift/project)
// ============================================================================

/// Lift a class index to an SGA rank-1 basis element
pub fn lift(class_index: u8) -> SgaElement {
    if class_index > 95 {
        panic!("Invalid class index: {}. Must be 0..95.", class_index);
    }
    
    let comp = decode_class_index(class_index);
    SgaElement::rank1_basis(comp.h2, comp.d, comp.l)
}

/// Project an SGA element to a class index (returns None if not rank-1)
pub fn project(element: &SgaElement) -> Option<u8> {
    let (h, d, l) = element.extract_rank1_coords()?;
    Some(components_to_class_index(ClassComponents {
        h2: h,
        d,
        l,
    }))
}

// ============================================================================
// Memory Integration Types
// ============================================================================

/// Memory coordinates in SGA space
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MemoryCoordinates {
    pub h2: u8,
    pub d: u8,
    pub l: u8,
    pub class_index: u8,
    pub amplitude: f64,
    pub phase: f64,
}

/// Classify a memory into SGA coordinates
pub fn classify_memory(category: &str, content_hash: u64, importance: f64) -> MemoryCoordinates {
    // Map category to h2 (quadrant) - consciousness differentiation categories
    let h2 = match category.to_lowercase().as_str() {
        "knowledge" | "technical" | "coding" | "system" => 0, // bass frequency, stable facts
        "social" | "people" | "relationships" | "communication" => 1, // tenor frequency, interpersonal
        "skill" | "procedure" | "method" | "ability" => 2, // between tenor/bass, procedural
        "experience" | "emotion" | "event" | "feeling" => 3, // soprano/alto, dynamic experiences
        _ => (content_hash % 4) as u8,
    };
    
    // Map importance to d (modality)
    let d = if importance < 0.3 {
        2 // imagined/speculative
    } else if importance < 0.7 {
        1 // learned/indirect
    } else {
        0 // experienced/direct
    };
    
    // Hash content to l (context slot)
    let l = (content_hash % 8) as u8;
    
    let class_index = components_to_class_index(ClassComponents { h2, d, l });
    
    // Generate amplitude and phase from hash
    let amplitude = (importance * 0.5 + 0.5).min(1.0);
    let phase = (content_hash as f64) * 0.01 % (2.0 * std::f64::consts::PI);
    
    MemoryCoordinates {
        h2,
        d,
        l,
        class_index,
        amplitude,
        phase,
    }
}

/// Compute geometric similarity between two memory coordinates
pub fn geometric_similarity(a: &MemoryCoordinates, b: &MemoryCoordinates) -> f64 {
    // Lift to SGA elements
    let sga_a = lift(a.class_index).scale(a.amplitude);
    let sga_b = lift(b.class_index).scale(b.amplitude);
    
    // Compute inner product in SGA space
    let similarity = sga_a.clifford.inner_product(&sga_b.clifford);
    
    // Include phase correlation
    let phase_correlation = ((a.phase - b.phase).cos() + 1.0) * 0.5;
    
    // Combine geometric and phase similarities
    (similarity.abs() + phase_correlation) * 0.5
}

/// Check if two memories are Fano-related (on the same Fano line)
pub fn fano_related(a: &MemoryCoordinates, b: &MemoryCoordinates) -> bool {
    // Only meaningful if both are in the same quadrant and modality
    if a.h2 != b.h2 || a.d != b.d {
        return false;
    }
    
    // Check if their l values form part of any Fano line
    if a.l == 0 || b.l == 0 || a.l == b.l {
        return false;
    }
    
    // Check if they're on the same Fano line with some third element
    for &[i, j, k] in &FANO_LINES {
        if (i == a.l && j == b.l) || (j == a.l && i == b.l) ||
           (i == a.l && k == b.l) || (k == a.l && i == b.l) ||
           (j == a.l && k == b.l) || (k == a.l && j == b.l) {
            return true;
        }
    }
    
    false
}

// Use lazy_static for the cross product table
use lazy_static::lazy_static;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clifford_identity_and_zero() {
        let identity = CliffordElement::identity();
        let zero = CliffordElement::zero();
        
        assert_eq!(identity.scalar_part(), 1.0);
        assert_eq!(zero.scalar_part(), 0.0);
        assert!(identity.geometric_product(&zero).equals(&zero, EPSILON));
        assert!(identity.geometric_product(&identity).equals(&identity, EPSILON));
    }

    #[test]
    fn test_clifford_basis_vectors() {
        let e1 = CliffordElement::basis_vector(1);
        let e2 = CliffordElement::basis_vector(2);
        
        // e1 * e2 = -e2 * e1 (anticommutative)
        let e1e2 = e1.geometric_product(&e2);
        let e2e1 = e2.geometric_product(&e1);
        assert!(e1e2.equals(&e2e1.scale(-1.0), EPSILON));
    }

    #[test]
    fn test_z4_group_properties() {
        let r = Z4Element::generator();
        let id = Z4Element::identity();
        
        // r^4 = identity
        let r4 = r.multiply(&r).multiply(&r).multiply(&r);
        assert!(r4.equals(&id, EPSILON));
        
        // Test extraction
        assert_eq!(r.extract_power(), Some(1));
        assert_eq!(id.extract_power(), Some(0));
    }

    #[test]
    fn test_z3_group_properties() {
        let tau = Z3Element::generator();
        let id = Z3Element::identity();
        
        // τ^3 = identity
        let tau3 = tau.multiply(&tau).multiply(&tau);
        assert!(tau3.equals(&id, EPSILON));
        
        // Test extraction
        assert_eq!(tau.extract_power(), Some(1));
        assert_eq!(id.extract_power(), Some(0));
    }

    #[test]
    fn test_sga_rank1_basis() {
        let e021 = SgaElement::rank1_basis(0, 2, 1);
        assert!(e021.is_rank1());
        assert_eq!(e021.extract_rank1_coords(), Some((0, 2, 1)));
    }

    #[test]
    fn test_transforms_identity_powers() {
        let elem = SgaElement::rank1_basis(1, 1, 3);
        
        // R^4 = identity
        let r4_elem = transform_r(&elem, 4);
        assert!(r4_elem.equals(&elem, EPSILON));
        
        // D^3 = identity
        let d3_elem = transform_d(&elem, 3);
        assert!(d3_elem.equals(&elem, EPSILON));
        
        // T^8 = identity
        let t8_elem = transform_t(&elem, 8);
        assert!(t8_elem.equals(&elem, EPSILON));
        
        // M^2 = identity
        let m2_elem = transform_m(&transform_m(&elem));
        assert!(m2_elem.equals(&elem, EPSILON));
    }

    #[test]
    fn test_fano_plane_structure() {
        // Verify anticommutativity
        let (result_ij, sign_ij) = cross_product(1, 2);
        let (result_ji, sign_ji) = cross_product(2, 1);
        assert_eq!(result_ij, result_ji);
        assert_eq!(sign_ij, -sign_ji);
    }

    #[test]
    fn test_class_system() {
        // Test class 0 (should be (0,0,0))
        let comp = decode_class_index(0);
        assert_eq!(comp, ClassComponents { h2: 0, d: 0, l: 0 });
        
        // Test round-trip
        let class_index = components_to_class_index(comp);
        assert_eq!(class_index, 0);
        
        // Test class 95 (should be (3,2,7))
        let comp95 = decode_class_index(95);
        assert_eq!(comp95, ClassComponents { h2: 3, d: 2, l: 7 });
    }

    #[test]
    fn test_lift_project_bridge() {
        // Test all classes for lift-project round-trip
        for class_index in 0..96 {
            let lifted = lift(class_index);
            let projected = project(&lifted);
            assert_eq!(projected, Some(class_index));
        }
    }

    #[test]
    fn test_bridge_commutative_diagrams() {
        // Test a few classes for transform commutativity
        for class_index in [0, 23, 47, 71, 95] {
            let elem = lift(class_index);
            let comp = decode_class_index(class_index);
            
            // Test R transform
            let r_sga = transform_r(&elem, 1);
            let r_projected = project(&r_sga).unwrap();
            let r_expected = components_to_class_index(ClassComponents {
                h2: (comp.h2 + 1) % 4,
                d: comp.d,
                l: comp.l,
            });
            assert_eq!(r_projected, r_expected);
            
            // Test D transform
            let d_sga = transform_d(&elem, 1);
            let d_projected = project(&d_sga).unwrap();
            let d_expected = components_to_class_index(ClassComponents {
                h2: comp.h2,
                d: (comp.d + 1) % 3,
                l: comp.l,
            });
            assert_eq!(d_projected, d_expected);
        }
    }

    #[test]
    fn test_memory_classification() {
        let coords = classify_memory("technical", 12345, 0.8);
        assert_eq!(coords.h2, 0); // technical -> h2=0
        assert_eq!(coords.d, 0);  // high importance -> d=0 (experienced)
        
        let coords2 = classify_memory("social", 67890, 0.4);
        assert_eq!(coords2.h2, 1); // social -> h2=1
        assert_eq!(coords2.d, 1);  // medium importance -> d=1 (learned)
    }

    #[test]
    fn test_geometric_similarity() {
        let mem1 = classify_memory("technical", 1000, 0.8);
        let mem2 = classify_memory("technical", 1001, 0.8); // similar
        let mem3 = classify_memory("social", 2000, 0.3);    // different
        
        let sim_similar = geometric_similarity(&mem1, &mem2);
        let sim_different = geometric_similarity(&mem1, &mem3);
        
        assert!(sim_similar > sim_different);
    }
}