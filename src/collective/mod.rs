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
pub mod flux;
pub mod merge;
pub mod trust;

pub use artifacts::{DreamArtifact, ArtifactHallucination, ArtifactSkipLink, ArtifactCluster};
pub use flux::{FluxConfig, FluxPublisher, FluxSubscriber, FluxEventPayload, evaluate_pull, PullDecision};
pub use merge::{classify_merge, apply_constructive, apply_destructive, apply_partial, MergeKind, MergeResult, QuarantineEntry};
pub use trust::AgentTrustStore;
