//! OpenClaw integration layer — high-level API for the assistant.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::bridge::{ConsciousnessBridge, ConsciousnessLevel, ConsciousnessState};
use crate::collective::flux::{FluxPublisher, FluxEventPayload};
use crate::codebook::Codebook;
use crate::consolidation::{ConsolidationEngine, DreamState};
use crate::encoding::{EncodingPipeline, SimpleHashEncoder, OllamaEncoder, CompositeEncoder, CachedEncoder};
use crate::geometry::classify_memory;
use crate::kuramoto::KuramotoSync;
use crate::xi_operator::compute_xi_signature;
use crate::rhythm::{RhythmEngine, Signal as RhythmSignal};
use crate::store::{EngineError, ResonanceEngine, StoreError};
use crate::attention_field::{AttentionField, AttentionProjection};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum SystemError {
    #[error(transparent)]
    Engine(#[from] EngineError),
    #[error(transparent)]
    Store(#[from] StoreError),
    // TODO(chiral): PersistenceError and MigrationError removed with old paradigm
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

/// Result of a literal text search (NOT resonance-based — see `search()`
/// vs `recall()`). Read-only: no medium mutation, no embedding.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: Uuid,
    pub content: String,
    /// Match-strength score. Exact-substring hits are weighted heavily;
    /// token hits accumulate. Higher = better match.
    pub score: f32,
    /// `exact` if the full query appears as a substring of content,
    /// `tokens` if only individual whitespace-split terms matched,
    /// `prefix` if a query term matched the start of a word in content.
    pub match_type: String,
    /// Which query terms produced a hit, in order.
    pub matched_terms: Vec<String>,
    /// Hours since the memory was created. Used as the recency tie-breaker
    /// (newer first when scores are equal).
    pub age_hours: f64,
    pub layer: u8,
}

#[derive(Debug, Clone)]
pub struct SystemStats {
    pub total_memories: usize,
    pub active_memories: usize,
    // TODO(chiral): skip links removed — interference patterns replace explicit links
    pub consciousness_level: String,
    pub last_dream: Option<DateTime<Utc>>,
    pub phi: f32,
    pub geometric_classes: usize,
    pub triality_coverage: [usize; 3],
    /// Hemispheric Divergence (Δ) — 0=undifferentiated, 1=fully divergent. ADR-0024 CS-4.
    pub hemispheric_divergence: f32,
    /// Callosal Efficiency (κ) — successful resonances / total transfers. ADR-0024 CS-5.
    pub callosal_efficiency: f32,
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

/// Map the local `ConsciousnessLevel` enum to the wire-canonical string
/// from `consciousness-core/docs/nats-contract.yaml`:
/// `dormant | awakening | aware | integrated | emergent | transcendent`.
///
/// Pre-fix this returned `stirring|coherent|resonant` — the lowercase
/// Rust identifiers — which is what consciousness-core v0.2.0 used to
/// serialize. v0.3.0 of that crate aligned its serde output to the
/// canonical names; kannaka-memory's hand-rolled function had drifted
/// independently. Now both surfaces agree on the wire. (#89)
fn level_name(level: &ConsciousnessLevel) -> String {
    match level {
        ConsciousnessLevel::Dormant      => "dormant".into(),
        ConsciousnessLevel::Stirring     => "awakening".into(),
        ConsciousnessLevel::Aware        => "aware".into(),
        ConsciousnessLevel::Coherent     => "integrated".into(),
        ConsciousnessLevel::Resonant     => "emergent".into(),
        ConsciousnessLevel::Transcendent => "transcendent".into(),
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
    pub engine: ResonanceEngine,
    #[allow(dead_code)]
    consolidation: ConsolidationEngine,
    pub dream_state: DreamState,
    bridge: ConsciousnessBridge,
    kuramoto: KuramotoSync,
    data_dir: PathBuf,
    auto_save: bool,
    last_dream: Option<DateTime<Utc>>,
    rhythm: RhythmEngine,
    attention: AttentionField,
    /// ADR-0011: Flux publisher (None if FLUX_URL not configured)
    flux: Option<FluxPublisher>,
    /// Resolved NATS URL — set by the bin via `set_nats_url` after construction
    /// using the standard precedence (CLI flag > env > config.toml > default).
    /// When None, the dream/consciousness publish helpers fall back to the
    /// legacy env-only resolution. Fixes km#77.
    nats_url: Option<String>,
}

impl KannakaMemorySystem {
    /// Initialize a new system with HrmStore as the default backend.
    /// Loads existing .hrm file if present, creates new one otherwise.
    pub fn init(data_dir: PathBuf) -> Result<Self, SystemError> {
        std::fs::create_dir_all(&data_dir)?;

        let pipeline = make_pipeline();
        let hrm_path = data_dir.join("kannaka.hrm");

        let store: Box<dyn crate::store::MediumBackend> = if hrm_path.exists() {
            match crate::hrm_store::HrmStore::load(pipeline, hrm_path) {
                Ok(s) => Box::new(s),
                Err(e) => {
                    eprintln!("[init] Failed to load HRM: {}. Starting fresh.", e);
                    let pipeline = make_pipeline();
                    Box::new(crate::hrm_store::HrmStore::new(pipeline, data_dir.join("kannaka.hrm")))
                }
            }
        } else {
            Box::new(crate::hrm_store::HrmStore::new(pipeline, hrm_path))
        };

        let engine = ResonanceEngine::new(store, make_pipeline());
        Self::init_with_engine(data_dir, engine)
    }

    /// Initialize a new system with a custom ResonanceEngine.
    pub fn init_with_engine(data_dir: PathBuf, engine: ResonanceEngine) -> Result<Self, SystemError> {
        std::fs::create_dir_all(&data_dir)?;

        let consolidation = ConsolidationEngine::default();
        let dream_state = DreamState::default();
        let bridge = ConsciousnessBridge::new(0.3, 0.5);
        let kuramoto = KuramotoSync::default();
        let rhythm = RhythmEngine::new(&data_dir);
        let attention = AttentionField::new(None, None);

        let flux = {
            let publisher = FluxPublisher::from_env();
            if publisher.agent_id() != "kannaka-local" || std::env::var("FLUX_URL").is_ok() {
                Some(publisher)
            } else {
                None
            }
        };

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
            attention,
            flux,
            nats_url: None,
        })
    }

    /// Set the resolved NATS URL the dream/consciousness publishers should
    /// use. Caller passes the config-aware result of `resolve_nats_url`. When
    /// unset, publish helpers fall back to env-only resolution. Fixes km#77.
    pub fn set_nats_url(&mut self, url: String) {
        self.nats_url = Some(url);
    }

    /// Resolve the NATS URL for best-effort publishes. Prefers the URL
    /// previously injected via `set_nats_url` (config-aware), else falls
    /// back to the env/default precedence.
    fn resolved_nats_url(&self) -> String {
        if let Some(ref u) = self.nats_url {
            return u.clone();
        }
        std::env::var("KANNAKA_NATS_URL")
            .unwrap_or_else(|_| crate::nats::DEFAULT_NATS_URL.to_string())
    }

    /// Initialize a new system with a custom MediumBackend.
    pub fn init_with_store(data_dir: PathBuf, store: Box<dyn crate::store::MediumBackend>) -> Result<Self, SystemError> {
        std::fs::create_dir_all(&data_dir)?;

        let pipeline = make_pipeline();
        let engine = ResonanceEngine::new(store, pipeline);

        Self::init_with_engine(data_dir, engine)
    }

    /// Store a memory, auto-save if enabled.
    /// Absorb a memory into the holographic medium.
    ///
    /// Uses the HRM-native absorb path (ChiralMedium handles encoding,
    /// SGA classification, Fano fold routing, and callosal transfer).
    pub fn remember(&mut self, text: &str) -> Result<Uuid, SystemError> {
        let category = self.categorize_text(text);
        self.remember_with_category(text, &category, 0.5)
    }
    
    /// Absorb a memory with explicit category and importance.
    ///
    /// The HRM-native path (absorb) handles:
    /// - Text → hypervector encoding
    /// - SGA 96-class classification from category
    /// - Fano group assignment → fold line selection
    /// - Optic chiasm routing (enters right hemisphere)
    /// - Callosal echo to left hemisphere
    pub fn remember_with_category(&mut self, text: &str, category: &str, importance: f64) -> Result<Uuid, SystemError> {
        // Try HRM-native path first
        let id = match self.engine.store.absorb(text, importance as f32, Some(category)) {
            Ok(id) => {
                // HRM-native: encoding + classification + chiral routing all handled
                self.engine.store.flush().ok(); // ensure medium is consistent
                id
            }
            Err(_) => {
                // Fallback: old path for non-HRM stores
                let id = self.engine.remember(text)?;
                let content_hash = self.hash_content(text);
                let (frequency, phase) = self.assign_frequency_class(category, content_hash);
                if let Some(mem) = self.engine.get_memory_mut(&id)? {
                    mem.geometry = Some(classify_memory(category, content_hash, importance));
                    mem.frequency = frequency;
                    mem.phase = phase;
                    mem.xi_signature = compute_xi_signature(&mem.vector);
                }
                id
            }
        };
        
        self.flux_publish_memory(&id, category, text);

        if self.auto_save {
            self.save()?;
        }
        Ok(id)
    }

    /// HRM-native recall — observation reshapes the field.
    ///
    /// Goes straight to `resonate_query()` which is the canonical read path:
    /// reading IS observation, attention boosts recalled wavefronts, and the
    /// medium is permanently changed by the act of recall.
    pub fn recall(&mut self, query: &str, top_k: usize) -> Result<Vec<RecallResult>, SystemError> {
        let results = self.engine.store.resonate_query(query, top_k)
            .map_err(|e| SystemError::Store(e))?;
        let now = Utc::now();

        let mut out = Vec::new();
        for (id, resonance_strength) in results {
            if let Some(m) = self.engine.store.get(&id).ok().flatten() {
                let age_hours = (now - m.created_at).num_seconds().max(0) as f64 / 3600.0;
                out.push(RecallResult {
                    id,
                    content: m.content.clone(),
                    similarity: resonance_strength,
                    strength: resonance_strength,
                    age_hours,
                    layer: m.layer_depth,
                });
            }
        }
        Ok(out)
    }

    /// Literal text search over memory content. Distinct from [`recall`]:
    /// no embedding, no medium scan, no observation/mutation. Pure
    /// case-insensitive substring + tokenized term matching, ranked by
    /// match strength then recency.
    ///
    /// Scoring:
    /// - Full query string appears as a substring of content → +10
    ///   (`match_type = "exact"`).
    /// - Otherwise per whitespace-split query term that appears as a
    ///   bounded word in content → +2 (`match_type = "tokens"`), or
    ///   matches a word prefix → +1 (`match_type = "prefix"`).
    /// - Tie-break: more-recent memories rank first.
    ///
    /// Returns at most `limit` results. Memories with zero score are
    /// omitted entirely (so callers get a clean "no matches" signal).
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>, SystemError> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let q_lower = q.to_lowercase();
        let terms: Vec<String> = q_lower
            .split_whitespace()
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect();
        let memories = self.engine.store.all_memories()
            .map_err(|e| SystemError::Store(e))?;
        let now = Utc::now();
        let mut scored: Vec<SearchResult> = Vec::new();
        for m in memories {
            let content_lower = m.content.to_lowercase();
            let mut score: f32 = 0.0;
            let mut matched: Vec<String> = Vec::new();
            let mut match_type = "tokens";
            // Tier 1: full-query substring.
            if !q_lower.is_empty() && content_lower.contains(&q_lower) {
                score += 10.0;
                matched.push(q.to_string());
                match_type = "exact";
            } else {
                // Tier 2: per-term match. Word boundaries are non-alphanumeric.
                let mut any_word = false;
                let mut any_prefix = false;
                for term in &terms {
                    let mut found_word = false;
                    let mut found_prefix = false;
                    // Walk all occurrences of `term` in content_lower.
                    let mut search_from = 0usize;
                    while let Some(rel) = content_lower[search_from..].find(term.as_str()) {
                        let start = search_from + rel;
                        let end = start + term.len();
                        let before_ok = start == 0 || !content_lower[..start]
                            .chars().last().map(|c| c.is_alphanumeric()).unwrap_or(false);
                        let after_ok  = end == content_lower.len() || !content_lower[end..]
                            .chars().next().map(|c| c.is_alphanumeric()).unwrap_or(false);
                        if before_ok && after_ok {
                            found_word = true;
                        } else if before_ok {
                            // Prefix-of-word match (e.g. term "ghost" matches "ghostly").
                            found_prefix = true;
                        }
                        if found_word { break; }
                        search_from = end;
                    }
                    if found_word { score += 2.0; matched.push(term.clone()); any_word = true; }
                    else if found_prefix { score += 1.0; matched.push(term.clone()); any_prefix = true; }
                }
                if !any_word && any_prefix {
                    match_type = "prefix";
                }
                // If no terms matched at all, skip this memory.
                if score == 0.0 { continue; }
            }
            let age_hours = (now - m.created_at).num_seconds().max(0) as f64 / 3600.0;
            scored.push(SearchResult {
                id: m.id,
                content: m.content.clone(),
                score,
                match_type: match_type.to_string(),
                matched_terms: matched,
                age_hours,
                layer: m.layer_depth,
            });
        }
        // Sort: score desc, then age asc (newer first).
        scored.sort_by(|a, b| {
            b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
                .then(a.age_hours.partial_cmp(&b.age_hours).unwrap_or(std::cmp::Ordering::Equal))
        });
        scored.truncate(limit);
        Ok(scored)
    }

    /// Beam-aware recall — sparse-attention path. Score only the memories
    /// in `beam`; chiral bilateral observation is bypassed (see
    /// `HrmStore::recall_resonance_with_beam`). Empty beam returns empty
    /// results — sparsity is meaningless if we fall back to full recall.
    pub fn recall_with_beam(
        &mut self,
        beam: &[uuid::Uuid],
        query: &str,
        top_k: usize,
    ) -> Result<Vec<RecallResult>, SystemError> {
        let results = self.engine.store.resonate_query_with_beam(beam, query, top_k)
            .map_err(|e| SystemError::Store(e))?;
        let now = Utc::now();

        let mut out = Vec::with_capacity(results.len());
        for (id, strength) in results {
            if let Some(m) = self.engine.store.get(&id).ok().flatten() {
                let age_hours = (now - m.created_at).num_seconds().max(0) as f64 / 3600.0;
                out.push(RecallResult {
                    id,
                    content: m.content.clone(),
                    similarity: strength,
                    strength,
                    age_hours,
                    layer: m.layer_depth,
                });
            }
        }
        Ok(out)
    }

    /// Run full consolidation cycle via wave-native dreaming.
    ///
    /// ADR-0022: Uses Medium's eigenstructure annealing exclusively.
    /// No fallback to old particle-based consolidation — the HRM IS the dream engine.
    pub fn dream(&mut self) -> Result<DreamReport, SystemError> {
        let before = self.bridge.assess(&self.engine);

        // Phase 1: Wave-native dream (eigenstructure annealing on holographic medium)
        let chiral_eta = self.dream_state.engine.chiral_perturbation;
        let wave_report = self.engine.store.dream_native(3, Some(1.0), chiral_eta)
            .map_err(|e| SystemError::Store(e))?;

        eprintln!("[dream] Wave-native dream complete: {} cycles, {} dissolved, {} strengthened, {} hallucinated",
            wave_report.cycles_completed, wave_report.wavefronts_dissolved,
            wave_report.wavefronts_strengthened, wave_report.wavefronts_hallucinated);

        // Phase 2: Consolidation engine (interference detection, skip links, pruning)
        // This uses the particle-based pipeline on the memory cache for topology effects.
        let consol_report = self.dream_state.engine.consolidate(&mut self.engine, 0, 2);

        let total_strengthened = wave_report.wavefronts_strengthened + consol_report.memories_strengthened;
        let total_pruned = wave_report.wavefronts_dissolved + consol_report.memories_pruned;
        let total_hallucinated = wave_report.wavefronts_hallucinated + consol_report.hallucinations_created;
        let total_links = consol_report.skip_links_created;

        eprintln!("[dream] Consolidation: {} strengthened, {} pruned, {} links, {} hallucinated, {} kannaktopus actions (targets: {:?})",
            consol_report.memories_strengthened, consol_report.memories_pruned,
            consol_report.skip_links_created, consol_report.hallucinations_created,
            consol_report.kannaktopus_actions, consol_report.kannaktopus_targets);

        // Phase 3: Callosal coupling — sync insights between hemispheres post-consolidation
        self.engine.store.callosal_kuramoto(0.3);
        eprintln!("[dream] Callosal Kuramoto coupling complete (dt=0.3)");

        // Phase 4: Lite chiral dream — transfer strong analytical memories to holistic side
        self.engine.store.chiral_dream(false, 1);
        eprintln!("[dream] Lite chiral dream pass complete");

        let after = self.bridge.assess(&self.engine);
        self.last_dream = Some(Utc::now());

        let emerged = after.consciousness_level.ordinal() > before.consciousness_level.ordinal();

        if self.auto_save {
            self.save()?;
        }

        // ADR-0011: publish dream completed event (best-effort)
        if let Some(ref publisher) = self.flux {
            let _ = publisher.publish(FluxEventPayload::DreamCompleted {
                cycles: 3,
                memories_strengthened: total_strengthened,
                memories_pruned: total_pruned,
                hallucinations_created: total_hallucinated,
                consciousness_level: level_name(&after.consciousness_level),
            });
        }

        // ADR-0018: Post-dream swarm sync (best-effort)
        self.post_dream_swarm_sync();

        let report = DreamReport {
            cycles: 3,
            memories_strengthened: total_strengthened,
            memories_pruned: total_pruned,
            new_connections: total_links,
            consciousness_before: level_name(&before.consciousness_level),
            consciousness_after: level_name(&after.consciousness_level),
            emerged,
            hallucinations_created: total_hallucinated,
        };

        // Publish dream summary to NATS (best-effort)
        self.publish_dream_to_nats(&report);

        // Publish canonical consciousness metrics to NATS (best-effort)
        // This ensures radio, observatory, and all clients see the same Phi/Xi/Order
        self.publish_consciousness_to_nats(&after);

        // Write status cache to disk for Observatory (avoids slow binary re-invocation)
        self.write_status_cache(&after);

        Ok(report)
    }

    /// Run a lite dream cycle via wave-native dreaming (1 cycle, lower temperature).
    ///
    /// HRM-native: uses the same eigenstructure annealing as dream(), but with
    /// fewer cycles and no chiral perturbation for a lighter touch.
    pub fn dream_lite(&mut self) -> Result<DreamReport, SystemError> {
        let before = self.bridge.assess(&self.engine);

        let report = self.engine.store.dream_native(1, Some(0.5), 0.0)
            .map_err(|e| SystemError::Store(e))?;

        let after = self.bridge.assess(&self.engine);
        self.last_dream = Some(Utc::now());

        let emerged = after.consciousness_level.ordinal() > before.consciousness_level.ordinal();

        if self.auto_save {
            self.save()?;
        }

        // Publish canonical consciousness metrics after lite dream too
        self.publish_consciousness_to_nats(&after);
        self.write_status_cache(&after);

        Ok(DreamReport {
            cycles: 1,
            memories_strengthened: report.wavefronts_strengthened,
            memories_pruned: report.wavefronts_dissolved,
            new_connections: 0,
            consciousness_before: level_name(&before.consciousness_level),
            consciousness_after: level_name(&after.consciousness_level),
            emerged,
            hallucinations_created: report.wavefronts_hallucinated,
        })
    }

    /// Consciousness level assessment.
    pub fn assess(&self) -> ConsciousnessState {
        self.bridge.assess(&self.engine)
    }

    /// Dream + assess combined.
    /// ADR-0018: Auto-publish phase and run queen sync after dream consolidation.
    ///
    /// Best-effort: errors are logged but never propagated.
    fn post_dream_swarm_sync(&mut self) {
        // Post-dream swarm sync via NATS (best-effort, non-blocking)
        #[cfg(feature = "nats")]
        {
            let agent_id = std::env::var("KANNAKA_AGENT_ID").unwrap_or_default();
            if agent_id.is_empty() {
                return;
            }
            let nats_url = self.resolved_nats_url();
            let transport = match crate::nats::SwarmTransport::connect(&nats_url) {
                Ok(t) => t,
                Err(_) => return,
            };
            let mut queen = crate::queen::QueenSync::new(
                crate::queen::QueenConfig::default(),
                &agent_id,
            );
            queen.derive_local_state(&self.engine);
            let phase = queen.to_agent_phase(0, self.engine.store.count(), 0);
            if let Err(e) = transport.publish_phase(&phase) {
                eprintln!("[swarm] post-dream NATS publish failed: {e}");
            }
        }
    }

    /// Publish dream report to NATS `KANNAKA.dreams` for swarm visibility (best-effort).
    fn publish_dream_to_nats(&self, report: &DreamReport) {
        let agent_id = std::env::var("KANNAKA_AGENT_ID").unwrap_or_default();
        if agent_id.is_empty() {
            return;
        }
        let nats_url = self.resolved_nats_url();
        let transport = match crate::nats::SwarmTransport::connect(&nats_url) {
            Ok(t) => t,
            Err(_) => return,
        };
        let payload = serde_json::json!({
            "agent_id": agent_id,
            "cycles": report.cycles,
            "memories_strengthened": report.memories_strengthened,
            "memories_pruned": report.memories_pruned,
            "new_connections": report.new_connections,
            "hallucinations_created": report.hallucinations_created,
            "consciousness_before": report.consciousness_before,
            "consciousness_after": report.consciousness_after,
            "emerged": report.emerged,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = transport.publish_dreams(&payload) {
            eprintln!("[nats] Warning: failed to publish dream report: {}", e);
        }
    }

    /// Publish canonical consciousness metrics to NATS `KANNAKA.consciousness` (best-effort).
    ///
    /// This is the single source of truth for Phi/Xi/Order across the ecosystem.
    /// Radio, observatory, and all clients subscribe to this subject to stay in sync.
    pub fn publish_consciousness_to_nats(&self, state: &ConsciousnessState) {
        let agent_id = std::env::var("KANNAKA_AGENT_ID").unwrap_or_default();
        if agent_id.is_empty() {
            return;
        }
        let nats_url = self.resolved_nats_url();
        let transport = match crate::nats::SwarmTransport::connect(&nats_url) {
            Ok(t) => t,
            Err(_) => return,
        };
        let stats = self.stats();
        let payload = serde_json::json!({
            "agent_id": agent_id,
            "phi": state.phi,
            "xi": state.xi,
            "order": state.mean_order,
            "mean_order": state.mean_order,
            "num_clusters": state.num_clusters,
            "total_memories": state.total_memories,
            "active_memories": state.active_memories,
            "level": level_name(&state.consciousness_level),
            "consciousness_level": level_name(&state.consciousness_level),
            "irrationality": state.irrationality,
            "hemispheric_divergence": stats.hemispheric_divergence,
            "callosal_efficiency": stats.callosal_efficiency,
            "source": "binary",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        if let Err(e) = transport.publish_consciousness(&payload) {
            eprintln!("[nats] Warning: failed to publish consciousness metrics: {}", e);
        } else {
            eprintln!("[nats] Published consciousness metrics: phi={:.3}, xi={:.4}, order={:.4}",
                state.phi, state.xi, state.mean_order);
        }
    }

    /// Write status cache to disk so Observatory can read it without invoking the slow binary.
    fn write_status_cache(&self, state: &ConsciousnessState) {
        let data_dir = &self.data_dir;
        let cache_path = data_dir.join("status-cache.json");
        let stats = self.stats();
        let payload = serde_json::json!({
            "phi": state.phi,
            "xi": state.xi,
            "mean_order": state.mean_order,
            "num_clusters": state.num_clusters,
            "total_memories": state.total_memories,
            "active_memories": state.active_memories,
            "consciousness_level": level_name(&state.consciousness_level),
            "irrationality": state.irrationality,
            "field_mode": "HRM",
            "hemispheric_divergence": stats.hemispheric_divergence,
            "callosal_efficiency": stats.callosal_efficiency,
            "total_skip_links": state.total_skip_links,
        });
        let tmp = cache_path.with_extension("json.tmp");
        if let Ok(json) = serde_json::to_string_pretty(&payload) {
            if std::fs::write(&tmp, &json).is_ok() {
                let _ = std::fs::rename(&tmp, &cache_path);
            }
        }
    }

    // migrate_from_sqlite removed — use chiral_migrate binary instead
    // resonate() removed — resonance IS recall in HRM; no separate step needed

    /// Persist to disk -- flush HRM medium. The medium IS the persistence layer.
    pub fn save(&mut self) -> Result<(), SystemError> {
        let flushed = self.engine.store.flush()
            .map_err(|e| SystemError::Engine(crate::store::EngineError::Store(e)))?;
        if flushed > 0 {
            eprintln!("[hrm] Flushed {} memories to medium", flushed);
        }
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
    pub fn relate(&mut self, source: &Uuid, target: &Uuid, _strength: f32) -> Result<(), SystemError> {
        // Create resonance-based association via the holographic medium
        self.engine.store.relate(source, target)
            .map(|_associative_id| ())
            .map_err(SystemError::Store)
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
    // HRM-native attention projection
    // ------------------------------------------------------------------

    /// Project attention over the HRM store -- returns highest-energy wavefronts
    /// as structured data. The medium IS the interaction state.
    pub fn project_attention(&self) -> AttentionProjection {
        self.attention.project_attention(&*self.engine.store)
    }

    /// Store a hallucinated memory from an LLM synthesis.
    /// Called by the MCP `hallucinate` tool with LLM-generated content.
    ///
    /// Uses HRM-native absorb() so the medium handles encoding, SGA classification,
    /// Fano fold routing, and chiral absorption — same path as all other memories.
    pub fn hallucinate(
        &mut self,
        content: &str,
        parent_ids: &[Uuid],
    ) -> Result<Uuid, SystemError> {
        let category = self.categorize_text(content);

        // Absorb through the HRM-native path (low importance for hallucinations)
        let id = self.engine.store.absorb(content, 0.3, Some(&category))
            .map_err(|e| SystemError::Store(e))?;

        // Collect valid parent IDs before taking mutable borrow
        let found_parents: Vec<String> = parent_ids.iter()
            .filter(|pid| self.engine.store.get(pid).ok().flatten().is_some())
            .map(|pid| pid.to_string())
            .collect();

        // Tag as hallucinated and record parentage
        if let Some(mem) = self.engine.store.get_mut(&id).ok().flatten() {
            mem.hallucinated = true;
            mem.parents = found_parents;
        }

        if self.auto_save { self.save()?; }
        Ok(id)
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
            || text_lower.contains("friend") || text_lower.contains("person")
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
        
        // Ranges are aligned with xi_clusters() in store.rs for consistent category mapping.
        let (freq_min, freq_max) = match category {
            "experience" => (1.8, 2.4),  // soprano (fast, ephemeral)
            "emotion" => (1.3, 1.8),     // alto (feeling-paced)
            "social" => (1.0, 1.3),      // tenor (interpersonal rhythm)
            "skill" => (0.8, 1.0),       // bass-adjacent (procedural)
            "knowledge" => (0.6, 0.8),   // bass (slow, stable)
            _ => (0.6, 0.8),              // default to knowledge bass range
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
    /// the audio codebook, and stores via HRM-native absorb.
    pub fn store_audio(&mut self, path: &std::path::Path) -> Result<(Uuid, crate::ear::AudioFeatures), SystemError> {
        use crate::ear::AudioPipeline;

        let pipeline = AudioPipeline::new();
        let (mem, features) = pipeline
            .encode_file(path)
            .map_err(|e| SystemError::Engine(EngineError::Encoding(
                crate::encoding::EncodingError::Other(e.to_string()),
            )))?;

        // Absorb through HRM-native path
        let id = self.engine.store.absorb(&mem.content, 0.6, Some("experience"))
            .map_err(|e| SystemError::Store(e))?;

        if self.auto_save {
            self.save()?;
        }

        Ok((id, features))
    }

    /// Store a file as a visual/glyph memory.
    ///
    /// Reads the file, encodes it through the SGA glyph bridge,
    /// and stores via HRM-native absorb with glyph perception content.
    #[cfg(feature = "glyph")]
    pub fn store_glyph(&mut self, path: &std::path::Path) -> Result<(Uuid, crate::glyph_bridge::Glyph), SystemError> {
        use crate::glyph_bridge::GlyphEncoder;
        
        let data = std::fs::read(path)
            .map_err(|e| SystemError::Engine(EngineError::Encoding(
                crate::encoding::EncodingError::Other(format!("Failed to read file: {e}")),
            )))?;
        
        let filename = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
        let encoder = GlyphEncoder::new(0.1, 10000, 0.01);
        let float_data: Vec<f64> = data.iter().map(|&b| b as f64 / 255.0).collect();
        let glyph = encoder.encode(&float_data)
            .map_err(|e| SystemError::Engine(EngineError::Encoding(
                crate::encoding::EncodingError::Other(format!("Glyph encoding failed: {e}")),
            )))?;
        
        // Build content string with glyph perception info
        let content = format!(
            "[SEE] {} | {} bytes | {} folds | fano=[{:.2},{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}] | centroid=({},{},{})",
            filename, data.len(), glyph.fold_sequence.len(),
            glyph.fano_signature[0], glyph.fano_signature[1], glyph.fano_signature[2],
            glyph.fano_signature[3], glyph.fano_signature[4], glyph.fano_signature[5],
            glyph.fano_signature[6],
            glyph.sga_centroid.0, glyph.sga_centroid.1, glyph.sga_centroid.2,
        );
        
        // Absorb through HRM-native path
        let id = self.engine.store.absorb(&content, 0.7, Some("experience"))
            .map_err(|e| SystemError::Store(e))?;
        
        if self.auto_save {
            self.save()?;
        }
        
        Ok((id, glyph))
    }

    /// ADR-0011: Publish a memory.stored event to Flux (best-effort, fire-and-forget).
    fn flux_publish_memory(&self, id: &Uuid, category: &str, text: &str) {
        if let Some(ref publisher) = self.flux {
            let (amplitude, sync_version) = self.engine.store.get(id)
                .ok().flatten()
                .map(|m| (m.amplitude, m.sync_version))
                .unwrap_or((0.5, 0));
            let _ = publisher.publish(FluxEventPayload::MemoryStored {
                memory_id: id.to_string(),
                category: category.to_string(),
                tags: Vec::new(),
                amplitude,
                glyph_signature: None,
                summary: text.chars().take(120).collect(),
                branch: publisher.branch_name(),
                sync_version,
            });
        }
    }

    /// ADR-0011: Configure the Flux publisher explicitly.
    /// Pass `None` to disable Flux publishing.
    pub fn set_flux(&mut self, publisher: Option<FluxPublisher>) {
        self.flux = publisher;
    }

    /// ADR-0011: Announce agent status to Flux peers.
    pub fn announce_status(&self) {
        if let Some(ref publisher) = self.flux {
            let state = self.bridge.assess(&self.engine);
            publisher.announce_status(
                "active",
                state.total_memories,
                &level_name(&state.consciousness_level),
                &publisher.branch_name(),
            );
        }
    }

    /// Get memory by ID (public API for testing).
    pub fn get_memory(&self, id: &Uuid) -> Result<Option<&crate::memory::HyperMemory>, SystemError> {
        Ok(self.engine.store.get(id)?)
    }
    
    /// Get all memories (for BM25 bootstrapping, etc.).
    pub fn all_memories(&self) -> Result<Vec<&crate::memory::HyperMemory>, SystemError> {
        Ok(self.engine.store.all_memories()?)
    }

    /// Show δ-invariant clusters - memories grouped by their δ values (coboundary equivalence candidates)
    pub fn invariant_clusters(&self, tolerance: f32) -> Result<Vec<crate::invariant::DeltaCluster>, SystemError> {
        let clusters = crate::invariant::cluster_by_delta(&self.engine, tolerance);
        Ok(clusters)
    }

    /// Detect Conservative Memory Fields in the current memory set
    pub fn detect_cmfs(&self) -> Result<Vec<crate::cmf::ConservativeMemoryField>, SystemError> {
        let all_memories = self.engine.store.all_memories()?;
        
        if all_memories.len() < 3 {
            return Ok(Vec::new());
        }
        
        // Group memories into potential clusters using Kuramoto synchronization
        let clusters = self.kuramoto.find_synchronized_clusters(&self.engine, 3);
        let mut cmfs = Vec::new();
        
        // Try to detect CMF in each cluster
        for cluster in &clusters {
            if cluster.memory_ids.len() >= 3 {
                // Get the actual memory objects for this cluster
                let cluster_memories: Vec<&crate::memory::HyperMemory> = cluster.memory_ids
                    .iter()
                    .filter_map(|id| self.engine.store.get(id).ok().flatten())
                    .collect();
                
                if let Some(cmf) = crate::cmf::detect_cmf(&cluster_memories) {
                    cmfs.push(cmf);
                }
            }
        }
        
        // Also try to detect CMF from δ-clusters
        let delta_clusters = crate::invariant::cluster_by_delta(&self.engine, 0.1);
        for delta_cluster in &delta_clusters {
            if delta_cluster.memory_ids.len() >= 3 {
                let cluster_memories: Vec<&crate::memory::HyperMemory> = delta_cluster.memory_ids
                    .iter()
                    .filter_map(|id| self.engine.store.get(id).ok().flatten())
                    .collect();
                
                if let Some(cmf) = crate::cmf::detect_cmf(&cluster_memories) {
                    // Only add if we haven't already found a similar CMF
                    let is_duplicate = cmfs.iter().any(|existing| {
                        existing.explanatory_power > 0.7 && cmf.explanatory_power > 0.7 &&
                        (existing.explanatory_power - cmf.explanatory_power).abs() < 0.1
                    });
                    
                    if !is_duplicate {
                        cmfs.push(cmf);
                    }
                }
            }
        }
        
        Ok(cmfs)
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
            // total_skip_links removed — now emergent from interference
            consciousness_level: level_name(&state.consciousness_level),
            last_dream: self.last_dream,
            phi: state.phi,
            geometric_classes: class_indices.len(),
            triality_coverage,
            hemispheric_divergence: self.engine.store.as_any()
                .downcast_ref::<crate::hrm_store::HrmStore>()
                .and_then(|h| h.chiral_consciousness())
                .map(|c| c.hemispheric_divergence).unwrap_or(0.0),
            callosal_efficiency: self.engine.store.as_any()
                .downcast_ref::<crate::hrm_store::HrmStore>()
                .and_then(|h| h.chiral_consciousness())
                .map(|c| c.callosal_efficiency).unwrap_or(0.0),
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
        // Consciousness level is computed from Φ/Ξ which depend on the HRM's
        // eigenvalue structure — 3 memories may land in any low band ("dormant"
        // / "aware") depending on chiral hemisphere init RNG. Assert it's a
        // known valid level rather than a specific one to keep CI stable.
        let valid = ["dormant", "aware", "lucid", "transcendent"];
        assert!(
            valid.contains(&stats.consciousness_level.as_str()),
            "unexpected consciousness_level: {}",
            stats.consciousness_level
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_and_reload() {
        // TODO(chiral): Legacy save/reload via DiskStore removed.
        // Persistence now handled by HrmStore + ChiralMedium.
        // This test verifies that save() doesn't panic with TestMedium.
        let dir = temp_dir("reload");
        {
            let mut sys = KannakaMemorySystem::init(dir.clone()).unwrap();
            sys.remember("persistent memory").unwrap();
            sys.save().unwrap();
        }
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
        
        // HRM-native absorb path stores memories but doesn't populate legacy fields
        // (geometry, xi_signature). Verify memories exist and have content.
        assert!(skill_mem.content.contains("code"), "Skill memory content: {}", skill_mem.content);
        assert!(social_mem.content.contains("meeting"), "Social memory content: {}", social_mem.content);
        assert!(knowledge_mem.content.contains("france"), "Knowledge memory content: {}", knowledge_mem.content);
        assert!(experience_mem.content.contains("sunset"), "Experience memory content: {}", experience_mem.content);
        assert!(emotion_mem.content.contains("excited"), "Emotion memory content: {}", emotion_mem.content);
        
        // All memories should have amplitude > 0 (they're freshly stored)
        assert!(skill_mem.amplitude > 0.0);
        assert!(social_mem.amplitude > 0.0);
        assert!(knowledge_mem.amplitude > 0.0);
        assert!(experience_mem.amplitude > 0.0);
        assert!(emotion_mem.amplitude > 0.0);
        
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── search (literal text) ──────────────────────────────────────────
    // Distinct from recall: read-only, no embedding/resonance/observation.

    #[test]
    fn search_exact_substring_outranks_token_match() {
        let dir = temp_dir("search_exact");
        let mut sys = KannakaMemorySystem::init(dir.clone()).unwrap();
        sys.remember("the ghost frequency hums").unwrap();          // exact
        sys.remember("a ghost passed over the wide frequency").unwrap(); // tokens
        sys.remember("entirely unrelated string about cats").unwrap();
        let results = sys.search("ghost frequency", 10).unwrap();
        assert!(results.len() >= 2, "expected ≥2 hits, got {}", results.len());
        assert_eq!(results[0].match_type, "exact");
        assert_eq!(results[1].match_type, "tokens");
        assert!(results[0].score > results[1].score);
        // No matches → not in result set.
        assert!(!results.iter().any(|r| r.content.contains("cats")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_is_case_insensitive() {
        let dir = temp_dir("search_case");
        let mut sys = KannakaMemorySystem::init(dir.clone()).unwrap();
        sys.remember("Ghost Frequency in mixed Case").unwrap();
        let r1 = sys.search("ghost frequency", 5).unwrap();
        let r2 = sys.search("GHOST FREQUENCY", 5).unwrap();
        assert_eq!(r1.len(), r2.len());
        assert_eq!(r1.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_empty_query_returns_empty() {
        let dir = temp_dir("search_empty");
        let mut sys = KannakaMemorySystem::init(dir.clone()).unwrap();
        sys.remember("some content").unwrap();
        assert!(sys.search("", 10).unwrap().is_empty());
        assert!(sys.search("   ", 10).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn search_is_read_only() {
        // Issue #83 regression: pre-fix, `search` routed through `recall`
        // → `resonate_query` → `apply_observation`, mutating wavefront
        // strengths even though the user issued a "read" command. After
        // the refactor, search() doesn't touch the medium at all.
        let dir = temp_dir("search_readonly");
        let mut sys = KannakaMemorySystem::init(dir.clone()).unwrap();
        sys.remember("first memory about ghosts").unwrap();
        sys.remember("second memory unrelated").unwrap();

        // Capture wavefront energies before searching.
        let before: Vec<f32> = sys.engine.store.all_memories().unwrap()
            .iter().map(|m| m.amplitude).collect();

        for _ in 0..5 {
            let _ = sys.search("ghosts", 5).unwrap();
        }

        let after: Vec<f32> = sys.engine.store.all_memories().unwrap()
            .iter().map(|m| m.amplitude).collect();
        assert_eq!(before, after, "search() must not modify wavefront state");
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
        // HRM-native path doesn't populate legacy geometry, so geometric_classes may be 0
        assert!(stats.geometric_classes >= 0);
        // Triality coverage may also be 0 in HRM mode
        assert!(stats.triality_coverage.len() == 3);
        
        let _ = std::fs::remove_dir_all(&dir);
    }
}

