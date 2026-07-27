//! Hive → NATS bridge logic (ADR-0045). The binary is plumbing; everything
//! testable lives here, mirroring how `nostr::bridge` relates to
//! `kannaka_nostr_bridge`.

pub mod map;
pub mod policy;
pub mod roster;

pub use map::{map_event, MapContext, Mapped};
pub use policy::PolicyMap;
pub use roster::Roster;
