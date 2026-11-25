//! Choice-body inspection shared by parsing and code generation.

use syn::{Expr, ExprMatch, Stmt};

/// Returns the sole match expression from a valid choice body.
#[must_use]
pub fn choice_match(body: &Expr) -> Option<&ExprMatch> {
    match single_body_expression(body)? {
        Expr::Match(choice) => Some(choice),
        _ => None,
    }
}

/// Extracts the only expression from a plain, unlabeled block, looking through
/// the invisible group a body written by another macro arrives in.
fn single_body_expression(body: &Expr) -> Option<&Expr> {
    let Expr::Block(block) = body else {
        return None;
    };
    if !block.attrs.is_empty() || block.label.is_some() {
        return None;
    }
    let [Stmt::Expr(expression, None)] = block.block.stmts.as_slice() else {
        return None;
    };
    Some(crate::parse::ungrouped(expression))
}
