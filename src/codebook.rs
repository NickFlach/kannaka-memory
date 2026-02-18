use rand::Rng;
use rand_chacha::ChaCha8Rng;
use rand::SeedableRng;

use crate::wave::normalize;

/// A codebook holding a random projection matrix for mapping embeddings
/// into hypervector space.
pub struct Codebook {
    /// Projection matrix stored as flat row-major: input_dim × output_dim
    matrix: Vec<f32>,
    pub input_dim: usize,
    pub output_dim: usize,
    seed: u64,
}

impl Codebook {
    /// Create a new codebook with a seeded random projection matrix.
    /// Each element is drawn from N(0, 1/sqrt(output_dim)) for variance preservation.
    pub fn new(input_dim: usize, output_dim: usize, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let scale = 1.0 / (output_dim as f32).sqrt();
        let len = input_dim * output_dim;
        let mut matrix = Vec::with_capacity(len);
        for _ in 0..len {
            // Box-Muller for normal distribution
            let u1: f32 = rng.gen::<f32>().max(1e-10);
            let u2: f32 = rng.gen();
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos();
            matrix.push(z * scale);
        }
        Self { matrix, input_dim, output_dim, seed }
    }

    /// Project an input embedding to hypervector space and normalize to unit length.
    pub fn project(&self, embedding: &[f32]) -> Vec<f32> {
        assert_eq!(embedding.len(), self.input_dim, "embedding dim mismatch");
        let mut out = vec![0.0f32; self.output_dim];
        for (i, &val) in embedding.iter().enumerate() {
            let row_start = i * self.output_dim;
            for j in 0..self.output_dim {
                out[j] += val * self.matrix[row_start + j];
            }
        }
        normalize(&mut out);
        out
    }

    /// Get the seed used to generate this codebook.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Generate a random atomic hypervector (unit length) using the codebook's RNG lineage.
    pub fn random_vector(&self) -> Vec<f32> {
        // Use a derived seed so it's deterministic but different from matrix generation
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed.wrapping_add(0xCAFE));
        let mut v: Vec<f32> = (0..self.output_dim).map(|_| {
            let u1: f32 = rng.gen::<f32>().max(1e-10);
            let u2: f32 = rng.gen();
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos()
        }).collect();
        normalize(&mut v);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave::cosine_similarity;

    #[test]
    fn projected_vectors_are_unit_length() {
        let cb = Codebook::new(384, 10_000, 42);
        let embedding = vec![1.0f32; 384];
        let hv = cb.project(&embedding);
        let norm: f32 = hv.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "norm was {}", norm);
    }

    #[test]
    fn reproducible_with_same_seed() {
        let cb1 = Codebook::new(128, 10_000, 99);
        let cb2 = Codebook::new(128, 10_000, 99);
        let emb = vec![0.5f32; 128];
        let hv1 = cb1.project(&emb);
        let hv2 = cb2.project(&emb);
        let sim = cosine_similarity(&hv1, &hv2);
        assert!((sim - 1.0).abs() < 1e-6, "same seed should produce identical projections");
    }

    #[test]
    fn random_vector_is_unit_length() {
        let cb = Codebook::new(128, 10_000, 42);
        let v = cb.random_vector();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "norm was {}", norm);
    }

    #[test]
    fn different_inputs_produce_different_vectors() {
        let cb = Codebook::new(128, 10_000, 42);
        let a = cb.project(&vec![1.0; 128]);
        let b = cb.project(&vec![-1.0; 128]);
        let sim = cosine_similarity(&a, &b);
        assert!(sim < 0.0, "opposite inputs should produce negatively correlated vectors");
    }
}
