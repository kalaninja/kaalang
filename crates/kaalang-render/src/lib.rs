//! Checks cycle boundaries and compacts a verified kaalang arrangement for the
//! renderers, cuts a staged flow's empty preparation to its start, and places
//! a type diagram's incoming cards in rows and columns under the headers
//! RFC 0010 gives them.
//!
//! Its API may change in any minor release.

pub use self::{
    cards::{CardPath, card_cells, card_headers},
    compact::compact_arrangement,
    stage::keep_header,
    verify::ArrangementVerifier,
};

mod cards;
mod compact;
mod stage;
mod verify;
