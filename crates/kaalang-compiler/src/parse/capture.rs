//! Checks direct parameter uses so hiding a parameter cannot expose a same-named
//! module item. Rust checks captures and authored local bindings normally.

use std::collections::BTreeSet;

use proc_macro2::Ident;
use syn::{
    Error, Expr, FnArg, Item, ItemFn, Local, Pat, PatIdent, Result, Stmt, UseTree,
    ext::IdentExt,
    visit::{self, Visit},
};

use crate::{BlockKind, Flow};

pub(super) fn validate(flow: &Flow, function: &ItemFn) -> Result<()> {
    let parameters = function
        .sig
        .inputs
        .iter()
        .filter_map(|input| match input {
            FnArg::Typed(input) => match input.pat.as_ref() {
                Pat::Ident(binding) => Some(binding.ident.unraw()),
                _ => None,
            },
            FnArg::Receiver(_) => None,
        })
        .collect::<BTreeSet<_>>();
    for block in &flow.blocks {
        if !matches!(
            block.kind,
            BlockKind::Action | BlockKind::Call | BlockKind::Question | BlockKind::Choice
        ) {
            continue;
        }
        let uncaptured = parameters
            .iter()
            .filter(|name| !super::captured(&block.inputs, name))
            .cloned()
            .collect();
        check(&block.body, uncaptured)?;
    }
    Ok(())
}

fn check(body: &Expr, uncaptured: BTreeSet<Ident>) -> Result<()> {
    if uncaptured.is_empty() {
        return Ok(());
    }
    let mut uses = Uses {
        uncaptured,
        locals: Vec::new(),
        opaque: false,
        error: None,
    };
    uses.visit_expr(body);
    uses.error.map_or(Ok(()), Err)
}

struct Uses {
    uncaptured: BTreeSet<Ident>,
    locals: Vec<Ident>,
    /// An opaque macro or import may introduce a local binding or hoisted item.
    opaque: bool,
    error: Option<Error>,
}

impl Uses {
    fn scoped(&mut self, visit: impl FnOnce(&mut Self)) {
        let saved = (self.locals.len(), self.opaque);
        visit(self);
        self.locals.truncate(saved.0);
        self.opaque = saved.1;
    }

    fn bind(&mut self, pattern: &Pat) {
        struct Bindings<'a>(&'a mut Vec<Ident>, &'a mut bool);
        impl<'ast> Visit<'ast> for Bindings<'_> {
            fn visit_pat_ident(&mut self, binding: &'ast PatIdent) {
                self.0.push(binding.ident.unraw());
                visit::visit_pat_ident(self, binding);
            }

            fn visit_pat(&mut self, pattern: &'ast Pat) {
                if matches!(pattern, Pat::Macro(_)) {
                    *self.1 = true;
                } else {
                    visit::visit_pat(self, pattern);
                }
            }

            fn visit_expr(&mut self, _: &'ast Expr) {}
        }
        Bindings(&mut self.locals, &mut self.opaque).visit_pat(pattern);
    }

    fn import(&mut self, tree: &UseTree, parent: Option<&Ident>) {
        match tree {
            UseTree::Path(path) => self.import(&path.tree, Some(&path.ident)),
            UseTree::Name(name) => {
                let ident = if name.ident == "self" {
                    parent.unwrap_or(&name.ident)
                } else {
                    &name.ident
                };
                self.locals.push(ident.unraw());
            }
            UseTree::Rename(rename) => self.locals.push(rename.rename.unraw()),
            UseTree::Group(group) => {
                for item in &group.items {
                    self.import(item, parent);
                }
            }
            UseTree::Glob(_) => self.opaque = true,
        }
    }
}

impl<'ast> Visit<'ast> for Uses {
    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        if path.qself.is_none()
            && path.path.leading_colon.is_none()
            && path.path.segments.len() == 1
            && let Some(name) = path.path.segments.first().map(|segment| &segment.ident)
            && self.uncaptured.contains(&name.unraw())
            && !self.locals.contains(&name.unraw())
            && !self.opaque
        {
            self.error.get_or_insert_with(|| {
                Error::new(
                    name.span(),
                    format!("a kaalang block body must capture `{name}` before using it"),
                )
            });
        }
        visit::visit_expr_path(self, path);
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.scoped(|this| {
            // Rust items are hoisted, including items emitted by statement macros.
            for statement in &block.stmts {
                match statement {
                    Stmt::Item(Item::Fn(item)) => this.locals.push(item.sig.ident.unraw()),
                    Stmt::Item(Item::Const(item)) => this.locals.push(item.ident.unraw()),
                    Stmt::Item(Item::Static(item)) => this.locals.push(item.ident.unraw()),
                    Stmt::Item(Item::Struct(item))
                        if !matches!(item.fields, syn::Fields::Named(_)) =>
                    {
                        this.locals.push(item.ident.unraw());
                    }
                    Stmt::Item(Item::Use(item)) => this.import(&item.tree, None),
                    Stmt::Macro(_) => this.opaque = true,
                    _ => {}
                }
            }
            visit::visit_block(this, block);
        });
    }

    fn visit_local(&mut self, local: &'ast Local) {
        // RFC 0007 §5.3 leaves conditional bindings to Rust's name resolution.
        // Parameter bindings remain hygienically hidden from uncaptured uses.
        let conditional = local.attrs.iter().any(|attribute| {
            attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
        });
        if !conditional {
            if let Some(initializer) = &local.init {
                self.visit_expr(&initializer.expr);
                if let Some((_, diverge)) = &initializer.diverge {
                    self.visit_expr(diverge);
                }
            }
            self.visit_pat(&local.pat);
        }
        self.bind(&local.pat);
    }

    fn visit_expr_closure(&mut self, closure: &'ast syn::ExprClosure) {
        self.scoped(|this| {
            for input in &closure.inputs {
                this.bind(input);
                this.visit_pat(input);
            }
            this.visit_return_type(&closure.output);
            this.visit_expr(&closure.body);
        });
    }

    fn visit_expr_for_loop(&mut self, loop_: &'ast syn::ExprForLoop) {
        self.visit_expr(&loop_.expr);
        self.scoped(|this| {
            this.bind(&loop_.pat);
            this.visit_pat(&loop_.pat);
            this.visit_block(&loop_.body);
        });
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        self.scoped(|this| {
            this.bind(&arm.pat);
            this.visit_pat(&arm.pat);
            this.visit_expr(&arm.body);
        });
    }

    fn visit_expr_let(&mut self, let_: &'ast syn::ExprLet) {
        self.visit_expr(&let_.expr);
        self.bind(&let_.pat);
        self.visit_pat(&let_.pat);
    }

    fn visit_expr_if(&mut self, if_: &'ast syn::ExprIf) {
        self.scoped(|this| {
            this.visit_expr(&if_.cond);
            this.visit_block(&if_.then_branch);
        });
        if let Some((_, else_)) = &if_.else_branch {
            self.visit_expr(else_);
        }
    }

    fn visit_expr_while(&mut self, while_: &'ast syn::ExprWhile) {
        self.scoped(|this| {
            this.visit_expr(&while_.cond);
            this.visit_block(&while_.body);
        });
    }

    fn visit_item(&mut self, _: &'ast Item) {}
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    fn check(body: &str) -> syn::Result<()> {
        super::check(
            &syn::parse_str(body).expect("a Rust expression"),
            [parse_quote!(value)].into(),
        )
    }

    #[test]
    fn direct_uses_require_captures_before_local_shadowing() {
        for body in [
            "value()",
            "value::<u32>()",
            "r#value",
            "{ value = 9; }",
            "{ (value, _) = (9, 0); }",
            "{ value += 9; }",
            "{ value.field = 9; }",
            "{ value[0] = 9; }",
            "{ *value = 9; }",
            "{ let value = value; }",
            "{ let value: [u8; value()] = [0; 2]; value }",
            "{ let make = || -> [u8; value()] { [0; 2] }; }",
            "{ #[allow(unused)] let ignored = value(); }",
            "{ let Some(value) = Some(0) else { value(); }; }",
            "{ { let value = 0; } value }",
        ] {
            assert!(check(body).is_err(), "{body}");
        }
    }

    #[test]
    fn native_bindings_shadow_only_in_their_own_scopes() {
        for body in [
            "{ let mut value = 0; value = 9; }",
            "{ let (mut value,) = (0,); value = 9; }",
            "{ let update = |mut value| { value = 9; }; }",
            "{ for mut value in 0..9 { value = 9; } }",
            "{ match Some(0) { Some(mut value) if { value = 1; true } => { value = 9; }, _ => {} } }",
            "{ if let Some(mut value) = Some(0) && { value = 1; true } { value = 9; } }",
            "{ while let Some(mut value) = Some(0) && { value = 1; true } { value = 9; } }",
        ] {
            check(body).unwrap_or_else(|error| panic!("{body}: {error}"));
        }
        for body in [
            "{ let update = || value; }",
            "{ let update = |value| {}; value }",
            "{ for value in 0..9 {} value }",
            "{ for value in value {} }",
            "{ match Some(0) { Some(value) => {}, _ => { value } } }",
            "{ match Some(0) { Some(other) if value() => {}, _ => {} } }",
            "{ if let Some(value) = Some(0) {} else { value } }",
            "{ if let Some(value) = value {} }",
            "{ while let Some(value) = Some(0) {} value }",
        ] {
            assert!(check(body).is_err(), "{body}");
        }
    }

    #[test]
    fn qualified_paths_and_hoisted_local_items_remain_rust_names() {
        for body in [
            "crate::value()",
            "crate::value::<u32>()",
            "{ value(); fn value() {} }",
            "{ value; const value: u32 = 9; }",
            "{ value; static value: u32 = 9; }",
            "{ value; struct value; }",
            "{ value(9); struct value(u32); }",
            "{ value(); use crate::other as value; }",
            "{ value(); use crate::{value}; }",
            "{ value(); use crate::value::{self}; }",
            "{ fn nested() { value(); } }",
            "{ let make = || -> [u8; crate::value()] { [0; 2] }; }",
            "{ const fn value() -> usize { 2 } let make = || -> [u8; value()] { [0; 2] }; }",
        ] {
            check(body).unwrap_or_else(|error| panic!("{body}: {error}"));
        }
        assert!(check("{ { fn value() {} } value() }").is_err());
        assert!(check("{ type value = u32; value() }").is_err());
    }

    #[test]
    fn opaque_bindings_use_rust_name_resolution() {
        for body in [
            "{ declare_value!(); value = 9; }",
            "{ value(); declare_value!(); }",
            "{ let binding!() = 0; value = 9; }",
            "{ assign!(value); }",
            "{ #[cfg(any())] let value = || 7; value() }",
            "{ #[cfg(any())] let value: [u8; value()] = [3; 2]; 7 }",
            "{ #[cfg(any())] let ignored = value(); 7 }",
            "{ #[cfg_attr(all(), cfg(any()))] let ignored = value(); 7 }",
            "{ value(); #[cfg(any())] fn value() {} }",
            "{ use crate::*; value() }",
        ] {
            check(body).unwrap_or_else(|error| panic!("{body}: {error}"));
        }
        assert!(check("{ { declare_value!(); } value() }").is_err());
        assert!(check("{ let binding!() = value; }").is_err());
    }
}
