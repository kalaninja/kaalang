//! Builds a choice's nested lowering when its joins fit nested case ranges.

use std::collections::BTreeSet;

use crate::model::{Branch, ExecutionPlan, Join};

pub(super) fn dispatch(index: usize, branches: Vec<Branch>, joins: Vec<Join>) -> ExecutionPlan {
    ExecutionPlan::Choice {
        index,
        branches,
        joins,
    }
}

/// Lowering joins use participation sets; semantic convergence was already
/// validated from dependencies. Each join is given by the executions it takes
/// and those that reach its level at all. Two joins are disjoint or one
/// contains the other, counting only executions that could reach both: a later
/// selection inside one case may send its routes to different joins, and a
/// repeat of an inner cycle never reaches a join outside it (RFC 0001 §7 as
/// refined by RFC 0006 §9). Every join wraps the whole dispatch, so case order
/// does not constrain lowering. A false answer here marks a compiler bug.
pub(super) fn joinable(joins: &[(BTreeSet<usize>, BTreeSet<usize>)]) -> bool {
    let apart = |(taken, _): &(BTreeSet<usize>, BTreeSet<usize>),
                 (other, reaching): &(BTreeSet<usize>, BTreeSet<usize>)| {
        taken
            .iter()
            .any(|execution| !other.contains(execution) && reaching.contains(execution))
    };
    joins.iter().enumerate().all(|(position, join)| {
        joins[..position]
            .iter()
            .all(|other| join.0.is_disjoint(&other.0) || !apart(join, other) || !apart(other, join))
    })
}
