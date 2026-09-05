//! Builds a choice's nested lowering when its joins fit separate case ranges.

use super::Group;
use crate::model::{Branch, ExecutionPlan, Join};

pub(super) fn dispatch(index: usize, branches: Vec<Branch>, joins: Vec<Join>) -> ExecutionPlan {
    ExecutionPlan::Choice {
        index,
        branches,
        joins,
    }
}

/// Lowering joins use participation sets; semantic convergence was already
/// validated from dependencies. Overlapping or separated ranges need guards.
pub(super) fn joinable(groups: &[Group]) -> bool {
    groups.iter().enumerate().all(|(position, (group, _))| {
        group.windows(2).all(|pair| pair[1] == pair[0] + 1)
            && groups[..position]
                .iter()
                .all(|(other, _)| group.iter().all(|case| !other.contains(case)))
    })
}
