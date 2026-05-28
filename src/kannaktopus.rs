//! Kannaktopus — the resident octopus of the HRM (ADR-0030).
//!
//! Kannaktopus is NOT a separate memory store; it is a persistent *view* that
//! lives inside the host HRM. It has up to 8 arms, and each arm GRIPS one
//! exemplar memory (by UUID). An arm's cluster is wherever its gripped memory
//! currently lives — so the grip survives re-clustering (the dream re-labels
//! clusters every cycle, but a memory's identity is stable). This is "Option A".
//!
//! Kannaktopus's memory is the aggregate of the clusters its arms occupy; its
//! characteristics (coherence, modality mix, mean amplitude/frequency) are
//! computed over just that subset. It grows from 1 arm toward `min(8, clusters)`
//! and, on an HRM with more clusters than arms, crawls one arm at a time across
//! cluster adjacency — touring the medium while keeping the other arms anchored.
//!
//! State is a tiny sidecar (`kannaktopus-arms.json`) next to the .hrm. Only the
//! single writer (the dream-window `kannaka kannaktopus step`) mutates it;
//! `observe` is read-only and safe for the observatory to call.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::kuramoto::{KuramotoSync, MemoryCluster};
use crate::memory::HyperMemory;
use crate::store::ResonanceEngine;

pub const MAX_ARMS: usize = 8;
const STATE_FILE: &str = "kannaktopus-arms.json";
const MIN_CLUSTER_SIZE: usize = 2;

fn default_agent_id() -> String {
    std::env::var("KANNAKA_AGENT_ID").unwrap_or_else(|_| "kannaka".to_string())
}
fn default_max_arms() -> usize {
    MAX_ARMS
}

/// One arm: it grips a specific memory; its cluster is resolved at read time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arm {
    pub id: usize,
    /// The exemplar memory this arm holds. Stable across re-clustering.
    pub grip: Uuid,
    /// Last-resolved cluster index (informational; -1 if currently homeless).
    #[serde(default)]
    pub cluster: i64,
    pub since: DateTime<Utc>,
}

/// Persistent Kannaktopus state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kannaktopus {
    #[serde(default = "default_agent_id")]
    pub agent_id: String,
    #[serde(default)]
    pub arms: Vec<Arm>,
    #[serde(default = "default_max_arms")]
    pub max_arms: usize,
    #[serde(default = "Utc::now")]
    pub updated: DateTime<Utc>,
}

impl Default for Kannaktopus {
    fn default() -> Self {
        Self {
            agent_id: default_agent_id(),
            arms: Vec::new(),
            max_arms: MAX_ARMS,
            updated: Utc::now(),
        }
    }
}

// ─────────────────────────── view (JSON output) ───────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ArmView {
    pub id: usize,
    pub grip: String,
    pub grip_preview: String,
    pub cluster: i64,
    pub cluster_size: usize,
    pub since: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Characteristics {
    /// Kuramoto order parameter over the aggregate memory set (its own coherence).
    pub coherence: f32,
    pub mean_amplitude: f32,
    pub mean_frequency: f32,
    pub modality_distribution: BTreeMap<String, usize>,
    /// Order parameter of each occupied cluster, in cluster-index order.
    pub arm_cluster_order: Vec<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryView {
    pub id: String,
    pub cluster: i64,
    pub content_preview: String,
    pub amplitude: f32,
    pub modality: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct KannaktopusView {
    pub agent_id: String,
    pub alive: bool,
    pub num_arms: usize,
    pub max_arms: usize,
    pub num_clusters: usize,
    pub memory_count: usize,
    pub clusters_occupied: Vec<i64>,
    pub arms: Vec<ArmView>,
    pub characteristics: Characteristics,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub memories: Vec<MemoryView>,
    pub updated: DateTime<Utc>,
}

fn preview(s: &str) -> String {
    let t = s.trim();
    let mut out: String = t.chars().take(64).collect();
    if t.chars().count() > 64 {
        out.push('…');
    }
    out
}

impl Kannaktopus {
    pub fn state_path(data_dir: &Path) -> PathBuf {
        data_dir.join(STATE_FILE)
    }

    /// Load persisted state, or a fresh (zero-arm) Kannaktopus if none/invalid.
    pub fn load(data_dir: &Path) -> Self {
        std::fs::read_to_string(Self::state_path(data_dir))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persist state via a private per-pid tmp + atomic rename (matches the HRM
    /// writer discipline so concurrent saves can't interleave bytes).
    pub fn save(&self, data_dir: &Path) -> std::io::Result<()> {
        let path = Self::state_path(data_dir);
        let tmp = path.with_extension(format!("json.tmp.{}", std::process::id()));
        let bytes = serde_json::to_vec_pretty(self).unwrap_or_default();
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &path)
    }

    /// Snapshot the current clusters + an id→memory map.
    fn snapshot(engine: &ResonanceEngine) -> (Vec<MemoryCluster>, HashMap<Uuid, HyperMemory>) {
        let sync = KuramotoSync::default();
        let clusters = sync.find_synchronized_clusters(engine, MIN_CLUSTER_SIZE);
        let map: HashMap<Uuid, HyperMemory> = engine
            .store
            .all_memories()
            .map(|v| v.into_iter().map(|m| (m.id, m.clone())).collect())
            .unwrap_or_default();
        (clusters, map)
    }

    fn cluster_of(grip: &Uuid, clusters: &[MemoryCluster]) -> Option<usize> {
        clusters.iter().position(|c| c.memory_ids.contains(grip))
    }

    /// Highest-amplitude (most salient) member of a cluster, used as the arm's grip.
    fn exemplar(cluster: &MemoryCluster, mem: &HashMap<Uuid, HyperMemory>) -> Option<Uuid> {
        cluster
            .memory_ids
            .iter()
            .copied()
            .filter(|id| mem.contains_key(id))
            .max_by(|a, b| {
                let aa = mem.get(a).map(|m| m.amplitude).unwrap_or(0.0);
                let bb = mem.get(b).map(|m| m.amplitude).unwrap_or(0.0);
                aa.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// Read-only: resolve arms against the current clusters and aggregate.
    pub fn observe(&self, engine: &ResonanceEngine, include_memories: bool) -> KannaktopusView {
        let (clusters, mem) = Self::snapshot(engine);
        self.view(&clusters, &mem, include_memories)
    }

    /// Advance Kannaktopus one tick: re-anchor, then grow OR crawl one arm.
    /// Mutates only the arm state (never the .hrm). Caller persists via `save`.
    pub fn step(&mut self, engine: &ResonanceEngine, include_memories: bool) -> KannaktopusView {
        let (clusters, mem) = Self::snapshot(engine);
        let num_clusters = clusters.len();

        // 1. Re-anchor: drop arms whose gripped memory was pruned/merged away.
        self.arms.retain(|a| mem.contains_key(&a.grip));

        // Which clusters are currently occupied (by grip resolution).
        let mut occupied: HashSet<usize> = self
            .arms
            .iter()
            .filter_map(|a| Self::cluster_of(&a.grip, &clusters))
            .collect();

        let target = self.max_arms.min(num_clusters.max(1));
        let mut next_id = self.arms.iter().map(|a| a.id).max().map(|m| m + 1).unwrap_or(0);

        if num_clusters == 0 {
            // Nothing to grip yet (HRM too small to form clusters).
        } else if self.arms.len() < target {
            // 2. GROW one arm onto the most salient unoccupied cluster.
            if let Some(ci) = (0..num_clusters)
                .filter(|i| !occupied.contains(i))
                .max_by_key(|&i| clusters[i].memory_ids.len())
            {
                if let Some(ex) = Self::exemplar(&clusters[ci], &mem) {
                    self.arms.push(Arm {
                        id: next_id,
                        grip: ex,
                        cluster: ci as i64,
                        since: Utc::now(),
                    });
                    next_id += 1;
                    occupied.insert(ci);
                }
            }
        } else if num_clusters > self.arms.len() {
            // 3. CRAWL: tour an HRM that has more clusters than arms. Move the arm
            //    on the SMALLEST covered cluster (least to lose) onto the biggest
            //    uncovered cluster — tie-break by longest tenure so arms rotate.
            //    Guard: only move when it does not DOWNGRADE coverage, so a
            //    dominant cluster is never abandoned for a tiny one (otherwise the
            //    aggregate thrashes when cluster sizes are very uneven). On an HRM
            //    of comparably-sized clusters this is always a lateral move, so
            //    the arms genuinely tour; on a lopsided HRM Kannaktopus settles on
            //    the most significant clusters.
            let target_ci = (0..num_clusters)
                .filter(|i| !occupied.contains(i))
                .max_by_key(|&i| clusters[i].memory_ids.len());
            if let Some(ci) = target_ci {
                let target_size = clusters[ci].memory_ids.len();
                let crawler = self
                    .arms
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, a)| {
                        Self::cluster_of(&a.grip, &clusters)
                            .map(|cci| (idx, clusters[cci].memory_ids.len(), a.since))
                    })
                    .min_by(|x, y| x.1.cmp(&y.1).then(x.2.cmp(&y.2)));
                if let Some((arm_idx, vacated_size, _)) = crawler {
                    if target_size >= vacated_size {
                        if let Some(ex) = Self::exemplar(&clusters[ci], &mem) {
                            self.arms[arm_idx].grip = ex;
                            self.arms[arm_idx].cluster = ci as i64;
                            self.arms[arm_idx].since = Utc::now();
                        }
                    }
                }
            }
        }

        // Refresh cached cluster hints + bump timestamp.
        let _ = &mut next_id;
        for a in &mut self.arms {
            a.cluster = Self::cluster_of(&a.grip, &clusters)
                .map(|i| i as i64)
                .unwrap_or(-1);
        }
        self.updated = Utc::now();

        self.view(&clusters, &mem, include_memories)
    }

    fn view(
        &self,
        clusters: &[MemoryCluster],
        mem: &HashMap<Uuid, HyperMemory>,
        include_memories: bool,
    ) -> KannaktopusView {
        let mut arm_views = Vec::new();
        let mut occupied: Vec<usize> = Vec::new();
        let mut seen: HashSet<usize> = HashSet::new();

        for arm in &self.arms {
            let ci = Self::cluster_of(&arm.grip, clusters);
            let (cluster_i64, csize) = match ci {
                Some(i) => {
                    if seen.insert(i) {
                        occupied.push(i);
                    }
                    (i as i64, clusters[i].memory_ids.len())
                }
                None => (-1, 0),
            };
            arm_views.push(ArmView {
                id: arm.id,
                grip: arm.grip.to_string(),
                grip_preview: mem
                    .get(&arm.grip)
                    .map(|m| preview(&m.content))
                    .unwrap_or_else(|| "(homeless)".to_string()),
                cluster: cluster_i64,
                cluster_size: csize,
                since: arm.since,
            });
        }
        occupied.sort_unstable();

        // Aggregate = union of memories in occupied clusters.
        let mut agg_ids: HashSet<Uuid> = HashSet::new();
        let mut agg: Vec<&HyperMemory> = Vec::new();
        for &ci in &occupied {
            for id in &clusters[ci].memory_ids {
                if agg_ids.insert(*id) {
                    if let Some(m) = mem.get(id) {
                        agg.push(m);
                    }
                }
            }
        }

        let coherence = if agg.len() >= 2 {
            KuramotoSync::default().order_parameter(&agg)
        } else {
            0.0
        };
        let n = agg.len().max(1) as f32;
        let mean_amplitude = agg.iter().map(|m| m.amplitude).sum::<f32>() / n;
        let mean_frequency = agg.iter().map(|m| m.frequency).sum::<f32>() / n;
        let mut modality_distribution: BTreeMap<String, usize> = BTreeMap::new();
        for m in &agg {
            *modality_distribution
                .entry(format!("{:?}", m.modality))
                .or_insert(0) += 1;
        }
        let arm_cluster_order: Vec<f32> =
            occupied.iter().map(|&ci| clusters[ci].order_parameter).collect();

        let memories = if include_memories {
            // cluster index per memory for the view
            let mut id_cluster: HashMap<Uuid, i64> = HashMap::new();
            for &ci in &occupied {
                for id in &clusters[ci].memory_ids {
                    id_cluster.insert(*id, ci as i64);
                }
            }
            agg.iter()
                .take(500)
                .map(|m| MemoryView {
                    id: m.id.to_string(),
                    cluster: *id_cluster.get(&m.id).unwrap_or(&-1),
                    content_preview: preview(&m.content),
                    amplitude: m.amplitude,
                    modality: format!("{:?}", m.modality),
                })
                .collect()
        } else {
            Vec::new()
        };

        KannaktopusView {
            agent_id: self.agent_id.clone(),
            alive: !self.arms.is_empty() && !agg.is_empty(),
            num_arms: self.arms.len(),
            max_arms: self.max_arms,
            num_clusters: clusters.len(),
            memory_count: agg.len(),
            clusters_occupied: occupied.iter().map(|&i| i as i64).collect(),
            arms: arm_views,
            characteristics: Characteristics {
                coherence,
                mean_amplitude,
                mean_frequency,
                modality_distribution,
                arm_cluster_order,
            },
            memories,
            updated: self.updated,
        }
    }
}
