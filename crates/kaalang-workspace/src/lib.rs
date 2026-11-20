//! Reads a Cargo workspace for kaalang's diagrams: its targets in the order
//! their cards stand, which files a module declaration loads, the module tree
//! each target reads, and how Rust paths resolve across those trees. The
//! compiler reads one source text; this crate knows how texts form crates.
//!
//! Its API may change in any minor release.

pub use self::{
    cargo_target::{CargoTarget, crate_name, target_group},
    modules::{ExternalModule, ModuleDirectory, ModuleFile, module_items},
    walk::{Files, TreeFile, walk},
};

mod cargo_target;
mod modules;
pub mod names;
mod walk;
