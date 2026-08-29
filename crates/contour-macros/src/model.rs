//! The flow as its author declared it, and the plan the compiler derived.

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
    pub(crate) inputs: Vec<Capture>,
    pub(crate) body: Expr,
    pub(crate) terminal: bool,
    pub(crate) span: Span,
}

#[derive(Clone)]
pub(crate) struct Capture {
    pub(crate) borrowed: bool,
    pub(crate) ident: Ident,
}

pub(crate) struct ParsedFlow {
    pub(crate) sources: Vec<Ident>,
    pub(crate) blocks: Vec<Block>,
}

/// Resolved flow: every wire name has a producer and an internal binding.
pub(crate) struct Graph {
    pub(crate) sources: Vec<Ident>,
    pub(crate) blocks: Vec<Block>,
    wires: HashMap<String, Ident>,
}

impl Graph {
    /// Creates a graph from resolved blocks and their internal wire bindings.
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
/// generation reads this instead of walking the graph a second time.
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
