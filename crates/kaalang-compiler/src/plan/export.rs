//! Ends a body route at the boundary consumer exporting its cycle's output.

use std::collections::BTreeSet;

use super::{
    Lowered,
    verify::{Exit, Replay},
};
use crate::model::{BlockKind, ExecutionPlan, Flow};

pub(super) fn lower<'e>(flow: &Flow, index: usize) -> Lowered<'e> {
    Lowered {
        plan: ExecutionPlan::Export {
            index,
            target: flow.blocks[index]
                .export_target
                .expect("a boundary consumer has a cycle"),
        },
        yielding: Vec::new(),
        emitted: BTreeSet::from([index]),
    }
}

pub(super) fn replay(replay: &mut Replay<'_>, index: usize, target: usize) -> Option<Exit> {
    replay.enter(index, BlockKind::Export)?;
    // A cycle with several outputs exports the one this execution selected.
    let selected = replay.flow.blocks[target].branch_count() == 0
        || replay.execution.selected(target) == Some(replay.flow.exported_output(index));
    (replay.flow.blocks[index].export_target == Some(target)
        && replay.cycle_indices.last() == Some(&target)
        && selected)
        .then_some(Exit::Export(target))
}
