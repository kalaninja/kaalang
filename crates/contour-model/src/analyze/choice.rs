//! Opens ordered choice frontiers and validates their skewer topology.

use proc_macro2::Ident;
use syn::{Error, Result};

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
        kind: WorkKind::Choice {
            id,
            index,
            branches,
            join: WorkJoin::None,
        },
        exit: None,
    })
}

/// Rejects a case that reaches End between two cases that enter a shared continuation,
/// because no skewer order can draw that crossing topology.
pub(super) fn adjacent_branches(continuing: &[bool], outputs: &[Ident]) -> Result<()> {
    let Some(first) = continuing.iter().position(|branch| *branch) else {
        return Ok(());
    };
    let last = continuing
        .iter()
        .rposition(|branch| *branch)
        .expect("a continuing branch was just found");
    let Some(offset) = continuing[first..last].iter().position(|branch| !*branch) else {
        return Ok(());
    };

    Err(Error::new(
        outputs[first + offset].span(),
        "a Contour case that reaches End must not separate cases that enter a shared continuation",
    ))
}
