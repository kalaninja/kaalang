//! Opens the two positional frontiers of a question.

use syn::Result;

use super::{
    Analysis, PathState, Producer,
    frontier::{WorkJoin, WorkKind, WorkPlan},
};

pub(super) fn enter(
    analysis: &mut Analysis<'_>,
    index: usize,
    state: PathState,
) -> Result<WorkPlan> {
    let next = analysis.enter(index, state);
    let outputs = &analysis.flow.blocks[index].outputs;
    let mut branches = Vec::with_capacity(outputs.len());
    for (output, ident) in outputs.iter().enumerate() {
        let mut branch = next.clone();
        branch.produce(ident, Producer::Block { index, output })?;
        branches.push(analysis.open(branch));
    }
    let id = analysis.branch_id();
    Ok(WorkPlan {
        kind: WorkKind::Question {
            id,
            index,
            branches,
            join: WorkJoin::None,
        },
        exit: None,
    })
}
