//! Shared test material for the kaalang crates: the fixture corpus, the
//! generated stress and cycle shapes, and the statistic every budget is
//! stated over.
//!
//! Each crate bounds the stages it owns, and they all measure the same flows
//! the same way.
//!
//! `kaalang-compiler` dev-depends on this crate while this crate depends on it.
//! Cargo allows that, but the model's own unit tests link a different instance
//! of the model than this crate does, so every item here stays on `syn` and
//! `std` types. Hand back a `SemanticModel` and those tests stop compiling.

pub mod corpus;
pub mod shapes;
pub mod statistics;
pub mod stress;
