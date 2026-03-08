//! ADR-0011: Collective Memory Architecture
//!
//! Three-layer architecture:
//! - **Dolt** (Local Memory): agent-local persistence, full memory store + skip links
//! - **Flux** (Nervous System): lightweight event signaling, metadata only
//! - **DoltHub** (Commons): shared repository, branch-based memory convergence
//!
//! # Branch Conventions
//! ```
//! main                          ← consensus (merged, vetted)
//! ├── <agent>/working           ← current memories (auto-push)
//! ├── <agent>/dream/<date>      ← dream cycle results
//! ├── collective/mars-sim       ← multi-agent speculation space
//! └── collective/quarantine     ← conflicting memories under review
//! ```

pub mod artifacts;
pub mod commitments;
pub mod flux;
pub mod glyph_store;
pub mod merge;
pub mod privacy;
pub mod proofs;
pub mod revelation;
pub mod search;
pub mod trust;
pub mod visual;

pub use artifacts::{DreamArtifact, ArtifactHallucination, ArtifactSkipLink, ArtifactCluster};
pub use flux::{FluxConfig, FluxPublisher, FluxSubscriber, FluxEventPayload, evaluate_pull, PullDecision};
pub use merge::{classify_merge, merge_guard, apply_constructive, apply_destructive, apply_partial, MergeKind, MergeResult, QuarantineEntry};
pub use commitments::{
    PedersenCommitment, CommitmentOpening, GlyphCommitments, GlyphOpenings,
    CommitmentError, commit_wave_properties, merge_commitments, merge_openings,
    verify_all, verify_amplitude_above, hash_vector, compute_fano_energies,
};
pub use privacy::{
    PrivacyGlyph, EncryptedCapsule, BloomParameters, BloomSolution,
    BloomHint, BloomedMemory, PrivacyLevel, PrivacyError, SealResult,
    seal, seal_with_commitments, bloom, bloom_with_hint, create_hint,
    suggest_difficulty,
};
pub use glyph_store::{
    GlyphStore, StoredGlyph, StoredProof, ProofType, GroupKey,
    ProofTrustRecord, GlyphMergeResult, GLYPH_SCHEMA,
    merge_glyphs, verify_merge,
};
pub use proofs::{
    ExistenceProof, AmplitudeRangeProof, CategoryProof, DepthProof,
    SimilarityProof, NonHallucinationProof,
    prove_existence, verify_existence,
    prove_amplitude_range, verify_amplitude_range,
    prove_category, verify_category,
    prove_depth, verify_depth,
    prove_similarity, verify_similarity,
    prove_non_hallucination, verify_non_hallucination,
};
pub use trust::AgentTrustStore;
pub use search::{
    SearchRequest, SearchResult, ProofExchangeEvent,
    hash_query, collective_search, respond_to_proof_request, process_proof_response,
};
pub use revelation::{
    RevelationPolicy, RevelationRule, RevelationAction,
    evaluate_policy, evaluate_pending_policies, execute_revelation,
    create_group, add_group_member, revoke_group_member, group_effective_difficulty,
};
pub use visual::{
    GlyphVisual, GlyphCluster,
    fano_to_visual, render_svg, render_collective_svg,
    visualize_store, cluster_visuals,
};
