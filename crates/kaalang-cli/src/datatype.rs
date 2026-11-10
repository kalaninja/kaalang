//! Draws a type diagram: finds the focus type in the workspace, resolves every
//! relationship there, and renders the focus under the types that refer to it.

use std::{collections::BTreeSet, fmt::Write as _, path::PathBuf};

use kaalang_compiler::TypeDeclaration;
use kaalang_svg::{RenderError, TypeCard, TypeOptions};
use syn::spanned::Spanned;

use crate::{
    resolve::{Resolution, Resolver, TypeRef},
    workspace::{self, Crate},
};

/// A rendered type diagram and every file it was drawn from.
pub(super) struct Drawn {
    pub(super) svg: String,
    pub(super) files: Vec<PathBuf>,
}

/// Draws the type whose path from its crate root is `path`, taken from
/// `package` when given. A file that cannot be read or parsed, or a type whose
/// annotations do not parse, is skipped and listed, as an error when the type
/// is not found and as a warning when it is.
pub(super) fn draw(
    path: &str,
    package: Option<&str>,
    options: TypeOptions,
) -> Result<Drawn, String> {
    let path = path.strip_prefix("crate::").unwrap_or(path);
    let crates = workspace::crates(None, None)?;
    if let Some(package) = package
        && !crates.iter().any(|krate| krate.package == package)
    {
        return Err(format!(
            "package `{package}` is not a member of this workspace"
        ));
    }
    let (declarations, unreadable) = declarations(&crates);
    let skipped = workspace::skipped(&crates);
    let focus = focus(&crates, path, package, &skipped).map_err(|mut message| {
        if !unreadable.is_empty() {
            message = listed(message + "\nthese types could not be read:", &unreadable);
        }
        message
    })?;
    workspace::warn_skipped(path, &skipped);
    warn("warning: these types could not be read:", &unreadable);
    let drawn = relationships(&crates, &declarations, &focus)?;
    render(&crates, &focus, &drawn, options)
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
    declarations: &[Vec<Option<TypeDeclaration>>],
    focus: &[TypeRef],
) -> Result<Vec<(TypeRef, Vec<usize>)>, String> {
    let resolver = Resolver::new(crates);
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
                    Resolution::Type(found)
                        if focus
                            .iter()
                            .any(|declaration| same(crates, found, *declaration)) =>
                    {
                        positions.push(position);
                    }
                    Resolution::Type(_) => {}
                    Resolution::NotAType(what) => {
                        errors.insert(format!("{at}: `{written}` is {what}, not a type"));
                    }
                    Resolution::Outside => {
                        errors.insert(format!("{at}: `{written}` is not a type of this workspace"));
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

/// Renders the focus under its incoming types in diagram order, one card for a
/// file that several crates include, and one for each declaration that
/// exclusive `cfg`s make of a type. Each package's library and binaries, its
/// tests, its examples, and its benches start a new row.
fn render(
    crates: &[Crate],
    focus: &[TypeRef],
    drawn: &[(TypeRef, Vec<usize>)],
    options: TypeOptions,
) -> Result<Drawn, String> {
    let mut own: &[usize] = &[];
    let mut incoming: Vec<(TypeRef, &[usize])> = Vec::new();
    for krate in order(crates, focus[0].krate) {
        for (source, positions) in drawn.iter().filter(|(source, _)| source.krate == krate) {
            if focus
                .iter()
                .any(|declaration| same(crates, *source, *declaration))
            {
                // Only the focus's first declaration takes lines.
                if own.is_empty() && identical(crates, *source, focus[0]) {
                    own = positions;
                }
            } else if !incoming
                .iter()
                .any(|(other, _)| identical(crates, *other, *source))
            {
                incoming.push((*source, positions));
            }
        }
    }
    let file = |source: TypeRef| {
        let entry = &crates[source.krate].types[source.id];
        crates[source.krate].modules[entry.module].file.path.clone()
    };

    let headers: Vec<String> = incoming
        .iter()
        .map(|(source, _)| header(crates, *source, focus[0].krate))
        .collect();
    let incoming_cfgs: Vec<Vec<String>> = incoming
        .iter()
        .map(|(source, _)| cfgs(crates, *source))
        .collect();
    let focus_header = header(crates, focus[0], focus[0].krate);
    let focus_cfgs: Vec<Vec<String>> = focus.iter().map(|found| cfgs(crates, *found)).collect();
    let focus_cards: Vec<TypeCard> = focus
        .iter()
        .zip(&focus_cfgs)
        .enumerate()
        .map(|(index, (found, cfgs))| {
            let relationships = if index == 0 { own } else { &[] };
            card(crates, *found, &focus_header, relationships, cfgs)
        })
        .collect();
    let incoming_cards: Vec<TypeCard> = incoming
        .iter()
        .zip(&headers)
        .zip(&incoming_cfgs)
        .map(|(((source, positions), header), cfgs)| card(crates, *source, header, positions, cfgs))
        .collect();
    let cards = || {
        focus
            .iter()
            .copied()
            .zip(focus_cards.iter().copied())
            .chain(
                incoming
                    .iter()
                    .map(|(source, _)| *source)
                    .zip(incoming_cards.iter().copied()),
            )
    };
    let mut groups: Vec<&[TypeCard]> = Vec::new();
    let mut start = 0;
    for end in 1..=incoming.len() {
        let group = |index: usize| {
            let krate = &crates[incoming[index].0.krate];
            (&krate.package, krate.group)
        };
        if end == incoming.len() || group(end) != group(start) {
            groups.push(&incoming_cards[start..end]);
            start = end;
        }
    }
    let svg = kaalang_svg::render_type(&focus_cards, &groups, options).map_err(|error| {
        // The renderer names no file, so find the card it rejects.
        let culprit = cards()
            .find(|(_, card)| kaalang_svg::render_type(&[*card], &[], options).is_err())
            .map_or(focus[0], |(source, _)| source);
        format!("{}: {error}", file(culprit).display())
    })?;
    let mut files: Vec<PathBuf> = cards().map(|(source, _)| file(source)).collect();
    files.sort();
    files.dedup();
    Ok(Drawn { svg, files })
}

fn card<'a>(
    crates: &'a [Crate],
    source: TypeRef,
    header: &'a str,
    relationships: &'a [usize],
    cfgs: &'a [String],
) -> TypeCard<'a> {
    let entry = &crates[source.krate].types[source.id];
    TypeCard {
        source: &crates[source.krate].modules[entry.module].file.source,
        path: &entry.in_file,
        occurrence: entry.occurrence,
        header,
        relationships,
        cfgs,
    }
}

/// The `cfg` attributes from the crate root down to a declaration, as
/// written, which tell a type's declarations apart; none for a type declared
/// once.
fn cfgs(crates: &[Crate], found: TypeRef) -> Vec<String> {
    let krate = &crates[found.krate];
    let entry = &krate.types[found.id];
    let declared_once = !(0..krate.types.len()).any(|id| {
        let other = TypeRef {
            krate: found.krate,
            id,
        };
        krate.types[id].path == entry.path && !identical(crates, other, found)
    });
    if declared_once {
        return Vec::new();
    }
    let mut modules = Vec::new();
    let mut module = Some(entry.module);
    while let Some(index) = module {
        modules.push(index);
        module = krate.modules[index].parent;
    }
    modules
        .iter()
        .rev()
        .flat_map(|&index| &krate.modules[index].cfgs)
        .chain(&entry.cfgs)
        .cloned()
        .collect()
}

/// The one type at `path`, from `package` when given, the first in crate order
/// when several crates include its file: each of its declarations there, in
/// source order.
fn focus(
    crates: &[Crate],
    path: &str,
    package: Option<&str>,
    skipped: &BTreeSet<String>,
) -> Result<Vec<TypeRef>, String> {
    let mut found: Vec<Vec<TypeRef>> = Vec::new();
    for krate in order(crates, usize::MAX) {
        if package.is_some_and(|package| crates[krate].package != package) {
            continue;
        }
        for (id, entry) in crates[krate].types.iter().enumerate() {
            let candidate = TypeRef { krate, id };
            if entry.path != path {
                continue;
            }
            match found
                .iter_mut()
                .find(|declarations| same(crates, declarations[0], candidate))
            {
                // Another crate that includes the file, or another reading of
                // it, declares it again.
                Some(declarations)
                    if declarations
                        .iter()
                        .any(|other| identical(crates, *other, candidate)) => {}
                Some(declarations) => declarations.push(candidate),
                None => found.push(vec![candidate]),
            }
        }
    }
    match found.as_slice() {
        [] => {
            let message = RenderError::TypeNotFound(path.to_owned()).to_string();
            Err(workspace::with_skipped(message, skipped))
        }
        [one] => Ok(one.clone()),
        many => {
            let listed: Vec<(&Crate, &std::path::Path)> = many
                .iter()
                .map(|declarations| {
                    let krate = &crates[declarations[0].krate];
                    let module = &krate.modules[krate.types[declarations[0].id].module];
                    (krate, module.file.path.as_path())
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
        (
            index != first,
            Some(krate.package.as_str()) != package,
            &krate.package,
            krate.group,
            &krate.name,
            &krate.root,
        )
    });
    order
}

/// Whether two declarations are of one type: a file that several crates
/// include, or one crate reads twice, declares it in each reading, and
/// exclusive `cfg`s may declare its path more than once in a crate, in one
/// file or in several.
fn same(crates: &[Crate], left: TypeRef, right: TypeRef) -> bool {
    let path = |found: TypeRef| &crates[found.krate].types[found.id].path;
    place(crates, left).0 == place(crates, right).0
        || (left.krate == right.krate && path(left) == path(right))
}

/// Whether two declarations are one: the same place in the same file, which
/// several crates may include or one crate may read twice.
fn identical(crates: &[Crate], left: TypeRef, right: TypeRef) -> bool {
    place(crates, left) == place(crates, right)
}

/// Where a declaration stands: its file and path within it, then which of that
/// path's declarations in the file it is.
fn place(crates: &[Crate], found: TypeRef) -> ((&std::path::Path, &str), usize) {
    let krate = &crates[found.krate];
    let entry = &krate.types[found.id];
    let file = &krate.modules[entry.module].file.canonical;
    ((file.as_path(), entry.in_file.as_str()), entry.occurrence)
}

/// The path a card shows, from its crate root, after the crate's name when it
/// is not the focus's crate.
fn header(crates: &[Crate], found: TypeRef, focus_crate: usize) -> String {
    let krate = &crates[found.krate];
    let path = &krate.types[found.id].path;
    if found.krate == focus_crate {
        path.clone()
    } else {
        format!("{}::{path}", krate.name)
    }
}

/// `file:line:column` of `span` in the file of `module`.
fn location(krate: &Crate, module: usize, span: proc_macro2::Span) -> String {
    let start = span.start();
    let file = krate.modules[module].file.path.display();
    format!("{file}:{}:{}", start.line, start.column + 1)
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
