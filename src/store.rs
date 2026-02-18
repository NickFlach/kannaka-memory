//! Storage trait and ghostvector backend.
//! Placeholder for Phase 2 implementation.

use crate::memory::HyperMemory;
use uuid::Uuid;

/// Trait for memory storage backends.
pub trait MemoryStore {
    fn insert(&mut self, memory: HyperMemory) -> Result<(), String>;
    fn get(&self, id: &Uuid) -> Option<&HyperMemory>;
    fn search(&self, query: &[f32], top_k: usize) -> Vec<&HyperMemory>;
}
