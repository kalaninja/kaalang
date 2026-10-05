//! Checks cycle boundaries and compacts a verified kaalang arrangement for the
//! renderers.
//!
//! It serves kaalang's renderers, and its API may change in any minor release.

pub use self::{compact::compact_arrangement, verify::ArrangementVerifier};

mod compact;
mod verify;
