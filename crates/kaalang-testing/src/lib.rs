//! Shared fixture corpus, stress and cycle generators, and performance statistics.
//!
//! The dev-dependency cycle with `kaalang-compiler` gives its unit tests a
//! separate compiler-crate instance. APIs use `syn`/`std` types to avoid mismatches.

pub mod corpus;
pub mod shapes;
pub mod statistics;
pub mod stress;
