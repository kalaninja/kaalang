//! Where a Rust file declares its external modules, and which files a host
//! looks for at each declaration.

use kaalang_compiler::{SourceFile, type_name};
use syn::{Expr, ExprLit, Item, ItemMod, Lit, Meta, ext::IdentExt};

/// A file's external module, including any inline modules around it.
#[derive(Debug, PartialEq, Eq)]
pub struct ExternalModule {
    /// The module path within this file, with raw identifiers as authored.
    pub module: String,
    /// The first byte of the module name in the original source.
    pub offset: usize,
    /// The explicit file or the default candidates, in lookup order.
    pub files: Vec<ModuleFile>,
}

/// One source file a module declaration may load.
#[derive(Debug, PartialEq, Eq)]
pub struct ModuleFile {
    /// Parts to join, starting in the declaring file's parent directory.
    /// A literal `#[path]` remains one part for the host's filesystem to read.
    pub parts: Vec<String>,
    /// Whether this file's default submodules start in its parent directory.
    pub owns_directory: bool,
}

/// The directory Rust searches from a file or an inline module within it.
pub struct ModuleDirectory {
    parts: Vec<String>,
    /// At file scope an explicit path starts in the file's parent directory.
    file_scope: bool,
}

impl ModuleDirectory {
    /// A crate root, a `mod.rs`, or an explicit `#[path]` owns its directory.
    #[must_use]
    pub fn for_file(file_stem: &str, owns_directory: bool) -> Self {
        Self {
            parts: if owns_directory || file_stem.is_empty() {
                Vec::new()
            } else {
                vec![file_stem.to_owned()]
            },
            file_scope: true,
        }
    }

    /// The directory an inline module declares for its children.
    #[must_use]
    pub fn inline(&self, declaration: &ItemMod) -> Self {
        let (base, path) = self.base(declaration);
        let mut parts = base.to_vec();
        parts.push(path.unwrap_or_else(|| declaration.ident.unraw().to_string()));
        Self {
            parts,
            file_scope: false,
        }
    }

    /// The candidates for an out-of-line module, in lookup order.
    #[must_use]
    pub fn files(&self, declaration: &ItemMod) -> Vec<ModuleFile> {
        let (base, path) = self.base(declaration);
        let mut parts = base.to_vec();
        if let Some(path) = path {
            parts.push(path);
            return vec![ModuleFile {
                parts,
                owns_directory: true,
            }];
        }
        let name = declaration.ident.unraw().to_string();
        let mut direct = parts.clone();
        direct.push(format!("{name}.rs"));
        parts.extend([name, "mod.rs".to_owned()]);
        vec![
            ModuleFile {
                parts: direct,
                owns_directory: false,
            },
            ModuleFile {
                parts,
                owns_directory: true,
            },
        ]
    }

    /// The directory the declaration starts in, and its explicit path if present.
    fn base(&self, declaration: &ItemMod) -> (&[String], Option<String>) {
        let path = declaration.attrs.iter().find_map(|attribute| {
            if let Meta::NameValue(pair) = &attribute.meta
                && pair.path.is_ident("path")
                && let Expr::Lit(ExprLit {
                    lit: Lit::Str(path),
                    ..
                }) = &pair.value
            {
                Some(path.value())
            } else {
                None
            }
        });
        let base = if path.is_some() && self.file_scope {
            &[]
        } else {
            self.parts.as_slice()
        };
        (base, path)
    }
}

/// Every external module and the name offsets of every struct, enum, union
/// and alias `file` compiles, in declaration order. The same file can own its
/// directory in one inclusion and have its submodules under `file_stem/` in
/// another.
#[must_use]
pub fn module_items(
    file: &SourceFile<'_>,
    file_stem: &str,
    owns_directory: bool,
) -> (Vec<ExternalModule>, Vec<usize>) {
    fn collect(
        file: &SourceFile<'_>,
        items: &[Item],
        prefix: &str,
        directory: &ModuleDirectory,
        modules: &mut Vec<ExternalModule>,
        types: &mut Vec<usize>,
    ) {
        for item in items {
            let Item::Mod(declaration) = item else {
                if let Some(name) = type_name(item) {
                    types.push(file.range(name.span()).start);
                }
                continue;
            };
            let module = format!("{prefix}{}", declaration.ident);
            if let Some((_, nested)) = &declaration.content {
                collect(
                    file,
                    nested,
                    &format!("{module}::"),
                    &directory.inline(declaration),
                    modules,
                    types,
                );
            } else {
                modules.push(ExternalModule {
                    module,
                    offset: file.range(declaration.ident.span()).start,
                    files: directory.files(declaration),
                });
            }
        }
    }
    let mut modules = Vec::new();
    let mut types = Vec::new();
    collect(
        file,
        file.items(),
        "",
        &ModuleDirectory::for_file(file_stem, owns_directory),
        &mut modules,
        &mut types,
    );
    (modules, types)
}

#[cfg(test)]
mod tests {
    use kaalang_compiler::{CfgSet, SourceFile};

    use super::{ModuleFile, module_items};

    fn candidate(parts: &[&str], owns_directory: bool) -> ModuleFile {
        ModuleFile {
            parts: parts.iter().map(|part| (*part).to_owned()).collect(),
            owns_directory,
        }
    }

    #[test]
    fn two_inclusions_of_one_file_search_different_directories() {
        let root = "mod shared; #[path = \"shared.rs\"] mod copy;";
        let file = SourceFile::parse(root, CfgSet::EMPTY).unwrap();
        let (modules, _) = module_items(&file, "lib", true);
        assert_eq!(modules[0].module, "shared");
        assert_eq!(
            modules[0].files,
            [
                candidate(&["shared.rs"], false),
                candidate(&["shared", "mod.rs"], true),
            ]
        );
        assert_eq!(modules[1].module, "copy");
        assert_eq!(modules[1].files, [candidate(&["shared.rs"], true)]);

        let file = SourceFile::parse("mod r#match;", CfgSet::EMPTY).unwrap();
        let children = |owned| module_items(&file, "shared", owned).0.remove(0);
        let direct = children(modules[0].files[0].owns_directory);
        let copied = children(modules[1].files[0].owns_directory);
        assert_eq!(direct.module, "r#match");
        assert_eq!(
            direct.files,
            [
                candidate(&["shared", "match.rs"], false),
                candidate(&["shared", "match", "mod.rs"], true),
            ]
        );
        assert_eq!(
            copied.files,
            [
                candidate(&["match.rs"], false),
                candidate(&["match", "mod.rs"], true),
            ]
        );
    }

    #[test]
    fn inline_paths_and_type_names_follow_the_configured_source() {
        let source = concat!(
            "\u{feff}#!/usr/bin/env rust-script\n// α\n",
            r#"
                #[cfg_attr(unix, path = "elsewhere")] mod r#type {
                    struct Present;
                    #[cfg(windows)] struct Absent;
                    mod inner {
                        enum Color {}
                        #[cfg_attr(unix, path = r"C:\source\chosen.rs")] mod explicit;
                        mod r#match;
                        #[cfg(windows)] mod absent;
                        type Alias = ();
                    }
                }
                union Other { field: u8 }
                fn local() { struct Hidden; }
            "#,
        );
        let file = SourceFile::parse(source, &"unix".parse().unwrap()).unwrap();
        let (modules, types) = module_items(&file, "shared", false);
        assert_eq!(modules.len(), 2);
        assert_eq!(modules[0].module, "r#type::inner::explicit");
        assert_eq!(modules[0].offset, source.find("explicit;").unwrap());
        assert_eq!(
            modules[0].files,
            [candidate(
                &["elsewhere", "inner", r"C:\source\chosen.rs"],
                true
            )]
        );
        assert_eq!(modules[1].module, "r#type::inner::r#match");
        assert_eq!(modules[1].offset, source.find("r#match;").unwrap());
        assert_eq!(
            modules[1].files,
            [
                candidate(&["elsewhere", "inner", "match.rs"], false),
                candidate(&["elsewhere", "inner", "match", "mod.rs"], true),
            ]
        );
        assert_eq!(
            types,
            ["Present", "Color", "Alias", "Other"].map(|name| source.find(name).unwrap())
        );
    }
}
