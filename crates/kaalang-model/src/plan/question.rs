//! Builds a question's nested lowering plan.

use crate::model::{Branch, ExecutionPlan, Join};

pub(super) fn dispatch(index: usize, branches: Vec<Branch>, mut joins: Vec<Join>) -> ExecutionPlan {
    let Ok(branches) = <[Branch; 2]>::try_from(branches) else {
        unreachable!("a question declares exactly two outputs")
    };
    debug_assert!(joins.len() <= 1, "two branches form at most one join");
    ExecutionPlan::Question {
        index,
        branches,
        join: joins.pop(),
    }
}
