//! Analyzes the two positional branches of a question.

use syn::Result;

use super::{Analysis, PathState, Walked};
use crate::model::{Branch, Plan};

/// Walks the yes and no branches and records their shared continuation.
pub(super) fn walk(analysis: &mut Analysis<'_>, index: usize, state: PathState) -> Result<Walked> {
    let next = analysis.enter(index, state);
    let outputs = &analysis.flow.blocks[index].outputs;
    let mut yes_state = next.clone();
    yes_state.available.insert(outputs[0].clone());
    let mut no_state = next;
    no_state.available.insert(outputs[1].clone());
    let walked = vec![analysis.walk(yes_state)?, analysis.walk(no_state)?];
    let (branches, merge, exit) = analysis.branches(walked)?;
    let Ok(branches) = <[Branch; 2]>::try_from(branches) else {
        unreachable!("a question declares exactly two outputs")
    };

    Ok(Walked {
        plan: Plan::Question {
            index,
            branches,
            merge,
        },
        exit,
    })
}
