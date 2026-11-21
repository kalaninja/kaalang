//! Selects the flows a Rust path names among the files of a crate, as rustc
//! resolves a path to a function, asking the resolver how names resolve, and
//! the types a path names from crate roots.

use std::{collections::BTreeSet, rc::Rc};

use kaalang_compiler::{Declarations, FlowItem, Impl, Owner, declarations};
use quote::ToTokens;
use syn::{
    Expr, ExprPath, GenericArgument, Generics, Ident, Path, PathArguments, ReturnType, Type,
    TypeParamBound, TypePath, ext::IdentExt,
};

use super::{Def, Resolver, TypeRef, resolve::same_item};

/// A flow a flow path names.
pub struct FoundFlow {
    /// The crate whose root the path was written from.
    pub root: usize,
    /// The crate whose module tree reads the declaring file, and the file's
    /// own module in it.
    pub krate: usize,
    pub module: usize,
    /// The flow's position among the file's flows, and its path within the
    /// file.
    pub position: usize,
    pub in_file: String,
}

impl Resolver<'_> {
    /// The flows `path` names from the root of each crate of `roots`, among
    /// the files of the crates listed with it, each flow once however many
    /// crates read its file.
    ///
    /// # Errors
    ///
    /// Returns the root crate from which glob imports make a name ambiguous.
    pub fn select_flows(
        &self,
        path: &FlowPath,
        roots: &[(usize, Vec<usize>)],
    ) -> Result<Vec<FoundFlow>, usize> {
        // Each crate's files, by their own modules, with their declarations.
        let files: Vec<Vec<(usize, Declarations)>> = self
            .crates
            .iter()
            .map(|krate| {
                let modules = krate.modules.iter().enumerate();
                let files = modules.filter(|(_, module)| module.in_file.is_empty());
                files
                    .map(|(at, module)| (at, declarations(&module.file.items)))
                    .collect()
            })
            .collect();
        let mut found = Vec::new();
        // A file that several crates include, or one crate reads twice,
        // declares a flow once: crates read it under their own `cfg`s, so a
        // flow is known by where it stands, not by its position.
        let mut seen = BTreeSet::new();
        for (root, spanned) in roots {
            let listed: Vec<((usize, usize), &Declarations)> = spanned
                .iter()
                .flat_map(|&krate| {
                    let files = files[krate].iter();
                    files.map(move |(at, declarations)| ((krate, *at), declarations))
                })
                .collect();
            let mut ambiguous = false;
            let selected = select(path, &listed, &mut |question| {
                self.answer(*root, question).unwrap_or_else(|| {
                    ambiguous = true;
                    false
                })
            });
            if ambiguous {
                return Err(*root);
            }
            for (file, position) in selected {
                let ((krate, module), declarations) = listed[file];
                let flow = &declarations.flows[position];
                let at = flow.function.sig.ident.span().start();
                let canonical = &self.crates[krate].modules[module].file.canonical;
                if seen.insert((canonical.clone(), at.line, at.column)) {
                    found.push(FoundFlow {
                        root: *root,
                        krate,
                        module,
                        position,
                        in_file: flow.path.clone(),
                    });
                }
            }
        }
        Ok(found)
    }

    /// The types `path` names from the root of each crate of `roots`, in
    /// `roots` order, each with a number the types of one declaration share:
    /// a file that several crates include, or one crate reads twice, declares
    /// a type in each.
    ///
    /// # Errors
    ///
    /// Returns the root crate from which glob imports make a name ambiguous.
    pub fn select_types(
        &self,
        path: &str,
        roots: &[usize],
    ) -> Result<Vec<(TypeRef, usize)>, usize> {
        let Ok(path) = syn::parse_str::<Path>(path) else {
            return Ok(Vec::new());
        };
        let mut found: Vec<(TypeRef, usize)> = Vec::new();
        for &root in roots {
            match self.find(root, 0, &path) {
                Def::Type(ty) if found.iter().all(|(other, _)| *other != ty) => {
                    let first = found
                        .iter()
                        .position(|(other, _)| self.place(*other) == self.place(ty))
                        .unwrap_or(found.len());
                    found.push((ty, first));
                }
                Def::Ambiguous(_) => return Err(root),
                _ => {}
            }
        }
        Ok(found)
    }

    /// Answers what [`select`] asks about a flow path written
    /// from the root of crate `krate`, `None` when glob imports make a name
    /// ambiguous. A place names a crate and a file's own module in it.
    fn answer(&self, krate: usize, question: Question<'_, (usize, usize)>) -> Option<bool> {
        // The module of a place: the file's own, or an inline one within it.
        let module = |at: Place<'_, (usize, usize)>| {
            let (source, own) = at.file;
            let modules = &self.crates[source].modules;
            let found = modules
                .iter()
                .position(|module| {
                    Rc::ptr_eq(&module.file, &modules[own].file) && module.in_file == at.module
                })
                .unwrap_or(own);
            (source, found)
        };
        let query = |def| (!matches!(def, Def::Ambiguous(_))).then_some(def);
        Some(match question {
            Question::Function { path, at, name } => {
                let (source, inside) = module(at);
                matches!(
                    query(self.find_value(krate, 0, path))?,
                    Def::Value(found, declared, value, "a function")
                        if (found, declared) == (source, inside) && name.unraw() == value
                )
            }
            Question::Type(path) => self.is_type(krate, 0, path)?,
            Question::Types {
                query,
                at,
                written,
                generics,
            } => self.types_match((krate, 0), query, module(at), written, generics),
            Question::Same(left, left_path, right, right_path) => {
                let last =
                    |path: &syn::Path| path.segments.last().map(|segment| segment.ident.unraw());
                let (left_crate, left_module) = left.map_or((krate, 0), module);
                let (right_crate, right_module) = module(right);
                let left_def = self.find(left_crate, left_module, left_path);
                let left_def = if left.is_none() {
                    query(left_def)?
                } else {
                    left_def
                };
                let right_def = self.find(right_crate, right_module, right_path);
                // rustc has not checked a path written in the flow path.
                let unchecked = left.is_none();
                same_item(
                    &left_def,
                    unchecked,
                    &right_def,
                    last(left_path) == last(right_path),
                )
            }
        })
    }
}

/// A path that selects a flow: plain, as `review::route` or `Report::approve`,
/// or qualified, as `<&Report as Approve>::approve`.
pub enum FlowPath {
    Plain(Path),
    Qualified {
        self_ty: Box<Type>,
        trait_: Option<Path>,
        name: Ident,
    },
}

impl FlowPath {
    /// Parses `text`, or returns `None` when it is not a path to a function.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let ExprPath { qself, path, .. } = syn::parse_str(text).ok()?;
        let Some(qself) = qself else {
            return Some(Self::Plain(path));
        };
        let leading_colon = path.leading_colon;
        let segments: Vec<_> = path.segments.into_iter().collect();
        // `<Report as a::Approve>::approve`: the trait is the path before
        // `position`, and one name follows it.
        let (trait_segments, name) = segments.split_at(qself.position);
        let [name] = name else {
            return None;
        };
        let trait_ = (!trait_segments.is_empty()).then(|| Path {
            leading_colon,
            segments: trait_segments.iter().cloned().collect(),
        });
        Some(Self::Qualified {
            self_ty: qself.ty,
            trait_,
            name: name.ident.clone(),
        })
    }
}

/// Where a path is written: a file, as the caller identifies it, and the
/// inline modules within it.
#[derive(Clone, Copy)]
struct Place<'a, X> {
    file: X,
    module: &'a str,
}

/// What [`select`] asks the caller, which knows how names resolve.
#[derive(Clone, Copy)]
enum Question<'a, X> {
    /// Whether the plain flow path names the free function `name` declared at
    /// the place.
    Function {
        path: &'a Path,
        at: Place<'a, X>,
        name: &'a Ident,
    },
    /// Whether two paths name one item. A path without a place is written in
    /// the flow path, from its root.
    Same(Option<Place<'a, X>>, &'a Path, Place<'a, X>, &'a Path),
    /// Whether a path written in the flow path, from its root, names a type,
    /// or may name one the source cannot tell.
    Type(&'a Path),
    /// Whether the query type matches the type written in an `impl`, after
    /// resolving aliases in their respective scopes.
    Types {
        query: &'a Type,
        at: Place<'a, X>,
        written: &'a Type,
        generics: &'a Generics,
    },
}

/// Selects the flows `path` names among the files' declarations, by RFC 0009
/// §6. `names` answers how names resolve. Returns each match as its file's
/// position and its position among that file's flows.
fn select<X: Copy>(
    path: &FlowPath,
    files: &[(X, &Declarations)],
    names: &mut dyn FnMut(Question<'_, X>) -> bool,
) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    match path {
        FlowPath::Plain(path) => {
            for_each_flow(files, |position, at, flow| {
                if matches!(flow.owner, Owner::Module) {
                    let name = &flow.function.sig.ident;
                    if names(Question::Function { path, at, name }) {
                        found.push(position);
                    }
                }
            });
            // A path names one item, so a free function leaves no method to
            // look for, and a trait's default method no type's method.
            let mut prefix = path.clone();
            if let Some(name) = prefix
                .segments
                .pop()
                .filter(|_| found.is_empty() && !prefix.segments.is_empty())
            {
                let name = name.ident;
                prefix.segments.pop_punct();
                for_each_flow(files, |position, at, flow| {
                    if let Owner::Trait(declared) = &flow.owner
                        && named(&flow.function.sig.ident, &name)
                        && names(Question::Same(None, &prefix, at, &declared.clone().into()))
                    {
                        found.push(position);
                    }
                });
                if found.is_empty() {
                    let self_ty = Type::Path(TypePath {
                        attrs: Vec::new(),
                        qself: None,
                        path: prefix,
                    });
                    found.extend(methods(&self_ty, None, &name, files, names));
                }
            }
        }
        FlowPath::Qualified {
            self_ty,
            trait_,
            name,
        } => found.extend(methods(self_ty, trait_.as_ref(), name, files, names)),
    }
    found.sort_unstable();
    found.dedup();
    found
}

/// The flows of `name` in the `impl` blocks of `self_ty`, of `trait_` when
/// given: the inherent ones, which shadow every trait's when there is no
/// trait, or each trait's, from the block or from the trait's default method.
fn methods<X: Copy>(
    self_ty: &Type,
    trait_: Option<&Path>,
    name: &Ident,
    files: &[(X, &Declarations)],
    names: &mut dyn FnMut(Question<'_, X>) -> bool,
) -> Vec<(usize, usize)> {
    // A blanket impl must not turn a module or a missing name into a type.
    if !query_type(self_ty, names) {
        return Vec::new();
    }
    let mut blocks = Vec::new();
    for (position, (file, declarations)) in files.iter().enumerate() {
        for (index, block) in declarations.impls.iter().enumerate() {
            let at = Place {
                file: *file,
                module: &block.module,
            };
            let of_trait = match (trait_, &block.trait_) {
                (None, _) => true,
                (Some(query), Some(written)) => names(Question::Types {
                    query: &syn::parse_quote!(#query),
                    at,
                    written: &syn::parse_quote!(#written),
                    generics: &block.generics,
                }),
                (Some(_), None) => false,
            };
            if of_trait
                && names(Question::Types {
                    query: self_ty,
                    at,
                    written: &block.self_ty,
                    generics: &block.generics,
                })
            {
                blocks.push((position, index, block));
            }
        }
    }
    let defines = |block: &Impl| {
        block
            .functions
            .iter()
            .any(|function| named_str(function, name))
    };
    if trait_.is_none() {
        let inherent = |block: &Impl| block.trait_.is_none() && defines(block);
        if blocks.iter().any(|(_, _, block)| inherent(block)) {
            blocks.retain(|(_, _, block)| inherent(block));
        } else {
            blocks.retain(|(_, _, block)| block.trait_.is_some());
        }
    }
    let mut found = Vec::new();
    for (position, index, block) in blocks {
        let declarations = files[position].1;
        if defines(block) {
            for (flow, item) in declarations.flows.iter().enumerate() {
                if matches!(item.owner, Owner::Impl(owner) if owner == index)
                    && named(&item.function.sig.ident, name)
                {
                    found.push((position, flow));
                }
            }
        } else if let Some(written) = &block.trait_ {
            let block_at = Place {
                file: files[position].0,
                module: &block.module,
            };
            for_each_flow(files, |default, at, flow| {
                if let Owner::Trait(declared) = &flow.owner
                    && named(&flow.function.sig.ident, name)
                    && names(Question::Same(
                        Some(block_at),
                        written,
                        at,
                        &declared.clone().into(),
                    ))
                {
                    found.push(default);
                }
            });
        }
    }
    found
}

/// Checks named types through structural wrappers. Generic arguments may be
/// constants, so their paths cannot be checked as types without type checking.
fn query_type<X: Copy>(ty: &Type, names: &mut dyn FnMut(Question<'_, X>) -> bool) -> bool {
    match bare(ty) {
        Type::Path(path) if path.qself.is_none() => names(Question::Type(&path.path)),
        Type::Reference(ty) => query_type(&ty.elem, names),
        Type::Ptr(ty) => query_type(&ty.elem, names),
        Type::Slice(ty) => query_type(&ty.elem, names),
        Type::Array(ty) => query_type(&ty.elem, names),
        Type::Tuple(ty) => ty.elems.iter().all(|ty| query_type(ty, names)),
        Type::FnPtr(ty) => {
            ty.inputs.iter().all(|input| query_type(&input.ty, names))
                && match &ty.output {
                    ReturnType::Default => true,
                    ReturnType::Type(_, ty) => query_type(ty, names),
                }
        }
        _ => true,
    }
}

fn for_each_flow<'a, X: Copy>(
    files: &'a [(X, &'a Declarations)],
    mut visit: impl FnMut((usize, usize), Place<'a, X>, &'a FlowItem),
) {
    for (position, (file, declarations)) in files.iter().enumerate() {
        for (index, flow) in declarations.flows.iter().enumerate() {
            let at = Place {
                file: *file,
                module: &flow.module,
            };
            visit((position, index), at, flow);
        }
    }
}

fn named(ident: &Ident, name: &Ident) -> bool {
    ident.unraw() == name.unraw()
}

fn named_str(text: &str, name: &Ident) -> bool {
    name.unraw() == text
}

/// Whether `query`, a type the flow path writes, matches `written`, the self
/// type of an `impl` block with `generics`. Lifetimes are ignored, and the
/// block's parameters, `_`, and omitted generic arguments match any type.
/// `same` tells whether a path of each names one item.
pub(super) fn types_match(
    query: &Type,
    written: &Type,
    generics: &Generics,
    same: &mut dyn FnMut(&Path, &Path) -> bool,
) -> bool {
    match (bare(query), bare(written)) {
        (Type::Infer(_), _) => true,
        (_, Type::Path(written)) if parameter(written, generics) => true,
        (Type::Path(query), Type::Path(written)) => {
            query.qself.is_none()
                && written.qself.is_none()
                && paths_match(&query.path, &written.path, generics, same)
        }
        (Type::Reference(query), Type::Reference(written)) => {
            query.mutability.is_some() == written.mutability.is_some()
                && types_match(&query.elem, &written.elem, generics, same)
        }
        (Type::Ptr(query), Type::Ptr(written)) => {
            matches!(query.mutability, syn::PointerMutability::Mut(_))
                == matches!(written.mutability, syn::PointerMutability::Mut(_))
                && types_match(&query.elem, &written.elem, generics, same)
        }
        (Type::Slice(query), Type::Slice(written)) => {
            types_match(&query.elem, &written.elem, generics, same)
        }
        (Type::Array(query), Type::Array(written)) => {
            types_match(&query.elem, &written.elem, generics, same)
                && constants_match(&query.len, &written.len, generics)
        }
        (Type::Tuple(query), Type::Tuple(written)) => {
            query.elems.len() == written.elems.len()
                && query
                    .elems
                    .iter()
                    .zip(&written.elems)
                    .all(|(query, written)| types_match(query, written, generics, same))
        }
        (Type::FnPtr(query), Type::FnPtr(written)) => {
            let abi = |abi: &Option<syn::Abi>| match abi {
                None => "Rust".to_owned(),
                Some(abi) => abi
                    .name
                    .as_ref()
                    .map_or_else(|| "C".to_owned(), syn::LitStr::value),
            };
            query.unsafety.is_some() == written.unsafety.is_some()
                && abi(&query.abi) == abi(&written.abi)
                && query.variadic.is_some() == written.variadic.is_some()
                && query.inputs.len() == written.inputs.len()
                && query
                    .inputs
                    .iter()
                    .zip(&written.inputs)
                    .all(|(query, written)| types_match(&query.ty, &written.ty, generics, same))
                && outputs_match(&query.output, &written.output, generics, same)
        }
        (Type::TraitObject(query), Type::TraitObject(written)) => {
            let traits = |bounds: &syn::punctuated::Punctuated<TypeParamBound, _>| {
                bounds
                    .iter()
                    .filter_map(|bound| match bound {
                        TypeParamBound::Trait(bound) => Some(bound.path.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            };
            let (query, written) = (traits(&query.bounds), traits(&written.bounds));
            query.iter().all(|query| {
                written
                    .iter()
                    .any(|written| paths_match(query, written, generics, same))
            }) && written.iter().all(|written| {
                query
                    .iter()
                    .any(|query| paths_match(query, written, generics, same))
            })
        }
        (query, written) => same_tokens(query, written),
    }
}

/// `ty` without the parentheses and invisible groups around it.
fn bare(mut ty: &Type) -> &Type {
    loop {
        ty = match ty {
            Type::Paren(inner) => &inner.elem,
            Type::Group(inner) => &inner.elem,
            _ => return ty,
        };
    }
}

/// Whether `ty` is a type or const parameter of `generics`.
fn parameter(ty: &TypePath, generics: &Generics) -> bool {
    ty.qself.is_none()
        && ty.path.get_ident().is_some_and(|ident| {
            generics.type_params().any(|param| param.ident == *ident)
                || generics.const_params().any(|param| param.ident == *ident)
        })
}

fn constants_match(query: &Expr, written: &Expr, generics: &Generics) -> bool {
    let parameter = matches!(written, Expr::Path(path) if path.path.get_ident().is_some_and(|ident| generics.const_params().any(|param| param.ident == *ident)));
    parameter || matches!(query, Expr::Infer(_)) || same_tokens(query, written)
}

fn outputs_match(
    query: &ReturnType,
    written: &ReturnType,
    generics: &Generics,
    same: &mut dyn FnMut(&Path, &Path) -> bool,
) -> bool {
    let unit: Type = syn::parse_quote!(());
    let query = match query {
        ReturnType::Default => &unit,
        ReturnType::Type(_, ty) => ty,
    };
    let written = match written {
        ReturnType::Default => &unit,
        ReturnType::Type(_, ty) => ty,
    };
    types_match(query, written, generics, same)
}

/// Whether two paths name one item, by `same`, with matching generic arguments
/// on their last segments.
fn paths_match(
    query: &Path,
    written: &Path,
    generics: &Generics,
    same: &mut dyn FnMut(&Path, &Path) -> bool,
) -> bool {
    if !same(query, written) {
        return false;
    }
    match (
        query.segments.last().map(|segment| &segment.arguments),
        written.segments.last().map(|segment| &segment.arguments),
    ) {
        (
            Some(PathArguments::AngleBracketed(query)),
            Some(PathArguments::AngleBracketed(written)),
        ) => arguments_match(query, written, generics, same),
        (
            Some(PathArguments::Parenthesized(query)),
            Some(PathArguments::Parenthesized(written)),
        ) => {
            query.inputs.len() == written.inputs.len()
                && query
                    .inputs
                    .iter()
                    .zip(&written.inputs)
                    .all(|(query, written)| types_match(&query.ty, &written.ty, generics, same))
                && outputs_match(&query.output, &written.output, generics, same)
        }
        (None | Some(PathArguments::None), _) | (_, None | Some(PathArguments::None)) => true,
        _ => false,
    }
}

fn arguments_match(
    query: &syn::AngleBracketedGenericArguments,
    written: &syn::AngleBracketedGenericArguments,
    generics: &Generics,
    same: &mut dyn FnMut(&Path, &Path) -> bool,
) -> bool {
    let mut positional = written.args.iter().filter(|argument| {
        matches!(
            argument,
            GenericArgument::Type(_) | GenericArgument::Const(_)
        )
    });
    query.args.iter().all(|query| {
        let written = match query {
            GenericArgument::Lifetime(_) => return true,
            GenericArgument::Type(_) | GenericArgument::Const(_) => positional.next(),
            GenericArgument::AssocType(query) => written.args.iter().find(|argument| {
                matches!(argument, GenericArgument::AssocType(written) if named(&query.ident, &written.ident))
            }),
            _ => return written.args.iter().any(|argument| same_tokens(query, argument)),
        };
        written.is_none_or(|written| match (query, written) {
            (GenericArgument::Type(query), GenericArgument::Type(written)) => {
                types_match(query, written, generics, same)
            }
            (GenericArgument::Const(query), GenericArgument::Const(written)) => {
                constants_match(query, written, generics)
            }
            // A const argument the block passes as a parameter parses as a type.
            (GenericArgument::Const(_), GenericArgument::Type(Type::Path(written))) => {
                parameter(written, generics)
            }
            (GenericArgument::AssocType(query), GenericArgument::AssocType(written)) => {
                query.generics.as_ref().zip(written.generics.as_ref()).is_none_or(|(query, written)| {
                    arguments_match(query, written, generics, same)
                }) && types_match(&query.ty, &written.ty, generics, same)
            }
            _ => false,
        })
    })
}

fn same_tokens(left: &impl ToTokens, right: &impl ToTokens) -> bool {
    left.to_token_stream().to_string() == right.to_token_stream().to_string()
}

#[cfg(test)]
mod tests {
    use kaalang_compiler::declarations;
    use syn::{Path, ext::IdentExt};

    use super::{FlowPath, Question, select};

    /// The paths of the flows `path` selects in `source` alone, which resolves
    /// nothing: a free function by the inline modules that enclose it, then
    /// its name, and every type and trait by its last segment.
    fn selected(source: &str, path: &str) -> Vec<String> {
        let file = syn::parse_file(source).expect("the source parses");
        let declarations = declarations(&file.items);
        let path = FlowPath::parse(path).expect("the path parses");
        let unraw = |text: &str| text.strip_prefix("r#").unwrap_or(text).to_owned();
        let last = |path: &Path| path.segments.last().map(|segment| segment.ident.unraw());
        select(
            &path,
            &[((), &declarations)],
            &mut |question| match question {
                Question::Function { path, at, name } => {
                    let written: Vec<String> = path
                        .segments
                        .iter()
                        .map(|segment| segment.ident.unraw().to_string())
                        .collect();
                    let declared: Vec<String> = at
                        .module
                        .split_terminator("::")
                        .map(unraw)
                        .chain([name.unraw().to_string()])
                        .collect();
                    path.leading_colon.is_none() && written == declared
                }
                Question::Same(_, left, _, right) => last(left) == last(right),
                Question::Type(_) => true,
                Question::Types {
                    query,
                    written,
                    generics,
                    ..
                } => super::types_match(query, written, generics, &mut |left, right| {
                    last(left) == last(right)
                }),
            },
        )
        .into_iter()
        .map(|(_, flow)| declarations.flows[flow].path.clone())
        .collect()
    }

    const METHODS: &str = "
        struct Report;
        trait Approve { #[kaa] fn approve(&self) {} #[kaa] fn check(&self) {} }
        trait Review { #[kaa] fn approve(&self) {} }
        impl Report { #[kaa] fn approve(&self) {} }
        impl Approve for Report { #[kaa] fn approve(&self) {} }
        impl Review for Report {}
        impl Approve for &Report { #[kaa] fn approve(&self) {} }
        mod nested { #[kaa] fn route() {} }
    ";

    #[test]
    fn a_free_function_is_selected_by_its_modules_and_name() {
        assert_eq!(selected(METHODS, "nested::route"), ["nested::route"]);
        assert_eq!(selected(METHODS, "route"), Vec::<String>::new());
    }

    #[test]
    fn an_inherent_method_shadows_the_traits_methods() {
        assert_eq!(selected(METHODS, "Report::approve"), ["Report::approve"]);
        assert_eq!(selected(METHODS, "<Report>::approve"), ["Report::approve"]);
    }

    #[test]
    fn a_qualified_path_selects_the_trait_implementation() {
        assert_eq!(
            selected(METHODS, "<Report as Approve>::approve"),
            ["<Report as Approve>::approve"]
        );
        assert_eq!(
            selected(METHODS, "<&Report as Approve>::approve"),
            ["<&Report as Approve>::approve"]
        );
    }

    #[test]
    fn a_kept_default_method_is_selected_through_the_type() {
        assert_eq!(
            selected(METHODS, "<Report as Review>::approve"),
            ["Review::approve"]
        );
        assert_eq!(selected(METHODS, "Report::check"), ["Approve::check"]);
        assert_eq!(selected(METHODS, "Review::approve"), ["Review::approve"]);
    }

    #[test]
    fn a_blanket_impl_does_not_take_a_free_function_or_default_methods_path() {
        let source = "
            mod nested { #[kaa] fn route() {} }
            trait Approve { #[kaa] fn approve(&self) {} }
            trait Route { fn route(&self); fn approve(&self); }
            impl<T> Route for T { #[kaa] fn route(&self) {} #[kaa] fn approve(&self) {} }
        ";
        assert_eq!(selected(source, "nested::route"), ["nested::route"]);
        assert_eq!(selected(source, "Approve::approve"), ["Approve::approve"]);
    }

    #[test]
    fn qualified_paths_check_named_types_before_matching_a_blanket_impl() {
        let file = syn::parse_file("impl<T> Review for T { #[kaa] fn approve() {} }").unwrap();
        let declarations = declarations(&file.items);
        for (ty, expected) in [
            ("Report", 1),
            ("&Report", 1),
            ("fn(Report) -> Report", 1),
            ("Array<SIZE>", 1),
            ("_", 1),
            ("NotAType", 0),
            ("(NotAType)", 0),
            ("&NotAType", 0),
            ("*const NotAType", 0),
            ("[NotAType]", 0),
            ("[NotAType; 2]", 0),
            ("(Report, NotAType)", 0),
            ("fn(NotAType)", 0),
            ("fn() -> NotAType", 0),
        ] {
            let path = FlowPath::parse(&format!("<{ty} as Review>::approve")).unwrap();
            let found = select(
                &path,
                &[((), &declarations)],
                &mut |question| match question {
                    Question::Type(path) => path.segments.last().is_some_and(|segment| {
                        matches!(segment.ident.to_string().as_str(), "Report" | "Array")
                    }),
                    Question::Types { .. } => true,
                    _ => false,
                },
            );
            assert_eq!(found.len(), expected, "{ty}");
        }
    }

    #[test]
    fn a_method_two_traits_provide_is_ambiguous_without_an_inherent_one() {
        let source = "
            struct Report;
            trait Approve { fn approve(&self); }
            trait Review { fn approve(&self); }
            impl Approve for Report { #[kaa] fn approve(&self) {} }
            impl Review for Report { #[kaa] fn approve(&self) {} }
        ";
        assert_eq!(selected(source, "Report::approve").len(), 2);
    }

    #[test]
    fn an_inherent_method_that_is_not_a_flow_still_shadows_the_traits() {
        let source = "
            struct Report;
            trait Approve { fn approve(&self); }
            impl Report { fn approve(&self) {} }
            impl Approve for Report { #[kaa] fn approve(&self) {} }
        ";
        assert_eq!(selected(source, "Report::approve"), Vec::<String>::new());
    }

    #[test]
    fn parameters_underscores_omitted_arguments_and_lifetimes_match_any_type() {
        let source = "
            trait Approve { fn approve(&self); }
            impl<T> Approve for Vec<T> { #[kaa] fn approve(&self) {} }
            impl<'a> Approve for &'a [u8] { #[kaa] fn approve(&self) {} }
            struct Pair<A, B>(A, B);
            impl Pair<u8, u16> { #[kaa] fn swap(&self) {} }
            impl Pair<u16, u8> { #[kaa] fn swap(&self) {} }
        ";
        for (path, flow) in [
            (
                "<Vec<u8> as Approve>::approve",
                "<Vec<T> as Approve>::approve",
            ),
            (
                "<Vec<_> as Approve>::approve",
                "<Vec<T> as Approve>::approve",
            ),
            (
                "<&'static [u8] as Approve>::approve",
                "<&'a [u8] as Approve>::approve",
            ),
            ("Pair::<u8, u16>::swap", "<Pair<u8, u16>>::swap"),
            ("<Pair<u16, _>>::swap", "<Pair<u16, u8>>::swap"),
        ] {
            assert_eq!(selected(source, path), [flow], "{path}");
        }
        assert_eq!(selected(source, "Pair::swap").len(), 2);
    }

    #[test]
    fn function_pointer_safety_abi_and_variadics_select_distinct_impls() {
        let source = r#"
            trait Run { fn run(); }
            impl Run for fn() { #[kaa] fn run() {} }
            impl Run for unsafe fn() { #[kaa] fn run() {} }
            impl Run for extern "C" fn() { #[kaa] fn run() {} }
            impl Run for unsafe extern "C" fn() { #[kaa] fn run() {} }
            impl Run for unsafe extern "C" fn(...) { #[kaa] fn run() {} }
        "#;
        for ty in [
            "fn()",
            "unsafe fn()",
            "extern \"C\" fn()",
            "unsafe extern \"C\" fn()",
            "unsafe extern \"C\" fn(...)",
        ] {
            let path = format!("<{ty} as Run>::run");
            assert_eq!(selected(source, &path), [path.as_str()]);
        }
        assert_eq!(
            selected(source, "<extern fn() as Run>::run"),
            ["<extern \"C\" fn() as Run>::run"]
        );
        assert_eq!(
            selected(source, "<extern \"Rust\" fn() as Run>::run"),
            ["<fn() as Run>::run"]
        );
    }

    #[test]
    fn trait_arguments_select_distinct_impls() {
        let source = "
            trait Run { fn run(); }
            impl Run for &dyn Fn() -> u8 { #[kaa] fn run() {} }
            impl Run for &dyn Fn(u8) { #[kaa] fn run() {} }
            impl Run for &dyn Fn(u8) -> u8 { #[kaa] fn run() {} }
            impl Run for &dyn Fn(u16) -> u8 { #[kaa] fn run() {} }
            impl Run for &dyn Fn(u8) -> u16 { #[kaa] fn run() {} }
            impl Run for &dyn Iterator<Item = u8> { #[kaa] fn run() {} }
            impl Run for &dyn Iterator<Item = u16> { #[kaa] fn run() {} }
        ";
        let file = syn::parse_file(source).unwrap();
        for flow in declarations(&file.items).flows {
            assert_eq!(selected(source, &flow.path), [flow.path]);
        }
        assert_eq!(
            selected(source, "<&dyn Fn(u8) -> () as Run>::run"),
            ["<&dyn Fn(u8) as Run>::run"],
        );
        assert_eq!(
            selected(source, "<&dyn Iterator<Item = _> as Run>::run").len(),
            2,
        );
    }

    #[test]
    fn associated_type_bindings_match_by_name_and_keep_omitted_arguments() {
        let source = "
            impl Run for &dyn Pair<Left = u8, Right = u16> { #[kaa] fn run() {} }
        ";
        for ty in [
            "&dyn Pair<Right = u16, Left = u8>",
            "&dyn Pair<Right = u16>",
            "&dyn Pair",
        ] {
            let path = format!("<{ty} as Run>::run");
            assert_eq!(selected(source, &path).len(), 1, "{path}");
        }
        assert_eq!(
            selected(source, "<&dyn Pair<Right = u8> as Run>::run"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn trait_object_bounds_match_independently_of_order_and_repetition() {
        let source = "
            impl<T> Run for &(dyn Fn(T) + Send + Sync) { #[kaa] fn run() {} }
            impl<T> Run for &(dyn Fn(T) + Send) { #[kaa] fn run() {} }
        ";
        for ty in [
            "&(dyn Sync + Fn(u8) + Send)",
            "&(dyn Fn(u8) + Send + Sync + Send)",
        ] {
            let path = format!("<{ty} as Run>::run");
            assert_eq!(
                selected(source, &path),
                ["<&(dyn Fn(T) + Send + Sync) as Run>::run"],
                "{path}",
            );
        }
        assert_eq!(
            selected(source, "<&(dyn Send + Fn(u8)) as Run>::run"),
            ["<&(dyn Fn(T) + Send) as Run>::run"],
        );
        assert_eq!(
            selected(source, "<&dyn Fn(u8) as Run>::run"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn types_that_are_not_paths_are_written_without_token_spacing() {
        let source = "
            trait Run { fn run(self); }
            impl Run for (u8, [u16; 2]) { #[kaa] fn run(self) {} }
            impl Run for *const u8 { #[kaa] fn run(self) {} }
            impl Run for fn(u8) -> u16 { #[kaa] fn run(self) {} }
            impl<'a> Run for &'a mut dyn Fn(u8) { #[kaa] fn run(self) {} }
        ";
        let file = syn::parse_file(source).expect("the source parses");
        let paths: Vec<String> = declarations(&file.items)
            .flows
            .into_iter()
            .map(|flow| flow.path)
            .collect();
        assert_eq!(
            paths,
            [
                "<(u8, [u16; 2]) as Run>::run",
                "<*const u8 as Run>::run",
                "<fn(u8) -> u16 as Run>::run",
                "<&'a mut dyn Fn(u8) as Run>::run",
            ]
        );
        for path in &paths {
            assert_eq!(selected(source, path), [path.as_str()], "{path}");
        }
    }
}
