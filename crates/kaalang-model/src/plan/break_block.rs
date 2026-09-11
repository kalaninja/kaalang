//! Ends a body route at its explicitly resolved loop exit.

use std::collections::BTreeSet;

use super::{
    Lowered,
    verify::{Exit, Replay},
};
use crate::model::{BlockKind, ExecutionPlan, Flow};

pub(super) fn lower<'e>(flow: &Flow, index: usize) -> Lowered<'e> {
    Lowered {
        plan: ExecutionPlan::Break {
            index,
            target: flow.blocks[index]
                .break_target
                .expect("a break has a target"),
        },
        yielding: Vec::new(),
        emitted: BTreeSet::from([index]),
    }
}

pub(super) fn replay(replay: &mut Replay<'_>, index: usize, target: usize) -> Option<Exit> {
    replay.enter(index, BlockKind::Break)?;
    (replay.flow.blocks[index].break_target == Some(target)
        && replay.loop_indices.contains(&target))
    .then_some(Exit::Break(target))
}
