//! Checks cycle boundaries and compacts a verified kaalang arrangement for the
//! renderers.
//!
//! Its API may change in any minor release.

pub use self::{compact::compact_arrangement, verify::ArrangementVerifier};

mod compact;
mod verify;
