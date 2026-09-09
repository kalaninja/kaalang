//! Builds a choice's nested lowering when its joins fit nested case ranges.

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
/// validated from dependencies. Each join covers a contiguous case range, and
/// two ranges are disjoint or one contains the other: a contained range joins
/// first and hands its value to the containing one. The branch rule of RFC 0001
/// §7 admits no other shape, so a false answer here marks a compiler bug.
pub(super) fn joinable(groups: &[Group]) -> bool {
    groups.iter().enumerate().all(|(position, (group, _))| {
        group.windows(2).all(|pair| pair[1] == pair[0] + 1)
            && groups[..position].iter().all(|(other, _)| {
                let shared = group.iter().filter(|case| other.contains(case)).count();
                shared == 0 || shared == group.len().min(other.len())
            })
    })
}
