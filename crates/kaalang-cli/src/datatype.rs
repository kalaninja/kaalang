//! Draws a type diagram: finds the focus type in the workspace, resolves every
//! relationship there, and renders the focus under the types that refer to it.

use std::{
    collections::{BTreeSet, HashMap, hash_map::Entry},
    fmt::Write as _,
};

use kaalang_compiler::{SourceError, SourceFile, SourceType, TypeDeclaration};
use kaalang_svg::{CardPath, TypeCard, TypeOptions};
use kaalang_workspace::{
    CargoTarget,
    names::{Def, Resolution, Resolver, TypeRef},
};
use syn::spanned::Spanned;

use crate::workspace::{self, Crate};

/// Draws the type whose path from its crate root is `path`, taken from
/// `package` when given. A file that cannot be read or parsed, or a type whose
/// annotations do not parse, is skipped and listed, as an error when the type
/// is not found and as a warning when it is.
pub(super) fn draw(
    crates: &[Crate],
    path: &str,
    package: Option<&str>,
    options: TypeOptions,
) -> Result<String, String> {
    let resolver = Resolver::new(crates.iter().map(|krate| &krate.names));
    let (declarations, unreadable) = declarations(crates);
    let skipped = workspace::skipped(crates);
    let focus = focus(crates, &resolver, path, package, &skipped).map_err(|mut message| {
        if !unreadable.is_empty() {
            message = listed(message + "\nthese types could not be read:", &unreadable);
        }
        message
    })?;
    workspace::warn_skipped(path, &skipped);
    warn("warning: these types could not be read:", &unreadable);
    let drawn = relationships(crates, &resolver, &declarations, focus)?;
    render(crates, focus, &drawn, options)
}

/// Each crate's type declarations, parsed with their annotations, and the
/// location and error of each that does not parse.
fn declarations(crates: &[Crate]) -> (Vec<Vec<Option<TypeDeclaration>>>, BTreeSet<String>) {
    let mut unreadable = BTreeSet::new();
    let declarations = crates
        .iter()
        .map(|krate| {
            krate
                .types
                .iter()
                .map(|entry| {
                    kaalang_compiler::type_declaration(&entry.item)
                        .map_err(|error| {
                            let at = location(krate, entry.module, error.span());
                            unreadable.insert(format!("{at}: {error}"));
                        })
                        .ok()
                })
                .collect()
        })
        .collect();
    (declarations, unreadable)
}

/// Resolves every relationship in the workspace and returns each type with the
/// positions of its relationships to the focus. A target that is not a type of
/// the workspace fails, listing every such relationship; one the source alone
/// cannot resolve is not drawn and is listed as a warning.
fn relationships(
    crates: &[Crate],
    resolver: &Resolver,
    declarations: &[Vec<Option<TypeDeclaration>>],
    focus: TypeRef,
) -> Result<Vec<(TypeRef, Vec<usize>)>, String> {
    let mut errors = BTreeSet::new();
    let mut unknown = BTreeSet::new();
    let mut drawn = Vec::new();
    for (index, krate) in crates.iter().enumerate() {
        for (id, entry) in krate.types.iter().enumerate() {
            let Some(declaration) = &declarations[index][id] else {
                continue;
            };
            let mut positions = Vec::new();
            for (position, relationship) in declaration.relationships().iter().enumerate() {
                let target = &relationship.target;
                let at = location(krate, entry.module, target.span());
                let written = written(target);
                match resolver.resolve(index, entry.module, target) {
                    Resolution::Type(found) if found == focus => {
                        positions.push(position);
                    }
                    Resolution::Type(_) => {}
                    Resolution::NotAType(what) => {
                        errors.insert(format!("{at}: `{written}` is {what}, not a type"));
                    }
                    Resolution::Outside => {
                        errors.insert(format!("{at}: `{written}` is not a type of this workspace"));
                    }
                    Resolution::Ambiguous => {
                        errors.insert(format!(
                            "{at}: `{written}` is ambiguous through glob imports"
                        ));
                    }
                    Resolution::Unknown(reason) => {
                        unknown.insert(format!("{at}: `{written}`: {reason}"));
                    }
                }
            }
            if !positions.is_empty() {
                drawn.push((TypeRef { krate: index, id }, positions));
            }
        }
    }
    if !errors.is_empty() {
        let heading =
            "relationships must refer to a struct, enum, union, or type alias of this workspace:";
        return Err(listed(heading.to_owned(), &errors));
    }
    warn(
        "warning: these relationships are not drawn, because their targets could not be resolved:",
        &unknown,
    );
    Ok(drawn)
}

/// Renders the focus under its incoming types in diagram order. A file that
/// several modules include gives each of them its own types, as rustc compiles
/// them. Each package's library and binaries, its tests, its examples, and its
/// benches start a new row.
fn render(
    crates: &[Crate],
    focus: TypeRef,
    drawn: &[(TypeRef, Vec<usize>)],
    options: TypeOptions,
) -> Result<String, String> {
    let drawn: HashMap<TypeRef, &[usize]> = drawn
        .iter()
        .map(|(ty, positions)| (*ty, positions.as_slice()))
        .collect();
    let own = drawn.get(&focus).copied().unwrap_or_default();
    let mut incoming: Vec<(TypeRef, &[usize])> = Vec::new();
    for krate in order(crates, focus.krate) {
        for id in 0..crates[krate].types.len() {
            let ty = TypeRef { krate, id };
            if ty != focus
                && let Some(positions) = drawn.get(&ty)
            {
                incoming.push((ty, positions));
            }
        }
    }
    let file = |source: TypeRef| {
        let entry = &crates[source.krate].types[source.id];
        &crates[source.krate].modules[entry.module].file
    };

    let (focus_header, headers) = headers(crates, focus, &incoming);
    let mut files = HashMap::new();
    let mut read = |source: TypeRef| -> Result<SourceType, String> {
        let krate = &crates[source.krate];
        let file = file(source);
        let located = |error| format!("{}: {error}", file.path.display());
        let parsed = match files.entry((&file.canonical, &krate.cfg)) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                entry.insert(SourceFile::parse(&file.source, &krate.cfg).map_err(located)?)
            }
        };
        parsed.ty(&krate.types[source.id].in_file).map_err(located)
    };
    let focus_type = read(focus)?;
    let incoming_types = incoming
        .iter()
        .map(|(source, _)| read(*source))
        .collect::<Result<Vec<_>, _>>()?;
    let focus_card = TypeCard {
        ty: &focus_type,
        header: &focus_header,
        relationships: own,
    };
    let incoming_cards: Vec<TypeCard> = incoming
        .iter()
        .zip(&incoming_types)
        .zip(&headers)
        .map(|(((_, positions), ty), header)| TypeCard {
            ty,
            header,
            relationships: positions,
        })
        .collect();
    let groups: Vec<_> = incoming
        .iter()
        .map(|(ty, _)| crates[ty.krate].cargo_target().group_key())
        .collect();
    let rendered = kaalang_svg::render_type(&focus_card, &incoming_cards, &groups, options);
    let (svg, _) = rendered.map_err(|error| {
        let culprit = error.card.checked_sub(1).map_or(focus, |i| incoming[i].0);
        format!("{}: {error}", file(culprit).path.display())
    })?;
    Ok(svg)
}

/// The one type that `path` resolves to from a crate root, from `package` when
/// given, or the first crate's when the crates that resolve it share its file.
fn focus(
    crates: &[Crate],
    resolver: &Resolver,
    path: &str,
    package: Option<&str>,
    skipped: &BTreeSet<String>,
) -> Result<TypeRef, String> {
    let target = syn::parse_str::<syn::Path>(path).ok();
    let searched: Vec<usize> = order(crates, usize::MAX)
        .into_iter()
        .filter(|&krate| {
            !crates[krate].dependency_only
                && package.is_none_or(|package| crates[krate].package == package)
        })
        .collect();
    let mut found: Vec<TypeRef> = Vec::new();
    for &krate in &searched {
        match target
            .as_ref()
            .map(|target| resolver.find(krate, 0, target))
        {
            Some(Def::Type(declared)) if !found.contains(&declared) => {
                found.push(declared);
            }
            Some(Def::Ambiguous(_)) => {
                return Err(format!(
                    "the type path `{path}` is ambiguous through glob imports in {}",
                    crates[krate].label()
                ));
            }
            _ => {}
        }
    }
    let one_declaration = |candidates: &[TypeRef]| {
        candidates.iter().enumerate().all(|(i, ty)| {
            candidates[..i]
                .iter()
                .all(|previous| resolver.place(*ty) == resolver.place(*previous))
        })
    };
    match found.as_slice() {
        [] => {
            let message = SourceError::TypeNotFound(path.to_owned()).to_string();
            Err(workspace::with_skipped(message, skipped))
        }
        [first, ..] if one_declaration(&found) => Ok(*first),
        _ => {
            let package = |ty: &TypeRef| &crates[ty.krate].package;
            found.sort_by_key(package);
            let listed: Vec<(&Crate, String)> = found
                .chunk_by(|left, right| package(left) == package(right))
                .flat_map(|group| {
                    if one_declaration(group) {
                        &group[..1]
                    } else {
                        group
                    }
                })
                .map(|declared| {
                    let krate = &crates[declared.krate];
                    let module = &krate.modules[krate.types[declared.id].module];
                    (krate, module.file.path.display().to_string())
                })
                .collect();
            Err(workspace::ambiguous(
                &format!("multiple types have the path `{path}`"),
                &listed,
            ))
        }
    }
}

/// The crates in diagram order: `first`, the rest of its package, then the
/// other packages by name. Within a package, its library and binaries come
/// first, then its tests, examples, and benches, each by name and then by the
/// path of its root.
fn order(crates: &[Crate], first: usize) -> Vec<usize> {
    let package = crates.get(first).map(|krate| krate.package.as_str());
    let mut order: Vec<usize> = (0..crates.len()).collect();
    order.sort_by_key(|&index| {
        let krate = &crates[index];
        krate.cargo_target().order(package, index == first)
    });
    order
}

impl Crate {
    fn cargo_target(&self) -> CargoTarget<'_> {
        CargoTarget {
            package: &self.package,
            group: self.group,
            name: &self.name,
            root: &self.root,
        }
    }
}

/// The focus's header and each incoming card's, by RFC 0010.
fn headers(
    crates: &[Crate],
    focus: TypeRef,
    incoming: &[(TypeRef, &[usize])],
) -> (String, Vec<String>) {
    let krate = &crates[focus.krate];
    let entry = &krate.types[focus.id];
    let cards: Vec<CardPath> = incoming
        .iter()
        .map(|(source, _)| {
            let other = &crates[source.krate];
            CardPath {
                path: &other.types[source.id].path,
                other_crate: (source.krate != focus.krate).then_some(other.name.as_str()),
            }
        })
        .collect();
    let module = &krate.modules[entry.module].path;
    let headers = kaalang_svg::card_headers(&entry.path, module, &cards);
    (entry.path.clone(), headers)
}

/// `file:line:column` of `span` in the file of `module`.
fn location(krate: &Crate, module: usize, span: proc_macro2::Span) -> String {
    let (line, column) = kaalang_compiler::line_and_column(span);
    format!(
        "{}:{line}:{column}",
        krate.modules[module].file.path.display()
    )
}

/// A path as written, without the spaces token printing adds.
fn written(path: &syn::Path) -> String {
    let mut written = String::new();
    if path.leading_colon.is_some() {
        written.push_str("::");
    }
    let segments: Vec<String> = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    written.push_str(&segments.join("::"));
    written
}

fn listed(mut message: String, lines: &BTreeSet<String>) -> String {
    for line in lines {
        write!(message, "\n  {line}").expect("writing to a String cannot fail");
    }
    message
}

fn warn(heading: &str, lines: &BTreeSet<String>) {
    if !lines.is_empty() {
        eprintln!("{}: {}", crate::BIN_NAME, listed(heading.to_owned(), lines));
    }
}
