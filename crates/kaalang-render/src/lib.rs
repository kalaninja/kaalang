//! Checks cycle boundaries and compacts a verified kaalang arrangement for the
//! renderers, and places a type diagram's incoming cards in rows and columns.
//!
//! Its API may change in any minor release.

pub use self::{cards::card_cells, compact::compact_arrangement, verify::ArrangementVerifier};

mod cards;
mod compact;
mod verify;
