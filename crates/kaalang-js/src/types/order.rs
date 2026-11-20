//! Where a file stands in the Cargo module tree, as the module tree walk gives
//! it, and where each of its types stands.

use std::path::Path;

use kaalang_workspace::{CargoTarget, crate_name, target_group};
use serde::{Deserialize, Serialize};

/// A Cargo target as its cards are ordered and headed.
#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Target {
    package: String,
    group: u8,
    #[serde(rename = "crate")]
    name: String,
    root: String,
}

impl Target {
    /// The target `cargo metadata` lists with `kinds` and `name`, none for a
    /// build script.
    pub(crate) fn of_cargo(
        package: String,
        kinds: &[String],
        name: &str,
        root: String,
    ) -> Option<Self> {
        Some(Self {
            package,
            group: target_group(kinds)?,
            name: crate_name(name),
            root,
        })
    }

    pub(crate) fn cargo_target(&self) -> CargoTarget<'_> {
        CargoTarget {
            package: &self.package,
            group: self.group,
            name: &self.name,
            root: Path::new(&self.root),
        }
    }
}

/// A target, the crate-relative module, and the source positions from the
/// crate root through module declarations to the file, or to a type's name.
#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Place {
    #[serde(flatten)]
    target: Target,
    pub module: String,
    order: Vec<usize>,
}

impl Place {
    pub(crate) fn new(target: Target, module: String, order: Vec<usize>) -> Self {
        Self {
            target,
            module,
            order,
        }
    }

    pub fn krate(&self) -> CargoTarget<'_> {
        self.target.cargo_target()
    }

    /// The card order of RFC 0009 §4.3 around a `focus`, if any.
    pub fn key(&self, focus: Option<&Place>) -> ((bool, bool, CargoTarget<'_>), &[usize]) {
        let krate = self.krate();
        let focus = focus.map(Place::krate);
        (
            krate.order(focus.map(|f| f.package), focus == Some(krate)),
            &self.order,
        )
    }

    /// The place of the type at `path` in this file, its name at `offset`:
    /// inside the inline modules of its path, after what precedes it.
    pub fn of_type(&self, path: &str, offset: u32) -> Self {
        let mut module = self.module.clone();
        if let Some((inline, _)) = path.rsplit_once("::") {
            if !module.is_empty() {
                module.push_str("::");
            }
            module.push_str(inline);
        }
        let mut order = self.order.clone();
        order.push(offset as usize);
        Self {
            module,
            order,
            ..self.clone()
        }
    }
}
