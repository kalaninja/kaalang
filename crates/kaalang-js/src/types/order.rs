//! A declaration's places in the Cargo module tree, as reported by the editor.

use std::{collections::BTreeMap, path::Path};

use kaalang_compiler::workspace::CardCrate;
use serde::Deserialize;

pub(super) type Places = BTreeMap<String, Vec<Place>>;

#[derive(Deserialize)]
pub(super) struct Place {
    package: String,
    group: u8,
    #[serde(rename = "crate")]
    name: String,
    root: String,
    pub module: String,
    /// From the crate root, the source positions of the enclosing module
    /// declarations, then the type's name.
    order: Vec<u32>,
}

impl Place {
    pub fn krate(&self) -> CardCrate<'_> {
        CardCrate {
            package: &self.package,
            group: self.group,
            name: &self.name,
            root: Path::new(&self.root),
        }
    }

    pub fn key(&self, focus: Option<&Place>) -> ((bool, bool, CardCrate<'_>), &[u32]) {
        let krate = self.krate();
        let focus = focus.map(Place::krate);
        (
            krate.order(focus.map(|f| f.package), focus == Some(krate)),
            &self.order,
        )
    }

    /// A shared declaration appears once, at its first place in diagram order.
    /// For the focus the hover identifies which module the editor is showing.
    pub fn first<'a>(
        places: &'a Places,
        path: &str,
        focus: Option<&Place>,
        hover: Option<&str>,
    ) -> Option<&'a Place> {
        places.get(path)?.iter().min_by_key(|place| {
            let matches_hover = hover.is_some_and(|module| {
                let (krate, inner) = module.split_once("::").unwrap_or((module, ""));
                krate == place.name && inner == place.module
            });
            (!matches_hover, place.key(focus))
        })
    }
}
