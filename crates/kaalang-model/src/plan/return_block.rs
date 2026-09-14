//! Ends a root route at its explicit return.

use std::collections::BTreeSet;

use super::{
    Lowered,
    verify::{Exit, Replay},
};
use crate::model::{BlockKind, ExecutionPlan};

pub(super) fn lower<'e>(index: usize) -> Lowered<'e> {
    Lowered {
        plan: ExecutionPlan::Return { index },
        yielding: Vec::new(),
        emitted: BTreeSet::from([index]),
    }
}

pub(super) fn replay(replay: &mut Replay<'_>, index: usize) -> Option<Exit> {
    replay.enter(index, BlockKind::Return)?;
    replay
        .loop_indices
        .is_empty()
        .then_some(Exit::Return(index))
}
