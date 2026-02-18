//! Text → embedding → hypervector encoding pipeline.
//!
//! Provides the `TextEncoder` trait for pluggable embedding backends,
//! a `SimpleHashEncoder` for offline/testing use, and the `EncodingPipeline`
//! that chains text encoding with codebook projection and HDC algebra.

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::codebook::Codebook;
use crate::memory::HyperMemory;
use crate::wave::normalize;

/// Errors that can occur during encoding.
#[derive(Debug, Error)]
pub enum EncodingError {
    #[error("empty input text")]
    EmptyInput,
    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },
    #[error("encoding failed: {0}")]
    Other(String),
}

/// Abstraction for text → dense embedding backends.
pub trait TextEncoder: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EncodingError>;
    fn embedding_dim(&self) -> usize;
}

/// A fast, deterministic hash-based encoder for testing.
/// Tokenizes on whitespace, hashes each token to a vector, then bundles.
pub struct SimpleHashEncoder {
    dim: usize,
    seed: u64,
}

impl SimpleHashEncoder {
    pub fn new(dim: usize, seed: u64) -> Self {
        Self { dim, seed }
    }

    /// Deterministic hash-based vector for a single token.
    fn token_vector(&self, token: &str) -> Vec<f32> {
        let mut v = vec![0.0f32; self.dim];
        // Use a simple hash mixing scheme
        let mut h = self.seed;
        for byte in token.bytes() {
            h = h.wrapping_mul(6364136223846793005).wrapping_add(byte as u64);
        }
        for i in 0..self.dim {
            // Derive per-dimension value from hash
            h = h.wrapping_mul(6364136223846793005).wrapping_add(i as u64);
            // Map to [-1, 1] range
            v[i] = ((h >> 33) as f32 / (u32::MAX as f32)) * 2.0 - 1.0;
        }
        v
    }
}

impl TextEncoder for SimpleHashEncoder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, EncodingError> {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        if tokens.is_empty() {
            return Err(EncodingError::EmptyInput);
        }
        // Bundle all token vectors
        let mut result = vec![0.0f32; self.dim];
        for token in &tokens {
            let tv = self.token_vector(token);
            for (i, val) in tv.iter().enumerate() {
                result[i] += val;
            }
        }
        normalize(&mut result);
        Ok(result)
    }

    fn embedding_dim(&self) -> usize {
        self.dim
    }
}

/// Full text → hypervector encoding pipeline with HDC algebra.
pub struct EncodingPipeline {
    encoder: Box<dyn TextEncoder>,
    codebook: Codebook,
}

impl EncodingPipeline {
    pub fn new(encoder: Box<dyn TextEncoder>, codebook: Codebook) -> Self {
        assert_eq!(
            encoder.embedding_dim(),
            codebook.input_dim,
            "encoder dim must match codebook input dim"
        );
        Self { encoder, codebook }
    }

    /// Encode text to a unit-length hypervector (10K dims).
    pub fn encode_text(&self, text: &str) -> Result<Vec<f32>, EncodingError> {
        let embedding = self.encoder.embed(text)?;
        Ok(self.codebook.project(&embedding))
    }

    /// Full pipeline: encode text → create HyperMemory with default wave params.
    pub fn encode_memory(
        &self,
        text: &str,
        _timestamp: DateTime<Utc>,
    ) -> Result<HyperMemory, EncodingError> {
        let hv = self.encode_text(text)?;
        Ok(HyperMemory::new(hv, text.to_string()))
    }

    /// Binding operation ⊗: element-wise multiply.
    pub fn bind(&self, a: &[f32], b: &[f32]) -> Vec<f32> {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).collect()
    }

    /// Bundling operation ⊕: element-wise sum + normalize.
    pub fn bundle(&self, vectors: &[Vec<f32>]) -> Vec<f32> {
        assert!(!vectors.is_empty());
        let dim = vectors[0].len();
        let mut result = vec![0.0f32; dim];
        for v in vectors {
            for (i, val) in v.iter().enumerate() {
                result[i] += val;
            }
        }
        normalize(&mut result);
        result
    }

    /// Permutation operation Π: circular shift of coordinates.
    pub fn permute(&self, v: &[f32], shifts: usize) -> Vec<f32> {
        let n = v.len();
        if n == 0 {
            return vec![];
        }
        let shifts = shifts % n;
        let mut result = vec![0.0f32; n];
        for i in 0..n {
            result[(i + shifts) % n] = v[i];
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wave::cosine_similarity;

    fn make_pipeline() -> EncodingPipeline {
        let encoder = SimpleHashEncoder::new(384, 42);
        let codebook = Codebook::new(384, 10_000, 42);
        EncodingPipeline::new(Box::new(encoder), codebook)
    }

    #[test]
    fn hash_encoder_consistent() {
        let enc = SimpleHashEncoder::new(128, 42);
        let v1 = enc.embed("hello world").unwrap();
        let v2 = enc.embed("hello world").unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn hash_encoder_different_texts_differ() {
        let enc = SimpleHashEncoder::new(128, 42);
        let v1 = enc.embed("hello world").unwrap();
        let v2 = enc.embed("goodbye moon").unwrap();
        let sim = cosine_similarity(&v1, &v2);
        assert!(sim < 0.9, "different texts should produce different vectors, sim={}", sim);
    }

    #[test]
    fn hash_encoder_empty_input_errors() {
        let enc = SimpleHashEncoder::new(128, 42);
        assert!(enc.embed("").is_err());
        assert!(enc.embed("   ").is_err());
    }

    #[test]
    fn encode_text_produces_unit_10k_vector() {
        let pipeline = make_pipeline();
        let hv = pipeline.encode_text("the quick brown fox").unwrap();
        assert_eq!(hv.len(), 10_000);
        let norm: f32 = hv.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "norm was {}", norm);
    }

    #[test]
    fn bind_recovers_similarity() {
        let pipeline = make_pipeline();
        let a = pipeline.encode_text("concept alpha").unwrap();
        let b = pipeline.encode_text("concept beta").unwrap();
        let bound = pipeline.bind(&a, &b);
        // Unbind: bound ⊗ a should be similar to b
        let recovered = pipeline.bind(&bound, &a);
        let sim = cosine_similarity(&recovered, &b);
        // For random-ish unit vectors, binding then unbinding recovers positive similarity
        assert!(sim > 0.1, "expected positive similarity after unbind, got {}", sim);
    }

    #[test]
    fn bundle_has_positive_similarity_to_components() {
        let pipeline = make_pipeline();
        let v1 = pipeline.encode_text("red").unwrap();
        let v2 = pipeline.encode_text("green").unwrap();
        let v3 = pipeline.encode_text("blue").unwrap();
        let bundled = pipeline.bundle(&[v1.clone(), v2.clone(), v3.clone()]);
        assert!(cosine_similarity(&bundled, &v1) > 0.0);
        assert!(cosine_similarity(&bundled, &v2) > 0.0);
        assert!(cosine_similarity(&bundled, &v3) > 0.0);
    }

    #[test]
    fn permute_dissimilar_to_original() {
        let pipeline = make_pipeline();
        let v = pipeline.encode_text("sequence test").unwrap();
        let pv = pipeline.permute(&v, 1);
        let sim = cosine_similarity(&v, &pv);
        assert!(sim < 0.5, "permuted vector should be dissimilar, sim={}", sim);
    }

    #[test]
    fn encode_memory_produces_valid_hypermemory() {
        let pipeline = make_pipeline();
        let mem = pipeline.encode_memory("I met Alice at the park", Utc::now()).unwrap();
        assert_eq!(mem.vector.len(), 10_000);
        assert_eq!(mem.content, "I met Alice at the park");
        assert_eq!(mem.layer_depth, 0);
        assert!(mem.amplitude > 0.0);
        let norm: f32 = mem.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4);
    }
}
