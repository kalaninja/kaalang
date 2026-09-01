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
    pub output_span: Span,
    pub inputs: Vec<Input>,
    pub body: Expr,
    pub terminal: bool,
    pub span: Span,
}

/// One consuming or borrowing block input.
pub struct Input {
    pub borrowed: bool,
    pub ident: Ident,
}

/// A flow's source wires and blocks, wire-validated by the time consumers see it.
pub struct Flow {
    pub sources: Vec<Ident>,
    pub blocks: Vec<Block>,
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
        convergence: Option<Convergence>,
    },
    Choice {
        index: usize,
        branches: Vec<Branch>,
        merge: Option<Merge>,
        convergence: Option<Convergence>,
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
    /// The branch yields path-exclusive producer values to an implicit
    /// convergence.
    Yield {
        wires: Vec<Ident>,
    },
}

/// One verified branch continuation.
pub struct Branch {
    pub plan: Box<Plan>,
    /// Set when siblings enter a shared continuation, which forces this
    /// terminating branch to return rather than yield a value.
    pub early_return: bool,
}

/// One verified merge and its shared continuation.
pub struct Merge {
    pub index: usize,
    pub next: Box<Plan>,
}

/// Logical wire bindings and the continuation shared by sibling paths.
pub struct Convergence {
    /// Logical wire names, ordered by the shared consumer's inputs.
    pub wires: Vec<Ident>,
    pub next: Box<Plan>,
}
