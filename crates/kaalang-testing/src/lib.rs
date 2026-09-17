//! Shared fixture corpus, generated probes, cycle shapes, and performance harness.
//!
//! The dev-dependency cycle with `kaalang-compiler` gives its unit tests a
//! separate compiler-crate instance. APIs use `syn`/`std` types to avoid mismatches.

pub mod corpus;
pub mod performance;
pub mod probes;
pub mod shapes;
