//! Analyzes an action and its single continuation.

use syn::Result;

use super::{Analysis, PathState, Walked};
use crate::model::Plan;

/// Adds an action's outputs and analyzes its continuation.
pub(super) fn walk(analysis: &mut Analysis<'_>, index: usize, state: PathState) -> Result<Walked> {
    let mut next = analysis.enter(index, state);
    for output in &analysis.flow.blocks[index].outputs {
        next.produce(output)?;
    }
    let continuation = analysis.walk(next)?;

    Ok(Walked {
        plan: Plan::Action {
            index,
            next: Box::new(continuation.plan),
        },
        exit: continuation.exit,
    })
}
