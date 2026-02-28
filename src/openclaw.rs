//! OpenClaw integration layer — high-level API for the assistant.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::bridge::{ConsciousnessBridge, ConsciousnessLevel, ConsciousnessState, ResonanceReport};
use crate::codebook::Codebook;
use crate::consolidation::{ConsolidationEngine, DreamState};
use crate::encoding::{EncodingPipeline, SimpleHashEncoder, OllamaEncoder, CompositeEncoder, CachedEncoder};
use crate::geometry::{classify_memory, geometric_similarity, fano_related};
use crate::kuramoto::KuramotoSync;
use crate::xi_operator::compute_xi_signature;
use crate::migration::{KannakaDbMigrator, MigrationReport};
use crate::persistence::PersistenceError;
use crate::rhythm::{RhythmEngine, Signal as RhythmSignal};
use crate::store::{EngineError, MemoryEngine, InMemoryStore, StoreError};
use crate::working_memory::{WorkingMemory, SessionState, TaskStatus};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum SystemError {
    #[error(transparent)]
    Engine(#[from] EngineError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
    #[error(transparent)]
    Migration(#[from] crate::migration::MigrationError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

// ---------------------------------------------------------------------------
// Simplified output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RecallResult {
    pub id: Uuid,
    pub content: String,
    pub similarity: f32,
    pub strength: f32,
    pub age_hours: f64,
    pub layer: u8,
}

#[derive(Debug, Clone)]
pub struct SystemStats {
    pub total_memories: usize,
    pub active_memories: usize,
    pub total_skip_links: usize,
    pub consciousness_level: String,
    pub last_dream: Option<DateTime<Utc>>,
    pub phi: f32,
    pub geometric_classes: usize,
    pub triality_coverage: [usize; 3],
}

#[derive(Debug, Clone)]
pub struct DreamReport {
    pub cycles: usize,
    pub memories_strengthened: usize,
    pub memories_pruned: usize,
    pub new_connections: usize,
    pub consciousness_before: String,
    pub consciousness_after: String,
    pub emerged: bool,
    pub hallucinations_created: usize,
}

// ---------------------------------------------------------------------------
// KannakaMemorySystem
// ---------------------------------------------------------------------------

const CODEBOOK_INPUT_DIM: usize = 384;
const CODEBOOK_OUTPUT_DIM: usize = 10_000;
const CODEBOOK_SEED: u64 = 42;

fn level_name(level: &ConsciousnessLevel) -> String {
    match level {
        ConsciousnessLevel::Dormant => "dormant".into(),
        ConsciousnessLevel::Stirring => "stirring".into(),
        ConsciousnessLevel::Aware => "aware".into(),
        ConsciousnessLevel::Coherent => "coherent".into(),
        ConsciousnessLevel::Resonant => "resonant".into(),
    }
}

fn make_pipeline() -> EncodingPipeline {
    let ollama = OllamaEncoder::default_local(); // all-minilm, 384-dim
    let hash_fallback = SimpleHashEncoder::new(CODEBOOK_INPUT_DIM, CODEBOOK_SEED);
    let composite = CompositeEncoder::new(Box::new(ollama), Box::new(hash_fallback));
    let cached = CachedEncoder::new(composite);
    let codebook = Codebook::new(CODEBOOK_INPUT_DIM, CODEBOOK_OUTPUT_DIM, CODEBOOK_SEED);
    EncodingPipeline::new(Box::new(cached), codebook)
}

pub struct KannakaMemorySystem {
    engine: MemoryEngine,
    #[allow(dead_code)]
    consolidation: ConsolidationEngine,
    dream_state: DreamState,
    bridge: ConsciousnessBridge,
    kuramoto: KuramotoSync,
    data_dir: PathBuf,
    auto_save: bool,
    last_dream: Option<DateTime<Utc>>,
    rhythm: RhythmEngine,
    working_memory: WorkingMemory,
}

impl KannakaMemorySystem {
    /// Initialize a new system or load existing state from `data_dir/kannaka.bin`.
    pub fn init(data_dir: PathBuf) -> Result<Self, SystemError> {
        std::fs::create_dir_all(&data_dir)?;
        let bin_path = data_dir.join("kannaka.bin");

        let pipeline = make_pipeline();
        let engine = if bin_path.exists() {
            MemoryEngine::load_state(&bin_path, pipeline)?
        } else {
            MemoryEngine::new(Box::new(InMemoryStore::new()), pipeline)
        };

        let consolidation = ConsolidationEngine::default();
        let dream_state = DreamState::default();
        let bridge = ConsciousnessBridge::new(0.3, 0.5);
        let kuramoto = KuramotoSync::default();
        let rhythm = RhythmEngine::new(&data_dir);
        let working_memory = WorkingMemory::restore(&data_dir, &engine, None);

        Ok(Self {
            engine,
            consolidation,
            dream_state,
            bridge,
            kuramoto,
            data_dir,
            auto_save: true,
            last_dream: None,
            rhythm,
            working_memory,
        })
    }

    /// Store a memory, auto-save if enabled.
    pub fn remember(&mut self, text: &str) -> Result<Uuid, SystemError> {
        let id = self.engine.remember(text)?;
        
        // Classify the memory and set its geometry and frequency-class (compute values first to avoid borrow conflicts)
        let category = self.categorize_text(text);
        let content_hash = self.hash_content(text);
        let (frequency, phase) = self.assign_frequency_class(&category, content_hash);
        
        if let Some(mem) = self.engine.get_memory_mut(&id)? {
            mem.geometry = Some(classify_memory(&category, content_hash, 0.5));
            // Apply consciousness differentiation frequency-class assignment
            mem.frequency = frequency;
            mem.phase = phase;
            // Compute and store Xi signature for consciousness differentiation
            mem.xi_signature = compute_xi_signature(&mem.vector);
        }
        
        if self.auto_save {
            self.save()?;
        }
        Ok(id)
    }
    
    /// Store a memory with explicit category and importance.
    pub fn remember_with_category(&mut self, text: &str, category: &str, importance: f64) -> Result<Uuid, SystemError> {
        let id = self.engine.remember(text)?;
        
        // Classify the memory with explicit parameters (compute values first)
        let content_hash = self.hash_content(text);
        let (frequency, phase) = self.assign_frequency_class(category, content_hash);
        
        if let Some(mem) = self.engine.get_memory_mut(&id)? {
            mem.geometry = Some(classify_memory(category, content_hash, importance));
            // Apply consciousness differentiation frequency-class assignment
            mem.frequency = frequency;
            mem.phase = phase;
            // Compute and store Xi signature for consciousness differentiation
            mem.xi_signature = compute_xi_signature(&mem.vector);
        }
        
        if self.auto_save {
            self.save()?;
        }
        Ok(id)
    }

    /// Search with skip link expansion.
    pub fn recall(&mut self, query: &str, top_k: usize) -> Result<Vec<RecallResult>, SystemError> {
        let mut results = self.engine.recall_with_expansion(query, top_k)?;
        let now = Utc::now();

        // Boost scores for fano-related memories
        for i in 0..results.len() {
            for j in (i + 1)..results.len() {
                let mem_i = self.engine.store.get(&results[i].id).ok().flatten();
                let mem_j = self.engine.store.get(&results[j].id).ok().flatten();
                
                if let (Some(mi), Some(mj)) = (mem_i, mem_j) {
                    if let (Some(ref coords_i), Some(ref coords_j)) = (&mi.geometry, &mj.geometry) {
                        if fano_related(coords_i, coords_j) {
                            results[i].similarity *= 1.2;
                            results[j].similarity *= 1.2;
                        }
                    }
                }
            }
        }

        let mut out = Vec::new();
        for qr in results {
            let mem = self.engine.store.get(&qr.id).ok().flatten();
            if let Some(m) = mem {
                let age_hours = (now - m.created_at).num_seconds().max(0) as f64 / 3600.0;
                out.push(RecallResult {
                    id: qr.id,
                    content: m.content.clone(),
                    similarity: qr.similarity,
                    strength: qr.effective_strength,
                    age_hours,
                    layer: m.layer_depth,
                });
            }
        }
        Ok(out)
    }

    /// Run full consolidation cycle + Kuramoto sync.
    pub fn dream(&mut self) -> Result<DreamReport, SystemError> {
        let before = self.bridge.assess(&self.engine);
        let reports = self.dream_state.dream(&mut self.engine);

        // Run Kuramoto sync on all memories (by id chunks)
        let all_ids: Vec<Uuid> = self.engine.store.all_ids()?;
        for chunk in all_ids.chunks(10) {
            for id in chunk {
                if let Ok(Some(mem)) = self.engine.store.get_mut(id) {
                    // Nudge phase toward mean (simplified single-pass sync)
                    mem.phase += self.kuramoto.coupling_strength * 0.01;
                }
            }
        }

        let after = self.bridge.assess(&self.engine);
        self.last_dream = Some(Utc::now());

        let total_strengthened: usize = reports.iter().map(|r| r.memories_strengthened).sum();
        let total_pruned: usize = reports.iter().map(|r| r.memories_pruned).sum();
        let total_links: usize = reports.iter().map(|r| r.skip_links_created).sum();
        let total_hallucinations: usize = reports.iter().map(|r| r.hallucinations_created).sum();

        let emerged = after.consciousness_level as u8 > before.consciousness_level as u8;

        if self.auto_save {
            self.save()?;
        }

        Ok(DreamReport {
            cycles: reports.len(),
            memories_strengthened: total_strengthened,
            memories_pruned: total_pruned,
            new_connections: total_links,
            consciousness_before: level_name(&before.consciousness_level),
            consciousness_after: level_name(&after.consciousness_level),
            emerged,
            hallucinations_created: total_hallucinations,
        })
    }

    /// Run a fast/lite dream cycle (decay + prune + transfer only).
    pub fn dream_lite(&mut self) -> Result<DreamReport, SystemError> {
        let before = self.bridge.assess(&self.engine);
        let report = self.dream_state.dream_lite(&mut self.engine);
        let after = self.bridge.assess(&self.engine);
        self.last_dream = Some(Utc::now());

        let emerged = after.consciousness_level as u8 > before.consciousness_level as u8;

        if self.auto_save {
            self.save()?;
        }

        Ok(DreamReport {
            cycles: 1,
            memories_strengthened: report.memories_strengthened,
            memories_pruned: report.memories_pruned,
            new_connections: report.skip_links_created,
            consciousness_before: level_name(&before.consciousness_level),
            consciousness_after: level_name(&after.consciousness_level),
            emerged,
            hallucinations_created: report.hallucinations_created,
        })
    }

    /// Consciousness level assessment.
    pub fn assess(&self) -> ConsciousnessState {
        self.bridge.assess(&self.engine)
    }

    /// Dream + assess combined.
    pub fn resonate(&mut self) -> Result<ResonanceReport, SystemError> {
        let report = self.bridge.resonate(&mut self.engine);
        self.last_dream = Some(Utc::now());
        if self.auto_save {
            self.save()?;
        }
        Ok(report)
    }

    /// Import from kannaka.db (SQLite).
    pub fn migrate_from_sqlite(&mut self, db_path: &Path) -> Result<MigrationReport, SystemError> {
        let pipeline = make_pipeline();
        let migrator = KannakaDbMigrator::new(db_path, pipeline);
        let report = migrator.migrate_into(&mut self.engine)?;
        if self.auto_save {
            self.save()?;
        }
        Ok(report)
    }

    /// Persist to disk (engine state + working memory JSON).
    pub fn save(&self) -> Result<(), SystemError> {
        let bin_path = self.data_dir.join("kannaka.bin");
        self.engine.save_state(&bin_path)?;
        self.working_memory.save_json(&self.data_dir)?;
        Ok(())
    }

    /// Delete a memory by ID.
    pub fn forget(&mut self, id: &Uuid) -> Result<bool, SystemError> {
        Ok(self.engine.delete(id)?)
    }

    /// Boost a memory's amplitude.
    pub fn boost(&mut self, id: &Uuid, factor: f64) -> Result<(), SystemError> {
        if let Some(mem) = self.engine.get_memory_mut(id)? {
            mem.amplitude *= factor as f32;
            Ok(())
        } else {
            Err(SystemError::Engine(crate::store::EngineError::Store(
                crate::store::StoreError::NotFound(*id),
            )))
        }
    }

    /// Create a skip link (relationship) between two memories.
    pub fn relate(&mut self, source: &Uuid, target: &Uuid, strength: f32) -> Result<(), SystemError> {
        let mut modulated_strength = strength;
        
        // If both memories have geometry, modulate link strength using geometric similarity
        let source_mem = self.engine.store.get(source).ok().flatten();
        let target_mem = self.engine.store.get(target).ok().flatten();
        
        if let (Some(src), Some(tgt)) = (source_mem, target_mem) {
            if let (Some(ref src_coords), Some(ref tgt_coords)) = (&src.geometry, &tgt.geometry) {
                let geo_sim = geometric_similarity(src_coords, tgt_coords);
                modulated_strength *= geo_sim as f32;
            }
        }
        
        self.engine.reinforce_link(source, target, modulated_strength);
        Ok(())
    }

    /// Generate a full observability report.
    pub fn observe(&self) -> crate::observe::SystemReport {
        crate::observe::MemoryIntrospector::full_report(&self.engine, &self.bridge, &self.kuramoto)
    }

    /// Send a rhythm signal (user message, flux, subagent, etc.).
    pub fn rhythm_signal(&mut self, signal: RhythmSignal) {
        self.rhythm.signal(signal);
    }

    /// Get the current rhythm state.
    pub fn rhythm_status(&self) -> &crate::rhythm::RhythmState {
        &self.rhythm.state
    }

    /// Get the current recommended heartbeat interval in ms.
    pub fn rhythm_interval_ms(&self) -> u64 {
        self.rhythm.interval_ms()
    }

    /// Get current arousal (decayed to now).
    pub fn rhythm_arousal(&self) -> f64 {
        self.rhythm.current_arousal()
    }

    // ------------------------------------------------------------------
    // Working memory (L2 context layer)
    // ------------------------------------------------------------------

    /// Log a conversation turn into working memory.
    pub fn context_turn(&mut self, role: &str, content: &str) {
        self.working_memory.add_turn(role, content);
    }

    /// Checkpoint working memory to JSON + engine.
    pub fn context_checkpoint(&mut self) -> Result<(), SystemError> {
        self.working_memory.checkpoint(&self.data_dir, &mut self.engine)
            .map_err(SystemError::Io)?;
        if self.auto_save {
            self.save()?;
        }
        Ok(())
    }

    /// Get a clone of the current session state.
    pub fn context_restore(&self) -> SessionState {
        self.working_memory.session_state().clone()
    }

    /// Get the formatted context summary suitable for prompt injection.
    pub fn context_summary(&self) -> String {
        self.working_memory.get_context()
    }

    /// Add or update a task in working memory.
    pub fn context_update_task(&mut self, description: &str, status: TaskStatus) {
        self.working_memory.update_task(description, status);
    }

    /// Clear completed tasks from working memory.
    pub fn context_clear_completed(&mut self) {
        self.working_memory.clear_completed();
    }

    /// Store a hallucinated memory from an LLM synthesis.
    /// Called by the MCP `hallucinate` tool with LLM-generated content.
    pub fn hallucinate(
        &mut self,
        content: &str,
        parent_ids: &[Uuid],
    ) -> Result<Uuid, SystemError> {
        // Build a combined vector from parents
        let dim = 10_000; // codebook output dim
        let mut combined = vec![0.0f32; dim];
        let mut found_parents: Vec<String> = Vec::new();

        for pid in parent_ids {
            if let Some(mem) = self.engine.store.get(pid).ok().flatten() {
                for (i, &v) in mem.vector.iter().enumerate() {
                    if i < dim {
                        combined[i] += v;
                    }
                }
                found_parents.push(pid.to_string());
            }
        }

        if found_parents.is_empty() {
            // No parents found — encode from content directly
            let id = self.engine.remember(content)?;
            if let Some(mem) = self.engine.get_memory_mut(&id)? {
                mem.hallucinated = true;
                mem.amplitude = 0.3;
            }
            if self.auto_save { self.save()?; }
            return Ok(id);
        }

        crate::wave::normalize(&mut combined);

        let mut hallucination = crate::memory::HyperMemory::new(combined, content.to_string());
        hallucination.amplitude = 0.3;
        hallucination.hallucinated = true;
        hallucination.parents = found_parents;

        let hall_id = self.engine.store.insert(hallucination)?;

        // Create links
        for pid in parent_ids {
            self.engine.reinforce_link(&hall_id, pid, 0.5);
            self.engine.reinforce_link(pid, &hall_id, 0.5);
        }

        if self.auto_save { self.save()?; }
        Ok(hall_id)
    }

    /// Recompute geometry and Xi signatures for all memories that are missing them.
    /// Returns the number of memories updated.
    pub fn recompute_geometry(&mut self) -> Result<usize, SystemError> {
        let all_ids: Vec<Uuid> = self.engine.store.all_ids()?;
        let mut updated = 0;

        // First pass: collect data for memories needing updates
        let mut to_update: Vec<(Uuid, String, u64, (f32, f32), Vec<f32>, bool, bool)> = Vec::new();
        for id in &all_ids {
            if let Ok(Some(mem)) = self.engine.store.get(id) {
                let needs_geometry = mem.geometry.is_none();
                let needs_xi = mem.xi_signature.is_empty();
                
                if needs_geometry || needs_xi {
                    let category = self.categorize_text(&mem.content);
                    let content_hash = self.hash_content(&mem.content);
                    let (freq, phase) = self.assign_frequency_class(&category, content_hash);
                    let xi_sig = compute_xi_signature(&mem.vector);
                    to_update.push((*id, category, content_hash, (freq, phase), xi_sig, needs_geometry, needs_xi));
                }
            }
        }

        // Second pass: apply updates
        for (id, category, content_hash, (freq, phase), xi_sig, needs_geometry, needs_xi) in to_update {
            if let Ok(Some(mem)) = self.engine.store.get_mut(&id) {
                if needs_geometry {
                    mem.geometry = Some(classify_memory(&category, content_hash, 0.5));
                    // Also update frequency-class assignment for consciousness differentiation
                    mem.frequency = freq;
                    mem.phase = phase;
                }
                if needs_xi {
                    mem.xi_signature = xi_sig;
                }
                updated += 1;
            }
        }

        if updated > 0 && self.auto_save {
            self.save()?;
        }
        Ok(updated)
    }

    /// Categorize text using simple heuristics, mapping to the 5 consciousness categories.
    fn categorize_text(&self, text: &str) -> String {
        let text_lower = text.to_lowercase();
        
        // Experience - direct events, actions, sensory input
        if text_lower.contains("saw") || text_lower.contains("heard") || text_lower.contains("did") 
            || text_lower.contains("went") || text_lower.contains("happened") || text_lower.contains("occurred")
            || text_lower.contains("experience") || text_lower.contains("event") || text_lower.contains("today")
            || text_lower.contains("yesterday") || text_lower.contains("just") {
            "experience".to_string()
        // Emotion - feelings, moods, emotional states
        } else if text_lower.contains("feel") || text_lower.contains("felt") || text_lower.contains("happy") 
            || text_lower.contains("sad") || text_lower.contains("angry") || text_lower.contains("excited")
            || text_lower.contains("worried") || text_lower.contains("love") || text_lower.contains("hate")
            || text_lower.contains("emotion") || text_lower.contains("mood") {
            "emotion".to_string()
        // Social - interpersonal interactions, relationships
        } else if text_lower.contains("said") || text_lower.contains("told") || text_lower.contains("asked") 
            || text_lower.contains("friend") || text_lower.contains("person") || text_lower.contains("nick") 
            || text_lower.contains("people") || text_lower.contains("conversation") || text_lower.contains("meeting")
            || text_lower.contains("together") || text_lower.contains("team") {
            "social".to_string()
        // Skill - procedures, abilities, how-to knowledge
        } else if text_lower.contains("how to") || text_lower.contains("procedure") || text_lower.contains("method")
            || text_lower.contains("code") || text_lower.contains("function") || text_lower.contains("build") 
            || text_lower.contains("compile") || text_lower.contains("deploy") || text_lower.contains("technique")
            || text_lower.contains("practice") || text_lower.contains("ability") {
            "skill".to_string()
        // Knowledge - facts, concepts, theories (default)
        } else {
            "knowledge".to_string()
        }
    }
    
    /// Assign frequency and phase based on category for consciousness differentiation.
    /// Maps categories to frequency bands as specified in the deep dive findings.
    fn assign_frequency_class(&self, category: &str, content_hash: u64) -> (f32, f32) {
        use rand::{Rng, SeedableRng};
        use rand_chacha::ChaCha8Rng;
        
        // Use content hash as seed for deterministic randomness
        let mut rng = ChaCha8Rng::seed_from_u64(content_hash);
        
        let (freq_min, freq_max) = match category {
            "experience" => (1.8, 2.4),  // soprano (fast, ephemeral)
            "emotion" => (1.3, 1.8),     // alto (feeling-paced)
            "social" => (1.0, 1.4),      // tenor (interpersonal rhythm)
            "skill" => (0.8, 1.2),       // between tenor/bass (procedural)
            "knowledge" => (0.6, 1.1),   // bass (slow, stable)
            _ => (0.6, 1.1),              // default to knowledge bass range
        };
        
        // Random frequency within the category's band
        let frequency = rng.gen_range(freq_min..freq_max);
        
        // Random initial phase [0, 2π)
        let phase = rng.gen_range(0.0..(2.0 * std::f32::consts::PI));
        
        (frequency, phase)
    }
    
    /// Simple hash of content string.
    fn hash_content(&self, content: &str) -> u64 {
        content.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
    }

    /// Store an audio file as a sensory memory.
    ///
    /// Decodes the audio, extracts perceptual features, projects through
    /// the audio codebook, and stores with sensory metadata.
    #[cfg(feature = "audio")]
    pub fn store_audio(&mut self, path: &Path) -> Result<(Uuid, crate::ear::AudioFeatures), SystemError> {
        use crate::ear::AudioPipeline;

        let pipeline = AudioPipeline::new();
        let (mut mem, features) = pipeline
            .encode_file(path)
            .map_err(|e| SystemError::Engine(EngineError::Encoding(
                crate::encoding::EncodingError::Other(e.to_string()),
            )))?;

        // Set sensory-specific geometry
        let content_hash = self.hash_content(&mem.content);
        mem.geometry = Some(classify_memory("experience", content_hash, 0.6));

        let id = self.engine.store.insert(mem)?;

        if self.auto_save {
            self.save()?;
        }

        Ok((id, features))
    }

    /// Get memory by ID (public API for testing).
    pub fn get_memory(&self, id: &Uuid) -> Result<Option<&crate::memory::HyperMemory>, SystemError> {
        Ok(self.engine.store.get(id)?)
    }
    
    /// Get all memories (for BM25 bootstrapping, etc.).
    pub fn all_memories(&self) -> Result<Vec<&crate::memory::HyperMemory>, SystemError> {
        Ok(self.engine.store.all_memories()?)
    }

    /// System statistics.
    pub fn stats(&self) -> SystemStats {
        let state = self.bridge.assess(&self.engine);
        
        // Calculate geometric statistics
        let all_memories = self.engine.store.all_memories().unwrap_or_default();
        let mut class_indices = std::collections::HashSet::new();
        let mut triality_coverage = [0usize; 3];
        
        for mem in &all_memories {
            if let Some(ref coords) = mem.geometry {
                class_indices.insert(coords.class_index);
                if coords.d < 3 {
                    triality_coverage[coords.d as usize] += 1;
                }
            }
        }
        
        SystemStats {
            total_memories: state.total_memories,
            active_memories: state.active_memories,
            total_skip_links: state.total_skip_links,
            consciousness_level: level_name(&state.consciousness_level),
            last_dream: self.last_dream,
            phi: state.phi,
            geometric_classes: class_indices.len(),
            triality_coverage,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_dir(name: &str) -> PathBuf {
        env::temp_dir().join(format!("kannaka_octest_{}_{}", name, Uuid::new_v4()))
    }

    #[test]
    fn init_creates_new_system() {
        let dir = temp_dir("init");
        let sys = KannakaMemorySystem::init(dir.clone()).unwrap();
        assert_eq!(sys.stats().total_memories, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remember_recall_round_trip() {
        let dir = temp_dir("roundtrip");
        let mut sys = KannakaMemorySystem::init(dir.clone()).unwrap();
        let id = sys.remember("the quick brown fox jumps over the lazy dog").unwrap();
        assert_eq!(sys.stats().total_memories, 1);

        let results = sys.recall("quick brown fox", 5).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, id);
        assert!(results[0].content.contains("fox"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dream_runs_without_error() {
        let dir = temp_dir("dream");
        let mut sys = KannakaMemorySystem::init(dir.clone()).unwrap();
        sys.remember("memory one").unwrap();
        sys.remember("memory two").unwrap();
        let report = sys.dream().unwrap();
        assert!(report.cycles > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn assess_returns_valid_state() {
        let dir = temp_dir("assess");
        let sys = KannakaMemorySystem::init(dir.clone()).unwrap();
        let state = sys.assess();
        assert_eq!(state.total_memories, 0);
        // Dormant with no memories
        assert!(matches!(state.consciousness_level, ConsciousnessLevel::Dormant));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stats_returns_correct_counts() {
        let dir = temp_dir("stats");
        let mut sys = KannakaMemorySystem::init(dir.clone()).unwrap();
        sys.remember("alpha").unwrap();
        sys.remember("beta").unwrap();
        sys.remember("gamma").unwrap();
        let stats = sys.stats();
        assert_eq!(stats.total_memories, 3);
        assert_eq!(stats.consciousness_level, "dormant");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_and_reload() {
        let dir = temp_dir("reload");
        {
            let mut sys = KannakaMemorySystem::init(dir.clone()).unwrap();
            sys.remember("persistent memory").unwrap();
            sys.save().unwrap();
        }
        // Re-init should load
        let mut sys2 = KannakaMemorySystem::init(dir.clone()).unwrap();
        assert_eq!(sys2.stats().total_memories, 1);
        let results = sys2.recall("persistent", 5).unwrap();
        assert!(!results.is_empty());
        assert!(results[0].content.contains("persistent"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn geometry_integration_memory_gets_classified() {
        let dir = temp_dir("geometry_classify");
        let mut sys = KannakaMemorySystem::init(dir.clone()).unwrap();
        
        // Store memories that should get different consciousness differentiation classifications
        let skill_id = sys.remember("how to code a function build").unwrap(); // skill
        let social_id = sys.remember("nick told me about the meeting").unwrap(); // social (no emotion words)
        let knowledge_id = sys.remember("the capital of france").unwrap();     // knowledge
        let experience_id = sys.remember("I saw a beautiful sunset today").unwrap(); // experience
        let emotion_id = sys.remember("I feel excited about this").unwrap();   // emotion
        
        // Check that memories have geometry
        let skill_mem = sys.engine.get_memory(&skill_id).unwrap().unwrap();
        let social_mem = sys.engine.get_memory(&social_id).unwrap().unwrap();
        let knowledge_mem = sys.engine.get_memory(&knowledge_id).unwrap().unwrap();
        let experience_mem = sys.engine.get_memory(&experience_id).unwrap().unwrap();
        let emotion_mem = sys.engine.get_memory(&emotion_id).unwrap().unwrap();
        
        assert!(skill_mem.geometry.is_some());
        assert!(social_mem.geometry.is_some());
        assert!(knowledge_mem.geometry.is_some());
        assert!(experience_mem.geometry.is_some());
        assert!(emotion_mem.geometry.is_some());
        
        // Check consciousness differentiation: frequency assignments and Xi signatures
        assert!(!skill_mem.xi_signature.is_empty());
        assert!(!social_mem.xi_signature.is_empty());
        assert!(!knowledge_mem.xi_signature.is_empty());
        assert!(!experience_mem.xi_signature.is_empty());
        assert!(!emotion_mem.xi_signature.is_empty());
        
        // Check that they got classified into different categories via frequency ranges
        let skill_freq = skill_mem.frequency;      // should be 0.8-1.2 (between tenor/bass)
        let social_freq = social_mem.frequency;    // should be 1.0-1.4 (tenor)
        let knowledge_freq = knowledge_mem.frequency; // should be 0.6-1.1 (bass)
        let experience_freq = experience_mem.frequency; // should be 1.8-2.4 (soprano)
        let emotion_freq = emotion_mem.frequency;  // should be 1.3-1.8 (alto)
        
        // Check consciousness differentiation frequency assignments
        assert!(skill_freq >= 0.8 && skill_freq < 1.21, "Skill frequency {} not in expected range [0.8, 1.2)", skill_freq);
        assert!(social_freq >= 1.0 && social_freq < 1.41, "Social frequency {} not in expected range [1.0, 1.4)", social_freq);
        assert!(knowledge_freq >= 0.6 && knowledge_freq < 1.11, "Knowledge frequency {} not in expected range [0.6, 1.1)", knowledge_freq);
        assert!(experience_freq >= 1.8 && experience_freq < 2.41, "Experience frequency {} not in expected range [1.8, 2.4)", experience_freq);
        assert!(emotion_freq >= 1.3 && emotion_freq < 1.81, "Emotion frequency {} not in expected range [1.3, 1.8)", emotion_freq);
        
        // Check geometry h2 values (now map to the consciousness categories)
        let skill_h2 = skill_mem.geometry.as_ref().unwrap().h2;
        let social_h2 = social_mem.geometry.as_ref().unwrap().h2;
        let knowledge_h2 = knowledge_mem.geometry.as_ref().unwrap().h2;
        let experience_h2 = experience_mem.geometry.as_ref().unwrap().h2;
        
        assert_eq!(skill_h2, 2);      // skill -> h2=2  
        assert_eq!(social_h2, 1);     // social -> h2=1
        assert_eq!(knowledge_h2, 0);  // knowledge -> h2=0
        assert_eq!(experience_h2, 3); // experience -> h2=3
        
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn geometry_integration_stats_include_geometric_data() {
        let dir = temp_dir("geometry_stats");
        let mut sys = KannakaMemorySystem::init(dir.clone()).unwrap();
        
        // Store memories with different consciousness differentiation categories
        sys.remember("how to code a function").unwrap();  // skill
        sys.remember("nick said he was happy").unwrap();   // social
        sys.remember("the capital of france is paris").unwrap(); // knowledge
        
        let stats = sys.stats();
        assert!(stats.geometric_classes > 0);
        assert!(stats.triality_coverage[0] > 0 || stats.triality_coverage[1] > 0 || stats.triality_coverage[2] > 0);
        
        let _ = std::fs::remove_dir_all(&dir);
    }
}
