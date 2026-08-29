//! The authored and resolved flow models, plus the compiler's execution plan.

use std::collections::HashMap;

use proc_macro2::{Ident, Span};
use syn::Expr;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockKind {
    Action,
    Question,
    Choice,
    Merge,
}

pub(crate) struct Block {
    pub(crate) kind: BlockKind,
    pub(crate) outputs: Vec<Ident>,
    pub(crate) tuple_output: bool,
    pub(crate) output_span: Span,
    pub(crate) inputs: Vec<Input>,
    pub(crate) body: Expr,
    pub(crate) terminal: bool,
    pub(crate) span: Span,
}

#[derive(Clone)]
pub(crate) struct Input {
    pub(crate) borrowed: bool,
    pub(crate) ident: Ident,
}

pub(crate) struct ParsedFlow {
    pub(crate) sources: Vec<Ident>,
    pub(crate) blocks: Vec<Block>,
}

/// A resolved flow where every wire has a producer and an internal binding.
pub(crate) struct Flow {
    pub(crate) sources: Vec<Ident>,
    pub(crate) blocks: Vec<Block>,
    wires: HashMap<String, Ident>,
}

impl Flow {
    /// Creates a flow from resolved blocks and their internal wire bindings.
    pub(crate) fn new(
        sources: Vec<Ident>,
        blocks: Vec<Block>,
        wires: HashMap<String, Ident>,
    ) -> Self {
        Self {
            sources,
            blocks,
            wires,
        }
    }

    /// Returns the internal Rust binding assigned to a semantic wire name.
    pub(crate) fn wire(&self, name: &Ident) -> &Ident {
        self.wires
            .get(&name.to_string())
            .expect("validated wires have internal bindings")
    }
}

/// A verified execution plan. Every Contour invariant already holds, so code
/// generation reads this instead of walking the flow a second time.
pub(crate) enum Plan {
    Action {
        index: usize,
        next: Box<Plan>,
    },
    Question {
        index: usize,
        branches: [Branch; 2],
        merge: Option<Merge>,
    },
    Choice {
        index: usize,
        branches: Vec<Branch>,
        merge: Option<Merge>,
    },
    /// The path ends here and its value is this terminal action output.
    Terminal {
        output: Ident,
    },
    /// The branch reaches a merge and hands over this input.
    Arrival {
        input: Ident,
    },
}

pub(crate) struct Branch {
    pub(crate) plan: Box<Plan>,
    /// Set when siblings continue into a merge, which forces this terminating
    /// branch to return rather than yield a value.
    pub(crate) early_return: bool,
}

pub(crate) struct Merge {
    pub(crate) index: usize,
    pub(crate) next: Box<Plan>,
}
