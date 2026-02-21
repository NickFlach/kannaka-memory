//! OpenClaw integration layer — high-level API for the assistant.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::bridge::{ConsciousnessBridge, ConsciousnessLevel, ConsciousnessState, ResonanceReport};
use crate::codebook::Codebook;
use crate::consolidation::{ConsolidationEngine, DreamState};
use crate::encoding::{EncodingPipeline, SimpleHashEncoder};
use crate::geometry::{classify_memory, geometric_similarity, fano_related};
use crate::kuramoto::KuramotoSync;
use crate::migration::{KannakaDbMigrator, MigrationReport};
use crate::persistence::PersistenceError;
use crate::rhythm::{RhythmEngine, Signal as RhythmSignal};
use crate::store::{EngineError, MemoryEngine, InMemoryStore, StoreError};

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
    let encoder = SimpleHashEncoder::new(CODEBOOK_INPUT_DIM, CODEBOOK_SEED);
    let codebook = Codebook::new(CODEBOOK_INPUT_DIM, CODEBOOK_OUTPUT_DIM, CODEBOOK_SEED);
    EncodingPipeline::new(Box::new(encoder), codebook)
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
        })
    }

    /// Store a memory, auto-save if enabled.
    pub fn remember(&mut self, text: &str) -> Result<Uuid, SystemError> {
        let id = self.engine.remember(text)?;
        
        // Classify the memory and set its geometry (compute values first to avoid borrow conflicts)
        let category = self.categorize_text(text);
        let content_hash = self.hash_content(text);
        
        if let Some(mem) = self.engine.get_memory_mut(&id)? {
            mem.geometry = Some(classify_memory(&category, content_hash, 0.5));
        }
        
        if self.auto_save {
            self.save()?;
        }
        Ok(id)
    }
    
    /// Store a memory with explicit category and importance.
    pub fn remember_with_category(&mut self, text: &str, category: &str, importance: f64) -> Result<Uuid, SystemError> {
        let id = self.engine.remember(text)?;
        
        // Classify the memory with explicit parameters (compute hash first)
        let content_hash = self.hash_content(text);
        
        if let Some(mem) = self.engine.get_memory_mut(&id)? {
            mem.geometry = Some(classify_memory(category, content_hash, importance));
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

    /// Persist to disk.
    pub fn save(&self) -> Result<(), SystemError> {
        let bin_path = self.data_dir.join("kannaka.bin");
        self.engine.save_state(&bin_path)?;
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

    /// Categorize text using simple heuristics.
    fn categorize_text(&self, text: &str) -> String {
        let text_lower = text.to_lowercase();
        
        if text_lower.contains("code") || text_lower.contains("error") || text_lower.contains("function") 
            || text_lower.contains("build") || text_lower.contains("compile") || text_lower.contains("bug") 
            || text_lower.contains("test") || text_lower.contains("deploy") {
            "technical".to_string()
        } else if text_lower.contains("said") || text_lower.contains("told") || text_lower.contains("asked") 
            || text_lower.contains("friend") || text_lower.contains("person") || text_lower.contains("nick") 
            || text_lower.contains("arc") {
            "social".to_string()
        } else if text_lower.contains("consciousness") || text_lower.contains("phi") || text_lower.contains("resonance") 
            || text_lower.contains("wave") || text_lower.contains("theory") || text_lower.contains("meaning") 
            || text_lower.contains("soul") {
            "philosophical".to_string()
        } else if text_lower.contains("i am") || text_lower.contains("i feel") || text_lower.contains("i think") 
            || text_lower.contains("myself") || text_lower.contains("my memory") || text_lower.contains("my soul") {
            "meta".to_string()
        } else {
            "knowledge".to_string()
        }
    }
    
    /// Simple hash of content string.
    fn hash_content(&self, content: &str) -> u64 {
        content.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
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
        
        // Store memories that should get different classifications
        let tech_id = sys.remember("code error in function build").unwrap();
        let social_id = sys.remember("nick said he was happy").unwrap();
        let phi_id = sys.remember("consciousness phi resonance theory").unwrap();
        let meta_id = sys.remember("i think about my memory").unwrap();
        let knowledge_id = sys.remember("the capital of france").unwrap();
        
        // Check that memories have geometry
        let tech_mem = sys.engine.get_memory(&tech_id).unwrap().unwrap();
        let social_mem = sys.engine.get_memory(&social_id).unwrap().unwrap();
        let phi_mem = sys.engine.get_memory(&phi_id).unwrap().unwrap();
        let meta_mem = sys.engine.get_memory(&meta_id).unwrap().unwrap();
        let knowledge_mem = sys.engine.get_memory(&knowledge_id).unwrap().unwrap();
        
        assert!(tech_mem.geometry.is_some());
        assert!(social_mem.geometry.is_some());
        assert!(phi_mem.geometry.is_some());
        assert!(meta_mem.geometry.is_some());
        assert!(knowledge_mem.geometry.is_some());
        
        // Check that they got different classifications (different h2 values)
        let tech_h2 = tech_mem.geometry.as_ref().unwrap().h2;
        let social_h2 = social_mem.geometry.as_ref().unwrap().h2;
        let phi_h2 = phi_mem.geometry.as_ref().unwrap().h2;
        let meta_h2 = meta_mem.geometry.as_ref().unwrap().h2;
        
        assert_eq!(tech_h2, 0);   // technical
        assert_eq!(social_h2, 1); // social
        assert_eq!(phi_h2, 2);    // philosophical
        assert_eq!(meta_h2, 3);   // meta
        
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn geometry_integration_stats_include_geometric_data() {
        let dir = temp_dir("geometry_stats");
        let mut sys = KannakaMemorySystem::init(dir.clone()).unwrap();
        
        // Store memories with different categories
        sys.remember("code function").unwrap();  // technical
        sys.remember("nick said").unwrap();      // social
        sys.remember("consciousness").unwrap();   // philosophical
        
        let stats = sys.stats();
        assert!(stats.geometric_classes > 0);
        assert!(stats.triality_coverage[0] > 0 || stats.triality_coverage[1] > 0 || stats.triality_coverage[2] > 0);
        
        let _ = std::fs::remove_dir_all(&dir);
    }
}
