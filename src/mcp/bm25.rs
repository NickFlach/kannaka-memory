//! Simple BM25 implementation for keyword search

use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Bm25Index {
    documents: HashMap<Uuid, String>,
    term_frequencies: HashMap<Uuid, HashMap<String, usize>>,
    document_frequencies: HashMap<String, usize>,
    document_lengths: HashMap<Uuid, usize>,
    total_documents: usize,
    average_length: f32,
}

impl Default for Bm25Index {
    fn default() -> Self {
        Self::new()
    }
}

impl Bm25Index {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            term_frequencies: HashMap::new(),
            document_frequencies: HashMap::new(),
            document_lengths: HashMap::new(),
            total_documents: 0,
            average_length: 0.0,
        }
    }

    pub fn add_document(&mut self, id: Uuid, text: &str) {
        let tokens = tokenize(text);
        let mut term_freq = HashMap::new();
        
        // Count term frequencies
        for token in &tokens {
            *term_freq.entry(token.clone()).or_insert(0) += 1;
        }

        // Update document frequencies
        for token in term_freq.keys() {
            if !self.term_frequencies.get(&id).map_or(false, |tf| tf.contains_key(token)) {
                *self.document_frequencies.entry(token.clone()).or_insert(0) += 1;
            }
        }

        // Remove old document if it exists
        if let Some(old_tf) = self.term_frequencies.remove(&id) {
            for token in old_tf.keys() {
                if let Some(df) = self.document_frequencies.get_mut(token) {
                    *df -= 1;
                    if *df == 0 {
                        self.document_frequencies.remove(token);
                    }
                }
            }
        } else {
            self.total_documents += 1;
        }

        // Store new document
        self.documents.insert(id, text.to_string());
        self.term_frequencies.insert(id, term_freq);
        self.document_lengths.insert(id, tokens.len());
        
        // Update average length
        let total_length: usize = self.document_lengths.values().sum();
        self.average_length = total_length as f32 / self.total_documents as f32;
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<(Uuid, f32)> {
        if self.total_documents == 0 {
            return Vec::new();
        }

        let query_tokens = tokenize(query);
        let mut scores = HashMap::new();

        for (doc_id, tf) in &self.term_frequencies {
            let mut score = 0.0;
            let doc_length = self.document_lengths[doc_id] as f32;

            for token in &query_tokens {
                if let Some(&term_freq) = tf.get(token) {
                    let doc_freq = self.document_frequencies.get(token).copied().unwrap_or(0) as f32;
                    if doc_freq > 0.0 {
                        // BM25 parameters
                        let k1 = 1.2;
                        let b = 0.75;

                        // IDF calculation
                        let idf = ((self.total_documents as f32 - doc_freq + 0.5) / (doc_freq + 0.5)).ln();

                        // TF normalization
                        let tf_norm = (term_freq as f32 * (k1 + 1.0)) / 
                                     (term_freq as f32 + k1 * (1.0 - b + b * (doc_length / self.average_length)));

                        score += idf * tf_norm;
                    }
                }
            }

            if score > 0.0 {
                scores.insert(*doc_id, score);
            }
        }

        // Sort by score and return top results
        let mut results: Vec<_> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    pub fn remove_document(&mut self, id: &Uuid) {
        if let Some(tf) = self.term_frequencies.remove(id) {
            // Update document frequencies
            for token in tf.keys() {
                if let Some(df) = self.document_frequencies.get_mut(token) {
                    *df -= 1;
                    if *df == 0 {
                        self.document_frequencies.remove(token);
                    }
                }
            }
            
            self.documents.remove(id);
            self.document_lengths.remove(id);
            self.total_documents -= 1;
            
            // Recalculate average length
            if self.total_documents > 0 {
                let total_length: usize = self.document_lengths.values().sum();
                self.average_length = total_length as f32 / self.total_documents as f32;
            } else {
                self.average_length = 0.0;
            }
        }
    }

    pub fn document_count(&self) -> usize {
        self.total_documents
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_search() {
        let mut index = Bm25Index::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        index.add_document(id1, "the quick brown fox");
        index.add_document(id2, "the lazy brown dog");

        let results = index.search("brown", 10);
        assert_eq!(results.len(), 2);
        assert!(results[0].1 > 0.0);
        assert!(results[1].1 > 0.0);
    }
}