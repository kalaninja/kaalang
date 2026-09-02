//! Opens an action and its single continuation frontier.

use syn::Result;

use super::{
    Analysis, PathState, Producer,
    frontier::{WorkKind, WorkPlan},
};

pub(super) fn enter(
    analysis: &mut Analysis<'_>,
    index: usize,
    state: PathState,
) -> Result<WorkPlan> {
    let mut next = analysis.enter(index, state);
    for output in &analysis.flow.blocks[index].outputs {
        next.produce(output, Producer::Block(index))?;
    }
    Ok(WorkPlan {
        kind: WorkKind::Action {
            index,
            next: Box::new(analysis.open(next)),
        },
        exit: None,
    })
}
