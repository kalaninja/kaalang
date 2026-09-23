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

/// Reports whether a block body is exactly an argument-free `todo!()` call.
#[must_use]
pub fn is_todo_body(body: &Expr) -> bool {
    single_body_expression(body).is_some_and(is_todo_macro)
}

/// Reports whether an expression is an argument-free `todo!()` call.
pub(crate) fn is_todo_macro(expression: &Expr) -> bool {
    let Expr::Macro(expression) = expression else {
        return false;
    };
    expression.attrs.is_empty()
        && expression.mac.path.is_ident("todo")
        && expression.mac.tokens.is_empty()
}

/// Extracts the only expression from a plain, unlabeled block.
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
    Some(expression)
}
