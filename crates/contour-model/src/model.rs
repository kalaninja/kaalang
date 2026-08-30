//! The authored and resolved flow models, plus the compiler's execution plan.

use proc_macro2::{Ident, Span};
use syn::{Expr, FnArg, ReturnType};

/// A validated Contour flow and its verified execution plan.
pub struct Graph {
    /// The authored flow function name.
    pub name: Ident,
    /// The authored flow parameters.
    pub parameters: Vec<FnArg>,
    /// The authored flow return type.
    pub return_type: ReturnType,
    /// The resolved blocks and wires.
    pub flow: Flow,
    /// The verified control-flow plan.
    pub plan: Plan,
}

/// The semantic role of one authored block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Action,
    Question,
    Choice,
    Merge,
}

/// One authored Contour block.
pub struct Block {
    pub kind: BlockKind,
    /// The exact authored description, absent only for structural merges.
    pub description: Option<String>,
    /// The ordered authored case descriptions of a choice.
    pub case_descriptions: Vec<String>,
    pub outputs: Vec<Ident>,
    pub tuple_output: bool,
    pub output_span: Span,
    pub inputs: Vec<Input>,
    pub body: Expr,
    pub terminal: bool,
    pub span: Span,
}

/// One consuming or borrowing block input.
#[derive(Clone)]
pub struct Input {
    pub borrowed: bool,
    pub ident: Ident,
}

pub(crate) struct ParsedFlow {
    pub(crate) sources: Vec<Ident>,
    pub(crate) blocks: Vec<Block>,
}

/// A resolved flow whose wire relationships have been validated.
pub struct Flow {
    pub sources: Vec<Ident>,
    pub blocks: Vec<Block>,
}

impl Flow {
    /// Creates a flow from resolved sources and blocks.
    pub(crate) fn new(sources: Vec<Ident>, blocks: Vec<Block>) -> Self {
        Self { sources, blocks }
    }
}

/// A verified execution plan. Every Contour invariant already holds, so
/// consumers do not need to walk the authored flow a second time.
pub enum Plan {
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
        /// The index of the merge block this branch arrives at.
        merge: usize,
    },
}

/// One verified branch continuation.
pub struct Branch {
    pub plan: Box<Plan>,
    /// Set when siblings continue into a merge, which forces this terminating
    /// branch to return rather than yield a value.
    pub early_return: bool,
}

/// One verified merge and its shared continuation.
pub struct Merge {
    pub index: usize,
    pub next: Box<Plan>,
}
