//! Resolves relationship targets in the Cargo module trees the editor has read.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use kaalang_compiler::workspace::{Crate, Inclusion, Module, Resolution, Resolver, TypeRef};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsError, prelude::wasm_bindgen};

use crate::utf16;

#[derive(Deserialize)]
struct Snapshot {
    files: Vec<InputFile>,
    crates: Vec<InputCrate>,
}

#[derive(Deserialize)]
struct InputFile {
    uri: String,
    text: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InputCrate {
    package_id: String,
    name: String,
    library: bool,
    #[serde(default)]
    host: bool,
    cfg: String,
    dependencies: Vec<(String, String)>,
    modules: Vec<Mount>,
}

#[derive(Deserialize)]
struct Mount {
    path: String,
    file: usize,
}

#[derive(Deserialize)]
struct FocusPlace {
    owner: usize,
    module: String,
}

/// The original target of one relationship, absent when the source cannot
/// resolve it. Other definite resolutions never contribute to a diagram.
struct Relationship {
    source: TypeRef,
    target: Option<TypeRef>,
    offset: u32,
}

struct Reading {
    file: usize,
    owner: usize,
    module: String,
    repeated: bool,
    relationships: Vec<Relationship>,
}

impl Reading {
    fn inclusion(&self) -> Inclusion<'_> {
        Inclusion {
            owner: self.owner,
            module: &self.module,
            repeated: self.repeated,
        }
    }
}

struct File {
    uri: String,
    mounts: usize,
    types: BTreeMap<u32, Vec<TypeRef>>,
}

struct Declaration {
    reading: usize,
    offset: u32,
}

#[derive(Serialize)]
struct Matches<'a> {
    uri: &'a str,
    owner: usize,
    cfg: &'a str,
    module: &'a str,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    repeated: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    focus: bool,
    offsets: Vec<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unknown: Vec<u32>,
}

/// The resolved relationships in one Cargo workspace snapshot. All persistent
/// positions are UTF-16 offsets, so calls can release the compiler's spans.
#[wasm_bindgen]
pub struct Workspace {
    files: Vec<File>,
    cfgs: Vec<String>,
    readings: Vec<Reading>,
    types: Vec<Vec<Declaration>>,
}

#[wasm_bindgen]
impl Workspace {
    /// Parses the files of each active Cargo target and resolves relationship targets.
    ///
    /// # Errors
    ///
    /// Throws when the snapshot, module source index, or target cfg is invalid.
    #[wasm_bindgen(constructor)]
    pub fn new(snapshot: &str) -> Result<Workspace, JsError> {
        crate::answer(
            serde_json::from_str(snapshot)
                .map_err(|err| err.to_string())
                .and_then(Self::index),
        )
    }

    /// Returns each reading's matching offsets and the unresolved offsets an
    /// editor can confirm by location, in UTF-16. A focus place selects a
    /// crate and module when a declaration belongs to several types. An
    /// ambiguous or absent focus gives JSON `null`.
    ///
    /// # Errors
    ///
    /// Throws when a JSON argument is invalid.
    pub fn references(
        &self,
        uri: &str,
        offset: u32,
        candidates: &str,
        place: Option<String>,
    ) -> Result<String, JsError> {
        let candidates: BTreeSet<String> =
            serde_json::from_str(candidates).map_err(|err| JsError::new(&err.to_string()))?;
        let selected: Option<FocusPlace> = place
            .map(|json| serde_json::from_str(&json))
            .transpose()
            .map_err(|err| JsError::new(&err.to_string()))?;
        let matches = self
            .focus(uri, offset, selected.as_ref())
            .map(|(focus, unique)| self.matches(focus, unique, &candidates));
        Ok(serde_json::to_string(&matches).expect("references serialize"))
    }
}

impl InputCrate {
    fn parse(&self, files: &[InputFile]) -> Result<Crate, String> {
        let mut modules = BTreeMap::new();
        for mount in &self.modules {
            let file = files.get(mount.file).ok_or("missing module source")?;
            modules.insert(
                mount.path.as_str(),
                (file.uri.as_str(), file.text.as_deref()),
            );
        }
        Ok(Crate::from_modules(
            self.package_id.clone(),
            self.name.clone(),
            self.library,
            self.host,
            self.dependencies.clone(),
            self.cfg.parse()?,
            &modules,
        ))
    }
}

impl Workspace {
    fn index(snapshot: Snapshot) -> Result<Self, String> {
        let crates: Vec<_> = snapshot
            .crates
            .iter()
            .map(|target| target.parse(&snapshot.files))
            .collect::<Result<_, _>>()?;
        let resolver = Resolver::new(&crates);
        let mut index = Self {
            files: Vec::new(),
            cfgs: Vec::new(),
            readings: Vec::new(),
            types: crates
                .iter()
                .map(|krate| Vec::with_capacity(krate.types.len()))
                .collect(),
        };
        let mut units = Vec::new();
        for file in snapshot.files {
            let text = file.text.as_deref().unwrap_or("");
            units.push((utf16::Offsets::new(text), text.contains("refer")));
            index.files.push(File {
                uri: file.uri,
                mounts: 0,
                types: BTreeMap::new(),
            });
        }
        for (owner, (target, krate)) in snapshot.crates.into_iter().zip(&crates).enumerate() {
            index.cfgs.push(target.cfg);
            let mut mounts = BTreeMap::new();
            for mount in target.modules {
                let file = &mut index.files[mount.file];
                file.mounts += 1;
                mounts.insert(mount.path.clone(), index.readings.len());
                index.readings.push(Reading {
                    file: mount.file,
                    owner,
                    module: mount.path,
                    repeated: resolver.repeated_file(owner, Path::new(&file.uri)),
                    relationships: Vec::new(),
                });
            }
            for (id, entry) in krate.types.iter().enumerate() {
                let module = &krate.modules[entry.module];
                let reading = *mounts
                    .get(file_module(module))
                    .ok_or("missing file module")?;
                let file = index.readings[reading].file;
                let (offsets, has_relationships) = &units[file];
                let source = TypeRef { krate: owner, id };
                let offset = offsets.at(entry.offset);
                index.files[file]
                    .types
                    .entry(offset)
                    .or_default()
                    .push(source);
                index.types[owner].push(Declaration { reading, offset });
                if !has_relationships {
                    continue;
                }
                let Ok(declaration) = kaalang_compiler::type_declaration(&entry.item) else {
                    continue;
                };
                for relationship in declaration.relationships() {
                    let target = match resolver.resolve(owner, entry.module, &relationship.target) {
                        Resolution::Type(found) => Some(found),
                        Resolution::Unknown(_) => None,
                        _ => continue,
                    };
                    let name = &relationship
                        .target
                        .segments
                        .last()
                        .expect("a target names a type")
                        .ident;
                    index.readings[reading].relationships.push(Relationship {
                        source,
                        target,
                        offset: offsets.at(module.file.offset(name.span())),
                    });
                }
            }
        }
        Ok(index)
    }

    fn same(&self, focus: TypeRef, other: TypeRef) -> bool {
        if focus.krate == other.krate {
            return focus.id == other.id;
        }
        let left = &self.types[focus.krate][focus.id];
        let right = &self.types[other.krate][other.id];
        let a = &self.readings[left.reading];
        let b = &self.readings[right.reading];
        left.offset == right.offset && a.file == b.file && a.inclusion().same_type(b.inclusion())
    }

    fn focus(
        &self,
        uri: &str,
        offset: u32,
        selected: Option<&FocusPlace>,
    ) -> Option<(TypeRef, bool)> {
        let file = self.files.iter().find(|file| file.uri == uri)?;
        let types = file.types.get(&offset)?;
        // Readings are indexed in Cargo target order. Two in one target are
        // distinct. Across targets only a repeated inclusion requires a name match.
        let repeated = types
            .iter()
            .copied()
            .find(|ty| self.readings[self.types[ty.krate][ty.id].reading].repeated);
        let unique = types.windows(2).all(|pair| pair[0].krate != pair[1].krate)
            && repeated.is_none_or(|ty| types.iter().all(|&other| self.same(ty, other)));
        if selected.is_none() && !unique {
            return None;
        }
        let focus = types.iter().copied().find(|ty| {
            let reading = &self.readings[self.types[ty.krate][ty.id].reading];
            selected
                .is_none_or(|place| place.owner == reading.owner && place.module == reading.module)
        })?;
        Some((focus, unique))
    }

    fn matches(
        &self,
        focus: TypeRef,
        unique: bool,
        candidates: &BTreeSet<String>,
    ) -> Vec<Matches<'_>> {
        let selected = self.types[focus.krate][focus.id].reading;
        let mut found = Vec::new();
        for (index, reading) in self.readings.iter().enumerate() {
            let uri = &self.files[reading.file].uri;
            if !candidates.contains(uri) {
                continue;
            }
            let mut result = Matches {
                uri,
                owner: reading.owner,
                cfg: &self.cfgs[reading.owner],
                module: &reading.module,
                repeated: reading.repeated,
                focus: index == selected,
                offsets: Vec::new(),
                unknown: Vec::new(),
            };
            for relationship in &reading.relationships {
                // Only the selected target supplies the focus card's own lines.
                if relationship.source != focus && self.same(focus, relationship.source) {
                    continue;
                }
                match relationship.target {
                    // A language-server location identifies a file and offset, without a Cargo owner.
                    None if unique && self.files[reading.file].mounts == 1 => {
                        result.unknown.push(relationship.offset);
                    }
                    Some(target) if self.same(focus, target) => {
                        result.offsets.push(relationship.offset);
                    }
                    _ => {}
                }
            }
            found.push(result);
        }
        found
    }
}

/// An inline module inherits the reading of its enclosing file.
fn file_module(module: &Module) -> &str {
    module.path[..module.path.len() - module.in_file.len()].trim_end_matches("::")
}
