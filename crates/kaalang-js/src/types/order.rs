//! Where a file stands in the Cargo module tree, as the editor reports it, and
//! where each of its types stands.

use std::path::Path;

use kaalang_workspace::CargoTarget;
use serde::Deserialize;

/// A Cargo target, the crate-relative module, and the source positions from
/// the crate root through module declarations to the file, or to a type's name.
#[derive(Clone, Deserialize)]
pub(super) struct Place {
    package: String,
    group: u8,
    #[serde(rename = "crate")]
    name: String,
    root: String,
    pub module: String,
    order: Vec<u32>,
}

impl Place {
    pub fn krate(&self) -> CargoTarget<'_> {
        CargoTarget {
            package: &self.package,
            group: self.group,
            name: &self.name,
            root: Path::new(&self.root),
        }
    }

    pub fn key(&self, focus: Option<&Place>) -> ((bool, bool, CargoTarget<'_>), &[u32]) {
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
        order.push(offset);
        Self {
            module,
            order,
            ..self.clone()
        }
    }
}
