//! The compilation pipeline, one module per phase, in the order `expand` runs them.

pub(crate) mod analyze;
pub(crate) mod codegen;
pub(crate) mod parse;
pub(crate) mod resolve;
