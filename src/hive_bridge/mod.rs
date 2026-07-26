//! Hive → NATS bridge logic (ADR-0045). The binary is plumbing; everything
//! testable lives here, mirroring how `nostr::bridge` relates to
//! `kannaka_nostr_bridge`.

pub mod map;
pub mod policy;
pub mod roster;

pub use map::{map_event, MapContext, Mapped};
// filled in by Task 6: policy.rs is a doc-comment-only placeholder so far —
// `PolicyMap` does not exist yet.
// pub use policy::PolicyMap;
// filled in by Task 7: roster.rs is a doc-comment-only placeholder so far —
// `Roster` does not exist yet.
// pub use roster::Roster;
