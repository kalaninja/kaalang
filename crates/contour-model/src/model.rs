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
    End,
}

/// One authored Contour block.
pub struct Block {
    pub kind: BlockKind,
    /// The exact authored description, absent for structural blocks.
    pub description: Option<String>,
    /// The ordered authored case descriptions of a choice.
    pub case_descriptions: Vec<String>,
    pub outputs: Vec<Ident>,
    pub output_span: Span,
    pub inputs: Vec<Input>,
    pub body: Expr,
    pub span: Span,
}

/// One consuming or borrowing block input.
pub struct Input {
    pub borrowed: bool,
    /// The logical wire name: raw and ordinary spellings normalize to one ident.
    pub ident: Ident,
    /// The authored spelling, which keeps `r#` so a keyword-named wire binds.
    pub alias: Ident,
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
        convergence: Option<Convergence>,
    },
    Choice {
        index: usize,
        branches: Vec<Branch>,
        convergence: Option<Convergence>,
    },
    /// The one authored End block and the execution that feeds it.
    End {
        index: usize,
        body: Box<Plan>,
    },
    /// One path yields its ordered values to End.
    EndArrival {
        inputs: Vec<Ident>,
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
    /// Set when siblings enter a shared continuation, which forces this path
    /// to return its End value rather than yield it to the enclosing expression.
    pub early_return: bool,
}

/// Logical wire bindings and the continuation shared by sibling paths.
pub struct Convergence {
    /// Logical wire names, ordered by the shared consumer's inputs.
    pub wires: Vec<Ident>,
    pub next: Box<Plan>,
}
