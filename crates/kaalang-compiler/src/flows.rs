//! Finds the `#[kaa]` functions and the `impl` blocks among a file's items.

use quote::ToTokens;
use syn::{
    Generics, Ident, ImplItem, Item, ItemFn, Path, TraitItem, Type, TypePath, ext::IdentExt,
};

use crate::is_kaalang_attribute;

/// A `#[kaa]` function among a file's items.
pub struct FlowItem {
    /// Its path within the file, which a flow path selects it by:
    /// `route`, `review::route`, `Report::approve`,
    /// `<&Report as Approve>::approve`, or `Approve::approve`.
    pub path: String,
    pub function: ItemFn,
    /// Whether a trait declares it, in the trait or in an `impl` of it. Such a
    /// method takes the trait's visibility and has none of its own.
    pub trait_method: bool,
    /// The inline modules that enclose it: `""`, or `review::` with its
    /// trailing separator.
    pub module: String,
    pub owner: Owner,
}

/// What declares a flow.
pub enum Owner {
    /// A module, as a free function.
    Module,
    /// The `impl` block at this position among the file's.
    Impl(usize),
    /// A trait, as its default method.
    Trait(Ident),
}

/// An `impl` block: its header, the inline modules that enclose it, and the
/// names of the functions it defines.
pub struct Impl {
    pub module: String,
    pub generics: Generics,
    pub trait_: Option<Path>,
    pub self_ty: Type,
    pub functions: Vec<String>,
}

/// A file's `#[kaa]` functions and `impl` blocks, in source order, inline
/// modules included.
pub struct Declarations {
    pub flows: Vec<FlowItem>,
    pub impls: Vec<Impl>,
}

#[must_use]
pub fn declarations(items: &[Item]) -> Declarations {
    let mut found = Declarations {
        flows: Vec::new(),
        impls: Vec::new(),
    };
    collect(items, "", &mut found);
    found
}

/// Collects `#[kaa]` functions and methods from these items, in source
/// order. Searches inline modules and trait methods with default bodies.
#[must_use]
pub fn flows(items: &[Item]) -> Vec<FlowItem> {
    declarations(items).flows
}

fn collect(items: &[Item], module: &str, found: &mut Declarations) {
    let flow = |path: String, function: ItemFn, trait_method: bool, owner: Owner| FlowItem {
        path,
        function,
        trait_method,
        module: module.to_owned(),
        owner,
    };
    for item in items {
        match item {
            Item::Fn(function) if declares_a_flow(&function.attrs) => {
                let path = format!("{module}{}", function.sig.ident);
                found
                    .flows
                    .push(flow(path, function.clone(), false, Owner::Module));
            }
            Item::Mod(inner) => {
                if let Some((_, items)) = &inner.content {
                    collect(items, &format!("{module}{}::", inner.ident), found);
                }
            }
            Item::Impl(block) => {
                let index = found.impls.len();
                let trait_ = block.trait_.as_ref().map(|(path, _)| path.clone());
                let mut functions = Vec::new();
                for item in &block.items {
                    let ImplItem::Fn(method) = item else {
                        continue;
                    };
                    functions.push(method.sig.ident.unraw().to_string());
                    if declares_a_flow(&method.attrs) {
                        let path = method_path(&block.self_ty, trait_.as_ref(), &method.sig.ident);
                        let function = ItemFn {
                            attrs: method.attrs.clone(),
                            vis: method.vis.clone(),
                            modifiers: method.modifiers.clone(),
                            sig: method.sig.clone(),
                            block: Box::new(method.block.clone()),
                        };
                        let owner = Owner::Impl(index);
                        found
                            .flows
                            .push(flow(path, function, trait_.is_some(), owner));
                    }
                }
                found.impls.push(Impl {
                    module: module.to_owned(),
                    generics: block.generics.clone(),
                    trait_,
                    self_ty: (*block.self_ty).clone(),
                    functions,
                });
            }
            Item::Trait(declaration) => {
                for item in &declaration.items {
                    if let TraitItem::Fn(method) = item
                        && declares_a_flow(&method.attrs)
                        && let Some(body) = &method.default
                    {
                        let path = format!("{}::{}", declaration.ident, method.sig.ident);
                        let function = ItemFn {
                            attrs: method.attrs.clone(),
                            vis: syn::Visibility::Inherited,
                            modifiers: method.modifiers.clone(),
                            sig: method.sig.clone(),
                            block: Box::new(body.clone()),
                        };
                        let owner = Owner::Trait(declaration.ident.clone());
                        found.flows.push(flow(path, function, true, owner));
                    }
                }
            }
            _ => {}
        }
    }
}

fn declares_a_flow(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(is_kaalang_attribute)
}

/// The path that selects a method within its file: `Report::approve`, or the
/// qualified form for a trait's method or a type that is not a bare path.
fn method_path(self_ty: &Type, trait_: Option<&Path>, name: &Ident) -> String {
    match (self_ty, trait_) {
        (
            Type::Path(TypePath {
                qself: None, path, ..
            }),
            None,
        ) if path
            .segments
            .iter()
            .all(|segment| segment.arguments.is_none()) =>
        {
            format!("{}::{name}", written(path))
        }
        (_, None) => format!("<{}>::{name}", written(self_ty)),
        (_, Some(trait_)) => format!("<{} as {}>::{name}", written(self_ty), written(trait_)),
    }
}

/// Token text as Rust is usually written: `&'a mut Report<T>`, `(A, B)`.
fn written(tokens: &impl ToTokens) -> String {
    let text = tokens.to_token_stream().to_string();
    let mut written = String::new();
    let mut previous = "";
    for token in text.split_whitespace() {
        // Token printing separates tokens, but a group prints its own spacing.
        let after = previous.chars().last();
        // `Fn(A)`, `Vec<T>`, and `Report::<T>` open without a space, unlike
        // `&'a [T]` or `&mut (A, B)`.
        let name = after.is_some_and(|last| last.is_alphanumeric() || last == '_')
            && !previous.starts_with('\'')
            && !matches!(previous, "mut" | "const" | "dyn" | "unsafe");
        let opens = (token.starts_with('(') && name)
            || (token.starts_with('<') && (name || previous.ends_with("::")));
        let tight = after.is_none_or(|last| matches!(last, '&' | '*' | '<' | '(' | '[' | ':'))
            || opens
            || token.starts_with([',', ';', '>', ')', ']', ':']);
        if !tight {
            written.push(' ');
        }
        written.push_str(token);
        previous = token;
    }
    written
}

#[cfg(test)]
mod tests {
    use super::declarations;

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
    fn flows_are_listed_in_source_order_across_traits_impls_and_modules() {
        let file = syn::parse_file(METHODS).expect("the source parses");
        let paths: Vec<String> = declarations(&file.items)
            .flows
            .into_iter()
            .map(|flow| flow.path)
            .collect();
        assert_eq!(
            paths,
            [
                "Approve::approve",
                "Approve::check",
                "Review::approve",
                "Report::approve",
                "<Report as Approve>::approve",
                "<&Report as Approve>::approve",
                "nested::route",
            ]
        );
    }
}
