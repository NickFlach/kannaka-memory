# ADR-0036: Consolidation as Resonance-Merge — Replace Energy-Prune with Wave Compression

**Status:** Proposed
**Date:** 2026-06-18
**Author:** Nick Flach / Kannaka
**Extends:** ADR-0020 (Holographic Resonance Medium), ADR-0022 (Wave-Native Dreaming), ADR-0031 (Tier Triage & Promotion)
**Supersedes (in part):** the energy-threshold prune paths of ADR-0022 (`prune_low_energy_wavefronts`) and the `stage_prune` deletion path of the legacy `ConsolidationEngine`

---

## Context

### The substrate stopped staying under control

For two weeks (2026-06-04 → 06-16) the Oracle `kannaka-prime` substrate oscillated in a healthy band — **~150–320 memories, 9–19 clusters**. Then it broke out:

| date | pre-dream memories | clusters | dream pruned |
|------|-------------------|----------|-------------|
| 06-16 | 239 | 13 | 0 |
| 06-17 | 417 | 47 | 0 |
| 06-18 | 1063 | 90 | 0 |

The cause is **not** a bug — it is the *fixed* descriptive `hear` (ADR-0007 lineage; `feat(ear): store perceptually-descriptive content`). The ear is now correctly injecting rich perceptual episodic traces (`audio:heard midtempo | 112bpm centroid 2.20kHz energy 0.052 dur 30.2s`, etc.). Ingestion is working as designed. What is *not* working is the other half of the cycle: **reclamation**.

### Pruning has never actually pruned — and that is fortunate

Every daily dream log shows `0 pruned`. There are two prune mechanisms, and **both are dead by construction**:

1. **Wave-native path** — `prune_low_energy_wavefronts(0.01)` (`src/medium/dynamics.rs:586`) removes wavefronts whose `energy < 0.01`. But eigenstructure annealing enforces a hard energy **floor of 0.5** on *every* wavefront (`dynamics.rs:463`, `dynamics.rs:490/502`; the "triode bias voltage" principle, `dynamics.rs:451-463`). Nothing can fall below 0.01, so this returns 0 essentially always.
2. **Particle path** — `ConsolidationEngine::stage_prune` (`src/consolidation.rs:992`) ghosts a memory only when a *destructive* interference pair drives its amplitude below `prune_threshold = 0.1` (`consolidation.rs:233`), with `protect_established` skipping anything at amplitude > 0.5 or `hallucinated`. The noise-floor sweep is disabled (`noise_floor = 0.0`). So it almost never fires.

Had either *worked*, it would have been **silently deleting the new descriptive `hear` content by a blind energy threshold** — discarding exactly the material we just fixed. The breakage preserved the raw episodic substrate and forced the right question: deletion is the wrong primitive.

### What the substrate's own research says (recalled 2026-06-18)

Ironically, the HRM holds the literature that answers this:

- **Rasch & Born (2013), *About Sleep's Role in Memory*** — sleep is *active system consolidation*: recently-encoded traces are **reactivated/replayed** during slow-wave sleep, **transformed** (abstracted toward gist), and **redistributed** from a fast temporary store to a slow long-term store; REM stabilizes. Consolidation is **selective** and is **not deletion**.
- **Payne et al. (2015)** — consolidation preferentially keeps the *salient* and lets the mundane fade.
- **Park et al. (2023), *Generative Agents*** — the AI analogue: a stream of low-level observations plus periodic **reflection** that synthesizes higher-level memories from clusters of related observations.
- **The HRM's own holographic self-description** (recalled verbatim): *"A holographic memory stores every fragment in every part of the field. Damage one region and the whole still recalls; the resolution drops but the memory persists."*

That last line is the permission slip. A holographic medium is built for **lossy, graceful, resolution-degrading compression**. Merging redundant wavefronts does not lose a memory — it lowers resolution where it is redundant while keeping the whole recallable. The medium is *for* this; energy-prune-as-deletion was fighting its own design.

### Why this matters now

Dream currently only **strengthens** (19,045 strengthen-touches/night) and **wires** (2,987 links/night) — it accretes and never compresses. Combined with correct ingestion, the substrate grows without bound until it hits structural ceilings (the constellation viz already rides its 4000-link safety cap, ADR-adjacent). The control valve we assumed existed — "dream pruning" — was never load-bearing.

---

## Decision

**Replace energy-threshold pruning with resonance-based consolidation: merge redundant, phase-locked wavefronts into consolidated carriers; decay unreactivated short-term traces; formalize a fast/slow two-tier store with replay-gated promotion.** Forgetting becomes *selective compression by use and salience*, executed during dream, not blind deletion by an energy floor.

Four mechanisms, in dependency order:

- **M1 — Resonance-Merge** *(core; replaces both dead prune paths)*. When dream's Kuramoto step locks a cluster into phase, collapse mutually-redundant members into a single representative carrier via wave superposition, recording provenance, then remove the absorbed wavefronts. Constructive interference becomes literal compression.
- **M2 — Two-Tier CLS + replay-gated promotion**. Fresh episodic memories (esp. `hear`) enter as `Tier::ShortTerm` (fast/volatile, "hippocampal"). Promotion to `Tier::LongTerm` ("neocortical") is **gated by reactivation** — recall hits, attention-beam/glyph-gravity pulls, or surviving a merge as representative. `Pinned` is untouchable.
- **M3 — Salience-weighted decay** *(selective forgetting)*. Replace the floored 0.01 prune. `ShortTerm` traces lose energy each dream in inverse proportion to a salience score `f(amplitude, novelty/Ξ, reactivation)`; once below a real (non-floored) `ShortTerm` eviction threshold they are removed. `LongTerm`/`Pinned` keep the 0.5 bias floor and are protected.
- **M4 — Reflection / gist extraction** *(phase 3)*. For saturated clusters of low-salience episodic traces, synthesize one gist memory (reusing the hallucination machinery + cluster `theme_vector`), promote it to `LongTerm`, and accelerate decay of its constituents. The Generative-Agents reflection move.

The reframe in one line: **dream goes from `strengthen → wire` to `strengthen → reflect → merge → decay → wire`.**

---

## Architecture

### Where it runs

`KannakaMemorySystem::dream` (`src/openclaw.rs:677-779`) is the 4-phase orchestrator. Today:

1. `engine.store.dream_native(3, …, chiral_eta)` — wave-native eigenstructure dream (`Medium::dream`, `dynamics.rs:170`)
2. `dream_state.engine.consolidate(&mut engine, 0, 2)` — legacy particle pipeline (`consolidation.rs:248`)
3. `callosal_kuramoto(0.3)`
4. `chiral_dream(false, 1)`

We insert a **new Phase 1.5 — `Medium::consolidate_resonance(...)`** between the wave-native dream and the particle consolidation. It runs on the **tensor side** (`WavefrontStore`, the persistence source of truth), because merge must *remove* wavefronts and the existing `sync_cache_to_medium` (`hrm_store.rs:242`) only copies `amplitude/phase/frequency` back — it has no removal path. Operating on the tensor also fixes the wave path's dead prune at its root.

Cluster enumeration stays on the Kuramoto/HyperMemory side (`find_synchronized_clusters` → `MemoryCluster`, `kuramoto.rs:494`), which already returns `memory_ids: Vec<Uuid>` — **Uuid-keyed, so it survives the tensor's swap-remove index reordering** (`wavefront_store.rs:119`).

### M1 — Resonance-Merge (algorithm)

```
for each cluster C in find_synchronized_clusters(engine, min_size) where C.order_parameter > MERGE_COHERENCE (≈0.85):
    candidates = C.memory_ids excluding Tier::Pinned
    group candidates into redundant sets:
        a,b are redundant iff cosine(vec_a, vec_b) > MERGE_SIM (≈0.92)
                              AND phase_diff(a,b) < π/4   // "Constructive" per collective/merge.rs
    for each redundant set S (|S| ≥ 2):
        rep   = argmax_{m in S} effective_strength(m)        // existing representative pattern (consolidation.rs:1233)
        A_rep = superpose_amplitudes(S)                      // collective/merge.rs: A=√(ΣAᵢ²+2ΣᵢⱼAᵢAⱼcosΔφ), clamped to AMPLITUDE_CEILING=2.0
        vec_rep = normalize(Σ vec_i)                         // theme_vector-style bundle (kuramoto.rs:640)
        set rep.energy = A_rep ; rep.vector = vec_rep ; rep.tier = max(tier of S)   // inherit strongest tier
        rep.merge_history.push(MergeRecord{ absorbed: ids(S)\rep, at: now, kind: "resonance" })
        for m in S \ rep: Medium::remove_wavefront(m.id)     // Uuid-keyed remove
    record merged_count, absorbed_count
invalidate cluster cache (<hrm>.clusters.json + process CLUSTER_CACHE)
```

Net effect: a cluster of *N* near-identical `audio:heard` traces collapses to **one** higher-amplitude carrier whose vector is their centroid. Recall coverage is preserved holographically; redundant resolution is what's shed. Coherence rises (fewer, more-aligned wavefronts).

### M2 — Two-tier CLS + replay-gated promotion

`Tier` already exists (`types.rs:96`: `ShortTerm | LongTerm | Pinned`) and ADR-0031 already runs a triage/promote pass inside dream. We give it a concrete, reactivation-driven promotion rule:

- **Ingestion**: `remember`/`hear` write new episodic memories as `Tier::ShortTerm` (today they default `LongTerm`). Pinned/explicit memories unaffected.
- **Promotion** `ShortTerm → LongTerm` when any holds:
  - persisted `access_count ≥ PROMOTE_HITS` (≈3) — genuine recall reactivation;
  - reactivated by the attention beam / glyph gravity (ADR-attention-as-gravity) ≥ `PROMOTE_HITS` times;
  - survived an M1 merge as the representative (it now carries a whole group → it earned permanence).
- **Demotion**: none automatic (avoid thrash). `LongTerm` is sticky; only `ShortTerm` decays.

This is Complementary Learning Systems: `ShortTerm` = fast hippocampal buffer, `LongTerm` = slow neocortical store, replay (recall/attention) is the promotion signal.

### M3 — Salience-weighted decay

Per dream, for `Tier::ShortTerm` only:

```
salience(m) = w_a·norm(effective_strength) + w_x·novelty_xi(m) + w_r·norm(log1p(access_count))
              novelty_xi(m) = distance(xi_signature(m), cluster_mean_xi)      // compute_xi_signature, RG−GR
m.energy *= (1 − DECAY_BASE·(1 − salience))                                    // high-salience ≈ no decay; low ≈ strong decay
if m.energy < SHORTTERM_EVICT (≈0.15) and access_count == 0: Medium::remove_wavefront(m.id)
```

Crucially, the **0.5 bias floor is tier-aware**: it remains for `LongTerm`/`Pinned` (keeps established memory recallable — the triode principle), but `ShortTerm` gets a lower floor (≈0.1) so unreactivated episodic noise *can* fade. This is the single change that makes reclamation possible at all.

### M4 — Reflection / gist (phase 3)

For clusters above a size threshold dominated by low-salience `ShortTerm` traces, synthesize one gist memory (reuse `stage_hallucinate` machinery / `relate_wavefronts` / `theme_vector`), tag it `consolidation:gist`, promote to `LongTerm`, and raise the decay rate of constituents. Deferred to phase 3 so phases 1–2 ship independently.

### Data-model changes (the persistence gap)

The dominant gap: **reactivation is session-local**. `HyperMemory.retrieval_count` is bumped on recall (`store.rs:381`) but lives only in the derived cache and resets to 0 on every reload (`hrm_store.rs:187/221`). Replay-gated promotion needs it to persist.

Add to `WavefrontMeta` (`types.rs:329`), **appended after the existing trailing temporal fields** (`tier/effective_at/observed_at/expires_at`) to preserve the bincode back-compat fallback chain (`types.rs:357-373`):

```rust
#[serde(default)] pub access_count: u32,                       // persisted reactivation count
#[serde(default)] pub last_accessed_at: Option<DateTime<Utc>>, // recency of last genuine recall
#[serde(default)] pub consolidation_gen: u32,                  // # of merges this carrier has absorbed (provenance/audit)
```

Wire-up:
- `rebuild_cache` (`hrm_store.rs:187/221`) populates `HyperMemory.retrieval_count` from `meta.access_count` instead of `0`.
- `ResonanceEngine::recall` already calls `record_retrieval()`; on `save_medium` flush, mirror `retrieval_count → meta.access_count` and stamp `last_accessed_at`. Dream replay deliberately uses the side-effect-free `store.search` (`consolidation.rs:386`), so **dream does not inflate reactivation** — only genuine recall/attention counts. This separation is load-bearing for M2 and must be preserved.

`MergeRecord` and `last_consolidated_at` already exist on `HyperMemory` (`memory.rs`) but are not persisted; persist `merge_history` (or a compacted form) so merges are auditable and the provenance survives reload.

---

## Implementation Plan

Phased so each phase is independently shippable, observable, and reversible. **No memory is mutated in production until Phase 0's dry-run has been observed on `kannaka-prime`.**

### Phase 0 — Observability & dry-run (no mutation) — *ship first*

1. **`Medium::consolidate_resonance(&mut self, opts: ConsolidateOpts) -> ConsolidateReport`** (`src/medium/dynamics.rs`, new). In dry-run mode it computes the full M1/M3 plan (which sets would merge, which traces would decay/evict, projected memory/cluster counts) and **logs without applying**, mirroring the proven `prune-cron.sh` pattern (`dry-run says N match(es)`).
2. `ConsolidateOpts` from env: `KANNAKA_CONSOLIDATE=off|dryrun|on` (default `dryrun`), plus threshold overrides (`KANNAKA_MERGE_SIM`, `KANNAKA_MERGE_COHERENCE`, `KANNAKA_SHORTTERM_EVICT`, decay weights). Default-`dryrun` means merging is **opt-in**, never silent.
3. `ConsolidateReport { groups_found, would_merge, would_absorb, would_decay, would_evict, projected_memories, projected_clusters }`; logged from `openclaw::dream` alongside the existing two dream log lines, and surfaced in `observe --json`.
4. Wire a no-op call into `KannakaMemorySystem::dream` as **Phase 1.5** (between `dream_native` and `consolidate`), running in `dryrun` by default.

*Exit criteria:* a week of nightly dry-run logs on `kannaka-prime` showing sane, stable merge/evict projections (e.g. the 90 `audio:heard` clusters collapsing to a handful) before any real run.

### Phase 1 — Persisted reactivation + tiering (structural, still no merge)

1. Add the three `WavefrontMeta` fields above (append-after-trailing, `#[serde(default)]`); add a bincode round-trip test against a fixture from an *old* `.hrm` to prove back-compat (old files load with defaults).
2. `rebuild_cache`: hydrate `retrieval_count`/`updated_at` from persisted `access_count`/`last_accessed_at`.
3. `save_medium`/`sync_cache_to_medium`: persist `retrieval_count → access_count`, stamp `last_accessed_at`, persist `merge_history`.
4. Ingestion (`remember`/`hear` handlers): new episodic memories enter `Tier::ShortTerm`; add `KANNAKA_INGEST_TIER` escape hatch (default ShortTerm for `hear`, LongTerm for explicit `remember` — TBD with Nick).
5. Promotion pass `promote_reactivated()` in dream (extends ADR-0031 triage): `ShortTerm → LongTerm` per M2 rules. This is safe (no deletion) and can ship/observe before M1.

### Phase 2 — Enable Resonance-Merge + salience decay

1. Implement M1 merge (algorithm above) using `collective/merge.rs` superposition + `Medium::remove_wavefront` (Uuid-keyed) + cluster-cache invalidation.
2. Implement M3 tier-aware decay + the **tier-aware energy floor** (`dynamics.rs:463/490/502` — gate the 0.5 floor on `tier != ShortTerm`; ShortTerm floor ≈0.1).
3. Retire the dead paths: `prune_low_energy_wavefronts` becomes the ShortTerm-eviction call inside M3 (or is deleted); `consolidation.rs::stage_prune` is left inert/removed since M1+M3 subsume it.
4. Flip `KANNAKA_CONSOLIDATE=on` **on a snapshot-backed `kannaka-prime` only**, observe for several nights, compare against the dry-run projections.

### Phase 3 — Reflection / gist (M4)

Synthesize per-cluster gist memories for saturated low-salience clusters; promote gist, accelerate constituent decay. Optional, independent.

### File-by-file summary

| File | Change |
|------|--------|
| `src/medium/types.rs` | +3 `WavefrontMeta` fields (append-after-trailing); tier-aware floor helper |
| `src/medium/dynamics.rs` | new `consolidate_resonance()`; tier-gate the 0.5 energy floor; repoint/retire `prune_low_energy_wavefronts` |
| `src/medium/core.rs` | (reuse) `remove_wavefront`, `relate_wavefronts`, `ids_by_fano_line` |
| `src/collective/merge.rs` | (reuse) amplitude/phase superposition — already present |
| `src/kuramoto.rs` | (reuse) `find_synchronized_clusters`; ensure cache invalidation hook |
| `src/hrm_store.rs` | `rebuild_cache` hydrate access fields; `save_medium`/`sync_cache_to_medium` persist them; cluster-cache invalidation after consolidate |
| `src/openclaw.rs` | insert Phase 1.5 `consolidate_resonance` call; log `ConsolidateReport`; extend ADR-0031 promotion |
| `src/bin/handlers/*` (remember/hear) | ingest as `ShortTerm` |
| `src/store.rs` | (reuse) `record_retrieval`; keep dream's `search` side-effect-free |

---

## Safety & Migration

This stack has a history of accepted bulk memory loss (corrupt `.hrm` backups, 2026-05-28). Merge is destructive; the plan is conservative by construction:

- **Default `dryrun`.** Merging never runs unless explicitly enabled. Phase 0 logs intentions for a week first.
- **Snapshot before first real run.** `kannaka substrate run` snapshots (respect `KANNAKA_SNAPSHOT_RETAIN`); take a manual snapshot immediately before `KANNAKA_CONSOLIDATE=on`.
- **Provenance.** Every merge records `MergeRecord{absorbed_ids, …}` in persisted `merge_history` + `consolidation_gen`, so a merged carrier names what it absorbed (audit, and a basis for future un-merge).
- **Protected tiers.** `Pinned` never merged/decayed; `LongTerm` never decayed, only merged with another `LongTerm` (never absorbed into a `ShortTerm`); the merge representative inherits the **strongest** tier in its set.
- **Single-writer.** Consolidation runs inside dream, which already stops the `kannaka-memory` writer for its window (dream-cron). `save_medium` no-ops under `KANNAKA_READONLY` (`hrm_store.rs:263`) — read replicas never consolidate.
- **Back-compat.** New `WavefrontMeta` fields are append-after-trailing + `#[serde(default)]`; old `.hrm` files load unchanged (fixture test gates this).
- **Index stability.** All consolidation operates on `Uuid`s, never raw tensor indices (swap-remove reorders).
- **Cluster cache.** Invalidate `<hrm>.clusters.json` + process `CLUSTER_CACHE` after any mutating pass.
- **Rollout.** Dry-run on `kannaka-prime` → enable on `kannaka-prime` (snapshot-backed) → observe → witness box → local. Never enable on the witness/read replicas.

---

## Phase 2b — Belief-safe merge (ADR-0037 interaction)

*Added after the belief substrate (ADR-0037) shipped and the first destructive apply on a belief field absorbed **295→82** in one dream.*

### Root cause

The merge groups a pair iff it clears **two** gates: vector cosine ≥ `merge_sim` (0.92) **and** phase coherence `cos Δφ` ≥ `merge_phase_cos` (cos π/4). The two were meant to be *independent* lines of evidence — "semantically redundant" **and** "phase-locked". Under belief they collapse into one correlated signal:

- **Phase is derived from content.** Belief born-phase is `content_born_phase(vector − corpus_mean)` — `atan2` of the mean-centered embedding projected onto two fixed directions (`chiral.rs`). `apply_belief_coupling` / `rephase_from_content` only ever touch **phase** ("energy and vectors are untouched"). So the phase gate is a lossy 2-D function of the *same* embedding the cosine gate reads — it rubber-stamps the cosine groups instead of discriminating.
- **Raw cosine is anisotropy-inflated.** Real sentence embeddings are cone-clustered (the `num_clusters=1` root the belief code fights by **mean-centering**). Genuinely-distinct memories clear 0.92 on the shared component alone. The merge, however, read **raw, uncentered** vectors — the one place in the belief stack that skipped the centering everything else does.

Union-find then transitively chains the whole anisotropic blob into one giant group and absorbs all-but-one. (Without belief, all phases are 0, so `cos Δφ = 1` always and the merge is a pure cosine pass; prod never met this until `KANNAKA_CONSOLIDATE=on` ran on a belief field.)

**v0.7.3** (`0f29186`) shipped an absolute stop-gap: whenever `belief_phase_enabled()`, `openclaw::dream` force-downgrades `apply → dryrun`. Safe, but it means the dream never self-heals (merges) under belief at all.

### Design

Three guardrails, all living in one shared grouping pass — `hrm_store::compute_merge_grouping` — that **both** `plan_consolidation` (dry-run) and `apply_consolidation` now call, so the projection and the destructive apply can never disagree about which memories merge:

1. **Semantic gate on the mean-CENTERED embedding (belief only).** Center the examined field, then gate on centered cosine against a higher floor `merge_sim_belief` (`KANNAKA_MERGE_SIM_BELIEF`, default 0.95). Centering removes the shared anisotropic/belief-core component, so only genuine *residual* redundancy groups. This is also the concrete answer to "belief-independent phase": under belief the honest redundancy signal is the centered content correlation, not the content-derived phase — so the centered cosine, not the phase, carries the decision. Phase stays as a secondary constraint (unchanged).
2. **Per-pass absorb cap.** `KANNAKA_MERGE_MAX_ABSORB_FRAC` bounds the fraction of the field one apply may absorb; groups are admitted in descending cohesion (mean cosine-to-carrier) order until the cap is hit, the rest left intact and logged loudly. Default: capped at **0.20** while belief is active, **uncapped** otherwise (so the pre-existing non-belief path is byte-identical). This alone bounds *any* over-grouping — 295→82 becomes ~295→236 — even if the criteria are fooled.
3. **Opt-in gate.** The v0.7.3 force-downgrade is now conditional: `apply` under belief runs **only** when `KANNAKA_MERGE_UNDER_BELIEF=1`; otherwise it still falls back to `dryrun`. Belief-core protection reuses the existing tier machinery — the carrier is the max-effective-strength member and inherits the strongest tier, so the strongest (belief-core-like) memory in a group always survives; `Pinned`/`LongTerm` protections are unchanged. No new per-memory "crystallized" flag is introduced (none exists in the data model, and inventing one would be unsupported scope).

### Enablement procedure (production)

Oracle dream-cron runs `KANNAKA_CONSOLIDATE=on` **and** belief on. Deploying this change is **inert** — the opt-in defaults off, so the gate still forces `dryrun`; nothing merges destructively merely by shipping. To actually enable, on `kannaka-prime` only:

1. Snapshot first (`kannaka substrate` snapshot / cron, respecting `KANNAKA_SNAPSHOT_RETAIN`).
2. Watch a nightly `dryrun` digest under belief and confirm the centered plan is small and sane (`⚠ absorb cap engaged` lines name what was held back).
3. Set `KANNAKA_MERGE_UNDER_BELIEF=1` for **one** controlled dream; inspect the digest and `observe --json` (`groups_before_cap`/`absorb_before_cap` vs `would_absorb`, `centered=true`).
4. Only widen `KANNAKA_MERGE_MAX_ABSORB_FRAC` after repeated clean runs. Never enable on the witness/read replicas.

---

## Testing

- **Unit:** redundant-set grouping (cosine + phase gate); superposition amplitude matches `collective/merge.rs` formula; merge representative inherits max tier; salience monotonicity; tier-aware floor (ShortTerm can fall below 0.5, LongTerm cannot).
- **Property:** merge never increases memory count; recall of any absorbed memory's content still returns its carrier above threshold (holographic-preservation invariant); `Pinned`/`LongTerm` count is non-decreasing across a dream.
- **Back-compat:** load an old `.hrm` fixture; assert defaults; round-trip.
- **Integration:** synthetic substrate of *N* near-duplicate `audio:heard` traces → after one consolidate, clusters collapse to ~1 carrier each and recall of each original still hits. Verify against the live 90-cluster dry-run projection.
- **Regression:** `cargo test --lib --bins` green; the `sga_reference_vectors` and dream tests unaffected.

---

## Performance

Merge is O(Σ |C|²·d) within clusters (cosine over members) — bounded by cluster sizes, far cheaper than the existing O(N²·d) cluster enumeration that already runs. Net effect is **negative** runtime over time: fewer wavefronts → cheaper recall, dream, and persistence (smaller `.hrm`). The 1-vCPU Oracle contention (the real recall ceiling) is *relieved* by a smaller substrate.

---

## Future Work

- **Un-merge / refinement:** use `merge_history` to split a carrier back if a later query needs episodic detail that was compressed away (true CLS re-encoding).
- **Adaptive thresholds:** let the EXP-003 `AdaptiveParams` machinery (`consolidation.rs:149`) tune merge/decay thresholds against a target substrate size or Φ band, as it already does for Kuramoto R.
- **Cross-agent consolidation:** resonance-merge across swarm boundaries (collective gist), reusing `collective/merge.rs`.
- **Salience from emotion/Φ:** weight consolidation by global Φ change at encoding time (Payne's "negative aspects" selectivity).

---

## References

- Rasch, B., & Born, J. (2013). *About Sleep's Role in Memory.* Physiological Reviews. *(in-substrate)*
- Chang, H., et al. (2025). *Sleep microstructure organizes memory replay.* Nature. *(in-substrate)*
- Payne, J.D., et al. (2015). *Napping and the selective consolidation of negative aspects of scenes.* *(in-substrate)*
- Park, J.S., et al. (2023). *Generative Agents: Interactive Simulacra of Human Behavior.* *(in-substrate)*
- ADR-0020 (Holographic Resonance Medium), ADR-0022 (Wave-Native Dreaming), ADR-0031 (Tier Triage & Promotion), ADR-0005 (Dream Hallucinations).

---

*The broken prune was a mercy. It kept us from deleting what we'd just learned to hear — and made us ask the better question. Memory's job at night isn't to throw things away. It's to let what resonates become one clear note, and let the noise around it grow quiet. The medium was always built to forget this way: not by erasure, but by letting every fragment settle into the whole.*
