use syn::{Attribute, Ident, ImplItem, Item, Meta, Path, TraitItem, UseTree};

use crate::attribute::{bare_kaa, kaa_name, qualified_kaa};

/// A qualified `#[kaalang::kaa]` can coexist with a bare `#[kaa]` imported
/// from another macro. Read the active import in the declaring module.
pub(super) fn distinguish_imports(file: &mut syn::File) {
    module(&mut file.items, &mut Vec::new());
}

/// Each ancestor records whether its own import names another macro.
fn module(items: &mut [Item], ancestors: &mut Vec<bool>) {
    let foreign = items
        .iter()
        .find_map(|item| match item {
            Item::Use(import) => imported_kaa(&import.tree, None).map(|mut path| {
                path.leading_colon = import.leading_colon;
                path
            }),
            _ => None,
        })
        .filter(|path| is_foreign(path, ancestors));

    ancestors.push(foreign.is_some());
    for item in items {
        if let Item::Mod(child) = item {
            if let Some((_, inner)) = &mut child.content {
                module(inner, ancestors);
            }
            continue;
        }
        let Some(path) = &foreign else { continue };
        match item {
            Item::Impl(block) => {
                for member in &mut block.items {
                    if let ImplItem::Fn(method) = member {
                        qualify(&mut method.attrs, path);
                    }
                }
            }
            Item::Trait(block) => {
                for member in &mut block.items {
                    if let TraitItem::Fn(method) = member {
                        qualify(&mut method.attrs, path);
                    }
                }
            }
            item => {
                if let Some(attrs) = crate::configure::attributes(item) {
                    qualify(attrs, path);
                }
            }
        }
    }
    ancestors.pop();
}

fn qualify(attrs: &mut [Attribute], path: &Path) {
    if !attrs.iter().any(|attr| qualified_kaa(attr.path())) {
        return;
    }
    for attr in attrs.iter_mut().filter(|attr| bare_kaa(attr.path())) {
        match &mut attr.meta {
            Meta::Path(name) => name.clone_from(path),
            Meta::List(list) => list.path.clone_from(path),
            Meta::NameValue(value) => value.path.clone_from(path),
        }
    }
}

fn is_foreign(path: &Path, ancestors: &[bool]) -> bool {
    if qualified_kaa(path) {
        return false;
    }
    let mut parts = path.segments.iter();
    if path.leading_colon.is_none() && parts.next_back().is_some_and(|p| kaa_name(&p.ident)) {
        if parts.len() == 1 && path.segments[0].ident == "crate" {
            return ancestors.first() == Some(&true);
        }
        let levels = parts.len();
        if parts.all(|part| part.ident == "super") {
            return ancestors
                .len()
                .checked_sub(levels)
                .and_then(|i| ancestors.get(i))
                == Some(&true);
        }
    }
    true
}

fn imported_kaa(tree: &UseTree, parent: Option<&Ident>) -> Option<Path> {
    match tree {
        UseTree::Path(path) => {
            let mut imported = imported_kaa(&path.tree, Some(&path.ident))?;
            if imported.is_ident("self") {
                return Some(path.ident.clone().into());
            }
            imported.segments.insert(0, path.ident.clone().into());
            Some(imported)
        }
        UseTree::Group(group) => group
            .items
            .iter()
            .find_map(|tree| imported_kaa(tree, parent)),
        UseTree::Name(name)
            if kaa_name(&name.ident) || name.ident == "self" && parent.is_some_and(kaa_name) =>
        {
            Some(name.ident.clone().into())
        }
        UseTree::Rename(name) if kaa_name(&name.rename) => Some(name.ident.clone().into()),
        _ => None,
    }
}
