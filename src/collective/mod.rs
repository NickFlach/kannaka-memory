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
pub mod merge;
pub mod privacy;
pub mod trust;

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
pub use trust::AgentTrustStore;
