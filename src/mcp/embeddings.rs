//! Ollama embedding client with fallback to simple hash encoding

use serde_json::{json, Value};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("Network error: {0}")]
    Network(#[from] ureq::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Ollama unavailable, using fallback")]
    OllamaUnavailable,
}

/// Get embedding from Ollama or fallback to hash-based encoding
pub fn embed(text: &str, model: &str, base_url: &str) -> Result<Vec<f32>, EmbeddingError> {
    match embed_ollama(text, model, base_url) {
        Ok(embedding) => Ok(embedding),
        Err(EmbeddingError::Network(_)) | Err(EmbeddingError::InvalidResponse(_)) => {
            // Fallback to simple hash encoding
            eprintln!("Ollama unavailable, using hash-based fallback encoding");
            Ok(embed_fallback(text))
        }
        Err(e) => Err(e),
    }
}

fn embed_ollama(text: &str, model: &str, base_url: &str) -> Result<Vec<f32>, EmbeddingError> {
    let url = format!("{}/api/embed", base_url.trim_end_matches('/'));
    
    let request_body = json!({
        "model": model,
        "input": text,
        "options": {
            "temperature": 0.0
        }
    });

    let response = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_json(&request_body)?;

    let response_json: Value = response.into_json()?;

    if let Some(embeddings) = response_json.get("embeddings") {
        if let Some(embedding_array) = embeddings.as_array() {
            if let Some(first_embedding) = embedding_array.first() {
                if let Some(embedding_values) = first_embedding.as_array() {
                    let mut result = Vec::with_capacity(embedding_values.len());
                    for value in embedding_values {
                        if let Some(float_val) = value.as_f64() {
                            result.push(float_val as f32);
                        } else {
                            return Err(EmbeddingError::InvalidResponse(
                                "Embedding value is not a number".to_string()
                            ));
                        }
                    }
                    return Ok(result);
                }
            }
        }
    } else if let Some(embedding) = response_json.get("embedding") {
        // Alternative response format
        if let Some(embedding_values) = embedding.as_array() {
            let mut result = Vec::with_capacity(embedding_values.len());
            for value in embedding_values {
                if let Some(float_val) = value.as_f64() {
                    result.push(float_val as f32);
                } else {
                    return Err(EmbeddingError::InvalidResponse(
                        "Embedding value is not a number".to_string()
                    ));
                }
            }
            return Ok(result);
        }
    }

    Err(EmbeddingError::InvalidResponse(
        "No embedding found in response".to_string()
    ))
}

fn embed_fallback(text: &str) -> Vec<f32> {
    // Simple fallback: create a 384-dimensional vector based on text hashing
    const DIMENSIONS: usize = 384;
    let mut result = vec![0.0; DIMENSIONS];
    
    // Use multiple hash seeds to create diverse features
    for i in 0..DIMENSIONS {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        i.hash(&mut hasher); // Add position-based variation
        
        let hash_val = hasher.finish();
        // Convert hash to float in range [-1, 1]
        result[i] = (hash_val as f32 / u64::MAX as f32) * 2.0 - 1.0;
    }
    
    // Normalize the vector
    let magnitude: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for val in &mut result {
            *val /= magnitude;
        }
    }
    
    result
}

/// Check if Ollama is available
pub fn check_ollama_available(base_url: &str) -> bool {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    ureq::get(&url).call().is_ok()
}

/// Get list of available embedding models from Ollama
pub fn list_embedding_models(base_url: &str) -> Result<Vec<String>, EmbeddingError> {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    
    let response = ureq::get(&url).call()?;
    let response_json: Value = response.into_json()?;
    
    let mut models = Vec::new();
    if let Some(models_array) = response_json.get("models").and_then(|m| m.as_array()) {
        for model in models_array {
            if let Some(name) = model.get("name").and_then(|n| n.as_str()) {
                // Filter for embedding models
                if name.contains("embed") || name.contains("minilm") || name.contains("nomic") {
                    models.push(name.to_string());
                }
            }
        }
    }
    
    Ok(models)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_embedding() {
        let text = "hello world";
        let embedding = embed_fallback(text);
        
        assert_eq!(embedding.len(), 384);
        
        // Check normalization
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 1e-6);
    }

    #[test] 
    fn test_fallback_deterministic() {
        let text = "test text";
        let embedding1 = embed_fallback(text);
        let embedding2 = embed_fallback(text);
        
        assert_eq!(embedding1, embedding2);
    }
}