//! Ends a body route at the transfer that repeats its cycle.

use std::collections::BTreeSet;

use super::{
    Lowered,
    verify::{Exit, Replay},
};
use crate::model::{BlockKind, ExecutionPlan};

pub(super) fn lower<'e>(index: usize) -> Lowered<'e> {
    Lowered {
        plan: ExecutionPlan::Continue { index },
        yielding: Vec::new(),
        emitted: BTreeSet::from([index]),
    }
}

/// Only the innermost active body may be the target of a native continue.
pub(super) fn replay(replay: &mut Replay<'_>, index: usize) -> Option<Exit> {
    replay.enter(index, BlockKind::Continue)?;
    let target = replay.flow.blocks[index].parent?;
    (replay.loop_indices.last() == Some(&target)).then_some(Exit::Repeat(target))
}
