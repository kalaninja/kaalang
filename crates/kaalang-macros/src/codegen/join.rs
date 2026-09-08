//! Binds alternative producer values and emits their shared continuation.

use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{quote, quote_spanned};

use super::Bindings;
use kaalang_model::{Branch, ExecutionPlan, Flow, Join, JoinTarget};

/// One question or choice enclosing the code being emitted.
#[derive(Clone, Copy)]
pub(crate) struct Frame<'a> {
    pub(crate) block: usize,
    /// The destinations that this block's branches may yield into.
    pub(crate) join: Option<&'a JoinRouting>,
}

impl<'a> Frame<'a> {
    /// The scope of a branch: the enclosing scope plus its own block.
    pub(crate) fn nest(
        scope: &[Frame<'a>],
        block: usize,
        join: Option<&'a JoinRouting>,
    ) -> Vec<Self> {
        let mut inner = scope.to_vec();
        inner.push(Frame { block, join });
        inner
    }
}

/// Distinguishes local joins from each other and from outward yields.
pub(crate) struct JoinRouting {
    joins: usize,
    through: bool,
    passed: Ident,
}

impl JoinRouting {
    pub(crate) fn new(
        index: usize,
        joins: usize,
        branches: &[Branch],
        scope: &[Frame<'_>],
    ) -> Option<Self> {
        let through = branches
            .iter()
            .any(|branch| yields_past(&branch.plan, scope));
        let variants = joins + usize::from(through);
        // A single destination binds a plain value; multiple destinations
        // use nested Result values, including a local join and an outward yield.
        (variants >= 2).then(|| Self {
            joins,
            through,
            passed: Ident::new(&format!("__kaalang_passed_{index}"), Span::mixed_site()),
        })
    }

    /// Wraps a destination as either a value or a pattern.
    fn variant(&self, value: TokenStream2, index: usize) -> TokenStream2 {
        nested(value, index, self.joins + usize::from(self.through))
    }
}

/// Tags a value or pattern as one of `count` alternatives with nested standard
/// `Result` variants, which need no generated type item in the author's scope:
/// `Ok(v)`, `Err(Ok(v))`, ..., `Err(Err(v))`.
pub(super) fn nested(mut value: TokenStream2, index: usize, count: usize) -> TokenStream2 {
    debug_assert!(index < count);
    if index + 1 < count {
        value = quote!(::core::result::Result::Ok(#value));
    }
    for _ in 0..index {
        value = quote!(::core::result::Result::Err(#value));
    }
    value
}

/// Only yields targeting an enclosing scope pass this brancher. Nested joins
/// resolve inside their own brancher and need no pass-through variant here.
fn yields_past(plan: &ExecutionPlan, scope: &[Frame<'_>]) -> bool {
    match plan {
        ExecutionPlan::Action { next, .. } | ExecutionPlan::End { body: next, .. } => {
            yields_past(next, scope)
        }
        ExecutionPlan::Question { branches, join, .. } => {
            branches
                .iter()
                .any(|branch| yields_past(&branch.plan, scope))
                || join
                    .as_ref()
                    .is_some_and(|join| yields_past(&join.next, scope))
        }
        ExecutionPlan::Choice {
            branches, joins, ..
        } => {
            branches
                .iter()
                .any(|branch| yields_past(&branch.plan, scope))
                || joins.iter().any(|join| yields_past(&join.next, scope))
        }
        ExecutionPlan::EndArrival { .. } | ExecutionPlan::Guarded { .. } => false,
        ExecutionPlan::Yield { join, .. } => scope.iter().any(|frame| frame.block == join.block),
    }
}

pub(super) fn value(bindings: &Bindings, wires: &[Ident]) -> TokenStream2 {
    let wires = wires
        .iter()
        .map(|wire| bindings.wire_at(wire))
        .collect::<Vec<_>>();
    super::tuple(Span::call_site(), &wires)
}

/// The value a yield hands to its join, wrapped in the join's variant and then
/// in the pass-through variant of every brancher between the join and the yield.
pub(super) fn yield_value(
    bindings: &Bindings,
    wires: &[Ident],
    join: JoinTarget,
    scope: &[Frame<'_>],
) -> TokenStream2 {
    let target = scope
        .iter()
        .position(|frame| frame.block == join.block)
        .expect("a yield targets an enclosing join");
    let mut value = value(bindings, wires);
    if let Some(routing) = scope[target].join {
        value = routing.variant(value, join.join);
    }
    for frame in &scope[target + 1..] {
        if let Some(routing) = frame.join {
            debug_assert!(routing.through, "an outward yield has a destination");
            value = routing.variant(value, routing.joins);
        }
    }
    value
}

/// Binds each join's wires and emits its shared continuation once. Values
/// destined for an enclosing join pass through without running a local one.
pub(super) fn emit(
    flow: &Flow,
    bindings: &Bindings,
    branch: TokenStream2,
    joins: &[Join],
    routing: Option<&JoinRouting>,
    span: Span,
    scope: &[Frame<'_>],
) -> TokenStream2 {
    let Some(routing) = routing else {
        let Some(join) = joins.first() else {
            return branch;
        };
        let pattern = value(bindings, &join.wires);
        let continuation =
            super::continuation(flow, &join.next, join.early_return, bindings, scope);
        return quote_spanned! {span=>
            let #pattern = #branch;
            #continuation
        };
    };
    let mut arms = joins
        .iter()
        .enumerate()
        .map(|(index, join)| {
            let pattern = routing.variant(value(bindings, &join.wires), index);
            let continuation =
                super::continuation(flow, &join.next, join.early_return, bindings, scope);
            quote!(#pattern => { #continuation },)
        })
        .collect::<Vec<_>>();
    if routing.through {
        let passed = &routing.passed;
        let pattern = routing.variant(quote!(#passed), routing.joins);
        arms.push(quote!(#pattern => #passed,));
    }

    quote! {
        match #branch {
            #(#arms)*
        }
    }
}
