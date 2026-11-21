//! The module trees of a workspace's targets, read through the editor host's
//! file system, with what each file declares kept until the host forgets it.

use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    io,
    path::{Path, PathBuf},
    rc::Rc,
};

use kaalang_compiler::{CfgSet, SourceFile};
use kaalang_workspace::{
    ExternalModule, Files, ModuleDirectory, ModuleFile, TreeFile, module_items, walk,
};
use serde::{Deserialize, Serialize, Serializer};
use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

use crate::{
    types::{Place, Target},
    utf16,
};

#[wasm_bindgen]
extern "C" {
    /// The host's file system, as [`Files`] asks it, in the host's own paths.
    pub type ModuleHost;

    #[wasm_bindgen(method)]
    fn join(this: &ModuleHost, parent: &str, parts: Vec<String>) -> String;

    /// `undefined` when no file is there.
    #[wasm_bindgen(method, catch)]
    fn canonical(this: &ModuleHost, path: &str) -> Result<Option<String>, JsValue>;

    #[wasm_bindgen(method, catch)]
    fn read(this: &ModuleHost, canonical: &str) -> Result<String, JsValue>;
}

/// The host, with the paths it joined: a join depends on nothing else, and
/// asking it again would cost more than looking it up.
struct Host<'a> {
    host: &'a ModuleHost,
    joined: &'a mut HashMap<String, HashMap<Vec<String>, String>>,
}

impl Files for Host<'_> {
    fn join(&mut self, parent: &Path, file: &ModuleFile) -> PathBuf {
        let joined = self.joined.entry(text(parent).to_owned()).or_default();
        if let Some(path) = joined.get(&file.parts) {
            return path.into();
        }
        let path = self.host.join(text(parent), file.parts.clone());
        joined.insert(file.parts.clone(), path.clone());
        path.into()
    }

    fn canonical(&mut self, path: &Path) -> io::Result<Option<PathBuf>> {
        let found = self
            .host
            .canonical(text(path))
            .map_err(|error| host_error(&error))?;
        Ok(found.map(PathBuf::from))
    }

    fn read(&mut self, canonical: &Path) -> io::Result<String> {
        self.host
            .read(text(canonical))
            .map_err(|error| host_error(&error))
    }
}

/// Every path came from the host as a string.
fn text(path: &Path) -> &str {
    path.to_str().expect("the host's paths are strings")
}

fn host_error(error: &JsValue) -> io::Error {
    io::Error::other(format!("{error:?}"))
}

/// A target as `cargo metadata` lists it, with the set its crate is read for.
#[derive(Deserialize)]
struct Root {
    package: String,
    kind: Vec<String>,
    name: String,
    root: String,
    cfg: String,
}

/// What a file declares for a set and a directory, in UTF-16 code units.
struct Declared {
    modules: Vec<ExternalModule>,
    types: Vec<u32>,
    /// Whether the text may hold a relationship: every one names `refer`.
    refers: bool,
}

#[derive(Default)]
struct Cache {
    /// By canonical path, then by set and directory.
    declared: HashMap<String, Vec<(String, ModuleDirectory, Rc<Declared>)>>,
    joined: HashMap<String, HashMap<Vec<String>, String>>,
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::default();
}

/// Forgets what the trees read of the file at `canonical`, or of every file.
pub(crate) fn forget(canonical: Option<String>) {
    CACHE.with_borrow_mut(|cache| match canonical {
        Some(file) => {
            cache.declared.remove(&file);
        }
        None => *cache = Cache::default(),
    });
}

/// One place of a file, with the root's position in the roots and its types'
/// name offsets when it could be read.
#[derive(Serialize)]
struct Reading {
    target: usize,
    place: Place,
    #[serde(rename = "types", serialize_with = "types")]
    declared: Option<Rc<Declared>>,
}

#[allow(clippy::ref_option)] // The signature `serialize_with` takes.
fn types<S: Serializer>(declared: &Option<Rc<Declared>>, serializer: S) -> Result<S::Ok, S::Error> {
    declared
        .as_ref()
        .map(|declared| &declared.types)
        .serialize(serializer)
}

#[derive(Serialize)]
struct Trees {
    files: BTreeMap<String, Vec<Reading>>,
    referring: Vec<String>,
    complete: bool,
}

pub(crate) fn tree(roots: &str, host: &ModuleHost) -> Result<String, String> {
    let roots: Vec<Root> = serde_json::from_str(roots).map_err(|error| error.to_string())?;
    let mut trees = Trees {
        files: BTreeMap::new(),
        referring: Vec::new(),
        complete: true,
    };
    for (index, root) in roots.iter().enumerate() {
        let (package, name, path) = (root.package.clone(), &root.name, root.root.clone());
        let Some(target) = Target::of_cargo(package, &root.kind, name, path) else {
            continue;
        };
        let cfg: CfgSet = root.cfg.parse()?;
        let tree = CACHE.with_borrow_mut(|Cache { declared, joined }| {
            let mut files = Host { host, joined };
            walk(
                root.root.clone().into(),
                &mut files,
                &mut |files, canonical, directory| {
                    let kept = declared.entry(text(canonical).to_owned()).or_default();
                    let found = kept
                        .iter()
                        .find(|(set, at, _)| *set == root.cfg && at == directory);
                    let found = if let Some((.., found)) = found {
                        Rc::clone(found)
                    } else {
                        let text = files.read(canonical)?;
                        let found = Rc::new(declare(&text, &cfg, directory)?);
                        kept.push((root.cfg.clone(), directory.clone(), Rc::clone(&found)));
                        found
                    };
                    Ok((Rc::clone(&found), found.modules.clone()))
                },
            )
        });
        collect(tree, index, &target, &mut trees);
    }
    for (file, readings) in &mut trees.files {
        if readings.iter().any(|reading| {
            reading
                .declared
                .as_ref()
                .is_some_and(|declared| declared.refers)
        }) {
            trees.referring.push(file.clone());
        }
        readings.sort_by(|left, right| left.place.key(None).cmp(&right.place.key(None)));
    }
    serde_json::to_string(&trees).map_err(|error| error.to_string())
}

fn declare(text: &str, cfg: &CfgSet, directory: &ModuleDirectory) -> io::Result<Declared> {
    let file = SourceFile::parse(text, cfg).map_err(io::Error::other)?;
    let (modules, types) = module_items(&file, directory);
    let offsets = utf16::Offsets::new(text);
    let modules = modules
        .into_iter()
        .map(|module| ExternalModule {
            offset: offsets.at(module.offset) as usize,
            ..module
        })
        .collect();
    let types = types.into_iter().map(|byte| offsets.at(byte)).collect();
    let word =
        |byte: Option<&u8>| byte.is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_');
    let refers = text.match_indices("refer").any(|(at, _)| {
        !word(text.as_bytes().get(at.wrapping_sub(1))) && !word(text.as_bytes().get(at + 5))
    });
    Ok(Declared {
        modules,
        types,
        refers,
    })
}

/// Adds each file of `tree` with its place in the root at `index`. A file that
/// exists but cannot be read or parsed, or an ambiguous module, may hide more
/// modules: the trees are then incomplete.
fn collect(tree: TreeFile<Rc<Declared>>, index: usize, target: &Target, trees: &mut Trees) {
    let declared = match tree.content {
        Ok(declared) => Some(declared),
        Err(error) => {
            trees.complete &= error.kind() == io::ErrorKind::NotFound;
            None
        }
    };
    if let Some(canonical) = tree.canonical {
        let file = canonical
            .into_os_string()
            .into_string()
            .expect("the host's paths are strings");
        trees.files.entry(file).or_default().push(Reading {
            target: index,
            place: Place::new(target.clone(), tree.module, tree.order),
            declared,
        });
    }
    for child in tree.children.into_iter().flatten() {
        collect(child, index, target, trees);
    }
}
