//! The module tree a target compiles, file by file: which file each `mod`
//! declaration loads, as rustc chooses it.

use std::{
    io,
    path::{Path, PathBuf},
};

use crate::{ExternalModule, ModuleDirectory, ModuleFile};

/// What a walk asks of the file system it reads. A walk only passes paths
/// back, so a host whose paths [`Path`] cannot take apart joins them itself.
pub trait Files {
    /// The path of `file`, a candidate starting in the directory of the file
    /// at `parent`.
    fn join(&mut self, parent: &Path, file: &ModuleFile) -> PathBuf {
        let directory = parent.parent().unwrap_or(Path::new(""));
        directory.join(file.parts.iter().collect::<PathBuf>())
    }

    /// The canonical path of the file at `path`, `None` when there is none.
    ///
    /// # Errors
    ///
    /// Why the file system cannot tell.
    fn canonical(&mut self, path: &Path) -> io::Result<Option<PathBuf>>;

    /// The text of the file at `canonical`.
    ///
    /// # Errors
    ///
    /// Why it cannot be read.
    fn read(&mut self, canonical: &Path) -> io::Result<String>;
}

/// A file as the crate root or one `mod` declaration loads it.
pub struct TreeFile<T> {
    /// The path the declaration names: the chosen candidate, else the first.
    pub path: PathBuf,
    /// The file's canonical path, none when no one file is there.
    pub canonical: Option<PathBuf>,
    /// Its module's path from the crate root, empty for the root.
    pub module: String,
    /// Where each module name stands, from the crate root's down to this
    /// file's, each in the file that declares it.
    pub order: Vec<usize>,
    /// What the walk's reader made of the file, or why there is nothing:
    /// [`io::ErrorKind::NotFound`] when no file is there.
    pub content: io::Result<T>,
    /// What each of its external modules loads, in source order; none for a
    /// file a module above it already reads.
    pub children: Vec<Option<TreeFile<T>>>,
}

/// Reads the module tree whose crate root is `root`. `read` reads each file at
/// its canonical path, with the directory its default submodules start in, and
/// lists its external modules.
pub fn walk<F, T>(
    root: PathBuf,
    files: &mut F,
    read: &mut impl FnMut(&mut F, &Path, &ModuleDirectory) -> io::Result<(T, Vec<ExternalModule>)>,
) -> TreeFile<T>
where
    F: Files,
{
    let located = files.canonical(&root);
    let directory = ModuleDirectory::for_file("", true);
    let mut open = Vec::new();
    let root = (root, located, directory);
    load(files, read, &mut open, root, String::new(), Vec::new())
}

type Located = (PathBuf, io::Result<Option<PathBuf>>, ModuleDirectory);

fn load<F: Files, T>(
    files: &mut F,
    read: &mut impl FnMut(&mut F, &Path, &ModuleDirectory) -> io::Result<(T, Vec<ExternalModule>)>,
    open: &mut Vec<PathBuf>,
    (path, located, directory): Located,
    module: String,
    order: Vec<usize>,
) -> TreeFile<T> {
    let (canonical, content) = match located {
        Ok(Some(canonical)) => {
            let content = read(files, &canonical, &directory);
            (Some(canonical), content)
        }
        Ok(None) => (None, Err(io::ErrorKind::NotFound.into())),
        Err(error) => (None, Err(error)),
    };
    let (content, modules) = match content {
        Ok((content, modules)) => (Ok(content), modules),
        Err(error) => (Err(error), Vec::new()),
    };
    let mut file = TreeFile {
        path,
        canonical,
        module,
        order,
        content,
        children: Vec::with_capacity(modules.len()),
    };
    open.extend(file.canonical.clone());
    for declared in modules {
        let located = choose(files, &file.path, &declared);
        let mut order = file.order.clone();
        order.push(declared.offset);
        let module = if file.module.is_empty() {
            declared.module
        } else {
            format!("{}::{}", file.module, declared.module)
        };
        let cycle = matches!(&located.1, Ok(Some(canonical)) if open.contains(canonical));
        let child = (!cycle).then(|| load(files, read, open, located, module, order));
        file.children.push(child);
    }
    if file.canonical.is_some() {
        open.pop();
    }
    file
}

/// The one candidate of `declared` that exists, or the first; two that exist
/// make the module ambiguous, as rustc rejects both `a.rs` and `a/mod.rs`.
fn choose(files: &mut impl Files, parent: &Path, declared: &ExternalModule) -> Located {
    let mut found = Vec::new();
    for file in &declared.files {
        let path = files.join(parent, file);
        let located = files.canonical(&path);
        if !matches!(located, Ok(None)) {
            found.push((path, located, file.directory()));
        }
    }
    let mut found = found.into_iter();
    match (found.next(), found.next()) {
        (Some((path, _, directory)), Some((other, ..))) => {
            let name = declared.module.rsplit("::").next().unwrap_or_default();
            let ambiguous = io::Error::other(format!(
                "module `{name}` is ambiguous: both this file and `{}` exist",
                other.display()
            ));
            (path, Err(ambiguous), directory)
        }
        (Some(chosen), None) => chosen,
        (None, _) => {
            let first = &declared.files[0];
            (files.join(parent, first), Ok(None), first.directory())
        }
    }
}

/// Files held in memory, each at its canonical path.
#[cfg(test)]
pub(crate) struct Memory(pub Vec<(PathBuf, String)>);

#[cfg(test)]
impl Files for Memory {
    fn canonical(&mut self, path: &Path) -> io::Result<Option<PathBuf>> {
        let found = self.0.iter().any(|(file, _)| file == path);
        Ok(found.then(|| path.to_owned()))
    }

    fn read(&mut self, canonical: &Path) -> io::Result<String> {
        let found = self.0.iter().find(|(file, _)| file == canonical);
        found
            .map(|(_, text)| text.clone())
            .ok_or_else(|| io::ErrorKind::NotFound.into())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io,
        path::{Path, PathBuf},
    };

    use kaalang_compiler::{CfgSet, SourceFile};

    use super::{Files, Memory, TreeFile, walk};
    use crate::module_items;

    /// The tree from the first of `files`, each file's text held in memory.
    fn tree(files: &[(&str, &str)]) -> TreeFile<()> {
        let held = files
            .iter()
            .map(|(path, text)| (PathBuf::from(path), (*text).to_owned()));
        let mut memory = Memory(held.collect());
        walk(
            PathBuf::from(files[0].0),
            &mut memory,
            &mut |files, canonical, directory| {
                let text = files.read(canonical)?;
                let file = SourceFile::parse(&text, CfgSet::EMPTY).map_err(io::Error::other)?;
                Ok(((), module_items(&file, directory).0))
            },
        )
    }

    /// Each file the tree reads, in walk order, by its module, path, and order.
    fn files(file: &TreeFile<()>) -> Vec<(&str, &Path, &[usize])> {
        let mut listed = vec![(
            file.module.as_str(),
            file.path.as_path(),
            file.order.as_slice(),
        )];
        for child in file.children.iter().flatten() {
            listed.extend(files(child));
        }
        listed
    }

    #[test]
    fn each_inclusion_of_a_file_searches_its_own_directory() {
        let root = "mod shared; #[path = \"shared.rs\"] mod alias;";
        let tree = tree(&[
            ("/src/lib.rs", root),
            ("/src/shared.rs", "mod child;"),
            ("/src/shared/child.rs", ""),
            ("/src/child.rs", ""),
        ]);
        let alias = root.find("alias").unwrap();
        assert_eq!(
            files(&tree),
            [
                ("", Path::new("/src/lib.rs"), &[][..]),
                ("shared", Path::new("/src/shared.rs"), &[4]),
                ("shared::child", Path::new("/src/shared/child.rs"), &[4, 4]),
                ("alias", Path::new("/src/shared.rs"), &[alias]),
                ("alias::child", Path::new("/src/child.rs"), &[alias, 4]),
            ]
        );
    }

    #[test]
    fn a_module_with_both_files_is_ambiguous() {
        let tree = tree(&[
            ("/src/lib.rs", "mod child;"),
            ("/src/child.rs", ""),
            ("/src/child/mod.rs", ""),
        ]);
        let child = tree.children[0].as_ref().unwrap();
        assert_eq!(child.canonical, None);
        let error = child.content.as_ref().err().unwrap().to_string();
        assert!(error.contains("ambiguous"), "{error}");
    }

    #[test]
    fn a_file_open_above_is_not_read_again_and_a_missing_one_is_not_found() {
        let tree = tree(&[(
            "/src/lib.rs",
            "#[path = \"lib.rs\"] mod again; mod missing;",
        )]);
        assert!(tree.children[0].is_none());
        let missing = tree.children[1].as_ref().unwrap();
        assert_eq!(missing.path, Path::new("/src/missing.rs"));
        let error = missing.content.as_ref().err().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn an_unparsable_file_is_found_with_its_error() {
        let tree = tree(&[("/src/lib.rs", "mod broken;"), ("/src/broken.rs", "fn (")]);
        let broken = tree.children[0].as_ref().unwrap();
        assert_eq!(
            broken.canonical.as_deref(),
            Some(Path::new("/src/broken.rs"))
        );
        assert_eq!(
            broken.content.as_ref().err().unwrap().kind(),
            io::ErrorKind::Other
        );
    }
}
